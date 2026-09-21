import type {FeatureCollection} from 'geojson';
import type {GeoJSONSource, Map as MapLibreMap} from 'maplibre-gl';

const SOURCE = 'stops';
const BRAND = '#00b360';

export interface StopPoint {
  stop_id: string;
  stop_name: string;
  lat: number;
  lng: number;
}

// geojson, dom markers kill panning at this volume
export const stopsLayer = {
  HIT: 'stops-halo',
  LAYERS: ['stops-halo', 'stops-dot'],

  add(map: MapLibreMap) {
    if (map.getSource(SOURCE)) return;

    map.addSource(SOURCE, {type: 'geojson', data: features([], new Set(), null)});
    map.addLayer({
      id: 'stops-halo',
      type: 'circle',
      source: SOURCE,
      paint: {
        'circle-radius': [
          'interpolate',
          ['linear'],
          ['zoom'],
          12,
          ['case', ['get', 'selected'], 7, 4],
          18,
          ['case', ['get', 'selected'], 14, 10],
        ],
        'circle-color': '#ffffff',
        'circle-opacity': ['case', ['get', 'dim'], 0.45, 1],
        'circle-stroke-width': ['case', ['get', 'selected'], 3, 1],
        'circle-stroke-color': ['case', ['get', 'selected'], BRAND, 'rgba(0,0,0,0.25)'],
      },
    });
    map.addLayer({
      id: 'stops-dot',
      type: 'circle',
      source: SOURCE,
      paint: {
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 12, 2, 18, 5],
        'circle-color': BRAND,
        'circle-opacity': ['case', ['get', 'dim'], 0.45, 1],
      },
    });
  },

  // dim, not hidden
  set(map: MapLibreMap, stops: StopPoint[], onRoute: Set<string>, selected: string | null) {
    (map.getSource(SOURCE) as GeoJSONSource | undefined)?.setData(features(stops, onRoute, selected));
  },
};

function features(stops: StopPoint[], onRoute: Set<string>, selected: string | null): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: stops.map((s) => ({
      type: 'Feature',
      geometry: {type: 'Point', coordinates: [s.lng, s.lat]},
      properties: {
        stop_id: s.stop_id,
        stop_name: s.stop_name,
        dim: onRoute.size > 0 && !onRoute.has(s.stop_id),
        selected: s.stop_id === selected,
      },
    })),
  };
}
