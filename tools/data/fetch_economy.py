#!/usr/bin/env python3
"""Fetches the published data the one economic derivation (derive_economy.py) needs beyond what the other fetchers
keep, into data/sources/raw/, all for 2019, the last year before the pandemic:

- The OECD's inter-country input-output tables (ICIO, 2023 edition), whole: for each economy they report, what each
  industry used of every industry's output, and what each final use (households, non-profit institutions serving
  them, government, fixed investment, inventories) took of it, each summed over the countries it came from; and the
  taxes less subsidies on products and the value added in each column.
- The OECD's national accounts, value added by activity split into compensation of employees, gross operating surplus
  and mixed income, and other taxes less subsidies on production (Table 6), in national currency.
- The OECD's financial balance sheets, non-consolidated, by sector and instrument, assets and liabilities (Table 720),
  and its balance sheets for non-financial assets by sector and asset (Table 9B), in national currency.

    python3 tools/data/fetch_economy.py [--cache DIR] [--only NAME ...]
"""
import argparse
import csv
import datetime
import io
import json
import tempfile
import zipfile
from pathlib import Path

from fetch import RAW, get, log
from fetch_pop import countries, table

YEAR = 2019
ICIO = "https://stats.oecd.org/wbos/fileview2.aspx?IDFile=d1ab2315-298c-4e93-9a81-c6f2273139fe"
ICIO_REST = "ROW"
# The rows beside the industries: taxes less subsidies on products, and value added.
ICIO_ROWS = ("TLS", "VA")
# The final uses a closed economy keeps; exports are another economy's uses, and direct purchases abroad by
# residents (DPABR) are folded into households'.
FINAL = ("HFCE", "NPISH", "GGFC", "GFCF", "INVNT", "DPABR")

OECD_NAD = "https://sdmx.oecd.org/public/rest/data/OECD.SDD.NAD,{flow},/{key}?startPeriod={year}&endPeriod={year}" \
           "&format=csvfile"
# Value added by activity and its parts: compensation of employees, gross operating surplus and mixed income, and
# other taxes less subsidies on production.
VA_PARTS = ["B1G", "D1", "B2A3G", "D29X39"]
# The sectors a closed economy of households, firms, banks, a central bank and a government is read from: households
# with the institutions serving them, non-financial corporations, the central bank, deposit-taking corporations, all
# financial corporations, and general government; and the whole economy.
SECTORS = ["S1", "S1M", "S11", "S12", "S121", "S122", "S13"]
# Currency, transferable and other deposits, debt securities, loans, equity and investment fund shares, and all
# financial assets or liabilities.
INSTRUMENTS = ["F", "F21", "F22", "F29", "F3", "F4", "F5", "F51", "F52", "F6"]
# Dwellings, other buildings and structures, machinery and equipment with weapons systems, cultivated biological
# resources, intellectual property products, inventories, and land.
NONFIN = ["N111", "N112", "N11M", "N115", "N117", "N12", "N211"]


def icio(cache: Path, manifest: dict, iso3: set) -> None:
    """Each economy's flows of 2019, every use summed over the countries what was used came from."""
    path = cache / "icio_2016_2020.zip"
    if not path.exists():
        log("downloading ICIO")
        path.write_bytes(get(ICIO, timeout=3600))
    flows: dict = {}
    with zipfile.ZipFile(path) as z, z.open(f"{YEAR}_SML.csv") as f:
        reader = csv.reader(io.TextIOWrapper(f, encoding="utf-8-sig"))
        header = next(reader)[1:]
        for r in reader:
            row, values = r[0], r[1:]
            used = row if row in ICIO_ROWS else row.split("_", 1)[1] if "_" in row else None
            if used is None:
                continue
            for col, v in zip(header, values):
                if not v or float(v) == 0.0 or "_" not in col:
                    continue
                economy, what = col.split("_", 1)
                if economy == ICIO_REST or economy not in iso3:
                    continue
                if what in FINAL or (what[0].isalpha() and what[0].isupper()):
                    key = (economy, used, "HFCE" if what == "DPABR" else what)
                    flows[key] = flows.get(key, 0.0) + float(v)
    manifest["series"]["icio/flows"] = {
        "title": f"Inter-country input-output table, {YEAR}, millions of US dollars: every use of each industry's "
                 "output (row) by each industry and final use (column: households with their purchases abroad, "
                 "non-profit institutions serving them, government, fixed investment, inventories), summed over the "
                 "origins of what was used, with the taxes less subsidies on products and the value added of each "
                 "column; OECD ICIO 2023 edition",
        "rows": table(RAW / "icio" / f"flows_{YEAR}.csv", ["iso3", "row", "column", "value"],
                      [(e, u, w, f"{v:.3f}") for (e, u, w), v in sorted(flows.items())]),
    }
    manifest["sources"]["icio"] = {"title": "OECD Inter-Country Input-Output tables, 2023 edition", "url": ICIO}


