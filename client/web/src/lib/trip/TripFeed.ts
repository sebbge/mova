import {createSignal, type Accessor, type Setter} from 'solid-js';
import {ApiError, api} from '~/lib/api/client';
import type {Category, Itinerary, Payment, PlaceHit, TravelMode} from '~/lib/api/types';
import {fullStopName} from '~/lib/format';
import type {Endpoint} from './endpoint';

export interface Place {
  lat: number;
  lng: number;
  label: string;
  ref?: string;
}

export type TripStatus = 'idle' | 'locating' | 'loading' | 'ready' | 'empty' | 'preparing' | 'error';

export interface TripInput {
  a: Endpoint | null;
  b: Endpoint | null;
  at: string | null;
  mode: TravelMode;
  payment: Payment;
  category: Category;
}

const POINT_LABEL = 'Punto en el mapa';

// reused for a minute
class Here {
  static readonly MAX_AGE_MS = 60_000;
  static readonly TIMEOUT_MS = 10_000;
  #fix: {lat: number; lng: number; at: number} | null = null;

  locate(): Promise<{lat: number; lng: number}> {
    if (this.#fix && Date.now() - this.#fix.at < Here.MAX_AGE_MS) return Promise.resolve(this.#fix);
    return new Promise((resolve, reject) => {
      if (!('geolocation' in navigator)) {
        reject(new Error('unsupported'));
        return;
      }
      navigator.geolocation.getCurrentPosition(
        (p) => {
          this.#fix = {lat: p.coords.latitude, lng: p.coords.longitude, at: Date.now()};
          resolve(this.#fix);
        },
        (e) => reject(new Error(e.code === e.PERMISSION_DENIED ? 'denied' : 'unavailable')),
        {enableHighAccuracy: true, timeout: Here.TIMEOUT_MS, maximumAge: Here.MAX_AGE_MS}
      );
    });
  }
}

const here = new Here();

export class TripFeed {
  readonly #a: Accessor<Place | null>;
  readonly #setA: Setter<Place | null>;
  readonly #b: Accessor<Place | null>;
  readonly #setB: Setter<Place | null>;
  readonly #hereFailed: Accessor<boolean>;
  readonly #setHereFailed: Setter<boolean>;
  readonly #itineraries: Accessor<Itinerary[]>;
  readonly #setItineraries: Setter<Itinerary[]>;
  readonly #status: Accessor<TripStatus>;
  readonly #setStatus: Setter<TripStatus>;
  #key = '';
  #inflight: AbortController | undefined;

  constructor() {
    [this.#a, this.#setA] = createSignal<Place | null>(null);
    [this.#b, this.#setB] = createSignal<Place | null>(null);
    [this.#hereFailed, this.#setHereFailed] = createSignal(false);
    [this.#itineraries, this.#setItineraries] = createSignal<Itinerary[]>([]);
    [this.#status, this.#setStatus] = createSignal<TripStatus>('idle');
  }

  get a(): Place | null {
    return this.#a();
  }

  get b(): Place | null {
    return this.#b();
  }

  get hereFailed(): boolean {
    return this.#hereFailed();
  }

  get itineraries(): Itinerary[] {
    return this.#itineraries();
  }

  get status(): TripStatus {
    return this.#status();
  }

  follow(input: TripInput | null) {
    const key = JSON.stringify(input);
    if (key === this.#key) return;
    this.#key = key;
    this.#inflight?.abort();
    void this.#follow(input);
  }

  retry() {
    const key = this.#key;
    this.#key = '';
    this.follow(key ? (JSON.parse(key) as TripInput) : null);
  }

  async #follow(input: TripInput | null) {
    if (!input) {
      this.#setA(null);
      this.#setB(null);
      this.#setItineraries([]);
      this.#setStatus('idle');
      return;
    }

    const controller = new AbortController();
    this.#inflight = controller;
    const current = () => !controller.signal.aborted;

    this.#setStatus(input.a?.kind === 'here' || input.b?.kind === 'here' ? 'locating' : 'loading');
    const [a, b] = await Promise.all([this.#place(input.a), this.#place(input.b)]);
    if (!current()) return;
    this.#setA(a);
    this.#setB(b);
    this.#name(this.#a, this.#setA, input.a, controller);
    this.#name(this.#b, this.#setB, input.b, controller);

    if (!a || !b) {
      this.#setItineraries([]);
      this.#setStatus('idle');
      return;
    }

    this.#setStatus('loading');
    try {
      const found = await api.plan(
        {
          from: `${a.lat.toFixed(6)},${a.lng.toFixed(6)}`,
          to: `${b.lat.toFixed(6)},${b.lng.toFixed(6)}`,
          at: input.at ?? undefined,
          payment: input.payment,
          category: input.category,
          mode: input.mode,
        },
        {signal: controller.signal}
      );
      if (!current()) return;
      this.#setItineraries(found);
      this.#setStatus(found.length > 0 ? 'ready' : 'empty');
    } catch (e) {
      if (!current()) return;
      this.#setItineraries([]);
      this.#setStatus(e instanceof ApiError && e.status === 503 ? 'preparing' : 'error');
    }
  }

  async #place(endpoint: Endpoint | null): Promise<Place | null> {
    if (!endpoint) return null;
    try {
      const place = await resolve(endpoint);
      if (endpoint.kind === 'here') this.#setHereFailed(false);
      return place;
    } catch {
      if (endpoint.kind === 'here') this.#setHereFailed(true);
      return null;
    }
  }

  // the trip doesn't wait for the name
  #name(
    get: Accessor<Place | null>,
    set: Setter<Place | null>,
    endpoint: Endpoint | null,
    controller: AbortController
  ) {
    const place = get();
    if (endpoint?.kind !== 'point' || !place) return;
    api
      .reversePlace(place)
      .then((near) => {
        if (!controller.signal.aborted && get() === place) set({...place, label: pointLabel(near)});
      })
      .catch(() => {});
  }
}

async function resolve(endpoint: Endpoint): Promise<Place> {
  switch (endpoint.kind) {
    case 'here':
      return {...(await here.locate()), label: 'Mi ubicación'};
    case 'point':
      return {lat: endpoint.lat, lng: endpoint.lng, label: POINT_LABEL};
    case 'stop': {
      const stop = await api.stop(endpoint.ref);
      return {lat: stop.lat, lng: stop.lng, label: fullStopName(stop.stop_name, stop.station), ref: endpoint.ref};
    }
    case 'place': {
      const place = await api.place(endpoint.id);
      return {lat: place.lat, lng: place.lng, label: place.name, ref: endpoint.id};
    }
  }
}

function pointLabel(near: PlaceHit | null): string {
  if (!near) return POINT_LABEL;
  return near.kind === 'corner' ? near.name : `${POINT_LABEL}, ${near.name}`;
}
