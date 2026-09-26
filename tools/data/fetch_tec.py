#!/usr/bin/env python3
"""Fetches the published data the opening ways are derived from, into data/sources/raw/.

- Eurostat's FIGARO inter-country input-output tables at basic prices, product by product, 2022: for each reporting
  economy, what each product used of every product, summed over the countries it came from, and its value added and
  taxes less subsidies on products, so each product's output is its column's sum.
- ILOSTAT's employment and average weekly hours actually worked by economic activity (ISIC Rev. 4 sections) and
  occupation (ISCO-08 major groups), from labour force surveys.
- The OECD's national accounts: net fixed assets at current prices by activity and asset (Table 9A) and gross value
  added at current prices by activity (Table 6), in national currency, so an asset's stock per unit of value added
  needs no exchange rate.
- The World Bank's agricultural land area; the International Comparison Program's 2021 price levels by expenditure
  category (world = 100) and the euro's 2022 exchange rate, which turn each product's euros into a quantity at
  world-average prices; and its commodity prices (the Pink Sheet) and the U.S. Geological Survey's unit value of
  crushed stone, which turn the extracted products' quantities into tonnes.

    python3 tools/data/fetch_tec.py [--cache DIR] [--only NAME ...]
"""
import argparse
import csv
import datetime
import gzip
import json
import tempfile
from pathlib import Path

from fetch import RAW, get, log
from fetch_pop import countries, table

YEAR = 2022
FIGARO = ("https://ec.europa.eu/eurostat/api/dissemination/sdmx/2.1/data/naio_10_fcp_ip4/A...{dest}..?"
          f"startPeriod={YEAR}&endPeriod={YEAR}&format=SDMX-CSV&compressed=true")
# The economies FIGARO reports as destinations; the rest of the world is reported only as an origin.
FIGARO_ECONOMIES = [
    "AR", "AT", "AU", "BE", "BG", "BR", "CA", "CH", "CN", "CY", "CZ", "DE", "DK", "EE", "EL", "ES", "FI", "FR", "HR",
    "HU", "ID", "IE", "IN", "IT", "JP", "KR", "LT", "LU", "LV", "MT", "MX", "NL", "NO", "PL", "PT", "RO", "RU", "SA",
    "SE", "SI", "SK", "TR", "UK", "US", "ZA",
]
# Eurostat's codes where they differ from ISO 3166.
EUROSTAT_ISO2 = {"EL": "GR", "UK": "GB"}

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
EURO_RATE = (f"https://api.worldbank.org/v2/country/EMU/indicator/PA.NUS.FCRF?format=json&date={YEAR}:{YEAR}")
PINK = ("https://thedocs.worldbank.org/en/doc/5d903e848db1d1b83e0ec8f744e55570-0350012021/related/"
        "CMO-Historical-Data-Annual.xlsx")
PINK_SERIES = ["Crude oil, average", "Coal, Australian", "Iron ore, cfr spot"]
USGS_STONE = "https://pubs.usgs.gov/periodicals/mcs2023/mcs2023-stone-crushed.pdf"


def iso3_of() -> dict:
    return {r["iso2"]: r["iso3"] for r in csv.DictReader((RAW / "wb" / "countries.csv").open())}


def figaro(cache: Path, manifest: dict, _iso3: set) -> None:
    """Each economy's uses summed over the origins of what it used: the row is the product used or the value-added
    component, the column the product made."""
    to3 = iso3_of()
    rows = []
    for dest in FIGARO_ECONOMIES:
        path = cache / f"figaro_{dest}.csv.gz"
        if not path.exists():
            log(f"downloading FIGARO for {dest}")
            path.write_bytes(get(FIGARO.format(dest=dest), timeout=900))
        summed = {}
        with gzip.open(path, "rt", encoding="utf-8-sig") as f:
            for r in csv.DictReader(f):
                if r["unit"] != "MIO_EUR" or not r["prd_use"].startswith("CPA_") or not r["OBS_VALUE"]:
                    continue
                key = (r["prd_ava"], r["prd_use"])
                summed[key] = summed.get(key, 0.0) + float(r["OBS_VALUE"])
        iso3 = to3[EUROSTAT_ISO2.get(dest, dest)]
        rows.extend((iso3, ava, use, f"{v:.3f}") for (ava, use), v in summed.items())
    manifest["series"]["figaro/io"] = {
        "title": f"Input-output table at basic prices, product by product, {YEAR}, millions of euros, each use summed "
                 "over the origins of what was used (naio_10_fcp_ip4), Eurostat FIGARO",
        "rows": table(RAW / "figaro" / f"io_{YEAR}.csv", ["iso3", "row", "product", "value"], rows),
    }
    manifest["sources"]["figaro"] = {"title": "Eurostat FIGARO, SDMX 2.1 API", "url": FIGARO}


def ilo(cache: Path, manifest: dict, iso3: set) -> None:
    """Employment and hours by ISIC Rev. 4 section and ISCO-08 major group, for the economies the input-output
    tables report and the years around theirs; the ILO's modelled estimates are left out, and each economy keeps
    every year and source it reports."""
    to3 = iso3_of()
    economies = {to3[EUROSTAT_ISO2.get(e, e)] for e in FIGARO_ECONOMIES} & iso3
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
    rate = json.loads(get(EURO_RATE))[1][0]["value"]
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
    stone = line.split()[-1]
    commodity.append(("Crushed stone, United States, average unit value", "($/mt)", stone))
    commodity.append(("Euro, official exchange rate", "(euros per US$)", f"{rate:.6f}"))
    manifest["series"]["prices/commodities"] = {
        "title": f"{YEAR} annual averages: crude oil, coal and iron ore from the World Bank's Pink Sheet (nominal US$), "
                 "the crushed stone unit value from U.S. Geological Survey Mineral Commodity Summaries 2023, and the "
                 "euro area's official exchange rate (PA.NUS.FCRF)",
        "rows": table(RAW / "prices" / f"commodities_{YEAR}.csv", ["name", "unit", "value"], commodity),
    }
    manifest["sources"]["prices"] = {"title": "World Bank ICP 2021 and Pink Sheet; USGS Mineral Commodity Summaries",
                                      "url": ICP.format(series="<heading>")}


SOURCES = {"figaro": figaro, "ilo": ilo, "oecd": oecd, "land": land, "prices": prices}


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
        source = {"figaro": "figaro", "ilo": "ilo_activity", "oecd": "oecd_nad", "land": "wb_land",
                  "prices": "prices"}[name]
        manifest["sources"][source]["fetched"] = datetime.date.today().isoformat()
        log(f"{name} done")
    (RAW / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
