#!/usr/bin/env python3
"""Fetches the published data households' balance sheets, housing, energy use, insolvency and hazards are read
from, into data/sources/raw/households/ (and the WDI road-death series into data/sources/raw/wdi/):

- scf: the Federal Reserve's Survey of Consumer Finances 2019, the summary extract's weights, demographics, income
  and whole balance sheet per implicate row, joined with the full public file's first mortgage on the home (year
  obtained or refinanced, amount borrowed and owed, term, expected payoff year, rate, adjustable).
- hfcs: the ECB's Household Finance and Consumption Survey statistical tables, wave 2017, every table's point
  values by breakdown (income and wealth quantile, age, tenure, household size) and country.
- prices: the OECD's analytical house price indicators (nominal and real prices, rents, price-to-rent and
  price-to-income ratios) and regional house price indices, annual; the BIS's residential property prices,
  quarterly, nominal and real.
- housing: the OECD Affordable Housing Database's dwelling stock, completions, vacancy and rooms per member by
  tenure; Eurostat's share of people who moved dwelling within five years by tenure; the US Census Bureau's
  annual mobility rates by type of move and months from start to completion of new residential buildings; the
  World Bank Doing Business property registration and construction permit procedures, time and cost.
- hazards: the JRC's global flood depth-damage functions and maximum damage by country; HAZUS default flood
  depth-damage functions; typed tables for tropical-cyclone wind damage (Emanuel's form, Eberenz et al. 2021) and
  crop yields' response to temperature (Schlenker and Roberts 2008).
- accidents: WDI road-traffic mortality; WHO age-standardised fire death rates; typed CTIF fire rates for 2019.
- energy: the EIA's Residential Energy Consumption Survey 2020 microdata, reduced to household composition,
  dwelling, climate, energy use by fuel and end use, and weights.
- insolvency: US bankruptcy cases commenced by chapter and business or nonbusiness debt; England and Wales
  individual insolvencies by procedure.
- mpc: typed marginal propensities to consume by cash on hand, income, financial assets and deposits (Jappelli
  and Pistaferri; Fagereng, Holm and Natvik).
- coal: the EIA's mine-level production (2019), the quality of coal each mine shipped to power plants (EIA-923,
  2019) and recoverable reserves by state and mining method.
- farms: Eurostat's farm holdings, persons and annual work units by economic size class, and the US Census of
  Agriculture 2017's farms and hired workers by number of hired workers.

    python3 tools/data/fetch_households.py [--cache DIR] [--only NAME ...]

Two sources are old Excel files (.xls), read with the xlrd package.
"""
import argparse
import csv
import datetime
import gzip
import io
import json
import re
import struct
import tempfile
import urllib.parse
import zipfile
from pathlib import Path

import openpyxl

from fetch import RAW, cached, get, log, merge_manifest
from fetch_pop import EUROSTAT_GEO, countries, iso3_by_name, table

OUT = RAW / "households"
TODAY = datetime.date.today().isoformat()
FIRST, LAST = 2015, 2025

SCF = "https://www.federalreserve.gov/econres/files/"
SCF_YEAR = 2019
# The summary extract's variables kept: the weight, the household head's demographics, income, every asset and
# debt item of the balance sheet (participation is a holding above zero), and distress flags.
SCF_SUMMARY = ["Y1", "WGT", "AGE", "HHSEX", "EDCL", "MARRIED", "KIDS", "LF", "OCCAT1", "INCOME", "WAGEINC",
               "NORMINC", "NETWORTH", "LIQ", "CDS", "NMMF", "STOCKS", "BOND", "RETQLIQ", "SAVBND", "CASHLI", "OTHMA",
               "OTHFIN", "VEHIC", "HOUSES", "ORESRE", "NNRESRE", "BUS", "OTHNFIN", "MRTHEL", "RESDBT", "OTHLOC",
               "CCBAL", "INSTALL", "ODEBT", "MORTPAY", "BNKRUPLAST5", "FORECLLAST5", "LATE60", "TURNDOWN"]
# The full file's first mortgage or loan on the home: year obtained or last refinanced, amount borrowed, amount
# owed, term in years, expected payoff year (asked only when not on schedule), rate in hundredths of a percent,
# and whether the rate is adjustable (1 yes, 5 no).
SCF_FULL = {"X802": "mort1_year", "X804": "mort1_borrowed", "X805": "mort1_owed", "X806": "mort1_term_years",
            "X815": "mort1_payoff_year", "X816": "mort1_rate_x100", "X820": "mort1_adjustable"}

HFCS = "https://www.ecb.europa.eu/home/pdf/research/hfcn/HFCS_Statistical_Tables_Wave_2017_June_2026.zip"
OECD_HP = ("https://sdmx.oecd.org/public/rest/data/OECD.ECO.MPD,DSD_AN_HOUSE_PRICES@DF_HOUSE_PRICES,1.0/all"
           "?startPeriod={first}&format=csvfile")
OECD_RHPI = ("https://sdmx.oecd.org/public/rest/data/OECD.SDD.TPS,DSD_RHPI_TARGET@DF_RHPI_TARGET,/all"
             "?startPeriod={first}&format=csvfile")
BIS_SPP = "https://data.bis.org/static/bulk/WS_SPP_csv_flat.zip"
# OECD and Eurostat regional codes begin with a two-letter country code that is not ISO's for these two.
TWO_LETTER = {"UK": "GBR", "EL": "GRC"}

AHD = "https://webfs.oecd.org/Els-com/Affordable_Housing_Database/"
AHD_ROOMS = {"Own outright": "own_outright", "Owner with mortgage": "own_mortgage", "Rent (private)": "rent_private",
             "Rent (subsidized)": "rent_subsidised", "Rent (subsidised)": "rent_subsidised"}
EUROSTAT = "https://ec.europa.eu/eurostat/api/dissemination/"
CPS_MOBILITY = ("https://www2.census.gov/programs-surveys/demo/tables/geographic-mobility/time-series/historic/"
                "hst_mig_a_1.xlsx")
CPS_COLUMNS = ["total", "same_residence", "movers", "movers_in_us", "same_county", "different_county",
               "different_county_same_state", "different_state", "from_abroad"]
CENSUS_MONTHS = "https://www.census.gov/construction/nrc/xls/avg_starttocomp_cust.xls"
CENSUS_MONTHS_COLUMNS = ["one_unit_total", "one_unit_built_for_sale", "one_unit_contractor_built",
                         "one_unit_owner_built", "multi_unit_total", "units_2_4", "units_5_9", "units_10_19",
                         "units_20_plus"]
DOING_BUSINESS = ("https://archive.doingbusiness.org/content/dam/doingBusiness/excel/db2020/"
                  "Historical-data---COMPLETE-dataset-with-scores.xlsx")
# Each topic's columns kept, by the header's text within the topic.
DB_COLUMNS = {
    "Registering property": {"Procedures (number)": "property_procedures", "Time (days)": "property_days",
                             "Cost (% of property value)": "property_cost_pct_value",
                             "Quality of land administration index (0-30) (DB17-20 methodology)":
                                 "land_administration_index"},
    "Dealing with construction permits": {"Procedures (number)": "permit_procedures", "Time (days)": "permit_days",
                                          "Cost (% of Warehouse value)": "permit_cost_pct_value"},
}

JRC = ("https://publications.jrc.ec.europa.eu/repository/bitstream/JRC105688/"
       "copy_of_global_flood_depth-damage_functions__30102017.xlsx")
JRC_REGIONS = ["europe", "north_america", "central_south_america", "asia", "africa", "oceania", "global"]
HAZUS = "https://zenodo.org/api/records/10027236/files/{name}/content"
# HAZUS's own default curves and the flood insurance administration's; the rest are single districts' studies.
HAZUS_SOURCES = {"Hazus Dflt", "FIA", "FIA (MOD.)"}

WDI_ROAD = ("https://api.worldbank.org/v2/country/all/indicator/SH.STA.TRAF.P5?format=json&date={first}:{last}"
            "&per_page=20000")
WHO_FIRE = "https://ghoapi.azureedge.net/api/SA_0000001443"

RECS = "https://www.eia.gov/consumption/residential/data/2020/csv/recs2020_public_v7.csv"
# Household composition, dwelling, climate, energy use by fuel (thousand Btu) and end use, spending, and weight.
RECS_COLUMNS = ["REGIONC", "BA_climate", "UATYP10", "HDD65", "CDD65", "TYPEHUQ", "KOWNRENT",
                "YEARMADERANGE", "TOTROOMS", "TOTSQFT_EN", "NHSLDMEM", "NUMCHILD", "HHAGE", "MONEYPY", "FUELHEAT",
                "AIRCOND", "BTUEL", "BTUNG", "BTULP", "BTUFO", "BTUWD", "TOTALBTUSPH", "TOTALBTUWTH", "BTUELCOL",
                "TOTALBTU", "TOTALDOL", "NWEIGHT"]

RECS_TEXT = {"REGIONC", "BA_climate", "UATYP10"}

USCOURTS = "https://www.uscourts.gov"
USCOURTS_PAGE = USCOURTS + "/data-news/data-tables/{year}/12/31/bankruptcy-filings/f-2"
UK_INSOLVENCY = "https://www.gov.uk/government/collections/individual-insolvency-statistics-releases"
UK_COLUMNS = ["EW_total_individuals_NSA", "EW_bankruptcy_NSA", "EW_DRO_NSA", "EW_IVA_NSA"]
UK_RATE = "EW_individual_rate_per_10000"

