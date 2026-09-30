#!/usr/bin/env python3
"""Fetches the published data energy and the open world are opened from, into data/sources/raw/open/ (and wb/ for
World Bank series), each kept as a compact CSV of what a mechanism reads:

- Ember, yearly electricity data (long format): per economy and year 2015-2025 (2025 partial), capacity (GW), generation (TWh) and
  power-sector emissions (MtCO2) by fuel, demand and net imports (TWh).
- WRI Global Power Plant Database v1.3: every plant of the world profiles' economies, its primary fuel, capacity,
  commissioning year and reported 2019 generation - plant sizes and ages by fuel.
- NREL Annual Technology Baseline, electricity: the 2020 edition's 2019 values and the 2023 edition's 2021 values of
  capital, fixed and variable operating costs, heat rates, capacity factors and fuel costs by technology (moderate
  scenario, market case, 30-year cost recovery).
- IEA/NEA Projected Costs of Generating Electricity 2020: its plant tables (3.1-3.10) and levelised cost tables
  (3.11-3.19) and its harmonised assumptions (lifetimes, construction lengths, fuel prices), typed from the report,
  with the EIA's 2019 round-trip efficiencies of storage; committed as typed_* files and only registered here.
- World Bank WDI: transmission and distribution losses, access to electricity, and applied and MFN tariff rates
  (simple and weighted means, all, manufactured and primary products).
- WITS (UNCTAD TRAINS) tariffs by product group (HS sections and stages of processing), effectively applied simple and
  weighted means and MFN weighted means, 2017-2021.
- OECD ICIO (2023 edition), 2019: each economy's imports by the origin's development level and industry, split by
  use, and its exports by the destination's development level and industry.
- Fernández, Klein, Rebucci, Schindler and Uribe capital control measures, 2021 update: restrictions on inflows and
  outflows by asset category, 2015-2019.
- Boz et al. invoicing currency shares of exports and imports (US dollar, euro, home, other), 2010-2019.
- OECD TiVA and TiM 2025 editions: output, value added, trade, employment and labour compensation of the five mining
  divisions (ISIC B05-B09), which the 2023 input-output tables merge in pairs.
- Shipping: UNCTADstat port calls (median days in port, vessel sizes and ages, number of calls by market), merchant
  fleet by flag and type, gross tonnage built; OECD TiVA output of shipbuilding (ISIC C301), and the value of a gross
  ton built derived from the two; the Fourth IMO GHG Study's fleet table (days at sea by ship type and size) and
  MARAD's daily operating costs by cost category, typed from the reports.

    python3 tools/data/fetch_open.py [--cache DIR] [--only NAME ...]

The downloads are kept in the cache directory (default: a temporary directory) and reused when present.
"""
import argparse
import csv
import datetime
import io
import json
import lzma
import re
import struct
import tempfile
import zipfile
from pathlib import Path

from fetch import RAW, cached, get, log, merge_manifest
from fetch_pop import countries, table

OUT = RAW / "open"
TODAY = datetime.date.today().isoformat()
FIRST, LAST = 2015, 2025
SNAPSHOT = 2019

EMBER = "https://storage.googleapis.com/emb-prod-bkt-publicdata/public-downloads/yearly_full_release_long_format.csv"
GPPD = "https://wri-dataportal-prod.s3.amazonaws.com/manual/global_power_plant_database_v_1_3.zip"
ATB = "https://oedi-data-lake.s3.amazonaws.com/ATB/electricity/csv/{path}"
ATB_EDITIONS = {2020: ("2020/ATBe.csv", "2019"), 2023: ("2023/ATBe.csv", "2021")}
ATB_PARAMETERS = {"CAPEX", "OCC", "Fixed O&M", "Variable O&M", "Heat Rate", "CF", "Fuel"}
PCGE = "https://iea.blob.core.windows.net/assets/ae17da3d-e8a5-4163-a3ec-2e6fb0b5677d/" \
       "Projected-Costs-of-Generating-Electricity-2020.pdf"
EIA_STORAGE = "https://www.eia.gov/todayinenergy/detail.php?id=46756"
WB_API = "https://api.worldbank.org/v2/country/all/indicator/{code}?format=json&date={first}:{last}&per_page=20000" \
         "&source=2"
WB_SERIES = ["EG.ELC.LOSS.ZS", "EG.ELC.ACCS.ZS", "TM.TAX.MRCH.SM.AR.ZS", "TM.TAX.MRCH.WM.AR.ZS",
             "TM.TAX.MANF.SM.AR.ZS", "TM.TAX.MANF.WM.AR.ZS", "TM.TAX.TCOM.SM.AR.ZS", "TM.TAX.TCOM.WM.AR.ZS",
             "TM.TAX.MRCH.SM.FN.ZS", "TM.TAX.MRCH.WM.FN.ZS"]
WITS = "https://wits.worldbank.org/API/V1/SDMX/V21/datasource/tradestats-tariff/reporter/all/year/{year}/partner/wld" \
       "/product/all/indicator/{indicator}"
WITS_INDICATORS = {"AHS-WGHTD-AVRG": "applied_weighted", "AHS-SMPL-AVRG": "applied_simple",
                   "MFN-WGHTD-AVRG": "mfn_weighted"}
