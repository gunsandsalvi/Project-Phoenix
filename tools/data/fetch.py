#!/usr/bin/env python3
"""Fetches the published country data the country-group profiles are derived from, into data/sources/raw/.

Each series is written as CSV (iso3, year, value), one file per source and series, with the date it was fetched in
the manifest beside them. The files are committed, so the derivation can be rerun without the network and every
number in the profiles traces to a published figure.
"""
import csv
import datetime
import json
import sys
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RAW = ROOT / "data" / "sources" / "raw"
YEARS = "2015:2024"

# World Bank World Development Indicators.
WDI = {
    "SP.DYN.LE00.IN": "Life expectancy at birth, total (years)",
    "SP.DYN.TFRT.IN": "Fertility rate, total (births per woman)",
    "SP.POP.65UP.TO.ZS": "Population ages 65 and above (% of total population)",
    "SP.POP.0014.TO.ZS": "Population ages 0-14 (% of total population)",
    "NY.GDP.PCAP.PP.KD": "GDP per capita, PPP (constant 2021 international $)",
    "SI.POV.GINI": "Gini index",
    "SL.EMP.TOTL.SP.ZS": "Employment to population ratio, 15+, total (%) (modeled ILO estimate)",
    "SL.UEM.TOTL.ZS": "Unemployment, total (% of total labor force) (modeled ILO estimate)",
    "FP.CPI.TOTL.ZG": "Inflation, consumer prices (annual %)",
    "FR.INR.DPST": "Deposit interest rate (%)",
    "FR.INR.LEND": "Lending interest rate (%)",
    "FB.BNK.CAPA.ZS": "Bank capital to assets ratio (%)",
    "GC.TAX.TOTL.GD.ZS": "Tax revenue (% of GDP)",
    "NV.AGR.TOTL.ZS": "Agriculture, forestry, and fishing, value added (% of GDP)",
    "NV.IND.TOTL.ZS": "Industry (including construction), value added (% of GDP)",
    "NV.SRV.TOTL.ZS": "Services, value added (% of GDP)",
    "NE.TRD.GNFS.ZS": "Trade (% of GDP)",
}

# IMF DataMapper: the World Economic Outlook and the Global Debt Database.
IMF = {
    "GGXWDG_NGDP": "General government gross debt (% of GDP), World Economic Outlook",
    "HH_LS": "Household debt, loans and debt securities (% of GDP), Global Debt Database",
    "NFC_LS": "Nonfinancial corporate debt, loans and debt securities (% of GDP), Global Debt Database",
}


def get(url: str, tries: int = 6) -> bytes:
    for attempt in range(tries):
        try:
            with urllib.request.urlopen(url, timeout=120) as r:
                return r.read()
        except Exception as e:  # the network is flaky; a series is retried whole
            wait = 2 ** (attempt + 1)
            print(f"  retry {attempt + 1} after {wait}s: {e}", file=sys.stderr)
            time.sleep(wait)
    raise SystemExit(f"could not fetch {url}")


def write(path: Path, rows: list) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["iso3", "year", "value"])
        for row in sorted(rows):
            w.writerow(row)


def countries() -> list:
    data = json.loads(get("https://api.worldbank.org/v2/country?format=json&per_page=400"))
    out = []
    for c in data[1]:
        if c["region"]["id"] == "NA":  # an aggregate, not a country
            continue
        out.append([c["id"], c["name"], c["incomeLevel"]["id"], c["region"]["value"]])
    return sorted(out)


def wdi(code: str) -> list:
    rows, page = [], 1
    while True:
        url = f"https://api.worldbank.org/v2/country/all/indicator/{code}?date={YEARS}&format=json&per_page=1000&page={page}"
        meta, data = json.loads(get(url))
        for r in data or []:
            if r["value"] is not None and r["countryiso3code"]:
                rows.append([r["countryiso3code"], int(r["date"]), r["value"]])
        if page >= meta["pages"]:
            return rows
        page += 1


def imf(code: str) -> list:
    data = json.loads(get(f"https://www.imf.org/external/datamapper/api/v1/{code}"))
    first, last = (int(y) for y in YEARS.split(":"))
    rows = []
    for iso, years in data["values"][code].items():
        for year, value in years.items():
            if first <= int(year) <= last and value is not None:
                rows.append([iso, int(year), value])
    return rows


def main() -> None:
    manifest = {"fetched": datetime.date.today().isoformat(), "years": YEARS, "series": {}}
    cs = countries()
    path = RAW / "wb" / "countries.csv"
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["iso3", "name", "income_level", "region"])
        w.writerows(cs)
    manifest["countries"] = "World Bank country list with income classification (api.worldbank.org/v2/country)"
    for code, title in WDI.items():
        print(f"WDI {code}", file=sys.stderr)
        rows = wdi(code)
        write(RAW / "wdi" / f"{code}.csv", rows)
        manifest["series"][f"wdi/{code}"] = {"title": title, "rows": len(rows)}
    for code, title in IMF.items():
        print(f"IMF {code}", file=sys.stderr)
        rows = imf(code)
        write(RAW / "imf" / f"{code}.csv", rows)
        manifest["series"][f"imf/{code}"] = {"title": title, "rows": len(rows)}
    (RAW / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
