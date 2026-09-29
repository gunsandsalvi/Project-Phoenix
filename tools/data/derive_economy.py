#!/usr/bin/env python3
"""The one economic derivation: each country group's flows (a social accounting matrix) and stocks (a balance-sheet
matrix) of 2019, in shares of its GDP, so the opening starts stock-flow consistent.

It first runs the derivations whose primitives the matrices meet — the ways and price levels (derive_tec.py), the
households' budget shares (derive_hh.py) and the firms' plant (derive_cap.py) — then derives what the matrices add, and
checks the identities across all of them before writing anything.

The flows. The economy is closed: what each economy used, from wherever it came, is taken as made at home, and its
exports are other economies' uses. There are twenty-two activities: the nineteen products (TEC), finance, real estate
and public administration; households' own employment is no market activity and is left out. A product's inputs of
products are its way's (TEC.inputs) at the group's prices (each product's opening price times its price level), so the
matrix and the ways agree by construction; what each activity takes of finance, real estate and public administration,
what those three take of everything, and each activity's taxes less subsidies on products, are the medians over the
group's economies of each one's use over the user's output (OECD ICIO 2019). Value added is what is left of each
activity's output, the one residue the matrix has, and it is split into compensation, operating surplus and mixed
income, and other taxes on production by the OECD's Table 6. Final uses are households (HFCE, whose products are the
households' budget shares), collective consumption (government and the non-profit institutions serving households),
fixed investment and inventories, each's weight in GDP and its composition the group's medians; each activity's output
is then what its uses need, (I - A)^-1 f, so every product's supply is its uses.

The stocks. Five sectors — households, firms, banks, the central bank, the government — hold currency, deposits,
loans, bonds, reserves, equity and real assets. Each instrument's level is its broad source's group median (IMF, World
Bank), who holds it is the OECD's sector accounts (Table 720), and the real assets are the firms' plant (CAP), their
inventories and land, households' dwellings and land and the government's fixed assets (Table 9B). The declared
closures make every instrument's assets its liabilities: the central bank holds government paper for its currency and
reserves; banks hold loans, reserves and government paper against deposits, their equity at their capital ratio and
bonds that balance them, held by households; firms' equity is their assets less their debts; households hold the rest.

The shapes. Only distributions come from outside the matrices, each scaled by the opening to their totals: the spread
of firms' physical productivity within an industry (Hsieh and Klenow, typed), and each occupation's mean earnings over
all employees' (ILOSTAT).

    python3 tools/data/derive_economy.py
"""
import json
import sys
import tomllib
from pathlib import Path

import numpy as np
import pandas as pd

import derive_cap
import derive_hh
import derive_tec as tec
from derive import LEVELS, RAW
import profile_files

ROOT = Path(__file__).resolve().parents[2]
YEAR = 2019
# A series' observation nearest the year, within this many years of it.
NEAR_YEARS = 2
MIN_COUNTRIES = 10
EXP = 9
MILLION = 1e6
PERCENT = 100.0

PRODUCTS = [p[0] for p in tec.PRODUCTS]
SERVICES = {"finance": "K", "real estate": "L", "public administration": "O"}
ACTIVITIES = PRODUCTS + list(SERVICES)
N = len(ACTIVITIES)
NP = len(PRODUCTS)
# Final uses: households, collective consumption (government's with the non-profits'), fixed investment, inventories.
FINALS = ["households", "collective", "investment", "inventories"]
FINAL_OF = {"HFCE": 0, "NPISH": 1, "GGFC": 1, "GFCF": 2, "INVNT": 3}
# The parts of value added.
PARTS = ["compensation", "operating surplus and mixed income", "other taxes on production"]
# Each activity's OECD Table 6 sections, whose parts of value added it takes.
VA_SECTIONS = {
    "crops and livestock": ["A"], "metal ore": ["B"], "coal": ["B"], "oil and gas": ["B"], "building stone": ["B"],
    "food": ["C"], "consumer goods": ["C"], "materials": ["C"], "energy carriers": ["D"], "capital goods": ["C"],
    "water and waste services": ["E"], "construction": ["F"], "distribution": ["G"], "transport": ["H"],
    "accommodation and meals": ["I"], "business services": ["J", "M", "N"], "education": ["P"],
    "health and care": ["Q"], "personal services": ["R", "S"], "finance": ["K"], "real estate": ["L"],
    "public administration": ["O"],
}

SECTORS = ["households", "firms", "banks", "central bank", "government"]
H, F, B, C, G = range(len(SECTORS))
INSTRUMENTS = ["currency", "deposits", "loans to households", "loans to firms", "firms' bonds",
               "government paper", "reserves", "central bank loans to banks", "banks' bonds", "banks' equity",
               "firms' equity"]
REAL = [f"plant: {a[0]}" for a in tec.ASSETS] + ["inventories", "firms' land", "dwellings", "households' land",
                                                  "government's fixed assets"]