WITS_YEARS = range(2017, 2022)
# WITS's own codes where they differ from ISO 3166.
WITS_ISO = {"ROM": "ROU", "ZAR": "COD"}
ICIO_ZIP = "icio_2016_2020.zip"
ICIO_URL = "https://stats.oecd.org/wbos/fileview2.aspx?IDFile=d1ab2315-298c-4e93-9a81-c6f2273139fe"
FINAL = {"HFCE": "hfce", "DPABR": "hfce", "NPISH": "npish", "GGFC": "ggfc", "GFCF": "gfcf", "INVNT": "invnt"}
FKRSU = "https://www.columbia.edu/~mu2166/fkrsu/2021-FKRSU-Update-12-08-2021.xlsx"
BOZ = "https://data.mendeley.com/public-files/datasets/6z6w78968m/files/4aaaadea-370e-4d8a-ac4f-2e6a47d36428/" \
      "file_downloaded"
# TiVA 2025 is served only from the STI endpoint, TiM 2025 only from the public one.
OECD_STI = "https://sdmx.oecd.org/{endpoint}/rest/data/OECD.STI.PIE,{flow}/{key}?startPeriod={first}" \
           "&endPeriod={last}&format=csvfile"
MINING = "B05+B06+B07+B08+B09"
UNCTAD = "https://unctadstat-api.unctad.org/bulkdownload/{code}/{file}"
UNCTAD_SETS = ["US.PortCalls", "US.PortCallsArrivals", "US.MerchantFleet", "US.ShipBuilding"]
M49 = "https://comtradeapi.un.org/files/v1/app/reference/partnerAreas.json"
IMO = "https://greenvoyage2050.imo.org/wp-content/uploads/2021/07/" \
      "Fourth-IMO-GHG-Study-2020-Full-report-and-annexes_compressed.pdf"
MARAD = "https://rosap.ntl.bts.gov/view/dot/42874/dot_42874_DS1.pdf"


def levels() -> dict:
    """Each economy's development level: the World Bank's income groups, high as developed, upper middle as
    emerging, lower middle and low as developing."""
    by_group = {"High income": "developed", "Upper middle income": "emerging", "Lower middle income": "developing",
                "Low income": "developing"}
    return {r["iso3"]: by_group[r["income_group"]] for r in csv.DictReader((RAW / "wb" / "countries.csv").open())}


def ember(cache: Path, iso3: set) -> tuple:
    fuels = ["Coal", "Gas", "Other Fossil", "Nuclear", "Hydro", "Wind", "Solar", "Bioenergy", "Other Renewables"]
    measures = {("Capacity", "GW"): "capacity_gw", ("Electricity generation", "TWh"): "generation_twh",
                ("Power sector emissions", "mtCO2"): "emissions_mtco2"}
    totals = {("Electricity generation", "Total Generation"): "generation_twh",
              ("Power sector emissions", "Total emissions"): "emissions_mtco2",
              ("Electricity demand", "Demand"): "demand_twh",
              ("Electricity imports", "Net Imports"): "net_imports_twh"}
    cells: dict = {}
    with cached(cache, "ember_yearly_long.csv", EMBER).open(encoding="utf-8-sig") as f:
        for r in csv.DictReader(f):
            area = "WLD" if r["Area"] == "World" else r["ISO 3 code"]
            year = int(r["Year"])
            if area not in iso3 | {"WLD"} or not FIRST <= year <= LAST or r["Value"] == "":
                continue
            if r["Subcategory"] == "Fuel" and (r["Category"], r["Unit"]) in measures:
                cells[(area, year, measures[(r["Category"], r["Unit"])], r["Variable"])] = r["Value"]
            elif (r["Category"], r["Variable"]) in totals:
                cells[(area, year, totals[(r["Category"], r["Variable"])], "total")] = r["Value"]
    keys = sorted({k[:3] for k in cells})
    header = ["iso3", "year", "measure"] + [f.lower().replace(" ", "_") for f in fuels] + ["total"]
    rows = [list(k) + [cells.get(k + (f,), "") for f in fuels + ["total"]] for k in keys]
    n = table(OUT / "ember_electricity.csv", header, rows)
    return ({"open/ember": {"title": "Ember, Yearly electricity data, long format", "url": EMBER, "fetched": TODAY}},
            {"open/ember_electricity": {
                "title": f"Electricity by economy and year {FIRST}-{LAST}: capacity (GW), generation (TWh) and "
                         "power-sector emissions (MtCO2) by fuel, with total generation, emissions, demand (TWh) and "
                         "net imports (TWh) in the total column; WLD is the world; Ember yearly electricity data",
                "rows": n, "for": ["S2.203", "S2.216"]}})


def gppd(cache: Path, iso3: set) -> tuple:
    z = zipfile.ZipFile(cached(cache, "gppd_v1_3.zip", GPPD))
    rows = []
    with z.open("global_power_plant_database.csv") as f:
        for r in csv.DictReader(io.TextIOWrapper(f, encoding="utf-8")):
            if r["country"] in iso3:
                year = r["commissioning_year"]
                rows.append((r["country"], r["primary_fuel"], r["other_fuel1"], r["capacity_mw"],
                             f"{float(year):.2f}" if year else "", r["year_of_capacity_data"],
                             r["generation_gwh_2019"]))
    n = table(OUT / "gppd_plants.csv", ["iso3", "primary_fuel", "other_fuel1", "capacity_mw", "commissioning_year",
                                        "year_of_capacity_data", "generation_gwh_2019"], rows)
    return ({"open/gppd": {"title": "WRI Global Power Plant Database v1.3", "url": GPPD, "fetched": TODAY,
                           "release": "2021-06-02"}},
            {"open/gppd_plants": {
                "title": "Power plants of 1 MW and more by economy: primary and first other fuel, capacity (MW), "
                         "commissioning year (capacity-weighted mean year of the units, fractional), year of the "
                         "capacity data, reported 2019 generation (GWh, where published); WRI Global Power Plant "
                         "Database v1.3", "rows": n, "for": ["S2.203"]}})


