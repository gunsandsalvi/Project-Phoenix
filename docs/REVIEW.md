# Review — the four questions, and the answers

*A single working file. The owner asked four questions; each is a section below. Every section is
filled in from the SOURCE (read, quoted, cited by file) and from MEASUREMENT (runs of this world,
numbers reported with the seed and the rung they came from), then against the best practice outside
this project, and ends in proposals that name the items they become and the recorded findings they
close. A section that has not been done yet says so.*

**Status.** §0 done · §1 done · §2 done · §3 done · §4 done. Written at `ab18535`; every measurement is from `rigWorld('review')` at 3 banks and 12 firms unless it names another rung.

---

## §0. The one measurement everything else hangs off

*Why the four questions are one question.*

Everything below is measured on the same run unless it says otherwise: `rigWorld('review')`, 3 banks
and 12 firms, 26 periods, at `ab18535`. Anybody can repeat it.

### 0.1 What this world does in twenty-six weeks

| | |
|---|---|
| instructions settled | 9,911 |
| instructions **failed** | **1,903** (16% of all of them) |
| market sessions run | 8,819 |
| market sessions **cleared** | **72** (0.8%) |
| sessions that found **no demand at all** | 7,903 |
| books that have **ever** traded | **10 of 395** |
| capabilities declared / **never once used** | 648 / **428** |
| modules that have never done **anything** | **13** |
| living parties | 135 at the opening, 111 after 26 weeks |

The thirteen modules that have never produced a single outcome: bond futures, CDS, commodity
futures, corporate control, corporate bonds, FX derivatives, index futures, interest-rate swaps,
merchants, options, short-term debt, spot FX, supply contracts. Two thirds of everything this world
is able to do has never happened once.

### 0.2 What it does instead

The events it does produce, over the same run, largest first:

```
16753 expectations.surprise     2125 credit.default        1835 labour.unsold
 9911 instruction.settled       2118 credit.declined       1716 supply.print
 8819 print                     1903 instruction.failed    1648 goods.perished
```

And the reasons, read off the same run:

- **1,902 of the 1,903 failures are `overdraftRefused`** — a party that did not have the money.
  **1,617 of them are MATURITIES**: debt falling due that nobody could pay. A further 145 are coupons.
- **1,141 credit refusals say `it cannot cost its own funding`** — a bank that cannot price its own
  funding does not quote. 399 more say `nobody lends to a party of this kind`.
- **Labour: 700,000 hours offered, 0 wanted, `noDemand`** — the same, every period, in every venue.
- **1,648 lots of goods perished** in warehouses while the consumer basket read empty (21.84).

### 0.3 The diagnosis these four questions share

> **The seed gives this world obligations and stocks, but no flows and no history. Obligations run
> by themselves, because they are on a calendar. Flows do not, because every mechanism that produces
> one needs a history to act on — and refuses to act without one. So the world spends twenty-six
> weeks failing to service debt it was born owing, while nothing trades, nothing is produced,
> nobody is hired and goods rot.**

Each link in that sentence is measured above, and each of the four questions is one link:

- **§1 The opening** is where the asymmetry is created: a coupon is a date, and a customer is a
  history. The seed writes the first and not the second.
- **§3 What it does badly** is why the world cannot climb out: the mechanisms **fail closed**. No
  outlook → no order. No funding cost → no quote. No sales history → no batch. Each refusal is
  individually correct and collectively fatal, and there is only one market microstructure for
  everything.
- **§4 What it does not do** is what would have caught it and what would break the loop: nothing
  measures liveness, nobody makes a market, nothing queues a payment, nothing reads the system as a
  network.
- **§2 Speed** is the same fact from the other side: **99.2% of the engine's market work is an
  auction nobody attends.** The engine has been made 2.4× faster this week at doing nothing.

The order that follows from it: make the world act (§1, §3), measure that it acts (§4), and only
then make it fast (§2).

## §1. The opening

*This world should not start from scratch; it should open as one already running.*

### 1.1 What the opening is, in the source

`seeds/foundation.ts` (3,168 lines) and `seeds/funding.ts` run once before period 0 through
`SeedContext`. Three properties of that door decide everything downstream.

**It writes balances; it settles nothing.** `ctx.settle` is called **zero times** in the seed.
Holdings are written straight into the register through `moneyDelta` and `adjustIssued` — the
exemption that lets the seed be the only writer besides settlement.

**Every line is issued at the epoch.** `issueDate: ctx.calendar.epoch` for every seeded sovereign
line. Maturities are spread across a grid; issue dates are not. Nothing has ever accrued, nothing is
seasoned, every bond is on-the-run.

**Stocks are walked from ratios.** The seed states the scale of the world and walks it up each
recipe: *"a world of thirty million people opens with thirty million people's worth of stock in it"*.
Recorded at 12c.3: one baker opened holding **314 million loaves against 27 million a period** of
expected demand — eleven periods of the whole world's consumption in one warehouse.

### 1.2 What that produces, measured

