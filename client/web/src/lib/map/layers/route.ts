import type {FeatureCollection} from 'geojson';
import type {GeoJSONSource, Map as MapLibreMap} from 'maplibre-gl';
import type {RouteShape} from '~/lib/api/types';
import {chevronImage} from '~/lib/map/images/chevron';

const SOURCE = 'route';
const CHEVRON = 'route-chevron';

// chevrons as the stroke
export const routeLayer = {
  add(map: MapLibreMap) {
    if (map.getSource(SOURCE)) return;

    const chevron = chevronImage();
    if (chevron && !map.hasImage(CHEVRON)) map.addImage(CHEVRON, chevron);

    map.addSource(SOURCE, {type: 'geojson', data: features(null)});
    map.addLayer({
      id: 'route-chevrons',
      type: 'symbol',
      source: SOURCE,
      layout: {
        'symbol-placement': 'line',
        'symbol-spacing': ['interpolate', ['linear'], ['zoom'], 11, 12, 14, 15, 17, 18],
        'icon-image': CHEVRON,
        'icon-size': ['interpolate', ['linear'], ['zoom'], 11, 0.55, 14, 0.8, 17, 1.05],
        'icon-rotation-alignment': 'map',
        'icon-allow-overlap': true,
        'icon-ignore-placement': true,
      },
    });
  },

  set(map: MapLibreMap, shape: RouteShape | null) {
    (map.getSource(SOURCE) as GeoJSONSource | undefined)?.setData(features(shape));
  },
};

function features(shape: RouteShape | null): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: shape ? [{type: 'Feature', geometry: shape.shape, properties: {}}] : [],
  };
}
