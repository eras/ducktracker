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
            [59.436962, 24.753574],
            [59.4369621, 24.7535741],
          ],
        },
        "42": {
          t: ["work", "group"],
          p: [
            [59.446962, 24.743574],
            [59.4469621, 24.7435741],
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
            [59.446962, 24.743574],
            [59.4469621, 24.7435741],
          ],
        },
        "99": {
          t: ["home"],
          p: [
            [59.445962, 24.745574],
            [59.4459621, 24.7455741],
          ],
        },
      };

      set((state) => ({
        locations: { ...state.locations, ...partialUpdate },
      }));
    }, 2000); // Simulate update after 2 seconds
  },
}));
