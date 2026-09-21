import {createEffect, Match, Switch} from 'solid-js';
import BusSheet from '~/lib/bus/BusSheet';
import type {Region} from '~/lib/domain/regions';
import LineSheet from '~/lib/line/LineSheet';
import MapCanvas from '~/lib/map/MapCanvas';
import StopSheet from '~/lib/stop/StopSheet';
import Sheet from '~/lib/ui/sheet/Sheet';
import {MapSelection} from './MapSelection';
import {MapUrl} from './MapUrl';

export default function Explore(props: {region: Region}) {
  const selection = new MapSelection(new MapUrl());

  createEffect(() => {
    const what = selection.lineId ? `Línea ${selection.lineId} en el mapa` : 'Mapa';
    document.title = `${what}, ${props.region.name} | bondi`;
  });

  return (
    <>
      <MapCanvas
        region={props.region}
        shape={selection.shape}
        buses={selection.buses}
        selectedStopId={selection.stopId}
        followBusId={selection.busId}
        onSelectStop={(id) => selection.openStop(id)}
        onSelectBus={(bus) => selection.selectBus(bus)}
        onBackground={() => selection.deselectBus()}
      />

      <Sheet open={selection.open} onClose={() => selection.close()}>
        <Switch>
          <Match when={selection.selectedBus}>
            {(bus) => (
              <BusSheet bus={bus()} onSelectLine={(line, destination) => selection.openLine(line, destination)} />
            )}
          </Match>
          <Match when={selection.lineId}>
            {(line) => (
              <LineSheet
                line={line()}
                variants={selection.variants}
                shape={selection.shape}
                routeId={selection.routeId}
                direction={selection.shape?.direction_id ?? 0}
                origin={selection.origin}
                region={props.region.slug}
                onSelect={(routeId, direction) => selection.selectVariant(routeId, direction)}
                onSelectStop={(id) => selection.openStop(id)}
              />
            )}
          </Match>
          <Match when={selection.stopId}>
            <StopSheet
              stop={selection.stop}
              arrivals={selection.arrivals}
              loading={selection.loadingArrivals}
              region={props.region.slug}
              onSelectLine={(line, destination) => selection.openLine(line, destination)}
            />
          </Match>
        </Switch>
      </Sheet>
    </>
  );
}
