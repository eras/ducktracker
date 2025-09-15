#!/usr/bin/env python3

import argparse
import sys
import time
import random
import signal
import requests  # Needed for closing the requests.Response object

from typing import Any, NoReturn, Optional, Set, Generator, List

from . import api, dt_types


# --- Global Flags for Signal Handling ---
_STOP_SIGNAL_RECEIVED: bool = False

# --- Configuration Constants ---
DEFAULT_INITIAL_TAGS: list[str] = [
    "work",
    "home",
    "travel",
    "park",
    "city",
    "street",
]
DEFAULT_NUM_TAGS_TO_SAMPLE: int = (
    2  # How many tags to pick for a subscription when reconnecting
)


# --- Helper Functions ---
def exit_with_error(message: str, exit_code: int = 1) -> NoReturn:
    """Prints an error message to stderr and exits."""
    print(f"Error: {message}", file=sys.stderr)
    sys.exit(exit_code)


# --- Signal Handler ---
def signal_handler(signum: int, frame: Any) -> None:
    """Sets a global flag when Ctrl+C (SIGINT) is received."""
    global _STOP_SIGNAL_RECEIVED
    _STOP_SIGNAL_RECEIVED = True
    print("\nCtrl+C detected. Attempting graceful shutdown...", file=sys.stderr)


# --- Core Logic Functions ---


def parse_arguments() -> argparse.Namespace:
    """
    Parses command-line arguments for the client simulator.

    Returns:
        argparse.Namespace: An object containing the parsed arguments.
    """
    parser = argparse.ArgumentParser(
        description="Simulates a client subscribing to a tracking service's SSE stream."
    )
    parser.add_argument(
        "endpoint_url",
        type=str,
        help="The base URL of the tracking service endpoint (e.g., http://localhost:8000/).",
    )
    parser.add_argument(
        "--duration",
        type=int,
        default=3600,  # 1 hour
        help="Total duration of the simulation in seconds (default: 3600).",
    )
    parser.add_argument(
        "--reconnect-interval",
        type=int,
        default=0,  # 0 means no explicit reconnect, stream for full duration
        help="If > 0, re-establish the SSE stream every X seconds. If 0, stream once for the full duration (default: 0).",
    )
    parser.add_argument(
        "--username",
        type=str,
        default="testuser",
        help="Username for authentication (default: 'testuser').",
    )
    parser.add_argument(
        "--password",
        type=str,
        default="testpassword",
        help="Password for authentication (default: 'testpassword').",
    )
    parser.add_argument(
        "--initial-tags",
        type=str,
        default=",".join(DEFAULT_INITIAL_TAGS),
        help="Comma-separated list of initial public tags known to the client. "
        "These are used for the first subscription and if no tags are learned.",
    )
    parser.add_argument(
        "--num-tags-to-sample",
        type=int,
        default=DEFAULT_NUM_TAGS_TO_SAMPLE,
        help="Number of known public tags to randomly sample for each new stream subscription (default: 2).",
    )
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Enable verbose output",
    )
    args = parser.parse_args()
    if not args.endpoint_url.endswith("/"):
        args.endpoint_url += "/"
    return args


def initialize_client_configuration(args: argparse.Namespace) -> api.DTConfig:
    """
    Initializes the client configuration for API calls.

    Args:
        args (argparse.Namespace): Parsed command-line arguments.

    Returns:
        api.DTConfig: The configured client API object.
    """
    return api.DTConfig(
        base_url=args.endpoint_url,
        username=args.username,
        password=args.password,
    )


def initialize_known_public_tags(initial_tags_str: str, verbose: bool) -> Set[str]:
    """
    Initializes the set of known public tags based on command-line input.
    Ensures there's always at least one default tag if the initial list is empty.

    Args:
        initial_tags_str (str): Comma-separated string of initial tags.
        verbose (bool): If true, print verbose messages.

    Returns:
        Set[str]: A set of unique, stripped public tags.
    """
    known_public_tags: Set[str] = set(
        tag.strip() for tag in initial_tags_str.split(",") if tag.strip()
    )
    if not known_public_tags:
        known_public_tags.update(DEFAULT_INITIAL_TAGS)
        if verbose:
            print(
                f"Warning: No valid initial tags provided, using defaults: {known_public_tags}",
                file=sys.stderr,
            )
    return known_public_tags


