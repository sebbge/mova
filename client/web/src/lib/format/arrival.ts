// clock time past an hour
export function eta(minutes: number | undefined, now = new Date()): string {
  if (minutes === undefined) return '—';
  if (minutes <= 0) return 'Ahora';
  if (minutes <= 60) return `${minutes}'`;

  const at = new Date(now.getTime() + minutes * 60_000);
  return at.toLocaleTimeString('es-UY', {hour: '2-digit', minute: '2-digit', hourCycle: 'h23'});
}
