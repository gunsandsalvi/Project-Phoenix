# Project Phoenix — Implementation Plan

This is the plan for building the whole world. It is read at the start of every worklist item and
followed step by step. It does not replace the specification (`docs/spec/PROJECT_PHOENIX.md`), which
says what must exist; it says **how** we build it so that each system is a module that can be
replaced without touching the others.

Companion documents, and what each is for:

| Document | Holds | Changed when |
|---|---|---|
| `docs/spec/PROJECT_PHOENIX.md` | what the world is (never edited except Appendix C re-marks) | a clause is met or deliberately not met |
| `docs/ARCHITECTURE.md` | every implementation decision, with its clause | a structural decision is taken |
| `docs/PLAN.md` (this) | how each item is built; the module contract; dos and don'ts | the build method changes |
| `docs/WORKLIST.md` | the one ordered list | an item opens, closes or is inserted |
| `docs/RECORD.md` | outcomes per item | an item closes |
| `docs/COVERAGE.md` | requirement → MET / PARTIAL / MISSING / OUT OF SCOPE | a clause's status changes |
| `CLAUDE.md` | the digest always in context | the rules digest changes |

---

## 1. Objective and definition of done

**Objective.** A closed circuit (spec, "The single ambition"): every unit of money and every share
has a named counterparty at every instant; every priced asset has a cleared price and shows it;
nothing is bounded, plugged or invented; the audit is true. Forty-seven systems, two instrument
contracts and seventeen mechanisms, each present as the specification describes it, running on a
Pixel-class phone from the same code that runs on GitHub Pages.

**The world is done when all of the following hold:**

1. Every REASON, VERIFY and FORBID in `docs/COVERAGE.md` is MET or OUT OF SCOPE with a stated reason.
   No MISSING and no PARTIAL rows remain.
2. Every worklist item through 15 is closed with a record entry.
3. The parameter register reports **zero placeholders**; every remaining shape is justified in its
   `why` and its count has only fallen across the record.
4. All nine audit families are built and hold, to dust, over the long run with shocks (Part XII run
   ladder), from the seed, reproducibly.
5. The Part XII measurement programme has been run once, in full, and its findings are recorded as
   findings, not as tuning.
6. The app builds as an APK, installs on the target device and runs a working-length run.

Nothing before that is "the model working" (Law 11). Until then the audit families that fail are
expected to fail, and the record says which and why.

---

## 2. Principles that shape every item

These are the laws restated as build rules. The full laws are in `CLAUDE.md`; the spec is the
authority when the two differ.

1. **A mechanism produces outcomes; it never assigns them.** If a piece of code writes a price, a
   quantity, a share, a default or a rate, ask what mechanism should have produced it. Write that.
2. **Every number is declared** (Law 2, XI-14). Before writing a literal, decide which of the five
   kinds it is, declare it in the module's `params` with unit, owner and reason, and read it via
   `params.get`. A placeholder names the mechanism that deletes it and the worklist item that does.
3. **State moves only by settlement** (Money D4). A module drafts instructions; it never holds a
   register. If you need a write the context does not offer, that is a kernel item to insert, not a
   cast to get around.
4. **A party decides from its own view** (Observer A4, Expectations D1). Participants and deciders
   are written against `ParticipantView`. If a decision needs something the view does not have,
   either it is public state the kernel should publish, or the decision is reading something it
   must not.
5. **Failure is a state, not an exception.** A market can fail to clear, a payment can fail, a party
   can cease. Each is a recorded outcome the next mechanism reads. Exceptions are for contract
   violations only.
6. **A module never talks to a module.** It reads the kernel's state and writes through the kernel's
   doors. Two systems that need each other communicate through instruments, prints, register rows
   and journal events, never through imports.
7. **Behaviour by kind lives in a profile** (Law 15). A `switch` on a kind id inside a mechanism is
   a defect; the lint refuses it. Put the behaviour in the kind's profile or in a dispatch table
   keyed by the kind, registered by the module that owns the kind.
8. **Insert, never append** (Law 10). When an item needs something that does not exist, the missing
   thing becomes an item inserted before it, and the record says where it landed.
9. **Do not measure mid-build** (Law 11). Deterministic checks are gates. Numbers are not.
10. **Cite as you go.** Every module, profile, phase and family carries `@spec` tags; every closed
    clause is re-marked in COVERAGE in the same commit.

---

## 3. The architecture in one page

```
                     +---------------------------------------------------------------+
                     |                            KERNEL                             |
                     |  calendar · registry+params · parties · instruments           |
  modules declare -> |  register (one writer: settlement) · ledger · settlement      |
  kinds, units,      |  price store (one writer: markets) · valuation · solver       |
  params, phases,    |  journal · audit runner · corporate actions · revaluation     |
  participants,      |  cells (one writer of weights) · world (the period loop)      |
  families, seed     +------------------+-----------------+--------------------------+
                                        |                 |
              SeedContext (period 0)    |   MechanismContext (phases)   ParticipantView (per party)
                                        v                 v                      v
   +---------------------+   +---------------------+   +---------------------+   ...
   | seeds/foundation    |   | mechanisms/sovereign|   | mechanisms/firms    |
   | (endowments)        |   | (auction, curve,    |   | (cost base, output, |
   +---------------------+   |  programme)         |   |  hiring, invest)    |
                             +---------------------+   +---------------------+
```

