#!/usr/bin/env python3
"""Writes GEO's hazard and deposit tables, data/shared/GEO_hazards.toml.

A hazard's yearly chance of starting on a tile is estimated from EM-DAT's disasters (via Our World in Data), the mean
number a year of 1994-2023, over the world's land (World Bank AG.LND.TOTL.K2) in tiles: the chance on an average
tile. Its exposure classes scale it by a quarter, one and four, sheltered to exposed ground. A tile's class comes
from its terrain and its climate class, read from the measured climate (GEO_climate.toml): floods on humid plains,
storms where the wind is strong, droughts where dry days prevail, earthquakes in mountains; the coast raises floods
and storms one class. The spread and severity of each class, and the deposits, are assumed, each with its reason.

    python3 tools/data/hazards.py [--fetch]
"""
import argparse
import csv
import json
import statistics
import tomllib
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RAW = ROOT / "data" / "sources" / "raw"
DISASTERS = RAW / "owid" / "number-of-natural-disaster-events.csv"
LAND = RAW / "wb" / "AG.LND.TOTL.K2.json"
CLIMATE = ROOT / "data" / "shared" / "GEO_climate.toml"
GEO = ROOT / "data" / "shared" / "GEO.toml"
OUT = ROOT / "data" / "shared" / "GEO_hazards.toml"
OWID_URL = "https://ourworldindata.org/grapher/number-of-natural-disaster-events.csv?v=1&csvType=full&useColumnShortNames=true"
WDI_URL = "https://api.worldbank.org/v2/country/WLD/indicator/AG.LND.TOTL.K2?format=json&date=2015:2023"
FIRST, LAST = 1994, 2023
MULTIPLIERS = [0.25, 1.0, 4.0]

# Each hazard: its EM-DAT entity, and by exposure class its spread chance to a neighbour and the mean share of what
# stands on a struck tile it destroys, with the beta's first shape. Spreads stay below site percolation's threshold on
# the eight-neighbour grid (about 0.407), so a footprint ends; the nearer the threshold, the wider it runs.
HAZARDS = {
    "flood": ("Flood", [0.15, 0.25, 0.32], [0.01, 0.03, 0.08], 0.8),
    "storm": ("Extreme weather", [0.20, 0.30, 0.36], [0.005, 0.02, 0.05], 0.8),
    "earthquake": ("Earthquake", [0.10, 0.25, 0.33], [0.01, 0.05, 0.15], 0.6),
    "drought": ("Drought", [0.30, 0.36, 0.39], [0.02, 0.05, 0.12], 1.0),
}


def fetch():
    DISASTERS.parent.mkdir(parents=True, exist_ok=True)
    LAND.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(OWID_URL, timeout=120) as r:
        DISASTERS.write_bytes(r.read())
    with urllib.request.urlopen(WDI_URL, timeout=120) as r:
        LAND.write_bytes(r.read())


def events_per_year():
    counts = {}
    with DISASTERS.open() as f:
        for row in csv.DictReader(f):
            if FIRST <= int(row["year"]) <= LAST:
                counts.setdefault(row["entity"], []).append(float(row["n_events"]))
    return {k: statistics.fmean(v) for k, v in counts.items()}


def land_km2():
    body = json.loads(LAND.read_text())
    latest = max((r for r in body[1] if r["value"]), key=lambda r: r["date"])
    return latest["value"], latest["date"]


def prim(entry, kind, source, ref, value):
    return f'[[primitive]]\nid = "{entry}"\nkind = "{kind}"\nowner = "GEO"\nsource = "{source}"\nsource_ref = "{ref}"\nvalue = {value}\n'


