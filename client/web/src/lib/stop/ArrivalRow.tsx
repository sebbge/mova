import {For, Show} from 'solid-js';
import type {StopArrival, StopLine} from '~/lib/api/types';
import {arrivalLabel, arrivalSpoken, arrivalTitle, splitCompanyLine} from '~/lib/format';
import LineBadge from '~/lib/line/LineBadge';
import MetaIcons from '~/lib/ui/icons/MetaIcons';
import {freshnessVar} from '~/lib/ui/palette';
import styles from './ArrivalRow.module.css';

function When(props: {arrival: StopArrival | undefined}) {
  return (
    <>
      <span aria-hidden='true'>{arrivalLabel(props.arrival)}</span>
      <span class='sr-only'>{arrivalSpoken(props.arrival)}</span>
    </>
  );
}

// link on the page, button in the sheet
export default function ArrivalRow(props: {
  line: StopLine;
  split?: boolean; // company glued to the number
  href?: string;
  onClick?: () => void;
}) {
  const label = () => (props.split ? splitCompanyLine(props.line.line) : {line: props.line.line, company: undefined});
  const first = () => props.line.arrivals[0];

  const content = () => (
    <>
      <LineBadge line={label().line} company={props.line.company || label().company} size='sm' variant='flat' />
      <span class={styles.info}>
        <span class={styles.destination}>{props.line.destination ?? ''}</span>
        <Show when={label().company}>
          <span class={styles.company}>{label().company}</span>
        </Show>
        <MetaIcons bus={first()?.bus} />
      </span>
      <span class={styles.etas}>
        <span
          class={styles.eta}
          classList={{
            [styles.estimated]: Boolean(first()?.zonal || first()?.interpolated),
            [styles.scheduled]: !first()?.live,
            [styles.now]: (first()?.eta ?? 1) <= 0,
          }}
          style={{color: freshnessVar(first()?.live ?? false, first()?.bus?.freshness)}}
          title={arrivalTitle(first())}>
          <When arrival={first()} />
        </span>
        <Show when={props.line.arrivals.length > 1}>
          <span class={styles.next}>
            <For each={props.line.arrivals.slice(1, 3)}>
              {(arrival) => (
                <span
                  classList={{[styles.scheduled]: !arrival.live, [styles.now]: (arrival.eta ?? 1) <= 0}}
                  style={{color: freshnessVar(arrival.live, arrival.bus?.freshness)}}>
                  <When arrival={arrival} />
                </span>
              )}
            </For>
          </span>
        </Show>
      </span>
    </>
  );

  return (
    <Show
      when={props.href}
      fallback={
        <button class={styles.row} type='button' onClick={() => props.onClick?.()}>
          {content()}
        </button>
      }>
      {(href) => (
        <a class={styles.row} href={href()}>
          {content()}
        </a>
      )}
    </Show>
  );
}
