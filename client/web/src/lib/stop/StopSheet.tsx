import {For, Match, Show, Switch} from 'solid-js';
import type {Stop, StopLine} from '~/lib/api/types';
import {eta} from '~/lib/format';
import LineBadge from '~/lib/line/LineBadge';
import {href} from '~/lib/ui/href';
import {freshnessVar} from '~/lib/ui/palette';
import parts from '~/lib/ui/sheet/parts.module.css';
import ArrivalsSkeleton from './ArrivalsSkeleton';
import styles from './StopSheet.module.css';

export default function StopSheet(props: {
  stop: Stop | null;
  arrivals: StopLine[];
  loading: boolean;
  region: string;
  onSelectLine?: (line: string, destination?: string) => void;
}) {
  return (
    <>
      <header class={styles.header}>
        <span class={parts.kicker}>Parada {props.stop?.stop_id ?? ''}</span>
        <h2 class={styles.name}>{props.stop?.stop_name ?? 'Cargando…'}</h2>
      </header>

      <Switch
        fallback={
          <ul class={parts.list}>
            <For each={props.arrivals}>
              {(line) => (
                <li>
                  <button
                    class={`${parts.row} ${styles.line}`}
                    onClick={() => props.onSelectLine?.(line.line, line.destination ?? undefined)}>
                    <LineBadge line={line.line} company={line.company} size='sm' />
                    <span class={styles.destination}>{line.destination ?? ''}</span>
                    <span class={styles.etas}>
                      <For each={line.arrivals.slice(0, 3)}>
                        {(arrival, i) => (
                          <span
                            class={styles.eta}
                            classList={{[styles.first]: i() === 0}}
                            style={{color: freshnessVar(arrival.live, arrival.bus?.freshness)}}
                            title={arrival.live ? 'Posición en vivo' : 'Horario de planilla'}>
                            {eta(arrival.eta)}
                          </span>
                        )}
                      </For>
                    </span>
                  </button>
                </li>
              )}
            </For>
          </ul>
        }>
        <Match when={props.loading && props.arrivals.length === 0}>
          <p class='sr-only' role='status'>
            Buscando llegadas
          </p>
          <ArrivalsSkeleton />
        </Match>
        <Match when={props.arrivals.length === 0}>
          <p class={parts.note}>No hay llegadas previstas ahora.</p>
        </Match>
      </Switch>

      <Show when={props.stop}>
        {(stop) => (
          <a class={parts.full} href={href.stop(props.region, stop().stop_id)}>
            Ver la parada completa
          </a>
        )}
      </Show>
    </>
  );
}
