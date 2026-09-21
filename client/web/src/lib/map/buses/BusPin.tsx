import {Show} from 'solid-js';
import type {Bus, BusFreshness} from '~/lib/api/types';
import {companyVar} from '~/lib/ui/palette';
import styles from './BusPin.module.css';

const PRECISION: Record<BusFreshness, string> = {
  live: '',
  low_precision: ', posición aproximada',
  imprecise: ', posición imprecisa',
};

export default function BusPin(props: {bus: Bus; onSelect?: (bus: Bus) => void}) {
  const hasDirection = () => props.bus.bearing_deg != null && !props.bus.still;

  return (
    <button
      class={`${styles.pin} ${styles[props.bus.freshness] ?? ''}`}
      style={{'--pin': companyVar(props.bus.company)}}
      onClick={() => props.onSelect?.(props.bus)}
      title={`Línea ${props.bus.line} · ${props.bus.company}${props.bus.destination ? ` → ${props.bus.destination}` : ''}`}
      aria-label={`Bus de la línea ${props.bus.line}${PRECISION[props.bus.freshness]}`}>
      <Show when={hasDirection()}>
        <span class={styles.dir} style={{rotate: `${props.bus.bearing_deg}deg`}} aria-hidden='true'>
          <svg viewBox='0 0 12 12' width='12' height='12'>
            <path d='M6 0 L11 9 L6 7 L1 9 Z' />
          </svg>
        </span>
      </Show>

      <span class={styles.badge}>{props.bus.line}</span>

      <Show when={props.bus.still}>
        <span class={styles.halt} aria-hidden='true' />
      </Show>
    </button>
  );
}
