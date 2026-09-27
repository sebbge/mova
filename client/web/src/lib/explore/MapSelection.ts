import {createEffect, createSignal, onCleanup, type Accessor, type Setter} from 'solid-js';
import type {Bus} from '~/lib/api/types';
import {RegionBuses} from '~/lib/bus/RegionBuses';
import {hasLines, providerOf} from '~/lib/domain/providers';
import {regionsShowingAllBuses} from '~/lib/domain/regions';
import {LineFeed} from '~/lib/line/LineFeed';
import {StopFeed} from '~/lib/stop/StopFeed';
import type {MapUrl} from './MapUrl';

const P = {stop: 'parada', line: 'linea', variant: 'v', direction: 'd', shape: 's', bus: 'bus'} as const;
const LINE_CLEARED = {[P.line]: null, [P.variant]: null, [P.direction]: null, [P.shape]: null};

// self-corrections use replace, or back loops
export class MapSelection {
  readonly stops = new StopFeed();
  readonly line = new LineFeed();
  readonly regionBuses = new RegionBuses();

  readonly #directionHint: Accessor<string | null>; // tapped arrival's destination
  readonly #setDirectionHint: Setter<string | null>;
  readonly #lineCompany: Accessor<string | undefined>; // not in the url
  readonly #setLineCompany: Setter<string | undefined>;
  readonly #knownName: Accessor<string | undefined>; // name the picker had
  readonly #setKnownName: Setter<string | undefined>;
  readonly #busFromStop: Accessor<boolean>;
  readonly #setBusFromStop: Setter<boolean>;
  readonly #lineFromBus: Accessor<string | null>;
  readonly #setLineFromBus: Setter<string | null>;
  readonly #pinnedBus: Accessor<string | null>; // reset when the open bus changes
  readonly #setPinnedBus: Setter<string | null>;
  readonly #liveRegions: Accessor<string>; // stable string
  readonly #setLiveRegions: Setter<string>;

