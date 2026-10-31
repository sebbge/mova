import {createSignal, For, Show} from 'solid-js';
import {href} from '~/lib/ui/href';
import type {CoverageSummary, Department} from './coverage';
import {DEPARTMENT_PATHS, DEPARTMENTS_VIEWBOX} from './departments';
import styles from './UruguayMap.module.css';

const LEGEND = [
  {coverage: 'urban', label: 'Líneas urbanas'},
  {coverage: 'suburban', label: 'Metropolitanas'},
  {coverage: 'inter', label: 'Solo interdepartamentales'},
] as const;

const count = (n: number) => n.toLocaleString('es-UY');
const atLeast = (n: number, step: number) => count(Math.floor(n / step) * step);

function summary(d: Department): string {
  if (d.liveOnly) return 'Buses en vivo';
  if (d.lines > 0) return `${count(d.lines)} ${d.lines === 1 ? 'línea' : 'líneas'} · ${count(d.stops)} paradas`;
  return `${count(d.stops)} ${d.stops === 1 ? 'parada' : 'paradas'}`;
}

// hover lights both
export default function UruguayMap(props: {coverage: CoverageSummary}) {
  const [active, setActive] = createSignal<string | null>(null);
  const shown = () => props.coverage.departments.find((d) => d.id === active());
  const local = () =>
    props.coverage.departments
      .filter((d) => d.coverage !== 'inter')
      .sort((a, b) => b.lines - a.lines || b.stops - a.stops);
  const inter = () =>
    props.coverage.departments.filter((d) => d.coverage === 'inter').sort((a, b) => a.name.localeCompare(b.name, 'es'));

  const track = (id: string) => ({
    onPointerEnter: () => setActive(id),
    onPointerLeave: () => setActive(null),
    onFocus: () => setActive(id),
    onBlur: () => setActive(null),
  });

  return (
    <section class={styles.coverage} aria-labelledby='coverage-title'>
      <div class={styles.copy}>
        <h2 class='title' id='coverage-title'>
          Todo Uruguay, departamento por departamento
        </h2>
        <p>
          Más de {atLeast(props.coverage.lines, 50)} líneas y {atLeast(props.coverage.stops, 500)} paradas. Los
          interdepartamentales llegan a los 19 departamentos.
        </p>
      </div>

      <div class={styles.atlas}>
        <figure class={styles.map}>
          {/* the list is the accessible path */}
          <svg viewBox={DEPARTMENTS_VIEWBOX} aria-hidden='true'>
            <For each={props.coverage.departments}>
              {(d) => (
                <a
                  href={href.region(d.slug)}
                  tabindex='-1'
                  aria-label={d.name}
                  class={`${styles.dept} ${styles[d.coverage]}`}
                  classList={{[styles.active]: active() === d.id}}
                  onPointerEnter={() => setActive(d.id)}
                  onPointerLeave={() => setActive(null)}>
                  <path d={DEPARTMENT_PATHS[d.id]} />
                </a>
              )}
            </For>
          </svg>
          <figcaption class={styles.readout} aria-hidden='true'>
            <Show
              when={shown()}
              fallback={
                <ul class={styles.legend}>
                  <For each={LEGEND}>
                    {(item) => (
                      <li>
                        <span class={`${styles.swatch} ${styles[item.coverage]}`} />
                        {item.label}
                      </li>
                    )}
                  </For>
                </ul>
              }>
              {(d) => (
                <>
                  <strong>{d().name}</strong>
                  <span>{summary(d())}</span>
                </>
              )}
            </Show>
          </figcaption>
        </figure>

        <div class={styles.lists}>
          <ul class={`rows ${styles.local}`}>
            <For each={local()}>
              {(d) => (
                <li>
                  <a href={href.region(d.slug)} classList={{[styles.active]: active() === d.id}} {...track(d.id)}>
                    <span class={`${styles.swatch} ${styles[d.coverage]}`} aria-hidden='true' />
                    <span class={styles.name}>{d.name}</span>
                    <span class={styles.meta}>{summary(d)}</span>
                  </a>
                </li>
              )}
            </For>
          </ul>

          <div class={styles.inter}>
            <h3>Solo interdepartamentales</h3>
            <ul>
              <For each={inter()}>
                {(d) => (
                  <li>
                    <a
                      href={href.region(d.slug)}
                      classList={{[styles.active]: active() === d.id}}
                      aria-label={`${d.name}, ${summary(d)}`}
                      {...track(d.id)}>
                      {d.name}
                    </a>
                  </li>
                )}
              </For>
            </ul>
          </div>
        </div>
      </div>
    </section>
  );
}
