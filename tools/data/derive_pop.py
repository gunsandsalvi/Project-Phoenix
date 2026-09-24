#!/usr/bin/env python3
"""Derives the opening population's tables for each country group from the fetched sources in data/sources/raw/.

Each table is the group's standard: a statistic over the group's economies, robust to any one of them (a median),
taken from the latest observation each economy has. What a country draws from it, given its derived values, is the
table's declared mapping, named in its source note and applied by the generator. Nothing is tuned: rerunning on the
same files gives the same tables.

    python3 tools/data/derive_pop.py

The tables are written to data/profiles/<level>/gen/, the opening's distributions and the technology they rest on.
"""
import json
from pathlib import Path

import numpy as np
import pandas as pd

from derive import LEVELS, RAW

ROOT = Path(__file__).resolve().parents[2]
PROFILES = ROOT / "data" / "profiles"
SEXES = ["female", "male"]
SNAPSHOT = 2023
DISABILITY_BANDS = ["15-24", "25-54", "55-64", "GE65"]
BAND_AGES = {"15-24": (15, 24), "25-54": (25, 54), "55-64": (55, 64), "GE65": (65, 100)}
MIN_COUNTRIES = 10


def groups() -> pd.DataFrame:
    c = pd.read_csv(RAW / "wb" / "countries.csv")
    c["level"] = c.income_group.map(LEVELS)
    return c[["iso3", "level"]].dropna()


def manifest() -> dict:
    return json.loads((RAW / "manifest.json").read_text())


def fetched(m: dict, *names: str) -> str:
    return "; ".join(f"{m['sources'][n]['title']} (fetched {m['sources'][n]['fetched']})" for n in names)


def num(x: float, places: int = 6) -> str:
    return f"{x:.{places}f}"


def grid(values: np.ndarray, places: int = 6) -> str:
    return "[" + ", ".join("[" + ", ".join(num(v, places) for v in row) + "]" for row in values) + "]"


def entry(pid: str, kind: str, owner: str, source: str, ref: str, value: str) -> str:
    ref = ref.replace('"', "'")
    return (f'[[primitive]]\nid = "{pid}"\nkind = "{kind}"\nowner = "{owner}"\nsource = "{source}"\n'
            f'source_ref = "{ref}"\nvalue = {value}\n')


def table2(rows, columns, values: np.ndarray, outside: str, places: int = 6) -> str:
    return (f"{{ rows = [{', '.join(str(r) for r in rows)}], columns = [{', '.join(str(c) for c in columns)}], "
            f"values = {grid(values, places)}, outside = \"{outside}\" }}")


def write(level: str, name: str, header: str, entries: list) -> None:
    path = PROFILES / level / "gen" / f"{name}.toml"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(header + "\n\n" + "\n".join(entries))


# ---- Demography --------------------------------------------------------------------------------------------------

def brass(lx: np.ndarray) -> np.ndarray:
    """Brass's logit of survivorship, 0.5 ln((1 - l) / l)."""
    return 0.5 * np.log((1 - lx) / lx)


def life_expectancy(lx: np.ndarray) -> float:
    """e0 from survivorship at single ages 0..100 (l0 = 1): each year lived whole by those alive at its end and half
    by those who die in it; no one survives beyond 100 a further year."""
    lx = np.append(lx, 0.0)
    return float(((lx[:-1] + lx[1:]) / 2).sum())


def demography(level: str, members: set, m: dict) -> list:
    life = pd.read_csv(RAW / "wpp" / "life_table.csv")
    life = life[life.iso3.isin(members)]
    ages = sorted(life.age.unique())
    logit = {}
    for sex in SEXES:
        s = life[life.sex == sex].pivot(index="iso3", columns="age", values="lx") / 100000
        logit[sex] = brass(s[[a for a in ages if a > 0]].to_numpy())
    n = len(life.iso3.unique())
    standard = np.column_stack([np.median(logit[sex], axis=0) for sex in SEXES])
    lx = 1 / (1 + np.exp(2 * standard))
    e0 = [life_expectancy(np.concatenate([[1.0], lx[:, i]])) for i in range(2)]
    ind = pd.read_csv(RAW / "wpp" / "indicators.csv")
    ind = ind[ind.iso3.isin(members)]
    srb = float(ind[ind.year == ind.year.max()].srb.median())
    pop = pd.read_csv(RAW / "wpp" / "population_by_age.csv")
    pop = pop[pop.iso3.isin(members)]
    shares = []
    for iso, p in pop.groupby("iso3"):
        p = p.sort_values("age")
        both = np.column_stack([p.female.to_numpy(), p.male.to_numpy()])
        shares.append(both / both.sum())
    age_standard = np.median(np.stack(shares), axis=0)
    age_standard = age_standard / age_standard.sum()
    life_ref = (f"Brass's logit of survivorship to each age 1-100, 0.5 ln((1 - l)/l), by sex (female, male): the median "
                f"over the {n} economies of the group of each age's logit in their {SNAPSHOT} complete "
                f"life tables, from {fetched(m, 'wpp')}. The mapping is Brass's relational model (Brass 1971) with "
                f"slope one: a country's logit is the standard's plus one level, the same for both sexes, solved so "
                f"that its life expectancy at birth, the sexes weighted by the sex ratio at birth, is its drawn "
                f"GEN.life_expectancy; the standard itself gives {e0[0]:.2f} years for women and {e0[1]:.2f} for men. "
                f"Beyond 100 no one survives a further year.")
    age_ref = (f"Share of the population at each single age 0-100 (100: 100 and over) and sex (female, male), 1 July "
               f"2023: the median over the {n} economies of the group of each cell's share of its own population, "
               f"the medians rescaled to sum to one, from {fetched(m, 'wpp')}. The mapping to a country rakes the "
               f"standard to its drawn GEN.share_under_15 and GEN.share_65_plus: each of the three bands (0-14, "
               f"15-64, 65 and over) is scaled to its drawn share, keeping the standard's shape within it, which is "
               f"the table closest to the standard in relative entropy with those shares (Deming and Stephan 1940).")
    srb_ref = (f"Males born per 100 females, the median over the {n} economies of the group in 2023, from "
               f"{fetched(m, 'wpp')}.")
    out = [
        entry("DEM.survival_logit_standard", "TECHNOLOGY", "DEM", "measured", life_ref,
              table2(range(1, 101), [0, 1], standard, "refuse")),
        entry("DEM.sex_ratio_at_birth", "TECHNOLOGY", "DEM", "measured", srb_ref, f'"{num(srb, 2)}"'),
        entry("DEM.age_standard", "ENDOWMENT", "DEM", "measured", age_ref,
              table2(range(0, 101), [0, 1], age_standard, "refuse", 9)),
    ]
    return out + disability(level, members, m, age_standard)


