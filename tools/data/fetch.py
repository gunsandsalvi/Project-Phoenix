#!/usr/bin/env python3
"""Fetches the published country data the country-group profiles are derived from, into data/sources/raw/.

Each source is taken whole in one download where it offers one: the World Bank's World Development Indicators, the
World Inequality Database and the BIS policy rates as bulk files, the IMF's debt series and two Our World in Data
tables by their APIs. The series the profiles read are kept as CSV (iso3, year, value), one file per series, with
the release and download date of each source in the manifest. The files are committed, so the derivation reruns
without the network and every number in the profiles traces to a published figure.

    python3 tools/data/fetch.py [--cache DIR]

The downloads are kept in the cache directory (default: a temporary directory) and reused when present.
"""
import argparse
import csv
import datetime
import io
import json
import sys
import tempfile
import time
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RAW = ROOT / "data" / "sources" / "raw"
FIRST, LAST = 2015, 2025

WDI_URL = "https://databankfiles.worldbank.org/public/ddpext_download/WDI_CSV.zip"
WID_URL = "https://wid.world/bulk_download/wid_all_data.zip"
BIS_URL = "https://data.bis.org/static/bulk/WS_CBPOL_csv_flat.zip"
IMF_URL = "https://www.imf.org/external/datamapper/api/v1/{code}"
OWID_URL = "https://ourworldindata.org/grapher/{slug}.csv?v=1&csvType=full&useColumnShortNames=true"
WB_API_URL = "https://api.worldbank.org/v2/country/all/indicator/{code}?format=json&date={first}:{last}&per_page=20000&source={source}"
SDBS_URL = ("https://sdmx.oecd.org/public/rest/data/OECD.SDD.TPS,DSD_SDBSBSC_ISIC4@DF_SDBS_ISIC4,/"
            "A..ENTR+EMPN.BTN_95XK._T+S_GE250.?startPeriod={first}&format=csvfile")

WDI = [
    "SP.DYN.LE00.IN",
    "SP.DYN.TFRT.IN",
    "SP.POP.65UP.TO.ZS",
    "SP.POP.0014.TO.ZS",
    "NY.GDP.PCAP.PP.KD",
    "SI.POV.GINI",
    "SL.EMP.TOTL.SP.ZS",
    "SL.UEM.TOTL.ZS",
    "FP.CPI.TOTL.ZG",
    "FR.INR.DPST",
    "FR.INR.LEND",
    "FB.BNK.CAPA.ZS",
    "GC.TAX.TOTL.GD.ZS",
    "NV.AGR.TOTL.ZS",
    "NV.IND.TOTL.ZS",
    "NV.SRV.TOTL.ZS",
    "NE.TRD.GNFS.ZS",
]

IMF = {
    "GGXWDG_NGDP": "General government gross debt (% of GDP), IMF World Economic Outlook",
    "HH_LS": "Household debt, loans and debt securities (% of GDP), IMF Global Debt Database",
    "NFC_LS": "Nonfinancial corporate debt, loans and debt securities (% of GDP), IMF Global Debt Database",
}

# Series read one by one from the World Bank's API: its source number and title. The Global Financial Development
# Database (source 32) is not in the WDI bulk file.
WB_API = {
    "GFDD.OI.01": (32, "Bank concentration (%): assets of the three largest commercial banks, World Bank GFDD"),
    "GFDD.OI.06": (32, "5-bank asset concentration (%), World Bank GFDD"),
    "GFDD.DI.02": (32, "Deposit money banks' assets to GDP (%), World Bank GFDD"),
    "GFDD.OI.02": (32, "Bank deposits to GDP (%), World Bank GFDD"),
    "GFDD.DI.06": (32, "Central bank assets to GDP (%), World Bank GFDD"),
    "FD.RES.LIQU.AS.ZS": (2, "Bank liquid reserves to bank assets ratio (%), World Bank WDI"),
    "NE.GDI.FTOT.ZS": (2, "Gross fixed capital formation (% of GDP), World Bank WDI"),
    "NY.GDP.MKTP.KD.ZG": (2, "GDP growth (annual %), World Bank WDI"),
}

