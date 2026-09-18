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
| `Cash`   | `pieces`, `ccy` — a value object, not a phantom (16.0)                              | `plus`/`minus`/`sumCash` throw `Impossible('Money A2.b')` where two currencies meet. No implicit currency (Currency A4): every amount names its money, and a record carries pieces beside a named `ccy` (Law 8). `inMoney`/`inOwnMoney` are TRANSLATIONS for a report or a mark at the rate in force, never conversions — a conversion is an FX trade with a counterparty (Currency B3, C4). |
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

`Holding (holder, instrument)` → `{ lots: Lot[], liens: Lien[] }`. A lot is a basis and a date, and
two credits with the same of both are one lot (0g.1: `credit` joins onto a matching last lot; a
re-key and a merge join runs of equal lots; a merge orders the two books by date first, which is
what first-in-first-out means once a holding has two histories). A lot carries quantity and basis
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

### 4.4 Parties: named or cell (XI-15, 0f)

A `Party` is `Named` or `Cell`. A cell has an integer `weight` (a count of members) and holds
**totals**: its holdings, accounts and equity are the cell's whole, and `perMember(party,
instrument)` is a read of the total over the count. A decision is taken for one member
(Households A2.f) from per-member reads and posted as a total (a per-member rule × the count is the
arithmetic of a population, `gridPerMember`/`totalOverMembers`); an instruction leg on a cell is a
total like any other leg. There is no per-member leg, no grain, no divisibility invariant and no
`sameState`.

**A cell's identity is its KEY on a declared lattice** (`registry/lattice.ts`, `PartyKindProfile.lattice`).
The lattice is data: categorical dimensions (region, bank, cohort, tenure, employment state, credit
record; a line and an age for small firms), each owned by the event that moves it, and banded
dimensions (liquid wealth in weeks of expected income, illiquid wealth, spell length; size and
leverage for small firms) whose edges are declared as RESOLUTION and tested by invariance
(`test/lattice.test.ts`: refine every edge by two and the population is the same population). The
seed states the seeded dimensions only; the kernel reads the rest off the holdings and the record
at the seal (`placeCellsOnLattice`) and a dimension it cannot read yet is `unread`, a real state.
**At most one live cell per key**, kept by the kernel: a move onto an occupied key merges
(`reKeyOntoStanding`; a bank move goes the same way).

Movement is the five weight events and nothing else — entry, death, promotion, merge, and the
crossings the kernel reads at the close of `revaluation` (`crossings()`, the one writer of a cell's
position on its bands). A weight event moving `n` members moves `floor(total × n / weight)` pieces
of every holding (`Register.moveShare`) and the remainder stays; the event journals what moved per
instrument and the `flows` family reads it as the explanation. A merge adds totals and weights and
carries what the mover issued (`Instruments.reseat`) and owed (`succeedAgreements`). A cell is
homogeneous by construction — it is what the lattice makes of one population — and nothing decides
on a band INDEX: a decision reads the cell's own quantities, because a rule keyed to a band would
make the band's edge a preference (0f.10).

Consequences the representation enforces rather than checks:

- aggregation is `integrate(f) = Σ f(cell) × weight`; there is no `mean()`, so the average is
  unreachable (XI-15: _a question that cannot be phrased will not be asked wrong_);
- a preference a cell holds between periods (its patience, its memory) is drawn under its
  POPULATION's name — the seeded dimensions of its key — so a member that crosses an edge keeps it
  and the world's answer does not move with the grain;
- lifting a relationship into the key is a data change (Small-Business Pools A6.a): a registry row,
  a lattice entry and the event that moves it; no mechanism and no other kernel type changes.

`cellKeyFaults(registry, cell, scope)` is the one writer of the key rule and has two readers with
opposite postures: `Parties.add` throws on the first fault (scope `seeded`), the `names` and `units`
audit families report each as a finding (scope `all`; the invariant is one live cell per key).
`KEY_DIMENSIONS` in `parties/party.ts` says what the kernel can CHECK about a dimension — where the
same fact also lives on the party (`region`, `bank`) and whether the value must be something the
registry declares — and every other dimension is the kind's own fact.

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

### 4.5b Contracts: the second register (Derivative X1, D1, D1.b)

**A derivative is not a holding, so it is not in the register.** Nobody issued it, it has no issued
amount and no holders, and a row that nobody issued sitting in the ownership identity would be a
defect in every period it existed (X1). It lives in `register/contracts.ts`: `Contract { id, kind,
a, b, terms, ccy, notional, struckAt, basis, opened, state, house }`, indexed by party AND by pair —
because `C1.a` nets per counterparty pair and `G3` forbids any wider number, so the read a party is
entitled to is "what I have with you" and there is no door that adds those up. Nothing ever
collapses two rows: an offsetting trade with a different counterparty is a third row, the market
risk is flat and the credit risk has doubled (`B3.a`), and two strikes on one underlying are two
contracts (`D12`).

**What it enters instead is the zero-sum identity**, and `D1.b` says EXACTLY: not dust, the same
number negated. So the family (`audit/families/zero-sum.ts`) cannot compare the kernel's own two
answers — `b`'s value is minus `a`'s by construction, and comparing them would check that a minus
sign works. It asks the PROFILE for the contract as each side states it: `flip(terms)` is the terms
as the other side wrote them, the profile marks that, and the two must negate. A kind whose mark is
not antisymmetric in its own terms lights this family and no other (Audit B8).

**A kind of contract has a profile like a kind of instrument** (`registry/derivatives.ts`,
`DerivativeKindProfile`): `underlying` (a print this world clears, an index it reads, or a public
event it records — `D3.a` and `G4` are checked when the book opens and again when a row is written),
`mark` and `flip`, `legs` (what the terms put in this period), `premiumPerUnit` (`D7.b`: zero for a
contract struck at par, and then the cleared price IS the rate that makes it so), `initialMargin`,
`closeOut`, `expires`, a `unit` and a `priceTick`. The mark is a function of PUBLIC state only
(`ContractReads`: prints, marks, indices, curves, the measured move, public events) — one contract
has one mark read from two sides (`A3`), and a mark that could see either party's own state would
answer two different things.

**Opening, closing and novating are LEGS** (`ledger/instruction.ts`, `ContractLeg`). Each is a
change of two balance sheets — a row is an asset to one side and a liability to the other from the
instant it exists — and Money D1 says a change of balance sheet goes over the wire. So settlement is
the contract store's one writer as it is the register's, a premium and the row it buys are in one
numbered instruction (Law 5), and `Settled.contracts` hands the drafter back the rows it wrote. The
value at inception is the row's **basis** in Register D4's sense (what the position cost); what the
equity accounts have recognised after that is `prices/contract-value.ts`, on the same
`recognisedFor` switch a lot's carrying value turns on (§4.5), and `world/revalue.ts` books the
change to both sides in the same step as every other mark.

**A `contract` market is the third market kind** (`clearing/market.ts`): same solver, same book, same
print, and what a fill becomes is a row rather than a delivery. Cleared, one trade becomes two rows
— member to house and house to member — so no member ever faces another and the house is flat by
construction (`C2`). The trade is **cut at the strike** to the smaller of the two sides' admitted
shares, with the margin in the same pass (`E2`), and what was refused is journaled
(`derivatives.refused`) and measured, never accommodated (`E4`). Neither the cut nor the margin is
the kernel's to compute: **exactly one module declares `clearingCapacity`** (`world/module.ts`),
because what a member keeps back is its own preference and what a margin claim IS is that module's
instrument. A world with a contract book and nobody answering cannot clear one.

### 4.6 The clearing engine (Clearing A–F)

One solver for every market. A participant posts a `Schedule`: a monotone step function from price to
signed quantity, built from the participant's own state (A3) and never from the clearing price (A4).
The solver finds the price where posted supply meets posted demand (C1), rations by the market's stated
rule (C3), and returns one of the representable outcomes (C4.b):

`cleared | noDemand | noSupply | noOverlap | excessCommitted`

A search bracket is never returned as a price (C4.c). Trades are emitted as instructions (D2, D3), and
the print becomes the mark (D4). The solver is a pure function of the schedules (C5) and is tested for
determinism with property tests.

**The posted curve** (`clearing/schedule.ts`, Clearing A2, A2.a). A party spends the same money
whatever the price, so what it wants at a price is its budget divided by it — a curve, and the size on
a limit order is the EXTRA that level adds, so a book adding up the orders at or above a level sees
exactly the curve there. `rungsOver` samples it, `rungsUpTo` samples it under a want or a published
limit, and the LEVELS come from three separate reasons: `pricesOver` (what a consumer expects to be
charged, §46 B3), `levelsBelow` (down from what a saver will pay, over its own width, Equity B3) and
`levelsUpTo` (down to nothing, for a bidder that is stopped by a size rather than a price). Both ends
of every span are numbers the party itself named and the STEP COUNT only samples between them — which
is what makes the count a RESOLUTION (Law 2, `A-32`: `levelsBelow` used to run `top × k/steps`, so its
bottom was `top / steps` and the count decided how far down a saver bid at all). It lives beside the
solver rather than inside `households` because a module never imports another module and three of them
post curves: the household's basket, the saver's ladder, and a bank bidding for a securitisation note.

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

A period is an **ordered list of phases** held as data, each with a name, its spec citations, an
**anchor**, what it **reads** and what it **writes**, and a `run(ctx)`. Markets clear at their
stated point (Clearing F1). Every period ends with settlement of the last cycle, then revaluation,
then the **audit** (Audit C1–C3). The loop is the same every period; phases are never skipped
conditionally (Audit C3).

**The anchor places it; the declaration checks the placing** (item 0a). A module says where it sits
against the three kernel acts — `corporateActions`, `markets`, `revaluation` — and that is not
derivable: `goods.spoilage` runs after the period's trades and before the marking, and no read or
write says so, because it reads holdings and writes holdings exactly as forty other phases do. What
IS derived is the **settlement cycle**, which is the anchor's (Money G2), so a module cannot state
one its own anchor contradicts — which is what `paper.backstop` did, with `cycle: 2` in front of a
cycle-0 anchor.

A dependency carries the period it is of. `thisPeriod` says the writer must already have run and is
the only thing that is an edge; `anyPeriod` is a read of history and orders nothing — eight phases
in this world read the very kind they write, and a check blind to the period would call each a
cycle. `world/order.ts` refuses, at the seal, a phase in front of a `thisPeriod` read's writer, and
a `thisPeriod` read nothing writes. At run time an **undeclared** read throws `Forbidden
'Clearing F1.a'` at the site, naming the phase and the kind.

This paragraph used to say that a phase reading an unproduced print got `NotYetProduced` rather than
a stale value. **That was false**, and the falseness is what stop 18 was: `lastOf` answered with
last period's event and `latest` with last period's price, so `reporting.publish` published a
company's worth from the week before and nothing complained. It is the seal that refuses the order
now, where it is a fact about two phases rather than an accident of which party was asked first.

**Almost nothing in this world reads the period it is in.** Of eighty-four phases, nine do; the
order that existed satisfied all nine. The declaration is a guard, not a re-ordering — which is what
two of item 0's stops needed and neither had.

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

- **Everything about a party is that party's TOTAL** (0f.1 for the register, 21a for the equity
  account). Holdings, cash, liens, the equity and revaluation accounts and every effect on them are
  what the whole party has; a member's share is a READ over the weight (`perMember`), and a named
  party is a cell of weight one. It was the other way round until 0f.1, and the years in between are
  the record: mixing the two was the first defect the audit caught and it was still catching it at
  21.101, 21.104 and 21a. There is one denomination now, and `Total`/`PerMember` in the type system
  is what keeps it that way.
- **A fact is declared, or it is counted** (0i). `registry/facts.ts` is the third register:
  `params` is to numbers what `nouns` is to categories and what this is to EVENTS. A fact names its
  fields and their kinds; `Journal.say` takes a payload typed by the declaration and `says` reads
  one back and throws if it does not match; a phase declares what it writes (`Produces.fact`) and
  assembly refuses two writers of one kind who disagree. There are no optional fields — a field that
  may have no value declares `orNone` and its writer writes the absence. A kind nobody has declared
  is counted, not refused, and `check:forbids` publishes the count and refuses a rise, so the
  migration is incremental and monotone. The kernel's thirteen are declared; the modules' are done
  per sector as each sector's item comes up, deleting that module's hand-written accessors with it.
- **A weight event that moves holdings states the arriving party's account as the READ** (Seed C1,
  21a). The register partitions lots in whole pieces (`floor(total × members / weight)`, remainder
  with the people who stayed), so what arrived is not a proportion of anything — `world/cells.ts`
  reads the one `balanceSheet` of what the arriving party now holds against what it owes and takes
  that same number off the source. It is read at `Valuation.carryingOfLots` — what the accounts have
  recognised — because a weight event happens around this period's markets and a mark that has not
  printed yet is refused (Clearing F1.a).
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
- **What this does not record, and what will.** `moveEquity` takes a `cause` for every move and keeps
  only the running balance, so comprehensive income is recoverable exactly — Δequity decomposes into
  revaluation plus the instructions' `EquityEffect`s — and nothing above the bottom line is. That is
  deliberate here: an accounting classification chosen by whoever wrote the leg would be the "nature
  flag" this section refuses. What is missing is not a classification but a **record**: the causes are
  written and discarded. Worklist 12a adds the **equity ledger** — the moves kept as append-only
  entries with the cause their writer already passes — so an income statement is a read of recorded
  events (§48 A2) rather than something recovered by parsing `reason` strings (Law 19). The entries
  are the itemisation, never the balance: `equityWalk` stays authoritative and the accounts family
  compares the two as independent records.

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

#### What a module may extend, and what is closed to it (13b.1)

`SystemModule` has grown a single-answer hook every time a module needed the kernel to ask a question
it had not asked before: `curveFamilies` (sovereign curves), `indices?`, `outlooks?` (XI-16),
`marks?` (Banks Lending D1), `creditDecisions?` (Money B3.a), `bankChoices?` (Banks Funding E1),
`venueParticipants?` (the labour venue), `resolves?` (bank resolution), `derivativeKinds?` and
`clearingCapacity?` (13a), `borrowNeeds?` (Securities Lending B1, item 9.4), `seed?`. Every one is
argued correctly in place and the argument is the
same each time: the answer belongs to the module that owns the party or the instrument, exactly one
module may answer, and a world where nobody answers must not seal. The problem is not any one hook —
it is that inventing a hook was the only way a module could teach the kernel a new question, and
inventing one is a kernel change that nothing said was allowed.

Measured on the three commits that built item 13: **13a — a new SHAPE of state — cost 24 kernel files
to 7 module files; 13b — seven CLASSES of that shape — cost 6 to 24.** The architecture is
plug-and-play for instances and costly for shapes, and that is largely irreducible: a contract
genuinely is new state, and new state means a store, a leg, an op, a value and an audit family. What
was avoidable is stated at 13b.1.

**The rule, taken as a decision rather than drifted into:**

- **Open to module extension**, under the single-answer rule assembly already enforces: `SystemModule`
  hooks, kind profiles a module registers, params, units, phases, participants, venues, audit
  contributions, index rules, curve families. A module adds a row; the kernel collects and never
  branches (Law 15).
- **Closed**: `core/ids.ts` (a module brands its own key rather than adding a kernel brand),
  `registry/kinds.ts`'s profile shapes, `MarketRunDeps`, and the contexts in `world/context.ts`.
  A module that needs something one of these does not have raises it as an inserted worklist item with
  the argument, the way 11.6 and 13b.1 were.
- **`registry/` is data.** It may not import `world/` or `clearing/`. A profile field that needs a
  party's private view belongs on a module the owning layer collects at assembly, not on a registry
  profile — which is what `DerivativeKindProfile.orders` got wrong and 13b.1 corrected.

**What 13b.1 did to each of the four, and each one is the rule made true rather than restated.**

- **`core/ids.ts` is closed, so a module brands its own.** `ModuleKey<B>` and `moduleKey(s, what)`
  give a module a distinct branded string in its own file: `ModuleKey<'Employment'>` is a different
  type from `ModuleKey<'Route'>` and from every kernel brand, and the kernel never hears about
  either. Labour's `EmploymentId` is the first. Five of the twenty-four kernel files item 13a cost
  were this file growing a brand per module.
- **`MarketRunDeps` is closed, so what a kind of market needs goes in the kind's own row.** `admits`,
  `marginLegs` and `derivativeKind` were three fields on the deps every market ever run is handed,
  for the one kind that uses them; they are `MarketRunDeps.kinds.contract` now. Asset and fx markets
  need nothing beyond the book, and the absence of their rows says so.
- **`MarketDecl` is a discriminated union**, `AssetMarketDecl | FxMarketDecl | ContractMarketDecl`,
  where it was one shape with three optional bags. A contract book naming no contract and a pair
  market naming no pair were states the type allowed and two runtime `throw`s forbade; a state the
  type can forbid is not a state to check for. `delivers` joined `trade` as a row in `MARKET_KINDS`,
  because it had been testing the two bags for ABSENCE and inferring the kind from that (Law 19).
  Modules ask `contractOf`, `pairOf` or `asContractMarket` — kernel reads, one each, so that no
  mechanism writes the discriminant test itself (`phoenix/no-kind-branch` refuses it there).
- **`registry/` imports no `world/` and no `clearing/`, anywhere.** `DerivativeKindProfile.orders`
  and `.measures` became `DerivativeClassDecl`, declared by the module that owns the kind and
  collected at assembly into a table the world exposes as `derivativeClass(kind)`. The dispatch is
  unchanged and so is its reason — one party shows one face to one book (Clearing A2), so the layer
  declares the participant once and asks the class the book carries. `registry/physical.ts` went the
  same way: the plant read states the shape it needs OF a holder (`PlantHolder`) instead of taking a
  participant's whole view. **A read of a PUBLIC fact that several modules need lives here for the
  same reason**: `storageRateIn` (what a piece of room cleared at) and, since item 10e.4,
  `registry/wages.ts` (what an hour costs the employer asking — its own last payroll first, the
  published going rate second, nothing where it has neither). A firm, a bank and a fund manager all
  need that answer and none of them may import the labour module, so writing it three times is what
  the alternative actually meant — and two of the three copies had already drifted apart on which
  key the going rate is published under (`E-19`). Each takes the NARROW shape it needs of a reader
  (`SessionReads`, `WageReads`), never a participant's whole view.

Modules reach the kernel only through three contexts (`world/context.ts`), and nothing else — and a
context hands out **facades, never a store**. `SeedContext` was the exception until 13b.1: it held
`Parties`, `Instruments` and `Register` themselves, so a seed could have applied a weight event,
restated a line or moved units with no instruction behind them. What a seed legitimately does is
STATE the opening (Seed A2, C4), so `add`, `credit`, `debit` and the money pair `endowMoney` is
built from are in its `Pick<>`s and the rest of each store is not.

| Context            | Who gets it                                                 | Can                                                                                                                                                                            | Cannot                                                                       |
| ------------------ | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| `ParticipantView`  | a party, when a participant declaration is evaluated for it | read its own holdings, cash, equity; who anybody IS (kind, region, bank, weight); public prints, public instrument terms, public events, its own record; its own random stream | see any other party's private state (Observer A4, Expectations D1)           |
| `MechanismContext` | a module phase                                              | read public state and any party's own view; settle instructions; register instruments and markets; apply cell events; cease a party; journal                                   | write the register, write a print, write a weight, reach the world container |
| `SeedContext`      | a seed module at period zero                                | add parties and instruments; endow money and units; write opening prints; open markets; ask the kernel's one valuer what a holding comes to                                    | hold a kernel store; apply a weight, restate a line, pledge; anything after the seal |

**THREE TIERS OF INFORMATION, not two** (Observer A3, A4; Reporting A1.a; item 17.0a). The two the
kernel had were PUBLIC (an event recorded `isPublic`) and OWN (what a party is a subject of, through
`lastOwn` and `visibleTo`). The third is DISCLOSED: `ctx.disclose(from, to, event)` records that one
party showed another one of its own events, and `view.disclosedToMe(kind, from)` reads it back. The
event itself does not move and does not become public; what is recorded is that it was shown, so
what a lender knows about a borrower is on the record and checkable. The kernel refuses a disclosure
of an event its shower is not a subject of.

What this is FOR is credit: every company prepares a full quarterly statement
(`mechanisms/reporting/statement.ts`, with `registry/statements.ts` settling what each line means
and `journal/published.ts` the kernel's read face onto the one parse), and whether anybody may read
it is a separate fact. The module that owns a RELATIONSHIP writes the disclosure, because the reason
to show is the relationship's: reporting shows a statement to the lenders of record and the banks
that keep the accounts, ratings to the assessor the issuer pays, the kernel to whoever a borrower
asks. The rule that door exists for is the one a module can break silently — reading a counterparty's
own view with `ctx.participant(x)` gives a mechanism private state no participant may have, so the
count of those reads per file is ratcheted in `tools/check-forbids.ts` and may only fall.

**A PREFERENCE THAT EVERY KIND OF PARTY CAN HAVE LIVES IN THE PARAMETER REGISTER** (XI-14; Trade
Credit B5; item 17.7a). A firm's hurdle and horizon are drawn at its birth and declared under its own
name, and that worked because firms have a module that draws for them. A seller's terms are the same
kind of number and sellers are not one kind of thing: a firm, a small-business cell and a merchant
all ship on terms, and no module draws for all three. The register is the home they share — keyed by
the party's name, declared once with a unit and a reason, readable by anybody, and `params.has` is
what lets a caller declare on first use instead of asking for a number that is not there yet. The
module states the WIDTH (`mechanisms/<system>/data.ts`, a `Spread` with its own why) and never the
number; the party's own stream, derived from its name, decides where in the width it lands.

**ONE DOOR OUT OF TERMS FIXED AT ISSUANCE, and the kind holds the key** (Banks Lending E3; item
17.7). Terms are the structure of an instrument and not an opening condition (Seed C4.b), so nothing
could change them: a claim that stopped performing stayed in default for the rest of its life and a
claim being paid fell due on the day it was written for. `ctx.reagree(instrument, terms, why)` is the
one path, `Instruments.reterm` the one writer, and it is narrow in three ways rather than general,
because a general terms-restatement is a way for any module to rewrite any line. The KIND declares
`reagree(was, now)` and returns the reason to refuse, so a kind that declares nothing cannot be
re-agreed at all — a share is not renegotiated and a bond's restructuring is an exchange offer to its
holders, not a private word with one of them — and a loan refuses everything that would make it a
different claim (the two parties, the drawing, the convention, the security, a date brought forward).
The STATUS decides which event it is: `rolled` is a performing line extended at maturity, and
`restructured` is one that stopped performing, and the kernel refuses the other pairing so a reader
counting workouts never counts rolls. And the line performs on the terms that stand — one sentence,
not a branch, and the only path that restores what `markDefaulted` took away. What is forgiven is a
redemption at what it fetched and leaves by the ordinary two-sided leg, not through here.

**The opening world is TWO seed modules, and the second one is why** (Seed A4, C1; item 12). Who
exists and what each party is endowed with is one question; **what stands behind a bank** is another,
and it cannot be answered until every module has handed out what it hands out. `equity` opens every
listed line and `funds` gives a bank a launch of an exchange-traded fund, and both need the parties
the foundation creates — so both seed after it, and a foundation that funded a bank against the
assets it had endowed itself funded it against the wrong number.

**And who opens holding a listed line is the SAVERS** (item 11.5; Equity A1, B3; Seed E1). `equity`
used to give it to the banks whose desks make its market, and that was a seeded outcome: a share is
a claim on the residual and a dealer's inventory is a position it takes by TRADING, so ownership of
the float was being assigned in advance to the one party whose holding of it is supposed to be a
market's result. It also put an asset the central bank's window will not take — four times a bank's
own capital of it — on the balance sheet whose whole liquidity is what its assets raise there, which
is what kept the banking system from lending at all. `seed.funding` runs
last, reads each bank's assets off the register (Law 19), and derives what its depositors hold from
the line that bank runs to: the regulatory minimum plus its own buffer, which is the only number in
it and is one each bank had already declared. It requires only `seed.foundation` — naming the
modules it must follow would make a world assembled without them unable to fund anybody at all.

**A kind's PROFILE belongs to its module; a kind's ID belongs to the kernel** (item 11.6). A profile
is behaviour — how a kind is represented, the ways it can fail, whether it borrows, what class of
depositor it is, whether it issues money — and 4.9b's "a kind is owned by exactly one module" is
about that. An id is a NAME, and a module that does not own a kind still has to say it: `labour`
posts openings for firms, `ratings` charges them, `equity` opens a line on one. So the ids sit in
`registry/profiles.ts` with the kernel's own, and `firm` and `household` are declared by `firms` and
`households` — naming them from the owning module instead would be the cross-module import the lint
rule forbids. What that buys: the guard that says a module declaring a depositor must say how it
leaves (Banks Funding A1.d, E1) now covers every depositor there is, with no exception list.

**Schedules come through one door, in a market and in a venue.** A market gathers its orders from
the `participants` modules declare per party kind, each evaluated with that party's own
`ParticipantView` (`world.ts` `runOne`). A participant may also declare `markets(view)` — which
books that party is in at all this cycle — and the kernel builds a per-cycle index from it and asks
only those (Law 18). The kernel cannot guess the answer, because which books a party is in is its
own business and changes period to period; and the answer is a READ of the same published plan the
orders come out of, so the two cannot disagree (Law 4, Law 19). Declaring nothing means every market
of the participant's kind, which is what every participant did before the door existed.

The derivative layer speaks for a party in every contract book (one face per book), so its `markets`
answer has to cover every class at once. It asks each class, through two doors on
`DerivativeClassDecl`: `subject(m)` — what a book is written ON, as a name — and `reasons(view)` —
the subjects this party could have a reason about, read off its own state. The kernel keeps the
books indexed by (class, subject), rebuilt whenever a book opens (`World.contractBooks`), so a party
names its books without walking the world's. A class declaring neither is asked about every book of
its kind, as before. The measurement that asked for it: a real period put **13.5 million questions**
to firms about option books and got **not one order** — what an option is worth to a party is its own
outlook's confidence about the underlying, and a party with no view of a line has nothing to say
about optionality on it, which every firm discovered separately for every line every period.

A VENUE — where something is struck that is not the
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

**Where a depositor banks is the depositor's decision, and it comes through the same door**
(Banks Funding E1, Observer A4, item 12). A module declares `bankChoices` per party kind it owns —
`chooses(view)` answering where that party would rather bank and why — and the module that runs the
deposit market calls `ctx.chooseBanks()` once a period after it has published the boards. It is the
venue door (§4.9b, item 11.1) pointed at a decision rather than a schedule, and for the same reason:
the money market used to walk every party in the world and decide for each of them out of a context
that can see private state no depositor may have. The reasons are not one reason under three names —
A1.a's retail money is insured and moves for the rate it is not being paid, A1.b's corporate money
banks where it transacts and moves off a bank that drew the window, A1.c's wholesale money is in the
market all day and leaves the one its own session refused — so each is written where its kind lives,
and E4.a (a run is a wholesale phenomenon first) falls out of who sees what, rather than being
stated.

**Why a party is short comes through the same door** (Securities Lending B1, Observer A4, item
9.4). A module declares `borrowNeeds` per party kind it owns — `needs(view)` answering what that
party must deliver that it has not got, how much it will pay per period for the use of it, and what
it will pledge — and the securities-lending module calls `ctx.borrowsWanted()` once a period, groups
the answers by LINE, and clears one book for each. The division is the one every door on this list
makes: the lending module knows how to strike a fee, pass title, bind collateral and manufacture a
payment, and knows nothing whatever about why anybody wants to be short — that is a position taken
out of a party's own view of a line it holds none of, and it lives in the module that owns the
party (today: a dealing desk whose own view of a line is below what the book last printed,
`banks/dealing.ts:deskBorrows`).

Without the door the whole system was unreachable and had been since it was built (`A-67`, `B-3`):
`runBorrows` and `wantsToBorrow` were exported and called by nobody, the one phase walked a book
nothing ever pushed to, and E1's *no short without a borrow* held vacuously because there was no
short in the world for a borrow to be behind. The alternative — the lending module walking every
party and deciding for each whether it should be short — is A4's prohibition exactly, and it is what
made `wantsToBorrow` a function that could have no caller: a module never imports another module, so
the only party that could ever have called it was one securities-lending owns, and it owns none.

**Two more reads, and why they are reads and not imports.** `ParticipantView.mark(instrument)`
answers what a unit of a line is carried at — the print, or, for a claim on a book, what that book
comes to (§4.5). `ParticipantView.lastPublicAbout(kind, subject)` answers the question a participant
actually asks: not "what was the last dividend anybody declared" but "what did THIS issuer declare".
Both are public by construction — a print is public by Clearing E1, a derived value is arithmetic on
a register anybody may read, and a public event is public — and both exist so that a module reading
another system's decision reads the event it PUBLISHED rather than importing the module that took
it. A saver values a share off `payout.declared` and a desk arbitrages off `etf.struck`, and neither
knows which module wrote it.

**A third: `ParticipantView.inOwnMoney(value, from)`** — what an amount in another money comes to on
THIS party's own book, at the rate in force this period. A balance sheet is kept in one money
(Currency C4.a) and adding two of them is a defect, so anything that walks a party's holdings and
sums them converts here. It leaks nothing: an FX rate is a print and prints are public (Clearing E1).
The same read is `Valuation.inOwnMoney(party, value, from, at)` on `MechanismContext` and
`DerivedReads`, and settlement's own `inOwn` is now a call to it rather than the conversion written
out a second time (Law 4). Which money a party's book is in is a fact about its REGION, injected into
the valuer as `bookMoneyOf` the way the curve and the calendar are, because the parties store is
built after it. `worthOf` carries the currency of its answer with the value for the same reason
(Law 8): six readers summed those across a book without it.

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

**A batch takes time, and what is between the input and the output is a thing** (§37 B3, 21f.3).
`Recipe.periods_to_make` is a TECHNOLOGY primitive like the batch size, and it is what makes there be
an in-between at all: `Making` draws the inputs in the period it decides, puts the batch on the line
in `stores::InProgress` — owned by a named maker, carrying what it cost — and proposes the `Create`
leg only in the period the line is done. Until it had a length, production drew and created in ONE
instruction, so nothing was ever in progress and B3 was a clause whose type existed and whose
instances did not. The cost travels with the batch rather than being recomputed when it lands: what a
unit cost is what went into THAT batch, not what its inputs were worth when it finished (Law 4).
`InProgress` is indexed by the period a batch is ready and a taken batch leaves that index, so what
comes off the line is a lookup rather than a walk over everything the world has ever made.

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

#### What a module knows of another module (0e, 0e′)

**Nothing, by name.** A module never imports a sibling (`phoenix/no-cross-module-import`), never
names a sibling's event kind (`tools/check-forbids.ts`, a FORBID since 0e′.3 — its ratchet closed
52 pairs across 31 kinds and is now an empty set), and never reads a sibling's store. What it may
know of another system comes through exactly three doors, and which door is decided by what the
fact IS:

- **A read of a public fact is a registry read.** What a market published, a bank said about
  itself, a firm said it is short of, an assessor graded: `registry/wages.ts`, `registry/banking.ts`,
  `registry/funding.ts`, `registry/notices.ts`, `registry/physical.ts`, `registry/switching.ts`,
  `registry/expectation.ts` (own outlook, else the tape) and `registry/capital.ts` (what a project
  must earn and the project itself — one arithmetic for a named firm and a cell of small firms, 11.2a.2).
  The registry names the event kind and does the fetch; a caller passes a party or a party's door
  (a narrow `Reads` interface, never a whole view) and gets the fact back in its own dimension. The
  extraction is written once. It was written twice or three times before, every time, and the
  copies had drifted apart on which key a rate was published under, which country's wage a state
  read, and which half of a funding gap a channel funded (0e′.1–0e′.3).
- **A behaviour the kernel needs from a module is a question** (`registry/questions.ts`, 0e): the
  module that owns the party or the instrument declares the answer, exactly one may, and a kind
  that needs one and has none refuses to seal. A question is asked BY the kernel — an overdraft
  decision, a valuation, an outlook, a measure the observer shows (`SystemModule.measures`,
  0e′.5) — and is never how one module reads another.
- **A request is an event with one kind and a kernel stamp** (`ctx.request` / `ctx.requests`, 0e):
  what a borrower is short of is published once, under one kind, with the party and the period
  stamped by the kernel, and a lender reads one thing. It says WHICH OF TWO THINGS it wants
  (`wants: 'money' | 'commitment'`, 17b.1): the money, or a lender that has agreed to lend and has
  not lent. The decision behind the two is the same decision — the same bank, the same price, the
  same room — and only what it produces differs, so it is one field on the one door and not a second
  door. A commitment is a `FACILITY` agreement (`registry/credit.ts`) whose undrawn limit consumes
  the lender's capital through `headroom` and which lapses if it is not drawn: the money it promises
  is made inside the instruction that draws it, which is what lets a deal be conditional on it
  (§29 B2, E1). It also says HOW LONG it wants the money for (`months`, 17b.8): a term is a decision
  about a need and the need is the borrower's, so a mortgage runs for decades and a week's working
  capital for a year, and the row is written for what the ask said. The lengths themselves are
  declared by the module that owns each product, with the ids of the ones more than one module has
  to say in `registry/credit.ts` (`TERM_MONTHS`) — a mortgage is asked for by `housing` for a
  landlord and by `households` for a family, and one product may not be two lengths (Law 4). And a
  borrower asking for a COMMITMENT has its BOOKS prepared for it (`reporting.interim`, 17b′): every
  company already prepares full quarterly financials (17.0a), but its first close is one to four
  quarters after the world opens, and in that window it has nothing to show a lender. So it prepares
  MANAGEMENT ACCOUNTS as at today — the same statement over a span that is not a quarter, private
  always, shown through the same door the quarterly one is plus to every bank that has quoted it —
  and the lender reads them in the next period, when it decides. **No accounts, no commitment.**
  `reporting` is not a `requires` of `banks` and cannot be: §48 reads what banks publish about their
  own regulation, so the two need each other and `requires` is a DAG. What orders them is the phase
  graph, which is where a mutual need belongs.

**The observer is under the same rule and has no sibling of its own.** It imports the kernel and the
registry and no module (`phoenix/no-cross-module-import` covers `src/observer/`); a measure it
shows that only a module can compute — the desks' consensus on a company, what a party has hedged
in a pair — is that module's answer to a world-scoped question, and a world assembled without the
module shows no such measure rather than a guessed one. The occupation tables it once reached into
two modules for are `registry/occupations.ts`, which is where data lives (Law 15).

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
the firm's** and lives in `mechanisms/registry/capital.ts`, because it is made of the same things every
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

All data lives in the **registry**: currencies (each naming its issuing central bank), **countries**
(each naming its currency), **regions** (each naming its country), units, party kinds, instrument
kinds, cell-key dimensions, platforms.

**In the Rust kernel this is `src/registry.rs`, and it arrived at 21e** — until then `CurrencyCode`,
`RegionId` and `UnitId` were bare row numbers with nothing behind them and the party kinds were
integer constants with no profile, which the ontology register carried as four homeless nouns. What is
built is the four clauses those ids exist to carry, and no more: **a currency names the party whose
liability it is** (`Registry::currency(issuer)` is the only constructor, so a money nobody owes cannot
be written); **`currency_of(region)` reads through the country**, so a region keeps no copy of its
money and Seed B3 stays literally true with one writer; **a unit says what one of it is divided into**,
so a dwelling counted in whole dwellings and a tonne milled a million ways are the same mechanism with
different data, rather than one grid for everything; and **a party kind has a profile the kernel asks**.

The profile is where the pressure was. `World::admit` could only say *a party banks at somebody who
issues money, OR at nobody at all* — a blanket escape that let any party be admitted with no bank,
because the rule it wanted (a central bank banks nowhere, a treasury at its central bank, everybody
else at a commercial bank) is a fact about the KIND and had nowhere to live. `admit` now asks
`KindProfile.banks` and enforces each case; a kind with no profile falls back to the old permissive
read, because `Missing` is missing and a world that has declared no profiles is not one the guard can
speak for. The profile holds what the kernel already asks and grows as items need it: one invented
ahead of a reader would be a store nothing reads.

The geography below — tiles, terrain, resources, sea areas — is not built in the Rust kernel yet.

**A country has the money; a region is a place** (13c.1). The two were one declaration until the map
landed, and a map needs many places per currency. A country is one currency, one central bank, one
treasury, one sovereign line, one FX pair and one equity index — it is what §39 Cross-Border and
Indices D1 mean by "region". A region is where a thing IS: its own ground, its own plant, its own
labour venue (one per `(region, occupation)`), its own goods prints. `RegionDecl.ccy` is gone and
`registry.currencyOf(region)` reads through the country, so one fact has one writer (Law 4); Seed B3
and Currency B1 stay literally true, because a region still determines its money uniquely. A region
and a **sea area** are both `PlaceId`s — every tile of the world belongs to exactly one place,
water included, because weather is published per place and a thing at sea has to be somewhere for
the weather to reach it. `PlaceId` is the union of the two brands rather than a third, so a region
goes wherever a place is wanted and narrowing back is a read against the registry, never a cast.
The grid itself is `registry/geography.ts`: the kernel owns the name and the reads (4.9b), the seed
owns the draw, and nothing in the period loop writes it.

**A tile's characteristics are Law 2 TECHNOLOGY primitives**, not derivations. Every tile carries
its place, a MIX of terrain shares summing to one, its elevation in metres, and a deposit of every
declared resource — all drawn, one writer, many readers. A mix rather than a type because a
fifty-kilometre square is not one thing, and because the mix takes every threshold out of what
follows: crossing costs `Σ share / kmPerDay` (you cross all of it), what gets through a gale is the
worst ground on the tile, what a hectare yields is the share-weighted quality. Water is a terrain
kind like any other. `TerrainDecl` and `ResourceDecl` are registry tables and **adding a row is the
whole of adding a terrain or a resource** — an assembly fault requires every resource to say what it
does on every terrain, so an incomplete row is refused rather than discovered later as a hole.
Every declared resource is drawn on every tile whether or not a recipe consumes it, because the map
must be STABLE: a world that re-draws when a line is added is not a world.

**Favourability is a read, never a stored score.** What a hectare yields, what plant costs to erect
here, how fast a voyage crosses and what survives a gale are arithmetic over those primitives,
computed where used. Which industry sits where is a firm's own decision reading them, never a
weighted coefficient.

**The voyage store** (`register/voyages.ts`) is the second thing the map made necessary. What is on a
voyage IS a holding — units of an in-transit instrument on the shipper's own book, Freight A3.a's
working capital — and the register owns that. What the register cannot say is WHERE those units have
got to, because a holding is a quantity and not a position. Two facts, two writers (Law 4). A
voyage's position is `kmTravelled` along its tile path, so the tile it has reached is a read and the
place that tile is in is what the weather searches. Its hulls are held by a LIEN rather than moved:
the register already refuses to move encumbered units, so the lien is the whole of "a hull cannot be
sold, sent on a second voyage, or counted as capacity twice" with no rule written (Freight E2,
Law 12). It is written only by settlement, through one `VoyageLeg` with four acts, like the
contract store.

**Nothing about a leg is declared.** A leg is every ordered pair of places a carrier of one kind can
get between, read off the map with its kilometres and its days and held per map (the draw is the
map's one writer, so that is memoisation and not a copy). A carrier boards at a PORT, which is not
declared either: it is where the ground a hull crosses meets the ground a lorry does, so a landlocked
place simply has none. `RouteDecl`'s `transitPeriods`, `unitsPerVesselPerPeriod`, `sailsIn` and
`sailsHardness` are deleted and each names its read. A
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

**Whether a line is carried at a mark or at cost is a question about the LINE, not only about its
kind** (item 10f.1, §29 C5, C5.a). A kind says how its lines are priced; whether one of them has a
MARKET is a fact about the line. A share of a private company is the same instrument as a share of a
public one and nobody trades it, so nothing ever cleared a price of it and its holders carry it at
what it cost — *"marked, not cleared"*, which is C5.a kept by there being no price to mistake for
one. `Valuation.atCost(instrument)` is the one reader of that (`carry === 'cost'`, or a cleared kind
whose line has no market), and every valuation asks it: `worthOf`, `valueOfLots` and the revaluation,
which skips such a line entirely rather than throwing on a print that was never going to exist.

A **party kind** states four things about its life beyond its representation and its money issuance
(item 7). `fails` says what a party of that kind can FAIL on — nothing, a cash failure it cannot
cure, its liabilities past its assets, or both — and a kind that names neither cannot die, which is
how XI-3's two exceptions (the central bank, and a treasury in its own money) are named consequences
rather than omissions. `terminal` says whether it may end the chain of successors, which only an
estate that has paid everything away may. `borrows` says whether anybody lends to it at all: a going
concern yes, an estate and a household no (Banks Lending A1, Households C1.d). `depositClass` says
which kind of depositor it is — many and small, fewer and operational, few and very large (Banks
Funding A1) — or that it is nobody's deposit base. All of them are read by mechanisms and never
branched on by kind id.

**Where a party of a kind BANKS is not one of them, and that is the point** (item 12). It used to be
a `choosesBank` flag beside them, and the flag was a claim sitting apart from the mechanism that
implements it: a kind could say it chose and have nobody who said how, and then it was a depositor
nobody ever asked — stickiness that cost nobody anything, which is the defect A1.d names arriving as
an omission. A kind chooses its bank if and only if the module that owns it declared a `bankChoices`
reason for it (§4.9b), which is the fact and its answer in one place (Law 4).

**Three questions about a party are answered by the module that owns its kind, never by the book
asking** (items 9.7, 13.3, 13.2b): whether it MAY take a position in a contract of a kind
(`mayTrade`), whether it MAY owe money at all (`mayBorrow`), and WHAT IT HAS BEHIND a position it
takes on its own account (`standsBehind`). All three have the same rule — exactly one module answers
for a kind, and a kind nobody answers for gets the honest default, because the absence of a rule is
not a prohibition. The defaults are: trade anything, whatever the party kind's own profile says
about borrowing, and the party's own equity account. **The third exists because that default is
wrong for a pool by construction**: a fund's equity is zero (Fund Shares A3), so every contract class
that sized a speculative position by `view.equity()` gave a hedge fund a position of nothing in every
book in this world — the door §28 C1 needs, open, with nothing to say at it. What stands behind a
pool's position is its investors' money, and only the module that runs pools can say so.

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

#### The lattice (XI-15, 0f.3)

A cell's identity is its **key on a declared lattice** (`registry/lattice.ts`). A kind that is a
population declares its lattice on its profile (`PartyKindProfile.lattice`): **categorical**
dimensions — a fact with a name, each owned by the one event that moves it (`movedBy`) — and
**banded** dimensions — a quantity read per member off the cell's own state against edges that are
RESOLUTION parameters, one edge per parameter with its own `why`. The dimension names are data; the
kernel checks the three it can check against something it declares (region, cohort, bank) and takes
every other as the kind's own fact. The seed supplies the dimensions it knows; at the seal the
kernel places every cell on the rest (`World.placeCellsOnLattice`): a categorical dimension the seed
did not supply is READ off the opening record (no hire is unemployed, no default is a clean record),
a band is read off the register, and a quantity that cannot be read yet — a band on expected income
before any outlook exists — is `unread`, a real state and not a default. From the seal a key moves
only by the five weight events and by the crossings the kernel reads at the close of revaluation
(0f.4).

### 4.10a What a module knows between periods, and within one

A module's own register — employment rows, a book of invoices, a party's outlooks — lives in a state
slot (`ctx.state(name, initial)`), keyed by the module that owns it, declared in `registry/nouns.ts`
as `noun | working | physics` or it does not open, and snapshotted by the observer as the data it
is. Two rules keep it honest: a slot is never a second copy of what a kernel store already holds,
and a module that must also **audit** its own register builds the object once, per world, and hands
the same object to its phases and to its audit contribution — one book, one writer (Law 4). That is
why `goods()` and `labour()` are factories: a module with a memory is this world's.

**The journal is a LOG, never a store** (0e′.4). A module that reads back an event it wrote in the
SAME period is using the log to get from one of its own phases to the next — a `working` store with
no declaration, invisible to the ontology register and to the phase-order check, and every field
comes back through `unknown`. Nine of those were found and each is a declared `working` store now:
a firm's plan, a cell's orders, a desk's arbitrage trip, a firm's buyback, a pool's strike position,
a bank's line allotment and its buffer, a treasury's need, the derivative layer's unmet calls. The
event stays in every case — it is the public record of the decision, written FROM the store, once,
and never read back by its writer. Two reads of one's own events are NOT this defect and stay: a
read of a strictly EARLIER period is a module remembering a public fact (Law 19), and an AUDIT
FAMILY reading the public record is what an audit is — handing it the module's store would check a
derivation against itself.

**A participant reaches its party's working store through `ParticipantView.working(name,
initial)`; a phase writes it through `MechanismContext.workingOf(party, name, initial)`.** They are
the two ends of one `Map<PartyId, T>` under (owner, name). The owner is whoever the kernel asked —
the participant it is evaluating, the phase that is running, or the module answering a question —
and the kernel always knows, so a read outside all three has no owner and throws. A participant
gets its OWN party's entry and never the map, so a firm cannot read another firm's plan: private by
construction (Observer A4), the property `blindView` already has for prices.

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

**Its lines compete for its own capital, and the treasury allots** (XI-4, Dealer Desks F2; item 12).
Each line used to size itself against the WHOLE bank — the lending line read the capital headroom,
the dealing line took its own declared share of capital — so two lines drew on one pool with no
allocation between them, which is two banks sharing an equity account rather than one bank with a
treasury. Now the capital walk keeps the three answers it was already computing (what the bank LENT,
what its treasury holds for liquidity, what its dealing line carries above that), the treasury reads
what each line EARNED on what it used off the wire (Law 19: every settled instruction carries its
equity effect, and the instruments its legs moved say whose line it was), and it allots the room the
bank has left to the higher earner first. There is no floor: a line behind the other in a period
when the room ran out gets nothing and stops adding to its book. What neither line claims is
published as `unattributed` rather than folded into one of them (Law 2). The allocation is PRIVATE
(`bank.lines`, Observer A4) — it is exactly what a rival would price against — and its consequence
is public, because a bank that stops quoting has stopped where everybody can see.

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

### 4.13 What is checked before anything is measured

- **`npm run check:opens`** steps the rig thirty periods and the four-country world twelve and
  asserts only that neither THROWS. It is the first thing `npm run check` does and it may run at any
  time: it is not a measurement, so Law 11 does not hold it back. Every mechanism test passed for
  months while the assembled world stopped in period 2, twenty-three times over.
- **The phase order is checked at the seal** and the declaration is checked at the site (4.8).
- **A cell's key belongs to its KIND** (`PartyKindProfile.cellKey`), not to the world. One list for
  every cell made two populations mutually exclusive: a household has no line of business and a
  small firm no cohort, so whichever kind was declared second lost every cell it added. The registry
  refuses a cell kind that declares no key, a named kind that declares one, and a key without
  `region`; `sameKey` answers false across kinds before it looks at a dimension.
- **The observer is READ-ONLY and holds a copy.** It takes the world's own reads, snapshots what it
  shows, and no surface changes the model (Appendix B, Observer). What it adds at 0a is the derived
  phase order with what each phase needs of the period it is in.
- **`Cash` carries its currency (16.0).** A currency is DRAWN, so a phantom type cannot tell two of
  them apart; `Cash` is `{ pieces, ccy }` at runtime and the arithmetic refuses two currencies where
  they meet (Money A2.b). The owner's correction at 16.0 is the design rule: **a party's balance
  sheet is not in one money.** Each party holds an account per currency at its bank (Currency
  B2.a); nothing converts at the ledger boundary (B3) — a party that wants its own money sells the
  other in the spot book, or keeps it, or hedges it; a numéraire is for a REPORT only (C4), so
  `inOwnMoney`/`inMoney` translate at the rate in force and are used where a report, a mark or a
  size in one money is what is wanted (a balance sheet, an index level, a sector total, a dealer's
  room, a bank's capital behind a foreign book), never to move money. Foreign positions revalue at
  the close (D2). The journal refuses a `Cash` value in event data: money on the record is its
  pieces beside a named `ccy` (Law 8), and every reader of a money event requires the `ccy` it
  names.
- **`npm run coverage:reached`** is the measurement behind an `UNMEASURED` mark: it steps both
  worlds and asks the kernel's `Reach` register what every declared capability has produced. It is
  taken at item 0d.

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
docs/                      spec/ ARCHITECTURE.md WORKLIST.md RECORD.md COVERAGE.md IMPLEMENTATION.md
```

