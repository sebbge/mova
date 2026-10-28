import {createSignal, For, Show, untrack} from 'solid-js';
import type {TravelMode} from '~/lib/api/types';
import {localIso} from '~/lib/format';
import {slide, transition} from '~/lib/ui/motion';
import {CATEGORIES, fareProfile, PAYMENTS} from './profile';
import styles from './TripOptions.module.css';

function Chip(props: {on: boolean; onClick: () => void; children: string}) {
  return (
    <button
      class={styles.chip}
      classList={{[styles.on]: props.on}}
      aria-pressed={props.on}
      onClick={() => props.onClick()}>
      {props.children}
    </button>
  );
}

export default function TripOptions(props: {mode: TravelMode; at: string | null; onAt?: (at: string | null) => void}) {
  const opened = untrack(() => props.at); // opens with the trip's time
  const [when, setWhen] = createSignal<'now' | 'depart'>(opened ? 'depart' : 'now');
  const [stamp, setStamp] = createSignal(opened ?? localIso(new Date()));
  const [fareOpen, setFareOpen] = createSignal(false);

  function pickWhen(next: 'now' | 'depart') {
    setWhen(next);
    props.onAt?.(next === 'now' ? null : stamp());
  }

  return (
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
          <div
            id='fare-options'
            ref={(el) => transition(el, slide, {enter: 180, leave: 180})}
            class={styles.fareOptions}>
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
  );
}
