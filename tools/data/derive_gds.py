#!/usr/bin/env python3
"""Derives the goods' technology: each extracted product's grade classes, how a deposit's grade falls as it is worked,
each storable product's rate of loss in stock and the room it is kept in, and which products are standardised.

- **Grade classes**: three for each extracted product, each holding a third of the deposits the map draws, so the
  bounds are the terciles of GEO's log-normal grade index, e^(sigma * z) at z = +/-0.4307, with sigma the deposit
  grade's spread GEO declares (data/shared/GEO_hazards.toml). Markets grade commodities in a few classes: coal in four
  ranks (EIA, Coal explained), crude oil in three bands of API gravity; three is assumed for every resource.
- **Grade fall**: assumed; the richest part of a deposit is worked first, and the grade left when it is exhausted is
  e^(-fall) of the opening's.
- **Spoilage**, a year's loss in stock, from the sources below; a loss L over d months is -ln(1-L)*12/d a year, and a
  loss per unit sold becomes a rate a stock-year by the times the stock turns over, 9.2 a year at the US Census MTIS
  total business inventory-to-sales ratio of 1.30 months (July 2026).
  - crops and livestock: APHLIS maize storage loss, Kagera (Tanzania) 2021, 5.26% at household storage over a store
    of up to ten to twelve months, taken as six (FAO, The State of Food and Agriculture 2019, p.29: storage losses of
    cereals median 7% in sub-Saharan Africa, 0.3-15% in eastern Asia, under 2% in India);
  - coal: 2% a year of energy in managed piles (Baruya, Losses in the coal supply chain, IEA CCC/212, 2012, p.37);
  - food: US retail-level loss 10% of retail supply (USDA ERS EIB-121, Buzby et al. 2014, Table 1) times the turnover;
  - consumer goods: the share of shrink from process, control failures and damage, 27% of 1.6% of sales (NRF National
    Retail Security Survey 2023, pp.8-9), times the turnover;
  - oil and gas and energy carriers: evaporation from floating-roof tanks, of the order of 0.1% a year (assumed;
    EPA AP-42 section 7.1 gives the method, not a rate);
  - metal ore, building stone, materials and capital goods: none (assumed: inert in store; their loss is obsolescence,
    a value, not a quantity).
- **Storage**: the room each is kept in, as the sources above keep it: grain in silos (USDA NASS Grain Stocks),
  food in cold stores (NASS Capacity of Refrigerated Warehouses), crude and fuels in tanks (EIA working storage
  capacity), ore, coal and stone in open yards, other goods in dry warehouses.
- **Standardised**: the products traded in call markets at each place, the commodities: crops and livestock and the
  four extracted products.

    python3 tools/data/derive_gds.py

Writes data/shared/GDS.toml.
"""
import math
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SHARED = ROOT / "data" / "shared"

TERCILE = 0.4307
TURNOVER = 12 / 1.30

# The products in TEC.products' order that are stored; the rest are services, never held.
PRODUCTS = [
    "crops and livestock", "metal ore", "coal", "oil and gas", "building stone",
    "food", "consumer goods", "materials", "energy carriers", "capital goods",
]
EXTRACTED = {1: 0, 2: 1, 3: 2, 4: 3}  # product place -> GEO resource


def spoilage() -> list[float]:
    crops = -math.log(1 - 0.0526375) * 12 / 6
    food = 0.10 * TURNOVER
    consumer = 0.27 * 0.016 * TURNOVER
    return [crops, 0.0, 0.02, 0.001, 0.0, food, consumer, 0.0, 0.001, 0.0]


# Rooms: 0 open yard, 1 dry warehouse, 2 cold store, 3 tank, 4 silo.
STORAGE = [4, 0, 0, 3, 0, 2, 1, 1, 3, 1]
GRADE_FALL = {1: 0.5, 2: 0.2, 3: 0.3, 4: 0.0}


