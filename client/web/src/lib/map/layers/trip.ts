import type {FeatureCollection} from 'geojson';
import {LngLatBounds, type ExpressionSpecification, type GeoJSONSource, type Map as MapLibreMap} from 'maplibre-gl';
import {outline, serviceColor, tripColor} from '~/lib/map/paint';

const SOURCE = 'trip';
const EMPTY: FeatureCollection = {type: 'FeatureCollection', features: []};
const ENDPOINTS: ExpressionSpecification = ['in', ['get', 'kind'], ['literal', ['origin', 'destination']]];
const FIT_MAX_ZOOM = 16;
const FIT_MS = 500;
const WIDE = 768; // px

export const tripLayer = {
  add(map: MapLibreMap) {
    map.addSource(SOURCE, {type: 'geojson', data: EMPTY});
    map.addLayer({
      id: 'trip-walk',
      type: 'line',
      source: SOURCE,
      filter: ['==', ['get', 'kind'], 'walk'],
      layout: {'line-cap': 'round', 'line-join': 'round'},
      paint: {
        'line-color': tripColor('walk'),
        'line-width': ['interpolate', ['linear'], ['zoom'], 11, 2.5, 17, 5],
        'line-dasharray': [0.1, 1.8],
      },
    });
    map.addLayer({
      id: 'trip-bus',
      type: 'line',
      source: SOURCE,
      filter: ['==', ['get', 'kind'], 'bus'],
      layout: {'line-cap': 'round', 'line-join': 'round'},
      paint: {
        'line-color': serviceColor(),
        'line-width': ['interpolate', ['linear'], ['zoom'], 11, 5, 14, 9, 17, 14],
      },
    });
    map.addLayer({
      id: 'trip-stops',
      type: 'circle',
      source: SOURCE,
      filter: ['==', ['get', 'kind'], 'transfer'],
      paint: {
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 11, 3.5, 17, 7],
        'circle-color': outline(),
        'circle-stroke-color': serviceColor(),
        'circle-stroke-width': ['interpolate', ['linear'], ['zoom'], 11, 2, 17, 3.5],
      },
    });
    map.addLayer({
      id: 'trip-points',
      type: 'circle',
      source: SOURCE,
      filter: ENDPOINTS,
      paint: {
        'circle-radius': 11,
        'circle-color': ['case', ['==', ['get', 'kind'], 'origin'], tripColor('origin'), tripColor('destination')],
        'circle-stroke-color': outline(),
        'circle-stroke-width': 2.5,
      },
    });
    map.addLayer({
      id: 'trip-labels',
      type: 'symbol',
      source: SOURCE,
      filter: ENDPOINTS,
      layout: {
        'text-field': ['get', 'label'],
        'text-font': ['Noto Sans Medium'],
        'text-size': 13,
        'text-allow-overlap': true,
        'text-ignore-placement': true,
      },
      paint: {'text-color': outline()},
    });
  },

  set(map: MapLibreMap, trip: FeatureCollection | null) {
    (map.getSource(SOURCE) as GeoJSONSource | undefined)?.setData(trip ?? EMPTY);
  },

  // clear of the sheet
  fit(map: MapLibreMap, trip: FeatureCollection) {
    const bounds = new LngLatBounds();
    for (const feature of trip.features) {
      const g = feature.geometry;
      if (g.type === 'Point') bounds.extend(g.coordinates as [number, number]);
      if (g.type === 'LineString') for (const c of g.coordinates) bounds.extend(c as [number, number]);
    }
    if (bounds.isEmpty()) return;
    const {width, height} = map.getCanvas().getBoundingClientRect();
    const padding =
      width >= WIDE
        ? {top: 56, right: 56, bottom: 56, left: 420}
        : {top: 48, right: 32, bottom: Math.round(height * 0.55) + 24, left: 32};
    map.fitBounds(bounds, {padding, duration: FIT_MS, maxZoom: FIT_MAX_ZOOM});
  },
};