**Data flow in a period.** Phases run in cycle order. A phase reads public state and the views it
needs, drafts instructions, and settles them; markets collect orders from participants (each with
its own view), clear, settle trades, and print; revaluation books mark changes; the audit closes the
period. Everything a later phase needs from an earlier one is in the kernel's stores: a print, a
holding, an issued amount, a journal event.

**What crosses module boundaries.** Only kernel objects: instruments (by id and public terms),
prints, register rows read through a view, journal events, parameters. Never a module's own
objects. If two modules seem to need a shared type, it is a kernel type or it is an instrument.

**Module-private state.** A module may keep state the kernel does not model yet (an outlook, an
order book, a workout in progress). Rules: one writer (the module), reproducible from the seed,
declared to the kernel through a state slot (`ctx.state`, kernel item inserted at worklist 4)
so it is snapshotted and reset with the world, and every change that matters to an observer is
journaled. Private state is never a second copy of something the register or price store holds.

---

## 4. The module contract, in detail

A module is a directory `packages/engine/src/mechanisms/<id>/` (or `src/seeds/<id>.ts`) whose
`index.ts` exports one `SystemModule`. Its fields, and what each must satisfy:

| Field | Rule |
|---|---|
| `id` | stable, lower-case, dotted or hyphenated; equals the directory name |
| `spec` | the spec system(s) it implements, e.g. `'Sovereign'`, `'Treasury Sovereign XI-9'` |
| `requires` | module ids it depends on; assembly orders by these; a cycle is a construction error |
| `instrumentKinds` | profiles for kinds this module owns; a kind is owned by exactly one module |
| `partyKinds` | profiles for party kinds it owns (a fund, an insurer, a clearing house) |
| `units` | physical units it introduces (tonnes, dwellings, hours) |
| `params` | every number it reads, with kind, unit, owner, why, and for a placeholder its death |
| `phases` | what it runs, each with a cycle and an anchor relative to `corporateActions`, `markets`, `revaluation` or another module's phase |
| `participants` | reasons to be in a market, per party kind; evaluated per party with that party's view |
| `families` | audit contributions: a check that reads the view and returns violations with owner and size |
| `seed` | opening state this module contributes; runs in assembly order |

**Lifecycle of a module in a run.** Assembly registers kinds, units and params; inserts phases;
registers participants and families; calls `seed`; the kernel states equity and seals. Then every
period: each phase runs at its cycle; each participant is evaluated per party at each market;
each family checks at the close.

**Anchors and cycles.** The kernel's phases are `corporateActions` (cycle 0), `markets` (cycle 1)
and `revaluation` (last cycle). A module phase names `{ before }` or `{ after }` one of these or
another module's phase, and a cycle. Cycles must be non-decreasing along the phase list; assembly
refuses otherwise. Section 8 gives the canonical map of where each system's phases sit.

**Instrument kind profile.** `pricing` (money | cleared | carriedAtCost), `liabilityOfIssuer`,
`unit(ccy)`, `validateTerms`, `displayName`, `due(instrument, period, calendar)`. The kernel never
looks inside terms. If a mechanism needs a term (a coupon rate, a maturity), the module that owns
the kind exports a typed accessor with a type guard, and the mechanism uses that.

**Party kind profile.** `representation` (named | cell) and `moneyIssuer` (null, or the overdraft
decision). Decision behaviour that varies by party kind (how a bank prices, how an insurer
quotes) lives in the module that owns the kind, as a dispatch table it registers, not in the kernel.

### 4.1 How to add a system

1. Read the spec section end to end, and its citations into Part XI. List every REASON, VERIFY and
   FORBID in COVERAGE.md and mark what the item will meet.
2. Decide the module's kinds, units and params. Write the profiles first; they are the contract.
3. Decide the phases and their anchors from Section 8. Decide the participants and which party
   kinds they are for.
4. Write the seed contribution, if the system needs opening state, with every endowment declared.
5. Write the audit contribution: the identities the system adds (a units identity, a zero-sum
   identity, a conservation).
6. Write tests in this order: profile validation; a single phase against a small assembled world;
   a year-long green run with the module assembled; property tests for any solver or allocation.
7. Cite, re-mark COVERAGE, write the record, commit.

### 4.2 How to replace a system

Replace the module directory. Keep the module id if the replacement owns the same kinds; otherwise
give it a new id and a data migration for any instruments of the old kinds (a re-kinding is an
instrument event with a cause, journaled). Nothing else changes: no other module imports it, and
the kernel learns the new behaviour through the profiles. The record says what replaced what and
why. A replaced module's tests are replaced with it; the kernel's tests do not change.

### 4.3 How to remove a system

