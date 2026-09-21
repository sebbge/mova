import {For, Match, Switch} from 'solid-js';
import {eta} from '~/lib/format';
import {freshnessVar} from '~/lib/ui/palette';
import styles from './Arrivals.module.css';
import {ArrivalsSource} from './ArrivalsSource';
import ArrivalsSkeleton from './ArrivalsSkeleton';

export default function Arrivals(props: {stopId: string}) {
  // eslint-disable-next-line solid/reactivity
  const source = new ArrivalsSource(props.stopId);

  return (
    <section class={styles.root}>
      <h2>Próximas llegadas</h2>

      <Switch
        fallback={
          <ul class={styles.lines}>
            <For each={source.lines}>
              {(line) => (
                <li>
                  <span class={styles.line}>{line.line}</span>
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
                </li>
              )}
            </For>
          </ul>
        }>
        <Match when={source.loading}>
          <p class='sr-only' role='status'>
            Buscando llegadas
          </p>
          <ArrivalsSkeleton />
        </Match>
        <Match when={source.lines.length === 0 && source.liveFailed}>
          <p class={styles.note}>No pudimos traer las llegadas. Los horarios de las líneas siguen abajo.</p>
        </Match>
        <Match when={source.lines.length === 0}>
          <p class={styles.note}>No hay llegadas previstas ahora.</p>
        </Match>
      </Switch>
    </section>
  );
}
