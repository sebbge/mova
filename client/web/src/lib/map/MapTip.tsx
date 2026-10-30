import {Show, type JSX} from 'solid-js';
import styles from './MapTip.module.css';

// mouse only, decoration
export default function MapTip(props: {
  text: string;
  subtitle?: string;
  lead: JSX.Element; // icon or badge
  trail?: JSX.Element;
}) {
  return (
    <div class={styles.tip} aria-hidden='true'>
      {props.lead}
      <span class={styles.body}>
        <span class={styles.text}>{props.text}</span>
        <Show when={props.subtitle}>
          <span class={styles.subtitle}>{props.subtitle}</span>
        </Show>
      </span>
      <Show when={props.trail}>
        <span class={styles.trail}>{props.trail}</span>
      </Show>
    </div>
  );
}
