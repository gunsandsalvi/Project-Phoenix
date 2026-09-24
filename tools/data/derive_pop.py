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
import math
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
    return "; ".join(f"{m['sources'][n]['title']} (fetched {m['sources'][n].get('fetched', m['fetched'])})"
                     for n in names)


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


# ---- Households and kin ------------------------------------------------------------------------------------------

TYPES = ["one_person", "couple_only", "couple_children", "single_parent", "extended", "non_relatives"]
SIZES = ["size_1", "size_2_3", "size_4_5", "size_6_plus"]
OLDER = ["one_person", "couple_only", "with_partner", "with_children_under_20", "with_children_20_plus"]
CHILD_BANDS = [0, 20, 40, 60]


def tfr() -> pd.Series:
    """Each economy's total fertility in each year, the sum of its age-specific rates."""
    f = pd.read_csv(RAW / "wpp" / "fertility_by_age.csv")
    return f.set_index(["iso3", "year"]).sum(axis=1) / 1000


def theil_sen(x: np.ndarray, y: np.ndarray) -> float:
    """The median of the slopes between every pair of points with distinct x (Theil 1950, Sen 1968)."""
    i, j = np.triu_indices(len(x), 1)
    dx = x[j] - x[i]
    ok = dx != 0
    return float(np.median((y[j] - y[i])[ok] / dx[ok]))


def log_ratio_fit(d: pd.DataFrame, parts: list, reference: str, g: pd.DataFrame) -> tuple:
    """Each part's log ratio to the reference part, ln(share / reference share), as a line in the log of total
    fertility: one slope for every economy, read within the groups (each group's medians taken out) by Theil-Sen,
    and each group's intercept the median of its economies' residuals. A zero share has no log ratio and leaves that
    economy out of that part's fit."""
    d = d.merge(g, on="iso3")
    x = np.log(d.tfr.to_numpy())
    fits = {}
    for p in parts:
        if p == reference:
            continue
        ok = (d[p] > 0) & (d[reference] > 0)
        y = pd.Series(np.nan, index=d.index)
        y[ok] = np.log(d[p][ok] / d[reference][ok])
        frame = pd.DataFrame({"x": x, "y": y, "level": d.level}).dropna()
        within = frame.groupby("level")[["x", "y"]].transform(lambda v: v - v.median())
        slope = theil_sen(within.x.to_numpy(), within.y.to_numpy())
        intercept = (frame.y - slope * frame.x).groupby(frame.level).median()
        fits[p] = (slope, intercept, len(frame))
    return fits


