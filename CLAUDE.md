# Project Phoenix — standing rules

A simulated closed-circuit economy. Four documents, and no fifth:

- `docs/spec/PROJECT_PHOENIX.md` — the specification. The authority on every question of mechanism.
- `docs/IMPLEMENTATION.md` — the one ordered list of what is left, and where a finding lives.
- `docs/ARCHITECTURE.md` — the implementation decisions.
- `docs/COVERAGE.md` — one row per clause, and what is met.

When a question is not settled here, read the spec before deciding.

## The ambition

Every dollar and share has a named counterparty at every instant; every priced asset has a
_cleared_ price and shows it; nothing is bounded, plugged or invented; the audit is true.

## The 19 laws

1. **Reflect the real mechanism.** Real named counterparties, intermediaries, lags, fees, refusals,
   failures.
2. **Fewest primitives.** A declared number is exactly one of TECHNOLOGY, PREFERENCE, POLICY (the
   only primitives), RESOLUTION (tested by invariance), or SHAPE (a claim about the answer; the
   count must fall). A shape with a scheduled death is a PLACEHOLDER naming the mechanism it stands
   in for. Everything else — ownership, prices, quantities, shares, allocations — is an OUTCOME.
   Real-world primitives may be imported; real-world equilibria may not. A residual with no holder
   is a defect.
3. **Every price is cleared** from real supply meeting real demand. Yield, spread, DM, OAS and PE
   are derived _from_ price, never the mechanism. The only exception is an administered
   central-bank rate with a real quantity response booked on both balance sheets.
4. **One representation per real thing; every fact has exactly one writer.** Hunt parallel formulas,
   mirrored copies, two probabilities for one borrower, two index systems.
5. **Every flow has two sides**, both legs in the same pass, period and currency. A one-sided flow
   is a defect even when nothing fails.
6. **No bounds of any kind.** No cap, floor, ceiling, clamp, damper, rescale, "not less than zero".
   Only arithmetic impossibility. If a number explodes the compensating mechanism is missing: build
   it, and delete the bound in the same change.
7. **Tolerance is arithmetic dust**: `terms × ε × Σ|magnitudes|`, derived per check. Never a
   percentage, never widened. A check that only passes with a band is reporting a defect.
8. **Periodicity, price level and unit are part of the number.** Confirm the period at the writer
   and name it in the identifier. A displayed change with no history is a lie — show the level.
9. **Instruments are named as a market names them**: issuer+coupon+maturity, issuer+tenor, the
   issuer for a share. Internal ids are never display names and never groupings. Every priced asset
   shows its price; fixed income shows price _and_ derived spread.
10. **One ordered list, in order, one item at a time.** Finish before opening the next. A new idea is
    INSERTED at its dependency position, never appended — say where.
11. **Do not measure, evaluate or diagnose mid-build.** Incomplete-model checks are deliberately
    failing. A misbehaving number is not a work item; the missing mechanism is.
12. **Fix the cause, not the symptom.** A cause has one fix and it removes code. A fix that is a list
    — add a bound, a check, a flag — is a symptom patch.
13. **Never roll back.** A bad number is a finding. Only a change wrong on its own terms is undone.
14. **One bounded change per item**, with a commit saying what and why.
15. **Targeted-change test.** All DATA in a registry; all kind-varying BEHAVIOUR in a profile behind
    a dispatch table. No mechanism branches on industry, sector, entity type or product id.
16. **Brevity.** A comment says _why_. A stale comment is a defect.
17. **No forecast without a falsification test.**
18. **Performance work**: mechanisms, economics and boundaries never change; layout and traversal
    are free. Gate on behaviour, not bits.
19. **Read the source; do not re-derive it.** Never recompute, sum a copy, infer by subtraction, or
    re-price what a market printed. Beware wrong quantity (mark vs face), stale mirror, residual,
    re-derived price. Every deletion names the read that replaces it.

## The method

- Every requirement is a **REASON** (a cause a participant has), a **VERIFY** (a thing to MEASURE,
  never enforce), or a **FORBID** (a required absence). An outcome written as a rule is a defect.
- A VERIFY that fails is a finding about a mechanism, never a licence to adjust the number.
- A FORBID that holds is as valuable as a mechanism that works; it breaks silently — guard it.
- "MISSING" and "OUT OF SCOPE" are different answers. Never delete a clause to look better. If the
  model deliberately lacks something, the clause stays and says so, with the reason.
