import type {Map as MapLibreMap} from 'maplibre-gl';
import {createSignal, onCleanup, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {NearbyStop} from '~/lib/api/types';

export class NearbyStops {
  static readonly MIN_ZOOM = 15; // below this, too many stops to read
  static readonly RADIUS = 1000; // m
  static readonly DEBOUNCE_MS = 400;

  readonly #stops: Accessor<NearbyStop[]>;
  readonly #setStops: Setter<NearbyStop[]>;
  readonly #zoomedOut: Accessor<boolean>;
  readonly #setZoomedOut: Setter<boolean>;
  #timer: ReturnType<typeof setTimeout> | undefined;
  #inflight: AbortController | undefined;

  constructor() {
    [this.#stops, this.#setStops] = createSignal<NearbyStop[]>([]);
    [this.#zoomedOut, this.#setZoomedOut] = createSignal(false);
    onCleanup(() => {
      clearTimeout(this.#timer);
      this.#inflight?.abort();
    });
  }

  get stops(): NearbyStop[] {
    return this.#stops();
  }

  get zoomedOut(): boolean {
    return this.#zoomedOut();
  }

  refreshSoon(map: MapLibreMap) {
    clearTimeout(this.#timer);
    this.#timer = setTimeout(() => void this.refresh(map), NearbyStops.DEBOUNCE_MS);
  }

  async refresh(map: MapLibreMap) {
    const zoomedOut = map.getZoom() < NearbyStops.MIN_ZOOM;
    this.#setZoomedOut(zoomedOut);
    if (zoomedOut) {
      this.#setStops([]);
      return;
    }

    const {lat, lng} = map.getCenter();
    this.#inflight?.abort();
    const controller = new AbortController();
    this.#inflight = controller;

    try {
      this.#setStops(await api.nearbyStops(lat, lng, NearbyStops.RADIUS, {signal: controller.signal}));
    } catch {
      // keeps the last frame
    }
  }
}
