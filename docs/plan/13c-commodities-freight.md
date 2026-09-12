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

### One dependency, from the review

**A second FINAL good waits for 13d.** `ConsumptionDecl.share` is a share of what a household spends,
so spending on a good never responds to its price and a price change never moves spending BETWEEN
goods — unit-elastic demand, which is exactly what `phoenix/no-value-recipe` refuses on the
production side. It is inert today because there is one final good at `share: 1`, and it becomes the
strongest substitution assumption in the model the day there are two. 13d restates a cohort's
preference as a quantity it wants per period; until that lands, anything this item adds to the
consumption basket carries that assumption. Commodities that are INTERMEDIATE are unaffected — the
recipes they feed are already physical (Goods A2.a).


### The rest of this item waits for the map (13c.1)

**Inserted before step 9.** Steps 1–8 are done and needed no geography. Steps 9, 10 and 14 all
measure DISTANCE, and until 13c.1 lands this world has one producing place and four legs whose
technology is declared identical on every one of them (`transitPeriods: 4,
unitsPerVesselPerPeriod: 25000, sailsIn: 4`, `seeds/foundation.ts:222`) — so a basis test has one
location, a "source locally" test has nowhere else to source from, and the indices have a freight
cost that does not vary with anything. Freight A4 says capacity on one route is not capacity on
another; today it is one route wearing four labels.

13c.1 draws the map, makes every tile belong to exactly one PLACE (a land region or a sea area),
splits `RegionDecl` into a country that has the money and a region that is a place, and puts the
things that move onto it: a voyage is a row with a tile path and a distance travelled, its hulls held
by a lien, advancing each period by what the weather AT THE PLACE IT IS IN allowed — so a gale slows
the ships in that sea area, sinks some of them with their cargo, and leaves the next session short
of hulls. `RouteDecl` and its four declared numbers die with it: capacity becomes what a carrier has
free at an origin.

**This item resumes at step 9** against a world with several producing places, voyages that are
somewhere, and legs of different lengths. Steps 11–13 (the `commodity.future` kind, its participants
and the curve) need nothing from it and are unchanged. **13c.2 — where a firm builds — comes after
this item**, because it moves production between places and the steps below measure prices, so each
measurement stays about one thing (Law 14).

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