def households(members_of: dict, m: dict) -> dict:
    g = groups()
    h = pd.read_csv(RAW / "un" / "households.csv")
    t = tfr().rename("tfr")
    h = h.join(t, on=["iso3", "year"])
    typed = h.dropna(subset=TYPES + ["tfr"]).copy()
    typed[TYPES] = typed[TYPES].div(typed[TYPES].sum(axis=1), axis=0)
    typed = typed.sort_values("year").groupby("iso3").last().reset_index()
    sized = h.dropna(subset=SIZES + ["tfr"]).copy()
    sized[SIZES] = sized[SIZES].div(sized[SIZES].sum(axis=1), axis=0)
    sized = sized.sort_values("year").groupby("iso3").last().reset_index()
    type_fit = log_ratio_fit(typed, TYPES, "couple_children", g)
    size_fit = log_ratio_fit(sized, SIZES, "size_2_3", g)
    older = pd.read_csv(RAW / "un" / "older_persons.csv")
    older = older[(older.ages == "65 or over") & (older.sex.isin(["Females", "Males"]))]
    out = {}
    for level, members in members_of.items():
        entries = []
        for name, parts, ref, fit, frame, what in [
            ("DEM.household_types", TYPES, "couple_children", type_fit, typed,
             "households by basic type (one person, couple only, couple with children, single parent with children, "
             "extended family, non-relatives), the unknown left out"),
            ("DEM.household_sizes", SIZES, "size_2_3", size_fit, sized,
             "households by size (1, 2-3, 4-5, 6 or more members)"),
        ]:
            n = int(frame.iso3.isin(members).sum())
            rows = []
            for p in parts:
                if p == ref:
                    rows.append([0.0, 0.0])
                else:
                    slope, intercept, _ = fit[p]
                    rows.append([float(intercept[level]), slope])
            pooled = min(f[2] for f in fit.values())
            ref_text = (f"{what.capitalize()}: each part's log ratio to the {ref.replace('_', ' ')} share as a line in "
                        f"the log of total fertility, the columns its intercept for the group and its slope. The slope "
                        f"is one for every economy, the Theil-Sen median of pairwise slopes over at least {pooled} "
                        f"economies with each group's medians taken out; the intercept is the median residual over the "
                        f"group's {n} economies. Each economy is read at its latest source reporting every part "
                        f"(censuses and surveys), its fertility that year's, from {fetched(m, 'un_households', 'wpp')}. "
                        f"A country's shares are each part's exp(intercept + slope ln TFR) over their sum, at its drawn "
                        f"GEN.fertility.")
            entries.append(entry(name, "ENDOWMENT", "DEM", "measured", ref_text,
                                 table2(range(len(parts)), [0, 1], np.array(rows), "refuse")))
        o = older[older.iso3.isin(members)].sort_values("year").groupby(["iso3", "sex"]).last().reset_index()
        values = np.array([[o[o.sex == sex][c].median() / 100 for sex in ("Females", "Males")] for c in OLDER])
        n = int(o.groupby("sex").size().min())
        older_ref = (f"Share of persons aged 65 and over living alone, as a couple only, with a partner, with a child "
                     f"under 20 and with a child of 20 or over (rows, in that order; the last three overlap), by sex "
                     f"(female, male): the median over at least {n} economies of the group at their latest source, "
                     f"from {fetched(m, 'un_households')}. The group's standard at every drawn value.")
        entries.append(entry("DEM.older_living_arrangements", "ENDOWMENT", "DEM", "measured", older_ref,
                             table2(range(len(OLDER)), [0, 1], values, "refuse")))
        entries.append(kin(level, members, m))
        out[level] = entries
    return out


def kin(level: str, members: set, m: dict) -> str:
    """Each parent age's expected living children by the children's age band: the births its cohort of mothers had at
    each age, from each year's fertility at that age, each child surviving to its age now by its birth year's under-five
    mortality and the snapshot's life table beyond five."""
    f = pd.read_csv(RAW / "wpp" / "fertility_by_age.csv").set_index(["iso3", "year"])
    ind = pd.read_csv(RAW / "wpp" / "indicators.csv").set_index(["iso3", "year"])
    life = pd.read_csv(RAW / "wpp" / "life_table.csv")
    first = int(f.index.get_level_values("year").min())
    tables = []
    for iso in sorted(members):
        if iso not in f.index.get_level_values("iso3"):
            continue
        fi = f.loc[iso]
        q5 = ind.loc[iso].q5
        lx = life[life.iso3 == iso].groupby("age").lx.mean().to_numpy() / 100000
        table = np.zeros((101 - 15, len(CHILD_BANDS)))
        for a in range(15, 101):
            for k in range(0, a - 14):
                mother = a - k
                if mother > 49:
                    continue
                year = max(SNAPSHOT - k, first)
                births = fi.loc[year, f"f{mother}"] / 1000
                survive = (1 - q5.loc[max(SNAPSHOT - k, first)] / 1000) * (lx[k] / lx[5] if k >= 5 else 1.0)
                band = max(i for i, lo in enumerate(CHILD_BANDS) if k >= lo)
                table[a - 15, band] += births * survive
        tables.append(table)
    standard = np.median(np.stack(tables), axis=0)
    ref = (f"Expected living children of a parent of each age 15-100 (rows) by the children's age band from its first "
           f"age (0-19, 20-39, 40-59, 60 and over): for a mother of that age, the births her cohort had at each age "
           f"15-49 by that year's age-specific fertility (the first year's, {first}, for years before it), each child "
           f"surviving to its age now by its birth year's under-five mortality and beyond five by the {SNAPSHOT} life "
           f"table, both sexes; the median over the group's {len(tables)} economies, from {fetched(m, 'wpp')}. As the "
           f"owner decided, a parent's children in other households are these less those living with it; a father is "
           f"read at his own age, as no source gives fathers' fertility by age.")
    return entry("DEM.living_children", "ENDOWMENT", "DEM", "measured", ref,
                 table2(range(15, 101), CHILD_BANDS, standard, "refuse"))