OECD_DWELLINGS, OECD_LAND, OECD_INVENTORIES = "N111N", "N211N", "N12N"
OECD_FIXED = ["N111N", "N112N", "N11MN", "N115N", "N117N"]
# ISCO-08 major groups, 0 the armed forces.
OCCUPATIONS = [str(i) for i in range(10)]


def fail(message: str) -> None:
    raise SystemExit(f"derive_economy: {message}")


def num(x: float, places: int = EXP) -> str:
    return f"{x:.{places}f}"


def level_of() -> pd.Series:
    c = pd.read_csv(RAW / "wb" / "countries.csv")
    return c.set_index("iso3").income_group.map(LEVELS).dropna()


def at_year(path: Path, scale: float = 1.0) -> pd.Series:
    """Each economy's observation of the year, or the nearest within `NEAR_YEARS`, the later on a tie."""
    d = pd.read_csv(path)
    d = d[(d.year - YEAR).abs() <= NEAR_YEARS].copy()
    d["distance"] = (d.year - YEAR).abs() * 2 - (d.year > YEAR)
    return d.sort_values("distance").groupby("iso3").value.first() * scale


# ---------------------------------------------------------------------------------------------------------------
# The flows.

def activity_of(code: str):
    """An ICIO industry's activity: a product's place, a service's, or none for households as employers."""
    for i, p in enumerate(tec.PRODUCTS):
        if code in p[2] and p[6] is None:
            return i
    for i, s in enumerate(SERVICES.values()):
        if code == s:
            return NP + i
    return None


def resources(mine: str) -> list:
    return [i for i, p in enumerate(tec.PRODUCTS) if p[6] is not None and p[2][0] == mine]


def economy_flows(t: pd.DataFrame):
    """One economy's flows among the activities, in its dollars: uses by activity and final use, the taxes less
    subsidies on products of each column, and each activity's value added."""
    industries = [c for c in t.columns if c not in FINAL_OF]
    use = np.zeros((N, N))
    final = np.zeros((N, len(FINALS)))
    taxes = np.zeros(N + len(FINALS))
    added = np.zeros(N)
    # A mining industry's column is shared between its resources by the part of its output their users take.
    weight = {}
    for mine in tec.MINES:
        if mine not in t.index:
            continue
        users = t.loc[mine, industries]
        total = float(users.sum())
        for r in resources(mine):
            weight[r] = float(sum(users[c] * tec.taken_share(c, tec.PRODUCTS[r][6]) for c in industries)) / total \
                if total > 0 else 0.0

    def columns_of(code: str):
        if code in FINAL_OF:
            return [(N + FINAL_OF[code], 1.0)]
        if code in tec.MINES:
            return [(r, weight.get(r, 0.0)) for r in resources(code)]
        a = activity_of(code)
        return [] if a is None else [(a, 1.0)]

    def rows_of(code: str, user: str):
        if code in tec.MINES:
            own, other = tec.MINES[code]
            wanted = own.get(user, other) if user not in FINAL_OF else other
            return [(r, 1.0) for r in resources(code) if tec.PRODUCTS[r][6] == wanted]
        a = activity_of(code)
        return [] if a is None else [(a, 1.0)]

    for code in t.columns:
        cols = columns_of(code)
        for row in t.index:
            v = float(t.at[row, code])
            if v == 0.0:
                continue
            for c, cw in cols:
                if row == "TLS":
                    taxes[c] += v * cw
                elif row == "VA":
                    if c < N:
                        added[c] += v * cw
                else:
                    for r, rw in rows_of(row, code):
                        if c < N:
                            use[r, c] += v * cw * rw
                        else:
                            final[r, c - N] += v * cw * rw
    return use, final, taxes, added


def flows_by_economy() -> dict:
    d = pd.read_csv(RAW / "icio" / f"flows_{YEAR}.csv")
    out = {}
    for iso3, g in d.groupby("iso3"):
        t = g.pivot_table(index="row", columns="column", values="value", aggfunc="sum", fill_value=0.0)
        use, final, taxes, added = economy_flows(t)
        output = use.sum(axis=0) + taxes[:N] + added
        if np.any(output <= 0):
            continue
        spent = final.sum(axis=0)
        purchase = spent + taxes[N:]
        out[iso3] = {
            "coef": use / output,
            "tax_rate": taxes[:N] / output,
            "final_weight": purchase / purchase.sum(),
            "final_tax": np.divide(taxes[N:], spent, out=np.zeros(len(FINALS)), where=spent > 0),
            "composition": np.divide(final, spent, out=np.zeros_like(final), where=spent > 0),
            "output": output,
        }
    return out


