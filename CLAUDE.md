# Project Phoenix — standing rules (always in context)

Phoenix is a simulated closed-circuit economy. The full specification is `docs/spec/PROJECT_PHOENIX.md`
(5,428 lines; 48 systems, 2 instrument contracts, 17 mechanisms). This file is its digest. When a
question is not settled here, **read the spec section** before deciding; cite it as `System Node`
(e.g. `Money C2.a`, `XI-15`, `Law 6`). Implementation decisions are in `docs/ARCHITECTURE.md`.
Work order is `docs/WORKLIST.md`; outcomes go in `docs/RECORD.md`; requirement status in
`docs/COVERAGE.md`.

## The single ambition

A closed circuit: every dollar and share has a named counterparty at every instant; every priced
asset has a _cleared_ price and shows it; nothing is bounded, plugged or invented; the audit is true.

## The 19 laws (Part I)

1. **Reflect the real mechanism.** Real named counterparties, intermediaries, lags, fees, refusals, failures.
2. **Fewest primitives.** A declared number is exactly one of: TECHNOLOGY, PREFERENCE, POLICY (the only
   primitives), RESOLUTION (tested by invariance), or SHAPE (a claim about the answer; count must fall).
   A shape with a scheduled death is a PLACEHOLDER naming the mechanism it stands in for. Everything
   else (ownership, prices, quantities, shares, allocations) is an OUTCOME. Real-world primitives may be
   imported; real-world equilibria may not. A residual with no holder is a defect.
3. **Every price is cleared** from real supply meeting real demand. Yield/spread/DM/OAS/PE are
   _derived from_ price, never the mechanism. Only exception: administered central-bank rates with a
   real quantity response booked on both balance sheets.
4. **"1$ is 1$".** One representation per real thing; every fact has exactly one writer. Hunt parallel
   formulas, mirrored copies, two probabilities for one borrower, two index systems.
5. **Every flow has two sides**, both legs in the same pass, same period, same currency. A one-sided
   flow is a defect even when nothing fails.
6. **No bounds of any kind.** No cap/floor/ceiling/clamp/damper/rescale/"not less than zero". Only
   arithmetic impossibility. If a number explodes, the compensating mechanism is missing — build it,
   and delete the bound in the same change.
7. **Tolerance is arithmetic dust**: `terms × ε × Σ|magnitudes|`, derived per check. Never a percentage.
   A check that only passes with a band is reporting a defect. Never widen a tolerance.
8. **Periodicity, price level and unit are part of the number.** Confirm the period at the writer,
   name it in the identifier. A displayed change with no history is a lie — show the level.
9. **Instruments are named as a market names them** (issuer+coupon+maturity; issuer+tenor; the
   issuer for a share). Internal ids are never display names and never groupings/buckets. Every
   priced asset shows its price; fixed income shows price _and_ derived spread.
10. **One ordered list, in order, one item at a time.** Finish (checks green, record written) before
    opening the next. A new idea is INSERTED at its dependency position, not appended; say where.
11. **Do not measure, evaluate or diagnose mid-build.** Incomplete-model checks are deliberately
    failing. A misbehaving number is not a work item; the missing mechanism is.
12. **Fix the cause, not the symptom.** A cause has one fix and it removes code. A fix that is a list
    (add a bound, a check, a flag) is a symptom patch.
13. **Never roll back.** A bad number is a finding, not a regression. Only a change wrong on its own
    terms is undone.
14. **One bounded change per item**, with a record saying what and why.
15. **Targeted-change test.** All DATA in a registry; all kind-varying BEHAVIOUR in a profile behind a
    dispatch table. No mechanism branches on industry/sector/entity type/product id.
16. **Brevity.** A comment says _why_; a stale comment is a defect. The record is a ledger of outcomes.
17. **No forecast without a falsification test.**
18. **Performance work**: mechanisms/economics/boundaries never change; layout/traversal is free.
    Gate on behaviour, not bits.
19. **Read the source; do not re-derive it.** Never recompute, sum a copy, infer by subtraction, or
    re-price what a market printed. Beware: wrong quantity (mark vs face), stale mirror, residual,
    re-derived price. Every deletion names the read that replaces it.

## The method (Part II)

- Every requirement is one of **REASON** (a cause a participant has), **VERIFY** (a thing to MEASURE,
  never enforce), **FORBID** (a required absence). An outcome written as a rule is a defect.
- A VERIFY that fails is a finding about a mechanism, never a licence to adjust the number.
- A FORBID that holds is as valuable as a mechanism that works; it breaks silently — guard it.
- "MISSING" and "OUT OF SCOPE" are different answers; never delete a clause to look better.

## Core structure (see ARCHITECTURE.md for detail)

- **Money is an instrument** issued by a bank/central bank; an account is a holding of it. Price 1 for
  money is the only hard-coded price.