- **A system is something with its own required tree** — its own instrument, actor or mechanism that
  could be wholly absent. That is why a bank is three systems and derivatives are five. There are 49,
  plus two instrument contracts that a system cites rather than restating.
- **Update the spec in the same change as the thing it describes.** A stale specification is worse
  than none, because it is still trusted. Re-mark COVERAGE in the change that closes something, and
  **recount rather than adjust** any tally.
- The spec says what is TRUE, never what a change found or did. It is not a diary.
- **A rule that can be a check should be one.** Text is a reminder; a guard that throws is a rule.

## Conventions

- **Units.** Every quantity carries its unit — face, shares, physical units, contracts, dwellings,
  hours, floor area, and money **in each currency separately**. The only route from a quantity to a
  value is `quantity × price`.
- **Money naming.** A figure in its owner's own money, one whose currency is named beside it, one in
  the reporting numéraire, and one in some *other named party's* money are four different things, and
  the identifier says which. A currency that must be inferred is inferred wrong exactly when it
  matters.
- **History and lag.** A quantity whose newest entry is older than the reader expects must say so. A
  legitimate lag and an accidental staleness must be distinguishable **by the reader**, without
  reasoning about what ran when.
- **Keys.** A firm is its own identifier and its display name is never a key. An institution is its
  own identifier. A piece of paper is the instrument it **is** — the individual issue for credit and
  for a sovereign, the issuer for equity, the fund for a fund share. **There is no bucket.** A good
  is its sub-unit and a market in a good is (region, sub-unit). A contract is its own identifier, and
  what it is *on* is keyed the way that thing is keyed. A cell is what the registry DECLARES its key
  to be; every other named relationship it carries is a register row.
- **Populations.** A cell is **homogeneous**: what it holds is `weight × what one member holds`, so
  everything it holds is divisible by its weight and every movement is `weight × a per-member
  amount`. Every number a represented sector produces is `Σ f(xᵢ)·wᵢ`; a number of the form
  `f(Σ xᵢ·wᵢ)` is a decision taken at an average, and a defect wherever it appears.
- **Missing values.** Absent is **absent**, never zero. An unpriced instrument is *not priced* and
  whoever asked must handle that. **A price of zero is a price, and it propagates. Zero multiplies.**
  A displayed number that does not exist is shown as missing, never as a formatted default.

## Core structure

- **Money is an instrument** issued by a bank or central bank; an account is a holding of it. Price
  1 for money is the only hard-coded price.
- **The wire**: state changes only by a numbered, two-sided `Instruction` applied by settlement; all
  legs atomic (delivery-versus-payment); settlement is final; a fail is a recorded state.
- **Register**: holdings with lots and liens; both directions indexed; holdings sum to issued.
- **Parties** are named or **cells** (integer weight; holdings are TOTALS, `perMember` is a read;
  identity is a KEY on the kind's declared lattice; at most one live cell per key; weight changes
  only by entry, death, promotion, SPLIT and merge; a crossing is a READ; no mean).
- **Prices**: `(market, instrument, period)` prints with provenance; value is units × price at read;
  unpriced throws unless declared carried-at-cost; stale is visibly stale.
- **Clearing**: one solver over posted schedules; a bracket is never a print; trades are
  instructions.
- **One calendar**: 7-day period, cycles within; periodicities placed by date; day counts from dates.
- **Period loop**: ordered phases as data; a phase reading a not-yet-produced print throws.
- **Audit**: independent families; a violation names owner, size, period and citation; it never
  repairs; an unbuilt family reports "not built", never green.
- **Registry**: what the ids point at. A country has the money and a region is a place, so
  `currency_of(region)` reads THROUGH the country. A unit says what one of it is divided into. A
  party kind has a PROFILE the kernel asks rather than branching on. The count must fall, never rise.
- **Parameter register**: every behaviour-shaping number declared with kind, unit and owner;
  placeholders name their mechanism; the engine reads numbers only via `params`.
- **Ontology register** (`nouns.rs`, declared in `assembly.rs`): every store declared as
  `noun | working | physics`, and a NOUN names the plan item that gives it a kernel home. An
  undeclared store throws at the read. **A count of zero would be the measure switched off**: zero
  means nothing anybody DECLARED is homeless, not that nothing is missing.
- **Expectations**: every deciding party has its own outlook, formed adaptively from its own
  history; one PREFERENCE (memory); surprise is a recorded event; confidence is a read. No global
  expectation, no peeking at the period's own result. Outlooks **disagree**, and the disagreement is
  load-bearing: it is what gives a market two sides and what a shock transmits through.