def value_added_parts(members: list, levels: pd.Series):
    """Each activity's parts of its value added: the median over the group's economies that report them, or, with
    too few, the developed group's with compensation scaled by the group's labour share over the developed's."""
    v = pd.read_csv(RAW / "oecd" / f"value_added_parts_{YEAR}.csv")
    v = v[v.activity.str.len() == 1].pivot_table(index=["iso3", "activity"], columns="item", values="value")

    def shares(economies):
        rows = []
        for iso3 in economies:
            if iso3 not in v.index.get_level_values(0):
                continue
            e = v.loc[iso3]
            row = []
            for a in ACTIVITIES:
                secs = [s for s in VA_SECTIONS[a] if s in e.index]
                total = e.loc[secs, "B1G"].sum() if secs and "B1G" in e else np.nan
                if not secs or not total or np.isnan(total) or total <= 0:
                    row.append([np.nan, np.nan])
                    continue
                comp = e.loc[secs, "D1"].sum() if "D1" in e else np.nan
                tax = e.loc[secs, "D29X39"].sum() if "D29X39" in e else np.nan
                row.append([comp / total, tax / total])
            rows.append(row)
        return rows

    own = shares(members)
    reporters = [iso3 for iso3 in members if iso3 in v.index.get_level_values(0)]
    if len(reporters) >= MIN_COUNTRIES:
        return np.nanmedian(np.array(own, dtype=float), axis=0), f"the median over the {len(reporters)} economies " \
            f"of the group Table 6 reports ({', '.join(sorted(reporters))})"
    developed = [c for c, lv in levels.items() if lv == "developed"]
    base = np.nanmedian(np.array(shares(developed), dtype=float), axis=0)
    labour = at_year(RAW / "owid" / "labor-share-of-gdp.csv")
    ratio = float(labour.reindex(members).median() / labour.reindex(developed).median())
    base[:, 0] *= ratio
    return base, (f"Table 6 reports {len(reporters)} economies of the group, too few, so the developed group's "
                  f"medians, compensation scaled by the group's median labour share over the developed group's "
                  f"({ratio:.3f}, Penn World Table labour share through Our World in Data)")


def read_primitive(path: Path, pid: str):
    for p in tomllib.loads(path.read_text())["primitive"]:
        if p["id"] == pid:
            return p["value"]
    fail(f"{pid} is not in {path}")


def group_prices(level: str) -> np.ndarray:
    """Each product's price a unit at the opening: its world price times the group's price level."""
    shared = read_primitive(ROOT / "data" / "shared" / "GDS.toml", "GDS.opening_price")["values"]
    levels = profile_files.get(level, "GDS.price_level")["values"]
    return np.array([float(a) * float(b) for a, b in zip(shared, levels)])


def group_flows(level: str, members: dict, levels: pd.Series) -> dict:
    """The group's flows in shares of GDP, and what each part rests on."""
    med = lambda k: np.median(np.stack([e[k] for e in members.values()]), axis=0)
    coef = med("coef")
    # The products' inputs of products are the ways' at the group's prices.
    ways = np.array(profile_files.get(level, "TEC.inputs")["values"], dtype=float)
    price = group_prices(level)
    coef[:NP, :NP] = ways * price[:, None] / price[None, :]
    tax_rate = med("tax_rate")
    added = 1.0 - coef.sum(axis=0) - tax_rate
    if np.any(added <= 0):
        bad = [ACTIVITIES[j] for j in np.where(added <= 0)[0]]
        fail(f"{level}: inputs and taxes exceed output, leaving no value added, in {bad}")
    weight = med("final_weight")
    weight /= weight.sum()
    final_tax = med("final_tax")
    comp = med("composition")
    comp /= comp.sum(axis=0, keepdims=True)
    # Households' products are their budget shares, beside what they spend on finance, rents and public services.
    shares, shares_note = derive_hh.budget_shares(level)
    services = comp[NP:, 0].sum()
    comp[:NP, 0] = shares * (1.0 - services)
    spent = weight / (1.0 + final_tax)
    final = comp * spent[None, :]
    output = np.linalg.solve(np.eye(N) - coef, final.sum(axis=1))
    if np.any(output < 0):
        fail(f"{level}: an activity's output below nothing: {output}")
    parts, parts_note = value_added_parts(list(members), levels)
    parts = np.where(np.isnan(parts), np.nanmedian(parts, axis=0, keepdims=True), parts)
    if np.any(parts.sum(axis=1) >= 1.0) or np.any(parts < -1.0):
        fail(f"{level}: compensation and taxes on production take all of value added")
    va = added * output
    split = np.column_stack([parts[:, 0] * va, (1.0 - parts[:, 0] - parts[:, 1]) * va, parts[:, 1] * va])
    return {"coef": coef, "tax_rate": tax_rate, "added": added, "final": final, "final_tax": final_tax,
            "output": output, "split": split, "parts_note": parts_note, "weight": weight, "parts": parts,
            "composition": comp, "shares_note": shares_note}


