import {createEffect, createSignal, For, on, onCleanup, Show} from 'solid-js';
import type {Bus, BusFreshness} from '~/lib/api/types';
import {ago, delayLabel, rawId, splitCompanyLine} from '~/lib/format';
import LineLead from '~/lib/line/LineLead';
import MetaIcon from '~/lib/ui/icons/MetaIcon';
import {metaTags} from '~/lib/ui/icons/MetaIcons';
import {companyListVar, companySheetVar, freshnessVar} from '~/lib/ui/palette';
import parts from '~/lib/ui/sheet/parts.module.css';
import SheetHeader from '~/lib/ui/sheet/SheetHeader';
import SplitPill from '~/lib/ui/SplitPill';
import styles from './BusSheet.module.css';

const CONFIDENCE: Record<BusFreshness, string> = {
  live: 'En vivo',
  low_precision: 'Aproximado',
  imprecise: 'Última posición conocida',
};

export default function BusSheet(props: {
  bus: Bus;
  direction?: number; // 0 outbound, 1 inbound
  onShowLine?: () => void;
  previewStop?: string | null; // marked on the map
  onSelectStop?: (stopId: string, name?: string) => void;
}) {
  const split = () =>
    props.bus.company ? splitCompanyLine(props.bus.line, props.bus.company) : {line: props.bus.line};
  // pin color, lower in dark
  const company = () => companySheetVar(props.bus.company);
  const previewing = () => props.previewStop != null && rawId(props.previewStop) === props.bus.next_stop?.id;

  const sense = () => (props.direction === 0 ? 'Ida' : props.direction === 1 ? 'Vuelta' : undefined);
  const subtitle = () => {
    const origin = props.bus.origin;
    if (!origin) return sense() ?? '';
    return sense() ? `${sense()} desde ${origin}` : `desde ${origin}`;
  };

  const [failedPhoto, setFailedPhoto] = createSignal<string>();
  const photo = () => (props.bus.photo_url !== failedPhoto() ? props.bus.photo_url : undefined);

  // 0 when stopped
  const speed = () => (props.bus.still ? 0 : props.bus.speed === undefined ? undefined : Math.round(props.bus.speed));

  // new reading restarts the clock
  const [now, setNow] = createSignal(Date.now());
  createEffect(
    on(
      () => props.bus.timestamp,
      () => {
        setNow(Date.now());
        const timer = setInterval(() => setNow(Date.now()), 1_000);
        onCleanup(() => clearInterval(timer));
      }
    )
  );
  const signal = () => ago(props.bus.timestamp, now());

  return (
    <>
      <div class={`${parts.band} ${styles.band}`} style={{'--sheet-header-bg': company()}}>
        <SheetHeader
          title={props.bus.destination ?? `Línea ${split().line}`}
          subtitle={subtitle()}
          lead={<LineLead line={split().line} company={split().company} />}
        />
      </div>

      <div class={`${styles.content} sheet-scroll`} style={{'--list': companyListVar(props.bus.company)}}>
        <Show when={photo()}>
          {(src) => (
            <img
              class={styles.photo}
              src={src()}
              alt={`Unidad ${props.bus.unit} de ${props.bus.company}`}
              onError={() => setFailedPhoto(src())}
            />
          )}
        </Show>

        <section class={styles.identity}>
          <div class={styles.top}>
            <SplitPill
              color={company()}
              lead={
                props.bus.company ? (
                  <>
                    <Show when={props.bus.logo_url}>{(logo) => <img class={styles.logo} src={logo()} alt='' />}</Show>
                    {props.bus.company}
                  </>
                ) : undefined
              }>
              N° {props.bus.unit}
            </SplitPill>
            {/* freshness color, never company */}
            <p class={styles.state} style={{'--dot': freshnessVar(true, props.bus.freshness)}}>
              <span class={styles.confidence}>
                <span class={styles.dot} aria-hidden='true'>
                  {/* new reading replays the beam */}
                  <Show when={props.bus.timestamp} keyed>
                    <span class={styles.beam} />
                  </Show>
                </span>
                {CONFIDENCE[props.bus.freshness]}
              </span>
              <Show when={signal()}>{(text) => <span class={styles.ago}>{text()}</span>}</Show>
            </p>
            <Show when={speed() !== undefined}>
              <p class={styles.speed}>
                <span class={styles.value}>{speed()}</span>
                <span class={styles.per}>km/h</span>
              </p>
            </Show>
          </div>
          <Show when={metaTags(props.bus).length > 0}>
            <ul class={styles.tags}>
              <For each={metaTags(props.bus)}>
                {(tag) => (
                  <li>
                    <SplitPill
                      color={`var(--meta-${tag.kind}-deep)`}
                      ink={`var(--meta-${tag.kind}-ink)`}
                      iconic
                      small
                      lead={<MetaIcon kind={tag.kind} size={14} />}>
                      {tag.text}
                    </SplitPill>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </section>

        {/* operator delay vs position age */}
        <Show when={delayLabel(props.bus.delay_min)}>{(delay) => <p class={styles.delay}>{delay()}</p>}</Show>

        <Show when={props.bus.next_stop}>
          {(next) => (
            <button
              class={styles.next}
              classList={{[styles.previewing]: previewing()}}
              aria-current={previewing() ? 'location' : undefined}
              onClick={() => props.onSelectStop?.(next().id, next().name)}>
              <span class={styles.nextLabel}>Próxima parada</span>
              <span class={styles.nextName}>
                {next().name}
                <Show when={previewing()}>
                  <span class='sr-only'>, ver llegadas</span>
                </Show>
              </span>
              <Show when={previewing()}>
                <svg viewBox='0 0 24 24' aria-hidden='true'>
                  <path d='m9 6 6 6-6 6' />
                </svg>
              </Show>
            </button>
          )}
        </Show>

        <Show when={props.onShowLine}>
          {(onShowLine) => (
            <div class={styles.footer}>
              <button class={styles.line} style={{'--company': company()}} onClick={() => onShowLine()()}>
                Ver toda la línea
              </button>
            </div>
          )}
        </Show>
      </div>
    </>
  );
}
