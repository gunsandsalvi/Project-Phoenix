#!/usr/bin/env python3
"""Fetches the published data the capital markets, funds, pensions, insurance, securitisation, ratings, clearing
houses and currencies are opened and measured from, into data/sources/raw/markets/:

- World Bank API: the Global Financial Development Database's market series (stock market size and turnover, listed
  companies, domestic and international debt securities, corporate bond and syndicated loan issuance and maturity,
  mutual fund, insurance and non-bank assets, premiums); WDI market capitalisation, listed companies, turnover,
  reserves and exchange rates (official and PPP); International Debt Statistics' currency composition of public
  external debt and external debt stocks.
- IMF SDMX API: COFER (reserves by currency, world), the International Investment Position by instrument and sector,
  its currency composition (IIPCC), and portfolio investment positions (CPIS): holdings by holder sector, by
  currency and, for 2019, by counterpart economy.
- OECD SDMX API: Global Pension Statistics (structure of asset-backed pensions; the main database's 2019 assets,
  allocation, flows and members by plan and vehicle), Global Insurance Statistics (indicators, asset allocation,
  premiums by class, companies and employees), institutional investors' indicators; Pensions at a Glance 2023 Table
  9.1 (participation in pension plans by type).
- FSB Global Monitoring Report on Non-Bank Financial Intermediation 2024, its monitoring dataset.
- Federal Reserve: the Gilchrist-Zakrajsek spread and excess bond premium; the Financial Accounts (Z.1), issuers of
  asset-backed securities and agency mortgage pools by collateral.
- AFME Securitisation Data Report Q1 2026: outstanding by collateral, Europe and the US, and by country.
- ECB: financial vehicle corporations' (securitisation vehicles') balance sheets by counterpart sector.
- ECB HFCS 2021 wave statistical tables: households' financial asset participation and holdings by wealth, income
  and age.
- Ilzetzki, Reinhart and Rogoff: de facto exchange-rate regime and anchor currency.
- ESMA CEREP: one-year rating transition matrices of S&P, Moody's and Fitch by issuer class.
- World Bank cross-country fiscal space database: sovereign ratings, foreign-currency and non-resident shares of
  government debt.
- New York Fed primary dealer list; typed: AFME European Primary Dealers Handbook counts, CCP12 public quantitative
  disclosure aggregates.

    python3 tools/data/fetch_markets.py [--cache DIR] [--only NAME ...]
"""
import argparse
import concurrent.futures
import csv
import datetime
import html
import io
import json
import re
import tempfile
import urllib.request
import zipfile
from pathlib import Path

import openpyxl

from fetch import RAW, get, log, cached, merge_manifest
from fetch_pop import countries, table

OUT = RAW / "markets"
FIRST, LAST = 2015, 2025
TODAY = datetime.date.today().isoformat()
SDMX_CSV = "application/vnd.sdmx.data+csv;version=1.0.0"

WB_URL = "https://api.worldbank.org/v2/country/all/indicator/{code}?format=json&date={first}:{last}&per_page=20000&source={source}"
WB = {
    32: ["GFDD.DM.01", "GFDD.DM.02", "GFDD.DM.03", "GFDD.DM.04", "GFDD.DM.05", "GFDD.DM.06", "GFDD.DM.07",
         "GFDD.DM.08", "GFDD.DM.09", "GFDD.DM.10", "GFDD.DM.11", "GFDD.DM.12", "GFDD.DM.13", "GFDD.DM.14",
         "GFDD.DM.15", "GFDD.AM.03", "GFDD.EM.01", "GFDD.OM.01", "GFDD.OM.02", "GFDD.DI.03", "GFDD.DI.07",
         "GFDD.DI.09", "GFDD.DI.10", "GFDD.DI.11"],
    2: ["CM.MKT.LCAP.GD.ZS", "CM.MKT.LCAP.CD", "CM.MKT.LDOM.NO", "CM.MKT.TRNR", "CM.MKT.TRAD.GD.ZS",
        "FI.RES.TOTL.CD", "FI.RES.TOTL.MO", "PA.NUS.FCRF", "PA.NUS.PPP", "DT.DOD.DECT.CD", "DT.DOD.DPPG.CD",
        "DT.DOD.DPNG.CD", "DT.DOD.DSTC.CD"],
}
# International Debt Statistics series carry a counterpart area, so they are read through the source's own path.
IDS_URL = ("https://api.worldbank.org/v2/sources/6/country/all/series/{code}/counterpart-area/WLD/time/all"
           "?format=json&per_page=20000")
IDS = ["DT.CUR.USDL.ZS", "DT.CUR.EURO.ZS", "DT.CUR.JYEN.ZS", "DT.CUR.UKPS.ZS", "DT.CUR.SWFR.ZS", "DT.CUR.SDRW.ZS",
       "DT.CUR.MULC.ZS", "DT.CUR.OTHC.ZS"]
WB_FOR = {"GFDD.DM": ["S3.04", "S3.05", "S5.04"], "GFDD.AM": ["S3.04"], "GFDD.EM": ["S3.05"], "GFDD.OM": ["S3.05"],
          "GFDD.DI.03": ["S3.08"], "GFDD.DI.07": ["S3.07"], "GFDD.DI.09": ["S4.03"], "GFDD.DI.10": ["S4.03"],
          "GFDD.DI.11": ["S4.03"], "CM.": ["S3.05"], "FI.": ["S5.04"], "PA.": ["S5.04"], "DT.": ["S5.04"]}

IMF_URL = "https://api.imf.org/external/sdmx/2.1/data/IMF.STA,{flow}/{key}?startPeriod={first}&endPeriod={last}"
IIP_ITEMS = ["IIP", "D", "D_F5", "D_FL", "P_MV", "P_F3_MV", "P_F5_MV", "O", "O_F4_NV", "O_F2_NV", "O_F81", "F_F7_T",
             "R", "NIIP"] + [f"P_F3_{s}_MV" for s in ("S121", "S122", "S12R", "S13", "S1Z")] \
    + [f"P_F5_{s}_MV" for s in ("S122", "S12R", "S1Z")] \
    + [f"O_F4_{s}_NV" for s in ("S121", "S122", "S12R", "S13", "S1Z")] \
    + [f"O_F2_{s}_NV" for s in ("S121", "S122", "S13")]
IIPCC_ITEMS = [f"{side}_DIC{sector}" for side in ("DLNRES", "DCNRES")
               for sector in ("", "_S121", "_S122", "_S12R", "_S13", "_S1Z", "_S1ZOTH")]
CPIS_CURRENCIES = ["USD", "EUR", "JPY", "GBP", "CNY", "CHF", "AUD", "CAD", "OTHC"]
CPIS_TOTALS = ["P_TOTINV_P_USD", "P_F51_P_USD", "P_F3_P_USD"]

# The measures kept by financing vehicle; every other measure is kept for all vehicles together.
VEHICLE_MEASURES = ("1000", "2000", "5000", "5100", "5200", "5300")
OECD_URL = "https://sdmx.oecd.org/public/rest/data/{agency},{flow},/{key}?startPeriod={first}&endPeriod={last}&format=csvfile"
PAG_T91 = "https://stat.link/64gd3b"
FSB_URL = "https://www.fsb.org/uploads/Monitoring-Dataset-2024.xlsx"
EBP_URL = "https://www.federalreserve.gov/econres/notes/feds-notes/ebp_csv.csv"
Z1_URL = "https://www.federalreserve.gov/releases/z1/current/z1_csv_files.zip"
AFME_SEC_URL = "https://www.afme.eu/media/ncxi2sq3/afme-securitisation-data-report-q1-2026.xlsx"
FVC_URL = "https://data-api.ecb.europa.eu/service/data/FVC/Q..N.F+T+S..A.1.....?startPeriod=2015&format=csvdata"
HFCS_URL = "https://www.ecb.europa.eu/home/pdf/research/hfcn/HFCS_Statistical_Tables_Wave_2021_June_2026.zip"
IRR_ERA = "https://www.ilzetzki.com/_files/ugd/b3763a_242513d0fba24aa1a64be41c8f73d887.xlsx?dn=ERA_Classification_Monthly_1940-2019.xlsx"
IRR_ANCHOR = "https://www.ilzetzki.com/_files/ugd/b3763a_7b72377cfe184f72ba0ad77dabbabae0.xlsx?dn=Anchor_monthly_1946-2019.xlsx"
CEREP_URL = "https://registers.esma.europa.eu/cerep-publication/exportCsv/Transition-Matrix"
FISCAL_SPACE = "https://thedocs.worldbank.org/en/doc/31a9f7ffd7c72b9fbed93f3fb79af70a-0050012026/original/Fiscal-space-data.xlsx"
NYFED_PD = "https://www.newyorkfed.org/markets/primarydealers"
AFME_PD = "https://www.afme.eu/media/1kbfvxb5/afmeprimarydealers202403012updated111.pdf"
CCP12_PQD = "https://ccp12.org/sites/default/files/2025-02/CCP12-PQD-Newsflash-Q4-2019-April.pdf"