Mechanisms (Parts V–X) live in `packages/engine/src/mechanisms/<system>/`, one directory per spec
system, each a `SystemModule`. They are added in worklist order only. `docs/PLAN.md` is the plan
for building them.

---

## 6b. The ontology register

`registry/nouns.ts` is to CATEGORIES what `registry/params.ts` is to NUMBERS. A module keeps its own
state through `MechanismContext.state(name, initial)`, which used to take a name and an object and
ask nothing — so it became the place every economic category the kernel had no home for ended up:
seventeen modules, nineteen slots, an employment register and a book of invoices and a party's
outlooks kept privately, invisible to the audit, to an estate's ranking and to every other module.

A store is now DECLARED or it does not open. `SystemModule.nouns` says what is in each one and which
of three things it is — `noun` (a thing this economy has, which the kernel should own), `working`
(state one phase hands to a later phase inside a period), `physics` (the module's own subject matter,
private by right). A `noun` is a placeholder and must name the plan item that gives it a kernel home,
the same guard `ParamRegister` puts on a placeholder number and for the same reason (Law 2: a
stand-in with no scheduled death is a permanent one). Assembly stamps the owner, never the module
(Law 4). The count of nouns still in a bag is reported, not hidden (Appendix C): **14 homeless
today** — seven `Agreement`, four `View`, two `PublishedStatement`, one `Process`. The denominator
moved at 0e′.4 and the numerator did not: nine `working` stores were declared where nine modules had
been using their own JOURNAL to get from one of their own phases to the next (`working` count 9 →
18). None of them is homeless — a plan nobody has acted on is not a kernel noun — so what the figure
says is that the register now SEES nine stores it could not see before.

