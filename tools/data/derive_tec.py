#!/usr/bin/env python3
"""Derives the products and each country group's opening ways from the fetched sources in data/sources/raw/.

A product is an aggregate of the input-output tables' industries' outputs (ISIC Rev. 4, the OECD's inter-country
tables of 2019). Its unit is physical where the product has one the world measures it in (the extracted products, in
kilograms, as deposits hold them, fine enough that what a way uses of one by the unit is whole) and otherwise the volume the national accounts measure an aggregate in: what one US
cent bought at the world's average prices in 2022, the tables' dollars of 2019 carried to 2022 by the United States'
GDP deflator and turned into quantities by the ICP's price level of the product's heading. A way states, per unit of
output, what it uses of each product, the hours of each occupation group, the stock of each kind of plant per unit of
output a year, and the land it needs. Each is the group's median over the
economies the sources report for it; what the group's economies lack is named in the note of what it rests on.

    python3 tools/data/derive_tec.py

Writes data/shared/TEC.toml (the products and what extraction takes from a deposit),
the group's ways and each product's price level over GDP's, which its opening price is carried by, into each level's
economy.toml.
"""
import json
from pathlib import Path

import numpy as np
import pandas as pd

from derive import LEVELS, RAW
from derive_cap import pwt_scale
import profile_files

ROOT = Path(__file__).resolve().parents[2]
SHARED = ROOT / "data" / "shared"
YEAR = 2019
UNIT_YEAR = 2022
MIN_COUNTRIES = 10
EXP = 9
# The places a product's part of its industry's output and its price level are written to.
SHARE_EXP = 6
WEEKS = 52
CENTS = 100
# Kilograms in a tonne, the extracted products' world prices being quoted by the tonne.
KG_PER_TONNE = 1000
# The tables report millions of dollars.
MILLION = 1e6
HECTARES_PER_KM2 = 100
LEAD_SEASON = 120
TONNES_PER_BARREL = 1 / 7.33

# The products: name, industry, the ICIO industries whose output they aggregate, the ICP headings their price level is
# read from (none: priced at world prices, as traded commodities are, and energy carriers as the fuels they are made
# from, the ICP publishing no heading for energy alone), storable, delivered as made, the deposit resource extracted.
PRODUCTS = [
    ("crops and livestock", "agriculture", ["A01_02", "A03"], ["1101100"], True, False, None),
    ("metal ore", "mining", ["B07_08"], [], True, False, 0),
    ("coal", "mining", ["B05_06"], [], True, False, 1),
    ("oil and gas", "mining", ["B05_06"], [], True, False, 2),
    ("building stone", "mining", ["B07_08"], [], True, False, 3),
    ("food", "food manufacturing", ["C10T12"], ["1101000"], True, False, None),
    ("consumer goods", "consumer goods manufacturing", ["C13T15", "C31T33"], ["1103000", "1105000"], True, False, None),
    ("materials", "materials manufacturing", ["C16", "C17_18", "C20", "C21", "C22", "C23", "C24"], ["1501300"],
     True, False, None),
    ("energy carriers", "energy", ["C19", "D"], [], True, False, None),
    ("capital goods", "capital goods manufacturing", ["C25", "C26", "C27", "C28", "C29", "C30"], ["1501100"],
     True, False, None),
    ("water and waste services", "utilities", ["E"], ["1000000"], False, True, None),
    ("construction", "construction", ["F"], ["1501200"], False, False, None),
    ("distribution", "trade", ["G"], ["1000000"], False, True, None),
    ("transport", "transport", ["H49", "H50", "H51", "H52", "H53"], ["1107300"], False, True, None),
    ("accommodation and meals", "accommodation and food services", ["I"], ["1111000"], False, True, None),
    ("business services", "business services", ["B09", "J58T60", "J61", "J62_63", "M", "N"], ["1000000"], False, True,
     None),
    ("education", "education", ["P"], ["9120000"], False, True, None),
    ("health and care", "health", ["Q"], ["9080000"], False, True, None),
    ("personal services", "personal services", ["R", "S"], ["9140000"], False, True, None),
]
# The ICP's heading for GDP, whose price level is the economy's.
GDP_HEADING = "1000000"
# Products the ways leave out, paid for otherwise: finance by fees and interest (BNK), real estate by rents (HSG),
# public administration by taxes (TRS), households' own employment and extraterritorial bodies.
LEFT_OUT = ["K", "L", "O", "T"]
# The activities the ways make no product of whose hours the opening still staffs, each its section's whole: their
# unit what a US cent bought at GDP's price level, as the accounts value their output.
SERVICES = [("finance", "K"), ("real estate", "L"), ("public administration", "O")]
# The value-added row.
VALUE_ADDED = "VA"
# Each mining industry's resources: the one a using industry's input is taken from, where it transforms one, and the
# one any other user's is. Energy mining yields coal to electricity and oil and gas to the rest, which run on fuel;
# other mining yields ore to the basic metals and stone to the rest, the minerals and construction above all.
MINES = {"B05_06": ({"D": 1}, 2), "B07_08": ({"C24": 0}, 3)}
# The world price per tonne each extracted resource is sold at, from the Pink Sheet and the USGS.
RESOURCE_PRICE = {0: "Iron ore, cfr spot", 1: "Coal, Australian", 2: "Crude oil, average",
                  3: "Crushed stone, United States, average unit value"}
