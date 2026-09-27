#!/usr/bin/env python3
"""Fetches the published data the opening ways are derived from, into data/sources/raw/.

- The OECD's inter-country input-output tables (ICIO, 2023 edition), industry by industry, 2019, the last year before
  the pandemic: for each of the 76 economies they report, what each industry used of every industry's output, summed
  over the countries it came from, with its taxes less subsidies on products and its value added, so each industry's
  output is its column's sum; and households' final consumption of each industry's output.
- ILOSTAT's employment and average weekly hours actually worked by economic activity (ISIC Rev. 4 sections) and
  occupation (ISCO-08 major groups), from labour force surveys.
- The OECD's national accounts: net fixed assets at current prices by activity and asset (Table 9A) and gross value
  added at current prices by activity (Table 6), in national currency, so an asset's stock per unit of value added
  needs no exchange rate.
- The World Bank's agricultural land area; the International Comparison Program's 2021 price levels by expenditure
  category (world = 100) and the United States' GDP deflator from the tables' year to the unit's, which turn each
  product's dollars into a quantity at world-average prices; and its commodity prices (the Pink Sheet) and the U.S.
  Geological Survey's unit value of crushed stone, which turn the extracted products' quantities into tonnes.

    python3 tools/data/fetch_tec.py [--cache DIR] [--only NAME ...]
"""
import argparse
import csv
import datetime
import gzip
import json
import io
import tempfile
import zipfile
from pathlib import Path

from fetch import RAW, get, log
from fetch_pop import countries, table

YEAR = 2019
# The year a product's unit is priced in: what a US cent bought at world-average prices then.
UNIT_YEAR = 2022
ICIO = "https://stats.oecd.org/wbos/fileview2.aspx?IDFile=d1ab2315-298c-4e93-9a81-c6f2273139fe"
# The rest of the world, reported only as an origin and a destination, not an economy.
ICIO_REST = "ROW"
# The rows below the industries: taxes less subsidies on the products used, and value added.
ICIO_ROWS = ["TLS", "VA"]

ILO = "https://sdmx.ilo.org/rest/data/ILO,{flow},1.0/all?startPeriod=2015"
ILO_CSV = "application/vnd.sdmx.data+csv;version=1.0.0"
# The labour force surveys nearest the tables' year: from three years before it on.
ILO_YEARS_BEFORE = 3
ILO_FLOWS = {
    "DF_EMP_TEMP_ECO_OCU_NB": ("employment", "Employment by economic activity and occupation, thousands"),
    "DF_HOW_TEMP_ECO_OCU_NB": ("hours", "Average weekly hours actually worked per employed person by economic activity "
                                        "and occupation"),
}

OECD = ("https://sdmx.oecd.org/public/rest/data/OECD.SDD.NAD,DSD_NAMAIN10@{flow},/{key}?"
        f"startPeriod={YEAR - 4}&endPeriod={YEAR}&format=csvfile")
# Net stocks of the fixed assets firms produce with; dwellings are households' and landlords' and are left out.
ASSETS = ["N112N", "N1131N", "N1132N", "N11ON", "N115N", "N117N"]
OECD_FLOWS = {
    "fixed_assets": ("DF_TABLE9A", "A........XDC.V..", "INSTR_ASSET", ASSETS,
                     "Net fixed assets at current prices by activity and asset, closing stocks, national currency, "
                     "millions (Table 9A)"),
    "value_added": ("DF_TABLE6", "A....B1G....XDC.V..", "TRANSACTION", ["B1G"],
                    "Gross value added at current prices by activity, national currency, millions (Table 6)"),
}

WB_LAND = ("https://api.worldbank.org/v2/country/all/indicator/AG.LND.AGRI.K2?format=json&date="
           f"{YEAR - 4}:{YEAR}&per_page=20000")


ICP = ("https://api.worldbank.org/v2/sources/90/country/all/series/{series}/classification/PX.WL/time/YR2021?"
       "format=json&per_page=1000")
# The ICP headings the products' price levels are read from.
ICP_SERIES = {
    "1000000": "GROSS DOMESTIC PRODUCT",
    "1101000": "FOOD AND NON-ALCOHOLIC BEVERAGES",
    "1101100": "FOOD",
    "1103000": "CLOTHING AND FOOTWEAR",
    "1105000": "FURNISHINGS, HOUSEHOLD EQUIPMENT AND ROUTINE HOUSEHOLD MAINTENANCE",
    "1107300": "TRANSPORT SERVICES",
    "1111000": "RESTAURANTS AND HOTELS",
    "1501100": "MACHINERY AND EQUIPMENT",
    "1501200": "CONSTRUCTION",
    "1501300": "OTHER PRODUCTS",
    "9080000": "ACTUAL HEALTH",
    "9120000": "ACTUAL EDUCATION",
    "9140000": "ACTUAL MISCELLANEOUS GOODS AND SERVICES",
}
DEFLATOR = (f"https://api.worldbank.org/v2/country/USA/indicator/NY.GDP.DEFL.ZS?format=json&date={YEAR}:{UNIT_YEAR}")
PINK = ("https://thedocs.worldbank.org/en/doc/5d903e848db1d1b83e0ec8f744e55570-0350012021/related/"
        "CMO-Historical-Data-Annual.xlsx")
