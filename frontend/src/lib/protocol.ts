import { create } from "zustand";
import { LocationData, ServerMessage } from "./types";

export interface ProtocolState {
  locations: LocationData;
  tags: string[];
  fetchData: () => void;
}

/**
 * This is a stub implementation of the protocol. It simulates
 * an SSE connection by providing initial data and then
 * a partial update after a delay. In a real application,
 * this would connect to the actual SSE endpoint.
 */
export const useProtocolStore = create<ProtocolState>((set) => ({
  locations: {},
  tags: [],

  fetchData: () => {
    // Initial data load simulation
    const initialData: ServerMessage = {
      tags: ["food", "home", "work"],
      locations: {
        "1": {
          t: ["food", "alone"],
          p: [
            [17.65431710431244, 32.954120326746775],
            [17.65431710431244, 32.955120326746775],
          ],
        },
        "42": {
          t: ["work", "group"],
          p: [
            [-74.005, 40.713],
            [-74.0051, 40.7131],
          ],
        },
      },
    };

    set({ locations: initialData.locations, tags: initialData.tags });

    // Simulate a partial update over SSE after a delay
    setTimeout(() => {
      const partialUpdate: LocationData = {
        "1": {
          t: ["food", "alone"],
          p: [
            ...initialData.locations["1"].p,
            [-74.0062, 40.7132],
            [-74.0063, 40.7133],
          ],
        },
        "99": {
          t: ["home"],
          p: [
            [-74.004, 40.711],
            [-74.0041, 40.7111],
          ],
        },
      };

      set((state) => ({
        locations: { ...state.locations, ...partialUpdate },
      }));
    }, 2000); // Simulate update after 2 seconds
  },
}));
