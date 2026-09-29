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
import profile_files

ROOT = Path(__file__).resolve().parents[2]
RAW = ROOT / "data" / "sources" / "raw"
NAMES = ROOT / "data" / "names"
EXP = 6
FIRST_YEAR = 2015
MIN_COUNTRIES = 10
MIN_PAIRS = 10

LEVELS = {
    "High income": "developed",
    "Upper middle income": "emerging",
    "Lower middle income": "developing",
    "Low income": "developing",
}

# Each derived value: its register id, the scale it is drawn on, how it is read from the series, and the series. A
# value the group's accounts hold — its debts' and banks' stocks aside, which the draw moves (§10.0a) — is the
# accounts', never drawn beside them.
VALUES = [
    ("GEN.life_expectancy", "log", ["wdi/SP.DYN.LE00.IN"], "years", None),
    ("GEN.fertility", "log", ["wdi/SP.DYN.TFRT.IN"], "births per woman", None),
    ("GEN.share_under_15", "logit_percent", ["wdi/SP.POP.0014.TO.ZS"], "% of population", None),
    ("GEN.share_65_plus", "logit_percent", ["wdi/SP.POP.65UP.TO.ZS"], "% of population", None),
    ("GEN.gdp_per_head", "log", ["wdi/NY.GDP.PCAP.PP.KD"], "constant 2021 international $ (PPP)", None),
    ("GEN.income_gini", "logit_percent", ["wdi/SI.POV.GINI"], "Gini index, 0-100", None),
    ("GEN.top10_wealth_share", "logit_share", ["wid/shweal_p90p100_992_j"], "share of net personal wealth", None),
    ("GEN.employment_rate", "logit_percent", ["wdi/SL.EMP.TOTL.SP.ZS"], "% of population 15+", None),
    ("GEN.unemployment_rate", "logit_percent", ["wdi/SL.UEM.TOTL.ZS"], "% of labour force", None),
    ("GEN.labour_share", "logit_percent", ["owid/labor-share-of-gdp"], "% of GDP", None),
    ("GEN.inflation", "log_growth_percent", ["wdi/FP.CPI.TOTL.ZG"], "% a year", None),
    ("GEN.policy_rate", "identity", ["rates/policy_rate"], "% a year, end of year", None),
    ("GEN.household_debt", "log", ["imf/HH_LS"], "% of GDP", None),
    ("GEN.firm_debt", "log", ["imf/NFC_LS"], "% of GDP", None),
    ("GEN.public_debt", "log", ["imf/GGXWDG_NGDP"], "% of GDP", None),
    ("GEN.bank_capital_ratio", "logit_percent", ["wdi/FB.BNK.CAPA.ZS"], "% of assets", None),
    ("GEN.trade", "log", ["wdi/NE.TRD.GNFS.ZS"], "% of GDP", None),
    ("GEN.bank_concentration5", "logit_percent", ["wb/GFDD.OI.06"], "% of assets, five largest banks", None),
    ("GEN.bank_top3_of_top5", "logit_share", ["wb/GFDD.OI.01", "wb/GFDD.OI.06"], "three largest banks' share of the five's", "ratio"),
    ("GEN.bank_deposits", "log", ["wb/GFDD.OI.02"], "% of GDP", None),
    ("GEN.liquid_reserves", "logit_percent", ["wb/FD.RES.LIQU.AS.ZS"], "% of bank assets", None),
    ("GEN.lending_rate", "identity", ["wdi/FR.INR.LEND"], "% a year", None),
    ("GEN.deposit_rate", "identity", ["wdi/FR.INR.DPST"], "% a year", None),
    ("GEN.growth", "log_growth_percent", ["wb/NY.GDP.MKTP.KD.ZG"], "% a year", None),
    ("GEN.home_ownership", "logit_share", ["housing/home_ownership"], "share of households owning their home", None),
]

# The values each setup choice pins, by level (spec GEN.14, GEN.15); risk appetite moves a PREFERENCE, not a value.
CHOICES = {
    "public_debt": ["GEN.public_debt"],
    "private_debt": ["GEN.household_debt", "GEN.firm_debt"],
    "inequality": ["GEN.income_gini", "GEN.top10_wealth_share"],
    "openness": ["GEN.trade"],
}
DEGREES = ["low", "medium", "high"]


