import {layers, namedFlavor} from '@protomaps/basemaps';
import {PUBLIC_TILES_URL} from 'astro:env/client';
import type {StyleSpecification} from 'maplibre-gl';

const ASSETS = 'https://protomaps.github.io/basemaps-assets';

export const ATTRIBUTION =
  '<a href="https://protomaps.com">Protomaps</a> &copy; <a href="https://openstreetmap.org">OpenStreetMap</a>';

// range requests
export function basemapStyle(dark: boolean): StyleSpecification {
  const flavor = dark ? 'dark' : 'light';

  return {
    version: 8,
    glyphs: `${ASSETS}/fonts/{fontstack}/{range}.pbf`,
    sprite: `${ASSETS}/sprites/v4/${flavor}`,
    sources: {
      protomaps: {type: 'vector', url: `pmtiles://${PUBLIC_TILES_URL}`, attribution: ATTRIBUTION},
    },
    layers: layers('protomaps', namedFlavor(flavor), {lang: 'es'}),
  };
}