def iso_maps() -> tuple:
    rows = list(csv.DictReader((RAW / "wb" / "countries.csv").open()))
    return {r["iso3"] for r in rows}, {r["iso2"]: r["iso3"] for r in rows if r["iso2"]}


def number(value) -> float | None:
    """A cell's number, or None when the cell holds no number (missing, suppressed or a bound like '< 0.1')."""
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return float(value)
    try:
        return float(str(value).strip().replace(",", ""))
    except ValueError:
        return None


def millions(value: str) -> str:
    """A US dollar amount in millions, to the nearest hundred thousand."""
    return f"{float(value) / 1e6:.1f}".rstrip("0").rstrip(".")


def series(key: str, title: str, rows: int, steps: list, note: str | None = None) -> dict:
    entry = {"title": title, "rows": rows, "for": steps}
    if note:
        entry["note"] = note
    return {f"markets/{key}": entry}


# ------------------------------------------------------------------------------------------------ World Bank API


def wb(cache: Path, iso3: set, iso2: dict) -> tuple:
    out = {}
    for source, codes in WB.items():
        for code in codes:
            page = json.loads(get(WB_URL.format(code=code, first=FIRST, last=LAST, source=source), timeout=300))
            data = page[1] if len(page) > 1 and page[1] else []
            rows = [(r["countryiso3code"], int(r["date"]), r["value"]) for r in data
                    if r["value"] is not None and r["countryiso3code"] in iso3]
            name = data[0]["indicator"]["value"] if data else code
            steps = next(v for k, v in WB_FOR.items() if code.startswith(k))
            out |= series(code, f"{name} ({code}), World Bank API source {source}, {FIRST}-{LAST}",
                          table(OUT / f"{code}.csv", ["iso3", "year", "value"], rows), steps)
            log(f"{code}: {len(rows)} rows")
    for code in IDS:
        data = json.loads(get(IDS_URL.format(code=code), timeout=300))["source"]["data"]
        rows, name = [], code
        for r in data:
            v = {x["concept"]: x for x in r["variable"]}
            year = int(v["Time"]["value"])
            name = v["Series"]["value"]
            if r["value"] is not None and v["Country"]["id"] in iso3 and FIRST <= year <= LAST:
                rows.append((v["Country"]["id"], year, r["value"]))
        out |= series(code, f"{name} ({code}), public and publicly guaranteed external debt, World Bank International "
                      f"Debt Statistics (low- and middle-income borrowers), {FIRST}-{LAST}",
                      table(OUT / f"{code}.csv", ["iso3", "year", "value"], rows), ["S5.04"])
        log(f"{code}: {len(rows)} rows")
    return {"markets/wb": {"title": "World Bank API: GFDD (source 32), WDI (source 2), International Debt Statistics "
                                    "(source 6)", "url": f"{WB_URL}; {IDS_URL}", "fetched": TODAY}}, out


# ------------------------------------------------------------------------------------------------ IMF SDMX


def imf_rows(cache: Path, name: str, flow: str, key: str, first=FIRST, last=LAST) -> list:
    path = cache / f"imf_{name}.csv"
    if not path.exists():
        log(f"downloading IMF {flow} {name}")
        path.write_bytes(get(IMF_URL.format(flow=flow, key=key, first=first, last=last), timeout=1800, accept=SDMX_CSV))
    with path.open(encoding="utf-8-sig", errors="replace") as f:
        return [r for r in csv.DictReader(f) if r["OBS_VALUE"]]


def imf(cache: Path, iso3: set, iso2: dict) -> tuple:
    out = {}
    rows = [(r["COUNTRY"], int(r["TIME_PERIOD"]), r["INDICATOR"], r["CURRENCY"], r["TYPE_OF_TRANSFORMATION"],
             r["OBS_VALUE"]) for r in imf_rows(cache, "cofer", "COFER", "all")
            if r["FREQUENCY"] == "A"]
    out |= series("imf_cofer", "Currency composition of official foreign exchange reserves, end of year, by currency: "
                  "allocated (AFXRA), unallocated (UFXRA) and total (TFXRA) reserves, in US dollars (NV_USD) or share "
                  "of allocated (SHRO_PT); area G001 is the world (per-country COFER is confidential), IMF COFER",
                  table(OUT / "imf_cofer.csv", ["area", "year", "indicator", "currency", "unit", "value"], rows),
                  ["S5.04"])
    # Sector splits are kept for the opening year only; the totals for every year.
    rows = [(r["COUNTRY"], int(r["TIME_PERIOD"]), r["BOP_ACCOUNTING_ENTRY"], r["INDICATOR"], millions(r["OBS_VALUE"]))
            for r in imf_rows(cache, "iip", "IIP", f"..{'+'.join(IIP_ITEMS)}.USD.A") if r["COUNTRY"] in iso3
            and ("_S1" not in r["INDICATOR"] or r["TIME_PERIOD"] == "2019")]
    out |= series("imf_iip", "International investment position, end of year, millions of US dollars: assets (A_P), "
                  "liabilities (L_P) and net (NETAL_P) by functional category and instrument (D direct investment, P "
                  "portfolio: "
                  "F3 debt securities, F5 equity; O other investment: F4 loans, F2 currency and deposits, F81 trade "
                  "credit; F_F7 derivatives; R reserves) and by resident sector (S121 central bank, S122 "
                  "deposit-taking "
                  "corporations, S13 general government, S12R other financial corporations, S1Z other sectors; "
                  "sector splits for 2019 only), IMF IIP (BPM6)",
                  table(OUT / "imf_iip.csv", ["iso3", "year", "entry", "indicator", "value"], rows), ["S5.04"])
    rows = [(r["COUNTRY"], int(r["TIME_PERIOD"]), r["BOP_ACCOUNTING_ENTRY"], r["INDICATOR"], r["CURRENCY"],
             millions(r["OBS_VALUE"]))
            for r in imf_rows(cache, "iipcc", "IIPCC", f"..{'+'.join(IIPCC_ITEMS)}..USD.A") if r["COUNTRY"] in iso3]
    out |= series("imf_iipcc", "Currency composition of the international investment position, end of year, millions "
                  "of US dollars: debt liabilities to (DLNRES) and claims on (DCNRES) non-residents by resident sector "
                  "suffix (none total, S121 central bank, S122 deposit-taking corporations, S12R other financial "
                  "corporations, S13 general government, S1Z other sectors, S1ZOTH other sectors excluding "
                  "corporations) and currency (XDC domestic, FC foreign, USD, EUR, JPY, OTHC, UALLC unallocated), "
                  "IMF IIPCC; about 20 reporting economies",
                  table(OUT / "imf_iipcc.csv", ["iso3", "year", "entry", "indicator", "currency", "value"], rows),
                  ["S5.04"])
    totals = "+".join(CPIS_TOTALS)
    rows = [(r["COUNTRY"], int(r["TIME_PERIOD"]), r["ACCOUNTING_ENTRY"], r["SECTOR"], r["INDICATOR"],
             millions(r["OBS_VALUE"]))
            for r in imf_rows(cache, "cpis_holders", "PIP", f".A+L.{totals}..S1.G001.A")
            if r["COUNTRY"] in iso3 and r["FREQUENCY"] == "A"]
    out |= series("cpis_holders", "Portfolio investment positions with the world, end of year, millions of US dollars: "
                  "assets (A) by resident holder sector (S1 total, S121 central bank, S122 deposit-taking "
                  "corporations, S123 "
                  "money market funds, S12P insurance and pension funds, S12QU other financial, S12R other financial "
                  "corporations, S13 government, S11 non-financial corporations, S14 households, S15 non-profits, "
                  "S1V other than deposit-taking and government) and derived liabilities (L), total (P_TOTINV), "
                  "equity and fund shares (P_F51) and debt securities (P_F3), IMF CPIS (PIP)",
                  table(OUT / "cpis_holders.csv", ["iso3", "year", "entry", "sector", "indicator", "value"], rows),
                  ["S5.04", "S3.07"])
    currency = "+".join(f"P_{i}_DIC_{c}_P_USD" for i in ("F3", "F51") for c in CPIS_CURRENCIES)
    rows = [(r["COUNTRY"], int(r["TIME_PERIOD"]), r["INDICATOR"], millions(r["OBS_VALUE"]))
            for r in imf_rows(cache, "cpis_currency", "PIP", f".A.{currency}.S1.S1.G001.A")
            if r["COUNTRY"] in iso3 and r["FREQUENCY"] == "A"]
    out |= series("cpis_currency", "Portfolio investment assets with the world by currency of denomination, all "
                  "resident sectors, end of year, millions of US dollars: debt securities (P_F3_DIC_<cur>) and "
                  "equity (P_F51_DIC_<cur>), IMF CPIS (PIP)",
                  table(OUT / "cpis_currency.csv", ["iso3", "year", "indicator", "value"], rows), ["S5.04"])
    rows = [(r["COUNTRY"], r["COUNTERPART_COUNTRY"], r["INDICATOR"], millions(r["OBS_VALUE"]))
            for r in imf_rows(cache, "cpis_bilateral", "PIP", f".A.{totals}.S1.S1..A", 2019, 2019)
            if r["COUNTRY"] in iso3 and r["FREQUENCY"] == "A"
            and (r["COUNTERPART_COUNTRY"] in iso3 or r["COUNTERPART_COUNTRY"] == "G001")]
    out |= series("cpis_bilateral_2019", "Portfolio investment assets by counterpart economy (G001 world), all "
                  "resident sectors, end 2019, millions of US dollars: total, equity (P_F51) and debt securities "
                  "(P_F3), "
                  "IMF CPIS (PIP)",
                  table(OUT / "cpis_bilateral_2019.csv", ["iso3", "counterpart", "indicator", "value"], rows),
                  ["S5.04", "S5.05"])
    return {"markets/imf": {"title": "IMF SDMX 2.1 API: COFER, IIP, IIPCC, PIP (CPIS)", "url": IMF_URL,
                            "fetched": TODAY}}, out


