from pydantic import BaseModel, RootModel
from typing import Literal, TypeAlias


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


class ExpireFetchContent(BaseModel):
    fetch_id: int  # Fetch_id expired


class ExpireFetch(BaseModel):
    expire_fetch: ExpireFetchContent


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
