import React, { useEffect } from "react";
import MapComponent from "./components/MapComponent";
import TagFilter from "./components/TagFilter";
import LoginOverlay from "./components/LoginOverlay";
import LocationButton from "./components/LocationButton"; // New import for the button
import { useProtocolStore } from "./lib/protocol";
import { useAppStore } from "./hooks/useStore";
import { useAuthStore } from "./hooks/useAuthStore";
import { useGeolocation } from "./hooks/useGeolocation"; // New import for the geolocation hook
import HamburgerMenu from "./components/HamburgerMenu.tsx";

const App: React.FC = () => {
  const { connect, disconnect } = useProtocolStore();
  const selectedTags = useAppStore((state) => state.selectedTags);
  const { username, password } = useAuthStore(); // username and password trigger re-connection

  useGeolocation(); // Call the geolocation hook here to manage client location

  useEffect(() => {
    const establishConnection = async () => {
      // `connect` now handles closing any existing connection internally
      // and also checks for credentials and shows login if needed.
      await connect(Array.from(selectedTags), useAppStore.getState().addTags);
    };

    establishConnection();

    return () => {
      // Cleanup on unmount, `disconnect` manages its internal eventSource
      disconnect();
    };
  }, [selectedTags, connect, disconnect, username, password]); // Added username/password as dependencies

  return (
    <div className="relative w-full h-full">
      <MapComponent />
      <TagFilter />
      <LoginOverlay />
      <LocationButton />
      <HamburgerMenu />
    </div>
  );
};

export default App;
