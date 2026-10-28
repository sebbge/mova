import {createSignal, onCleanup, type Accessor, type Setter} from 'solid-js';

// shareable links, back closes the sheet
export class MapUrl {
  readonly #search: Accessor<string>;
  readonly #setSearch: Setter<string>;

  constructor() {
    [this.#search, this.#setSearch] = createSignal(location.search);
    const onPop = () => this.#setSearch(location.search);
    addEventListener('popstate', onPop);
    onCleanup(() => removeEventListener('popstate', onPop));
  }

  get(key: string): string | null {
    return new URLSearchParams(this.#search()).get(key);
  }

  has(key: string): boolean {
    return new URLSearchParams(this.#search()).has(key);
  }

  get search(): string {
    return this.#search();
  }

  restore(search: string) {
    const url = new URL(location.href);
    url.search = search;
    history.pushState(null, '', url);
    this.#setSearch(url.search);
  }

  select(params: Record<string, string | null>, replace = false) {
    const url = new URL(location.href);
    for (const [key, value] of Object.entries(params)) {
      if (value === null) url.searchParams.delete(key);
      else url.searchParams.set(key, value);
    }
    history[replace ? 'replaceState' : 'pushState'](null, '', url);
    this.#setSearch(url.search);
  }
}
