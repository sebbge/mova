import {Show, type JSX} from 'solid-js';
import styles from './SplitPill.module.css';

export default function SplitPill(props: {
  color: string;
  ink?: string; // plain part's color
  iconic?: boolean; // icon only, circle
  small?: boolean;
  lead?: JSX.Element;
  children: JSX.Element;
}) {
  return (
    <span
      class={styles.pill}
      classList={{[styles.small]: Boolean(props.small)}}
      style={{'--pill': props.color, '--pill-ink': props.ink}}>
      <Show when={props.lead}>
        <span class={styles.lead} classList={{[styles.iconic]: Boolean(props.iconic)}}>
          {props.lead}
        </span>
      </Show>
      <span class={styles.text}>{props.children}</span>
    </span>
  );
}