def disability(level: str, members: set, m: dict, age_standard: np.ndarray) -> list:
    d = pd.read_csv(RAW / "ilo" / "disability.csv")
    d = d[d.iso3.isin(members)]
    latest = d.groupby("iso3").year.max().rename("last")
    d = d.join(latest, on="iso3")
    d = d[d.year == d["last"]]
    # One survey per economy-year: the one reporting the most rows, then by name, so the pick is the data's own.
    size = d.groupby(["iso3", "source"]).size().rename("n").reset_index()
    size = size.sort_values(["iso3", "n", "source"], ascending=[True, False, True]).groupby("iso3").head(1)
    d = d.merge(size[["iso3", "source"]], on=["iso3", "source"])
    d["share"] = d.disabled / d.total
    prevalence = np.full((len(DISABILITY_BANDS), 2), np.nan)
    counts = []
    for i, band in enumerate(DISABILITY_BANDS):
        for j, sex in enumerate(["F", "M"]):
            s = d[(d.age == band) & (d.sex == sex)].share.dropna()
            counts.append(len(s))
            if len(s) >= MIN_COUNTRIES:
                prevalence[i, j] = s.median()
    n = min(counts)
    if n < MIN_COUNTRIES:
        return []
    # Each band's mean age in the group's standard population, so the onset hazard is read between the ages at which
    # the prevalences are observed rather than at assumed midpoints.
    mean_age = np.zeros((len(DISABILITY_BANDS), 2))
    for i, band in enumerate(DISABILITY_BANDS):
        lo, hi = BAND_AGES[band]
        w = age_standard[lo:hi + 1]
        mean_age[i] = (w * np.arange(lo, hi + 1)[:, None]).sum(axis=0) / w.sum(axis=0)
    onset = np.zeros((len(DISABILITY_BANDS) - 1, 2))
    for i in range(len(DISABILITY_BANDS) - 1):
        span = mean_age[i + 1] - mean_age[i]
        onset[i] = np.log((1 - prevalence[i]) / (1 - prevalence[i + 1])) / span
    ages = [BAND_AGES[b][0] for b in DISABILITY_BANDS]
    prev_ref = (f"Share of persons with a disability (mostly the Washington Group's questions, each survey's own) by "
                f"age band from its first age (15-24, 25-54, 55-64, 65 and over) and sex (female, male): the median "
                f"over at least {n} economies of the group, each at its latest survey 2010-2025, from "
                f"{fetched(m, 'ilo_disability')} (DF_POP_XWAP_SEX_AGE_DSB_NB). Below 15 none is observed.")
    rows = [round(float(a), 2) for a in mean_age[:-1].mean(axis=1)]
    onset_ref = (f"Annual hazard of the onset of lasting disability by age from the row's age (the bands' mean ages in "
                 f"the group's standard population, the sexes averaged) and sex (female, male), derived from "
                 f"DEM.disability_prevalence by the owner's mapping: between consecutive bands' mean ages, "
                 f"ln((1 - P1)/(1 - P2)) over the years between, which holds if no one recovers and the disabled die "
                 f"at the others' rates, both assumptions of the mapping. Before the first row and after the last, "
                 f"the nearest row's hazard. A negative value is a fall in prevalence the mapping cannot give, a "
                 f"finding, not a recovery rate.")
    return [
        entry("DEM.disability_prevalence", "ENDOWMENT", "DEM", "measured", prev_ref,
              table2(ages, [0, 1], prevalence, "refuse")),
        entry("DEM.disability_onset", "TECHNOLOGY", "DEM", "measured", onset_ref,
              table2(rows, [0, 1], onset, "edge")),
    ]


def main() -> None:
    g = groups()
    m = manifest()
    for level in sorted(set(LEVELS.values())):
        members = set(g[g.level == level].iso3)
        dem = demography(level, members, m)
        write(level, "DEM", f"# The {level} group's demography (spec POP.3, POP.4, GEN.2), derived by "
                            "tools/data/derive_pop.py; never edited by hand.", dem)
        print(level, [e.split('"')[1] for e in dem])


if __name__ == "__main__":
    main()
