import {createSignal, For, Show} from 'solid-js';
import type {Bus, BusRoute, RouteShape, StopLine} from '~/lib/api/types';
import {providerOf} from '~/lib/domain/providers';
import {isSelected, type StopSelection} from '~/lib/domain/selection';
import {fullStopName, splitCompanyLine} from '~/lib/format';
import {companyListVar, companySheetVar} from '~/lib/ui/palette';
import parts from '~/lib/ui/sheet/parts.module.css';
import SheetHeader from '~/lib/ui/sheet/SheetHeader';
import SheetTabs from '~/lib/ui/sheet/SheetTabs';
import SourceCredit from '~/lib/ui/SourceCredit';
import LineLead from './LineLead';
import styles from './LineSheet.module.css';
import RouteSkeleton from './RouteSkeleton';
import StopTrack from './StopTrack';

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
  selected?: StopSelection;
  times?: (number | null)[]; // aligned with shape.stops
  buses?: Bus[];
  arrivals?: StopLine[]; // selected stop's
  arrivalsLoading?: boolean;
  onSelect?: (routeId: string, direction?: number) => void;
  onSelectStop?: (stopId: string, stopName?: string) => void;
  onOpenStop?: (stopId: string, stopName?: string) => void; // second tap on the selected stop
  onSchedule?: (stopId: string) => void;
}) {
  const [picking, setPicking] = createSignal(false);

  const inter = () => providerOf(props.routeId ?? '') === 'inter' || props.variants[0]?.region === 'uy';
  const split = () =>
    props.company
      ? splitCompanyLine(props.line, props.company)
      : inter()
        ? splitCompanyLine(props.line)
        : {line: props.line};
  // bus sheet's color
  const route = () => companySheetVar(props.company || split().company);
  // both always, a missing one is information
  const directions = () => {
    const available = new Set(props.shape?.available_directions.map((d) => d.direction_id) ?? []);
    return DIRECTIONS.map((d) => ({...d, disabled: !available.has(Number(d.id)) || !props.routeId}));
  };
  const here = () => {
    const selected = props.selected;
    return props.shape && selected ? props.shape.stops.findIndex((s) => isSelected(selected, s.stop_id)) : -1;
  };
  const scheduleStop = () => (props.shape && !props.loading && here() >= 0 ? props.shape.stops[here()] : undefined);
  const current = () => props.variants.find((v) => v.route_id === props.routeId);

  function pick(routeId: string) {
    setPicking(false);
    props.onSelect?.(routeId);
  }

  return (
    <>
      <div class={`${parts.band} ${styles.band}`} style={{'--sheet-header-bg': route()}}>
        <SheetHeader
          title={props.shape?.headsign ?? props.variants[0]?.route_long_name ?? `Línea ${split().line}`}
          subtitle={props.origin ? `desde ${props.origin}` : ''}
          loading={props.loading}
          lead={<LineLead line={split().line} company={split().company} />}
        />
      </div>

      <div
        class={`${styles.content} ${parts.onTint}`}
        style={{'--list': companyListVar(props.company || split().company), '--route': route()}}>
        <Show when={props.variants.length !== 1}>
          <section class={styles.variants}>
            <button
              class={styles.picker}
              aria-expanded={picking()}
              aria-controls='line-variants'
              disabled={props.variants.length === 0}
              onClick={() => setPicking(!picking())}>
              <span class={styles.label}>Recorrido</span>
              <Show when={props.variants.length > 0} fallback={<span class={`${styles.current} shimmer`} />}>
                <span class={styles.current}>{current()?.route_long_name ?? current()?.route_id ?? 'Elegí uno'}</span>
              </Show>
              <svg class={styles.chevron} classList={{[styles.up]: picking()}} viewBox='0 0 24 24' aria-hidden='true'>
                <path d='m6 9 6 6 6-6' />
              </svg>
            </button>
            <Show when={picking()}>
              {/* scrolls with many variants */}
              <div id='line-variants' class={`${styles.options} sheet-scroll`}>
                <For each={props.variants}>
                  {(variant) => (
                    <button
                      classList={{[styles.active]: variant.route_id === props.routeId}}
                      aria-current={variant.route_id === props.routeId || undefined}
                      onClick={() => pick(variant.route_id)}>
                      {variant.route_long_name ?? variant.route_id}
                    </button>
                  )}
                </For>
              </div>
            </Show>
          </section>
        </Show>

        <section class={styles.stops}>
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
              <StopTrack
                shape={shape()}
                line={props.line}
                here={here()}
                times={props.times ?? []}
                buses={props.buses ?? []}
                arrivals={props.arrivals ?? []}
                arrivalsLoading={props.arrivalsLoading ?? false}
                onSelectStop={props.onSelectStop}
                onOpenStop={props.onOpenStop}
              />
            )}
          </Show>
        </section>

        <Show when={scheduleStop()}>
          {(stop) => (
            <div class={styles.footer}>
              <button class={styles.schedule} onClick={() => props.onSchedule?.(stop().stop_id)}>
                <span class={styles.scheduleTitle}>Ver planilla completa</span>
                <span class={styles.scheduleFor}>para {fullStopName(stop().stop_name, stop().station)}</span>
              </button>
            </div>
          )}
        </Show>

        <SourceCredit provider={props.routeId ? providerOf(props.routeId) : undefined} tinted />
      </div>
    </>
  );
}
