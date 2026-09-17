# Review — the four questions, and the answers

*A single working file. The owner asked four questions; each is a section below. Every section is
filled in from the SOURCE (read, quoted, cited by file) and from MEASUREMENT (runs of this world,
numbers reported with the seed and the rung they came from), then against the best practice outside
this project, and ends in proposals that name the items they become and the recorded findings they
close. A section that has not been done yet says so.*

**Status.** §0 done · §1 done · §2 done · §3 done · §4 done.

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

*What a run costs, where the cost is, how other simulators of this kind are made fast, and what is
worth doing here in what order.*

## §3. What it does, done badly

*The mechanisms that exist and are weaker than the best available: what each does today, what the
best practice is, and the change.*

## §4. What it does not do at all

*Mechanisms and readings that are absent, ranked by what their absence costs this world.*

---
