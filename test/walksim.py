#!/usr/bin/env python3

import argparse
import requests
import sys
import time
import random
import math
import signal  # Import signal module
from typing import Any, NoReturn, Optional  # Import Optional

# --- Global Flags for Signal Handling ---
_STOP_SIGNAL_RECEIVED: bool = False

# --- Configuration Constants ---
DEFAULT_PUBLIC_TAGS: list[str] = [
    "work",
    "home",
    "travel",
    "park",
    "city",
    "street",
]
DEFAULT_PRIVATE_TAGS: list[str] = [
    "secret",
    "personal",
    "friends",
    "family",
    "confidential",
]

# Initial location near Tallinn, Estonia
INITIAL_LAT: float = 59.436962
INITIAL_LON: float = 24.753574

# Earth's mean radius in meters for rough conversion (WGS84 ellipsoid mean radius)
EARTH_RADIUS_METERS: float = 6371000.0

# --- Helper Functions ---


def exit_with_error(message: str, exit_code: int = 1) -> NoReturn:
    """Prints an error message to stderr and exits."""
    print(f"Error: {message}", file=sys.stderr)
    sys.exit(exit_code)


def meters_to_degrees(meters: float) -> float:
    """
    Converts meters to approximate degrees of latitude.
    This is a rough estimation; 1 degree of latitude is approximately 111,139 meters.
    For small movements, this serves as a reasonable delta for both lat and lon.
    """
    return meters / 111139.0


def parse_api_response_lines(response_text: str) -> list[str]:
    """Parses a multi-line API response, expecting 'OK' as the first line."""
    lines = response_text.strip().split("\n")
    if not lines or lines[0] != "OK":
        exit_with_error(
            f"Unexpected API response: '{response_text.strip()}'", exit_code=2
        )
    return lines


def generate_random_tags(
    public_tags: list[str],
    private_tags: list[str],
    num_public: int = 2,
    num_private: int = 1,
) -> str:
    """Selects a random number of tags from provided lists and formats them."""
    selected_public = random.sample(
        [f"public:{x}" for x in public_tags], min(num_public, len(public_tags))
    )
    selected_private = random.sample(
        [f"private:{x}" for x in public_tags], min(num_private, len(private_tags))
    )
    all_selected_tags = selected_public + selected_private
    random.shuffle(all_selected_tags)  # Mix them up
    return ",".join(all_selected_tags)


def simulate_movement(
    current_lat: float, current_lon: float, distance_degrees: float
) -> tuple[float, float]:
    """
    Simulates movement by picking a random direction and moving a fixed distance.
    This uses a simplified Cartesian approximation for small distances on the Earth's surface.
    """
    angle_radians = random.uniform(0, 2 * math.pi)  # Random angle in radians

    delta_lat = distance_degrees * math.cos(angle_radians)

    # Longitudinal change needs to account for longitude lines converging at poles (cos(latitude)).
    # Convert current_lat to radians for `math.cos` calculation.
    lat_radians = math.radians(current_lat)

    # Avoid division by zero or extreme values near poles.
    # For a more robust solution near poles, more advanced Vincenty/Haversine formulas are needed.
    if abs(lat_radians) > math.pi / 2 - 1e-6:  # Very close to a pole
        delta_lon = 0.0
    else:
        delta_lon = distance_degrees * math.sin(angle_radians) / math.cos(lat_radians)

    new_lat = current_lat + delta_lat
    new_lon = current_lon + delta_lon

    # Clamp latitude to valid range [-90, 90]
    new_lat = max(-90.0, min(90.0, new_lat))
    # Normalize longitude to valid range [-180, 180]
    new_lon = (new_lon + 180) % 360 - 180

    return new_lat, new_lon


# --- API Interaction Functions ---


def create_session(
    base_url: str,
    username: str,
    password: str,
    tags_str: str,
    duration: int,
    interval: int,
) -> tuple[str, str, str]:
    """Creates a new session and returns session_id, share_link, share_id."""
    create_data: dict[str, Any] = {
        "usr": username,
        "pwd": password,
        "mod": 0,
        "lid": tags_str,
        "dur": duration,
        "int": interval,
    }

    try:
        response = requests.post(f"{base_url}create.php", data=create_data)
        response.raise_for_status()  # Raise an exception for HTTP errors (4xx or 5xx)
        lines = parse_api_response_lines(response.text)
        if len(lines) < 4:
            exit_with_error(
                f"Incomplete create.php response: '{response.text.strip()}'",
                exit_code=3,
            )
        session_id = lines[1]
        share_link = lines[2]
        share_id = lines[3]
        return session_id, share_link, share_id
    except requests.exceptions.RequestException as e:
        exit_with_error(
            f"Failed to create session due to network or HTTP error: {e}", exit_code=4
        )
    except Exception as e:
        exit_with_error(
            f"An unexpected error occurred during session creation: {e}", exit_code=5
        )


