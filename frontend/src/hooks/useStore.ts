import { create } from "zustand";
import { useProtocolStore } from "../lib/protocol";
import { Fetches } from "../lib/types";
import { difference, union } from "../lib/set";

interface AppState {
  selectedTags: Set<string>;
  fetches: Fetches;
  tags: Set<string>;
  customTags: Set<string>;
  addTags: (tags: Set<string>) => void;
  toggleTag: (tag: string) => void;
  addCustomTag: (tag: string) => void;
  removeCustomTag: (tag: string) => void;
}

const CUSTOM_TAGS_KEY = "customTags";
const SELECTED_TAGS_KEY = "selectedTags";

// Helper function for localStorage
const getStoredTags = (): Set<string> => {
  try {
    const stored = localStorage.getItem(CUSTOM_TAGS_KEY);
    return stored ? new Set(JSON.parse(stored)) : new Set<string>();
  } catch (e) {
    console.error("Failed to load custom tags from localStorage", e);
    return new Set();
  }
};

// Helper function for localStorage
const getSelectedTags = (): Set<string> => {
  try {
    const stored = localStorage.getItem(SELECTED_TAGS_KEY);
    return stored ? new Set(JSON.parse(stored)) : new Set<string>();
  } catch (e) {
    console.error("Failed to load selected tags from localStorage", e);
    return new Set();
  }
};

const saveTagsToStorage = (tags: Set<string>) => {
  try {
    localStorage.setItem(CUSTOM_TAGS_KEY, JSON.stringify([...tags]));
  } catch (e) {
    console.error("Failed to save custom tags to localStorage", e);
  }
};

const saveSelectedTagsToStorage = (selectedTags: Set<string>) => {
  try {
    localStorage.setItem(SELECTED_TAGS_KEY, JSON.stringify([...selectedTags]));
  } catch (e) {
    console.error("Failed to save selected tags to localStorage", e);
  }
};

export const useAppStore = create<AppState>((set) => ({
  selectedTags: getSelectedTags(),
  fetches: {},
  tags: new Set<string>(),
  customTags: getStoredTags(),

  toggleTag: (tag: string) =>
    set((state) => {
      const newTags = new Set(state.selectedTags);
      if (newTags.has(tag)) {
        newTags.delete(tag);
      } else {
        newTags.add(tag);
      }
      saveSelectedTagsToStorage(newTags);
      return { selectedTags: newTags };
    }),

  addTags: (tags: Set<string>) =>
    set((state) => {
      const newTags = union(state.tags, tags);
      if (difference(newTags, state.tags).size == 0) {
        return {};
      } else {
        return { tags: newTags };
      }
    }),

  addCustomTag: (tag: string) =>
    set((state) => {
      const trimmedTag = tag.trim().toLowerCase();
      if (!trimmedTag || state.customTags.has(trimmedTag)) {
        return {}; // Do nothing if tag is empty or already exists
      }
      const newCustomTags = new Set([...state.customTags, trimmedTag]);
      const newSelectedTags = new Set([...state.selectedTags, trimmedTag]);
      saveTagsToStorage(newCustomTags);
      return { customTags: newCustomTags, selectedTags: newSelectedTags };
    }),

  removeCustomTag: (tag: string) =>
    set((state) => {
      const newCustomTags = difference(state.customTags, new Set([tag]));
      saveTagsToStorage(newCustomTags);
      return { customTags: newCustomTags };
    }),
}));

// Subscribe to the protocol store and merge with custom tags
useProtocolStore.subscribe((protocolState) => {
  const combinedTags = union(
    protocolState.subscribedTags,
    useAppStore.getState().customTags,
  );
  useAppStore.setState({
    fetches: protocolState.fetches,
    tags: combinedTags,
  });
});

// Subscribe to the app store's customTags to re-sync combined tags when they change.
// The selector function was removed to resolve the TS2554 error.
useAppStore.subscribe((state, prevState) => {
  // Only update if the custom tags have actually changed
  if (state.customTags !== prevState.customTags) {
    const protocolTags = useProtocolStore.getState().subscribedTags;
    const combinedTags = union(protocolTags, state.customTags);
    useAppStore.setState({
      tags: combinedTags,
    });
  }
});
