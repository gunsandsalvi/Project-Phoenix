#!/usr/bin/env python3
"""Derives each country group's firms by industry and size class from the fetched sources in data/sources/raw/.

A firm's industry is drawn at the opening given its size. The share of a size class's firms in each industry is the
group's mix of firms over the industries, tilted by how that industry's firms are spread over the size classes:

- the mix is the group's business owners (employers and own-account workers, ILOSTAT's labour force surveys) by ISIC
  section, each running a firm of their own, a section split among its industries by the OECD's enterprise counts;
- the tilt of an industry toward a size class is its share of that class's enterprises over its share of all
  enterprises (OECD structural business statistics), the median over the economies that report it.

The sections whose firms make no product of the ways (finance, real estate, public administration, households'
employment and extraterritorial bodies) are left out: banks, landlords and the state are carried by their own systems.

    python3 tools/data/derive_frm.py

Writes data/profiles/<level>/FRM_industries.toml.
"""
import json
from pathlib import Path

import numpy as np
import pandas as pd

from derive import LEVELS, RAW

ROOT = Path(__file__).resolve().parents[2]
PROFILES = ROOT / "data" / "profiles"
YEAR = 2022
MIN_COUNTRIES = 10
EXP = 6
# The industries in the order the products declare them, each with the ISIC section its owners are counted in and the
# OECD activities its share of the section is read from (none: the whole section).
INDUSTRIES = [
    ("agriculture", "A", None),
    ("mining", "B", None),
    ("food manufacturing", "C", ["C10", "C11", "C12"]),
    ("consumer goods manufacturing", "C", ["C13", "C14", "C15", "C31", "C32"]),
    ("materials manufacturing", "C", ["C16", "C17", "C18", "C20", "C21", "C22", "C23", "C24"]),
    ("energy", "C", ["C19"]),
    ("capital goods manufacturing", "C", ["C25", "C26", "C27", "C28", "C29", "C30", "C33"]),
    ("utilities", "E", None),
    ("construction", "F", None),
    ("trade", "G", None),
    ("transport", "H", None),
    ("accommodation and food services", "I", None),
    ("business services", "J", None),
    ("education", "P", None),
    ("health", "Q", None),
    ("personal services", "R", None),
]
# Sections whose owners join another's industry: electricity and gas are energy, information and the professional
# and support services are business services, the other services personal.
JOINS = {"D": "energy", "M": "business services", "N": "business services", "S": "personal services"}
# The OECD activities each industry's enterprises are counted from, for its tilt over the size classes.
OECD_OF = {
    "mining": ["B"], "utilities": ["E"], "construction": ["F"], "trade": ["G"], "transport": ["H"],
    "accommodation and food services": ["I"], "business services": ["J", "M", "N"], "education": ["P"],
    "health": ["Q"], "personal services": ["R", "S95", "S96"], "energy": ["C19", "D"],
}
# The size classes by persons employed, each by its smallest size.
SIZES = [("S1T9", 1), ("S10T49", 10), ("S50T249", 50), ("S_GE250", 250)]
OWNERS = ["2", "3"]


def num(x: float) -> str:
    return f"{x:.{EXP}f}"


def nearest_year(years) -> int:
    """The year nearest the snapshot's sources, a later one on a tie."""
    return min(years, key=lambda y: (abs(y - YEAR), -y))


def groups() -> pd.Series:
    c = pd.read_csv(RAW / "wb" / "countries.csv")
    return c.set_index("iso3").income_group.map(LEVELS).dropna()


def oecd_activities(industry: str, section: str, split) -> list:
    return OECD_OF.get(industry) or split or [section]


def enterprises() -> dict:
    """Each reporting economy's enterprises by OECD activity and size class, in its latest year."""
    s = pd.read_csv(RAW / "sdbs" / "by_activity_size.csv")
    s = s[s.measure == "ENTR"]
    out = {}
    for iso3, g in s.groupby("iso3"):
        g = g[g.year == g.year.max()]
        out[iso3] = g.pivot_table(index="activity", columns="size", values="value", aggfunc="sum")
    return out


def tilts(ent: dict) -> tuple:
    """Each industry's share of a size class's enterprises over its share of all, and each manufacturing industry's
    share of its section's enterprises: the medians over the economies that report them. Agriculture, which the OECD
    does not count, has no tilt."""
    per = []
    splits = []
    for iso3, t in ent.items():
        counts = {}
        for name, section, split in INDUSTRIES[1:]:
            acts = oecd_activities(name, section, split)
            if not all(a in t.index for a in acts):
                continue
            counts[name] = t.reindex(acts).sum(min_count=len(acts))
        if len(counts) < len(INDUSTRIES) - 1:
            continue
        frame = pd.DataFrame(counts).T
        if "_T" not in frame.columns or frame["_T"].isna().any():
            continue
        all_share = frame["_T"] / frame["_T"].sum()
        tilt = pd.DataFrame(index=frame.index)
        for code, _ in SIZES:
            if code in frame.columns and not frame[code].isna().any() and frame[code].sum() > 0:
                tilt[code] = (frame[code] / frame[code].sum()) / all_share
            else:
                tilt[code] = np.nan
        per.append(tilt)
        if "C" in t.index and not pd.isna(t.loc["C", "_T"]):
            c = t.loc["C", "_T"]
            splits.append({n: counts[n]["_T"] / c for n, s, sp in INDUSTRIES if s == "C" and sp is not None})
    stack = np.stack([p.reindex(columns=[c for c, _ in SIZES]).values for p in per])
    tilt = pd.DataFrame(np.nanmedian(stack, axis=0), index=per[0].index, columns=[c for c, _ in SIZES])
    split = pd.DataFrame(splits).median()
    return tilt, split / split.sum(), len(per), len(splits)


