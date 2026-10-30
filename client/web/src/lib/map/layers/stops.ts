import type {FeatureCollection} from 'geojson';
import type {ExpressionSpecification, GeoJSONSource, Map as MapLibreMap, SymbolLayerSpecification} from 'maplibre-gl';
import type {StopItem} from '~/lib/api/types';
import {comboKey, isTerminalStop, SERVICE_COMBOS, stopServices, type Service} from '~/lib/domain/providers';
import {STOP_PIN_PIXEL_RATIO, stopDotImage, stopPinImage} from '~/lib/map/images/stopPin';
import {mapText} from '~/lib/map/paint';

export interface StopsMarks {
  selectedId: string | null;
  siblings: readonly string[]; // any stop of the shelter
  dim: boolean;
}

const SOURCE = 'stops';
const PIN_MIN_ZOOM = 15; // dot -> pin

// off route, or another one selected
export const STOP_DIM_OPACITY = 0.45;
export const STOP_OPACITY: ExpressionSpecification = [
  'case',
  ['any', ['get', 'dim'], ['get', 'muted']],
  STOP_DIM_OPACITY,
  1,
];
const ORDER: ExpressionSpecification = ['case', ['get', 'selected'], 1, 0];

const imageId = (kind: 'pin' | 'dot', combo: readonly Service[], selected: boolean) =>
  `stop-${kind}-${comboKey(combo)}${selected ? '-selected' : ''}`;

const imageExpr = (kind: 'pin' | 'dot'): ExpressionSpecification => [
  'concat',
  `stop-${kind}-`,
  ['coalesce', ['get', 'service'], 'urban'],
  ['case', ['get', 'selected'], '-selected', ''],
];

// off-route stops only
export const stopsLayer = {
  HITS: ['stops-selected', 'stops-pin', 'stops-dot'],

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

  add(map: MapLibreMap) {
    const text = mapText();
    map.addSource(SOURCE, {type: 'geojson', data: features([], {selectedId: null, siblings: [], dim: false})});
    map.addLayer({
      id: 'stops-dot',
      type: 'symbol',
      source: SOURCE,
      maxzoom: PIN_MIN_ZOOM,
      filter: ['!', ['get', 'selected']],
      layout: {
        'icon-image': imageExpr('dot'),
        // under 12.75 they shrink faster
        'icon-size': ['interpolate', ['linear'], ['zoom'], 11, 0.45, 12.75, 0.775, PIN_MIN_ZOOM, 1],
        'icon-allow-overlap': true,
        'icon-ignore-placement': true,
        'symbol-sort-key': ORDER,
      },
      paint: {'icon-opacity': STOP_OPACITY},
    });

    const pinLayout = {
      'icon-image': imageExpr('pin'),
      'icon-allow-overlap': true,
      'icon-ignore-placement': true,
      // platform under the pin
      'text-field': ['get', 'platform'],
      'text-font': ['Noto Sans Medium'],
      'text-size': 12,
      'text-anchor': 'top',
      'text-offset': [0, 1.6],
      'text-allow-overlap': true,
      'text-ignore-placement': true,
    } satisfies SymbolLayerSpecification['layout'];
    const pinPaint = {
      'icon-opacity': STOP_OPACITY,
      'text-color': text.color,
      'text-halo-color': text.halo,
      'text-halo-width': 1.5,
      'text-opacity': STOP_OPACITY,
    } satisfies SymbolLayerSpecification['paint'];

    map.addLayer({
      id: 'stops-pin',
      type: 'symbol',
      source: SOURCE,
      minzoom: PIN_MIN_ZOOM,
      filter: ['!', ['get', 'selected']],
      layout: {
        ...pinLayout,
        // never past 1, smears
        'icon-size': [
          'interpolate',
          ['linear'],
          ['zoom'],
          PIN_MIN_ZOOM,
          ['case', ['get', 'muted'], 0.7, 0.78],
          18,
          ['case', ['get', 'muted'], 0.9, 1],
        ],
        'symbol-sort-key': ORDER,
      },
      paint: pinPaint,
    });

    // selected is a pin at any zoom
    map.addLayer({
      id: 'stops-selected',
      type: 'symbol',
      source: SOURCE,
      filter: ['get', 'selected'],
      layout: {
        ...pinLayout,
        'icon-size': ['interpolate', ['linear'], ['zoom'], 11, 0.55, PIN_MIN_ZOOM, 0.78, 18, 1],
      },
      paint: pinPaint,
    });
  },

  set(map: MapLibreMap, stops: StopItem[], marks: StopsMarks) {
    (map.getSource(SOURCE) as GeoJSONSource | undefined)?.setData(features(stops, marks));
  },
};

// backend serves one point per shelter
function features(stops: StopItem[], marks: StopsMarks): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: stops.map((stop) => {
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
          service: comboKey(stopServices(stop)),
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
