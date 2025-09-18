import React, { useEffect, useRef } from "react";
import { useAppStore } from "../hooks/useStore";
import { intersection } from "../lib/set";
import useThrottle from "../hooks/useThrottle";
import "leaflet";
import { useProtocolStore } from "../lib/protocol"; // Import useProtocolStore to get serverTime

// Use a global L from the script tag
declare const L: typeof import("leaflet");

// --- Color Fading Constants ---
const START_COLOR_RGBA = [0, 0, 255, 1];
const END_COLOR_RGBA = [128, 128, 128, 0.2];
const MAX_AGE_FADE_SECONDS = 600;

// Helper function to interpolate RGBA colors
const interpolateColor = (
  color1: number[],
  color2: number[],
  factor: number,
): string => {
  const r = Math.round(color1[0] + factor * (color2[0] - color1[0]));
  const g = Math.round(color1[1] + factor * (color2[1] - color1[1]));
  const b = Math.round(color1[2] + factor * (color2[2] - color1[2]));
  const a = color1[3] + factor * (color2[3] - color1[3]);
  return `rgba(${r},${g},${b},${a.toFixed(2)})`;
};

const MapComponent: React.FC = () => {
  const mapContainerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<L.Map | null>(null);
  const markersRef = useRef<L.LayerGroup | null>(null);
  const polylinesRef = useRef<L.LayerGroup | null>(null); // Reference for the trace lines
  const markerInstancesRef = useRef<Map<string, L.CircleMarker>>(new Map());
  const polylineInstancesRef = useRef<Map<string, L.Polyline[]>>(new Map()); // For polylines per fetch
  const {
    fetches,
    selectedTags,
    showClientLocation,
    clientLocation,
    showTraces,
    showNames,
  } = useAppStore();
  const { serverTime } = useProtocolStore(); // Get serverTime from protocol store
  const throttledFetches = useThrottle(fetches, 1000);

  // Initialize the map (runs only once)
  useEffect(() => {
    if (mapRef.current || !L) return;

    if (!mapContainerRef.current) return;

    // Use default view for Helsinki, Finland
    const map = L.map(mapContainerRef.current, { preferCanvas: true }).setView(
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

  useEffect(() => {
    if (!markersRef.current || !polylinesRef.current || !mapRef.current) return;

    const markersToKeep = new Set<string>(); // Keep track of markers that correspond to current data
    const polylinesToKeep = new Set<string>(); // Keep track of polylines that correspond to current data

    // --- Handle Markers ---
    Object.entries(throttledFetches).forEach(([fetch_id, fetch]) => {
      const hasSelectedTag = intersection(fetch.tags, selectedTags).size !== 0;
      const isFiltered = selectedTags.size > 0 && !hasSelectedTag;

      if (!isFiltered && fetch.locations.length > 0) {
        const loc = fetch.locations[fetch.locations.length - 1]; // Last location

        let marker = markerInstancesRef.current.get(fetch_id);
        let tooltipContent = "";

        if (fetch.name) {
          tooltipContent += `<b>${fetch.name}</b>`;
        }
        if (!showNames) {
          if (tooltipContent) {
            tooltipContent += "<br/>";
          }
          tooltipContent += [...fetch.tags].join(", ");
        }

        if (marker) {
          // Update existing marker
          marker.setLatLng(loc.latlon);
          // Update tooltip content if needed
          const tooltip = marker.getTooltip();
          if (tooltip?.getContent() !== tooltipContent) {
            if (tooltip) {
              tooltip.setContent(tooltipContent);
            } else if (tooltipContent) {
              // If it had no tooltip, but now needs one
              marker.bindTooltip(tooltipContent, {
                direction: "bottom",
                offset: L.point(0, 10),
                permanent: showNames,
                className: "tooltip",
              });
            }
          }
          // Update permanent state if showNames changed
          if (tooltip && tooltip.options.permanent !== showNames) {
            marker.unbindTooltip(); // Unbind to reset options
            marker.bindTooltip(tooltipContent, {
              direction: "bottom",
              offset: L.point(0, 10),
              permanent: showNames,
              className: "tooltip",
            });
          }
        } else {
          // Create new marker
          marker = L.circleMarker(loc.latlon, {
            radius: 6,
            fillColor: "#0078A8",
            color: "#fff",
            weight: 1,
            opacity: 1,
            fillOpacity: 0.8,
          });
          if (tooltipContent) {
            marker.bindTooltip(tooltipContent, {
              direction: "bottom",
              offset: L.point(0, 10),
              permanent: showNames,
              className: "tooltip",
            });
          }
          markersRef.current?.addLayer(marker);
          markerInstancesRef.current.set(fetch_id, marker);
        }
        markersToKeep.add(fetch_id);
      }
    });

    // Remove markers that are no longer in `throttledFetches` or are filtered out
    markerInstancesRef.current.forEach((marker, fetch_id) => {
      if (!markersToKeep.has(fetch_id)) {
        markersRef.current?.removeLayer(marker);
        markerInstancesRef.current.delete(fetch_id);
      }
    });

    // --- Handle Polylines ---
    // For polylines, due to the segment-specific fading, it's often easier
    // to clear and redraw *per trace* rather than try to reuse individual segments.
    // However, we can still remove traces that no longer exist.

    if (showTraces) {
      Object.entries(throttledFetches).forEach(([fetch_id, fetch]) => {
        const hasSelectedTag =
          intersection(fetch.tags, selectedTags).size !== 0;
        const isFiltered = selectedTags.size > 0 && !hasSelectedTag;

        if (!isFiltered) {
          // Clear existing polylines for this specific fetch
          let existingPolylines = polylineInstancesRef.current.get(fetch_id);
          if (existingPolylines) {
            existingPolylines.forEach((p) =>
              polylinesRef.current?.removeLayer(p),
            );
          }
          const newPolylines: L.Polyline[] = [];

          // Render new polyline segments with fading effect
          for (let i = 0; i < fetch.locations.length - 1; i++) {
            const loc1 = fetch.locations[i];
            const loc2 = fetch.locations[i + 1];

            const ageSeconds = Math.max(0, serverTime - loc2.time);

            let factor = Math.min(1, ageSeconds / MAX_AGE_FADE_SECONDS);

            const segmentColor = interpolateColor(
              START_COLOR_RGBA,
              END_COLOR_RGBA,
              factor,
            );

            const segmentPolyline = L.polyline([loc1.latlon, loc2.latlon], {
              color: segmentColor,
              weight: 3,
            });
            polylinesRef.current?.addLayer(segmentPolyline);
            newPolylines.push(segmentPolyline);
          }
          polylineInstancesRef.current.set(fetch_id, newPolylines);
          polylinesToKeep.add(fetch_id);
        }
      });
    }

    // Remove polylines for traces that no longer exist or are filtered out
    polylineInstancesRef.current.forEach((polylines, fetch_id) => {
      if (!polylinesToKeep.has(fetch_id) || !showTraces) {
        // Also remove if showTraces is false
        polylines.forEach((p) => polylinesRef.current?.removeLayer(p));
        polylineInstancesRef.current.delete(fetch_id);
      }
    });

    // ... (client location logic remains largely the same)
    // Make sure the clientLocationMarkerRef is handled to avoid being cleared by markerInstancesRef
    // ... (fitBounds logic remains the same)
    // No need for markersRef.current.clearLayers() and polylinesRef.current.clearLayers() anymore at the start
  }, [
    throttledFetches,
    selectedTags,
    showClientLocation,
    clientLocation,
    serverTime,
    showTraces,
    showNames,
  ]);
  return <div ref={mapContainerRef} className="w-full h-full z-0" />;
};

export default MapComponent;
