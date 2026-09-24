#!/usr/bin/env python3
"""Fetches the published data the opening population is derived from, into data/sources/raw/.

The sources surveyed in data/sources/notes/population-sources.md, each kept as a compact CSV of the rows the
derivation reads, with its release and download date in the manifest beside fetch.py's. The files are committed, so
tools/data/derive_pop.py reruns without the network.

    python3 tools/data/fetch_pop.py [--cache DIR] [--only NAME ...]

The downloads are kept in the cache directory (default: a temporary directory) and reused when present.
"""
import argparse
import csv
import datetime
import gzip
import io
import json
import tempfile
from pathlib import Path

from fetch import RAW, cached, get, log

SNAPSHOT = 2023
WPP = "https://population.un.org/wpp/assets/Excel%20Files/1_Indicator%20(Standard)/CSV_FILES/"
WPP_FILES = {
    "life_female": "WPP2024_Life_Table_Complete_Medium_Female_1950-2023.csv.gz",
    "life_male": "WPP2024_Life_Table_Complete_Medium_Male_1950-2023.csv.gz",
    "population": "WPP2024_PopulationBySingleAgeSex_Medium_1950-2023.csv.gz",
    "fertility": "WPP2024_Fertility_by_Age1.csv.gz",
    "indicators": "WPP2024_Demographic_Indicators_Medium.csv.gz",
}
FERTILE = range(15, 50)


def countries() -> set:
    return {r["iso3"] for r in csv.DictReader((RAW / "wb" / "countries.csv").open())}


def table(path: Path, header: list, rows: list) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as f:
        w = csv.writer(f)
        w.writerow(header)
        for r in sorted(rows):
            w.writerow(r)
    return len(rows)


def rows_of(path: Path):
    with gzip.open(path, "rt", encoding="utf-8-sig") as f:
        yield from csv.DictReader(f)


def age(r: dict) -> int:
    return int(r["AgeGrpStart"])


def wpp(cache: Path, manifest: dict, iso3: set) -> None:
    """The snapshot year's single-age life tables by sex and population by single age and sex; age-specific
    fertility for every year, for the children each cohort of mothers bore; under-five mortality for every year and
    the sex ratio at birth."""
    files = {k: cached(cache, v, WPP + v) for k, v in WPP_FILES.items()}
    life = []
    for sex in ("female", "male"):
        for r in rows_of(files[f"life_{sex}"]):
            if r["ISO3_code"] in iso3 and int(r["Time"]) == SNAPSHOT:
                life.append((r["ISO3_code"], sex, age(r), r["qx"], r["lx"]))
    manifest["series"]["wpp/life_table"] = {
        "title": f"Complete life tables by sex, {SNAPSHOT}: probability of dying between ages x and x+1 (qx) and "
                 "survivors of 100,000 born (lx), UN World Population Prospects 2024",
        "rows": table(RAW / "wpp" / "life_table.csv", ["iso3", "sex", "age", "qx", "lx"], life),
    }
    pop = [(r["ISO3_code"], age(r), r["PopMale"], r["PopFemale"]) for r in rows_of(files["population"])
           if r["ISO3_code"] in iso3 and int(r["Time"]) == SNAPSHOT]
    manifest["series"]["wpp/population_by_age"] = {
        "title": f"Population by single age and sex, 1 July {SNAPSHOT}, thousands, UN World Population Prospects 2024",
        "rows": table(RAW / "wpp" / "population_by_age.csv", ["iso3", "age", "male", "female"], pop),
    }
    asfr = {}
    for r in rows_of(files["fertility"]):
        if r["ISO3_code"] in iso3 and int(r["Time"]) <= SNAPSHOT and age(r) in FERTILE:
            asfr.setdefault((r["ISO3_code"], int(r["Time"])), {})[age(r)] = r["ASFR"]
    fert = [(iso, year, *(v[a] for a in FERTILE)) for (iso, year), v in asfr.items()]
    manifest["series"]["wpp/fertility_by_age"] = {
        "title": "Age-specific fertility rates by single age of mother 15-49, births per 1,000 women, 1950-"
                 f"{SNAPSHOT}, UN World Population Prospects 2024",
        "rows": table(RAW / "wpp" / "fertility_by_age.csv", ["iso3", "year", *(f"f{a}" for a in FERTILE)], fert),
    }
    ind = [(r["ISO3_code"], int(r["Time"]), r["Q5"], r["SRB"]) for r in rows_of(files["indicators"])
           if r["ISO3_code"] in iso3 and int(r["Time"]) <= SNAPSHOT]
    manifest["series"]["wpp/indicators"] = {
        "title": "Under-five deaths per 1,000 live births (Q5) and males per 100 females at birth (SRB), 1950-"
                 f"{SNAPSHOT}, UN World Population Prospects 2024",
        "rows": table(RAW / "wpp" / "indicators.csv", ["iso3", "year", "q5", "srb"], ind),
    }
    manifest["sources"]["wpp"] = {
        "title": "UN DESA, World Population Prospects 2024, standard projections (medium variant), CSV files",
        "url": WPP + "<file>",
    }