EIA_MINES = "https://www.eia.gov/coal/data/public/xls/coalpublic{year}.xls"
EIA_923 = "https://www.eia.gov/electricity/data/eia923/archive/xls/f923_{year}.zip"
EIA_RESERVES = "https://www.eia.gov/coal/annual/xls/table15.xlsx"
COAL_YEAR = 2019
NASS = "https://www.nass.usda.gov/datasets/qs.census2017.txt.gz"

# Typed paper tables, each value read from the document named in its row, downloaded when this fetcher was written.
JP = ("Jappelli and Pistaferri, Fiscal Policy and MPC Heterogeneity, CSEF Working Paper 325 (December 2012 version; "
      "published AEJ: Macroeconomics 6(4), 2014), https://www.csef.it/WP/wp325.pdf; Italy, SHIW 2010, self-reported "
      "share of an unexpected reimbursement equal to a month's income that would be consumed")
FHN = ("Fagereng, Holm and Natvik, MPC heterogeneity and household balance sheets, Statistics Norway Discussion "
       "Papers 852 (November 2016; published AEJ: Macroeconomics 13(4), 2021), https://www.ssb.no/en/forskning/"
       "discussion-papers/mpc-heterogeneity-and-household-balance-sheets; Norway, lottery prizes 1994-2006, share of "
       "a prize spent (or saved) within the year of winning")
# (source, table, group, measure, value, standard error)
MPC = [
    (JP, "Table 1", "all households", "mean MPC", "0.476", ""),
    (JP, "Table 2 column 2", "cash-on-hand quintile 1 (lowest)", "MPC difference from quintile 5", "0.293", "0.024"),
    (JP, "Table 2 column 2", "cash-on-hand quintile 2", "MPC difference from quintile 5", "0.186", "0.021"),
    (JP, "Table 2 column 2", "cash-on-hand quintile 3", "MPC difference from quintile 5", "0.133", "0.020"),
    (JP, "Table 2 column 2", "cash-on-hand quintile 4", "MPC difference from quintile 5", "0.063", "0.019"),
    (JP, "Table 2 column 3", "unemployed head", "MPC difference from employed", "0.070", "0.034"),
    (JP, "Table 4 column 1", "income quintile 1 (lowest)", "MPC difference from quintile 5", "0.115", "0.028"),
    (JP, "Table 4 column 1", "income quintile 2", "MPC difference from quintile 5", "0.058", "0.024"),
    (JP, "Table 4 column 1", "income quintile 3", "MPC difference from quintile 5", "0.057", "0.021"),
    (JP, "Table 4 column 1", "income quintile 4", "MPC difference from quintile 5", "0.032", "0.020"),
    (JP, "Table 4 column 1", "financial asset quintile 1 (lowest)", "MPC difference from quintile 5", "0.258",
     "0.024"),
    (JP, "Table 4 column 1", "financial asset quintile 2", "MPC difference from quintile 5", "0.146", "0.022"),
    (JP, "Table 4 column 1", "financial asset quintile 3", "MPC difference from quintile 5", "0.098", "0.020"),
    (JP, "Table 4 column 1", "financial asset quintile 4", "MPC difference from quintile 5", "0.045", "0.020"),
    (JP, "Table 4 column 1", "positive debt", "MPC difference from no debt", "-0.090", "0.015"),
    (JP, "Table 7", "transfer to bottom cash-on-hand decile", "aggregate MPC", "0.62", ""),
    (JP, "Table 7", "transfer to top cash-on-hand decile", "aggregate MPC", "0.36", ""),
    (JP, "Table 7", "transfer to households with an unemployed member", "aggregate MPC", "0.58", ""),
    (FHN, "Table 2 levels OLS IV", "all winners", "MPC", "0.336", "0.009"),
    (FHN, "Table 3", "all winners", "share to deposits", "0.507", "0.013"),
    (FHN, "Table 3", "all winners", "share to stocks, bonds and mutual funds", "0.071", "0.009"),
    (FHN, "Table 3", "all winners", "change in debt per unit won", "-0.118", "0.009"),
    (FHN, "Table 7", "deposit quartile 1 (low)", "MPC", "0.436", "0.021"),
    (FHN, "Table 7", "deposit quartile 2", "MPC", "0.416", "0.020"),
    (FHN, "Table 7", "deposit quartile 3", "MPC", "0.336", "0.022"),
    (FHN, "Table 7", "deposit quartile 4 (high)", "MPC", "0.224", "0.021"),
    (FHN, "Table 7", "deposit quartile 1 (low)", "share to deposits", "0.398", "0.024"),
    (FHN, "Table 7", "deposit quartile 2", "share to deposits", "0.437", "0.030"),
    (FHN, "Table 7", "deposit quartile 3", "share to deposits", "0.530", "0.025"),
    (FHN, "Table 7", "deposit quartile 4 (high)", "share to deposits", "0.727", "0.038"),
    (FHN, "Table 7", "deposit quartile 1 (low)", "share to stocks, bonds and mutual funds", "0.031", "0.009"),
    (FHN, "Table 7", "deposit quartile 2", "share to stocks, bonds and mutual funds", "0.056", "0.014"),
    (FHN, "Table 7", "deposit quartile 3", "share to stocks, bonds and mutual funds", "0.086", "0.021"),
    (FHN, "Table 7", "deposit quartile 4 (high)", "share to stocks, bonds and mutual funds", "0.081", "0.028"),
    (FHN, "Table 7", "deposit quartile 1 (low)", "change in debt per unit won", "-0.149", "0.019"),
    (FHN, "Table 7", "deposit quartile 2", "change in debt per unit won", "-0.123", "0.022"),
    (FHN, "Table 7", "deposit quartile 3", "change in debt per unit won", "-0.087", "0.016"),
    (FHN, "Table 7", "deposit quartile 4 (high)", "change in debt per unit won", "-0.027", "0.018"),
    (FHN, "Table 8 OLS", "prize USD 1,100-2,150", "MPC", "1.007", "0.111"),
    (FHN, "Table 8 OLS", "prize USD 2,150-5,332", "MPC", "0.687", "0.051"),
    (FHN, "Table 8 OLS", "prize USD 5,332-8,926", "MPC", "0.588", "0.027"),
    (FHN, "Table 8 OLS", "prize above USD 8,926", "MPC", "0.316", "0.009"),
    (FHN, "Table 8 LAD", "prize USD 1,100-2,150", "five-year cumulative consumption response", "0.862", "0.1421"),
    (FHN, "Table 8 LAD", "prize USD 2,150-5,332", "five-year cumulative consumption response", "0.640", "0.0639"),
    (FHN, "Table 8 LAD", "prize USD 5,332-8,926", "five-year cumulative consumption response", "0.541", "0.0329"),
    (FHN, "Table 8 LAD", "prize above USD 8,926", "five-year cumulative consumption response", "0.404", "0.0121"),
]

EBERENZ = ("Eberenz, Lüthi and Bresch, Regional tropical cyclone impact functions for globally consistent risk "
           "assessments, Natural Hazards and Earth System Sciences 21, 393-415, 2021, Table A1 (regions) and Table A2 "
           "(Vhalf), section 2.2.3 (Vthresh 25.7 m/s, default Vhalf 74.7 m/s, Emanuel 2011's form); "
           "https://nhess.copernicus.org/articles/21/393/2021/nhess-21-393-2021.pdf")
# (region, matched events, Vhalf calibrated on the spread of event damage ratios, Vhalf calibrated on total
# damage, member economies)
WIND = [
    ("NA1", 73, "59.6", "66.3", "AIA ATG ARG ABW BHS BRB BLZ BMU BOL CPV CYM CHL COL CRI CUB DMA DOM ECU SLV FLK GUF "
                                "GRD GLP GTM GUY HTI HND JAM MTQ MEX MSR NIC PAN PRY PER PRI SHN KNA LCA VCT SXM SUR "
                                "TTO TCA URY VEN VGB VIR"),
    ("NA2", 43, "86", "89.2", "CAN USA"),
    ("NI", 31, "58.7", "70.8", "AFG ARM AZE BHR BGD BTN DJI ERI ETH GEO IND IRN IRQ ISR JOR KAZ KWT KGZ LBN MDV MNG "
                               "MMR NPL OMN PAK QAT SAU SOM LKA SYR TJK TKM UGA ARE UZB YEM"),
    ("OC", 48, "49.7", "64.1", "ASM AUS COK FJI PYF GUM KIR MHL FSM NRU NCL NZL NIU NFK MNP PLW PNG PCN WSM SLB TLS "
                               "TKL TON TUV VUT WLF"),
    ("SI", 19, "46.8", "52.4", "COM COD SWZ MDG MWI MLI MUS MOZ ZAF TZA ZWE"),
    ("WP1", 43, "56.7", "66.4", "KHM IDN LAO MYS THA VNM"),
    ("WP2", 83, "84.7", "188.4", "PHL"),
    ("WP3", 69, "80.2", "112.8", "CHN"),
    ("WP4", 64, "135.6", "190.5", "HKG JPN KOR MAC TWN"),
    ("global", 473, "73.4", "110.1", ""),
]
WIND_THRESHOLD = "25.7"

