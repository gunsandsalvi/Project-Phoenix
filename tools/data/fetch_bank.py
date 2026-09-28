#!/usr/bin/env python3
"""Fetches the published data on banking, credit, the central bank and sovereign debt the later build steps read,
into data/sources/raw/bank/, for 2015 to 2025 (end of year) unless a source says otherwise:

- BIS total credit statistics: credit to households, firms, the private non-financial sector, government and the
  whole non-financial sector, from all lenders and from domestic banks, as a share of GDP and in domestic currency.
- BIS credit-to-GDP gaps: the private credit ratio, its trend and the gap.
- BIS central bank total assets, in domestic currency and as a share of GDP.
- BIS debt securities statistics (national sources): debt securities issued by sector, by original and remaining
  maturity, currency and market, and held by sector where the source says who holds them.
- BIS international debt securities: amounts outstanding by issuer sector, currency group and maturity.
- IMF Monetary and Financial Statistics: the central bank survey and the other depository corporations survey
  (reserves, currency, claims on government, the private sector and banks, transferable and other deposits,
  capital), and the interest rates (money market, treasury bills, government bonds, deposit and lending rates).
- IMF Financial Soundness Indicators of deposit takers: capital to risk-weighted assets, non-performing loans and
  provisions, liquidity, foreign-currency loans and liabilities, large exposures, real estate loans, and the loans
  from which the interbank share is derived.
- IMF Investment and Capital Stock Dataset: public, private and partnership capital stocks and investment.
- World Bank Global Financial Development Database: stability (SI) and depth (DI.01 to DI.05) indicators, the
  banking crisis dummy, and WDI's bank non-performing loan ratio.
- World Bank Quarterly Public Sector Debt: central and general government debt by creditor residence, currency,
  instrument and maturity.
- OECD financial balance sheets with counterpart information (Table 725): who holds whose deposits, securities,
  loans and equity.
- World Bank Deposit Insurance Database (as of 2013) and Bank Regulation and Supervision Survey (2019 release, data
  as of 2016); premiums as of 2003 typed from Demirguc-Kunt, Karacaovali and Laeven (2005), Table A.1.4.
- Laeven and Valencia (2020) Systemic Banking Crises Database II: crisis dates, costs and policy responses.
- Arslanalp and Tsuda Sovereign Investor Base (advanced and emerging, 2016 vintage): who holds government debt.

    python3 tools/data/fetch_bank.py [--cache DIR] [--only NAME ...]

The downloads are kept in the cache directory (default: a temporary directory) and reused when present.
"""
import argparse
import csv
import datetime
import io
import json
import re
import tempfile
import unicodedata
import zipfile
from pathlib import Path

from fetch import FIRST, LAST, RAW, cached, get, log, merge_manifest
from fetch_pop import countries, table

OUT = RAW / "bank"
TODAY = datetime.date.today().isoformat()
SDMX_CSV = "application/vnd.sdmx.data+csv;version=1.0.0"

BIS_BULK = "https://data.bis.org/static/bulk/{flow}_csv_flat.zip"
IMF_SDMX = "https://api.imf.org/external/sdmx/2.1/data/{flow}/{key}?startPeriod={first}&endPeriod={last}"
WB_API = ("https://api.worldbank.org/v2/country/all/indicator/{code}?format=json&date={first}:{last}"
          "&per_page=20000&source={source}")
OECD_725 = ("https://sdmx.oecd.org/public/rest/data/OECD.SDD.NAD,DSD_NASEC20@DF_T725R_A,/A.................?"
            "startPeriod={first}&format=csvfile")
DIS_URL = ("https://datacatalogfiles.worldbank.org/ddh-published/0040209/1/DR0050060/"
           "deposit_insurance_database_july2015.xlsx")
BRSS_URL = ("https://datacatalogfiles.worldbank.org/ddh-published/0038632/DR0047735/"
            "survey-20191104-brss-public-release.xlsx")
DKKL_URL = "https://documents1.worldbank.org/curated/en/593131468330040612/pdf/wps36280rev.pdf"
LV_URL = ("https://static-content.springer.com/esm/art%3A10.1057%2Fs41308-020-00107-3/MediaObjects/"
          "41308_2020_107_MOESM1_ESM.xlsx")
SIB_URL = {"advanced": "https://www.imf.org/external/pubs/ft/wp/2012/Data/wp12284.zip",
           "emerging": "https://www.imf.org/external/pubs/ft/wp/2014/Data/wp1439.zip"}


def years() -> range:
    return range(FIRST, LAST + 1)


def wide(path: Path, keys: list, columns: list, cells: dict) -> int:
    """Writes one row per key tuple with a column per name; an absent cell stays empty."""
    rows = {}
    for (*key, column), value in cells.items():
        rows.setdefault(tuple(key), {})[column] = value
    return table(path, keys + columns, [list(k) + [r.get(c, "") for c in columns] for k, r in rows.items()])


def num(value) -> str:
    """A figure to three decimals, trailing zeros dropped: the sources' further digits are computational noise."""
    text = f"{float(value):.3f}".rstrip("0").rstrip(".")
    return "0" if text == "-0" else text


def code(text: str) -> str:
    return text.split(":", 1)[0].strip()


def header_codes(fields: list) -> list:
    """A BIS header's codes; a label with a comma inside it arrives split, its pieces starting with a space."""
    header = []
    for h in fields:
        if h.startswith(" ") and header:
            continue
        header.append(code(h))
    return header


def bis_rows(cache: Path, flow: str):
    """Streams a BIS bulk file as dicts of codes; header fields broken by commas inside their labels are rejoined."""
    z = zipfile.ZipFile(cached(cache, f"{flow}.zip", BIS_BULK.format(flow=flow)))
    with z.open(z.namelist()[0]) as f:
        reader = csv.reader(io.TextIOWrapper(f, encoding="utf-8-sig"))
        header = header_codes(next(reader))
        for r in reader:
            yield dict(zip(header, (code(x) for x in r)))


def end_of_year(period: str) -> int | None:
    """The year a period closes, for the fourth quarter, December or a whole year; None for any other period."""
    if re.fullmatch(r"\d{4}", period) or re.fullmatch(r"\d{4}-Q4", period) or re.fullmatch(r"\d{4}-(M)?12", period):
        return int(period[:4])
    return None


def iso3_of_iso2() -> dict:
    return {r["iso2"]: r["iso3"] for r in csv.DictReader((RAW / "wb" / "countries.csv").open())}


# Total credit: borrowers (C whole non-financial sector, P private non-financial, H households, N non-financial
# corporations, G general government) and lenders (A all sectors, B domestic banks).
TC_UNITS = {"770": "pct_gdp", "XDC": "bn_xdc"}