### An agreement has a kind and its own terms (item 9.1)

`register/agreements.ts` holds what one party owes another that is not a tradeable instrument. It
was built for ARREARS — six mechanisms opening a row when a payment failed — and a row carried a
free-text `what` saying which. Six spellings of "in arrears" across six modules, and the one reader
that had to tell two rows apart did it by comparing a string it had built at the other end
(`owed.what !== \`dividend ${action.id}\``), which is A-52's shape and would have matched nothing the
moment either side was spelled differently.

An `Agreement` now carries a declared `kind` and its own `terms`, the same construction `Terms` is
for an instrument and `Contract['terms']` for a contract. Four things are the kernel's and are the
same for every kind — two named parties, a money, what is owed now, a state — and everything else
belongs to the kind: a wage and a notice period, a rent and a term, a line and a covenant test, a
pool and what a mandate may hold. The kernel holds the row, indexes it by debtor, creditor and
kind, and ranks it in an estate; it narrows nothing and branches on nothing. A module declares its
kinds in `SystemModule.agreementKinds`, exactly as it declares instrument and party kinds, and
narrows a row back to its own with a STRUCTURAL type predicate (`isDividendOwed`), the idiom
`isShare`, `isPolicy` and `isRow` already use — the lint rule refuses a comparison of kind ids.
A kind no module declared cannot open a row at all.

