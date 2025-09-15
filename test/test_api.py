#!/usr/bin/env python3

import unittest
import requests
import time
import random
import string

from . import dt_types
from .api import DTConfig, stream_sse


def random_string() -> str:
    return "".join(random.choices(string.ascii_letters + string.digits, k=4))


class HaukApiTest(unittest.TestCase):
    # Base URL with the new .php extension
    BASE_URL = "http://127.0.0.1:8080/api/"
    TEST_USERNAME = "testuser"
    TEST_PASSWORD = "testpassword"

    def setUp(self) -> None:
        self.api_config = DTConfig(
            base_url=self.BASE_URL,
            username=self.TEST_USERNAME,
            password=self.TEST_PASSWORD,
        )

    def parse_response(self, response: str) -> list[str]:
        lines = response.strip().split("\n")
        self.assertEqual(lines[0], "OK")
        return lines

    def test_create_and_fetch_session(self) -> None:
        """Tests the session creation and basic data retrieval."""
        session_id: str = ""
        response: requests.Response
        sse_response: requests.Response | None = None  # To ensure cleanup

        try:
            tag = f"test_create_and_fetch_session_{random_string()}"

            # Data for the new create.php endpoint
            create_data = {
                "usr": self.TEST_USERNAME,
                "pwd": self.TEST_PASSWORD,
                "mod": 0,
                "lid": tag,
                "dur": 3600,
                "int": 30,
            }

            # 1. Test create_session endpoint. The request is now a POST with form data.
            response = requests.post(f"{self.BASE_URL}create.php", data=create_data)
            self.assertEqual(response.status_code, 200)

            # The response is still "OK\n<session_id>"
            lines = self.parse_response(response.text)
            session_id = lines[1]
            share_link = lines[2]
            share_id = lines[3]

            # 2. Test fetch_location endpoint with the new session ID
            stream_gen, sse_response = stream_sse(
                self.api_config, [tag], stop_on_quiescent=True
            )
            first = next(stream_gen)
            self.assertEqual(first.changes[0], "reset")
            self.assertEqual(list(first.changes[1].add_fetch.tags.values()), [[tag]])
            self.assertEqual(list(first.changes[2].add.points.values()), [[]])

        except requests.exceptions.RequestException as e:
            self.fail(f"HTTP request failed: {e}")
        except (ValueError, IndexError):
            self.fail(
                f"Invalid text response from server. Response text: {response.text}"
            )
        finally:
            if sse_response:
                sse_response.close()

    def test_post_and_fetch_location(self) -> None:
        """Tests posting a location and fetching it."""
        session_id: str = ""
        sse_response: requests.Response | None = None  # To ensure cleanup

        try:
            tag = f"test_post_and_fetch_location_{random_string()}"
            # Create a session first, using the new endpoint and parsing.
            create_data = {
                "usr": self.TEST_USERNAME,
                "pwd": self.TEST_PASSWORD,
                "mod": 0,
                "lid": tag,
                "dur": 3600,
                "int": 30,
            }
            response = requests.post(f"{self.BASE_URL}create.php", data=create_data)
            self.assertEqual(response.status_code, 200)
            lines = response.text.strip().split("\n")
            self.assertEqual(lines[0], "OK")
            session_id = lines[1]
            share_id = lines[3]

            # Define the location data to post
            # The data is now sent as form-urlencoded, not JSON.
            location_data = {
                "sid": session_id,
                "lat": 34.0522,
                "lon": -118.2437,
                "acc": 10.5,
                "alt": 500,
                "speed": 60,
                "dir": 90,
                "batt": 75,
                "prv": 0,
                "time": int(time.time()),
            }

            # 1. Test post_location endpoint with form data
            response = requests.post(f"{self.BASE_URL}post.php", data=location_data)
            self.assertEqual(response.status_code, 200)

            # Check for the simple "OK" response.
            self.assertEqual(self.parse_response(response.text)[0], "OK")

            # 2. Test fetch_location to retrieve the posted data
            stream_gen, sse_response = stream_sse(
                self.api_config, [tag], stop_on_quiescent=True
            )
            first = next(stream_gen)
            self.assertEqual(first.changes[0], "reset")
            self.assertEqual(list(first.changes[1].add_fetch.tags.values()), [[tag]])
            points = list(first.changes[2].add.points.values())
            self.assertAlmostEqual(points[0][0][0], location_data["lat"])
            self.assertAlmostEqual(points[0][0][1], location_data["lon"])

        except requests.exceptions.RequestException as e:
            self.fail(f"HTTP request failed: {e}")
        except (ValueError, IndexError):
            self.fail(
                f"Invalid text response from server. Response text: {response.text}"
            )
        finally:
            if sse_response:
                sse_response.close()

    def test_post_and_fetch_location_no_tag(self) -> None:
        """
        Tests posting a location and then trying to fetch it without providing its tag.
        It expects no relevant SSE events to be received.
        """
        session_id: str = ""
        sse_response: requests.Response | None = None

        try:
            tag = f"test_post_and_fetch_location_no_tag_{random_string()}"
            # Create a session first
            create_data = {
                "usr": self.TEST_USERNAME,
                "pwd": self.TEST_PASSWORD,
                "mod": 0,
                "lid": tag,
                "dur": 5,
                "int": 30,
            }
            response = requests.post(f"{self.BASE_URL}create.php", data=create_data)
            self.assertEqual(response.status_code, 200)
            lines = self.parse_response(response.text)
            session_id = lines[1]
            share_id = lines[3]  # Not directly used but parsed for completeness

            # Post location data associated with the created tag
            location_data = {
                "sid": session_id,
                "lat": 34.0522,
                "lon": -118.2437,
                "acc": 10.5,
                "alt": 500,
                "speed": 60,
                "dir": 90,
                "batt": 75,
                "prv": 0,
                "time": int(time.time()),
            }
            response = requests.post(f"{self.BASE_URL}post.php", data=location_data)
            self.assertEqual(response.status_code, 200)
            self.assertEqual(self.parse_response(response.text)[0], "OK")

            # Try to fetch events WITHOUT providing the specific tag, for at most 2 seconds.
            # We expect no events related to the posted location or session.
            stream_gen, sse_response = stream_sse(
                self.api_config, [], read_timeout=2, stop_on_quiescent=True
            )  # Empty list of tags
            collected_events: list[dt_types.StreamEvent] = []

            try:
                # Iterate over the generator. A requests.exceptions.Timeout will be raised
                # if no data (including keep-alives) is received within the read_timeout period.
                for event in stream_gen:
                    collected_events.append(event)
            except requests.exceptions.Timeout:
                # This is an expected outcome if the server correctly sends no events
                # and the connection eventually times out due to inactivity.
                pass
            except StopIteration:
                # The generator finished naturally, e.g., if the server closed the connection.
                pass
            except Exception as e:
                self.fail(f"An unexpected error occurred during SSE streaming: {e}")

            # Assertions: We should not find any dt_types.AddTags or Add events corresponding to our test tag/session_id
            found_relevant_changes = False
            for event in collected_events:
                for change in event.changes:
                    if isinstance(change, dt_types.AddTags):
                        # Check if this dt_types.AddTags event contains the tag we just created
                        for fetch_id_tags in change.add_fetch.tags.values():
                            if tag in fetch_id_tags:
                                found_relevant_changes = True
                                break
                if found_relevant_changes:
                    break

            self.assertFalse(
                found_relevant_changes,
                f"Found unexpected dt_types.AddTags or Add events for tag '{tag}' / session '{session_id}' "
                f"while not subscribed to the tag. Collected events: {collected_events}",
            )

        except requests.exceptions.RequestException as e:
            self.fail(f"HTTP request failed: {e}")
        except (ValueError, IndexError):
            self.fail(
                f"Invalid text response from server. Response text: {response.text}"
            )
        finally:
            if sse_response:
                sse_response.close()  # Ensure the SSE connection is closed

    def test_public_tag_stream_awareness(self) -> None:
        """
        Tests that a fetch session subscribed to public tags becomes aware of a newly
        published public tag stream.
        """
        session_id: str = ""
        sse_response: requests.Response | None = None

        try:
            # 1. Create a session with a PUBLIC tag
            public_tag = f"test_public_tag_{random_string()}"
            create_data = {
                "usr": self.TEST_USERNAME,
                "pwd": self.TEST_PASSWORD,
                "mod": 0,  # 0 for public tag
                "lid": f"public:{public_tag}",
                "dur": 3600,
                "int": 30,
            }
            response = requests.post(f"{self.BASE_URL}create.php", data=create_data)
            self.assertEqual(response.status_code, 200)
            lines = self.parse_response(response.text)
            session_id = lines[1]
            # No need to post location data for this test, as we only care about the tag's existence.

            # 2. Start a fetch session WITHOUT specifying any tags to subscribe to ALL public tags
            stream_gen, sse_response = stream_sse(
                self.api_config, [], read_timeout=5, stop_on_quiescent=True
            )  # Empty list subscribes to public tags

            first_event: dt_types.StreamEvent | None = None
            try:
                first_event = next(stream_gen)
            except StopIteration:
                self.fail(
                    "SSE stream ended prematurely before receiving initial event."
                )
            except requests.exceptions.Timeout:
                self.fail("SSE stream timed out before receiving initial event.")

            self.assertIsNotNone(first_event, "Did not receive any initial SSE event.")
            self.assertEqual(
                first_event.changes[0], "reset", "First change was not 'reset'"
            )

            # 3. Assert that the fetch session is aware of the new public tag
            found_public_tag = False
            for change in first_event.changes:
                if isinstance(change, dt_types.AddTags):
                    # Check if the public_tag is present in the general tags list for any fetch_id
                    for fetch_id_tags in change.add_fetch.tags.values():
                        if public_tag in fetch_id_tags:
                            # And specifically if it's marked as a public tag for that fetch_id
                            if public_tag in change.add_fetch.public:
                                found_public_tag = True
                                break
                if found_public_tag:
                    break

            self.assertTrue(
                found_public_tag,
                f"Fetch session was not aware of public tag '{public_tag}' in initial events. "
                f"Collected events: {first_event}",
            )

        except requests.exceptions.RequestException as e:
            self.fail(f"HTTP request failed: {e}")
        except (ValueError, IndexError):
            self.fail(
                f"Invalid text response from server. Response text: {response.text}"
            )
        finally:
            if sse_response:
                sse_response.close()

    def test_public_tag_stream_awareness_late_publish(self) -> None:
        """
        Tests that a fetch session, started before a public tag is published,
        becomes aware of that public tag once it is published.
        """
        sse_response: requests.Response | None = None

        try:
            public_tag = f"test_public_tag_late_{random_string()}"

            # 1. Start a fetch session WITHOUT specifying any tags to subscribe to ALL public tags.
            # Use a long enough read_timeout to allow for tag creation and event propagation.
            stream_gen, sse_response = stream_sse(
                self.api_config, [], read_timeout=15, stop_on_quiescent=True
            )  # 15 seconds for this scenario

            collected_events: list[dt_types.StreamEvent] = []
            found_public_tag = False

            # 2. Consume any initial 'reset' or other events until we create our tag,
            # or until the stream eventually times out/closes.
            # We'll also collect events for debugging if the assertion fails.

            # 3. Publish the public tag after the SSE stream is established.
            create_data = {
                "usr": self.TEST_USERNAME,
                "pwd": self.TEST_PASSWORD,
                "mod": 0,  # 0 for public tag
                "lid": f"public:{public_tag}",
                "dur": 10,
                "int": 30,
            }
            response = requests.post(f"{self.BASE_URL}create.php", data=create_data)
            self.assertEqual(
                response.status_code, 200, "Failed to create public tag session."
            )
            lines = self.parse_response(response.text)  # Check for OK

            # 4. Wait for and verify the dt_types.AddTags event on the already-open stream.
            try:
                for event in stream_gen:
                    collected_events.append(event)
                    for change in event.changes:
                        if isinstance(change, dt_types.AddTags):
                            if public_tag in change.add_fetch.public:
                                found_public_tag = True
                                break
                        if found_public_tag:
                            break
                    if found_public_tag:
                        break
            except requests.exceptions.Timeout:
                # Expected if the event doesn't arrive within read_timeout and no other data.
                pass
            except StopIteration:
                # Generator finished naturally (e.g., server closed connection).
                pass
            except Exception as e:
                self.fail(
                    f"An unexpected error occurred during SSE streaming after tag publish: {e}"
                )

            self.assertTrue(
                found_public_tag,
                f"Fetch session was not aware of public tag '{public_tag}' after it was published. "
                f"Collected events: {[e.model_dump_json() for e in collected_events]}",
            )

        except requests.exceptions.RequestException as e:
            self.fail(f"HTTP request failed: {e}")
        except (ValueError, IndexError):
            self.fail(
                f"Invalid text response from server. Response text: {response.text}"
            )
        finally:
            if sse_response:
                sse_response.close()


if __name__ == "__main__":
    unittest.main()