def bis_total_credit(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    of = iso3_of_iso2()
    cells, columns = {}, set()
    for r in bis_rows(cache, "WS_TC"):
        year = end_of_year(r["TIME_PERIOD"])
        if (r["BORROWERS_CTY"] not in of or r["TC_ADJUST"] != "A" or r["UNIT_TYPE"] not in TC_UNITS
                or year is None or year not in years() or not r["OBS_VALUE"]):
            continue
        column = f'{r["TC_BORROWERS"]}_{r["TC_LENDERS"]}_{r["VALUATION"]}_{TC_UNITS[r["UNIT_TYPE"]]}'
        columns.add(column)
        cells[(of[r["BORROWERS_CTY"]], year, column)] = num(r["OBS_VALUE"])
    series["bank/bis_total_credit"] = {
        "title": "Total credit to the non-financial sectors, end of year (Q4), adjusted for breaks, BIS total credit "
                 "statistics (WS_TC): column <borrower>_<lender>_<valuation>_<unit>, borrower C non-financial sector, "
                 "P private non-financial, H households and NPISH, N non-financial corporations, G general "
                 "government; lender A all sectors, B domestic banks; valuation M market, N nominal; unit pct_gdp "
                 "(% of GDP) or bn_xdc (billions of domestic currency)",
        "rows": wide(OUT / "bis_total_credit.csv", ["iso3", "year"], sorted(columns), cells),
        "for": ["S1.22", "S6.05", "S7.01"],
    }
    sources["bank/bis_tc"] = {"title": "BIS total credit statistics, bulk CSV", "url": BIS_BULK.format(flow="WS_TC"),
                              "fetched": TODAY}


def bis_credit_gap(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    of = iso3_of_iso2()
    names = {"A": "ratio", "B": "trend", "C": "gap"}
    cells = {}
    for r in bis_rows(cache, "WS_CREDIT_GAP"):
        year = end_of_year(r["TIME_PERIOD"])
        if r["BORROWERS_CTY"] in of and year in years() and r["OBS_VALUE"] and r["CG_DTYPE"] in names:
            cells[(of[r["BORROWERS_CTY"]], year, names[r["CG_DTYPE"]])] = num(r["OBS_VALUE"])
    series["bank/bis_credit_gap"] = {
        "title": "Credit to the private non-financial sector from all sectors, % of GDP, end of year (Q4): the ratio, "
                 "its one-sided HP-filter trend and the gap (ratio less trend), BIS credit-to-GDP gaps (WS_CREDIT_GAP)",
        "rows": wide(OUT / "bis_credit_gap.csv", ["iso3", "year"], ["ratio", "trend", "gap"], cells),
        "for": ["S2.08", "S7.01"],
    }
    sources["bank/bis_credit_gap"] = {"title": "BIS credit-to-GDP gaps, bulk CSV",
                                      "url": BIS_BULK.format(flow="WS_CREDIT_GAP"), "fetched": TODAY}


def bis_cb_assets(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    """End of year from the finest frequency the BIS gives: quarterly, else monthly, else annual."""
    of = iso3_of_iso2()
    units = {"XDC": "bn_xdc", "XDF_R_B1GQ": "pct_gdp"}
    rank = {"Q": 3, "M": 2, "A": 1}
    best = {}
    for r in bis_rows(cache, "WS_CBTA"):
        year = end_of_year(r["TIME_PERIOD"])
        if (r["REF_AREA"] not in of or year not in years() or not r["OBS_VALUE"] or r["UNIT_MEASURE"] not in units
                or r["TRANSFORMATION"] not in ("B", "N")):
            continue
        key = (of[r["REF_AREA"]], year, units[r["UNIT_MEASURE"]])
        # A break-adjusted series is preferred to the raw one, then the finer frequency.
        score = (r["TRANSFORMATION"] == "B", rank.get(r["FREQ"], 0))
        value = num(r["OBS_VALUE"])
        if key not in best or score > best[key][0]:
            best[key] = (score, value)
    cells = {k: v for k, (_, v) in best.items()}
    series["bank/bis_cb_assets"] = {
        "title": "Central bank total assets, end of year, billions of domestic currency (bn_xdc) and % of GDP "
                 "(pct_gdp), BIS-spliced, break-adjusted where the BIS adjusts, BIS central bank total assets (WS_CBTA)",
        "rows": wide(OUT / "bis_cb_assets.csv", ["iso3", "year"], ["bn_xdc", "pct_gdp"], cells),
        "for": ["S1.22", "S3.02"],
    }
    sources["bank/bis_cbta"] = {"title": "BIS central bank total assets, bulk CSV",
                                "url": BIS_BULK.format(flow="WS_CBTA"), "fetched": TODAY}


# Debt securities by issuer: the sectors a closed economy of households, firms, banks, a central bank and a government
# issues from, and the holders where the source names them.
DSS_SECTORS = {"S1", "S11", "S12", "S121", "S122", "S13", "S1311", "S1M"}
DSS_VALUATION = {"N": 3, "F": 2, "M": 1}


def bis_debt_securities(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    of = iso3_of_iso2()
    q4 = {f"{y}-Q4" for y in years()}
    best = {}
    for r in bis_rows(cache, "WS_NA_SEC_DSS"):
        if r["TIME_PERIOD"] not in q4 or r["STO"] != "LE" or r["CUST_BREAKDOWN"] != "_T" or not r["OBS_VALUE"]:
            continue
        if r["REF_AREA"] not in of or r["REF_SECTOR"] not in DSS_SECTORS:
            continue
        if r["ACCOUNTING_ENTRY"] == "L" and r["INSTR_ASSET"] != "F3" and r["REF_SECTOR"] not in ("S13", "S1311"):
            continue
        # Maturities are kept for all currencies in all markets; the currency and market splits, and the holdings,
        # for all maturities only.
        split = r["CURRENCY_DENOM"] != "_T" or r["COUNTERPART_AREA"] != "XW" or r["ACCOUNTING_ENTRY"] == "A"
        if split and r["MATURITY"] != "T":
            continue
        iso, year = of[r["REF_AREA"]], int(r["TIME_PERIOD"][:4])
        dims = (r["ACCOUNTING_ENTRY"], r["REF_SECTOR"], r["COUNTERPART_SECTOR"], r["COUNTERPART_AREA"],
                r["INSTR_ASSET"], r["MATURITY"], r["CURRENCY_DENOM"])
        # One figure per series: in the national currency rather than dollars, at nominal rather than market value.
        score = (r["UNIT_MEASURE"] != "USD", DSS_VALUATION.get(r["VALUATION"], 0))
        key = (iso, year) + dims
        if key not in best or score > best[key][0]:
            best[key] = (score, (r["VALUATION"], r["UNIT_MEASURE"], num(r["OBS_VALUE"])))
    rows = [k + v for k, (_, v) in best.items()]
    series["bank/bis_debt_securities"] = {
        "title": "Debt securities outstanding, end of year (Q4), billions of the unit named (national currency where "
                 "reported, else US dollars), BIS debt securities statistics from national sources (WS_NA_SEC_DSS): "
                 "entry L issued by sector (S1 all, S11 non-financial corporations, S12 financial corporations, S121 "
                 "central bank, S122 deposit takers, S13 general government, S1311 central government, S1M "
                 "households), entry A held by sector with counterpart sector the issuer; market XW all, 1E "
                 "residents/local, 5Z non-residents/cross-border; instrument F3, and fixed (F3FR) and variable "
                 "(F3VR, F3VRA inflation-linked, F3VRB rate-linked, F3VRC asset-price-linked) rate for government; "
                 "maturity T all, S short-term original, L long-term original, LS long-term original due within a "
                 "year, Y12/Y25/Y5A/YA_ original 1-2/2-5/5-10/over 10 years; currency _T all, XDC domestic, X1 "
                 "foreign; valuation N nominal, F face, M market (nominal preferred)",
        "rows": table(OUT / "bis_debt_securities.csv",
                      ["iso3", "year", "entry", "sector", "counterpart_sector", "market", "instrument", "maturity",
                       "currency", "valuation", "unit", "value_bn"], rows),
        "for": ["S1.22", "S3.03", "S3.04"],
    }
    sources["bank/bis_dss"] = {"title": "BIS debt securities statistics, bulk CSV",
                               "url": BIS_BULK.format(flow="WS_NA_SEC_DSS"), "fetched": TODAY}


# International debt securities by the issuer's immediate sector: 1 all issuers, 2 general government, B financial
# corporations, E private banks, I public banks, J non-financial corporations.
IDS_SECTORS = {"1": "all", "2": "gov", "B": "fin", "E": "privbank", "I": "pubbank", "J": "nfc"}
IDS_CUTS = {("A", "A", "A"): "all", ("F", "A", "A"): "fx", ("D", "A", "A"): "dc", ("A", "C", "A"): "short",
            ("A", "K", "A"): "long", ("A", "A", "U"): "due1y"}


def bis_international_debt(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    of = iso3_of_iso2()
    q4 = re.compile(r",(20(1[5-9]|2[0-5]))-Q4,")
    z = zipfile.ZipFile(cached(cache, "WS_DEBT_SEC2_PUB.zip", BIS_BULK.format(flow="WS_DEBT_SEC2_PUB")))
    cells, columns = {}, set()
    with z.open(z.namelist()[0]) as f:
        text = io.TextIOWrapper(f, encoding="utf-8-sig")
        header = header_codes(next(csv.reader([next(text)])))
        for line in text:
            # A cheap filter first: the file is several gigabytes and only year-end amounts outstanding are kept.
            if "I: Amounts outstanding" not in line or not q4.search(line):
                continue
            r = dict(zip(header, (code(x) for x in next(csv.reader([line])))))
            cut = IDS_CUTS.get((r["ISSUE_CUR_GROUP"], r["ISSUE_OR_MAT"], r["ISSUE_RE_MAT"]))
            if (r["ISSUER_RES"] not in of or r["ISSUER_NAT"] != "3P" or r["ISSUER_BUS_ULT"] != "1"
                    or r["ISSUER_BUS_IMM"] not in IDS_SECTORS or cut is None or r["ISSUE_RATE"] != "A"
                    or r["ISSUE_CUR"] != "TO1" or r["ISSUE_TYPE"] != "A" or r["MARKET"] != "C" or not r["OBS_VALUE"]):
                continue
            column = f"{IDS_SECTORS[r['ISSUER_BUS_IMM']]}_{cut}"
            columns.add(column)
            cells[(of[r["ISSUER_RES"]], int(r["TIME_PERIOD"][:4]), column)] = r["OBS_VALUE"]
    series["bank/bis_international_debt"] = {
        "title": "International debt securities outstanding by residence of issuer, end of year (Q4), millions of US "
                 "dollars, BIS international debt securities (WS_DEBT_SEC2_PUB): column <issuer>_<cut>, issuer all, "
                 "gov general government, fin financial corporations, privbank and pubbank private and public banks, "
                 "nfc non-financial corporations; cut all, fx foreign currencies, dc domestic currency, short and "
                 "long original maturity, due1y remaining maturity up to one year",
        "rows": wide(OUT / "bis_international_debt.csv", ["iso3", "year"], sorted(columns), cells),
        "for": ["S3.03", "S3.04", "S5.04"],
    }
    sources["bank/bis_ids"] = {"title": "BIS international debt securities, bulk CSV",
                               "url": BIS_BULK.format(flow="WS_DEBT_SEC2_PUB"), "fetched": TODAY}


def imf_csv(flow: str, key: str) -> list:
    url = IMF_SDMX.format(flow=flow, key=key, first=FIRST, last=LAST)
    return list(csv.DictReader(io.StringIO(get(url, timeout=900, accept=SDMX_CSV).decode("utf-8-sig"))))


# The central bank survey's lines, each with a short column name.
CBS = {
    "S121_A_TA_ASEC_CB1SR": "total_assets",
    "S121_A_ACO_NRES_CBS": "claims_nonresidents",
    "S121_A_ACO_ODCORP_CBS": "claims_banks",
    "S121_A_ACO_S12R_CBS": "claims_other_financial",
    "S121_A_ACO_S1311MIXED_CBS": "claims_central_gov",
    "S121_A_ACO_S13M1_CBS": "claims_local_gov",
    "S121_A_ACO_S11001_CBS": "claims_public_firms",
    "S121_A_ACO_PS_CBS": "claims_private",
    "S121_A_ACO_S1_Z_CBS": "claims_other_sectors",
    "S121_L_MB_CBS": "monetary_base",
    "S121_L_CIC_IMB_CBS": "currency_in_circulation",
    "S121_L_IMB_LT_ODCORP_CBS": "base_liab_banks",
    "S121_L_OLT_ODCORP_CBS": "other_liab_banks",
    "S121_L_IMB_LT_S1_Z_CBS": "base_liab_other_sectors",
    "S121_L_F2M_XMBIBM_CBS": "deposits_broad_money",
    "S121_L_F3_XMBIBM_CBS": "securities_broad_money",
    "S121_L_F2M_XMBXBM_CBS": "deposits_excluded",
    "S121_L_F3_XMBXBM_CBS": "securities_excluded",
    "S121_L_F4_CBS": "loans",
    "S121_L_F71_CBS": "derivatives",
    "S121_L_F51KOE_CBS": "shares_equity",
    "S121_L_LT_S1311MIXED_CBS": "liab_central_gov",
    "S121_L_LT_NRES_CBS": "liab_nonresidents",
    "S121_N_NFRA_CBS": "net_foreign_assets",
    "S121_N_NCO_S1311MIXED_CBS": "net_claims_central_gov",
    "S121_N_OIN_CBS": "other_items_net",
}
# The other depository corporations survey's lines.
ODC = {
    "ODCORP_A_T_ASEC_ODC2SR": "total_assets",
    "ODCORP_A_ACO_NRES_ODCS": "claims_nonresidents",
    "ODCORP_A_ACO_S121_ODCS": "claims_central_bank",
    "ODCORP_A_F21_ACO_S121_ODCS": "currency_held",
    "ODCORP_A_F2MSOTS_RR_ACO_S121_ODCS": "reserve_deposits",
    "ODCORP_A_OCO_S121_ODCS": "other_claims_central_bank",
    "ODCORP_A_ACO_S1311MIXED_ODCS": "claims_central_gov",
    "ODCORP_A_ACO_S13M1_ODCS": "claims_local_gov",
    "ODCORP_A_ACO_S11001_ODCS": "claims_public_firms",
    "ODCORP_A_ACO_PS_ODCS": "claims_private",
    "ODCORP_A_ACO_S12R_ODCS": "claims_other_financial",
    "ODCORP_A_ACO_S1_Z_ODCS": "claims_other_sectors",
    "ODCORP_L_F22_IBM_ODCS": "transferable_deposits",
    "ODCORP_L_F29_IBM_ODCS": "other_deposits",
    "ODCORP_L_F2M_XBM_ODCS": "deposits_excluded",
    "ODCORP_L_F3_IBM_ODCS": "securities_broad_money",
    "ODCORP_L_F3_XBM_ODCS": "securities_excluded",
    "ODCORP_L_F4_ODCS": "loans",
    "ODCORP_L_F6_ODCS": "insurance_reserves",
    "ODCORP_L_F71_ODCS": "derivatives",
    "ODCORP_L_F51KOE_ODCS": "shares_equity",
    "ODCORP_L_LT_S121_ODCS": "liab_central_bank",
    "ODCORP_L_LT_S1311MIXED_ODCS": "liab_central_gov",
    "ODCORP_L_LT_NRES_ODCS": "liab_nonresidents",
    "ODCORP_NETAL_NFRA_ODCS": "net_foreign_assets",
    "ODCORP_NETAL_OIN_ASEC_ODCS": "other_items_net",
}


def mfs_survey(flow: str, lines: dict) -> tuple:
    """A survey's end-of-year lines in the economy's own currency (the euro for its members, the dollar where it is
    the currency), in millions."""
    rank = {"XDC": 3, "EUR": 2, "USD": 1}
    best = {}
    for r in imf_csv(f"IMF.STA,{flow}", "*." + "+".join(lines) + ".XDC+EUR+USD.A"):
        if r["OBS_VALUE"] and r["TIME_PERIOD"].isdigit():
            best.setdefault(r["COUNTRY"], {}).setdefault(r["TYPE_OF_TRANSFORMATION"], []).append(r)
    cells = {}
    for iso, by in best.items():
        unit = max(by, key=rank.get)
        for r in by[unit]:
            cells[(iso, int(r["TIME_PERIOD"]), unit, lines[r["INDICATOR"]])] = num(float(r["OBS_VALUE"]) / 1e6)
    return cells


def imf_mfs(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    for flow, lines, name, what in (("MFS_CBS", CBS, "imf_mfs_cbs", "Central bank survey"),
                                    ("MFS_ODC", ODC, "imf_mfs_odc", "Other depository corporations survey")):
        cells = {k: v for k, v in mfs_survey(flow, lines).items() if k[0] in iso3}
        series[f"bank/{name}"] = {
            "title": f"{what}, end of year, millions of the currency named (the economy's own; EUR for euro area "
                     f"members; USD where the dollar is the currency), IMF Monetary and Financial Statistics "
                     f"({flow}); columns are the survey's lines: " + ", ".join(f"{v} = {k}" for k, v in lines.items()),
            "rows": wide(OUT / f"{name}.csv", ["iso3", "year", "currency"], list(lines.values()), cells),
            "for": ["S1.22", "S2.06", "S3.01", "S3.02"],
        }
        sources[f"bank/{name}"] = {"title": f"IMF SDMX 2.1 API, {flow}",
                                   "url": IMF_SDMX.format(flow=f"IMF.STA,{flow}", key="<lines>.XDC+EUR+USD.A",
                                                          first=FIRST, last=LAST), "fetched": TODAY}


IR = {
    "MMRT_RT_PT_A_PT": "money_market",
    "MFS172_RT_PT_A_PT": "repo",
    "DISR_RT_PT_A_PT": "discount",
    "GSTBILY_RT_PT_A_PT": "treasury_bill",
    "S13BOND_RT_PT_A_PT": "government_bond",
    "MFS135_RT_PT_A_PT": "deposit",
    "MFS174_RT_PT_A_PT": "savings",
    "MFS162_RT_PT_A_PT": "lending",
    "MFS135_FC_RT_PT_A_PT": "deposit_fx",
    "MFS162_FC_RT_PT_A_PT": "lending_fx",
}


def imf_rates(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    cells = {(r["COUNTRY"], int(r["TIME_PERIOD"]), IR[r["INDICATOR"]]): num(r["OBS_VALUE"])
             for r in imf_csv("IMF.STA,MFS_IR", "*." + "+".join(IR) + ".A")
             if r["COUNTRY"] in iso3 and r["OBS_VALUE"] and r["TIME_PERIOD"].isdigit()}
    series["bank/imf_mfs_ir"] = {
        "title": "Interest rates, % a year, annual (the IMF's annual figure, an average over the year), IMF Monetary "
                 "and Financial Statistics (MFS_IR): money market, repurchase agreement, discount, treasury bill "
                 "yield, government bond yield, deposit, savings and lending rates, and deposit and lending rates in "
                 "foreign currency",
        "rows": wide(OUT / "imf_mfs_ir.csv", ["iso3", "year"], list(IR.values()), cells),
        "for": ["S2.06", "S3.01", "S3.02", "S3.03"],
    }
    sources["bank/imf_mfs_ir"] = {"title": "IMF SDMX 2.1 API, MFS_IR",
                                  "url": IMF_SDMX.format(flow="IMF.STA,MFS_IR", key="<rates>.A", first=FIRST,
                                                         last=LAST), "fetched": TODAY}


# Financial soundness indicators of deposit takers (sector S12CFSI) and of real estate markets (REM), in %. The two
# spreads the FSI also carry are left out: reporters give them in basis points and in percentage points alike.
FSI = {
    "FSI688_CFSI_PT": "reg_capital_to_rwa",
    "FSI626_CFSI_PT": "tier1_to_rwa",
    "FSI15_CFSI_PT": "cet1_to_rwa",
    "T1KTA_CFSI_PT": "tier1_to_assets",
    "AQ12_CFSI_PT": "npl_to_loans",
    "AQ14_CFSI_PT": "provisions_to_npl",
    "FSI17_CFSI_PT": "npl_net_to_capital",
    "ROA_CFSI_PT": "return_on_assets",
    "ROE_CFSI_PT": "return_on_equity",
    "FSI99_CFSI_PT": "interest_margin_to_income",
    "FSI107_CFSI_PT": "noninterest_expense_to_income",
    "FSI354_AFSI_PT": "personnel_to_noninterest_expense",
    "FSI690_AFSI_PT": "trading_income_to_income",
    "FSI283_LIQATTA_PT": "liquid_to_assets",
    "FSI765_CFSI_PT": "liquid_to_short_liabilities",
    "FSI288_CFSI_PT": "liquidity_coverage_ratio",
    "FSI289_CFSI_PT": "net_stable_funding_ratio",
    "FSI55_AFSI_PT": "customer_deposits_to_loans",
    "FSI131_AFSI_PT": "fx_loans_to_loans",
    "FSI680_AFSI_PT": "fx_liabilities_to_liabilities",
    "FSI555_CFSI_PT": "net_open_fx_to_capital",
    "FSI214_AFSI_PT": "large_exposures_to_capital",
    "FSI524_CFSI_PT": "residential_re_to_loans",
    "FSI520_AFSI_PT": "commercial_re_to_loans",
}
# Levels in domestic currency read only to derive the interbank share of loans and risk-weighted assets per asset.
FSI_LEVELS = {"FSI303_TGF4_XDC": "gross_loans", "FSI55_TNINTBKF4_XDC": "noninterbank_loans",
              "FSI688_RWA_XDC": "rwa", "T1KTA_TA_XDC": "total_assets"}


def imf_fsi(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    """End of year: the fourth quarter, else December, else the annual figure."""
    names = {**FSI, **FSI_LEVELS}
    rank = {"Q": 3, "M": 2, "A": 1}
    best = {}
    for r in imf_csv("IMF.STA,FSIC", "*.S12CFSI+REM." + "+".join(names) + ".Q+M+A"):
        year = end_of_year(r["TIME_PERIOD"])
        if r["COUNTRY"] not in iso3 or year is None or not r["OBS_VALUE"]:
            continue
        key = (r["COUNTRY"], year, names[r["INDICATOR"]])
        score = rank[r["FREQUENCY"]]
        if key not in best or score > best[key][0]:
            best[key] = (score, r["OBS_VALUE"])
    cells = {k: num(v) for k, (_, v) in best.items() if k[2] in FSI.values()}
    level = {k: float(v) for k, (_, v) in best.items() if k[2] in FSI_LEVELS.values()}
    for iso, year in {(k[0], k[1]) for k in level}:
        at = lambda n: level.get((iso, year, n))
        if at("gross_loans") and at("noninterbank_loans") is not None:
            cells[(iso, year, "interbank_to_loans")] = num(100 * (1 - at("noninterbank_loans") / at("gross_loans")))
            if at("total_assets"):
                cells[(iso, year, "interbank_to_assets")] = \
                    num(100 * (at("gross_loans") - at("noninterbank_loans")) / at("total_assets"))
        if at("rwa") and at("total_assets"):
            cells[(iso, year, "rwa_to_assets")] = num(100 * at("rwa") / at("total_assets"))
    columns = list(FSI.values()) + ["interbank_to_loans", "interbank_to_assets", "rwa_to_assets"]
    series["bank/imf_fsi"] = {
        "title": "Financial soundness indicators of deposit takers (and real estate loans), %, end of year (Q4, else December, else annual; flow ratios such as returns cover the year "
                 "to that quarter), IMF Financial Soundness Indicators (FSIC): " +
                 ", ".join(f"{v} = {k}" for k, v in FSI.items()),
        "note": "Three columns are derived from the FSI reporting's own levels in domestic currency: "
                "interbank_to_loans = 100 x (1 - noninterbank loans (FSI55_TNINTBKF4_XDC) / total gross loans "
                "(FSI303_TGF4_XDC)); interbank_to_assets = 100 x (gross loans - noninterbank loans) / total assets "
                "(T1KTA_TA_XDC); rwa_to_assets = 100 x risk-weighted assets (FSI688_RWA_XDC) / total assets. The "
                "interbank share stands for the supervisory exposure data no public source gives.",
        "rows": wide(OUT / "imf_fsi.csv", ["iso3", "year"], columns, cells),
        "for": ["S1.15", "S2.01", "S2.06", "S2.07", "S3.01", "S5.04"],
    }
    sources["bank/imf_fsi"] = {"title": "IMF SDMX 2.1 API, FSIC",
                               "url": IMF_SDMX.format(flow="IMF.STA,FSIC", key="*.S12CFSI+REM.<indicators>.Q+M+A",
                                                      first=FIRST, last=LAST), "fetched": TODAY}


ICSD = {
    "CAPSTCK_S13_V_XDC": "k_gov_xdc", "CAPSTCK_PS_V_XDC": "k_private_xdc", "CAPSTCK_PUPVT_V_XDC": "k_ppp_xdc",
    "P51G_S13_V_XDC": "i_gov_xdc", "P51G_PS_V_XDC": "i_private_xdc", "B1GQ_V_XDC": "gdp_xdc",
    "CAPSTCK_S13_Q_POGDP_PT": "k_gov_pct_gdp", "CAPSTCK_PS_Q_POGDP_PT": "k_private_pct_gdp",
    "CAPSTCK_PUPVT_Q_POGDP_PT": "k_ppp_pct_gdp",
}


def imf_icsd(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    cells = {(r["COUNTRY"], int(r["TIME_PERIOD"]), ICSD[r["INDICATOR"]]): num(r["OBS_VALUE"])
             for r in imf_csv("IMF.FAD,ICSD", "*." + "+".join(ICSD) + ".A")
             if r["COUNTRY"] in iso3 and r["OBS_VALUE"]}
    series["bank/imf_icsd"] = {
        "title": "Capital stock and gross fixed capital formation by general government, private sector and "
                 "public-private partnerships, current prices, domestic currency (billions as published, _xdc), and "
                 "capital stock at constant prices as % of GDP (_pct_gdp), 2015-2019 (the dataset ends in 2019), IMF "
                 "Investment and Capital Stock Dataset (ICSD, 2025 release)",
        "rows": wide(OUT / "imf_icsd.csv", ["iso3", "year"], list(ICSD.values()), cells),
        "for": ["S1.22"],
    }
    sources["bank/imf_icsd"] = {"title": "IMF SDMX 2.1 API, IMF.FAD ICSD",
                                "url": IMF_SDMX.format(flow="IMF.FAD,ICSD", key="<indicators>.A", first=FIRST,
                                                       last=LAST), "fetched": TODAY}


GFDD = {
    "GFDD.SI.01": "bank_z_score", "GFDD.SI.02": "npl_to_gross_loans", "GFDD.SI.03": "capital_to_assets",
    "GFDD.SI.04": "credit_to_deposits", "GFDD.SI.05": "reg_capital_to_rwa",
    "GFDD.SI.06": "liquid_to_deposits_short_funding", "GFDD.SI.07": "provisions_to_npl",
    "GFDD.DI.01": "private_credit_by_banks_to_gdp", "GFDD.DI.03": "nonbank_assets_to_gdp",
    "GFDD.DI.04": "bank_to_bank_plus_cb_assets", "GFDD.DI.05": "liquid_liabilities_to_gdp",
    "GFDD.OI.19": "banking_crisis",
}


def wb_series(code: str, source: int, first, last) -> list:
    page = json.loads(get(WB_API.format(code=code, first=first, last=last, source=source), timeout=300))
    return [r for r in page[1] or [] if r["value"] is not None]


def wb_gfdd(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    cells = {}
    for code_, name in GFDD.items():
        for r in wb_series(code_, 32, FIRST, LAST):
            if r["countryiso3code"] in iso3:
                cells[(r["countryiso3code"], int(r["date"]), name)] = num(r["value"])
    for r in wb_series("FB.AST.NPER.ZS", 2, FIRST, LAST):
        if r["countryiso3code"] in iso3:
            cells[(r["countryiso3code"], int(r["date"]), "npl_wdi")] = num(r["value"])
    columns = list(GFDD.values()) + ["npl_wdi"]
    series["bank/gfdd"] = {
        "title": "Bank stability and depth, % unless stated, annual, World Bank Global Financial Development Database "
                 "(source 32): " + ", ".join(f"{v} = {k}" for k, v in GFDD.items()) +
                 " (1 = systemic banking crisis, Laeven-Valencia dating); npl_wdi = bank non-performing loans to "
                 "total gross loans, WDI FB.AST.NPER.ZS",
        "rows": wide(OUT / "gfdd.csv", ["iso3", "year"], columns, cells),
        "for": ["S1.15", "S1.22", "S2.01", "S2.07", "S3.02", "S7.01"],
    }
    sources["bank/wb_gfdd"] = {"title": "World Bank API, GFDD (source 32) and WDI (source 2)",
                               "url": WB_API.format(code="<series>", first=FIRST, last=LAST, source="32"),
                               "fetched": TODAY}


# Quarterly Public Sector Debt, % of GDP, for central (CG) and general (GG) government: <code> -> column stem.
QPSD = {
    "DECT": "total", "DECD": "domestic_creditors", "DECX": "external_creditors", "DECN": "domestic_currency",
    "DECF": "foreign_currency", "DLDS": "securities", "DLLO": "loans", "DLCD": "currency_deposits",
    "DSTC": "short_term", "DSDS": "short_term_securities", "DSLO": "short_term_loans", "DLTC": "long_term",
    "DLTC.CR.L1": "long_term_due_1y", "DLDS.CR.L1": "long_term_securities_due_1y",
    "DLTC.CR.M1": "long_term_due_after_1y", "DLDS.CR.M1": "long_term_securities_due_after_1y",
    "DLDS.CR.MV": "securities_market_value",
}


def wb_qpsd(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    cells, columns = {}, []
    for stem, name in QPSD.items():
        for level in ("CG", "GG"):
            ident = f"DP.DOD.{stem}.{level}.Z1" if ".CR." in stem else f"DP.DOD.{stem}.CR.{level}.Z1"
            column = f"{level.lower()}_{name}"
            columns.append(column)
            for r in wb_series(ident, 20, f"{FIRST}Q4", f"{LAST}Q4"):
                if r["countryiso3code"] in iso3 and r["date"].endswith("Q4"):
                    cells[(r["countryiso3code"], int(r["date"][:4]), column)] = num(r["value"])
    series["bank/qpsd"] = {
        "title": "Gross public sector debt of central (cg_) and general (gg_) government, % of GDP, nominal value "
                 "unless stated, end of year (Q4), World Bank Quarterly Public Sector Debt (source 20): total, by "
                 "creditor residence, by currency, by instrument (securities, loans, currency and deposits), short-term "
                 "original maturity, long-term, long-term with payment due within one year and after one year, and "
                 "securities at market value",
        "rows": wide(OUT / "qpsd.csv", ["iso3", "year"], columns, cells),
        "for": ["S1.15", "S3.03", "S3.04", "S5.04"],
    }
    sources["bank/wb_qpsd"] = {"title": "World Bank API, Quarterly Public Sector Debt (source 20)",
                               "url": WB_API.format(code="DP.DOD.<series>.Z1", first=f"{FIRST}Q4", last=f"{LAST}Q4",
                                                    source="20"), "fetched": TODAY}


# The whole economy is left out: it is the sum of the four resident sectors.
OECD_SECTORS = {"S11", "S12", "S13", "S1M", "S2"}
OECD_INSTRUMENTS = {"F2", "F22", "F29", "F3", "F4", "F5", "F51"}


def oecd_counterparts(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    path = cache / "oecd_t725.csv"
    if not path.exists():
        log("downloading OECD Table 725")
        path.write_bytes(get(OECD_725.format(first=FIRST), timeout=1800))
    rows = []
    with path.open(encoding="utf-8-sig") as f:
        for r in csv.DictReader(f):
            if (r["REF_AREA"] in iso3 and r["OBS_VALUE"] and r["ACCOUNTING_ENTRY"] == "A"
                    and r["SECTOR"] in OECD_SECTORS and r["COUNTERPART_SECTOR"] in OECD_SECTORS
                    and r["INSTR_ASSET"] in OECD_INSTRUMENTS and r["MATURITY"] in ("T", "_Z", "S", "L")):
                rows.append((r["REF_AREA"], int(r["TIME_PERIOD"]), r["SECTOR"], r["COUNTERPART_SECTOR"],
                             r["INSTR_ASSET"], r["MATURITY"], r["OBS_VALUE"], r["UNIT_MULT"]))
    series["bank/oecd_counterparts"] = {
        "title": "Financial assets held by each sector on each counterpart sector (from whom to whom), closing stocks, "
                 "national currency (x 10^unit_mult), OECD financial balance sheets, counterpart information (Table "
                 "725): holder and counterpart S11 non-financial corporations, S12 financial "
                 "corporations, S13 general government, S1M households and NPISH, S2 rest of the world; instruments "
                 "F2 deposits (F22 transferable, F29 other), F3 debt securities, F4 loans, F5 equity (F51); maturity "
                 "T all, S short-term, L long-term, _Z not applicable. 13 economies report it",
        "note": "Deposit takers are not reported apart from other financial corporations here, so interbank "
                "positions appear only inside S12 on S12; the interbank share by economy is derived in bank/imf_fsi.",
        "rows": table(OUT / "oecd_counterparts.csv",
                      ["iso3", "year", "holder", "counterpart", "instrument", "maturity", "value", "unit_mult"],
                      sorted(set(rows))),
        "for": ["S1.22", "S2.06", "S3.01", "S3.03"],
    }
    sources["bank/oecd_t725"] = {"title": "OECD Data Explorer, SDMX API, financial balance sheets with counterpart "
                                          "information (DF_T725R_A)",
                                 "url": OECD_725.format(first=FIRST), "fetched": TODAY}


def norm(name: str) -> str:
    text = unicodedata.normalize("NFKD", str(name)).encode("ascii", "ignore").decode().lower()
    text = re.sub(r"\s*\d+/\s*", " ", text)
    return re.sub(r"[^a-z]+", " ", text).strip()


# Names the papers print that the World Bank's list spells otherwise.
ALIASES = {
    "czech rep": "CZE", "czech republic": "CZE", "slovak republic": "SVK", "korea": "KOR", "south korea": "KOR",
    "russia": "RUS", "egypt": "EGY", "iran": "IRN", "venezuela": "VEN", "yemen": "YEM", "gambia": "GMB",
    "congo dem rep": "COD", "congo rep": "COG", "dem rep of congo": "COD", "congo dem rep of": "COD",
    "cote d ivoire": "CIV", "kyrgyz republic": "KGZ", "kyrgyzstan": "KGZ", "lao pdr": "LAO", "laos": "LAO",
    "macedonia": "MKD", "north macedonia": "MKD", "macedonia fyr": "MKD", "bosnia herzegovina": "BIH",
    "bosnia and herzegovina": "BIH", "micronesia": "FSM", "taiwan": "TWN", "hong kong": "HKG", "turkey": "TUR",
    "slovakia": "SVK", "vietnam": "VNM", "viet nam": "VNM", "trinidad tobago": "TTO", "cape verde": "CPV",
    "swaziland": "SWZ", "st kitts and nevis": "KNA", "st lucia": "LCA", "st vincent and the grenadines": "VCT",
    "united states": "USA", "united kingdom": "GBR", "syria": "SYR", "bahamas": "BHS", "sao tome and principe": "STP",
    "guinea bissau": "GNB", "central african rep": "CAF", "central african republic": "CAF", "uruguay": "URY",
    "china p r mainland": "CHN", "china": "CHN", "brunei": "BRN", "burma": "MMR", "myanmar": "MMR",
    "serbia montenegro": "SRB", "congo rep of": "COG", "cote divoire": "CIV", "iran i r of": "IRN",
    "lao peoples dem rep": "LAO", "sao tome principe": "STP", "serbia": "SRB", "yugoslavia": "SRB", "ussr": "RUS",
}


def name_to_iso3() -> callable:
    table_ = {norm(r["name"]): r["iso3"] for r in csv.DictReader((RAW / "wb" / "countries.csv").open())}
    table_.update(ALIASES)

    def of(name):
        n = norm(name)
        if n in table_:
            return table_[n]
        head = n.split(" ")[0]
        hits = {v for k, v in table_.items() if k.split(" ")[0] == head and len(head) > 4}
        return hits.pop() if len(hits) == 1 else None
    return of


def laeven_valencia(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    import openpyxl
    wb = openpyxl.load_workbook(cached(cache, "laeven_valencia_2020.xlsx", LV_URL), read_only=True, data_only=True)
    of = name_to_iso3()
    missing = set()

    def iso(name):
        found = of(name)
        if found is None:
            missing.add(name)
        return found or ""

    kinds = ["banking", "currency", "sovereign_debt", "sovereign_restructuring"]
    rows = []
    for r in list(wb["Crisis Years"].iter_rows(values_only=True))[2:]:
        if not r[0] or not any(r[1:5]):
            continue
        for kind, cell in zip(kinds, r[1:5]):
            for year in re.findall(r"\d{4}", str(cell or "")):
                rows.append((iso(r[0]), str(r[0]).strip(), kind, int(year)))
    series["bank/crisis_years"] = {
        "title": "Start years of systemic banking, currency and sovereign debt crises and of sovereign debt "
                 "restructurings, 1970-2017, Laeven and Valencia (2020), Systemic Banking Crises Database II, IMF "
                 "Economic Review 68, 307-361, sheet 'Crisis Years'",
        "rows": table(OUT / "crisis_years.csv", ["iso3", "country", "kind", "year"], rows),
        "for": ["S7.01"],
    }
    out = []
    for r in list(wb["Crisis Resolution and Outcomes"].iter_rows(values_only=True))[1:]:
        if not r[0] or not re.fullmatch(r"\d{4}", str(r[1] or "").strip()):
            continue
        clean = lambda v: "" if v is None or str(v).strip() in ("...", "…") else num(v) if isinstance(v, (int, float)) \
            else str(v).strip()
        flags = " ".join(re.findall(r"\d+/", f"{r[0]} {r[2]}"))
        out.append((iso(re.sub(r"\s*\d+/", "", str(r[0]))), int(r[1]), re.sub(r"\s*\d+/", "", clean(r[2])),
                    *(clean(v) for v in r[3:11]), flags))
    series["bank/crisis_outcomes"] = {
        "title": "Systemic banking crises 1970-2017: start and end year, output loss (% of trend GDP, cumulative over "
                 "T..T+3), fiscal cost gross and net (% of GDP) and gross (% of financial system assets), peak "
                 "liquidity support and liquidity support (% of deposits and foreign liabilities), peak NPLs (% of "
                 "loans), increase in public debt (% of GDP); flags are the sheet's footnotes (6/ end dated from GDP "
                 "only, 7/ duration truncated at 5 years, 8/ borderline case), Laeven and Valencia (2020), sheet "
                 "'Crisis Resolution and Outcomes'",
        "rows": table(OUT / "crisis_outcomes.csv",
                      ["iso3", "start", "end", "output_loss", "fiscal_cost", "fiscal_cost_net",
                       "fiscal_cost_pct_fin_assets", "peak_liquidity", "liquidity_support", "peak_npl",
                       "public_debt_increase", "flags"], out),
        "for": ["S2.08", "S3.02", "S7.01"],
    }
    detail = list(wb["Additional Details-Bk Crises"].iter_rows(values_only=True))
    labels = []
    for r in detail:
        label = norm(r[0] or "").replace(" ", "_")
        # A label the sheet repeats (an introduction date, a duration) is numbered by its order.
        seen = sum(1 for other in labels if other == label or other.startswith(label + "_"))
        labels.append(f"{label}_{seen + 1}" if label and seen else label)
    keep = [i for i, label in enumerate(labels) if label and label not in ("brief_description_of_crisis", "country_name")
            and any(detail[i][1:])]
    header = ["iso3", "country"] + [labels[i] for i in keep]
    crises = []
    for col in range(1, len(detail[0])):
        if not detail[0][col]:
            continue
        value = lambda v: v.date().isoformat() if isinstance(v, datetime.datetime) else "" if v is None else str(v)
        crises.append([iso(detail[0][col]), str(detail[0][col]).strip()] + [value(detail[i][col]) for i in keep])
    series["bank/crisis_details"] = {
        "title": "Policy responses and outcomes per systemic banking crisis (one row per crisis): deposit insurance "
                 "and its coverage, deposit freezes and bank holidays, guarantees, liquidity support, restructuring, "
                 "nationalisation, asset purchases, recapitalisation and its cost, losses imposed on depositors, IMF "
                 "programme, peak NPLs and fiscal cost (shares as fractions), Laeven and Valencia (2020), sheet "
                 "'Additional Details-Bk Crises'",
        "rows": table(OUT / "crisis_details.csv", header, crises),
        "for": ["S2.08", "S3.02", "S7.01"],
    }
    sources["bank/laeven_valencia"] = {"title": "Laeven and Valencia (2020), Systemic Banking Crises Database II, "
                                                "electronic supplementary material of IMF Economic Review 68",
                                       "url": LV_URL, "fetched": TODAY}
    if missing:
        log(f"Laeven-Valencia names without an iso3 (kept with an empty code): {sorted(missing)}")


def deposit_insurance(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    import openpyxl
    wb = openpyxl.load_workbook(cached(cache, "deposit_insurance_2013.xlsx", DIS_URL), read_only=True,
                                data_only=True)
    rows = list(wb["Data as of end 2013"].iter_rows(values_only=True))
    # The design and funding columns, without the scheme's name and website.
    keep = [i for i, h in enumerate(rows[0]) if h and i not in (0, 2, 3, 4, 6, 7)]
    header = ["iso3"] + [norm(rows[0][i]).replace(" ", "_")[:60] for i in keep[1:]]
    out = [[r[1]] + ["" if r[i] is None else str(r[i]).strip() for i in keep[1:]]
           for r in rows[1:] if r[1] in iso3]
    series["bank/deposit_insurance_2013"] = {
        "title": "Deposit insurance design as of end 2013 (explicit scheme, inception, administration, membership, "
                 "coverage of foreign-currency and interbank deposits, statutory coverage limits 2003/2010/2013 in "
                 "reported currency and US dollars and over GDP per head, coinsurance, ex-ante fund, funding source, "
                 "government backstop, risk-adjusted premiums, assessment base, payout basis, losses imposed, "
                 "crisis-time guarantees), World Bank Deposit Insurance Database (Demirguc-Kunt, Kane and Laeven, "
                 "2014, Policy Research Working Paper 6934), sheet 'Data as of end 2013'",
        "rows": table(OUT / "deposit_insurance_2013.csv", header, out),
        "for": ["S2.06", "S2.08"],
    }
    sources["bank/deposit_insurance"] = {"title": "World Bank Data Catalog, Deposit Insurance Database (0040209)",
                                         "url": DIS_URL, "release": "2015-07", "fetched": TODAY}
    premiums(cache, sources, series, iso3)


def premiums(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    """Table A.1.4 read by the position of each line on its page: the columns are at fixed horizontal offsets."""
    import pymupdf
    doc = pymupdf.open(cached(cache, "wps3628.pdf", DKKL_URL))
    of = name_to_iso3()
    rows = []
    for page in (36, 37, 38):
        lines = []
        for block in doc[page].get_text("dict")["blocks"]:
            for line in block.get("lines", []):
                text = "".join(s["text"] for s in line["spans"]).strip()
                if text:
                    lines.append((round(line["bbox"][1]), line["bbox"][0], text))
        for y, x, text in sorted(lines):
            if y < 135 or y > 735 or text.startswith(("yes=1", "Notes", "n.a. stands")):
                continue
            column = 0 if x < 180 else 1 if x < 226 else 2 if x < 305 else 3 if x < 490 else 4
            if column == 0:
                rows.append([text, "", "", "", ""])
            elif rows:
                rows[-1][column] = (rows[-1][column] + " " + text).strip()
    out = []
    for name, funded, base, premium, risk in rows:
        # Where a base's text runs into the premium's column in print, the premium begins at its first rate term.
        if "%" in base:
            split = re.search(r"\b(risk|until)\b", base)
            if split:
                base, premium = base[:split.start()].strip(), (base[split.start():] + " " + premium).strip()
        out.append((of(name) or "", name, funded, base, premium, risk))
    series["bank/typed_deposit_insurance_premiums_2003"] = {
        "title": "Deposit insurance funding and premiums as of 2003: permanent fund (1 funded, 0 unfunded), premium "
                 "or assessment base, annual premium as % of the base (as printed), risk-adjusted premiums (1 yes)",
        "note": "Typed from Demirguc-Kunt, A., B. Karacaovali and L. Laeven (2005), 'Deposit Insurance around the "
                "World: A Comprehensive Database', World Bank Policy Research Working Paper 3628, Table A.1.4 'Type "
                "of fund and premium information', pp. 36-38 of the PDF downloaded from documents1.worldbank.org; "
                "read from the PDF's text by column position, the printed text kept verbatim. No later public source "
                "gives premium rates by country (the IADI survey is for members only).",
        "rows": table(OUT / "typed_deposit_insurance_premiums_2003.csv",
                      ["iso3", "country", "funded", "base", "annual_premium", "risk_adjusted"], out),
        "for": ["S2.08"],
    }
    sources["bank/dkkl_2005"] = {"title": "Demirguc-Kunt, Karacaovali and Laeven (2005), World Bank PRWP 3628",
                                 "url": DKKL_URL, "fetched": TODAY}


# The survey's sections kept whole: capital, liquidity, deposit protection, provisioning, problem banks and exit,
# and the banking sector's characteristics; and from entry and supervision only the questions named.
BRSS_WHOLE = {"03", "07", "08", "09", "11", "13"}
BRSS_SOME = {"01": ("Q1_4", "Q1_7", "Q1_12"), "06": ("Q6_7_1",),
             "12": ("Q12_23", "Q12_28", "Q12_29", "Q12_30", "Q12_31", "Q12_33", "Q12_34", "Q12_39", "Q12_45")}


def brss(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    import openpyxl
    wb = openpyxl.load_workbook(cached(cache, "brss_2019.xlsx", BRSS_URL), read_only=True, data_only=True)
    general = list(wb["General"].iter_rows(values_only=True))
    code_of = {str(r[0]).strip(): r[1] for r in general[1:] if r[0] and r[1]}
    currency = [(r[1], r[3] or "", r[4] or "") for r in general[1:] if r[1] in iso3]
    answers, questions = [], []
    for sheet in sorted(BRSS_WHOLE | set(BRSS_SOME)):
        rows = list(wb[sheet].iter_rows(values_only=True))
        header = rows[0]
        parent = ""
        for r in rows[1:]:
            qid, text = (r[0] or "").strip(), str(r[1] or "").strip()
            if not qid:
                parent = text
                continue
            # Free text, the "not applicable" and "do not know" boxes, and the years before 2015 are left out.
            if ("_Text_" in qid or "_Descr_" in qid or re.search(r"(na|dk|NoneAbove)_\d{4}$", qid)
                    or re.search(r"_201[1-4]$", qid)
                    or (sheet in BRSS_SOME and not qid.startswith(BRSS_SOME[sheet]))):
                continue
            label = text if re.match(r"\d+\.\d", text) else f"{parent} | {text}"
            questions.append((qid, sheet, label))
            for name, value in zip(header[2:], r[2:]):
                iso = code_of.get(str(name).strip())
                if iso in iso3 and value not in (None, ""):
                    answers.append((iso, qid, re.sub(r"(_x000D_)?\s*\n\s*", " ", str(value)).strip()))
    series["bank/brss_2019"] = {
        "title": "Bank Regulation and Supervision Survey, 2019 release (answers as of end 2016, some questions by "
                 "year, of which 2015 and 2016 are kept), one row per economy and question: minimum entry capital and licensing, capital "
                 "requirements and actual ratios, buffers, large exposures and liquidity requirements, deposit "
                 "protection (coverage limit, coinsurance, share of deposits and depositors covered, fund ratio, "
                 "premium basis, payout days, backstops), asset classification, provisioning and write-offs, problem "
                 "bank powers and resolutions, macroprudential limits (LTV, DTI), supervisory staff and budget, and "
                 "number and structure of banks; 'X' marks a ticked option; money answers are in the unit of "
                 "brss_2019_currency; free-text answers and the 'not applicable', 'do not know' and 'none of the above' boxes are "
                 "left out. World Bank",
        "rows": table(OUT / "brss_2019.csv", ["iso3", "question", "answer"], answers),
        "for": ["S2.01", "S2.06", "S2.07", "S2.08", "S3.02"],
    }
    series["bank/brss_2019_questions"] = {
        "title": "The questions of bank/brss_2019: identifier, survey section and text (parent question | option)",
        "rows": table(OUT / "brss_2019_questions.csv", ["question", "section", "text"], questions),
        "for": ["S2.01", "S2.06", "S2.07", "S2.08", "S3.02"],
    }
    series["bank/brss_2019_currency"] = {
        "title": "The currency and currency unit each economy answered bank/brss_2019's money questions in",
        "rows": table(OUT / "brss_2019_currency.csv", ["iso3", "currency", "unit"], currency),
        "for": ["S2.08"],
    }
    sources["bank/brss_2019"] = {"title": "World Bank Data Catalog, Bank Regulation and Supervision Survey (0038632), "
                                          "2019 database", "url": BRSS_URL, "release": "2019-11-04", "fetched": TODAY}


SIB_TABLES = {"Table 1. Total": "total", "Table 2. Foreign": "foreign", "Table 2.1 ForeignOfficial": "foreign_official",
              "Table 2.2 ForeignBank": "foreign_bank", "Table 2.3 ForeignNonbank": "foreign_nonbank",
              "Table 3. Domestic": "domestic", "Table 3.1 DomesticCentralBank": "domestic_central_bank",
              "Table 3.2 DomesticBank": "domestic_bank", "Table 3.3 DomesticNonbank": "domestic_nonbank"}


def sovereign_investor_base(cache: Path, sources: dict, series: dict, iso3: set) -> None:
    import openpyxl
    of = name_to_iso3()
    cells, missing = {}, set()
    for group, url in SIB_URL.items():
        z = zipfile.ZipFile(cached(cache, f"sib_{group}.zip", url))
        wb = openpyxl.load_workbook(io.BytesIO(z.read(z.namelist()[0])), read_only=True, data_only=True)
        for sheet, holder in SIB_TABLES.items():
            block, periods = None, None
            for r in wb[sheet].iter_rows(values_only=True):
                first = str(r[0] or "").strip()
                if first.startswith("o/w"):
                    block = "lc_cg_securities" if "local" in first else "securities"
                elif first == "Billion LC":
                    block = block or "debt"
                    periods = r
                elif first.startswith("Percent"):
                    periods = None
                elif not first:
                    if periods:
                        block, periods = None, None
                elif periods:
                    iso = of(first)
                    if iso is None:
                        missing.add(first)
                        continue
                    for period, value in zip(periods[1:], r[1:]):
                        if period and str(period)[:4].isdigit() and int(str(period)[:4]) >= FIRST - 1 \
                                and isinstance(value, (int, float)):
                            cells[(iso, str(period), block, holder)] = num(value)
    series["bank/sovereign_investor_base"] = {
        "title": "Holders of general government gross debt (block debt), of its securities (securities) and of "
                 "central government local-currency securities (lc_cg_securities, emerging only), billions of "
                 "national currency, quarterly 2014Q1-2016Q2: total, foreign (official, banks, nonbanks), domestic "
                 "(central bank, banks, nonbanks), IMF Sovereign Investor Base datasets for 24 advanced and 24 "
                 "emerging economies (Arslanalp and Tsuda, 2014a,b; October 2016 vintages, the latest the IMF "
                 "publishes openly)",
        "rows": wide(OUT / "sovereign_investor_base.csv", ["iso3", "period", "block"], list(SIB_TABLES.values()),
                     cells),
        "for": ["S3.03", "S5.04"],
    }
    sources["bank/sovereign_investor_base"] = {"title": "IMF Sovereign Investor Base datasets (WP/12/284, WP/14/39)",
                                               "url": " ".join(SIB_URL.values()), "release": "2016-10",
                                               "fetched": TODAY}
    if missing:
        log(f"investor-base names without an iso3 (left out): {sorted(missing)}")


SOURCES = {
    "bis_total_credit": bis_total_credit, "bis_credit_gap": bis_credit_gap, "bis_cb_assets": bis_cb_assets,
    "bis_debt_securities": bis_debt_securities, "bis_international_debt": bis_international_debt,
    "imf_mfs": imf_mfs, "imf_rates": imf_rates, "imf_fsi": imf_fsi, "imf_icsd": imf_icsd, "wb_gfdd": wb_gfdd,
    "wb_qpsd": wb_qpsd, "oecd_counterparts": oecd_counterparts, "laeven_valencia": laeven_valencia,
    "deposit_insurance": deposit_insurance, "brss": brss, "sovereign_investor_base": sovereign_investor_base,
}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cache", type=Path, default=None)
    parser.add_argument("--only", nargs="*", choices=sorted(SOURCES))
    args = parser.parse_args()
    cache = args.cache or Path(tempfile.mkdtemp())
    cache.mkdir(parents=True, exist_ok=True)
    iso3 = countries()
    for name in args.only or sorted(SOURCES):
        sources, series = {}, {}
        SOURCES[name](cache, sources, series, iso3)
        merge_manifest(sources, series)
        log(f"{name} done: " + ", ".join(f"{k} {v['rows']}" for k, v in series.items()))


if __name__ == "__main__":
    main()
