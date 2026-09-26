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
  return slug ? `var(--company-${slug})` : 'var(--text-secondary)';
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

export function freshnessVar(live: boolean, freshness?: BusFreshness): string {
  if (!live) return 'var(--text-secondary)';
  return FRESHNESS[freshness ?? 'live'];
}
