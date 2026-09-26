#!/usr/bin/env python3
"""Derives the kinds of plant and each country group's stock of each from the fetched sources in data/sources/raw/.

The six kinds are TEC.capital's rows: structures, transport equipment, ICT equipment, other machinery and equipment,
cultivated biological resources and intellectual property products. For each kind:

- its geometric depreciation rate is the BEA's own: 2024's current-cost depreciation of the kind's private
  nonresidential assets over their mean net stock of 2023 and 2024, which covers the assets the BEA's table gives no
  service life (computers, autos, R&D, originals);
- its declining-balance rate is the BEA table's, each asset row weighted by its 2024 investment, over the rows that
  have one; its mean service life is that rate over the geometric rate, as the BEA divides a declining-balance rate by
  a service life to reach a geometric rate;
- cultivated assets, which the BEA does not capitalise, take the median of the geometric rates of the BEA working paper
  that tracks them and the median of the Australian Bureau of Statistics' mean lives of livestock and plantings;
- its efficiency shape is the hyperbolic age-efficiency parameter of the BLS (0.75 structures, 0.5 equipment) and of
  the ABS (0.5 software and livestock);
- its lead time is measured where a source measures it: a structure's months from start to completion (Census), an
  equipment order's months of shipments on the books (Census M3), none for intellectual property and cultivated
  assets;
- it is bought as the product that makes it.

Each group's stock of each kind per unit of GDP is the net stock of the sections the firms work in over all gross
value added (OECD Table 9A over Table 6), the median over the group's reporting economies.

    python3 tools/data/derive_cap.py

Writes data/shared/CAP_kinds.toml and data/profiles/<level>/CAP.toml.
"""
import csv
import json
import statistics
from pathlib import Path

import numpy as np
import pandas as pd

from derive import LEVELS, RAW

ROOT = Path(__file__).resolve().parents[2]
SHARED = ROOT / "data" / "shared"
PROFILES = ROOT / "data" / "profiles"
YEAR = 2022
WEIGHT_YEAR = "2024"
MIN_COUNTRIES = 10
DAYS_A_MONTH = 365.25 / 12
KINDS = ["structures", "transport equipment", "ICT equipment", "other machinery and equipment",
         "cultivated biological resources", "intellectual property products"]
# The OECD's asset codes of the kinds, in their order.
ASSETS = ["N112N", "N1131N", "N1132N", "N11ON", "N115N", "N117N"]
# The sections the firms work in: finance, real estate, public administration, households as employers and
# extraterritorial bodies are carried by their own systems.
FIRM_SECTIONS = list("ABCDEFGHIJMNPQRS")
# The product each kind is bought as, by its place in TEC.products: construction, capital goods, crops and livestock,
# business services.
PRODUCTS = {0: 11, 1: 9, 2: 9, 3: 9, 4: 0, 5: 15}
# BLS: "BLS assumes β = 0.75 for structures and β = 0.5 for equipment" (Handbook of Methods, productivity measures,
# concepts); ABS: "For computer software, b is set to 0.5. For livestock, b is also set to 0.5" (Australian System of
# National Accounts: Concepts, Sources and Methods 2020-21, para 14.37-14.38).
BETA = [0.75, 0.5, 0.5, 0.5, 0.5, 0.5]
# Cultivated assets: the geometric rates of Soloveichik, "Tracking Cultivated Assets in Measures of Capital", BEA
# Working Paper WP2021-6, section 4; the mean lives of the ABS's Concepts, Sources and Methods 2020-21, Table 14.4.
CULTIVATED_RATES = {"dairy cows": .11, "beef cows": .05, "bulls": .16, "sheep and goats": .10, "honeybees": .53,
                    "horses": .05, "fruit and nut trees": .05, "alfalfa pastures": .15, "grass pastures": .10}
CULTIVATED_LIVES = {"sheep (wool)": 6.4, "dairy": 10.3, "breeding cattle": 7.5, "thoroughbred horses": 10.3,
                    "standardbred horses": 10.3, "other horses": 10.3, "pigs for breeding": 8.5, "orchards": 29.5,
                    "plantations": 7.5, "grapevines": 40.6}
# The Census Bureau's table cell of every private nonresidential project, of every type and value.
ALL_PROJECTS = ("All projects", "All Values")
# Equipment's backlog is read over the last twelve months the survey reports.
BACKLOG_MONTHS = 12

