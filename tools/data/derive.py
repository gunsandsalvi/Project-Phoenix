#!/usr/bin/env python3
"""Derives the country-group profiles and the real-name pre-fills from the fetched series in data/sources/raw/.

For each development level (the World Bank's income groups: high income developed, upper-middle income emerging,
lower-middle and low income developing), each derived value's latest observation per country (2015-2025) is put on
its declared scale (logs for positive amounts, logits for percentages and shares, log ratios for sector shares), and
the group's profile is estimated robustly, so no country's extreme year sets it:

- location: the median; dispersion: the interquartile range over 1.349, the normal's;
- correlation: Spearman's rank correlation over the countries that report both values, mapped to the normal's
  correlation by 2 sin(pi rho / 6); a pair reported by fewer than MIN_PAIRS countries is taken as uncorrelated, and
  says so in the output;
- the nearest positive definite correlation matrix to those estimates (Higham 2002), since pairwise estimates need
  not form one.

A value that fewer than MIN_COUNTRIES countries of a group report is left out of that group's profile and listed
as a gap. Nothing is tuned: rerunning on the same files gives the same tables.

    python3 tools/data/derive.py
"""
import csv
import json
import math
from pathlib import Path

import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parents[2]
RAW = ROOT / "data" / "sources" / "raw"
PROFILES = ROOT / "data" / "profiles"
NAMES = ROOT / "data" / "names"
EXP = 6
MIN_COUNTRIES = 10
MIN_PAIRS = 10

LEVELS = {
    "High income": "developed",
    "Upper middle income": "emerging",
    "Lower middle income": "developing",
    "Low income": "developing",
}

# Each derived value: its register id, the scale it is drawn on, how it is read from the series, and the series.
VALUES = [
    ("GEN.life_expectancy", "log", ["wdi/SP.DYN.LE00.IN"], "years", None),
    ("GEN.fertility", "log", ["wdi/SP.DYN.TFRT.IN"], "births per woman", None),
    ("GEN.share_under_15", "logit_percent", ["wdi/SP.POP.0014.TO.ZS"], "% of population", None),
    ("GEN.share_65_plus", "logit_percent", ["wdi/SP.POP.65UP.TO.ZS"], "% of population", None),
    ("GEN.gdp_per_head", "log", ["wdi/NY.GDP.PCAP.PP.KD"], "constant 2021 international $ (PPP)", None),
    ("GEN.income_gini", "logit_percent", ["wdi/SI.POV.GINI"], "Gini index, 0-100", None),
    ("GEN.household_wealth_to_income", "log", ["wid/whweal_p0p100_999_i"], "ratio to national income", None),
    ("GEN.top10_wealth_share", "logit_share", ["wid/shweal_p90p100_992_j"], "share of net personal wealth", None),
    ("GEN.employment_rate", "logit_percent", ["wdi/SL.EMP.TOTL.SP.ZS"], "% of population 15+", None),
    ("GEN.unemployment_rate", "logit_percent", ["wdi/SL.UEM.TOTL.ZS"], "% of labour force", None),
    ("GEN.labour_share", "logit_percent", ["owid/labor-share-of-gdp"], "% of GDP", None),
    ("GEN.inflation", "log_growth_percent", ["wdi/FP.CPI.TOTL.ZG"], "% a year", None),
    ("GEN.policy_rate", "identity", ["bis/CBPOL"], "% a year, end of year", None),
    ("GEN.household_debt", "log", ["imf/HH_LS"], "% of GDP", None),
    ("GEN.firm_debt", "log", ["imf/NFC_LS"], "% of GDP", None),
    ("GEN.public_debt", "log", ["imf/GGXWDG_NGDP"], "% of GDP", None),
    ("GEN.bank_capital_ratio", "logit_percent", ["wdi/FB.BNK.CAPA.ZS"], "% of assets", None),
    ("GEN.tax_revenue", "logit_percent", ["wdi/GC.TAX.TOTL.GD.ZS"], "% of GDP", None),
    ("GEN.social_spending", "logit_percent", ["owid/social-spending-oecd-longrun"], "% of GDP", None),
    ("GEN.agriculture_to_services", "log", ["wdi/NV.AGR.TOTL.ZS", "wdi/NV.SRV.TOTL.ZS"], "ratio of value added", "ratio"),
    ("GEN.industry_to_services", "log", ["wdi/NV.IND.TOTL.ZS", "wdi/NV.SRV.TOTL.ZS"], "ratio of value added", "ratio"),
    ("GEN.trade", "log", ["wdi/NE.TRD.GNFS.ZS"], "% of GDP", None),
    ("GEN.bank_concentration5", "logit_percent", ["wb/GFDD.OI.06"], "% of assets, five largest banks", None),
    ("GEN.bank_top3_of_top5", "logit_share", ["wb/GFDD.OI.01", "wb/GFDD.OI.06"], "three largest banks' share of the five's", "ratio"),
    ("GEN.bank_assets", "log", ["wb/GFDD.DI.02"], "% of GDP", None),
    ("GEN.bank_deposits", "log", ["wb/GFDD.OI.02"], "% of GDP", None),
    ("GEN.central_bank_assets", "log", ["wb/GFDD.DI.06"], "% of GDP", None),
    ("GEN.liquid_reserves", "logit_percent", ["wb/FD.RES.LIQU.AS.ZS"], "% of bank assets", None),
    ("GEN.lending_rate", "identity", ["wdi/FR.INR.LEND"], "% a year", None),
    ("GEN.deposit_rate", "identity", ["wdi/FR.INR.DPST"], "% a year", None),
    ("GEN.investment", "logit_percent", ["wb/NE.GDI.FTOT.ZS"], "% of GDP", None),
    ("GEN.growth", "log_growth_percent", ["wb/NY.GDP.MKTP.KD.ZG"], "% a year", None),
]

