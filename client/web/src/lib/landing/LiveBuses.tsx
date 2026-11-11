import {createResource, Show} from 'solid-js';
import {api} from '~/lib/api/client';
import styles from './LiveBuses.module.css';

const count = (n: number) => n.toLocaleString('es-UY');

export default function LiveBuses(props: {region: string}) {
  // null: no figure to show
  const [live] = createResource(
    () => props.region,
    (region) =>
      api.regionBuses(region).then(
        (buses) => buses.length || null,
        () => null
      )
  );

  return (
    <Show when={live() !== null}>
      <p class={styles.live} classList={{shimmer: live.loading}}>
        <Show when={live()}>
          {(n) => (
            <>
              <span class={styles.dot} aria-hidden='true' />
              {count(n())} {n() === 1 ? 'bus' : 'buses'} en vivo ahora
            </>
          )}
        </Show>
      </p>
    </Show>
  );
}
