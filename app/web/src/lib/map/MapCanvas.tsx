import {GeolocateControl, Map as MapLibreMap, NavigationControl, setWorkerUrl} from 'maplibre-gl';
import 'maplibre-gl/dist/maplibre-gl.css';
import workerUrl from 'maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url';
import {createSignal, Match, onCleanup, onMount, Switch} from 'solid-js';
import type {Region} from '~/lib/domain/regions';
import {href} from '~/lib/ui/href';
import styles from './MapCanvas.module.css';
import {NearbyStops} from './NearbyStops';
import {basemapStyle} from './style';

export default function MapCanvas(props: {region: Region}) {
  let container!: HTMLDivElement;
  const [error, setError] = createSignal<string | null>(null);
  const [stops, setStops] = createSignal<NearbyStops>();

  onMount(() => {
    const region = props.region;
    setWorkerUrl(workerUrl);
    const darkQuery = matchMedia('(prefers-color-scheme: dark)');

    let map: MapLibreMap;
    try {
      map = new MapLibreMap({
        container,
        style: basemapStyle(darkQuery.matches),
        center: [region.center.lng, region.center.lat],
        zoom: region.zoom,
        attributionControl: {compact: true},
      });
    } catch (e) {
      setError((e as Error).message); // no webgl
      return;
    }

    const layer = new NearbyStops(map);
    setStops(layer);

    const resizer = new ResizeObserver(() => map.resize()); // a 0x0 container leaves a blank canvas
    resizer.observe(container);

    map.on('error', (e) => {
      setError(e.error?.message ?? 'No se pudo cargar el mapa');
      console.error('[bondi/map]', e.error ?? e);
    });

    map.addControl(new NavigationControl({showCompass: false}), 'bottom-right');
    map.addControl(new GeolocateControl({trackUserLocation: true, showAccuracyCircle: true}), 'bottom-right');

    map.on('load', () => {
      layer.addLayers();
      void layer.refresh();
    });
    map.on('moveend', () => layer.refreshSoon());

    map.on('click', NearbyStops.HIT_LAYER, (e) => {
      const stopId = e.features?.[0]?.properties?.stop_id;
      if (stopId) location.assign(href.stop(region.slug, String(stopId)));
    });
    map.on('mouseenter', NearbyStops.HIT_LAYER, () => (map.getCanvas().style.cursor = 'pointer'));
    map.on('mouseleave', NearbyStops.HIT_LAYER, () => (map.getCanvas().style.cursor = ''));

    // setStyle drops custom layers
    const onScheme = (e: MediaQueryListEvent) => {
      map.setStyle(basemapStyle(e.matches));
      map.once('idle', () => {
        layer.addLayers();
        void layer.refresh();
      });
    };
    darkQuery.addEventListener('change', onScheme);

    onCleanup(() => {
      layer.close();
      resizer.disconnect();
      darkQuery.removeEventListener('change', onScheme);
      map.remove();
    });
  });

  return (
    <>
      <div class={styles.map} ref={container} />
      <Switch>
        <Match when={error()}>
          <p class={`${styles.banner} ${styles.error}`}>{error()}</p>
        </Match>
        <Match when={stops()?.zoomedOut}>
          <p class={styles.banner}>Acercá el mapa para ver las paradas</p>
        </Match>
        <Match when={stops()?.failed}>
          <p class={`${styles.banner} ${styles.error}`}>No pudimos cargar las paradas</p>
        </Match>
      </Switch>
    </>
  );
}