# The values each setup choice pins, by level (spec GEN.14, GEN.15); risk appetite moves a PREFERENCE, not a value.
CHOICES = {
    "public_debt": ["GEN.public_debt"],
    "private_debt": ["GEN.household_debt", "GEN.firm_debt"],
    "inequality": ["GEN.income_gini", "GEN.top10_wealth_share"],
    "openness": ["GEN.trade"],
}
DEGREES = ["low", "medium", "high"]


def latest(series: str) -> pd.DataFrame:
    d = pd.read_csv(RAW / f"{series}.csv")
    d = d.sort_values("year").groupby("iso3").last().reset_index()
    return d.rename(columns={"value": series, "year": f"{series}@year"})


def transform(kind: str, x: pd.Series) -> pd.Series:
    x = x.astype(float)
    if kind == "identity":
        return x
    if kind == "log":
        return np.log(x.where(x > 0))
    if kind == "logit_percent":
        p = (x / 100).where((x > 0) & (x < 100))
        return np.log(p / (1 - p))
    if kind == "logit_share":
        p = x.where((x > 0) & (x < 1))
        return np.log(p / (1 - p))
    if kind == "log_growth_percent":
        return np.log1p((x / 100).where(x > -100))
    raise ValueError(kind)


def table() -> pd.DataFrame:
    countries = pd.read_csv(RAW / "wb" / "countries.csv")
    countries["level"] = countries.income_group.map(LEVELS)
    out = countries[["iso3", "name", "level", "currency"]].copy()
    for name, kind, series, _, how in VALUES:
        frames = [latest(s) for s in series]
        merged = frames[0][["iso3", series[0]]]
        for f, s in zip(frames[1:], series[1:]):
            merged = merged.merge(f[["iso3", s]], on="iso3")
        raw = merged[series[0]] / merged[series[1]] if how == "ratio" else merged[series[0]]
        out = out.merge(pd.DataFrame({"iso3": merged.iso3, name: transform(kind, raw)}), on="iso3", how="left")
    return out


def nearest_correlation(a: np.ndarray, tries: int = 1000) -> np.ndarray:
    """Higham's alternating projections: the nearest correlation matrix, then its eigenvalues kept above a small
    positive floor so it factors."""
    y, ds = a.copy(), np.zeros_like(a)
    for _ in range(tries):
        r = y - ds
        w, v = np.linalg.eigh(r)
        x = v @ np.diag(np.maximum(w, 0)) @ v.T
        ds = x - r
        y = x.copy()
        np.fill_diagonal(y, 1.0)
        if np.linalg.norm(y - x) < 1e-12:
            break
    w, v = np.linalg.eigh(y)
    y = v @ np.diag(np.maximum(w, 1e-4)) @ v.T
    d = np.sqrt(np.diag(y))
    y = y / np.outer(d, d)
    np.fill_diagonal(y, 1.0)
    return y


