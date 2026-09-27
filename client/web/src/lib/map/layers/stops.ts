import type {FeatureCollection} from 'geojson';
import type {ExpressionSpecification, GeoJSONSource, Map as MapLibreMap} from 'maplibre-gl';
import type {StopItem} from '~/lib/api/types';
import {
  comboKey,
  isTerminalStop,
  providerInfo,
  SERVICE_COMBOS,
  SERVICES,
  serviceOf,
  type Service,
} from '~/lib/domain/providers';
import {STOP_PIN_PIXEL_RATIO, stopDotImage, stopPinImage} from '~/lib/map/images/stopPin';
import {mapText} from '~/lib/map/paint';

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

  // re-baked on theme change
  bakeImages(map: MapLibreMap) {
    const bake = (id: string, image: ImageData | null) => {
      if (!image) return;
      if (map.hasImage(id)) map.updateImage(id, image);
      else map.addImage(id, image, {pixelRatio: STOP_PIN_PIXEL_RATIO});
    };
    for (const combo of SERVICE_COMBOS) {
      for (const selected of [false, true]) {
        bake(imageId('pin', combo, selected), stopPinImage(selected, combo, selected ? 92 : 80));
        bake(imageId('dot', combo, selected), stopDotImage(selected, combo));
      }
    }
  },

  add(map: MapLibreMap, source: StopsSource) {
    const text = mapText();
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
        // platform under the pin
        'text-field': ['get', 'platform'],
        'text-font': ['Noto Sans Medium'],
        'text-size': 12,
        'text-anchor': 'top',
        'text-offset': [0, 1.6],
        'text-allow-overlap': true,
        'text-ignore-placement': true,
      },
      paint: {
        'icon-opacity': OPACITY,
        'text-color': text.color,
        'text-halo-color': text.halo,
        'text-halo-width': 1.5,
        'text-opacity': OPACITY,
      },
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
        geometry: {
          type: 'Point' as const,
          // a regular stop next to a terminal stays at its corner
          coordinates:
            stop.footprint && isTerminalStop(stop.name, stop.station) ? centroid(stop.footprint) : [stop.lng, stop.lat],
        },
        properties: {
          stop_id: stop.id,
          stop_name: stop.name,
          platform: stop.platform ? `A${stop.platform}` : '',
          service: comboKey(SERVICES.filter((s) => services.has(s))),
          dim: marks.dim && !selected,
          muted: marks.selectedId != null && !selected,
          selected,
        },
      };
    }),
  };
}

// pin mid-terminal
function centroid(footprint: number[]): [number, number] {
  let area = 0;
  let x = 0;
  let y = 0;
  for (let i = 0; i + 3 < footprint.length; i += 2) {
    const [x0, y0, x1, y1] = footprint.slice(i, i + 4);
    const cross = x0 * y1 - x1 * y0;
    area += cross;
    x += (x0 + x1) * cross;
    y += (y0 + y1) * cross;
  }
  return area ? [x / (3 * area), y / (3 * area)] : [footprint[0], footprint[1]];
}
