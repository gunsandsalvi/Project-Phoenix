# The opening world

*A proposal, second draft, written against `41d7e66`. It answers the owner's question — this world
should not start from scratch, it should open as one already running, like peeking inside it — by
measuring what the opening actually is today, separating the three different problems hiding inside
"seeding", reading how six other fields solve each of them, and proposing one construction.*

---

## 1. What the opening is, measured

Not asserted — these are readings taken from the rig at `41d7e66`.

| | period 0 | after 12 periods |
|---|---|---|
| living parties | 135 | **101** |
| parties with an outlook of anything | **0** | 101 |
| markets | 136 | 355 |
| markets carrying a price | 132 | 135 |
| instructions in the ledger | **0** | 4,167 |
| events in the journal | 1 | 24,716 |

Read those six rows together and the opening states itself.

**The ledger is empty and 132 markets already have a price.** Every one of those prices was
asserted. Nothing traded to produce them, because nothing has happened at all: `ctx.settle` is called
**zero times** in `seeds/foundation.ts` — balances are written straight into the register through
`moneyDelta` and `adjustIssued`.

**Not one party has an outlook.** §46 says an outlook is formed from what the party itself observed,
and `observations()` reads exactly one source: the instructions the party was a side of. With an
empty ledger there are none. So every mechanism that decides against an outlook — post a supply
order, plan a batch, bid in a book, take a position, vote — opens **blind**.

**A third of the world dies in twelve weeks** (135 → 101 living parties). Seed D1.a names this
exactly: *"otherwise period one is a shock the model never recovers from, and everything measured
afterwards measures the recovery."*

Two more facts from the source:

- **Every seeded line is issued at the epoch** (`issueDate: ctx.calendar.epoch`). Maturities are
  spread over a grid; issue dates are not. So nothing has ever accrued, nothing is seasoned, every
  bond is on-the-run, and Seed D2's *"anything that accrues starts from a stated accrual position"*
  holds only because every position is zero.
- **Stocks are walked from ratios**, which was the right fix to a worse problem (seventy-two
  hand-written quantities) and is the source of the biggest one now. The seed states the scale of
  the world and walks it up each recipe. Recorded at 12c.3: **one baker opened holding 314 million
  loaves against an expected 27 million a period** — eleven periods of the entire world's demand, in
  one firm's warehouse, on day one.

---

## 2. Three problems wear one name

"Seeding" is doing three jobs, and every field that has faced this separates them. Phoenix's
findings are what you get when all three are attempted by one table of declarations.

**(I) CONSISTENCY — is the opening arithmetically admissible?** Every asset somebody's liability,
every balance sheet closing, the audit green at period 0 (Seed A1, A2, C1, C2). *This one Phoenix
already does well.* The audit is green at period 0 and that is a real achievement.

**(II) HISTORY — does the world have a past?** Ages, accruals, seasoned paper, lot bases with
acquisition dates, employment tenure, and above all **what each party has observed**, because that is
what expectations are made of. *Phoenix has none of this: the ledger is empty.*

**(III) LIVENESS — is the opening in a living region of the state space?** A world can be perfectly
consistent and perfectly dead: full of stock, so nobody produces; nobody produces, so nobody is paid;
nobody is paid, so nobody buys. *Phoenix is here, and consistency cannot see it.*

The twelve recorded findings are (III) seen from twelve sides, with (II) as the mechanism that keeps
the world from climbing out: 12c.3 (firms produce once, never again), 21.79 (0 bids in 66 supply
books over 16 periods), 21.84 (the consumer basket is empty, so **no rate has ever moved in this
world for a reason**), 21.77 (two overnight fixings in twelve periods), 21.72 (banks stop quoting
every name by period 9), 21.81 (0 securitisation cuts, every deal failing for want of a bidder),
21.76 (the first named firm publishes accounts at period 25), 21.73 (`control` never runs).

---

## 3. How six other fields solve it

### Stock-flow-consistent macroeconomics — (I), and a lesson about the shape of the answer

