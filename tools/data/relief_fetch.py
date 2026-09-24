#!/usr/bin/env python3
"""Fetches NOAA's ETOPO1 relief over the map's analogue region, 35-58°N and 10°W-60°E, every 2 arc-minutes, from
the NOAA CoastWatch ERDDAP server, one degree of latitude per request, into data/sources/raw/etopo/ (gzipped CSV of
latitude, longitude, metres). tools/data/relief.py derives GEO's relief tables from it.

    python3 tools/data/relief_fetch.py
"""
import gzip
import sys
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "data" / "sources" / "raw" / "etopo"
URL = "https://coastwatch.pfeg.noaa.gov/erddap/griddap/etopo180.csv?altitude%5B({a}):2:({b})%5D%5B(-10):2:(60)%5D"
SOUTH, NORTH = 35, 58


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for lat in range(SOUTH, NORTH):
        path = OUT / f"{lat:02d}.csv.gz"
        if path.exists():
            continue
        # the band's southern edge up to the last row before the next band's
        url = URL.format(a=f"{lat}.0", b=f"{lat}.97")
        for attempt in range(4):
            try:
                with urllib.request.urlopen(url, timeout=300) as r:
                    body = r.read()
                break
            except Exception as e:
                print(f"{lat}: {e}; retrying", flush=True)
                time.sleep(2 ** (attempt + 1))
        else:
            sys.exit(f"{lat}: no answer from ERDDAP")
        lines = body.decode().splitlines()[2:]
        path.write_bytes(gzip.compress(("\n".join(lines) + "\n").encode(), mtime=0))
        print(f"{lat}°N: {len(lines)} heights", flush=True)


if __name__ == "__main__":
    main()
