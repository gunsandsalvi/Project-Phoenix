# Item 13b — The derivative classes

**Objective.** Four classes as modules on 13a's layer, each answering the derivative contract its
own way: credit default swaps with a curve, an event settled at the estate's realised recovery and
an index series; interest-rate swaps whose fixed rate clears and whose curve is a read; FX forwards,
FX swaps and cross-currency swaps that carry the interest differential through an arbitrage banks
take with balance sheet, with one basis; an equity index future that settles against the index
read, which is what a desk hedges inventory with. Every side of every book has a reason, and every
book has a participant with a view (XI-13).

**Read first.** §17 CDS, §18 IRS, §19 FX Forwards (all); Derivative contract D7.b (at par); Indices
C3; Dealer Desks E1, E2; Corporate Credit H4, H4.a; Sovereign C (the curve); XI-13; XI-12. Code:
13a's profile and market variant, 12's `spot-fx`, benchmark and `view.index`, item 7's estate
recovery, item 5's credit events.

**Clauses this item meets.** CDS A1, A1.a–A1.d, A2, A3, A4, A4.a, A5, A5.a, A5.b, B1, B1.a, B2,
B2.a, B3, B4, B5, C1, C2, C3, C3.a, C3.b, C4, D1, D2, D2.a, D2.b, D3, D4, D5, E1–E4; IRS A1,
A1.a–A1.d, A2, A3, A4, B1, B2, B2.a (the pension at 13h; declared PARTIAL until then), B3, B4, B5,
C1, C1.a, C2, C3, C3.a, C4, D1–D4, D3.a, E1–E3; FX Forwards A1, A1.a–A1.d, A2, A3, A4, B1, B2,
B2.a, B2.b, B3, B3.a, B3.b, B4, C1, C1.a, C2, C3, C4, D1, D2, D2.a, D3, D4, E1–E4; Indices C3;
Dealer Desks E1, E2; Corporate Credit H4, H4.a; Equity (hedging) from item 9's PARTIAL list.

---

## Findings this item carries

Three are one thing: **a desk's book is not the bank's book**, and nothing in this item may hedge,
quote or margin off a position read wrong. The fourth is a currency a bank never funded. The last
two come from 13a, and both are about what the layer does not SAY: a margin of nothing, and a door
nothing has opened.

### A line's makers are drawn and nobody reads them (`12d-10`)

`ListedDecl.makers` (`equity/data.ts:44`) says which of this world's banks make a market in a given
share line — "a market in one name has a few makers, not all of them and not one" (Dealer Desks A3),
drawn `MAKERS_PER_LINE` at a time. Nothing reads it. `dealingOrders` asks the BANK's own `makes`
list, which is by instrument KIND, so every bank that deals shares at all quotes every line in the
world — which is the "every bank has a view of every firm" that A3 says a dealer is not. It went
unread when 11.5 moved the equity float off the desks and onto the household cells that save. Two
things follow: A3 is MISSING rather than out of scope, and a desk opens holding no inventory in any
line, which is what four of `dealing.test.ts`'s tests are about. Seen at
`packages/engine/src/mechanisms/equity/data.ts:44`, `packages/engine/src/mechanisms/banks/dealing.ts:203`.

### A desk's limit is measured against its bank's liquidity portfolio (`12d-12`)

`opening-liquidity.test.ts` asks that every desk opens inside its own limit (Dealer Desks D1, D4).
Measured at the rig's six banks: `roomLeft -58,287,103,191` against a book of 53,417,676,666, and
the book is almost entirely SOVEREIGN PAPER — tens of billions of `ust.bill.*` and `ust.*` per bank,
against 30,531 of the one fund share on it. That paper is the bank treasury's liquidity buffer, not
a position its desk took. `quoteFor` reads inventory as `view.free(instrument)` and the aggregate
book as the whole of it, so a bank holding what the liquidity standard requires is a desk over its
dealing limit before it has quoted anything. The per-line arithmetic already knows the difference —
`state.targetIn(instrument)` is where the treasury wants the line, and `away = inventory − target` —
and the AGGREGATE does not. One holding, two purposes, one of them measured with the other's ruler
(Law 4). Seen at `packages/engine/src/mechanisms/banks/dealing-quote.ts:208`.

### And what `12d-12` turned out to need (measured during 13b)

The finding read the symptom right and the cause only half right. `bookValue` (`banks/dealing.ts`)
ALREADY measures the distance from the treasury's target rather than the whole holding — so the
aggregate is not simply the raw book. What makes `roomLeft` −58,304,064,007 at the opening is that
the TARGET is small and the HOLDING is large: `liquidityPlan` derives what the treasury wants from
the coverage rule and its own cushion against its worst week, and the seed endows a bank with tens
of billions of sovereign paper. The distance between the two is real, and the desk's book is being
charged with all of it.

What it needs is the distinction the finding names and no arithmetic can stand in for: **a desk's
position is what the DESK took, and the treasury's portfolio is not it.** One holding, two owners
inside one bank, and the register has no way to say which lots are whose — so the fix is a desk
sub-ledger (which lots the dealing line acquired), or a treasury that hands the desk an inventory
and takes it back. Both are mechanisms, not reads, and `bookValue` cannot be corrected into either.

Until one exists, every desk in every seed opens over its aggregate limit and quotes with `binds:
'book'` — which is what `opening-liquidity.test.ts` reports and why it is red.

### A bank lends a money it has not funded, and a desk carries one in a money nobody named (`12d-14`)