# ---- Income and wealth -------------------------------------------------------------------------------------------

Z90 = 1.2815515655446004  # the standard normal's 90th percentile, where the Pareto top begins
SHARE_GROUPS = ["p0p50", "p50p90", "p90p100", "p99p100"]


def phi(x: float) -> float:
    return 0.5 * math.erfc(-x / math.sqrt(2))


def body_sigma(ratio: float) -> float:
    """The lognormal's sigma whose bottom half holds `ratio` of what its bottom 90% hold: Phi(-s) / Phi(z90 - s),
    which falls as s grows, solved by bisection; none when no sigma gives it."""
    f = lambda s: phi(-s) / phi(Z90 - s) - ratio
    lo, hi = 1e-6, 10.0
    if f(lo) < 0 or f(hi) > 0:
        return float("nan")
    for _ in range(200):
        mid = (lo + hi) / 2
        lo, hi = (mid, hi) if f(mid) > 0 else (lo, mid)
    return (lo + hi) / 2


def tail_alpha(top1: float, top10: float) -> float:
    """The Pareto exponent at which the top hundredth holds `top1 / top10` of the top tenth's share:
    (0.01 / 0.1)^(1 - 1/alpha)."""
    return 1 / (1 - math.log(top1 / top10) / math.log(0.1))


def shapes(what: str, members: set) -> pd.DataFrame:
    w = pd.read_csv(RAW / "wid" / "shares.csv")
    w = w[(w.what == what) & w.iso3.isin(members) & w.group.isin(SHARE_GROUPS)]
    wide = w.pivot_table(index=["iso3", "year"], columns="group", values="share").dropna().reset_index()
    wide = wide.sort_values("year").groupby("iso3").last().reset_index()
    wide["sigma"] = [body_sigma(b / (b + mdl)) if b > 0 else float("nan") for b, mdl in zip(wide.p0p50, wide.p50p90)]
    wide["alpha"] = [tail_alpha(a, b) for a, b in zip(wide.p99p100, wide.p90p100)]
    return wide


def income_wealth(level: str, members: set, m: dict) -> list:
    out = []
    for what, pid, drawn, variable in [
        ("income", "DEM.income_shape", "GEN.income_gini", "sptincj992, pre-tax national income"),
        ("wealth", "DEM.wealth_shape", "GEN.top10_wealth_share", "shwealj992, net personal wealth"),
    ]:
        d = shapes(what, members)
        ok = d.dropna(subset=["sigma", "alpha"])
        ok = ok[ok.alpha > 1]
        sigma, alpha = float(ok.sigma.median()), float(ok.alpha.median())
        threshold = math.exp(sigma * Z90)
        left = len(d) - len(ok)
        ref = (f"The group's shape of {what}: a lognormal body with a Pareto top above its 90th percentile (the "
               f"generator's lognormal_pareto_tail). Each economy's body sigma is solved from the bottom half's share "
               f"of the bottom 90%'s, Phi(-s)/Phi(z90 - s), and its Pareto exponent from the top 1%'s share of the top "
               f"10%'s, 0.1^(1 - 1/alpha); the medians over the group's {len(ok)} economies at their latest year "
               f"2010-2025 ({left} left out, their bottom half holding nothing or less, or their top no finite mean), "
               f"WID {variable}, equal-split adults, from {fetched(m, 'wid_shares')}. The body's median is one; the "
               f"mapping keeps the exponent and the top's start at the body's 90th percentile, solves the body's "
               f"sigma so that the whole distribution's {'Gini' if what == 'income' else 'top tenth share'} is the "
               f"country's drawn {drawn}, and scales it to the country's mean.")
        out.append(entry(pid, "ENDOWMENT", "DEM", "measured", ref,
                         f'{{ family = "lognormal_pareto_tail", discretisation = "equal_shares", mu = "0", '
                         f'sigma = "{num(sigma)}", threshold = "{num(threshold)}", alpha = "{num(alpha)}" }}'))
    return out


