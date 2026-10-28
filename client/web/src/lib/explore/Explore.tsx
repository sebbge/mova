import {createEffect, Match, Show, Switch} from 'solid-js';
import BusSheet from '~/lib/bus/BusSheet';
import {MAP_CAMERA} from '~/lib/domain/regions';
import LineSheet from '~/lib/line/LineSheet';
import MapCanvas from '~/lib/map/MapCanvas';
import StopSheet from '~/lib/stop/StopSheet';
import TripSearch from '~/lib/trip/TripSearch';
import TripSheet from '~/lib/trip/TripSheet';
import Sheet from '~/lib/ui/sheet/Sheet';
import styles from './Explore.module.css';
import MapHeader from './MapHeader';
import {MapSelection} from './MapSelection';
import {MapUrl} from './MapUrl';

export default function Explore() {
  const selection = new MapSelection(new MapUrl());
  const trip = selection.planner;

  createEffect(() => {
    const what = selection.lineId ? `Línea ${selection.lineId} en el mapa` : trip.active ? 'Cómo llegar' : 'Mapa';
    document.title = `${what} | Bondi`;
  });

  return (
    <div class={styles.screen}>
      <MapHeader destination={trip.active ? trip.feed.b?.label : undefined} onSearch={() => trip.openSearch('b')} />
      <div class={styles.canvas}>
        <MapCanvas
          camera={MAP_CAMERA}
          shape={selection.line.shape}
          routeCompany={selection.routeCompany}
          buses={selection.fleet.all}
          selectedStopId={selection.stopId}
          selectedSiblings={selection.stops.refs}
          selectedStop={selection.stops.stop}
          selectedBusId={selection.busId}
          followBusId={selection.followBusId}
          onSelectStop={(id, name) => selection.openStop(id, name)}
          onSelectBus={(bus) => selection.tapBus(bus)}
          onBackground={() => selection.deselectBus()}
          onView={(center, km2) => selection.onView(center, km2)}
          trip={trip.drawing}
          tripFitKey={trip.fitKey}
          picking={trip.picking !== null}
          onPick={(p) => {
            const field = trip.picking;
            if (field) trip.setPoint(field, {kind: 'point', lat: p.lat, lng: p.lng});
          }}
        />

        <Show when={trip.searching}>
          <TripSearch
            a={trip.feed.a}
            b={trip.feed.b}
            field={trip.searchField}
            near={trip.center}
            mode={trip.mode}
            at={trip.at}
            onPoint={(field, endpoint) => trip.choose(field, endpoint)}
            onPick={(field) => trip.pickOnMap(field)}
            onSwap={() => trip.swap()}
            onAt={(at) => trip.setAt(at)}
            onClose={() => trip.closeSearch()}
          />
        </Show>

        <Sheet
          open={selection.open}
          onBack={selection.back}
          onClose={() => selection.closeSheet()}
          sticky={trip.active && !selection.childOpen}>
          <Switch>
            <Match when={selection.fleet.selected}>
              {(bus) => (
                <BusSheet
                  bus={bus()}
                  direction={selection.fleet.direction}
                  onShowLine={selection.lineId === bus().line ? () => selection.showLine() : undefined}
                />
              )}
            </Match>
            <Match when={selection.lineId}>
              {(line) => (
                <LineSheet
                  line={line()}
                  variants={selection.line.variants}
                  shape={selection.line.shape}
                  routeId={selection.routeId}
                  direction={selection.line.shape?.direction_id ?? 0}
                  origin={selection.line.origin}
                  company={selection.lineCompany}
                  loading={selection.settlingLine}
                  onSelect={(routeId, direction) => selection.selectVariant(routeId, direction)}
                  onSelectStop={(id, name) => selection.openStop(id, name)}
                />
              )}
            </Match>
            <Match when={selection.stopId}>
              {(stopId) => (
                <StopSheet
                  stop={selection.stops.stop}
                  stopRef={stopId()}
                  stopName={selection.knownName}
                  arrivals={selection.stops.arrivals}
                  loading={selection.stops.loading}
                  failed={selection.stops.failed}
                  onSelectLine={(line, destination, company) => selection.openArrival(line, destination, company)}
                  onSelectService={(ref) => selection.switchService(ref)}
                />
              )}
            </Match>
            <Match when={trip.active}>
              <TripSheet
                feed={trip.feed}
                live={trip.live}
                option={trip.option}
                mode={trip.mode}
                at={trip.at}
                picking={trip.picking}
                onEdit={(field) => trip.openSearch(field)}
                onOption={(i) => trip.setOption(i)}
                onSelectStop={(ref, name) => selection.openStop(ref, name)}
                onSelectLine={(leg) => selection.openLeg(leg)}
              />
            </Match>
          </Switch>
        </Sheet>
      </div>
    </div>
  );
}
