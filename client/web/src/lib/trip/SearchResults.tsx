import {createEffect, createSignal, For, Match, on, onCleanup, Show, Switch} from 'solid-js';
import {api} from '~/lib/api/client';
import type {PlaceHit, StopHit} from '~/lib/api/types';
import {fullStopName} from '~/lib/format';
import type {Endpoint} from './endpoint';
import styles from './SearchResults.module.css';

const DEBOUNCE_MS = 250;

// empty query shows location and map
export default function SearchResults(props: {
  term: string;
  near?: {lat: number; lng: number};
  onChoose: (endpoint: Endpoint, label: string) => void;
  onPick: () => void;
}) {
  const [places, setPlaces] = createSignal<PlaceHit[]>([]);
  const [stops, setStops] = createSignal<StopHit[]>([]);
  const [status, setStatus] = createSignal<'idle' | 'loading' | 'error'>('idle');

  let timer: ReturnType<typeof setTimeout> | undefined;
  let inflight: AbortController | undefined;
  onCleanup(() => {
    clearTimeout(timer);
    inflight?.abort();
  });

  createEffect(
    on(
      () => props.term,
      (q) => {
        clearTimeout(timer);
        inflight?.abort();
        if (!q) {
          setPlaces([]);
          setStops([]);
          setStatus('idle');
          return;
        }
        setStatus('loading');
        timer = setTimeout(async () => {
          const controller = new AbortController();
          inflight = controller;
          const o = {signal: controller.signal};
          const near = props.near;
          const [p, s] = await Promise.allSettled([api.searchPlaces(q, near, o), api.searchStops(q, near, o)]);
          if (controller.signal.aborted) return;
          setPlaces(p.status === 'fulfilled' ? p.value : []);
          setStops(s.status === 'fulfilled' ? s.value : []);
          setStatus(p.status === 'rejected' && s.status === 'rejected' ? 'error' : 'idle');
        }, DEBOUNCE_MS);
      }
    )
  );

  return (
    <Show
      when={props.term}
      fallback={
        <ul class={styles.results}>
          <li>
            <button onClick={() => props.onChoose({kind: 'here'}, 'Mi ubicación')}>
              <span class={styles.name}>Mi ubicación</span>
            </button>
          </li>
          <li>
            <button onClick={() => props.onPick()}>
              <span class={styles.name}>Elegir en el mapa</span>
              <span class={styles.kind}>Un punto o una parada</span>
            </button>
          </li>
        </ul>
      }>
      <ul class={styles.results}>
        <Switch>
          <Match when={status() === 'error'}>
            <li class={styles.note}>No pudimos buscar ahora. Revisá la conexión.</li>
          </Match>
          <Match when={status() === 'idle' && places().length === 0 && stops().length === 0}>
            <li class={styles.note}>No encontramos lugares ni paradas con ese nombre.</li>
          </Match>
        </Switch>
        <For each={places()}>
          {(hit) => (
            <li>
              <button onClick={() => props.onChoose({kind: 'place', id: hit.id}, hit.name)}>
                <span class={styles.name}>{hit.name}</span>
                <span class={styles.kind}>{hit.context ? `${hit.category} · ${hit.context}` : hit.category}</span>
              </button>
            </li>
          )}
        </For>
        <For each={stops()}>
          {(hit) => (
            <li>
              <button onClick={() => props.onChoose({kind: 'stop', ref: hit.id}, fullStopName(hit.name, hit.station))}>
                <span class={styles.name}>{fullStopName(hit.name, hit.station)}</span>
                <span class={styles.kind}>Parada</span>
              </button>
            </li>
          )}
        </For>
      </ul>
    </Show>
  );
}