def atb(cache: Path, iso3: set) -> tuple:
    rows = []
    for edition, (path, year) in ATB_EDITIONS.items():
        with cached(cache, f"atb_{edition}_ATBe.csv", ATB.format(path=path)).open(encoding="utf-8") as f:
            for r in csv.DictReader(f):
                if (r["core_metric_parameter"] in ATB_PARAMETERS and r["core_metric_variable"] == year
                        and r["scenario"] in ("Moderate", "*") and r["core_metric_case"] == "Market"
                        and r["crpyears"] in ("30", "*", "")):
                    rows.append((edition, year, r["technology"], r.get("techdetail", ""),
                                 r.get("techdetail2", ""), r["core_metric_parameter"], r["units"],
                                 f"{float(r['value']):.6g}"))
    n = table(OUT / "atb_electricity.csv", ["atb_edition", "year", "technology", "techdetail", "techdetail2",
                                            "parameter", "units", "value"], rows)
    return ({"open/atb": {"title": "NREL Annual Technology Baseline, electricity, CSV (2020 and 2023 editions)",
                          "url": ATB.format(path="<edition>/ATBe.csv"), "fetched": TODAY}},
            {"open/atb_electricity": {
                "title": "Electricity generation technologies, United States: capital cost (CAPEX, overnight OCC, "
                         "$/kW), fixed O&M ($/kW-yr), variable O&M ($/MWh), heat rate (MMBtu/MWh), capacity factor, "
                         "fuel cost ($/MWh); the 2020 edition's 2019 values (2018 dollars) and the 2023 edition's "
                         "2021 values (2021 dollars), moderate scenario, market case, 30-year cost recovery; NREL ATB",
                "rows": n, "for": ["S2.202", "S2.212"]}})


def typed_energy(cache: Path, iso3: set) -> tuple:
    """The typed tables are committed; this checks they are present and registers them."""
    files = {"typed_pcge2020_plants": ("Projected Costs of Generating Electricity 2020, Tables 3.1-3.10 (pp. 43-55): "
                                       "per plant submitted by country, net capacity (MWe), electrical efficiency (%), "
                                       "capacity factor (%), storage hours, overnight and investment costs at 3, 7 "
                                       "and 10% (2018 USD/kWe); Table 3.1's summary by technology"),
             "typed_pcge2020_lcoe": ("Projected Costs of Generating Electricity 2020, Tables 3.11-3.19 (pp. 56-69, "
                                     "CHP excluded): per plant, the levelised cost's parts (investment, "
                                     "decommissioning, fuel, carbon, O&M, charging) and LCOE at 3, 7 and 10% "
                                     "(2018 USD/MWh); row numbers pair each left-page row with its right-page row"),
             "typed_ene_assumptions": ("Harmonised assumptions of Projected Costs of Generating Electricity 2020 "
                                       "(section 2.2, pp. 36-40: lifetimes, construction lengths, contingency, "
                                       "decommissioning, fuel prices of Table 2.1, carbon price) and the EIA's "
                                       "2019 round-trip efficiencies of US batteries and pumped storage")}
    series = {}
    for name, title in files.items():
        path = OUT / f"{name}.csv"
        if not path.exists():
            raise SystemExit(f"{path} is missing: it is typed from the report, not fetched")
        with path.open() as f:
            n = sum(1 for _ in f) - 1
        series[f"open/{name}"] = {
            "title": title, "rows": n, "for": ["S2.202", "S2.212"],
            "note": "typed: the tables were read from the report's PDF (downloaded "
                    f"{TODAY}) with a table extractor and checked by eye against the page; values as printed, "
                    "footnote marks dropped from country names, the report's 'Autralia' read as Australia, 'na' "
                    "cells left out. IEA/NEA (2020), Projected Costs of Generating Electricity 2020 Edition, OECD "
                    "Publishing, Paris; EIA, Today in Energy, 12 February 2021, 'Utility-scale batteries and pumped "
                    "storage return about 80% of the electricity they store'"}
    return ({"open/pcge2020": {"title": "IEA/NEA, Projected Costs of Generating Electricity 2020 Edition (pdf)",
                               "url": PCGE, "fetched": TODAY},
             "open/eia_storage": {"title": "EIA Today in Energy, round-trip efficiency of storage, 2019",
                                  "url": EIA_STORAGE, "fetched": TODAY}}, series)