def profile(group: pd.DataFrame):
    names = [v[0] for v in VALUES if group[v[0]].notna().sum() >= MIN_COUNTRIES]
    gaps = [v[0] for v in VALUES if v[0] not in names]
    stats = []
    for name in names:
        x = group[name].dropna()
        q1, med, q3 = np.percentile(x, [25, 50, 75])
        stats.append((name, med, (q3 - q1) / 1.349, len(x)))
    n = len(names)
    corr = np.eye(n)
    thin = []
    for i in range(n):
        for j in range(i + 1, n):
            both = group[[names[i], names[j]]].dropna()
            if len(both) < MIN_PAIRS:
                thin.append((names[i], names[j], len(both)))
                continue
            rho = both[names[i]].rank().corr(both[names[j]].rank())
            corr[i, j] = corr[j, i] = 2 * math.sin(math.pi * rho / 6)
    fixed = nearest_correlation(corr)
    return stats, fixed, gaps, thin, float(np.max(np.abs(fixed - corr)))


def num(x: float) -> str:
    return f"{x:.{EXP}f}"


def write_profile(level: str, group: pd.DataFrame, manifest: dict) -> dict:
    stats, corr, gaps, thin, moved = profile(group)
    kinds = {v[0]: v[1] for v in VALUES}
    series = {v[0]: v[2] for v in VALUES}
    used = sorted({s.split("/")[0] for name, *_ in stats for s in series[name]})
    sources = "; ".join(f"{manifest['sources'][s]['title']} "
                        f"({manifest['sources'][s].get('release', manifest['sources'][s].get('fetched', manifest['fetched']))})"
                        for s in used)
    ref = (f"Derived by tools/data/derive.py from data/sources/raw/ (fetched {manifest['fetched']}): {sources}. "
           f"{len(group)} economies of the World Bank's {level} income groups; each value's latest observation "
           f"2015-2025 on its scale; median, IQR/1.349, Spearman correlations mapped to the normal's, pairs of fewer "
           f"than {MIN_PAIRS} countries uncorrelated ({len(thin)} pairs), then the nearest positive definite "
           f"correlation matrix (largest change {moved:.3f}). Left out for want of data: {', '.join(gaps) or 'none'}.")
    lines = [
        f"# The {level} country group's joint profile of derived values (spec GEN.15), derived from published data by",
        "# tools/data/derive.py; never edited by hand.",
        "",
        "[[primitive]]",
        'id = "GEN.profile"',
        'kind = "ENDOWMENT"',
        'owner = "GEN"',
        'source = "measured"',
        f"source_ref = {json.dumps(ref)}",
        "[primitive.value]",
        "values = [",
    ]
    for name, med, sd, count in stats:
        lines.append(f'  {{ name = "{name}", transform = "{kinds[name]}", mean = "{num(med)}", sd = "{num(sd)}", '
                     f"countries = {count} }},")
    lines.append("]")
    lines.append("correlation = [")
    for row in corr:
        lines.append("  [" + ", ".join(f'"{num(r)}"' for r in row) + "],")
    lines.append("]")
    path = PROFILES / level / "GEN.toml"
    path.write_text("\n".join(lines) + "\n")
    return {"values": [s[0] for s in stats], "gaps": gaps, "thin_pairs": len(thin), "moved": moved,
            "stats": {s[0]: (s[1], s[2]) for s in stats}}


