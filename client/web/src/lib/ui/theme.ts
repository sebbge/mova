import {createRoot, createSignal} from 'solid-js';

const KEY = 'theme'; // Base.astro's inline script reads it too

function stored(): string | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

class Theme {
  readonly #dark = createRoot(() => createSignal(false));
  #system: MediaQueryList | null = null;

  constructor() {
    if (typeof document === 'undefined') return;
    this.#system = matchMedia('(prefers-color-scheme: dark)');
    this.#dark[1](document.documentElement.dataset.theme === 'dark');
    this.#system.addEventListener('change', (e) => {
      if (!stored()) this.#apply(e.matches);
    });
  }

  get dark(): boolean {
    return this.#dark[0]();
  }

  // system choice clears the stored one
  toggle() {
    const dark = !this.dark;
    try {
      if (dark === this.#system?.matches) localStorage.removeItem(KEY);
      else localStorage.setItem(KEY, dark ? 'dark' : 'light');
    } catch {
      // private mode, this page only
    }
    this.#apply(dark);
  }

  #apply(dark: boolean) {
    this.#dark[1](dark);
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  }
}

export const theme = new Theme();
