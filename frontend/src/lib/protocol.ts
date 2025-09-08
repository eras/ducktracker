import { create } from "zustand";
import { Tag } from "../bindings/Tag";
import { Update } from "../bindings/Update";
import { UpdateChange } from "../bindings/UpdateChange";
import { Fetches, parseLocation } from "./types";
import { union } from "../lib/set";
import { useAuthStore } from "../hooks/useAuthStore";
import { LoginRequest } from "../bindings/LoginRequest";
import { LoginResponse } from "../bindings/LoginResponse";

export interface ProtocolState {
  fetches: Fetches;
  subscribedTags: Set<Tag>;
  publicTags: Set<Tag>;
  connect: (
    tags: string[],
    addTags: (tags: Set<string>) => void,
  ) => Promise<void>;
  disconnect: () => void;
}

const API_URL = "/api";

let processUpdates = (
  updates: Array<UpdateChange>,
  stateIn: ProtocolState,
  addedTags: Set<string>,
): {
  fetches: Fetches;
  subscribedTags: Set<Tag>;
  publicTags: Set<Tag>;
} => {
  let state = {
    fetches: { ...stateIn.fetches },
    subscribedTags: new Set([...stateIn.subscribedTags]),
    publicTags: new Set([...stateIn.publicTags]),
  };
  for (const change of updates) {
    if (change == "reset") {
      state = {
        fetches: {},
        subscribedTags: new Set(),
        publicTags: new Set(),
      };
    } else {
      if ("add_fetch" in change) {
        let newTags = new Set([...change.add_fetch.public]);
        Object.entries(change.add_fetch.tags).forEach(([fetch_id, tags]) => {
          if (tags) {
            let fetch_index = parseInt(fetch_id);
            let fetch =
              fetch_index in state.fetches
                ? state.fetches[fetch_index]
                : {
                    // construct new Fetch
                    locations: [],
                    tags: new Set<string>(),
                    max_points: change.add_fetch.max_points,
                  };
            fetch.tags = union(fetch.tags, tags);
            for (const tag of tags) {
              newTags.add(tag);
            }
            state.fetches[fetch_index] = fetch;
          }
        });
        state.publicTags = new Set([
          ...state.publicTags,
          ...change.add_fetch.public,
        ]);
        state.subscribedTags = union(state.subscribedTags, newTags);
        for (const tag of newTags) {
          addedTags.add(tag);
        }
      } else if ("add" in change) {
        Object.entries(change.add.points).forEach(([fetch_id, points]) => {
          if (points) {
            let fetch = state.fetches[parseInt(fetch_id)];
            const parsedPoints = points.map(parseLocation);
            fetch.locations = [...fetch.locations, ...parsedPoints];
            if (fetch.locations.length > fetch.max_points) {
              fetch.locations.splice(
                0,
                fetch.locations.length - fetch.max_points,
              );
            }
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
export const useProtocolStore = create<ProtocolState>((set) => {
  let eventSource: EventSource | null = null;
  let retryCount = 0;
  let reconnectTimeoutId: number | null = null;
  const MAX_RECONNECT_INTERVAL = 5000;

  const connect = async (
    // Changed to async
    subscribedTags: string[],
    addTags: (tags: Set<string>) => void,
  ): Promise<void> => {
    // Changed return type
    // Clear any pending reconnect
    if (reconnectTimeoutId) {
      clearTimeout(reconnectTimeoutId);
      reconnectTimeoutId = null;
    }

    // Close existing connection if any
    if (eventSource) {
      eventSource.close();
      eventSource = null;
    }

    const { username, password, showLogin, clearCredentials } =
      useAuthStore.getState();

    // Pre-flight check with fetch for authentication
    if (!username || !password) {
      console.warn("Attempted to connect without credentials. Showing login.");
      showLogin();
      return; // Return early if no credentials
    }

    let token: string | null = null;

    try {
      const loginRequest: LoginRequest = {
        username: username ?? "",
        password: password ?? "",
      };
      const response = await fetch(`${API_URL}/login`, {
        method: "POST",
        body: JSON.stringify(loginRequest),
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
        },
      });

      if (response.status === 401) {
        console.error("Authentication failed during pre-flight check.");
        clearCredentials(); // Old credentials are bad
        showLogin();
        return; // Return early, connection cannot be established
      }
      if (!response.ok) {
        scheduleReconnect(subscribedTags, addTags);
        return;
      }
      const result: LoginResponse = await response.json();
      token = result.token;
    } catch (e) {
      console.error("Pre-flight connection check failed:", e);
      // Decide if you want to show login on network errors too (e.g., server down, network issues)
      showLogin();
      return; // Return early, connection cannot be established
    }

    // 2. Connect to EventSource
    const tagsQuery =
      subscribedTags.length > 0 ? `tags=${subscribedTags.join(",")}` : "";
    // Note: EventSource does not support custom headers. Credentials must be in the URL for Basic Auth.
    // Ensure your backend is configured to read `user` and `pass` query parameters.
    const credentials = `token=${encodeURIComponent(token)}`;
    const url = `${API_URL}/stream?${credentials}${tagsQuery ? `&${tagsQuery}` : ""}`; // Combine queries carefully

    eventSource = new EventSource(url);

    eventSource.onopen = () => {
      retryCount = 0;
    };

    eventSource.onmessage = (event: MessageEvent) => {
      try {
        const data: Update = JSON.parse(event.data);
        let addedTags = new Set<string>();
        set((state) => processUpdates(data.changes, state, addedTags));
        addTags(addedTags);
      } catch (e) {
        console.error("Failed to parse SSE message:", e);
      }
    };

    eventSource.onerror = (error: Event) => {
      console.error("EventSource failed:", error);
      eventSource?.close(); // Close the current faulty connection
      eventSource = null; // Clear the reference
      scheduleReconnect(subscribedTags, addTags);
    };

    // No explicit return value needed, as EventSource is managed internally
  };

  const scheduleReconnect = (
    tags: string[],
    addTags: (tags: Set<string>) => void,
  ): void => {
    // Only schedule if not already connecting and not disconnected explicitly
    if (eventSource === null) {
      // If eventSource is null, it means it was explicitly closed or failed
      const delay = Math.min(
        MAX_RECONNECT_INTERVAL,
        100 * Math.pow(1.5, retryCount),
      );
      console.log(`Attempting to reconnect in ${delay / 1000} seconds...`);
      reconnectTimeoutId = window.setTimeout(() => {
        if (delay < MAX_RECONNECT_INTERVAL) {
          retryCount++;
        }
        connect(tags, addTags); // Try to connect again
      }, delay);
    }
  };

  const disconnect = (): void => {
    if (eventSource) {
      eventSource.close();
      eventSource = null;
    }
    if (reconnectTimeoutId) {
      clearTimeout(reconnectTimeoutId);
      reconnectTimeoutId = null;
    }
    retryCount = 0; // Reset retry count on explicit disconnect
  };

  return {
    fetches: {},
    subscribedTags: new Set<string>(),
    publicTags: new Set<string>(),
    connect,
    disconnect,
  };
});
