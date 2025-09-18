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

  showClientLocation: boolean;
  clientLocation: L.LatLngTuple | null; // Leaflet's LatLngTuple for consistency [latitude, longitude]
  toggleClientLocation: () => void;
  setClientLocation: (location: L.LatLngTuple | null) => void;
  disableClientLocationPersisted: () => void; // Action to disable and persist (e.g., on permission denied)

  showTraces: boolean;
  toggleShowTraces: () => void;

  showNames: boolean;
  toggleShowNames: () => void;
}

const CUSTOM_TAGS_KEY = "customTags";
const SELECTED_TAGS_KEY = "selectedTags";
const SHOW_CLIENT_LOCATION_KEY = "showClientLocation";
const SHOW_TRACES_KEY = "showTraces";
const SHOW_NAMES_KEY = "showNames";

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

const getStoredShowClientLocation = (): boolean => {
  try {
    const stored = localStorage.getItem(SHOW_CLIENT_LOCATION_KEY);
    // Default to false if not found or invalid
    return stored ? JSON.parse(stored) : false;
  } catch (e) {
    console.error("Failed to load showClientLocation from localStorage", e);
    return false;
  }
};

const getStoredShowTraces = (): boolean => {
  try {
    const stored = localStorage.getItem(SHOW_TRACES_KEY);
    return stored ? JSON.parse(stored) : true;
  } catch (e) {
    console.error("Failed to load showTraces from localStorage", e);
    return false;
  }
};

const getStoredShowNames = (): boolean => {
  try {
    const stored = localStorage.getItem(SHOW_NAMES_KEY);
    return stored ? JSON.parse(stored) : false;
  } catch (e) {
    console.error("Failed to load showNames from localStorage", e);
    return false;
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

const saveShowClientLocationToStorage = (value: boolean) => {
  try {
    localStorage.setItem(SHOW_CLIENT_LOCATION_KEY, JSON.stringify(value));
  } catch (e) {
    console.error("Failed to save showClientLocation to localStorage", e);
  }
};

const saveShowTracesToStorage = (value: boolean) => {
  try {
    localStorage.setItem(SHOW_TRACES_KEY, JSON.stringify(value));
  } catch (e) {
    console.error("Failed to save showTraces to localStorage", e);
  }
};

const saveShowNamesToStorage = (value: boolean) => {
  try {
    localStorage.setItem(SHOW_NAMES_KEY, JSON.stringify(value));
  } catch (e) {
    console.error("Failed to save showNames to localStorage", e);
  }
};

const parseUrl = (url: string): { pub: Set<string>; priv: Set<string> } => {
  const regex = /(#|&)?pub:([^&]*)(&(?:#|$))?/g;
  const regex2 = /(#|&)?priv:([^&]*)(&(?:#|$))?/g;

  let match;
  const pubTags = new Set<string>();
  while ((match = regex.exec(url)) !== null) {
    const tag = decodeURIComponent(match[2])
      .replace(/\s*,\s*/g, ",")
      .split(",");
    for (const t of tag) {
      if (t) pubTags.add(t); // Add check for empty strings
    }
  }

  let matchPriv;
  const privTags = new Set<string>();
  while ((matchPriv = regex2.exec(url)) !== null) {
    const tag = decodeURIComponent(matchPriv[2])
      .replace(/\s*,\s*/g, ",")
      .split(",");
    for (const t of tag) {
      if (t) privTags.add(t); // Add check for empty strings
    }
  }

  return { pub: pubTags, priv: privTags };
};

// --- Global Initialization Logic ---
const { pub: urlPubTags, priv: urlPrivTags } = parseUrl(window.location.href);

// Get initial values from localStorage
const initialStoredSelectedTags = getSelectedTags();
const initialStoredCustomTags = getStoredTags();
const initialStoredShowClientLocation = getStoredShowClientLocation();
const initialStoredShowTraces = getStoredShowTraces();
const initialStoredShowNames = getStoredShowNames();

// Combine stored tags with URL tags
const combinedInitialSelectedTags = union(
  initialStoredSelectedTags,
  urlPubTags,
  urlPrivTags,
);
const combinedInitialCustomTags = union(initialStoredCustomTags, urlPrivTags);

// Persist the combined initial tags immediately
saveSelectedTagsToStorage(combinedInitialSelectedTags);
saveTagsToStorage(combinedInitialCustomTags);

// Initial 'tags' state includes all tags from URL, and custom tags.
// The protocol store will add its own tags later via subscription.
const initialTotalTags = union(
  urlPubTags,
  urlPrivTags,
  combinedInitialCustomTags,
);

export const useAppStore = create<AppState>((set) => ({
  selectedTags: combinedInitialSelectedTags,
  fetches: {},
  tags: initialTotalTags,
  customTags: combinedInitialCustomTags,
  // --- New client location initial state ---
  showClientLocation: initialStoredShowClientLocation,
  showTraces: initialStoredShowTraces,
  showNames: initialStoredShowNames,
  clientLocation: null,

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
      if (difference(newTags, state.tags).size === 0) {
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
      // Also update selectedTags if the new custom tag is added
      saveSelectedTagsToStorage(newSelectedTags);
      return { customTags: newCustomTags, selectedTags: newSelectedTags };
    }),

  removeCustomTag: (tag: string) =>
    set((state) => {
      const newCustomTags = difference(state.customTags, new Set([tag]));
      const newSelectedTags = difference(state.selectedTags, new Set([tag]));
      saveTagsToStorage(newCustomTags);
      // Also update selectedTags if the custom tag is removed
      saveSelectedTagsToStorage(newSelectedTags);
      return { customTags: newCustomTags, selectedTags: newSelectedTags };
    }),

  toggleClientLocation: () =>
    set((state) => {
      const newShowLocation = !state.showClientLocation;
      saveShowClientLocationToStorage(newShowLocation);
      // If toggling off, clear the location
      return {
        showClientLocation: newShowLocation,
        clientLocation: newShowLocation ? state.clientLocation : null,
      };
    }),

  setClientLocation: (location: L.LatLngTuple | null) =>
    set({ clientLocation: location }),

  toggleShowTraces: () =>
    set((state) => {
      const newShowTraces = !state.showTraces;
      saveShowTracesToStorage(newShowTraces);
      return {
        showTraces: newShowTraces,
      };
    }),

  toggleShowNames: () =>
    set((state) => {
      const newShowNames = !state.showNames;
      saveShowNamesToStorage(newShowNames);
      return {
        showNames: newShowNames,
      };
    }),

  disableClientLocationPersisted: () =>
    set(() => {
      saveShowClientLocationToStorage(false); // Persist as disabled
      return { showClientLocation: false, clientLocation: null };
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
