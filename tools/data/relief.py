#!/usr/bin/env python3
"""Derives GEO's measured height curves from NOAA's ETOPO1 relief of the analogue region (data/sources/raw/etopo/, by
tools/data/relief_fetch.py) and writes data/shared/GEO_relief.toml:

- GEO.land_heights: the land's heights at each part per thousand of the land's cells, lowest first;
- GEO.sea_depths: the sea's heights (below zero) at each part per thousand of the sea's cells, deepest first;
- GEO.land_relief: the range of heights within the land's windows of about 10 km (three cells of latitude by four of
  longitude, a tile's span) at each part per thousand of them, least first.

Each cell and window is weighted by its area (the cosine of its latitude). It also prints the share of land in each
terrain class of data/shared/GEO.toml, read over the same windows, so the generated map's terrain can be judged against
the real ground's.

    python3 tools/data/relief.py
"""
import csv
import gzip
import math
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RAW = ROOT / "data" / "sources" / "raw" / "etopo"
OUT = ROOT / "data" / "shared" / "GEO_relief.toml"
GEO = ROOT / "data" / "shared" / "GEO.toml"
AXIS = list(range(0, 1000, 10)) + [995, 999, 1000]


def read():
    rows = {}
    for path in sorted(RAW.glob("*.csv.gz")):
        with gzip.open(path, "rt") as f:
            for lat, lon, alt in csv.reader(f):
                rows.setdefault(float(lat), {})[float(lon)] = float(alt)
    lats = sorted(rows)
    lons = sorted(rows[lats[0]])
    return lats, lons, [[rows[la][lo] for lo in lons] for la in lats]


def curve(values, weights):
    order = sorted(range(len(values)), key=lambda i: values[i])
    total = sum(weights)
    out, acc, k = [], 0.0, 0
    targets = [a / 1000 * total for a in AXIS]
    for i in order:
        acc += weights[i]
        while k < len(targets) and acc >= targets[k]:
            out.append(round(values[i]))
            k += 1
    while len(out) < len(AXIS):
        out.append(round(values[order[-1]]))
    return out


def main():
    lats, lons, grid = read()
    land, land_w, sea, sea_w = [], [], [], []
    for la, row in zip(lats, grid):
        w = math.cos(math.radians(la))
        for h in row:
            (land if h > 0 else sea).append(h)
            (land_w if h > 0 else sea_w).append(w)
    land_curve, sea_curve = curve(land, land_w), curve(sea, sea_w)
    geo = {p["id"]: p["value"] for p in tomllib.loads(GEO.read_text())["primitive"]}
    elev, relief = geo["GEO.terrain_max_elevation"]["values"], geo["GEO.terrain_max_relief"]["values"]
    counts = [0.0] * len(elev)
    ranges, range_w = [], []
    for i in range(0, len(lats) - 2, 3):
        w = math.cos(math.radians(lats[i]))
        for j in range(0, len(lons) - 3, 4):
            block = [grid[i + a][j + b] for a in range(3) for b in range(4)]
            if sum(h > 0 for h in block) * 2 < len(block):
                continue
            mean, rng = sum(block) / len(block), max(block) - min(block)
            ranges.append(rng)
            range_w.append(w)
            k = next((c for c, (e, r) in enumerate(zip(elev, relief)) if mean <= e and rng <= r), len(elev) - 1)
            counts[k] += w
    shares = [c / sum(counts) for c in counts]
    relief_curve = curve(ranges, range_w)
    ref = (f"NOAA ETOPO1 (Amante and Eakins 2009) via NOAA CoastWatch ERDDAP (etopo180), 35-58°N and 10°W-60°E every "
           f"2 arc-minutes, {len(land):,} land and {len(sea):,} sea cells weighted by area; derived by "
           f"tools/data/relief.py.")
    text = "# GEO's measured height curves, written by tools/data/relief.py from ETOPO1; do not edit.\n\n"
    for id_, doc, values in [
        ("GEO.land_heights", "The land's heights in metres at each part per thousand of its area, lowest first.", land_curve),
        ("GEO.sea_depths", "The sea's heights in metres at each part per thousand of its area, deepest first.", sea_curve),
        ("GEO.land_relief", f"The range of heights in metres within the land's windows of about 10 km (three cells of "
         f"latitude by four of longitude, {len(ranges):,} windows mostly land), at each part per thousand of their "
         f"area, least first.", relief_curve),
    ]:
        text += (f'[[primitive]]\nid = "{id_}"\nkind = "ENDOWMENT"\nowner = "GEO"\nsource = "measured"\n'
                 f'source_ref = "{doc} {ref}"\n'
                 f'value = {{ axis = {AXIS}, values = {values}, outside = "refuse" }}\n\n')
    OUT.write_text(text)
    print(f"wrote {OUT.relative_to(ROOT)}")
    print("land height quantiles (m) at 10, 25, 50, 75, 90, 99%:",
          [land_curve[AXIS.index(a)] for a in (100, 250, 500, 750, 900, 990)])
    print("real terrain shares by class of GEO.toml, over ~10 km windows:", [round(s, 3) for s in shares])


if __name__ == "__main__":
    main()