# OECD Structural and Demographic Business Statistics: enterprises and persons employed in the business economy
# (ISIC Rev. 4 sections B to N and S95, except K), all sizes and 250 or more persons employed.
SDBS = {
    ("ENTR", "_T"): "Enterprises, business economy except financial, all sizes, OECD SDBS",
    ("EMPN", "_T"): "Persons employed, business economy except financial, all sizes, OECD SDBS",
    ("ENTR", "S_GE250"): "Enterprises of 250 or more persons employed, business economy except financial, OECD SDBS",
    ("EMPN", "S_GE250"): "Persons employed in enterprises of 250 or more, business economy except financial, OECD SDBS",
}

# WID series by (variable, percentile): the top tenth's share of net personal wealth (adults, equal split), and net
# household wealth to national income.
WID = {
    ("shwealj992", "p90p100"): ("shweal_p90p100_992_j", "Top 10% share of net personal wealth (adults, equal-split), WID"),
    ("whweali999", "p0p100"): ("whweal_p0p100_999_i", "Net household wealth to net national income ratio, WID"),
}

OWID = {
    "labor-share-of-gdp": "Labour share of GDP (%), ILO, SDG indicator 10.4.1, via Our World in Data",
    "social-spending-oecd-longrun": "Public social spending (% of GDP), OECD SOCX via Our World in Data",
}


def log(message: str) -> None:
    print(f"{datetime.datetime.now():%H:%M:%S} {message}", file=sys.stderr, flush=True)


def get(url: str, tries: int = 4, timeout: int = 60) -> bytes:
    for attempt in range(tries):
        try:
            request = urllib.request.Request(url, headers={"User-Agent": "phoenix-data-fetch/1"})
            with urllib.request.urlopen(request, timeout=timeout) as r:
                return r.read()
        except Exception as e:  # a flaky connection is retried a few times, then the fetch stops
            log(f"retry {attempt + 1} of {url}: {e}")
            time.sleep(2 ** (attempt + 1))
    raise SystemExit(f"could not fetch {url}")


def cached(cache: Path, name: str, url: str) -> Path:
    path = cache / name
    if not path.exists():
        log(f"downloading {url}")
        path.write_bytes(get(url, timeout=600))
    log(f"{name}: {path.stat().st_size} bytes")
    return path


def write(path: Path, rows: list) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["iso3", "year", "value"])
        for iso, year, value in sorted(rows):
            w.writerow([iso, year, value])
    return len(rows)


def wdi(cache: Path, manifest: dict) -> dict:
    """The WDI series and the country list, from the bulk file; returns each country's two-letter code's iso3."""
    z = zipfile.ZipFile(cached(cache, "WDI_CSV.zip", WDI_URL))
    release = max(i.date_time for i in z.infolist())
    countries = list(csv.DictReader(io.TextIOWrapper(z.open("WDICountry.csv"), encoding="utf-8-sig")))
    rows = [c for c in countries if c["Income Group"]]
    path = RAW / "wb" / "countries.csv"
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["iso3", "iso2", "name", "income_group", "region", "currency"])
        for c in sorted(rows, key=lambda c: c["Country Code"]):
            w.writerow([c["Country Code"], c["2-alpha code"], c["Table Name"], c["Income Group"], c["Region"],
                        c["Currency Unit"]])
    iso = {c["Country Code"] for c in rows}
    series = {code: [] for code in WDI}
    titles = {}
    for r in csv.DictReader(io.TextIOWrapper(z.open("WDICSV.csv"), encoding="utf-8-sig")):
        code = r["Indicator Code"]
        if code not in series or r["Country Code"] not in iso:
            continue
        titles[code] = r["Indicator Name"]
        for year in range(FIRST, LAST + 1):
            value = r.get(str(year), "")
            if value:
                series[code].append((r["Country Code"], year, value))
    for code, s in series.items():
        n = write(RAW / "wdi" / f"{code}.csv", s)
        manifest["series"][f"wdi/{code}"] = {"title": titles.get(code, code), "rows": n}
    manifest["sources"]["wdi"] = {
        "title": "World Bank, World Development Indicators, bulk CSV",
        "url": WDI_URL,
        "release": datetime.date(*release[:3]).isoformat(),
        "countries": "WDICountry.csv: every economy with an income group (FY2026 classification)",
    }
    return {c["2-alpha code"]: c["Country Code"] for c in rows if c["2-alpha code"]}


