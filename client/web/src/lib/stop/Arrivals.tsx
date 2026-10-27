import {For, Match, onCleanup, Show, Switch} from 'solid-js';
import type {StopLine} from '~/lib/api/types';
import {href} from '~/lib/ui/href';
import ArrivalRow from './ArrivalRow';
import styles from './Arrivals.module.css';
import {ArrivalsSource} from './ArrivalsSource';
import ArrivalsSkeleton from './ArrivalsSkeleton';

export default function Arrivals(props: {stopId: string; split?: boolean}) {
  // eslint-disable-next-line solid/reactivity
  const source = new ArrivalsSource(props.stopId);
  onCleanup(() => source.close());

  // live -> bus, scheduled -> line
  const mapHref = (line: StopLine) => {
    const bus = line.arrivals.find((a) => a.live && a.bus)?.bus;
    return href.map({parada: props.stopId, ...(bus ? {bus: bus.id} : {linea: line.line})});
  };

  return (
    <section class={styles.root}>
      <h2>Próximas llegadas</h2>

      <Switch
        fallback={
          <>
            <Show when={source.liveFailed}>
              <p class={styles.note}>No pudimos actualizar el tiempo real. Lo que ves puede no estar al día.</p>
            </Show>
            <ul class='rows'>
              <For each={source.lines}>
                {(line) => (
                  <li>
                    <ArrivalRow line={line} split={props.split} href={mapHref(line)} />
                  </li>
                )}
              </For>
            </ul>
          </>
        }>
        <Match when={source.loading}>
          <p class='sr-only' role='status'>
            Buscando llegadas
          </p>
          <ArrivalsSkeleton rows={6} />
        </Match>
        <Match when={source.lines.length === 0 && source.liveFailed}>
          <p class={styles.note}>No pudimos traer las llegadas. Volvemos a intentar cada 30 segundos.</p>
        </Match>
        <Match when={source.lines.length === 0}>
          <p class={styles.note}>No hay llegadas previstas ahora.</p>
        </Match>
      </Switch>
    </section>
  );
}