# The ISIC section each industry belongs to, for hours and occupations (ILO) and fixed assets (OECD).
SECTIONS = "ABCDEFGHIJKLMNOPQRSTU"
# The kinds of plant, SNA asset codes: other buildings and structures, transport equipment, ICT equipment, other
# machinery and equipment, cultivated biological resources, intellectual property products.
ASSETS = [("structures", "N112N", "1501200"), ("transport equipment", "N1131N", "1501100"),
          ("ICT equipment", "N1132N", "1501100"), ("other machinery", "N11ON", "1501100"),
          ("cultivated biological resources", "N115N", "1101100"), ("intellectual property", "N117N", "1501300")]
OCCUPATIONS = [str(i) for i in range(10)]


def section(industry: str) -> str:
    return industry[0]


def num(x: float, places: int = EXP) -> str:
    return f"{x:.{places}f}"


def grid(values: np.ndarray, places: int = EXP) -> str:
    return "[" + ", ".join("[" + ", ".join(num(v, places) for v in row) + "]" for row in values) + "]"


def manifest() -> dict:
    return json.loads((RAW / "manifest.json").read_text())


def groups() -> pd.Series:
    c = pd.read_csv(RAW / "wb" / "countries.csv")
    return c.set_index("iso3").income_group.map(LEVELS).dropna()


def io_tables() -> dict:
    """Each economy's table: rows are the industries whose output was used, taxes less subsidies and value added,
    columns the industries that used it."""
    d = pd.read_csv(RAW / "icio" / f"io_{YEAR}.csv")
    return {c: g.pivot_table(index="row", columns="industry", values="value", aggfunc="sum", fill_value=0.0)
            for c, g in d.groupby("iso3")}


def price_levels() -> pd.DataFrame:
    return pd.read_csv(RAW / "icp" / f"price_levels_2021.csv", dtype={"heading": str}).pivot(
        index="iso3", columns="heading", values="value")


def commodities() -> dict:
    d = pd.read_csv(RAW / "prices" / f"commodities_{YEAR}.csv")
    return dict(zip(d.name, d.value))


def product_level(pl: pd.Series, headings: list) -> float:
    """A product's price level relative to the world's: the geometric mean of its headings', or one at world prices."""
    if not headings:
        return 1.0
    return float(np.exp(np.mean([np.log(pl[h] / 100.0) for h in headings])))


def per_unit(prices: dict, deflator: float) -> np.ndarray:
    """Each product's dollars of the unit's year per unit: a cent, or the price of a kilogram of what is extracted in
    the tables' year carried to the unit's."""
    out = np.full(len(PRODUCTS), 1.0 / CENTS)
    for i, p in enumerate(PRODUCTS):
        if p[6] is not None:
            price = prices[RESOURCE_PRICE[p[6]]]
            out[i] = (price / TONNES_PER_BARREL if p[6] == 2 else price) * deflator / KG_PER_TONNE
    return out


