import type {BusFreshness} from '~/lib/api/types';

const COMPANY: Record<string, string> = {
  CUTCSA: 'var(--company-cutcsa)',
  COME: 'var(--company-come)',
  UCOT: 'var(--company-ucot)',
  COETC: 'var(--company-coetc)',
};

const FRESHNESS: Record<BusFreshness, string> = {
  live: 'var(--freshness-live)',
  low_precision: 'var(--freshness-low)',
  imprecise: 'var(--freshness-imprecise)',
};

export function companyVar(company: string | undefined): string {
  return COMPANY[company?.toUpperCase() ?? ''] ?? 'var(--text-secondary)';
}

export function freshnessVar(live: boolean, freshness?: BusFreshness): string {
  if (!live) return 'var(--text-secondary)';
  return FRESHNESS[freshness ?? 'live'];
}
