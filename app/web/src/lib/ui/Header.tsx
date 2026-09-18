import {Show} from 'solid-js';
import styles from './Header.module.css';

export default function Header(props: {back?: string}) {
  return (
    <header class={styles.header}>
      <div class={styles.inner}>
        <Show when={props.back}>
          {(back) => (
            <a class={styles.back} href={back()} aria-label='Volver'>
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
          )}
        </Show>
        <a class={styles.brand} href='/'>
          bondi
        </a>
      </div>
    </header>
  );
}
