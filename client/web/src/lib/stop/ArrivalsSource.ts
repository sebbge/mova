import {createMemo, createRoot, createSignal, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {Bus, StopLine, StopSchedule} from '~/lib/api/types';
import {scheduleToLines} from '~/lib/domain/schedule';

// schedule first, live wins
export class ArrivalsSource {
  static readonly REFRESH_MS = 30_000;

  readonly #setLive: Setter<StopLine[]>;
  readonly #setArrivalsAnswered: Setter<boolean>;
  readonly #liveFailed: Accessor<boolean>;
  readonly #setLiveFailed: Setter<boolean>;
  readonly #setTick: Setter<number>;
  readonly #lines: Accessor<StopLine[]>;
  readonly #loading: Accessor<boolean>;
  readonly #dispose: () => void;
  readonly #timer: ReturnType<typeof setInterval>;
  protected readonly controller = new AbortController();

  constructor(readonly ref: string) {
    const [schedule, setSchedule] = createSignal<StopSchedule | null>(null);
    const [live, setLive] = createSignal<StopLine[]>([]);
    const [scheduleAnswered, setScheduleAnswered] = createSignal(false);
    const [arrivalsAnswered, setArrivalsAnswered] = createSignal(false);
    const [tick, setTick] = createSignal(Date.now());
    [this.#liveFailed, this.#setLiveFailed] = createSignal(false);
    this.#setLive = setLive;
    this.#setArrivalsAnswered = setArrivalsAnswered;
    this.#setTick = setTick;

    let dispose!: () => void;
    let lines!: Accessor<StopLine[]>;
    let loading!: Accessor<boolean>;
    createRoot((d) => {
      dispose = d;
      const scheduled = createMemo(() => {
        const s = schedule();
        return s ? scheduleToLines(s, new Date(tick())) : [];
      });
      lines = createMemo(() => (live().length > 0 ? live() : scheduled()));
      // empty schedule keeps waiting for live
      loading = createMemo(() => !arrivalsAnswered() && (!scheduleAnswered() || scheduled().length === 0));
    });
    this.#dispose = dispose;
    this.#lines = lines;
    this.#loading = loading;

    api
      .stopSchedule(ref, {signal: this.controller.signal})
      .then((s) => setSchedule(s))
      .catch(() => {})
      .finally(() => !this.controller.signal.aborted && setScheduleAnswered(true));

    void this.#load();
    this.#timer = setInterval(() => void this.#load(), ArrivalsSource.REFRESH_MS);
  }

  get lines(): StopLine[] {
    return this.#lines();
  }

  get loading(): boolean {
    return this.#loading();
  }

  get liveFailed(): boolean {
    return this.#liveFailed();
  }

  get buses(): Bus[] {
    return this.lines.flatMap((l) => l.arrivals).flatMap((a) => (a.bus ? [a.bus] : []));
  }

  close() {
    clearInterval(this.#timer);
    this.controller.abort();
    this.#dispose();
  }

  async #load() {
    const signal = this.controller.signal;
    try {
      this.#setLive((await api.stopArrivals(this.ref, {signal})).lines);
      this.#setLiveFailed(false);
    } catch {
      if (!signal.aborted) this.#setLiveFailed(true); // keeps the last arrivals
    } finally {
      if (!signal.aborted) {
        this.#setArrivalsAnswered(true);
        this.#setTick(Date.now()); // schedule keeps counting
      }
    }
  }
}
