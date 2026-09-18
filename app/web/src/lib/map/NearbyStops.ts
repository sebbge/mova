import type {FeatureCollection} from 'geojson';
import type {GeoJSONSource, Map as MapLibreMap} from 'maplibre-gl';
import {createSignal, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {NearbyStop} from '~/lib/api/types';

// geojson, dom markers kill panning at this volume
export class NearbyStops {
  static readonly MIN_ZOOM = 15;
  static readonly RADIUS = 1000; // m
  static readonly DEBOUNCE_MS = 400;
  static readonly SOURCE = 'stops';
  static readonly HIT_LAYER = 'stops-halo';

  readonly #zoomedOut: Accessor<boolean>;
  readonly #setZoomedOut: Setter<boolean>;
  readonly #failed: Accessor<boolean>;
  readonly #setFailed: Setter<boolean>;
  #timer: ReturnType<typeof setTimeout> | undefined;
  #inflight: AbortController | undefined;

  constructor(private readonly map: MapLibreMap) {
    [this.#zoomedOut, this.#setZoomedOut] = createSignal(false);
    [this.#failed, this.#setFailed] = createSignal(false);
  }

  get zoomedOut() {
    return this.#zoomedOut();
  }

  get failed() {
    return this.#failed();
  }

  addLayers() {
    if (this.map.getSource(NearbyStops.SOURCE)) return;

    this.map.addSource(NearbyStops.SOURCE, {type: 'geojson', data: features([])});
    this.map.addLayer({
      id: NearbyStops.HIT_LAYER,
      type: 'circle',
      source: NearbyStops.SOURCE,
      paint: {
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 15, 6, 18, 11],
        'circle-color': '#ffffff',
        'circle-stroke-width': 1,
        'circle-stroke-color': 'rgba(0,0,0,0.25)',
      },
    });
    this.map.addLayer({
      id: 'stops-dot',
      type: 'circle',
      source: NearbyStops.SOURCE,
      paint: {
        'circle-radius': ['interpolate', ['linear'], ['zoom'], 15, 3, 18, 6],
        'circle-color': '#00b360',
      },
    });
  }

  refreshSoon() {
    clearTimeout(this.#timer);
    this.#timer = setTimeout(() => void this.refresh(), NearbyStops.DEBOUNCE_MS);
  }

  async refresh() {
    const zoomedOut = this.map.getZoom() < NearbyStops.MIN_ZOOM;
    this.#setZoomedOut(zoomedOut);
    if (zoomedOut) {
      this.#show([]);
      return;
    }

    const {lat, lng} = this.map.getCenter();
    this.#inflight?.abort();
    const controller = new AbortController();
    this.#inflight = controller;

    try {
      this.#show(await api.nearbyStops(lat, lng, NearbyStops.RADIUS, {signal: controller.signal}));
      this.#setFailed(false);
    } catch {
      if (!controller.signal.aborted) this.#setFailed(true);
    }
  }

  close() {
    clearTimeout(this.#timer);
    this.#inflight?.abort();
  }

  #show(stops: NearbyStop[]) {
    (this.map.getSource(NearbyStops.SOURCE) as GeoJSONSource | undefined)?.setData(features(stops));
  }
}

function features(stops: NearbyStop[]): FeatureCollection {
  return {
    type: 'FeatureCollection',
    features: stops.map((s) => ({
      type: 'Feature',
      geometry: {type: 'Point', coordinates: [s.lng, s.lat]},
      properties: {stop_id: s.stop_id, stop_name: s.stop_name},
    })),
  };
}