# ---- Education, occupation, status -------------------------------------------------------------------------------

EDUCATION_AGES = list(range(15, 101, 5))


def education(level: str, members: set, m: dict) -> list:
    e = pd.read_csv(RAW / "wcde" / "attainment.csv")
    e = e[e.iso3.isin(members)]
    n = e.iso3.nunique()
    out = []
    for sex, name in [("F", "female"), ("M", "male")]:
        x = e[e.sex == sex].pivot_table(index=["iso3", "age"], columns="level", values="percent")
        x = x.div(x.sum(axis=1), axis=0)
        standard = x.groupby("age").median()
        standard = standard.div(standard.sum(axis=1), axis=0).reindex(EDUCATION_AGES)
        ref = (f"Share of {name}s in each five-year age group from its first age (rows; 100: 100 and over) by highest "
               f"level of education (columns: none, incomplete primary, primary, lower secondary, upper secondary, "
               f"short post-secondary, bachelor, master and higher): the median over the group's {n} economies of "
               f"each share in 2020, rescaled to sum to one, from {fetched(m, 'wcde')}. The group's standard at every "
               f"drawn value.")
        out.append(entry(f"DEM.education_{name}", "ENDOWMENT", "DEM", "measured", ref,
                         table2(EDUCATION_AGES, range(8), standard.to_numpy(), "edge")))
    return out


def latest_survey(d: pd.DataFrame) -> pd.DataFrame:
    """Each economy's latest year, and within it the source reporting the most rows, then by name."""
    d = d[d.year == d.groupby("iso3").year.transform("max")]
    size = d.groupby(["iso3", "source"]).size().rename("n").reset_index()
    size = size.sort_values(["iso3", "n", "source"], ascending=[True, False, True]).groupby("iso3").head(1)
    return d.merge(size[["iso3", "source"]], on=["iso3", "source"])


def employment(level: str, members: set, m: dict) -> list:
    out = []
    for name, pid, classes, what in [
        ("occupation", "LAB.occupation_shares", [str(c) for c in range(10)],
         "ISCO-08 major group (rows: 0 armed forces, 1 managers, 2 professionals, 3 technicians and associate "
         "professionals, 4 clerical support, 5 service and sales, 6 skilled agricultural, forestry and fishery, 7 "
         "craft and related trades, 8 plant and machine operators and assemblers, 9 elementary occupations)"),
        ("status", "LAB.status_shares", ["1", "2", "3", "4", "5"],
         "status in employment, ICSE-93 (rows: 1 employees, 2 employers, 3 own-account workers, 4 members of "
         "producers' cooperatives, 5 contributing family workers)"),
    ]:
        d = pd.read_csv(RAW / "ilo" / f"{name}.csv", dtype={"class": str})
        d = latest_survey(d[d.iso3.isin(members) & d["class"].isin(classes)])
        cols = []
        for sex in ("F", "M"):
            x = d[d.sex == sex].pivot_table(index="iso3", columns="class", values="employed").reindex(columns=classes)
            x = x.fillna(0).div(x.fillna(0).sum(axis=1), axis=0)
            med = x.median()
            cols.append((med / med.sum()).to_numpy())
        n = d.iso3.nunique()
        ref = (f"Share of the employed by {what}, by sex (columns: female, male): the median over the group's {n} "
               f"economies of each share at their latest survey or census 2010-2025 (a class it does not report "
               f"counted as none), the medians rescaled to sum to one, from {fetched(m, 'ilo_employment')}. The "
               f"group's standard at every drawn value.")
        out.append(entry(pid, "ENDOWMENT", "LAB", "measured", ref,
                         table2([int(c) for c in classes], [0, 1], np.column_stack(cols), "refuse")))
    return out


