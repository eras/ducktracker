import { create } from "zustand";
import { Tag } from "../bindings/Tag";
import { Update } from "../bindings/Update";
import { UpdateChange } from "../bindings/UpdateChange";
import { Fetches, parseLocation } from "./types";
import { union } from "../lib/set";

export interface ProtocolState {
  fetches: Fetches;
  tags: Set<Tag>;
  publicTags: Set<Tag>;
  connect: (
    tags: string[],
    addTags: (tags: Set<string>) => void,
  ) => EventSource;
  disconnect: (eventSource: EventSource) => void;
}

const API_URL = "/api";

let processUpdates = (
  updates: Array<UpdateChange>,
  stateIn: ProtocolState,
  addTags: (tags: Set<string>) => void,
): {
  fetches: Fetches;
  tags: Set<Tag>;
  publicTags: Set<Tag>;
} => {
  let state = {
    fetches: { ...stateIn.fetches },
    tags: new Set([...stateIn.tags]),
    publicTags: new Set([...stateIn.publicTags]),
  };
  for (const change of updates) {
    if (change == "reset") {
      state = {
        fetches: {},
        tags: new Set(),
        publicTags: new Set(),
      };
    } else {
      if ("add_tags" in change) {
        let new_tags = new Set([...change.add_tags.public]);
        Object.entries(change.add_tags.tags).forEach(([fetch_id, tags]) => {
          if (tags) {
            let fetch_index = parseInt(fetch_id);
            let fetch =
              fetch_index in state.fetches
                ? state.fetches[fetch_index]
                : { locations: [], tags: new Set<string>() };
            fetch.tags = union(fetch.tags, tags);
            for (const tag of tags) {
              new_tags.add(tag);
            }
            state.fetches[fetch_index] = fetch;
          }
        });
        state.publicTags = new Set([
          ...state.publicTags,
          ...change.add_tags.public,
        ]);
        addTags(new_tags);
      } else if ("add" in change) {
        Object.entries(change.add.points).forEach(([fetch_id, points]) => {
          if (points) {
            let fetch = state.fetches[parseInt(fetch_id)];
            const parsedPoints = points.map(parseLocation);
            fetch.locations = [...fetch.locations, ...parsedPoints];
          }
        });
      } else if ("expire_fetch" in change) {
        let fetch_index = change.expire_fetch.fetch_id;
        delete state.fetches[fetch_index];
      } else {
        console.error("Unknown update:", change);
        break;
      }
    }
  }
  return state;
};

/**
 * Manages the Server-Sent Events (SSE) connection to the API.
 * This is a real implementation that connects to the /api/stream endpoint.
 */
export const useProtocolStore = create<ProtocolState>((set) => ({
  fetches: {},
  tags: new Set<Tag>(),
  publicTags: new Set<Tag>(),

  connect: (tags: string[], addTags: (tags: Set<string>) => void) => {
    const tagsQuery = tags.length > 0 ? `tags=${tags.join(",")}` : "";
    const url = `${API_URL}/stream?${tagsQuery}`;

    const eventSource = new EventSource(url);

    eventSource.onmessage = (event) => {
      try {
        const data: Update = JSON.parse(event.data);
        console.log(`Processing ${JSON.stringify(data)}`);
        set((state) => processUpdates(data.changes, state, addTags));
      } catch (e) {
        console.error("Failed to parse SSE message:", e);
      }
    };

    eventSource.onerror = (error) => {
      console.error("EventSource failed:", error);
      eventSource.close();
    };

    return eventSource;
  },

  disconnect: (eventSource: EventSource) => {
    eventSource.close();
  },
}));
