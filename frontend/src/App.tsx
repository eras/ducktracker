import React, { useEffect } from "react";
import MapComponent from "./components/MapComponent";
import TagFilter from "./components/TagFilter";
import { useProtocolStore } from "./lib/protocol";

const App: React.FC = () => {
  const fetchData = useProtocolStore((state) => state.fetchData);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  return (
    <div className="relative w-full h-full">
      <MapComponent />
      <TagFilter />
    </div>
  );
};

export default App;