def owners(level_of: pd.Series, split: pd.Series) -> dict:
    """Each economy's business owners over the industries, as shares, from its survey nearest the snapshot."""
    i = pd.read_csv(RAW / "ilo" / "status_by_activity.csv", dtype={"status": str})
    i = i[i.status.isin(OWNERS)]
    out = {}
    for iso3, g in i.groupby("iso3"):
        if iso3 not in level_of.index:
            continue
        g = g[g.year == nearest_year(set(g.year))]
        source = g.groupby("source").size().sort_index().idxmax()
        g = g[g.source == source]
        by_section = g.groupby("activity").value.sum()
        sections = {s for _, s, _ in INDUSTRIES} | set(JOINS)
        if not sections <= set(by_section.index):
            continue
        mix = {}
        for name, section, sp in INDUSTRIES:
            mix[name] = by_section[section] * (split[name] if sp is not None else 1.0)
        for section, name in JOINS.items():
            mix[name] += by_section[section]
        mix = pd.Series(mix)
        if mix.sum() <= 0:
            continue
        out[iso3] = mix / mix.sum()
    return out


def shares(mix: pd.Series, tilt: pd.DataFrame) -> np.ndarray:
    """Each size class's firms over the industries: the mix times the industry's tilt toward the class, as shares."""
    rows = []
    for code, _ in SIZES:
        t = np.array([1.0 if name == "agriculture" else tilt.loc[name, code] for name, _, _ in INDUSTRIES])
        w = mix.reindex([n for n, _, _ in INDUSTRIES]).values * t
        rows.append(w / w.sum())
    return np.array(rows)


def write_level(level: str, members: dict, tilt: pd.DataFrame, reporting: int, m: dict) -> None:
    ilo, sdbs = m["sources"]["ilo_status"], m["sources"]["sdbs_activity"]
    names = ", ".join(sorted(members))
    mix = pd.DataFrame(members).T.median()
    mix = mix / mix.sum()
    values = shares(mix, tilt)
    thin = "measured" if len(members) >= MIN_COUNTRIES else "estimated"
    ref = (f"The share of each size class's firms (rows: the smallest persons employed of the OECD's classes, "
           f"{', '.join(str(s) for _, s in SIZES)}) in each industry (columns, in the products' order: "
           f"{', '.join(n for n, _, _ in INDUSTRIES)}). The group's mix of firms is the median over the {len(members)} "
           f"economies of its World Bank income groups that ILOSTAT's labour force surveys report ({names}) of their "
           f"employers and own-account workers by ISIC section in the survey nearest {YEAR}, each owner running a firm "
           f"({ilo['title']}, fetched {ilo['fetched']}), manufacturing split by the OECD's enterprise counts; each "
           f"industry's tilt toward a size class is its share of the class's enterprises over its share of all, the "
           f"median over the {reporting} economies the OECD reports ({sdbs['title']}, fetched {sdbs['fetched']}); "
           f"agriculture, which the OECD does not count, is taken as spread over the classes as all firms are. "
           f"Finance, real estate, public administration and households' employment are left out, their parties "
           f"carried by their own systems.")
    table = (f"{{ rows = [{', '.join(str(s) for _, s in SIZES)}], columns = [{', '.join(str(i) for i in range(len(INDUSTRIES)))}], "
             f"values = [" + ", ".join("[" + ", ".join(num(v) for v in row) + "]" for row in values) + "], "
             f"outside = \"refuse\" }}")
    lines = [f"# The {level} group's firms by industry and size (spec GEN.2, FRM.2), derived by tools/data/derive_frm.py;",
             "# never edited by hand.", "", "[[primitive]]", 'id = "FRM.industry_by_size"', 'kind = "ENDOWMENT"',
             'owner = "FRM"', f'source = "{thin}"', f"source_ref = {json.dumps(ref)}", f"value = {table}"]
    (PROFILES / level / "FRM_industries.toml").write_text("\n".join(lines) + "\n")


def main() -> None:
    m = json.loads((RAW / "manifest.json").read_text())
    level_of = groups()
    tilt, split, reporting, _ = tilts(enterprises())
    economies = owners(level_of, split)
    for level in sorted(set(LEVELS.values())):
        members = {c: s for c, s in economies.items() if level_of.get(c) == level}
        write_level(level, members, tilt, reporting, m)
        print(level, len(members))


if __name__ == "__main__":
    main()
