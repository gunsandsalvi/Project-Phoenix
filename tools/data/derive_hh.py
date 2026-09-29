#!/usr/bin/env python3
"""Derives each country group's households' budget shares over the products from the input-output tables the opening
ways are derived from.

A share is the part of households' final consumption expenditure (the OECD's inter-country tables' HFCE column, at
basic prices, summed over where what was bought came from) spent on a product: its industries' part, and of a mining
industry the resource households take, as the ways split each mining industry by its users; the median over the
group's economies and renormalised to sum to one. Finance, real estate, public administration and households as
employers are left out, as the ways leave them out: they are paid for by fees, rents and taxes, or are not firms'
products. The group's economies are the ones its ways are the median over.

    python3 tools/data/derive_hh.py

Reads data/sources/raw/icio/households_2019.csv (fetched by tools/data/fetch_tec.py). Writes
data/profiles/<level>/HH.toml.
"""
import json

import numpy as np
import pandas as pd

from derive import RAW
from derive_tec import LEFT_OUT, MIN_COUNTRIES, PRODUCTS, YEAR, groups, io_tables, price_levels, taken_share
import profile_files

RAW_FILE = RAW / "icio" / f"households_{YEAR}.csv"
PLACES = 6


# Households as a user of a mining industry's output: none of the industries that transform a resource, so they take
# the resource other users take.
HOUSEHOLDS = "HFCE"


def products() -> list:
    """Each product with the ICIO industries it spans and the part of their output households take as it."""
    return [(p[0], p[2], taken_share(HOUSEHOLDS, p[6]) if p[6] is not None else 1.0) for p in PRODUCTS]


def members() -> dict:
    """Each group's economies: those its ways are the median over, the tables' economies with ICP price levels."""
    level_of = groups()
    pls = price_levels()
    out = {}
    for iso3 in io_tables():
        if iso3 in pls.index and iso3 in level_of.index:
            out.setdefault(level_of[iso3], []).append(iso3)
    return out


def shares(spent: pd.Series, prods: list):
    """One economy's spending share of each product and of what is left out, over all its purchases at basic
    prices."""
    codes = sorted({c for _, cs, _ in prods for c in cs})
    total = float(spent.reindex(codes + LEFT_OUT).dropna().sum())
    by_product = np.array([float(spent.reindex(cs).dropna().sum()) * part for _, cs, part in prods]) / total
    left = float(spent.reindex(LEFT_OUT).dropna().sum()) / total
    return by_product, left


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
    prods = products()
    stack, left = zip(*(shares(spent[c], prods) for c in economies))
    median = np.median(np.stack(stack), axis=0)
    values = apportion(median)
    left_median = float(np.median(left))
    source = "measured" if len(economies) >= MIN_COUNTRIES else "estimated"
    axis = ", ".join(str(i) for i in range(len(prods)))
    ref = (f"The share of households' final consumption expenditure (the inter-country tables' HFCE column, at "
           f"basic prices, each purchase summed over where it came from) spent on each product ("
           + ", ".join(p[0] for p in prods) + "; of a mining industry's output the resource other users than the "
           f"industry transforming one take, as the ways split it): the median over the {len(economies)} economies of the World Bank's "
           f"{level} income groups whose tables the group's ways are derived from ({', '.join(sorted(economies))}), "
           f"renormalised to sum to one after leaving out finance, real estate, public administration and households "
           f"as employers ({', '.join(LEFT_OUT)}; a median {left_median:.1%} of the spending), paid for by fees, "
           f"rents and taxes ({note['title']}, {YEAR}, fetched {note['fetched']}).")
    lines = [f"# The {level} group's households: their budget shares over the products (spec HH.5, HH.20), derived by",
             "# tools/data/derive_hh.py; never edited by hand.",
             "", "[[primitive]]", 'id = "HH.budget_shares"', 'kind = "PREFERENCE"', 'owner = "HH"',
             f'source = "{source}"', f"source_ref = {json.dumps(ref)}",
             f"value = {{ axis = [{axis}], values = [{', '.join(values)}], "
             'outside = "refuse" }']
    profile_files.put_text(level, "\n".join(lines) + "\n")
    return values, left_median


def main() -> None:
    note = json.loads((RAW / "manifest.json").read_text())["sources"]["icio"]
    d = pd.read_csv(RAW_FILE)
    spent = {c: g.set_index("row").value for c, g in d.groupby("iso3")}
    groups_ = members()
    for level in ["developed", "emerging", "developing"]:
        economies = [c for c in groups_.get(level, []) if c in spent]
        values, left = write_level(level, economies, spent, note)
        print(level, len(economies), " ".join(values), f"left out {left:.3f}")


if __name__ == "__main__":
    main()