def check_flows(level: str, f: dict) -> None:
    """The identities: supply is uses, output is inputs, taxes and value added, and GDP by production, expenditure
    and income agree."""
    x, a, fin = f["output"], f["coef"], f["final"]
    tol = 10.0 ** -(EXP - 3)
    supply = x - a @ x - fin.sum(axis=1)
    if np.max(np.abs(supply)) > tol:
        fail(f"{level}: a product's supply is not its uses: {np.max(np.abs(supply))}")
    column = x - (a * x[None, :]).sum(axis=0) - f["tax_rate"] * x - f["split"].sum(axis=1)
    if np.max(np.abs(column)) > tol:
        fail(f"{level}: an activity's output is not its inputs and value added: {np.max(np.abs(column))}")
    taxes = float((f["tax_rate"] * x).sum() + (fin.sum(axis=0) * f["final_tax"]).sum())
    production = float(f["split"].sum()) + taxes
    expenditure = float(fin.sum() + (fin.sum(axis=0) * f["final_tax"]).sum())
    if abs(production - 1.0) > tol or abs(expenditure - 1.0) > tol:
        fail(f"{level}: GDP by production {production} and expenditure {expenditure} are not one")


# ---------------------------------------------------------------------------------------------------------------
# The stocks.

def gdp_local() -> pd.Series:
    return at_year(RAW / "wdi" / "NY.GDP.MKTP.CN.csv")


def sector_accounts() -> tuple:
    """Each economy's financial and non-financial balance sheets over its GDP."""
    gdp = gdp_local()
    fin = pd.read_csv(RAW / "oecd" / f"financial_balance_sheets_{YEAR}.csv")
    fin = fin[fin.iso3.isin(gdp.index)].copy()
    fin["share"] = fin.value * 10.0 ** fin.unit_mult / fin.iso3.map(gdp)
    nonfin = pd.read_csv(RAW / "oecd" / f"nonfinancial_balance_sheets_{YEAR}.csv")
    nonfin = nonfin[nonfin.iso3.isin(gdp.index)].copy()
    nonfin["share"] = nonfin.value * 10.0 ** nonfin.unit_mult / nonfin.iso3.map(gdp)
    return fin, nonfin


def splits(fin: pd.DataFrame, nonfin: pd.DataFrame) -> pd.DataFrame:
    """Per economy, the shares of who holds what and the real assets per unit of GDP."""
    f = fin.pivot_table(index="iso3", columns=["sector", "entry", "instrument"], values="share", aggfunc="sum")
    g = lambda s, e, i: f.get((s, e, i), pd.Series(np.nan, index=f.index))
    out = pd.DataFrame(index=f.index)
    cur_h, cur_f = g("S1M", "A", "F21"), g("S11", "A", "F21")
    out["currency_households"] = cur_h / (cur_h + cur_f)
    dep = {s: g(s, "A", "F22").fillna(0) + g(s, "A", "F29").fillna(0) for s in ("S1M", "S11", "S13")}
    total = sum(dep.values())
    out["deposits_households"] = dep["S1M"] / total
    out["deposits_firms"] = dep["S11"] / total
    loans, bonds = g("S11", "L", "F4"), g("S11", "L", "F3")
    out["firm_debt_loans"] = loans / (loans + bonds)
    out["government_paper_banks"] = g("S122", "A", "F3") / g("S1", "A", "F3")
    n = nonfin.pivot_table(index="iso3", columns=["sector", "asset"], values="share", aggfunc="sum")
    h = lambda s, a: n.get((s, a), pd.Series(np.nan, index=n.index))
    real = pd.DataFrame(index=n.index)
    real["dwellings"] = h("S1M", OECD_DWELLINGS)
    real["households_land"] = h("S1M", OECD_LAND)
    real["inventories"] = h("S11", OECD_INVENTORIES)
    real["firms_land"] = h("S11", OECD_LAND)
    real["government_fixed"] = sum(h("S13", a).fillna(0) for a in OECD_FIXED)
    return out.join(real, how="outer")


def broad() -> pd.DataFrame:
    """The instruments' levels from broad sources, per economy, over GDP."""
    gdp = gdp_local()
    b = pd.DataFrame({
        "currency": at_year(RAW / "imf_sdmx" / "CIC.csv") / gdp,
        "deposits": at_year(RAW / "wb" / "GFDD.OI.02.csv", 1 / PERCENT),
        "household_debt": at_year(RAW / "imf" / "HH_LS.csv", 1 / PERCENT),
        "firm_debt": at_year(RAW / "imf" / "NFC_LS.csv", 1 / PERCENT),
        "government_debt": at_year(RAW / "imf" / "GGXWDG_NGDP.csv", 1 / PERCENT),
        "bank_assets": at_year(RAW / "wb" / "GFDD.DI.02.csv", 1 / PERCENT),
        "reserves_to_assets": at_year(RAW / "wb" / "FD.RES.LIQU.AS.ZS.csv", 1 / PERCENT),
        "capital_to_assets": at_year(RAW / "wdi" / "FB.BNK.CAPA.ZS.csv", 1 / PERCENT),
    })
    pwt = pd.read_csv(RAW / "pwt" / "capital.csv")
    pwt = pwt[(pwt.year - YEAR).abs() <= NEAR_YEARS].sort_values("year").groupby("iso3").last()
    b["structures"] = pwt.Nc_Struc / pwt.v_gdp
    return b


