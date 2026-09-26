#!/usr/bin/env python3
"""Fetches the published data the opening's firms by industry and size are derived from, into data/sources/raw/.

- ILOSTAT's employment by status in employment (ICSE-93) and economic activity (ISIC Rev. 4 sections), from labour
  force surveys: employers and own-account workers are the firms' owners, each running a firm of their own.
- The OECD's structural business statistics: enterprises and persons employed by activity and size class.

    python3 tools/data/fetch_frm.py [--cache DIR] [--only NAME ...]
"""
import argparse
import csv
import datetime
import json
import tempfile
from pathlib import Path

from fetch import RAW, get, log
from fetch_pop import countries, table

FIRST = 2015
# One status at a time, both sexes, annual: the whole flow outruns the API's gateway.
ILO = ("https://sdmx.ilo.org/rest/data/ILO,DF_EMP_TEMP_SEX_STE_ECO_NB,1.0/.A..SEX_T.{status}.?startPeriod="
       + str(FIRST))
# Employers, own-account workers, and all employment.
STATUSES = ("STE_ICSE93_2", "STE_ICSE93_3", "STE_ICSE93_TOTAL")
ILO_CSV = "application/vnd.sdmx.data+csv;version=1.0.0"
SDBS = ("https://sdmx.oecd.org/public/rest/data/OECD.SDD.TPS,DSD_SDBSBSC_ISIC4@DF_SDBS_ISIC4,/"
        "A..ENTR+EMPN...?startPeriod=" + str(FIRST) + "&format=csvfile")
# The activities the firms' industries are read at: ISIC sections and the divisions the products aggregate.
SDBS_ACTIVITIES = {"B", "C", "D", "E", "F", "G", "H", "I", "J", "L", "M", "N", "P", "Q", "R", "S95", "S96",
                   "C10", "C11", "C12", "C13", "C14", "C15", "C16", "C17", "C18", "C19", "C20", "C21", "C22", "C23",
                   "C24", "C25", "C26", "C27", "C28", "C29", "C30", "C31", "C32", "C33", "H49", "H50", "H51", "H52",
                   "H53", "J58", "J59", "J60", "J61", "J62", "J63", "M69", "M70", "M71", "M72", "M73", "M74", "M75",
                   "N77", "N78", "N79", "N80", "N81", "N82", "R90", "R91", "R92", "R93", "E36", "E37", "E38", "E39",
                   "G45", "G46", "G47", "Q86", "Q87", "Q88", "P85", "D35", "B05", "B06", "B07", "B08", "B09"}
SIZES = {"_T", "S1T9", "S10T49", "S50T249", "S_GE250"}


def ilo_status(cache: Path, manifest: dict, iso3: set) -> None:
    """Employment by status and ISIC section, both sexes; the ILO's modelled estimates are left out."""
    rows = []
    for status in STATUSES:
        path = cache / f"DF_EMP_TEMP_SEX_STE_ECO_NB_{status}.csv"
        if not path.exists():
            log(f"downloading DF_EMP_TEMP_SEX_STE_ECO_NB, {status}")
            path.write_bytes(get(ILO.format(status=status), timeout=3600, accept=ILO_CSV))
        with path.open(encoding="utf-8-sig") as f:
            for r in csv.DictReader(f):
                if r["FREQ"] != "A" or r["REF_AREA"] not in iso3 or r["SEX"] != "SEX_T" or not r["OBS_VALUE"] \
                        or not r["ECO"].startswith("ECO_ISIC4_") or not r["STE"].startswith("STE_ICSE93_") \
                        or "Modelled" in r.get("SOURCE", ""):
                    continue
                rows.append((r["REF_AREA"], int(r["TIME_PERIOD"]), r["ECO"][len("ECO_ISIC4_"):],
                             r["STE"][len("STE_ICSE93_"):], r["OBS_VALUE"], r.get("SOURCE", "")))
    manifest["series"]["ilo/status_by_activity"] = {
        "title": "Employment by status in employment (ICSE-93) and economic activity (ISIC Rev. 4 sections), "
                 "thousands, both sexes (DF_EMP_TEMP_SEX_STE_ECO_NB), ILOSTAT",
        "rows": table(RAW / "ilo" / "status_by_activity.csv",
                      ["iso3", "year", "activity", "status", "value", "source"], rows),
    }
    manifest["sources"]["ilo_status"] = {"title": "ILOSTAT SDMX API", "url": ILO}


def sdbs(cache: Path, manifest: dict, iso3: set) -> None:
    """Enterprises and persons employed by activity and size class."""
    path = cache / "sdbs_isic4.csv"
    if not path.exists():
        log("downloading SDBS by activity and size")
        path.write_bytes(get(SDBS, timeout=3600))
    rows = []
    with path.open(encoding="utf-8-sig") as f:
        for r in csv.DictReader(f):
            if r["REF_AREA"] not in iso3 or r["ACTIVITY"] not in SDBS_ACTIVITIES or r["SIZE_CLASS"] not in SIZES \
                    or not r["OBS_VALUE"]:
                continue
            rows.append((r["REF_AREA"], int(r["TIME_PERIOD"]), r["ACTIVITY"], r["SIZE_CLASS"], r["MEASURE"],
                         r["OBS_VALUE"]))
    manifest["series"]["sdbs/by_activity_size"] = {
        "title": "Enterprises (ENTR) and persons employed (EMPN) by ISIC Rev. 4 activity and size class, OECD "
                 "Structural and Demographic Business Statistics",
        "rows": table(RAW / "sdbs" / "by_activity_size.csv",
                      ["iso3", "year", "activity", "size", "measure", "value"], rows),
    }
    manifest["sources"]["sdbs_activity"] = {"title": "OECD Data Explorer, SDMX API", "url": SDBS}


SOURCES = {"ilo_status": ilo_status, "sdbs": sdbs}


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
        source = {"ilo_status": "ilo_status", "sdbs": "sdbs_activity"}[name]
        manifest["sources"][source]["fetched"] = datetime.date.today().isoformat()
        log(f"{name} done")
    (RAW / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