ILO = "https://sdmx.ilo.org/rest/data/ILO,{flow},1.0/all?startPeriod=2010"
ILO_CSV = "application/vnd.sdmx.data+csv;version=1.0.0"


def ilo_rows(cache: Path, flow: str):
    path = cache / f"{flow}.csv"
    if not path.exists():
        log(f"downloading {flow}")
        path.write_bytes(get(ILO.format(flow=flow), timeout=900, accept=ILO_CSV))
    with path.open(encoding="utf-8-sig") as f:
        yield from csv.DictReader(f)


def ilo_disability(cache: Path, manifest: dict, iso3: set) -> None:
    """Persons of working age and over by disability status, sex and age band, from labour force and household
    surveys; a year's rows come from one survey, named with them."""
    bands = {"AGE_AGGREGATE_Y15-24", "AGE_AGGREGATE_Y25-54", "AGE_AGGREGATE_Y55-64", "AGE_AGGREGATE_YGE65"}
    keep = {}
    for r in ilo_rows(cache, "DF_POP_XWAP_SEX_AGE_DSB_NB"):
        if r["FREQ"] == "A" and r["REF_AREA"] in iso3 and r["AGE"] in bands and r["DSB"] in ("DSB_STATUS_DIS", "DSB_STATUS_TOTAL") \
                and r["SEX"] in ("SEX_M", "SEX_F") and r["OBS_VALUE"]:
            key = (r["REF_AREA"], int(r["TIME_PERIOD"]), r["SEX"][4:], r["AGE"].split("_Y")[-1], r["SOURCE"])
            keep.setdefault(key, {})[r["DSB"][11:]] = r["OBS_VALUE"]
    rows = [(*k[:4], v["DIS"], v["TOTAL"], k[4]) for k, v in keep.items() if "DIS" in v and "TOTAL" in v]
    manifest["series"]["ilo/disability"] = {
        "title": "Population aged 15 and over with a disability and in total, thousands, by sex and age band "
                 "(DF_POP_XWAP_SEX_AGE_DSB_NB), ILOSTAT",
        "rows": table(RAW / "ilo" / "disability.csv",
                      ["iso3", "year", "sex", "age", "disabled", "total", "source"], rows),
    }
    manifest["sources"]["ilo_disability"] = {"title": "ILOSTAT SDMX API", "url": ILO.format(flow="<dataflow>")}


UN_HH = "https://population.un.org/household/assets/UNDESA_PD_2026_hh-size-composition.xlsx"
UN_OLDER = "https://population.un.org/LivingArrangements/assets/UNDESA_PD_2026_living-arrangements-older-persons.xlsx"
HH_COLUMNS = {
    "Average household size (number of members)": "mean_size",
    "1 member": "size_1",
    "2-3 members": "size_2_3",
    "4-5 members": "size_4_5",
    "6 or more members": "size_6_plus",
    "One-person": "one_person",
    "Couple only": "couple_only",
    "Couple with children": "couple_children",
    "Single parent with children": "single_parent",
    "Extended family": "extended",
    "Non-relatives": "non_relatives",
    "Unknown": "unknown",
    "Multi-generation": "multi_generation",
    "Three generation": "three_generation",
}
OLDER_COLUMNS = {
    "One person": "one_person",
    "Couple only": "couple_only",
    "With spouse or partner": "with_partner",
    "With children under age 20 years": "with_children_under_20",
    "With children aged 20 years or over": "with_children_20_plus",
}


def un_sheet(path: Path, sheet: str, columns: dict, keys: list) -> list:
    """The rows of a UN DESA household workbook, its header on the fifth row; each economy's rows keep their
    source, catalogue number and reference year, and '..' stays missing."""
    import openpyxl
    rows = list(openpyxl.load_workbook(path, read_only=True)[sheet].iter_rows(values_only=True))
    header = [str(h).strip() if h is not None else "" for h in rows[4]]
    at = {name: header.index(name) for name in ["ISO3 Code", "Data source category", "Data source catalog ID",
                                                 "Reference year", *keys, *columns]}
    out = []
    for r in rows[5:]:
        if r[at["ISO3 Code"]] is None:
            continue
        values = []
        for name in columns:
            v = r[at[name]]
            values.append("" if v in (None, "..", "") else str(v))
        if not any(values):
            continue
        out.append((str(r[at["ISO3 Code"]]), int(r[at["Reference year"]]), str(r[at["Data source category"]]),
                    str(r[at["Data source catalog ID"]]), *(str(r[at[k]]) for k in keys), *values))
    return out


