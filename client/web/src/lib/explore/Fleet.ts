import {createEffect, createSignal, onCleanup, untrack, type Accessor, type Setter} from 'solid-js';
import type {Bus} from '~/lib/api/types';
import {RegionBuses} from '~/lib/bus/RegionBuses';
import {follows} from '~/lib/domain/buses';
import {regionsShowingAllBuses} from '~/lib/domain/regions';
import type {LineFeed} from '~/lib/line/LineFeed';
import type {StopFeed} from '~/lib/stop/StopFeed';

export interface FleetFocus {
  readonly busId: string | null;
  readonly lineId: string | null;
  readonly line: LineFeed;
  readonly stops: StopFeed;
}

const reportedAt = (bus: Bus) => (bus.timestamp ? Date.parse(bus.timestamp) || 0 : 0);

// keeps every bus's latest position
export class Fleet {
  readonly #region = new RegionBuses();
  readonly #latest: Accessor<Record<string, Bus>>;
  readonly #setLatest: Setter<Record<string, Bus>>;
  readonly #liveRegions: Accessor<string>; // stable string
  readonly #setLiveRegions: Setter<string>;

  constructor(private readonly focus: FleetFocus) {
    [this.#latest, this.#setLatest] = createSignal<Record<string, Bus>>({});
    [this.#liveRegions, this.#setLiveRegions] = createSignal('');

    createEffect(() => onCleanup(this.#region.follow(this.#liveRegions()) ?? (() => {})));
    createEffect(() => this.#remember(focus.stops.buses));
    createEffect(() => this.#remember(focus.stops.positions));
    createEffect(() => this.#remember(focus.line.buses));
    createEffect(() => this.#remember(this.#region.buses));
  }

  // from a stop, the open bus comes from the arrival
  get selected(): Bus | null {
    const id = this.focus.busId;
    if (!id) return null;
    const found =
      this.#own().find((b) => b.id === id) ??
      this.focus.stops.buses.find((b) => b.id === id) ??
      this.#region.buses.find((b) => b.id === id);
    return found ? this.#freshest(found) : null;
  }

  // selection first, region buses behind
  get all(): Bus[] {
    const own = this.#own();
    const selected = this.selected;
    const focused = selected && !own.some((b) => b.id === selected.id) ? [...own, selected] : own;
    const ids = new Set(focused.map((b) => b.id));
    return [...focused, ...this.#region.buses.filter((b) => !ids.has(b.id))].map((b) => this.#freshest(b));
  }

  // only if it follows the drawn line
  get direction(): number | undefined {
    const bus = this.selected;
    const shape = this.focus.line.shape;
    return bus && shape && this.focus.lineId === bus.line && follows(bus, shape) ? shape.direction_id : undefined;
  }

  providerOf(busId: string): string | undefined {
    return this.#region.providerOf(busId);
  }

  view(center: {lng: number; lat: number}, km2: number) {
    this.#setLiveRegions(
      regionsShowingAllBuses(center.lng, center.lat, km2)
        .map((r) => r.id)
        .join(',')
    );
  }

  #own(): Bus[] {
    return this.focus.lineId ? this.focus.line.buses : this.focus.stops.buses;
  }

  #freshest(bus: Bus): Bus {
    return this.#latest()[bus.id] ?? bus;
  }

  #remember(list: Bus[]) {
    untrack(() => {
      const latest = this.#latest();
      let next: Record<string, Bus> | null = null;
      for (const bus of list) {
        const known = (next ?? latest)[bus.id];
        if (known === bus || (known && reportedAt(known) > reportedAt(bus))) continue;
        (next ??= {...latest})[bus.id] = bus;
      }
      if (next) this.#setLatest(next);
    });
  }
}