def home_ownership() -> pd.DataFrame:
    """Each economy's share of households owning their home, from the best source it has, in the owner's order:
    the OECD Affordable Housing Database's owners outright and with a mortgage (households);
    ECLAC's owners (households); Eurostat's persons in owner households; and, for economies with none of these, the
    DHS surveys' share of adults 15-49 owning a house alone or jointly, the mean of women's and men's where both are
    surveyed, a person-level proxy for household ownership."""
    ahd = pd.read_csv(RAW / "oecd" / "ahd_tenure.csv")
    ahd = ahd[ahd.tenure.isin(["own_outright", "own_mortgage"])].groupby(["iso3", "year"]).share.sum().reset_index()
    cep = pd.read_csv(RAW / "cepalstat" / "tenure.csv")
    cep = cep[cep.tenure == "owner"][["iso3", "year", "share"]]
    es = pd.read_csv(RAW / "eurostat" / "tenure.csv")
    es = es[es.tenure == "OWN"][["iso3", "year", "share"]]
    dhs = pd.read_csv(RAW / "dhs" / "house_owners.csv").groupby(["iso3", "year"]).share.mean().reset_index()
    out, taken = [], set()
    for d in (ahd, cep, es, dhs):
        d = d[d.year >= FIRST_YEAR]
        d = d[~d.iso3.isin(taken)]
        out.append(d)
        taken |= set(d.iso3)
    return pd.concat(out).rename(columns={"share": "value"})


def policy_rate() -> pd.DataFrame:
    """Each economy's policy rate at the end of each year: the BIS's where it reports the economy, the IMF's
    monetary policy-related rate at the year's last month otherwise."""
    bis = pd.read_csv(RAW / "bis" / "CBPOL.csv")
    imf = pd.read_csv(RAW / "imf_sdmx" / "MFS166.csv")
    return pd.concat([bis, imf[~imf.iso3.isin(set(bis.iso3))]])


COMBINED = {"housing/home_ownership": home_ownership, "rates/policy_rate": policy_rate}
# The sources a combined series reads, as the manifest names them.
COMBINED_SOURCES = {"rates": ["bis", "imf_sdmx"]}


def latest(series: str) -> pd.DataFrame:
    d = COMBINED[series]() if series in COMBINED else pd.read_csv(RAW / f"{series}.csv")
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
    used = sorted({u for name, *_ in stats for s in series[name]
                   for u in COMBINED_SOURCES.get(s.split("/")[0], [s.split("/")[0]])})
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
    profile_files.put_text(level, "\n".join(lines) + "\n")
    return {"values": [s[0] for s in stats], "gaps": gaps, "thin_pairs": len(thin), "moved": moved,
            "stats": {s[0]: (s[1], s[2]) for s in stats}}


def product_sections() -> list:
    """Each product's ISIC sections, from the input-output tables' industries it aggregates; mining support services,
    which the ways count in business services, left in mining's section."""
    from derive_tec import PRODUCTS
    return [sorted({c[0] for c in p[2] if c != "B09"}) for p in PRODUCTS]


def by_product(per_section: dict, sections: list) -> list:
    """A ratio's numerator and denominator summed over each product's sections, where every one of them is reported."""
    out = []
    for secs in sections:
        if all(s in per_section for s in secs):
            num_, den = (sum(per_section[s][k] for s in secs) for k in (0, 1))
            out.append(num_ / den if den > 0 else np.nan)
        else:
            out.append(np.nan)
    return out


def self_employed_by_section() -> dict:
    """Each economy's self-employed — employers and own-account workers, each running a business — and everyone
    employed, by ISIC section, at its latest labour force survey (ILOSTAT, ICSE-93), its survey the one listed last
    where two report that year. The source counts no other status apart, so members of producers' cooperatives and
    contributing family workers are counted with the employees."""
    d = pd.read_csv(RAW / "ilo" / "status_by_activity.csv", dtype={"status": str})
    out = {}
    for (iso3, year, source), g in d.groupby(["iso3", "year", "source"]):
        table = {}
        for sec, h in g.groupby("activity"):
            by = h.groupby("status").value.sum()
            if {"2", "3", "TOTAL"} <= set(by.index) and by["TOTAL"] > 0:
                table[sec] = (float(by["2"] + by["3"]), float(by["TOTAL"]))
        out.setdefault(iso3, {})[(year, source)] = table
    return {iso3: v[max(v)] for iso3, v in out.items()}


def enterprises_by_section() -> dict:
    """Each economy's enterprises and persons employed by ISIC section in its latest year (OECD SDBS), personal
    services the sum of the two divisions reported."""
    d = pd.read_csv(RAW / "sdbs" / "by_activity_size.csv", dtype={"activity": str})
    d = d[d["size"] == "_T"]
    out = {}
    for iso3, g in d.groupby("iso3"):
        g = g[g.year == g.year.max()]
        m = g.pivot_table(index="activity", columns="measure", values="value", aggfunc="sum")
        table = {}
        for sec in m.index:
            if len(sec) == 1 and {"ENTR", "EMPN"} <= set(m.columns) and m.loc[sec, "EMPN"] > 0:
                table[sec] = (float(m.loc[sec, "ENTR"]), float(m.loc[sec, "EMPN"]))
        s = [a for a in m.index if a.startswith("S") and len(a) > 1]
        if s and {"ENTR", "EMPN"} <= set(m.columns):
            table["S"] = (float(m.loc[s, "ENTR"].sum()), float(m.loc[s, "EMPN"].sum()))
        out[iso3] = table
    return out