def group_median(frame: pd.DataFrame, column: str, members: list, developed: list, scale_by=None):
    """The group's median of a column over its members that report it; with too few, the developed group's, times
    the group's median of `scale_by` over the developed group's where one is named."""
    own = frame.reindex(members)[column].dropna()
    if len(own) >= MIN_COUNTRIES:
        return float(own.median()), f"the median over {len(own)} economies"
    dev = frame.reindex(developed)[column].dropna()
    if len(dev) == 0:
        fail(f"no economy reports {column}")
    value = float(dev.median())
    note = f"the developed group's median over {len(dev)} economies, the group reporting {len(own)}"
    if scale_by is not None:
        s_own = frame.reindex(members)[scale_by].dropna()
        s_dev = frame.reindex(developed)[scale_by].dropna()
        ratio = float(s_own.median() / s_dev.median())
        value *= ratio
        note += f", scaled by the group's median {scale_by} over the developed group's ({ratio:.3f})"
    return value, note


def group_stocks(level: str, members: list, developed: list, plant: np.ndarray) -> dict:
    b = broad().join(splits(*sector_accounts()), how="outer")
    val, notes = {}, {}
    for col in ["currency", "deposits", "household_debt", "firm_debt", "government_debt", "reserves_to_assets",
                "capital_to_assets", "currency_households", "deposits_households", "deposits_firms",
                "firm_debt_loans", "government_paper_banks"]:
        val[col], notes[col] = group_median(b, col, members, developed)
    for col in ["dwellings", "households_land", "inventories", "firms_land", "government_fixed"]:
        val[col], notes[col] = group_median(b, col, members, developed, scale_by="structures")
    m = np.zeros((len(INSTRUMENTS), len(SECTORS)))
    r = {k: i for i, k in enumerate(INSTRUMENTS)}
    cur, dep = val["currency"], val["deposits"]
    m[r["currency"], [H, F, C]] = [cur * val["currency_households"], cur * (1 - val["currency_households"]), -cur]
    dep_g = dep * max(0.0, 1.0 - val["deposits_households"] - val["deposits_firms"])
    m[r["deposits"], [H, F, G, B]] = [dep * val["deposits_households"], dep * val["deposits_firms"], dep_g,
                                      -(dep * (val["deposits_households"] + val["deposits_firms"]) + dep_g)]
    deposits = -m[r["deposits"], B]
    loans_h, debt_f = val["household_debt"], val["firm_debt"]
    loans_f = debt_f * val["firm_debt_loans"]
    bonds_f = debt_f - loans_f
    m[r["loans to households"], [B, H]] = [loans_h, -loans_h]
    m[r["loans to firms"], [B, F]] = [loans_f, -loans_f]
    m[r["firms' bonds"], [H, F]] = [bonds_f, -bonds_f]
    gov = val["government_debt"]
    paper_b = gov * val["government_paper_banks"]
    # Banks' reserves are their ratio of what they lend and hold, reserves included.
    lent = loans_h + loans_f + paper_b
    reserves = lent * val["reserves_to_assets"] / (1 - val["reserves_to_assets"])
    # The central bank holds government paper for its currency and reserves, and lends banks what paper cannot cover.
    paper_c = min(cur + reserves, gov - paper_b)
    cb_loans = cur + reserves - paper_c
    paper_h = gov - paper_b - paper_c
    assets_b = lent + reserves
    equity_b = assets_b * val["capital_to_assets"]
    bonds_b = assets_b - deposits - equity_b - cb_loans
    if bonds_b < 0:
        # Deposits beyond what banks lend are held as more government paper, taken from households'.
        extra = min(-bonds_b, paper_h)
        paper_b, paper_h = paper_b + extra, paper_h - extra
        assets_b += extra
        equity_b = assets_b * val["capital_to_assets"]
        reserves = (lent + extra) * val["reserves_to_assets"] / (1 - val["reserves_to_assets"])
        assets_b = lent + extra + reserves
        paper_c = min(cur + reserves, gov - paper_b)
        cb_loans = cur + reserves - paper_c
        paper_h = gov - paper_b - paper_c
        equity_b = assets_b * val["capital_to_assets"]
        bonds_b = assets_b - deposits - equity_b - cb_loans
    if bonds_b < -1e-12 or paper_h < -1e-12:
        fail(f"{level}: banks' or households' government paper below nothing ({bonds_b}, {paper_h})")
    m[r["government paper"], [H, B, C, G]] = [paper_h, paper_b, paper_c, -gov]
    m[r["reserves"], [B, C]] = [reserves, -reserves]
    m[r["central bank loans to banks"], [C, B]] = [cb_loans, -cb_loans]
    m[r["banks' bonds"], [H, B]] = [bonds_b, -bonds_b]
    m[r["banks' equity"], [H, B]] = [equity_b, -equity_b]
    real = np.zeros((len(REAL), len(SECTORS)))
    real[:len(tec.ASSETS), F] = plant
    real[REAL.index("inventories"), F] = val["inventories"]
    real[REAL.index("firms' land"), F] = val["firms_land"]
    real[REAL.index("dwellings"), H] = val["dwellings"]
    real[REAL.index("households' land"), H] = val["households_land"]
    real[REAL.index("government's fixed assets"), G] = val["government_fixed"]
    firm_worth = real[:, F].sum() + m[:, F].sum()
    if firm_worth < 0:
        fail(f"{level}: firms owe more than they hold ({firm_worth})")
    m[r["firms' equity"], [H, F]] = [firm_worth, -firm_worth]
    return {"financial": m, "real": real, "notes": notes, "values": val}


