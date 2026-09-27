import styles from './ThemeToggle.module.css';
import {theme} from './theme';

// icons switch by css, prerendered html can't know the theme
export default function ThemeToggle() {
  return (
    <button
      class={styles.toggle}
      type='button'
      aria-label={theme.dark ? 'Usar tema claro' : 'Usar tema oscuro'}
      onClick={() => theme.toggle()}>
      <svg class={styles.moon} viewBox='0 0 24 24' width='20' height='20' aria-hidden='true'>
        <path
          d='M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z'
          fill='none'
          stroke='currentColor'
          stroke-width='2'
          stroke-linejoin='round'
        />
      </svg>
      <svg class={styles.sun} viewBox='0 0 24 24' width='20' height='20' aria-hidden='true'>
        <g fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round'>
          <circle cx='12' cy='12' r='4' />
          <path d='M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4' />
        </g>
      </svg>
    </button>
  );
}
