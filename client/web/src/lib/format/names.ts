// no %20 in shared links
export function lineSlug(line: string): string {
  return line.replace(/ /g, '_');
}

export function lineFromSlug(slug: string): string {
  return slug.replace(/_/g, ' ');
}

// cut at the known company prefix, else at the last space
export function splitCompanyLine(line: string, company?: string): {line: string; company?: string} {
  if (company) return line.startsWith(`${company} `) ? {line: line.slice(company.length + 1), company} : {line};
  const i = line.lastIndexOf(' ');
  return i < 0 ? {line} : {line: line.slice(i + 1), company: line.slice(0, i)};
}

// "Andén 4" needs its station
export function fullStopName(name: string, station?: string | null): string {
  return station ? `${station} · ${name}` : name;
}

export function rawId(ref: string): string {
  const i = ref.indexOf('-');
  return i === -1 ? ref : ref.slice(i + 1);
}