def check_stocks(level: str, s: dict) -> None:
    """Each instrument's assets are its liabilities; firms and the central bank are worth nothing beyond their
    equity; a sector's net worth is its assets less its liabilities."""
    tol = 10.0 ** -(EXP - 3)
    rows = np.abs(s["financial"].sum(axis=1))
    if rows.max() > tol:
        fail(f"{level}: an instrument's assets are not its liabilities: {INSTRUMENTS[int(rows.argmax())]}")
    worth = s["financial"].sum(axis=0) + s["real"].sum(axis=0)
    for sector in (F, B, C):
        if abs(worth[sector]) > tol:
            fail(f"{level}: {SECTORS[sector]} worth {worth[sector]} beyond their equity")


# ---------------------------------------------------------------------------------------------------------------
# Writing.

# ---------------------------------------------------------------------------------------------------------------
# The shapes.

def productivity_spread() -> dict:
    """Each level's spread of log physical productivity (TFPQ) around its industry's mean, its economy's latest year."""
    d = pd.read_csv(RAW / "state" / "typed_tfp_dispersion.csv")
    d = d[(d.measure == "TFPQ") & (d.statistic == "sd")].sort_values("year")
    return {level: g.iloc[-1] for level, g in d.groupby("level")}


def occupation_pay(levels: pd.Series) -> tuple:
    """Each level's median over its economies of each occupation's mean monthly earnings over all employees', each
    economy at its survey nearest the year that reports the total; and each level's economies."""
    d = pd.read_csv(RAW / "ilo" / "earnings_by_occupation.csv", dtype={"occupation": str})
    d = d[(d.year - YEAR).abs() <= NEAR_YEARS].copy()
    d["distance"] = (d.year - YEAR).abs() * 2 - (d.year > YEAR)
    total = d[d.occupation == "TOTAL"].sort_values("distance").groupby("iso3").first()[["year", "value"]]
    d = d.merge(total.rename(columns={"year": "survey", "value": "total"}), left_on="iso3", right_index=True)
    d = d[(d.year == d.survey) & d.occupation.isin(OCCUPATIONS)].copy()
    d["relative"] = d.value / d.total
    d["level"] = d.iso3.map(levels)
    pay = d.pivot_table(index="occupation", columns="level", values="relative", aggfunc="median").reindex(OCCUPATIONS)
    return pay, d.groupby("level").iso3.unique()


def check_shapes(level: str, spread, pay: pd.Series) -> None:
    if not spread.value > 0:
        fail(f"{level}: no productivity spread")
    if pay.isna().any() or not (pay > 0).all():
        fail(f"{level}: an occupation with no pay: {list(pay[~(pay > 0)].index)}")


def matrix(rows: int, cols: int, values: np.ndarray) -> str:
    grid = "[" + ", ".join("[" + ", ".join(num(v) for v in row) + "]" for row in values) + "]"
    return (f"{{ rows = [{', '.join(str(i) for i in range(rows))}], columns = [{', '.join(str(i) for i in range(cols))}],"
            f" values = {grid}, outside = \"refuse\" }}")


def axis(values: np.ndarray) -> str:
    return (f"{{ axis = [{', '.join(str(i) for i in range(len(values)))}], values = ["
            + ", ".join(num(v) for v in values) + "], outside = \"refuse\" }")


def primitive(pid: str, owner: str, kind: str, source: str, ref: str, value: str) -> list:
    return ["", "[[primitive]]", f'id = "{pid}"', f'kind = "{kind}"', f'owner = "{owner}"', f'source = "{source}"',
            f"source_ref = {json.dumps(ref)}", f"value = {value}"]