# ---- Tenure, housing costs, accounts and borrowing ---------------------------------------------------------------

FIRST_YEAR = 2015


def logit(p: pd.Series) -> pd.Series:
    p = p.where((p > 0) & (p < 1))
    return np.log(p / (1 - p))


def last_value(d: pd.DataFrame) -> pd.Series:
    d = d[d.year >= FIRST_YEAR].sort_values("year")
    return d.groupby("iso3").value.last()


def gdp_per_head() -> pd.Series:
    d = pd.read_csv(RAW / "wdi" / "NY.GDP.PCAP.PP.KD.csv")
    return np.log(last_value(d))


def standard_or_fit(values: pd.Series, members: set, x: pd.Series) -> tuple:
    """A share's logit for the group: the median over its economies when at least MIN_COUNTRIES report it, with no
    slope; otherwise, as the owner decided for values a group lacks, the Theil-Sen line in the log of GDP per head
    across every economy that reports it, its intercept their median residual. Returns (intercept, slope, count,
    the GDP per head range of the economies fitted over, or None)."""
    y = logit(values).dropna()
    own = y[y.index.isin(members)]
    if len(own) >= MIN_COUNTRIES:
        return float(own.median()), 0.0, len(own), None
    both = pd.concat([y.rename("y"), x.rename("x")], axis=1).dropna()
    slope = theil_sen(both.x.to_numpy(), both.y.to_numpy())
    span = (math.exp(both.x.min()), math.exp(both.x.max()))
    return float((both.y - slope * both.x).median()), slope, len(both), span


def tenure_parts() -> dict:
    ahd = pd.read_csv(RAW / "oecd" / "ahd_tenure.csv").pivot_table(index=["iso3", "year"], columns="tenure",
                                                                    values="share").reset_index()
    es = pd.read_csv(RAW / "eurostat" / "tenure.csv").pivot_table(index=["iso3", "year"], columns="tenure",
                                                                   values="share").reset_index()
    cost = pd.read_csv(RAW / "oecd" / "ahd_cost.csv")

    def series(frame, value):
        return frame.assign(value=value).dropna(subset=["value"])[["iso3", "year", "value"]]

    def first_of(*frames):
        out, taken = [], set()
        for f in frames:
            v = last_value(f)
            out.append(v[~v.index.isin(taken)])
            taken |= set(v.index)
        return pd.concat(out)

    return {
        "mortgaged": first_of(series(ahd, ahd.own_mortgage / (ahd.own_mortgage + ahd.own_outright)),
                              series(es, es.OWN_L / es.OWN)),
        "subsidised": first_of(series(ahd, ahd.rent_subsidised / (ahd.rent_subsidised + ahd.rent_private)),
                               series(es, es.RENT_FR / es.RENT)),
        "mortgage_burden": last_value(cost[cost.measure == "mortgage_burden"].rename(columns={"share": "value"})),
        "rent_burden": last_value(cost[cost.measure == "rent_burden"].rename(columns={"share": "value"})),
    }


def fit_rows(parts: dict, names: list, members: set, x: pd.Series) -> tuple:
    rows, notes = [], []
    for name in names:
        a, b, n, span = standard_or_fit(parts[name], members, x)
        rows.append([a, b])
        where = f"fitted over {n} economies of GDP per head {span[0]:,.0f} to {span[1]:,.0f}" if span else \
            f"the median over {n} economies"
        notes.append(f"{name.replace('.', ' ').replace('_', ' ')}: {where}")
    return np.array(rows), "; ".join(notes)