SCHLENKER = ("Schlenker and Roberts, Estimating the Impact of Climate Change on Crop Yields: The Importance of "
             "Nonlinear Temperature Effects, NBER Working Paper 13799 (February 2008; published PNAS 106(37), 2009), "
             "https://www.nber.org/papers/w13799, abstract and section on results (text p. 14)")
# (crop, measure, value, unit)
CROPS = [
    ("corn", "temperature above which yields fall", "29", "degrees C"),
    ("soybeans", "temperature above which yields fall", "30", "degrees C"),
    ("cotton", "temperature above which yields fall", "32", "degrees C"),
    ("corn", "yield-maximising growing-season precipitation", "25.0", "inches"),
    ("soybeans", "yield-maximising growing-season precipitation", "27.2", "inches"),
    ("corn", "yield change from replacing a full day at 29 C by a full day at 40 C", "-0.07", "share of yield"),
]

CTIF = ("CTIF Center of Fire Statistics, World Fire Statistics Report No. 26 (2021), Table 1.2 (common indicators "
        "of fire statistics in the countries of the world in 2019) and Table 1.4 (distribution of fires by type), "
        "https://www.ctif.org/sites/default/files/2021-06/CTIF_Report26_0.pdf")
# (iso3, calls per 1000 inhabitants, fires per 1000, fire deaths per 100 000, deaths per 100 fires, injuries per
# 100 000, injuries per 100 fires, residential building fires' share of fires in %); blank where the report has none.
FIRES = [
    ("USA", "113.6", "3.9", "1.1", "0.3", "5.1", "1.3", "28.0"),
    ("RUS", "7.9", "3.2", "5.8", "1.8", "6.4", "2.0", "25.0"),
    ("JPN", "69.6", "0.3", "1.2", "3.9", "4.6", "15.6", ""),
    ("EGY", "", "0.5", "0.3", "0.5", "1.2", "2.4", ""),
    ("VNM", "", "0.0", "0.1", "2.2", "0.1", "3.3", ""),
    ("FRA", "72.3", "4.7", "0.4", "0.1", "1.9", "0.4", "1.0"),
    ("GBR", "10.9", "3.4", "0.5", "0.1", "13.5", "3.9", ""),
    ("KOR", "", "0.8", "0.6", "0.7", "4.3", "5.5", ""),
    ("MMR", "", "0.0", "0.2", "3.7", "0.4", "10.5", ""),
    ("UKR", "6.4", "2.3", "4.5", "2.0", "3.6", "1.6", "31.2"),
    ("POL", "13.3", "4.0", "1.3", "0.3", "9.8", "2.5", "10.5"),
    ("KAZ", "3.4", "0.7", "1.7", "2.3", "5.3", "7.1", "62.2"),
    ("NLD", "8.3", "2.3", "0.1", "0.1", "", "", "13.5"),
    ("GRC", "6.7", "2.6", "0.2", "0.1", "0.3", "0.1", "14.8"),
    ("JOR", "72.2", "3.4", "0.5", "0.1", "102.3", "29.7", "8.4"),
    ("CZE", "215.8", "1.8", "1.2", "0.7", "13.0", "7.4", "16.7"),
    ("SWE", "7.7", "2.0", "1.1", "0.3", "8.5", "3.3", "26.9"),
    ("HUN", "8.2", "2.1", "1.2", "0.5", "7.8", "3.6", "21.3"),
    ("BLR", "8.7", "0.6", "5.2", "8.0", "4.7", "7.3", ""),
    ("AUT", "31.5", "4.9", "", "", "", "", ""),
    ("CHE", "8.3", "1.5", "", "", "", "", ""),
    ("BGR", "", "6.0", "1.9", "0.3", "4.2", "0.7", ""),
    ("SGP", "34.6", "0.5", "0.0", "0.0", "2.5", "5.0", ""),
    ("SVK", "22.6", "1.8", "0.8", "0.5", "6.3", "3.6", ""),
    ("IRL", "24.4", "4.2", "0.3", "0.1", "", "", ""),
    ("NZL", "17.6", "4.9", "0.7", "0.1", "", "", ""),
    ("HRV", "7.7", "3.7", "0.7", "0.2", "4.1", "1.1", ""),
    ("MNG", "411.8", "1.3", "1.6", "1.3", "2.1", "1.6", ""),
    ("LTU", "11.0", "4.1", "2.5", "0.6", "5.9", "1.4", ""),
    ("SVN", "73.4", "2.1", "0.6", "0.3", "10.0", "4.7", ""),
    ("LVA", "", "5.3", "4.0", "0.8", "14.5", "2.8", ""),
    ("EST", "19.6", "3.5", "3.2", "0.9", "8.5", "2.4", ""),
    ("BRN", "", "4.8", "0.2", "0.0", "1.9", "0.4", ""),
    ("LIE", "", "1.3", "2.6", "2.0", "", "", ""),
]


def two_letter() -> dict:
    """ISO two-letter codes to the world's three-letter codes, with the codes OECD and Eurostat use instead."""
    iso2 = {r["iso2"]: r["iso3"] for r in csv.DictReader((RAW / "wb" / "countries.csv").open()) if r["iso2"]}
    return {**iso2, **TWO_LETTER, **EUROSTAT_GEO}


def fmt(v: float, digits: int = 10) -> str:
    """A number as plain text, to the given significant digits, without an exponent where it is whole."""
    rounded = float(f"{v:.{digits}g}")
    return str(int(rounded)) if rounded.is_integer() else repr(rounded)


def number(v, digits: int = 10) -> str:
    """A cell's number as text, or '' where the cell holds none ('..', '-', 'M', '(D)', blanks)."""
    if isinstance(v, bool):
        return ""
    if isinstance(v, (int, float)):
        return fmt(v, digits)
    text = str(v or "").strip().replace(",", "")
    try:
        return fmt(float(text), digits)
    except ValueError:
        return ""


def dta(path: Path, wanted: list) -> list:
    """The named variables of a Stata 118 file, row by row: numbers, with Stata's missing codes as None."""
    widths = {65530: 1, 65529: 2, 65528: 4, 65527: 4, 65526: 8}
    formats = {65530: "<b", 65529: "<h", 65528: "<i", 65527: "<f", 65526: "<d"}
    missing = {65530: 100, 65529: 32740, 65528: 2147483620, 65527: 1.7014118e38, 65526: 8.98846567431158e307}
    with path.open("rb") as f:
        head = f.read(4096)
        if b"<release>118</release>" not in head or b"<byteorder>LSF</byteorder>" not in head:
            raise SystemExit(f"{path}: not a little-endian Stata 118 file")
        k = struct.unpack("<H", head[head.index(b"<K>") + 3:][:2])[0]
        n = struct.unpack("<Q", head[head.index(b"<N>") + 3:][:8])[0]
        at = head.index(b"<map>") + 5
        offsets = struct.unpack("<14Q", head[at:at + 112])
        f.seek(offsets[2] + len(b"<variable_types>"))
        types = struct.unpack(f"<{k}H", f.read(2 * k))
        f.seek(offsets[3] + len(b"<varnames>"))
        names = [f.read(129).split(b"\0", 1)[0].decode().upper() for _ in range(k)]
        width = [widths.get(t, 8 if t == 32768 else t) for t in types]
        start = [sum(width[:i]) for i in range(k)]
        row = sum(width)
        columns = [names.index(w) for w in wanted]
        if any(types[c] not in formats for c in columns):
            raise SystemExit(f"{path}: a wanted variable is not numeric")
        f.seek(offsets[9] + len(b"<data>"))
        out = []
        for _ in range(n):
            record = f.read(row)
            values = []
            for c in columns:
                v = struct.unpack(formats[types[c]], record[start[c]:start[c] + width[c]])[0]
                values.append(None if v > missing[types[c]] else v)
            out.append(values)
        return out


def scf(cache: Path, manifest: dict, iso3: set) -> None:
    """The summary extract's rows joined with the full file's first mortgage, by the implicate's identifier."""
    summary = zipfile.ZipFile(cached(cache, f"scfp{SCF_YEAR}excel.zip", SCF + f"scfp{SCF_YEAR}excel.zip"))
    full_zip = zipfile.ZipFile(cached(cache, f"scf{SCF_YEAR}s.zip", SCF + f"scf{SCF_YEAR}s.zip"))
    name = f"p{SCF_YEAR % 100}i6.dta"
    full_path = cache / name
    if not full_path.exists():
        full_path.write_bytes(full_zip.read(name))
    full = {int(r[0]): r[1:] for r in dta(full_path, ["Y1", *SCF_FULL])}
    rows = []
    with summary.open(f"SCFP{SCF_YEAR}.csv") as f:
        for r in csv.DictReader(io.TextIOWrapper(f, encoding="utf-8-sig")):
            y1 = int(r["Y1"])
            values = [r["Y1"]] + [number(r[c], 5) for c in SCF_SUMMARY[1:]]
            values += ["" if v is None else fmt(v, 6) for v in full[y1]]
            rows.append(values)
    header = SCF_SUMMARY + list(SCF_FULL.values())
    manifest["series"]["households/scf2019"] = {
        "title": f"Survey of Consumer Finances {SCF_YEAR}, one row per household and implicate (Y1; five implicates "
                 "per household; the weights WGT summed over all five implicates give the population of households, "
                 "each implicate's summing to a fifth of it): the head's "
                 "age, sex, education class, marital status, children, labour force status and occupation; income, "
                 "wages and normal income; net worth and every asset and debt item of the summary extract (US "
                 "dollars of 2019); payments on mortgages; bankruptcy and foreclosure in the last five years, 60 "
                 "days late and turned down for credit; and from the full public file the first mortgage on the "
                 "home: year obtained or last refinanced (X802), amount borrowed (X804) and owed (X805), term in "
                 "years (X806, -1 none set), expected payoff year when not on schedule (X815), rate in hundredths of "
                 "a percent (X816, -1 no interest) and adjustable rate (X820: 1 yes, 5 no), 0 where inapplicable; "
                 "Federal Reserve Board",
        "rows": table(OUT / "scf2019.csv", header, rows),
        "for": ["S2.05", "S2.11", "S3.05", "S6.03", "S6.05"],
        "note": "summary extract variables as published (SCFP2019.csv); full-file variables read from p19i6.dta by "
                "Y1; summary values rounded to five significant figures",
    }
    manifest["sources"]["households/scf"] = {
        "title": f"Federal Reserve Board, Survey of Consumer Finances {SCF_YEAR}, summary extract (CSV) and full "
                 "public data set (Stata)", "url": SCF + f"scfp{SCF_YEAR}excel.zip; " + SCF + f"scf{SCF_YEAR}s.zip",
        "fetched": TODAY}


