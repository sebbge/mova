import type {BusFreshness} from '~/lib/api/types';

const COMPANIES: Record<string, string> = {
  CUTCSA: 'cutcsa',
  COME: 'come',
  UCOT: 'ucot',
  COETC: 'coetc',
  CODESA: 'codesa',
  'MALDONADO TURISMO': 'mdoturismo',
  MICRO: 'micro',
  'SOL ANTIGUA': 'solantigua',
  CITA: 'cita',
  COPSA: 'copsa',
  'RUTAS DEL PLATA': 'rutasdelplata',
  COT: 'cot',
  TURISMAR: 'turismar',
  'BRUNO TI': 'turismar',
  CASANOVA: 'casanova',
  'TALA-PANDO-MONTEVIDEO': 'tpm',
  'NUÑEZ T. Y T.': 'nunez',
  'AGENCIA CENTRAL': 'agenciacentral',
  CHADRE: 'chadre',
  SABELIN: 'sabelin',
  EGA: 'ega',
  NOSSAR: 'nossar',
  'EXPRESO MINUANO': 'nossar',
  COPAY: 'copay',
  'RUTAS DEL SOL': 'rutasdelsol',
  'TUR ESTE': 'tureste',
  TURIL: 'turil',
  'EL NORTEÑO': 'elnorteno',
  BUQUEBUS: 'buquebus',
};

const FRESHNESS: Record<BusFreshness, string> = {
  live: 'var(--freshness-live)',
  low_precision: 'var(--freshness-low)',
  imprecise: 'var(--freshness-imprecise)',
};

export function companySlug(company: string | undefined): string | undefined {
  return company ? COMPANIES[company.toUpperCase()] : undefined;
}

export function companyVar(company: string | undefined): string {
  const slug = companySlug(company);
  return slug ? `var(--company-${slug})` : 'var(--company-unknown)';
}

export function companyOutlineVar(company: string | undefined): string {
  const slug = companySlug(company);
  return slug ? `var(--company-${slug}-outline)` : 'var(--text-hint)';
}

// unknown -> urban
export function companyTint(company: string | undefined): {header: string; list: string} {
  const slug = companySlug(company);
  return slug
    ? {header: `var(--company-${slug}-header)`, list: `var(--company-${slug}-list)`}
    : {header: 'var(--service-urban-header)', list: 'var(--service-urban-list)'};
}

// schedule in text color
export function freshnessVar(live: boolean, freshness?: BusFreshness): string {
  if (!live) return 'inherit';
  return FRESHNESS[freshness ?? 'live'];
}
