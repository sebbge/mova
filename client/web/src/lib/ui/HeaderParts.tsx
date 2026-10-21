import {Show} from 'solid-js';
import styles from './Header.module.css';

export function BackLink(props: {href: string}) {
  return (
    <a class={styles.back} href={props.href} aria-label='Volver'>
      <svg viewBox='0 0 24 24' width='22' height='22' aria-hidden='true'>
        <path
          d='M15 5l-7 7 7 7'
          fill='none'
          stroke='currentColor'
          stroke-width='2'
          stroke-linecap='round'
          stroke-linejoin='round'
        />
      </svg>
    </a>
  );
}

// link on pages, button on the map
export function TripQuery(props: {destination?: string; onSearch?: () => void}) {
  const content = () => (
    <>
      <svg viewBox='0 0 24 24' width='18' height='18' aria-hidden='true'>
        <circle cx='11' cy='11' r='6.5' fill='none' stroke='currentColor' stroke-width='2' />
        <path d='M16 16l4.5 4.5' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' />
      </svg>
      <span classList={{[styles.placeholder]: !props.destination}}>{props.destination ?? '¿A dónde vas?'}</span>
    </>
  );

  return (
    <Show
      when={props.onSearch}
      fallback={
        <a class={styles.search} href='/mapa?viaje'>
          {content()}
        </a>
      }>
      {(onSearch) => (
        <button class={styles.search} type='button' onClick={() => onSearch()()}>
          {content()}
        </button>
      )}
    </Show>
  );
}