def hfcs(cache: Path, manifest: dict, iso3: set) -> None:
    """Every statistical table's point values: the table, its unit, its row labels and each country's value."""
    z = zipfile.ZipFile(cached(cache, "hfcs2017.zip", HFCS))
    book = openpyxl.load_workbook(io.BytesIO(z.read(z.namelist()[0])), read_only=True, data_only=True)
    codes = {**two_letter(), "euro area": "EMU"}
    rows, titles = [], {}
    for sheet in book.sheetnames:
        if not re.match(r"^[A-K]\d", sheet):
            continue
        lines = list(book[sheet].iter_rows(values_only=True))
        title = next(str(c[0]) for c in lines if isinstance(c[0], str) and c[0].startswith("Table"))
        name = title.split()[1]
        unit = str(lines[[i for i, c in enumerate(lines) if c[0] == title][0] + 1][0] or "")
        head_at = next(i for i, c in enumerate(lines) if "euro area" in c)
        head = lines[head_at]
        cols = {i: codes[str(c).strip()] for i, c in enumerate(head) if c and str(c).strip() in codes}
        first = min(cols)
        label1 = ""
        for line in lines[head_at + 1:]:
            if isinstance(line[0], str) and line[0].startswith(("Source", "M = ")):
                break
            labels = [str(c).strip() for c in line[:first] if c is not None and str(c).strip()]
            if not labels:
                continue
            if line[0] is not None and str(line[0]).strip():
                label1 = str(line[0]).strip()
                labels = labels[1:]
            label2 = " | ".join(labels)
            for i, iso in cols.items():
                v = line[i] if i < len(line) else None
                if isinstance(v, str) and v.strip().startswith("("):
                    continue
                value = number(v)
                if value and (iso in iso3 or iso == "EMU"):
                    rows.append((name, unit, label1, label2, iso, value))
                    titles[name] = title
    manifest["series"]["households/hfcs2017_tables"] = {
        "title": "Household Finance and Consumption Survey, wave 2017 (reference years 2016-2018 by country), "
                 "statistical tables A-K: medians, means, participation rates, shares and ratios of households' "
                 "real and financial assets, debt, debt burdens, income and net wealth, broken down by income and "
                 "net wealth quantile, age of the reference person, household size, housing status and more, per "
                 "country and the euro area (EMU), in each table's unit (EUR thousands, %, ratios); ECB",
        "rows": table(OUT / "hfcs2017_tables.csv", ["table", "unit", "row", "item", "iso3", "value"], rows),
        "for": ["S3.05", "S6.03", "S6.05"],
        "note": "point values only; standard errors, 'M' (missing) and 'N' (too few observations) left out; each "
                "table's title in hfcs2017_titles.csv",
    }
    manifest["series"]["households/hfcs2017_titles"] = {
        "title": "Titles of the HFCS wave 2017 statistical tables, by table code; ECB",
        "rows": table(OUT / "hfcs2017_titles.csv", ["table", "title"], sorted(titles.items())),
        "for": ["S3.05", "S6.03", "S6.05"],
    }
    manifest["sources"]["households/hfcs"] = {"title": "ECB, HFCS statistical tables, wave 2017 (June 2026 release)",
                                              "url": HFCS, "fetched": TODAY}


def prices(cache: Path, manifest: dict, iso3: set) -> None:
    """House prices and rents: the OECD's national indicators and regional indices, annual, and the BIS's
    residential property prices, quarterly."""
    text = cached(cache, "oecd_house_prices.csv", OECD_HP.format(first=FIRST)).read_text(encoding="utf-8-sig")
    rows = [(r["REF_AREA"], int(r["TIME_PERIOD"]), r["MEASURE"], r["UNIT_MEASURE"], r["OBS_VALUE"])
            for r in csv.DictReader(io.StringIO(text))
            if r["FREQ"] == "A" and r["REF_AREA"] in iso3 and r["OBS_VALUE"] and int(r["TIME_PERIOD"]) <= LAST]
    manifest["series"]["households/oecd_house_prices"] = {
        "title": "Analytical house price indicators, annual: nominal (HPI) and real (RHP) house prices, rent prices "
                 "(RPI), price-to-rent (HPI_RPI) and price-to-income (HPI_YDH) ratios as indices (2015 = 100), and "
                 "the two ratios against their long-term average (_AVG); OECD",
        "rows": table(OUT / "oecd_house_prices.csv", ["iso3", "year", "measure", "unit", "value"], rows),
        "for": ["S2.05"],
    }
    text = cached(cache, "oecd_rhpi.csv", OECD_RHPI.format(first=FIRST)).read_text(encoding="utf-8-sig")
    codes = two_letter()
    rows = []
    for r in csv.DictReader(io.StringIO(text)):
        if r["FREQ"] != "A" or r["UNIT_MEASURE"] != "IX" or not r["OBS_VALUE"] or r["ADJUSTMENT"] != "N":
            continue
        area = r["REF_AREA"]
        iso = area if r["REF_AREA_TYPE"] == "COU" else codes.get(area[:2])
        if iso in iso3 and int(r["TIME_PERIOD"]) <= LAST:
            rows.append((iso, area, r["REF_AREA_TYPE"], r["DWELLINGS"], r["VINTAGE"], int(r["TIME_PERIOD"]),
                         r["MEASURE"], r["OBS_VALUE"]))
    manifest["series"]["households/oecd_regional_house_prices"] = {
        "title": "Regional house price indices, annual, not seasonally adjusted, index (base per series), by region "
                 "(large and small territorial regions, cities, whole country), dwelling type and vintage, real "
                 "(RHPI) and nominal (OHPI); OECD",
        "rows": table(OUT / "oecd_regional_house_prices.csv",
                      ["iso3", "region", "region_type", "dwellings", "vintage", "year", "measure", "value"], rows),
        "for": ["S2.05"],
    }
    z = zipfile.ZipFile(cached(cache, "bis_spp.zip", BIS_SPP))
    code = lambda text: text.split(":", 1)[0].strip()
    rows = []
    with z.open(z.namelist()[0]) as f:
        reader = csv.reader(io.TextIOWrapper(f, encoding="utf-8-sig"))
        header = [code(h) for h in next(reader)]
        at = {h: header.index(h) for h in ("FREQ", "REF_AREA", "VALUE", "UNIT_MEASURE", "TIME_PERIOD", "OBS_VALUE")}
        for r in reader:
            area, value, unit, period, obs = (code(r[at[h]]) for h in
                                              ("REF_AREA", "VALUE", "UNIT_MEASURE", "TIME_PERIOD", "OBS_VALUE"))
            iso = codes.get(area)
            if code(r[at["FREQ"]]) == "Q" and unit == "628" and iso in iso3 and obs \
                    and FIRST <= int(period[:4]) <= LAST:
                rows.append((iso, period, value, obs))
    manifest["series"]["households/bis_property_prices"] = {
        "title": "Residential property prices, quarterly, index 2010 = 100, nominal (N) and real (R); BIS selected "
                 "residential property price series (WS_SPP)",
        "rows": table(OUT / "bis_property_prices.csv", ["iso3", "quarter", "value_type", "index"], rows),
        "for": ["S2.05"],
    }
    manifest["sources"]["households/oecd_house_prices"] = {
        "title": "OECD Data Explorer, analytical house price indicators and regional house price indices",
        "url": OECD_HP.format(first=FIRST) + "; " + OECD_RHPI.format(first=FIRST), "fetched": TODAY}
    manifest["sources"]["households/bis_spp"] = {"title": "BIS, selected residential property prices, bulk CSV",
                                                 "url": BIS_SPP, "fetched": TODAY}


def eurostat(dataset: str, filters: dict) -> list:
    """A Eurostat dataset's observations as dicts of its dimensions' codes and 'value', from the JSON-stat API."""
    query = urllib.parse.urlencode([("lang", "en")] + [(k, v) for k, vs in filters.items() for v in vs])
    data = json.loads(get(f"{EUROSTAT}statistics/1.0/data/{dataset}?{query}", timeout=600))
    dims, sizes = data["id"], data["size"]
    codes = [sorted(data["dimension"][d]["category"]["index"].items(), key=lambda kv: kv[1]) for d in dims]
    out = []
    for flat, value in data["value"].items():
        at, rest = {}, int(flat)
        for d, size, cs in reversed(list(zip(dims, sizes, codes))):
            at[d] = cs[rest % size][0]
            rest //= size
        out.append({**at, "value": value})
    return out