Two consequences. `owed` may be ZERO: the store holds commitments now and not only failures, and a
performing employment owes nothing this instant and is still an employment — what is refused is a
NEGATIVE amount, which is the other party's row written backwards. And `MechanismContext.owedBy`
and `owedTo`, the only two questions a book of arrears could answer, are replaced by the whole
read-only store (`ctx.agreements`), because a module that declares a kind has to be able to read its
own book back.

It is the spine of item 9: the seven private books — employment, lease, invoice, stock loan, loan,
covenant, deal — are seven kinds of this one noun, and `Mandate` is the eighth.

### The employment register has one home, and wages read it (item 12b.1)

`register/employment.ts` is where the noun lives. The rows were already the kernel's — agreements
of kind `labour.employment`, indexed by debtor, creditor and kind, succeeded when a party ceases —
but the READS over them were a labour module's private book: an index in a `WeakMap`, the wage
bill, the going rate and the headcount, reachable by nobody else. So every other reader of "what
does this party pay its people" — a bank staffing its desks, a firm costing a batch, the treasury
posting public service, a small firm's own wage bill — read a TALLY the labour module published once
a period (`labour.wages`: hours, due, paid, productive, headcount per employer): a stored aggregate
of the register, re-derived every period and left standing when the rows changed (Law 19, Appendix
B: no stored aggregate), declared as a read by eight phases across four modules.