  constructor(private readonly url: MapUrl) {
    [this.#directionHint, this.#setDirectionHint] = createSignal<string | null>(null);
    [this.#lineCompany, this.#setLineCompany] = createSignal<string>();
    [this.#knownName, this.#setKnownName] = createSignal<string>();
    [this.#busFromStop, this.#setBusFromStop] = createSignal(false);
    [this.#lineFromBus, this.#setLineFromBus] = createSignal<string | null>(null);
    [this.#liveRegions, this.#setLiveRegions] = createSignal('');
    [this.#pinnedBus, this.#setPinnedBus] = createSignal<string | null>(null);

    createEffect(() => onCleanup(this.regionBuses.follow(this.#liveRegions()) ?? (() => {})));

    createEffect(() => this.stops.follow(this.stopId));
    onCleanup(() => this.stops.close());
    createEffect(() => onCleanup(this.line.followVariants(this.lineId) ?? (() => {})));
    createEffect(() =>
      onCleanup(this.line.followBuses(this.routeId ? providerOf(this.routeId) : null, this.lineId) ?? (() => {}))
    );
    createEffect(() => {
      const shapeRef = this.shapeRef;
      const stop = shapeRef
        ? this.line.followVariant(shapeRef, () => this.url.select({[P.shape]: null}, true))
        : this.line.followShape(this.routeId, this.requestedDirection);
      onCleanup(stop ?? (() => {}));
    });

    // exact shape -> route and direction
    createEffect(() => {
      const shape = this.line.shape;
      const shapeRef = this.shapeRef;
      if (!this.lineId || this.routeId || !shapeRef || !shape) return;
      if (`${providerOf(shape.route_id)}-${shape.shape_id}` !== shapeRef) return;
      this.url.select({[P.variant]: shape.route_id, [P.direction]: String(shape.direction_id)}, true);
    });

    // stop's provider variant, else first
    createEffect(() => {
      const variants = this.line.variants;
      if (!this.lineId || this.routeId || this.shapeRef || variants.length === 0) return;
      const stopProvider = this.stopId ? providerOf(this.stopId) : null;
      const same = variants.find((v) => providerOf(v.route_id) === stopProvider);
      this.url.select({[P.variant]: (same ?? variants[0]).route_id}, true);
    });

    createEffect(() => {
      const shape = this.line.shape;
      const hint = this.#directionHint()?.toUpperCase();
      if (!hint || !shape || !this.routeId) return;
      this.#setDirectionHint(null);
      const match = shape.available_directions.find((d) => d.headsign?.toUpperCase() === hint);
      if (match && match.direction_id !== shape.direction_id) {
        this.url.select({[P.direction]: String(match.direction_id)}, true);
      }
    });
  }

  get stopId(): string | null {
    return this.url.get(P.stop);
  }

  get lineId(): string | null {
    return this.url.get(P.line);
  }

  get routeId(): string | null {
    return this.url.get(P.variant);
  }

  // without d the server decides (one-way routes)
  get requestedDirection(): number | undefined {
    const d = this.url.get(P.direction);
    return d === null ? undefined : Number(d);
  }

  get busId(): string | null {
    return this.url.get(P.bus);
  }

  get followBusId(): string | null {
    const pinned = this.#pinnedBus();
    return pinned !== null && pinned === this.busId ? pinned : null;
  }

  get shapeRef(): string | null {
    return this.url.get(P.shape);
  }

  get knownName(): string | undefined {
    return this.#knownName();
  }

  get lineCompany(): string | undefined {
    return this.#lineCompany() ?? this.line.buses[0]?.company;
  }

  // avoids flashing the other direction
  get settlingLine(): boolean {
    const shape = this.line.shape;
    if (!shape || this.#directionHint() !== null) return true;
    const requested = this.requestedDirection;
    if (requested === undefined) return false;
    const exists = shape.available_directions.some((d) => d.direction_id === requested);
    return exists && shape.direction_id !== requested;
  }

  get selectedBus(): Bus | null {
    const id = this.busId;
    if (!id) return null;
    return (
      this.#lineBuses().find((b) => b.id === id) ??
      this.stops.buses.find((b) => b.id === id) ??
      this.regionBuses.buses.find((b) => b.id === id) ??
      null
    );
  }

  // selection first, region buses behind
  get buses(): Bus[] {
    const own = this.#lineBuses();
    const selected = this.selectedBus;
    const focused = selected && !own.some((b) => b.id === selected.id) ? [...own, selected] : own;
    const ids = new Set(focused.map((b) => b.id));
    return [...focused, ...this.regionBuses.buses.filter((b) => !ids.has(b.id))];
  }

  get routeCompany(): string | undefined {
    return this.selectedBus?.company ?? this.lineCompany;
  }

  onView(center: {lng: number; lat: number}, km2: number) {
    this.#setLiveRegions(
      regionsShowingAllBuses(center.lng, center.lat, km2)
        .map((r) => r.id)
        .join(',')
    );
  }

  get open(): boolean {
    return Boolean(this.stopId || this.lineId || this.selectedBus);
  }

  // no browser history
  get back(): (() => void) | undefined {
    if (this.busId) return () => this.closeBus();
    const fromBus = this.#lineFromBus();
    if (this.lineId && fromBus) {
      return () => {
        this.#setLineFromBus(null);
        this.url.select({[P.bus]: fromBus});
      };
    }
    if (this.lineId && this.stopId) return () => this.url.select(LINE_CLEARED);
    return undefined;
  }

  openStop(id: string, name?: string) {
    this.#setKnownName(name);
    this.#setBusFromStop(false);
    this.#setLineFromBus(null);
    this.url.select({[P.stop]: id, ...LINE_CLEARED, [P.bus]: null});
  }

  // first tap opens, then pins/unpins the camera
  tapBus(bus: Bus) {
    if (bus.id !== this.busId) {
      this.#setPinnedBus(null);
      this.openBus(bus);
      return;
    }
    this.#setPinnedBus((pinned) => (pinned === bus.id ? null : bus.id));
  }

  // brings its line
  openBus(bus: Bus) {
    const provider = this.regionBuses.providerOf(bus.id) ?? (this.stopId ? providerOf(this.stopId) : undefined);
    if (provider && !hasLines(provider)) {
      this.openBusOnly(bus);
      return;
    }
    const fromStop = this.lineId !== bus.line;
    this.#setBusFromStop(fromStop);
    if (!fromStop) {
      this.url.select({[P.bus]: bus.id});
      return;
    }
    const shape = bus.shape_id && provider ? `${provider}-${bus.shape_id}` : null;
    this.#setDirectionHint(shape ? null : (bus.destination ?? null));
    this.#setLineCompany(bus.company);
    this.#setLineFromBus(null);
    this.url.select({...LINE_CLEARED, [P.line]: bus.line, [P.shape]: shape, [P.bus]: bus.id});
  }

  // provider without lines
  openBusOnly(bus: Bus) {
    this.#setBusFromStop(false);
    this.#setLineFromBus(null);
    this.url.select({[P.bus]: bus.id});
  }

  showLine() {
    this.#setLineFromBus(this.busId);
    this.url.select({[P.bus]: null});
  }

  // same shelter, no history entry
  switchService(id: string) {
    this.url.select({[P.stop]: id}, true);
  }

  openLine(line: string, destination?: string, company?: string) {
    this.#setDirectionHint(destination ?? null);
    this.#setLineCompany(company);
    this.#setLineFromBus(null);
    this.url.select({...LINE_CLEARED, [P.line]: line, [P.bus]: null});
  }

  // live -> bus, scheduled -> line
  openArrival(line: string, destination?: string, company?: string) {
    const arrival = this.stops.arrivals.find((l) => l.line === line)?.arrivals.find((a) => a.live && a.bus);
    if (arrival?.bus && arrival.stop_position) this.openBusOnly(arrival.bus);
    else if (arrival?.bus) this.openBus(arrival.bus);
    else this.openLine(line, destination, company);
  }

  selectVariant(routeId: string, direction?: number) {
    this.url.select({
      [P.variant]: routeId,
      [P.direction]: direction === undefined ? null : String(direction),
      [P.shape]: null,
    });
  }

  deselectBus() {
    if (this.busId) this.url.select({[P.bus]: null});
  }

  closeBus() {
    if (this.#busFromStop()) this.url.select({...LINE_CLEARED, [P.bus]: null});
    else this.url.select({[P.bus]: null});
  }

  close() {
    if (this.busId) this.closeBus();
    else this.url.select({[P.stop]: null, ...LINE_CLEARED, [P.bus]: null});
  }

  #lineBuses(): Bus[] {
    return this.lineId ? this.line.buses : this.stops.buses;
  }
}