def imf(manifest: dict, iso3: set) -> None:
    for code, title in IMF.items():
        data = json.loads(get(IMF_URL.format(code=code)))
        rows = []
        for iso, years in data["values"][code].items():
            if iso not in iso3:
                continue
            for year, value in years.items():
                if FIRST <= int(year) <= LAST and value is not None:
                    rows.append((iso, int(year), value))
        manifest["series"][f"imf/{code}"] = {"title": title, "rows": write(RAW / "imf" / f"{code}.csv", rows)}
    manifest["sources"]["imf"] = {"title": "IMF DataMapper API", "url": IMF_URL.format(code="<series>")}


def wid(cache: Path, manifest: dict, iso3_of: dict) -> None:
    z = zipfile.ZipFile(cached(cache, "wid_all_data.zip", WID_URL))
    rows = {key: [] for key in WID}
    for info in z.infolist():
        name = info.filename
        if not (name.startswith("WID_data_") and name.endswith(".csv")):
            continue
        two = name[len("WID_data_"):-len(".csv")]
        if two not in iso3_of:
            continue
        for r in csv.DictReader(io.TextIOWrapper(z.open(info), encoding="utf-8"), delimiter=";"):
            key = (r["variable"], r["percentile"])
            if key in rows and FIRST <= int(r["year"]) <= LAST and r["value"]:
                rows[key].append((iso3_of[two], int(r["year"]), r["value"]))
    for key, (code, title) in WID.items():
        manifest["series"][f"wid/{code}"] = {"title": title, "rows": write(RAW / "wid" / f"{code}.csv", rows[key])}
    release = max(i.date_time for i in z.infolist())
    manifest["sources"]["wid"] = {
        "title": "World Inequality Database, bulk download",
        "url": WID_URL,
        "release": datetime.date(*release[:3]).isoformat(),
    }


def bis(cache: Path, manifest: dict, iso3_of: dict) -> None:
    """Each central bank's policy rate at the end of each year: its last observation of the year, from the monthly
    series where the BIS has one and the daily otherwise."""
    z = zipfile.ZipFile(cached(cache, "bis_cbpol.zip", BIS_URL))
    name = next(i.filename for i in z.infolist() if i.filename.endswith(".csv"))
    code = lambda text: text.split(":", 1)[0].strip()
    last = {}
    for r in csv.reader(io.TextIOWrapper(z.open(name), encoding="utf-8-sig")):
        if r[0] == "STRUCTURE":
            header = [code(h) for h in r]
            at = {h: header.index(h) for h in ("FREQ", "REF_AREA", "TIME_PERIOD", "OBS_VALUE")}
            continue
        freq, area, period, value = (code(r[at[h]]) for h in ("FREQ", "REF_AREA", "TIME_PERIOD", "OBS_VALUE"))
        if area not in iso3_of or not value or freq not in ("M", "D") or not period[:4].isdigit():
            continue
        year = int(period[:4])
        if FIRST <= year <= LAST:
            key = (iso3_of[area], year)
            rank = (freq == "M", period)
            if key not in last or rank > last[key][0]:
                last[key] = (rank, value)
    rows = [(iso, year, value) for (iso, year), (_, value) in last.items()]
    manifest["series"]["bis/CBPOL"] = {
        "title": "Central bank policy rate (%, end of year), BIS",
        "rows": write(RAW / "bis" / "CBPOL.csv", rows),
    }
    release = max(i.date_time for i in z.infolist())
    manifest["sources"]["bis"] = {
        "title": "BIS, central bank policy rates, bulk CSV",
        "url": BIS_URL,
        "release": datetime.date(*release[:3]).isoformat(),
    }


