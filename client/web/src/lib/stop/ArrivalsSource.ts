import {createMemo, createSignal, onCleanup, type Accessor, type Setter} from 'solid-js';
import {api} from '~/lib/api/client';
import type {Bus, StopLine, StopSchedule} from '~/lib/api/types';
import {scheduleToLines} from '~/lib/domain/schedule';

// schedule first, live wins
export class ArrivalsSource {
  static readonly REFRESH_MS = 30_000;

  readonly #schedule: Accessor<StopSchedule | null>;
  readonly #setSchedule: Setter<StopSchedule | null>;
  readonly #live: Accessor<StopLine[]>;
  readonly #setLive: Setter<StopLine[]>;
  readonly #answered: Accessor<number>;
  readonly #setAnswered: Setter<number>;
  readonly #liveFailed: Accessor<boolean>;
  readonly #setLiveFailed: Setter<boolean>;
  readonly #tick: Accessor<number>;
  readonly #setTick: Setter<number>;
  readonly #lines: Accessor<StopLine[]>;
  readonly #controller = new AbortController();

  constructor(readonly stopId: string) {
    [this.#schedule, this.#setSchedule] = createSignal<StopSchedule | null>(null);
    [this.#live, this.#setLive] = createSignal<StopLine[]>([]);
    [this.#answered, this.#setAnswered] = createSignal(0);
    [this.#liveFailed, this.#setLiveFailed] = createSignal(false);
    [this.#tick, this.#setTick] = createSignal(Date.now());

    this.#lines = createMemo(() => {
      const live = this.#live();
      if (live.length > 0) return live;
      const schedule = this.#schedule();
      return schedule ? scheduleToLines(schedule, new Date(this.#tick())) : [];
    });

    api
      .stopSchedule(stopId, {signal: this.#controller.signal})
      .then((s) => this.#setSchedule(s))
      .catch(() => {})
      .finally(() => this.#answer());

    void this.#load();
    const timer = setInterval(() => void this.#load(), ArrivalsSource.REFRESH_MS);
    onCleanup(() => {
      clearInterval(timer);
      this.#controller.abort();
    });
  }

  get lines(): StopLine[] {
    return this.#lines();
  }

  get loading(): boolean {
    return this.#answered() === 0;
  }

  get liveFailed(): boolean {
    return this.#liveFailed();
  }

  get buses(): Bus[] {
    return this.lines.flatMap((l) => l.arrivals).flatMap((a) => (a.bus ? [a.bus] : []));
  }

  async #load() {
    const signal = this.#controller.signal;
    try {
      this.#setLive((await api.stopArrivals(this.stopId, {signal})).lines);
      this.#setLiveFailed(false);
    } catch {
      if (!signal.aborted) this.#setLiveFailed(true); // keeps the last arrivals
    } finally {
      if (!signal.aborted) {
        this.#answer();
        this.#setTick(Date.now()); // schedule keeps counting
      }
    }
  }

  #answer() {
    if (!this.#controller.signal.aborted) this.#setAnswered((n) => n + 1);
  }
}
