import { Tag } from "../bindings/Tag";
import { FetchId } from "../bindings/FetchId";
import { Location } from "../bindings/Location";

export interface ParsedLocation {
  latlon: [number, number];
  accuracy?: number;
  speed?: number;
  provider: number;
  time: number; // seconds from epoch
}

export let parseLocation = (location: Location): ParsedLocation => {
  return {
    latlon: [location[0], location[1]],
    time: location[2], // seconds from epoch
    speed: location[3],
    accuracy: location[4],
    provider: location[5],
  };
};

export interface Fetch {
  locations: Array<ParsedLocation>;
  tags: Set<Tag>;
  name: string | null;
  max_points: number;
  max_point_age: number | null;
}

export type Fetches = { [key in FetchId]: Fetch };