| | period 0 | after 12 | after 26 |
|---|---|---|---|
| living parties | 135 | 101 | 111 |
| parties with an outlook of anything | **0** | 101 | — |
| instructions in the ledger | **0** | 4,167 | 11,814 |
| markets carrying a price | 132 | 135 | — |

At period 0 the ledger is empty and 132 markets already carry a price: every one of them asserted,
none of them traded. And **not one party has an outlook**, because `observations()` reads exactly one
source — the instructions a party was a side of — and there are none.

### 1.3 The asymmetry that makes this fatal

A seeded world is handed two kinds of thing, and they behave completely differently once the clock
starts.

**An obligation is a DATE.** A coupon, a maturity, a rent, a wage bill: they arrive whether or not
anybody is ready. The seed writes plenty of them — and §0 measures the result: **1,617 failed
maturities and 145 failed coupons in 26 weeks, 1,902 failures for want of money.**

**A relationship is a HISTORY.** A customer, a supplier, an employer, a lender: each exists in this
model only as an accumulated record — an outlook formed from observations, a cost of funds read from
what a bank has paid, a sales expectation read from fills. **The seed writes none of them.**

So the opening is not merely "a bit off". It is **systematically biased toward liabilities**: it
hands over everything that takes money out on a schedule, and nothing that brings money in. Seed D1
asks for stocks *"consistent with the flows that will run"* and D1.a warns that otherwise *"period
one is a shock the model never recovers from"*. That is exactly what 135 → 101 parties in twelve
weeks is.

### 1.4 What everyone else does

**Stock-flow-consistent economics — land in a body, never solve for a point.** The admissible
openings of an SFC model form a **polytope**: *"the set of solutions is a polytope, which volume
depends on the constraints applied and reveals the potential fragility of the economic circuit, with
no need to specify the dynamics"*, and *"not all conceivable sets of stocks are consistent with the
model's accounting"* ([Volume of the steady-state space](https://arxiv.org/pdf/1601.00822)). Useful
twice: the volume is a diagnostic computable without running anything, and the geometry says landing
inside a set is a different act from solving for a point (which Seed D4 forbids).