# The BEA's table "BEA Rates of Depreciation, Service Lives, Declining-Balance Rates, and Hulten-Wykoff Categories",
# private nonresidential rows, transcribed: (kind, BEA asset code, industries or ANY, row, geometric rate, service
# life, declining-balance rate); None where the table gives none. Current vintages only.
MANUF = ["3210", "3270", "3310", "3320", "3330", "3340", "3350", "336M", "336O", "3370", "3380", "3390",
         "3110", "3120", "313T", "315A", "3220", "3230", "3240", "3250", "3260"]
# Service lives of the three industry-specific machinery rows, same order as MANUF (table p.8-9).
MANUF_LIFE = [12, 19, 27, 24, 25, 14, 14, 14, 17, 14, 14, 17, 20, 21, 16, 15, 16, 15, 22, 16, 14]
EI30_RATE = [.1633, .1032, .0726, .0817, .0784, .14, .14, .14, .1153, .14, .14, .1153, .098, .0933, .1225, .1307,
             .1225, .1307, .0891, .1225, .14]
EI40_RATE = [.1375, .0868, .0611, .0688, .066, .1179, .1179, .1179, .0971, .1179, .1179, .0971, .0825, .0786,
             .1031, .11, .1031, .11, .075, .1031, .1179]
EI50_RATE = [.1429, .0903, .0635, .0715, .0686, .1225, .1225, .1225, .1009, .1225, .1225, .1009, .0858, .0817,
             .1072, .1143, .1072, .1143, .078, .1072, .1225]
TRADE = ["4210", "4220", "4410", "4450", "4520", "442R"]
AIRCRAFT_LONG = ["4810", "5221", "5223", "5229", "5243", "5500", "5250", "5320"]
ANY = None  # all industries not claimed by a more specific row of the same asset

# (kind, BEA asset code, industries or ANY, table row label, rate, life, declining-balance rate)
# None = the table shows "……" (no value). Current-vintage rows only (e.g. "1992 and later", "1960 and later").
BEA_ROWS = []
def row(kind, asset, inds, label, rate, life, db):
    BEA_ROWS.append(dict(kind=kind, asset=asset, inds=inds, label=label, rate=rate, life=life, db=db))

# ICT equipment (SNA N1132: computer hardware and telecommunications equipment)
for a, n in [("EP1A", "Mainframes"), ("EP1B", "PCs"), ("EP1C", "DASDs"), ("EP1D", "Printers"), ("EP1E", "Terminals"),
             ("EP1F", "Tape drives"), ("EP1G", "Storage devices"), ("EP1H", "System integrators")]:
    row(2, a, ANY, f"Computers and peripheral equipment ({n}) /2/", None, None, None)
row(2, "EP20", ["5320", "5415"], "Communications equipment: rental and leasing and computer systems design", .15, 11, 1.65)
row(2, "EP20", ANY, "Communications equipment: other industries", .11, 15, 1.65)
# Other machinery and equipment (SNA N11O less weapons; no weapons in private assets)
row(3, "EP36", ANY, "Nonmedical instruments", .135, 12, 1.6203)
row(3, "EP34", ANY, "Medical instruments", .135, 12, 1.6203)
row(3, "EP35", ANY, "Electromedical equipment", .1834, 9, 1.65)
row(3, "EP31", ANY, "Photocopy and related equipment", .18, 9, 1.6203)
row(3, "EP12", ANY, "Office and accounting equipment, 1978 and later", .3119, 7, 2.1832)
row(3, "EI11", ANY, "Nuclear fuel /7/ (straight-line, Winfrey)", None, 4, None)
row(3, "EI12", ANY, "Other fabricated metal products", .0917, 18, 1.65)
row(3, "EI21", ANY, "Steam engines and turbines", .0516, 32, 1.65)
row(3, "EI22", ANY, "Internal combustion engines", .2063, 8, 1.65)
for asset, rates, db, name in [("EI30", EI30_RATE, 1.96, "Metalworking machinery"),
                               ("EI40", EI40_RATE, 1.65, "Special industry machinery, nec"),
                               ("EI50", EI50_RATE, 1.715, "General industrial, incl. materials handling")]:
    for ind, life, rate in zip(MANUF, MANUF_LIFE, rates):
        row(3, asset, [ind], f"{name}: manufacturing {ind}", rate, life, db)
    row(3, asset, ANY, f"{name}: nonmanufacturing industries",
        {"EI30": .1225, "EI40": .1031, "EI50": .1072}[asset], 16, db)
