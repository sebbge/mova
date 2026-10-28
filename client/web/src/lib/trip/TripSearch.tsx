import {createSignal, For, Match, onCleanup, onMount, Show, Switch} from 'solid-js';
import {createStore} from 'solid-js/store';
import {api} from '~/lib/api/client';
import type {PlaceHit, StopHit, TravelMode} from '~/lib/api/types';
import {fullStopName, localIso} from '~/lib/format';
import type {Endpoint} from './endpoint';
import {CATEGORIES, fareProfile, PAYMENTS} from './profile';
import type {Place} from './TripFeed';
import styles from './TripSearch.module.css';

type Field = 'a' | 'b';

const DEBOUNCE_MS = 250;

export default function TripSearch(props: {
  a: Place | null;
  b: Place | null;
  field: Field;
  near?: {lat: number; lng: number};
  mode: TravelMode;
  at: string | null;
  onPoint?: (field: Field, endpoint: Endpoint, label: string) => void;
  onPick?: (field: Field) => void;
  onSwap?: () => void;
  onAt?: (at: string | null) => void;
  onClose?: () => void;
}) {
  const [active, setActive] = createSignal<Field>('b');
  const [terms, setTerms] = createStore<Record<Field, string>>({a: '', b: ''});
  const [places, setPlaces] = createSignal<PlaceHit[]>([]);
  const [stops, setStops] = createSignal<StopHit[]>([]);
  const [status, setStatus] = createSignal<'idle' | 'loading' | 'error'>('idle');
  const [when, setWhen] = createSignal<'now' | 'depart'>('now');
  const [stamp, setStamp] = createSignal(localIso(new Date()));
  const [fareOpen, setFareOpen] = createSignal(false);
  const inputs: Partial<Record<Field, HTMLInputElement>> = {};
  let panel!: HTMLDivElement;
  const term = () => terms[active()].trim();

  let timer: ReturnType<typeof setTimeout> | undefined;
  let inflight: AbortController | undefined;
  onCleanup(() => {
    clearTimeout(timer);
    inflight?.abort();
  });

  onMount(() => {
    setActive(props.field);
    setWhen(props.at ? 'depart' : 'now');
    if (props.at) setStamp(props.at);
    const opener = document.activeElement as HTMLElement | null;
    inputs[props.field]?.focus();
    onCleanup(() => opener?.focus());
  });

  // focus trap, escape closes the panel only
  function trap(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      props.onClose?.();
      return;
    }
    if (e.key !== 'Tab') return;
    const focusable = [...panel.querySelectorAll<HTMLElement>('button:not(:disabled), input')];
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last?.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first?.focus();
    }
  }

  function search(which: Field, value: string) {
    setTerms(which, value);
    setActive(which);
    clearTimeout(timer);
    inflight?.abort();
    const q = value.trim();
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
      const [p, s] = await Promise.allSettled([api.searchPlaces(q, props.near, o), api.searchStops(q, props.near, o)]);
      if (controller.signal.aborted) return;
      setPlaces(p.status === 'fulfilled' ? p.value : []);
      setStops(s.status === 'fulfilled' ? s.value : []);
      setStatus(p.status === 'rejected' && s.status === 'rejected' ? 'error' : 'idle');
    }, DEBOUNCE_MS);
  }

  function choose(endpoint: Endpoint, label: string) {
    const which = active();
    setTerms(which, '');
    setPlaces([]);
    setStops([]);
    props.onPoint?.(which, endpoint, label);
    if (which === 'a' && !props.b) {
      setActive('b');
      inputs.b?.focus();
    }
  }

  function pickWhen(next: 'now' | 'depart') {
    setWhen(next);
    props.onAt?.(next === 'now' ? null : stamp());
  }

  const Field = (p: {field: Field; label: string; placeholder: string}) => (
    <div class={styles.field}>
      <span class={`${styles.dot} ${p.field === 'a' ? styles.origin : styles.destination}`} aria-hidden='true'>
        {p.field.toUpperCase()}
      </span>
      <input
        ref={(el) => (inputs[p.field] = el)}
        value={terms[p.field]}
        onInput={(e) => search(p.field, e.currentTarget.value)}
        onFocus={() => setActive(p.field)}
        placeholder={p.placeholder}
        aria-label={p.label}
        type='search'
      />
    </div>
  );

  const Chip = (p: {on: boolean; onClick: () => void; children: string}) => (
    <button class={styles.chip} classList={{[styles.on]: p.on}} aria-pressed={p.on} onClick={() => p.onClick()}>
      {p.children}
    </button>
  );

  return (
    <div
      ref={panel}
      class={styles.panel}
      role='dialog'
      aria-modal='true'
      aria-label='Planificar viaje'
      tabindex='-1'
      on:keydown={trap}>
      <div class={styles.fields}>
        <Field field='a' label='Origen' placeholder={props.a?.label ?? 'Origen'} />
        <Field field='b' label='Destino' placeholder={props.b?.label ?? '¿A dónde vas?'} />
        <button class={styles.swap} onClick={() => props.onSwap?.()} aria-label='Invertir origen y destino'>
          <svg viewBox='0 0 24 24' width='18' height='18' aria-hidden='true'>
            <path
              d='M8 4v16M8 4L4 8M8 4l4 4M16 20V4M16 20l-4-4M16 20l4-4'
              fill='none'
              stroke='currentColor'
              stroke-width='2'
              stroke-linecap='round'
              stroke-linejoin='round'
            />
          </svg>
        </button>
      </div>

      <Show
        when={term()}
        fallback={
          <>
            <ul class={styles.results}>
              <li>
                <button onClick={() => choose({kind: 'here'}, 'Mi ubicación')}>
                  <span class={styles.name}>Mi ubicación</span>
                </button>
              </li>
              <li>
                <button onClick={() => props.onPick?.(active())}>
                  <span class={styles.name}>Elegir en el mapa</span>
                  <span class={styles.kind}>Un punto o una parada</span>
                </button>
              </li>
            </ul>

            <div class={styles.options}>
              <p class={styles.label}>Cuándo</p>
              <div class={styles.chips}>
                <Chip on={when() === 'now'} onClick={() => pickWhen('now')}>
                  Salir ahora
                </Chip>
                <Chip on={when() === 'depart'} onClick={() => pickWhen('depart')}>
                  Salir a las…
                </Chip>
              </div>
              <Show when={when() === 'depart'}>
                <input
                  class={styles.stamp}
                  type='datetime-local'
                  value={stamp()}
                  onInput={(e) => setStamp(e.currentTarget.value)}
                  onChange={() => stamp() && props.onAt?.(stamp())}
                  aria-label='Hora de salida'
                />
              </Show>

              <Show when={props.mode === 'transit'}>
                {/* folded, rarely changed */}
                <button
                  class={styles.fare}
                  aria-expanded={fareOpen()}
                  aria-controls='fare-options'
                  onClick={() => setFareOpen(!fareOpen())}>
                  <span class={styles.label}>Boleto</span>
                  <span class={styles.fareValue}>{fareProfile.label}</span>
                  <svg viewBox='0 0 24 24' width='16' height='16' aria-hidden='true'>
                    <path d='M6 9l6 6 6-6' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' />
                  </svg>
                </button>
                <Show when={fareOpen()}>
                  <div id='fare-options' class={styles.fareOptions}>
                    <div class={styles.chips}>
                      <For each={PAYMENTS}>
                        {(p) => (
                          <Chip
                            on={fareProfile.payment === p.id}
                            onClick={() => fareProfile.set(p.id, p.id === 'cash' ? 'common' : fareProfile.category)}>
                            {p.label}
                          </Chip>
                        )}
                      </For>
                    </div>
                    <Show when={fareProfile.payment === 'card'}>
                      <div class={styles.chips}>
                        <For each={CATEGORIES}>
                          {(c) => (
                            <Chip on={fareProfile.category === c.id} onClick={() => fareProfile.set('card', c.id)}>
                              {c.label}
                            </Chip>
                          )}
                        </For>
                      </div>
                    </Show>
                  </div>
                </Show>
              </Show>
            </div>
          </>
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
                <button onClick={() => choose({kind: 'place', id: hit.id}, hit.name)}>
                  <span class={styles.name}>{hit.name}</span>
                  <span class={styles.kind}>{hit.context ? `${hit.category} · ${hit.context}` : hit.category}</span>
                </button>
              </li>
            )}
          </For>
          <For each={stops()}>
            {(hit) => (
              <li>
                <button onClick={() => choose({kind: 'stop', ref: hit.id}, fullStopName(hit.name, hit.station))}>
                  <span class={styles.name}>{fullStopName(hit.name, hit.station)}</span>
                  <span class={styles.kind}>Parada</span>
                </button>
              </li>
            )}
          </For>
        </ul>
      </Show>

      <button class={styles.done} onClick={() => props.onClose?.()}>
        Listo
      </button>
    </div>
  );
}
