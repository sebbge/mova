import type {JSX} from 'solid-js';
import styles from './Phone.module.css';

export default function Phone(props: {top?: string; children: JSX.Element}) {
  return (
    <div class={styles.phone}>
      <div class={styles.screen}>
        {/* behind the island */}
        <div class={styles.status} style={{'--top': props.top ?? 'var(--sheet)'}}>
          <span class={styles.island} />
        </div>
        {props.children}
      </div>
    </div>
  );
}