row(3, "EI60", ANY, "Electrical transmission, distribution, and industrial apparatus", .05, 33, 1.65)
row(3, "EO11", ANY, "Household furniture", .1375, 12, 1.65)
row(3, "EO12", ANY, "Other furniture", .1179, 14, 1.65)
row(3, "EO21", ANY, "Farm tractors", .1452, 9, 1.3064)
row(3, "EO30", ANY, "Agricultural machinery, except tractors", .1179, 14, 1.65)
row(3, "EO22", ANY, "Construction tractors", .1633, 8, 1.3064)
row(3, "EO40", ANY, "Construction machinery, except tractors", .155, 10, 1.55)
row(3, "EO50", ANY, "Mining and oil field machinery", .15, 11, 1.65)
row(3, "EO60", TRADE, "Service industry machinery: wholesale and retail trade", .165, 10, 1.65)
row(3, "EO60", ANY, "Service industry machinery: other industries", .15, 11, 1.65)
row(3, "EO71", ANY, "Household appliances", .165, 10, 1.65)
row(3, "EO72", ANY, "Miscellaneous electrical equipment", .1834, 9, 1.65)
row(3, "EO80", ANY, "Other", .1473, 11, 1.6203)
# Transport equipment (SNA N1131)
row(1, "ET11", ANY, "Light trucks, 1992 and later", .1925, 17, 3.2725)
row(1, "ET12", ["4850"], "Other trucks, buses, trailers: transit and ground passenger", .1232, 14, 1.7252)
row(1, "ET12", ["4840"], "Other trucks, buses, trailers: trucking and other services", .1725, 10, 1.7252)
row(1, "ET12", ANY, "Other trucks, buses, trailers: other industries", .1917, 9, 1.7252)
row(1, "ET20", ANY, "Autos /12/ (from used-auto prices)", None, None, None)
row(1, "ET30", AIRCRAFT_LONG, "Aircraft 1960+: air transportation, finance, holding cos., rental and leasing", .066, 25, 1.65)
row(1, "ET30", ANY, "Aircraft 1960+: other industries", .11, 15, 1.65)
row(1, "ET40", ANY, "Ships and boats", .0611, 27, 1.65)
row(1, "ET50", ANY, "Railroad equipment", .0589, 28, 1.65)
# Structures (SNA N112 other buildings and structures; BEA includes mining exploration, shafts and wells)
for a, label, rate, life, db in [
        ("SOO1", "Office buildings", .0247, 36, .8892), ("SOO2", "Medical buildings", .0247, 36, .8892),
        ("SC01", "Commercial warehouses", .0222, 40, .8892), ("SC02", "Other commercial buildings", .0262, 34, .8992),
        ("SC03", "Multimerchandise shopping", .0262, 34, .8992),
        ("SC04", "Food and beverage establishments", .0262, 34, .8992),
        ("SOMO", "Mobile offices", .0556, 16, .8892), ("SB31", "Hospitals", .0188, 48, .9024),
        ("SB32", "Special care", .0188, 48, .9024), ("SI00", "Manufacturing", .0314, 31, .9747),
        ("SU30", "Electric light and power, 1946 and later", .0211, 45, .948), ("SU40", "Gas", .0237, 40, .948),
        ("SU50", "Petroleum pipelines", .0237, 40, .948), ("SU60", "Wind and solar", .0303, 30, .909),
        ("SU20", "Communication", .0237, 40, .948), ("SU12", "Railroad replacement track", .0249, 38, .948),
        ("SU11", "Other railroad structures", .0176, 54, .948),
        ("SM01", "Mining exploration, shafts, wells: petroleum and natural gas, 1973 and later", .0751, 12, .9008),
        ("SM02", "Mining exploration, shafts, wells: other", .045, 20, .9008),
        ("SB10", "Religious buildings", .0188, 48, .9024), ("SB20", "Educational buildings", .0188, 48, .9024),
        ("SB41", "Lodging", .0281, 32, .899), ("SB42", "Amusement and recreational buildings", .03, 30, .899),
        ("SN00", "Farm", .0239, 38, .91), ("SB44", "Local transit", .0237, 38, .899),
        ("SB43", "Air transportation", .0237, 38, .899), ("SB45", "Other transportation", .0237, 38, .899),
        ("SB46", "Other land transportation", .0237, 38, .899), ("SO01", "Water supply", .0225, 40, .899),
        ("SO02", "Sewage and waste disposal", .0225, 40, .899), ("SO03", "Public safety", .0237, 38, .899),
        ("SO04", "Highway and conservation and development", .0225, 40, .899)]:
    row(0, a, ANY, label, rate, life, db)