- **The wire**: state changes only by a numbered, two-sided `Instruction` applied by settlement;
  all legs atomic (delivery-versus-payment); settlement is final; a fail is a recorded state.
- **Register**: holdings with lots (basis) and liens; both directions indexed; holdings sum to issued.
- **Parties** are named or **cells** (integer weight, per-member state, `integrate(f)`; no mean;
  partial events split; weight changes only by entry/death/promotion/split/merge).
- **Prices**: `(market, instrument, period)` prints with provenance; value = units × price at read;
  unpriced throws unless declared carried-at-cost; stale is visibly stale.
- **Clearing**: one solver over posted schedules; outcomes `cleared | noDemand | noSupply |
noOverlap | excessCommitted`; a bracket is never a print; trades are instructions.
- **One calendar**: 7-day period, cycles within; periodicities placed by date; day counts from dates.
- **Period loop**: ordered phases as data; a phase reading a not-yet-produced print throws.
- **Audit**: independent families; violation = owner + size + period + citation; never repairs;
  unbuilt family reports "not built", never green.
- **Registry + parameter register**: every behaviour-shaping number declared with kind, unit, owner;
  placeholders name their mechanism; engine reads numbers only via `params`.
- **Expectations** (§46, XI-16): every deciding party has its own outlook formed adaptively from its
  own history; one PREFERENCE (memory); surprise is a recorded event; confidence is a read; no global
  expectation; no peeking at the period's own result. Outlooks **disagree**, and the disagreement is
  load-bearing (§46 A3): it is what gives a market two sides and what a shock transmits through; a
  world where every party expected the same thing would trade once and stop.

## Kernel and modules (docs/ARCHITECTURE.md 4.9b, docs/PLAN.md)

- The **kernel** (`core calendar registry parties register ledger prices clearing journal audit
world`) owns every store and the period loop. It changes only by an inserted worklist item.
- A **module** (`src/mechanisms/<system>/`, `src/seeds/<name>.ts`) is one spec system, instrument
  family or seed: a `SystemModule` declaring kinds+profiles, units, params, phases (anchored to
  `corporateActions | markets | revaluation`), participants (per party kind, evaluated with that
  party's `ParticipantView`), audit contributions, and a seed contribution.
- A module reaches the kernel **only** through `ParticipantView`, `MechanismContext`, `SeedContext`.
  It never imports another module or `world/world.ts` (lint). It never writes the register, a print
  or a weight: settlement, markets and the cell events are the one writer of each.
- Adding a system = one module + one worklist item. Replacing a system = replacing its module.
- Kinds are registered at assembly; the kernel asks a kind's profile, never branches on its id.

## Error discipline

- **Contract violations throw** `PhoenixError` with a citation, at the site, never caught in the
  engine: currency/unit mismatch, one-sided leg, moving encumbered units, unpriced read, missing
  period/periodicity, NaN/Infinity, phase ordering, weight written outside the five events.
- **Invariant violations are audit findings**: reported with owner and size; never thrown, never
  repaired.
- No `?? 0`, no `|| 0`, no numeric defaults, no optional-means-unset numbers. Missing is `Missing`.
- No `Math.min/max/clamp` outside `core/num.ts`. No numeric literals outside `core/`, `registry/`,
  tests (except 0, 1, -1, 2). No `Date`, `Math.random`, `console` in the engine. No kind branches
  in mechanisms. Exhaustive switches with `assertNever`.
- Every module cites the clauses it implements with `@spec`; `npm run check:spec` must pass.

## Consolidated prohibitions (Appendix B, abridged — read the full list before touching an area)

Money/ownership: no money without an issuer; two currencies never added; no holding without holder
or issuer; no move without a two-sided numbered instruction; no silent overdraft; no conversion at the
ledger boundary or without a counterparty; the numéraire is not where value lives; no short without a
borrow; no collateral counted twice; no residual with no holder.
Prices: no price-taker of an unproduced price; no buyer of last resort; no demand added to clear;
price never from yield/spread/multiple/DCF/target; no written price path; no posted benchmark; no
index that inputs to its constituents, no stored index level; no spread on a mid, no spread table;
no derivative on an uncleared price; no parity-formula forward; no free arbitrage, no unlimited
arbitrageur.
Bounds: no cap/floor/clamp; no percentage tolerance; no fixed recovery/discount rate/constant NAV;
no enforced convergence; no investment rate, earnings path, unemployment rate, birth rate, hazard-rate
default, exogenous trade or capital-flow series.
Institutions: nothing immortal (firm, fund, bank — liquidity _and_ solvency —, clearing house,
sovereign in foreign money); no death without a destination; no constant population; no central-bank
overdraft for the treasury; no forced buyer; LOLR has all four classical conditions; no unlimited
exposure or infinite balance sheet; no netting across counterparties; no margin that is only a number;
no leverage without a lender; capital is never a pot.
Structure: no representative agent where decisions are thresholds; no stored aggregate; no stored
value beside units; no pool without loans to named borrowers; no liability without beneficiaries;
no income without cash received; no instant settlement by construction; no employment without an
employer; no costless transport; no negative inventory, no inventory above cost (non-dealer); one
cost in one place, one writer, one PD model per borrower, one payment convention; no kind branch;
no imported equilibrium, no seeded outcome; no decision at an average; a weight is a count.
Observer: no observer sees private state; no privileged actor; no display-only number; no scripted
narrative; **no surface that changes the model**.
Expectations/polity: no global or model-forecast expectation; no sentiment coefficient; no vote from
an aggregate; no turnout/swing/bloc; no policy set directly or by path; parliament never sets a
price, quantity, outcome, or the central-bank rate.
Method: the audit never repairs; no forecast without its killer; no clause deleted to look better.

## Sequencing (Part XIII) — the worklist follows this

1 money+settlement+calendar → 2 register+clearing (+cells XI-15, DvP XI-5, value XI-6, parameter
register XI-14) → 3 sovereign funding constraint XI-9 → 4 firm cost base + households + labour +
outlooks XI-16 → 5 loss is an event XI-1 → 6 loans are rows → 7 forced seller XI-2 + nothing immortal
XI-3 + estate XI-8 → 8 redeemable claims → 9 equity + dealers with inventory → 10 cost of capital XI-4
→ 11 bank capital raisable → 12 currency layer XI-12 + benchmarks XI-7 + second opinion XI-13 → 12a reporting and estimates §48 →
13 employment XI-10, housing, securitisation XI-11, corporate control → 14 polity XI-17 →
15 the recipe → then, and only then, measure (Part XII).

## Working here

- Toolchain: TypeScript strict, npm workspaces (`packages/engine`, `packages/app`), Vitest,
  fast-check, ESLint (custom rules in `tools/eslint-rules`), Vite, Capacitor for Android.
- Run `npm run check` (lint + typecheck + tests + spec citations + plan progress) at the END OF A
  MODULE, not mid-item. All green or the module is not done. Lint and typecheck are cheap and can
  run whenever; the suite is a measurement and measurements come last (Law 11).
- **A test never names a party.** This world's banks, firms, listings and funds are DRAWN (Seed
  B1.a): `firm.4` is not "the big farm", it is whatever the draw made it. A test asks the draw for
  a mill, a dealer, a listed line (`packages/engine/test/rig.ts`), and builds a SCALE MODEL of the
  world rather than the world — same modules, same laws, fewer of each.