# ------------------------------------------------------------------------------------------------ OECD


def oecd_rows(cache: Path, name: str, agency: str, flow: str, key: str, first=FIRST, last=LAST) -> list:
    path = cache / f"oecd_{name}.csv"
    if not path.exists():
        log(f"downloading OECD {flow}")
        path.write_bytes(get(OECD_URL.format(agency=agency, flow=flow, key=key, first=first, last=last), timeout=1800))
    with path.open(encoding="utf-8-sig") as f:
        return [r for r in csv.DictReader(f) if r["OBS_VALUE"]]


def oecd(cache: Path, iso3: set, iso2: dict) -> tuple:
    out = {}
    cm = "OECD.DAF.CM"
    rows = [(r["REF_AREA"], int(r["TIME_PERIOD"]), r["MEASURE"], r["UNIT_MEASURE"], r["PLAN_TYPE"],
             r["DEFINITION_TYPE"], r["VEHICLE_TYPE"], r["OBS_VALUE"])
            for r in oecd_rows(cache, "pension_structure", cm, "DSD_FP@DF_SPS", "all") if r["REF_AREA"] in iso3]
    out |= series("pension_structure", "Structure of asset-backed pensions: assets by plan type (OCC occupational, PER "
                  "personal), definition (DB defined benefit, DBH hybrid, DCP protected and DCU unprotected defined "
                  "contribution) and financing vehicle (PF autonomous pension fund, BR book reserve, PIC pension "
                  "insurance contract, INV/BANK managed funds, OTH), as % of total pension assets (PT_AS), of "
                  "occupational assets (PT_AS_PENS_OCC) or of GDP (PT_B1GQ); measures DBA/DCA/OA/PA/AFV, OECD Global "
                  "Pension Statistics (DSD_FP@DF_SPS)",
                  table(OUT / "pension_structure.csv",
                        ["iso3", "year", "measure", "unit", "plan", "definition", "vehicle", "value"], rows),
                  ["S4.04", "S3.07"])
    rows = [(r["REF_AREA"], r["MEASURE"], r["UNIT_MEASURE"], r["PLAN_TYPE"], r["DEFINITION_TYPE"], r["VEHICLE_TYPE"],
             r["UNIT_MULT"], r["OBS_VALUE"])
            for r in oecd_rows(cache, "pension_main_2019", cm, "DSD_FP@DF_FPS",
                               ".A..USD+PT_B1GQ+PS._T+OCC+PER..", 2019, 2019)
            if r["REF_AREA"] in iso3 and (r["VEHICLE_TYPE"] == "_T" or r["MEASURE"] in VEHICLE_MEASURES)]
    out |= series("pension_main_2019", "Asset-backed pensions, 2019: investment (1000) and its allocation (1110-1290: "
                  "cash and deposits, bills and bonds public and private, loans, equity listed and unlisted, land and "
                  "buildings, mutual funds by kind, unallocated insurance contracts, hedge and private equity funds, "
                  "structured products, other; 1121/1122 abroad and in foreign currency), liabilities (2000-2300), "
                  "income and contributions (3000-3400), benefits (4000-4500) and members (5000 total, 5100 active, "
                  "5200 deferred, 5300 passive, 5400 other beneficiaries, 5010/5020 by sex), in US dollars (unit_mult "
                  "6: millions), % of GDP or persons (PS, unit_mult 3: thousands), by plan type and definition, and "
                  "by vehicle for investment, liabilities and members, OECD Global Pension Statistics (DSD_FP@DF_FPS)",
                  table(OUT / "pension_main_2019.csv",
                        ["iso3", "measure", "unit", "plan", "definition", "vehicle", "unit_mult", "value"], rows),
                  ["S4.04"])
    rows = [(r["REF_AREA"], int(r["TIME_PERIOD"]), r["MEASURE"], r["UNIT_MEASURE"], r["PREMIUMS"],
             r["INSURANCE_TYPE"], r["UNIT_MULT"], r["OBS_VALUE"])
            for r in oecd_rows(cache, "ins_ind", cm, "DSD_INS@DF_IND", "all")
            if r["REF_AREA"] in iso3 and r["MEASURE"] not in ("MKT", "TGP_EMP")]
    out |= series("insurance_indicators", "Insurance indicators by year and type (LIFE, NLIFE, _T): penetration "
                  "(PEN, % of GDP), density (DST, US dollars per person), gross premiums (TGP, US dollars, unit_mult "
                  "6), "
                  "retention ratio (RET), reinsurance share (REINS), life/non-life shares (LIS/NLIS), shares of "
                  "domestic premiums by foreign-controlled undertakings and branches (FN_BRA/FNC/BRA), OECD Global "
                  "Insurance Statistics (DSD_INS@DF_IND)",
                  table(OUT / "insurance_indicators.csv",
                        ["iso3", "year", "measure", "unit", "premiums", "insurance_type", "unit_mult", "value"], rows),
                  ["S4.03"])
    rows = [(r["REF_AREA"], int(r["TIME_PERIOD"]), r["MEASURE"], r["INSURANCE_TYPE"], r["INSURER_TYPE"], r["OBS_VALUE"])
            for r in oecd_rows(cache, "ins_alloc", cm, "DSD_INS@DF_ASSET_ALLOC", "all")
            if r["REF_AREA"] in iso3 and r["UNIT_MEASURE"] == "USD" and r["OWNERSHIP"] == "UND_T"
            and 2018 <= int(r["TIME_PERIOD"]) <= 2020
            and r["DESTINATION"] == "_T" and r["INSURANCE_TYPE"] in ("_T", "LIFE", "NLIFE", "COMP")]
    out |= series("insurance_assets", "Insurers' investment by asset (INV total; CASH, BOND with PUBB public and PRIB "
                  "private, LOAN, SHARE with PUBS listed and UNLISTED, REST land and buildings, CIS mutual funds by "
                  "kind, PEQUF, HEDGE, STRPRDT, UNITLINK, INV_O other), all undertakings, domestic and foreign "
                  "destination, by type (_T, LIFE, NLIFE, COMP) and insurer (DIR direct, REI reinsurer), millions of "
                  "US dollars, 2018-2020 around the opening year, OECD Global Insurance Statistics "
                  "(DSD_INS@DF_ASSET_ALLOC)",
                  table(OUT / "insurance_assets.csv",
                        ["iso3", "year", "measure", "insurance_type", "insurer", "value"], rows), ["S4.03"])
    rows = [(r["REF_AREA"], int(r["TIME_PERIOD"]), r["INSURANCE_CLASS"], r["INSURANCE_BUSINESS"], r["OBS_VALUE"])
            for r in oecd_rows(cache, "ins_classes", cm, "DSD_INS@DF_CLASSES", "all")
            if r["REF_AREA"] in iso3 and r["UNIT_MEASURE"] == "USD" and r["MEASURE"] == "GRS"]
    out |= series("insurance_classes", "Gross written premiums by class of insurance (LI life: ANN annuities, UL unit "
                  "linked, OL other, PC pension contracts; NLI non-life: MV motor, MAOT marine-aviation-transport, FRT "
                  "freight, FOPD fire and property, PL pecuniary loss, GL general liability, AH accident and health, "
                  "HLTH health, ONL other; TREINS treaty reinsurance) and business (_T, DINS direct, RINS reinsurance "
                  "accepted), millions of US dollars, OECD Global Insurance Statistics (DSD_INS@DF_CLASSES)",
                  table(OUT / "insurance_classes.csv", ["iso3", "year", "class", "business", "value"], rows),
                  ["S4.03"])
    rows = [(r["REF_AREA"], int(r["TIME_PERIOD"]), r["MEASURE"], r["OWNERSHIP"], r["INSURANCE_TYPE"],
             r["EMPLOYER_TYPE"], r["OBS_VALUE"])
            for r in oecd_rows(cache, "ins_companies", cm, "DSD_INS@DF_NB_COMP", "all") if r["REF_AREA"] in iso3]
    out |= series("insurance_companies", "Insurance undertakings (UND, number) by ownership (UND_T all, DOM "
                  "domestic, FOR foreign-controlled, BRN branches) and type (_T, LIFE, NLIFE, COMP composite, "
                  "REINS), and insurance "
                  "employees (EMP, persons) by employer (INS insurers, INT intermediaries), OECD Global Insurance "
                  "Statistics (DSD_INS@DF_NB_COMP)",
                  table(OUT / "insurance_companies.csv",
                        ["iso3", "year", "measure", "ownership", "insurance_type", "employer", "value"], rows),
                  ["S4.03"])
    rows = [(r["REF_AREA"], int(r["TIME_PERIOD"]), r["MEASURE"], r["UNIT_MEASURE"], r["OBS_VALUE"])
            for r in oecd_rows(cache, "inst_investors", "OECD.SDD.NAD", "DSD_FIN_DASH@DF_7II_INDIC", "all")
            if r["REF_AREA"] in iso3 and r["FREQ"] == "A" and r["UNIT_MEASURE"] in ("PT_B1GQ", "PT_FAS", "PT_FAS_S12L")]
    out |= series("institutional_investors", "Institutional investors' indicators, annual: financial assets and "
                  "liabilities of pension funds (S129), insurers (S128) and investment funds (S12L: MMF S123, non-MMF "
                  "S124, open-end S12LO, closed-end S124B, equity S12L2, bond S12L3, mixed S12L4, hedge S12L5, real "
                  "estate S12L1, other S12L9) as % of GDP, instrument shares of their assets (PT_FAS: F2 deposits, F3 "
                  "debt securities, F4 loans, F5 equity and fund shares, F6 insurance and pension, F7 derivatives, F8 "
                  "other) and fund types' shares of all funds' assets (PT_FAS_S12L), OECD (DSD_FIN_DASH@DF_7II_INDIC)",
                  table(OUT / "institutional_investors.csv", ["iso3", "year", "measure", "unit", "value"], rows),
                  ["S3.07", "S4.03", "S4.04"])
    wbk = openpyxl.load_workbook(cached(cache, "pag2023_t91.xlsx", PAG_T91), read_only=True, data_only=True)
    sheet = list(wbk["t9-1"].iter_rows(values_only=True))
    years = [int(y) for y in sheet[4] if str(y).isdigit()]
    names = {r["name"]: r["iso3"] for r in csv.DictReader((RAW / "wb" / "countries.csv").open())}
    names |= {"Czechia": "CZE", "Korea": "KOR", "Slovak Republic": "SVK", "Türkiye": "TUR", "United States": "USA",
              "Hong Kong (China)": "HKG", "Russian Federation": "RUS"}
    rows = []
    for r in sheet[5:]:
        if not r[0] or r[0] not in names:
            continue
        for year, value in zip(years, r[3:3 + len(years)]):
            if FIRST <= year <= LAST and number(value) is not None:
                rows.append((names[r[0]], year, r[1], value))
    out |= series("pension_participation", "Participation rate in pension plans by type (mandatory or quasi-mandatory, "
                  "auto-enrolment, voluntary occupational, voluntary personal, voluntary total), % of the working-age "
                  "population 15-64, OECD Pensions at a Glance 2023, Table 9.1 (StatLink https://stat.link/64gd3b)",
                  table(OUT / "pension_participation.csv", ["iso3", "year", "plan", "value"], rows), ["S4.04"],
                  "Private pension coverage by age is not published in a downloadable table (Pensions at a Glance "
                  "2023 has participation by plan type only); coverage is by type of plan, working-age population.")
    return {"markets/oecd": {"title": "OECD SDMX API (Global Pension Statistics, Global Insurance Statistics, "
                                      "institutional investors) and Pensions at a Glance 2023 StatLink",
                             "url": OECD_URL, "fetched": TODAY}}, out