# Intellectual property products (SNA N117)
row(5, "ENS1", ANY, "Software: prepackaged", .55, 3, 1.65)
row(5, "ENS2", ANY, "Software: custom", .33, 5, 1.65)
row(5, "ENS3", ANY, "Software: own-account", .33, 5, 1.65)
for a, label, rate in [("RD11", "R&D: pharmaceutical and medicine mfg", .10), ("RD12", "R&D: chemical mfg ex. pharma", .16),
                       ("RD23", "R&D: semiconductor and other component mfg", .25),
                       ("RD25", "R&D: other computer and electronic product mfg, nec", .40),
                       ("RD21", "R&D: computers and peripheral equipment mfg", .40),
                       ("RD22", "R&D: communications equipment mfg", .27),
                       ("RD24", "R&D: navigational, measuring, electromedical, control instruments", .29),
                       ("RD31", "R&D: motor vehicles, bodies, trailers, parts mfg", .31),
                       ("RD32", "R&D: aerospace products and parts mfg", .22), ("RDOM", "R&D: other manufacturing", .16),
                       ("RD70", "R&D: scientific R&D services", .16), ("RD40", "R&D: software publishers", .22),
                       ("RD50", "R&D: financial and real estate services", .16),
                       ("RD60", "R&D: computer systems design", .36), ("RD80", "R&D: all other nonmanufacturing, nec", .16),
                       ("RD91", "R&D: universities and colleges", .16), ("RD92", "R&D: other nonprofit institutions", .16),
                       ("AE10", "Originals: theatrical movies", .093), ("AE20", "Originals: long-lived TV programs", .168),
                       ("AE30", "Originals: books", .121), ("AE40", "Originals: music", .267),
                       ("AE50", "Originals: other", .109)]:
    row(5, a, ANY, label, rate, None, None)



def manifest() -> dict:
    return json.loads((RAW / "manifest.json").read_text())


def bea(name: str) -> dict:
    """A BEA series by (industry, asset) and year."""
    out: dict = {}
    for r in csv.DictReader((RAW / "cap" / f"bea_{name}.csv").open()):
        out.setdefault((r["industry"], r["asset"]), {})[r["year"]] = float(r["value"])
    return out


def weights(inv: dict) -> None:
    """Each table row's 2024 investment, summed over the industries it covers."""
    claimed: dict = {}
    for r in BEA_ROWS:
        if r["inds"] is not ANY:
            claimed.setdefault(r["asset"], set()).update(r["inds"])
    for r in BEA_ROWS:
        keys = [(i, a) for (i, a) in inv if a == r["asset"]
                and (i in r["inds"] if r["inds"] is not ANY else i not in claimed.get(a, set()))]
        r["weight"] = sum(inv[k].get(WEIGHT_YEAR, 0.0) for k in keys)


def kinds() -> list:
    """Each kind's geometric rate, declining-balance rate and service life."""
    inv, stock, dep = bea("investment"), bea("net_stock"), bea("depreciation")
    weights(inv)
    detail = {a for (_, a) in inv} - {"EQ00", "ST00", "IP00"}
    mapped = {r["asset"] for r in BEA_ROWS}
    if detail != mapped:
        raise SystemExit(f"assets mapped to no kind or to none the BEA reports: {detail ^ mapped}")
    out = []
    for k in range(len(KINDS)):
        rows = [r for r in BEA_ROWS if r["kind"] == k]
        if not rows:
            rate = statistics.median(CULTIVATED_RATES.values())
            life = statistics.median(CULTIVATED_LIVES.values())
            out.append(dict(rate=rate, life=life, db=rate * life, weighted=0.0))
            continue
        assets = {r["asset"] for r in rows}
        depreciation = sum(v["2024"] for (_, a), v in dep.items() if a in assets)
        mean_stock = sum((v["2023"] + v["2024"]) / 2 for (_, a), v in stock.items() if a in assets)
        with_db = [r for r in rows if r["db"] is not None]
        db = sum(r["db"] * r["weight"] for r in with_db) / sum(r["weight"] for r in with_db)
        rate = depreciation / mean_stock
        out.append(dict(rate=rate, life=db / rate, db=db,
                        weighted=sum(r["weight"] for r in with_db) / sum(r["weight"] for r in rows)))
    return out


