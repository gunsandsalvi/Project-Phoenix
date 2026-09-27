#!/usr/bin/env python3
"""Fetches what the households' cash is read from: each economy's currency in circulation, from the IMF's central
bank survey, and its GDP in local currency, from the World Bank, by year.

    python3 tools/data/fetch_mon.py

Writes data/sources/raw/imf_sdmx/CIC.csv and data/sources/raw/wdi/NY.GDP.MKTP.CN.csv, and records both in the
manifest.
"""
import csv
import datetime
import io
import json

from fetch import RAW, get, log
from fetch_pop import table

FIRST, LAST = 2015, 2019
CIC_URL = ("https://api.imf.org/external/sdmx/2.1/data/IMF.STA,MFS_CBS/*.S121_L_CIC_IMB_CBS.XDC.A"
           f"?startPeriod={FIRST}&endPeriod={LAST}")
GDP_URL = ("https://api.worldbank.org/v2/country/all/indicator/NY.GDP.MKTP.CN?format=json"
           f"&date={FIRST}:{LAST}&per_page=20000")


def main() -> None:
    manifest_path = RAW / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    iso3 = {r["iso3"] for r in csv.DictReader(open(RAW / "wb" / "countries.csv"))}
    log("IMF central bank survey: currency in circulation")
    text = get(CIC_URL, timeout=600, accept="application/vnd.sdmx.data+csv;version=1.0.0").decode("utf-8-sig")
    rows = [(r["COUNTRY"], int(r["TIME_PERIOD"][:4]), r["OBS_VALUE"]) for r in csv.DictReader(io.StringIO(text))
            if r["COUNTRY"] in iso3 and r["OBS_VALUE"]]
    manifest["series"]["imf_sdmx/CIC"] = {
        "title": "Currency in circulation, national currency, end of year (S121_L_CIC_IMB_CBS), IMF Monetary and "
                 "Financial Statistics, central bank survey",
        "rows": table(RAW / "imf_sdmx" / "CIC.csv", ["iso3", "year", "value"], rows),
    }
    log("World Bank: GDP in local currency")
    page = json.loads(get(GDP_URL))
    rows = [(r["countryiso3code"], int(r["date"]), r["value"]) for r in page[1] or []
            if r["value"] is not None and r["countryiso3code"] in iso3]
    manifest["series"]["wdi/NY.GDP.MKTP.CN"] = {
        "title": "GDP, current local currency units (NY.GDP.MKTP.CN), World Bank WDI",
        "rows": table(RAW / "wdi" / "NY.GDP.MKTP.CN.csv", ["iso3", "year", "value"], rows),
    }
    manifest["sources"]["currency"] = {"title": "IMF SDMX 2.1 API; World Bank API", "url": f"{CIC_URL}; {GDP_URL}",
                                       "fetched": f"{datetime.date.today():%Y-%m-%d}"}
    manifest_path.write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
