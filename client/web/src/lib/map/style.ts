import {layers, namedFlavor} from '@protomaps/basemaps';
import type {StyleSpecification} from 'maplibre-gl';

const ASSETS = 'https://protomaps.github.io/basemaps-assets';

// range requests
export function basemapStyle(dark: boolean, regionId: string): StyleSpecification {
  const flavor = dark ? 'dark' : 'light';

  return {
    version: 8,
    glyphs: `${ASSETS}/fonts/{fontstack}/{range}.pbf`,
    sprite: `${ASSETS}/sprites/v4/${flavor}`,
    sources: {
      protomaps: {
        type: 'vector',
        url: `pmtiles:///tiles/${regionId}.pmtiles`,
        attribution:
          '<a href="https://protomaps.com">Protomaps</a> &copy; <a href="https://openstreetmap.org">OpenStreetMap</a>',
      },
    },
    layers: layers('protomaps', namedFlavor(flavor), {lang: 'es'}),
  };
}
