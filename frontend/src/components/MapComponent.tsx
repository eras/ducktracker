import React, { useEffect, useRef } from "react";
import { useAppStore } from "../hooks/useStore";
import "leaflet";

// Use a global L from the script tag
declare const L: typeof import("leaflet");

const MapComponent: React.FC = () => {
  const mapContainerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<L.Map | null>(null);
  const markersRef = useRef<L.LayerGroup | null>(null);
  const isFirstUpdateRef = useRef(true);
  const { locations, selectedTags } = useAppStore();

  // Initialize the map (runs only once)
  useEffect(() => {
    if (mapRef.current || !L) return;

    if (!mapContainerRef.current) return;

    // Use default view for Helsinki, Finland
    const map = L.map(mapContainerRef.current).setView(
      [59.436962, 24.753574],
      12,
    );

    L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
      attribution:
        '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
    }).addTo(map);

    const markers = L.layerGroup().addTo(map);
    mapRef.current = map;
    markersRef.current = markers;

    return () => {
      if (mapRef.current) {
        mapRef.current.remove();
        mapRef.current = null;
      }
    };
  }, []);

  // Update markers based on data and filters
  useEffect(() => {
    if (!markersRef.current) return;

    // Clear existing markers
    markersRef.current.clearLayers();

    // Add new markers based on filtered data
    Object.values(locations).forEach((trace) => {
      const hasSelectedTag = trace.t.some((tag) => selectedTags.has(tag));
      const isFiltered = selectedTags.size > 0 && !hasSelectedTag;

      if (!isFiltered) {
        trace.p.forEach((point) => {
          // Use [lon, lat] order as requested
          const marker = L.marker([point[0], point[1]]);
          marker.bindTooltip(`Tags: ${trace.t.join(", ")}`);
          markersRef.current?.addLayer(marker);
        });
      }
    });

    const allPoints = Object.values(locations).flatMap((trace) => trace.p);
    if (allPoints.length > 0 && mapRef.current && isFirstUpdateRef.current) {
      // Use [lon, lat] order as requested
      const bounds = L.latLngBounds(allPoints.map((p) => [p[0], p[1]]));
      mapRef.current.fitBounds(bounds, { padding: [50, 50], animate: false });
      isFirstUpdateRef.current = false;
    }
  }, [locations, selectedTags]);

  return <div ref={mapContainerRef} className="w-full h-full z-0 flex-grow" />;
};

export default MapComponent;
