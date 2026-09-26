import type {FeatureCollection} from 'geojson';
import type {GeoJSONSource, Map as MapLibreMap} from 'maplibre-gl';
import type {RouteShape} from '~/lib/api/types';
import {CHEVRON_PIXEL_RATIO, chevronImage} from '~/lib/map/images/chevron';
import {routeColor} from '~/lib/map/paint';

const SOURCE = 'route';
const CHEVRON = 'route-chevron';

export const routeLayer = {
  addSource(map: MapLibreMap) {
    const chevron = chevronImage();
    if (chevron && !map.hasImage(CHEVRON)) map.addImage(CHEVRON, chevron, {pixelRatio: CHEVRON_PIXEL_RATIO});
    map.addSource(SOURCE, {type: 'geojson', data: features(null)});
  },

  addLayers(map: MapLibreMap, company: string | undefined) {
    map.addLayer({
      id: 'route-line',
      type: 'line',
      source: SOURCE,
      layout: {'line-cap': 'round', 'line-join': 'round'},
      paint: {
        'line-color': routeColor(company),
        'line-width': ['interpolate', ['linear'], ['zoom'], 11, 6, 14, 11, 17, 18],
      },
    });
    map.addLayer({
      id: 'route-chevrons',
      type: 'symbol',
      source: SOURCE,
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
  },

  paint(map: MapLibreMap, company: string | undefined) {
    if (map.getLayer('route-line')) map.setPaintProperty('route-line', 'line-color', routeColor(company));
  },

  set(map: MapLibreMap, shape: RouteShape | null) {
    (map.getSource(SOURCE) as GeoJSONSource | undefined)?.setData(features(shape));
  },
};

function features(shape: RouteShape | null): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: shape?.shape ? [{type: 'Feature', geometry: shape.shape, properties: {}}] : [],
  };
}