def taken_share(user: str, resource: int) -> float:
    """How much of a using industry's input from its mining industry comes from a resource: all of it from the one
    its industry transforms, or else from the one other users take."""
    for mine, (own, other) in MINES.items():
        if resource in own.values() or resource == other:
            return 1.0 if own.get(user, other) == resource else 0.0
    raise ValueError(resource)


def mine_of(p) -> str:
    return p[2][0]


def extracted_weight(t: pd.DataFrame, p) -> float:
    """An extracted product's part of its mining industry's output: the part its resource's users take."""
    mine = mine_of(p)
    users = t.loc[mine, :]
    return float(sum(users[c] * taken_share(c, p[6]) for c in t.columns) / users.sum())


def relative_levels(pl: pd.Series) -> np.ndarray:
    """Each product's price level over the economy's GDP's: what a unit costs in the currency a GDP at purchasing power
    parity is counted in, the quantity a US cent buys at world-average prices."""
    gdp = pl[GDP_HEADING] / 100.0
    return np.array([product_level(pl, p[3]) / gdp for p in PRODUCTS])


def economy_ways(t: pd.DataFrame, pl: pd.Series, deflator: float, unit_dollars: np.ndarray):
    """One economy's inputs per unit of output and its output in units, from its table and price levels."""
    cols = list(t.columns)
    output = t.sum(axis=0)
    level = np.array([product_level(pl, p[3]) for p in PRODUCTS])
    units_per_dollar = deflator / (level * unit_dollars)
    n = len(PRODUCTS)
    made = np.zeros(n)
    used = np.zeros((n, n))
    for j, p in enumerate(PRODUCTS):
        codes = [c for c in p[2] if c in cols]
        weight = extracted_weight(t, p) if p[6] is not None else 1.0
        made[j] = output[codes].sum() * MILLION * weight * units_per_dollar[j]
        for i, q in enumerate(PRODUCTS):
            if q[6] is not None:
                taken = sum(float(t.loc[mine_of(q), c]) * taken_share(c, q[6]) for c in codes if mine_of(q) in t.index)
            else:
                taken = float(t.loc[[c for c in q[2] if c in t.index], codes].values.sum())
            used[i, j] = taken * MILLION * weight * units_per_dollar[i]
    inputs = np.divide(used, made, out=np.zeros_like(used), where=made > 0)
    gdp_units = deflator * CENTS / product_level(pl, [GDP_HEADING])
    served = np.array([float(output[code]) * MILLION * gdp_units if code in cols else 0.0 for _, code in SERVICES])
    return inputs, np.concatenate([made, served])


def nearest_year(years) -> int:
    """The survey year nearest the tables', a later one on a tie."""
    return min(years, key=lambda y: (abs(y - YEAR), -y))


def labour_hours() -> dict:
    """Each economy's hours worked in a year by ISIC section and ISCO-08 major group, from employment and average
    weekly hours in its survey nearest the tables' year; hours missing for a group are its section's average, and
    employment the survey left unclassified is spread over the groups as they stand."""
    e = pd.read_csv(RAW / "ilo" / "employment_by_activity.csv", dtype={"occupation": str})
    h = pd.read_csv(RAW / "ilo" / "hours_by_activity.csv", dtype={"occupation": str})
    out = {}
    for iso3, g in e.groupby("iso3"):
        hg = h[h.iso3 == iso3]
        if hg.empty:
            continue
        year = nearest_year(set(g.year) & set(hg.year)) if set(g.year) & set(hg.year) else None
        if year is None:
            continue
        g, hg = g[g.year == year], hg[hg.year == year]
        source = g.groupby("source").size().sort_index().idxmax()
        g = g[g.source == source]
        hsrc = hg[hg.source == source] if (hg.source == source).any() else hg[hg.source == hg.source.min()]
        emp = g.pivot_table(index="activity", columns="occupation", values="value", aggfunc="sum")
        hrs = hsrc.pivot_table(index="activity", columns="occupation", values="value", aggfunc="mean")
        table = {}
        for sec in SECTIONS:
            if sec not in emp.index or "TOTAL" not in emp.columns or pd.isna(emp.loc[sec, "TOTAL"]):
                continue
            groups = emp.reindex(columns=OCCUPATIONS).loc[sec].fillna(0.0)
            classified = float(groups.sum())
            if classified <= 0:
                continue
            groups = groups * float(emp.loc[sec, "TOTAL"]) / classified
            average = hrs.loc[sec, "TOTAL"] if sec in hrs.index and "TOTAL" in hrs.columns else np.nan
            if pd.isna(average):
                continue
            weekly = np.array([hrs.loc[sec, o] if sec in hrs.index and o in hrs.columns
                               and not pd.isna(hrs.loc[sec, o]) else average for o in OCCUPATIONS])
            table[sec] = groups.values * 1000.0 * weekly * WEEKS
        out[iso3] = (year, source, table)
    return out