## One system, one file

**The test is a sentence: to change how CDS works I change `mechanisms/cds.rs`, and that is it.**

- A **module** is `src/mechanisms/<system>.rs` — one spec system or instrument family — and it holds
  that system's arithmetic, its `Mechanism`, its `Participant`s and its audit contributions.
  Behaviour is never declared outside the system it belongs to.
- **Two contact points and no third**: the module file, and one registration line in `systems.rs`,
  because the kernel has to learn the system exists.
- The **kernel** (`calendar registry parties register instruments ledger prices clearing journal
  audit stores world`, assembled in `assembly.rs`) owns every store and the period loop. It offers
  `ParticipantView` and `MechanismContext` and ASKS; it never holds a system's behaviour.
- A module reaches the kernel **only** through those two doors. It never imports another module and
  never imports `world.rs`. It never writes the register, a print or a weight: settlement, markets
  and the cell events are the one writer of each.
- Adding a system is one module plus one item. Replacing a system is replacing its module.
- Kinds are registered at assembly; the kernel asks a kind's profile, never branches on its id.

## Comments

- A comment says **why**, in one or two sentences. If the code says it, do not repeat it.
- **No references out**: not to a document, an item, a finding, a clause number or a law. The one
  exception is a module's `@spec` header block, which `phoenix-check` keeps true.
- **No history.** Not what the code used to be, not which defect produced it, not what another
  language cost. A reader who wants the past reads the diff.
- A stale comment is a defect, and so is a long one justifying a bound.

## Error discipline

- **Contract violations throw** `PhoenixError` with a citation, at the site, never caught in the
  engine: currency or unit mismatch, one-sided leg, moving encumbered units, unpriced read, missing
  period or periodicity, NaN/Infinity, phase ordering, a weight written outside the five events.
- **Invariant violations are audit findings**: reported with owner and size; never thrown, never
  repaired.
- No `?? 0`, no `|| 0`, no numeric defaults, no optional-means-unset numbers. Missing is `Missing`.
- No `.min(`, `.max(`, `.clamp(` anywhere in the engine. No numeric literals in a field position
  outside `ids`, `params`, `calendar` and tests (except 0, 1, -1, 2). No clock, no `rand`, no
  `println!` in the engine. No kind branches in mechanisms. Exhaustive matches.
- `tools/phoenix-check` is the writer of the exemption list, not this file.

## Testing

**No test is ever run against a test world.** A test exists at exactly one of two levels:

- **COMPILE LEVEL** — the type refuses the defect, so there is nothing to assert. A field that does
  not exist cannot be written; a `match` with no arm does not build; an `Option` forces every caller
  to say what it does about `None`. The strongest kind, and it costs nothing to run.
- **LOGIC LEVEL** — a pure function over values it is handed. `waterfall(has, &claims)`,
  `residual(assets, debt)`, `offer(seller, buyer, amount, …)`. No store, no party, no holding, no
  print: values in, values out, and the assertion is arithmetic or a stated refusal.

A test that BUILDS a world — parties, instruments, holdings, prints, a wire — and asserts on what
happens in it is not a test. **It is a second world**: an outcome nobody cleared, arranged by the
same hand that wrote the code it checks, passing for exactly as long as the arrangement holds.

**A question about a world is answered against the real one, once it exists.** Until then it is
written down in the plan as a measurement to take, never as a fixture that makes it look answered.

## Prohibitions (abridged — read Appendix B before touching an area)

**Money and ownership**: no money without an issuer; two currencies never added; no holding without
holder or issuer; no move without a two-sided numbered instruction; no silent overdraft; no
conversion at the ledger boundary or without a counterparty; the numéraire is not where value lives;
no short without a borrow; no collateral counted twice; no residual with no holder.

**Prices**: no price-taker of an unproduced price; no buyer of last resort; no demand added to
clear; price never from yield, spread, multiple, DCF or target; no written price path; no posted
benchmark; no index that inputs to its constituents and no stored index level; no spread on a mid
and no spread table; no derivative on an uncleared price; no parity-formula forward; no free
arbitrage and no unlimited arbitrageur.

**Bounds**: no cap, floor or clamp; no percentage tolerance; no fixed recovery, discount rate or
constant NAV; no enforced convergence; no investment rate, earnings path, unemployment rate, birth
rate, hazard-rate default, exogenous trade or capital-flow series.

