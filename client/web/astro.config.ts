import solid from '@astrojs/solid-js';
import {defineConfig, envField} from 'astro/config';

export default defineConfig({
  output: 'static',
  outDir: '../../.artifact/web',
  trailingSlash: 'never',
  build: {format: 'file', concurrency: 16},
  server: {port: 5173},
  integrations: [solid()],
  env: {
    schema: {
      PUBLIC_API_URL: envField.string({context: 'client', access: 'public'}),
      PUBLIC_SITE_URL: envField.string({context: 'client', access: 'public'}),
      PUBLIC_TILES_URL: envField.string({context: 'client', access: 'public'}),
    },
  },
  vite: {
    optimizeDeps: {exclude: ['maplibre-gl']}, // prebundling 404s the maplibre worker
    worker: {format: 'es'}, // maplibre loads its worker as a module
  },
});