def capital_ratios() -> dict:
    """Each reporting economy's net stock of each kind of plant per unit of gross value added, by ISIC section, in
    its latest year up to the tables'."""
    k = pd.read_csv(RAW / "oecd" / "fixed_assets_by_activity.csv")
    v = pd.read_csv(RAW / "oecd" / "value_added_by_activity.csv")
    k, v = k[k.year <= YEAR], v[v.year <= YEAR]
    out = {}
    for iso3, g in k.groupby("iso3"):
        vg = v[v.iso3 == iso3]
        years = set(g.year) & set(vg.year)
        if not years:
            continue
        year = max(years)
        g, vg = g[g.year == year], vg[vg.year == year]
        va = vg.groupby("activity").value.sum()
        ratios = {}
        for sec in SECTIONS:
            if sec not in va.index or va[sec] <= 0:
                continue
            stock = g[g.activity == sec].set_index("item").value
            if not all(a[1] in stock.index for a in ASSETS):
                continue
            ratios[sec] = np.array([stock[a[1]] / va[sec] for a in ASSETS])
        if ratios:
            out[iso3] = ratios
    return out


def economy_factors(t, made, pl, deflator, hours, ratios):
    """One economy's hours per unit by occupation group and stock of each kind of plant per unit a year, each
    section's hours and stocks attributed to its industries by their value added."""
    cols = list(t.columns)
    added = {c: float(t.loc[VALUE_ADDED, c]) for c in cols}
    section_added = {s: sum(v for c, v in added.items() if section(c) == s) for s in SECTIONS}
    n = len(PRODUCTS)
    labour = np.full((len(OCCUPATIONS), n + len(SERVICES)), np.nan)
    capital = np.full((len(ASSETS), n), np.nan)
    asset_level = np.array([pl[a[2]] / 100.0 for a in ASSETS])
    for j, p in enumerate(PRODUCTS):
        codes = [c for c in p[2] if c in cols]
        weight = extracted_weight(t, p) if p[6] is not None else 1.0
        if hours is not None and all(section(c) in hours for c in codes):
            h = sum(hours[section(c)] * added[c] / section_added[section(c)] for c in codes
                    if section_added[section(c)] > 0)
            labour[:, j] = h * weight / made[j]
        if ratios is not None and all(section(c) in ratios for c in codes):
            stock_dollars = sum(ratios[section(c)] * added[c] for c in codes) * MILLION * weight
            capital[:, j] = stock_dollars * deflator * CENTS / asset_level / made[j]
    for k, (_, code) in enumerate(SERVICES):
        if hours is not None and code in hours and made[n + k] > 0:
            labour[:, n + k] = hours[code] / made[n + k]
    return labour, capital


def land_per_unit(t, made, land_km2) -> float:
    """Hectares of agricultural land a unit of the land-using product takes a year."""
    return land_km2 * HECTARES_PER_KM2 / made[0]


