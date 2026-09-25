export type Service = 'urban' | 'suburban' | 'inter';

// display and pin split order
export const SERVICES: readonly Service[] = ['urban', 'suburban', 'inter'];

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
  only: string; // single-service stop
}

const PROVIDERS: Record<string, ProviderInfo> = {
  stm: {service: 'urban', letter: 'U', label: 'Urbanos', only: 'Solo urbano'},
  dnt: {service: 'suburban', letter: 'S', label: 'Suburbanos', only: 'Solo suburbano'},
};

export const INTER: ProviderInfo = {
  service: 'inter',
  letter: 'I',
  label: 'Interdepartamentales',
  only: 'Solo interdepartamental',
};

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

// urban first
export function orderedRefs(refs: readonly string[]): string[] {
  return [...refs].sort((a, b) => order(a) - order(b));
}

function order(ref: string): number {
  return SERVICES.indexOf(providerInfo(providerOf(ref)).service);
}
