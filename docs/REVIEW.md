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

### 1.1 The seed, inventoried

`seeds/foundation.ts` (3,168 lines) plus the `seed.funding` module inside it, run once before period
0 through `SeedContext`. The whole opening is **nineteen calls**:

| door | calls | what it writes |
|---|---|---|
| `endowMoney` | 5 | deposits and reserves, out of nowhere |
| `endowUnits` | 10 | paper, plant, inventory, shares, land, out of nowhere |
| `prices.write` | 3 | opening prints for every priced line |
| `parties.add` | 9 | the population |
| `ctx.owes` | 1 | one agreement |
| **`ctx.settle`** | **0** | — |

That last row is the whole of §1. **The seed never settles anything**, so nothing it creates has a
counterparty, a date, a price anybody accepted, or a record. Three consequences follow mechanically:

**Every line is issued at the epoch.** `issueDate: ctx.calendar.epoch` for every seeded sovereign
line. Maturities are spread across a grid; issue dates are not. Nothing has accrued, nothing is
seasoned, every bond is on-the-run, and Seed D2's *"anything that accrues starts from a stated
accrual position"* holds only because every position is zero.

**Stocks are walked from ratios.** The seed states the scale of the world and walks it up each
recipe — *"a world of thirty million people opens with thirty million people's worth of stock in
it"*. Recorded at 12c.3: one baker opened with **314 million loaves against 27 million a period** of
expected demand.

**The banks are funded backwards.** `seed.funding` runs last, reads each bank's assets off the
register, and *derives what its depositors hold* from the leverage line that bank declared. The
deposits of this world are therefore a residual of the banks' asset endowment — which is the
accounting identity satisfied and the economics inverted: in a real system the deposits came first,
as somebody's borrowing.

### 1.2 What it produces, measured

| | period 0 | after 12 | after 26 |
|---|---|---|---|
| living parties | 135 | 101 | 111 |
| parties with an outlook of anything | **0** | 101 | — |
| instructions in the ledger | **0** | 4,167 | 11,814 |
| markets carrying a price | 132 | 135 | — |

An empty ledger and 132 priced markets: every one of those prices asserted, none traded. And **not
one party has an outlook**, because `observations()` reads exactly one source — the instructions a
party was a side of.

### 1.3 The asymmetry, which is the whole problem

A seeded world is handed two kinds of thing, and they behave completely differently once the clock
starts.

**An obligation is a DATE.** A coupon, a maturity, a rent, a payroll: it arrives whether or not
anybody is ready, because it is written on the calendar. The seed writes plenty of them.

**A relationship is a HISTORY.** A customer, a supplier, an employer, a lender, a going wage, a cost
of funds: in this model each exists *only* as an accumulated record — an outlook formed from
observations, a funding cost read from what a bank has paid, a sales expectation read from fills.
**The seed writes none of them, and cannot: there is no door.**

So the opening is not "a bit off". It is **systematically biased toward liabilities**: everything
that takes money out on a schedule is present at full strength; everything that brings money in is
absent. §0's measurements are that sentence: 1,617 failed maturities, 145 failed coupons, 1,902
failures for want of money, 700,000 hours of labour offered and none wanted, 135 → 111 parties.

Seed D1 asks for stocks *"consistent with the flows that will run"*; D1.a warns that otherwise
*"period one is a shock the model never recovers from, and everything measured afterwards measures
the recovery"*. That clause is not aspirational here — it is a description of every measurement this
project has ever taken.

### 1.4 The five ways to build an opening, and which are admissible here

| | who does it | what it gives | admissible here? |
|---|---|---|---|
| **A. Stated stock** | today's Phoenix; most teaching ABMs | consistency only | yes, and it is what fails |
| **B. Solved steady state** | DSGE; SFC calibration | consistency + stationarity | **no** — Seed D4: *the seed is a fixed point of nothing*; Law 2: no imported equilibrium |
| **C. Fitted synthetic population** | microsimulation (IPF, combinatorial optimisation) | realistic cross-section | **no** — Seed B5, C5: no observed ratio copied in, no sector fitted to a target |
| **D. Scripted past** | event-sourced systems; blockchain genesis | consistency + history | **yes**, under a grammar |
| **E. Lived past** | DF worldgen; ocean spin-up; SFC toolkits | consistency + history + liveness | **yes**, and it is the destination |

