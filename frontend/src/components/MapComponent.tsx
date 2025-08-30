import React, { useEffect, useRef, useState } from "react";
import maplibregl from "maplibre-gl";
import { useAppStore } from "../hooks/useStore";

const MapComponent: React.FC = () => {
  const mapContainerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<maplibregl.Map | null>(null);
  const [isMapLoaded, setIsMapLoaded] = useState(false);
  const { locations, selectedTags } = useAppStore();

  // Effect for map initialization (runs only once)
  useEffect(() => {
    if (mapRef.current) return;

    if (!mapContainerRef.current) return;

    const map = new maplibregl.Map({
      container: mapContainerRef.current,
      style: "https://demotiles.maplibre.org/style.json",
      //style: "https://www.openstreetmap.org/styles/osm-carto/style.json",
      center: [17.65431710431244, 32.954120326746775],
      zoom: 12,
    });

    mapRef.current = map;

    // Add source and layer only after the map style has fully loaded
    map.on("load", () => {
      map.addSource("traces", {
        type: "geojson",
        data: {
          type: "FeatureCollection",
          features: [],
        },
      });

      map.addLayer({
        id: "traces-layer",
        type: "circle",
        source: "traces",
        paint: {
          "circle-radius": 5,
          "circle-color": "#EF4444", // Tailwind's red-500
        },
      });
      // Set state to indicate map is ready for data updates
      setIsMapLoaded(true);
    });

    // Cleanup function
    return () => {
      if (mapRef.current) {
        mapRef.current.remove();
        mapRef.current = null;
      }
    };
  }, []);

  // Effect for updating data (runs on location/filter changes and when map is loaded)
  useEffect(() => {
    if (!isMapLoaded || !mapRef.current) {
      return;
    }

    const map = mapRef.current;

    // Get the source. This is now safe because we know the map is loaded.
    const source = map.getSource("traces") as
      | maplibregl.GeoJSONSource
      | undefined;

    if (source) {
      const geojson: GeoJSON.FeatureCollection = {
        type: "FeatureCollection",
        features: [],
      };

      Object.values(locations).forEach((trace) => {
        const hasSelectedTag = trace.t.some((tag) => selectedTags.has(tag));
        const isFiltered = selectedTags.size > 0 && !hasSelectedTag;

        if (!isFiltered) {
          // Add each point from the trace as a GeoJSON Feature
          trace.p.forEach((point) => {
            geojson.features.push({
              type: "Feature",
              geometry: {
                type: "Point",
                coordinates: [point[0], point[1]],
              },
              properties: {
                tags: trace.t,
              },
            });
          });
        }
      });

      source.setData(geojson);
    }
  }, [isMapLoaded, locations, selectedTags]);

  return <div ref={mapContainerRef} className="w-full h-screen" />;
};

export default MapComponent;
