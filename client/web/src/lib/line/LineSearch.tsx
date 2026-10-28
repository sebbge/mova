import {createMemo, createSignal, For, Match, onCleanup, Show, Switch} from 'solid-js';
import {api} from '~/lib/api/client';
import type {BusRoute} from '~/lib/api/types';
import {regionById} from '~/lib/domain/regions';
import {href} from '~/lib/ui/href';
import LineBadge from './LineBadge';
import styles from './LineSearch.module.css';

const DEBOUNCE_MS = 250;

// no area -> map
function lineHref(short: string, area: string | undefined): string {
  const region = regionById(area);
  return region ? href.line(region.slug, short) : href.map({linea: short});
}

export default function LineSearch() {
  const [term, setTerm] = createSignal('');
  const [results, setResults] = createSignal<BusRoute[]>([]);
  const [status, setStatus] = createSignal<'idle' | 'loading' | 'error'>('idle');

  let timer: ReturnType<typeof setTimeout> | undefined;
  let inflight: AbortController | undefined;
  onCleanup(() => {
    clearTimeout(timer);
    inflight?.abort();
  });

  const lines = createMemo(() => {
    const byName = new Map<string, BusRoute[]>();
    for (const r of results()) {
      const group = byName.get(r.route_short_name) ?? [];
      group.push(r);
      byName.set(r.route_short_name, group);
    }
    return [...byName.entries()].map(([short, variants]) => ({short, variants}));
  });

  const query = () => term().trim();

  function onInput(value: string) {
    setTerm(value);
    clearTimeout(timer);
    inflight?.abort();

    const q = query();
    if (!q) {
      setResults([]);
      setStatus('idle');
      return;
    }

    setStatus('loading');
    timer = setTimeout(async () => {
      const controller = new AbortController();
      inflight = controller;
      try {
        setResults(await api.searchRoutes(q, {signal: controller.signal}));
        setStatus('idle');
      } catch {
        if (controller.signal.aborted) return;
        setResults([]);
        setStatus('error');
      }
    }, DEBOUNCE_MS);
  }

  return (
    <div class={styles.search}>
      <input
        type='search'
        value={term()}
        onInput={(e) => onInput(e.currentTarget.value)}
        placeholder='Buscá una línea: 522, 183…'
        aria-label='Buscar línea'
      />

      <Switch>
        <Match when={status() === 'error'}>
          <p class={styles.note}>No pudimos buscar ahora. Revisá la conexión e intentá de nuevo.</p>
        </Match>
        <Match when={status() === 'idle' && query() && lines().length === 0}>
          <p class={styles.note}>Sin resultados para "{query()}".</p>
        </Match>
        <Match when={lines().length > 0}>
          <ul class='rows'>
            <For each={lines()}>
              {(line) => (
                <li>
                  <a class={styles.item} href={lineHref(line.short, line.variants[0].area)}>
                    <LineBadge line={line.short} />
                    <span class={styles.names}>
                      {line.variants[0].route_long_name ?? ''}
                      <Show when={line.variants.length > 1}>
                        <span class={styles.variants}>{line.variants.length} recorridos</span>
                      </Show>
                    </span>
                  </a>
                </li>
              )}
            </For>
          </ul>
        </Match>
      </Switch>
    </div>
  );
}