`publishQuotes` asks every bank in the world what it would lend a borrower and prices the answer off
`costOfFunds(bank, the BORROWER's currency)`. A US bank quoting a European firm is quoting a euro
loan — with `owedBy(bank, EUR) = 0`. `costOfFunds` then blends an interest cost of nothing with the
bank's whole equity, so the quote comes out at the bank's required return on capital and nothing
else: measured at 0.0887 against 0.0044 for the same bank's own money. Worse inside `costOfFunds`
itself: `owed` and `couponsPaid` are per currency, but `capital` is `equity()` — the bank's whole
capital in its own money — so the blend adds two currencies (Appendix B). And `carryRate` applies
what funding costs the bank to every line it quotes, including one denominated in somebody else's
money. 12d fixed the publication only (`bank.costOfFunds` now carries `alsoIn`, and `carryRate`
names the currency it is taking). **The mechanism is this item's**: a bank that has not funded a
currency borrows or swaps it, and the FX swap below is where it does. Seen at
`packages/engine/src/mechanisms/banks/index.ts:248`, `:1082`, `.../banks/dealing.ts:118`.

### An exchange-traded fund's own market clears nothing (`12d-4`)

`nothingSettled` every period in the world `etf.test.ts` builds: there are bids and no trade comes of
them, and no failed instruction names the line either — so the trades are not being drafted rather
than being refused. Four of `etf.test.ts`'s reds are this one thing (a market that clears, a slice
taken against shares, a mark nobody traded at, a book somebody would close). It lands here because
this item puts a **listed vehicle on every index** and every one of them would inherit it. Seen at
`packages/engine/src/mechanisms/funds/etf.ts`.

### A requirement below one piece of cash posts nothing, and G2's "stated reason" is silent (`13a-1`)

The layer's capacity door admits a trade at its full size whenever the kind can say what the
underlying's measured move is, and the margin leg then converts the requirement to cash. On a
forward on a bill moving a tick, a five-contract book asks for **four hundredths of a cent** of
margin — which is zero pieces (Law 8: a payment is a count of pieces), so nothing is posted, the
default fund is sized from nothing, and a member that then defaults leaves the whole loss unfunded.
Measured in 13a's own layer tests: `waterfall.unfunded` for the entire close-out value, three
periods running, with every line of the waterfall paying zero.

