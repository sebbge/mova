import {createEffect, Match, Switch} from 'solid-js';
import BusSheet from '~/lib/bus/BusSheet';
import {MAP_CAMERA} from '~/lib/domain/regions';
import LineSheet from '~/lib/line/LineSheet';
import MapCanvas from '~/lib/map/MapCanvas';
import StopSheet from '~/lib/stop/StopSheet';
import Sheet from '~/lib/ui/sheet/Sheet';
import {MapSelection} from './MapSelection';
import {MapUrl} from './MapUrl';

export default function Explore() {
  const selection = new MapSelection(new MapUrl());

  createEffect(() => {
    document.title = `${selection.lineId ? `Línea ${selection.lineId} en el mapa` : 'Mapa'} | bondi`;
  });

  return (
    <>
      <MapCanvas
        camera={MAP_CAMERA}
        shape={selection.line.shape}
        buses={selection.buses}
        selectedStopId={selection.stopId}
        selectedSiblings={selection.stops.refs}
        followBusId={selection.busId}
        onSelectStop={(id, name) => selection.openStop(id, name)}
        onSelectBus={(bus) => selection.openBus(bus)}
        onBackground={() => selection.deselectBus()}
      />

      <Sheet open={selection.open} onBack={selection.back} onClose={() => selection.close()}>
        <Switch>
          <Match when={selection.selectedBus}>
            {(bus) => <BusSheet bus={bus()} onShowLine={() => selection.showLine()} />}
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
                onSelectLine={(line, destination, company) => selection.openArrival(line, destination, company)}
                onSelectService={(ref) => selection.switchService(ref)}
              />
            )}
          </Match>
        </Switch>
      </Sheet>
    </>
  );
}