Remove the module. Assembly refuses if another module `requires` it. Instruments of its kinds must
have ceased or been re-kinded first. COVERAGE rows for its clauses go back to MISSING or, with a
reason, OUT OF SCOPE (never deleted: Appendix C).

### 4.4 When the kernel may change

Only for a Part III clause (Money, Register, Clearing, Audit, Seed, Currency), or for a door a
module needs that no context offers (a module-state slot, a new corporate-action vocabulary word, a
new leg kind). Each such change is its own inserted worklist item with a record entry, and it never
branches on a module's kind. The list of single writers is fixed: settlement writes holdings and
issued amounts; markets write prints; the cell events write weights; the seed writes endowments
before the seal; nothing else.

---

## 5. The build loop for one item

Each item follows the same loop. Do not skip steps; do not reorder them.

1. **Open.** Take the first open item. Read its spec sections in full, and every Part XI
   mechanism they cite. Read `docs/ARCHITECTURE.md` sections the item touches.
2. **Split.** If the item is coarse (13 and later), split it into bounded sub-items, each one
   module or one kernel door, and insert them in dependency order. Record the split.
3. **Coverage first.** List the clauses the sub-item will meet. Anything it deliberately will not
   meet gets a PARTIAL row with what is missing and which item brings it.
4. **Profiles and params.** Write kinds, units, params. Every literal a decision. Every placeholder
   a scheduled death.
5. **Mechanism.** Phases, participants, seed. Draft instructions; never write stores. Decisions from
   views. Failure as outcomes.
6. **Audit.** Add the identities the system makes checkable. A family reports itself built only
   when it checks something real.
7. **Tests.** Unit tests on profiles; phase tests on a small assembled world; a year-long run that
   stays green (or whose expected failures are named in the test and the record); property tests
   for solvers; a determinism test when randomness is drawn.
8. **Gates.** `npm run check` green: lint, typecheck, tests, citations. `npm run coverage:spec`
   recounted. The app smoke test green if the surface changed.
9. **Record.** What, why, what was found, what was deleted (bounds and placeholders), the
   forecast with its killer if any.
10. **Commit.** One commit per sub-item, message says what and why. Push.

**Definition of done for an item:** steps 3 to 10 complete; no `TODO` in code (a TODO is a
worklist item or it is nothing); no PARTIAL row without a named item; no placeholder without a
named death.

---

## 6. Strategies that recur

### 6.1 Cells and weights (XI-15)

- Every household and small-firm quantity is per member. The cell's total is a read. A leg on a
  cell is per member at the struck weight (`cellSide`, `totalFor`).
- A partial event (some members hired, some default) splits the cell first, through
  `ctx.cells.split`. Never carry a fraction of a member. Never carry a headcount inside a cell.
- The key is registry data. Stratifying on a new relationship (a landlord, an employer) is a data
  change that lifts a row into the key, not a mechanism change. Declare it in the record.
- Resolution is measured: Part XII runs the same seed at 1x, 2x, 4x `cellsPerKey`.

### 6.2 Decisions and outlooks (XI-16, §46)

- Every decision reads the decider's own outlook. The outlook is module-private state of the
  expectations module (worklist 4), keyed by party and variable, formed from the party's own
  observations recorded as surprises. One preference: memory, drawn once at entry.
- The expectations module publishes nothing a decision can consult except through the party's own
  view: `view.outlook(variable)` is added to `ParticipantView` at worklist 4 as a kernel door that
  reads the module's slot. No global expectation exists anywhere.
- A decision is a function of (own state, own outlook, public prints, params). If it needs more, it
  is peeking.

### 6.3 Markets and participants (Clearing)

- A market is declared by the module that owns the instrument kind, opened when the instrument is
  registered (`ctx.openMarket`). One instrument, one market, one print per period.
- A participant declaration says why a party of a kind is in a market. Its orders come from the
  view. A dealer is a party of a kind with a limit, a funding cost and inventory; its width is a
  consequence. There is no residual absorber anywhere.
- Primary markets (an auction, a book-build) are the same solver with a fixed supply side: the
  issuer posts its size and a reservation; buyers post schedules. Failure is `noOverlap` and it has
  consequences the issuer's module handles next.

### 6.4 Failure, loss and the estate (XI-1, XI-2, XI-3, XI-8)

- A failed instruction is a recorded state. The payee's module reads it and decides (a missed
  coupon is a default event for the issuer; a failed trade leaves the seller its paper).
- A loss is an event with a date: a status written on a row, a provision booked, a write-off
  settled. Never a rate.
- Death: a party ceases through `ctx.cease(party, estate)`; the estate is a named party of kind
  `estate` (owned by the estate module) that holds everything the dead party held, sells it into
  the same markets over a stated programme, ranks claims by each instrument's stated seniority, and
  distributes by settlement. Every reference resolves through `parties.resolve`.
- Forced sales are orders posted by the party's own module in the next market session, with the
  reason (a margin call, a redemption, a withdrawn line, a mandate breach) journaled.

### 6.5 Time and ordering (Money G, Clearing F)

