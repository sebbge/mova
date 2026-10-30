import {createEffect, For, Match, Switch} from 'solid-js';
import type {DayType, RouteSchedule, RouteStop} from '~/lib/api/types';
import {providerOf} from '~/lib/domain/providers';
import {fullStopName, splitCompanyLine} from '~/lib/format';
import {companyListVar, companySheetVar} from '~/lib/ui/palette';
import parts from '~/lib/ui/sheet/parts.module.css';
import SheetHeader from '~/lib/ui/sheet/SheetHeader';
import SheetTabs from '~/lib/ui/sheet/SheetTabs';
import SourceCredit from '~/lib/ui/SourceCredit';
import LineLead from './LineLead';
import styles from './ScheduleSheet.module.css';

const DAYS: {id: DayType; label: string; empty: string}[] = [
  {id: 'habil', label: 'Hábiles', empty: 'Los días hábiles'},
  {id: 'sabado', label: 'Sábados', empty: 'Los sábados'},
  {id: 'domingo', label: 'Domingos', empty: 'Los domingos'},
];

const pad = (n: number) => String(n).padStart(2, '0');

// laid out like the printed one
export default function ScheduleSheet(props: {
  line: string;
  routeId: string | null;
  company?: string;
  stop?: RouteStop;
  headsign?: string;
  schedule: RouteSchedule | null;
  failed: boolean;
  onDay?: (day: DayType) => void;
}) {
  const split = () =>
    props.company
      ? splitCompanyLine(props.line, props.company)
      : providerOf(props.routeId ?? '') === 'inter'
        ? splitCompanyLine(props.line)
        : {line: props.line};
  const color = () => companySheetVar(props.company || split().company);
  // server's mark, not the clock
  const nextHour = () => props.schedule?.hours.findIndex((h) => h.departures.some((d) => d.next)) ?? -1;

  let grid: HTMLOListElement | undefined;
  let scrolledFor = '';

  // once per day, never moves the list under the user
  createEffect(() => {
    const schedule = props.schedule;
    if (!grid || !schedule || scrolledFor === schedule.day) return;
    scrolledFor = schedule.day;
    const row = nextHour() >= 0 ? grid.children[nextHour()] : undefined;
    grid.scrollTop = row instanceof HTMLElement ? row.offsetTop - (grid.clientHeight - row.offsetHeight) / 2 : 0;
  });

  return (
    <>
      <div class={`${parts.band} ${styles.band}`} style={{'--sheet-header-bg': color()}}>
        <SheetHeader
          title={props.headsign ?? `Línea ${split().line}`}
          subtitle={
            props.stop ? `Planilla para: ${fullStopName(props.stop.stop_name, props.stop.station)}` : 'Planilla'
          }
          wrapSubtitle
          loading={!props.stop}
          lead={<LineLead line={split().line} company={split().company} />}
        />
      </div>

      <div
        class={`${styles.content} ${parts.onTint}`}
        style={{'--list': companyListVar(props.company || split().company), '--route': color()}}>
        <div class={styles.tabs}>
          <SheetTabs
            label='Día'
            tabs={DAYS}
            active={props.schedule?.day ?? ''}
            onSelect={(id) => props.onDay?.(id as DayType)}
            tab={(d) => d.label}
          />
        </div>

        <Switch
          fallback={
            <>
              <p class='sr-only' role='status'>
                Cargando la planilla
              </p>
              <ul class={`${styles.grid} sheet-scroll`} aria-hidden='true'>
                <For each={Array.from({length: 8})}>
                  {() => (
                    <li>
                      <span class={`${styles.hour} shimmer`} />
                      <span class={styles.minutes}>
                        <For each={Array.from({length: 4})}>{() => <span class={`${styles.minute} shimmer`} />}</For>
                      </span>
                    </li>
                  )}
                </For>
              </ul>
            </>
          }>
          <Match when={props.schedule && props.schedule.hours.length > 0 && props.schedule}>
            {(schedule) => (
              <ol ref={grid} class={`${styles.grid} sheet-scroll`}>
                <For each={schedule().hours}>
                  {(hour, i) => (
                    <li classList={{[styles.past]: nextHour() > 0 && i() < nextHour()}}>
                      <span class={styles.hour}>
                        {pad(hour.hour)}
                        {hour.late && (
                          <span class={styles.late} title='Madrugada del día siguiente'>
                            +1
                          </span>
                        )}
                      </span>
                      <span class={styles.minutes}>
                        <For each={hour.departures}>
                          {(departure) => (
                            <span
                              class={styles.minute}
                              classList={{
                                [styles.next]: Boolean(departure.next),
                                [styles.estimated]: Boolean(departure.zonal || departure.interpolated),
                              }}
                              aria-current={departure.next ? 'time' : undefined}>
                              {pad(departure.minute)}
                            </span>
                          )}
                        </For>
                      </span>
                    </li>
                  )}
                </For>
              </ol>
            )}
          </Match>
          <Match when={props.schedule}>
            <p class={styles.note}>
              {DAYS.find((d) => d.id === props.schedule?.day)?.empty ?? 'Ese día'} esta línea no para acá.
            </p>
          </Match>
          <Match when={props.failed}>
            <p class={styles.note}>No pudimos traer la planilla.</p>
          </Match>
        </Switch>

        <SourceCredit provider={props.routeId ? providerOf(props.routeId) : undefined} tinted />
      </div>
    </>
  );
}