def select_subscription_tags(
    known_tags: Set[str],
    num_tags_to_sample: int,
    fallback_tags: list[str],
    verbose: bool,
) -> list[str]:
    """
    Selects a random sample of known public tags for a new SSE stream subscription.
    Uses fallback tags if `known_tags` is empty.

    Args:
        known_tags (Set[str]): The current set of learned public tags.
        num_tags_to_sample (int): The desired number of tags to sample.
        fallback_tags (list[str]): Default tags to use if `known_tags` is empty.
        verbose (bool): If true, print verbose messages.

    Returns:
        list[str]: A list of tags to be used for the next subscription.

    Raises:
        SystemExit: If no tags can be selected (e.g., all tag lists are empty).
    """
    source_tags: Set[str] = known_tags if known_tags else set(fallback_tags)

    if not source_tags:
        exit_with_error(
            "Cannot establish stream: no tags available to subscribe with, and fallback tags are empty.",
            exit_code=1,
        )

    # Ensure we don't try to sample more tags than we currently know
    sample_size = min(num_tags_to_sample, len(source_tags))
    tags_to_use = random.sample(list(source_tags), sample_size)

    # Only print warning if we explicitly fell back to defaults *and* tags_to_use came from fallback
    if not known_tags and verbose and tags_to_use:
        print(
            f"Warning: Known public tags became empty, falling back to default tags for subscription: {tags_to_use}",
            file=sys.stderr,
        )

    return tags_to_use


def update_known_public_tags(
    event: dt_types.StreamEvent,
    known_public_tags: Set[str],
    tags_for_subscription: list[str],
    verbose: bool,
) -> None:
    """
    Processes a StreamEvent to identify and add new public tags to the `known_public_tags` set.
    Logs other change types if verbose.

    Args:
        event (dt_types.StreamEvent): The event received from the SSE stream.
        known_public_tags (Set[str]): The mutable set of all public tags learned so far.
        tags_for_subscription (list[str]): The tags used for the current subscription.
        verbose (bool): If true, print verbose messages.
    """
    for change in event.changes:
        if isinstance(change, dt_types.AddTags):
            new_public_tags_from_event = change.add_fetch.public
            if new_public_tags_from_event:
                old_len = len(known_public_tags)
                known_public_tags.update(new_public_tags_from_event)
                if verbose and len(known_public_tags) > old_len:
                    # Show only tags that were truly new and added and not already in the current subscription
                    added_tags_set = new_public_tags_from_event.difference(
                        set(tags_for_subscription)
                    )
                    if added_tags_set:
                        print(
                            f"  Learned new public tags: {added_tags_set}. Total known: {len(known_public_tags)}"
                        )
        elif verbose:
            # Optional: Log other change types if relevant for simulation monitoring
            if isinstance(change, dt_types.Add):
                # Log first few keys to avoid excessively long lines for many points
                print(
                    f"  Received Add (points for fetch_ids: {list(change.add.points.keys())[:2]}...)"
                )
            elif isinstance(change, dt_types.ExpireFetch):
                print(
                    f"  Received ExpireFetch (fetch_id={change.expire_fetch.fetch_id})"
                )
            elif change == "reset":
                print(f"  Received Reset event (all client data should be cleared).")


def stream_and_process_events(
    dt_config: api.DTConfig,
    tags_for_subscription: list[str],
    stream_read_timeout: float | None,
    known_public_tags: Set[str],
    reconnect_interval: int,
    verbose: bool,
) -> None:
    """
    Establishes an SSE stream, processes events, and updates known tags.
    Handles stream-specific errors (timeout, connection errors) for a single session.

    Args:
        dt_config (api.DTConfig): The client configuration.
        tags_for_subscription (list[str]): Tags to subscribe to for this stream session.
        stream_read_timeout (float): The timeout for this specific stream connection.
        known_public_tags (Set[str]): Mutable set of all public tags learned so far.
        reconnect_interval (int): The configured reconnect interval.
        verbose (bool): If true, print verbose messages.
    """
    global _STOP_SIGNAL_RECEIVED
    event_generator: Optional[Generator[dt_types.StreamEvent, None, None]] = None
    raw_sse_response: Optional[requests.Response] = None

    try:
        if verbose:
            print(
                f"[{time.strftime('%H:%M:%S')}] Connecting to stream with tags: {','.join(tags_for_subscription)} (stream timeout: {stream_read_timeout}s)"
            )

        event_generator, raw_sse_response = api.stream_sse(
            dt_config,
            tags=tags_for_subscription,
            read_timeout=stream_read_timeout,
        )

        print(".")

        for event in event_generator:
            if _STOP_SIGNAL_RECEIVED:
                if verbose:
                    print("Stop signal received, breaking from event processing.")
                break

            if verbose:
                print(
                    f"[{time.strftime('%H:%M:%S')}] Received event (serverTime={event.meta.serverTime}, interval={event.meta.interval})"
                )

            update_known_public_tags(
                event, known_public_tags, tags_for_subscription, verbose
            )

        print("loop done")

    except requests.exceptions.Timeout:
        if verbose:
            print(
                f"[{time.strftime('%H:%M:%S')}] Stream connection timed out after {stream_read_timeout:.1f}s (expected)."
            )
    except requests.exceptions.ConnectionError as e:
        print(
            f"[{time.strftime('%H:%M:%S')}] Connection error: {e}. "
            f"Will retry in {reconnect_interval if reconnect_interval > 0 else 5}s...",
            file=sys.stderr,
        )
        if reconnect_interval > 0:
            # Sleep briefly before the next attempt to prevent a tight loop on errors.
            time.sleep(
                min(5, reconnect_interval)
            )  # Don't sleep longer than reconnect_interval
        else:  # If not configured to reconnect, a persistent error means we should exit.
            exit_with_error(
                f"Persistent connection error without reconnect-interval: {e}",
                exit_code=4,
            )
    except requests.exceptions.RequestException as e:
        # Catch other HTTP/network-related errors
        exit_with_error(f"Network or HTTP error during stream: {e}", exit_code=4)
    # except Exception as e:
    #     # Catch any other unexpected errors
    #     exit_with_error(
    #         f"An unexpected error occurred during stream processing: {e}", exit_code=5
    #     )
    finally:
        # Ensure the raw SSE response object is closed to release network resources
        if raw_sse_response:
            raw_sse_response.close()
            if verbose:
                print(f"[{time.strftime('%H:%M:%S')}] Closed SSE response object.")


