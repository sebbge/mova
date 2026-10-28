import type {StopArrival} from '~/lib/api/types';

// clock time past an hour
export function eta(minutes: number | undefined, now = new Date()): string {
  if (minutes === undefined) return '—';
  if (minutes <= 0) return 'Ahora';
  if (minutes <= 60) return `${minutes}'`;

  const at = new Date(now.getTime() + minutes * 60_000);
  return at.toLocaleTimeString('es-UY', {hour: '2-digit', minute: '2-digit', hourCycle: 'h23'});
}

export function arrivalLabel(arrival: Pick<StopArrival, 'eta' | 'stop_position'> | undefined): string {
  if (arrival?.stop_position === 'at') return 'En parada';
  if (arrival?.stop_position === 'next') return 'Próxima';
  return eta(arrival?.eta);
}

// screen readers read "12'" as "12 apostrophe"
export function arrivalSpoken(arrival: Pick<StopArrival, 'eta' | 'stop_position' | 'live'> | undefined): string {
  const minutes = arrival?.eta;
  const origin = arrival?.live ? ', en vivo' : ', según horario';
  if (arrival?.stop_position || minutes === undefined || minutes <= 0) return arrivalLabel(arrival) + origin;
  if (minutes > 60) return `a las ${eta(minutes)}${origin}`;
  return (minutes === 1 ? 'en 1 minuto' : `en ${minutes} minutos`) + origin;
}

export function arrivalTitle(
  arrival: Pick<StopArrival, 'live' | 'zonal' | 'interpolated' | 'stop_position'> | undefined
): string {
  if (arrival?.stop_position === 'at') return 'El operador lo ubica en esta parada';
  if (arrival?.stop_position === 'next') return 'El operador indica que esta es su próxima parada';
  if (arrival?.live) return 'Posición en vivo';
  if (arrival?.zonal) return 'Horario zonal: la planilla lo publica en la esquina, no en esta parada';
  if (arrival?.interpolated) return 'Hora estimada entre dos puntos de la planilla';
  return 'Horario de planilla';
}

// operator's schedule, not freshness
export function delayLabel(minutes: number | undefined): string | undefined {
  if (minutes === undefined) return undefined;
  if (minutes === 0) return 'Puntual';
  return minutes > 0 ? `${minutes}' de atraso` : `${-minutes}' adelantado`;
}