def wb_api(manifest: dict, iso3: set) -> None:
    for code, (source, title) in WB_API.items():
        url = WB_API_URL.format(code=code, first=FIRST, last=LAST, source=source)
        page = json.loads(get(url))
        rows = [(r["countryiso3code"], int(r["date"]), r["value"]) for r in page[1] or []
                if r["value"] is not None and r["countryiso3code"] in iso3]
        manifest["series"][f"wb/{code}"] = {"title": title, "rows": write(RAW / "wb" / f"{code}.csv", rows)}
    manifest["sources"]["wb"] = {"title": "World Bank API (GFDD and WDI series)", "url": WB_API_URL}


def sdbs(manifest: dict, iso3: set) -> None:
    text = get(SDBS_URL.format(first=FIRST), timeout=600).decode("utf-8-sig")
    rows = {key: [] for key in SDBS}
    for r in csv.DictReader(io.StringIO(text)):
        key = (r["MEASURE"], r["SIZE_CLASS"])
        if key in rows and r["OBS_VALUE"] and r["REF_AREA"] in iso3 and FIRST <= int(r["TIME_PERIOD"]) <= LAST:
            rows[key].append((r["REF_AREA"], int(r["TIME_PERIOD"]), r["OBS_VALUE"]))
    for (measure, size), title in SDBS.items():
        name = f"{measure}_{size.strip('_')}"
        manifest["series"][f"sdbs/{name}"] = {"title": title, "rows": write(RAW / "sdbs" / f"{name}.csv", rows[(measure, size)])}
    manifest["sources"]["sdbs"] = {"title": "OECD SDMX API, Structural and Demographic Business Statistics (ISIC Rev. 4)",
                                   "url": SDBS_URL.format(first=FIRST)}


def owid(manifest: dict, iso3: set) -> None:
    for slug, title in OWID.items():
        text = get(OWID_URL.format(slug=slug)).decode("utf-8")
        reader = csv.reader(io.StringIO(text))
        header = next(reader)
        value = next(i for i, h in enumerate(header) if h not in ("entity", "code", "year", "owid_region"))
        rows = []
        for r in reader:
            iso, year = r[header.index("code")], int(r[header.index("year")])
            if iso in iso3 and FIRST <= year <= LAST and r[value]:
                rows.append((iso, year, r[value]))
        manifest["series"][f"owid/{slug}"] = {"title": title, "rows": write(RAW / "owid" / f"{slug}.csv", rows)}
    manifest["sources"]["owid"] = {"title": "Our World in Data grapher CSV", "url": OWID_URL.format(slug="<chart>")}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cache", type=Path, default=None)
    parser.add_argument("--only", nargs="*", choices=["wb", "sdbs"],
                        help="fetch only these sources into the existing manifest, keeping the others' files")
    args = parser.parse_args()
    if args.only:
        manifest = json.loads((RAW / "manifest.json").read_text())
        iso3 = set(r["iso3"] for r in csv.DictReader((RAW / "wb" / "countries.csv").open()))
        for name in args.only:
            {"wb": wb_api, "sdbs": sdbs}[name](manifest, iso3)
            manifest["sources"][name]["fetched"] = datetime.date.today().isoformat()
            log(f"{name} done")
        (RAW / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
        return
    cache = args.cache or Path(tempfile.mkdtemp())
    cache.mkdir(parents=True, exist_ok=True)
    manifest = {"fetched": datetime.date.today().isoformat(), "years": f"{FIRST}-{LAST}", "sources": {}, "series": {}}
    for old in RAW.glob("*/*.csv"):
        old.unlink()
    iso3_of = wdi(cache, manifest)
    iso3 = set(iso3_of.values())
    log("WDI done")
    imf(manifest, iso3)
    log("IMF done")
    owid(manifest, iso3)
    log("OWID done")
    bis(cache, manifest, iso3_of)
    log("BIS done")
    wid(cache, manifest, iso3_of)
    log("WID done")
    wb_api(manifest, iso3)
    log("World Bank API done")
    sdbs(manifest, iso3)
    log("SDBS done")
    (RAW / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    for key, s in sorted(manifest["series"].items()):
        log(f"{key}: {s['rows']} rows")


if __name__ == "__main__":
    main()