- Take the first open item in `docs/WORKLIST.md`. Read `docs/PLAN.md` (the build loop, the module
  contract, the canonical period) and the item's own file `docs/plan/<item>.md` (design, steps,
  tests, exit criteria, guard) before writing code. Tick the item's steps (`- [x]`) as they close;
  run `npm run plan:progress` to recount the completion figure; delete the item file when the item
  closes. One item, one commit. Write the RECORD entry and re-mark COVERAGE in the same commit.
- **BUILD FORWARD. EVERY BUG GOES IN `docs/BUGS.md`. TESTS RUN AT THE END OF A MODULE.**
  The three rules the owner set, and they override the instinct to stop and fix:
  1. **The work is the next item.** A session implements; it does not go bug-hunting.
  2. **Every bug is written down — all of them.** A red test, a number that looks wrong, an audit
     family that fires, a mechanism that never runs, a world that stops: it goes in `docs/BUGS.md`
     with what was measured and where it was seen, and the item carries on from the step it was on.
     Nothing is chased, and nothing is silently dropped: a finding not written down is lost.
  3. **Tests are WRITTEN as the item goes and RUN when the module is complete.** A suite run
     mid-item measures a half-built world and reports the half that is missing (Law 11). Run it at
     the end, read what it says, and put what it says in the bug file before changing anything.

  The reason is Law 10 and Law 11 together: a misbehaving number is not a work item, the missing
  mechanism is — and most of what looks wrong in a world this unfinished is a mechanism nobody has
  built yet. The exception is a violation that STOPS THE BUILD: an impossible quantity, a one-sided
  flow, a fact with two writers. The engine will not run past those, so they are fixed where they
  are — and written down too. Fix what is impossible; record what is merely improbable.

  When the item closes, every finding in that file is POSITIONED — moved into the
  `docs/plan/<item>.md` of the item that should fix it, or inserted as its own item at its
  dependency position — and the record says where each landed. A finding leaves that file only by
  being placed. The file is temporary and goes when it is empty.
- Update `docs/ARCHITECTURE.md` in the same change as any structural decision.
- Ask the owner only for decisions the spec explicitly reserves (e.g. §45 A4 inspector vs
  participant surface); everything else is derived from the spec and stated in the record.
- Deployment: GitHub Pages on `main`; APK via `android.yml` (Capacitor); target Pixel 11 Pro XL.