PINK_SERIES = ["Crude oil, average", "Coal, Australian", "Iron ore, cfr spot"]
USGS_STONE = "https://pubs.usgs.gov/periodicals/mcs2023/mcs2023-stone-crushed.pdf"


def pd_economies() -> list:
    """The economies the input-output tables report."""
    with (RAW / "icio" / f"io_{YEAR}.csv").open() as f:
        return sorted({r["iso3"] for r in csv.DictReader(f)})


def icio(cache: Path, manifest: dict, iso3: set) -> None:
    """Each economy's uses summed over the origins of what it used: the row is the industry whose output was used, or
    taxes less subsidies, or value added, the column the industry that used it; and its households' consumption of
    each industry's output, likewise summed over origins, with the taxes on it."""
    path = cache / "icio_2016_2020.zip"
    if not path.exists():
        log("downloading ICIO")
        path.write_bytes(get(ICIO, timeout=1800))
    with zipfile.ZipFile(path) as z, z.open(f"{YEAR}_SML.csv") as f:
        reader = csv.reader(io.TextIOWrapper(f, encoding="utf-8-sig"))
        header = next(reader)[1:]
        uses, bought = {}, {}
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
                if what == "HFCE":
                    bought[(economy, used)] = bought.get((economy, used), 0.0) + float(v)
                elif what[0].isalpha() and what[0].isupper() and what not in ("NPISH", "GGFC", "GFCF", "INVNT",
                                                                              "DPABR"):
                    uses[(economy, used, what)] = uses.get((economy, used, what), 0.0) + float(v)
    manifest["series"]["icio/io"] = {
        "title": f"Inter-country input-output table, industry by industry, {YEAR}, millions of US dollars, each use "
                 "summed over the origins of what was used, OECD ICIO 2023 edition",
        "rows": table(RAW / "icio" / f"io_{YEAR}.csv", ["iso3", "row", "industry", "value"],
                      [(e, u, w, f"{v:.3f}") for (e, u, w), v in uses.items()]),
    }
    manifest["series"]["icio/households"] = {
        "title": f"Households' final consumption expenditure by industry, {YEAR}, millions of US dollars, summed over "
                 "origins, with taxes less subsidies on it, OECD ICIO 2023 edition",
        "rows": table(RAW / "icio" / f"households_{YEAR}.csv", ["iso3", "row", "value"],
                      [(e, u, f"{v:.3f}") for (e, u), v in bought.items()]),
    }
    manifest["sources"]["icio"] = {"title": "OECD Inter-Country Input-Output tables, 2023 edition", "url": ICIO}


def ilo(cache: Path, manifest: dict, iso3: set) -> None:
    """Employment and hours by ISIC Rev. 4 section and ISCO-08 major group, for the economies the input-output
    tables report and the years around theirs; the ILO's modelled estimates are left out, and each economy keeps
    every year and source it reports."""
    economies = set(pd_economies()) & iso3
    for flow, (name, title) in ILO_FLOWS.items():
        path = cache / f"{flow}.csv"
        if not path.exists():
            log(f"downloading {flow}")
            path.write_bytes(get(ILO.format(flow=flow), timeout=1800, accept=ILO_CSV))
        rows = []
        with path.open(encoding="utf-8-sig") as f:
            for r in csv.DictReader(f):
                if r["FREQ"] != "A" or r["REF_AREA"] not in economies or not r["OBS_VALUE"] \
                        or int(r["TIME_PERIOD"]) < YEAR - ILO_YEARS_BEFORE \
                        or not r["ECO"].startswith("ECO_ISIC4_") or not r["OCU"].startswith("OCU_ISCO08_") \
                        or "Modelled" in r.get("SOURCE", ""):
                    continue
                rows.append((r["REF_AREA"], int(r["TIME_PERIOD"]), r["ECO"][len("ECO_ISIC4_"):],
                             r["OCU"][len("OCU_ISCO08_"):], r["OBS_VALUE"], r.get("SOURCE", "")))
        manifest["series"][f"ilo/{name}_by_activity"] = {
            "title": f"{title} ({flow}), ILOSTAT",
            "rows": table(RAW / "ilo" / f"{name}_by_activity.csv",
                          ["iso3", "year", "activity", "occupation", "value", "source"], rows),
        }
    manifest["sources"]["ilo_activity"] = {"title": "ILOSTAT SDMX API", "url": ILO.format(flow="<dataflow>")}