- Dates are placed by the calendar. A module never counts periods to place a payment.
- A phase reads prints of the current period only after the market has run; before that,
  `printOrThrow` throws `NotYetProduced`. If a phase needs a print from later in the period, it is
  anchored in the wrong place. Move it; never read the previous period silently.
- Within a period, the order is Section 8's. A new phase is anchored explicitly; "at the end" is not
  an anchor.

### 6.6 Currencies (Currency, Spot FX, XI-12)

- Until worklist 12, settlement refuses an instruction that touches an instrument in a money other
  than the party's home money. That guard is deleted in item 12 together with the FX revaluation
  that replaces it (Currency D2), in one change.
- After 12, every amount still carries its currency; conversion is a trade with a counterparty in
  the spot market; nothing converts at the ledger boundary.

### 6.7 Parameters and placeholders (Law 2, XI-14)

- Declare before use. `kind` is a decision: technology, preference, policy, resolution, shape,
  placeholder. Policy has an owner (parliament, central bank, standard-setter); the register
  prints it.
- The count of shapes and placeholders is on the inspector surface and in every audit report. It
  must fall. An item that raises it says why in the record.

### 6.8 Naming and identity (Law 9, Appendix A)

- Instrument ids are opaque; display names come from the kind's profile. A bond is issuer +
  coupon + maturity; a bill issuer + tenor; a share its issuer; a good its sub-unit; a market
  (region, sub-unit) or the instrument.
- There are no buckets. A book is keyed by the instrument bought.

---

## 7. The worklist in depth

Items 0, 1, 2 and 2a are closed (see `docs/RECORD.md`). For each open item: objective, spec
sections, modules, kernel doors, phases and anchors, participants, parameters, audit, seed, tests,
exit criteria, and the FORBIDs to guard. Sub-item splits are proposed; the split is confirmed when
the item opens (Law 11: do not plan a world that has not arrived).

### Item 3 — The sovereign's funding constraint (Treasury, Sovereign, XI-9)

**Objective.** A treasury that must raise before it spends, from an auction that can fail, into a
secondary market with a curve read from prints; a central bank that buys in the market at a size it
chooses; no overdraft anywhere.

**Spec.** §30 Treasury A–F; §8 Sovereign A, C, D, E, F, H; §31 Central Bank A, C, E; XI-9; Bond
N7, N9, N9.b; Money D3 (`no central-bank overdraft` is already a refused overdraft: the treasury's
bank is the central bank whose profile allows a reserve overdraft; item 3 changes the treasury's
account to a **refused** overdraft by giving the treasury its own account profile).

**Modules.** `mechanisms/treasury` (outlays, receipts, the programme, debt management),
`mechanisms/sovereign-auction` (the primary market: uniform-price sealed-bid with primary dealers'
obligation), `mechanisms/sovereign-curve` (the secondary market participants and the curve with
provenance), `mechanisms/central-bank-omo` (open-market purchases, remittance). `sovereign-instruments`
already exists.

**Kernel doors needed.** (a) A primary-market form of `runMarket` where the supply side is one
posted size at a reservation with the issuer's walk-away (Sovereign C2, C5, C7): extend the market
declaration with `primary: { issuer, size, reservation }` evaluated by the same solver. (b) Accrued
interest travelling with a trade (Bond N9.b): settlement takes `accruedPerUnit` on an asset leg
and moves it as a second money amount, clean price quoted, dirty settled. (c) A `curve` read on
`ParticipantView` for a stated benchmark family, with provenance per point. Each is an inserted
sub-item.

**Phases and anchors.** `treasury.programme` after `corporateActions` (size the auction from the
period's known outlays and redemptions, read from instruments' due actions and the mandate's
outlay programme); `treasury.outlays` after `treasury.programme` (transfers to cells and firms:
until item 4 exists, outlays are the standing mandate's transfers and public wages to household
cells, declared as policy params owned by `parliament` with a standing mandate); `treasury.receipts`
in the same cycle (taxes on bases that exist: until item 4, interest income only); the auction and
secondary market are markets, so they sit in `markets`; `centralBank.remittance` after
`revaluation` on the period the calendar places it; `centralBank.omo` before `markets` (posts its
quantity as a price-taker, Central Bank C3).

**Participants.** Banks (liquidity buffer: a preference derived from their deposit base, §11
A2.a: until the money market exists, a declared preference param), households cells (a saver's
substitution: until item 4, a declared preference), primary dealers (a party kind `primaryDealer`?
No: a bank with the obligation, as a policy param listing the obligated banks), the central bank
(quantity only).

**Parameters.** Policy: the standing mandate's outlay programme and tax rates (owner parliament;
XI-17 replaces the standing mandate at item 14), the treasury's buffer preference, the auction
calendar (policy), the obligated dealers' minimum bid share (policy). Deleted: the opening price
placeholder.

**Audit.** Treasury D6 (debt outstanding reconciles with issuance and redemption) as a `flows`
contribution; Sovereign E1.a as an `ownership` read.

**Seed.** The treasury opens with a maturity profile spread across lines (Seed C3.a) instead of one
line: several bond lines and bill lines seeded outstanding at opening prices, each a placeholder
until its first print.