def lead_days() -> list:
    """Days from order to service: a structure's months from start to completion, and an equipment order's months of
    shipments on the books, each in days."""
    months = {(r["type"], r["projects"]): float(r["months"])
              for r in csv.DictReader((RAW / "cap" / "construction_months.csv").open())}
    series = lambda s: {r["month"]: float(r["value"]) for r in csv.DictReader((RAW / "cap" / f"{s}.csv").open())}
    orders, shipped = series("ANXAUO"), series("ANXAVS")
    last = sorted(set(orders) & set(shipped))[-BACKLOG_MONTHS:]
    backlog = sum(orders[m] / shipped[m] for m in last) / len(last)
    structures = months[ALL_PROJECTS]
    return [round(structures * DAYS_A_MONTH)] + [round(backlog * DAYS_A_MONTH)] * 3, (structures, backlog, last)


def num(x: float, places: int = 6) -> str:
    return f"{x:.{places}f}"


def table1(values: list, places: int = 6, axis=None) -> str:
    axis = axis if axis is not None else list(range(len(values)))
    return (f"{{ axis = [{', '.join(str(a) for a in axis)}], values = [{', '.join(num(v, places) for v in values)}], "
            f"outside = \"refuse\" }}")


def primitive(pid: str, kind: str, source: str, ref: str, value: str) -> list:
    return ["", "[[primitive]]", f'id = "{pid}"', f'kind = "{kind}"', 'owner = "CAP"', f'source = "{source}"',
            f"source_ref = {json.dumps(ref)}", f"value = {value}"]


def write_kinds(m: dict) -> None:
    ks = kinds()
    days, (structures, backlog, last) = lead_days()
    b = m["sources"]["bea_fixed_assets"]
    note = (f"(BEA Fixed Assets, detailed estimates, fetched {b['fetched']}), each kind's assets as tools/data/"
            f"derive_cap.py maps the BEA's rows to it")
    lines = ["# The kinds of plant (spec CAP.13, TEC.capital's rows), derived by tools/data/derive_cap.py; never edited",
             "# by hand."]
    lines += primitive(
        "CAP.depreciation", "TECHNOLOGY", "measured",
        "Each kind's geometric rate of depreciation a year (axis: " + ", ".join(KINDS) + "): the BEA's 2024 "
        f"current-cost depreciation of the kind's private nonresidential assets over their mean net stock of 2023 and "
        f"2024 {note}; cultivated assets, which the BEA does not capitalise, the median of the geometric rates of "
        "Soloveichik, Tracking Cultivated Assets in Measures of Capital, BEA Working Paper WP2021-6, section 4.",
        table1([k["rate"] for k in ks]))
    lines += primitive(
        "CAP.service_life", "TECHNOLOGY", "measured",
        "Each kind's mean service life in years: its declining-balance rate over its geometric rate, as the BEA divides "
        "the one by a service life to reach the other; the declining-balance rate is the BEA table's (BEA Rates of "
        "Depreciation, Service Lives, Declining-Balance Rates, and Hulten-Wykoff Categories), each asset row weighted "
        f"by its 2024 investment {note}, over the rows that have one ("
        + ", ".join(f"{KINDS[i]} {k['weighted']:.0%} of the kind's investment" for i, k in enumerate(ks) if k['weighted'])
        + "); cultivated assets the median of the mean lives of livestock and plantings in the ABS's Australian System "
        "of National Accounts: Concepts, Sources and Methods 2020-21, Table 14.4.",
        table1([k["life"] for k in ks], 3))
    lines += primitive(
        "CAP.efficiency_shape", "TECHNOLOGY", "measured",
        "Each kind's hyperbolic age-efficiency parameter β, a unit's efficiency (L − a) ÷ (L − β·a) at age a of a life "
        "L: 0.75 for structures and 0.5 for equipment, as the BLS assumes them close to Hulten and Wykoff's estimates "
        "(BLS Handbook of Methods, productivity measures, concepts); 0.5 for software and livestock, as the ABS sets "
        "them (Concepts, Sources and Methods 2020-21, paragraphs 14.37-14.38).",
        table1(BETA, 3))
    lines += primitive(
        "CAP.lead_days", "TECHNOLOGY", "measured",
        f"Days from order to service of the kinds a source measures: a structure {structures:.1f} months, the Census "
        "Bureau's average from start to completion of private nonresidential projects completed in 2024 and 2025 "
        f"(Construction Length of Time, Table 1, {m['sources']['census_length']['fetched']}); equipment {backlog:.2f} "
        "months, the unfilled orders of nondefense capital goods excluding aircraft over their monthly shipments "
        f"(Census M3 survey through FRED, {last[0]} to {last[-1]}), the months an order waits on the books; a month "
        "365.25 ÷ 12 days. Intellectual property and cultivated assets have no measured lead time.",
        table1(days, 0, axis=[0, 1, 2, 3]))
    lines += primitive(
        "CAP.bought_as", "TECHNOLOGY", "measured",
        "The product each kind is bought as, by its place in TEC.products: structures from construction (F), transport, "
        "ICT and other equipment from capital goods (C25-C30, C33), cultivated assets from crops and livestock (A01), "
        "intellectual property from business services (J58-J63, M71-M72), as the input-output tables' products make "
        "them.",
        table1([PRODUCTS[k] for k in range(len(KINDS))], 0))
    (SHARED / "CAP_kinds.toml").write_text("\n".join(lines) + "\n")


