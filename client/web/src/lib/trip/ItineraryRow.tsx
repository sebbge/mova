import {For, Show} from 'solid-js';
import type {Itinerary} from '~/lib/api/types';
import {arrivalSpoken, clock, distance, duration, eta, fareLabel} from '~/lib/format';
import LineBadge from '~/lib/line/LineBadge';
import WalkIcon from '~/lib/ui/icons/WalkIcon';
import styles from './ItineraryRow.module.css';
import {firstBus} from './LiveBoard';

export default function ItineraryRow(props: {itinerary: Itinerary; live?: number | null; onOpen?: () => void}) {
  const bus = () => firstBus(props.itinerary);

  return (
    <button class={styles.row} onClick={() => props.onOpen?.()}>
      {/* duration and departure on top */}
      <span class={styles.top}>
        <span class={styles.duration}>{duration(props.itinerary.duration_s)}</span>
        <Show
          when={props.live != null && bus()}
          fallback={<span class={styles.depart}>Sale {clock(bus()?.departure ?? props.itinerary.departure)}</span>}>
          {(leg) => (
            <span class={`${styles.depart} ${styles.live}`}>
              <span aria-hidden='true'>
                El {leg().line} en {eta(props.live ?? undefined)}
              </span>
              <span class='sr-only'>
                El {leg().line} pasa {arrivalSpoken({eta: props.live ?? undefined})}
              </span>
            </span>
          )}
        </Show>
      </span>

      <span class={styles.legs}>
        <For each={props.itinerary.legs}>
          {(leg, i) => (
            <>
              <Show when={i() > 0}>
                <span class={styles.sep} aria-hidden='true'>
                  ›
                </span>
              </Show>
              {leg.mode === 'walk' ? (
                <span class={styles.walk} title={`A pie ${distance(leg.distance_m)}`}>
                  <WalkIcon size={16} />
                  <span>{Math.max(1, Math.round(leg.duration_s / 60))} min</span>
                </span>
              ) : (
                <LineBadge line={leg.line} size='sm' />
              )}
            </>
          )}
        </For>
      </span>

      <span class={styles.bottom}>
        <span class={styles.times}>
          {clock(props.itinerary.departure)} → {clock(props.itinerary.arrival)}
        </span>
        <span class={styles.dot} aria-hidden='true'>
          ·
        </span>
        <span class={styles.fare} classList={{[styles.unknown]: props.itinerary.fare.total === null}}>
          {fareLabel(props.itinerary.fare)}
        </span>
        <span class={styles.dot} aria-hidden='true'>
          ·
        </span>
        <span>{distance(props.itinerary.walk_m)} a pie</span>
      </span>
    </button>
  );
}
