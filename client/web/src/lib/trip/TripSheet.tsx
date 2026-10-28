import {createEffect, For, Match, Show, Switch} from 'solid-js';
import type {BusLeg, TravelMode} from '~/lib/api/types';
import {departureLabel} from '~/lib/format';
import parts from '~/lib/ui/sheet/parts.module.css';
import SheetHeader from '~/lib/ui/sheet/SheetHeader';
import ItineraryDetail from './ItineraryDetail';
import ItineraryRow from './ItineraryRow';
import {firstBus, type LiveBoard} from './LiveBoard';
import {fareProfile} from './profile';
import type {TripFeed} from './TripFeed';
import styles from './TripSheet.module.css';

export default function TripSheet(props: {
  feed: TripFeed;
  live: LiveBoard;
  option: number | null;
  mode: TravelMode;
  at: string | null;
  picking: 'a' | 'b' | null;
  onEdit?: (field: 'a' | 'b') => void;
  onOption?: (index: number) => void;
  onSelectStop?: (ref: string, name: string) => void;
  onSelectLine?: (leg: BusLeg) => void;
}) {
  const origin = () => props.feed.a?.label ?? 'el origen';
  const destination = () => props.feed.b?.label ?? 'Elegí el destino';
  const selected = () => (props.option !== null ? (props.feed.itineraries[props.option] ?? null) : null);

  createEffect(() => props.live.watch(props.feed.itineraries));

  return (
    <>
      <div class={`${parts.band} ${styles.band}`}>
        <SheetHeader
          kicker='Cómo llegar'
          title={destination()}
          subtitle={props.feed.a ? `Desde ${props.feed.a.label}` : 'Falta el origen'}
        />
        <div class={styles.chips}>
          <button
            class={styles.chip}
            aria-label={`Cambiar la hora: ${departureLabel(props.at)}`}
            onClick={() => props.onEdit?.('b')}>
            {departureLabel(props.at)}
          </button>
          <Show when={props.mode === 'transit'}>
            <button
              class={styles.chip}
              aria-label={`Cambiar el boleto: ${fareProfile.label}`}
              onClick={() => props.onEdit?.('b')}>
              {fareProfile.label}
            </button>
          </Show>
        </div>
      </div>

      <div class={styles.body}>
        <Switch>
          <Match when={props.picking}>
            {(picking) => (
              <div class={styles.note}>
                <Show when={picking() === 'a' && props.feed.hereFailed}>
                  <p>No pudimos obtener tu ubicación.</p>
                </Show>
                <p class={styles.strong}>
                  Tocá el mapa o una parada para elegir {picking() === 'a' ? 'el origen' : 'el destino'}.
                </p>
                <button class={styles.action} onClick={() => props.onEdit?.(picking())}>
                  Buscar una parada
                </button>
              </div>
            )}
          </Match>
          <Match when={props.feed.status === 'locating'}>
            <p class={styles.note} role='status'>
              Buscando tu ubicación…
            </p>
          </Match>
          <Match when={props.feed.status === 'loading'}>
            <p class='sr-only' role='status'>
              Calculando el viaje
            </p>
            <ul class={styles.skeleton} aria-hidden='true'>
              <For each={[0, 1, 2]}>
                {() => (
                  <li>
                    <span class={`shimmer ${styles.wide}`} />
                    <span class='shimmer' />
                    <span class={`shimmer ${styles.narrow}`} />
                  </li>
                )}
              </For>
            </ul>
          </Match>
          <Match when={props.feed.status === 'preparing'}>
            <div class={styles.note}>
              <p>El planificador se está preparando. Probá en unos segundos.</p>
              <button class={styles.action} onClick={() => props.feed.retry()}>
                Reintentar
              </button>
            </div>
          </Match>
          <Match when={props.feed.status === 'error'}>
            <div class={styles.note}>
              <p>No pudimos calcular el viaje. Revisá la conexión.</p>
              <button class={styles.action} onClick={() => props.feed.retry()}>
                Reintentar
              </button>
            </div>
          </Match>
          <Match when={props.feed.status === 'empty'}>
            <div class={styles.note}>
              <p>No encontramos cómo llegar a esa hora. Probá con otra hora o con puntos más cerca de una parada.</p>
              <button class={styles.action} onClick={() => props.onEdit?.('b')}>
                Cambiar la búsqueda
              </button>
            </div>
          </Match>
          <Match when={selected()}>
            {(itinerary) => (
              <ItineraryDetail
                itinerary={itinerary()}
                live={props.live.eta(firstBus(itinerary()))}
                originLabel={origin()}
                destinationLabel={destination()}
                onSelectStop={props.onSelectStop}
                onSelectLine={props.onSelectLine}
              />
            )}
          </Match>
          <Match when={props.feed.status === 'ready'}>
            <ul class={`${styles.options} sheet-scroll`}>
              <For each={props.feed.itineraries}>
                {(itinerary, i) => (
                  <li>
                    <ItineraryRow
                      itinerary={itinerary}
                      live={props.live.eta(firstBus(itinerary))}
                      onOpen={() => props.onOption?.(i())}
                    />
                  </li>
                )}
              </For>
            </ul>
          </Match>
        </Switch>
      </div>
    </>
  );
}
