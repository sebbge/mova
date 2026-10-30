import {createEffect, createSignal, onCleanup, untrack, type Accessor, type Setter} from 'solid-js';
import type {Bus, BusLeg, DayType, RouteStop} from '~/lib/api/types';
import {follows} from '~/lib/domain/buses';
import {hasLines, providerOf} from '~/lib/domain/providers';
import {LineFeed} from '~/lib/line/LineFeed';
import {StopFeed} from '~/lib/stop/StopFeed';
import {Fleet} from './Fleet';
import type {MapUrl} from './MapUrl';
import {TripPlanner} from './TripPlanner';

const P = {
  stop: 'parada',
  line: 'linea',
  variant: 'v',
  direction: 'd',
  shape: 's',
  bus: 'bus',
  schedule: 'planilla',
} as const;
const LINE_CLEARED = {[P.line]: null, [P.variant]: null, [P.direction]: null, [P.shape]: null, [P.schedule]: null};
const DAY_TYPES: readonly string[] = ['habil', 'sabado', 'domingo'];

// back returns to it before the line
interface BusStep {
  bus: string;
  search: string;
  busFromStop: boolean;
  lineFromBus: string | null;
  lineCompany: string | undefined;
}

// line before a stop was picked
interface LineOrigin {
  line: string;
  stop: string | null;
}

// sheet the arrivals were opened from
interface SheetStep {
  stop: string;
  search: string;
  origin: LineOrigin | null;
  busFromStop: boolean;
  lineFromBus: string | null;
}

// self-corrections use replace, or back loops
export class MapSelection {
  readonly stops = new StopFeed();
  readonly line = new LineFeed();
  readonly fleet: Fleet;
  readonly planner: TripPlanner;

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
  readonly #busTrail: Accessor<BusStep[]>;
  readonly #setBusTrail: Setter<BusStep[]>;
  readonly #previewStop: Accessor<string | null>; // map only
  readonly #setPreviewStop: Setter<string | null>;
  readonly #lineOrigin: Accessor<LineOrigin | null>;
  readonly #setLineOrigin: Setter<LineOrigin | null>;
  readonly #stopFrom: Accessor<SheetStep | null>;
  readonly #setStopFrom: Setter<SheetStep | null>;

