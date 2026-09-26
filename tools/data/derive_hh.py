#!/usr/bin/env python3
"""Derives each country group's households' budget shares over the products' industries from the input-output tables
the opening ways are derived from.

A share is the part of households' final consumption expenditure (FIGARO's final-use column P3_S14, at basic prices,
summed over where what was bought came from) spent on an industry's products, the median over the group's economies
and renormalised to sum to one. Finance, real estate, public administration and households as employers are left
out, as the ways leave them out: they are paid for by fees, rents and taxes, or are not firms' products. The group's
economies are the ones its ways are the median over.

    python3 tools/data/derive_hh.py [--fetch]

--fetch downloads the households' column into data/sources/raw/figaro/households_2022.csv first (it is fetched too
when absent). Writes data/profiles/<level>/HH.toml.
"""
import argparse
import csv
import datetime
import gzip
import io
import json

import numpy as np
import pandas as pd

from derive import RAW
from derive_tec import LEFT_OUT, MIN_COUNTRIES, PRODUCTS, PROFILES, YEAR, groups, io_tables, price_levels
from fetch import get, log

HOUSEHOLDS = ("https://ec.europa.eu/eurostat/api/dissemination/sdmx/2.1/data/naio_10_fcp_ip4/A.P3_S14...MIO_EUR.?"
              f"startPeriod={YEAR}&endPeriod={YEAR}&format=SDMX-CSV&compressed=true")
RAW_FILE = RAW / "figaro" / f"households_{YEAR}.csv"
RAW_NOTE = RAW / "figaro" / f"households_{YEAR}.json"
TITLE = (f"Households' final consumption expenditure (P3_S14) at basic prices by product, {YEAR}, millions of euros, "
         "summed over the origins of what was bought (naio_10_fcp_ip4), Eurostat FIGARO")
# Eurostat's codes where they differ from ISO 3166.
EUROSTAT_ISO2 = {"EL": "GR", "UK": "GB"}
PLACES = 6


def industries() -> list:
    """The products' industries in the order the products first name them, each with the CPA codes it makes."""
    out = []
    for p in PRODUCTS:
        if out and out[-1][0] == p[1]:
            out[-1][1].extend(c for c in p[2] if c not in out[-1][1])
        else:
            out.append((p[1], list(p[2])))
    return out


def fetch() -> None:
    """Each destination economy's household purchases by product, summed over origins."""
    log("downloading FIGARO households' final consumption")
    to3 = {r["iso2"]: r["iso3"] for r in csv.DictReader((RAW / "wb" / "countries.csv").open())}
    summed = {}
    with gzip.open(io.BytesIO(get(HOUSEHOLDS, timeout=900)), "rt", encoding="utf-8-sig") as f:
        for r in csv.DictReader(f):
            if not r["prd_ava"].startswith("CPA_") or not r["OBS_VALUE"]:
                continue
            iso3 = to3.get(EUROSTAT_ISO2.get(r["c_dest"], r["c_dest"]))
            if iso3 is None:
                continue
            key = (iso3, r["prd_ava"].replace("CPA_", ""))
            summed[key] = summed.get(key, 0.0) + float(r["OBS_VALUE"])
    with RAW_FILE.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["iso3", "row", "value"])
        w.writerows((c, row, f"{v:.3f}") for (c, row), v in sorted(summed.items()))
    RAW_NOTE.write_text(json.dumps({"title": TITLE, "source": "Eurostat FIGARO, SDMX 2.1 API", "url": HOUSEHOLDS,
                                    "fetched": datetime.date.today().isoformat()}, indent=2) + "\n")


def members() -> dict:
    """Each group's economies: those its ways are the median over, the tables' economies with ICP price levels."""
    level_of = groups()
    pls = price_levels()
    out = {}
    for iso3 in io_tables():
        if iso3 in pls.index and iso3 in level_of.index:
            out.setdefault(level_of[iso3], []).append(iso3)
    return out


def shares(spent: pd.Series, inds: list):
    """One economy's spending share of each industry and of what is left out, over all its products."""
    total = float(spent.sum())
    by_industry = np.array([float(spent.reindex(codes).dropna().sum()) for _, codes in inds]) / total
    left = float(spent.reindex(LEFT_OUT).dropna().sum()) / total
    return by_industry, left


def apportion(values: np.ndarray) -> list:
    """The shares rounded to PLACES decimals so that they still sum to one: the parts left by rounding down go to the
    shares rounding dropped most of."""
    scale = 10 ** PLACES
    exact = values / values.sum() * scale
    units = np.floor(exact).astype(int)
    for i in np.argsort(-(exact - units), kind="stable")[:scale - int(units.sum())]:
        units[i] += 1
    return [f"{u / scale:.{PLACES}f}" for u in units]


def write_level(level: str, economies: list, spent: dict, note: dict) -> tuple:
    inds = industries()
    stack, left = zip(*(shares(spent[c], inds) for c in economies))
    median = np.median(np.stack(stack), axis=0)
    values = apportion(median)
    left_median = float(np.median(left))
    source = "measured" if len(economies) >= MIN_COUNTRIES else "estimated"
    axis = ", ".join(str(i) for i in range(len(inds)))
    ref = (f"The share of households' final consumption expenditure (FIGARO's final-use column P3_S14, at basic "
           f"prices, each purchase summed over where it came from) spent on each industry's products ("
           + ", ".join(i[0] for i in inds) + f"): the median over the {len(economies)} economies of the World Bank's "
           f"{level} income groups whose tables the group's ways are derived from ({', '.join(sorted(economies))}), "
           f"renormalised to sum to one after leaving out finance, real estate, public administration and households "
           f"as employers ({', '.join(LEFT_OUT)}; a median {left_median:.1%} of the spending), paid for by fees, "
           f"rents and taxes ({note['source']}, table naio_10_fcp_ip4, {YEAR}, fetched {note['fetched']}).")
    lines = [f"# The {level} group's households: their budget shares over the products (spec HH.5, HH.20), derived by",
             "# tools/data/derive_hh.py; never edited by hand.",
             "", "[[primitive]]", 'id = "HH.budget_shares"', 'kind = "PREFERENCE"', 'owner = "HH"',
             f'source = "{source}"', f"source_ref = {json.dumps(ref)}",
             f"value = {{ axis = [{axis}], values = [{', '.join(values)}], "
             'outside = "refuse" }']
    (PROFILES / level / "HH.toml").write_text("\n".join(lines) + "\n")
    return values, left_median


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fetch", action="store_true")
    if parser.parse_args().fetch or not RAW_FILE.exists():
        fetch()
    note = json.loads(RAW_NOTE.read_text())
    d = pd.read_csv(RAW_FILE)
    spent = {c: g.set_index("row").value for c, g in d.groupby("iso3")}
    groups_ = members()
    for level in ["developed", "emerging", "developing"]:
        economies = [c for c in groups_.get(level, []) if c in spent]
        values, left = write_level(level, economies, spent, note)
        print(level, len(economies), " ".join(values), f"left out {left:.3f}")


if __name__ == "__main__":
    main()