def write_shared(prices: dict, src: dict) -> None:
    lines = [
        "# The products and what extraction takes from a deposit (spec TEC.1, TEC.2), derived by",
        "# tools/data/derive_tec.py; never edited by hand.",
        "",
        "[[primitive]]",
        'id = "TEC.products"',
        'kind = "TECHNOLOGY"',
        'owner = "TEC"',
        'source = "measured"',
        "source_ref = " + json.dumps(
            "The industries of the input-output tables (OECD ICIO, ISIC Rev. 4) aggregated to the products the "
            "circular flow needs: " + "; ".join(f"{p[0]} ({', '.join(p[2])})" for p in PRODUCTS) + ". Energy mining "
            "yields coal and oil and gas, other mining ore and stone, each in the part its resource's users take. "
            "Finance, real "
            "estate, public administration and households as employers are left out, being paid for by fees, rents "
            "and taxes. A service is not storable and is delivered as it is made; construction is work done for its "
            "owner, not stored."),
        "value = [",
    ]
    for p in PRODUCTS:
        extracts = f", extracts = {p[6]}" if p[6] is not None else ""
        lines.append(f'  {{ name = "{p[0]}", unit = "{p[0]}", industry = "{p[1]}", '
                     f'storable = {str(p[4]).lower()}, delivered_at_once = {str(p[5]).lower()}{extracts} }},')
    lines.append("]")
    draws = []
    for i, p in enumerate(PRODUCTS):
        draws.append(1.0 if p[6] is not None else 0.0)
    lines += [
        "",
        "[[primitive]]",
        'id = "TEC.deposit_draw"',
        'kind = "TECHNOLOGY"',
        'owner = "TEC"',
        'source = "measured"',
        "source_ref = " + json.dumps(
            "Kilograms an extracted product takes from its deposit per unit made: its unit is the kilogram, so one; "
            "the others take none."),
        f"value = {{ axis = [{', '.join(str(i) for i in range(len(PRODUCTS)))}], "
        f"values = [{', '.join(num(v, 0) for v in draws)}], outside = \"refuse\" }}",
    ]
    axis = ", ".join(str(i) for i in range(len(PRODUCTS)))
    lead = [LEAD_SEASON if p[3] == ["1101100"] else (0 if p[5] else 1) for p in PRODUCTS]
    lines += [
        "",
        "[[primitive]]",
        'id = "TEC.lead_time"',
        'kind = "TECHNOLOGY"',
        'owner = "TEC"',
        'source = "assumed"',
        "source_ref = " + json.dumps(
            f"Days from starting a unit to finishing it: a growing season of {LEAD_SEASON} days for crops and livestock, "
            "about the months from sowing to harvest of the main field crops (FAO crop calendars: wheat, maize and "
            "rice take 90 to 150 days); none for a service, delivered as it is made; and a day for every other "
            "product, whose aggregate is made continuously and finishes within the day at the day's resolution."),
        f"value = {{ axis = [{axis}], values = [{', '.join(str(v) for v in lead)}], outside = \"refuse\" }}",
        "",
        "[[primitive]]",
        'id = "TEC.yield"',
        'kind = "TECHNOLOGY"',
        'owner = "TEC"',
        'source = "measured"',
        "source_ref = " + json.dumps(
            "The share of what is started that is finished, in parts per million: all of it, since the input-output "
            "tables measure inputs per unit finished, so what a process loses is already in its inputs."),
        f"value = {{ axis = [{axis}], values = [{', '.join('1000000' for _ in PRODUCTS)}], outside = \"refuse\" }}",
        "",
        "[[primitive]]",
        'id = "TEC.batch"',
        'kind = "TECHNOLOGY"',
        'owner = "TEC"',
        'source = "measured"',
        "source_ref = " + json.dumps(
            "The least quantity started at once, in units: one, since each product is an aggregate measured in "
            "continuous volume or kilograms, with no lot size the tables distinguish."),
        f"value = {{ axis = [{axis}], values = [{', '.join('1' for _ in PRODUCTS)}], outside = \"refuse\" }}",
    ]
    (SHARED / "TEC.toml").write_text("\n".join(lines) + "\n")


def median(stack: list) -> np.ndarray:
    """The cell-by-cell median over the economies that report the cell."""
    return np.nanmedian(np.stack(stack), axis=0)


def table2(rows, values: np.ndarray) -> str:
    cols = range(values.shape[1])
    return (f"{{ rows = [{', '.join(str(r) for r in rows)}], columns = [{', '.join(str(c) for c in cols)}], "
            f"values = {grid(values)}, outside = \"refuse\" }}")


