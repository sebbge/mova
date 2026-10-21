import type {Fare} from '~/lib/api/types';

export function distance(meters: number): string {
  return meters < 1000 ? `${Math.round(meters)} m` : `${(meters / 1000).toFixed(1)} km`;
}

export function clock(iso: string): string {
  return iso.slice(11, 16);
}

export function duration(secs: number): string {
  const minutes = Math.max(1, Math.round(secs / 60));
  if (minutes < 60) return `${minutes} min`;
  const rest = minutes % 60;
  return rest ? `${Math.floor(minutes / 60)} h ${rest} min` : `${minutes / 60} h`;
}

export function money(amount: number): string {
  return `$${Number.isInteger(amount) ? amount : amount.toFixed(2).replace('.', ',')}`;
}

export function fareLabel(fare: Fare): string {
  if (fare.total === null) return 'Precio no disponible';
  return fare.total === 0 ? 'Gratis' : money(fare.total);
}

export function localIso(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function departureLabel(at: string | null): string {
  if (!at) return 'Salir ahora';
  const today = localIso(new Date()).slice(0, 10);
  const time = at.slice(11, 16);
  return at.slice(0, 10) === today
    ? `Salir a las ${time}`
    : `Salir el ${at.slice(8, 10)}/${at.slice(5, 7)} a las ${time}`;
}
