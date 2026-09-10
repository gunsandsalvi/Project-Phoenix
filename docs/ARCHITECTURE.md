# Project Phoenix — Architecture

The specification (`docs/spec/PROJECT_PHOENIX.md`) contains no implementation. This document is the
set of implementation decisions, each derived from a clause of the specification. A decision that
cannot cite a clause is a preference and is marked as one. Update this file **in the same change** as
the thing it describes (Appendix C).

Citations use the spec's grammar: `Money C2.a` is system 1 (Money and Settlement), node C2.a.
`Law 7` is First Principle 7. `XI-15` is a mechanism. `App B 22` is a consolidated prohibition.

---

## 1. Platform and language

**Decision.** One TypeScript codebase. The **engine** is a pure library with no DOM, no I/O, no clock
and no randomness of its own. The **app** is a web front-end that runs the engine inside a Web
Worker. The Android build wraps the same web app with Capacitor.

**Derived from.** The deployment path is stated by the owner: continuous testing on GitHub Pages, final
delivery as an APK for a Pixel 11 Pro XL. A browser runtime is the only one that serves both without a
second implementation, and one implementation is law 4 applied to the codebase itself. The worker
boundary is §45 E3 — _no surface that changes the model_ — made physical: the UI never holds a
reference to engine memory; it receives structured-clone snapshots.

**Toolchain.** TypeScript 5.9 in strict mode with `exactOptionalPropertyTypes`,
`noUncheckedIndexedAccess`, `noImplicitOverride`, `noFallthroughCasesInSwitch`. Vite for the app,
Vitest for tests, fast-check for property tests, Playwright (preinstalled Chromium) for an app smoke
test, ESLint with typed rules and project-specific rules (§9 below). npm workspaces:
`packages/engine`, `packages/app`.

---

## 2. Numbers

**Decision.** IEEE-754 doubles everywhere. Every identity check derives its own tolerance from the
arithmetic that produced the number: `sum(terms)` returns the value **and** its dust bound,
`count × ε × Σ|terms|`, using Neumaier compensated summation. There is no global epsilon and no
percentage anywhere.

`NaN`, `±Infinity` and `-0` cannot enter the state: every numeric constructor (`money()`, `qty()`,
`price()`, `rate()`) and every arithmetic helper validates and **throws** on a non-finite result.

**A balance reached by accumulation carries the walk that produced it.** An equity account, a money
balance and an issued amount are not numbers one rounding old: they are stated once and then moved,
one named event at a time, for as long as their owner exists. `Running` (`opened` / `moved`) carries
the value with `Σ ε|balance after each move|`, and a check comparing a fresh read against such a
balance adds that dust to its own. Derived any smaller — as though two readings of one number — a
busy account reports a violation the moment it is paid more than a handful of times in a week, and
the audit spends its credibility on floating point. This is Law 7 taken literally in the one place
it is easy to get wrong: the tolerance is what the arithmetic **did**, not what it looks like it did.
`moveDust(balance, delta)` is the single rule for what one more move costs. The register keeps a walk per equity account (`equityWalk`) and per money account (`moneyWalk`), and the families that ask whether a balance is negative, whether a dead party still holds something, and whether holdings sum to issued all read the same walk settlement itself reads when it decides whether an account is short enough to ask its issuer for an overdraft — one fact, one tolerance (Law 4).

**Derived from.** Law 7 defines the only admissible tolerance as _(number of terms) × (machine epsilon)
× (sum of absolute magnitudes)_, i.e. floating-point error — so the representation is floating point,
and the check carries its own dust. Audit A4/A4.a. Register B2.b.

**Rejected.** Integer minor units. Every trade at a real price would then produce a rounding residual,
and law 2 says a residual with no holder is a defect; the spec's own tolerance definition presumes
floats.

---

## 3. Value objects and units

Every quantity carries its unit and cannot be combined with a different one (App A, Units):

| Type     | Fields                                                                             | Rule                                                                                                                                     |
| -------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `Money`  | `amount`, `ccy`                                                                    | `add` throws across currencies (Money A2.b). No implicit currency (Currency A4).                                                         |
| `Qty`    | `amount`, `unit` (`par`, `shares`, `tonnes`, `contracts`, `dwellings`, `hours`, …) | `add` throws across units.                                                                                                               |
| `Price`  | `amount`, `ccy`, `perUnit`                                                         | A price of money in itself is `1` — the only hard-coded one (Money D2).                                                                  |
| `Rate`   | `amount`, `per: Periodicity`                                                       | A rate without its periodicity does not construct (Law 8). Conversion is an explicit call through the calendar's day count (Money G3.c). |
| `Period` | integer index on the one calendar                                                  | No default period; a record without one does not construct (Money G4.a).                                                                 |

**Missing is missing** (App A, Missing values). A read of something absent throws a
`Missing` error naming what was asked. There is no `?? 0`, no `|| 0`, no formatted default; the
lint forbids them in the engine. Callers that can legitimately proceed without a value use the
explicit `tryX` variant returning `Option<T>` and handle `none` in code that a reader can see.

**Identifiers** are branded string types (`PartyId`, `InstrumentId`, `AccountId`, `MarketId`,
`InstructionId`) so they cannot be mixed. An id is never a display name (Law 9). Display names come
from one naming grammar (`naming.ts`) that reads the instrument's own terms: issuer + coupon +
maturity, issuer + tenor, the issuer for a share.

---

## 4. The data model

### 4.1 Money is an instrument (Money D2, A1, B1)

A deposit at bank X in USD is the instrument `money:X:USD`, issued by X, unit `USD`. A reserve at the
central bank is `money:CB:USD`, issued by the central bank. An **account** (holder, issuer, currency)
is therefore a **holding** of a money instrument, and the register holds it exactly like a bond. Money
has no separate store, so money conservation and ownership conservation are one check family each
with no special cases. The issuer's money liability is the read of holdings in its money instrument
(Money A4, B1.b), never a stored aggregate.

### 4.2 The wire: instructions (Money D1–D4)

The **only** way state changes is by applying a numbered `Instruction`:

```
Instruction {
  id: sequential integer          // D1.a: replayable
  period, cycle                   // G2, G4: belongs to the cycle it was issued in
  legs: Leg[]                     // every leg names both sides
  reason: Reason                  // C1.b: why the units moved
  cause: CauseRef                 // C2: trade | coupon | maturity | corporate action | default | …
  price?: Price                   // C2.a: if a trade, the print
}
Leg = MoneyLeg { fromAccount, toAccount, amount: Money }
    | AssetLeg { from, to, instrument, qty: Qty, lots? }
    | CreateLeg / DestroyLeg { party, instrument, qty }   // Goods B, E4: units entering or leaving
    | AssumeLeg { from, to, instrument }                  // XI-8: who OWES a line changes
    | PledgeLeg / ReleaseLeg { pledgor, beneficiary, instrument }  // B3.c: units bound and freed
```