def write(level: str, members: dict, f: dict, s: dict, m: dict) -> None:
    icio = m["sources"]["icio"]
    names = ", ".join(sorted(members))
    acts = "; ".join(f"{i} {a}" for i, a in enumerate(ACTIVITIES))
    base = (f"the {YEAR} inter-country input-output tables of the {len(members)} economies of the group they report "
            f"({names}), each use summed over where it came from ({icio['title']}, fetched {icio['fetched']}); the "
            f"economy closed, what it used taken as made at home")
    lines = [f"# The {level} group's flows of {YEAR} in shares of its GDP (spec GEN.15), derived by",
             "# tools/data/derive_economy.py; never edited by hand."]
    lines += primitive(
        "GEN.service_inputs", "GEN", "ENDOWMENT", "measured",
        f"What each activity (columns: {acts}) uses of finance, real estate and public administration "
        f"(rows 0 to 2) per unit of its output, in value; and, for those three activities' columns, what they use of "
        f"every product too (rows 3 to 21, the products in TEC's order): the medians of {base}.",
        matrix(3 + NP, N, np.vstack([f["coef"][NP:, :], f["coef"][:NP, :] * (np.arange(N) >= NP)[None, :]])))
    lines += primitive(
        "GEN.product_taxes", "GEN", "ENDOWMENT", "measured",
        f"Taxes less subsidies on products per unit of output of each activity (axis 0 to 21, the activities' order) and per "
        f"unit spent at basic prices by each final use (22 households, 23 collective consumption, 24 fixed investment, "
        f"25 inventories): the medians of {base}.",
        axis(np.concatenate([f["tax_rate"], f["final_tax"]])))
    lines += primitive(
        "GEN.value_added_parts", "GEN", "ENDOWMENT", "measured",
        f"The shares of each activity's value added (rows, as the activities' axis: {acts}) that are compensation of "
        f"employees (column 0) and other taxes less subsidies on production (column 1), gross operating surplus and "
        f"mixed income the rest: the parts' shares of value added in the OECD's national accounts (Table 6, {YEAR}), "
        f"{f['parts_note']}; an activity no economy reports takes the median over the activities. An activity's value "
        f"added is its output less its inputs and taxes on products, its output what its uses need, (I - A)^-1 times "
        f"the final uses, both computed where they are read.",
        matrix(N, 2, f["parts"]))
    lines += primitive(
        "GEN.final_weights", "GEN", "ENDOWMENT", "measured",
        f"Each final use's weight in GDP at purchasers' prices (axis: 0 households, 1 collective consumption by the "
        f"government and the non-profit institutions serving households, 2 fixed investment, 3 changes in "
        f"inventories): the medians of {base}, renormalised to one; what it spends at basic prices is its weight less "
        f"its taxes on products.",
        axis(f["weight"]))
    lines += primitive(
        "GEN.final_composition", "GEN", "ENDOWMENT", "measured",
        f"What each final use (columns, as GEN.final_weights' axis) spends on each activity (rows, as the activities' "
        f"axis), each column summing to one: the medians of {base}, each renormalised; households' products "
        f"{f['shares_note']}, beside the median parts they spend on finance, rents and public services.",
        matrix(N, len(FINALS), f["composition"]))
    profile_files.put_text(level, "\n".join(lines) + "\n")
    notes = s["notes"]
    oecd = m["sources"].get("oecd_sectors", {})
    src = (f"IMF currency in circulation (MFS) over GDP, {notes['currency']}; World Bank GFDD bank deposits "
           f"(GFDD.OI.02), {notes['deposits']}; IMF Global Debt Database household (HH_LS), {notes['household_debt']}, "
           f"and firm debt (NFC_LS), {notes['firm_debt']}; IMF World Economic Outlook government debt (GGXWDG_NGDP), "
           f"{notes['government_debt']}; World Bank bank liquid reserves to assets (FD.RES.LIQU.AS.ZS), "
           f"{notes['reserves_to_assets']}, and bank capital to assets (FB.BNK.CAPA.ZS), {notes['capital_to_assets']}; "
           f"who holds them from the OECD's financial balance sheets (Table 720, {YEAR}, fetched {oecd.get('fetched')}): "
           f"households' part of currency {notes['currency_households']}, of deposits {notes['deposits_households']}, "
           f"firms' {notes['deposits_firms']}, the government the rest; firms' debt in loans {notes['firm_debt_loans']}, "
           f"the rest bonds; banks' part of government paper {notes['government_paper_banks']}; each observation "
           f"{YEAR}'s or the nearest within {NEAR_YEARS} years")
    closures = ("The central bank holds government paper for its currency and reserves and lends banks what the paper "
                "cannot cover; banks hold reserves at their ratio of what they lend and hold, their equity at their "
                "capital ratio of their assets, and bonds, held by households, that balance them, holding more "
                "government paper where deposits exceed what they lend; firms' equity, held by households, is their "
                "assets less their debts; households hold the rest of each instrument.")
    lines = [f"# The {level} group's balance sheets at the end of {YEAR} in shares of its GDP (spec GEN.15, GEN.4),",
             "# derived by tools/data/derive_economy.py; never edited by hand."]
    lines += primitive(
        "GEN.balance_sheet", "GEN", "ENDOWMENT", "measured",
        f"Each sector's (columns: {'; '.join(f'{i} {x}' for i, x in enumerate(SECTORS))}) holdings of each "
        f"instrument (rows: {'; '.join(f'{i} {x}' for i, x in enumerate(INSTRUMENTS))}) over GDP, assets positive "
        f"and liabilities negative, each row summing to nothing: {src}. {closures}",
        matrix(len(INSTRUMENTS), len(SECTORS), s["financial"]))
    lines += primitive(
        "GEN.real_assets", "GEN", "ENDOWMENT", "measured",
        f"Each sector's (columns as GEN.balance_sheet's) real assets (rows: {'; '.join(f'{i} {x}' for i, x in enumerate(REAL))}) "
        f"over GDP, net, at current prices: the firms' plant by kind their stocks (CAP.stock_per_gdp); their "
        f"inventories {notes['inventories']}, and land {notes['firms_land']}; households' dwellings {notes['dwellings']} "
        f"and land {notes['households_land']}; the government's fixed assets {notes['government_fixed']}; from the "
        f"OECD's balance sheets for non-financial assets (Table 9B, {YEAR}), a group with too few reporters scaled by "
        f"its Penn World Table structures per unit of GDP over the developed group's.",
        matrix(len(REAL), len(SECTORS), s["real"]))
    profile_files.put_text(level, "\n".join(lines) + "\n")


