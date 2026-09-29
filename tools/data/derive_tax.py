#!/usr/bin/env python3
"""Derives the developing group's income tax bands from the IMF's means for low-income developing countries.

The IMF reports the bands' edges as multiples of output per person; the world's bands are multiples of the mean wage.
The two are related by the group's mean wage over its output per person: each economy's employees' mean monthly
earnings times twelve (ILOSTAT) over its GDP per head (World Bank), both in local currency and the same year, at its
latest year reporting both; the group's median.

    python3 tools/data/derive_tax.py

Writes data/profiles/developing/TAX.toml.
"""
import json

import pandas as pd

from derive import LEVELS, RAW
import profile_files

LEVEL = "developing"
MONTHS = 12
# IMF Working Paper 22/20 (Collecting Taxes data, 2019 values): the means for low-income developing countries of the
# personal income tax's first taxed income and top-rate threshold, in multiples of output per person, and its rates.
EDGES_PER_OUTPUT = [0.0, 1.3, 34.0]
RATES = [0.0, 0.104, 0.290]
# The standard rate of value-added tax: the median over the group's economies in PwC's Worldwide Tax Summaries.
CONSUMPTION = ("0.160", "The standard rate of value-added tax: 16.0%, the median over 38 lower-middle- and low-income "
               "economies (PwC Worldwide Tax Summaries, VAT rates quick chart, fetched 2026-09-26).")


def wage_over_output() -> tuple:
    """The group's median of mean yearly earnings over GDP per head, and the economies it is over."""
    earnings = pd.read_csv(RAW / "ilo" / "mean_earnings.csv").groupby(["iso3", "year"]).value.median() * MONTHS
    gdp = pd.read_csv(RAW / "wdi" / "NY.GDP.PCAP.CN.csv").set_index(["iso3", "year"]).value
    both = pd.concat([earnings.rename("wage"), gdp.rename("gdp")], axis=1).dropna().reset_index()
    both = both.sort_values("year").groupby("iso3").last()
    level = pd.read_csv(RAW / "wb" / "countries.csv").set_index("iso3").income_group.map(LEVELS)
    ratio = (both.wage / both.gdp)[both.index.map(level) == LEVEL]
    return float(ratio.median()), len(ratio)


def main() -> None:
    m = json.loads((RAW / "manifest.json").read_text())
    ratio, n = wage_over_output()
    edges = [e / ratio for e in EDGES_PER_OUTPUT]
    fetched = m["sources"]["wage_to_output"]["fetched"]
    imf = ("the IMF's means for low-income developing countries: tax starting at 1.3 times output per person at a "
           "lowest rate of 10.4%, the top rate 29% from 34 times (Collecting Taxes data, IMF Working Paper 22/20, 2019 "
           "values)")
    scale = (f"each multiple of output per person over the group's mean wage over output per person, {ratio:.2f}: the "
             f"median over {n} economies of the group of employees' mean monthly earnings times twelve over GDP per "
             f"head, both in local currency at each one's latest year reporting both (ILOSTAT DF_EAR_EMTA_SEX_CUR_NB, "
             f"World Bank NY.GDP.PCAP.CN, fetched {fetched})")
    entries = [
        ("TAX.income_band_edges", f"Each income tax band's lower edge as a multiple of the mean wage: {imf}, {scale}.",
         "{ axis = [0, 1, 2], values = [" + ", ".join(f"{e:.2f}" for e in edges) + '], outside = "refuse" }'),
        ("TAX.income_band_rates", f"Each income tax band's marginal rate: {imf}.",
         "{ axis = [0, 1, 2], values = [" + ", ".join(f"{r:.3f}" for r in RATES) + '], outside = "refuse" }'),
        ("TAX.consumption_rate", CONSUMPTION[1], f'"{CONSUMPTION[0]}"'),
    ]
    lines = ["# The developing group's taxes (spec TAX.1): the income tax's marginal bands over a member's yearly wage "
             "and the consumption tax's rate, derived by tools/data/derive_tax.py; never edited by hand."]
    for pid, ref, value in entries:
        lines += ["", "[[primitive]]", f'id = "{pid}"', 'kind = "POLICY"', 'owner = "TAX"',
                  'decided_by = "parliament"', 'source = "measured"', f"source_ref = {json.dumps(ref)}",
                  f"value = {value}"]
    profile_files.put_text(LEVEL, "\n".join(lines) + "\n")
    print(f"mean wage over output per person {ratio:.3f} over {n} economies; edges {edges}")


if __name__ == "__main__":
    main()
