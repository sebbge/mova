import {createEffect, createResource, For, Match, Switch} from 'solid-js';
import {api} from '~/lib/api/client';
import type {Region} from '~/lib/domain/regions';
import {eta} from '~/lib/format';
import LineBadge from '~/lib/line/LineBadge';
import {href} from '~/lib/ui/href';
import {freshnessVar} from '~/lib/ui/palette';
import styles from './StopArrivals.module.css';

export default function StopArrivals(props: {region: Region}) {
  const stopId = new URLSearchParams(location.search).get('id') ?? '';

  const [detail] = createResource(stopId || undefined, (id) => api.stopDetail(id));
  const lines = () => detail()?.lines ?? [];

  createEffect(() => {
    document.title = `Parada ${stopId}, ${props.region.name} | bondi`;
  });

  return (
    <>
      <div>
        <span class={styles.kicker}>Parada</span>
        <h1 class={styles.id}>{stopId}</h1>
      </div>

      <Switch>
        <Match when={!stopId || detail.error}>
          <p class={styles.empty}>No pudimos cargar las llegadas de esta parada. Puede ser un problema de conexión.</p>
        </Match>
        <Match when={detail() && lines().length === 0}>
          <p class={styles.empty}>No hay líneas registradas en esta parada.</p>
        </Match>
        <Match when={lines().length > 0}>
          <ul class={styles.lines}>
            <For each={lines()}>
              {(line) => (
                <li>
                  <a class={styles.line} href={href.line(props.region.slug, line.line)}>
                    <LineBadge line={line.line} company={line.company} />
                    <span class={styles.destination}>{line.destination ?? ''}</span>
                  </a>
                  <ul class={styles.arrivals}>
                    <For each={line.arrivals}>
                      {(arrival) => (
                        <li style={{color: freshnessVar(arrival.live, arrival.bus?.freshness)}}>{eta(arrival.eta)}</li>
                      )}
                    </For>
                  </ul>
                </li>
              )}
            </For>
          </ul>
        </Match>
      </Switch>
    </>
  );
}