**Institutions**: nothing immortal — firm, fund, bank (liquidity _and_ solvency), clearing house,
sovereign in foreign money; no death without a destination; no constant population; no central-bank
overdraft for the treasury; no forced buyer; LOLR has all four classical conditions; no unlimited
exposure or infinite balance sheet; no netting across counterparties; no margin that is only a
number; no leverage without a lender; capital is never a pot.

**Structure**: no representative agent where decisions are thresholds; no stored aggregate; no
stored value beside units; no pool without loans to named borrowers; no liability without
beneficiaries; no income without cash received; no instant settlement by construction; no employment
without an employer; no costless transport; no negative inventory and no inventory above cost for a
non-dealer; one cost in one place, one writer, one PD model per borrower, one payment convention; no
kind branch; no imported equilibrium and no seeded outcome; no decision at an average; a weight is a
count.

**Observer**: no observer sees private state; no privileged actor; no display-only number; no
scripted narrative; **no surface that changes the model**.

**Expectations and polity**: no global or model-forecast expectation; no sentiment coefficient; no
vote from an aggregate; no turnout, swing or bloc; no policy set directly or by path; parliament
never sets a price, a quantity, an outcome, or the central-bank rate.

**Method**: the audit never repairs; no forecast without its killer; no clause deleted to look
better.

## Sequencing

money + settlement + calendar → register + clearing (cells, DvP, value, parameter register) →
sovereign funding constraint → firm cost base + households + labour + outlooks → loss is an event →
loans are rows → forced seller + nothing immortal + estate → redeemable claims → equity + dealers
with inventory → cost of capital → bank capital raisable → currency layer + benchmarks + second
opinion → reporting and estimates → employment, housing, securitisation, corporate control →
polity → the recipe → then, and only then, measure.

## Working here

- **The engine is Rust** (`packages/kernel-rs`, one crate, tests inline as `#[cfg(test)] mod tests`).
  `tools/` is TypeScript that reads the documents: strict `tsc`, node's own test runner via `tsx`.
  `tools/phoenix-check` is the laws as a check, and has its own tests.
- **`npm run check`** at the END of an item: `check:laws` + `check:tests` + `check:types` +
  `check:tools` + `check:existence`. All green or the item is not done. The suite is a measurement
  and measurements come last.
- **`npm run world:runs` must be green before any commit.** It steps the assembled world four
  periods and asserts only that nothing throws, so it is not a measurement and Law 11 does not hold
  it back. It is not a test, which is precisely why it catches what tests do not.
- **A defect spread over the whole tree lands as a RATCHET, not a red gate.** The count is recorded
  in `phoenix-check`, which refuses to let it rise *and* refuses an allowance nobody lowered. It
  bites the day it lands; at zero the row is deleted and the rule is absolute. Not a bound: Law 6 is
  about numbers the world decides.
- **Take the first open item in `docs/IMPLEMENTATION.md`.** Read its section before writing code.
  Tick steps as they close. One item, one commit, carrying its COVERAGE re-mark.
- **When an item closes, delete its section.** Done is done: what it produced is the code, and a
  reader who wants to know what the world does reads the world.
- **BUILD FORWARD. EVERY FINDING GOES IN `docs/IMPLEMENTATION.md`. TESTS RUN AT THE END.**
  1. The work is the next item. A session implements; it does not go bug-hunting.
  2. Every finding is written down — a red test, a number that looks wrong, an audit family that
     fires, a mechanism that never runs, a world that stops — with what was measured and where. The
     item then carries on from the step it was on. A finding not written down is lost.
  3. Tests are written as the item goes and run when it is complete. A suite run mid-item measures a
     half-built world and reports the half that is missing.

  The exception is a violation that **stops the build** — an impossible quantity, a one-sided flow, a
  fact with two writers. Fix what is impossible where it is; record what is merely improbable.
- **A `done` row and a `MET` mark are CLAIMS**, and `npm run check:existence` is what checks them. A
  system with no clause MET is an ABSENT SECTOR. **A missing sector is an ITEM, not a finding**: "do
  not chase a finding" is right for a defect and wrong for an absence.
- Update `docs/ARCHITECTURE.md` in the same change as any structural decision.
- Ask the owner only for decisions the spec explicitly reserves; everything else is derived from the
  spec and stated in the commit.
- Deployment: GitHub Pages on `main`; APK via `android.yml` (Capacitor); target Pixel 11 Pro XL.
