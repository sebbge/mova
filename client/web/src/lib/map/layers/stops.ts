import type {FeatureCollection} from 'geojson';
import type {ExpressionSpecification, GeoJSONSource, Map as MapLibreMap} from 'maplibre-gl';
import type {StopItem} from '~/lib/api/types';
import {comboKey, providerInfo, SERVICE_COMBOS, SERVICES, serviceOf, type Service} from '~/lib/domain/providers';
import {STOP_PIN_PIXEL_RATIO, stopDotImage, stopPinImage} from '~/lib/map/images/stopPin';

export type StopsSource = 'stops' | 'route-stops';

export interface StopsMarks {
  selectedId: string | null;
  siblings: readonly string[]; // any stop of the shelter
  dim: boolean;
}

const PIN_MIN_ZOOM = 15; // dot -> pin

const OPACITY: ExpressionSpecification = ['case', ['get', 'dim'], 0.45, ['get', 'muted'], 0.75, 1];
const ORDER: ExpressionSpecification = ['case', ['get', 'selected'], 1, 0];

const imageId = (kind: 'pin' | 'dot', combo: readonly Service[], selected: boolean) =>
  `stop-${kind}-${comboKey(combo)}${selected ? '-selected' : ''}`;

const imageExpr = (kind: 'pin' | 'dot'): ExpressionSpecification => [
  'concat',
  `stop-${kind}-`,
  ['coalesce', ['get', 'service'], 'urban'],
  ['case', ['get', 'selected'], '-selected', ''],
];

export const stopsLayer = {
  HITS: ['route-stops-pin', 'route-stops-dot', 'stops-pin', 'stops-dot'],

  addImages(map: MapLibreMap) {
    for (const combo of SERVICE_COMBOS) {
      for (const selected of [false, true]) {
        const pin = imageId('pin', combo, selected);
        const dot = imageId('dot', combo, selected);
        if (!map.hasImage(pin)) {
          const image = stopPinImage(selected, combo, selected ? 92 : 80);
          if (image) map.addImage(pin, image, {pixelRatio: STOP_PIN_PIXEL_RATIO});
        }
        if (!map.hasImage(dot)) {
          const image = stopDotImage(selected, combo);
          if (image) map.addImage(dot, image, {pixelRatio: STOP_PIN_PIXEL_RATIO});
        }
      }
    }
  },

  add(map: MapLibreMap, source: StopsSource) {
    map.addSource(source, {type: 'geojson', data: features([], {selectedId: null, siblings: [], dim: false})});
    map.addLayer({
      id: `${source}-dot`,
      type: 'symbol',
      source,
      maxzoom: PIN_MIN_ZOOM,
      layout: {
        'icon-image': imageExpr('dot'),
        'icon-size': ['interpolate', ['linear'], ['zoom'], 12, 0.7, PIN_MIN_ZOOM, 1],
        'icon-allow-overlap': true,
        'icon-ignore-placement': true,
        'symbol-sort-key': ORDER,
      },
      paint: {'icon-opacity': OPACITY},
    });
    map.addLayer({
      id: `${source}-pin`,
      type: 'symbol',
      source,
      minzoom: PIN_MIN_ZOOM,
      layout: {
        'icon-image': imageExpr('pin'),
        'icon-size': ['interpolate', ['linear'], ['zoom'], PIN_MIN_ZOOM, 0.78, 18, 1],
        'icon-allow-overlap': true,
        'icon-ignore-placement': true,
        'symbol-sort-key': ORDER,
      },
      paint: {'icon-opacity': OPACITY},
    });
  },

  set(map: MapLibreMap, source: StopsSource, stops: StopItem[], marks: StopsMarks) {
    (map.getSource(source) as GeoJSONSource | undefined)?.setData(features(stops, marks));
  },
};

// backend serves one point per shelter
function features(stops: StopItem[], marks: StopsMarks): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: stops.map((stop) => {
      const services = new Set(
        stop.providers ? stop.providers.map((p) => providerInfo(p).service) : [serviceOf(stop.id)]
      );
      const selected = stop.id === marks.selectedId || marks.siblings.includes(stop.id);
      return {
        type: 'Feature' as const,
        geometry: {type: 'Point' as const, coordinates: [stop.lng, stop.lat]},
        properties: {
          stop_id: stop.id,
          stop_name: stop.name,
          service: comboKey(SERVICES.filter((s) => services.has(s))),
          dim: marks.dim && !selected,
          muted: marks.selectedId != null && !selected,
          selected,
        },
      };
    }),
  };
}
