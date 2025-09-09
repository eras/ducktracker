import React from "react";
import { useAppStore } from "../hooks/useStore";

const LocationButton: React.FC = () => {
  const showClientLocation = useAppStore((state) => state.showClientLocation);
  const toggleClientLocation = useAppStore(
    (state) => state.toggleClientLocation,
  );

  // Using Unicode symbols
  const buttonSymbol = showClientLocation ? "🚫" : "📍";

  return (
    <button
      onClick={toggleClientLocation}
      className="absolute bottom-4 right-4 bg-white p-2 rounded-md shadow-lg z-10 text-sm font-medium flex items-center space-x-1"
    >
      <span>{buttonSymbol}</span>
    </button>
  );
};

export default LocationButton;
