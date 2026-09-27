import {Show} from 'solid-js';
import LineBadge from './LineBadge';
import styles from './LineLead.module.css';

export default function LineLead(props: {line: string; company?: string}) {
  return (
    <span class={styles.lead}>
      <LineBadge line={props.line} variant='plain' />
      <Show when={props.company}>
        <span class={styles.company}>{props.company}</span>
      </Show>
    </span>
  );
}