def wb_series(cache: Path, iso3: set) -> tuple:
    series = {}
    for code in WB_SERIES:
        page = json.loads(get(WB_API.format(code=code, first=FIRST, last=LAST)))
        title = page[1][0]["indicator"]["value"]
        rows = [(r["countryiso3code"], int(r["date"]), r["value"]) for r in page[1]
                if r["value"] is not None and r["countryiso3code"] in iso3]
        path = RAW / "wb" / f"{code}.csv"
        n = table(path, ["iso3", "year", "value"], rows)
        series[f"wb/{code}"] = {"title": f"{title}, World Bank WDI", "rows": n,
                                "for": ["S2.206", "S2.210"] if code.startswith("EG.") else ["S5.174"]}
    return ({"wb_open": {"title": "World Bank API, WDI (source 2): electricity and tariff series",
                         "url": WB_API.format(code="<series>", first=FIRST, last=LAST), "fetched": TODAY}}, series)


def wits(cache: Path, iso3: set) -> tuple:
    cells: dict = {}
    for year in WITS_YEARS:
        for indicator, column in WITS_INDICATORS.items():
            path = cached(cache, f"wits_{indicator}_{year}.xml", WITS.format(year=year, indicator=indicator))
            for m in re.finditer(r'<Series [^>]*REPORTER="(\w+)"[^>]*PRODUCTCODE="([^"]+)"[^>]*>\s*<Obs '
                                 r'TIME_PERIOD="(\d+)" OBS_VALUE="([^"]+)"', path.read_text(encoding="utf-8-sig")):
                reporter, product, period, value = m.groups()
                reporter = WITS_ISO.get(reporter, reporter)
                if reporter in iso3:
                    cells[(reporter, int(period), product, column)] = f"{float(value):.4g}"
    keys = sorted({k[:3] for k in cells})
    columns = list(WITS_INDICATORS.values())
    n = table(OUT / "wits_tariffs.csv", ["iso3", "year", "product_group"] + columns,
              [list(k) + [cells.get(k + (c,), "") for c in columns] for k in keys])
    return ({"open/wits": {"title": "WITS TradeStats-Tariff API (UNCTAD TRAINS)",
                           "url": WITS.format(year="<year>", indicator="<indicator>"), "fetched": TODAY}},
            {"open/wits_tariffs": {
                "title": "Tariff rates by product group (HS sections 01-05_Animal ... 90-99_Miscellan, UNCTAD stages "
                         "of processing SoP1 raw, SoP2 intermediate, SoP3 consumer, SoP4 capital goods, WITS "
                         f"groups and Total), %, {WITS_YEARS[0]}-{WITS_YEARS[-1]}: effectively applied, weighted and "
                         "simple means, and MFN weighted mean, against the world; WITS/TRAINS",
                "rows": n, "for": ["S5.174"]}})


def icio(cache: Path, iso3: set) -> tuple:
    """Each economy's imports of 2019 by the origin's level and the product's industry, per use; its exports by the
    destination's level and industry. The rest of the world and economies without an income group are
    'unclassified'."""
    level = levels()
    path = cache / ICIO_ZIP
    if not path.exists():
        log("downloading ICIO")
        path.write_bytes(get(ICIO_URL, timeout=3600))
    imports: dict = {}
    exports: dict = {}
    with zipfile.ZipFile(path) as z, z.open(f"{SNAPSHOT}_SML.csv") as f:
        reader = csv.reader(io.TextIOWrapper(f, encoding="utf-8-sig"))
        header = [c.split("_", 1) if "_" in c else (c, None) for c in next(reader)[1:]]
        for r in reader:
            if "_" not in r[0]:
                continue
            origin, industry = r[0].split("_", 1)
            origin_level = level.get(origin, "unclassified") if origin != "ROW" else "unclassified"
            for (user, what), v in zip(header, r[1:]):
                if what is None or user == origin or not v or float(v) == 0.0:
                    continue
                use = FINAL.get(what, "intermediate")
                if user in iso3:
                    key = (user, industry, origin_level)
                    imports.setdefault(key, {}).setdefault(use, 0.0)
                    imports[key][use] += float(v)
                if origin in iso3:
                    user_level = level.get(user, "unclassified") if user != "ROW" else "unclassified"
                    key = (origin, industry, user_level)
                    side = "intermediate" if use == "intermediate" else "final"
                    exports.setdefault(key, {}).setdefault(side, 0.0)
                    exports[key][side] += float(v)
    uses = ["intermediate", "hfce", "npish", "ggfc", "gfcf", "invnt"]
    cell = lambda d, u: f"{d[u]:.3f}" if u in d else ""
    n_imports = table(OUT / f"icio_imports_by_level_{SNAPSHOT}.csv", ["iso3", "industry", "origin_level"] + uses,
                      [list(k) + [cell(d, u) for u in uses] for k, d in imports.items()])
    n_exports = table(OUT / f"icio_exports_by_level_{SNAPSHOT}.csv",
                      ["iso3", "industry", "destination_level", "intermediate", "final"],
                      [list(k) + [cell(d, u) for u in ("intermediate", "final")] for k, d in exports.items()])
    note = ("derived: the OECD ICIO 2023 edition's 2019 table (millions of US dollars, basic prices) with the rows' "
            "origins kept and grouped by the World Bank income group of the origin or destination (high: developed; "
            "upper middle: emerging; lower middle and low: developing; the rest of the world and Chinese Taipei, "
            "which has no group in wb/countries.csv: unclassified), the groups being wb/countries.csv's current ones, "
            "not those of 2019; flows within an economy are left out, so "
            "domestic use is icio/flows less these imports")
    return ({"icio": {"title": "OECD Inter-Country Input-Output tables, 2023 edition", "url": ICIO_URL,
                      "fetched": TODAY}},
            {"open/icio_imports_by_level": {
                "title": f"Imports by economy, product industry (ICIO's 45) and origin's development level, "
                         f"{SNAPSHOT}, millions of US dollars, by use: intermediate (all industries), households "
                         "(with purchases abroad), NPISH, government, fixed investment, inventories; OECD ICIO",
                "rows": n_imports, "for": ["S5.100", "S5.186"], "note": note},
             "open/icio_exports_by_level": {
                 "title": f"Exports by economy, product industry and destination's development level, {SNAPSHOT}, "
                          "millions of US dollars, intermediate and final use; OECD ICIO",
                 "rows": n_exports, "for": ["S5.100", "S5.186"], "note": note}})