def un_households(cache: Path, manifest: dict, iso3: set) -> None:
    """Households by size and basic type, and older persons by living arrangement, from censuses and surveys."""
    hh = [r for r in un_sheet(cached(cache, "un_hh.xlsx", UN_HH), "HH size and composition 2026", HH_COLUMNS, [])
          if r[0] in iso3]
    manifest["series"]["un/households"] = {
        "title": "Households by size and by basic and intergenerational type, % of households, and mean size, each "
                 "source's reference year, UN DESA Database on Household Size and Composition 2026",
        "rows": table(RAW / "un" / "households.csv",
                      ["iso3", "year", "source", "catalog", *HH_COLUMNS.values()], hh),
    }
    older = [r for r in un_sheet(cached(cache, "un_older.xlsx", UN_OLDER), "HHLA of Older Persons 2026",
                                 OLDER_COLUMNS, ["Age range", "Sex"]) if r[0] in iso3]
    manifest["series"]["un/older_persons"] = {
        "title": "Older persons (60+, 65+, 80+) by sex living alone, as a couple only, with a partner, with children "
                 "under 20 and with children 20 or over, % of older persons, UN DESA Database on the Households and "
                 "Living Arrangements of Older Persons 2026",
        "rows": table(RAW / "un" / "older_persons.csv",
                      ["iso3", "year", "source", "catalog", "ages", "sex", *OLDER_COLUMNS.values()], older),
    }
    manifest["sources"]["un_households"] = {
        "title": "UN DESA Population Division, Database on Household Size and Composition 2026 and Database on the "
                 "Households and Living Arrangements of Older Persons 2026",
        "url": f"{UN_HH}; {UN_OLDER}",
    }


WID_URL = "https://wid.world/bulk_download/wid_all_data.zip"
WID_SHARES = {"sptincj992": "income", "shwealj992": "wealth"}
WID_GROUPS = ["p0p50", "p50p90", "p90p100", "p99p100", "p99.9p100"]


def wid_shares(cache: Path, manifest: dict, iso3: set) -> None:
    """Shares of pre-tax national income and of net personal wealth held by the bottom half, the middle 40%, the top
    tenth, hundredth and thousandth, equal-split adults, 2010 on."""
    import zipfile
    z = zipfile.ZipFile(cached(cache, "wid_all_data.zip", WID_URL))
    two = {r["iso2"]: r["iso3"] for r in csv.DictReader((RAW / "wb" / "countries.csv").open()) if r["iso2"]}
    rows = []
    for info in z.infolist():
        name = info.filename
        if not (name.startswith("WID_data_") and name.endswith(".csv")):
            continue
        iso = two.get(name[len("WID_data_"):-len(".csv")])
        if iso not in iso3:
            continue
        for r in csv.DictReader(io.TextIOWrapper(z.open(info), encoding="utf-8"), delimiter=";"):
            if r["variable"] in WID_SHARES and r["percentile"] in WID_GROUPS and int(r["year"]) >= 2010 \
                    and r["value"]:
                rows.append((iso, int(r["year"]), WID_SHARES[r["variable"]], r["percentile"], r["value"]))
    manifest["series"]["wid/shares"] = {
        "title": "Shares of pre-tax national income (sptincj992) and net personal wealth (shwealj992), equal-split "
                 "adults, held by the bottom 50%, the middle 40%, the top 10%, 1% and 0.1%, 2010 on, WID",
        "rows": table(RAW / "wid" / "shares.csv", ["iso3", "year", "what", "group", "share"], rows),
    }
    release = max(i.date_time for i in z.infolist())
    manifest["sources"]["wid_shares"] = {
        "title": "World Inequality Database, bulk download", "url": WID_URL,
        "release": datetime.date(*release[:3]).isoformat(),
    }


SOURCES = {"wpp": wpp, "ilo_disability": ilo_disability, "un_households": un_households, "wid_shares": wid_shares}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cache", type=Path, default=None)
    parser.add_argument("--only", nargs="*", choices=sorted(SOURCES))
    args = parser.parse_args()
    cache = args.cache or Path(tempfile.mkdtemp())
    cache.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((RAW / "manifest.json").read_text())
    iso3 = countries()
    for name in args.only or sorted(SOURCES):
        SOURCES[name](cache, manifest, iso3)
        manifest["sources"][name]["fetched"] = datetime.date.today().isoformat()
        log(f"{name} done")
    (RAW / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
