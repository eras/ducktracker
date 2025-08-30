import { create } from 'zustand';
import { useProtocolStore } from '../lib/protocol';
import { LocationData } from '../lib/types';

interface AppState {
  selectedTags: Set<string>;
  locations: LocationData;
  tags: string[];
  toggleTag: (tag: string) => void;
}

/**
 * The main application state store using Zustand.
 * It combines data from the protocol store and manages
 * UI-specific state like selected tags.
 */
export const useAppStore = create<AppState>((set) => ({
  selectedTags: new Set<string>(),
  locations: {},
  tags: [],

  toggleTag: (tag: string) =>
    set((state) => {
      const newTags = new Set(state.selectedTags);
      if (newTags.has(tag)) {
        newTags.delete(tag);
      } else {
        newTags.add(tag);
      }
      return { selectedTags: newTags };
    }),
}));

// Subscribe to the protocol store to keep the app state in sync
useProtocolStore.subscribe(
  (protocolState) => {
    useAppStore.setState({
      locations: protocolState.locations,
      tags: protocolState.tags,
    });
  },
  (state) => state
);