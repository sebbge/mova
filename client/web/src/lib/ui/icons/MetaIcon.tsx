import {Match, Show, Switch} from 'solid-js';
import styles from './MetaIcon.module.css';

export type MetaKind = 'emissions' | 'thermal' | 'access';

// decoration without a label
export default function MetaIcon(props: {kind: MetaKind; label?: string; size?: number}) {
  return (
    <svg
      class={styles.icon}
      classList={{[styles.solid]: props.kind === 'access'}}
      viewBox='0 0 24 24'
      width={props.size ?? 15}
      height={props.size ?? 15}
      role={props.label ? 'img' : undefined}
      aria-label={props.label}
      aria-hidden={props.label ? undefined : 'true'}>
      <Show when={props.label}>{(label) => <title>{label()}</title>}</Show>
      <Switch>
        <Match when={props.kind === 'emissions'}>
          <path d='M11 20a10 10 0 0010-10 25.9 25.9 0 00-1.04-7.281 1 1 0 00-1.755-.325C15.833 5.5 13 5.5 9.8 6.1A7 7 0 0011 20' />
          <path d='M2 21a5 5 0 012.911-4.544C7.613 15.212 8.351 15.24 11 13' />
        </Match>
        <Match when={props.kind === 'thermal'}>
          <path d='m10 20-1.25-2.5L6 18' />
          <path d='M10 4 8.75 6.5 6 6' />
          <path d='m14 20 1.25-2.5L18 18' />
          <path d='m14 4 1.25 2.5L18 6' />
          <path d='m17 21-3-6h-4' />
          <path d='m17 3-3 6 1.5 3' />
          <path d='M2 12h6.5L10 9' />
          <path d='m20 10-1.5 2 1.5 2' />
          <path d='M22 12h-6.5L14 15' />
          <path d='m4 10 1.5 2L4 14' />
          <path d='m7 21 3-6-1.5-3' />
          <path d='m7 3 3 6h4' />
        </Match>
        <Match when={props.kind === 'access'}>
          <circle cx='12' cy='4' r='2' />
          <path d='M19 13v-2c-1.54.02-3.09-.75-4.07-1.83l-1.29-1.43c-.17-.19-.38-.34-.61-.45-.01 0-.01-.01-.02-.01H13c-.35-.2-.75-.3-1.19-.26C10.76 7.11 10 8.04 10 9.09V15c0 1.1.9 2 2 2h5v5h2v-5.5c0-1.1-.9-2-2-2h-3v-3.45c1.29 1.07 3.25 1.94 5 1.95m-6.17 5c-.41 1.16-1.52 2-2.83 2-1.66 0-3-1.34-3-3 0-1.31.84-2.41 2-2.83V12.1c-2.28.46-4 2.48-4 4.9 0 2.76 2.24 5 5 5 2.42 0 4.44-1.72 4.9-4z' />
        </Match>
      </Switch>
    </svg>
  );
}