def post_location_update(
    base_url: str, session_id: str, lat: float, lon: float, current_time: int
) -> None:
    """Sends a location update to the session."""
    location_data: dict[str, Any] = {
        "sid": session_id,
        "lat": lat,
        "lon": lon,
        "acc": random.uniform(5.0, 20.0),  # Random accuracy in meters
        "alt": random.uniform(0.0, 1000.0),  # Random altitude in meters
        "speed": random.uniform(0.0, 30.0),  # Random speed in m/s
        "dir": random.uniform(0.0, 360.0),  # Random direction in degrees
        "batt": random.randint(30, 100),  # Random battery percentage
        "prv": 0,  # Provider (e.g., 0 for GPS)
        "time": current_time,  # Unix timestamp
    }

    try:
        response = requests.post(f"{base_url}post.php", data=location_data)
        response.raise_for_status()
        parse_api_response_lines(response.text)  # Just check for "OK"
    except requests.exceptions.RequestException as e:
        exit_with_error(
            f"Failed to post location update due to network or HTTP error: {e}",
            exit_code=6,
        )
    except Exception as e:
        exit_with_error(
            f"An unexpected error occurred during location post: {e}", exit_code=7
        )


def stop_session(base_url: str, session_id: str) -> None:
    """Stops the session via the stop.php endpoint."""
    stop_data: dict[str, str] = {"sid": session_id}
    try:
        response = requests.post(f"{base_url}stop.php", data=stop_data)
        response.raise_for_status()
        # We only expect "OK\n" or similar success, parse_api_response_lines will validate
        parse_api_response_lines(response.text)
    except requests.exceptions.RequestException as e:
        # Don't exit_with_error here, as we are already in a shutdown path.
        # Just log a warning to stderr.
        print(
            f"Warning: Failed to stop session {session_id} due to network or HTTP error: {e}",
            file=sys.stderr,
        )
    except Exception as e:
        print(
            f"Warning: An unexpected error occurred while stopping session {session_id}: {e}",
            file=sys.stderr,
        )


# --- Signal Handler ---
def signal_handler(signum: int, frame: Any) -> None:
    """Sets a global flag when Ctrl+C (SIGINT) is received."""
    global _STOP_SIGNAL_RECEIVED
    _STOP_SIGNAL_RECEIVED = True
    # In a real application, you might log here or take minimal action.
    # Avoid complex I/O in signal handlers. The main loop will handle cleanup.


# --- Main Logic ---


def main() -> None:
    global _STOP_SIGNAL_RECEIVED  # Declare global flag usage

    parser = argparse.ArgumentParser(
        description="Simulates a location client sending updates to a tracking service."
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
        help="Duration of the simulation in seconds (default: 3600).",
    )
    parser.add_argument(
        "--interval",
        type=int,
        default=30,  # 30 seconds
        help="Interval between location updates in seconds (default: 30).",
    )
    parser.add_argument(
        "--move-distance",
        type=float,
        default=10.0,  # 10 meters
        help="Approximate distance to move per interval in meters (default: 10.0).",
    )
    parser.add_argument(
        "--preload",
        type=float,
        default=0,
        help="If set, first share this many points without sleeping",
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
        "--public-tags",
        type=str,
        default=",".join(DEFAULT_PUBLIC_TAGS),
        help="Public tags to select from",
    )
    parser.add_argument(
        "--private-tags",
        type=str,
        default=",".join(DEFAULT_PUBLIC_TAGS),
        help="Private tags to select from",
    )
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Enable verbose output",
    )

    args = parser.parse_args()
    base_url = args.endpoint_url
    if not base_url.endswith("/"):
        base_url += "/"

    share_duration: int = args.duration
    share_interval: int = args.interval
    move_distance_meters: float = args.move_distance
    move_distance_degrees: float = meters_to_degrees(move_distance_meters)
    username: str = args.username
    password: str = args.password
    preload: int | None = args.preload if args.preload else None
    public_tags: list[str] = args.public_tags.split(",")
    private_tags: list[str] = args.private_tags.split(",")
    verbose: bool = args.verbose

    # Register the SIGINT handler
    signal.signal(signal.SIGINT, signal_handler)

    session_id: Optional[str] = None  # Initialize session_id to None

    try:
        # 1. Generate random tags for the session
        selected_tags_str = generate_random_tags(public_tags, private_tags)

        # 2. Create the tracking session
        temp_session_id, _share_link, _share_id = create_session(
            base_url,
            username,
            password,
            selected_tags_str,
            share_duration,
            share_interval,
        )
        session_id = temp_session_id  # Store the created session_id

        # 3. Simulate movement and send location updates
        current_lat, current_lon = INITIAL_LAT, INITIAL_LON
        start_time = time.monotonic()  # Use monotonic for reliable duration measurement
        end_time = start_time + share_duration

        num_points = 0

        while not _STOP_SIGNAL_RECEIVED and time.monotonic() < end_time:
            loop_start_time = time.monotonic()
            current_time_unix = int(time.time())

            # Simulate movement to a new location
            current_lat, current_lon = simulate_movement(
                current_lat, current_lon, move_distance_degrees
            )

            # Post the new location update to the server
            if verbose:
                print(f"{current_time_unix}")
            post_location_update(
                base_url, session_id, current_lat, current_lon, current_time_unix
            )

            num_points += 1

            if preload is None or num_points > preload:
                # Calculate time to sleep to maintain the interval, accounting for execution time
                elapsed_since_loop_start = time.monotonic() - loop_start_time
                sleep_duration = share_interval - elapsed_since_loop_start
                if sleep_duration > 0:
                    time.sleep(sleep_duration)

    finally:
        # This block always executes, even if Ctrl+C is pressed or an error occurs.
        if (
            session_id
        ):  # Only attempt to stop the session if it was successfully created
            stop_session(base_url, session_id)

    # Exit normally (code 0) whether the simulation completed naturally or was interrupted by Ctrl+C
    sys.exit(0)


if __name__ == "__main__":
    main()