The admissible openings of an SFC model form a **polytope**: the accounting constraints are linear,
so the set of consistent stock configurations is a convex body, and *"the set of solutions is a
polytope, which volume depends on the constraints applied and reveals the potential fragility of the
economic circuit, with no need to specify the dynamics"*
([Volume of the steady-state space](https://arxiv.org/pdf/1601.00822)). Two things follow that
Phoenix should take seriously. First, **you do not need to pick a point; you need to land in a
body** — and the body's volume is a diagnostic in its own right, computable without running
anything. Second, *"not all conceivable sets of stocks are consistent with the model's accounting"*
and *"the model will follow a different traverse path for every possible set of stocks"* — path
dependence is not a nuisance here, it is the subject.

On how practitioners actually get an initial state, the SFC toolkits are blunt about it: setting
initial conditions by hand *"is particularly difficult if there are multiple types of financial
assets within the model"*, and the working answer is to **simulate the past** — *"the solver can
start a new simulation at an earlier point in time and then simulate forward until reaching the
desired initialization point, with starting time typically set to 200 periods prior"*
([Bond Economics, `sfc_models`](http://www.bondeconomics.com/2017/09/calculating-initial-steady-state-in.html)).
That is the single most relevant sentence in the literature for this project: a serious SFC engine
with multiple financial assets **gave up on stating the opening and ran to it instead**.

### Macroeconomic agent-based models — (I) and (III)

The benchmark AB-SFC model states the problem in the language of bias: initialization must
*"distribute initial endowments across agents in a way such that initial conditions do not entail any
a-priori bias for the phenomena one wants to analyze"*
([Caiani et al. 2016](https://www.sciencedirect.com/science/article/abs/pii/S0165188915301020)) —
which is Seed E1 in other words, arrived at independently.

And (III) is not hypothetical: it is the best-studied feature of this model class. Mark-0 has a
**phase diagram** with *"the generic existence of a phase transition between a 'good economy' where
unemployment is low, and a 'bad economy' where unemployment is high"*, with four identified phases —
full employment, full unemployment, residual unemployment, endogenous crises — and *"after a
transient that can be surprisingly long, the unemployment rate settles around a well-defined average
value"* ([Naumann-Woleske et al.](https://arxiv.org/pdf/2111.08654),
[Gualdi et al.](https://arxiv.org/pdf/1307.5319)). **The opening is a choice of basin.** Phoenix is
not suffering from a bad number; it is opening inside the dead phase, and no local fix to a stock
basis changes which phase a world lands in.

### Discrete-event simulation — (III), and the method Phoenix is missing

DES has a sixty-year literature on exactly one question: *when has the initial transient ended?*
*"The state in which a simulation is started causes the estimators for equilibrium measures to be
biased, and to reduce this bias, the collection of data is delayed until a so-called warm-up period
is completed."* The methods are mostly automatic: **MSER-5** *"gives the truncation point as the
point where the minimum standard error in the data occurs, when the data before that point is
ignored"* and was *"found to be the most effective and robust method"* against Schruben's test, batch
means and the rest; **Welch's** graphical procedure is the classic and is explicitly criticised
because *"the user decides the warm-up length by observing a graph… thus there is subjectivity"*
([Warwick AutoSimOA review](https://warwick.ac.uk/fac/soc/wbs/projects/autosimoa/current_work/website_warmup_methods_total_doc.pdf),
[Hoad et al., WSC](https://www.informs-sim.org/wsc11papers/044.pdf)).

The transferable idea is not "run for a while". It is: **the length of the past is measured, not
chosen.** A number somebody picks is a shape; a truncation point a statistic selects is a resolution
tested by invariance, which is what Law 2 asks for.

### Earth-system models — the artifact, and the honesty about drift

Ocean models cannot state an initial state either. *"The spin-up timescale… is determined by the slow
processes in the deep ocean and is usually in the order of a few thousand years"*, and *"climate
drift can be caused by a not-yet-equilibrated ocean initial state"*, which post-processing *"has been
found to be ineffective in removing"*
([Yool et al., UKESM1 spin-up](https://agupubs.onlinelibrary.wiley.com/doi/full/10.1029/2019MS001933),
[JAS drift study](https://journals.ametsoc.org/view/journals/atsc/70/10/jas-d-13-0149.1.pdf)). Two
practices follow, and both are structural rather than numerical: the spin-up is run **once** and its
end state is written as a **restart file** that every subsequent experiment opens from; and when
spin-up is too expensive, people do not shorten it, they **accelerate the convergence**
([sequence acceleration](https://www.science.org/doi/10.1126/sciadv.adn2839), Newton–Krylov methods).
The first of those is the architectural point Phoenix is missing: **the world should be an artifact,
not a function called at t=0.**

### Microsimulation — the road not to take

Population synthesis is the closest thing to what Phoenix's seed does today, and it is a warning.
The two dominant families are **iterative proportional fitting** — *"combines disaggregated data with
aggregated data… altering the joint distribution to fit the marginal distribution"* — and
**combinatorial optimisation**
([IPF review](https://www.sciencedirect.com/science/article/pii/S2352146516306925),
[IPF vs simulated annealing](https://www.sciencedirect.com/science/article/abs/pii/S0198971517301382)).
Both *fit the population to observed margins*. That is exactly Seed B5 and C5's forbid — no observed
real-world ratio copied in, and the sector's opening sheet is a read of its members, never a target
they were fitted to. So Phoenix may take the **vocabulary** (a synthetic population is drawn, not
stated) and must refuse the **method**.

### Procedural world generation — the construction that matches the owner's words

Dwarf Fortress does literally what was asked. It generates terrain, places civilizations, and then
*"generates a history for that world, tracking civilizations, sites, populations, and other events"*
over histories up to two thousand years long; the result is browsable in **Legends mode** —
*"centuries of simulated interactions yield complex, interconnected tales"*
([DF wiki: world generation](https://dwarffortresswiki.org/index.php/DF2014:World_generation)).

The part that matters most is the part nobody quotes. Worldgen is **generate-and-reject against
declared parameters**: *"if the world does not meet the requirements of any one rejection parameter
the world is rejected and re-randomised"*, and there is a switch, `LOG_MAP_REJECTS`, that writes
*"a file called map_rejection_log.txt where every time a world is rejected the reason will be
logged"* ([DF wiki: world rejection](https://dwarffortresswiki.org/index.php/v0.31:World_rejection)).

That is a design Phoenix can adopt wholesale, and it is legally clean in a way calibration is not:
**rejection discards a world, it never adjusts one.** No number inside an accepted world was touched
by the criterion, so nothing is fitted and no outcome is seeded (E1). And the rejection log is
precisely the artifact this project keeps wishing it had: a named reason, every time, for why a world
could not be born.

### Event sourcing — the mechanism, already in the kernel

*"The state of a system is not stored as a current snapshot but as a sequence of events that lead to
the current state… current state is computed by replay"*, with **snapshots** used *"when the number
of events that need to be replayed to restore the state of an aggregate need to be reduced"*
([Kurrent](https://www.kurrent.io/blog/snapshots-in-event-sourcing/),
[arc42](https://quality.arc42.org/approaches/event-sourcing)). Blockchains are the industrial case:
a genesis block with pre-funded allocations, and replay from genesis to rebuild state.

Phoenix is *already* an event-sourced system — the ledger is the log, settlement is the reducer, the
register is the projection — with **one exception carved out for the seed**. The proposal below is
just: close the exception. Note also the blockchain case is the counter-example that proves the
point: a genesis `alloc` block is exactly Phoenix's `endowMoney`, and it is tolerated there because a
chain has no notion of where coins "came from" — an economy does.

---

## 4. What may be borrowed, and what must be refused

| Practice | Field | Take it? | Why |
|---|---|---|---|
| Replay a log to build state | event sourcing | **yes** | the kernel is already this; the seed is the exception |
| Simulate the past instead of stating it | SFC toolkits, ESMs | **yes** | the only way (II) gets solved at all |
| Generate-and-reject with a logged reason | DF worldgen | **yes** | discards worlds, adjusts none — no fitting, no seeded outcome |
| Measure the transient's end (MSER-5) | DES | **yes** | turns the chronicle's length from a shape into a tested resolution |
| Write the end state as a reusable artifact | ESMs (restart files) | **yes** | makes an expensive past affordable, once per world |
| The admissible set is a polytope | SFC | **yes, as a check** | landing in a body is a constraint; solving for a point is not |
| Fit to observed margins (IPF, CO) | microsimulation | **no** | Seed B5, C5: a target the members were fitted to |
| Solve for a steady state | DSGE, SFC calibration | **no** | Seed D4: the seed is a fixed point of nothing |
| Nudge toward targets during spin-up | data assimilation | **no** | that is a bound with a nicer name |

---

## 5. The proposal: THE CHRONICLE

**A world is generated once, lives a past, is accepted or rejected against declared criteria, and is
then saved. Runs open from the saved world.** Four stages, one of which already exists.

### 5.1 The world is an artifact, not a function

Today `assemble()` builds a world by calling seed modules at t=0, every run, and the opening IS the
seed. Instead:

```
  draw        → the physical world and the primitives           (declared, small, auditable)
  chronicle   → N periods of the engine's own period loop       (the past; produces the ledger)
  accept      → the census of an existing world, or REJECT      (logged reason, re-draw)
  snapshot    → the accepted world, written as an artifact      (restart file; runs open here)
```

`period(0)` is the end of the chronicle, not its start. The calendar's epoch moves back by the
chronicle's length, so **the past has real dates**: a bond issued in chronicle period 40 has that
issue date for ever, and its accrual at the opening is a read of it.

### 5.2 What the seed is allowed to state, after this

Only what cannot have been made by an instruction: **land and places; people (as cells, with their
weights); the technologies (recipes, lead times, plant lives); the primitives (preferences,
policies); and the constitutional parties** — a state, a central bank, and the first bank, because a
banking system cannot bootstrap itself from nothing inside its own rules.

Everything else — money, deposits, loans, bonds, shares, inventory, plant, employment, contracts —
arrives the way it arrives in the world: somebody issued it to somebody, both legs, on a date.

Deleted: `endowMoney`, `endowUnits`, `prices.write` from `SeedContext`, `seed.openingPrice.*`,
`seed.openingWage`, `seed.openingYield`, `seed.openingRate`, `seed.households.openingHoldingShare`,
`PUBLIC_AT_THE_OPENING`, `SEED_STOCK_BASIS`. That is nearly all of item 22a, reached from the other
side: 22a asks the seed to claim less; this removes the doors through which it claims.

### 5.3 Two sources for the chronicle, in this order

**(a) Scripted past (phase 1).** Dated `InstructionDraft`s, settled through the ordinary path. Their
grammar is restricted, and the restriction is the whole of what keeps them legal: a chronicle
instruction may create a stock (issue, buy, hire, lend, deliver) and **may never write a price, a
rate, a mark or a decision**. What something cost in the past is what that instruction says; what
anything is worth at the opening is cleared. This is an endowment expressed as a flow, and it is
already enough to fix (I) and (II) completely.

**(b) Lived past (phase 2).** The engine's own period loop, with the modules deciding, replacing the
scripted events module by module. A module graduates when the world can produce that behaviour
itself — so **the migration is a measurement of whether a mechanism works**, which is the thing this
project keeps discovering three items too late.

Phase 2 is where (III) is actually solved, and it cannot be rushed: a lived past through today's
goods chain produces today's dead world. That is not an argument against it; it is the argument for
doing (a) first, because (a) gives every party a history and an outlook, and several of the
mechanisms that look broken may simply have been blind.

### 5.4 Acceptance: the census of an existing world

A candidate world is accepted only if it looks like one that has been running. Properties, never
values — *"every market has traded"*, never *"unemployment is five per cent"*:

1. every market has at least one print that came from a session;
2. every living party has at least one outlook;
3. every firm has produced, sold and been paid, in different periods;
4. every bank has lent and has been repaid;
5. every instrument kind that can exist, exists, and is held by somebody who chose to hold it;
6. the maturity profile spans more than one period, and issue dates are dispersed;
7. ages are dispersed: no cohort is the same age as the world;
8. the audit is green (Seed A2), and every balance sheet closes (C1);
9. nothing in the register is younger than the world.

A world failing any of them is **rejected, re-drawn from the next seed value, and the reason is
logged** — `docs/` gets a rejection log like DF's, and it is the most valuable diagnostic in the
project: a criterion that fails for every seed is a missing mechanism with a name.

### 5.5 The chronicle's length is measured, not chosen

Run the chronicle long, record a small set of named series (money per member, the going wage, the
count of living parties, sessions cleared, the credit stock), and take the truncation point from
**MSER-5** rather than from a preference. The chronicle length that results is a RESOLUTION (Law 2),
and it is testable the way a resolution must be: double it and the opening's properties must not
move.

### 5.6 Cost, and why 0g was worth doing

At `41d7e66` the first rung costs **554 ms/period** (it was 1354 this morning). A 200-period
chronicle is therefore about **two minutes** at the scale model, and the snapshot means it is paid
once per world rather than once per run. At the third rung it is hours — which is what the restart
file is for, and what 0g.3 and 0g.11 are for.

### 5.7 Two consequences worth having

**Legends mode, free.** The chronicle is a journal, and the observer surface already renders
journals. "Peek inside an existing world" becomes literal: you can read how this world's firms were
founded, who lent to whom, which bank failed in chronicle period 63.

**Every test gets a real world.** `test/rig.ts` builds scale models by hand today. With a snapshot
format, a rig is a small accepted world, and a test asks the chronicle for "a mill that has traded
for a year" instead of constructing one.

---

## 6. What this predicts will break

Stated in advance, so that when it happens it is a confirmation rather than a surprise.

- **The first chronicle will not be accepted.** Criterion 3 (every firm has produced, sold and been
  paid) is 12c.3 and will fail on the first try. That is the point: the rejection log names it.
- **Money will be hard to create legally.** Every deposit must come from a bank's lending or a
  central bank's purchase. A chronicle that cannot create the money stock this world needs is telling
  us something true about the mechanism, and it will be uncomfortable.
- **Period 0 will move.** Everything measured before will be measured against a different world, and
  the comparison is not meaningful. Findings recorded against the old opening must be re-read, not
  carried over.
- **The audit at period 0 becomes a real test.** It is currently a test of a hand-made balance sheet.
  Against a replayed past it is a test of settlement, revaluation and the audit together.

---

## 7. The items

Inserted before 22a, which they largely absorb; after item 21's stops, because a chronicle cannot run
through a world that throws.

- **22b.1** `world/chronicle.ts`: dated replay through ordinary settlement; the epoch shift so
  `period(0)` is the end of the past; the grammar guard (a chronicle instruction may not write a
  price, a rate or a mark); determinism from one seed value.
- **22b.2** `check:opening`: the census of §5.4 as nine named checks, red until it passes, with the
  rejection log and its reason.
- **22b.3** Money and the sovereign through the chronicle: reserves issued, bills sold at auction on
  their own dates, the central bank's holding bought rather than endowed. Deletes `endowMoney`.
- **22b.4** Firms, plant and inventory through the chronicle: plant bought from its maker on its
  vintage date, inventory bought up the chain from somebody who made it. Deletes `endowUnits` and
  `SEED_STOCK_BASIS` — and this is where 12c.3 is answered, because a firm cannot buy eleven periods
  of world demand from anybody.
- **22b.5** Households, employment and savings through the chronicle: hired, paid, banked, invested.
  This is what gives cells the income history their outlooks and their votes are made of.
- **22b.6** Delete the opening prints (22a.1): by now every market has traded in the chronicle.
- **22b.7** The snapshot: a world written as an artifact and opened by `assemble()`; the rig's scale
  models become small accepted worlds.
- **22b.8** MSER-5 on the named series; the chronicle length becomes a tested resolution; doubling it
  must not move the opening's properties.
- **22b.9** Phase 2's first module: warm-up periods replacing scripted events wherever the mechanism
  can produce them, one module at a time, each with its record.

**Exit.** `grep -rn "endowMoney\|endowUnits\|openingPrice\|prices\.write" src/` returns nothing;
`check:opening` is green; the census of period 0 is indistinguishable in KIND from the census of
period 100; and the rejection log is empty for the chosen seed.

---

## Sources

- [Volume of the steady-state space of financial flows in a monetary SFC model](https://arxiv.org/pdf/1601.00822)
- [Bond Economics — Calculating an initial steady state in `sfc_models`](http://www.bondeconomics.com/2017/09/calculating-initial-steady-state-in.html)
- [Caiani, Godin, Caverzasi, Gallegati, Kinsella, Stiglitz — Agent based-stock flow consistent macroeconomics: towards a benchmark model (JEDC 2016)](https://www.sciencedirect.com/science/article/abs/pii/S0165188915301020)
- [Naumann-Woleske, Knicker, Benzaquen, Bouchaud — Exploration of the parameter space in macroeconomic ABMs](https://arxiv.org/pdf/2111.08654)
- [Gualdi, Tarzia, Zamponi, Bouchaud — Tipping points in macroeconomic agent-based models](https://arxiv.org/pdf/1307.5319)
- [Warwick AutoSimOA — review of warm-up methods for the initial transient](https://warwick.ac.uk/fac/soc/wbs/projects/autosimoa/current_work/website_warmup_methods_total_doc.pdf)
- [Hoad, Robinson, Davies — Implementing MSER-5 in commercial simulation software (WSC)](https://www.informs-sim.org/wsc11papers/044.pdf)
- [Yool et al. — Spin-up of UKESM1 for CMIP6](https://agupubs.onlinelibrary.wiley.com/doi/full/10.1029/2019MS001933)
- [Impact of climate drift on twenty-first-century projection](https://journals.ametsoc.org/view/journals/atsc/70/10/jas-d-13-0149.1.pdf)
- [Efficient spin-up of Earth System Models using sequence acceleration (Science Advances)](https://www.science.org/doi/10.1126/sciadv.adn2839)
- [Population synthesis using iterative proportional fitting: a review](https://www.sciencedirect.com/science/article/pii/S2352146516306925)
- [Comparison of IPF and simulated annealing as synthetic population generation techniques](https://www.sciencedirect.com/science/article/abs/pii/S0198971517301382)
- [Dwarf Fortress wiki — World generation](https://dwarffortresswiki.org/index.php/DF2014:World_generation)
- [Dwarf Fortress wiki — World rejection](https://dwarffortresswiki.org/index.php/v0.31:World_rejection)
- [Kurrent — Snapshots in event sourcing](https://www.kurrent.io/blog/snapshots-in-event-sourcing/)
- [arc42 quality model — Event sourcing](https://quality.arc42.org/approaches/event-sourcing)