def primitive(pid: str, source: str, ref: str, value: str) -> list:
    return ["", "[[primitive]]", f'id = "{pid}"', 'kind = "TECHNOLOGY"', 'owner = "TEC"', f'source = "{source}"',
            f"source_ref = {json.dumps(ref)}", f"value = {value}"]


def write_level(level: str, members: dict, m: dict) -> None:
    """The group's ways: each quantity the median over the group's economies that report it."""
    tables, ilo, oecd = (m["sources"][k] for k in ("icio", "ilo_activity", "oecd_nad"))
    unit_note = (f"per unit of output: a unit is a kilogram of an extracted product and otherwise what one US cent bought "
                 f"at world-average prices in {UNIT_YEAR}, each product's dollars of {YEAR} carried to {UNIT_YEAR} by "
                 f"the United States' GDP deflator and turned into quantities by its ICP 2021 price level, energy "
                 f"carriers' at world prices as the fuels they are made from, the ICP publishing no level for energy")
    names = lambda cs: ", ".join(sorted(cs))
    inputs = median([w["inputs"] for w in members.values()])
    lab = {c: w for c, w in members.items() if not np.all(np.isnan(w["labour"]))}
    cap = {c: w for c, w in members.items() if not np.all(np.isnan(w["capital"]))}
    labour = median([w["labour"] for w in lab.values()])
    capital = median([w["capital"] for w in cap.values()])
    land = float(np.median([w["land"] for w in members.values() if not np.isnan(w["land"])]))
    thin = lambda k: "measured" if k >= MIN_COUNTRIES else "estimated"
    own_capital = [c for c, w in cap.items() if w["capital_own"]]
    lines = [f"# The {level} group's opening ways (spec TEC.2, TEC.13), derived by tools/data/derive_tec.py; never",
             "# edited by hand."]
    lines += primitive(
        "TEC.inputs", thin(len(members)),
        f"What a way uses of each product (rows) {unit_note}, for each product made (columns): the median over the "
        f"{len(members)} economies of the World Bank's {level} income groups the {YEAR} inter-country input-output "
        f"tables report ({names(members)}), each use summed over where it came from ({tables['title']}, fetched "
        f"{tables['fetched']}). An extracted product's use is its mining industry's column's, in the part its "
        f"resource's users take; what an industry takes from a mining industry is taken from the resource the "
        f"industry transforms, or else from the one other users take.",
        table2(range(len(PRODUCTS)), inputs))
    lines += primitive(
        "TEC.labour", thin(len(lab)),
        f"Hours of each occupation group (rows, ISCO-08 major groups 0 armed forces to 9 elementary) {unit_note}: the "
        f"median over the {len(lab)} economies of the group that both ILOSTAT's surveys and the tables report "
        f"({names(lab)}), each section's employment times its average weekly hours times {WEEKS} in the survey nearest "
        f"{YEAR} ({ilo['title']}, fetched {ilo['fetched']}), shared among the section's industries by their value "
        f"added in the tables. Columns 0 to {len(PRODUCTS) - 1} the products, then "
        + ", ".join(f"{len(PRODUCTS) + k} {name} (section {code})" for k, (name, code) in enumerate(SERVICES))
        + ", which make no product: their hours a unit of their output, a unit what one US cent bought at GDP's "
        "price level, as the accounts value it.",
        table2(range(len(OCCUPATIONS)), labour))
    lines += primitive(
        "TEC.capital", "measured" if len(own_capital) >= MIN_COUNTRIES else "estimated",
        f"Net stock of each kind of plant (rows: " + ", ".join(a[0] for a in ASSETS) + ") a unit of output a year "
        f"needs, in cents of plant at world-average prices: each section's stock per unit of gross value added "
        f"(OECD Table 9A over Table 6, {oecd['fetched']}) times the product's value added in the tables, by the kind's "
        f"ICP heading; the median over {len(cap)} economies of the group. "
        + (f"The stocks are the group's own, reported by {len(own_capital)} economies ({names(own_capital)})."
           if own_capital else
           "The OECD reports the stocks of no economy of the group, so each section's stock per unit of value added is "
           "the developed economies' median scaled, kind by kind, by the group's stock of each Penn World Table asset "
           "per unit of GDP over the developed group's."),
        table2(range(len(ASSETS)), capital))
    lines += primitive(
        "TEC.land", thin(len(members)),
        f"Hectares of agricultural land a unit of output takes a year: the land-using product's is the World Bank's "
        f"agricultural land (AG.LND.AGRI.K2) in {YEAR} or the latest year before it, over its output in the {YEAR} "
        f"inter-country input-output tables, the median over the group's economies; the others take none.",
        f"{{ axis = [{', '.join(str(i) for i in range(len(PRODUCTS)))}], values = ["
        + ", ".join(num(land if i == 0 else 0.0) for i in range(len(PRODUCTS))) + "], outside = \"refuse\" }")
    profile_files.put_text(level, "\n".join(lines) + "\n")
    levels = median([w["levels"] for w in members.values()])
    icp = m["sources"]["prices"]
    ref = (f"Each product's price at the opening over its world price (axis: the products' places): its ICP 2021 price "
           f"level over GDP's, the geometric mean over its headings' (World Bank ICP 2021, fetched {icp['fetched']}), or "
           f"one over GDP's for a product priced at world prices, as the extracted products are; the median over the "
           f"economies the ICP reports of the {len(members)} of the World Bank's {level} income groups the {YEAR} inter-country input-output "
           f"tables report ({names(members)}). A currency's smallest unit buys a US cent's worth at world-average "
           f"prices, a GDP at purchasing power parity, so a unit that costs a US cent at world-average prices costs "
           f"its price level over GDP's.")
    gds = [f"# The {level} group's products' price levels (spec GEN.5, GDS.1), derived by tools/data/derive_tec.py;",
           "# never edited by hand.", "", "[[primitive]]", 'id = "GDS.price_level"', 'kind = "ENDOWMENT"',
           'owner = "GDS"', f'source = "{thin(len(members))}"', f"source_ref = {json.dumps(ref)}",
           f"value = {{ axis = [{', '.join(str(i) for i in range(len(PRODUCTS)))}], values = ["
           + ", ".join(num(v, SHARE_EXP) for v in levels) + "], outside = \"refuse\" }"]
    profile_files.put_text(level, "\n".join(gds) + "\n")


