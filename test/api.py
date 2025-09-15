from dataclasses import dataclass
import requests
import sseclient
import time
from typing import Generator

from . import dt_types


@dataclass(frozen=True)
class DTConfig:
    base_url: str
    username: str
    password: str


def stream_sse(
    config: DTConfig,
    tags: list[str],
    read_timeout: float | None = None,
) -> tuple[Generator[dt_types.StreamEvent, None, None], requests.Response]:
    """
    Connects to the SSE stream and returns a generator for events and the raw requests.Response object.
    The caller is responsible for closing the requests.Response object using its .close() method.
    """
    response = requests.post(
        f"{config.base_url}login",
        json={"username": config.username, "password": config.password},
    )
    token = dt_types.LoginResponse.model_validate(response.json()).token

    headers = {"Accept": "text/event-stream"}
    tags_str = ",".join(tags)

    # Set a connect timeout (e.g., 5 seconds) and use the provided read_timeout.
    # If read_timeout is None, requests defaults to no read timeout.
    req_timeout = (5, read_timeout) if read_timeout is not None else 5

    # The 'stream=True' is crucial for sseclient to read incrementally
    raw_sse_response = requests.get(
        f"{config.base_url}stream?token={token}&tags={tags_str}",
        stream=True,
        headers=headers,
        timeout=req_timeout,
    )

    deadline = time.monotonic() + read_timeout if read_timeout else None

    def event_generator() -> Generator[dt_types.StreamEvent, None, None]:
        client = sseclient.SSEClient(raw_sse_response)
        for event in client.events():
            if deadline and time.monotonic() >= deadline:
                break
            parsed = dt_types.StreamEvent.model_validate_json(event.data)
            if parsed.changes:
                yield parsed
            else:
                break

    return event_generator(), raw_sse_response
