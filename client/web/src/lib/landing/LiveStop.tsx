import {createSignal, For, Show} from 'solid-js';
import {providerOf} from '~/lib/domain/providers';
import Arrivals from '~/lib/stop/Arrivals';
import parts from '~/lib/ui/sheet/parts.module.css';
import SheetHeader from '~/lib/ui/sheet/SheetHeader';
import SourceCredit from '~/lib/ui/SourceCredit';
import type {DemoStop} from './demo';
import styles from './LiveStop.module.css';
import Phone from './Phone';

export default function LiveStop(props: {stops: DemoStop[]}) {
  const [current, setCurrent] = createSignal(0);
  const stop = () => props.stops[current()];

  return (
    <Show when={stop()}>
      {(s) => (
        <div class={styles.live}>
          <Show when={props.stops.length > 1}>
            <div class={styles.try} role='group' aria-label='Probá con otra parada'>
              <For each={props.stops}>
                {(other, i) => (
                  <button
                    type='button'
                    class={styles.chip}
                    aria-pressed={i() === current()}
                    onClick={() => setCurrent(i())}>
                    {other.label}
                  </button>
                )}
              </For>
            </div>
          </Show>

          <Phone top={`var(--service-${s().service}-header)`}>
            <div class={parts.band} style={{'--sheet-header-bg': `var(--service-${s().service}-header)`}}>
              <SheetHeader kicker={s().kicker} title={s().name} subtitle={s().place} />
            </div>
            <div class={styles.arrivals}>
              <Show when={s().id} keyed>
                {(id) => <Arrivals stopId={id} split={s().service === 'inter'} />}
              </Show>
            </div>
          </Phone>

          <SourceCredit provider={providerOf(s().id)} />
        </div>
      )}
    </Show>
  );
}