  constructor(private readonly url: MapUrl) {
    this.fleet = new Fleet(this);
    this.planner = new TripPlanner(url, () => this.childOpen);
    [this.#directionHint, this.#setDirectionHint] = createSignal<string | null>(null);
    [this.#lineCompany, this.#setLineCompany] = createSignal<string>();
    [this.#knownName, this.#setKnownName] = createSignal<string>();
    [this.#busFromStop, this.#setBusFromStop] = createSignal(false);
    [this.#lineFromBus, this.#setLineFromBus] = createSignal<string | null>(null);
    [this.#pinnedBus, this.#setPinnedBus] = createSignal<string | null>(null);
    [this.#busTrail, this.#setBusTrail] = createSignal<BusStep[]>([]);
    [this.#previewStop, this.#setPreviewStop] = createSignal<string | null>(null);
    [this.#lineOrigin, this.#setLineOrigin] = createSignal<LineOrigin | null>(null);
    [this.#stopFrom, this.#setStopFrom] = createSignal<SheetStep | null>(null);

    // marked stop was the previous bus's
    createEffect(() => {
      void this.busId;
      untrack(() => this.#setPreviewStop(null));
    });

    // trail: close empties, back pops
    createEffect(() => {
      const id = this.busId;
      untrack(() => {
        if (!id) this.#setBusTrail([]);
        else if (this.#busTrail().at(-1)?.bus === id) this.#setBusTrail((trail) => trail.slice(0, -1));
      });
    });

    createEffect(() => this.stops.follow(this.stopId));
    onCleanup(() => this.stops.close());
    createEffect(() => onCleanup(this.line.followVariants(this.lineId) ?? (() => {})));
    createEffect(() => onCleanup(this.line.followBuses(this.#lineProvider(), this.lineId) ?? (() => {})));
    createEffect(() =>
      onCleanup(this.line.followTimes(this.lineId ? this.line.shape : null, this.#lineStopId()) ?? (() => {}))
    );
    createEffect(() =>
      onCleanup(
        this.line.followSchedule(this.scheduleOpen ? this.line.shape : null, this.#lineStopId(), this.#scheduleDay()) ??
          (() => {})
      )
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

  get previewStop(): string | null {
    return this.#previewStop();
  }

  // marked or url stop
  get mapStopId(): string | null {
    return this.#previewStop() ?? this.stopId;
  }

  // may be a sibling of the same shelter
  get lineStop(): RouteStop | undefined {
    const id = this.stopId;
    if (!id) return undefined;
    const refs = this.stops.refs;
    return this.line.shape?.stops.find((s) => s.stop_id === id || refs.includes(s.stop_id));
  }

  // hoy: server picks the day type
  get scheduleOpen(): boolean {
    return this.lineId !== null && this.url.get(P.schedule) !== null;
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

  get routeCompany(): string | undefined {
    return this.fleet.selected?.company ?? this.lineCompany;
  }

  onView(center: {lng: number; lat: number}, km2: number) {
    this.planner.setCenter({lat: center.lat, lng: center.lng});
    this.fleet.view(center, km2);
  }

  get childOpen(): boolean {
    return Boolean(this.stopId || this.lineId || this.fleet.selected);
  }

  get open(): boolean {
    return this.childOpen || this.planner.active;
  }

  // no browser history
  get back(): (() => void) | undefined {
    if (this.busId) return this.#busTrail().length > 0 ? () => this.#backToBus() : () => this.closeBus();
    if (this.scheduleOpen) return () => this.closeSchedule();
    const fromBus = this.#lineFromBus();
    if (this.lineId && fromBus) {
      return () => {
        this.#setLineFromBus(null);
        this.url.select({[P.bus]: fromBus});
      };
    }
    if (this.lineId && this.#originStop()) return () => this.#leaveLine();
    if (!this.lineId && this.stopId && this.#stopFrom()?.stop === this.stopId) return () => this.#backToSheet();
    const trip = this.planner;
    if (!this.childOpen) return trip.active && trip.option !== null ? () => trip.setOption(null) : undefined;
    return trip.active ? () => this.close() : undefined;
  }

  openStop(id: string, name?: string) {
    const picking = this.planner.picking;
    if (picking) {
      this.planner.setPoint(picking, {kind: 'stop', ref: id});
      return;
    }
    this.#setKnownName(name);
    this.#setBusFromStop(false);
    this.#setLineFromBus(null);
    this.#setLineOrigin(null);
    this.#setStopFrom(null);
    this.url.select({[P.stop]: id, ...LINE_CLEARED, [P.bus]: null});
  }

  // stays in the line sheet
  selectMapStop(id: string, name?: string) {
    const onLine = this.lineId && !this.fleet.selected && this.line.shape?.stops.some((s) => s.stop_id === id);
    if (onLine && id === this.stopId) this.openStopFromLine(id, name);
    else if (onLine) this.pickLineStop(id);
    else this.openStop(id, name);
  }

  pickLineStop(id: string) {
    const picking = this.planner.picking;
    if (picking) {
      this.planner.setPoint(picking, {kind: 'stop', ref: id});
      return;
    }
    const line = this.lineId;
    if (!line) return;
    if (this.#lineOrigin()?.line !== line) this.#setLineOrigin({line, stop: this.stopId});
    this.url.select({[P.stop]: id}, true);
  }

  // back returns to the line
  openStopFromLine(id: string, name?: string) {
    const line = this.lineId;
    const origin = this.#lineOrigin();
    this.#setStopFrom({
      stop: id,
      search: this.url.search,
      origin: !line ? null : origin?.line === line ? origin : {line, stop: this.stopId},
      busFromStop: this.#busFromStop(),
      lineFromBus: this.#lineFromBus(),
    });
    this.#setKnownName(name);
    this.url.select({[P.stop]: id, ...LINE_CLEARED, [P.bus]: null});
  }

  // first tap marks, second opens arrivals
  pickBusStop(stopId: string, name?: string) {
    const provider = (this.busId && this.fleet.providerOf(this.busId)) || this.#lineProvider();
    if (!provider) return;
    const ref = `${provider}-${stopId}`;
    if (this.#previewStop() === ref) this.openStopFromLine(ref, name);
    else this.#setPreviewStop(ref);
  }

  // first tap opens, then pins/unpins the camera
  tapBus(bus: Bus) {
    if (bus.id !== this.busId) {
      const open = this.busId;
      if (open) {
        const step: BusStep = {
          bus: open,
          search: this.url.search,
          busFromStop: this.#busFromStop(),
          lineFromBus: this.#lineFromBus(),
          lineCompany: this.#lineCompany(),
        };
        this.#setBusTrail((trail) => [...trail, step]);
      }
      this.#setPinnedBus(null);
      this.openBus(bus);
      return;
    }
    this.#setPinnedBus((pinned) => (pinned === bus.id ? null : bus.id));
  }

  // brings its line
  openBus(bus: Bus) {
    // fb line's provider, or the bus list empties
    const provider =
      this.fleet.providerOf(bus.id) ?? this.#lineProvider() ?? (this.stopId ? providerOf(this.stopId) : undefined);
    if (provider && !hasLines(provider)) {
      this.openBusOnly(bus);
      return;
    }
    const fromStop = this.lineId !== bus.line;
    this.#setBusFromStop(fromStop);
    // same trace: only the bus changes
    const drawn = this.line.shape;
    const sameTrace = !drawn || (follows(bus, drawn) && (!bus.shape_id || bus.shape_id === drawn.shape_id));
    if (!fromStop && sameTrace) {
      this.url.select({[P.bus]: bus.id});
      return;
    }
    const shapeRef = bus.shape_id && provider ? `${provider}-${bus.shape_id}` : null;
    this.#setDirectionHint(shapeRef ? null : (bus.destination ?? null));
    this.#setLineCompany(bus.company);
    this.#setLineFromBus(null);
    this.url.select({...LINE_CLEARED, [P.line]: bus.line, [P.shape]: shapeRef, [P.bus]: bus.id});
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
    this.#setLineOrigin(null);
    this.url.select({...LINE_CLEARED, [P.line]: line, [P.bus]: null});
  }

  // live -> bus, scheduled -> line
  openArrival(line: string, destination?: string, company?: string) {
    const arrival = this.stops.arrivals
      .find((l) => l.line === line && (l.destination ?? undefined) === destination)
      ?.arrivals.find((a) => a.live && a.bus);
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

  openLeg(leg: BusLeg) {
    this.#setDirectionHint(leg.headsign);
    this.#setLineCompany(undefined);
    this.#setLineFromBus(null);
    this.#setLineOrigin(null);
    this.url.select({...LINE_CLEARED, [P.line]: leg.line, [P.variant]: leg.route_id, [P.bus]: null});
  }

  // open child first, else the trip
  closeSheet() {
    if (this.childOpen) this.close();
    else this.planner.close();
  }

  deselectBus() {
    if (this.busId) this.url.select({[P.bus]: null});
  }

  closeBus() {
    if (this.#busFromStop()) this.url.select({...LINE_CLEARED, [P.bus]: null});
    else this.url.select({[P.bus]: null});
  }

  // over the line, back returns to it
  openSchedule() {
    this.url.select({[P.schedule]: 'hoy'});
  }

  pickScheduleDay(day: DayType) {
    this.url.select({[P.schedule]: day}, true);
  }

  closeSchedule() {
    this.url.select({[P.schedule]: null});
  }

  close() {
    if (this.busId) this.closeBus();
    else if (this.lineId && this.#originStop() && !this.scheduleOpen) this.#leaveLine();
    else {
      this.#setLineOrigin(null);
      this.#setStopFrom(null);
      this.url.select({[P.stop]: null, ...LINE_CLEARED, [P.bus]: null});
    }
  }

  #lineStopId(): string | null {
    return this.lineStop?.stop_id ?? null;
  }

  #scheduleDay(): DayType | null {
    const day = this.url.get(P.schedule);
    return day && DAY_TYPES.includes(day) ? (day as DayType) : null;
  }

  #originStop(): string | null {
    const origin = this.#lineOrigin();
    return origin?.line === this.lineId ? origin.stop : this.stopId;
  }

  #leaveLine() {
    const origin = this.#originStop();
    this.#setLineOrigin(null);
    this.url.select({[P.stop]: origin, ...LINE_CLEARED, [P.bus]: null});
  }

  #backToSheet() {
    const step = this.#stopFrom();
    if (!step) return;
    this.#setStopFrom(null);
    this.#setLineOrigin(step.origin);
    this.#setBusFromStop(step.busFromStop);
    this.#setLineFromBus(step.lineFromBus);
    this.url.restore(step.search);
  }

  // fb while s arrives before v
  #lineProvider(): string | null {
    const ref = this.routeId ?? this.shapeRef;
    return ref ? providerOf(ref) : null;
  }

  #backToBus() {
    const step = this.#busTrail().at(-1);
    if (!step) return;
    this.#setBusFromStop(step.busFromStop);
    this.#setLineFromBus(step.lineFromBus);
    this.#setLineCompany(step.lineCompany);
    this.url.restore(step.search);
  }
}