def stock_ratios() -> dict:
    """Each reporting economy's net stock of each kind in the firms' sections over its whole gross value added."""
    k = pd.read_csv(RAW / "oecd" / "fixed_assets_by_activity.csv")
    v = pd.read_csv(RAW / "oecd" / "value_added_by_activity.csv")
    k, v = k[k.year <= YEAR], v[v.year <= YEAR]
    out = {}
    for iso3, g in k.groupby("iso3"):
        vg = v[(v.iso3 == iso3) & (v.activity == "_T")]
        years = set(g.year) & set(vg.year)
        if not years:
            continue
        year = max(years)
        g, total = g[(g.year == year) & g.activity.isin(FIRM_SECTIONS)], float(vg[vg.year == year].value.sum())
        stock = g.groupby("item").value.sum()
        if total <= 0 or not all(a in stock.index for a in ASSETS):
            continue
        out[iso3] = np.array([stock[a] / total for a in ASSETS])
    return out


def write_levels(m: dict) -> None:
    ratios = stock_ratios()
    c = pd.read_csv(RAW / "wb" / "countries.csv").set_index("iso3").income_group.map(LEVELS).dropna()
    oecd = m["sources"]["oecd_nad"]
    developed = [r for iso3, r in ratios.items() if c.get(iso3) == "developed"]
    for level in sorted(set(LEVELS.values())):
        own = {iso3: r for iso3, r in ratios.items() if c.get(iso3) == level}
        values = np.median(np.stack(list(own.values()) or developed), axis=0)
        source = "measured" if len(own) >= MIN_COUNTRIES else ("estimated" if own else "assumed")
        who = (f"the median over the {len(own)} economies of the group the OECD reports ({', '.join(sorted(own))})"
               if own else
               f"the OECD reports no economy of the group, so the developed group's median over {len(developed)} "
               "economies is assumed until a source covers the group")
        lines = [f"# The {level} group's stock of plant (spec CAP.1, GEN.2), derived by tools/data/derive_cap.py; never",
                 "# edited by hand."]
        lines += ["", "[[primitive]]", 'id = "CAP.stock_per_gdp"', 'kind = "ENDOWMENT"', 'owner = "CAP"',
                  f'source = "{source}"',
                  "source_ref = " + json.dumps(
                      "Each kind's net stock (axis: " + ", ".join(KINDS) + ") per unit of GDP: the closing net stock "
                      "at current prices of the sections the firms work in (ISIC A to J, M, N, P to S) over all gross "
                      f"value added, OECD Table 9A over Table 6 in national currency, the latest year to {YEAR} "
                      f"({oecd['title']}, fetched {oecd['fetched']}); {who}."),
                  f"value = {table1(list(values))}"]
        (PROFILES / level / "CAP.toml").write_text("\n".join(lines) + "\n")


def main() -> None:
    m = manifest()
    write_kinds(m)
    write_levels(m)


if __name__ == "__main__":
    main()
