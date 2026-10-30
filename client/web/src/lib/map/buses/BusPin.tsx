import {createEffect, createSignal, For, on, Show, untrack} from 'solid-js';
import type {Bus, BusFreshness} from '~/lib/api/types';
import {delayLabel, splitCompanyLine} from '~/lib/format';
import MapTip from '~/lib/map/MapTip';
import BusIcon from '~/lib/ui/icons/BusIcon';
import MetaIcon from '~/lib/ui/icons/MetaIcon';
import {metaTags} from '~/lib/ui/icons/MetaIcons';
import {companyOutlineVar, companyPinVar} from '~/lib/ui/palette';
import styles from './BusPin.module.css';

const PRECISION: Record<BusFreshness, string> = {
  live: '',
  low_precision: ', posición aproximada',
  imprecise: ', posición imprecisa',
};

export default function BusPin(props: {
  bus: Bus;
  dimmed?: boolean; // off route, or another bus in focus
  receded?: boolean; // far or off route, 44px tap
  compact?: boolean; // known line, icon only
  selected?: boolean; // open bus
  onSelect?: (bus: Bus) => void;
}) {
  const hasDirection = () => props.bus.bearing_deg != null;
  const parts = () =>
    props.bus.company ? splitCompanyLine(props.bus.line, props.bus.company) : {line: props.bus.line};
  const number = () => parts().line;

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

  const [hovering, setHovering] = createSignal(false); // mouse only, tip built lazily

  // ellipse from the number length, no dom measure
  const reach = () => {
    const n = number().length;
    const a = props.compact ? 14 : Math.max(14, (12 + n * (n > 4 ? 6.6 : 7.2)) / 2);
    const t = (rotation() * Math.PI) / 180;
    return 1 / Math.hypot(Math.sin(t) / a, Math.cos(t) / 14) + 3;
  };

  // or the map reads a background click
  const select = (e: MouseEvent) => {
    e.stopPropagation();
    props.onSelect?.(props.bus);
  };

  return (
    <>
      <button
        class={`${styles.pin} ${styles[props.bus.freshness] ?? ''}`}
        classList={{
          [styles.still]: props.bus.still,
          [styles.dimmed]: Boolean(props.dimmed),
          [styles.receded]: Boolean(props.receded),
          [styles.selected]: Boolean(props.selected),
        }}
        style={{'--pin': companyPinVar(props.bus.company), '--pin-outline': companyOutlineVar(props.bus.company)}}
        onClick={select}
        onPointerEnter={(e) => setHovering(e.pointerType === 'mouse')}
        onPointerLeave={() => setHovering(false)}
        aria-label={`Bus de la línea ${props.bus.line}${PRECISION[props.bus.freshness]}`}>
        <Show when={hasDirection()}>
          <span class={styles.dir} style={{rotate: `${rotation()}deg`, '--reach': `${reach()}px`}} aria-hidden='true'>
            <svg viewBox='0 0 12 12' width='12' height='12'>
              <path d='M6 0 L11 9 L6 7 L1 9 Z' />
            </svg>
          </span>
        </Show>

        <Show
          when={props.compact}
          fallback={
            <span class={styles.badge} classList={{[styles.long]: number().length > 4}}>
              {number()}
            </span>
          }>
          <span class={`${styles.badge} ${styles.compact}`}>
            <BusIcon />
          </span>
        </Show>

        <Show when={parts().company && !props.compact}>
          <span class={styles.company} aria-hidden='true'>
            {props.bus.company}
          </span>
        </Show>
      </button>

      {/* outside the button, skips dimmed opacity */}
      <Show when={hovering()}>
        <div class={styles.tip} style={{'--pin': companyPinVar(props.bus.company)}}>
          <MapTip
            text={props.bus.destination ?? `Línea ${number()}`}
            subtitle={delayLabel(props.bus.delay_min)}
            lead={<span class={styles.tipLine}>{number()}</span>}
            trail={
              <For each={metaTags(props.bus)}>
                {(tag) => (
                  <span class={styles.meta} style={{'--meta': `var(--meta-${tag.kind}-deep)`}}>
                    <MetaIcon kind={tag.kind} size={12} />
                  </span>
                )}
              </For>
            }
          />
        </div>
      </Show>
    </>
  );
}