def housing(cache: Path, manifest: dict, iso3: set) -> None:
    """The dwelling stock, completions, vacancy and space, residential mobility, construction length, and the
    procedures of a property transfer and a building permit."""
    unmatched = set()
    names = iso3_by_name()
    book = openpyxl.load_workbook(cached(cache, "ahd_hm11.xlsx", AHD + "HM1-1-Housing-stock-and-construction.xlsx"),
                                  read_only=True, data_only=True)
    rows = []
    points = ("around_2011", "around_2018", "latest")
    for r in list(book["HM1.1.A1"].iter_rows(values_only=True))[5:]:
        iso = names.get(str(r[0] or "").strip())
        if iso in iso3:
            for i, point in enumerate(points):
                if number(r[1 + i]) and number(r[10 + i]):
                    rows.append((iso, str(int(r[10 + i])), "dwellings", number(r[1 + i])))
                    rows.append((iso, str(int(r[10 + i])), "dwellings_per_1000", number(r[7 + i])))
        elif r[0] and len(str(r[0])) < 40:
            unmatched.add(str(r[0]).strip())
    for r in list(book["HM1.1.A2 "].iter_rows(values_only=True))[4:]:
        iso = names.get(str(r[0] or "").strip())
        if iso in iso3:
            for i in range(3):
                if number(r[1 + i]) and number(r[4 + i]):
                    rows.append((iso, str(int(r[4 + i])), "completions", number(r[1 + i])))
    for r in list(book["HM1.1.2"].iter_rows(values_only=True))[5:]:
        iso = names.get(str(r[12] or "").strip())
        if iso in iso3 and number(r[13]):
            rows.append((iso, "", "vacant_pct_latest", number(r[13])))
    manifest["series"]["households/ahd_stock"] = {
        "title": "Dwelling stock (number and per 1000 inhabitants), dwellings completed in the year, around 2011, "
                 "around 2018 and the latest year, and vacant dwellings excluding seasonal homes as a percentage of "
                 "the stock (latest year, around 2022); OECD Affordable Housing Database HM1.1 (tables A1, A2 and "
                 "figure 1.1.2's data)",
        "rows": table(OUT / "ahd_stock.csv", ["iso3", "year", "measure", "value"], sorted(set(rows))),
        "for": ["S2.05"],
    }
    book = openpyxl.load_workbook(cached(cache, "ahd_hc21.xlsx", AHD + "HC2-1-Living-space.xlsx"), read_only=True,
                                  data_only=True)
    lines = list(book["HC2.1.A1"].iter_rows(values_only=True))
    head = next(r for r in lines if sum(isinstance(c, int) and 2000 <= c <= 2100 for c in r) > 3)
    years = {i: c for i, c in enumerate(head) if isinstance(c, int) and 2000 <= c <= 2100}
    rooms, country = [], None
    for r in lines[lines.index(head) + 1:]:
        if isinstance(r[0], str) and r[0].strip():
            country = names.get(r[0].strip())
        label = next((AHD_ROOMS[c.strip()] for c in r[:3] if isinstance(c, str) and c.strip() in AHD_ROOMS), None)
        if country in iso3 and label:
            rooms += [(country, year, label, number(r[i])) for i, year in years.items() if number(r[i])]
    manifest["series"]["households/ahd_rooms"] = {
        "title": "Average number of rooms per household member by tenure (own outright, owner with mortgage, rent "
                 "private, rent subsidised), by year; OECD Affordable Housing Database HC2.1.A1",
        "rows": table(OUT / "ahd_rooms.csv", ["iso3", "year", "tenure", "rooms_per_member"],
                      rooms),
        "for": ["S2.05"],
    }
    if unmatched:
        log(f"AHD names not matched: {sorted(unmatched)}")
    geo = two_letter()
    moved = [(geo.get(r["geo"]), r["time"], r["tenure"], r["deg_urb"], fmt(r['value']))
             for r in eurostat("ilc_hcmp05", {})]
    manifest["series"]["households/eurostat_moved_5y"] = {
        "title": "Share of population having moved to another dwelling within the last five years (%), by tenure "
                 "(owner with and without loan, tenant at market and reduced rent) and degree of urbanisation, 2012; "
                 "Eurostat EU-SILC ad hoc module (ilc_hcmp05)",
        "rows": table(OUT / "eurostat_moved_5y.csv", ["iso3", "year", "tenure", "urbanisation", "pct"],
                      [r for r in moved if r[0] in iso3]),
        "for": ["S2.05"],
    }
    book = openpyxl.load_workbook(cached(cache, "cps_hst_mig_a_1.xlsx", CPS_MOBILITY), read_only=True,
                                  data_only=True)
    rows, section = [], None
    for r in book.active.iter_rows(values_only=True):
        if isinstance(r[0], str) and r[0].strip().startswith(("Numbers", "Percent")) and r[1] is None:
            section = "thousands" if r[0].strip().startswith("Numbers") else "percent"
            continue
        year = re.match(r"^\s*(\d{4})", str(r[0] or ""))
        if section and year and FIRST <= int(year.group(1)) <= LAST and number(r[1]):
            rows += [(str(r[0]).strip(), section, name, number(v)) for name, v in zip(CPS_COLUMNS, r[1:10])
                     if number(v)]
    manifest["series"]["households/us_mobility"] = {
        "title": "Annual geographic mobility of persons one year old and over, by survey year (the year to March), "
                 "in thousands and percent: total, same residence, movers, movers within the United States (same "
                 "county; different county, same state; different state) and from abroad; US Census Bureau, CPS "
                 "ASEC, historical table A-1",
        "rows": table(OUT / "us_mobility.csv", ["survey_year", "unit", "measure", "value"], rows),
        "for": ["S2.05"],
        "note": "survey years 2020 and 2021 appear twice, with 2010 and 2020 population controls, as published",
    }
    import xlrd
    sheet = xlrd.open_workbook(str(cached(cache, "avg_starttocomp_cust.xls", CENSUS_MONTHS))).sheet_by_index(0)
    rows, region = [], None
    for i in range(sheet.nrows):
        r = sheet.row_values(i)
        if isinstance(r[0], str) and r[0].strip() and not any(str(c).strip() for c in r[1:]):
            region = r[0].strip()
        elif isinstance(r[0], float) and region and FIRST <= int(r[0]) <= LAST:
            rows += [(region, int(r[0]), kind, number(v)) for kind, v in zip(CENSUS_MONTHS_COLUMNS, r[1:10])
                     if number(v)]
    manifest["series"]["households/us_residential_construction_months"] = {
        "title": "Average number of months from start to completion of new privately owned residential buildings "
                 "started in permit-issuing places, by year of completion, region, buildings with one unit (by "
                 "purpose: built for sale, contractor-built, owner-built) and with two or more units (by units in "
                 "the building); US Census Bureau, Survey of Construction",
        "rows": table(OUT / "us_residential_construction_months.csv", ["region", "year", "kind", "months"], rows),
        "for": ["S2.05"],
    }
    book = openpyxl.load_workbook(cached(cache, "db2020.xlsx", DOING_BUSINESS), read_only=True, data_only=True)
    lines = book[book.sheetnames[0]].iter_rows(values_only=True)
    topics = header = None
    kept, records = [], []
    for r in lines:
        if topics is None and r and any(c == "Registering property" for c in r):
            topics = {c: i for i, c in enumerate(r) if isinstance(c, str)}
            continue
        if topics is not None and header is None:
            header = [str(c).strip() if c else "" for c in r]
            starts = sorted(topics.values())
            for topic, columns in DB_COLUMNS.items():
                start = topics[topic]
                end = next((s for s in starts if s > start), len(header))
                for text, name in columns.items():
                    kept.append((name, next(i for i in range(start, end) if header[i] == text.strip())))
            at = {h: header.index(h) for h in ("Country code", "Economy", "DB Year")}
            continue
        if header is not None and r[at["Economy"]] and r[at["DB Year"]] and 2016 <= int(r[at["DB Year"]]) <= 2020:
            records.append(r)
    # The eleven economies measured in two cities have a row per city besides the economy's own, and a city's row
    # may carry the economy's code; a city's row is named after its economy and the city.
    economies = {str(r[at["Economy"]]).strip() for r in records}
    rows, unmatched = [], set()
    for r in records:
        economy = str(r[at["Economy"]]).strip()
        if any(economy.startswith(e + " ") for e in economies):
            continue
        iso = names.get(economy, r[at["Country code"]])
        if iso not in iso3:
            unmatched.add(economy)
            continue
        rows += [(iso, int(r[at["DB Year"]]), name, number(r[i])) for name, i in kept if number(r[i])]
    if unmatched:
        log(f"Doing Business economies not matched: {sorted(unmatched)}")
    manifest["series"]["households/doing_business_property"] = {
        "title": "Registering property (procedures, days, cost as % of the property's value, quality of land "
                 "administration index 0-30) and dealing with construction permits (procedures, days, cost as % of "
                 "the warehouse's value), by Doing Business year 2016-2020 (data of the year before); World Bank "
                 "Doing Business, DB2020 historical data",
        "rows": table(OUT / "doing_business_property.csv", ["iso3", "db_year", "measure", "value"], rows),
        "for": ["S2.05"],
        "note": "the case studied is a commercial property transfer between two firms and a warehouse's permit, "
                "the nearest cross-country measure of a dwelling sale's legal costs and a builder's permission lead "
                "time; economy-level rows only (the city rows of the eleven two-city economies are left out)",
    }
    manifest["sources"]["households/housing"] = {
        "title": "OECD Affordable Housing Database; Eurostat API; US Census Bureau CPS ASEC and Survey of "
                 "Construction; World Bank Doing Business archive",
        "url": "; ".join([AHD, EUROSTAT, CPS_MOBILITY, CENSUS_MONTHS, DOING_BUSINESS]), "fetched": TODAY}