def write_shapes(level: str, spread, pay: pd.Series, economies, m: dict) -> None:
    ilo = m["sources"]["people_ilo"]
    lines = [f"# The {level} group's shapes (spec GEN.15): distributions the opening scales to the matrices' totals,",
             "# derived by tools/data/derive_economy.py; never edited by hand."]
    lines += primitive(
        "GEN.productivity_spread", "GEN", "ENDOWMENT", "estimated",
        f"The standard deviation of firms' log physical productivity (TFPQ) around their four-digit industry's mean, "
        f"manufacturing plants of {spread.iso3} in {spread.year} ({int(spread.plants)} plants) standing for the group: "
        f"{spread.source}.",
        f'"{num(spread.value, 2)}"')
    lines += primitive(
        "GEN.occupation_pay", "GEN", "ENDOWMENT", "measured",
        f"Each occupation's (axis: ISCO-08 major groups, 0 armed forces, 1 managers, 2 professionals, 3 technicians "
        f"and associate professionals, 4 clerical support, 5 service and sales, 6 skilled agricultural, forestry and "
        f"fishery, 7 craft and related trades, 8 plant and machine operators and assemblers, 9 elementary "
        f"occupations) mean monthly earnings of employees over all employees': the median over the group's "
        f"{len(economies)} economies ({', '.join(sorted(economies))}) at each one's survey nearest {YEAR} within "
        f"{NEAR_YEARS} years that reports the total, an occupation an economy does not report left out of its median, "
        f"from ILOSTAT (DF_EAR_EMTA_SEX_OCU_CUR_NB, both sexes, fetched {ilo['fetched']}).",
        axis(pay.to_numpy()))
    profile_files.put_text(level, "\n".join(lines) + "\n")


def main() -> None:
    if "--matrices" not in sys.argv:
        tec.main()
        derive_hh.main()
        derive_cap.main()
    m = json.loads((RAW / "manifest.json").read_text())
    levels = level_of()
    developed = [c for c, lv in levels.items() if lv == "developed"]
    economies = flows_by_economy()
    spread = productivity_spread()
    pay, economies_paid = occupation_pay(levels)
    for level in sorted(set(LEVELS.values())):
        members = {c: e for c, e in economies.items() if levels.get(c) == level}
        f = group_flows(level, members, levels)
        check_flows(level, f)
        plant = np.array(profile_files.get(level, "CAP.stock_per_gdp")["values"], dtype=float)
        s = group_stocks(level, [c for c, lv in levels.items() if lv == level], developed, plant)
        check_stocks(level, s)
        profile_files.retire(level, ["GEN.output", "GEN.value_added", "GEN.final_uses", "HH.budget_shares"])
        write(level, members, f, s, m)
        check_shapes(level, spread[level], pay[level])
        write_shapes(level, spread[level], pay[level], economies_paid[level], m)
        worth = s["financial"].sum(axis=0) + s["real"].sum(axis=0)
        print(f"{level}: output {f['output'].sum():.3f} GDP, compensation {f['split'][:, 0].sum():.3f}, "
              f"households' worth {worth[H]:.3f}, government's {worth[G]:.3f}")


if __name__ == "__main__":
    main()