def write_firms(manifest: dict, levels: pd.Series) -> dict:
    """Firms per person employed and the self-employed's share of the employed, by product. Firms per person employed:
    the developed group's the median of enterprises over persons employed in the product's sections (OECD SDBS), the
    owner's decision (plan section 12), agriculture's, which SDBS does not count, its employers and own-account workers
    over its employed; the emerging and developing groups', which SDBS barely covers, their employers and own-account
    workers over everyone employed in the product's sections (ILOSTAT), the source that decision waited for. The
    self-employed's share, by product, each group's ILOSTAT median."""
    sections = product_sections()
    own = self_employed_by_section()
    ent = enterprises_by_section()
    sdbs, ilo = manifest["sources"]["sdbs"], manifest["sources"]["ilo_status"]
    out = {}
    for level in ["developed", "emerging", "developing"]:
        members = [c for c, lv in levels.items() if lv == level]
        shares = np.array([by_product(own[c], sections) for c in members if c in own], dtype=float)
        share = np.nanmedian(shares, axis=0)
        n_share = int(np.sum(~np.all(np.isnan(shares), axis=1)))
        if level == "developed":
            dens = np.array([by_product(ent[c], sections) for c in members if c in ent], dtype=float)
            density = np.where(np.isnan(np.nanmedian(dens, axis=0)), share, np.nanmedian(dens, axis=0))
            n_dens = int(np.sum(~np.all(np.isnan(dens), axis=1)))
            ref = (f"Derived by tools/data/derive.py: each product's enterprises over persons employed in its ISIC "
                   f"sections (axis: the products' places), the median over the {n_dens} economies of the World Bank's "
                   f"{level} income groups that report them, latest year 2015-2025, from {sdbs['title']} (fetched "
                   f"{sdbs.get('fetched', manifest['fetched'])}); a product whose sections the OECD does not count "
                   f"(crops and livestock) its employers and own-account workers over its employed, the median over "
                   f"{n_share} economies, from {ilo['title']} (fetched {ilo.get('fetched', manifest['fetched'])}).")
        else:
            density = share
            ref = (f"Derived by tools/data/derive.py: each product's employers and own-account workers over everyone "
                   f"employed in its ISIC sections (axis: the products' places), the median over the {n_share} "
                   f"economies of the World Bank's {level} income groups reporting them, each at its latest labour "
                   f"force survey 2015-2025 (ICSE-93), from {ilo['title']} (fetched "
                   f"{ilo.get('fetched', manifest['fetched'])}); the OECD's business statistics, the developed group's "
                   f"source, report too few of the group's economies.")
        if np.any(np.isnan(density)) or np.any(np.isnan(share)):
            raise SystemExit(f"{level}: a product with no firm density or self-employed share")
        out[level] = density
        axis = ", ".join(str(i) for i in range(len(sections)))
        share_ref = (f"Derived by tools/data/derive.py: the share of each product's employed (axis: the products' "
                     f"places) who are employers or own-account workers, in its ISIC sections, the median over the "
                     f"{n_share} economies of the World Bank's {level} income groups reporting them, each at its "
                     f"latest labour force survey 2015-2025 (ICSE-93), from {ilo['title']} (fetched "
                     f"{ilo.get('fetched', manifest['fetched'])}).")
        lines = [
            f"# The {level} group's firm density and owners (spec GEN.2, FRM), derived by tools/data/derive.py.",
            "",
            "[[primitive]]",
            'id = "FRM.firms_per_employed"',
            'kind = "ENDOWMENT"',
            'owner = "FRM"',
            'source = "measured"',
            f"source_ref = {json.dumps(ref)}",
            f'value = {{ axis = [{axis}], values = [{", ".join(num(v) for v in density)}], outside = "refuse" }}',
            "",
            "[[primitive]]",
            'id = "LAB.self_employed_shares"',
            'kind = "ENDOWMENT"',
            'owner = "LAB"',
            'source = "measured"',
            f"source_ref = {json.dumps(share_ref)}",
            f'value = {{ axis = [{axis}], values = [{", ".join(num(v) for v in share)}], outside = "refuse" }}',
        ]
        profile_files.put_text(level, "\n".join(lines) + "\n")
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
    print("firms per person employed: " + "; ".join(f"{k} " + " ".join(f"{v:.3f}" for v in x) for k, x in firms.items()))


if __name__ == "__main__":
    main()
