#!/usr/bin/env python3
"""Measures the map's climate classes from NASA POWER's daily reanalysis (1991-2020) and writes GEO's climate tables.

Each class of tools/data/climate_sites.toml is measured at its places: every day's mean temperature, precipitation,
wind speed at 10 m and clear-sky index (all-sky over clear-sky surface irradiance). Per class and month:

- temperature: mean and standard deviation;
- rain: the share of dry days (under 1 mm, the WMO wet-day threshold), and a gamma fitted to wet days' amounts by
  maximum likelihood (Thom's estimator);
- wind: a Weibull by the moment method of Justus et al. (1978);
- clear-sky index: a beta by the method of moments;

and per class, each variable's persistence: the lag-one correlation of its normal scores, ranked within each calendar
month so the seasons do not count as persistence. A class's value is the mean of its places'. The highland threshold
of a band is halfway between the mean elevation of its lowland places' reanalysis cells and its highland places'.

Each place's days are kept in data/sources/raw/power/, gzipped, so the tables rebuild without the network.

    python3 tools/data/climate.py [--fetch]
"""
import argparse
import csv
import datetime
import gzip
import io
import json
import math
import statistics
import sys
import time
import tomllib
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SITES = ROOT / "tools" / "data" / "climate_sites.toml"
RAW = ROOT / "data" / "sources" / "raw" / "power"
OUT = ROOT / "data" / "shared" / "GEO_climate.toml"
URL = (
    "https://power.larc.nasa.gov/api/temporal/daily/point?parameters=T2M,PRECTOTCORR,WS10M,ALLSKY_SFC_SW_DWN,"
    "CLRSKY_SFC_SW_DWN&community=RE&longitude={lon}&latitude={lat}&start=19910101&end=20201231&format=JSON"
)
WET_MM = 1.0
MIN_SUNLIT = 30
MIN_WET = 30
PARAMS = ["T2M", "PRECTOTCORR", "WS10M", "ALLSKY_SFC_SW_DWN", "CLRSKY_SFC_SW_DWN"]


def log(msg):
    print(msg, flush=True)


def site_file(site):
    return RAW / f"{site['lat']:+07.2f}_{site['lon']:+08.2f}.csv.gz"


def fetch(site):
    path = site_file(site)
    if path.exists():
        return
    for attempt in range(4):
        try:
            with urllib.request.urlopen(URL.format(**site), timeout=120) as r:
                body = json.load(r)
            break
        except Exception as e:  # network errors are retried, then reported
            log(f"  {site['name']}: {e}; retrying")
            time.sleep(2 ** (attempt + 1))
    else:
        sys.exit(f"{site['name']}: no answer from NASA POWER")
    elevation = body["geometry"]["coordinates"][2]
    p = body["properties"]["parameter"]
    buf = io.StringIO()
    w = csv.writer(buf)
    w.writerow(["date", *PARAMS, "elevation_m"])
    for day in sorted(p["T2M"]):
        w.writerow([day, *(p[k][day] for k in PARAMS), elevation])
    RAW.mkdir(parents=True, exist_ok=True)
    path.write_bytes(gzip.compress(buf.getvalue().encode(), mtime=0))


def days(site):
    """Each day's values, a variable the reanalysis does not cover that day (its fill value) as None."""
    with gzip.open(site_file(site), "rt") as f:
        rows = list(csv.DictReader(f))
    out = []
    for r in rows:
        t, p, w, alls, clrs = (None if float(r[k]) == -999.0 else float(r[k]) for k in PARAMS)
        csi = alls / clrs if alls is not None and clrs is not None and clrs > 0 else None
        out.append((datetime.date(int(r["date"][:4]), int(r["date"][4:6]), int(r["date"][6:])), t, p, w, csi))
    return out, float(rows[0]["elevation_m"])


def gamma_mle(xs):
    mean = statistics.fmean(xs)
    a = math.log(mean) - statistics.fmean(math.log(x) for x in xs)
    shape = (1 + math.sqrt(1 + 4 * a / 3)) / (4 * a)
    return shape, mean / shape


