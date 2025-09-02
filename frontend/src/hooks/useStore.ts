import { create } from "zustand";
import { useProtocolStore } from "../lib/protocol";
import { Fetches } from "../lib/types";

interface AppState {
  selectedTags: Set<string>;
  fetches: Fetches;
  tags: string[];
  customTags: string[];
  toggleTag: (tag: string) => void;
  addCustomTag: (tag: string) => void;
  removeCustomTag: (tag: string) => void;
}

const CUSTOM_TAGS_KEY = "customTags";

// Helper function for localStorage
const getStoredTags = (): string[] => {
  try {
    const stored = localStorage.getItem(CUSTOM_TAGS_KEY);
    return stored ? JSON.parse(stored) : [];
  } catch (e) {
    console.error("Failed to load custom tags from localStorage", e);
    return [];
  }
};

const saveTagsToStorage = (tags: string[]) => {
  try {
    localStorage.setItem(CUSTOM_TAGS_KEY, JSON.stringify(tags));
  } catch (e) {
    console.error("Failed to save custom tags to localStorage", e);
  }
};

export const useAppStore = create<AppState>((set) => ({
  selectedTags: new Set<string>(),
  fetches: {},
  tags: [],
  customTags: getStoredTags(),

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

  addCustomTag: (tag: string) =>
    set((state) => {
      const trimmedTag = tag.trim().toLowerCase();
      if (!trimmedTag || state.customTags.includes(trimmedTag)) {
        return {}; // Do nothing if tag is empty or already exists
      }
      const newCustomTags = [...state.customTags, trimmedTag];
      saveTagsToStorage(newCustomTags);
      return { customTags: newCustomTags };
    }),

  removeCustomTag: (tag: string) =>
    set((state) => {
      const newCustomTags = state.customTags.filter((t) => t !== tag);
      saveTagsToStorage(newCustomTags);
      return { customTags: newCustomTags };
    }),
}));

// Subscribe to the protocol store and merge with custom tags
useProtocolStore.subscribe((protocolState) => {
  const combinedTags = new Set([
    ...protocolState.tags,
    ...useAppStore.getState().customTags,
  ]);
  useAppStore.setState({
    fetches: protocolState.fetches,
    tags: Array.from(combinedTags),
  });
});

// Subscribe to the app store's customTags to re-sync combined tags when they change.
// The selector function was removed to resolve the TS2554 error.
useAppStore.subscribe((state, prevState) => {
  // Only update if the custom tags have actually changed
  if (state.customTags !== prevState.customTags) {
    const protocolTags = useProtocolStore.getState().tags;
    const combinedTags = new Set([...protocolTags, ...state.customTags]);
    useAppStore.setState({
      tags: Array.from(combinedTags),
    });
  }
});
