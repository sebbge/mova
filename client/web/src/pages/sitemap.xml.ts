import {PUBLIC_SITE_URL} from 'astro:env/client';
import {REGIONS} from '~/lib/domain/regions';
import {href} from '~/lib/ui/href';
import {lineAreas, stopRefs} from './_paths';

export async function GET() {
  const [stops, lines] = await Promise.all([stopRefs(), lineAreas()]);

  const urls = ['/'];
  for (const region of REGIONS) {
    urls.push(href.region(region.slug));
    for (const {line} of lines.filter((l) => l.area === region.id)) urls.push(href.line(region.slug, line));
  }
  for (const {region, id} of stops) urls.push(href.stop(region.slug, id));

  const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls.map((u) => `  <url><loc>${PUBLIC_SITE_URL}${u}</loc></url>`).join('\n')}
</urlset>`;

  return new Response(body, {headers: {'content-type': 'application/xml'}});
}
