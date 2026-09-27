import {For, Match, Show, Switch} from 'solid-js';
import type {Stop, StopLine} from '~/lib/api/types';
import {orderedRefs, providerInfo, providerOf, serviceOf, stopKicker} from '~/lib/domain/providers';
import {arrivalLabel, arrivalTitle, rawId, splitCompanyLine} from '~/lib/format';
import LineBadge from '~/lib/line/LineBadge';
import MetaIcons from '~/lib/ui/icons/MetaIcons';
import ServiceIcon from '~/lib/ui/icons/ServiceIcon';
import StopIcon from '~/lib/ui/icons/StopIcon';
import {freshnessVar} from '~/lib/ui/palette';
import SourceCredit from '~/lib/ui/SourceCredit';
import SheetHeader from '~/lib/ui/sheet/SheetHeader';
import SheetTabs from '~/lib/ui/sheet/SheetTabs';
import ArrivalsSkeleton from './ArrivalsSkeleton';
import styles from './StopSheet.module.css';

export default function StopSheet(props: {
  stop: Stop | null;
  stopRef: string; // before the backend answers
  stopName?: string; // skips "loading"
  arrivals: StopLine[];
  loading: boolean;
  onSelectLine?: (line: string, destination?: string, company?: string) => void;
  onSelectService?: (ref: string) => void; // same shelter, already loaded
}) {
  const ref = () => props.stop?.stop_id ?? props.stopRef;
  const service = () => serviceOf(ref());
  const title = () => props.stop?.stop_name ?? props.stopName ?? 'Cargando…';
  const current = () => providerInfo(providerOf(ref()));
  const tabs = () =>
    orderedRefs(props.stop?.siblings ?? []).map((r) => {
      const info = providerInfo(providerOf(r));
      return {id: r, letter: info.letter, label: info.label};
    });

  return (
    <>
      <div class={`${styles.band} ${styles[service()] ?? ''}`}>
        <SheetHeader
          kicker={stopKicker(ref(), title(), rawId(props.stop?.siblings?.[0] ?? ref()), props.stop?.station)}
          title={title()}
          lead={<StopIcon />}
        />

        <Show
          when={tabs().length >= 2}
          fallback={
            <Show when={current().only}>
              <p class={styles.only}>
                <ServiceIcon letter={current().letter} />
                {current().only}
              </p>
            </Show>
          }>
          <SheetTabs
            label='Tipo de servicio'
            tabs={tabs()}
            active={ref()}
            onSelect={(r) => props.onSelectService?.(r)}
            tab={(t) => (
              <>
                <ServiceIcon letter={t.letter} />
                {t.label}
              </>
            )}
          />
        </Show>
      </div>

      <div class={`${styles.list} ${styles[service()] ?? ''}`}>
        <Switch
          fallback={
            <ul class={`${styles.lines} sheet-scroll`}>
              <For each={props.arrivals}>
                {(line) => {
                  const first = () => line.arrivals[0];
                  const label = () => (service() === 'inter' ? splitCompanyLine(line.line) : {line: line.line});
                  return (
                    <li>
                      <button
                        onClick={() => props.onSelectLine?.(line.line, line.destination ?? undefined, line.company)}>
                        <LineBadge
                          line={label().line}
                          company={line.company || label().company}
                          size='sm'
                          variant='flat'
                        />
                        <span class={styles.info}>
                          <Show when={label().company}>
                            <span class={styles.company}>{label().company}</span>
                          </Show>
                          <span class={styles.destination}>{line.destination ?? ''}</span>
                          <MetaIcons bus={first()?.bus} />
                        </span>
                        <span class={styles.etas}>
                          <span
                            class={styles.eta}
                            classList={{[styles.scheduleGuess]: Boolean(first()?.zonal || first()?.interpolated)}}
                            style={{color: freshnessVar(first()?.live ?? false, first()?.bus?.freshness)}}
                            title={arrivalTitle(first())}>
                            {arrivalLabel(first())}
                          </span>
                          <Show when={line.arrivals.length > 1}>
                            <span class={styles.next}>
                              <For each={line.arrivals.slice(1, 3)}>
                                {(arrival) => (
                                  <span style={{color: freshnessVar(arrival.live, arrival.bus?.freshness)}}>
                                    {arrivalLabel(arrival)}
                                  </span>
                                )}
                              </For>
                            </span>
                          </Show>
                        </span>
                      </button>
                    </li>
                  );
                }}
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
            <p class={styles.note}>No hay llegadas previstas ahora.</p>
          </Match>
        </Switch>
        <SourceCredit provider={providerOf(ref())} tinted />
      </div>
    </>
  );
}
