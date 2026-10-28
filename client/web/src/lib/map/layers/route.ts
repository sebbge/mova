import type {FeatureCollection} from 'geojson';
import type {ExpressionSpecification, GeoJSONSource, Map as MapLibreMap} from 'maplibre-gl';
import type {RouteShape} from '~/lib/api/types';
import {CHEVRON_PIXEL_RATIO, chevronImage} from '~/lib/map/images/chevron';
import {routeTerminalImage, STOP_PIN_PIXEL_RATIO} from '~/lib/map/images/stopPin';
import {outline, routeColor} from '~/lib/map/paint';
import type {RouteTrack} from '~/lib/map/RouteTrack';
import {metersPerPixel} from '~/lib/map/viewport';

const CHEVRON = 'route-chevron';
const EMPTY: FeatureCollection = {type: 'FeatureCollection', features: []};

// under 12.75 the dots shrink
const ZOOMS = [11, 12.75, 14, 17];
// px, bigger than the band (half width 3, 5.5, 9)
const STOP = {radius: [2.5, 4.67, 5.5, 8.5], stroke: [1, 2.17, 3, 5.5]};
const STOP_SELECTED = {radius: [4.5, 5.67, 6.5, 9.5], stroke: [1, 2.17, 3, 5.5]};
const TERMINAL_HALF = [6, 9, 11, 16]; // px, half side, the image is 32

const ORDER: ExpressionSpecification = ['case', ['get', 'selected'], 1, 0];

export interface RouteView {
  shape: RouteShape | null;
  track: RouteTrack | null;
  isSelected: (stopId: string) => boolean;
}

// band, stop dots, O/D squares
export const routeLayer = {
  HITS: ['route-terminals', 'route-stops-circle'],

  addSources(map: MapLibreMap) {
    const chevron = chevronImage();
    if (chevron && !map.hasImage(CHEVRON)) map.addImage(CHEVRON, chevron, {pixelRatio: CHEVRON_PIXEL_RATIO});
    for (const id of ['route', 'route-sections', 'route-stops']) map.addSource(id, {type: 'geojson', data: EMPTY});
  },

  addLayers(map: MapLibreMap, company: string | undefined) {
    const color = routeColor(company);
    map.addLayer({
      id: 'route-line',
      type: 'line',
      source: 'route',
      layout: {'line-cap': 'round', 'line-join': 'round'},
      paint: {
        'line-color': color,
        'line-width': ['interpolate', ['linear'], ['zoom'], 11, 6, 14, 11, 17, 18],
      },
    });
    // per section, chevrons never land on a dot
    map.addLayer({
      id: 'route-chevrons',
      type: 'symbol',
      source: 'route-sections',
      layout: {
        'symbol-placement': 'line',
        'symbol-spacing': ['interpolate', ['linear'], ['zoom'], 11, 5, 14, 9, 17, 14],
        'icon-image': CHEVRON,
        'icon-size': ['interpolate', ['linear'], ['zoom'], 11, 0.35, 14, 0.6, 17, 1],
        'icon-rotation-alignment': 'map',
        'icon-allow-overlap': true,
        'icon-ignore-placement': true,
      },
    });
    map.addLayer({
      id: 'route-stops-circle',
      type: 'circle',
      source: 'route-stops',
      filter: ['==', ['get', 'terminal'], ''],
      layout: {'circle-sort-key': ORDER},
      paint: {
        'circle-radius': byZoom('radius'),
        'circle-stroke-width': byZoom('stroke'),
        ...stopColors(color),
      },
    });
    bakeTerminals(map, color);
    map.addLayer({
      id: 'route-terminals',
      type: 'symbol',
      source: 'route-stops',
      filter: ['!=', ['get', 'terminal'], ''],
      layout: {
        'icon-image': ['concat', 'route-terminal-', ['get', 'terminal'], ['case', ['get', 'selected'], '-sel', '']],
        'icon-size': [
          'interpolate',
          ['linear'],
          ['zoom'],
          ...ZOOMS.flatMap((z, i) => [z, TERMINAL_HALF[i] / 16]),
        ] as ExpressionSpecification,
        'icon-allow-overlap': true,
        'icon-ignore-placement': true,
        'symbol-sort-key': ORDER,
      },
    });
  },

  paint(map: MapLibreMap, company: string | undefined) {
    if (!map.getLayer('route-line')) return;
    const color = routeColor(company);
    map.setPaintProperty('route-line', 'line-color', color);
    const colors = stopColors(color);
    map.setPaintProperty('route-stops-circle', 'circle-color', colors['circle-color']);
    map.setPaintProperty('route-stops-circle', 'circle-stroke-color', colors['circle-stroke-color']);
    bakeTerminals(map, color);
  },

  set(map: MapLibreMap, view: RouteView) {
    const {shape, track, isSelected} = view;
    const stops = shape?.stops ?? [];
    source(map, 'route')?.setData({
      type: 'FeatureCollection',
      features: shape?.shape ? [{type: 'Feature', geometry: shape.shape, properties: {}}] : [],
    });
    source(map, 'route-stops')?.setData({
      type: 'FeatureCollection',
      features: stops.map((stop, i) => ({
        type: 'Feature' as const,
        geometry: {type: 'Point' as const, coordinates: track?.points[i] ?? [stop.lng, stop.lat]},
        properties: {
          stop_id: stop.stop_id,
          stop_name: stop.stop_name,
          selected: isSelected(stop.stop_id),
          terminal: terminalOf(i, stops.length),
        },
      })),
    });
  },

  // only on half zoom steps
  cut(map: MapLibreMap, view: RouteView, zoom: number) {
    const {shape, track, isSelected} = view;
    const stops = shape?.stops ?? [];
    if (!track) {
      source(map, 'route-sections')?.setData(EMPTY);
      return;
    }
    const mpp = metersPerPixel(zoom, track.points[0]?.[1] ?? 0);
    const margin = (i: number) => clearance(zoom, isSelected(stops[i].stop_id), terminalOf(i, stops.length) !== '');
    source(map, 'route-sections')?.setData(track.sections((i) => margin(i) * mpp));
  },
};