**B is forbidden and worth understanding anyway.** The admissible openings of an SFC model form a
**polytope** — *"the set of solutions is a polytope, which volume depends on the constraints applied
and reveals the potential fragility of the economic circuit, with no need to specify the dynamics"*,
and *"not all conceivable sets of stocks are consistent with the model's accounting"*
([Volume of the steady-state space](https://arxiv.org/pdf/1601.00822)). Two things transfer without
importing an equilibrium: the *volume* is a diagnostic computable without running anything, and
"land in a body" is a different act from "solve for a point".

**C is the one to name and refuse.** Population synthesis by iterative proportional fitting
*"alters the joint distribution to fit the marginal distribution of the attributes from the
aggregated data"*
([IPF review](https://www.sciencedirect.com/science/article/pii/S2352146516306925)) — it changes the
world until it matches the target. Seed B5 and C5 forbid exactly that. Take the vocabulary (a
population is *drawn*), refuse the method.

**E is what the serious practitioners do, in three different fields.**

- **SFC toolkits.** Setting initial conditions by hand *"is particularly difficult if there are
  multiple types of financial assets within the model"*, so *"the solver can start a new simulation
  at an earlier point in time and then simulate forward until reaching the desired initialization
  point, with starting time typically set to 200 periods prior"*
  ([Bond Economics, `sfc_models`](http://www.bondeconomics.com/2017/09/calculating-initial-steady-state-in.html)).
- **Earth-system models.** Ocean spin-up *"is usually in the order of a few thousand years"*;
  *"climate drift can be caused by a not-yet-equilibrated ocean initial state"* and post-processing
  *"has been found to be ineffective in removing it"*
  ([UKESM1](https://agupubs.onlinelibrary.wiley.com/doi/full/10.1029/2019MS001933)). The practice:
  spin up once, write a **restart file**, open every experiment from it — and when it is too
  expensive, *accelerate the convergence* rather than shorten it
  ([sequence acceleration](https://www.science.org/doi/10.1126/sciadv.adn2839)).
- **Dwarf Fortress.** *"There's a giant zero-player strategy game going on with somewhat loose turn
  rules and bad AI (but thousands of agents), and history is just a record of that. Procedurally
  generating stories by recording a log of a simulation is a valid enough approach."*
  ([world generation](https://dwarffortresswiki.org/index.php/World_generation)) — and nothing is
  accepted blindly: *"if the world does not meet the requirements of any one rejection parameter the
  world is rejected and re-randomised"*, with `LOG_MAP_REJECTS` writing *"a file… where every time a
  world is rejected the reason will be logged"*
  ([world rejection](https://dwarffortresswiki.org/index.php/v0.31:World_rejection)).

**And one thing DF does that Phoenix must NOT copy.** DF's worldgen is a *cheaper, coarser* model
than the fortress simulation — "loose turn rules and bad AI". A coarse pre-history here would be a
**second model of the same economy**, which Law 4 forbids outright. Phoenix's chronicle must run the
same engine; what makes a *scripted* past legal is not that it is a simpler model but that **it
contains no model at all** — only stock-creating instructions, no decisions (§1.5).

**The transient's length is measured, not chosen.** Sixty years of discrete-event literature on one
question: MSER-5 *"gives the truncation point as the point where the minimum standard error in the
data occurs"* and is *"the most effective and robust method"* tested; Welch's graphical procedure is
criticised because *"the user decides the warm-up length by observing a graph… thus there is
subjectivity"*
([AutoSimOA review](https://warwick.ac.uk/fac/soc/wbs/projects/autosimoa/current_work/website_warmup_methods_total_doc.pdf)).

**And the destination is a basin, not a number.** Mark-0's phase diagram shows *"the generic
existence of a phase transition between a 'good economy' where unemployment is low, and a 'bad
economy' where unemployment is high"*, with *"a transient that can be surprisingly long"*
([Naumann-Woleske et al.](https://arxiv.org/pdf/2111.08654),
[Gualdi et al.](https://arxiv.org/pdf/1307.5319)). No adjustment to a stock ratio moves a world
between basins. Only a path does.

### 1.5 The design — THE CHRONICLE

```
  draw       the physical world and the primitives         declared, small, auditable
  chronicle  a past, lived through ordinary settlement     produces the ledger
  accept     a census of properties, or REJECT with a reason   logged; re-draw; never adjust
  snapshot   the accepted world as an artifact             an optimisation, never the truth
```

**(a) The kernel surface.** One new store and one new pre-period phase:

```ts
// world/chronicle.ts
export interface Told {                    // one moment of the past
  readonly at: Civil;                      // a real date, before the epoch
  readonly draft: InstructionDraft;        // settled through the ordinary path
  readonly why: string;                    // Law 16: a reader must know why this happened
}
export interface Chronicle {
  readonly from: Civil;                    // when the past begins
  readonly told: readonly Told[];          // in date order; stable under the same seed value
}
```

The world's epoch moves back to `chronicle.from`; `period(0)` is the first period AFTER the last
told moment. Every date in the past is a real date on the one calendar (Money G3), so accruals,
seasoning, anniversaries (item 20's `crossesAnniversary`) and maturity profiles are reads rather
than statements.

**(b) The grammar, which is what makes a scripted past legal.** A chronicle instruction may create a
stock — issue, buy, hire, lend, deliver, pay — and may **never write a price, a rate, a mark or a
decision**. The guard is at the door, not in a comment:

```ts
forbid(draft.legs.every(notAPrint), 'Seed E1', 'a chronicle may not write a price');
forbid(!touchesParams(draft), 'Seed E1', 'a chronicle may not set a policy number');
```

What something cost in the past is what that instruction says two parties agreed; what anything is
worth from period 0 is cleared. This is an **endowment expressed as a flow**, and it is why a
scripted past is not a scripted narrative (§45 E2): it decides nothing that a mechanism owns.

**(c) Determinism, taken from the event-sourcing literature's scar tissue.** *"Your event handlers
must be deterministic — given the same sequence of events, they should always produce the same
result"*, and *"snapshots are an optimization, not the source of truth. The event log still is"*
([Kurrent](https://www.kurrent.io/blog/snapshots-in-event-sourcing/),
[replay pitfalls](https://dev.to/alex_aslam/when-event-sourcing-fails-war-stories-from-production-1nk2)).
Three rules follow, and they are cheap to enforce:

1. **The seed value plus the chronicle generator is the truth; the snapshot is a cache.** A snapshot
   must be reproducible from the seed value, and `check:opening` regenerates one world per run to
   prove it. Otherwise a snapshot becomes a new way to smuggle in a stated opening.
2. **A told moment carries everything it needs.** No chronicle instruction may read a live market —
   what it needs is in the instruction, which is the event-sourcing rule *"add the information
   retrieved in the event"*.
3. **The chronicle is versioned.** A told moment's shape is a schema with a version, because
   *"if you change the meaning of an event without versioning it properly, replays can silently
   become incorrect"*.

**(d) What the seed may still state, after this.** Land and places; people, as cells with weights;
technologies (recipes, lead times, plant lives); the primitives (preferences, policies); and the
constitutional parties — a state, a central bank, a first bank — because a banking system cannot
bootstrap itself from nothing *inside its own rules*. That last exemption is real and has a
literature: the search-theoretic account of how a medium of exchange arises
([Kiyotaki–Wright](https://en.wikipedia.org/wiki/Kiyotaki%E2%80%93Wright_model_of_money)) is a model
of a different question than this project is asking, and the honest position is that **Phoenix opens
after money exists, and says so in one place** rather than pretending an issuer appeared by
instruction.

**(e) Acceptance: nine properties, never values.**

| # | property | what its failure means |
|---|---|---|
| 1 | every market has traded at least once | a book nobody uses |
| 2 | every living party has at least one outlook | §3.3's fail-closed loop is still live |
| 3 | every firm has produced, sold and been paid, in different periods | 12c.3 |
| 4 | every bank has lent and been repaid | the credit channel is dead |
| 5 | every declared instrument kind is held by somebody who chose to hold it | a dead module (there are 13) |
| 6 | the maturity profile spans more than one period; issue dates are dispersed | a wall |
| 7 | ages are dispersed; no cohort is the age of the world | everything was born at once |
| 8 | the audit is green and every balance sheet closes | Seed A2 |
| 9 | nothing in the register is younger than the world | a stock with no history |

A world failing any of them is **rejected, re-drawn from the next seed value, and the reason logged**
— `docs/rejections.log`, one line per attempt: seed value, criterion, the party or market that
failed it. **A criterion that fails for every seed is a missing mechanism with a name**, which is
exactly the diagnostic this project has been producing by hand.

**(f) Why rejection is legal where calibration is not.** Rejection **discards a world; it never
adjusts one.** No number inside an accepted world was touched by the criterion — nothing is fitted
(B5), no outcome is seeded (E1), and the accepted world is a world the mechanisms themselves
produced. IPF, by contrast, moves the world until it matches. The difference is the whole of why D
and E are admissible and C is not.

**(g) The length.** Run the chronicle long; record a handful of named series (money per member, the
going wage, living parties, sessions cleared, the credit stock); take the truncation point from
MSER-5. The length is then a RESOLUTION under Law 2 and is tested as one: **double it, and the
opening's properties must not move.**

### 1.6 Migration — what is replaced, in what order, and what it deletes

Each step replaces endowment with instruction for one layer, and each is independently checkable
because the census (e) tightens as the layers land.

| step | layer | replaces | deletes |
|---|---|---|---|
| 22b.3 | money and the sovereign | reserves and bills endowed | `endowMoney` (5 sites) |
| 22b.4 | firms, plant, inventory | stock walked from ratios | `endowUnits` (10 sites), `SEED_STOCK_BASIS` |
| 22b.5 | households, employment, savings | deposits as a leverage residual | `seed.funding`'s inversion |
| 22b.6 | prices | 3 `prices.write` calls | `seed.openingPrice.*`, `seed.openingWage` |

**And then the second source.** Scripted moments are replaced, module by module, by **warm-up
periods the engine runs itself** — the same machinery, a different origin for the instructions. A
module graduates when the world can produce that behaviour on its own, which makes each graduation a
measurement of whether a mechanism works. The goods chain cannot graduate before §3's M1–M4 land;
that is not an obstacle, it is the dependency stated.

### 1.7 What this predicts, so that it is a confirmation and not a surprise

- **The first chronicle is rejected on criterion 3** — every firm has produced, sold and been paid.
  That is 12c.3, and the rejection log naming it on attempt one is the proposal working.
- **Money will be hard to create legally.** Every deposit must arrive as somebody's borrowing or a
  central bank's purchase. A chronicle that cannot create this world's money stock is telling us
  something true about the mechanism, and it will be uncomfortable.
- **Period 0 moves, and the findings register must be re-read.** Everything measured before was
  measured against a different world.
- **The audit at period 0 becomes a real test** — today it tests a hand-made balance sheet; against
  a replayed past it tests settlement, revaluation and the audit together.

### 1.8 The items

Before 22a, which they absorb; after item 21's stops, because a chronicle cannot run through a world
that throws.

- **22b.1** `world/chronicle.ts` — the store, the pre-period replay, the epoch shift, the grammar
  guard, the version on a told moment, determinism from one seed value.
- **22b.2** `check:opening` — the nine properties, the rejection log, and the regeneration check that
  keeps the snapshot an optimisation.
- **22b.3–22b.6** the four migration steps above.
- **22b.7** the snapshot format and `assemble()` opening from one; the rig's scale models become
  small accepted worlds.
- **22b.8** MSER-5 on the named series; the length becomes a tested resolution.
- **22b.9** the first module graduated from scripted to lived.

**Exit.** `grep -rn "endowMoney\|endowUnits\|openingPrice\|prices\.write" src/` is empty;
`check:opening` is green; `docs/rejections.log` is empty for the chosen seed; and the census of
period 0 is indistinguishable in KIND from the census of period 100.

## §2. Speed

*What a run costs, where the cost actually is, and what is worth doing in what order.*

### 2.1 The three rungs, and the one that does not run

26 periods each, at `ab18535`:

| rung (banks, firms) | parties | markets | 26 periods | ms/period |
|---|---|---|---|---|
| (3, 12) | 127 | 350 | 8.6 s | **331** |
| (6, 60) | 229 | 661 | **stops at period 20** | ~750 |
| (12, 200) | 866 | 765 | 236 s | **9,078** |

**Cost grows as about the 1.7th power of the population.** Parties ×6.8 from the first rung to the
third, cost per period ×27. 0g's exit — *"(12, 200) one country: a 52-period year under 60 s"* — is a
**factor of eight** away, and the gap is structural rather than constant-factor.

**The middle rung stops** at period 20 with `[Money C1.a] instruction 23596: a payment from an
account to itself is not a payment`, from the treasury's receipts: finding 21.87. So the performance
programme's own instrument cannot produce its own three-rung comparison, and every published speed
number in this project is a first-rung number.

### 2.2 Where the time goes, at the scale that matters

A CPU profile of eight periods at the **third** rung, attributed by inclusive time to the nearest
source directory on the stack:

```
34.4%  calendar/     20.7%  prices/      7.2%  core/      5.9%  world/
 3.7%  register/      3.0%  registry/    1.9%  expectations   <1% everything else
```

**Over half the run is date arithmetic and curve reading, and the economics is under a tenth of it.**
Walking up from every sample inside `calendar/` and `prices/` to the first frame outside them gives
one caller:

```
53.2% of the entire run is inside World.curveAt
```

### 2.3 What `curveAt` actually does

`world.ts:3412` → `prices/curve.ts readCurve`, and the body is this, **on every call, with nothing
remembered**:

```ts
for (const i of inputs.instruments()) {          // EVERY instrument in the world
  if (!issuedBy(i, family.issuer) || i.ccy !== family.ccy || !i.status.live) continue;
  const flows = inputs.cashFlows(i, on);          // rebuild the schedule
  const print = inputs.prices.latest(i.id, at);
  const tenorYears = yearFraction(family.dayCount, on, last.date);
  const y = yieldOf(flows, dirty, on, family.dayCount, …);   // a ROOT-FIND, per instrument
}
points.sort(…);
```

Measured call counts, over four periods:

| rung | instruments | curve reads / period |
|---|---|---|
| (3, 12) | 853 | **28** |
| (12, 200) | 1,387 | **1,079** |

Instruments ×1.6 between the rungs; curve reads ×38, because the callers are parties and valuations
rather than lines. At the third rung that is **~1.5 million instrument-visits per period, each with a
yield inversion**, to produce four curves that do not change within a period.

This also explains why the first rung's profile never showed it: at 28 reads a period the curve is
invisible, and at 1,079 it is the program. **The hot set is a function of scale** — which is the
general lesson, and it means the 2.4× this week's 0g work bought (1,354 → 554 ms/period at the first
rung) was real work aimed at a world nobody will run.

### 2.4 The other half: the engine's effort goes into auctions nobody attends

From §0: **8,819 market sessions, 72 cleared, 7,903 finding no demand at all.** The loop opens 350
books at the first rung and 765 at the third, asks every eligible participant in each, and runs a
solver over the answers. No constant-factor work touches this; the saving is **not holding the
auction**, which is §3's M1–M3 and happens to be the largest performance item in the review.

### 2.5 How this class of engine is made fast elsewhere

**Cache what does not change within a step.** This is not exotic — it is what every risk system does
with a discount curve: build it once per valuation date, from the prints of that date, and hand out
the same object. Phoenix already has the machinery, built this week: `view.memo(key, at, compute)`
with `versions()` giving the register's write count, the price store's and the instrument store's.
A curve is a pure function of (family, period, prices.version, instruments.version).

**Entities sleep, and something wakes them.** Factorio: *"entities not in the 'Active' section don't
get touched during normal tick updates"*, and the wake is caused rather than polled — *"an inserter…
will be deactivated if the assembler's input is filled. Once the assembler produces an item, the
assembler activates the inserter"*
([diagnosing performance](https://wiki.factorio.com/Tutorial:Diagnosing_performance_issues),
[active entities](https://forums.factorio.com/viewtopic.php?t=38047)). Their memory lessons are the
other half — *"all active entities are read at every tick… too much data for caches"*, answered with
prefetching and per-chunk allocators ([FFF-204](https://factorio.com/blog/post/fff-204),
[FFF-215](https://factorio.com/blog/post/fff-215)) — and multithreading is gated by the constraint
Phoenix shares: *"the game needs to remain fully deterministic"*
([FFF-421](https://www.factorio.com/blog/post/fff-421)).

**Process what happened, not what might.** *"The default execution scheme of agent-based modeling
relies on fixed-increment time advances, whereas discrete event simulation… is assessed at specific
time points triggered by an event"*, and the discrete-event implementation *"performs better on
CPU"* ([JASSS](https://www.jasss.org/27/1/10.html)). The week stays — the audit closes on it — but
*within* a period the same rule applies exactly.

**In a JavaScript engine specifically**, the ceiling is set by allocation and object shape: *"assign
all of an object's properties in its constructor"* because *"adding properties after instantiation
will force a hidden class change"*; *"for numeric data, typed arrays are 3-10x faster than regular
arrays because they use contiguous memory"*; and *"code that executes the same method repeatedly will
run faster… due to inline caching"* ([V8 hidden classes](https://v8.dev/blog/fast-properties),
[optimising for V8](https://medium.com/@zlatkov/how-javascript-works-inside-the-v8-engine-5-tips-on-how-to-write-optimized-code-ac089e62b12e)).
That is the honest frame for 0g.3 and 0g.11: this engine allocates a `Cash` object per addition, and
the collector is 6.3% of the third rung and was 8.7% of the first.

**Not applicable, and worth saying.** FLAME GPU's *"hundreds of millions of agents"* and *"1000x
speedup"* are for local, homogeneous, embarrassingly parallel rules
([FLAME GPU 2](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3207)). A Phoenix period is a
sequence of global serialisations — one solver, one settlement, one audit.

### 2.6 Proposals

**S1 — Repair the ladder's middle rung (21.87).** One guard in `runReceipts`. Until it lands, the
performance programme cannot measure the thing its exit is written about. *Hours.*

**S2 — Memoise the curve.** `curveAt` keyed on (family, period, `prices.version`,
`instruments.version`), through the `view.memo` built at 0g.5a. 1,079 reads a period become four.
*Expected: ~2× at the third rung, for hours of work, with no mechanism touched — this is the single
best-value item in the whole review.*

**S3 — Memoise the day count and warm-start the inversion.** `yearFraction` and `dayNumber` are pure
functions of (date, date, convention) and are **34% of the third rung between them**; `yieldOf`'s
`invertDecreasing` starts from a cold bracket every time and should start from that instrument's last
yield (0g.7 says so and is unbuilt). *Expected: most of what S2 leaves.*

**S4 — Open a book only where somebody has a reason to be in it.** A participant registers an
interest in a venue; the session opens where interests exist; the books that never open become a
measured fact (§4's liveness read) instead of invisible work. *Expected: most of the 99.2%. It is a
modelling change (§3) and is listed here because it is also the largest speed item.*

**S5 — Wake, don't poll, inside a period.** A phase visits the parties something happened to. The raw
material exists: the register's write count (0g.5a) and the period's ledger slice. *This is 0g.2's
period index with a purpose.*

**S6 — Then the structural passes, in this order:** 0g.3 (`Measure` as a number brand rather than an
object — the allocator, everywhere), 0g.11 (columnar state; typed arrays where the data is numeric),
0g.14 (tiered journal — 105,186 events in eight periods at the third rung). *Mechanical passes over
the whole engine; they belong after S1–S5, not before.*

**S7 — Gate it.** The ladder is wired to nothing: the first rung went from 5.1 s a year at 0g.1 to
70 s a year this week with no check noticing. `check:forbids` already demonstrates the pattern — a
baseline that may only fall. *An hour.*

**What not to do:** GPUs; threads, until determinism is settled and measured; and any further
first-rung micro-optimisation, which §2.3 shows is optimisation of a world nobody runs.

## §3. What it does, done badly

*The mechanisms that exist and are weaker than the best available.*

### 3.1 The rule that runs this world: fail closed

**The shape.** A decision reads an outlook; if the party has none, the decision returns nothing.
Fifteen sites do it explicitly, one per module that matters, and the variable each waits for is the
one that never arrives:

| module | waits for | and returns |
|---|---|---|
| `households/consume.ts` | `income`, then `price` per good | `return` / `continue` |
| `households/portfolio.ts` | `income` | `return` |
| `firms/produce.ts` | `earnings` | `return` |
| `firms/index.ts` | `condition` | `return` |
| `supply/index.ts` | `sold` / `bought` | `return` |
| `banks/prime.ts`, `funds/things.ts`, `options`, `securities-lending`, `treasury` | `price` | `return` |
| `housing/index.ts` | `income` | `return` |
| `reporting/guidance.ts` | `income` | `return` |
| `money-market/policy.ts` | `index` | `continue` |
| `households/index.ts` | `mortality` | `return` |

Each refusal is individually correct — a party may not invent a number it does not have — and
together they are why **13 modules have never produced anything** and why the world is measurably
asleep.

**The three loops they close.** Every one of §0's measurements is one of these:

```
  no sales expectation → hours = expectsToSell/yield × hoursPerUnit = 0  → nobody is hired
      → no household income → no income outlook → no demand schedule → no sales
  no funding history → "it cannot cost its own funding" (1,141×) → no quote → no lending
      → no funding history
  no price outlook → no supply-contract order → no contract → no fills → no price outlook
```

The first is verifiable line by line: `firms/decide.ts` sets
`hours = downTick(scale(perPeriod, tech.hoursPerUnit))` with `perPeriod = expectsToSell / yieldRate`
when `worthMaking`, and `0` otherwise — which is exactly the measured **700,000 hours offered, 0
wanted, every period, in every venue.**

**The fix already exists in this codebase and is used for something else.** `expectations/index.ts`
keeps **two tracks** per party and variable: the party's own adaptive outlook, and an `anchored` one
— *"the last PUBLIC level"* — and it switches to whichever has surprised the party less. But the
record is created only on a first observation:

```ts
const h = (forParty[variable] ??= {
  // "A party that has never seen this variable has no outlook to be surprised against:
  //  its first observation IS its outlook, and it is surprised by nothing (B2)."
  expected: seen.value, …
});
```

So a party with no private observation **never consults the public track it is entitled to read**.

**Proposal M1 — an outlook exists from the moment a public level does.** For any variable with a
public level, every party that may read it has an outlook: expectation = the public level;
confidence = the dispersion of the public record; and the party's own observations pull it away
exactly as they do now, at its own memory. This is a READ of a source (Law 19), not a default
(`?? 0`) — it is what a new entrant in a real market does: look at the posted price, the published
index, the policy rate. Its correctness condition is already written in §46: *"the anchored one is
the last PUBLIC level"*, and *"no peeking at the period's own result"* still holds because the public
level is last period's.

*This is the smallest change in the review with the largest reach.* It turns on supply contracts,
dealer quotes, firm batches, household schedules and the bank's cost of funds at once, and it is the
join between §1 and §3: the chronicle supplies the history, this supplies behaviour for everything
the chronicle has not touched.

### 3.2 One market microstructure, for everything

**Today.** `clearing/solver.ts` is *"one solver for every market"* — participants post limit
schedules; the solver takes the uniform price that executes the most volume. `VenueDecl` declares an
id, a name, the clearing module, a unit, a currency and a key; **there is no protocol field**, and
the only variation anywhere is a tie-break (`sellersCompete` | `marginalBid`).

**Why it is wrong on this project's own terms.** Law 1: *reflect the real mechanism — real named
counterparties, intermediaries, lags, fees, refusals.* A Walrasian auctioneer for bread is the one
intermediary that has never existed. And the auction is not neutral: a weekly uniform-price call with
no resting orders clears **only** when two parties independently want opposite sides of the same book
in the same week, which in a world of 350 books and 127 parties is almost never — 7,903 of 8,819
sessions found no demand at all.

**What the best practice is.** The dominant protocol in macroeconomic ABMs is not an auction: demand
agents *"observe the prices or interest rates charged by a random subset of suppliers"* and *"switch
from the old partner to the best potential partner selected in this random subset with a probability…
as a non-linear function of the percentage difference in their prices"*, applied *"in four markets:
goods, labour, credit and deposit — according to a fully decentralized matching mechanism"*
([Caiani et al. 2016](https://www.sciencedirect.com/science/article/abs/pii/S0165188915301020),
[Riccetti, Russo & Gallegati](https://link.springer.com/article/10.1007/s11403-014-0130-8)). Exchanges
went the other way — *"progressively adopted continuous double auctions"* — keeping the call for
openings and closings ([Walras to CDA](https://arxiv.org/pdf/1506.03758)).

**Proposal M2 — `VenueDecl.protocol`, DATA behind a dispatch table (Law 15).**

```ts
export type Protocol = 'call' | 'posted' | 'book';
export interface VenueDecl { …; readonly protocol: Protocol; readonly seenBy?: number; }
```

- `call` — today's solver, unchanged: sovereign auctions, fixings, anything a real market strikes at
  one price at one moment.
- `posted` — a seller posts an ask it stands behind; a buyer sees `seenBy` sellers (a TECHNOLOGY: how
  much of a market a buyer can see) and buys the best it saw, at that seller's price. **Still cleared
  from real supply meeting real demand (Law 3)**: the trade happens because a buyer accepted a price
  a seller was standing behind — which is what a price in a shop is.
- `book` — resting orders with continuous matching (M3), for exchanges.

Each protocol is a module with its own matching function; the kernel dispatches on the venue's
declaration and never branches on a kind.

### 3.3 Nothing rests

**Today.** `World.runOne` asks every eligible participant for orders, clears, and throws the orders
away. Nothing survives the session.

**Why this is the deeper half of §3.2.** It is why `noDemand` (7,903) dwarfs `noOverlap` (323): the
two sides are not failing to agree on a price — **they are failing to be in the room in the same
week**. In every real market an offer stands until taken, withdrawn or expired: a limit order, a
price on a shelf, a bank's standing quote, a firm's price list.

**Proposal M3 — a standing-order register**, a kernel noun beside agreements and processes:
`(party, venue, side, level, quantity, from, until, why)` — entered by a participant, cancelled by
its owner, expired by the calendar, consumed by a match. Every session opens with the standing book.
Not a cache: **an offer is a commitment with a date**, which is this project's ontology everywhere
else.

### 3.4 A firm with a full warehouse stops existing instead of cutting its price

**Today** (`firms/decide.ts`):

```ts
const wanted = worthMaking ? over(minus(expectsToSell, stock), tech.yieldRate) : 0;
```

A stock-adjustment rule whose **desired buffer is exactly zero**: a firm wants to hold nothing beyond
one period's expected sales. And the ask is *"what it expects a unit to fetch, less what perishes
before it could"* — **no feedback whatever from unsold stock into the price**. So a firm with stock
above expectation starts nothing and waits, at a price it has no reason to lower. 12c.3 records the
result: `batch 0, bound demand`, for ever; §0 measures the other end: 1,648 lots perished.

**What the best practice is.** In the K+S and AB-SFC families a firm holds a desired inventory as a
share of expected demand and adjusts its PRICE on two signals — did it sell out, and are inventories
above or below that desire
([Caiani et al.](https://www.sciencedirect.com/science/article/abs/pii/S0165188915301020)); in the
labour-augmented K+S line the same adaptive structure carries wages, *"with its dynamics related to
the inflation gap, productivity, and unemployment"*
([Dosi et al. line](https://academic.oup.com/ser/article/16/4/687/4739737)). The adjustment is
bounded in size, never in direction. It is what makes a glut clear.

**Proposal M4 — two changes, both reasons rather than levels.**

1. **A desired cover**, drawn per firm as a PREFERENCE — how many periods of expected sales it wants
   on the shelf — dispersed like `payoutPatience`. Nothing stated globally, no ratio imported.
2. **The ask answers the shelf.** What a unit is worth to the firm is what it expects to fetch *if it
   can sell it*; a firm holding more cover than it wants has an alternative to selling today that it
   can already price — storage, perishing, and the money tied up. Subtracting it is not a markdown
   rule; it is the value of holding, which the docstring already claims to compute and currently
   computes as if the shelf were empty.

### 3.5 Nobody may post a schedule of zero width

**Today.** Confidence is the mean absolute surprise, and a party with no surprises has none:
`households/consume.ts` posts `width: outlook.some ? outlook.value.confidence : 0`. A point demand
meets a point supply, and they cross only on exact equality.

**Why that is fatal in a thin market.** Gode and Sunder: *"allocative efficiency of a double auction
derives largely from its structure"*, and a budget constraint over **dispersed** reservation prices
reaches *"close to 100 percent"* efficiency with zero-intelligence traders
([JPE 1993](https://ideas.repec.org/a/ucp/jpolec/v101y1993i1p119-37.html)). The dispersion is not
noise; it is the thing that makes a market cross.

**Proposal M5 — width is the dispersion the party has seen, and what it has seen includes the public
record.** No floor, no widening by decree: *how uncertain am I?* is answered from a longer source
than *how often have I personally been surprised?*. Falls out of M1 almost for free.

### 3.6 Nobody holds the stock

**Today.** Producers sell what they made; `merchants` buy in one place to sell in another (and has
never once run); banks quote credit. **No party's business is to hold goods and stand on both sides
of one book**, which is precisely the role that bridges the timing gap of §3.3.

**What the world does.** Between a mill and a household there is a wholesaler and a shop. Microstructure
prices the same role the same way: a dealer's spread is the cost of carrying inventory and the risk of
being adversely selected ([Glosten–Milgrom onward](https://ideas.repec.org/p/arx/papers/1902.10743.html)).

**Proposal M6 — a `stockist` in the goods chain.** Buys from producers at what it expects to sell for,
less its own required return on the money tied up; posts an ask to households out of the lots it
holds; wears the loss when the gap closes the wrong way. Its margin is an OUTCOME of turnover,
carrying cost and the prices it meets — no spread table, no fee schedule, which is how `merchants` is
already written. It is the missing intermediary Law 1 asks for and what makes a consumer price index
possible at all (21.84).

### 3.7 Order, and what each closes

| | change | closes |
|---|---|---|
| 1 | **M1** an outlook from the public level | the enabling change for all 13 dead modules and the banks' 1,141 refusals |
| 2 | **M5** width from dispersion seen | rides with M1 |
| 3 | **M2 + M3** protocols per venue; orders that rest | the demand side of 12c.3; 21.79; 21.81; most of the 99.2% empty sessions |
| 4 | **M4** desired cover; an ask that answers the shelf | the supply side of 12c.3; the 1,648 perished lots |
| 5 | **M6** the stockist | 21.84 |

M1 first, because it is days of work and it makes every other change measurable: with parties that
can act, a failure to trade is a fact about the market rather than about the bootstrap.

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

---

## §5. One programme

*The twenty proposals above are not a menu. They are four cuts through the single fact in §0, and
they have an order that follows from it.*

**The loop to break.** No history → no outlook → no order → no trade → no income → no history. Two
proposals break it, and nothing else in this review matters until one of them lands.

| # | do this | from | breaks | costs |
|---|---|---|---|---|
| 1 | **M3** a party with no private history follows the public record | §3.3 | the loop, at the outlook | days |
| 2 | **N1** the liveness family, so the world says when it is asleep | §4 | the blindness that let this run for months | ~2 days |
| 3 | **S1** repair the ladder's middle rung (21.87) | §2.5 | measurement at any scale but the smallest | hours |
| 4 | **N6** an explainer over the journal | §4 | hand-written probes as the diagnostic method | days |
| 5 | **22b.1–22b.2** the chronicle and its census | §1.6 | the loop, at the history | weeks |
| 6 | **M1 + M2** protocols per venue; orders that rest | §3.1, §3.2 | 99.2% empty sessions, and S3 with it | weeks |
| 7 | **M4 + M6** desired cover, an ask that answers the shelf, a stockist | §3.4, §3.6 | the goods chain, and 21.84 | weeks |
| 8 | **N2** ensembles, and a finding that must reproduce | §4 | one seed standing in for a world | days |
| 9 | **S2** 0g.7: memoise the curve, the day count, the inversion | §2.5 | 55% of the cost at the rung that matters | ~1 day |
| 10 | **N3** the settlement queue and the gridlock pass | §4 | 1,617 failed maturities; a fifth of the deaths | weeks |
| 11 | **N4** the network read · **N7** the metamorphic family · **S6** the ladder gate | §4, §2.5 | three cheap, independent instruments | days each |
| 12 | **S4, S5** wake-don't-poll, then the structural passes (0g.3, 0g.11, 0g.14) | §2.5 | the remaining factor of ten | weeks |
| 13 | **N5, N8** trade credit; the ten-clause gaps in Goods and Observer | §4 | worklist items at their dependency positions | — |

**Why this order.** 1–4 are cheap and they are *instruments*: after them, the world can act, say when
it is not acting, be measured at the scale that matters, and explain itself. Everything from 5 down
is expensive, and every one of those is easier to do — and impossible to get wrong silently — once
the instruments exist. 9 is out of order on purpose: it is a day's work for half the cost at the
third rung, and it makes 5 affordable.

**What it would close.** 12c.3, 21.72, 21.73, 21.76, 21.77, 21.79, 21.81, 21.84, 21.86, 21.87 and a
share of 21.36 — which is most of the open findings register, and all of the "a mechanism that never
fires" class.

**The one-line test of whether it worked.** Run the §0 census again. *428 never-used capabilities,
13 dead modules, 72 clears in 8,819 sessions* is the before. There is no arguing with the after.
