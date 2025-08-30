import React, { useEffect } from 'react';
import MapComponent from './components/MapComponent';
import TagFilter from './components/TagFilter';
import { useProtocolStore } from './lib/protocol';

const App: React.FC = () => {
  const fetchData = useProtocolStore((state) => state.fetchData);

  useEffect(() => {
    // Start fetching data from the server's SSE stream
    fetchData();
  }, [fetchData]);

  return (
    <div className="relative w-screen h-screen">
      <MapComponent />
      <TagFilter />
    </div>
  );
};

export default App;