Now the kernel declares the kind at assembly (beside the arrear), `register/employment.ts` holds
the terms — occupation, region, wage per hour, hours per member, `since`, `productiveFrom`,
`notice`, headcount — and `EmploymentReads` answers every question a mechanism has: the one row a
worker cell holds, an employer's rows, the rows at a trade and place, the hours under contract, the
going rate, the headcount employed, an employer's payroll, whether it ever employed anybody. They
are on `ctx.employment` for a phase and `view.employs()` for a participant, built at the read from
the store's own indexes. Wages are instructions that read it: `payWages` walks the rows and settles
one leg each; what is DUE is the row's and what was PAID is the ledger's (`payrollSettledIn` walks
the period's settled wage legs). The labour module keeps the one thing that is not a fact about a
row — the trade each cell can work in, noun `skill` — and the `labour.wagesInArrears` agreement it
wrote beside settlement's arrear on a failed wage is gone (Law 4: one debt, one writer).

### An agreement can carry the whole PRODUCT, and then a roster becomes an outcome (item 10e.4)

`MandateTerms` carries what a pool may hold, how its investors get in and out, whether it tracks
something, **what its manager charges, what it keeps in cash, and what its investors require of it**
— and whether notice has been given. That is the whole product, in one place, struck when two named
parties agreed it.

Three of those were per-fund PARAMETERS (`fund.fee.x`, `fund.buffer.x`, `fund.requiredYield.x`) and
the decision to move them is the general one: **a number a mechanism produces is not a parameter,
and a parameter register is for numbers the WORLD declares** (Law 2, XI-14). A fee is what a manager
charges, and a manager decides that; the fees this world opens with are drawn, which is an opening
condition (Seed A3) exactly like a drawn balance sheet.

The consequence is structural and is why it belongs here. **Parameters are declared at assembly, so
anything whose behaviour depends on one can never be created while the world runs.** As long as a
pool's economics lived in `params`, the set of pools was fixed before period zero — and every phase
in the module walked the declaration list, which said the same thing from the other end. Both are
gone: the run-time object is the performing MANDATE (`livingPools`), the declaration is read once by
the seed, and a manager opens and closes pools mid-run like any other decision.

**The general rule: where a population can change while the world runs, the population is a read of
the kernel store that holds it, never a list the module was built with.** The same shape applies to
firms (item 12), and it is the reason `SystemModule.seed` and the module's phases must not share a
list.

---

## 7. Citations in code

Every module, mechanism and audit family carries `@spec` tags naming the clauses it implements:

```ts
/** @spec Money C2 C2.a C2.b */
```

`tools/check-citations.ts` parses the spec into an index of requirement ids and fails the build if a
citation does not resolve. `docs/COVERAGE.md` is the requirement → status map (`MET at <path>`,
`MISSING`, `OUT OF SCOPE (reason)`), re-marked in the same change that meets a requirement (App C).

**A citation is a claim about the source, and `MET` inherits its reach from that.** It says a module
implementing the clause exists; it cannot say the module has ever run, because nothing about a
`@spec` tag depends on the world. Ninety-nine `MET` rows cite a module that has never produced an
outcome, and the ninety-six citing nothing else say **NEVER REACHED** in their `where` cell
(`docs/IMPLEMENTATION.md` B-12). Whether a mechanism produces anything is measured by reading the world, and
that is the audit's job, not the citation checker's.

---

## 8. Work discipline (Laws 10–17, Part XIII)

- **`docs/IMPLEMENTATION.md` is the one ordered list.** Twenty-eight items, each carrying the
  findings it closes; the first open one is the work. `docs/WORKLIST.md` is HISTORY: every row it
  had open is carried by a plan item and says which, and it is the one writer of one thing only —
  which items it worked and closed, so `plan:progress` counts a closed item's steps after its
  section was deleted. The two files used one id for two items (`14` was the polity there and is
  the insurers here), which is why the plan's ids win and the superseded rows point at them.
  `tools/plan-progress.ts` counts the PLAN's sections (item 0c): counting the worklist's rows made
  every step of items 0a to 24 invisible, because none of them has a row there.
- One bounded change per item; a commit per item; the commit message says what and why.
- `docs/RECORD.md` is a ledger of outcomes, not a diary. A finding leaves the plan only when its
  item closes.
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
| `phoenix/no-bounds`: no `Math.min`, `Math.max`, or a call to anything named `clamp`/`saturate`/`bound`, outside `core/num.ts` | Law 6, App B 22      |
| `phoenix/no-numeric-default`: no `?? <number>`, `\|\| <number>`, `= 0` default params for amounts                          | App A Missing values |
| `phoenix/no-magic-numbers`: literals other than `0, 1, -1, 2` outside `core/`, `registry/`, tests                          | Law 2, XI-14         |
| `phoenix/no-kind-branch`: no `=== '<kind>'` comparisons on `.kind/.sector/.industry` inside `mechanisms/`                  | Law 15, App B 48     |
| `phoenix/no-clock-no-random`: no `Date`, `Math.random`, `performance.now` in engine                                        | Seed A5, Audit D3    |
| `phoenix/no-cross-module-import`: a module imports only the kernel, never a sibling module or the world container          | Law 15, 4.9b         |
| `phoenix/no-value-recipe`: no `costShare`, `valueShare`, `costPerRevenue` — a recipe is a physical quantity per unit       | Goods A2.b           |
| `phoenix/no-console`, `no-empty` catch, `no-restricted-syntax` on `try` inside mechanisms                                  | §5                   |
| `@typescript-eslint/switch-exhaustiveness-check`, `no-explicit-any`, `no-non-null-assertion`, `strict-boolean-expressions` | §5                   |

**What `no-bounds` does not see, and what reads it instead.** A minimum written by hand — `let most
= room; if (canMake < most) most = canMake` — is invisible to the rule, and so is `Math.abs`. Both
are deliberate: a rule that fired on `Math.abs` would fire on every dust comparison in `num.ts` and
every violation size the audit reports, and a guard nobody can leave on is not a guard. So the rule
catches the shape a bound is usually WRITTEN in, and whether a comparison is arithmetic or a
decision is read at the site. That reading is not a formality: the bound that got furthest into
this tree was a floor under a dealer's offer and a cap over its bid, argued from a lender's
reservation, and what gave it away was the length of the comment justifying it (`RECORD.md`, 11.3).
A minimum is arithmetic when the smaller number is a thing that does not exist — units nobody
holds, a lender's money already lent — and a bound when it is a number the model chose not to go
past.

The parameter register is checked at engine start: a placeholder without a named mechanism and
worklist item fails construction, a non-placeholder that names one fails, and a SHAPE whose reason
names a worklist item fails — a shape with a scheduled death is a placeholder (Law 2), and writing
the death in prose is how two of them once stayed out of the count that measures them.

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

### An estate takes pledged units with their lien; an instruction may free and move the same units (item 12c.3)

A party that dies with margin pledged to a clearing house left those lots on its own dead book:
the estate moved what was free and skipped the rest, and the next phase to touch the lots threw
(Money E4). The estate now takes them in one numbered instruction — the release of each lien, the
asset move — and binds them again on its own book to the same beneficiary for the same reason, so
the secured creditor ranks on the estate as it ranked on the dead (Firm Birth D2) and the
collateral is never briefly nobody's. For that the wire's precheck credits what an instruction
FREES to what it may deliver: free units are the register's answer plus the liens the same
instruction releases, as they were already the register's answer less what the same instruction
takes out. The walk applies the release before the debit and asks the register again (Law 4).

The same item made every employer's posting whole people (`registry/wages.ts wholePeople`): 12b.5
said all did, and firms, small firms and the treasury did not.

### Outlooks: the exposure set, and two predictors (items 12d.1, 12d.2)

An outlook is formed from what a party observed, and what it observes is (a) the legs it was a side
of and what they did to its account, and (b) since 12d.1 what is PUBLIC about what it is EXPOSED
to — the print of every line it holds, what a company whose paper it holds published (the period
after), the going rate where its employment rows put it, the board of its own bank. The exposure
set is a read of holdings and rows; the public facts are read through the registry, never by an
event's name; the kernel's `Subject` vocabulary grew three entries for them (`wage`, `deposit`,
`reported`).

Since 12d.2 a `Held` carries two predictors: the adaptive one (its own history corrected at its
memory) and an anchored one (the variable's last public level, where it has one). Each keeps its
own track of surprises over the same memory; `form` switches to the narrower track and never
blends them (§46 B1.b). `ParticipantView.outlook` returns the FOLLOWED predictor's value and the
width of ITS track, and `expectations.dispersion` is the dispersion of followed values. The
surprise event records the surprise the party took, the predictor it followed and both tracks'
entries, so the switch replays off the journal.

### The environment is read from a view too (item 12d.3)

`registry/environment.ts` had one door, `conditionsFor(ctx, region, facts)`, for a phase with a
journal. A participant costing its basket has a view and no journal, so the registry gained the
same read off the view's public event about the region (`conditionsSeen`, `conditionsStanding`),
sharing the one product. A basket row names what a member takes more of to stand against the
weather (`ConsumptionDecl.standsAgainst`), and the period's quantity is the normal week's over how
the condition stands — the same shape a crop's yield takes, from the other side. The central
bank's and the insurers' reads of the condition go with the decisions that need them (18a.1,
14.2), through the same doors.

### Insurers are seeded like companies, and what a death leaves is the successor's (item 14.1)

The foundation seed creates one insurer per place that has a bank and endows its opening surplus
in cash (a SHAPE per head), before the equity seed runs; the equity seed then floats its line like
any company's — shares worth its book at the opening price, held by the savers at cost. The
insurers module's own seed leaves a party the foundation made alone, so a scale model assembled
without the module (`test/rig.ts withDependencies`) has no insurer and the equity seed skips a row
whose party is absent.

Two rules the first living insurer forced into the open: a closed-end fund's commitment is a SHARE
of what the investor holds when the promise is opened (an amount drawn blind was a seeded default);
and a pool passes on the dividends and interest that reached it, read off the legs, never the money
a subscriber put in. Two succession rules the wire now keeps for named parties as it did for cells:
`cease` reseats a dead party's live issued paper to its successor (Register F2), and a bank books a
drawing to the borrower as it is now (`parties.resolve`).

### Cover has buyers, and the weather and the dead are things a party expects (item 14.2)

`registry/insurance.ts` spells the names of cover — the policy kind, the unit, the venue per
currency, the term — so a firm and a household can post into the insurers' book without importing
the module that writes the cover; the insurers module re-exports them and opens the book at the
seed. Two subjects joined the outlook vocabulary: `condition.<fact>.<region>` (every party observes
its region's published weather every period) and `mortality.<cohort>` (a cell observes its cohort's
published deaths over who there were). A firm bids for cover on its plant at the share of it the
wind it expects would take over the term — `registry/physical.ts survivesWind`, the one relation the
weather itself scraps plant by — and a cell bids for life cover on what a member owes at the chance
a member dies within the term; both as ladders (`clearing/schedule.ts rungsUpTo`), which now
compares what the money would take with what was wanted before making it a count.

### A claim is a redemption of cover against a loss the weather said (item 14.3)

The capital programme's `capital.weathered` carries what the lost units were on the holder's books
at, drawn off the lots as the register draws them; `registry/physical.ts weatheredIn` reads the
period's losses; the insurers' `claims` phase pays each covered loser up to the cover it holds and
takes the cover used back in the same numbered instruction — an asset leg to the issuer is a
redemption, so the cover is gone. `claim` is a receipt class of its own (`ledger/instruction.ts`,
`register/arrears.ts`: it ranks with what the state and a counterparty are owed) and is nobody's
income (`treasury`).

### The insurer's quote is its experience plus what its capital costs (item 14.4)

`claims` is an outlook subject: every party with cover outstanding observes each period what a unit
of its cover cost it in claims, off its own `claim` legs over the cover it has out. `quoteCover`
prices a unit as that outlook over the term plus the return its capital requires — the registry's
`costOfCapital`, over the term's fraction of a year, on the surplus behind a unit of cover — and
posts its surplus as capacity. Nothing reads the last claim; a refusal to quote is a public event.

### A row with a schedule is marked like a line with a price (item 14.5)

`AgreementKindDecl.valued` lets a kind say what a live row of its is worth now — a schedule at a
curve, with the debtor's own outlook to read (`RowValuationReads`). `revalue` marks every such row
each period: the creditor's account up and the debtor's down by the change since the last mark, in
the same pass, each in its own money; a row that has ended unwinds its mark the same way. The
kernel keeps nothing but the last mark (`Agreements.mark/markOf`). A policy is the first such kind
(`registry/insurance.ts POLICY_ROW`): cover, a term and a curve, worth the insurer's expected claims
on it; claims restate the cover down and end the row, and a term that has passed ends it. Cover
fills are paired by the solver's allocation, one premium and one row per pair. The `POLICY`
instrument kind is gone.

### A pension is the second profile, and membership is a key (item 14.6)

The insurers module declares two party kinds behind one dispatch table: `insurance` (fails on cash
and solvency, quotes cover) and `pension` (`insurers/pensions.ts pensionKind`: fails on cash only —
a fund short of its promises has a shortfall its sponsors are called for, Insurers D3). Who is a
member of the scheme is a lattice dimension of the household (`pension: member | none`), opened off
the record like a hire and moved by `pension.enrolled` when a payroll first pays the cell; the key
is carried through every job and cohort, so the standing cell of retired members is the one a fund
owes. The payroll carries the contributions (`labour/matching.ts payFrom` reads the employer's
sponsorship row through `registry/insurance.ts sponsorshipOf` and adds two `contribution` legs to
the wage instruction); the promise is a valued row (`PENSION_ROW`, `promiseOf`) whose schedule reads
the cell's weight, the going rate of the trade it is indexed to and the scheme's params — so
`RowValuationReads` now offers `weightOf`, `goingRate` and `params` beside the curve, the day and
the outlook — and `AgreementReads.markOf` exposes the kernel's mark so the funding ratio reads it
rather than re-deriving it (Law 19). Two receipts join the wire: `pension` (income to the cell,
taxed and ranked as a wage) and `contribution` (not income; ranked with a claim).

### What an institution keeps back is a read of its own outlooks (item 14.7)

`insurers/allocate.ts buffersOf` names the three things an institution keeps out of what it puts to
work — the claims it expects a period (its `claims` outlook on the cover it has out), the pensions
its rows pay next period, the calls it expects (its `called` outlook) — and `insurer.allocated`
publishes them. `called` is an own-observation subject of §46 (`registry/funding.ts callsOn` reads the
funds module's calls by the registry's name; `expectations exposed` observes them for any party that
has ever been called, a quiet period as none). A door (`doors`) is live only while its fund party is;
a missed call is read for last period through the same registry read. `longestPromise` reads pension
rows to the end of the mortality table.

### The ground has a seller in each place, and a project buys it at the residual (item 15.1)

`registry/land.ts` names the ground (`LAND`, `landId`, `landMarket`), the party present in each
place that holds what nobody has built on (`LOCAL_AUTHORITY`, `authorityIdFor`, a kind the land
module declares and seeds) and the planning policy (`PLANNING_RELEASE`, a count of hectares a
period). The authority's ask is its own outlook of the line's price, the last print, or `'market'`;
the firm's bid lives in its investment decision (`registry/capital.ts project` takes a
`GroundForProject`; `firms/decide.ts groundFor` builds it): the hectares the plant wants less the
free ground held, at the project's surplus over the plant per hectare it takes, ground before
plant. The capital programme stands plant only on free ground (`groundToCarry`), pledges the
ground under a new vintage to the authority in the vintage's name (a `pledge` leg in the
commissioning instruction) and releases it at retirement. `groundUnder` in `registry/physical.ts`
is the one arithmetic for ground under plant.