def weibull_moments(xs):
    mean, sd = statistics.fmean(xs), statistics.pstdev(xs)
    k = (sd / mean) ** -1.086
    return k, mean / math.gamma(1 + 1 / k)


def beta_moments(xs):
    m, v = statistics.fmean(xs), statistics.pvariance(xs)
    common = m * (1 - m) / v - 1
    return m * common, (1 - m) * common


def normal_scores(series, months):
    """Each value's normal score, ranked among the values of its calendar month, ties sharing their mean rank."""
    scores = [0.0] * len(series)
    for month in range(1, 13):
        idx = [i for i, m in enumerate(months) if m == month]
        order = sorted(idx, key=lambda i: series[i])
        n, pos = len(order), 0
        while pos < n:
            end = pos
            while end + 1 < n and series[order[end + 1]] == series[order[pos]]:
                end += 1
            rank = (pos + end) / 2 + 1
            z = statistics.NormalDist().inv_cdf(rank / (n + 1))
            for j in range(pos, end + 1):
                scores[order[j]] = z
            pos = end + 1
    return scores


def lag_one(values, dates):
    """The lag-one correlation of a variable's normal scores over consecutive days it covers."""
    kept = [(d, v, d.month) for d, v in zip(dates, values) if v is not None]
    scores = normal_scores([v for _, v, _ in kept], [m for _, _, m in kept])
    pairs = [
        (scores[i], scores[i + 1]) for i in range(len(kept) - 1) if (kept[i + 1][0] - kept[i][0]).days == 1
    ]
    xs, ys = zip(*pairs)
    return statistics.correlation(xs, ys)


def measure(site):
    rows, elevation = days(site)
    dates = [r[0] for r in rows]
    column = lambda i, sel: [r[i] for r in sel if r[i] is not None]
    all_csi = column(4, rows)
    all_wet = [x for x in column(2, rows) if x >= WET_MM]
    monthly = {}
    for m in range(1, 13):
        sel = [r for r in rows if r[0].month == m]
        temps, rain, wind, csi = column(1, sel), column(2, sel), column(3, sel), column(4, sel)
        wet = [x for x in rain if x >= WET_MM]
        monthly[m] = {
            "temperature_mean": statistics.fmean(temps),
            "temperature_sd": statistics.pstdev(temps),
            "dry_share": 1 - len(wet) / len(rain),
            # A month with almost no wet days fits no gamma of its own; its few wet days take the place's over the
            # whole year.
            "rain": gamma_mle(wet if len(wet) >= MIN_WET else all_wet),
            "wind": weibull_moments(wind),
            # A month of polar night has no clear-sky irradiance, so no index; its sunshine takes the place's
            # distribution over all its sunlit days, which multiplies nothing there.
            "sunshine": beta_moments(csi if len(csi) >= MIN_SUNLIT else all_csi),
        }
    persistence = [lag_one([r[i] for r in rows], dates) for i in (1, 2, 3, 4)]
    return monthly, persistence, elevation


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--fetch", action="store_true", help="download the places not yet kept")
    args = ap.parse_args()
    spec = tomllib.loads(SITES.read_text())
    classes = spec["class"]
    if args.fetch:
        sites = [s for c in classes for s in c["sites"]]
        for i, s in enumerate(sites, 1):
            log(f"[{i}/{len(sites)}] {s['name']}")
            fetch(s)
    measured = []
    for n, c in enumerate(classes):
        per_site = [measure(s) for s in c["sites"]]
        log(f"class {n}: {c['band']} {c['kind']} from {', '.join(s['name'] for s in c['sites'])}")
        measured.append(per_site)
    write(spec, measured)


def mean_of(values):
    present = [v for v in values if v is not None]
    return statistics.fmean(present) if present else None


