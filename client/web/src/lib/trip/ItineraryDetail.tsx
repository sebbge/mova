import {For, Show} from 'solid-js';
import type {BusLeg, Itinerary, TripPlace, WalkLeg} from '~/lib/api/types';
import {clock, distance, duration, eta, fareLabel, money} from '~/lib/format';
import LineBadge from '~/lib/line/LineBadge';
import WalkIcon from '~/lib/ui/icons/WalkIcon';
import styles from './ItineraryDetail.module.css';
import {firstBus} from './LiveBoard';

function placeName(place: TripPlace, fallback: string): string {
  return 'name' in place ? place.name : fallback;
}

export default function ItineraryDetail(props: {
  itinerary: Itinerary;
  live?: number | null;
  originLabel: string;
  destinationLabel: string;
  onSelectStop?: (ref: string, name: string) => void;
  onSelectLine?: (leg: BusLeg) => void;
}) {
  const first = () => firstBus(props.itinerary);
  const last = () => props.itinerary.legs.length - 1;

  const Walk = (p: {leg: WalkLeg; index: number}) => (
    <li class={`${styles.leg} ${styles.walk}`}>
      <span class={styles.rail} aria-hidden='true'>
        <WalkIcon />
      </span>
      <div class={styles.content}>
        <p class={styles.title}>Caminá {distance(p.leg.distance_m)}</p>
        <p class={styles.detail}>
          {duration(p.leg.duration_s)} hasta{' '}
          {'id' in p.leg.to ? (
            <button
              class={styles.link}
              onClick={() => 'id' in p.leg.to && props.onSelectStop?.(p.leg.to.id, p.leg.to.name)}>
              {p.leg.to.name}
            </button>
          ) : p.index === last() ? (
            props.destinationLabel
          ) : (
            placeName(p.leg.to, 'el destino')
          )}
        </p>
        <Show when={p.index === 0}>
          <p class={`${styles.detail} ${styles.muted}`}>
            Desde {placeName(p.leg.from, props.originLabel)}, a las {clock(p.leg.departure)}
          </p>
        </Show>
      </div>
    </li>
  );

  const Bus = (p: {leg: BusLeg}) => (
    <li class={`${styles.leg} ${styles.bus}`}>
      <span class={styles.rail} aria-hidden='true' />
      <div class={styles.content}>
        <button class={styles.line} onClick={() => props.onSelectLine?.(p.leg)}>
          <LineBadge line={p.leg.line} size='sm' />
          <span class={styles.headsign}>{p.leg.headsign ?? 'Ver línea'}</span>
        </button>
        <Show when={p.leg === first() && props.live != null}>
          <p class={styles.live}>En vivo: pasa en {eta(props.live ?? undefined)}</p>
        </Show>
        <p class={styles.stop}>
          <span class={styles.at} classList={{[styles.estimated]: p.leg.interpolated}}>
            {clock(p.leg.departure)}
          </span>
          <button class={styles.link} onClick={() => props.onSelectStop?.(p.leg.from.id, p.leg.from.name)}>
            {p.leg.from.name}
          </button>
        </p>
        <p class={`${styles.detail} ${styles.muted}`}>
          {p.leg.stops} {p.leg.stops === 1 ? 'parada' : 'paradas'}
        </p>
        <p class={styles.stop}>
          <span class={styles.at} classList={{[styles.estimated]: p.leg.interpolated}}>
            {clock(p.leg.arrival)}
          </span>
          <button class={styles.link} onClick={() => props.onSelectStop?.(p.leg.to.id, p.leg.to.name)}>
            {p.leg.to.name}
          </button>
        </p>
        <Show when={p.leg.interpolated}>
          <p class={`${styles.detail} ${styles.muted}`}>Horario estimado entre puntos de la planilla</p>
        </Show>
      </div>
    </li>
  );

  return (
    <>
      <div class={styles.summary}>
        <span class={styles.duration}>{duration(props.itinerary.duration_s)}</span>
        <span class={styles.times}>
          {clock(props.itinerary.departure)} → {clock(props.itinerary.arrival)}
        </span>
        <span class={styles.fare} classList={{[styles.unknown]: props.itinerary.fare.total === null}}>
          {fareLabel(props.itinerary.fare)}
        </span>
      </div>

      <ol class={`${styles.timeline} sheet-scroll`}>
        <For each={props.itinerary.legs}>
          {(leg, i) => (leg.mode === 'walk' ? <Walk leg={leg} index={i()} /> : <Bus leg={leg} />)}
        </For>

        <li class={`${styles.leg} ${styles.end}`}>
          <span class={styles.rail} aria-hidden='true' />
          <div class={styles.content}>
            <p class={styles.title}>Llegás a las {clock(props.itinerary.arrival)}</p>
            <p class={`${styles.detail} ${styles.muted}`}>{props.destinationLabel}</p>
          </div>
        </li>

        <li class={styles.tickets}>
          <Show when={props.itinerary.fare.tickets.length > 0}>
            <p class={styles.title}>Boletos</p>
            <ul>
              <For each={props.itinerary.fare.tickets}>
                {(ticket) => (
                  <li>
                    <span>{ticket.name}</span>
                    <span>{money(ticket.price)}</span>
                  </li>
                )}
              </For>
            </ul>
          </Show>
          <Show when={!props.itinerary.fare.complete}>
            <p class={`${styles.detail} ${styles.muted}`}>Algunos tramos todavía no tienen tarifa cargada.</p>
          </Show>
        </li>
      </ol>
    </>
  );
}
