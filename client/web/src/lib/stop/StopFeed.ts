import {type Accessor, createSignal, type Setter, untrack} from 'solid-js';
import {api} from '~/lib/api/client';
import type {Bus, Stop, StopLine} from '~/lib/api/types';
import {ArrivalsSource} from './ArrivalsSource';

class StopSource extends ArrivalsSource {
  readonly #stop: Accessor<Stop | null>;

  constructor(ref: string, onStop: (stop: Stop) => void) {
    super(ref);
    let setStop: Setter<Stop | null>;
    [this.#stop, setStop] = createSignal<Stop | null>(null);
    api
      .stop(ref, {signal: this.controller.signal})
      .then((s) => {
        setStop(s);
        onStop(s);
      })
      .catch(() => {});
  }

  get stop(): Stop | null {
    return this.#stop();
  }
}

// shelter stops load together
export class StopFeed {
  readonly #sources: Accessor<Record<string, StopSource>>;
  readonly #setSources: Setter<Record<string, StopSource>>;
  readonly #active: Accessor<string | null>;
  readonly #setActive: Setter<string | null>;

  constructor() {
    [this.#sources, this.#setSources] = createSignal<Record<string, StopSource>>({});
    [this.#active, this.#setActive] = createSignal<string | null>(null);
  }

  get stop(): Stop | null {
    return this.#current()?.stop ?? null;
  }

  get arrivals(): StopLine[] {
    return this.#current()?.lines ?? [];
  }

  get buses(): Bus[] {
    return this.#current()?.buses ?? [];
  }

  get loading(): boolean {
    return this.#current()?.loading ?? false;
  }

  get refs(): string[] {
    return Object.keys(this.#sources());
  }

  // tracks ref only, or the effect loops
  follow(ref: string | null) {
    untrack(() => {
      if (ref && ref in this.#sources()) {
        this.#setActive(ref);
        return;
      }
      this.close();
      if (!ref) return;
      this.#setSources({[ref]: this.#source(ref)});
      this.#setActive(ref);
    });
  }

  close() {
    for (const source of Object.values(this.#sources())) source.close();
    this.#setSources({});
    this.#setActive(null);
  }

  #current(): StopSource | null {
    const active = this.#active();
    return active ? (this.#sources()[active] ?? null) : null;
  }

  #source(ref: string): StopSource {
    return new StopSource(ref, (stop) => {
      const missing = stop.siblings.filter((s) => !(s in this.#sources()));
      if (missing.length === 0) return;
      this.#setSources((all) => ({...all, ...Object.fromEntries(missing.map((s) => [s, this.#source(s)]))}));
    });
  }
}