def fkrsu(cache: Path, iso3: set) -> tuple:
    import openpyxl
    wb = openpyxl.load_workbook(cached(cache, "fkrsu_2021.xlsx", FKRSU), read_only=True, data_only=True)
    rows = list(wb["DATASET"].iter_rows(values_only=True))
    header = list(rows[0])
    keep = [h for h in header[7:] if h]
    out = []
    for r in rows[1:]:
        d = dict(zip(header, r))
        if d["code_wdi"] in iso3 and isinstance(d["year"], (int, float)) and FIRST <= int(d["year"]) <= LAST:
            out.append([d["code_wdi"], int(d["year"])] +
                       [("" if not isinstance(d[h], (int, float)) else f"{d[h]:.4g}") for h in keep])
    n = table(OUT / "capital_controls.csv", ["iso3", "year"] + keep, out)
    return ({"open/fkrsu": {"title": "Fernández, Klein, Rebucci, Schindler and Uribe, Capital Control Measures, "
                                     "2021 update (12 August 2021)", "url": FKRSU, "fetched": TODAY}},
            {"open/capital_controls": {
                "title": "Capital control measures, 0 (none) to 1 (restricted), by economy and year: overall (ka), "
                         "inflows (kai), outflows (kao) and by asset category - equity (eq), bonds (bo), money market "
                         "(mm), collective investment (ci), derivatives (de), commercial credit (cc), financial "
                         "credit (fc), guarantees (gs), direct investment (di, ldi liquidation), real estate (re) - "
                         "and transaction (plbn purchase locally by non-residents, siln sale or issue locally by "
                         "non-residents, pabr purchase abroad by residents, siar sale or issue abroad by residents); "
                         "Fernández et al. (2016), IMF Economic Review 64(3), 2021 update",
                "rows": n, "for": ["S5.174"]}})


def stata(data: bytes) -> list:
    """A Stata 117/118 file's rows of fixed-width variables as dicts, a missing value as None."""
    release = int(data[28:31])
    order = "<" if data[data.index(b"<byteorder>") + 11:][:3] == b"LSF" else ">"
    k = struct.unpack_from(order + "H", data, data.index(b"<K>") + 3)[0]
    n = struct.unpack_from(order + ("Q" if release >= 118 else "I"), data, data.index(b"<N>") + 3)[0]
    offsets = struct.unpack_from(order + "14Q", data, data.index(b"<map>") + 5)
    types = struct.unpack_from(order + f"{k}H", data, offsets[2] + len(b"<variable_types>"))
    width = 129 if release >= 118 else 33
    base = offsets[3] + len(b"<varnames>")
    names = [data[base + i * width:base + (i + 1) * width].split(b"\0")[0].decode() for i in range(k)]
    # Each numeric type's struct code, width and the largest value that is not one of Stata's missing codes.
    numeric = {65526: ("d", 8, 8.988465674311579e307), 65527: ("f", 4, 1.701e38), 65528: ("l", 4, 2147483620),
               65529: ("h", 2, 32740), 65530: ("b", 1, 100)}
    sizes = [t if t <= 2045 else numeric[t][1] for t in types]
    at = offsets[9] + len(b"<data>")
    rows = []
    for _ in range(n):
        row = {}
        for name, t, s in zip(names, types, sizes):
            raw = data[at:at + s]
            at += s
            if t <= 2045:
                row[name] = raw.split(b"\0")[0].decode("utf-8")
            else:
                code, _, top = numeric[t]
                v = struct.unpack(order + code, raw)[0]
                row[name] = None if v > top else v
        rows.append(row)
    return rows


def boz(cache: Path, iso3: set) -> tuple:
    z = zipfile.ZipFile(cached(cache, "boz_section2.zip", BOZ))
    rows = stata(z.read("Data/all_countries_update.dta"))
    columns = ["USDExport", "EURExport", "HomeExport", "OtherExport", "UnclassifiedExport", "USDImport", "EURImport",
               "HomeImport", "OtherImport", "UnclassifiedImport"]
    out = [[r["ISO3C"], int(r["year"])] + ["" if r[c] is None else f"{r[c]:.4g}" for c in columns]
           for r in rows if r["ISO3C"] in iso3 and r["year"] and 2010 <= r["year"] <= LAST
           and any(r[c] is not None for c in columns)]
    n = table(OUT / "invoicing_currency.csv", ["iso3", "year"] + [c.lower() for c in columns], out)
    return ({"open/boz": {"title": "Boz, Casas, Georgiadis, Gopinath, Le Mezo, Mehl and Nguyen (2022), Patterns of "
                                   "invoicing currency in global trade: new evidence, J. Int. Econ. 136; Mendeley "
                                   "Data 10.17632/6z6w78968m.1, Section2.zip", "url": BOZ, "fetched": TODAY}},
            {"open/invoicing_currency": {
                "title": "Shares of exports and imports invoiced in US dollars, euros, the home currency, other "
                         "currencies and unclassified (%), by economy and year 2010-2019; Boz et al. (2022)",
                "rows": n, "for": ["S5.100", "S5.186"]}})