How the toolkits actually get an opening is blunter: setting initial conditions by hand *"is
particularly difficult if there are multiple types of financial assets within the model"*, so
*"the solver can start a new simulation at an earlier point in time and then simulate forward until
reaching the desired initialization point, with starting time typically set to 200 periods prior"*
([Bond Economics, `sfc_models`](http://www.bondeconomics.com/2017/09/calculating-initial-steady-state-in.html)).
A serious SFC engine with several financial assets **gave up on stating an opening and ran to one.**

**Macro ABMs — the opening chooses a basin, not a number.** Mark-0's phase diagram shows *"the
generic existence of a phase transition between a 'good economy' where unemployment is low, and a
'bad economy' where unemployment is high"*, with four phases and *"a transient that can be
surprisingly long"* ([Naumann-Woleske et al.](https://arxiv.org/pdf/2111.08654),
[Gualdi et al.](https://arxiv.org/pdf/1307.5319)). Phoenix is in the bad phase. No adjustment to a
stock ratio moves a world between basins; only the path does.

**Discrete-event simulation — the length of a past is measured, not chosen.** Sixty years of
literature on the initial transient: MSER-5 *"gives the truncation point as the point where the
minimum standard error in the data occurs"* and is *"the most effective and robust method"* tested;
Welch's graphical method is criticised precisely because *"the user decides the warm-up length by
observing a graph… thus there is subjectivity"*
([AutoSimOA review](https://warwick.ac.uk/fac/soc/wbs/projects/autosimoa/current_work/website_warmup_methods_total_doc.pdf)).

**Earth-system models — the world is an artifact, not a function.** Ocean spin-up *"is usually in the
order of a few thousand years"*, *"climate drift can be caused by a not-yet-equilibrated ocean
initial state"* and post-processing *"has been found to be ineffective in removing it"*
([UKESM1 spin-up](https://agupubs.onlinelibrary.wiley.com/doi/full/10.1029/2019MS001933)). The
practice that follows: spin up ONCE, write a **restart file**, open every experiment from it.

**Dwarf Fortress — generate, then reject, and log why.** Worldgen *"generates a history for that
world, tracking civilizations, sites, populations, and other events"*, and accepts nothing blindly:
*"if the world does not meet the requirements of any one rejection parameter the world is rejected
and re-randomised"*, with `LOG_MAP_REJECTS` writing *"a file called map_rejection_log.txt where every
time a world is rejected the reason will be logged"*
([world generation](https://dwarffortresswiki.org/index.php/DF2014:World_generation),
[world rejection](https://dwarffortresswiki.org/index.php/v0.31:World_rejection)).

**Microsimulation — the road to refuse.** Population synthesis by iterative proportional fitting
*"alters the joint distribution to fit the marginal distribution of the attributes from the
aggregated data"* ([IPF review](https://www.sciencedirect.com/science/article/pii/S2352146516306925)).
That is fitting to targets, which Seed B5 and C5 forbid outright. Take the vocabulary (a population
is drawn), refuse the method.

### 1.5 The proposal — THE CHRONICLE

**Four stages, and the third is the one nobody else in this project's history has had.**

```
  draw       the physical world and the primitives      (declared; small; auditable)
  chronicle  a past, lived through ordinary settlement  (produces the ledger)
  accept     against a census, or REJECT with a reason  (logged; re-draw; never adjust)
  snapshot   the accepted world, written as an artifact (runs open from here)
```

**`period(0)` becomes the END of the past, not its start.** The calendar epoch moves back by the
chronicle's length, so a bond issued in chronicle period 40 carries that issue date for ever and its
accrual at the opening is a read rather than a zero.

**What the seed may still state:** land and places; people, as cells with weights; technologies
(recipes, lead times, plant lives); the primitives (preferences, policies); and the constitutional
parties — a state, a central bank, a first bank — because a banking system cannot bootstrap itself
from nothing inside its own rules. **Everything else arrives as an instruction with two sides and a
date.**

**Two sources, in order.**

1. **A scripted past.** Dated `InstructionDraft`s settled through the ordinary path, under a grammar
   that is the whole of what keeps them legal: a chronicle instruction may **create a stock** — issue,
   buy, hire, lend, deliver — and may **never write a price, a rate, a mark or a decision**. It is an
   endowment expressed as a flow. This alone fixes the asymmetry in §1.3: a firm that has sold for
   forty weeks has a sales history, a bank that has funded itself has a cost of funds, and every
   party that was ever paid has an outlook.
2. **A lived past.** The engine's own period loop, module by module, replacing scripted events as the
   mechanisms become able to produce them. **The migration is a measurement of whether a mechanism
   works** — which is the thing this project keeps discovering three items too late.

**Acceptance is a census of PROPERTIES, never values.** Nine, each a statement about an existing
world and none about a number: every market has traded; every living party has an outlook; every
firm has produced, sold and been paid in different periods; every bank has lent and been repaid;
every instrument kind that can exist is held by somebody who chose to hold it; the maturity profile
spans more than one period and issue dates are dispersed; ages are dispersed; the audit is green and
every balance sheet closes; nothing in the register is younger than the world. **A world failing any
of them is rejected, re-drawn from the next seed, and the reason is logged** — and that log is the
most valuable diagnostic this project could own, because a criterion that fails for every seed is a
missing mechanism with a name.

**Why rejection is legal where calibration is not.** Rejection **discards a world; it never adjusts
one**. No number inside an accepted world was touched by the criterion, so nothing is fitted (B5)
and no outcome is seeded (E1). IPF, by contrast, changes the world until it matches — which is why
§1.4 takes it only as a warning.

**The length is measured.** Run the chronicle long, record a handful of named series, take the
truncation point from MSER-5. The resulting length is a RESOLUTION under Law 2, and it is tested the
way a resolution must be: double it, and the opening's properties must not move.

**Cost.** 554 ms/period at the first rung, so a 200-period chronicle is about two minutes at the
scale model — and the snapshot means it is paid once per world rather than once per run. At the
third rung it is hours, which is what the snapshot and the structural performance items are for.

**Two consequences worth having.** The chronicle is a journal, and the observer already renders
journals — so *"peek inside an existing world"* becomes literal: who founded which firm, who lent to
whom, which bank failed in chronicle period 63. And a test rig stops being built by hand: a rig is a
small accepted world, and a test asks for *a mill that has traded for a year* instead of
constructing one.

### 1.6 The items

Before 22a, which they absorb; after item 21's stops, because a chronicle cannot run through a world
that throws.

- **22b.1** `world/chronicle.ts` — dated replay through ordinary settlement; the epoch shift; the
  grammar guard (no price, no rate, no mark); determinism from one seed value.
- **22b.2** `check:opening` — the nine-property census, red until it passes, with the rejection log.
- **22b.3** Money and the sovereign through the chronicle. Deletes `endowMoney`.
- **22b.4** Firms, plant and inventory through the chronicle. Deletes `endowUnits` and
  `SEED_STOCK_BASIS` — and this is where 12c.3 dies, because no firm can BUY eleven periods of world
  demand from anybody.
- **22b.5** Households, employment and savings through the chronicle — the income history that
  outlooks and votes are made of.
- **22b.6** Delete the opening prints (22a.1): by then every market has traded.
- **22b.7** The snapshot as an artifact; the rig's scale models become small accepted worlds.
- **22b.8** MSER-5 on the named series; the length becomes a tested resolution.
- **22b.9** The first module migrated from scripted to lived.

**Exit.** `grep -rn "endowMoney\|endowUnits\|openingPrice\|prices\.write" src/` is empty;
`check:opening` is green; the census of period 0 is indistinguishable in KIND from the census of
period 100.

**What it predicts will break.** The first chronicle will be rejected on *every firm has produced,
sold and been paid* — that is 12c.3, and it is the point. Money will be hard to create legally,
because every deposit must come from somebody's lending. And every finding recorded against the old
opening must be re-read rather than carried over.

## §2. Speed

*What a run costs, where the cost actually is, and what is worth doing in what order.*

### 2.1 What it costs, at three scales

The ladder's own three rungs, 26 periods each, at `ab18535`:

| rung (banks, firms) | parties | markets | 26 periods | ms/period |
|---|---|---|---|---|
| (3, 12) | 127 | 350 | 8.6 s | **331** |
| (6, 60) | 229 | 661 | **stops at period 20** | ~750 |
| (12, 200) | 866 | 765 | 236 s | **9,078** |

Two facts, and the second is worse than the first.

**Cost grows as about the 1.7th power of the population.** Parties ×6.8 from the first rung to the
third; cost per period ×27. A world ten times bigger costs fifty times more. The exit criterion 0g
sets — *"(12, 200) one country: a 52-period year under 60 s"* — is 8 minutes away at that rung, and
the gap is structural, not constant-factor.

**The middle rung does not run.** `(6, 60)` stops at period 20 with
`[Money C1.a] instruction 23596: a payment from an account to itself is not a payment`, from the
treasury's receipts — the same stop recorded as 21.87. So **the project's own performance instrument
cannot produce its own three-rung comparison**, and the one measurement that would show a scaling
exponent has been unavailable the whole time.

### 2.2 Where the time is — and it is not where the first rung said

A CPU profile of eight periods at the **third** rung:

```
21.9%  yearFraction        calendar/daycount.ts
14.9%  (anon)              prices/curve.ts
10.2%  dayNumber           calendar/civil.ts
 6.3%  garbage collector
 4.3%  invertDecreasing    core/num.ts
 2.2%  readCurve           prices/curve.ts
```

**More than half of a run at the scale that matters is yield-curve arithmetic** — a numerical
inversion (`invertDecreasing`) calling a discount function that calls a day-count fraction that
walks a civil date into a day number, over and over, with nothing remembered between calls.

At the **first** rung, the same week, the profile was `exposureTo` 11%, `instruments.get` 10%,
`register.snapshot` 9%, `register.quantity` 8% — a completely different hot set, which is what the
four steps of 0g removed (1,354 → 554 ms/period, 2.4×). **Those steps were real and they were aimed
at the wrong world.** The lesson is general and belongs in the record: *profile at the scale whose
cost you care about, because the hot set is a function of scale.*

The plan already predicted this and never built it. 0g.7, still open, reads:
*"`curveAt` memoised per (family, period, prices.version); `invertDecreasing` seeded by the last
yield; `index()` keyed on the basket's own prints."*

### 2.3 What the engine spends its effort on

From §0: **8,819 market sessions, 72 cleared.** The engine opens 350–765 books every period, asks
every eligible participant in each, runs a solver over the answers, and 99.2% of the time nothing
happens. No constant-factor work on the solver can touch this; the saving is **not holding the
auction**, which is a modelling change (§3) that happens to be the largest performance item on the
list.

### 2.4 How this class of simulator is made fast elsewhere

**Entities sleep, and something wakes them.** In Factorio *"entities not in the 'Active' section
don't get touched during normal tick updates"*, and the wake is caused rather than polled: *"an
inserter that is supposed to fill an assembler will be deactivated if the assembler's input is
filled. Once the assembler produces an item, the assembler activates the inserter"*
([diagnosing performance](https://wiki.factorio.com/Tutorial:Diagnosing_performance_issues),
[active entities](https://forums.factorio.com/viewtopic.php?t=38047)). Their other lessons are about
memory rather than logic — *"all active entities are read at every tick… this is too much data for
caches"*, answered with prefetching and per-chunk allocators
([FFF-204](https://factorio.com/blog/post/fff-204), [FFF-215](https://factorio.com/blog/post/fff-215))
— and multithreading is gated by the same constraint Phoenix has: *"the game needs to remain fully
deterministic"* ([FFF-421](https://www.factorio.com/blog/post/fff-421)).

**Process what happens, not what might.** *"The default execution scheme of agent-based modeling
relies on fixed-increment time advances, whereas discrete event simulation… is assessed at specific
time points triggered by an event"*, and the discrete-event implementation *"performs better on
CPU"* ([JASSS](https://www.jasss.org/27/1/10.html)). Phoenix's week is a real unit and should stay —
the audit closes on it — but *within* a period the same idea applies exactly.

**Layout is the algorithm.** Structure-of-arrays with archetype storage is reported at *"10-100x
performance improvements over object-oriented architectures"*
([ECS in C++](https://cppcat.com/entity-component-system-implementation/)). That is 0g.11, and it is
the ceiling for an engine that allocates an object per addition — the garbage collector is 6.3% of
the third rung and was 8.7% of the first.

**Spin up once, save the world.** Ocean models write **restart files** rather than re-deriving a
state every experiment ([UKESM1](https://agupubs.onlinelibrary.wiley.com/doi/full/10.1029/2019MS001933)).
That is §1's snapshot, and it is also a performance item: the chronicle is paid once per world.

**Not applicable, and worth saying so.** FLAME GPU's *"hundreds of millions of agents"* and
*"1000x speedup"* are for local, homogeneous, embarrassingly parallel agent rules
([FLAME GPU 2](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3207)). A Phoenix period is a
sequence of global serialisations — one solver, one settlement, one audit. GPUs are not the answer
here, and neither is threading until the determinism story is settled.

### 2.5 Proposals

**S1 — Fix the ladder before optimising anything else.** The performance programme's own exit is
defined at a rung that stops. 21.87 (the treasury self-payment) blocks it; it is one guard in
`runReceipts` away from being measurable. *Cost: hours. Without it every speed number in this
project is a first-rung number, and §2.2 shows what first-rung numbers are worth.*

**S2 — Build 0g.7, which the profile has now named twice.** Memoise `curveAt` per
(family, period, prices.version) — `view.memo` already exists from this week's 0g.5a and takes
exactly this key; seed `invertDecreasing` from the last yield rather than from a cold bracket; and
memoise `yearFraction` and `dayNumber`, which are pure functions of (date, date, convention) and are
**55% of the third rung between them**. *Expected: a factor of about two at the scale that matters,
for a day's work, with no mechanism touched.*

**S3 — A book opens when somebody has a reason to be in it.** Today every declared market is asked
every period. Let a participant register an interest in a venue when it has one, and open the
session only where interests exist; the books that never open become a measured fact (§4's liveness
read) instead of invisible work. *Expected: most of the 99.2%. This is a modelling change and it
belongs to §3 — it is listed here because it is also the largest speed item there is.*

**S4 — Wake, don't poll, inside a period.** Factorio's rule, applied to phases: a phase should visit
the parties something happened to. The kernel already has the raw material — the register's write
count from 0g.5a, the period's ledger slice — so a "dirty party" set is a read away.
*Expected: large at scale, where most parties are idle in most periods; it is 0g.2's period index
with a purpose.*

**S5 — Then the structural items, in this order:** 0g.3 (`Measure` as a number brand rather than an
object — the allocator is 6-9% everywhere), 0g.11 (columnar state), 0g.14 (tiered journal — 105,186
events in eight periods at the third rung). *These are mechanical passes over the whole engine and
should be taken after S1–S4, not before.*

**S6 — Gate it.** The ladder exists and is wired to nothing: the first rung went from 5.1 s a year
at 0g.1 to 70 s a year this week with no check noticing. `check:forbids` already demonstrates the
pattern — a baseline that may only fall. *Cost: an hour.*

**What not to do:** GPUs, threads (until determinism is settled and measured), and any further
first-rung micro-optimisation.

## §3. What it does, done badly

*The mechanisms that exist and are weaker than the best available.*

Two of these are the machinery between the decisions and four are the decisions themselves. The
first two matter more.

### 3.1 One market microstructure, for everything

**Today.** `clearing/solver.ts` is *"one solver for every market"*: participants post limit
schedules, the solver takes the uniform price that executes the most volume. `VenueDecl`
(`clearing/venue.ts`) declares an id, a name, the clearing module, a unit, a currency and a key —
**there is no protocol field**. The only variation anywhere is a tie-break (`sellersCompete` |
`marginalBid`). A household buying bread and a treasury auctioning ten-year paper run identical code.

**Why it is wrong on this project's own terms.** Law 1 is *reflect the real mechanism: real named
counterparties, intermediaries, lags, fees, refusals*. A Walrasian auctioneer for bread is the one
intermediary that has never existed. Nobody has ever bought a loaf at a uniform price struck against
every other buyer in the region that week.

**What it costs.** §0: 8,819 sessions, 72 cleared, 7,903 finding no demand at all. A call auction
with no resting orders clears only when two parties independently want opposite sides of the same
book in the same week — which, in a world of 350 books and 127 parties, is almost never.

**What the best practice is.** The dominant protocol in macroeconomic ABMs is not an auction: demand
agents *"observe the prices or interest rates charged by a random subset of suppliers"* and *"switch
from the old partner to the best potential partner selected in this random subset with a probability…
as a non-linear function of the percentage difference in their prices"*, applied *"in four markets:
goods, labour, credit and deposit — according to a fully decentralized matching mechanism"*
([Caiani et al. 2016](https://www.sciencedirect.com/science/article/abs/pii/S0165188915301020),
[Riccetti, Russo & Gallegati](https://link.springer.com/article/10.1007/s11403-014-0130-8)).
Exchanges went the other way — *"progressively adopted continuous double auctions"* — keeping the
call for openings and closings ([Walras to CDA](https://arxiv.org/pdf/1506.03758)).

**Proposal M1 — `VenueDecl.protocol`, as DATA behind a dispatch table (Law 15).** Three rows:

- `call` — today's solver, unchanged: sovereign auctions, fixings, anything a real market strikes at
  one price at one moment.
- `posted` — the seller posts an ask it stands behind; a buyer sees a random subset of sellers (the
  size of the subset is a TECHNOLOGY: how much of a market a buyer can see) and buys the best it saw,
  at that seller's price. **Still cleared from real supply meeting real demand (Law 3)**: a trade
  happens because a buyer accepted a price a seller was standing behind, which is what a price in a
  shop is.
- `book` — resting limit orders with continuous matching, for exchanges (needs M2).

The engine keeps one solver *for auctions* and stops pretending a grocery is one.

### 3.2 Nothing rests

**Today.** `World.runOne` builds a book by asking every eligible participant for orders, clears, and
throws the orders away. Nothing survives the session.

**Why it matters more than it looks.** This is why `noDemand` is 90% of outcomes rather than
`noOverlap`: the two sides are not failing to agree on a price — **they are failing to be in the room
in the same week.** In every real market an offer stands until it is taken, withdrawn or expires: a
limit order on a book, a price on a shelf, a bank's standing quote, a firm's price list.

**Proposal M2 — a standing-order register**, beside agreements and processes: an order is
`(party, venue, side, level, quantity, from, until, why)`, entered by a participant, cancelled by its
owner, expired by the calendar, consumed by a match. Every session opens with the standing book. This
is not a cache: it is the fact that **an offer is a commitment with a date**, which is this project's
ontology everywhere else.

### 3.3 Every mechanism fails closed

**Today.** **39 decision sites read an outlook; at least 21 of them return nothing when it is
missing.** The pattern, from `mechanisms/supply/index.ts:164`:

```ts
const seen = view.outlook(about({ on: makes ? 'sold' : 'bought', instrument: good }));
if (!seen.some || seen.value.expected <= 0) return [];
```

The same shape decides whether a bank quotes (1,141 refusals in §0 say *"it cannot cost its own
funding"*), whether a firm starts a batch, whether a household posts a demand curve. Each refusal is
individually correct — a party may not invent a number it does not have — and collectively they are
why 13 modules have never run.

**The mechanism to fix it already exists and is used for something else.** `expectations/index.ts`
keeps **two tracks** per party and variable: its own adaptive outlook, and an `anchored` one — *"the
last PUBLIC level"* — and it switches to whichever has surprised the party less. But a party with no
private observation has **no entry at all**, so the public track it is entitled to read is never
consulted.

**Proposal M3 — a party with no private history follows the public one.** Not a default and not an
invention: a READ of the public record, which is Law 19 rather than `?? 0`. A new entrant in a real
market looks at the posted price, the published index, the policy rate — and that is precisely what
`anchored` already is. Concretely: an outlook exists for any variable with a public level, from the
moment that level exists; its expectation is the public level and its confidence is the dispersion of
the public record; the party's own observations then pull it away, exactly as they do now.

*This is the single smallest change in this review with the largest reach.* It turns on supply
contracts, dealer quotes, firm batches and household schedules simultaneously, and it is how the
chronicle (§1) and the mechanisms meet: the chronicle supplies the history, and this supplies the
behaviour for anything the chronicle has not touched.

### 3.4 A firm with a full warehouse stops existing instead of cutting its price

**Today** (`mechanisms/firms/decide.ts`): `wanted = (expectsToSell − stock) / yield`. A
stock-adjustment rule whose **desired buffer is exactly zero**. And the ask is *"what it expects a
unit to fetch, less what perishes before it could"* — an expectation formed from its own past fills,
with **no feedback at all from unsold stock into the price**. 12c.3 records the consequence: every
plan after period 1 is `batch 0, bound demand`, for ever.

**What the best practice is.** In the K+S and AB-SFC families a firm carries a desired inventory as a
share of expected demand, and adjusts its PRICE on two signals: did it sell out, and are inventories
above or below that desire
([Caiani et al.](https://www.sciencedirect.com/science/article/abs/pii/S0165188915301020)). The
adjustment is bounded in size, never in direction. It is what makes a glut clear.

**Proposal M4 — two changes, both reasons rather than levels.** A **desired cover** drawn per firm as
a PREFERENCE (how many periods of expected sales it wants on the shelf), dispersed like
`payoutPatience`, nothing stated globally. And **the ask answers the shelf**: what a unit is worth to
the firm is what it expects to fetch *if it can sell it*, and a firm holding more cover than it wants
is one whose alternative to selling today is selling later at a carrying cost it can already read
(storage, perishing, the money tied up). Subtracting that is not a markdown rule — it is the value of
holding, which the docstring claims to compute and currently computes as if the shelf were empty.

### 3.5 Nobody may post a schedule of zero width

**Today.** The expectation machinery is genuinely strong — adaptive at a per-party memory with a
switch to the public anchor, which is heuristic switching built honestly. But *"a party that has
never seen this variable has no outlook… its first observation IS its outlook, and it is surprised by
nothing"*, and confidence is the mean absolute surprise. So a party's first schedule has **width
zero** (`households/consume.ts`: `width: outlook.some ? outlook.value.confidence : 0`), and a point
demand meets a point supply.

**Why that is fatal in a thin market.** Gode and Sunder's result is that *"allocative efficiency of a
double auction derives largely from its structure"* and that a budget constraint over **dispersed**
reservation prices reaches *"close to 100 percent"* efficiency with zero-intelligence traders
([JPE 1993](https://ideas.repec.org/a/ucp/jpolec/v101y1993i1p119-37.html)). The dispersion is not
noise; it is what makes a market cross.

**Proposal M5 — width is a read of the dispersion the party has seen, and what it has seen includes
the public record.** No floor is introduced and nothing is widened by decree: what changes is that
*how uncertain am I?* is answered from a longer source than *how often have I personally been
surprised?*. Falls out of M3 almost for free.

### 3.6 Nobody holds the stock

**Today.** Producers sell what they made; `merchants` buy in one place to sell in another (and have
never once run); banks quote credit. **No party's business is to hold goods and stand on both sides
of one book.**

**What the world does.** Between a mill and a household there is a wholesaler and a shop, and what
they are for is exactly the timing gap in §3.2: they buy when the producer wants to sell and sell
when the household wants to buy. Microstructure prices the same role the same way — a dealer's spread
is the cost of carrying inventory and the risk of being adversely selected
([Glosten–Milgrom onward](https://ideas.repec.org/p/arx/papers/1902.10743.html)).

**Proposal M6 — a `stockist` in the goods chain.** Buys from producers at what it expects to sell for
less its own required return on the money tied up; posts an ask to households out of the lots it
holds; wears the loss when the gap closes the wrong way. Its margin is an OUTCOME of turnover,
carrying cost and the prices it actually meets — no spread table, no fee schedule, which is how
`merchants` is already written. It is the missing intermediary Law 1 asks for, and it is what makes a
consumer price index possible at all (21.84).

### 3.7 What these close

M1+M2 between them are the demand side of 12c.3 and most of 21.79, 21.81 and 21.73. M3 is the
enabling change for all thirteen dead modules and for the banks' 1,141 refusals. M4 is the supply
side of 12c.3. M6 is 21.84 directly. M5 rides with M3.

**Order:** M3 first — it is the smallest and it unblocks measurement of everything else. Then M1+M2
together (a protocol table is worth little while every order evaporates). Then M4 and M6, which are
the goods chain. M5 falls out of M3.

## §4. What it does not do at all

*Ranked by what the absence costs. The first four are the ones that would have changed the last six
months of this project's history.*

### N1 — Nothing checks that the world is ALIVE

**Today.** Nine audit families: `money`, `ownership`, `prices`, `crossMarket`, `accounts`, `names`,
`flows`, `zeroSum`, `units`. **Every one of them is a consistency check.** A world can be perfectly
consistent and asleep, and this one is: 72 clears in 8,819 sessions, 13 modules that have never run,
428 of 648 capabilities never once used — and **every gate in this project is green**.

The findings register tells the same story from the other side. 12c.3, 21.72, 21.73, 21.76, 21.77,
21.79, 21.81 and 21.84 are all the same class — *a mechanism that never fires* — and every one of
them was found **by hand, months after the item that introduced it**, by somebody writing a probe.

**The vocabulary exists in the literature.** Statistical model checking applied to a macroeconomic
ABM *"provides a principled analysis layer… without rewriting the simulator in a dedicated
formalism"*, driven by *"reusable temporal queries, observable-specific precision targets, and
confidence-based stopping rules"*, and *"automatically determines the minimum number of simulations
needed to achieve user-specified confidence levels"*
([SMC of Keynes+Schumpeter](https://arxiv.org/html/2605.10447),
[MultiVeStA + Mesa](https://link.springer.com/chapter/10.1007/978-3-031-75434-0_26)).

**Proposal.** A **liveness family**, in the audit, reported with owner and size like any other
violation and never repaired. Its checks are temporal and each carries its own horizon, declared with
a reason: *every declared book has cleared within N periods; every living firm has produced, sold and
been paid within N; every bank has lent within N; every declared capability has produced an outcome
within N; no party has stood in arrears for more than N.* The reach read (`world/reach.ts`) already
computes the raw material — 428 never-used capabilities with their owning module — and publishes it
as a number. What is missing is that **nothing fails when the number is 428.**

*This is the highest-value absent thing in the engine, and it is perhaps two days' work.*

### N2 — One run of one seed is the entire evidence base

**Today.** Every number in this project — including all of §0 — comes from a single deterministic
trajectory. No ensemble exists, no test asserts anything about a distribution, and there is no way to
tell a fact about this world from a fact about this seed.

**Why it is not a luxury.** The validation literature's verdict on the class is blunt: *"due to
over-parameterization and the corresponding degrees of freedom, almost any simulation output can be
generated with an ABM, and thus replication of stylized facts only represents a weak test"*
([validation methodology](https://d-nb.info/1246195569/34)). If even matching real data weakly
identifies a model, one trajectory identifies nothing.

**Proposal.** `npm run ensemble -- <k>`: k seeds, the §0 census reported as a distribution, and a
rule with teeth — **a finding is not a finding until it reproduces across seeds**, and the record
says how many it was seen in. The number of seeds is itself computed, from SMC's confidence-based
stopping rule, rather than chosen.

### N3 — A payment that could wait has nowhere to wait

**Today.** Every instruction settles atomically or fails, and a failed money leg writes an arrear in
the same pass. There is no queue, no retry within the period, and no resolution of circular
shortfalls: A cannot pay B because B has not yet paid A, and both fail. §0 measures the result —
**1,902 failures for want of money, 1,617 of them maturities** — and 135 → 111 parties in 26 weeks.

**What real systems do.** Large-value payment systems queue, and the queue is the mechanism:
liquidity-saving features find cycles and settle them together, because a gridlock is a timing
failure, not a default, and resolving it needs no new money.

**Proposal.** A settlement queue with a stated lifetime, in states this project already has words
for: due → queued (retried after each later instruction that funds the payer) → failed (the arrear,
as today). Plus one pass at the end of the period that finds cycles and settles them together, **each
leg at full value** — which is not netting (still forbidden), it is a DvP cycle. One TECHNOLOGY (how
long a payment may wait) and nothing else.

### N4 — Nothing reads the system as a network

**Today.** Bilateral exposures are everywhere — loans, deposits, contracts, arrears — and nothing
ever reads them **as a graph**. When the banks stopped quoting every name (21.72), the available
reasons were per-bank: `appetite`, `it cannot cost its own funding`, `nobody lends to a party of this
kind`. Three local reasons for what is almost certainly a system-level fact.

**The measure exists.** DebtRank *"quantifies the extent of financial distress that a particular node
should face under external shocks and the corresponding risk contagion"*, recursively, without
waiting for a capital buffer to be exhausted ([DebtRank](https://arxiv.org/pdf/1504.01857)); the
empirical regularity is that contagion *"decreases with capitalization but increases with
concentration"*, non-monotonically in connectivity
([interbank networks](https://arxiv.org/pdf/2109.14360)).

**Proposal.** A network READ in Part XII's sense — published, causing nothing, never repairing: the
exposure graph each period, its concentration, and a DebtRank-style distress propagation from each
node. It needs **no new mechanism**: the edges are already in the register and the agreement book.

### N5 — Trade credit, which is how firms actually survive a timing problem

**Today.** 9 clauses MET of 22 — the weakest-covered system that is not a derivative. Firms in this
world buy with money or not at all, which is why so many of them die of a cash timing problem
(N3): real firms pay in thirty days.

**Proposal.** Promote it in the worklist: an invoice with terms; a discount for early payment that is
a price somebody quotes, never a rate somebody states; and the failure chain when an invoice is not
paid — which is also the missing channel by which a customer's failure becomes a supplier's.

### N6 — The world cannot say why it did anything

**Today.** Every diagnosis in the record was a hand-written probe: this session alone wrote six
throwaway test files to answer *why is nothing happening?*. The journal is already an event log with
subjects and data; there is no read over it that answers a question.

**Proposal.** An explainer over the journal: *why did this firm not produce?* → the chain of its own
reads and refusals in that period, in order, ending at the one that returned nothing. The material is
already recorded; what is missing is the query. Given that the project's scarcest resource is the
owner's diagnostic time, this may be the highest leverage per line of code in the whole review.

### N7 — Metamorphic relations, of which there is exactly one

**Today.** `ladder.test.ts` asserts scale invariance — and is itself red (76 small firms at grain 1
against 77 at grain 2), so the project's one invariance check does not currently hold.

**Why it is the right kind of test here.** Simulation validation *"poses a particularly potent form
of the oracle problem, and often no oracle exists"*, which is exactly what metamorphic relations are
for: *"necessary properties of the intended functionality… involving multiple executions"*
([metamorphic testing](https://en.wikipedia.org/wiki/Metamorphic_testing),
[MT for simulation validation](https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=932547)).

**Proposal.** A family of them, with `fast-check` which is already in the toolchain: scale invariance
(the ladder's, repaired), resolution invariance (double the share tick, nothing moves), phase-order
invariance (reorder phases that commute, the prints are identical), unit invariance (restate a
currency's subdivision, every ratio holds), and seed invariance **of properties, never of values**.

### N8 — What the spec says and nobody has built

Ranked by missing clauses, for the systems where the absence is a mechanism rather than a detail:
**Goods 10, Observer 10, Trade Credit 10, Equity 9, Hedge Funds 9, Money 8, M&A 8, Expectations 8,
CDS 8, FX Forwards 8.** Two of these deserve naming here because they are not exotic: **Goods** is
the real economy and is missing ten clauses while 1,648 lots perish unsold; **Observer** is how
anybody sees any of this, and is missing ten while the project's diagnostic method is hand-written
probes (N6).

### 4.1 Order

N1 first — without it, everything else in this review is unverifiable and the next dead mechanism
will again be found by hand in six months. N6 second, because it is cheap and it pays for itself in
the next diagnosis. N2 third, because it decides what a finding even is. Then N3 and N4, which are
mechanisms and reads respectively. N5 and N8 belong in the worklist at their dependency positions;
N7 rides with the ladder's repair (§2's S1).

---