def hazards(cache: Path, manifest: dict, iso3: set) -> None:
    """Assets' vulnerability by hazard: flood depth-damage curves and maximum damage, wind damage and crop yields'
    response to heat."""
    book = openpyxl.load_workbook(cached(cache, "jrc_flood.xlsx", JRC), read_only=True, data_only=True)
    rows, kind = [], None
    for r in list(book["Damage functions"].iter_rows(values_only=True))[3:]:
        if r[0]:
            kind = str(r[0]).strip()
        if kind is None or not isinstance(r[1], (int, float)):
            continue
        for i, region in enumerate(JRC_REGIONS):
            if number(r[2 + i]):
                rows.append((kind, region, number(r[1]), number(r[2 + i]), number(r[9 + i])))
    manifest["series"]["households/jrc_flood_damage"] = {
        "title": "Flood depth-damage functions: share of the maximum damage destroyed at each flood depth (m), by "
                 "damage class (residential, commercial and industrial buildings, transport, roads, agriculture) and "
                 "continent, with the standard deviation across the curves averaged; JRC global flood depth-damage "
                 "functions (Huizinga, de Moel and Szewczyk 2017, EUR 28552 EN)",
        "rows": table(OUT / "jrc_flood_damage.csv", ["class", "region", "depth_m", "damage_share", "sd"], rows),
        "for": ["S2.05", "S4.03"],
    }
    data = list(book["MaxDamage-Data"].iter_rows(values_only=True))[2:]
    iso_of = {str(r[0]).strip(): r[1] for r in data if r[0] and r[1]}
    maxima = {}
    for sheet, kind in (("MaxDamage-Residential", "residential"), ("MaxDamage-Commercial", "commercial"),
                        ("MaxDamage-Industrial", "industrial")):
        for r in list(book[sheet].iter_rows(values_only=True))[3:]:
            iso = iso_of.get(str(r[0] or "").strip())
            if iso in iso3 and number(r[1]) and float(number(r[1])) > 0:
                maxima[(iso, kind)] = (number(r[1]), number(r[2]))
    rows = [(r[1], kind, number(r[3 + i]), *maxima.get((r[1], kind), ("", "")))
            for r in data if r[1] in iso3 for i, kind in enumerate(("residential", "commercial", "industrial"))
            if number(r[3 + i]) and float(number(r[3 + i])) > 0]
    manifest["series"]["households/jrc_max_damage"] = {
        "title": "Construction cost per square metre of building (EUR of 2010) by country and building class, and "
                 "the maximum flood damage to structure and to content per square metre (EUR of 2010, building "
                 "based); JRC global flood depth-damage functions database",
        "rows": table(OUT / "jrc_max_damage.csv",
                      ["iso3", "class", "construction_cost_eur_m2", "max_damage_structure_eur_m2",
                       "max_damage_content_eur_m2"], rows),
        "for": ["S2.05", "S4.03"],
    }
    occupancy = {r["Occupancy"]: r for r in csv.DictReader(
        cached(cache, "hazus_haz_fl_occ.csv", HAZUS.format(name="haz_fl_occ.csv")).open(encoding="utf-8-sig"))}
    with cached(cache, "hazus_haz_fl_dept.csv", HAZUS.format(name="haz_fl_dept.csv")).open(encoding="utf-8-sig") as f:
        reader = csv.DictReader(f)
        depths = [c for c in reader.fieldnames if re.fullmatch(r"ft\d\dm?", c)]
        feet = lambda c: -int(c[2:4]) if c.endswith("m") else int(c[2:4])
        rows = []
        for r in reader:
            if r["Source"] not in HAZUS_SOURCES:
                continue
            occ = occupancy.get(r["Occupancy"], {})
            for c in depths:
                if number(r[c]):
                    rows.append((r["DmgFnId"], r["Occupancy"], occ.get("Occ_Desc2", ""), r["Cover_Class"],
                                 r["Source"], r["Description"], feet(c), number(r[c])))
    manifest["series"]["households/hazus_flood_damage"] = {
        "title": "Flood depth-damage functions: percent of the building's (Bldg), contents' (Cont) or inventory's "
                 "(Inv) value damaged at each depth of water above the first floor (feet, -4 to 24), by occupancy "
                 "class; FEMA HAZUS default and Federal Insurance Administration curves (HAZUS 5.1 library, archived "
                 "by A. Pollack, Zenodo doi 10.5281/zenodo.10027236)",
        "rows": table(OUT / "hazus_flood_damage.csv",
                      ["function", "occupancy", "occupancy_name", "cover", "source", "description", "depth_ft",
                       "damage_pct"], rows),
        "for": ["S2.05", "S4.03"],
    }
    manifest["series"]["households/typed_wind_damage"] = {
        "title": "Tropical cyclone wind damage: share destroyed f = v^3 / (1 + v^3), v = max(V - Vthresh, 0) / "
                 "(Vhalf - Vthresh), V the maximum sustained wind (m/s); Vthresh and Vhalf per calibration region, "
                 "calibrated on the spread of event damage ratios (vhalf_rmsf) and on total damage (vhalf_tdr), "
                 "with the region's economies",
        "rows": table(OUT / "typed_wind_damage.csv",
                      ["region", "events", "vthresh_ms", "vhalf_rmsf_ms", "vhalf_tdr_ms", "iso3s"],
                      [(r, n, WIND_THRESHOLD, a, b, c) for r, n, a, b, c in WIND]),
        "for": ["S2.05", "S4.03"],
        "note": "typed from " + EBERENZ,
    }
    manifest["series"]["households/typed_crop_temperature"] = {
        "title": "Crop yields' response to temperature and rain in the United States (county yields 1950-2005): "
                 "the temperature above which yields fall, yield-maximising growing-season precipitation and the "
                 "effect on corn of a day at 40 C instead of 29 C",
        "rows": table(OUT / "typed_crop_temperature.csv", ["crop", "measure", "value", "unit"], CROPS),
        "for": ["S2.05"],
        "note": "typed from " + SCHLENKER + "; the slopes below and above the threshold are shown only in the "
                "paper's Figure 2, not tabulated",
    }
    manifest["sources"]["households/hazards"] = {
        "title": "JRC global flood depth-damage functions (xlsx); HAZUS flood depth-damage library (Zenodo); "
                 "Eberenz et al. 2021 (NHESS); Schlenker and Roberts, NBER WP 13799",
        "url": "; ".join([JRC, HAZUS.format(name="<file>"),
                          "https://nhess.copernicus.org/articles/21/393/2021/nhess-21-393-2021.pdf",
                          "https://www.nber.org/system/files/working_papers/w13799/w13799.pdf"]),
        "fetched": TODAY}


def accidents(cache: Path, manifest: dict, iso3: set) -> None:
    """Road-traffic and fire mortality, and fire frequency."""
    page = json.loads(get(WDI_ROAD.format(first=FIRST, last=LAST)))
    rows = [(r["countryiso3code"], int(r["date"]), r["value"]) for r in page[1] or []
            if r["value"] is not None and r["countryiso3code"] in iso3]
    manifest["series"]["wdi/SH.STA.TRAF.P5"] = {
        "title": "Mortality caused by road traffic injury (per 100,000 population), WHO via World Bank WDI",
        "rows": table(RAW / "wdi" / "SH.STA.TRAF.P5.csv", ["iso3", "year", "value"], rows),
        "for": ["S4.03"],
    }
    data = json.loads(get(WHO_FIRE))["value"]
    rows = [(r["SpatialDim"], r["TimeDim"], r["NumericValue"]) for r in data
            if r["Dim1"] == "SEX_BTSX" and r["SpatialDim"] in iso3 and r["NumericValue"] is not None]
    manifest["series"]["households/who_fire_death_rate"] = {
        "title": "Age-standardised death rate from fires, both sexes (per 100,000), 2004; WHO Global Health "
                 "Observatory (SA_0000001443)",
        "rows": table(OUT / "who_fire_death_rate.csv", ["iso3", "year", "value"], rows),
        "for": ["S4.03"],
    }
    manifest["series"]["households/typed_ctif_fires"] = {
        "title": "Fire service calls and fires per 1000 inhabitants, fire deaths and injuries per 100,000 "
                 "inhabitants and per 100 fires, and residential buildings' share of fires (%), 2019",
        "rows": table(OUT / "typed_ctif_fires.csv",
                      ["iso3", "calls_per_1000", "fires_per_1000", "deaths_per_100k", "deaths_per_100_fires",
                       "injuries_per_100k", "injuries_per_100_fires", "residential_share_pct"],
                      [r for r in FIRES if r[0] in iso3]),
        "for": ["S4.03"],
        "note": "typed from " + CTIF + "; the report's first row (population 328 240 thousand) carries no name and "
                "is the United States (Table 1.4's first row, same population); residential shares only where "
                "Table 1.4's row reads unambiguously",
    }
    manifest["sources"]["households/accidents"] = {
        "title": "World Bank API (WDI); WHO Global Health Observatory OData API; CTIF World Fire Statistics No. 26",
        "url": "; ".join([WDI_ROAD, WHO_FIRE, "https://www.ctif.org/sites/default/files/2021-06/CTIF_Report26_0.pdf"]),
        "fetched": TODAY}
    log("household insurance claims frequency: no open cross-country source found (OECD Global Insurance "
        "Statistics carry premiums and claims paid, not claim counts)")


