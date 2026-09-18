import {createEffect, createResource, For, Match, Show, Switch} from 'solid-js';
import {api} from '~/lib/api/client';
import type {DirectionInfo} from '~/lib/api/types';
import type {Region} from '~/lib/domain/regions';
import {stopName} from '~/lib/format';
import {href} from '~/lib/ui/href';
import LineBadge from './LineBadge';
import styles from './LineRoute.module.css';

const SINGLE_DIRECTION: DirectionInfo[] = [{direction_id: 0}];

export default function LineRoute(props: {region: Region}) {
  const params = new URLSearchParams(location.search);
  const short = params.get('l') ?? '';
  const direction = Number(params.get('d') ?? 0);

  const [variants] = createResource(short || undefined, async (q) =>
    (await api.searchRoutes(q)).filter((r) => r.route_short_name === q)
  );
  const routeId = () => params.get('v') ?? variants()?.[0]?.route_id;
  const [shape] = createResource(routeId, (id) => api.routeShape(id, direction));

  const longName = () => variants()?.[0]?.route_long_name ?? '';
  const directions = () => {
    const available = shape()?.available_directions ?? [];
    return available.length > 0 ? available : SINGLE_DIRECTION;
  };

  createEffect(() => {
    document.title = `Línea ${short}${longName() ? `, ${longName()}` : ''} | bondi`;
  });

  return (
    <>
      <div class={styles.head}>
        <LineBadge line={short} />
        <h1 class={styles.title}>{longName()}</h1>
      </div>

      <Show when={variants.state === 'ready' || variants.error || !short}>
        <Switch>
          <Match when={!variants()?.length}>
            <p class={styles.empty}>No encontramos la línea {short}.</p>
          </Match>
          <Match when={variants()}>
            {(found) => (
              <>
                <Show when={found().length > 1 || directions().length > 1}>
                  <nav class={styles.variants} aria-label='Recorrido'>
                    <For each={found()}>
                      {(variant) => (
                        <For each={directions()}>
                          {(dir) => (
                            <a
                              href={href.line(props.region.slug, short, {
                                route: variant.route_id,
                                direction: dir.direction_id,
                              })}
                              aria-current={
                                variant.route_id === routeId() && dir.direction_id === direction ? 'true' : undefined
                              }>
                              {dir.headsign ?? (dir.direction_id === 0 ? 'Ida' : 'Vuelta')}
                            </a>
                          )}
                        </For>
                      )}
                    </For>
                  </nav>
                </Show>

                <Switch>
                  <Match when={shape.error}>
                    <p class={styles.empty}>No pudimos cargar el recorrido.</p>
                  </Match>
                  <Match when={shape()}>
                    {(route) => (
                      <ol class={styles.stops}>
                        <For each={route().stops}>
                          {(stop) => (
                            <li>
                              <a href={href.stop(props.region.slug, stop.stop_id)}>
                                <span class={styles.stopId}>{stop.stop_id}</span>
                                <span class={styles.stopName}>{stopName(stop.stop_name)}</span>
                              </a>
                            </li>
                          )}
                        </For>
                      </ol>
                    )}
                  </Match>
                </Switch>
              </>
            )}
          </Match>
        </Switch>
      </Show>
    </>
  );
}