def write(spec, measured):
    classes = spec["class"]
    names = ", ".join(sorted({s["name"] for c in classes for s in c["sites"]}))
    rows = list(range(len(classes)))

    def table(entry, exp, key, doc):
        values = []
        for per_site in measured:
            row = []
            for m in range(1, 13):
                row.append(round(mean_of([key(site[0][m]) for site in per_site]), exp))
            values.append(row)
        return primitive(entry, doc, f"{{ rows = {rows}, columns = {list(range(1, 13))}, values = {values}, outside = \"refuse\" }}")

    def primitive(entry, doc, value):
        return (
            f'[[primitive]]\nid = "{entry}"\nkind = "ENDOWMENT"\nowner = "GEO"\n'
            f'source = "measured"\n'
            f'source_ref = "{doc} NASA POWER daily (MERRA-2, CERES SYN1deg), 1991-2020, measured by '
            f'tools/data/climate.py at the places of tools/data/climate_sites.toml: {names}."\nvalue = {value}\n'
        )

    rain = lambda k: (lambda month: month["rain"][k])
    parts = [
        "# GEO's climate tables, written by tools/data/climate.py from NASA POWER's daily reanalysis; do not edit.\n",
        table("GEO.temperature_mean", 1, lambda m: m["temperature_mean"], "Mean daily temperature, degrees C."),
        table("GEO.temperature_sd", 1, lambda m: m["temperature_sd"], "Daily temperature's standard deviation, degrees C."),
        table("GEO.dry_share", 3, lambda m: m["dry_share"], "Share of days under 1 mm of rain."),
        table("GEO.rain_shape", 2, rain(0), "Gamma shape of wet days' rain, by maximum likelihood; a month of fewer than 30 wet days in the thirty years takes the place's over the year."),
        table("GEO.rain_scale", 1, rain(1), "Gamma scale of wet days' rain, mm."),
        table("GEO.wind_shape", 2, lambda m: m["wind"][0], "Weibull shape of daily mean wind at 10 m, by moments."),
        table("GEO.wind_scale", 1, lambda m: m["wind"][1], "Weibull scale of daily mean wind at 10 m, m/s."),
        table("GEO.sunshine_a", 2, lambda m: m["sunshine"][0], "Beta shape a of the daily clear-sky index, by moments; in a month of polar night, the place's over all its sunlit days."),
        table("GEO.sunshine_b", 2, lambda m: m["sunshine"][1], "Beta shape b of the daily clear-sky index, by moments."),
    ]
    persistence = [[round(mean_of([site[1][v] for site in per_site]), 3) for v in range(4)] for per_site in measured]
    parts.append(primitive(
        "GEO.persistence",
        "Lag-one correlation of each variable's normal scores within calendar months, in the order temperature, rain, wind, sunshine.",
        f"{{ rows = {rows}, columns = [0, 1, 2, 3], values = {persistence}, outside = \"refuse\" }}",
    ))
    bands = spec["bands"]
    kinds = spec["kinds"]
    index = {(c["band"], c["kind"]): n for n, c in enumerate(classes)}
    lowland = [[index[(b, k)] for k in kinds] for b in bands]
    highland = [index[(b, "highland")] for b in bands]
    thresholds = []
    for b in bands:
        low = mean_of([site[2] for k in kinds for site in measured[index[(b, k)]]])
        high = mean_of([site[2] for site in measured[index[(b, "highland")]]])
        thresholds.append(round((low + high) / 2))
    parts.append(primitive(
        "GEO.climate_lowland",
        "A lowland tile's class by latitude band (degrees) and distance to the sea (km): coastal under 50, inland to 300, interior beyond.",
        f"{{ rows = {bands}, columns = {spec['distances_km']}, values = {lowland}, outside = \"edge\" }}",
    ))
    parts.append(primitive(
        "GEO.highland_class",
        "Each latitude band's highland class.",
        f"{{ axis = {bands}, values = {highland}, outside = \"edge\" }}",
    ))
    parts.append(primitive(
        "GEO.highland_elevation",
        "Elevation above which a tile's climate is its band's highland class: halfway between the reanalysis cells' mean elevation of the band's lowland and highland places, m.",
        f"{{ axis = {bands}, values = {thresholds}, outside = \"edge\" }}",
    ))
    OUT.write_text("\n".join(parts))
    log(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