def energy(cache: Path, manifest: dict, iso3: set) -> None:
    """RECS 2020 microdata reduced to what energy needs per degree-day by household composition read."""
    rows = []
    with cached(cache, "recs2020.csv", RECS).open(encoding="utf-8-sig") as f:
        for r in csv.DictReader(f):
            rows.append([r[c] if c in RECS_TEXT else number(r[c], 5) for c in RECS_COLUMNS])
    manifest["series"]["households/recs2020"] = {
        "title": "Residential Energy Consumption Survey 2020, one row per household: census region, "
                 "building-America climate zone, urban type, heating and cooling degree-days (base 65 F, 2020), "
                 "housing unit type, tenure, year built range, rooms, floor area (sq ft), household members, "
                 "children, householder's age, income range, main heating fuel, air conditioning, energy use by fuel "
                 "(electricity, natural gas, propane, fuel oil, wood; thousand Btu), space heating, water heating "
                 "and electric cooling use (thousand Btu), total use and total spending (USD), and the final weight "
                 "NWEIGHT (replicate weights left out); US EIA",
        "rows": table(OUT / "recs2020.csv", RECS_COLUMNS, rows),
        "for": ["S2.09"],
        "note": "numbers rounded to five significant figures",
    }
    manifest["sources"]["households/recs"] = {"title": "US EIA, Residential Energy Consumption Survey 2020, public "
                                                       "use microdata (v7)", "url": RECS, "fetched": TODAY}


def insolvency(cache: Path, manifest: dict, iso3: set) -> None:
    """Personal bankruptcies: US cases by chapter and nature of debt, and England and Wales by procedure."""
    rows = []
    for year in range(FIRST, LAST + 1):
        page = get(USCOURTS_PAGE.format(year=year)).decode("utf-8", "replace")
        link = re.search(r'href="([^"]*bf_f2_1231\.\d{4}\.xlsx)"', page)
        if link is None:
            log(f"US courts F-2 for {year}: no .xlsx workbook linked (only .xls or none); year left out")
            continue
        book = openpyxl.load_workbook(cached(cache, f"bf_f2_{year}.xlsx", USCOURTS + link.group(1)),
                                      read_only=True, data_only=True)
        lines = list(book[book.sheetnames[0]].iter_rows(values_only=True))
        text = lambda c: re.sub(r"\s+", " ", str(c or "")).replace("¹", "").strip()
        top = next(r for r in lines if any(text(c).startswith("Total Chapter") for c in r))
        kinds = next(r for r in lines if any(text(c).startswith("Nonbusiness") for c in r))
        chapters = next(r for r in lines[lines.index(kinds) + 1:] if any(text(c).startswith("Chapter") for c in r))
        total = next(r for r in lines if text(r[0]).upper() == "TOTAL")
        nature = "all"
        for i in range(1, len(total)):
            if text(kinds[i]).startswith(("Business", "Nonbusiness")):
                nature = text(kinds[i]).split()[0].lower()
            label = text(chapters[i] if nature != "all" else top[i])
            label = re.sub(r"^Total\s*|\s*Filings$", "", label).strip() or "All Chapters"
            if label.startswith(("Chapter", "Other", "All")) and number(total[i]):
                rows.append((year, nature, label.replace("All Chapters", "all").replace("Other Chapters", "other"),
                             number(total[i])))
    manifest["series"]["households/us_bankruptcy_filings"] = {
        "title": "US bankruptcy cases commenced in the calendar year, by chapter (7, 11, 13, other) and "
                 "predominant nature of debt (all, business, nonbusiness), whole country; Administrative Office of "
                 "the US Courts, Table F-2 (12 months ending 31 December)",
        "rows": table(OUT / "us_bankruptcy_filings.csv", ["year", "nature", "chapter", "cases"], rows),
        "for": ["S2.11"],
    }
    collection = get(UK_INSOLVENCY).decode("utf-8", "replace")
    releases = sorted(set(re.findall(r'/government/statistics/individual-insolvency-statistics-october-to-december-'
                                     r'(\d{4})', collection)))
    release = get(f"https://www.gov.uk/government/statistics/individual-insolvency-statistics-october-to-december-"
                  f"{releases[-1]}").decode("utf-8", "replace")
    url = re.search(r'(https://assets\.publishing\.service\.gov\.uk/media/\w+/Long-Run_Series_in_CSV_Format[^"]*'
                    r'\.csv)', release).group(1)
    by_year = {}
    with cached(cache, f"uk_insolvency_{releases[-1]}.csv", url).open(encoding="utf-8-sig") as f:
        for r in csv.DictReader(f):
            year = int(r["Year"])
            if FIRST <= year <= LAST:
                sums = by_year.setdefault(year, {c: 0 for c in UK_COLUMNS} | {"quarters": 0})
                sums["quarters"] += 1
                for c in UK_COLUMNS:
                    sums[c] += int(number(r[c])) if number(r[c]) else 0
                if r["Quarter"] == "Q4":
                    sums[UK_RATE] = number(r[UK_RATE])
    rows = [(year, *(s[c] for c in UK_COLUMNS), s.get(UK_RATE, "")) for year, s in by_year.items()
            if s["quarters"] == 4]
    manifest["series"]["households/uk_individual_insolvency"] = {
        "title": "Individual insolvencies in England and Wales by calendar year: total, bankruptcies, debt relief "
                 "orders and individual voluntary arrangements (not seasonally adjusted, sums of quarters), and the "
                 "individual insolvency rate per 10,000 adults in the twelve months to Q4; UK Insolvency Service, "
                 "long-run series",
        "rows": table(OUT / "uk_individual_insolvency.csv",
                      ["year", "total", "bankruptcy", "debt_relief_orders", "iva", "rate_per_10000_adults"], rows),
        "for": ["S2.11"],
        "note": "England and Wales only (Scotland and Northern Ireland have their own procedures); sums of the four "
                "quarters of each complete year, a procedure not yet existing in a quarter ('[z]') counting none",
    }
    manifest["sources"]["households/insolvency"] = {
        "title": "US Courts, Table F-2 (Business and Nonbusiness Cases Commenced, by Chapter); UK Insolvency "
                 "Service, individual insolvency statistics, long-run series CSV",
        "url": USCOURTS_PAGE.format(year="<year>") + "; " + url, "fetched": TODAY}


def mpc(cache: Path, manifest: dict, iso3: set) -> None:
    manifest["series"]["households/typed_mpc"] = {
        "title": "Marginal propensities to consume (share of a windfall spent within the year) by cash on hand, "
                 "income, financial assets, deposits, debt and windfall size, and the rest of the windfall's use, "
                 "from two papers (Italy, survey; Norway, lottery prizes)",
        "rows": table(OUT / "typed_mpc.csv", ["source", "table", "group", "measure", "value", "se"], MPC),
        "for": ["S6.03"],
        "note": "typed from the two papers' working-paper versions downloaded on the fetch date; each row names "
                "its paper and table",
    }
    manifest["sources"]["households/mpc"] = {
        "title": "Jappelli and Pistaferri (CSEF WP 325); Fagereng, Holm and Natvik (Statistics Norway DP 852)",
        "url": "https://www.csef.it/WP/wp325.pdf; https://www.ssb.no/en/forskning/discussion-papers/"
               "mpc-heterogeneity-and-household-balance-sheets", "fetched": TODAY}