def run_simulation_loop(
    args: argparse.Namespace, dt_config: api.DTConfig, known_public_tags: Set[str]
) -> None:
    """
    Manages the main simulation loop, including session establishment,
    stream processing, and reconnection logic for the entire simulation duration.

    Args:
        args (argparse.Namespace): Parsed command-line arguments.
        dt_config (api.DTConfig): The client configuration.
        known_public_tags (Set[str]): The mutable set of all public tags learned so far.
    """
    global _STOP_SIGNAL_RECEIVED

    total_duration = args.duration
    reconnect_interval = args.reconnect_interval
    num_tags_to_sample = args.num_tags_to_sample
    verbose = args.verbose

    simulation_start_time = time.monotonic()
    simulation_end_time = simulation_start_time + total_duration

    print(f"Client simulation started. Connecting to {args.endpoint_url}")
    if reconnect_interval > 0:
        print(
            f"  Total duration: {total_duration}s, Reconnecting every: {reconnect_interval}s"
        )
    else:
        print(f"  Total duration: {total_duration}s, Single stream session.")
    print(f"  Initial known tags: {known_public_tags}")

    while not _STOP_SIGNAL_RECEIVED and time.monotonic() < simulation_end_time:
        current_time_mono = time.monotonic()
        remaining_simulation_time = simulation_end_time - current_time_mono

        if remaining_simulation_time <= 0:
            if verbose:
                print("No effective time left for a new stream session, exiting loop.")
            break

        # Determine the read_timeout for the current SSE stream connection.
        # This controls how long requests.get is allowed to wait for data.
        # If reconnect_interval is 0, we try to stream for the full remaining duration.
        # Otherwise, we stream for reconnect_interval or the remaining duration, whichever is smaller.
        # if reconnect_interval > 0:
        #     stream_read_timeout = min(reconnect_interval, remaining_simulation_time)
        # else:
        #     stream_read_timeout = remaining_simulation_time
        stream_read_timeout = None

        tags_for_subscription = select_subscription_tags(
            known_public_tags, num_tags_to_sample, DEFAULT_INITIAL_TAGS, verbose
        )

        stream_and_process_events(
            dt_config,
            tags_for_subscription,
            stream_read_timeout,
            known_public_tags,
            reconnect_interval,
            verbose,
        )

        # Break from the main loop if:
        # - The client is not configured to reconnect (reconnect_interval is 0).
        # - A stop signal has been received.
        # - The total simulation duration has passed.
        if (
            reconnect_interval == 0
            or _STOP_SIGNAL_RECEIVED
            or time.monotonic() >= simulation_end_time
        ):
            break

    if _STOP_SIGNAL_RECEIVED:
        print("Client simulation interrupted by user (Ctrl+C).", file=sys.stderr)
    elif time.monotonic() >= simulation_end_time:
        print(f"Client simulation completed after {total_duration} seconds.")
    else:  # This block should theoretically not be reached under normal operation if exit conditions are met.
        print("Client simulation loop exited unexpectedly.", file=sys.stderr)

    sys.exit(0)


# --- Main function (orchestrator) ---
def main() -> None:
    """
    Main entry point for the client simulator script.
    Parses arguments, sets up configuration, and runs the simulation loop.
    """
    # Register the SIGINT handler for graceful shutdown as early as possible.
    signal.signal(signal.SIGINT, signal_handler)

    args = parse_arguments()
    dt_config = initialize_client_configuration(args)
    known_public_tags = initialize_known_public_tags(args.initial_tags, args.verbose)

    run_simulation_loop(args, dt_config, known_public_tags)


if __name__ == "__main__":
    main()