# ------------------------------------------------------------------------------------------------ FSB


def fsb(cache: Path, iso3: set, iso2: dict) -> tuple:
    wbk = openpyxl.load_workbook(cached(cache, "fsb_monitoring_2024.xlsx", FSB_URL), read_only=True, data_only=True)
    code = lambda c: iso2.get("GB" if c == "UK" else c, c)
    rows = [(code(r[2]), r[0], r[3], r[4], r[5], r[8], r[9])
            for r in list(wbk["Monitoring dataset"].iter_rows(values_only=True))[1:] if r[0] and r[0] >= FIRST]
    out = series("fsb_nbfi", "Financial assets by sector (banks, central bank, public financial institutions, "
                 "insurance corporations, pension funds, other financial intermediaries OFIs, financial auxiliaries, "
                 "NBFI, total) "
                 "and the narrow measure of NBFI by economic function (EF1 collective investment vehicles with run "
                 "features, EF2 lending dependent on short-term funding, EF3 market intermediation dependent on "
                 "short-term funding, EF4 facilitation of credit creation, EF5 securitisation-based credit), USD "
                 "trillions, % of GDP and % of the topic's total, 29 jurisdictions (iso3; EA euro area, AEs, EMEs, "
                 "FSB, G21, G29 aggregates), FSB Global Monitoring Report on NBFI 2024, monitoring dataset",
                 table(OUT / "fsb_nbfi.csv", ["area", "year", "topic", "entity", "usd_tn", "pct_gdp", "pct_topic"],
                       rows), ["S3.08", "S3.07"])
    rows = [(r[2], r[0], r[1], r[3]) for r in list(wbk["OFIs breakdown"].iter_rows(values_only=True))[1:]
            if r[0] and r[0] >= FIRST]
    out |= series("fsb_ofi", "Other financial intermediaries by entity type (MMFs, hedge funds HFs, other investment "
                  "funds OIFs, REITs, trust companies TCs, finance companies FinCos, broker-dealers BDs, structured "
                  "finance "
                  "vehicles SFVs, CCPs, captive financial institutions and money lenders CFIMLs, others), USD "
                  "trillions, group aggregate, FSB Global Monitoring Report on NBFI 2024",
                  table(OUT / "fsb_ofi.csv", ["entity", "year", "area", "usd_tn"], rows), ["S3.08", "S3.07"])
    return {"markets/fsb": {"title": "FSB Global Monitoring Report on Non-Bank Financial Intermediation 2024, "
                                     "monitoring dataset (data to end-2023)", "url": FSB_URL, "fetched": TODAY,
                            "release": "2024-12-16"}}, out