function source(map: MapLibreMap, id: string): GeoJSONSource | undefined {
  return map.getSource(id) as GeoJSONSource | undefined;
}

function terminalOf(i: number, n: number): '' | 'O' | 'D' {
  return n < 2 ? '' : i === 0 ? 'O' : i === n - 1 ? 'D' : '';
}

// linear over ZOOMS, like the layers
function atZoom(values: number[], zoom: number): number {
  const z = Math.min(ZOOMS[ZOOMS.length - 1], Math.max(ZOOMS[0], zoom));
  let i = 0;
  while (i < ZOOMS.length - 2 && z > ZOOMS[i + 1]) i++;
  return values[i] + ((values[i + 1] - values[i]) * (z - ZOOMS[i])) / (ZOOMS[i + 1] - ZOOMS[i]);
}

// px free around a stop
function clearance(zoom: number, selected: boolean, terminal: boolean): number {
  if (terminal) return atZoom(TERMINAL_HALF, zoom);
  const m = selected ? STOP_SELECTED : STOP;
  return atZoom(m.radius, zoom) + atZoom(m.stroke, zoom);
}

function byZoom(key: 'radius' | 'stroke'): ExpressionSpecification {
  return [
    'interpolate',
    ['linear'],
    ['zoom'],
    ...ZOOMS.flatMap((z, i) => [z, ['case', ['get', 'selected'], STOP_SELECTED[key][i], STOP[key][i]]]),
  ] as ExpressionSpecification;
}

// selected inverted
function stopColors(color: string) {
  return {
    'circle-color': ['case', ['get', 'selected'], color, outline()] as ExpressionSpecification,
    'circle-stroke-color': ['case', ['get', 'selected'], outline(), color] as ExpressionSpecification,
  };
}

// color baked in
function bakeTerminals(map: MapLibreMap, color: string) {
  for (const letter of ['O', 'D'] as const) {
    for (const selected of [false, true]) {
      const id = `route-terminal-${letter}${selected ? '-sel' : ''}`;
      const image = routeTerminalImage(letter, color, selected);
      if (!image) continue;
      if (map.hasImage(id)) map.updateImage(id, image);
      else map.addImage(id, image, {pixelRatio: STOP_PIN_PIXEL_RATIO});
    }
  }
}
