/**
 * Defines the structure for a single location point.
 * [longitude, latitude, ...other_data]
 */
export type LocationPoint = [number, number, ...any[]];

/**
 * Defines the data structure for a single tracked object, including its tags and location points.
 */
export interface TrackedObject {
  t: string[];
  p: LocationPoint[];
}

/**
 * The full data structure received from the server, mapping object IDs to their location data.
 */
export interface LocationData {
  [id: string]: TrackedObject;
}

/**
 * The expected structure of a server-sent event message.
 */
export interface ServerMessage {
  tags: string[];
  locations: LocationData;
}
