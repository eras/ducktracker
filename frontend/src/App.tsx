import React, { useEffect, useMemo } from "react"; // Add useMemo
import MapComponent from "./components/MapComponent";
import TagFilter from "./components/TagFilter";
import LoginOverlay from "./components/LoginOverlay";
import { useProtocolStore } from "./lib/protocol";
import { useAppStore } from "./hooks/useStore";
import { useAuthStore } from "./hooks/useAuthStore";
import { useGeolocation } from "./hooks/useGeolocation";
import HamburgerMenu from "./components/HamburgerMenu.tsx";
import useThrottle from "./hooks/useThrottle"; // Import useThrottle

const App: React.FC = () => {
  const { connect, disconnect } = useProtocolStore();
  const rawSelectedTags = useAppStore((state) => state.selectedTags); // Get the raw Set
  const { username, password } = useAuthStore();

  // 1. Throttle the Set itself to reduce update frequency
  // The returned throttledSelectedTagsSet reference only changes after `limit` and value update.
  const throttledSelectedTagsSet = useThrottle(rawSelectedTags, 1000); // Throttle for 1 second

  // 2. Memoize the conversion to a sorted array.
  // This ensures the array reference only changes if throttledSelectedTagsSet itself changes.
  const memoizedSelectedTagsArray = useMemo(() => {
    // Sorting ensures a consistent array order for comparison in 'connect'
    return Array.from(throttledSelectedTagsSet).sort();
  }, [throttledSelectedTagsSet]); // Dependency on the throttled Set

  useGeolocation();

  useEffect(() => {
    const establishConnection = async () => {
      // Pass the memoized array to connect
      await connect(memoizedSelectedTagsArray, useAppStore.getState().addTags);
    };

    establishConnection();

    return () => {
      disconnect();
    };
  }, [
    memoizedSelectedTagsArray, // Use the memoized array as a dependency
    connect,
    disconnect,
    username,
    password,
  ]); // username/password as dependencies are correct for re-authentication

  return (
    <div className="relative w-full h-full">
      <MapComponent />
      <TagFilter />
      <LoginOverlay />
      <HamburgerMenu />
    </div>
  );
};

export default App;