# ------------------------------------------------------------------------------------------------ Federal Reserve


def gz(cache: Path, iso3: set, iso2: dict) -> tuple:
    text = cached(cache, "ebp.csv", EBP_URL).read_text()
    rows = []
    for r in csv.DictReader(io.StringIO(text)):
        month, _, year = r["date"].split("/")
        rows.append((f"{year}-{int(month):02d}", r["gz_spread"], r["ebp"], r["est_prob"]))
    out = series("gz_ebp", "Gilchrist-Zakrajsek credit spread and excess bond premium, percentage points, and the "
                 "estimated probability of a recession in the next 12 months, monthly 1973 on, US, Federal Reserve "
                 "Board (FEDS Notes update)",
                 table(OUT / "gz_ebp.csv", ["month", "gz_spread", "ebp", "est_prob"], rows), ["S7.01", "S3.04"],
                 "Kept whole from 1973: the realism reads relate it to the cycle over long spans.")
    return {"markets/gz": {"title": "Federal Reserve Board, updated Gilchrist-Zakrajsek excess bond premium",
                           "url": EBP_URL, "fetched": TODAY}}, out


# ------------------------------------------------------------------------------------------------ Securitisation

AFME_AREAS = {"Belgium": "BEL", "France": "FRA", "Germany": "DEU", "Greece": "GRC", "Ireland": "IRL", "Italy": "ITA",
              "Netherlands": "NLD", "Portugal": "PRT", "Spain": "ESP", "Switzerland": "CHE", "UK": "GBR"}


def afme_period(label) -> tuple | None:
    """A column header's (year, quarter, whether the quarter is printed): 'YYYY:Qn', or a bare year read as its
    fourth quarter."""
    if isinstance(label, int) and 1900 < label < 2100:
        return label, 4, False
    m = re.match(r"(\d{4}):\s*Q([1-4])", str(label or ""))
    return (int(m.group(1)), int(m.group(2)), True) if m else None


def afme_table(sheet: list, title: str) -> list:
    """The rows of one titled table, over all its header blocks: (label, year, value) at each year's fourth quarter,
    a printed quarter preferred to a bare year (they come from different data sources)."""
    start = next(i for i, r in enumerate(sheet) if r[0] and str(r[0]).strip().startswith(title))
    found, periods = {}, {}
    for r in sheet[start + 1:]:
        if r[0] and re.match(r"(\d+\.\d+ |Source| \*)", str(r[0])):
            break
        heads = {j: afme_period(c) for j, c in enumerate(r) if j and afme_period(c)}
        if heads:
            periods = heads
            continue
        if not r[0] or not periods:
            continue
        for j, (year, quarter, printed) in periods.items():
            if quarter == 4 and FIRST <= year <= LAST and number(r[j]) is not None:
                key = (str(r[0]).strip(), year)
                if key not in found or printed:
                    found[key] = round(number(r[j]), 3)
    return [(label, year, v) for (label, year), v in found.items()]


def sec(cache: Path, iso3: set, iso2: dict) -> tuple:
    out = {}
    with zipfile.ZipFile(cached(cache, "z1_csv_files.zip", Z1_URL)) as z:
        rows = []
        for tab in ("S125s1_3_s", "S125s1_2_s"):
            names = {}
            for line in z.read(f"data_dictionary/{tab}.txt").decode("utf-8", "replace").splitlines():
                parts = line.split("\t")
                if len(parts) > 1:
                    names[parts[0]] = parts[1].strip()
            for r in csv.DictReader(io.StringIO(z.read(f"csv/{tab}.csv").decode("utf-8"))):
                year, quarter = r["date"].split(":")
                if quarter == "Q4" and FIRST <= int(year) <= LAST:
                    rows += [(code, names.get(code, code), int(year), v) for code, v in r.items()
                             if code != "date" and number(v) is not None]
    out |= series("us_abs_issuers", "Issuers of asset-backed securities (S125s1.3) and agency- and GSE-backed mortgage "
                  "pools (S125s1.2), levels at the end of each year by asset (consumer credit, one-to-four-family, "
                  "multifamily and commercial mortgages, home equity, trade receivables, private credit and other "
                  "loans, consumer leases, agency securities, Treasuries) and liability (debt securities, commercial "
                  "paper, bonds, repo), millions of US dollars, Federal Reserve Financial Accounts of the US (Z.1)",
                  table(OUT / "us_abs_issuers.csv", ["series", "description", "year", "value"], rows), ["S4.05"],
                  "The agency pools sector holds only the pools off the GSEs' balance sheets (Fannie Mae and Freddie "
                  "Mac pools are on them since 2010); agency MBS outstanding in total is in "
                  "markets/afme_securitisation.")
    wbk = openpyxl.load_workbook(cached(cache, "afme_securitisation_q1_2026.xlsx", AFME_SEC_URL), read_only=True,
                                 data_only=True)
    tab3 = list(wbk["3"].iter_rows(values_only=True))
    rows = [("Europe", "Total", c, y, v) for c, y, v in afme_table(tab3, "3.1 Total European Outstandings")]
    rows += [("US", "Total", c, y, v) for c, y, v in afme_table(tab3, "3.3 Total US Outstandings")]
    tab4 = list(wbk["4"].iter_rows(values_only=True))
    rows += [("US" if c.startswith("US") else "Europe", AFME_AREAS.get(c, c), "Total ex-CLO", y, v)
             for c, y, v in afme_table(tab4, "3.4 Total European Outstandings")]
    tab5 = list(wbk["5"].iter_rows(values_only=True))
    for i, r in enumerate(tab5):
        p = afme_period(r[0])
        if p and p[1] == 4 and p[2] and FIRST <= p[0] <= LAST:
            collateral = {j: str(c).strip() for j, c in enumerate(r) if j and c}
            for row in tab5[i + 1:]:
                if not row[0]:
                    break
                area = str(row[0]).strip()
                rows += [("Europe", AFME_AREAS.get(area, area), c, p[0], round(number(row[j]), 3))
                         for j, c in collateral.items() if number(row[j]) is not None]
    out |= series("afme_securitisation", "Securitisation outstanding at the end of each year by collateral (Europe: "
                  "auto, cards, SME ABS, CMBS, consumer, leases, RMBS, other, CLO/CDO; US: ABS, agency MBS, non-agency "
                  "RMBS and CMBS) and, for Europe, by country of collateral (iso3, or Pan Europe, Other Europe, "
                  "Eurozone, EU Total, European Total), billions (the sheets state euro billions; the US table is "
                  "sourced from SIFMA), AFME Securitisation Data Report Q1 2026, tables 3.1, 3.3, 3.4 and 3.5",
                  table(OUT / "afme_securitisation.csv", ["region", "area", "collateral", "year", "value"], rows),
                  ["S4.05"],
                  "SIFMA's own US statistics are behind a registration form; the US outstanding by collateral is "
                  "taken from AFME's table 3.3 (SIFMA-sourced) and the Fed's Z.1. European CLO/CDO outstanding is "
                  "unavailable from 2019Q4 to 2022Q1 per AFME's footnote; AFME changed its European source in 2020 "
                  "and the US non-agency CMBS series jumps in 2022 (a change of coverage, not of market). Year-end "
                  "values: the fourth quarter where printed, else the bare year column.")
    fvc = cached(cache, "ecb_fvc.csv", FVC_URL)
    rows = []
    with fvc.open(encoding="utf-8-sig") as f:
        for r in csv.DictReader(f):
            if not r["TIME_PERIOD"].endswith("Q4") or not r["OBS_VALUE"]:
                continue
            area = "EA" if r["REF_AREA"] == "U2" else iso2.get(r["REF_AREA"], r["REF_AREA"])
            rows.append((area, int(r["TIME_PERIOD"][:4]), r["FVC_REP_SECTOR"], r["FVC_ITEM"], r["COUNT_AREA"],
                         r["BS_COUNT_SECTOR"], r["FVC_ORI_SECTOR"], r["OBS_VALUE"]))
    out |= series("ecb_fvc", "Financial vehicle corporations (securitisation vehicles), euro area countries and the "
                  "euro area (EA), outstanding at the end of each year, millions of euros: reporting sector (F all, T "
                  "traditional, S synthetic securitisation), item (A10 deposits and loan claims, A20 securitised "
                  "loans, A30 debt securities held, A40 other securitised assets, A50 equity, AT1 other assets, L20 "
                  "loans received, L40 debt securities issued, L60 capital, LT1 other liabilities, T00 total), "
                  "counterpart area (A1 world, U2 euro area, U4 extra-euro area, U5 other euro members, U6 domestic), "
                  "counterpart (borrower) sector (0000 all, 1000 MFIs, 2000 non-MFIs, 2100 government, 2200 non-MFIs "
                  "excluding government, 2210 OFIs, 2220 insurers and pension funds, 2240 non-financial corporations, "
                  "2250 households and NPISH, 2260 non-MMF funds, 2270 OFIs incl. auxiliaries and captives, 2271 FVCs) "
                  "and originator (00 all, E0 euro area, E1 euro area MFIs, E2 government, E3 OFIs/funds/ICPFs, E4 "
                  "NFCs, R0 non-euro area, ZZ not applicable), ECB FVC statistics",
                  table(OUT / "ecb_fvc.csv", ["area", "year", "fvc_sector", "item", "counterpart_area",
                                              "counterpart_sector", "originator", "value"], rows), ["S4.05"])
    return {"markets/securitisation": {"title": "Federal Reserve Z.1 CSV release; AFME Securitisation Data Report Q1 "
                                                "2026 (xlsx); ECB Data Portal FVC dataset",
                                       "url": f"{Z1_URL}; {AFME_SEC_URL}; {FVC_URL}", "fetched": TODAY}}, out


