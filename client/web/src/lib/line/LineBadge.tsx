import {companyVar} from '~/lib/ui/palette';
import styles from './LineBadge.module.css';

// flat, pills too noisy in long lists
export default function LineBadge(props: {
  line: string;
  company?: string;
  size?: 'sm' | 'md';
  variant?: 'pill' | 'flat' | 'plain';
}) {
  return (
    <span
      class={`${styles.badge} ${styles[props.size ?? 'md']} ${styles[props.variant ?? 'pill']}`}
      style={{'--badge': companyVar(props.company)}}>
      {props.line}
    </span>
  );
}
