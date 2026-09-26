export type Service = 'urban' | 'suburban' | 'inter' | 'neutral';

// display and pin split order
export const SERVICES: readonly Service[] = ['urban', 'suburban', 'inter', 'neutral'];

// one pin per combo
export const SERVICE_COMBOS: readonly (readonly Service[])[] = [
  ...SERVICES.map((s) => [s]),
  ...SERVICES.flatMap((a, i) => SERVICES.slice(i + 1).map((b) => [a, b])),
];

export function comboKey(services: readonly Service[]): string {
  return services.join('+');
}

export interface ProviderInfo {
  service: Service;
  letter: string;
  label: string;
  only?: string; // single-service stop
  attribution?: {label: string; source: string; license: string; href: string}; // license credit
}

const PROVIDERS: Record<string, ProviderInfo> = {
  stm: {service: 'urban', letter: 'U', label: 'Urbanos', only: 'Solo urbano'},
  dnt: {service: 'suburban', letter: 'S', label: 'Suburbanos', only: 'Solo suburbano'},
  inter: {service: 'inter', letter: 'I', label: 'Interdepartamentales', only: 'Solo interdepartamental'},
  codesa: {service: 'neutral', letter: 'P', label: 'Paradas'}, // Maldonado has no urban/suburban split
  solantigua: {
    service: 'urban',
    letter: 'U',
    label: 'Urbanos',
    only: 'Solo urbano',
    attribution: {
      label: 'Horarios y paradas',
      source: 'colonia-gtfs',
      license: 'CC BY 4.0',
      href: 'https://github.com/manufarfaro/colonia-gtfs',
    },
  },
};

const WITHOUT_LINES = new Set(['mdoturismo', 'micro']); // live only, no lines

export const DEFAULT_PROVIDER = PROVIDERS.stm; // unknown -> urban

export function providerOf(ref: string): string {
  const i = ref.indexOf('-');
  return i === -1 ? '' : ref.slice(0, i);
}

export function providerInfo(id: string | undefined): ProviderInfo {
  return (id && PROVIDERS[id]) || DEFAULT_PROVIDER;
}

export function serviceOf(ref: string): Service {
  return providerInfo(providerOf(ref)).service;
}

export function hasLines(provider: string): boolean {
  return !WITHOUT_LINES.has(provider);
}

// urban first
export function orderedRefs(refs: readonly string[]): string[] {
  return [...refs].sort((a, b) => order(a) - order(b));
}

// no public number
export function placeKicker(ref: string, name: string): string | null {
  if (serviceOf(ref) !== 'inter') return null;
  return /^terminal/i.test(name) ? 'Terminal' : 'Parada';
}

function order(ref: string): number {
  return SERVICES.indexOf(providerInfo(providerOf(ref)).service);
}
