# Item 13c — Commodities and freight

**Objective.** Commodities as standardised goods whose location is part of their identity, produced
by named producers at dispersed costs with fixed short-run capacity, stored at a cost paid to an
owner, consumed by firms and households, and cleared per grade and location with inventory as the
state variable the price reads; freight as a service bought from named carriers with finite capacity
on distinct routes, so that moving goods costs money and time and the gap between two locations'
prices is bounded by somebody actually shipping; commodity futures on 13a's layer whose curve reads
the physical state because delivery is possible. This item also closes the producer/consumer price
indices left PARTIAL at item 12.

**Read first.** §21 Commodities Spot (all); §20 Commodity Futures (all); §38 Freight (all); Goods A1,
D (locations and delivery, PARTIAL from item 4), E2.b, G; Indices D4, D4.a; Capital Programme A4
(capital kinds); Part XII chains (inventories → backwardation; commodity shock → margins →
inflation → policy). Code: item 4's `goods` and `firms`, item 10's `plant` and capital kinds, 13a's
contract markets, 12's `indices`.

**Clauses this item meets.** Commodities Spot A1, A1.a, A2, A3, A4, B1, B1.a, B2, B2.a, B3, B4,
C1–C4, D1, D2, D2.a, D3, D4, D5, E1, E2, E3 (with 13i), E4 (a chain measured at 16), F1–F3;
Commodity Futures A1, A1.a–A1.d, A2, A3, A4, B1–B5, B3.a, C1, C1.a, C1.b, C2, C3, C4, C4.a, D1–D4,
E1–E3; Freight A1–A4, A3.a, B1–B4, B2.a, C1, C1.a, C2, C3, D1–D6, D3.a, E1–E3; Goods D (all), E2.b,
G1.c; Indices D4, D4.a (PARTIAL → MET).

---

## Design

### Module `commodities`

- `requires: ['goods', 'firms', 'households', 'capital-programme']`.
- **Identity** (A1, A1.a): a commodity is a `good` kind whose registry row carries `grade` and whose
  instances are per **location**: the good's instrument id is `good:<grade>@<location>`, so the same
  grade in two places is two instruments with two prints (D1). Units are the registry's physical unit
  for the grade (tonnes, barrels, MWh: units declared by the module).
- **Inventory** (A4, D2.a): units held in the register by named parties at a location; it carries
  across periods; production adds by a `create` leg on the producer's holding (a `firms.produce`
  outcome from its recipe and plant: B1); consumption destroys by the consumer's recipe or a
  household's consumption instruction; the units family's D5 contribution: produced + opening =
  consumed + closing per (grade, location), exactly, from the ledger (F1, F2: a negative holding is
  a kernel contract violation already).
- **Storage** (A3, D3): a holder pays storage per unit per period to the owner of the storage: a
  `storage` capital kind (item 10: plant owned by a firm or a `warehouse` party of kind `firm`) with
  capacity in units; the storage fee is a price cleared in a small market per location (`storage@
<location>`: capacity offered against demand to hold); a holder with no storage cannot hold units
  past the period (its excess is offered into the spot market: no free storage).
- **Supply** (B): producers are firms whose recipe outputs the grade at a location from plant and
  inputs; their costs differ by seed dispersion (B1.a: the supply schedule is the set of their
  reservations at own unit cost, posted through `firms.decide`); capacity fixed short-run by plant
  (B2, item 10); **disruption** (B3) is a journaled destroy leg on a producer's plant or inventory
  with a cause, and the cause is the `environment` module's own published event (below), never a
  price multiplier and never a draw private to this module; a producer may hold rather
  than sell when its outlook of the price exceeds today's print plus carry (B4: its own view).
- **Demand** (C): firms whose recipes consume the grade (C1, C2: the recipe is fixed in the period, so
  their bids are inelastic: quantity needed at any price up to their own margin); households for
  energy (E2: a consumption line in the cell's basket); investors that buy to hold (C3: a fund with a
  commodity mandate holding physical units in storage it pays for).
