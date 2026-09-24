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


SOURCES = {"wpp": wpp, "ilo_disability": ilo_disability}


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