`AssumeLeg` is the odd one and it is here for a reason (item 7). When an issuer dies its paper does
not die with it: whoever succeeds it owes the line from then on (Firm Birth D5). Nothing moves in the
register — the same holders hold the same units on the same terms — but the **obligation** moves
between two balance sheets, and a change of balance sheet that does not go over the wire is exactly
what D1 exists to prevent. So it is a leg, valued at what the holders carry it at (Register B3: a
liability is the same number read from the other side), and both equity effects land in the one
instruction.

`PledgeLeg` and `ReleaseLeg` are the same kind of oddity, and they arrived with the money market
(item 11). Secured borrowing prices the collateral (Money Market B3), and pledged collateral is
**encumbered**: it cannot be sold and it cannot be pledged twice, which is how a solvent bank runs
out of the ability to borrow (B3.c). Nothing changes hands — the pledgor holds the paper, carries it
and collects on it — so neither leg has an equity effect; what moves is what is FREE. They are legs
because an encumbrance names two parties and belongs in the same numbered instruction as the money
it secures: collateral bound in one pass and lent against in another was briefly nobody's. A pledge
says what it `secures` (Register D5.b), which is how the release finds its lien; a pledge of more
than is free is an economic outcome (`insufficientCollateral`) and not a violation, so the
instruction fails and the row is never written.

