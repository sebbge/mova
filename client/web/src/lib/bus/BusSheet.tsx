import {createSignal, For, onCleanup, Show} from 'solid-js';
import type {Bus, BusFreshness} from '~/lib/api/types';
import {ago, delayLabel, splitCompanyLine} from '~/lib/format';
import LineLead from '~/lib/line/LineLead';
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
  const split = () =>
    props.bus.company ? splitCompanyLine(props.bus.line, props.bus.company) : {line: props.bus.line};
  // operator delay vs position age
  const delay = () => delayLabel(props.bus.delay_min);
  const tags = () =>
    [props.bus.access, props.bus.emissions, props.bus.thermal_comfort].filter((t): t is string => Boolean(t));

  const [now, setNow] = createSignal(Date.now());
  const timer = setInterval(() => setNow(Date.now()), 5_000);
  onCleanup(() => clearInterval(timer));
  const signal = () => ago(props.bus.timestamp, now());

  return (
    <>
      <div class={`${parts.band} ${styles.band}`} style={{'--sheet-header-bg': tint().header}}>
        <SheetHeader
          title={props.bus.destination ?? `Línea ${split().line}`}
          subtitle={props.bus.origin ? `desde ${props.bus.origin}` : ''}
          lead={<LineLead line={split().line} company={split().company} />}
        />
      </div>

      <div class={`${styles.content} ${parts.onTint} sheet-scroll`} style={{'--list': tint().list}}>
        <p class={styles.state} style={{'--dot': freshnessVar(true, props.bus.freshness)}}>
          <span class={styles.dot} aria-hidden='true' />
          {CONFIDENCE[props.bus.freshness]}
          <Show when={signal()}>
            <span class={styles.ago}>Última señal: {signal()}</span>
          </Show>
        </p>

        <dl class={styles.facts}>
          <Show when={delay()}>
            <div>
              <dt>Horario</dt>
              <dd>{delay()}</dd>
            </div>
          </Show>
          <div>
            <dt>Velocidad</dt>
            <dd>
              {props.bus.still
                ? 'Detenido'
                : props.bus.speed === undefined
                  ? '—'
                  : `${Math.round(props.bus.speed)} km/h`}
            </dd>
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