# ------------------------------------------------------------------------------------------------ ECB HFCS


def hfcs(cache: Path, iso3: set, iso2: dict) -> tuple:
    with zipfile.ZipFile(cached(cache, "hfcs2021.zip", HFCS_URL)) as z:
        name = next(n for n in z.namelist() if n.endswith(".xlsx"))
        (cache / name).write_bytes(z.read(name))
    wbk = openpyxl.load_workbook(cache / name, read_only=True, data_only=True)
    rows = []
    for ws in wbk.worksheets:
        if not re.match(r"[CD]\d ", ws.title):
            continue
        sheet = [list(r) for r in ws.iter_rows(values_only=True)]
        table_id = ws.title.split(" ")[0]
        at = next(i for i, r in enumerate(sheet) if r[0] and str(r[0]).startswith("Table"))
        unit = str(sheet[at + 1][0] or "")
        columns, group, last = {}, "", None
        for r in sheet:
            cells = [c for c in r if c is not None]
            if not cells:
                continue
            if "euro area" in [str(c).strip() for c in r]:
                columns = {j: ("EA" if str(c).strip() == "euro area" else iso2.get(str(c).strip(), str(c).strip()))
                           for j, c in enumerate(r) if c and str(c).strip()}
                start = min(columns)
                continue
            if not columns or str(cells[0]).startswith(("Source", "M = ", "See country")):
                if columns and str(cells[0]).startswith("Source"):
                    columns = {}
                continue
            if r[0]:
                group = str(r[0]).strip()
            label = " ".join(str(c).strip() for c in r[1:start] if c is not None)
            first_value = next((r[j] for j in columns if r[j] is not None), None)
            if first_value is not None and str(first_value).startswith("("):
                if last:
                    for j, area in columns.items():
                        se = number(str(r[j]).strip("()")) if r[j] is not None else None
                        key = (table_id, last[0], last[1], area)
                        if key in last[2] and se is not None:
                            last[2][key][-1] = se
                continue
            found = {}
            for j, area in columns.items():
                v = number(r[j]) if r[j] is not None else None
                if v is not None:
                    found[(table_id, group, label, area)] = [table_id, unit, group, label, area, v, None]
            rows += found.values()
            last = (group, label, found)
    out = series("hfcs_2021", "Household Finance and Consumption Survey, wave 2021 (reference 2020-2021), statistical "
                 "tables C (financial assets: participation by asset type, conditional values, distributions, "
                 "participation in publicly traded shares by income and net wealth quantile, household size, "
                 "housing status and age of the reference person) and D (portfolio shares of asset types, financial "
                 "over total assets by the same breakdowns): euro area (EA) and each country, value and standard "
                 "error, ECB (June 2026 release, version 4.1)",
                 table(OUT / "hfcs_2021.csv", ["table", "unit", "breakdown", "category", "area", "value", "se"],
                       [tuple(r) for r in rows]), ["S3.05", "S3.07", "S6.03"],
                 "Cells marked M (missing), N (too few observations) or given as a bound ('< 0.1') are left out.")
    return {"markets/hfcs": {"title": "ECB, HFCS statistical tables, wave 2021 (June 2026)", "url": HFCS_URL,
                             "fetched": TODAY}}, out


# ------------------------------------------------------------------------------------------------ IRR regimes

IRR_NAMES = {"Azerbaijan Rep. of": "AZE", "The Bahamas": "BHS", "Bahrain Kingdom of": "BHR",
             "Bosnia & Herzegovina": "BIH", "Central African Rep.": "CAF", "China, PR": "CHN",
             "Congo Dem. Rep. of": "COD", "Congo Rep. of": "COG", 'Cote D"Ivoire': "CIV", "Curacao": "CUW",
             "Czech Rep.": "CZE", "The Gambia": "GMB", "Guinea Bissau": "GNB", "Korea": "KOR", "Kyrgyz Rep.": "KGZ",
             "Lao Dem. Rep.": "LAO", "Liechtesntein": "LIE", "Macedonia FYR": "MKD", "Micronesia": "FSM",
             "Montenegro": "MNE", "Netherlands Antilles": "ANT", "PNG": "PNG", "Sao Tome & Principe": "STP",
             "Serbia, Rep. of": "SRB", "St Vincent & Grenadines": "VCT", "Syrian Arab Rep.": "SYR",
             "Trinidad Tobago": "TTO", "UAE": "ARE", "United States": "USA", "West Bank and Gaza": "PSE",
             "Yemen Rep. of": "YEM"}
# The authors' Table 1 groups the fifteen fine codes into six coarse ones.
IRR_COARSE = {1: 1, 2: 1, 3: 1, 4: 1, 5: 2, 6: 2, 7: 2, 8: 2, 9: 3, 10: 3, 11: 3, 12: 3, 13: 4, 14: 5, 15: 6}


