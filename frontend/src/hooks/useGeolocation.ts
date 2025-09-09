import { useEffect, useRef } from "react";
import { useAppStore } from "./useStore";

export const useGeolocation = () => {
  const showClientLocation = useAppStore((state) => state.showClientLocation);
  const setClientLocation = useAppStore((state) => state.setClientLocation);
  const disableClientLocationPersisted = useAppStore(
    (state) => state.disableClientLocationPersisted,
  );
  const watcherId = useRef<number | null>(null);

  useEffect(() => {
    if (!navigator.geolocation) {
      console.warn("Geolocation is not supported by your browser.");
      if (showClientLocation) {
        // If location was enabled but browser doesn't support, disable it and persist.
        disableClientLocationPersisted();
      }
      return;
    }

    const successHandler = (position: GeolocationPosition) => {
      setClientLocation([position.coords.latitude, position.coords.longitude]);
    };

    const errorHandler = (error: GeolocationPositionError) => {
      console.error("Geolocation error:", error);
      setClientLocation(null); // Clear location on error
      if (error.code === error.PERMISSION_DENIED) {
        // If permission is denied, disable location tracking and persist this state.
        // The user must manually re-enable it.
        console.warn(
          "Geolocation permission denied. Disabling location tracking.",
        );
        disableClientLocationPersisted();
      } else if (showClientLocation) {
        // For other errors (e.g., position unavailable, timeout), keep trying
        // but log the issue. Do not disable persistenly unless it's a permission issue.
        console.warn(
          "Geolocation watch position encountered an error, will retry.",
        );
      }
    };

    if (showClientLocation) {
      // Start watching the position
      watcherId.current = navigator.geolocation.watchPosition(
        successHandler,
        errorHandler,
        {
          enableHighAccuracy: true,
          timeout: 10000, // Increased timeout for better mobile experience
          maximumAge: 0, // Always try to get a fresh position
        },
      );
    } else {
      // Stop watching if showClientLocation is false
      if (watcherId.current !== null) {
        navigator.geolocation.clearWatch(watcherId.current);
        watcherId.current = null;
        setClientLocation(null); // Clear location when tracking stops
      }
    }

    return () => {
      // Cleanup on component unmount or when showClientLocation becomes false
      if (watcherId.current !== null) {
        navigator.geolocation.clearWatch(watcherId.current);
        watcherId.current = null;
      }
      setClientLocation(null); // Ensure location is cleared on unmount
    };
  }, [showClientLocation, setClientLocation, disableClientLocationPersisted]);
};
