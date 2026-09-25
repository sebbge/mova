import busSvg from '~/lib/assets/bus.svg?raw';
import styles from './StopIcon.module.css';

export default function StopIcon(props: {size?: number}) {
  return (
    <span
      class={styles.pin}
      style={{'--pin-size': `${props.size ?? 36}px`}}
      aria-hidden='true'
      // eslint-disable-next-line solid/no-innerhtml
      innerHTML={busSvg}
    />
  );
}