def oecd_sti(cache: Path, name: str, flow: str, key: str, first: int, last: int) -> list:
    endpoint = "sti-public" if flow.startswith("DSD_TIVA") else "public"
    path = cached(cache, f"{name}.csv", OECD_STI.format(endpoint=endpoint, flow=flow, key=key, first=first,
                                                         last=last))
    with path.open(encoding="utf-8-sig") as f:
        return list(csv.DictReader(f))


def mining(cache: Path, iso3: set) -> tuple:
    rows = []
    for r in oecd_sti(cache, "tiva_mining", "DSD_TIVA_MAINLV@DF_MAINLV,1.1",
                      f"PROD+VALU+EXGR+IMGR..{MINING}.W..A", SNAPSHOT - 1, SNAPSHOT + 1):
        if r["REF_AREA"] in iso3 and r["OBS_VALUE"]:
            rows.append((r["REF_AREA"], int(r["TIME_PERIOD"]), r["ACTIVITY"], r["MEASURE"], r["UNIT_MEASURE"],
                         r["OBS_VALUE"], r["UNIT_MULT"]))
    for r in oecd_sti(cache, "tim_mining", "DSD_TIM_2025@DF_TIM_2025,", f"EMPN+LABR..{MINING}.W..A",
                      SNAPSHOT - 1, SNAPSHOT + 1):
        if r["REF_AREA"] in iso3 and r["OBS_VALUE"]:
            rows.append((r["REF_AREA"], int(r["TIME_PERIOD"]), r["ACTIVITY"], r["MEASURE"], r["UNIT_MEASURE"],
                         r["OBS_VALUE"], r["UNIT_MULT"]))
    n = table(OUT / "mining_divisions.csv", ["iso3", "year", "activity", "measure", "unit", "value", "unit_mult"], rows)
    return ({"open/oecd_tiva_tim_2025": {"title": "OECD Trade in Value Added and Trade in Employment, 2025 editions "
                                                  "(from the ICIO 2025 tables), SDMX API",
                                         "url": OECD_STI.format(endpoint="<sti-public|public>", flow="<flow>", key="<key>",
                                                                first="<y>", last="<y>"),
                                         "fetched": TODAY}},
            {"open/mining_divisions": {
                "title": f"Mining divisions B05 (coal and lignite), B06 (crude petroleum and natural gas), B07 (metal "
                         f"ores), B08 (other mining and quarrying), B09 (support services), {SNAPSHOT - 1}-"
                         f"{SNAPSHOT + 1}: output (PROD), value added (VALU), gross exports (EXGR) and imports (IMGR), "
                         "US dollars (TiVA 2025), employment (EMPN, persons) and labour compensation (LABR, US "
                         "dollars, and PT_VA as a percentage of value added) (TiM 2025); value times 10^unit_mult; "
                         "OECD",
                "rows": n, "for": ["S1.02", "S1.436"]}})


