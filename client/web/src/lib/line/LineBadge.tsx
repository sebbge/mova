import {companyVar} from '~/lib/ui/palette';
import styles from './LineBadge.module.css';

export default function LineBadge(props: {line: string; company?: string; size?: 'sm' | 'md'}) {
  return (
    <span class={`${styles.badge} ${styles[props.size ?? 'md']}`} style={{'--badge': companyVar(props.company)}}>
      {props.line}
    </span>
  );
}
