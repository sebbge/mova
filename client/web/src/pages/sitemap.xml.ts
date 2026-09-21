import {PUBLIC_SITE_URL} from 'astro:env/client';
import {api} from '~/lib/api/client';
import {REGIONS} from '~/lib/domain/regions';
import {href} from '~/lib/ui/href';

export async function GET() {
  const [stops, routes] = await Promise.all([api.stops(), api.routes()]);
  const lines = [...new Set(routes.map((r) => r.route_short_name))];

  const urls = ['/'];
  for (const region of REGIONS) {
    urls.push(href.region(region.slug));
    for (const line of lines) urls.push(href.line(region.slug, line));
    for (const stop of stops) urls.push(href.stop(region.slug, stop.stop_id));
  }

  const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls.map((u) => `  <url><loc>${PUBLIC_SITE_URL}${u}</loc></url>`).join('\n')}
</urlset>`;

  return new Response(body, {headers: {'content-type': 'application/xml'}});
}