- **The market** (D1): one market per (grade, location) in the `markets` phase, cleared by the kernel
  solver; C4 (large moves from small imbalances) is measured at 16.
- **What it feeds** (E1): the commodity's cleared price enters unit cost through item 4's recipe cost
  lines (a firm's input purchase is a real trade at the print), so producer prices move before
  consumer prices (Indices D4.a); E3 (terms of trade) is read at 13i.

### Module `freight`

- `requires: ['goods', 'capital-programme']`.
- **Party kind** `carrier` (B1): a firm-like named party owning plant of capital kinds `vessel`,
  `truck`, `warehouse` (units of capacity per route per period: technology), with an operating cost
  per unit moved (B3: fuel bought as a commodity at 13c, labour from the labour market, the capital
  charge from item 10).
- **Routes** (A4): data: `(from, to, transitPeriods, capacityKind)`; capacity on one route is not
  capacity on another; a **blocked route** is a journaled event that takes a route's capacity away
  for the periods the event lasts (B4: a real loss of units moved), read from the same
  `environment.state` event a producer's disruption is read from — one cause, two consequences.
- **The market** (D1): one market per route in the `markets` phase, ordered **before** the goods
  markets whose deliveries need it: a shipper's goods order across locations is posted with a freight
  order for the units, and the goods trade is admitted only up to the freight fill (D6: capacity
  rations quantity; shippers turned away are journaled `freight.refused` with sizes; the goods order's
  remainder is `noSupply` for that buyer). Demand is read from the goods trades of the session (C3:
  never a separate series; C1.a).
- **In transit** (A3, A3.a, E3): on the freight trade, the goods move to the shipper's holding at
  location `transit:<route>` (an instrument `good:<grade>@transit:<route>` owned by the shipper,
  carried at cost); after `transitPeriods` the kernel's corporate actions deliver them to the
  destination holding (a `reseat` of the units' location: journaled). Working capital is tied up for
  the transit (A3.a: it shows in the shipper's balance sheet and cash).
- **Delivered price** (D2): the buyer's cost line for the good = price at origin + freight paid; item
  4's unit-cost read includes it; the seller's margin is unchanged.
- **Location basis** (D3, D3.a): the gap between two locations' prints is an outcome; a shipper
  arbitrages it when the gap exceeds freight plus carry and capacity exists; when capacity binds or
  the route is blocked, the gap widens and stays (a scenario test asserts direction).
- **Substitution** (C2): a shipper facing a freight price above what the trade earns holds, sources
  locally (its goods order goes to the local market), or does not trade: its `decide` phase compares
  delivered prices per source location from its own view.

### Module `environment` (the physical world as standing state)

Four sections require the consequences of a physical shock and nothing in this world produces one:
`Commodities B3` (production disrupted — a real loss of units at the point they would have been
made), `Goods B4` (**yield**: not everything started is finished), `Freight B4` (capacity lost or
blocked) and `Insurers B4` (a catastrophe is one event hitting many policies at once, which is
different from the average being higher). What produces any of them otherwise is a scenario seed's
event plus, in insurance, per-line frequency and severity declared as technology — so the insurer's
loss and the producer's loss would be **two unrelated draws for one event**, which is `Law 4`'s "one
representation per real thing" one level above a number. And the chain `Commodities E4` names —
commodity shock to margins to inflation to policy — could only ever be exercised by injecting a
scenario, never by the world producing one.

- `requires: ['goods']` (it needs units and regions and nothing else; every reader reads it through
  the kernel, not through an import).
- **What it is** (`Law 2`): TECHNOLOGY. A physical fact with a **unit, an owner, a region and a
  period**, carried as state by this module and written by nobody else: growing conditions per region
  and season, water and wind, the state of a route's passage, the hazard standing over a region this
  period. Each is a declared parameter with its unit and its owner, drawn per region and period from
  the world's own stream — never a probability living inside the module that consumes it (13h refuses
  `catastrophe.probability` as a primitive, and this is that refusal seen from the other side).
- **How it crosses** (4.9b): as a **public event**, `environment.state`, published each period per
  region before the production phases, plus the legs it draws itself where the loss actually lands.
  Commodities, goods, freight and insurance READ the event; none of them writes it. A module never
  imports another, and this is the same door `bank.capital` and `deposit.classes` already cross.
- **One event, several consequences** (the whole point): one disruption is a `destroy` leg on a named
  producer's inventory or plant **where the units were**, a yield shortfall on the batches that were
  started (`Goods B4`: the batch's own `due` returns fewer units than its recipe promised, and the
  difference is a real loss on the lot), a route's capacity gone for the periods the event lasts
  (`Freight B4`), and — when 13h's cover market lands — the claims of every policy of that line in
  that region at once (`Insurers B4`). Each is a real destruction of units or capacity at its own
  site; nothing anywhere multiplies a price.
- **What it is not**: not a shock schedule, not a written path, not a "scenario mode". The state
  varies period to period as state, and a scenario at 16 is a stated OPENING of it, not a second
  mechanism.

### Module `commodity-futures`

- `requires: ['commodities', 'derivative-layer', 'dealers', 'bank-lending']`.
- **Kind** `commodity.future`: terms `{ grade, location, expiry, contractSize }` (A1.a, A1.d: size in
  contracts × a fixed quantity); a **series** of expiries per (grade, location) (A3: the curve is the
  set of prints); underlying = the spot print at expiry (A1.a, D4); margined by 13a (A4).
- **Expiry** (D1): physical delivery: an asset leg of the units from the short's holding at the
  location to the long against cash at the strike (one instruction, DvP); a party with no units to
  deliver or no storage to receive must have closed or rolled before expiry (D3: its participant
  posts the closing trade in the last session before expiry; a party that neither delivers nor
  closed is in item 5's failed-delivery state and the contract cash-settles against the spot print at
  the failure's cost); cash settlement for contracts declared cash-settled (D1, D4).
- **Participants** (B): producers hedging output they will have (B1: from their production plan),
  consumers hedging inputs (B2: from their recipes), investors rolling (B3, B3.a: the roll is a
  closing and an opening trade at the two prints; its cost lands in P&L by construction: E3),
  the **storage arbitrageur** (B4: a party that can store and finance: a firm with storage capacity
  and a loan quote from item 6: it buys spot, stores, sells the future when the future exceeds spot
  - storage + financing; it cannot act without storage: C1.a's bound is this participant's limit),
    desks (B5).
- **The curve** (C): contango bounded above by carry through B4's arbitrage; backwardation unbounded
  (C1.b: no participant can short a shortage); convergence at expiry because delivery is possible
  (C4, C4.a: no boundary condition in code: the profile's `mark` at expiry is the settlement, and
  the last session's print is what the participants struck knowing delivery is real); E2: open
  interest against deliverable supply is a read; a squeeze is reachable when shorts must buy from a
  small deliverable stock (measured at 16).

### Price indices (Indices D4)

Item 12's producer and consumer price indices gain freight and the distribution margin (Goods E2.b,
G1.c): the consumer index reads the delivered retail print, the producer index the origin print;
their divergence is the margin story (D4.a). PARTIAL → MET.

### Parameters

`environment.<fact>.<region>` (technology: unit, owner, region, period — the standing physical
state); `commodity.grades.*` (data: unit, locations), `storage.capacity.<location>` (technology via plant),
`route.*` (data), `carrier.capacity.<kind>` (technology), `commodity.future.contractSize.<grade>`
(data), `commodity.future.expiries` (data). No `inventory.percent`, no `basis.*`, no
`convergence.*`, no `volatility.*` exist.

### Audit contributions

- `units`: D5 per (grade, location) including transit; carrier capacity used ≤ owned (E2).
- `flows`: freight paid by shippers equals freight received by carriers per route per period.
- `names`: every unit in transit has an owner (E3); every route has a carrier.

### Files

```
packages/engine/src/mechanisms/environment/{index.ts,state.ts,events.ts}
packages/engine/src/mechanisms/commodities/{index.ts,inventory.ts,storage.ts,producers.ts,consumers.ts}
packages/engine/src/mechanisms/freight/{index.ts,routes.ts,market.ts,transit.ts,carrier.ts}
packages/engine/src/mechanisms/commodity-futures/{index.ts,contract.ts,expiry.ts,participants.ts,arbitrage.ts}
packages/engine/src/mechanisms/goods/… (delivered price, location in identity)
packages/engine/src/mechanisms/indices/prices.ts (freight and distribution margin)
packages/engine/test/{environment,commodity-spot,inventory,storage,disruption,freight,transit,location-basis,futures,expiry,roll,curve}.test.ts
```

---

## Steps

- [ ] `environment` as standing state: a physical fact per (region, period) with a unit and an owner, declared TECHNOLOGY, written by this module and by nothing else, published as a public event every period before production; tests: no reader writes it and no consumer of it declares a hazard of its own (Law 2, Law 4)
- [ ] One event, several consequences: the same published state is a producer's destroyed units, a yield shortfall on batches already started, a route's capacity gone, and (declared PARTIAL to 13h) every policy of a line in that region; each a real loss where it happens and none a multiplier on a price; tests (Commodities B3, Goods B4, Freight B4, Insurers B4)
- [ ] Commodity as a good with grade and location in its identity; inventory per (holder, location) carried across periods; storage as capacity with a cleared fee paid to its owner; no free holding; tests (A1–A4, D2.a, D3, F2)
- [ ] Producers with dispersed costs and plant-fixed capacity; disruption as a journaled loss of units with a cause; a producer holds on its own outlook; tests (B1–B4, F3)
- [ ] Consumers by recipe, households for energy, investors holding physical in paid storage; spot clears per (grade, location); units family D5 contribution; tests (C1–C3, D1, D5, F1)
- [ ] `carrier` party kind with route capacity as plant, operating cost lines, blocked-route event; tests (Freight A, B)
- [ ] Freight market per route ordered before the goods it carries; capacity rations quantity with refusals journaled; demand read from the session's goods trades; tests (C1, C3, D1, D6, E2)
- [ ] Goods in transit owned by the shipper at a transit location, delivered after the transit lag by corporate actions; delivered price includes freight in the buyer's cost line; tests (A3, A3.a, D2, D4, E1, E3)
- [ ] Location basis as an outcome of shipping with capacity; scenario test: a blocked route widens the gap and it stays (D3, D3.a, D5 direction)
- [ ] Shipper substitution: hold, source locally, or not trade, from delivered prices in its own view; test (C2)
- [ ] `commodity.future` kind on the layer: series of expiries, size in contracts, margined; physical delivery as DvP at expiry or cash against the spot print; failed delivery as a recorded state; tests (A1–A4, D1–D4)
- [ ] Futures participants: producer and consumer hedges from their own plans, investors that roll with the roll cost landing in P&L, the storage arbitrageur that needs storage and a loan quote, desks; tests (B1–B5, E3)
- [ ] The curve as an outcome: contango bounded by carry only through the arbitrageur, backwardation unbounded, convergence because delivery is possible, open interest vs deliverable supply as a read; tests assert mechanism, not level (C1–C4, E1, E2)
- [ ] Producer and consumer price indices complete with freight and the distribution margin (Indices D4 → MET); commodity cost into unit cost; observer: inventories, curves, routes, refusals, bases; year-long run green; determinism
- [ ] Coverage re-marked; record entry
- [ ] Delete this file; worklist row 13c → done; commit and push

## Exit criteria

Two locations have two prices and the gap is bounded by a shipper with capacity; inventory carries
and the spot price reads it; a future converges at expiry because a short can deliver; the roll
costs the roller; the consumer index can diverge from the producer index.

## Guard

Commodities Spot F1–F3, D2.a; Commodity Futures C4.a, E1–E3; Freight E1–E3; Commodities Spot F2 (no
negative inventory); Law 4 (one physical event has one representation, read by four systems and
written by none of them).
