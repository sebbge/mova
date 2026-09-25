import {PUBLIC_SITE_URL} from 'astro:env/client';
import {api} from '~/lib/api/client';
import {REGIONS} from '~/lib/domain/regions';
import {href} from '~/lib/ui/href';
import {lineAreas} from './_paths';

export async function GET() {
  const [stopsByRegion, lines] = await Promise.all([Promise.all(REGIONS.map((r) => api.stops(r.id))), lineAreas()]);

  const urls = ['/'];
  for (const [i, region] of REGIONS.entries()) {
    urls.push(href.region(region.slug));
    for (const {line} of lines.filter((l) => l.area === region.id)) urls.push(href.line(region.slug, line));
    for (const stop of stopsByRegion[i]) urls.push(href.stop(region.slug, stop.id));
  }

  const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls.map((u) => `  <url><loc>${PUBLIC_SITE_URL}${u}</loc></url>`).join('\n')}
</urlset>`;

  return new Response(body, {headers: {'content-type': 'application/xml'}});
}