def write_firms(manifest: dict, levels: pd.Series) -> dict:
    """Firms per person employed in the business economy (enterprises over persons employed, OECD SDBS), per
    country's latest year: measured as the developed group's median, and for the emerging and developing groups,
    which SDBS barely covers, assumed at the median over every economy SDBS reports (the owner's decision, plan
    section 12)."""
    ent, emp = latest("sdbs/ENTR_T"), latest("sdbs/EMPN_T")
    d = ent.merge(emp, on="iso3").merge(levels.rename("level"), left_on="iso3", right_index=True, how="left")
    d["ratio"] = d["sdbs/ENTR_T"] / d["sdbs/EMPN_T"]
    everyone = float(d.ratio.median())
    out = {}
    src = manifest["sources"]["sdbs"]
    for level in ["developed", "emerging", "developing"]:
        group = d[d.level == level].ratio.dropna()
        if len(group) >= MIN_COUNTRIES:
            value, source = float(group.median()), "measured"
            ref = (f"Derived by tools/data/derive.py: the median over {len(group)} economies of the World Bank's {level} "
                   f"income groups of enterprises over persons employed in the business economy except finance, "
                   f"latest year 2015-2025, from {src['title']} (fetched {src.get('fetched', manifest['fetched'])}).")
        else:
            value, source = everyone, "assumed"
            ref = (f"SDBS reports {len(group)} economies of the World Bank's {level} income groups, too few to measure; "
                   f"assumed at the median over all {len(d)} economies it reports ({src['title']}), the owner's "
                   f"decision of 2026-09-24 (plan section 12): firm sizes follow Zipf's law across countries, and the "
                   f"density of firms per worker sets the law's scale. A finding until a source covers the group.")
        out[level] = value
        lines = [
            f"# The {level} group's firm density (spec GEN.2, FRM), derived by tools/data/derive.py; never edited by hand.",
            "",
            "[[primitive]]",
            'id = "FRM.firms_per_employed"',
            'kind = "ENDOWMENT"',
            'owner = "FRM"',
            f'source = "{source}"',
            f"source_ref = {json.dumps(ref)}",
            f'value = "{num(value)}"',
        ]
        (PROFILES / level / "FRM.toml").write_text("\n".join(lines) + "\n")
    return out


def degree(value: float, med: float, sd: float) -> str:
    """Which third of the group's fitted distribution a value falls in."""
    z = (value - med) / sd
    third = 0.4307272992954576  # the standard normal's 2/3 quantile
    return "low" if z < -third else ("high" if z > third else "medium")


def settle(degrees: list) -> str:
    """One degree from the degrees of a choice's values: their mean on low -1, medium 0, high 1, rounded toward
    medium, so a tie between two degrees never depends on an ordering; no values give medium."""
    if not degrees:
        return "medium"
    mean = sum(DEGREES.index(d) - 1 for d in degrees) / len(degrees)
    return DEGREES[int(mean) + 1]


def write_real_names(t: pd.DataFrame, fitted: dict, manifest: dict) -> None:
    lines = [
        "# Real countries' names for a new game's setup (spec GEN.14): each labels the country's institutions and its",
        "# currency, and pre-fills its choices with the levels its own latest data fall in within its group's",
        "# profile (derived by tools/data/derive.py from the same sources); the economy is always generated.",
        "# A choice whose values the country does not report is left at its middle level. Risk appetite has no",
        "# country-level source here and is always left at its middle level.",
        "",
    ]
    for _, c in t.sort_values("iso3").iterrows():
        if not isinstance(c.level, str):
            continue
        stats = fitted[c.level]["stats"]
        levels = {}
        for choice, values in CHOICES.items():
            ds = [degree(c[v], *stats[v]) for v in values if v in stats and not pd.isna(c[v])]
            levels[choice] = settle(ds)
        lines += [
            "[[country]]",
            f'iso3 = "{c.iso3}"',
            f"name = {json.dumps(c['name'])}",
            f"currency = {json.dumps(c.currency if isinstance(c.currency, str) else '')}",
            f'development = "{c.level}"',
            *[f'{k} = "{v}"' for k, v in levels.items()],
            "",
        ]
    NAMES.mkdir(parents=True, exist_ok=True)
    (NAMES / "real.toml").write_text("\n".join(lines))


def main() -> None:
    manifest = json.loads((RAW / "manifest.json").read_text())
    t = table()
    fitted = {}
    for level in ["developed", "emerging", "developing"]:
        fitted[level] = write_profile(level, t[t.level == level], manifest)
        f = fitted[level]
        print(f"{level}: {len(f['values'])} values, gaps {f['gaps']}, {f['thin_pairs']} thin pairs, "
              f"nearest-matrix change {f['moved']:.3f}")
    write_real_names(t, fitted, manifest)
    firms = write_firms(manifest, t.set_index("iso3").level)
    print(f"firms per person employed: {firms}")


if __name__ == "__main__":
    main()
