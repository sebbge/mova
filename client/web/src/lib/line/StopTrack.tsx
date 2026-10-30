import {createEffect, For, Show} from 'solid-js';
import type {Bus, RouteShape, StopArrival, StopLine} from '~/lib/api/types';
import {arrivalLabel, arrivalSpoken, clockOf, fullStopName, rawId} from '~/lib/format';
import BusIcon from '~/lib/ui/icons/BusIcon';
import {freshnessVar} from '~/lib/ui/palette';
import styles from './StopTrack.module.css';

// unknown without a matching destination
function liveArrival(lines: StopLine[], line: string, headsign?: string): StopArrival | undefined {
  const ofLine = lines.filter((l) => l.line === line);
  const target = headsign?.toUpperCase();
  const match =
    ofLine.length === 1 ? ofLine[0] : ofLine.find((l) => target && l.destination?.toUpperCase().startsWith(target));
  return match?.arrivals.find((a) => a.live);
}

export default function StopTrack(props: {
  shape: RouteShape;
  line: string;
  here: number; // -1 if none
  times: (number | null)[]; // aligned with `shape.stops`
  buses: Bus[];
  arrivals: StopLine[];
  arrivalsLoading: boolean;
  onSelectStop?: (stopId: string, stopName?: string) => void;
  onOpenStop?: (stopId: string, stopName?: string) => void;
}) {
  let list!: HTMLOListElement;

  // each bus before the stop it heads to
  const approaching = () => {
    const byStop = new Map<string, Bus[]>();
    for (const bus of props.buses) {
      const next = bus.next_stop?.id;
      if (next) byStop.set(next, [...(byStop.get(next) ?? []), bus]);
    }
    return byStop;
  };

  const live = () => (props.here >= 0 ? liveArrival(props.arrivals, props.line, props.shape.headsign) : undefined);
  const terminal = (i: number) => {
    const n = props.shape.stops.length;
    return n < 2 ? '' : i === 0 ? 'O' : i === n - 1 ? 'D' : '';
  };

  createEffect(() => {
    const row = props.here >= 0 ? list.children[props.here] : undefined;
    if (!(row instanceof HTMLElement)) return;
    // tapped row is in view, don't move the list
    const top = row.offsetTop - list.scrollTop;
    if (top >= 0 && top + row.offsetHeight <= list.clientHeight) return;
    list.scrollTop = row.offsetTop - (list.clientHeight - row.offsetHeight) / 2;
  });

  return (
    <ol ref={list} class={`${styles.track} sheet-scroll`}>
      <For each={props.shape.stops}>
        {(stop, i) => {
          const here = () => i() === props.here;
          const coming = () => approaching().get(rawId(stop.stop_id)) ?? [];
          return (
            <li classList={{[styles.passed]: i() < props.here, [styles.here]: here()}}>
              <button
                aria-current={here() ? 'location' : undefined}
                onClick={() => (here() ? props.onOpenStop : props.onSelectStop)?.(stop.stop_id, stop.stop_name)}>
                <span class={styles.node} classList={{[styles.terminal]: terminal(i()) !== ''}} aria-hidden='true'>
                  {terminal(i())}
                </span>
                <span class={styles.name}>
                  {fullStopName(stop.stop_name, stop.station)}
                  <Show when={here()}>
                    <svg class={styles.open} viewBox='0 0 24 24' aria-hidden='true'>
                      <path d='m9 6 6 6-6 6' />
                    </svg>
                    <span class='sr-only'>, ver llegadas</span>
                  </Show>
                </span>
                <Show
                  when={here() && live()}
                  fallback={
                    <Show when={props.times[i()] != null}>
                      <time class={styles.at} classList={{[styles.pending]: here() && props.arrivalsLoading}}>
                        <span class={styles.approx} aria-hidden='true'>
                          ≈
                        </span>
                        <span class='sr-only'>cerca de las </span>
                        {clockOf(props.times[i()] ?? 0)}
                      </time>
                    </Show>
                  }>
                  {(arrival) => (
                    <span
                      class={`${styles.at} ${styles.live}`}
                      style={{color: freshnessVar(true, arrival().bus?.freshness)}}>
                      <span aria-hidden='true'>{arrivalLabel(arrival())}</span>
                      <span class='sr-only'>{arrivalSpoken(arrival())}</span>
                    </span>
                  )}
                </Show>
              </button>
              <For each={coming()}>
                {(bus) => (
                  <span
                    class={`${styles.bus} ${styles[bus.freshness] ?? ''}`}
                    classList={{[styles.still]: bus.still}}
                    aria-hidden='true'>
                    <BusIcon />
                  </span>
                )}
              </For>
              <Show when={coming().length > 0}>
                <span class='sr-only'>Un bus en camino a esta parada</span>
              </Show>
            </li>
          );
        }}
      </For>
    </ol>
  );
}