- [x] **`environment` is standing state and depends on NOTHING** — the physical world is not an economic outcome and does not wait for one, so the module requires no other, reads no price, no holding and no party, and runs at the top of the period before anything decides or produces. Three facts, each read by several systems: growing conditions, the wind standing over a region, and warmth. **Each is stated as how this period stands to what the region NORMALLY has, not in millimetres** (PLAN §5.2: the step said "with a unit", and an absolute scale would be a second copy of a number every consumer already declares — a recipe has a yield, a route a capacity, a policy a sum insured; nobody in this world measures rain). What is declared is a MEMORY and a WIDTH per fact per region, both TECHNOLOGY, both drawn from the module's own labelled stream (Seed B1.a); the period's departure is an AR(1) in log space, so the condition is positive BY ARITHMETIC and not by a bound (Law 6) — half the normal rain and twice it are the same size of departure in opposite directions, which is true of weather and false of any additive index. It crosses as the public event `environment.state`, read through `events.ts` and never by an import. Tests: one public fact per region every period and no more, positive and finite everywhere, this period within its own arithmetic reach of where the last one left it, two technologies declared per fact per region, and the same weather from the same seed. **And a fourth silent FORBID**: no module outside `environment` may name a hazard, catastrophe or disaster — the refusal 13h makes of `catastrophe.probability` as a primitive, made checkable from the other side
- [x] **One event, several consequences, and the first of them is real: Goods B4's yield.** A physical line's terms carry `exposedTo` — the facts its yield stands in, by the NAME the environment publishes them under, which is how a module says a thing it does not own (4.9b). The read is the kernel's (`registry/environment.ts`), because four modules will need it and four parsers would be four ways to misread one event; the environment module re-exports it so there is one spelling. A crop names `growing`; a mill and an oven name nothing, and an empty list is a real answer rather than a high yield (Law 16). **B4 IS ONE-DIRECTIONAL** — "not everything started is finished" — so the season moves the SURVIVAL RATE and never multiplies the batch: `yield ^ (1 / season)` is a fraction between nothing and all of it raised to a positive power, so it stays inside its own range by arithmetic and there is no clamp anywhere (Law 6). Grain's declared 0.92 stops standing in for weather and says so. The other three consequences read the SAME event and arrive with their own modules in this item's later steps — a producer's destroyed units at step 4, a route's capacity at step 6 — and the policies at 13h. Tests: the shortfall is tonnes that were never made, started equals finished plus scrapped exactly, the survival rate is inside its range, one season reaches every line of a good in a region in a period, and a line made indoors stands at one
- [x] **Storage is plant, and room binds like plant.** Location is already in a physical line's identity (`good.<subUnit>.<region>`) and inventory already carries in the register with its own spoilage, so what the step actually needed was the half that was missing: HOLDING WAS FREE. Covered space is now a second kind of capital (`STORAGE`) — a stock of productive assets with a life, made from a good, owned by named parties (A4) — and a good declares how much of it one unit takes (`GoodTerms.storagePerUnit`), null being a real answer for a line nobody holds in bulk. A venue per region clears it before anybody decides what to make: letters offer what they own and are not using at what that room costs THEM, takers bid for what they are short of at what the thing that would otherwise have nowhere to go is worth to THEM, and a taker rents from named letters down a walk of the two books, so the sum paid is the sum received exactly (Law 5). **The consequence is not destruction but a BINDING CONSTRAINT** (PLAN §5.2: the step said the excess is offered into the spot market, and building that first would have destroyed the world's grain before the escapes existed): room is one more plant need in the arithmetic that already takes the scarcest kind, so a farm with a full barn does not start what it would have nowhere to put, and one that wants to grow rents a barn or builds one — and the capital programme already builds the kind that binds. Nothing is capped, refused or destroyed (Law 6). **Law 8 twice over**: the declared ratio is in NAMED units on both sides and the register counts in pieces, so `spaceFor` converts where the ratio is read and `spacePerPiece` is the unrounded RATIO for the arithmetic that divides — rounding a thousandth of a silo to a whole one makes it nothing and makes room bind nothing; and a fee below one piece of money is not a fee, so that room is not let rather than let for free. The seed gives every holder room for exactly what it opens holding, computed by the SAME function the module checks with, and no headroom — whether a line that wants to grow rents or builds is a market question from the first period. The session says what it did including when it did not clear and why (Clearing C4.b)
- [x] **Disruption is a real loss of plant where it stood, and there is no threshold in it.** Producers with dispersed costs (B1.a) and plant-fixed short-run capacity (B2) were already there — `labourScale` is the dispersion and `capacityFrom` is the capacity — so what was missing was B3. A storm is the environment's own published wind, and this is the second of the several consequences of that one event: units of plant destroyed on the named party that owned them, at the site they stood. **What survives is `exp(-(wind / standsWind) ^ hardness)`** — positive at every wind and never reaching one, so an ordinary week takes a little and a storm takes most, continuously, with nothing happening AT any level and nothing clamped (Law 6). Two technologies per kind say it: what the structure is built for and how sharply what it was not built for fails; a silo stands more than a machine shed, which is part of why it lasts twenty years. Nothing multiplies a price: what the loss costs its owner is what settlement charged its equity. **And the capital-kinds table was in TWO places** and the copies disagreed the moment a kind gained a field — the kernel's is the one, because `firms` needs the list to decide anything and a module never imports another; `capital-programme/data.ts` is now its re-export and forty lines shorter. Tests: the loss is units on a named holder and never a write-down, the wind is the same fact the environment published for that region in that period, survival is strictly inside its range everywhere, and it falls monotonically with the wind and nowhere steps. **B4 — a producer holding on its own outlook — lands in the NEXT step**, because what it holds against is the carry and the carry is a price that did not exist until the room had a market
- [x] **What the wait costs reaches the decision to hold, and a party stands on the other side of it.** Consumers by recipe (C1, C2), the spot session per (good, region) (D1) and the units family's D5 contribution (F1) were already built at item 4 — what 13c adds is the two halves that needed a real carry. **B4**: a producer parts with stock only above what HOLDING it is worth, and that is now what it expects to get, less what will not survive the wait, **less what the wait costs** — the rate the room cleared at, read off the session's own print through a kernel read, so a world where room is scarce is a world where more of every stock comes to market. The plan publishes the carry it used, so the decision is legible. **C3**: a fund that holds the thing itself, the third mandate beside the money fund's and the tracker's, and a fund has exactly one of them. What makes it a commodity fund is structural — every kind it may hold is one nobody issued — and never a flag. It is the party the storage market was missing: every producer opens with room for exactly what it holds, so an investor wanting to hold what it did not make is short of room by construction. Its own outlook against the carry is what it will pay, so the number is nobody's to write down. **A NaN was found and fixed where it was**: a fund with a mandate it opens holding none of reaches its first session with no book and no shares, and the first party to ask for one asked for `0/0` of them. A subscription is at the NAV and there has to be one — nobody buys a claim on nothing — so it is refused at the site, and WHY a share can be worth nothing while shares are outstanding stays `13b-9`'s, at 13h. **`E2` (households for energy) is MISSING, not done**: the item's own dependency note says a second FINAL good waits for 13d, because `ConsumptionDecl.share` is a share of spending and a second one would make demand unit-elastic — the strongest substitution assumption in the model, and exactly what `phoenix/no-value-recipe` refuses on the production side
- [x] **Carriers are FIRMS, and capacity is hulls.** A carrier is a named party that owns plant, banks somewhere and can fail — which is what a firm is — so a party kind of its own would be a second kind behaving identically and every rule written for one would have to be written again (Law 15, Law 4; Small-Business Pools A6.b: the boundary between sectors is a SIZE). What makes it a carrier is its business: hulls, and a leg to sail them on. Vessels are the third kind of capital, with a life of twenty-five years and a build lag of half of one — which is why freight capacity answers a shortage slowly and a blocked leg stays dear longer than the block does. A fleet is drawn from a heavy-tailed width (Seed B4: shipping is a few large owners and a long tail), so two carriers on one leg do not cost the same and one of them is marginal. **The blocked route is the same one event**: what can sail is `exp(-(wind / sailsIn) ^ hardness)`, the route's own technology because a strait closes whatever the hull stands, and a storm takes the hulls that were in it through `capital.weathered`. Four technologies per leg, all declared, and **there is no freight rate anywhere** — the test greps the register for one
- [x] **The session, before the goods markets it feeds.** One book per leg, cleared by the same solver every market uses: carriers post the room their hulls have at what the voyage costs THEM — the wear on the hull a unit uses, times their own crews' scale — and shippers post what they hold at what moving it is worth to them, which is the gap between the two places' prints and nothing else (C1.a: never a series, C3: read off two prints both already made). Capacity rations quantity and nothing else does: a shipper is paired with named carriers down a walk of the two books, so every unit moved has somebody on each side of it, and what is not carried stays where it is — which is what being unable to ship means (D6). The session publishes what it did per leg including when it did not clear and why (Clearing C4.b). **THE LEGS ARE REAL AND IDLE**: this world makes its goods in the one region that has firms in it and the three abroad are a central bank, a treasury and a bond line until 13i builds their economies, so every leg says `noDemand` and says it out loud. The mechanism is built and tested; what it needs is somewhere to carry to
- [x] **A cargo is somewhere while it is neither here nor there.** What is shipped leaves the origin and becomes units at a place of its own — one instrument per good per leg — on the SHIPPER'S own book, which is A3.a's working capital: a shipper that has paid for a cargo and not yet got it is short of both. After the transit the kernel's own corporate-action phase reseats it at the destination, at what it cost INCLUDING the voyage (D2), so the delivered price is the origin price plus the freight by construction and nothing adds them up a second time (Law 19). Nothing teleports, nothing is in two places, and every unit in transit has an owner the whole time (E3)
- [ ] Location basis as an outcome of shipping with capacity, **between two regions of one country over a drawn leg** (13c.1): the gap between two prints is what it cost somebody to actually move the thing, and nothing computes it. Scenario test: a blocked pass widens the gap and it stays wide while the pass is shut (D3, D3.a, D5 direction)
- [ ] Shipper substitution: hold, **source locally** — which needs a second place that makes the thing, and 13c.1 is what gives it one — or not trade at all, from delivered prices in its own view; test (C2)
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
