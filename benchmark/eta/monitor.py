"""Mide qué tan bien acierta el backend los ETA en vivo de una parada.

Captura lo mismo que ve la web (llegadas y posiciones) y, al terminar, cruza cada ETA
anunciado con el momento en que ese bus pasó de verdad por la parada.

    python benchmark/eta/monitor.py --stop stm/3663 --minutes 30
    python benchmark/eta/monitor.py --analyze benchmark/eta/runs/<corrida>
"""

import argparse
import json
import math
import statistics
import sys
import time
import urllib.parse
import urllib.request
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

RUNS = Path(__file__).parent / "runs"
# Un bus que deja de figurar se sigue un rato más: hay que verlo cruzar la parada.
FOLLOW_S = 300
# Más lejos que esto del punto de la parada, el trazo no pasa por ella.
CROSS_M = 60
# Un ETA cuya hora anunciada cae tan cerca del final no llega a verificarse sin sesgo.
TAIL_S = 600
HORIZONS = [(0, 2), (2, 5), (5, 10), (10, 20), (20, 999)]


def now() -> datetime:
    return datetime.now(timezone.utc)


def parse_ts(value: str | None) -> float | None:
    return datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp() if value else None


def get(api: str, path: str):
    with urllib.request.urlopen(api + path, timeout=60) as response:
        return json.load(response)


# ── Captura ──────────────────────────────────────────────────────────────────


def capture(api: str, stop: str, minutes: float, every: float) -> Path:
    provider, stop_id = stop.split("/")
    run = RUNS / f"{now():%Y%m%d-%H%M}-{provider}-{stop_id}"
    run.mkdir(parents=True, exist_ok=True)

    info = get(api, f"/stop/{provider}/{stop_id}")
    meta = {"api": api, "stop": stop, "lat": info["lat"], "lng": info["lng"], "name": info["stop_name"],
            "started": now().isoformat(), "every": every}
    (run / "meta.json").write_text(json.dumps(meta, indent=2), encoding="utf-8")

    followed: dict[str, float] = {}
    deadline = time.time() + minutes * 60
    polls = 0
    with (run / "snapshots.jsonl").open("w", encoding="utf-8") as out:
        def write(row: dict):
            out.write(json.dumps(row, ensure_ascii=False) + "\n")

        while time.time() < deadline:
            tick = time.time()
            poll = now().isoformat()
            try:
                detail = get(api, f"/stop/arrivals/{provider}/{stop_id}")
                for line in detail["lines"]:
                    for a in line["arrivals"]:
                        bus = a.get("bus") or {}
                        write({"kind": "arrival", "poll": poll, "generated": detail.get("generated_at"),
                               "line": line["line"], "destination": line.get("destination"),
                               "eta": a.get("eta"), "live": a["live"], "source": a.get("source"),
                               "stop_position": a.get("stop_position"), "bus": bus.get("id"),
                               "bus_ts": bus.get("timestamp"), "location": bus.get("location"),
                               "freshness": bus.get("freshness"), "delay": bus.get("delay_min"),
                               "still": bus.get("still"), "past_stop": bus.get("past_stop")})
                        if bus.get("id"):
                            followed[bus["id"]] = tick

                ids = [b for b, seen in followed.items() if tick - seen <= FOLLOW_S]
                if ids:
                    query = urllib.parse.urlencode({"ids": ",".join(ids)})
                    for bus in get(api, f"/stop/buses/{provider}/{stop_id}?{query}"):
                        write({"kind": "position", "poll": poll, "bus": bus["id"], "line": bus["line"],
                               "bus_ts": bus.get("timestamp"), "location": bus["location"],
                               "speed": bus.get("speed"), "freshness": bus.get("freshness")})
            except Exception as error:  # noqa: BLE001 — la corrida sigue aunque falle un pedido
                write({"kind": "error", "poll": poll, "error": str(error)})
            out.flush()
            polls += 1
            print(f"\r{polls} lecturas, {len(followed)} buses vistos", end="", flush=True)
            time.sleep(max(0.0, every - (time.time() - tick)))

    meta["ended"] = now().isoformat()
    (run / "meta.json").write_text(json.dumps(meta, indent=2), encoding="utf-8")
    print(f"\ncaptura en {run}")
    return run


# ── Análisis ─────────────────────────────────────────────────────────────────


def crossing(points: list[tuple[float, float, float]]) -> float | None:
    """La primera vez que el bus pasa por la parada (en el origen), interpolada en el tramo."""
    for (t1, x1, y1), (t2, x2, y2) in zip(points, points[1:]):
        if math.hypot(x1, y1) <= CROSS_M / 2:
            return t1
        dx, dy = x2 - x1, y2 - y1
        length2 = dx * dx + dy * dy
        if length2 == 0:
            continue
        u = -(x1 * dx + y1 * dy) / length2
        if 0 <= u <= 1 and math.hypot(x1 + u * dx, y1 + u * dy) <= CROSS_M:
            return t1 + u * (t2 - t1)
    return None


def stats(errors: list[float]) -> dict:
    absolute = sorted(abs(e) for e in errors)
    return {
        "n": len(errors),
        "sesgo_s": round(statistics.fmean(errors)),
        "mediana_abs_s": round(statistics.median(absolute)),
        "p90_abs_s": round(absolute[min(len(absolute) - 1, int(len(absolute) * 0.9))]),
        "antes_pct": round(100 * sum(e < -60 for e in errors) / len(errors)),
        "despues_pct": round(100 * sum(e > 120 for e in errors) / len(errors)),
    }


