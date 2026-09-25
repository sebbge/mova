import {createEffect, createSignal, on, Show, untrack} from 'solid-js';
import type {Bus, BusFreshness} from '~/lib/api/types';
import {companyOutlineVar, companyVar} from '~/lib/ui/palette';
import styles from './BusPin.module.css';

const PRECISION: Record<BusFreshness, string> = {
  live: '',
  low_precision: ', posición aproximada',
  imprecise: ', posición imprecisa',
};

export default function BusPin(props: {bus: Bus; dimmed?: boolean; onSelect?: (bus: Bus) => void}) {
  const hasDirection = () => props.bus.bearing_deg != null && !props.bus.still;

  // shortest turn, 350° -> 10° is 20°
  let heading = untrack(() => props.bus.bearing_deg) ?? 0;
  const [rotation, setRotation] = createSignal(heading);
  createEffect(
    on(
      () => props.bus.bearing_deg,
      (target) => {
        if (target == null) return;
        heading += ((((target - heading) % 360) + 540) % 360) - 180;
        setRotation(heading);
      },
      {defer: true}
    )
  );

  // or the map reads a background click
  const select = (e: MouseEvent) => {
    e.stopPropagation();
    props.onSelect?.(props.bus);
  };

  return (
    <button
      class={`${styles.pin} ${styles[props.bus.freshness] ?? ''}`}
      classList={{[styles.still]: props.bus.still, [styles.dimmed]: Boolean(props.dimmed)}}
      style={{'--pin': companyVar(props.bus.company), '--pin-outline': companyOutlineVar(props.bus.company)}}
      onClick={select}
      title={`${props.bus.line} ${props.bus.destination ?? ''} - ${props.bus.company}`}
      aria-label={`Bus de la línea ${props.bus.line}${PRECISION[props.bus.freshness]}`}>
      <Show when={hasDirection()}>
        <span class={styles.dir} style={{rotate: `${rotation()}deg`}} aria-hidden='true'>
          <svg viewBox='0 0 12 12' width='12' height='12'>
            <path d='M6 0 L11 9 L6 7 L1 9 Z' />
          </svg>
        </span>
      </Show>

      <span
        class={styles.badge}
        classList={{[styles.long]: props.bus.line.length > 3, [styles.xlong]: props.bus.line.length > 4}}>
        {props.bus.line}
      </span>
    </button>
  );
}