**Tests.** An auction that fails when dealers are at their limit; a failed auction leaves the
treasury's account lower than planned and journaled; a refused overdraft on the treasury's account
fails an outlay; the curve's points carry provenance; accrued travels with a trade and no coupon is
a windfall; a year green.

**Exit.** Sovereign A–H MET except G (default; item 5/7); Treasury A–F MET; the opening-price
placeholder gone; Bond N9.b MET.

**Guard.** No forced buyer; no overdraft; no yield-derived price; no interpolated point read as a
trade.

### Item 4 — A firm's real cost base, households, labour, outlooks (Firm, Goods A–F, Labour A–D, Households A–E, Expectations, XI-16)

**Objective.** Firms that produce, employ and sell to households that earn, consume and save, with
every party deciding from its own outlook.

**Spec.** §32 Firm A–F; §37 Goods A–F; §39 Labour A–D; §41 Households A–E; §46 Expectations; XI-16;
XI-10 (first half: the employment register).

**Proposed split (confirm at open).**
- 4.1 Kernel door: module state slots (`ctx.state<T>(name)`) snapshotted with the world; `view.outlook`.
- 4.2 `mechanisms/expectations`: outlook per (party, variable), memory preference drawn at entry,
  surprise events, confidence read. Phase `expectations.form` first in cycle 0 (before any
  decision), `expectations.score` after `revaluation`.
- 4.3 `mechanisms/goods`: goods as instrument kinds with physical units (countable or not) held as
  inventory lots at cost; the recipe (Leontief, fixed coefficients) as technology params; a market
  per (region, sub-unit); inventory as a units identity in the `units` family; lower-of-cost-and-NRV
  as a revaluation rule in the goods kind profile (pricing `carriedAtCost` with a write-down event).
- 4.4 `mechanisms/labour`: the employment register as rows (firm, cell, wage, start, headcount);
  vacancies as posted bids; matching with highest bids first, pro rata at the margin (the solver,
  with hours as the unit); participation from the cell's own view; separations as split events.
- 4.5 `mechanisms/firms`: production decision from outlook and capacity; input purchases; wages
  settled to cells; invoice book (receivables as rows, item 13 trade credit reuses them); pricing
  decision; dividend decision; margin as a read.
- 4.6 `mechanisms/households`: consumption from own outlook, income, wealth, liquidity; allocation
  across goods; saving as the residual; the deposit / bill substitution as a participant in the
  bill market. Seed: household endowments from wages and saving replace the three shapes.

**Phases.** `expectations.form` first; `firms.decide` and `households.decide` in cycle 0 after
outlooks; `labour.match` before `markets` (a market in hours, cycle 1); goods markets in `markets`;
`firms.produce` after `markets` (consumes inputs, adds work in progress and finished lots) in
cycle 2; `firms.invoice` and `households.pay` cycle 2; `expectations.score` after `revaluation`.