def oecd_table(cache: Path, name: str, flow: str, key: str) -> list:
    """An OECD national accounts table's 2019 rows, downloaded once into the cache."""
    path = cache / f"oecd_{name}_{YEAR}.csv"
    if not path.exists():
        log(f"downloading OECD {flow}")
        path.write_bytes(get(OECD_NAD.format(flow=flow, key=key, year=YEAR), timeout=1800))
    with path.open(encoding="utf-8-sig") as f:
        return list(csv.DictReader(f))


def value_added(cache: Path, manifest: dict, iso3: set) -> None:
    rows = [(r["REF_AREA"], r["ACTIVITY"], r["TRANSACTION"], r["OBS_VALUE"], r["UNIT_MULT"])
            for r in oecd_table(cache, "value_added_parts", "DSD_NAMAIN10@DF_TABLE6", "A...." + "+".join(VA_PARTS)
                                + "....XDC.V..")
            if r["REF_AREA"] in iso3 and r["OBS_VALUE"] and r.get("SECTOR", "S1") in ("S1", "_Z")]
    manifest["series"]["oecd/value_added_parts"] = {
        "title": f"Gross value added by activity and its parts, compensation of employees (D1), gross operating surplus "
                 f"and mixed income (B2A3G) and other taxes less subsidies on production (D29X39), {YEAR}, current "
                 "prices, national currency (Table 6), OECD",
        "rows": table(RAW / "oecd" / f"value_added_parts_{YEAR}.csv",
                      ["iso3", "activity", "item", "value", "unit_mult"], sorted(rows)),
    }


def balance_sheets(cache: Path, manifest: dict, iso3: set) -> None:
    fin = oecd_table(cache, "financial_balance_sheets", "DSD_NASEC20@DF_T720R_A",
                     "A...." + "+".join(SECTORS) + ".S1.N.A+L.LE." + "+".join(INSTRUMENTS) + "..XDC......")
    rows = [(r["REF_AREA"], r["SECTOR"], r["ACCOUNTING_ENTRY"], r["INSTR_ASSET"], r["OBS_VALUE"], r["UNIT_MULT"])
            for r in fin if r["REF_AREA"] in iso3 and r["OBS_VALUE"] and r["MATURITY"] in ("T", "_Z")]
    manifest["series"]["oecd/financial_balance_sheets"] = {
        "title": f"Financial balance sheets, non-consolidated, closing stocks {YEAR}, by sector, counterpart the whole "
                 "economy, assets (A) and liabilities (L), by instrument, national currency (Table 720), OECD",
        "rows": table(RAW / "oecd" / f"financial_balance_sheets_{YEAR}.csv",
                      ["iso3", "sector", "entry", "instrument", "value", "unit_mult"], sorted(set(rows))),
    }
    nonfin = oecd_table(cache, "nonfinancial_balance_sheets", "DSD_NASEC10@DF_TABLE9B", "A.." +
                        "+".join(SECTORS) + "......XDC....")
    rows = [(r["REF_AREA"], r["SECTOR"], r["INSTR_ASSET"], r["OBS_VALUE"], r["UNIT_MULT"])
            for r in nonfin if r["REF_AREA"] in iso3 and r["OBS_VALUE"]
            and any(r["INSTR_ASSET"].startswith(a) for a in NONFIN)]
    manifest["series"]["oecd/nonfinancial_balance_sheets"] = {
        "title": f"Balance sheets for non-financial assets, closing stocks {YEAR}, by sector and asset, net, current "
                 "prices, national currency (Table 9B), OECD",
        "rows": table(RAW / "oecd" / f"nonfinancial_balance_sheets_{YEAR}.csv",
                      ["iso3", "sector", "asset", "value", "unit_mult"], sorted(set(rows))),
    }
    manifest["sources"]["oecd_sectors"] = {"title": "OECD Data Explorer, SDMX API, sector accounts",
                                            "url": OECD_NAD.format(flow="<table>", key="<key>", year=YEAR)}


SOURCES = {"icio": icio, "value_added": value_added, "balance_sheets": balance_sheets}
SOURCE_KEY = {"icio": "icio", "value_added": "oecd_nad", "balance_sheets": "oecd_sectors"}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cache", type=Path, default=None)
    parser.add_argument("--only", nargs="*", choices=sorted(SOURCES))
    args = parser.parse_args()
    cache = args.cache or Path(tempfile.mkdtemp())
    cache.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((RAW / "manifest.json").read_text())
    iso3 = countries()
    for name in args.only or sorted(SOURCES):
        SOURCES[name](cache, manifest, iso3)
        manifest["sources"].setdefault(SOURCE_KEY[name], {})["fetched"] = datetime.date.today().isoformat()
        log(f"{name} done")
    (RAW / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