def coal(cache: Path, manifest: dict, iso3: set) -> None:
    """Coal deposits as worked in the United States: each mine's output, workforce and the quality of what it
    shipped, and reserves by state."""
    import xlrd
    sheet = xlrd.open_workbook(str(cached(cache, f"coalpublic{COAL_YEAR}.xls",
                                          EIA_MINES.format(year=COAL_YEAR)))).sheet_by_index(0)
    head_at = next(i for i in range(sheet.nrows) if sheet.row_values(i)[:2] == ["Year", "MSHA ID"])
    head = sheet.row_values(head_at)
    keep = ["MSHA ID", "Mine State", "Mine County", "Mine Status", "Mine Type", "Coal Supply Region",
            "Production (short tons)", "Average Employees", "Labor Hours"]
    at = [head.index(c) for c in keep]
    rows = []
    for i in range(head_at + 1, sheet.nrows):
        r = sheet.row_values(i)
        rows.append([str(int(r[at[0]])) if isinstance(r[at[0]], float) else r[at[0]]]
                    + [str(r[j]).strip() for j in at[1:6]] + [number(r[j]) for j in at[6:]])
    manifest["series"]["households/coal_mines"] = {
        "title": f"US coal mines in {COAL_YEAR}: MSHA identifier, state, county, status, surface or underground, "
                 "coal supply region, production (short tons), average employees and labour hours; US EIA and MSHA, "
                 "historical coal production data",
        "rows": table(OUT / "coal_mines.csv",
                      ["msha_id", "state", "county", "status", "mine_type", "supply_region", "production_short_tons",
                       "employees", "labour_hours"], rows),
        "for": ["S2.05", "S0.13"],
    }
    z = zipfile.ZipFile(cached(cache, f"f923_{COAL_YEAR}.zip", EIA_923.format(year=COAL_YEAR)))
    name = next(n for n in z.namelist() if "Schedules_2_3_4_5" in n)
    book = openpyxl.load_workbook(io.BytesIO(z.read(name)), read_only=True, data_only=True)
    lines = book["Page 5 Fuel Receipts and Costs"].iter_rows(values_only=True)
    head = None
    mines = {}
    for r in lines:
        if head is None:
            if r[0] == "YEAR":
                head = [str(c).replace("\n", " ") for c in r]
                col = {c: head.index(c) for c in ("FUEL_GROUP", "ENERGY_SOURCE", "Coalmine Type", "Coalmine State",
                                                  "Coalmine County", "Coalmine Msha Id", "QUANTITY",
                                                  "Average Heat Content", "Average Sulfur Content",
                                                  "Average Ash Content")}
            continue
        if r[col["FUEL_GROUP"]] != "Coal" or not number(r[col["QUANTITY"]]):
            continue
        key = tuple(str(r[col[c]] or "").strip() for c in ("Coalmine Msha Id", "Coalmine State", "Coalmine County",
                                                            "Coalmine Type", "ENERGY_SOURCE"))
        q = float(number(r[col["QUANTITY"]]))
        m = mines.setdefault(key, [0.0] * 7)
        m[0] += q
        for j, c in enumerate(("Average Heat Content", "Average Sulfur Content", "Average Ash Content")):
            if number(r[col[c]]):
                m[1 + 2 * j] += q
                m[2 + 2 * j] += q * float(number(r[col[c]]))
    rows = [(*key, fmt(m[0], 12), *(fmt(m[2 + 2 * j] / m[1 + 2 * j], 4) if m[1 + 2 * j] else "" for j in range(3)))
            for key, m in mines.items() if m[0] > 0]
    manifest["series"]["households/coal_mine_quality"] = {
        "title": f"Coal delivered to US power plants in {COAL_YEAR} by originating mine (MSHA id, or IMP for "
                 "imports and blank where not reported), mine state and county, mine type (S surface, U "
                 "underground, P preparation plant, SU/US mixed), and coal rank (BIT bituminous, SUB subbituminous, "
                 "LIG lignite, ANT anthracite, WC waste coal, RC refined): tons received, and the quantity-weighted "
                 "heat content (MMBtu per ton), sulfur and ash (% by weight); US EIA-923 schedule 2 (fuel receipts "
                 "and costs)",
        "rows": table(OUT / "coal_mine_quality.csv",
                      ["msha_id", "state", "county", "mine_type", "rank", "tons", "mmbtu_per_ton", "sulfur_pct",
                       "ash_pct"], rows),
        "for": ["S2.05", "S0.13"],
        "note": "summed from the plant-month receipts; each average is weighted by the tons of the receipts that "
                "report it, and is blank where none does",
    }
    book = openpyxl.load_workbook(cached(cache, "eia_table15.xlsx", EIA_RESERVES), read_only=True, data_only=True)
    lines = list(book.active.iter_rows(values_only=True))
    title = str(lines[0][0])
    columns = ["underground_at_producing_mines", "underground_estimated_recoverable", "underground_reserve_base",
               "surface_at_producing_mines", "surface_estimated_recoverable", "surface_reserve_base",
               "total_at_producing_mines", "total_estimated_recoverable", "total_reserve_base"]
    rows = []
    for r in lines[5:]:
        if isinstance(r[0], str) and r[0].strip() and any(number(c) for c in r[1:10]):
            rows += [(r[0].strip(), name, number(v)) for name, v in zip(columns, r[1:10]) if number(v)]
    manifest["series"]["households/coal_reserves"] = {
        "title": f"{title}: million short tons by coal-resource state and mining method (recoverable reserves at "
                 "producing mines, estimated recoverable reserves, demonstrated reserve base); US EIA Annual Coal "
                 "Report Table 15",
        "rows": table(OUT / "coal_reserves.csv", ["state", "measure", "million_short_tons"], rows),
        "for": ["S2.05", "S0.13"],
    }
    manifest["sources"]["households/coal"] = {
        "title": "US EIA: historical coal production data (mine level), EIA-923 fuel receipts, Annual Coal Report "
                 "Table 15", "url": "; ".join([EIA_MINES.format(year=COAL_YEAR), EIA_923.format(year=COAL_YEAR),
                                               EIA_RESERVES]), "fetched": TODAY}
    log("USGS COALQUAL: its sample data are reached only through an ASP.NET search form; mine-level quality is "
        "taken from EIA-923 receipts instead")


def farms(cache: Path, manifest: dict, iso3: set) -> None:
    """Farms by economic size with their labour force, and US farms by hired workers."""
    geo = two_letter()
    rows = []
    for r in eurostat("ef_m_farmleg", {"leg_form": ["TOTAL"], "farmtype": ["TOTAL"], "uaarea": ["TOTAL"],
                                       "statinfo": ["TOTAL"], "unit": ["HLD", "AWU", "EUR"]}):
        if geo.get(r["geo"]) in iso3:
            rows.append((geo[r["geo"]], r["time"], r["so_eur"], "all", r["unit"], fmt(r['value'])))
    for r in eurostat("ef_lf_leg", {"leg_form": ["TOTAL"], "sex": ["T"], "statinfo": ["TOTAL"]}):
        if geo.get(r["geo"]) in iso3:
            rows.append((geo[r["geo"]], r["time"], r["so_eur"], r["wstatus"], r["unit"], fmt(r['value'])))
    manifest["series"]["households/eurostat_farms_by_size"] = {
        "title": "Agricultural holdings (HLD), their standard output (EUR) and annual work units (AWU), and their "
                 "labour force in persons (PER) and AWU by work status (sole holders, family members, regular "
                 "non-family workers, non-regular, not directly employed, managers), by economic size class of "
                 "standard output (EUR 0-2k, 2-8k, 8-25k, 25-100k, 100k+), all legal forms, by country and survey "
                 "year; Eurostat farm structure survey and integrated farm statistics (ef_m_farmleg, ef_lf_leg)",
        "rows": table(OUT / "eurostat_farms_by_size.csv",
                      ["iso3", "year", "so_class", "work_status", "unit", "value"], rows),
        "for": ["S1.24", "S1.03"],
    }
    rows = []
    with gzip.open(cached(cache, "qs.census2017.txt.gz", NASS), "rt", encoding="utf-8", errors="replace") as f:
        reader = csv.reader(f, delimiter="\t")
        head = next(reader)
        at = {h: head.index(h) for h in ("SHORT_DESC", "DOMAIN_DESC", "DOMAINCAT_DESC", "AGG_LEVEL_DESC", "VALUE")}
        for r in reader:
            if r[at["AGG_LEVEL_DESC"]] != "NATIONAL":
                continue
            short = r[at["SHORT_DESC"]]
            # Farm counts by one characteristic at a time; the cross-tabulations are left out.
            if short.startswith("LABOR, HIRED") or (short == "FARM OPERATIONS - NUMBER OF OPERATIONS"
                                                     and " AND " not in r[at["DOMAIN_DESC"]]):
                rows.append((short, r[at["DOMAIN_DESC"]], r[at["DOMAINCAT_DESC"]], number(r[at["VALUE"]])))
    manifest["series"]["households/us_farms_hired_workers"] = {
        "title": "US farm operations and hired farm labour in 2017, national: operations by area operated, economic "
                 "class, sales, NAICS industry, legal organisation, producers and tenure; operations with hired "
                 "workers, workers and labour expense, by number of hired workers (1, 2, 3-4, 5-9, 10 or more), days "
                 "worked (150 or more, under 150) and expense class; USDA NASS Census of Agriculture 2017 (Quick "
                 "Stats bulk file)",
        "rows": table(OUT / "us_farms_hired_workers.csv", ["item", "domain", "class", "value"],
                      [r for r in rows if r[3]]),
        "for": ["S1.24", "S1.03"],
        "note": "values withheld for disclosure ('(D)') are left out",
    }
    manifest["sources"]["households/farms"] = {
        "title": "Eurostat API (ef_m_farmleg, ef_lf_leg); USDA NASS Quick Stats, Census of Agriculture 2017 bulk file",
        "url": EUROSTAT + "statistics/1.0/data/<dataset>; " + NASS, "fetched": TODAY}


SOURCES = {"scf": scf, "hfcs": hfcs, "prices": prices, "housing": housing, "hazards": hazards,
           "accidents": accidents, "energy": energy, "insolvency": insolvency, "mpc": mpc, "coal": coal,
           "farms": farms}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cache", type=Path, default=None)
    parser.add_argument("--only", nargs="*", choices=sorted(SOURCES))
    args = parser.parse_args()
    cache = args.cache or Path(tempfile.mkdtemp())
    cache.mkdir(parents=True, exist_ok=True)
    iso3 = countries()
    for name in args.only or list(SOURCES):
        manifest = {"sources": {}, "series": {}}
        SOURCES[name](cache, manifest, iso3)
        merge_manifest(manifest["sources"], manifest["series"])
        for key, s in manifest["series"].items():
            log(f"{key}: {s['rows']} rows")


if __name__ == "__main__":
    main()
