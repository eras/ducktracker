import React, { useEffect, useRef } from "react";
import MapComponent from "./components/MapComponent";
import TagFilter from "./components/TagFilter";
import { useProtocolStore } from "./lib/protocol";
import { useAppStore } from "./hooks/useStore";

const App: React.FC = () => {
  const { connect, disconnect } = useProtocolStore();
  const selectedTags: Set<string> = useAppStore((state) => state.selectedTags);
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    // Disconnect any existing connection before creating a new one
    if (eventSourceRef.current) {
      disconnect(eventSourceRef.current);
      eventSourceRef.current = null;
    }

    const tagsArray = Array.from(selectedTags);
    const newEventSource = connect(tagsArray);
    eventSourceRef.current = newEventSource;

    return () => {
      // Cleanup on unmount
      if (eventSourceRef.current) {
        disconnect(eventSourceRef.current);
        eventSourceRef.current = null;
      }
    };
  }, [selectedTags, connect, disconnect]);

  return (
    <div className="relative w-full h-full">
      <MapComponent />
      <TagFilter />
    </div>
  );
};

export default App;
