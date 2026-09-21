import {For, Show} from 'solid-js';
import type {BusRoute, RouteShape} from '~/lib/api/types';
import {href} from '~/lib/ui/href';
import parts from '~/lib/ui/sheet/parts.module.css';
import LineBadge from './LineBadge';
import styles from './LineSheet.module.css';

const DIRECTIONS = [
  {id: 0, label: 'Ida'},
  {id: 1, label: 'Vuelta'},
];

export default function LineSheet(props: {
  line: string;
  variants: BusRoute[];
  shape: RouteShape | null;
  routeId: string | null;
  direction: number;
  origin: string | null;
  region: string;
  onSelect?: (routeId: string, direction?: number) => void;
  onSelectStop?: (stopId: string) => void;
}) {
  const available = () => new Set(props.shape?.available_directions.map((d) => d.direction_id) ?? []);
  const title = () => props.shape?.headsign ?? props.variants[0]?.route_long_name ?? `Línea ${props.line}`;

  return (
    <>
      <header class={parts.header}>
        <LineBadge line={props.line} />
        <div class={parts.meta}>
          <span class={parts.title}>{title()}</span>
          <Show when={props.origin}>
            <span class={parts.sub}>desde {props.origin}</span>
          </Show>
        </div>
      </header>

      <Show when={props.variants.length > 1}>
        <section class={styles.variants}>
          <span class={styles.label}>Dirección</span>
          <For each={props.variants}>
            {(variant) => (
              <button
                classList={{[styles.active]: variant.route_id === props.routeId}}
                onClick={() => props.onSelect?.(variant.route_id)}>
                {variant.route_long_name ?? variant.route_id}
              </button>
            )}
          </For>
        </section>
      </Show>

      {/* both always, a missing one is information */}
      <nav class={styles.directions} aria-label='Sentido'>
        <For each={DIRECTIONS}>
          {(d) => (
            <button
              classList={{[styles.active]: d.id === props.direction}}
              disabled={!available().has(d.id) || !props.routeId}
              aria-current={d.id === props.direction ? 'true' : undefined}
              onClick={() => props.routeId && props.onSelect?.(props.routeId, d.id)}>
              {d.label}
            </button>
          )}
        </For>
      </nav>

      <Show when={props.shape} fallback={<p class={parts.note}>Cargando el recorrido…</p>}>
        {(shape) => (
          <ol class={parts.list}>
            <For each={shape().stops}>
              {(stop) => (
                <li>
                  <button class={`${parts.row} ${styles.stop}`} onClick={() => props.onSelectStop?.(stop.stop_id)}>
                    <span class={styles.id}>{stop.stop_id}</span>
                    <span class={styles.name}>{stop.stop_name}</span>
                  </button>
                </li>
              )}
            </For>
          </ol>
        )}
      </Show>

      <a class={parts.full} href={href.line(props.region, props.line)}>
        Ver la línea completa
      </a>
    </>
  );
}