def irr(cache: Path, iso3: set, iso2: dict) -> tuple:
    anchor = list(openpyxl.load_workbook(cached(cache, "irr_anchor.xlsx", IRR_ANCHOR), read_only=True,
                                         data_only=True)["Master"].iter_rows(values_only=True))
    code_of = {str(n).strip(): c for c, n in zip(anchor[5], anchor[6]) if c and n}
    anchors = {}
    for r in anchor:
        m = re.match(r"(\d{4})M12$", str(r[0] or ""))
        if m and FIRST <= int(m.group(1)) <= LAST:
            anchors |= {(c, int(m.group(1))): v for c, v in zip(anchor[5][1:], r[1:]) if c and v}
    fine = list(openpyxl.load_workbook(cached(cache, "irr_era.xlsx", IRR_ERA), read_only=True,
                                       data_only=True)["Fine"].iter_rows(values_only=True))
    names = [(str(a or "").strip() + " " + str(b or "").strip()).strip() for a, b in zip(fine[4], fine[5])]
    rows = []
    for r in fine:
        m = re.match(r"(\d{4})M12$", str(r[1] or ""))
        if not m or not FIRST <= int(m.group(1)) <= LAST:
            continue
        year = int(m.group(1))
        for name, v in zip(names[2:], r[2:]):
            code = IRR_NAMES.get(name) or code_of.get(name)
            if code in iso3 and number(v) is not None:
                rows.append((code, year, int(v), IRR_COARSE[int(v)], anchors.get((code, year), "")))
    out = series("irr_regimes", "De facto exchange-rate arrangement at December of each year: fine code 1-15 (1 no "
                 "separate legal tender, 2 currency board or pre-announced peg, 3 narrow pre-announced band, 4 de "
                 "facto peg, 5-8 crawling pegs and narrow crawling bands, 9-11 wider or moving bands, 12 managed "
                 "floating, 13 freely floating, 14 freely falling, 15 dual market without parallel data), coarse code "
                 "1-6, and the anchor currency, Ilzetzki, Reinhart and Rogoff (2019, 2021), data to 2019",
                 table(OUT / "irr_regimes.csv", ["iso3", "year", "fine", "coarse", "anchor"], rows), ["S5.04"],
                 "The coarse code is derived from the fine one by the authors' Table 1 grouping (the coarse sheet "
                 "ends in 2016); country names are matched to iso3 through the anchor file's ISO codes.")
    return {"markets/irr": {"title": "Ilzetzki-Reinhart-Rogoff exchange rate arrangement classification (monthly, "
                                     "1940-2019) and anchor currencies (1946-2019)",
                            "url": f"{IRR_ERA}; {IRR_ANCHOR}", "fetched": TODAY}}, out


# ------------------------------------------------------------------------------------------------ Ratings

CRAS = {"STPGB": "S&P", "MDYGB": "Moody's", "FITGB": "Fitch"}
# ESMA's classes: corporate by industry (CO non-financial, FI financial, IN insurance), sovereign and public finance
# by sector (SV sovereign, SM sub-sovereign and municipal, PE public entities), structured finance by asset, covered
# bonds.
CLASSES = [("C", "industry", "CO"), ("C", "industry", "FI"), ("C", "industry", "IN"), ("S", "sector", "SV"),
           ("S", "sector", "SM"), ("S", "sector", "PE"), ("T", None, None), ("T", "asset", "RMBS"),
           ("T", "asset", "ABS"), ("T", "asset", "CMBS"), ("T", "asset", "CDO"), ("B", None, None)]
FISCAL = {"sovrate": "S3.10", "fxsovsh": "S5.04", "secnres": "S5.04", "fordebtsh": "S5.04", "fxdebtall": "S5.04",
          "avglife": "S3.03", "debtduey": "S3.03"}


def millis(day: datetime.date) -> str:
    return str(int(datetime.datetime(day.year, day.month, day.day, tzinfo=datetime.timezone.utc).timestamp() * 1000))


