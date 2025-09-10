#!/usr/bin/env python3

import unittest
import requests
import json
import time
import typing
import sseclient
import random
import string

from pydantic import BaseModel, RootModel
from typing import Any, Literal, Generator, TypeAlias


def random_string() -> str:
    return "".join(random.choices(string.ascii_letters + string.digits, k=4))


class AddTagsTags(BaseModel):
    tags: dict[str, list[str]]  # Set of tags (in general) published by a fetch_id
    public: set[str]  # Public tags published by a fetch_id


class AddTags(BaseModel):
    add_fetch: AddTagsTags  # tags added by this add tags update


# lat, lon, time (epoch in micros), speed, accuracy, GPS source (0 or 1)
Point: TypeAlias = tuple[float, float, float, float | None, float | None, float]


class AddPoints(BaseModel):
    points: dict[str, list[Point]]  # points added by this add points update


class Add(BaseModel):
    add: AddPoints  # Add update


class ExpireFetch(BaseModel):
    fetch_id: str  # Fetch_id expired


# Reset = Remove everything in client
Change = Literal["reset"] | AddTags | Add | ExpireFetch


class LoginResponse(BaseModel):
    token: str


class Meta(BaseModel):
    serverTime: int
    interval: int


class StreamEvent(BaseModel):
    meta: Meta
    changes: list[Change]


class HaukApiTest(unittest.TestCase):
    # Base URL with the new .php extension
    BASE_URL = "http://127.0.0.1:8080/api/"
    TEST_USERNAME = "testuser"
    TEST_PASSWORD = "testpassword"

    def parse_response(self, response: str) -> list[str]:
        lines = response.strip().split("\n")
        self.assertEqual(lines[0], "OK")
        return lines

    def stream_sse(self, tags: list[str]) -> Generator[StreamEvent, None, None]:
        response = requests.post(
            f"{self.BASE_URL}login",
            json={"username": self.TEST_USERNAME, "password": self.TEST_PASSWORD},
        )
        token = LoginResponse.model_validate(response.json()).token

        headers = {"Accept": "text/event-stream"}
        tags_str = ",".join(tags)
        response = requests.get(
            f"{self.BASE_URL}stream?token={token}&tags={tags_str}",
            stream=True,
            headers=headers,
        )
        client = sseclient.SSEClient(response)
        return (
            StreamEvent.model_validate_json(event.data) for event in client.events()
        )

    def test_create_and_fetch_session(self) -> None:
        """Tests the session creation and basic data retrieval."""
        session_id: str = ""
        response: requests.Response

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
            stream_sse = self.stream_sse([tag])
            first = next(stream_sse)
            self.assertEqual(first.changes[0], "reset")
            self.assertEqual(list(first.changes[1].add_fetch.tags.values()), [[tag]])
            self.assertEqual(list(first.changes[2].add.points.values()), [[]])

        except requests.exceptions.RequestException as e:
            self.fail(f"HTTP request failed: {e}")
        except (ValueError, IndexError):
            self.fail(
                f"Invalid text response from server. Response text: {response.text}"
            )

    def test_post_and_fetch_location(self) -> None:
        """Tests posting a location and fetching it."""
        session_id: str = ""
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
            stream_sse = self.stream_sse([tag])
            first = next(stream_sse)
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


if __name__ == "__main__":
    unittest.main()
