import type {FeatureCollection} from 'geojson';
import type {GeoJSONSource, Map as MapLibreMap} from 'maplibre-gl';
import type {StopItem} from '~/lib/api/types';
import {stopServices, type Service} from '~/lib/domain/providers';
import {serviceColor} from '~/lib/map/paint';
import {STOP_OPACITY, type StopsMarks} from './stops';

const SOURCE = 'terminals';

// widest service first
const TERMINAL_SERVICES: readonly Service[] = ['inter', 'suburban', 'urban', 'neutral'];

export const terminalsLayer = {
  HIT: 'terminals-fill',

  addSource(map: MapLibreMap) {
    map.addSource(SOURCE, {type: 'geojson', data: features([], {selectedId: null, siblings: [], dim: false})});
  },

  addLayers(map: MapLibreMap) {
    const color = serviceColor();
    map.addLayer({
      id: 'terminals-fill',
      type: 'fill',
      source: SOURCE,
      paint: {
        'fill-color': color,
        'fill-opacity': ['case', ['get', 'selected'], 0.45, ['get', 'dim'], 0.12, ['get', 'muted'], 0.2, 0.28],
      },
    });
    map.addLayer({
      id: 'terminals-line',
      type: 'line',
      source: SOURCE,
      layout: {'line-join': 'round'},
      paint: {
        'line-color': color,
        'line-width': ['case', ['get', 'selected'], 3, 1.5],
        'line-opacity': STOP_OPACITY,
      },
    });
  },

  set(map: MapLibreMap, stops: StopItem[], marks: StopsMarks) {
    (map.getSource(SOURCE) as GeoJSONSource | undefined)?.setData(features(stops, marks));
  },
};

function features(stops: StopItem[], marks: StopsMarks): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: stops.flatMap((stop) => {
      if (!stop.footprint) return [];
      const ring: [number, number][] = [];
      for (let i = 0; i + 1 < stop.footprint.length; i += 2) ring.push([stop.footprint[i], stop.footprint[i + 1]]);
      const services = new Set(stopServices(stop));
      const selected = stop.id === marks.selectedId || marks.siblings.includes(stop.id);
      return [
        {
          type: 'Feature' as const,
          geometry: {type: 'Polygon' as const, coordinates: [ring]},
          properties: {
            stop_id: stop.id,
            stop_name: stop.name,
            service: TERMINAL_SERVICES.find((s) => services.has(s)) ?? 'urban',
            dim: marks.dim && !selected,
            muted: marks.selectedId != null && !selected,
            selected,
          },
        },
      ];
    }),
  };
}