def table1(values, exp):
    return f'{{ axis = {list(range(len(values)))}, values = {[round(v, exp) for v in values]}, outside = "refuse" }}'


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--fetch", action="store_true")
    if ap.parse_args().fetch:
        fetch()
    geo = {p["id"]: p["value"] for p in tomllib.loads(GEO.read_text())["primitive"]}
    climate = {p["id"]: p["value"] for p in tomllib.loads(CLIMATE.read_text())["primitive"]}
    tile_km2 = (geo["GEO.tile_m"] / 1000) ** 2
    terrains = len(geo["GEO.terrain_max_elevation"]["axis"])
    dry = [statistics.fmean(row) for row in climate["GEO.dry_share"]["values"]]
    wind = [statistics.fmean(row) for row in climate["GEO.wind_scale"]["values"]]
    classes = len(dry)
    land, land_year = land_km2()
    tiles = land / tile_km2
    per_year = events_per_year()
    plains, mountains = 0, terrains - 1

    def exposure(hazard, coastal):
        rows = []
        for t in range(terrains):
            row = []
            for c in range(classes):
                if hazard == "flood":
                    e = (1 if t == plains else 0) if dry[c] < 0.85 else 0
                    e += 1 if coastal and t <= 1 else 0
                elif hazard == "storm":
                    e = 2 if wind[c] >= 5.0 else 1 if wind[c] >= 3.5 else 0
                    e = min(2, e + (1 if coastal else 0))
                elif hazard == "earthquake":
                    e = 2 if t == mountains else 1 if t > plains else 0
                else:
                    e = 2 if dry[c] >= 0.85 else 1 if dry[c] >= 0.7 else 0
                row.append(min(2, e))
            rows.append(row)
        return f'{{ rows = {list(range(terrains))}, columns = {list(range(classes))}, values = {rows}, outside = "refuse" }}'

    parts = ["# GEO's hazard and deposit tables, written by tools/data/hazards.py; do not edit.\n"]
    for name, (entity, spread, mean_share, a) in HAZARDS.items():
        rate = per_year[entity] / tiles
        parts.append(prim(f"GEO.{name}_exposure_inland", "TECHNOLOGY", "assumed",
            f"Exposure class of an inland tile to {name}s by terrain and climate class, read from the measured climate: floods on plains of humid classes (under 85% dry days), storms by the climate's mean wind scale (3.5 and 5 m/s), earthquakes by relief, droughts by dry days (70% and 85%).",
            exposure(name, False)))
        parts.append(prim(f"GEO.{name}_exposure_coastal", "TECHNOLOGY", "assumed",
            f"Exposure class of a coastal tile to {name}s: as inland, with floods on low coasts and storms one class higher.",
            exposure(name, True)))
        parts.append(prim(f"GEO.{name}_rate", "TECHNOLOGY", "estimated",
            f"Yearly chance {'an' if name[0] in 'aeiou' else 'a'} {name} starts on a tile, by exposure class: EM-DAT's {per_year[entity]:.1f} a year ({entity}, {FIRST}-{LAST}, via Our World in Data) over the world's {land:,.0f} km2 of land ({land_year}, World Bank AG.LND.TOTL.K2) in tiles of {tile_km2:.0f} km2, scaled by a quarter, one and four from sheltered to exposed ground.",
            table1([rate * m for m in MULTIPLIERS], 8)))
        parts.append(prim(f"GEO.{name}_spread", "TECHNOLOGY", "assumed",
            f"Chance a {name} spreads to a neighbouring tile, by the neighbour's exposure class: below site percolation's threshold on the eight-neighbour grid (about 0.407), so every footprint ends, wider the more exposed the ground.",
            table1(spread, 3)))
        b = [a * (1 - m) / m for m in mean_share]
        parts.append(prim(f"GEO.{name}_severity_a", "TECHNOLOGY", "assumed",
            f"First shape of the share of what stands on a struck tile a {name} destroys, by exposure class: most is damaged, little destroyed, mean shares {mean_share}.",
            table1([a] * len(mean_share), 2)))
        parts.append(prim(f"GEO.{name}_severity_b", "TECHNOLOGY", "assumed",
            f"Second shape of the share a {name} destroys, by exposure class, giving the mean shares {mean_share}.",
            table1(b, 2)))
    deposits = [
        ("metal ore", [0.0, 0.002, 0.006, 0.012], 0.0, 0.8, 16.1, 1.5, 0),
        ("coal", [0.004, 0.006, 0.003, 0.0], 0.0, 0.5, 17.7, 1.5, 0),
        ("oil and gas", [0.004, 0.002, 0.0005, 0.0], 0.0, 0.6, 16.1, 2.0, 0),
        ("building stone", [0.0, 0.03, 0.05, 0.08], 0.0, 0.3, 0.0, 0.0, 1),
    ]
    names = ", ".join(f"{i} {d[0]}" for i, d in enumerate(deposits))
    parts.append(prim("GEO.deposit_density", "ENDOWMENT", "assumed",
        f"Chance a tile holds a deposit, by resource ({names}) and terrain class: ores and stone in raised ground, coal in plains and hills, oil and gas in sedimentary lowlands; none is present on every terrain. The resources' own data come with the goods that extract them.",
        f'{{ rows = {list(range(len(deposits)))}, columns = {list(range(terrains))}, values = {[d[1] for d in deposits]}, outside = "refuse" }}'))
    for key, i, doc, exp in [("grade_mu", 2, "Mean of a deposit's log grade, a quality index around one", 3),
                             ("grade_sigma", 3, "Standard deviation of a deposit's log grade", 3),
                             ("quantity_mu", 4, "Mean of a finite deposit's log opening quantity in tonnes (ore about ten million, coal fifty million)", 3),
                             ("quantity_sigma", 5, "Standard deviation of a finite deposit's log opening quantity: deposit sizes span orders of magnitude", 3)]:
        parts.append(prim(f"GEO.deposit_{key}", "ENDOWMENT", "assumed", f"{doc}, by resource ({names}).",
            table1([d[i] for d in deposits], exp)))
    parts.append(prim("GEO.deposit_unbounded", "ENDOWMENT", "assumed",
        f"Whether a resource's deposits are unbounded, by resource ({names}): building stone is, at the scale of a world's quarries.",
        table1([d[6] for d in deposits], 0)))
    OUT.write_text("\n".join(parts))
    print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
