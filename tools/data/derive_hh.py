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

Reads data/sources/raw/icio/households_2019.csv (fetched by tools/data/fetch_tec.py). derive_economy.py reads the
shares into households' column of the final uses' composition; this prints them.
"""
import json

import numpy as np
import pandas as pd

from derive import RAW
from derive_tec import LEFT_OUT, MIN_COUNTRIES, PRODUCTS, YEAR, groups, io_tables, price_levels, taken_share

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


def budget_shares(level: str) -> tuple:
    """The group's budget shares over the products, each at its places and summing to one, with the median share of
    households' spending the products leave out and what they rest on."""
    note = json.loads((RAW / "manifest.json").read_text())["sources"]["icio"]
    d = pd.read_csv(RAW_FILE)
    spent = {c: g.set_index("row").value for c, g in d.groupby("iso3")}
    economies = [c for c in members().get(level, []) if c in spent]
    prods = products()
    stack, left = zip(*(shares(spent[c], prods) for c in economies))
    values = np.array([float(v) for v in apportion(np.median(np.stack(stack), axis=0))])
    ref = (f"households' budget shares over the products: the share of their final consumption expenditure (the "
           f"inter-country tables' HFCE column, at basic prices) spent on each, the median over the {len(economies)} "
           f"economies whose tables the group's ways are derived from ({', '.join(sorted(economies))}), renormalised "
           f"after leaving out finance, real estate, public administration and households as employers "
           f"({', '.join(LEFT_OUT)}; a median {float(np.median(left)):.1%} of the spending) ({note['title']}, {YEAR}, "
           f"fetched {note['fetched']})")
    return values, ref


def main() -> None:
    for level in ["developed", "emerging", "developing"]:
        values, _ = budget_shares(level)
        print(level, " ".join(f"{v:.6f}" for v in values))


if __name__ == "__main__":
    main()