### A port has an owner and a berth (item 15.2)

`registry/ports.ts` names the quay's owner (the place's local authority), the berths it works a
period (`PORT_BERTHS`, a placeholder dying at 22a) and the read of the calls it has worked
(`callsAt`: settled `sail` legs from the place and `land` legs into it, off the ledger). The freight
module asks `berthFree` before a vessel lands and before a cargo loads; what is turned away waits
(a landed-but-not-alongside voyage stays under way) or does not sail, and is said as
`port.congested`. Nothing is stored between the reads.

### Commercial property: a lease is a row, a landlord is a cell (item 15.3)

`mechanisms/property/` owns the landlord kind (`LANDLORD`, cells with `LANDLORD_LATTICE`: region,
bank, size), the lease row kind (`LEASE_ROW`; terms in `registry/physical.ts LeaseTerms` so
`rentedRoom` can count leased plant as room), the lettings book per place (`lettingsVenue`, cleared
by `property.let` before the markets, `marginalBid`), the rent collection (`property.collect`, a
`rent` receipt, whole pieces a landlord) and the landlord's build and its secured ask
(`buildOf`, `askForLoans` through `ctx.request`). The names are `registry/property.ts`; the tenant's
reason is read off its own published plan (`planOf`: `firms.plan`, bound on `premises`). The land
seed runs after the property seed (`land` requires `property`) and gives cells the ground under
their opening plant per member where it comes to whole hectares.