def main() -> None:
    m = manifest()
    tables = io_tables()
    pls = price_levels()
    prices = commodities()
    level_of = groups()
    deflator = prices[f"United States GDP deflator, {UNIT_YEAR} over {YEAR}"]
    unit_dollars = per_unit(prices, deflator)
    hours = labour_hours()
    ratios = capital_ratios()
    land = pd.read_csv(RAW / "wb" / "AG.LND.AGRI.K2.csv")
    land = land[land.year <= YEAR].sort_values("year").groupby("iso3").value.last()
    developed = [ratios[c] for c in ratios if level_of.get(c) == "developed"]
    fallback = {sec: np.median(np.stack([r[sec] for r in developed if sec in r]), axis=0)
                for sec in SECTIONS if any(sec in r for r in developed)}
    # A group the OECD reports no economy of takes the developed ratios scaled by its Penn World Table stocks.
    scaled = {level: {sec: v * pwt_scale(level)[0] for sec, v in fallback.items()}
              for level in ["emerging", "developing"]}
    ways = {}
    for iso3, t in tables.items():
        if iso3 not in pls.index:
            continue
        inputs, made = economy_ways(t, pls.loc[iso3], deflator, unit_dollars)
        own = iso3 in ratios
        labour, capital = economy_factors(t, made, pls.loc[iso3], deflator,
                                          hours[iso3][2] if iso3 in hours else None,
                                          ratios.get(iso3, scaled.get(level_of.get(iso3), fallback)))
        ways[iso3] = {"inputs": inputs, "labour": labour, "capital": capital, "capital_own": own,
                      "land": land_per_unit(t, made, land[iso3]) if iso3 in land.index else np.nan,
                      "levels": relative_levels(pls.loc[iso3])}
    write_shared(prices, m)
    for level in ["developed", "emerging", "developing"]:
        members = {c: w for c, w in ways.items() if level_of.get(c) == level}
        write_level(level, members, m)
        print(level, len(members))


if __name__ == "__main__":
    main()
