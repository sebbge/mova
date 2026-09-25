import {For, type JSX} from 'solid-js';
import styles from './SheetTabs.module.css';

export default function SheetTabs<T extends {id: string; disabled?: boolean}>(props: {
  label: string;
  tabs: T[];
  active: string;
  tab: (tab: T) => JSX.Element;
  onSelect?: (id: string) => void;
}) {
  return (
    <nav class={styles.tabs} aria-label={props.label}>
      <For each={props.tabs}>
        {(t) => (
          <button
            classList={{[styles.active]: t.id === props.active}}
            disabled={t.disabled}
            aria-current={t.id === props.active ? 'true' : undefined}
            onClick={() => props.onSelect?.(t.id)}>
            {props.tab(t)}
          </button>
        )}
      </For>
    </nav>
  );
}