**Settlement** applies an instruction with one rule (C2): payer minus, payee plus. For a money leg
between accounts at different issuers it generates the **interbank reserve leg** itself (C2.a); a
same-bank payment moves no reserves (C2.b). All legs of an instruction apply **atomically**: every leg
is pre-checked (free units, currency match, account existence, the issuer's overdraft decision), and if
any leg cannot apply, **none** does and a `Fail` is recorded with the instruction that failed and why
(Register C3, XI-5). Delivery versus payment is therefore a property of the ledger, not a convention.

Settlement is **final** (E2): there is no reversal API; a correction is a new instruction. Order within
a cycle is the instruction order (E3).

### 4.3 The register (Register A–F)

`Holding (holder, instrument)` → `{ lots: Lot[], liens: Lien[] }`. A lot carries quantity and basis
(D4); a lien encumbers units (D5); free units = held − encumbered, and **only free units can move**
(D5.a). Both directions are indexed — by holder and by instrument (D2.a) — and both indexes are
written by the single settlement path. Issued amount per instrument (B1) is changed only by issuance,
re-opening, buyback, amortisation, maturity; the audit compares holdings to it (B2).

**Which way a lot may be re-measured is the kind's rule, and one place holds it.** The register
re-marks a lot (`remark`); whether the move may be upward is asked of the profile in revaluation,
where the kind is known. Inventory is written down and never up because nobody but a dealer marks up
a thing it made (Goods E2.c); a claim moves both ways because a provision unwinds when its holder
stops expecting the loss (Banks Lending D2.a). The register held a copy of that rule for two items
and made the second case impossible.

**One reader of what a party can deliver.** Whether a holding can give up `qty` is asked and answered
in exactly one place, `Register.deliverable`, which compares the ask against the free quantity within
the dust of the walk that produced both — the lots it was summed over, matched one at a time. Before
that reader existed the settlement pre-check and the register's own draw each derived that dust for
themselves, disagreed at the fifteenth decimal, and turned a trade that had already been admitted
into a throw. A number two callers must agree about has one writer (Law 4).

**A split is one door and it moves nothing** (Register E4, E5, Equity D4). `MechanismContext.split`
restates a line: every holding's quantity is multiplied and its basis per unit divided, every lien
scales with the units it binds, every print ever written is re-denominated, and the issued amount
moves with them. No holding changes hands, so there is no instruction and no money leg — which is
exactly what the event says out loud, and is why a split is the invariance that proves an opening
price was a RESOLUTION rather than a shape. Only a kind whose profile says it `splits` may, and the
audit's flow family reads the published ratio so a restated unit is not mistaken for a move.

Derivative contracts are **not holdings** (Derivative X1). They live in a separate `Contracts` store
whose invariant is zero-sum (D1.b). The foundation defines the store interface; the layer (§16) fills
it in its worklist position.

### 4.4 Parties: named or cell (XI-15)

A `Party` is `Named` or `Cell`. A cell has an integer `weight` (a count) and holds **per-member**
state: its holdings and accounts are stored per member, and the cell's total is `weight × member` at
read. An instruction leg on a cell is denominated **per member** and records the weight it was applied
at, so the counterparty's total is `perMember × weight` and both legs still sum to zero.

Consequences the representation enforces rather than checks:

- a cell is homogeneous by construction — there is no field a member could differ in;
- an event that applies to part of a cell must **split** it first (`splitCell(cell, members)`), which
  is exact; identical cells **merge**;
- the weight changes by exactly five events — entry, death, promotion, split, merge — through one
  `weightEvent` API that journals cause and date; nothing else can write it;
- aggregation is `cell.integrate(f) = f(memberState) × weight`. There is no `mean()`; the average is
  unreachable (XI-15: _a question that cannot be phrased will not be asked wrong_).

The cell **key** (region, cohort, bank) is registry data; lifting a relationship into the key is a data
change and a re-stratification event, never a mechanism change (Small-Business Pools A6.a).

### 4.5 Prices and value (XI-6, Clearing D/E)

`PriceStore (market, instrument, period)` → `Print { price, provenance, tradedQty }`, written only by
the clearing engine. Provenance is one of `traded`, `stale(fromPeriod)`, `interpolated`,
`extrapolated`, `none`. A market with no trades writes a **stale** print carried from the last traded
one, visibly (Clearing E4, C4.b); it never silently refreshes.

**A print is not always the mark, and one line can have both** (XI-6, Fund Shares E2). A kind whose
`pricing` is `derived` is a claim ON A BOOK: what a unit is carried at is what that book comes to
over the claims on it, read fresh at every ask, and the kernel asks the kind BEFORE it looks for a
print. That ordering is what lets a derived kind also name a market: an exchange-traded fund's
shares trade, so a session prints what somebody paid for one, and the fund's own book says what one
is a claim on. The two are different numbers and neither is the other's approximation — which is
E2 rather than a discrepancy, and is why nothing anywhere reconciles them.

**Whether a market MADE a price is part of the price** (Clearing E4, Law 3). `wasTraded(print)` is
the kernel's one reader of that: a mark carried forward because nobody traded is not a level anybody
could transact at, and a party deriving anything from one would be building a derivative on an
uncleared price (App B). The desks' creation and redemption arbitrage stands down on a carried mark
for exactly that reason, and the premium an exchange-traded fund publishes says which of its two
halves is a trade.

`value(holding, period) = qty × price(instrument, period)` at read. There is no stored value beside
units (App B 39). An instrument declared `carriedAtCost` in the registry values at its lots' basis; any
other unpriced instrument **throws** `Unpriced` when valued.

**A kind says what a LOT is carried at, per lot** (Goods E2, Capital Programme A3, A6; item 10.1).
`InstrumentKindProfile.carriedAt(instrument, lot, marked, period, calendar) -> Option<perUnit>` is
asked lot by lot at revaluation; the kernel books the difference against the equity account and
re-marks that lot to the answer, so one number lands on the stock and on income together. `none` means
this lot is carried at what it was. It is per lot because the answer is: a good is written down to
what its market last said (E2), and a vintage of plant wears out on a schedule of its own from what
its own holder paid for it — two vintages of one kind have different lives left, and a lot bought
second-hand carries what its buyer paid. A rise is refused unless the kind declares
`fairValueThroughIncome` (E2.c), in that one place. The mark is handed in as an `Option` and a kind
may ignore it: a thing that wears out has no print and does not want one.

### 4.6 The clearing engine (Clearing A–F)

One solver for every market. A participant posts a `Schedule`: a monotone step function from price to
signed quantity, built from the participant's own state (A3) and never from the clearing price (A4).
The solver finds the price where posted supply meets posted demand (C1), rations by the market's stated
rule (C3), and returns one of the representable outcomes (C4.b):

`cleared | noDemand | noSupply | noOverlap | excessCommitted`

A search bracket is never returned as a price (C4.c). Trades are emitted as instructions (D2, D3), and
the print becomes the mark (D4). The solver is a pure function of the schedules (C5) and is tested for
determinism with property tests.

**A quantity with no level** (Central Bank C3). A participant may post `price: 'market'`: a size and no
level. It is resolved before price formation to the worst level the other side actually posted — the
highest ask for a buyer, the lowest bid for a seller — so it is a level somebody posted (C4.c) and
never a bracket. With nothing posted opposite it, there is no level to take and the order is not in
the book.

**The stated price rule.** When several levels execute the same volume with the same imbalance the tie
is struck by a rule the venue states, like its rationing rule: `sellersCompete` (the lowest such
level: an open book where supply exceeds demand is sellers undercutting each other) or `marginalBid`
(the highest: the **stop-out** a uniform-price sealed-bid auction allots at, Sovereign C2).

**The primary form** (Sovereign C). An issuer's own supply for one session is a `PrimaryOffer`: a size
(C1.b) and a walk-away level (C5, C7), posted through `MechanismContext.offer` before the session and
public to bidders through `ParticipantView.offer` (C1.a). The same solver clears it under the
`marginalBid` rule, so every winner pays the stop-out (C2); the market reads the **cover** and the
**tail** off the book (C4) and journals `auction.result`; the paper nobody bid for is withdrawn and
the withdrawal is that event (C7). Nothing absorbs the remainder (Treasury D5.a). A trade whose seller
is the instrument's issuer is an issuance leg, which settlement already knows, so a debut and a
re-opening (B3.a) are the same act in the same book, and there is still one print per instrument per
period.

**A line with no price.** A market that did not clear and has nothing to carry writes **no print**: a
line that has never traded has no price, and the reader is told so at the reading site (XI-6). The
prices family reports a _held_ position with no print, because its holder cannot mark it; an unheld
line with no print is marked by nobody and is not a defect (Audit B3).

**A module's own state** (Law 4). `ctx.state(name, initial)` hands a module an object it owns, kept
between phases and periods under `<module>/<name>`: an employment register, a book of invoices, a
party's outlooks. It is keyed data, never a second copy of what a kernel store already holds, and
the observer reads it as data (a copy, so looking changes nothing).

**What a bank does about an overdrawn customer** (Money B3.a). A party kind may state its answer to
B3 or say that its answer is a **credit decision**, and then it answers nothing: a decision weighing
the room a bank's own capital supports is not something a kind profile could take. The module that
owns lending registers it, one module per kind, and the kernel calls it through that module's own
context. A world whose kind says this and has nobody to answer **cannot be sealed** — a defaulted-to
refusal would look exactly like a bank with a credit standard, which is the thing C3.a says must
never be invisible.

**What a lot with no market is worth** (XI-6, Banks Lending D1, D2). Almost everything is worth what
a market said, and the kernel reads that from the price store. A loan is the exception the spec
names: not a security, no market price, carried at amortised cost less what its holder expects to
lose on it — which is that holder's own assessment and cannot be anybody else's. So one module per
kind answers, and the kernel asks it **only when the price store has nothing**: a holder's own
assessment stands where there is no market, never instead of one.

**What a party expects** (Expectations A2, XI-16). `view.outlook(variable)` answers with that
party's own outlook or with nothing; there is no global expectation to fall back on (A2.b). Exactly
one module may answer — an expectation is a fact about a party and has one writer — and the kernel
asks it through that module's own context, so what it keeps stays its own. The door has two halves
and they are one door: `of(party, variable)` and `variables(party)`. A surface that could ask the
first but not the second would have to guess the names it asks about, and a guessed name is a default
outlook by another route.

**Made and used up, not issued** (Goods A1, E4, F1). A physical thing has **no issuer**:
`Instrument.issuer` is an option, and a claim on nobody is a defect the names family reports. Units
come into existence and leave it through `create` and `destroy` legs — one side each, because
nobody is on the other end of a harvest — admitted only for a kind that says its units are physical,
and a `create` must sit in the same instruction as the `destroy` legs of what it was made from. The
units family checks the identity: what the stock did against what said why.

**Priced one way, carried another** (Goods E1, E2). `pricing` says where a price comes from;
`carry` says how a holder carries it. Inventory clears in a market and is carried at what it cost,
and `profile.revalue` writes it **down** to what the market says — never up, unless the kind marks
both ways (E2.c). A write-down lands on the lot's basis and the equity account together, because
they are one fact. A holder carrying at cost needs no print to value its book, so the prices family
asks only about positions carried at the mark.

**The curve is a read** (Sovereign D3). A curve family is declared once by the module that owns it
(D3.a) with one compounding convention and one day count (D3.c). `ctx.curve(family)` builds it at
the moment somebody asks, from the prints the market has already produced and the cash flows the
instruments' own terms promise (`profile.cashFlows`); nothing stores it, so the fit's own previous
output can never be an observation (D3.b). Every point says whether it traded this period or was
carried; a tenor between points reads `interpolated`, beyond them `extrapolated`, and a family with
no points answers `none`. The yield is derived from the price and never the other way round (D2,
N7.b): `yieldOf` inverts the discounting of the flows, and `priceAt` is how a participant turns a
yield **it** decided into the level it posts.

**Accrued interest travels with the paper** (Bond N9.b). An `AssetLeg` carries `accruedPerUnit`
beside its clean `pricePerUnit`; the money leg moves the dirty amount. The lot's basis is the clean
price, so the buyer's equity falls by the accrued now and rises by the whole coupon on the date: the
coupon is not a windfall to whoever holds it then, and the seller's income is what it earned. What
accrued is a read of the instrument's own terms through `profile.accrued`, never a stored receivable
(Appendix B: no stored value beside units).

### 4.7 Time (Money G)

One `Calendar`: an epoch date, a period length of **7 days**, `cyclesPerPeriod` (a RESOLUTION
parameter), and one mapping period ↔ date. A periodicity is placed by **advancing a date** and
landing in the first period on or after it (G3.a); nothing counts periods. Day counts read the
calendar's dates (G3.c). No periodicity finer than a period exists (G3.b); finer structure is cycles.

### 4.8 The period loop

A period is an **ordered list of phases** held as data (`schedule.ts`), each a `Mechanism` with a
name, its spec citations, and a `run(world)`. Markets clear at their stated point (Clearing F1); a
phase that reads a print not yet produced this period gets a `NotYetProduced` error, not a stale
value — the fix is the order (F1.a). Every period ends with settlement of the last cycle, then
revaluation ordering checks, then the **audit** (Audit C1–C3). The loop is the same every period;
phases are never skipped conditionally (Audit C3).

**Where a phase goes is decided by what it READS, and one system can need two slots.** A test of
solvency asks whether liabilities exceed assets AT MARKS, so it belongs after revaluation — asked
before it, a party whose own liabilities are marked reads as insolvent by whatever it paid out this
period. Paying and being paid belongs with the period's other payments, before the marks are taken —
done after them, a write-off nobody has marked yet leaves the party that carried it owing more than
it holds for a whole period. So the estate opens in one phase and settles in another, and the split
is not bookkeeping: each half sits where the thing it reads is true.

### 4.9 The audit (Audit A–E)

Each **family** (money, ownership, prices, cross-market, accounts, names, flows, zero-sum, units) is a
module exporting `check(view): Violation[]` where `view` is a **readonly** projection of the world.
A violation names the owner (party, instrument or instruction), the size in a unit, the period and
the citation (A2, A3). Tolerance is the dust returned by the arithmetic (A4). The audit **never
repairs** (C4) — the type of `view` has no mutating methods. Output is counts by family, worst
instances, and it is reproducible from the seed (D1–D3). A family that is not yet built reports itself
as `not built`, never as green.

Equity is a **stated account** per party, moved only by named events; the accounts family compares it
to the read of assets minus liabilities (Audit B5, B5.a). The seed sets it once to the read; after
that only events move it.

### 4.9a Accounting: how equity moves, and the routing rule

These fell out of building items 1 and 2 and are recorded here because they are load-bearing.

- **Everything about a party is per member of that party.** Holdings, cash, the equity account and
  every effect on them are per member (a named party is a cell of weight one). The only place a
  total appears is a leg's other side and the register's `heldTotal`. Mixing the two was the first
  defect the audit caught.
- **Routing across issuers** (Money C2.a). A money leg from account (h1, i1) to (h2, i2): payer minus
  (or _creation_ if h1 = i1), payee plus (or _destruction_ if h2 = i2). If i1 ≠ i2, then for each
  issuer that is a bank (not the central bank): the bank's own money is redeemed on the paying side or
  issued on the receiving side, and its reserve account at the central bank moves by the amount. Money
  whose issuer is the central bank changes holder and is never redeemed by a transfer. Creation and
  destruction happen only through an issuer's own account (Money C4), which is what the money family
  counts.
- **Equity effects of an instruction** are computed from the legs, never from a "nature" flag: what
  came in at its price (or at the giver's carrying value for a transfer without a price) minus what
  went out at carrying value; for the issuer of a liability, minus what it issued at price plus what
  it redeemed at the holder's carrying value; and, on a holder-to-holder transfer, the issuer's
  liability is re-marked from the giver's carrying value to the receiver's basis (Register B3: the
  liability is the same number read from the other side). An exchange at a price nets to zero, a
  transfer is income and expense, a sale away from the mark is a realised gain or loss. Instruments
  whose profile says they are not a liability of their issuer (shares) produce no issuer effect, so
  capital paid in is simply cash received.
- **Carrying value** of a lot during period t, before revaluation, is the period t−1 print if the lot
  was acquired before t, else its basis. It is derived, never stored. Revaluation books
  `qty × (mark_t − carrying)` to the holder and the reverse to the issuer of a liability, so the
  accounts family holds exactly after it and world equity is zero-sum across every event.
- **The audit remembers** the previous period's issued amounts and per-member holdings so that the
  flows and money families compare the ledger's deltas with the register's change: two independent
  records (Audit A1.a). Both skip when asked twice in one period.

### 4.9b Kernel and modules (Law 15, Granularity)

The engine is a **kernel** and a set of **system modules**.

The kernel owns the stores and the loop: calendar, registry and parameter register, parties,
instruments, register, ledger and settlement, price store and valuation, clearing solver and market
runner, journal, audit runner, corporate actions, revaluation, cells. It owns the money instrument
kind and the party kinds money needs. It changes only when a Part III clause demands it, by an
inserted worklist item.

A module (`src/mechanisms/<system>/`, `src/seeds/<name>/`) implements one system of the
specification, one instrument family, or one seed. It declares, as data (`SystemModule` in
`world/module.ts`): instrument kinds with their profiles, party kinds with their profiles, units,
parameters, phases anchored to kernel phases, participants evaluated per party of a kind, audit
contributions, and a seed contribution. Assembly (`world/assemble.ts`) merges these in dependency
order (`requires`), seeds, states equity as the read, and seals the world with the audit.

Modules reach the kernel only through three contexts (`world/context.ts`), and nothing else:

| Context            | Who gets it                                                 | Can                                                                                                                                                                            | Cannot                                                                       |
| ------------------ | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| `ParticipantView`  | a party, when a participant declaration is evaluated for it | read its own holdings, cash, equity; who anybody IS (kind, region, bank, weight); public prints, public instrument terms, public events, its own record; its own random stream | see any other party's private state (Observer A4, Expectations D1)           |
| `MechanismContext` | a module phase                                              | read public state and any party's own view; settle instructions; register instruments and markets; apply cell events; cease a party; journal                                   | write the register, write a print, write a weight, reach the world container |
| `SeedContext`      | a seed module at period zero                                | add parties and instruments; endow money and units; write opening prints; open markets                                                                                         | anything after the seal                                                      |

**Schedules come through one door, in a market and in a venue.** A market gathers its orders from
the `participants` modules declare per party kind, each evaluated with that party's own
`ParticipantView` (`world.ts` `runOne`). A VENUE — where something is struck that is not the
transfer of an instrument, a job at a wage or a week of money at a rate — is cleared by the module
that opened it (Clearing B2, Labour C5), and its schedules come the same way: a module declares
`venueParticipants`, the module that opened the venue calls `ctx.gather(venue)` at the top of its
clearing phase, and the kernel evaluates every declared schedule with each party's own view and
posts it. Only the module that opened a venue may gather it, and a venue is gathered once a period.

The rule that door exists for: **a module that builds another party's schedule inside its own phase
is deciding for a party it does not own**, with a `MechanismContext` that can see private state no
participant may have (Observer A4). Clearing is the venue's; the schedule is the party's.

Two rules hold this shape: a module never imports another module or the world container (lint
`phoenix/no-cross-module-import`), and the register's write methods are reachable only through a
store the kernel hands to settlement, the seed and the cell events. `World.register` is a
runtime-frozen read facade, not a type alias.

**The one import a module may make** is the **terms accessor** the owner of a kind publishes — the
type, its guard, and the functions that name that kind's instruments (`goodTerms`, `goodId`). It is
not a door into another module: the thing being read is an INSTRUMENT, which is kernel state that
every module can already see, and the accessor is only the shape of its terms, which the kernel
itself refuses to look at (Law 15). The treasury reads a bond's coupon this way; a firm reads a
good's recipe this way. Nothing else crosses: no state, no phase, no behaviour, and a module that
wants another module's decision reads the event it published or the row it wrote.

**Who somebody is, is public** (Observer A3). `ParticipantView.parties` answers a party's kind, its
region, its bank, whether it is still here, and how many people a cell stands for — the facts a
market needs to know whose paper it is trading. What that party holds, owes, expects or is worth is
not reachable from any view but its own (A4). **`ParticipantView.lastOwn(kind)`** answers with the
most recent event of a kind the party is a SUBJECT of — its own record: what it announced this
period, what its own wage bill came to. A decision taken in a phase and an order posted into a
market are one decision (Law 4), and this is how the second reads the first instead of taking it
again.

**Two more reads, and why they are reads and not imports.** `ParticipantView.mark(instrument)`
answers what a unit of a line is carried at — the print, or, for a claim on a book, what that book
comes to (§4.5). `ParticipantView.lastPublicAbout(kind, subject)` answers the question a participant
actually asks: not "what was the last dividend anybody declared" but "what did THIS issuer declare".
Both are public by construction — a print is public by Clearing E1, a derived value is arithmetic on
a register anybody may read, and a public event is public — and both exist so that a module reading
another system's decision reads the event it PUBLISHED rather than importing the module that took
it. A saver values a share off `payout.declared` and a desk arbitrages off `etf.struck`, and neither
knows which module wrote it.

Kinds are registered at assembly, not closed unions: adding an instrument kind is one profile in
one module; the kernel learns how a kind behaves only by asking its profile (pricing, liability,
unit, terms validation, display name, actions due). An instrument whose kind has no profile cannot
be registered. A kind's `unit` is a property of the kind, so a family whose members are measured in
different things is a kind per member, generated from one data table by one factory: the goods are
`good.grain`, `good.flour`, `good.bread`, each with its unit, and nothing branches on which.

**How technology crosses the boundary.** A module cannot import another, so a fact one system owns
and another must read travels through the kernel's own stores. For a good's RECIPE that store is the
instrument's **terms**: public structure (which inputs, and the `ParamId` of each quantity), with
every number in the parameter register where its unit and owner are declared (XI-14). A firm reads
what a thing takes to make from the thing itself; the coefficient exists once. The same route serves
anything else a system must publish about an instrument.

**Units come into the world by production, and by nothing else.** A `create` leg is admitted only
in an instruction whose cause is `production` (or `seed`), and only for a kind whose profile says
its units are physical: a claim that appeared with nobody on the other side is invented money
(Money C1). What a batch had to DRAW to make those units is the recipe's business, and the recipe
lives in the good's own terms, which the kernel never opens — so the kernel cannot check it and does
not pretend to. The goods module checks it instead, as an audit contribution: what a production
instruction consumed is the recipe times what it created, and a good comes off its own batch and
takes at least its own units of it (Goods B2, B4). The first stage of every chain is drawn from
labour and land and consumes no units at all, which is why the kernel cannot require a destroy.

**A batch is a thing (Goods B3).** Work in progress is an instrument kind of its own per good —
physical, carried at what it has cost, no market, never written down because there is nothing to
write it down to. Its LOTS are the batch book: each carries what that batch cost and the period it
was started, so what is due off the line is a read of the register rather than a second register in
a module's state (Law 4, Law 19). A firm's balance sheet is therefore true at every instant between
the spending and the selling, instead of having a hole in it for the length of the lead time.

**Venues (Clearing B2, Labour D1).** A market moves an instrument against money and the kernel
settles it. A **venue** is where posted schedules clear into something that is not a transfer: a
labour market strikes a RELATIONSHIP — a firm, a worker, a wage, a start date — which then persists
and is paid period after period, so what a match produces is a row in the owning module's register.
The kernel owns the book (`world.post(venue, order)`, emptied at the top of every period) and the
solver; the module that declared the venue clears it in its own phase and acts on the matches. A
venue is declared like a market and is public — an employer's opening is a posted intention anyone
can see — and its `key` says what makes it itself (region, occupation), so another system finds the
venue it needs without knowing how this one names things.

**Names are composed, not stored.** `registry/naming.ts` resolves the parts of a display name that
are state — the issuer's name, a region's name — and hands them to the kind's profile as a `Namer`;
the profile composes the name from its own terms (Law 9). A claim demands an issuer and throws
without one; a physical thing has none and is named by what it is and where it trades.

### 4.9c The capital programme, and where the investment decision lives (Capital Programme, XI-4)

**Plant is a dated VINTAGE, and a vintage is an instrument** (A6): its own kind, physical, issued by
nobody, counted in its own unit, with the date it went into service and the date it is worn out in
its terms. Vintages are keyed by `(capital kind, region, service date)` and shared by everybody who
commissioned in that period, so their number is bounded by the kind's life rather than by the number
of purchases — and a machine sold out of an estate keeps its age, which is the whole reason the date
is on the instrument rather than on the lot. What it COST is the lot's, where a basis lives.

**One schedule, charged in both places** (A3): the kind answers `carriedAt` with the straight line
over the service the vintage has left, and the kernel does the rest (4.5). There is no accumulated
total beside the stock; accumulated depreciation is a read over the journal's charges and gross is
net plus accumulated.

**Which module owns what.** `mechanisms/capital-programme` owns the STOCK — the kinds registry, the
vintage instruments and their markets, the wearing-out schedule, retirement, the commissioning of a
bought good into plant, and the A6.b units identity. It owns no decision. **The decision to invest is
the firm's** and lives in `mechanisms/firms/invest.ts`, because it is made of the same things every
other firm decision is made of (its own outlook, its own cash, the prices it faces) and it shares
the same order list. `firms` therefore `requires` `capital-programme`, and `capital-programme`
requires only `goods`.

**Whether a purchase is plant is read off the wire** (A4.c). What a firm BOUGHT of a capital good is
commissioned; what it MADE of one is stock. That is why nothing has to ask what industry a party is
in, and why a workshop holding its own output is not investing in itself (Law 15).

**A recipe names its capital services** (Goods A2.c): the good's own terms carry, per capital kind,
the units of plant that let a line start one unit per period. Capacity is the scarcest of them
(A4); a recipe naming none is not limited by plant, which is a different answer from a large number.

### 4.10 Registry and parameters (Law 2, Law 15, XI-14)

All data lives in the **registry**: currencies (each naming its issuing central bank), regions (each
naming its currency), units, party kinds, instrument kinds, cell-key dimensions, platforms. A
module's own tables are its own registry, in `mechanisms/<system>/data.ts` — the treasury's maturity
grid, the goods and their recipes — and the numbers in them are declared parameters generated from
those tables, so a table row and a register entry are never two copies of one number. Behaviour that varies by kind lives in a **profile** behind a dispatch table keyed by kind,
with exhaustiveness enforced by the type system. Mechanics never branch on a kind (lint, §9).

An **instrument kind** says where its price comes from: `money` (the one hard-coded one),
`cleared` (a market printed it), `carriedAtCost` (nobody prices it and its holder carries what it
cost), or `derived` (item 8) — a value that is neither, because the thing IS a claim on a book and
is worth what that book comes to over how many claims there are (Fund Shares B1). A derived value is
read at every ask and stored nowhere; it is given the kernel's own reads and nothing else, so it is
the same number for every holder — which is what separates it from the `marks` valuer, which says
what a lot is worth to the party HOLDING it and has a different answer per holder. The registry
refuses a derived kind that derives nothing, and the valuation refuses a book whose value depends on
its own claim.

A **party kind** states four things about its life beyond its representation and its money issuance
(item 7). `fails` says what a party of that kind can FAIL on — nothing, a cash failure it cannot
cure, its liabilities past its assets, or both — and a kind that names neither cannot die, which is
how XI-3's two exceptions (the central bank, and a treasury in its own money) are named consequences
rather than omissions. `terminal` says whether it may end the chain of successors, which only an
estate that has paid everything away may. `borrows` says whether anybody lends to it at all: a going
concern yes, an estate and a household no (Banks Lending A1, Households C1.d). `choosesBank` says
whether it picks where it banks and leaves when somebody pays it more (Money Market E1) — a household
and a firm do; a bank and a treasury settle at the central bank because that is what settling in
central bank money means; a trading desk IS its bank's own arm and an account at a rival would make
it a different firm (Dealer Desks A1); an estate holds the account of the party it succeeded and is
realising it, not running it. All four are read by mechanisms and never branched on by kind id.

Every number that shapes behaviour is declared in the **parameter register** with value, unit, owner,
and provenance kind: `technology | preference | policy | resolution | shape | placeholder`. A
placeholder names the mechanism whose absence it stands in for and the worklist item that deletes it.
The count of shapes and placeholders is a reported metric and must fall (XI-14). Engine code reads
numbers only through `params.get(id)`; numeric literals other than `0, 1, -1, 2` are linted out of the
engine except in `core/num.ts`.

A parameter that is an **AMOUNT** of something — thirty thousand PHX a period, seven thousand hours,
four hundred thousand units of a line — says so with `denominated: true` and is read with
`params.amount(id, unit)` rather than `get`. The declaration then says what a person means and the
register hands back the count of indivisible pieces that unit is counted in at this world's
resolution (§4.11a), so **a declared amount moves with `pieceShift`** instead of being multiplied by
a subdivision at every declaration site. WHICH unit is the reader's to name, because one policy is a
number for whatever money, time or paper the party reading it deals in — a `get` on such a parameter
throws rather than answering in the wrong number. The register is therefore built twice at assembly
out of one list of declarations: once with no units, which can answer only `resolution.pieceShift`,
and once against the registry that number built.

### 4.10a What a module knows between periods

A module's own register — employment rows, a book of invoices, a party's outlooks — lives in a state
slot (`ctx.state(name, initial)`), keyed by the module that owns it and snapshotted by the observer
as the data it is. Two rules keep it honest: a slot is never a second copy of what a kernel store
already holds, and a module that must also **audit** its own register builds the object once, per
world, and hands the same object to its phases and to its audit contribution — one book, one writer
(Law 4). That is why `goods()` and `labour()` are factories: a module with a memory is this world's.

### 4.11 Events and the observer surface (§45)

The `Journal` is an append-only list of events **generated from state transitions** by the engine
(defaults, fails, weight events, prints, failed auctions, …); it is a read of what happened, never an
input. The observer API (`observer/`) takes a `Scope` — `inspector` or `party(id)` — and answers
queries from a snapshot: prints with their provenance and age (A1.a), positions, public state,
published aggregates with a stated lag (A5), the outlooks of the parties in scope, and the journal.
A party scope filters out other parties' private state (A4): it sees its own outlooks and no module
state at all, because a module's slot holds other parties' private state as often as not and a
surface that showed it would be showing it to them. The module slots — employment rows, inventories,
the outlook book — are therefore the **inspector's** product and are null for a party. The UI can
only reach the engine through this API over the worker bridge.

A viewer asks for the journal two ways, and the difference matters. `journalTail` is the last N
events of **everything** — a feed, a display depth, nothing more. `follow` names the **kinds** it
wants the recent events of, and they come back per kind at that same depth (`Journal.recentOfKind`,
filtered by the same visibility rule as the feed). Anything a party says once a period — an issuer's
programme, an auction's result — is read that way, because how far back a feed of everything reaches
shrinks every time the world finds more to say, so sifting such an event out of the feed is a read
that goes quiet as the model grows and never says that it has.

### 4.11b Where a bank's own economics live (Banks Lending, Banks Funding, Banks Capital, Dealer Desks)

**Modules are cut along DECIDERS and VENUES, not along spec systems.** A spec system is a
requirement; a decider is a party that has to live with what it chose. §23, §24, §25, §26 and §8.E
all describe ONE bank from different sides, and mapping each to a module turned one bank into a
committee that never met: a treasury that made markets, a forced seller that could cross its own
buyer, an auction bidder that stood down by hand, and a desk that was a separate party. Every one of
those was a bug before it was a principle.

- **`banks` decides everything a bank decides.** One row per bank (`data.ts`) carries what it is like
  as a lender, a funder, a treasury and a dealer, because those are four sides of one disposition.
  Its TREASURY (`treasury.ts`) owns the balance sheet — what it holds liquid and in what form, what
  it pays each class of depositor, what it will lend a rival and at what, what it is short of — and
  it POSTS NOTHING. Its DEALING line (`dealing.ts`) is the bank's only face to any market: it quotes
  around the treasury's target, it bids at the auction for the obligation the issuer announces, it
  sells with urgency when the last session refused the bank, and it creates and redeems a fund's
  shares. Its LENDING line prices credit and answers the overdraft door. Its CAPITAL (`capital.ts`)
  is one position over one balance sheet, weighted BY INTENT: what it holds up to the treasury's
  target weighs what a claim on that issuer weighs, and what it holds above that weighs what a
  trading position weighs (Dealer Desks F2, with nothing exempt because there is nothing to exempt).
- **`money-market` is a VENUE and decides nothing.** It declares the books, calls `gather` so the
  banks' own schedules arrive through the kernel's door, seats the window (the central bank's own
  offer), clears what was posted, writes the rows, publishes the refusal, and takes charge when a
  bank fails — because a bank does not go to an estate (Banks Capital C3.b) and it says so to the
  kernel with `resolves: [BANK]`. No function in it reads a bank's preference.
- **`sovereign-curve` is a curve family and a check**, and `sovereign-auction` does not exist: the
  primary bid is the dealing line's, at the dealer's own price.

Neither module imports the other. What crosses between them are **public events**: the capital
position and the line it will fund for one name (`bank.capital`), what it pays for money
(`bank.costOfFunds`), what it requires of a name (`bank.reservation`), what its funding looks like
(`bank.liquidity`), what its own account did to it and what it holds against a bad week
(`bank.buffer`), the board it is showing (`bank.depositRate`), and what its dealing book is carrying
(`bank.dealing`). The market's own facts go the other way: the segments depositors are grouped into
(`deposit.classes`) and what the window would advance each bank today (`centralBank.collateral`).
That is the same door a depositor, a rival bank and the observer read them through — which is what
makes "a bank near the line behaves differently" a thing the world can see rather than a thing one
module tells another (Banks Capital B3.a, Banks Funding E2.a).

**One face per market is a contract, not a convention.** Exactly one module gives `BANK` a
participant, in markets and in venues. The kernel refuses a party on both sides of one book at
crossing prices at the site (`pairFills`, Clearing A2, Register D2) — it used to step past it,
because a bank really could post both sides, and stepping past it was a patch in the kernel for a
defect in the bank.

The **subordinated layer is not a special case anywhere**: it is a claim whose profile declares a
seniority behind every other claim on the bank, and the resolution and the estate both work through
the ranks they read off the instruments themselves (Bond N13.a, Law 15). A layer added later takes
its place in the queue by declaring one.

### 4.11a A quantity is a whole number of indivisible pieces (Law 1, Law 8)

A unit of anything real has a smallest piece and nothing finer exists: there is no half-cent, no
gram of a cargo weighed in kilos, no thousandth of a share certificate. **So a quantity IS A COUNT
OF PIECES and the count is an integer** — not a fraction of a named unit. The state holds 18,849
cents, never 188.49 of anything; a cargo is counted in grams, a workforce in hours, a register of
members in whole shares.

**Integers are why this works.** Integers add, subtract and compare exactly in binary floating point
up to 2^53 of them, so a balance moved a million times is exactly the balance and the checks that
compare it need no tolerance at all rather than a derived one. The decimal arithmetic everybody
actually does (x.xx + y.yy) is exact because it is integer arithmetic on cents, which is what it
always was. The earlier attempt at this — a power-of-two tick of 2^-20 of a named unit — bought the
same exactness and paid for it with a granularity nothing real has, and it showed: a bank was
recorded as refused funding for a shortfall of 1.8e-7, and every uninsured depositor left it.

`UnitDecl.perUnit` says **how many pieces one NAMED unit is divided into**; the numbers are chosen
together in `registry/grid.ts` (money 100 — the cent; a tonne 1,000,000 — the gram; a whole thing 1;
an hour 1) and `resolution.pieceShift` multiplies them all at once so the choice can be tested.

**The boundary is the registry and it is the only one.** `registry.pieces(unit, named)` turns a
person's number into the count the state holds and `registry.named(unit, pieces)` turns it back for
a reader; `registry.priceOf(ccy, unit, perNamedUnit)` does the same for a price, which is a ratio
carrying both subdivisions. Nothing between those two ever divides by a subdivision, because
everything between them is already a count. A **parameter** declared as an AMOUNT says so
(`ParamDecl.denominated`) and is read with `params.amount(id, unit)`: the declaration says what a
person means — thirty thousand PHX, seven thousand hours — and the register hands back the count of
pieces, so every declared amount moves with `pieceShift` instead of being restated against it at
every site.

The wire enforces it: `Settlement` throws `Impossible [Law 8]` on any leg carrying a quantity that is
not a whole number of pieces — money amounts, asset and physical quantities, pledges, and the
**per-member** side of every cell leg (each member of a cell is a real holder with a real account,
XI-15). It never rounds for you: the kernel rounding somebody's payment would be the kernel deciding
what they paid. Whoever builds the leg decides, with `registry.payable` / `deliverable` (what
somebody CAN pay or deliver: down), `registry.cashFor` (what a value COMES TO: nearest), `upTick`
(what a requirement NEEDS: up) or `shareFor` (a cell's own share, per member).

**Splitting is where the real mechanism shows.** Ten cents shared three ways is four, three and
three: `splitOnTick` gives the odd piece to the largest remainder, ties to the earlier claimant, and
the parts sum to **exactly** the whole. That makes "no residual with no holder" (Law 2) something the
arithmetic cannot violate rather than something the audit reports afterwards. Where two parties trade
and one is a population, the smallest amount they can exchange is `commonGrain` — the least common
multiple of their weights — because a cell of five hundred deals in five hundred pieces at a time.

What is **not** a count: prices, rates and values. A price is a ratio of two counts and rounding
happens where it becomes a payment; a value is an opinion about worth. So Law 7's arithmetic dust
survives exactly where it belongs — in what things are worth — and has left the places where money
moved.

**The world is denominated so that a cent is a sensible piece of it.** A weekly wage is around 940
PHX, a firm opens with 25,000-125,000 PHX, grain is 400 PHX the tonne. Before that a person's week
was two-thirds of a currency unit, at which scale a cent is a fifth of a week's pay: if the world
changes when the amounts are made realistic, the world was wrong.

How fine the pieces are, is a **RESOLUTION** (Law 2), and `test/tick.test.ts` measures it two ways.
Every **structural** invariant holds EXACTLY at the cent, a tenth of a cent and a hundredth of one —
money conserved, holdings summing to what is issued, no residual anywhere, nothing "within" anything.
The **path** is not exactly invariant and honestly so: what a payment or a batch comes to is rounded
to a whole piece and this world's decisions are thresholds, so a firm on the edge of starting a batch
starts it in one run and not the other. What is asserted instead is that a factor of ten finer is a
factor of ten closer, in what the world made and in the money it holds — which says the difference is
the rounding and nothing else, without anybody choosing a band. A number that stopped scaling with
the grid fails that test rather than passing quietly.

**The surface reads back in named units.** `Snapshot.subdivisions` hands the reader how many pieces
one named unit is, and the app divides by it where it prints (`packages/app/src/ui/render.ts`).
Nothing is converted on the way out of the engine: a surface that quietly rewrote the state's numbers
would be a surface with arithmetic of its own in it (Observer E3).

### 4.12 Reproducibility (Seed A5, Audit D3)

The engine takes a seed and a registry; all randomness comes from one injected PRNG (`sfc32`) advanced
in a defined order. The engine never reads the wall clock or `Math.random` (lint). A run is identified
by `(seed, registry hash, periods)`; the same run produces the same instructions, prints and
violations.

---

## 5. Error discipline

Two kinds of wrongness, kept apart on purpose:

| Kind                                                                | Example                                                                                                                                                                                                                     | What happens                                                                                                                                                                                           |
| ------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Contract violation** — something impossible by construction       | adding USD to EUR; a leg with one side; a move of encumbered units; an unpriced read; a rate with no periodicity; NaN; a missing period; a phase reading a print not yet produced; a weight changed outside the five events | **Throws** a `PhoenixError` subclass carrying the spec citation. Never caught inside the engine. The run stops at the site.                                                                            |
| **Invariant violation** — a statement about the state that is false | holdings ≠ issued; money stock moved with no issuer act; equity account ≠ assets − liabilities                                                                                                                              | **Audit finding** with owner and size; reported, never repaired, never thrown (Audit C4; Law 11: deliberately failing checks are the normal state of an incomplete model). Tests assert on the report. |

Rules that follow:

- no `catch` that swallows; no `try` in mechanism code (lint);
- no `console` in the engine (lint);
- no default values for numbers; no optional numeric fields that mean "unset";
- exhaustive `switch` over kinds, checked by the compiler (`assertNever`);
- every public engine function validates its inputs with the value-object constructors, so a bad
  number cannot get past the first call.

---

## 6. Package layout

```
packages/engine/src/
  core/        num.ts errors.ts ids.ts money.ts rate.ts option.ts assert.ts format.ts
  calendar/    calendar.ts periodicity.ts daycount.ts
  registry/    registry.ts params.ts profiles.ts naming.ts
  parties/     party.ts cells.ts
  ledger/      instruction.ts settlement.ts ledger.ts
  register/    instruments.ts holdings.ts lots.ts liens.ts register.ts
  prices/      price-store.ts value.ts
  clearing/    schedule.ts solver.ts outcome.ts
  contracts/   (arrives with the derivative layer: the zero-sum store, Derivative X1)
  audit/       audit.ts families/{money,ownership,prices,accounts,names,flows,units,...}.ts
  journal/     journal.ts events.ts
  world/       world.ts (kernel) module.ts context.ts assemble.ts actions.ts revalue.ts cells.ts
  mechanisms/  <system>/index.ts       one module per spec system or instrument family
  seeds/       <name>.ts               seed modules (foundation.ts)
  observer/    observer.ts
  rng/         prng.ts
  index.ts
packages/engine/test/      mirrors src; property tests under test/property
packages/app/src/          worker.ts (engine host), main.ts, ui/
tools/                     spec-index.ts (parses the spec), check-citations.ts
docs/                      spec/ ARCHITECTURE.md WORKLIST.md RECORD.md COVERAGE.md
```

Mechanisms (Parts V–X) live in `packages/engine/src/mechanisms/<system>/`, one directory per spec
system, each a `SystemModule`. They are added in worklist order only. `docs/PLAN.md` is the plan
for building them.

---

## 7. Citations in code

Every module, mechanism and audit family carries `@spec` tags naming the clauses it implements:

```ts
/** @spec Money C2 C2.a C2.b */
```

`tools/check-citations.ts` parses the spec into an index of requirement ids and fails the build if a
citation does not resolve. `docs/COVERAGE.md` is the requirement → status map (`MET at <path>`,
`MISSING`, `OUT OF SCOPE (reason)`), re-marked in the same change that meets a requirement (App C).

---

## 8. Work discipline (Laws 10–17, Part XIII)

- `docs/WORKLIST.md` is the **one ordered list**. Work the first open item; a new item is inserted at
  the position its dependencies put it, and the record says where and why.
- One bounded change per item; a commit per item; the commit message says what and why.
- `docs/RECORD.md` is a ledger of outcomes, not a diary.
- No measurement, tuning or diagnosis of numbers until Part XII is reached. Deterministic checks
  (lint, types, tests, audit at period zero) are gates, not experiments.
- Never roll back a number. Only a change wrong on its own terms is undone.
- A bound is deleted in the same change that builds the mechanism it covered; until then it is a
  registered placeholder.

---

## 9. Project lint rules (self-correction)

Enforced by ESLint over `packages/engine/src` (rules in `eslint.config.js`, custom rules in
`tools/eslint-rules/`):

| Rule                                                                                                                       | Law                  |
| -------------------------------------------------------------------------------------------------------------------------- | -------------------- |
| `phoenix/no-bounds`: no `Math.min`, `Math.max`, `clamp`, `Math.abs` used as a floor, outside `core/num.ts`                 | Law 6, App B 22      |
| `phoenix/no-numeric-default`: no `?? <number>`, `\|\| <number>`, `= 0` default params for amounts                          | App A Missing values |
| `phoenix/no-magic-numbers`: literals other than `0, 1, -1, 2` outside `core/`, `registry/`, tests                          | Law 2, XI-14         |
| `phoenix/no-kind-branch`: no `=== '<kind>'` comparisons on `.kind/.sector/.industry` inside `mechanisms/`                  | Law 15, App B 48     |
| `phoenix/no-clock-no-random`: no `Date`, `Math.random`, `performance.now` in engine                                        | Seed A5, Audit D3    |
| `phoenix/no-cross-module-import`: a module imports only the kernel, never a sibling module or the world container          | Law 15, 4.9b         |
| `phoenix/no-value-recipe`: no `costShare`, `valueShare`, `costPerRevenue` — a recipe is a physical quantity per unit       | Goods A2.b           |
| `phoenix/no-console`, `no-empty` catch, `no-restricted-syntax` on `try` inside mechanisms                                  | §5                   |
| `@typescript-eslint/switch-exhaustiveness-check`, `no-explicit-any`, `no-non-null-assertion`, `strict-boolean-expressions` | §5                   |

The parameter register is checked at engine start: a placeholder without a named mechanism and
worklist item fails construction.

---

## 10. Deployment

- **GitHub Pages** (`.github/workflows/pages.yml`): on push to `main`, build the app with base path
  `/Project-Phoenix/` and deploy. Every push to any branch runs `ci.yml` (lint, typecheck, unit and
  property tests, citation check, app build, Playwright smoke).
- **Android** (`.github/workflows/android.yml`, manual and on tags): build the web app, run
  `cap add android` and `cap sync` from `capacitor.config.ts` (the generated project is not committed),
  `gradle assembleDebug`, upload the APK as an artifact. Release signing is added when a keystore
  exists; it is not part of the foundation.
- The app ships a web manifest so it installs as a PWA on the Pixel during development; the APK is
  the final form of the same build.

---

## 11. Decisions deferred, with the clause that makes them a decision

- **Inspector or participant surface** (§45 A4): the observer API takes a scope, so both are
  possible; which one the shipped app is remains an owner decision. Development uses the inspector
  scope.
- **Engine performance** (Law 18): storage layout is the campaign's to change later; nothing in
  the API exposes it. Cell arithmetic is per member so a resolution change is a data change.
- **Persistence**: a run is reproducible from `(seed, registry, periods)`, so saving a run is saving
  those; a materialised snapshot format is not needed until the app needs resume.
- **Single currency in settlement**: until the currency layer (worklist 12) revalues foreign positions
  into equity (Currency D2), an instruction may not touch an instrument in a money other than the
  party's home money; settlement throws `Mismatch` (Money A2.b). The guard is deleted in that item.
