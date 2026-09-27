#!/usr/bin/env python3
"""Derives each group's currency in circulation as a share of GDP: each economy's currency in circulation over its
GDP, both in national currency, at its latest year from 2015 to 2019 reporting both; the median over the group's
economies.

    python3 tools/data/derive_mon.py

Writes data/profiles/<group>/CB_currency.toml.
"""
import json

import pandas as pd

from derive import LEVELS, PROFILES, RAW


def main() -> None:
    m = json.loads((RAW / "manifest.json").read_text())
    fetched = m["sources"]["currency"]["fetched"]
    cic = pd.read_csv(RAW / "imf_sdmx" / "CIC.csv").set_index(["iso3", "year"]).value
    gdp = pd.read_csv(RAW / "wdi" / "NY.GDP.MKTP.CN.csv").set_index(["iso3", "year"]).value
    both = pd.concat([cic.rename("cic"), gdp.rename("gdp")], axis=1).dropna().reset_index()
    both = both.sort_values("year").groupby("iso3").last()
    level = pd.read_csv(RAW / "wb" / "countries.csv").set_index("iso3").income_group.map(LEVELS)
    for group in sorted(set(LEVELS.values())):
        ratio = (both.cic / both.gdp * 100)[both.index.map(level) == group]
        value = f"{ratio.median():.2f}"
        ref = (f"Currency in circulation as a share of GDP: {value}%, the median over {len(ratio)} economies of the "
               f"group of currency in circulation over GDP, both in national currency at each one's latest year of "
               f"2015-2019 reporting both (IMF Monetary and Financial Statistics, central bank survey, "
               f"S121_L_CIC_IMB_CBS; World Bank NY.GDP.MKTP.CN; fetched {fetched}).")
        text = (f"# The {group} group's currency (spec MON.4): what its central bank has in circulation, derived by "
                f"tools/data/derive_mon.py; never edited by hand.\n\n[[primitive]]\nid = \"CB.currency\"\n"
                f"kind = \"ENDOWMENT\"\nowner = \"CB\"\nsource = \"measured\"\nsource_ref = \"{ref}\"\n"
                f"value = \"{value}\"\n")
        (PROFILES / group / "CB_currency.toml").write_text(text)
        print(group, value, len(ratio))


if __name__ == "__main__":
    main()