def oecd(cache: Path, manifest: dict, iso3: set) -> None:
    for name, (flow, key, dim, keep, title) in OECD_FLOWS.items():
        path = cache / f"oecd_{name}.csv"
        if not path.exists():
            log(f"downloading OECD {flow}")
            path.write_bytes(get(OECD.format(flow=flow, key=key), timeout=1800))
        rows = []
        with path.open(encoding="utf-8-sig") as f:
            for r in csv.DictReader(f):
                if r["REF_AREA"] not in iso3 or r[dim] not in keep or not r["OBS_VALUE"] \
                        or r.get("SECTOR", "S1") not in ("S1", "_Z"):
                    continue
                rows.append((r["REF_AREA"], int(r["TIME_PERIOD"]), r["ACTIVITY"], r[dim], r["OBS_VALUE"],
                             r["UNIT_MULT"]))
        manifest["series"][f"oecd/{name}"] = {
            "title": f"{title}, OECD",
            "rows": table(RAW / "oecd" / f"{name}_by_activity.csv",
                          ["iso3", "year", "activity", "item", "value", "unit_mult"], rows),
        }
    manifest["sources"]["oecd_nad"] = {"title": "OECD Data Explorer, SDMX API",
                                        "url": OECD.format(flow="<table>", key="<key>")}


def land(_cache: Path, manifest: dict, iso3: set) -> None:
    page = json.loads(get(WB_LAND))
    rows = [(r["countryiso3code"], int(r["date"]), r["value"]) for r in page[1] or []
            if r["value"] is not None and r["countryiso3code"] in iso3]
    manifest["series"]["wb/AG.LND.AGRI.K2"] = {
        "title": "Agricultural land, square kilometres (AG.LND.AGRI.K2), World Development Indicators",
        "rows": table(RAW / "wb" / "AG.LND.AGRI.K2.csv", ["iso3", "year", "value"], rows),
    }
    manifest["sources"]["wb_land"] = {"title": "World Bank API", "url": WB_LAND}


def prices(cache: Path, manifest: dict, iso3: set) -> None:
    """Price levels, the euro's rate and the extracted products' prices per tonne."""
    rows = []
    for series in ICP_SERIES:
        page = json.loads(get(ICP.format(series=series)))
        for r in page["source"]["data"]:
            var = {v["concept"]: v["id"] for v in r["variable"]}
            if r["value"] is not None and var["Country"] in iso3:
                rows.append((var["Country"], series, f"{r['value']:.6f}"))
    manifest["series"]["icp/price_levels"] = {
        "title": "Price level index (world = 100) by expenditure heading, ICP 2021, World Bank",
        "rows": table(RAW / "icp" / "price_levels_2021.csv", ["iso3", "heading", "value"], rows),
    }
    deflator = {int(r["date"]): r["value"] for r in json.loads(get(DEFLATOR))[1]}
    import openpyxl
    path = cache / "pink.xlsx"
    if not path.exists():
        path.write_bytes(get(PINK, timeout=300))
    sheet = openpyxl.load_workbook(path, read_only=True, data_only=True)["Annual Prices (Nominal)"]
    table_rows = list(sheet.iter_rows(values_only=True))
    names = next(r for r in table_rows if r[1] == "Crude oil, average")
    units = table_rows[table_rows.index(names) + 1]
    year = next(r for r in table_rows if r[0] == YEAR)
    commodity = [(n, units[names.index(n)], f"{year[names.index(n)]:.4f}") for n in PINK_SERIES]
    import pypdf
    path = cache / "stone.pdf"
    if not path.exists():
        path.write_bytes(get(USGS_STONE, timeout=300))
    text = pypdf.PdfReader(path).pages[0].extract_text()
    line = next(l for l in text.splitlines() if l.startswith("Price, average unit value, dollars per metric ton"))
    stone = line.split()[-(UNIT_YEAR - YEAR + 1)]
    commodity.append(("Crushed stone, United States, average unit value", "($/mt)", stone))
    commodity.append((f"United States GDP deflator, {UNIT_YEAR} over {YEAR}", "(ratio)",
                      f"{deflator[UNIT_YEAR] / deflator[YEAR]:.6f}"))
    manifest["series"]["prices/commodities"] = {
        "title": f"{YEAR} annual averages: crude oil, coal and iron ore from the World Bank's Pink Sheet (nominal US$), "
                 "the crushed stone unit value from U.S. Geological Survey Mineral Commodity Summaries 2023, and the "
                 f"United States' GDP deflator (NY.GDP.DEFL.ZS) of {UNIT_YEAR} over {YEAR}",
        "rows": table(RAW / "prices" / f"commodities_{YEAR}.csv", ["name", "unit", "value"], commodity),
    }
    manifest["sources"]["prices"] = {"title": "World Bank ICP 2021 and Pink Sheet; USGS Mineral Commodity Summaries",
                                      "url": ICP.format(series="<heading>")}


SOURCES = {"icio": icio, "ilo": ilo, "oecd": oecd, "land": land, "prices": prices}


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
        source = {"icio": "icio", "ilo": "ilo_activity", "oecd": "oecd_nad", "land": "wb_land",
                  "prices": "prices"}[name]
        manifest["sources"][source]["fetched"] = datetime.date.today().isoformat()
        log(f"{name} done")
    (RAW / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