Nothing is arithmetically wrong with it — the requirement really is below a cent. What is missing is
G2: an exposure is admitted with no margin against it and the layer says nothing, where G2 allows
that only with a STATED reason. The reason exists ("the requirement rounds below the smallest piece
of this money") and it is not written down. The classes are where it bites, because a class with a
real notional is where the difference between "too small to post" and "nobody measured it" stops
being invisible.

### Collateral is cash, and a lien has no user yet (`13a-2`)

D9.a says posted collateral leaves the poster's free balance, is still owned, and comes back. The
layer meets it in CASH — money out, a claim on the holder in, an asset swap (C3.a) — and the
kernel's lien (Register D, `pledge`/`release` legs, the encumbrance check) is built and unused by
it. Securities posted as collateral is the other half of D9 and it has no user until a class posts
them: a CDS seller pledging a bond, a repo'd future. It is not missing from the kernel; it is
unexercised, and a door nothing has ever opened is a door nobody has tested.

---

### Every class quotes the book's own last print, and one of them cannot start (from the review)

All five bilateral classes have this shape (`irs/participants.ts:81`, `bond-futures/index.ts:260`,
`fx-derivatives/participants.ts:91`, `options/index.ts:247`, `cds/participants.ts:44`):

```ts
const last = view.print(t.book);
const level = last.some ? last.value.price : /* bootstrap */;
let price = level;                       // the order, before any view
if (outlook.some && expects > level + tick) { price = expects; }
```

**A party with no outlook posts exactly the last print.** `outlook('price.<book>')` exists only for a
party that has traded in this book (Expectations A2), so a borrower hedging its own coupon or a
holder buying its first put posts a limit AT the print. A book whose participants are all in that
state is a crowd of orders at one level: it clears there, prints that level, and prints it again.
And when a party does have an outlook, `observations()` formed it from that party's own fills in
that book — so `expects` lands inside `level ± tick`, the nudge never fires, and the book is frozen
at whatever it first printed. That is XI-13's fixed point, closed.

**`banks/dealing-quote.ts` already diagnosed this and cured it**, and the incident is in the file:
"the print moves the outlook, the outlook moves the view, the view moves the quote, the quote moves
the print — and in the one market this model funds itself through it walked a bill that pays 1.00 in
seven days to a print of 3.3337, at which no representable yield discounts its own payments to what
somebody paid, so the curve threw and the world stopped." `viewOf` asks its own required yield FIRST
and the print-derived outlook second. These seven classes were written afterwards and do the
opposite.

**The option book cannot open at all.** `options/index.ts:247`: with no print the level is
`tickForDerivative(...)` — one tick. Every party's own `asks` is then above it, `if (asks > level)`
fires for everybody, every party subtracts room from `want`, so **every party in the first session is
a writer**: all sells, no buys, `noDemand`, no print, and next period the bootstrap is one tick
again. It is also a declared number standing in for a mechanism, written in code rather than in the
parameter register.

**The other four bootstraps are real values used exactly once.** CDS off the reference's own bond
below par, IRS off the overnight fixing, FX forward off spot, bond future off the cash deliverable —
each genuinely print-independent, each correctly argued, and each reachable only while the book has
NEVER printed. From the second session the print displaces them permanently, including when it is
stale and the underlying has moved: a forward whose spot moved five per cent and whose own book did
not trade has every party posting the old number, so the cash-and-carry relationship the class exists
to express is live in period one and dead from period two.

### The contract store validates neither its terms nor their kind (from the review)

`Instruments.add` checks `terms.kind === decl.kind` (cited Law 4) and calls
`profile.validateTerms(decl.terms)`. `Contracts.open` checks only that the two sides differ and that
the notional is positive. `world.ts` validates a contract MARKET's template terms at assembly, which
is a different object from the row settlement later opens. So a row can be written with `kind: 'cds'`
and IRS terms and the failure is silent rather than loud: every class's mark begins
`if (!isCds(c.terms)) return 0`, so the contract sits open on two balance sheets, marked zero on both
sides, passing the zero-sum family forever. And the notional never passes `onTheGrid`, while
`Instruments` routes every change to `issued` through it and explains why — a fractional notional
multiplies into every mark, margin call and close-out, and the zero-sum family still passes because
both sides are equally fractional.

### A member's clearing capacity is reduced twice for every unit of margin it posts (from the review)

`derivative-layer/index.ts` `admits`:
`sub(sub(cash, keep, 'net of its buffer'), committed(view, m.ccy), 'left to commit')`. `committed`
sums the margin CLAIMS the party holds, and a party holds one because it posted cash for it —
`moveMargin` moves the claim to the poster and the money to the holder in one instruction, settled
before the next trade is admitted. So `view.cash(ccy)` is already net of everything posted, and
subtracting `committed` takes it off a second time: a member that has posted 100 has its room cut by
200. The intent (E3: capacity drawn down as it is consumed) is right and `cash` alone already does
it. The effect is conservative, which is why nothing caught it; it makes every derivative book
thinner than the mechanism says and E4's refusal measurement wrong in the direction that looks like
prudence.


## Design

### Rate-priced markets

A swap or a CDS clears on a **rate**, not a price: the print is the fixed rate (bp per annum) or the
running spread, in a declared unit (`bp.pa`), and the contract is struck at par at that print (D7.b:
the mechanism is the solver over schedules of rate against size; the valuation formula is never run
backwards). The `MarketDecl` variant `{ kind: 'contract' }` from 13a gains a `quotedAs: 'price' |
'rate'` term; the solver is unchanged (a schedule is size against a level; the level's unit is data).
A schedule for a rate-priced market is sorted the natural way for the side (a protection buyer pays
less spread for more size; a seller wants more): the module's participants post correctly and the
solver's monotonicity check throws on a malformed one.

### Module `cds`

- `requires: ['derivative-layer', 'ratings']`.
- **Kind** `cds`: terms `{ reference: PartyId, tenor, currency, notional }`; underlying = the named
  reference's credit event (A1.a, A4: the reference is a party with debt outstanding; A4.a: assembly
  refuses a reference nobody can observe failing, i.e. a party with no defaultable instrument);
  payoff par − recovery on the event (A1.b); premium leg periodic in cash, stopping on the event
  (A2); mark = the protection leg's expected value read from the cleared spread curve (A3, C2: the
  implied default probability and expected recovery are **derived** from the spread and the term
  structure; the profile's `mark` reads the curve, never a PD model).
- **Curve** (A1.d): one market per (reference, tenor) for several tenors (data: `cds.tenors`);
  the term structure of credit is the set of prints.
- **Participants**: protection buyers: banks holding the reference's loans they cannot sell (B1.a:
  the bank's participant reads its own loan rows and its large-exposure headroom), holders of the
  reference's bonds, and a **view** (B1: a party whose outlook of the reference's coverage is worse
  than the spread implies); sellers: funds and insurers wanting credit exposure without funding a
  bond (B2), and a view (B2: spread too wide for the risk); desks (B4). B5: the assembly check from
  item 12 requires a `speculative` participant on both sides of every CDS book: desks' own outlook
  qualifies here; 13h's hedge funds join. Naked sellers are capitalised as an unfunded exposure
  (B3: risk weight per 11's capital: `regulation.riskWeight.cds.sold`, policy).
- **Reservations** carry the counterparty term (13a D10.a; E2: a seller whose own PD, as read by
  the buyer, correlates with the reference: the buyer's view of the seller's exposure to the
  reference through `exposureTo` and public holdings: wrong-way risk is a cost in the reservation).
- **The event** (D): item 5's credit event for the reference (D1: the stated definition); the
  contract then pays no premium, marks at its expected payoff, and **holds past its maturity** until
  item 7's estate closes (D2.b); at close the settlement is par − the realised recovery per unit of
  the defaulted obligations (D2: read from the estate's distribution: no fixed recovery: D2.a); the
  payment is cash seller → buyer (D3: it can fail the seller: the same Failed state); the contract
  terminates (D4).
- **The index** (A5): a `cds.index` kind on a series of names fixed at the roll (data per series);
  a name's event settles its weight once for every contract on the line; the line runs on with
  survivors; its own book; the index-vs-single-name basis is a read (A5.b). The series come in
  **two grades**, investment grade and high yield, each rolled on its own grade off the assessors'
  published opinions (A5.a; below, "The index set"): one series is a market with no quality
  dimension in it, and the high-yield series is the one where a constituent's credit event actually
  fires and settles its weight.
- **Reads**: basis vs the cash bond's spread over the sovereign curve at every tenor both books print
  (C3, C3.a: a read on the observer; C3.b: a standing observation); net notional per reference (E3).

### Module `irs`

- `requires: ['derivative-layer', 'indices']`.
- **Kind** `irs`: terms `{ currency, tenor, notional, fixedRate (the strike), fixedPeriodicity,
floatPeriodicity, accrual conventions }` (A2: legs need not match); underlying = the overnight
  benchmark print (12's transacted rate: A1.a, E3); the floating leg fixes on its date as the
  **compounded** overnight prints over the accrual period (A3: a read of history); payoff: the
  **net** of fixed and floating per payment date (A1.b); no notional exchange (E1: the profile's
  legs never include one; a test asserts it).
- **Curve** (C1): one market per tenor (data `irs.tenors`); the swap curve is the set of cleared
  fixed rates (C1.a: a read); forward rates derived (C2); the swap spread vs the sovereign curve is a
  read (C3, C3.a). E2: the fixed rate never comes from the discount curve: the profile has no
  `parRate()`; the mark discounts the fixed leg against the floating leg's forwards read from the
  curve of prints.
- **Participants**: issuers with fixed debt wanting floating and vice versa (B1: the firm's and the
  bank's participants from their own debt rows), a bank managing its gap (B3: from its own repricing
  read of item 11), a **view** on rates (B4: a party whose outlook of the policy path differs from
  the curve: desks here, hedge funds at 13h), desks (B5); pensions (B2, B2.a) at 13h: declared
  PARTIAL here and closed there.
- **Cash**: variation margin from 13a turns the mark into cash (D3, D3.a: the hedger's funding
  problem shows in its cash while its hedged item revalues the other way).

### Module `fx-derivatives`

- `requires: ['derivative-layer', 'spot-fx', 'money-market']`.
- **Kind** `fx.forward`: terms `{ buy: Currency, sell: Currency, buyAmount, sellAmount, maturity }`
  (A1.d: a leg states its own money); payoff at maturity: both notionals move, two money legs in two
  currencies in one instruction (A1.b, A2, E3: DvP across currencies); the price is the forward rate
  (A1.c) cleared in a market per (pair, tenor); the mark is against the forward print for the tenor
  **left** (A3: a parity-struck forward is worth nothing at strike and earns carry over its life).
- **FX swap** (A4): spot one way, forward back: two instructions written together by the
  participant that wants a secured loan of one currency against another; the bank's funding
  participant for a foreign book (D3) uses it; it is booked as what it is: a spot trade and a forward
  row.
- **The forward rate** (B1): cleared. **CIP** (B2, B2.a): an arbitrage participant per bank: borrow
  one currency (a money-market quote), buy the other spot, lend it (a money-market offer), sell it
  forward: the four legs are real orders sized by the bank's balance-sheet headroom and capital
  charge (B2.b: not free), so the forward sits near spot × the funding differential because banks
  take the trade, and the residual **basis** is one read (B3, B3.b: no second basis; the only basis
  is `forward − spot × (1 + r_buy)/(1 + r_sell)` read from prints).
- **Kind** `xccy`: notionals exchanged at start and end at the original rate (C1, C3), periodic
  interest both legs each on its own currency's benchmark or a fixed rate (C1.a); its price is the
  basis on one leg (C4); used by issuers that raised in one currency and need another (C2).
- **Participants**: importers/exporters with known foreign payments (D1: from their invoice books),
  investors holding foreign assets who roll (D2, D2.a: a recurring cost in their P&L), banks funding
  foreign books (D3), desks whose width is their carrying cost (D4).
- **Reads**: E4: for a party with a hedged foreign asset, the asset's FX revaluation and the
  forward's mark move against each other and the residual is the basis and the imperfection: an
  observer read per party, never zeroed.

### Module `index-futures`

- `requires: ['derivative-layer', 'indices', 'equity', 'dealers']`.
- **Kind** `index.future`: terms `{ index, expiry, multiplier }`; underlying = `view.index(id)`
  (Indices C3: the index is a settlement price); cash-settled at expiry against the index read;
  margined daily by 13a.
- **Participants**: desks hedging equity inventory (Dealer Desks E1: a desk long shares posts a sell
  schedule sized to its net inventory; E2: the hedge is a contract with a counterparty and its own
  margin), funds with mandates that track the index (a cheaper way to be exposed), a view.

### Module `options` (the price of optionality)

Positioned here because this specification requires the price of optionality in three places and
provides no market in which one is formed: `Bond N11.a` requires **a price the issuer pays** for an
early-termination regime, `Banks Lending` has prepayment at the borrower's option, and
`Short-Term Debt B4` states it outright — a committed line with no fee on undrawn headroom is *"a
free option the lender did not sell"*. It also costs the expectations layer its second dimension:
`§46 A3` makes the DISAGREEMENT load-bearing, and parties can disagree about a level while nothing
lets them disagree about **dispersion**, so a world of identical variance opinions is assumed rather
than cleared. It comes after the classes above and after the equity book clears (12c), because
`D3.a` forbids an underlying that exists only inside the derivative.

- `requires: ['derivative-layer', 'equity', 'indices', 'dealers']`.
- **Kind** `option`: terms `{ underlying, right: 'call' | 'put', strike: Price, expiry, exercise:
'european', multiplier }`; underlying = a listed line's own cleared print or an index that clears
  (`Indices C3`); two named counterparties, holder and writer, the contract an asset to one and a
  liability to the other with the marks summing to zero (`D1`, `D1.b`).
- **The premium is what clears** (`D7`, `D7.a`, Law 3): a market per (underlying, right, strike,
  expiry); the print is the premium per unit in the underlying's money. Clearing a volatility and
  deriving the premium from it is the shape `Bond N7.b` forbids for a bond — **implied volatility is
  the derived read an observer takes back off the premium**, like a spread off a bond price, and no
  parameter or store named `volatility.*` exists anywhere (a lint over the module, as `no-value-recipe`
  is over recipes).
- **Cash** (`D2`, `D8`, `D9`): the premium is a periodic leg that fires exactly once, holder → writer
  in the period the contract is struck; after that the mark moves every period as variation margin
  through 13a (`D8.a`, `D9`), so a writer's loss is cash it has to find. **Expiry is an event**
  (`D11`, `D11.a`): exercised at intrinsic value against the expiry print or expired worthless,
  truing up beyond what the marks already paid, and the contract ceases on both books at once.
- **Initial margin** from the underlying's own measured move through 13a's read, and the house's
  member limits binding **at the strike**: a member that cannot margin the position does not get it
  (`Derivative Layer E1–E4`).
- **Demand with a reason** (`B`-shaped, never a hedge ratio): a holder buying cover for a book it
  actually holds, sized by what its own surplus must absorb over the tenor at its management's own
  risk aversion, net of cover it already holds — the insurer's and the fund's participants at 13h
  post it, the desks post it here.
- **Supply with a reason**: a desk writing out of the same balance-sheet budget it runs every other
  class on, quoting from the move it expects the underlying to realise plus the return its capital
  requires on what the position consumes; 13h's funds add the strategy that is short dispersion.

### Module `bond-futures` (Sovereign I, and the trade that makes repo demand real)

`Sovereign I1`–`I3.a` specify a deliverable future on the benchmark bond and nothing owns them:
`docs/COVERAGE.md` carries `I1`, `I2`, `I3` and `I3.a` as MISSING with no evidence and had no row at
all for `I1.a`. That is not a finding and not an owner's decision — a specified clause that no item
names is a hole in the plan (Appendix C) — so it is owned here, with the classes, and the row for
`I1.a` is added to COVERAGE in this item.

- `requires: ['derivative-layer', 'sovereign-curve', 'money-market', 'dealers']`.
- **Kind** `bond.future`: terms `{ deliverable: InstrumentId (a named benchmark line), expiry,
contractSize }`; the price is **per unit of face** (`I1`); at delivery the contract settles to that
  bond's own cleared cash price — an asset leg of the line against cash at the settlement price, one
  instruction, DvP; margined by 13a.
- **The carry and the net basis** (`I1.a`): the bond financed in repo to delivery earns its coupon
  and pays the financing (both reads of things this world already prints: the coupon from the
  instrument's terms, the financing from item 11's secured book); the print against that is the **net
  basis**, a MEASUREMENT on the observer and never a setting. No `basis.*` parameter exists.
- **Who is on the line** (`I2`): a duration mandate short of duration goes **long** the future below
  carry (13h's insurers and pensions); a holder over its sovereign target **shorts** the excess above
  it (a bank treasury, from item 11's own target read); a dealer quotes **both ways** at carry.
- **The basis trade** (`I3`): long the cash bond, financed in repo, short the future when the basis
  pays for it — *"the largest single source of real repo demand in a real market"*. The participant
  posts all three legs in the same session from its own funded line; `I3.a` FORBID: it is funded,
  margined and **cut on a drawdown** — the trader's own module closes the position when its equity
  falls past its own tolerance, and nothing makes it whole. The repo demand it creates is a read
  (item 11's secured book, by reason).

### The index set: size and grade segments, a global line, and a vehicle on each

The machinery is built and the DECLARED SET is two. `Indices A1` admits any stated rule over any
stated constituent set at any stated weights; levels are reads chained across a rebalance and never
stored (`A2`, `B2.a`, `E2`); a weight is a count of a real thing (`B1`); a listed vehicle exists with
its NAV and its print published side by side and a premium nothing clamps (`Fund Shares E1`–`E4`,
`G1.a`); and a tracker holds the index's own basket and trades a rebalance for real (`Indices C1`,
`C2`). What is declared is an equity index per region (`D1`), a credit index per currency (`D2`), the
rate benchmark (`D3`) and the price level (`D4`), with one listed vehicle on the equity line — so the
segmentation a real market is organised by does not exist, and a **flow** cannot mean anything:
with one basket per region the constituent set changes only when a firm is born or dies, so `C1`'s
"a manager is measured against it, and that measurement drives flows" and `C2.a`'s "inclusion should
be visible in the constituent's price" have almost nothing to be about; `C2`'s simultaneity — every
tracker, at the same time — is a market of one; and no index measures anything across regions, which
leaves the currency layer with no aggregate equity claim in it.

Built here (the equity and default halves; the cash credit split and the loan index are 13f's, where
corporate paper and a secondary loan market start to clear):

- **The size split**: `largeCap`, `smallCap` and `allCap` per region, a stated rule on a real count
  (`B1`: capitalisation is a price times a count), read from the **constituents' own prints** and
  never from the index (`A3`: else the index selects its own members by its own level). The boundary
  must be **crossable**: a firm graduates into the large-cap line and drops out of the small-cap one,
  and every vehicle on both has to trade it.
- **The grade split in the default index**: IG and HY series, names fixed at the roll by grade
  (`CDS A5.a`), each clearing on its own book with its own index-against-single-name basis (`A5.b`).
  This world has **three assessors that disagree** (`Ratings`), so the rule states whose grade counts
  or how they combine, publicly and in advance (`A1.a`) — that disagreement is the point: an index
  boundary that depends on whose opinion you take is what makes a downgrade contestable rather than
  arithmetic.
- **The global line**: the only index that crosses regions, so its level is expressed in a stated
  money at cleared rates (`Spot FX`, `XI-12`) — and stating that money must not make it the vehicle
  currency of the model by construction (the rule names the money as data, and the level is a read
  through the period's own prints).
- **A vehicle per index**, holding that index's basket by mandate, which is how the tracker already
  works. It posts **reservations, not market orders**: a forced seller is real and a forced buyer is
  forbidden, which this build learned once by walking an equity index to 4153 in two sessions (12c).
- **All of it is data** (`Law 15`, `Indices D5`): rules, boundaries, constituent sets and weight
  choices are registry rows inside the one index system; no mechanism branches on a segment id. An
  empty basket reports Missing rather than a base level, which is already how this system behaves.

### Parameters

`cds.tenors`, `irs.tenors`, `fx.forward.tenors` (data); `cds.index.series.*` (data per grade:
names, weights, roll date); `regulation.riskWeight.cds.sold`, `regulation.riskWeight.derivative`
(policy, parliament); `option.strikes`, `option.expiries` (data: the ladder a market is opened on);
`bond.future.contractSize`, `bond.future.expiries` (data); `index.rules.*` (data: the stated rule,
its constituent set, its weights, its boundary and whose grade counts); no `recovery.rate` (D2.a),
no `basis.*`, no `parity.*`, no `volatility.*` and no `margin.rate` exist.

### Audit contributions

- `zeroSum`: each class's marks and legs enter 13a's family unchanged.
- `crossMarket`: CDS C3 basis and IRS C3 swap spread are reads, not checks (they are consequences);
  Indices C3: the future's settlement equals the index read on expiry (a check).
- `flows`: FX Forwards E3: no maturity passes without both legs settling (an instruction that fails
  is a recorded fail in the estate path, never a silent skip).
- `zeroSum` again for the option book: a premium paid once is received once, and an exercise moves
  intrinsic value from one named side to the other.
- `crossMarket`: `Sovereign I1` — a bond future's settlement equals the deliverable line's own cleared
  cash price on the delivery date (a check); the net basis and the implied volatility are READS and
  are reported as such, never checked against a level.
- `ownership`: every index's constituent set resolves to live instruments, and the vehicle on an index
  holds the basket its mandate names (`Indices B1`, `C1`).

### Files

```
packages/engine/src/clearing/contractMarket.ts (quotedAs: 'rate')
packages/engine/src/mechanisms/cds/{index.ts,contract.ts,event.ts,indexSeries.ts,participants.ts}
packages/engine/src/mechanisms/irs/{index.ts,contract.ts,curve.ts,participants.ts}
packages/engine/src/mechanisms/fx-derivatives/{index.ts,forward.ts,swap.ts,xccy.ts,cip.ts,participants.ts}
packages/engine/src/mechanisms/index-futures/{index.ts,contract.ts}
packages/engine/src/mechanisms/options/{index.ts,contract.ts,premium.ts,exercise.ts,participants.ts}
packages/engine/src/mechanisms/bond-futures/{index.ts,contract.ts,delivery.ts,basis.ts,participants.ts}
packages/engine/src/mechanisms/indices/{data.ts,segments.ts,global.ts} (the set, as registry rows)
packages/engine/src/mechanisms/funds/tracker.ts (a vehicle per index, posting reservations)
packages/engine/src/mechanisms/banks/{dealing.ts,dealing-quote.ts} (per-line makers; the aggregate book net of the treasury's target)
packages/engine/src/mechanisms/dealers/hedge.ts (E1)
packages/engine/test/{rate-markets,cds,cds-event,cds-index,irs,swap-curve,fx-forward,fx-swap,cip,xccy,index-future,desk-hedge}.test.ts
```

---

## Steps

- [x] Rate-priced contract markets: the print is a rate in a declared unit; struck at par by the solver, never by a formula run backwards (D7.b); malformed schedules throw; tests
- [x] `cds` kind: reference must be a party with defaultable debt; premium leg in cash stopping on the event; mark read from the spread curve; tests (A1–A4)
- [x] CDS participants with reasons on both sides and a view on both; naked sellers capitalised; the second-opinion assembly check holds for every CDS book; tests (B1–B5)
- [x] CDS curve across tenors; implied PD and recovery derived from spreads; basis vs the cash bond at every tenor both books print, as a read; tests (A1.d, C1–C4)
- [x] CDS event: from item 5's credit event; no premium, expected mark, held past maturity until the estate closes; settlement at the realised recovery; payment can fail the seller; termination; tests (D1–D5)
- [x] CDS index series: names fixed at the roll, a name's event settles its weight once per contract, the line runs on; own book; index-vs-single-name basis read; tests (A5)
- [x] Wrong-way risk in the buyer's reservation through the counterparty term; net notional per reference as a read; tests (E1–E4)
- [x] `irs` kind: two legs with own periodicity and accrual; floating fixes on the compounded overnight print; only the net moves; no notional exchange; tests (A1–A4, E1, E3)
- [x] IRS participants: fixed↔floating issuers from their own debt, banks' gap from their own repricing read, a view, desks; B2 pension declared PARTIAL to 13h; tests
- [x] Swap curve as a read of cleared fixed rates; forward rates derived; swap spread vs the sovereign curve read; no par rate from a discount curve exists (E2); tests (C1–C4)
- [x] `fx.forward` kind: two money legs in two currencies settle in one instruction at maturity; mark against the forward for the tenor left; margined; tests (A1–A3, E3)
- [x] FX swap as spot plus forward booked as a secured loan of one currency — NOT a third kind: a party short of a money buys it spot and fixes the cost forward, so what exists is a pair market and a forward book. A desk quoting a foreign line is charged the cost of THAT money, which the bank publishes beside its own (`12d-14` closed); tests (A4, D3)
- [x] CIP as an arbitrage banks take with four real legs bounded by balance sheet and capital; one basis read from prints; no parity formula anywhere (E1, B3.b); tests (B1–B4)
- [x] `xccy` kind: notionals exchanged at start and end at the original rate, periodic interest both legs, price includes the basis; tests (C1–C4)
- [x] `Contracts.open` validates `terms.kind === decl.kind` and calls `profile.validateTerms`, as `Instruments.add` does, and puts the notional `onTheGrid`; tests: a mis-kinded row is refused at the wire and not marked zero forever
- [x] The layer's `admits` drops the `committed` term — the cash balance is the source and `committed` re-derives what it already carries (Law 19); test: a member that has posted margin has its room cut once
- [x] FOUR of the five bilateral classes price from their OWN value first and reads the book's print only as the comparator that decides which side it is on: the `else` branch each class already has becomes the primary, and the order is posted at that number. `banks/dealing-quote.ts`'s `viewOf` is the pattern and its comment is the reason. Done for `cds` (the reference's cash bond), `irs` (the overnight fixing), `bond.future` (the cash deliverable) and `fx.forward` (its own carry, which its arbitrageur already used and its hedger did not) — each a read of ANOTHER market, so each party has a level whether or not this book has printed. **`xccy` could not be swapped and is positioned to 13f (`13b-4`)**: it quotes a BASIS, which is a residual whose parity value is zero, so naming one needs a borrower's own funding cost per money — and the parity level could not be posted anyway, because the order book treats a level at or below zero as the absence of a price. Tests: a book whose every participant has no outlook does not print the same level twice; a forward whose spot moved and whose book did not trade quotes the new number (XI-13, Law 3)
- [x] The option's bootstrap is its own arithmetic and not a tick: what the party's own view of the move says optionality is worth is already computed three lines below (`moves = expected × confidence`), so the fallback and the view become one expression and the branch goes. Test: an option book opens with buyers and sellers in its first session
- [x] **NOT BUILT, and the reason is the step above** (PLAN §5.2). `parLevel` was to give every participant a number of its own without reading the book — and the two steps above did that, inside each class, out of what that PARTY can see. A public par level on the profile would now be a SECOND expression of "what is this worth", which is the defect just deleted from the layer's own `capacityOf`/`admits` (Law 4), and worse: `parLevel(terms, at, reads)` sees no party, so it is one number every participant would read, and a single anchor everybody agrees on is what §46 A3 says takes the two sides out of a book. The kernel's own version of the question already exists and is `mark` (D8), which is what the zero-sum family and revaluation ask. Nothing is missing
- [x] FX hedgers' E4 RESIDUAL as an observer read per party. The hedgers themselves are built — a party short of a money covers what it is short of, less what it has already fixed, read off its own obligations (`positionIn`, `alreadyForward`) — and `hedgedResidual` computes what a hedge does not cover. What is missing is putting it on the observer, per party, beside the asset's own revaluation, so the two can be seen not to cancel (D2.a, E4)
- [ ] A desk's book is the position it TOOK: which lots the dealing line acquired, distinct from the portfolio its treasury holds for liquidity — one holding, two owners inside one bank, and the register cannot presently say which is whose; the aggregate limit then measured against the desk's own book (D1, D4); tests: a bank holding the liquidity standard is not over its dealing limit before it has quoted (`12d-12`)
- [ ] A VEHICLE per index, launched in a phase and not by the seed: a size segment's membership is read from its constituents' own prints (A3), which at period zero do not exist — so a tracker on one is launched the period its index first has a level, with a share of the float the trackers hold BETWEEN them (three vehicles each taking the whole of it is a market owned three times, measured: `etf.us` and `etf.equity.large.us` both opened with negative equity). Then the vehicle's own market drafts the trades it clears (`12d-4`); tests (Indices B2, B2.a, C1, C2, C2.a, E2; Fund Shares E1–E4, G1.a)
- [ ] Observer, the rest of it: bases, net notional per reference, hedged residuals, implied volatility as a derived read, the index set with its boundaries and its vehicles; year-long run green in two currencies; determinism; scenario test: swap spread and CDS basis behave differently calm and stressed (direction only). DONE so far: every contract book's own level with the word for what that level IS (`ContractPrintView.quotedAs` — money per unit, or a rate per annum), and those prints gathered by subject into curves in tenor order (`DerivativeCurveView`), each point a print somebody paid and no point between two of them
- [x] `index.future` kind: cash-settled against the index read at expiry; margined; tests (Indices C3)
- [x] Desks hedge inventory with a contract that has a counterparty and margin (Dealer Desks E1, E2); item 9's hedging PARTIAL closed; tests. The test-only forward is KEPT: it is what exercises the layer's own doors without depending on a class, which is what makes `derivative-layer.test.ts` a test of the layer and not of a class
- [x] A requirement that rounds below one piece of the money it is owed in is a STATED reason there is no margin, not a silence: the layer records it where it admits the trade, and a default fund sized from nothing is visible before somebody defaults; tests (G2, Law 8; `13a-1`)
- [x] Securities as collateral: a class that pledges a holding rather than paying cash, the lien in favour of the holder, the units gone from the free balance and back when the requirement falls; tests (D9, D9.a, Register D; `13a-2`)
- [x] A desk quotes the lines it MAKES: the makers drawn with the listing are read by the desk that quotes (Dealer Desks A3), so a bank that has never taken a view of a firm does not quote it (`12d-10`)
- [x] `option` kind: holder and writer, premium as a periodic leg that fires once, a mark every period as variation margin, expiry exercised at intrinsic against the expiry print or worthless, ceasing on both books at once; underlying a print this world clears; tests (D1, D1.b, D2, D3, D3.a, D8, D8.a, D9, D11, D11.a)
- [x] The premium CLEARS on its own book and implied volatility is the read taken back off it; no volatility parameter or store exists anywhere (lint + test); initial margin from the underlying's own measured move with the house's limits binding at the strike; tests (D7, D7.a, D7.b, Law 3, Derivative Layer E1–E4)
- [x] Option demand and supply with reasons: cover for a book actually held, sized by what the holder's own surplus must absorb at its own risk aversion net of cover held; a desk writing from its balance-sheet budget at the move it expects — which is its own outlook's CONFIDENCE, the width of its own surprises, so two parties whose prices agree and whose surprises differ disagree about what optionality is worth (Expectations A3's second dimension) — plus the return its capital requires; no hedge ratio anywhere; 13h's short-dispersion fund declared PARTIAL to there
- [x] `bond.future` kind: a named benchmark line as the deliverable, price per unit of face, delivery as DvP at the bond's own cleared cash price, margined; the crossMarket check on the delivery date; tests (Sovereign I1)
- [x] The carry, the net basis and the basis trade: carry read from the coupon its terms promise and the financing the secured book prints; the net basis measured and never set; a holder over target short above carry, a party without duration long below it; the trade funded, margined and cut on a drawdown against the trader's own tolerance with nothing making it whole; tests (I1.a, I2, I3.a)
- [x] The index set's RULES: large-, small- and all-cap per region from the constituents' own prints with a boundary a firm can cross both ways, the global line in a stated money at cleared rates, the default index in IG and HY series rolled on the assessors' published grades; every rule, boundary and weight a registry row (Indices A1, A1.a, A2, A3, B1, D5; CDS A5, A5.a, A5.b)
- [x] Coverage re-marked: fifty rows across CDS, IRS, FX Forwards, Sovereign I, Indices C3 and Dealer Desks, each citing the file that meets it
- [ ] PARTIAL rows for IRS B2/B2.a named to 13h; record entry
- [ ] Delete this file; worklist row 13b → done; commit and push

## What this pass learned, that the plan did not know

**A new book never gets a first print if every schedule quotes off the last one.** No order, no
print; no print, no order. What breaks it is that a derivative is never the only price of the thing
it is on: a credit's own debt is already trading, and an overnight book already printed what money
costs. So the first reservation in a CDS book is read off the reference's own bond against par over
the term it has, and the first in a swap book is the fixing the overnight market actually paid —
and from the moment each book prints, the difference between the two IS the basis C3 asks for.

**A level is held in money pieces per piece of the thing, and a rate said out loud is not that
number.** Three basis points a year on a unit of face is three hundredths of a cent. Schedules
posted at 0.0003 were below their own book's tick and dropped at the grid, which is how books full
of orders printed nothing at all.

**A party in a contract book observed nothing.** A fill in one produces a contract leg, not an asset
leg, so the expectations system — which reads the price a party traded at off its own fills — never
saw one. A party could be in a swap book every week and form no view of what a swap costs, which
left every derivative book with one side in it: the side hedging something. The open leg now carries
the line its level is a level of, and a fill in a contract book is an observation like any other.

**A listed contract has a CYCLE, not a fresh book every period.** An expiry of `now + life` opens a
new ladder every week and leaves the last one with one trade in it. Options, index futures and bond
futures are all dated to the next multiple of their own life (`nextCycle`), and an option's strike
is set from what the line was worth when its book OPENED, so the ladder stands.

**Exposure to a name is everything it owes you.** Protection is on a NAME, not on a line, and a
bank that read only the line the book was written on measured a fraction of its own exposure.

**Funds are not among the kinds that trade contracts, and that is 13h's to change.** A fund's equity
is zero by construction because its own claim on itself absorbs its book (Fund Shares A3) — and the
pass that re-marks that claim reads the register, where a contract is not (X1). A tracker with a
derivative position carries a mark its own share value was never told about: measured at 83,247,864
on `etf.us`.

## Exit criteria

Every class clears on its own book at a rate or price the solver struck; a credit event pays what
the estate actually recovered; the swap curve and the forward curve are reads of prints; the basis
is one number; a desk's hedge is a position with a counterparty.

## Guard

CDS D2.a, E4, B5; IRS E1–E3; FX Forwards B3.b, E1–E3; Corporate Credit H4.a; Derivative contract
D3.a, D7.b, X3; Sovereign I1.a and I3.a (the basis is measured, and the basis trader can lose);
Indices A3 (a boundary is read off the constituents, never off the index) and B2.a (nothing stores a
level); Law 3 (no premium derived from a volatility somebody chose).
