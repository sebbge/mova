import {For, Show} from 'solid-js';
import type {BusRoute, RouteShape} from '~/lib/api/types';
import {providerOf} from '~/lib/domain/providers';
import {companyTint} from '~/lib/ui/palette';
import parts from '~/lib/ui/sheet/parts.module.css';
import SheetHeader from '~/lib/ui/sheet/SheetHeader';
import SheetTabs from '~/lib/ui/sheet/SheetTabs';
import SourceCredit from '~/lib/ui/SourceCredit';
import {fullStopName, rawId, splitCompanyLine} from '~/lib/format';
import LineLead from './LineLead';
import styles from './LineSheet.module.css';
import RouteSkeleton from './RouteSkeleton';

const DIRECTIONS = [
  {id: '0', label: 'Ida'},
  {id: '1', label: 'Vuelta'},
];

export default function LineSheet(props: {
  line: string;
  variants: BusRoute[];
  shape: RouteShape | null;
  routeId: string | null;
  direction: number;
  origin: string | null;
  company?: string; // from where the user entered
  loading?: boolean; // skeletons until the direction is known
  onSelect?: (routeId: string, direction?: number) => void;
  onSelectStop?: (stopId: string, stopName?: string) => void;
}) {
  const inter = () => providerOf(props.routeId ?? '') === 'inter' || props.variants[0]?.region === 'uy';
  const split = () =>
    props.company
      ? splitCompanyLine(props.line, props.company)
      : inter()
        ? splitCompanyLine(props.line)
        : {line: props.line};
  const tint = () => companyTint(props.company || split().company);
  const directions = () => {
    const available = new Set(props.shape?.available_directions.map((d) => d.direction_id) ?? []);
    return DIRECTIONS.map((d) => ({...d, disabled: !available.has(Number(d.id)) || !props.routeId}));
  };

  return (
    <>
      <div class={`${parts.band} ${parts.onTint} ${styles.band}`} style={{'--sheet-header-bg': tint().header}}>
        <SheetHeader
          title={props.shape?.headsign ?? props.variants[0]?.route_long_name ?? `Línea ${split().line}`}
          subtitle={props.origin ? `desde ${props.origin}` : ''}
          loading={props.loading}
          lead={<LineLead line={split().line} company={split().company} />}
        />
      </div>

      <div class={`${styles.content} ${parts.onTint}`} style={{'--list': tint().list}}>
        <Show when={props.variants.length !== 1}>
          <section class={`${styles.block} ${styles.directions}`}>
            <span class={styles.label}>Dirección</span>
            <div class={`${styles.list} sheet-scroll`}>
              <Show
                when={props.variants.length > 0}
                fallback={
                  <>
                    <span class={`${styles.row} shimmer`} />
                    <span class={`${styles.row} shimmer`} />
                  </>
                }>
                <For each={props.variants}>
                  {(variant) => (
                    <button
                      classList={{[styles.active]: variant.route_id === props.routeId}}
                      onClick={() => props.onSelect?.(variant.route_id)}>
                      {variant.route_long_name ?? variant.route_id}
                    </button>
                  )}
                </For>
              </Show>
            </div>
          </section>
        </Show>

        <section class={`${styles.block} ${styles.stops}`}>
          <span class={styles.label}>Paradas</span>
          <div class={styles.tabs}>
            {/* no tab until a direction resolves */}
            <SheetTabs
              label='Sentido'
              tabs={directions()}
              active={props.loading ? '' : String(props.direction)}
              onSelect={(id) => props.routeId && props.onSelect?.(props.routeId, Number(id))}
              tab={(d) => d.label}
            />
          </div>

          <Show
            when={!props.loading && props.shape}
            fallback={
              <>
                <p class='sr-only' role='status'>
                  Cargando el recorrido
                </p>
                <div class='sheet-scroll'>
                  <RouteSkeleton />
                </div>
              </>
            }>
            {(shape) => (
              <ol class={`${styles.list} sheet-scroll`}>
                <For each={shape().stops}>
                  {(stop) => (
                    <li>
                      <button onClick={() => props.onSelectStop?.(stop.stop_id, stop.stop_name)}>
                        <span class={styles.id}>{rawId(stop.stop_id)}</span>
                        <span class={styles.name}>{fullStopName(stop.stop_name, stop.station)}</span>
                      </button>
                    </li>
                  )}
                </For>
              </ol>
            )}
          </Show>
        </section>

        <SourceCredit provider={props.routeId ? providerOf(props.routeId) : undefined} tinted />
      </div>
    </>
  );
}