**Kernel doors.** State slots; the `units` family contribution API is already there; a
`carriedAtCost` write-down needs a kernel event `writeDown` that moves equity through settlement
(an instruction with an asset leg to nobody is forbidden: a write-down is a valuation event, so it
is a revaluation-phase hook the goods kind's profile provides: `profile.revalue(lot, mark)`).
Insert as 4.1.

**Parameters.** Technology: recipes, lead times, useful lives. Preference: memory, risk aversion,
patience per cell (dispersed, drawn at entry). Policy: benefit replacement rate, tax rates (owner
parliament). Deleted: the seed's three shapes.

**Audit.** `units`: produced + opening = consumed + closing per good and location; employed +
unemployed + inactive = population (Labour B5). `flows`: receivables = payables (Trade Credit C4,
already checkable once invoices are rows).

**Tests.** A mean-preserving spread of cell incomes changes consumption and crossings while the
mean does not (Households A2.g); wages leave the firm's account and arrive at the cell; an input
shortage stops production rather than being smoothed; outlooks lag a turning point; a year green.

**Guard.** No investment rate, no earnings path, no unemployment rate, no representative agent, no
recipe as a value share, no global expectation, no decision at an average.

### Item 5 — A loss is an event (XI-1; Banks Lending E; Firm D; Firm Birth C)

**Objective.** Claims go unpaid as dated events with named consequences.

**Split.** 5.1 the default definition per instrument kind (`profile.default`: missed payment for a
sovereign, missed payment or breached covenant for a corporate); a failed coupon or maturity
instruction becomes a default event journaled by the kernel's corporate actions phase, read by the
issuer's module. 5.2 `mechanisms/credit-events`: status rows (performing, impaired, defaulted),
provisions as equity events booked by the holder, write-offs as instructions. 5.3 holders'
reactions as participants (a defaulted issuer's paper trades on its recovery expectation).

**Guard.** No loss rate; no hazard draw; no default that a firm's own actions could not have
prevented.

### Item 6 — Loans are rows (Banks Lending A–D, F; Banks Funding B; Money B3.a)

**Objective.** A loan is an instrument (kind `loan`, carried at cost, a row per (lender, borrower)),
written by creating a deposit, priced from the bank's own cost of funds, expected loss, capital and
operating cost; the borrower shops; the bank can decline.

**Modules.** `mechanisms/bank-lending` (loan kind; quotes as a dispatch on the bank's own state; the
credit decision replaces the refused-overdraft placeholder for customers); the money issuer profile
for `bank` moves its overdraft decision to this module's credit decision (a kernel profile field
becomes module-provided: insert a kernel sub-item that lets a module supply the overdraft decision
for a party kind).

**Audit.** `flows`: the loan book is the sum of rows (F1.a is then true by construction; F2 as a
check on new, amortised, prepaid, written off).

**Guard.** One default-probability model per borrower; no lending "out of" deposits; a bank that
never says no has no credit standard (declined volume visible).

### Item 7 — The forced seller, nothing is immortal, the estate (XI-2, XI-3, XI-8; Firm Birth D; Banks Capital C–E)

**Objective.** Every kind of party can cease; a cascade has a push, a place to terminate and a
distribution at the end.

**Split.** 7.1 `mechanisms/estate`: the estate party kind, the waterfall by stated seniority
(seniority is a term in each debt kind's profile), asset sales as orders into the existing markets
over a stated programme (policy), abandonment as an outcome, real-economy consequences (employees
released through the labour module's separation path via journal events it reads). 7.2 firm death:
triggers (cash, solvency) in `firms`; `ctx.cease(firm, estate)`. 7.3 bank resolution: valuation,
bail-in hierarchy (instrument kinds with ranks), acquirer bid or public path, deposit insurance per
member. 7.4 forced sales: margin-call, redemption, line-withdrawal and mandate-breach reasons posted
as orders; the "no floor at zero" guard on lines.

**Guard.** No party that cannot die; no death without a destination for everything; no formula
discount; no floor on available credit.

### Item 8 — Redeemable claims (Fund Shares)

**Objective.** Funds as parties with a share count, NAV as a read, subscriptions that buy and
redemptions that sell into markets that must clear; the money fund breaks the buck.

**Module.** `mechanisms/funds` (party kind `fund`, instrument kind `fund.share` whose price is the
NAV read, not a print; the ETF variant with a traded share and a NAV; the manager as a separate
party earning the fee).

**Guard.** No constant NAV; no fund that creates assets; no redemption rationed by the fund's
cash with the remainder dropped.

### Item 9 — Equity, and dealers that carry inventory (Equity, Dealer Desks)

**Objective.** Shares as a non-liability instrument with a vote; issuance and buyback as decisions
priced by the market; dealers with a limit, a funding cost and a skew, whose stepping back is what a
failed market is.

**Modules.** `mechanisms/equity` (kind `share`, `liabilityOfIssuer: false`, countable; dividend,
split, issuance, buyback as instructions), `mechanisms/dealers` (a desk as a party kind inside a
bank: inventory, cost of funds rent charged every period as an instruction to its bank, a limit,
quotes as schedules from its own state, interdealer market).

**Guard.** No spread on a mid; no stated spread table; no desk that cannot lose money; no price
from a multiple.

### Item 10 — The cost of capital (XI-4, Capital Programme)

**Objective.** A financial price changes a real decision through three joints: the bank's cost of
funds in the loan price, the firm's hurdle against its marginal debt and equity cost, the desk's
rent on inventory.

**Module.** `mechanisms/capital-programme` (plant as an instrument kind with vintages, kinds of
capital as units, commissioning lag, depreciation as one schedule charged in both places, the
investment decision from the firm's own outlook and hurdle, the capital-goods producers as sellers).

**Guard.** No investment rate; no cost of capital from the average coupon; plant that can be
priced only once its unit is defined.

### Item 11 — Bank capital that can be raised, and the money market with its corridor (Banks Funding, Banks Capital A–B, Money Market, Central Bank B–D)

**Objective.** Reserve positions redistributed by everyone else's payments, a money market that
clears after the flows with unsecured and secured books, a corridor with a floor and a
collateralised, penalised ceiling, and a bank that can raise equity and subordinated debt in
markets that can refuse.

**Modules.** `mechanisms/money-market` (overnight and term books; every bank posts a schedule from
its own position and cost of funds; the facility as a seat priced at the top of the corridor,
bounded by unencumbered eligible collateral; the reserve overdraft's placeholder deleted here),
`mechanisms/bank-capital` (requirements against risk weights, the buffer as the bank's choice,
distributions restricted near the line, issuance into the equity and credit markets).

**Phases.** `moneyMarket.clear` after every other market in the last cycle before revaluation:
the need is knowable only after the flows (§11 A3).

**Guard.** The policy rate never appears as a cleared rate; the LOLR has all four conditions; a
bank out of collateral cannot draw; a run is a wholesale phenomenon first.

### Item 12 — The currency layer, benchmarks, the second opinion (Currency, Spot FX, FX Forwards, Indices, Ratings, XI-12, XI-7, XI-13)

**Objective.** More than one currency; every pair clearing on its own flow; forwards carrying the
interest differential; one index system read from constituents; a transacted overnight benchmark;
ratings as named opinions that rules refer to.

**Split.** 12.1 kernel: FX revaluation into equity (Currency D2) and deletion of the
single-currency guard, in one change; a second region and currency in the seed. 12.2 `spot-fx`,
12.3 `fx-forwards` (derivative layer prerequisite: see item 13), 12.4 `indices` (a read, never
stored), 12.5 `benchmarks` (the overnight rate read from the money market's own prints), 12.6
`ratings` (assessors as parties; a rating from state, never from price; consumers: mandates,
capital charges, haircuts).

**Guard.** No vehicle currency by construction; no parity formula; no stored index level; no rating
that reads the spread.

### Item 13 — Employment's other half, housing, securitisation, corporate control, and the remaining systems

This item is a container to be split at open into the following modules, in this order (each a
bounded sub-item with its own record entry):

1. `derivative-layer` (contracts store: a kernel door for the zero-sum family; bilateral and
   cleared; a clearing house party kind with a waterfall; initial and variation margin as
   instructions with a cash test; capacity as admission).
2. `interest-rate-swaps`, `cds`, `fx-forwards` (from 12.3), `commodity-futures` (each answering the
   derivative contract its own way; a speculative participant required on both sides).
3. `commodities-spot`, `freight` (location as identity; routes as capacity that rations quantity).
4. `labour` second half (occupational mobility, quits, vacancy withdrawal), `housing` (dwellings
   as countable units in locations; mortgages as loan rows; foreclosure returns supply), `households`
   second half (life cycle: cohort splits by date; inheritance to a named heir cell).
5. `trade-credit` (terms as a price; lateness as a state; the supply-network chain), `small-business`
   (pool as cells; promotion to a named firm), `securitisation` (vehicle, tranches with attachment,
   waterfall of real losses).
6. `corporate-credit` (covenants, waivers, book-build, taps, restructuring), `short-term-debt`
   (rolls that can fail), `securities-lending`, `prime-brokerage`.
7. `insurers-pensions` (a liability with a schedule discounted at a market rate), `hedge-funds`,
   `private-equity`.
8. `mna` (bids, refusals, rivals, funding), `firm-birth` (entry as a consequence, opening size what
   founders can fund).
9. `cross-border` (the balance as a read; every flow with two named sides).

**Guard.** Each module's FORBIDs are in its spec section; the record for each names which it
guards and how.

### Item 14 — The polity (Polity, XI-17)

**Objective.** A parliament whose seat-weighted platform is the value of every fiscal and
regulatory policy primitive; cells vote from their own outlook; abstention is a decision.

**Module.** `mechanisms/polity` (platforms as data rows; the election placed by date; the
allotment rule as a policy primitive of the constitution; the mandate as a read; the treasury's
programme reads the mandate: the standing mandate declared at the seed in item 3 is deleted here).

**Guard.** No turnout, swing or bloc parameter; no policy set directly; the parliament never sets a
price, a quantity or the central bank's rate.

### Item 15 — The recipe (Goods A2)

**Objective.** Change the input-output relationships against a stable measurement, and compare.

### Item 16 — Measure (Part XII)

Only then: the invariant families over the run ladder; the VERIFY nodes as reads; the causal
chains named in Part XII; the population-resolution test; findings recorded as findings.

---

## 8. The canonical period

Where each system's phases sit, as the world fills in. Cycles are settlement cycles (five per
period). A module anchors to the nearest kernel phase and names its cycle; assembly enforces
monotonic cycles. This map is the plan; the assembled phase list is the fact (`world.phases`).

| Cycle | Order within cycle | Phase | Owner |
|---|---|---|---|
| 0 | 1 | `expectations.form` — every party's outlook from its history | expectations |
| 0 | 2 | `corporateActions` — coupons, maturities, dividends, amortisation | kernel |
| 0 | 3 | `credit.events` — missed payments become defaults; statuses written | credit-events |
| 0 | 4 | `margin.calls` — requirements re-measured against last close; calls issued | derivative-layer, prime-brokerage |
| 0 | 5 | `treasury.programme`, `treasury.outlays`, `treasury.receipts` | treasury |
| 0 | 6 | `firms.decide`, `households.decide`, `banks.decide`, `funds.decide`, `insurers.decide` — decisions from own views | each system |
| 1 | 1 | `labour.match` — the market in hours | labour |
| 1 | 2 | `markets` — goods, freight, housing, primary (auctions, books), secondary (bonds, shares, funds), FX, derivatives, commodities; forced sales post here | kernel, participants from every system |
| 2 | 1 | `firms.produce`, `firms.invoice`, `households.pay` — consumption of inputs, work in progress, deliveries, invoices, payments on terms | goods, firms, households, trade-credit |
| 2 | 2 | `estates.sell` — estate programmes post into the next session; distributions settle | estate |
| 3 | 1 | `moneyMarket.clear` — after every flow: unsecured, secured, the facility | money-market |
| 3 | 2 | `resolution` — liquidity and solvency triggers, bank resolution, deaths | banks-capital, firms, estate |
| 4 | 1 | `revaluation` — marks, FX, write-downs, liability re-marks | kernel |
| 4 | 2 | `expectations.score` — surprises recorded | expectations |
| 4 | 3 | `polity.election` — on its date | polity |
| 4 | 4 | audit | kernel |

A forced sale caused by a margin call at cycle 0 sells in cycle 1's session of the same period; a
call caused by cycle 4's revaluation is issued next period's cycle 0. That one-period lag is the
honest consequence of "a market sees what has already happened" (Clearing F1), and it is stated
here so nobody builds a second session to hide it.

---

## 9. Testing strategy

| Layer | What | Where |
|---|---|---|
| Profile tests | terms validation, display names, due actions, unit | `test/<module>.test.ts` |
| Phase tests | one phase on a small assembled world; instructions it drafts; failures it records | same |
| Assembly tests | a world with the module assembled passes the seed audit and a year of stepping stays green, or its expected red families are named | same |
| Property tests | solvers, allocations, conservation across random instruction sets, determinism under seed | `test/property/` |
| Resolution tests | 1x, 2x, 4x cells give the same aggregates to dust (Part XII; from item 4 on) | `test/resolution/` |
| Kernel tests | settlement routing, DvP, cells, calendar, register facade, module ordering | `test/*.test.ts` (exist) |
| Surface tests | the inspector renders a stepped world in a real browser | `packages/app/e2e` |

Rules: a test never widens a tolerance; a test that needs a bound to pass is a finding; a failing
audit family is asserted by name in the test and named in the record; every test that draws
randomness names its seed.

---

## 10. Performance strategy (Law 18)

- Do nothing until a standing observation says cost grows over a run (Part XII: cost growth).
- Then, in a declared campaign: storage layout, traversal order, decomposition, scaffolding removal.
  Never a mechanism, an economic quantity, or a named boundary.
- Gate on behaviour: the audit families still hold exactly; the reported numbers do not move beyond
  dust; a relabelling is a declared re-baseline.
- Candidates already visible: the audit's `allHoldings()` copies every period; `holdersOf` filters;
  per-lot arrays for cells. None matters until the world has thousands of cells and hundreds of
  instruments; measure first.

---

## 11. Dos and don'ts

**Do**
- Read the spec section before the code, every time.
- Declare every number; cite every clause; re-mark coverage in the same commit.
- Draft instructions; let settlement fail them; read the failure.
- Decide from `ParticipantView`; post schedules, not points.
- Anchor every phase explicitly; name its cycle.
- Split a cell for a partial event.
- Report a family as not built until it checks something real.
- Insert the missing prerequisite before the item in hand.
- Write the record as outcomes: what changed, why, what was found, what was deleted.

**Don't**
- Don't write a bound, a default, a clamp, a percentage tolerance. Ever.
- Don't branch on a kind id in a mechanism. Put it in the profile.
- Don't import another module. Don't import the world container.
- Don't hold a reference to a store. Don't cache a value beside its units.
- Don't read a print the period has not produced. Don't read last period's silently.
- Don't seed an outcome. Don't import an observed ratio.
- Don't give a party another party's private state.
- Don't roll back a number. Don't tune a number to pass a VERIFY.
- Don't measure mid-build. Don't chase a moved baseline.
- Don't leave a TODO. It is a worklist item or it is nothing.

---

## 12. Risks and how they are contained

| Risk | Containment |
|---|---|
| A mechanism quietly assigns an outcome | REASON/VERIFY/FORBID review at coverage time; a VERIFY is never enforced in code; lint on bounds and literals |
| Two modules grow a hidden dependency | the cross-module import lint; shared types must be kernel types or instruments |
| The kernel accretes kind branches | lint on kind branches in mechanisms; kernel changes only by inserted item with a record |
| A placeholder outlives its death | the count is on the surface and in every report; the record names each death |
| Floating-point drift across a long run | dust tolerances derived per check; the flows and money families compare ledger to register every period |
| Non-reproducibility | one PRNG, derived streams per module, party and period; no clock; determinism tests |
| The surface changes the model | worker boundary; snapshots are copies; the observer never writes |
| The phone is too slow | Law 18 campaign, measured first; the engine has no layout commitments in its API |

---

## 13. Glossary

- **Kernel**: the stores and the loop; owns every writer.
- **Module**: one spec system, instrument family or seed, declared as a `SystemModule`.
- **Profile**: the behaviour of a kind, registered by its module, asked by the kernel.
- **Context**: `ParticipantView`, `MechanismContext`, `SeedContext`; the only doors.
- **Phase**: a module's work at a stated cycle, anchored to a kernel phase.
- **Participant**: a reason for a party of a kind to be in a market, evaluated with that party's view.
- **Family**: one of the nine audit families; made of contributions.
- **Cell**: a party standing for a population, with an integer weight and per-member state.
- **Print**: a cleared price with provenance, one per instrument per period.
- **Placeholder**: a declared number with a scheduled death.
- **Dust**: the only tolerance: terms × ε × Σ|terms|.