def table(title: str, groups: dict[str, list[float]], minimum: int = 3) -> str:
    rows = [f"### {title}", "",
            "| grupo | n | sesgo | mediana abs | p90 abs | llegó >1' antes | llegó >2' después |",
            "|---|---|---|---|---|---|---|"]
    for name, errors in sorted(groups.items(), key=lambda g: -len(g[1])):
        if len(errors) < minimum:
            continue
        s = stats(errors)
        rows.append(f"| {name} | {s['n']} | {s['sesgo_s']:+} s | {s['mediana_abs_s']} s | {s['p90_abs_s']} s "
                    f"| {s['antes_pct']}% | {s['despues_pct']}% |")
    return "\n".join(rows) + "\n"


def analyze(run: Path) -> str:
    meta = json.loads((run / "meta.json").read_text(encoding="utf-8"))
    rows = [json.loads(line) for line in (run / "snapshots.jsonl").open(encoding="utf-8")]
    end = parse_ts(meta.get("ended")) or max(parse_ts(r["poll"]) for r in rows)
    lat0, lng0 = meta["lat"], meta["lng"]
    kx, ky = 111_320 * math.cos(math.radians(lat0)), 110_540

    # La trayectoria de cada bus, una vez por lectura AVL; sin hora, la del pedido.
    tracks: dict[str, dict[float, tuple[float, float]]] = defaultdict(dict)
    for r in rows:
        if r["kind"] in ("arrival", "position") and r.get("bus") and r.get("location"):
            t = parse_ts(r.get("bus_ts")) or parse_ts(r["poll"])
            loc = r["location"]
            tracks[r["bus"]][t] = ((loc["lng"] - lng0) * kx, (loc["lat"] - lat0) * ky)

    arrived: dict[str, float] = {}
    for bus, points in tracks.items():
        ordered = [(t, x, y) for t, (x, y) in sorted(points.items())]
        if (t := crossing(ordered)) is not None:
            arrived[bus] = t
    for r in rows:
        if r["kind"] == "arrival" and r.get("stop_position") == "at" and r.get("bus") not in arrived:
            arrived[r["bus"]] = parse_ts(r["poll"])

    by_source, by_horizon, by_line, by_freshness, by_source_horizon = (defaultdict(list) for _ in range(5))
    worst, unverified, seen = [], 0, set()
    for r in rows:
        if r["kind"] != "arrival" or not r["live"] or r.get("eta") is None or not r.get("bus"):
            continue
        made = parse_ts(r.get("generated")) or parse_ts(r["poll"])
        # La misma respuesta cacheada se lee varias veces: cuenta una.
        key = (r["bus"], made, r["eta"])
        if key in seen:
            continue
        seen.add(key)
        announced = made + r["eta"] * 60
        if announced > end - TAIL_S:
            continue
        actual = arrived.get(r["bus"])
        if actual is None or actual < made - 60:
            unverified += 1
            continue
        error = actual - announced
        horizon = next(f"{a}–{b}'" if b < 999 else f"{a}'+" for a, b in HORIZONS if a <= r["eta"] < b)
        source = r.get("source") or "?"
        by_source[source].append(error)
        by_horizon[horizon].append(error)
        by_source_horizon[f"{source} · {horizon}"].append(error)
        by_line[r["line"]].append(error)
        by_freshness[r.get("freshness") or "?"].append(error)
        worst.append((abs(error), error, r["line"], source, r["eta"], r["bus"]))

    every_error = [e for errors in by_source.values() for e in errors]
    lines = [
        f"# ETA en vivo — {meta['name']} ({meta['stop']})", "",
        f"{meta['started']} → {meta.get('ended', '?')}, lectura cada {meta['every']} s.", "",
        f"- Buses seguidos: {len(tracks)}; vistos cruzar la parada: {len(arrived)}.",
        f"- Predicciones verificadas: {len(every_error)}; sin verificar (no se vio cruzar): {unverified}.",
        "- Error = llegada real − hora anunciada. Negativo: llegó antes de lo anunciado (el usuario lo pierde).",
        "- El backend trunca el ETA a minutos: eso solo suma ~+30 s de sesgo.", "",
    ]
    if every_error:
        lines += [table("Total", {"todas": every_error}, 1), table("Por vía", by_source),
                  table("Por horizonte", by_horizon), table("Por vía y horizonte", by_source_horizon),
                  table("Por frescura de la posición", by_freshness), table("Por línea", by_line, 5)]
        lines += ["### Peores casos", "", "| error | línea | vía | ETA anunciado | bus |", "|---|---|---|---|---|"]
        for _, error, line, source, eta, bus in sorted(worst, reverse=True)[:12]:
            lines.append(f"| {error:+.0f} s | {line} | {source} | {eta}' | {bus} |")
    report = "\n".join(lines) + "\n"
    (run / "report.md").write_text(report, encoding="utf-8")
    return report


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--api", default="http://localhost:3000")
    parser.add_argument("--stop", default="stm/3663", help="proveedor/id")
    parser.add_argument("--minutes", type=float, default=30)
    parser.add_argument("--every", type=float, default=10, help="segundos entre lecturas")
    parser.add_argument("--analyze", type=Path, help="solo analizar una corrida ya capturada")
    args = parser.parse_args()
    print(analyze(args.analyze or capture(args.api, args.stop, args.minutes, args.every)))