def cerep_matrix(cache: Path, cra: str, cls: tuple, year: int) -> str:
    kind, dim, code = cls
    path = cache / "cerep" / f"{cra}_{kind}_{code or 'all'}_{year}.csv"
    if path.exists():
        return path.read_text()
    filters = {"cra": [cra], "ratingType": [kind], "timeHorizon": ["L"],
               "begOfPrd": [millis(datetime.date(year, 1, 1))], "endOfPrd": [millis(datetime.date(year, 12, 31))]}
    if dim:
        filters[dim] = [code]
    body = json.dumps({"filters": {k: {"filterList": [{"code": v, "selected": True} for v in vs]}
                                   for k, vs in filters.items()}}).encode()
    for attempt in range(4):
        try:
            request = urllib.request.Request(CEREP_URL, data=body, method="POST",
                                             headers={"Content-Type": "application/json",
                                                      "User-Agent": "phoenix-data-fetch/1"})
            with urllib.request.urlopen(request, timeout=300) as r:
                text = r.read().decode("utf-8-sig")
            break
        except Exception as e:  # a slow or dropped answer is asked again a few times
            log(f"retry {attempt + 1} CEREP {cra} {cls} {year}: {e}")
    else:
        raise SystemExit(f"could not fetch CEREP {cra} {cls} {year}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return text


def ratings(cache: Path, iso3: set, iso2: dict) -> tuple:
    jobs = [(cra, cls, year) for cra in CRAS for cls in CLASSES for year in range(FIRST, 2025)]
    rows = []
    with concurrent.futures.ThreadPoolExecutor(8) as pool:
        texts = pool.map(lambda j: cerep_matrix(cache, *j), jobs)
        for (cra, (kind, _, code), year), text in zip(jobs, texts):
            lines = [l.split(";") for l in text.strip().splitlines()]
            if len(lines) < 2:
                continue
            header = lines[0][1:]
            for line in lines[1:]:
                for to, v in zip(header, line[1:]):
                    if number(v):
                        rows.append((CRAS[cra], kind, code or "all", year, line[0], to, int(number(v))))
    out = series("cerep_transitions", "One-year rating transitions (calendar years 2015-2024, long-term ratings), "
                 "counts of ratings by rating at the start of the year (from) and at its end (to, including default "
                 "grades "
                 "D/SD/RD, WR/NR and Withdrawals), for S&P, Moody's and Fitch (their EU-registered entities' "
                 "reported global books), by class: C corporate (CO non-financial, FI financial institutions, IN "
                 "insurance), S sovereign and public finance (SV sovereign, SM sub-sovereign and municipal, PE public "
                 "entities), T structured finance (all, RMBS, ABS, CMBS, CDO), B covered bonds; zero cells left out; "
                 "ESMA Central Rating Repository (CEREP)",
                 table(OUT / "cerep_transitions.csv", ["agency", "type", "class", "year", "from", "to", "count"], rows),
                 ["S3.10", "S4.05"],
                 "The rating distribution by issuer class is each matrix's row sums; one-year default rates are the "
                 "default columns over the row sums. The S&P and Moody's default studies themselves are not "
                 "reachable (spglobal.com answers 403, Moody's research needs a login); CEREP carries the same "
                 "agencies' reported statistics.")
    wbk = openpyxl.load_workbook(cached(cache, "fiscal_space.xlsx", FISCAL_SPACE), read_only=True, data_only=True)
    rows = []
    for var in FISCAL:
        sheet = list(wbk[var].iter_rows(values_only=True))
        years = {j: int(y) for j, y in enumerate(sheet[0]) if str(y).isdigit() and FIRST <= int(y) <= LAST}
        for r in sheet[1:]:
            if r[0] in iso3:
                rows += [(r[0], y, var, r[j]) for j, y in years.items() if number(r[j]) is not None]
    out |= series("fiscal_space", "Cross-country fiscal space database, by year: sovrate foreign-currency long-term "
                  "sovereign rating index 1-21 (21 = AAA/Aaa, the average of S&P, Moody's and Fitch), fxsovsh "
                  "general government debt in foreign currency (% of total), secnres debt securities held by "
                  "non-residents (% of total), fordebtsh government debt held by non-residents (% of total), "
                  "fxdebtall external debt in foreign currency (% of total), avglife sovereign debt average maturity "
                  "(years), debtduey central government debt maturing within 12 months (% of GDP); Kose, Kurlat, "
                  "Ohnsorge and Sugawara (2022), World Bank, Spring 2026 version",
                  table(OUT / "fiscal_space.csv", ["iso3", "year", "variable", "value"], rows),
                  ["S3.10", "S5.04", "S3.03"])
    return {"markets/cerep": {"title": "ESMA CEREP publication, transition matrix CSV export", "url": CEREP_URL,
                              "fetched": TODAY},
            "markets/fiscal_space": {"title": "World Bank, A Cross-Country Database of Fiscal Space (Spring 2026)",
                                     "url": FISCAL_SPACE, "fetched": TODAY, "release": "2026-04-20"}}, out


# ------------------------------------------------------------------------------------------------ Dealers, CCPs

# Government bond primary dealers (or their equivalent), read from each country's "A. List of Primary Dealers" in the
# AFME European Primary Dealers Handbook, updated 2024: (iso3, market, count, as of, section).
AFME_DEALERS = [
    ("AUT", "bonds and bills", 21, "2024-06", "1.2"), ("BEL", "bonds", 14, "2024-05", "2.2"),
    ("BGR", "government securities", 8, "2024-01", "3.2"), ("CZE", "bonds", 9, "2023-06", "4.2"),
    ("DNK", "bonds", 8, "2024-05", "5.2"), ("DNK", "bills", 4, "2024-05", "5.2"),
    ("FIN", "bonds", 13, "2024-02", "6.2"),
    ("FRA", "bonds", 15, "2024-05", "7.2"), ("DEU", "Bund Issues Auction Group", 32, "2023-12", "8.2-8.3"),
    ("GRC", "bonds", 17, "2024-01", "9.2"), ("HUN", "bonds", 13, "2023-12", "10.2"),
    ("IRL", "bonds", 14, "2024-03", "11.2"),
    ("ITA", "bonds", 21, "2024-06", "12.2"), ("NLD", "bonds", 13, "2024-04", "13.2"),
    ("POL", "bonds", 11, "2024-04", "14.2"),
    ("PRT", "bonds", 17, "2024-04", "15.2"), ("PRT", "bills", 19, "2024-04", "15.3"),
    ("SVK", "bonds", 9, "2024-03", "16.2"),
    ("SVN", "bonds", 14, "2024", "17.2"), ("SVN", "bills", 5, "2024", "17.2"), ("ESP", "bonds", 18, "2024-04", "18.2"),
    ("ESP", "bills", 19, "2024-04", "18.2"), ("SWE", "nominal bonds", 6, "2024-03", "19.2"),
    ("SWE", "inflation-linked bonds", 5, "2024-03", "19.2"), ("SWE", "bills", 5, "2024-03", "19.2"),
    ("GBR", "gilts (GEMMs)", 18, "2024-05", "20.2"), ("GBR", "bills", 23, "2024-05", "20.3"),
]
# CCP12 Public Quantitative Disclosure Newsflash Q4 2019 (April 2020), page 3: 42 CCPs (13 Americas, 16 APAC, 13
# EMEA); billions of US dollars and shares of collateral held.
CCP12 = [
    ("initial_margin_required", "USD bn", 796, "6.1.1"), ("default_fund_required", "USD bn", 102, "4.3.15"),
    ("collateral_required", "USD bn", 898, "6.1.1+4.3.15"), ("avg_daily_variation_margin", "USD bn", 24.1, "6.6.1"),
    ("im_house_share", "%", 36, "6.1.1"), ("im_client_share", "%", 64, "6.1.1"),
    ("im_house", "USD bn", 275, "6.1.1"), ("im_client_net", "USD bn", 100, "6.1.1"),
    ("im_client_gross", "USD bn", 378, "6.1.1"), ("ccps", "count", 42, ""),
    ("im_cash_central_bank", "%", 16, "6.2.1+6.2.2"), ("df_cash_central_bank", "%", 28, "4.3.1+4.3.2"),
    ("im_cash_secured", "%", 9, "6.2.3"), ("df_cash_secured", "%", 12, "4.3.3"),
    ("im_cash_unsecured_banks", "%", 5, "6.2.4"), ("df_cash_unsecured_banks", "%", 8, "4.3.4"),
    ("im_sovereign_domestic", "%", 33, "6.2.5"), ("df_sovereign_domestic", "%", 34, "4.3.5"),
    ("im_sovereign_foreign", "%", 20, "6.2.6"), ("df_sovereign_foreign", "%", 4, "4.3.6"),
    ("im_agency_bonds", "%", 1, "6.2.7"), ("df_agency_bonds", "%", 4, "4.3.7"),
    ("im_corporate_bonds", "%", 7, "6.2.9"), ("df_corporate_bonds", "%", 1, "4.3.9"),
    ("im_equities", "%", 5, "6.2.10"), ("df_equities", "%", 0, "4.3.10"),
    ("im_other", "%", 3, "6.2.14"), ("df_other", "%", 8, "4.3.14"),
]


def dealers(cache: Path, iso3: set, iso2: dict) -> tuple:
    page = cached(cache, "nyfed_primarydealers.html", NYFED_PD).read_text(errors="replace")
    tables = re.findall(r"<table.*?</table>", page, re.S)
    cell = lambda c: html.unescape(re.sub(r"<[^>]+>", "", c)).strip()
    names = max(re.findall(r"<td.*?</td>", tables[0], re.S), key=lambda c: c.count("\n"))
    current = [n.strip() for n in cell(names).split("\n") if n.strip()]
    rows = [("USA", "Treasury securities (current)", len(current), TODAY[:7], "NY Fed list")]
    # The count at the end of 2019 is the current list less the additions and plus the removals effective since.
    net = 0
    for tr in re.findall(r"<tr.*?</tr>", tables[1], re.S):
        cells = [cell(c) for c in re.findall(r"<t[dh].*?</t[dh]>", tr, re.S)]
        m = re.match(r"\w{3} \d+, (\d{4})", cells[0]) if cells else None
        if m and int(m.group(1)) >= 2020:
            net += cells[1].count("has been added") - cells[1].count("has been removed")
    rows.append(("USA", "Treasury securities", len(current) - net, "2019-12", "NY Fed list less later changes"))
    rows += [(c, market, n, asof, f"AFME handbook section {s}") for c, market, n, asof, s in AFME_DEALERS]
    out = series("primary_dealers", "Primary dealers (government securities dealers with the debt office's agreement) "
                 "by country and market: the US from the New York Fed's list (current, and at end-2019 by reversing "
                 "later additions and removals), European sovereigns typed from the AFME European Primary Dealers "
                 "Handbook (updated 2024)",
                 table(OUT / "primary_dealers.csv", ["iso3", "market", "count", "as_of", "source"], rows), ["S3.06"],
                 "Typed: AFME, European Primary Dealers Handbook, updated 2024 (afmeprimarydealers202403012updated111"
                 ".pdf), each country's section 'A. List of Primary Dealers' (section numbers in the source column), "
                 "the count its printed total; as_of is the list's date as the handbook states it. No obtainable "
                 "table was found for emerging or developing economies' debt offices.")
    rows = [("2019Q4", k, u, v, d) for k, u, v, d in CCP12]
    out |= series("typed_ccp12_pqd", "Central counterparties' aggregates, end-2019: initial margin and default fund "
                  "required, total collateral, average daily variation margin, house/client split of initial margin, "
                  "and composition of collateral held for initial margin (im_) and default funds (df_), 42 CCPs of "
                  "CCP12 members (13 Americas, 16 APAC, 13 EMEA)",
                  table(OUT / "typed_ccp12_pqd.csv", ["period", "item", "unit", "value", "disclosure"], rows),
                  ["S4.01"],
                  "Typed from CCP12, Public Quantitative Disclosure Newsflash Q4 2019 (April 2020), pages 2-3 "
                  "(CCP12-PQD-Newsflash-Q4-2019-April.pdf); the disclosure column gives the CPMI-IOSCO PQD reference. "
                  "Per-house default funds are not typed: the houses' own disclosures are separate files per house.")
    return {"markets/nyfed_pd": {"title": "Federal Reserve Bank of New York, primary dealers list and changes",
                                 "url": NYFED_PD, "fetched": TODAY},
            "markets/afme_pd": {"title": "AFME, European Primary Dealers Handbook, updated 2024 (pdf)", "url": AFME_PD,
                                "fetched": TODAY},
            "markets/ccp12": {"title": "CCP12, Public Quantitative Disclosure Newsflash Q4 2019 (pdf)",
                              "url": CCP12_PQD, "fetched": TODAY}}, out


SOURCES = {"wb": wb, "imf": imf, "oecd": oecd, "fsb": fsb, "gz": gz, "sec": sec, "hfcs": hfcs, "irr": irr,
           "ratings": ratings, "dealers": dealers}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cache", type=Path, default=None)
    parser.add_argument("--only", nargs="*", choices=sorted(SOURCES))
    args = parser.parse_args()
    cache = args.cache or Path(tempfile.mkdtemp())
    cache.mkdir(parents=True, exist_ok=True)
    iso3, iso2 = iso_maps()
    for name in args.only or sorted(SOURCES):
        sources, found = SOURCES[name](cache, iso3, iso2)
        merge_manifest(sources, found)
        for key, s in sorted(found.items()):
            log(f"{key}: {s['rows']} rows")


if __name__ == "__main__":
    main()
