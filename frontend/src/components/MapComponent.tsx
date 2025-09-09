import React, { useEffect, useRef } from "react";
import { useAppStore } from "../hooks/useStore";
import { intersection } from "../lib/set";
import useThrottle from "../hooks/useThrottle";
import "leaflet";

// Use a global L from the script tag
declare const L: typeof import("leaflet");

const MapComponent: React.FC = () => {
  const mapContainerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<L.Map | null>(null);
  const markersRef = useRef<L.LayerGroup | null>(null);
  const polylinesRef = useRef<L.LayerGroup | null>(null); // Reference for the trace lines
  const clientLocationMarkerRef = useRef<L.Marker | null>(null);
  const isFirstUpdateRef = useRef(true); // Used for initial map bounds/centering
  const { fetches, selectedTags, showClientLocation, clientLocation } =
    useAppStore();
  const throttledFetches = useThrottle(fetches, 1000);

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
    const polylines = L.layerGroup().addTo(map); // Add a layer for polylines
    mapRef.current = map;
    markersRef.current = markers;
    polylinesRef.current = polylines;

    return () => {
      if (mapRef.current) {
        mapRef.current.remove();
        mapRef.current = null;
      }
    };
  }, []);

  // Update markers and polylines based on data and filters, and client location
  useEffect(() => {
    if (!markersRef.current || !polylinesRef.current || !mapRef.current) return;

    // Clear existing trace markers and polylines
    markersRef.current.clearLayers();
    polylinesRef.current.clearLayers();

    // Add new markers and polylines based on filtered data
    Object.entries(throttledFetches).forEach(([_fetch_id, fetch]) => {
      const hasSelectedTag = intersection(fetch.tags, selectedTags).size !== 0;
      const isFiltered = selectedTags.size > 0 && !hasSelectedTag;

      if (!isFiltered) {
        const points = fetch.locations.map(
          (loc) => loc.latlon as L.LatLngTuple,
        );

        // Render polyline
        const polyline = L.polyline(points, { color: "blue", weight: 3 });
        polylinesRef.current?.addLayer(polyline);

        // Render markers for trace end points
        if (fetch.locations.length) {
          const loc = fetch.locations[fetch.locations.length - 1];
          const marker = L.circleMarker(loc.latlon, {
            radius: 6,
            fillColor: "#0078A8",
            color: "#fff",
            weight: 1,
            opacity: 1,
            fillOpacity: 0.8,
          });
          marker.bindTooltip(`${[...fetch.tags].join(", ")}`);
          markersRef.current?.addLayer(marker);
        }
      }
    });

    // Handle client location marker
    if (showClientLocation && clientLocation) {
      if (!clientLocationMarkerRef.current) {
        // Create a custom icon for the client location
        // TailwindCSS classes for a visible, distinct marker
        const clientIcon = L.divIcon({
          className: "client-location-icon",
          html: '<div class="w-4 h-4 rounded-full bg-red-600 border-2 border-white shadow-md"></div>',
          iconSize: [20, 20],
          iconAnchor: [10, 10], // Centered
        });

        clientLocationMarkerRef.current = L.marker(clientLocation, {
          icon: clientIcon,
        }).addTo(mapRef.current);
        clientLocationMarkerRef.current
          .bindTooltip("Your Location")
          .openTooltip();
      } else {
        clientLocationMarkerRef.current.setLatLng(clientLocation);
      }
      // On first load or if client location is enabled, center map to client location if no other fetches
      if (
        isFirstUpdateRef.current &&
        Object.values(throttledFetches).length === 0
      ) {
        mapRef.current.setView(clientLocation, mapRef.current.getZoom() || 15);
        isFirstUpdateRef.current = false;
      }
    } else {
      // If client location tracking is off or location is null, remove the marker
      if (clientLocationMarkerRef.current) {
        mapRef.current.removeLayer(clientLocationMarkerRef.current);
        clientLocationMarkerRef.current = null;
      }
    }

    const allLocs = Object.values(throttledFetches).flatMap(
      (trace) => trace.locations,
    );
    if (allLocs.length > 0 && isFirstUpdateRef.current) {
      const bounds = L.latLngBounds(allLocs.map((p) => p.latlon));
      mapRef.current.fitBounds(bounds, { padding: [50, 50], animate: false });
      isFirstUpdateRef.current = false;
    }
  }, [throttledFetches, selectedTags, showClientLocation, clientLocation]); // Add new dependencies

  return <div ref={mapContainerRef} className="w-full h-full z-0" />;
};

export default MapComponent;
