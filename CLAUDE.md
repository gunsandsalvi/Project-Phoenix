# Project Phoenix — standing rules (always in context)

Phoenix is a simulated closed-circuit economy. The full specification is `docs/spec/PROJECT_PHOENIX.md`
(5,248 lines; 47 systems, 2 instrument contracts, 17 mechanisms). This file is its digest. When a
question is not settled here, **read the spec section** before deciding; cite it as `System Node`
(e.g. `Money C2.a`, `XI-15`, `Law 6`). Implementation decisions are in `docs/ARCHITECTURE.md`.
Work order is `docs/WORKLIST.md`; outcomes go in `docs/RECORD.md`; requirement status in
`docs/COVERAGE.md`.

## The single ambition
A closed circuit: every dollar and share has a named counterparty at every instant; every priced
asset has a *cleared* price and shows it; nothing is bounded, plugged or invented; the audit is true.

## The 19 laws (Part I)
1. **Reflect the real mechanism.** Real named counterparties, intermediaries, lags, fees, refusals, failures.
2. **Fewest primitives.** A declared number is exactly one of: TECHNOLOGY, PREFERENCE, POLICY (the only
   primitives), RESOLUTION (tested by invariance), or SHAPE (a claim about the answer; count must fall).
   A shape with a scheduled death is a PLACEHOLDER naming the mechanism it stands in for. Everything
   else (ownership, prices, quantities, shares, allocations) is an OUTCOME. Real-world primitives may be
   imported; real-world equilibria may not. A residual with no holder is a defect.
3. **Every price is cleared** from real supply meeting real demand. Yield/spread/DM/OAS/PE are
   *derived from* price, never the mechanism. Only exception: administered central-bank rates with a
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
   priced asset shows its price; fixed income shows price *and* derived spread.
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
16. **Brevity.** A comment says *why*; a stale comment is a defect. The record is a ledger of outcomes.
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
  expectation; no peeking at the period's own result.

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
Institutions: nothing immortal (firm, fund, bank — liquidity *and* solvency —, clearing house,
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
→ 11 bank capital raisable → 12 currency layer XI-12 + benchmarks XI-7 + second opinion XI-13 →
13 employment XI-10, housing, securitisation XI-11, corporate control → 14 polity XI-17 →
15 the recipe → then, and only then, measure (Part XII).

## Working here
- Toolchain: TypeScript strict, npm workspaces (`packages/engine`, `packages/app`), Vitest,
  fast-check, ESLint (custom rules in `tools/eslint-rules`), Vite, Capacitor for Android.
- Run `npm run check` (lint + typecheck + tests + spec citations) before every commit. All green or
  the item is not done.
- Take the first open item in `docs/WORKLIST.md`. One item, one commit. Write the RECORD entry and
  re-mark COVERAGE in the same commit.
- Update `docs/ARCHITECTURE.md` in the same change as any structural decision.
- Ask the owner only for decisions the spec explicitly reserves (e.g. §45 A4 inspector vs
  participant surface); everything else is derived from the spec and stated in the record.
- Deployment: GitHub Pages on `main`; APK via `android.yml` (Capacitor); target Pixel 11 Pro XL.