### A shop sells from a lease; a cell's marks are per member (item 15.4)

`RecipePlantDecl.leased` (goods data) → `RecipePlant.leased` → `PlantNeed.leased`: a need a lease
meets and a project never builds (`registry/capital.ts project` filters it out of the bundle). The
foundation seeds the landlord cells and their premises before the firms' plant (`seedLandlords`
block) and `leaseFromLandlords` opens a shop's lease instead of its vintages; the landlords' cash is
given in the property module's seed, after the banks' sheets are built. In `world/revalue.ts` a
cell's marks move its account by the whole cell's number, because both are totals (21a; they were
divided by the weight while the account was per member). `securities-lending` marks at the last
print (`lastMarkOf`).

### A tenancy has a term and a landlord has a view of its tenant (item 15.5)

`housing/index.ts`: `TenancyTerms.until` (`housing.tenancyTermPeriods`); `collect` ends a tenancy on
its day, or when the landlord's outlook `credit.<tenant>` makes the rent struck worth less than the
wear (`housing.tenancy.ended`), and pays the rent with `receipt: rent`. `letIn` signs at
`outcome.price` and carries the last rent struck onto a print that struck none. The book's unit is
occupancy (`DWELLING_WEEKS`); `occupancyOf`/`dwellingsOf` are the one conversion from the register's
dwellings, and `housing.shortfall` publishes the register's pieces for `homeBid`. The landlord kind
is a `venueParticipant` of the lettings book with the same `ordersOf`; the housing seed endows each
landlord member with `housing.seed.dwellingsPerLandlord` dwellings at the last print (placeholder,
22a), so housing `requires: property`. `ledger/instruction.ts paysWhatWasOwed` classifies a receipt
as a promise kept or a bargain struck (one exhaustive switch beside the union); `expectations
observations` reads it to form, for the party owed, the share of what fell due that arrived as
`credit.<payer>` — the first writer of the `credit` subject. `Instruments.byKind`/`ofKind` index
lines by kind (`InstrumentsReads`, the audit and seed picks widened); `foreclose` reads
`ofKind(LOAN)`, and both mortgage reads compare the security's instrument and the lien's `secures`
by name.

### A peopled place without a dwelling line is a seed finding (item 15.6)

`housing/index.ts roofs()`: a `names` family contributed by the housing module — one violation per
region with living household cells and no `good.dwelling.<region>` line, owner the region, size its
people; and one per dwelling holder that has ceased (E1). The seal runs the audit at period 0, which
is what makes it a seed finding.