def seven_zip(data: bytes) -> dict:
    """A 7z archive's files, {name: bytes}, for archives of one LZMA- or LZMA2-coded folder per file."""
    if data[:6] != b"7z\xbc\xaf\x27\x1c":
        raise SystemExit("not a 7z archive")
    offset, size = struct.unpack_from("<QQ", data, 12)

    class Reader:
        def __init__(self, b: bytes):
            self.b, self.i = b, 0

        def byte(self) -> int:
            self.i += 1
            return self.b[self.i - 1]

        def take(self, n: int) -> bytes:
            self.i += n
            return self.b[self.i - n:self.i]

        def num(self) -> int:
            first, mask, value = self.byte(), 0x80, 0
            for k in range(8):
                if not first & mask:
                    return value | ((first & (mask - 1)) << (8 * k))
                value |= self.byte() << (8 * k)
                mask >>= 1
            return value

    def digests(r: Reader, n: int) -> None:
        if r.byte() == 0:
            n = sum(bin(b).count("1") for b in r.take((n + 7) // 8))
        r.take(4 * n)

    def streams(r: Reader) -> dict:
        info = {"pack_pos": 0, "pack_sizes": [], "folders": [], "unpack": []}
        while (t := r.byte()) != 0x00:
            if t == 0x06:
                info["pack_pos"], n = r.num(), r.num()
                while (u := r.byte()) != 0x00:
                    if u == 0x09:
                        info["pack_sizes"] = [r.num() for _ in range(n)]
                    elif u == 0x0A:
                        digests(r, n)
            elif t == 0x07:
                r.byte()
                folders = r.num()
                r.byte()
                for _ in range(folders):
                    coders = []
                    for _ in range(r.num()):
                        flag = r.byte()
                        cid = r.take(flag & 0x0F)
                        outs = 1
                        if flag & 0x10:
                            r.num()
                            outs = r.num()
                        props = r.take(r.num()) if flag & 0x20 else b""
                        coders.append((cid, props, outs))
                    for _ in range(sum(c[2] for c in coders) - 1):
                        r.num(), r.num()
                    info["folders"].append(coders)
                r.byte()
                info["unpack"] = [[r.num() for _ in range(sum(c[2] for c in coders))] for coders in info["folders"]]
                while (u := r.byte()) != 0x00:
                    if u == 0x0A:
                        digests(r, folders)
            elif t == 0x08:
                while (u := r.byte()) != 0x00:
                    if u == 0x0D and any(r.num() != 1 for _ in info["folders"]):
                        raise SystemExit("7z: more than one file per folder")
                    elif u == 0x0A:
                        digests(r, len(info["folders"]))
        return info

    def unpack(info: dict, k: int) -> bytes:
        start = 32 + info["pack_pos"] + sum(info["pack_sizes"][:k])
        packed = data[start:start + info["pack_sizes"][k]]
        (cid, props, _), = info["folders"][k]
        method = {b"\x03\x01\x01": lzma.FILTER_LZMA1, b"\x21": lzma.FILTER_LZMA2}[cid]
        filters = [lzma._decode_filter_properties(method, props)]
        out = lzma.LZMADecompressor(lzma.FORMAT_RAW, filters=filters).decompress(packed, info["unpack"][k][0])
        if len(out) != info["unpack"][k][0]:
            raise SystemExit("7z: short stream")
        return out

    r = Reader(data[32 + offset:32 + offset + size])
    if r.byte() == 0x17:
        r = Reader(unpack(streams(r), 0))
        r.byte()
    info, names = None, []
    while (t := r.byte()) != 0x00:
        if t == 0x04:
            info = streams(r)
        elif t == 0x05:
            count = r.num()
            while (p := r.byte()) != 0x00:
                body = r.take(r.num())
                if p == 0x11:
                    names = body[1:].decode("utf-16-le").split("\x00")[:count]
    return {name: unpack(info, k) for k, name in enumerate(names)}


def unctad(cache: Path, code: str) -> list:
    path = cached(cache, f"{code}.7z", UNCTAD.format(code=code, file=code.replace(".", "_")))
    (name, body), = seven_zip(path.read_bytes()).items()
    return list(csv.DictReader(io.StringIO(body.decode("utf-8-sig"))))


def maritime(cache: Path, iso3: set) -> tuple:
    numeric = {}
    for a in json.loads(cached(cache, "comtrade_partners.json", M49).read_text(encoding="utf-8-sig"))["results"]:
        if a.get("PartnerCodeIsoAlpha3") in iso3:
            numeric[int(a["id"])] = a["PartnerCodeIsoAlpha3"]
    area = lambda code: None if not code.isdigit() else "WLD" if int(code) == 0 else numeric.get(int(code))
    number = lambda v: v.replace(",", "") if v not in ("", None) else ""

    port = [r for r in unctad(cache, "US.PortCalls") if area(r["Economy"]) and int(r["Year"]) <= SNAPSHOT + 2]
    measures = ["Median time in port (days)", "Average age of vessels (years)", "Average size (GT) of vessels",
                "Average cargo carrying capacity (dwt) per vessel",
                "Average container carrying capacity (TEU) per container ship"]
    calls = {(r["Year"], r["Economy"], r["CommercialMarket"]): r["Number of port calls"]
             for r in unctad(cache, "US.PortCallsArrivals")}
    rows = [(area(r["Economy"]), int(r["Year"]), r["CommercialMarket Label"],
             number(calls.get((r["Year"], r["Economy"], r["CommercialMarket"]), "")))
            + tuple(number(r[m]) for m in measures) for r in port]
    n_port = table(OUT / "port_calls.csv", ["iso3", "year", "market", "port_calls", "median_days_in_port",
                                            "avg_vessel_age_years", "avg_size_gt", "avg_dwt", "avg_teu"], rows)

    fleet = [(area(r["Economy"]), int(r["Year"]), r["ShipType Label"], number(r["Number of ships"]),
              number(r["Dead weight tons in thousands"]), number(r["Gross Tonnage in thousands"]),
              number(r["Average age of vessels (years)"]))
             for r in unctad(cache, "US.MerchantFleet")
             if area(r["Economy"]) and SNAPSHOT - 1 <= int(r["Year"]) <= SNAPSHOT + 1]
    n_fleet = table(OUT / "merchant_fleet.csv", ["iso3", "year", "ship_type", "ships", "dwt_thousands",
                                                 "gt_thousands", "avg_age_years"], fleet)

    built = {(area(r["Economy"]), int(r["Year"])): number(r["Gross Tonnage"]) for r in unctad(cache, "US.ShipBuilding")
             if area(r["Economy"]) and FIRST <= int(r["Year"]) <= LAST}
    output = {}
    for r in oecd_sti(cache, "tiva_shipbuilding", "DSD_TIVA_MAINLV@DF_MAINLV,1.1", "PROD..C301.W..A", FIRST, 2022):
        if r["REF_AREA"] in iso3 and r["OBS_VALUE"]:
            output[(r["REF_AREA"], int(r["TIME_PERIOD"]))] = float(r["OBS_VALUE"]) * 10 ** int(r["UNIT_MULT"])
    keys = sorted(set(built) | set(output))
    rows = [(e, y, built.get((e, y), ""), f"{output[(e, y)]:.6g}" if (e, y) in output else "",
             f"{output[(e, y)] / float(built[(e, y)]):.1f}" if (e, y) in output and built.get((e, y))
             and float(built[(e, y)]) > 0 else "") for e, y in keys]
    n_built = table(OUT / "shipbuilding.csv", ["iso3", "year", "gt_built", "c301_output_usd", "usd_per_gt_built"],
                    rows)

    series = {
        "open/port_calls": {"title": "Port calls by economy and market segment, 2018-2021: number of calls, median "
                                     "time in port (days), average vessel age (years), size (GT), cargo capacity "
                                     "(dwt), container capacity (TEU); UNCTADstat (from AIS)",
                            "rows": n_port, "for": ["S1.07", "S5.174"]},
        "open/merchant_fleet": {"title": f"Merchant fleet by flag of registration and ship type, {SNAPSHOT - 1}-"
                                         f"{SNAPSHOT + 1}: ships, deadweight and gross tonnage (thousands), average "
                                         "age (years); WLD the world; UNCTADstat",
                                "rows": n_fleet, "for": ["S1.07", "S5.05"]},
        "open/shipbuilding": {"title": f"Ships built by economy, {FIRST}-2022: gross tonnage delivered (UNCTADstat), "
                                       "output of ISIC C301 building of ships and boats (US dollars, OECD TiVA 2025) "
                                       "and their ratio, US dollars of output per gross ton built",
                              "rows": n_built, "for": ["S1.07"],
                              "note": "derived: usd_per_gt_built = c301_output_usd / gt_built, the value of a gross "
                                      "ton of new ships where the industry's output is mostly ships delivered; its "
                                      "output also counts boats, naval vessels and work in progress, so the ratio is "
                                      "read for the large builders (China, Korea, Japan), whose deliveries dominate"},
        "open/typed_imo4ghg_fleet_2018": {
            "title": "World fleet 2018 by ship type and size (international, domestic and fishing): ships by IHS "
                     "match type, average deadweight, main engine power (kW), design speed, days at sea, days on "
                     "international voyages, days in SECAs, speed at sea (kn), distance (nm), median AER, fuel "
                     "(kt: main, auxiliary, boiler), GHG and CO2 (Mt); Fourth IMO GHG Study 2020, Table 35",
            "rows": sum(1 for _ in (OUT / "typed_imo4ghg_fleet_2018.csv").open()) - 1, "for": ["S1.07", "S1.465"],
            "note": "typed: Table 35 (pp. 99-101 of the report, PDF pp. 127-129) read with a table extractor from the "
                    "full report and annexes PDF downloaded from GreenVoyage2050 and checked by eye; IMO (2020), "
                    "Fourth IMO Greenhouse Gas Study 2020, London"},
        "open/typed_marad2011_opcosts": {
            "title": "Daily operating costs of oceangoing vessels, US and foreign flag, 2009 and 2010, US dollars per "
                     "day: wages, stores and lubricants, maintenance and repair, insurance, overhead, total; by type "
                     "(containership, Ro/Ro, bulk carrier, all types); MARAD (2011), Appendix B",
            "rows": sum(1 for _ in (OUT / "typed_marad2011_opcosts.csv").open()) - 1, "for": ["S1.07", "S5.05"],
            "note": "typed: Appendix B tables (PDF pp. 17-18) of US Maritime Administration (2011), Comparison of "
                    "U.S. and Foreign-Flag Operating Costs, September 2011, read from the PDF on ROSA P and "
                    "checked: the five categories sum to the total within a dollar"},
    }
    for name in ("typed_imo4ghg_fleet_2018", "typed_marad2011_opcosts"):
        if not (OUT / f"{name}.csv").exists():
            raise SystemExit(f"{name}.csv is missing: it is typed from the report, not fetched")
    return ({"open/unctadstat": {"title": "UNCTADstat bulk downloads (7z): US.PortCalls, US.PortCallsArrivals, "
                                          "US.MerchantFleet, US.ShipBuilding",
                                 "url": UNCTAD.format(code="<dataset>", file="<file>"), "fetched": TODAY},
             "open/comtrade_areas": {"title": "UN Comtrade partner areas (M49 numeric to ISO alpha-3)", "url": M49,
                                     "fetched": TODAY},
             "open/imo4ghg": {"title": "IMO, Fourth IMO GHG Study 2020, full report and annexes (pdf)", "url": IMO,
                              "fetched": TODAY},
             "open/marad2011": {"title": "MARAD, Comparison of U.S. and Foreign-Flag Operating Costs (2011, pdf)",
                                "url": MARAD, "fetched": TODAY}}, series)


SOURCES = {"ember": ember, "gppd": gppd, "atb": atb, "typed_energy": typed_energy, "wb": wb_series, "wits": wits,
           "icio": icio, "fkrsu": fkrsu, "boz": boz, "mining": mining, "maritime": maritime}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cache", type=Path, default=None)
    parser.add_argument("--only", nargs="*", choices=sorted(SOURCES))
    args = parser.parse_args()
    cache = args.cache or Path(tempfile.mkdtemp())
    cache.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    iso3 = countries()
    for name in args.only or list(SOURCES):
        sources, series = SOURCES[name](cache, iso3)
        merge_manifest(sources, series)
        for key, s in series.items():
            log(f"{key}: {s['rows']} rows")


if __name__ == "__main__":
    main()
