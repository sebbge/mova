import {For, Match, Show, Switch} from 'solid-js';
import type {Stop, StopLine} from '~/lib/api/types';
import {orderedRefs, providerInfo, providerOf, serviceOf, stopKicker} from '~/lib/domain/providers';
import {rawId} from '~/lib/format';
import ServiceIcon from '~/lib/ui/icons/ServiceIcon';
import StopIcon from '~/lib/ui/icons/StopIcon';
import SourceCredit from '~/lib/ui/SourceCredit';
import parts from '~/lib/ui/sheet/parts.module.css';
import SheetHeader from '~/lib/ui/sheet/SheetHeader';
import SheetTabs from '~/lib/ui/sheet/SheetTabs';
import ArrivalRow from './ArrivalRow';
import ArrivalsSkeleton from './ArrivalsSkeleton';
import styles from './StopSheet.module.css';

export default function StopSheet(props: {
  stop: Stop | null;
  stopRef: string; // before the backend answers
  stopName?: string; // skips "loading"
  arrivals: StopLine[];
  loading: boolean;
  failed?: boolean;
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
      <div class={`${parts.band} ${styles.band} ${styles[service()] ?? ''}`}>
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
            <>
              <Show when={props.failed}>
                <p class={styles.stale}>No pudimos actualizar el tiempo real. Lo que ves puede no estar al día.</p>
              </Show>
              <ul class={`${styles.lines} sheet-scroll`}>
                <For each={props.arrivals}>
                  {(line) => (
                    <li>
                      <ArrivalRow
                        line={line}
                        split={service() === 'inter'}
                        onClick={() => props.onSelectLine?.(line.line, line.destination ?? undefined, line.company)}
                      />
                    </li>
                  )}
                </For>
              </ul>
            </>
          }>
          <Match when={props.loading && props.arrivals.length === 0}>
            <p class='sr-only' role='status'>
              Buscando llegadas
            </p>
            <ArrivalsSkeleton />
          </Match>
          <Match when={props.arrivals.length === 0}>
            <p class={styles.note}>
              {props.failed
                ? 'No pudimos traer las llegadas. Volvemos a intentar cada 30 segundos.'
                : 'No hay llegadas previstas ahora.'}
            </p>
          </Match>
        </Switch>
        <SourceCredit provider={providerOf(ref())} tinted />
      </div>
    </>
  );
}
