import {For, Show} from 'solid-js';
import type {Bus, BusFreshness} from '~/lib/api/types';
import LineBadge from '~/lib/line/LineBadge';
import {delayLabel} from '~/lib/format';
import {companyTint, freshnessVar} from '~/lib/ui/palette';
import parts from '~/lib/ui/sheet/parts.module.css';
import SheetHeader from '~/lib/ui/sheet/SheetHeader';
import styles from './BusSheet.module.css';

const CONFIDENCE: Record<BusFreshness, string> = {
  live: 'Posición en vivo',
  low_precision: 'Posición aproximada',
  imprecise: 'Última posición conocida',
};

export default function BusSheet(props: {bus: Bus; onShowLine?: () => void}) {
  const tint = () => companyTint(props.bus.company);
  const tags = () =>
    [delayLabel(props.bus.delay_min), props.bus.access, props.bus.emissions, props.bus.thermal_comfort].filter(
      (t): t is string => Boolean(t)
    );

  return (
    <>
      <div class={parts.band} style={{'--sheet-header-bg': tint().header}}>
        <SheetHeader
          title={props.bus.destination ?? `Línea ${props.bus.line}`}
          subtitle={props.bus.origin ? `desde ${props.bus.origin}` : ''}
          lead={<LineBadge line={props.bus.line} variant='plain' />}
        />
      </div>

      <div class={`${styles.content} ${parts.onTint} sheet-scroll`} style={{'--list': tint().list}}>
        <p class={styles.state} style={{'--dot': freshnessVar(true, props.bus.freshness)}}>
          <span class={styles.dot} aria-hidden='true' />
          {CONFIDENCE[props.bus.freshness]}
        </p>

        <dl class={styles.facts}>
          <div>
            <dt>Velocidad</dt>
            <dd>{props.bus.still ? 'Detenido' : `${Math.round(props.bus.speed)} km/h`}</dd>
          </div>
          <div>
            <dt>Empresa</dt>
            <dd>{props.bus.company || '—'}</dd>
          </div>
          <div>
            <dt>Unidad</dt>
            <dd class={styles.unit}>{props.bus.unit}</dd>
          </div>
        </dl>

        <Show when={tags().length > 0}>
          <ul class={styles.tags}>
            <For each={tags()}>{(tag) => <li>{tag}</li>}</For>
          </ul>
        </Show>
      </div>

      {/* outside the scroll */}
      <Show when={props.onShowLine}>
        {(onShowLine) => (
          <div class={styles.footer} style={{'--list': tint().list}}>
            <button class={styles.line} onClick={() => onShowLine()()}>
              Ver línea completa
            </button>
          </div>
        )}
      </Show>
    </>
  );
}
