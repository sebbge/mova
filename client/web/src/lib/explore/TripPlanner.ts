import type {FeatureCollection} from 'geojson';
import {createEffect, createMemo, createSignal, on, type Accessor, type Setter} from 'solid-js';
import type {Itinerary, TravelMode} from '~/lib/api/types';
import {endpointParam, parseEndpoint, type Endpoint} from '~/lib/trip/endpoint';
import {busPath, tripFeatures} from '~/lib/trip/geometry';
import {LiveBoard} from '~/lib/trip/LiveBoard';
import {fareProfile} from '~/lib/trip/profile';
import {TripFeed} from '~/lib/trip/TripFeed';
import type {MapUrl} from './MapUrl';

type Field = 'a' | 'b';
type Coord = [number, number];

const P = {trip: 'viaje', a: 'a', b: 'b', at: 'sale', mode: 'modo', option: 'opcion'} as const;

// state in the url
export class TripPlanner {
  readonly feed = new TripFeed();
  readonly live = new LiveBoard();

  readonly #searching: Accessor<boolean>;
  readonly #setSearching: Setter<boolean>;
  readonly #searchField: Accessor<Field>;
  readonly #setSearchField: Setter<Field>;
  readonly #pickField: Accessor<Field | null>;
  readonly #setPickField: Setter<Field | null>;
  readonly #center: Accessor<{lat: number; lng: number} | undefined>;
  readonly #setCenter: Setter<{lat: number; lng: number} | undefined>;
  readonly #paths: Accessor<Map<number, Coord[]>>;
  readonly #setPaths: Setter<Map<number, Coord[]>>;
  readonly #drawn: Accessor<Itinerary | null>;

  constructor(
    private readonly url: MapUrl,
    private readonly childOpen: () => boolean
  ) {
    [this.#searching, this.#setSearching] = createSignal(false);
    [this.#searchField, this.#setSearchField] = createSignal<Field>('b');
    [this.#pickField, this.#setPickField] = createSignal<Field | null>(null);
    [this.#center, this.#setCenter] = createSignal<{lat: number; lng: number}>();
    [this.#paths, this.#setPaths] = createSignal(new Map<number, Coord[]>());

    this.#drawn = createMemo(() => {
      if (!this.active) return null;
      const all = this.feed.itineraries;
      return (this.option !== null ? all[this.option] : all[0]) ?? null;
    });

    createEffect(() =>
      this.feed.follow(
        this.active
          ? {
              a: parseEndpoint(this.aRaw),
              b: parseEndpoint(this.bRaw),
              at: this.at,
              mode: this.mode,
              payment: fareProfile.payment,
              category: fareProfile.category,
            }
          : null
      )
    );

    createEffect(() => {
      if (this.active && !this.aRaw) this.url.select({[P.a]: 'aqui'}, true);
    });

    createEffect(
      on(
        () => this.active,
        (active, was) => {
          if (active && !was && !this.bRaw) this.openSearch('b');
        }
      )
    );

    createEffect(() => {
      const itinerary = this.#drawn();
      this.#setPaths(new Map());
      if (!itinerary) return;
      void Promise.all(
        itinerary.legs.map(async (leg, i) => (leg.mode === 'bus' ? ([i, await busPath(leg)] as const) : null))
      ).then((entries) => {
        if (this.#drawn() === itinerary) this.#setPaths(new Map(entries.filter((e) => e !== null)));
      });
    });
  }

  get active(): boolean {
    return this.url.has(P.trip);
  }

  get aRaw(): string | null {
    return this.url.get(P.a);
  }

  get bRaw(): string | null {
    return this.url.get(P.b);
  }

  get at(): string | null {
    return this.url.get(P.at);
  }

  get mode(): TravelMode {
    return this.url.get(P.mode) === 'pie' ? 'walk' : 'transit';
  }

  get option(): number | null {
    const raw = this.url.get(P.option);
    return raw === null || Number.isNaN(Number(raw)) ? null : Number(raw);
  }

  get searching(): boolean {
    return this.#searching();
  }

  get searchField(): Field {
    return this.#searchField();
  }

  get center(): {lat: number; lng: number} | undefined {
    return this.#center();
  }

  get picking(): Field | null {
    if (!this.active || this.searching || this.childOpen()) return null;
    const field = this.#pickField();
    if (field) return field;
    if (!this.aRaw || (this.aRaw === 'aqui' && this.feed.hereFailed)) return 'a';
    if (!this.bRaw) return 'b';
    return null;
  }

  get drawing(): FeatureCollection | null {
    return this.active ? tripFeatures(this.#drawn(), this.#paths(), this.feed.a, this.feed.b) : null;
  }

  get fitKey(): string | null {
    if (!this.active || !(this.feed.a || this.feed.b)) return null;
    const drawn = this.#drawn();
    return `${this.aRaw}|${this.bRaw}|${this.option ?? 'list'}|${drawn?.departure ?? ''}|${drawn ? this.#paths().size : ''}`;
  }

  setCenter(center: {lat: number; lng: number}) {
    this.#setCenter(center);
  }

  openSearch(field: Field = 'b') {
    if (!this.active) this.url.select({[P.trip]: ''});
    this.#setSearchField(field);
    this.#setPickField(null);
    this.#setSearching(true);
  }

  closeSearch() {
    this.#setSearching(false);
  }

  pickOnMap(field: Field) {
    this.#setSearching(false);
    this.#setPickField(field);
  }

  setPoint(field: Field, endpoint: Endpoint) {
    this.#setPickField(null);
    this.url.select({[P[field]]: endpointParam(endpoint), [P.option]: null});
  }

  choose(field: Field, endpoint: Endpoint) {
    this.setPoint(field, endpoint);
    if (field === 'b' || this.bRaw) this.#setSearching(false);
  }

  swap() {
    this.url.select({[P.a]: this.bRaw, [P.b]: this.aRaw, [P.option]: null});
  }

  setAt(at: string | null) {
    this.url.select({[P.at]: at, [P.option]: null});
  }

  setOption(index: number | null) {
    this.url.select({[P.option]: index === null ? null : String(index)});
  }

  close() {
    this.#setSearching(false);
    this.#setPickField(null);
    this.url.select({[P.trip]: null, [P.a]: null, [P.b]: null, [P.at]: null, [P.mode]: null, [P.option]: null});
  }
}
