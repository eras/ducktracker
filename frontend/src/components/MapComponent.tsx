import React, { useEffect, useRef } from 'react';
import maplibregl from 'maplibre-gl';
import { useAppStore } from '../hooks/useStore';

const MapComponent: React.FC = () => {
  const mapContainerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<maplibregl.Map | null>(null);
  const { locations, selectedTags } = useAppStore();

  useEffect(() => {
    if (mapRef.current) return; // Initialize map only once

    if (!mapContainerRef.current) return;

    // NOTE: The map style URL should be provided by your backend.
    // This is a placeholder for demonstration purposes.
    const map = new maplibregl.Map({
      container: mapContainerRef.current,
      style: 'https://demotiles.maplibre.org/style.json',
      center: [-74.006, 40.7128], // Initial center
      zoom: 12,
    });

    mapRef.current = map;

    return () => {
      map.remove();
    };
  }, []);

  useEffect(() => {
    if (!mapRef.current) return;

    const map = mapRef.current;

    const geojson: GeoJSON.FeatureCollection = {
      type: 'FeatureCollection',
      features: [],
    };

    Object.values(locations).forEach((trace) => {
      const hasSelectedTag = trace.t.some(tag => selectedTags.has(tag));
      const isFiltered = selectedTags.size > 0 && !hasSelectedTag;

      if (!isFiltered) {
        // Add each point from the trace as a GeoJSON Feature
        trace.p.forEach(point => {
          geojson.features.push({
            type: 'Feature',
            geometry: {
              type: 'Point',
              coordinates: [point[0], point[1]],
            },
            properties: {
              tags: trace.t,
            },
          });
        });
      }
    });

    const source = map.getSource('traces') as maplibregl.GeoJSONSource | undefined;

    if (source) {
      source.setData(geojson);
    } else {
      map.on('load', () => {
        map.addSource('traces', {
          type: 'geojson',
          data: geojson,
        });

        map.addLayer({
          id: 'traces-layer',
          type: 'circle',
          source: 'traces',
          paint: {
            'circle-radius': 5,
            'circle-color': '#EF4444', // Tailwind's red-500
          },
        });
      });
    }
  }, [locations, selectedTags]);

  return <div ref={mapContainerRef} className="w-full h-screen" />;
};

export default MapComponent;