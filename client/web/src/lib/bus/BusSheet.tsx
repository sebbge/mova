import {For, Show} from 'solid-js';
import type {Bus, BusFreshness} from '~/lib/api/types';
import LineBadge from '~/lib/line/LineBadge';
import {freshnessVar} from '~/lib/ui/palette';
import parts from '~/lib/ui/sheet/parts.module.css';
import styles from './BusSheet.module.css';

const CONFIDENCE: Record<BusFreshness, string> = {
  live: 'Posición en vivo',
  low_precision: 'Posición aproximada',
  imprecise: 'Última posición conocida',
};

export default function BusSheet(props: {bus: Bus; onSelectLine?: (line: string, destination?: string) => void}) {
  const tags = () =>
    [props.bus.access, props.bus.emissions, props.bus.thermal_comfort].filter((t): t is string => Boolean(t));

  return (
    <>
      <header class={parts.header}>
        <LineBadge line={props.bus.line} company={props.bus.company} />
        <div class={parts.meta}>
          <span class={parts.title}>{props.bus.destination ?? `Línea ${props.bus.line}`}</span>
          <Show when={props.bus.origin}>
            <span class={parts.sub}>desde {props.bus.origin}</span>
          </Show>
        </div>
      </header>

      <dl class={styles.facts}>
        <div>
          <dt>Estado</dt>
          <dd style={{color: freshnessVar(true, props.bus.freshness)}}>{CONFIDENCE[props.bus.freshness]}</dd>
        </div>
        <div>
          <dt>Velocidad</dt>
          <dd>{props.bus.still ? 'Detenido' : `${Math.round(props.bus.speed)} km/h`}</dd>
        </div>
        <div>
          <dt>Empresa</dt>
          <dd>{props.bus.company}</dd>
        </div>
        <div>
          <dt>Unidad</dt>
          <dd>{props.bus.id}</dd>
        </div>
      </dl>

      <Show when={tags().length > 0}>
        <ul class={styles.tags}>
          <For each={tags()}>{(tag) => <li>{tag}</li>}</For>
        </ul>
      </Show>

      <button
        class={parts.full}
        onClick={() => props.onSelectLine?.(props.bus.line, props.bus.destination ?? undefined)}>
        Ver el recorrido de la línea {props.bus.line}
      </button>
    </>
  );
}