def main() -> None:
    geo = tomllib.loads((SHARED / "GEO_hazards.toml").read_text())
    sigma = next(p for p in geo["primitive"] if p["id"] == "GEO.deposit_grade_sigma")["value"]["values"]
    bounds = [[round(math.exp(-sigma[r] * TERCILE), 3), round(math.exp(sigma[r] * TERCILE), 3)] for r in EXTRACTED.values()]
    fmt = lambda xs, d: "[" + ", ".join(f"{x:.{d}f}" for x in xs) + "]"
    places = list(range(len(PRODUCTS)))
    out = f'''# The goods' technology (GDS.13), derived by tools/data/derive_gds.py; each value's source is its entry's.

[[primitive]]
id = "GDS.grade_bounds"
kind = "TECHNOLOGY"
owner = "GDS"
source = "assumed"
source_ref = "The upper bounds of each extracted product's first two grade classes (rows: metal ore, coal, oil and gas, building stone; columns: the classes) in the deposit's grade index: three classes each a third of the deposits the map draws, the terciles of GEO's log-normal grade, as markets grade commodities in a few classes (coal in four ranks, EIA; crude in three bands of API gravity)."
value = {{ rows = [1, 2, 3, 4], columns = [0, 1], values = [{", ".join(fmt(b, 3) for b in bounds)}], outside = "refuse" }}

[[primitive]]
id = "GDS.grade_fall"
kind = "ENDOWMENT"
owner = "GDS"
source = "assumed"
source_ref = "How fast a deposit's grade falls as it is worked, the richest part first, by extracted product (metal ore, coal, oil and gas, building stone): the grade left when it is exhausted is e^-fall of the opening's; a quarry's stone is alike throughout."
value = {{ axis = [1, 2, 3, 4], values = {fmt(list(GRADE_FALL.values()), 3)}, outside = "refuse" }}

[[primitive]]
id = "GDS.spoilage_rate"
kind = "TECHNOLOGY"
owner = "GDS"
source = "estimated"
source_ref = "Each storable product's loss in stock a year (axis: the products' places): crops and livestock from APHLIS's 5.26% maize storage loss over a six-month store (FAO SOFA 2019 p.29 gives 0.3-15% by region); coal 2% a year of energy in managed piles (IEA CCC/212, 2012, p.37); food the US retail loss of 10% of supply (USDA ERS EIB-121, Table 1) and consumer goods the process and damage share of shrink, 27% of 1.6% of sales (NRF NRSS 2023), each times 9.2 turns a year (Census MTIS inventory to sales of 1.30 months); oil and gas and energy carriers 0.1% a year, assumed, from tank evaporation (EPA AP-42 7.1 gives no rate); ore, stone, materials and capital goods none, assumed, being inert in store."
value = {{ axis = {places}, values = {fmt(spoilage(), 4)}, outside = "refuse" }}

[[primitive]]
id = "GDS.storage"
kind = "TECHNOLOGY"
owner = "GDS"
source = "measured"
source_ref = "The room each storable product is kept in (axis: the products' places; 0 open yard, 1 dry warehouse, 2 cold store, 3 tank, 4 silo): grain in silos and bins (USDA NASS Grain Stocks), food in refrigerated warehouses (NASS Capacity of Refrigerated Warehouses), crude and fuels in tanks (EIA working storage capacity), ore, coal and stone in open stockpiles, other goods in dry warehouses."
value = {{ axis = {places}, values = {STORAGE}, outside = "refuse" }}

[[primitive]]
id = "GDS.standardised"
kind = "TECHNOLOGY"
owner = "GDS"
source = "assumed"
source_ref = "Whether each product is a standardised commodity traded in a call at each place (axis: the products' places): crops and livestock and the four extracted products, as exchanges list grains, livestock, metals, coal, crude and gas; made goods are sold between firms at posted prices."
value = {{ axis = {places}, values = [1, 1, 1, 1, 1, 0, 0, 0, 0, 0], outside = "refuse" }}

[[primitive]]
id = "GDS.extraction_days"
kind = "PREFERENCE"
owner = "GDS"
source = "assumed"
source_ref = "Days between an extractor's decisions on its deposits: a week, as firms plan production (FRM.production_days)."
value = 7

[[primitive]]
id = "GDS.spoilage_days"
kind = "RESOLUTION"
owner = "GDS"
source = "assumed"
source_ref = "Days between the realisations of a holder's spoilage, each over the days since the last: a month, which the loss's yearly rates make small per realisation."
value = 30
'''
    (SHARED / "GDS.toml").write_text(out)


if __name__ == "__main__":
    main()