def housing(level: str, members: set, m: dict) -> list:
    parts = tenure_parts()
    x = gdp_per_head()
    names = ["mortgaged", "subsidised", "mortgage_burden", "rent_burden"]
    rows, notes = fit_rows(parts, names, members, x)
    ref = (f"Logits of four shares as lines in the log of GDP per head (PPP, constant 2021 dollars), the columns "
           f"intercept and slope; a country's share is the inverse logit at its drawn GEN.gdp_per_head. Rows: 0 owners "
           f"with a mortgage among owners; 1 subsidised or reduced-rent tenants among tenants; 2 the median mortgage "
           f"burden (principal and interest over disposable income) of owners with a mortgage; 3 the median rent "
           f"burden of tenants. Each economy at its latest year 2015-2025, from the OECD Affordable Housing Database "
           f"(households), else Eurostat's EU-SILC (persons); where the group has at least {MIN_COUNTRIES} economies "
           f"the row is their median with no slope, otherwise, as the owner decided for values a group lacks, the "
           f"Theil-Sen line across every economy that has it ({notes}), from {fetched(m, 'housing', 'wdi')}. "
           f"Owners' and tenants' shares of households are GEN.home_ownership's; a mortgage's rate and remaining "
           f"term have no source here.")
    return [entry("HSG.tenure_and_costs", "ENDOWMENT", "HSG", "measured", ref,
                  table2(range(len(names)), [0, 1], rows, "refuse"))]


def banking(level: str, members: set, m: dict) -> list:
    parts = {code: last_value(pd.read_csv(RAW / "findex" / f"{code}.csv").rename(columns={"share": "value"}))
             for code in ["account.t.d", "fin17a", "fin22a"]}
    x = gdp_per_head()
    rows, notes = fit_rows(parts, list(parts), members, x)
    ref = (f"Logits of three shares of adults 15 and over as lines in the log of GDP per head (PPP, constant 2021 "
           f"dollars), the columns intercept and slope: 0 holding an account at a bank or other financial institution "
           f"or a mobile money provider; 1 having saved at a bank in the past year; 2 having borrowed from a bank in "
           f"the past year. Each economy at its latest survey wave 2015-2025; the group's median with no slope where "
           f"at least {MIN_COUNTRIES} economies report it, otherwise the Theil-Sen line across all ({notes}), from "
           f"{fetched(m, 'findex', 'wdi')}.")
    return [entry("BNK.accounts_and_borrowing", "ENDOWMENT", "BNK", "measured", ref,
                  table2(range(len(parts)), [0, 1], rows, "refuse"))]


def main() -> None:
    g = groups()
    m = manifest()
    members_of = {level: set(g[g.level == level].iso3) for level in sorted(set(LEVELS.values()))}
    hh = households(members_of, m)
    for level, members in members_of.items():
        dem = demography(level, members, m) + hh[level] + education(level, members, m)
        write(level, "DEM", f"# The {level} group's demography and households (spec POP.3, POP.4, GEN.2), derived "
                            "by tools/data/derive_pop.py; never edited by hand.", dem)
        money = income_wealth(level, members, m)
        write(level, "HH", f"# The {level} group's shapes of income and wealth (spec GEN.2), derived by "
                           "tools/data/derive_pop.py; never edited by hand.", money)
        lab = employment(level, members, m)
        write(level, "LAB", f"# The {level} group's employment by occupation and status (spec GEN.2, LAB), derived "
                            "by tools/data/derive_pop.py; never edited by hand.", lab)
        hsg = housing(level, members, m)
        write(level, "HSG", f"# The {level} group's tenure and housing costs (spec GEN.2, HSG), derived by "
                            "tools/data/derive_pop.py; never edited by hand.", hsg)
        bnk = banking(level, members, m)
        write(level, "BNK", f"# The {level} group's accounts and borrowing (spec GEN.2, BNK), derived by "
                            "tools/data/derive_pop.py; never edited by hand.", bnk)
        print(level, [e.split('"')[1] for e in dem + money + lab + hsg + bnk])


if __name__ == "__main__":
    main()
