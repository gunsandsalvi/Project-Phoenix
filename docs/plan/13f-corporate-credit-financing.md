# Item 13f — Corporate credit, short-term debt, securities lending, prime brokerage, syndication

**Objective.** The corporate bond as a full instrument: a capital-structure decision by management,
covenants that are observed before default and can be waived at a price, a primary market that is
brought by a bank on a stated basis (best effort as agent, or backstopped by an underwriter that
commits), built from indications, priced at one level, allocated, walked away from,
tapped, and — when the deal is too big for one underwriter's own limit — **syndicated** among named
banks each carrying a stated share against its own limit and capital; a secondary market with the
existing dealers; holders that are leveraged by named lenders whose funding can be withdrawn;
restructuring decided by the holders; default as information. Short-term debt that must be rolled
into a market that can refuse, with a backstop that costs money. Securities lending where title
passes and the economics stay. Prime brokerage where leverage is a loan from a named broker whose
margin requirement is its own decision and whose line is never floored at zero. Syndicated loans as
one loan with several lenders of record. This closes Equity C5 and C7 (leverage and shorts) and
Banks Lending D4/D4.a.

**Read first.** §7 Corporate Credit (all; C10 in particular); §9 Short-Term Debt (all); §14
Securities Lending (all); §15 Prime Brokerage (all); §23 Banks Lending D4, D4.a, F3; §26 Dealer
Desks D1, D2 (the limit a syndicate share sits against); Bond contract N5, N11, N12, N13.a; Equity
C5, C7; XI-2 doors 1 and 4; §13 B2 (a fund lends securities); §28 (the client, at 13h). Code: item
3.1's primary block and `auction.result`, item 6's loan rows and quote shopping, item 9's desks and
their limits, item 7's estate waterfall and `reseat`, 13a's `collateral.claim` and `margin.calls`
phase, item 5's events.

**Clauses this item meets.** Corporate Credit A1, A2, A2.a, A2.b, A2.c, A3, A3.a, A3.b, A4, A4.a,
A4.b, A4.c, B1, B2, B2.a, B2.b, B3, B4, C1–C9, C2.a, C2.b, C7.a, C7.b, C10, C10.a, C10.b, C10.c,
C11, C11.a–C11.e,
D1–D8, D3.a, D3.b, D3.c, E1–E9, E4.a, E5.d, E6.a, E6.b, F1–F6, G4, G5, G5.a, G7, G8, H1, H2, H3
(H4 at 13b); Short-Term Debt A1, A1.a–A1.d, A2, A2.a, A3, B1–B5, B3.a, B3.b, C1–C4, C2.a, D1–D4,
E1–E3; Securities Lending A1–A5, A5.a, A5.b, B1–B4, B2.a, C1–C5, C2.a, D1–D3, D2.a, E1–E3; Prime
Brokerage A1–A4, B1–B5, B1.a, C1–C5, C1.a, C1.b, C3.a, C3.b, C4.a, D1–D4, E1–E4; Banks Lending D4,
D4.a, F3 (fully); Equity C5, C7; Fund Shares B2.

---

## Findings this item carries

### A loan's rate is derived twice, and the two do not agree (`12d-3`)

`mechanisms/banks/index.ts`: `publishQuotes` calls `quote(...)` per bank and records the keenest as
`credit.quoted`; `runRequests` calls `shop(...)` — which calls `quote(...)` again — and writes the
loan at `best.rate`. Same borrower, same period, same bank; quoted `0.013676115348016367` and written
`0.013676161104839884`. Three parts in a million apart, which is nowhere near the dust of either
derivation: the inputs move between the two phases (a bank's cost of funds is read afresh, and so is
what it has seen default). **Law 4**: one fact, two writers. A borrower took a loan at a rate it was
not quoted, and both numbers are published under its name. The fix is a read, not a second
derivation: what is written is what was QUOTED, and where the quoting bank cannot lend after all,
that is a refusal to record rather than a silently different price (`C3.a`: declined volume is
visible). It lands here because this item is the one that opens the facility and the syndicate —
two more places a quote becomes a row.

### An exchange-traded fund's desks cannot create, because none of them holds the basket (`12d-8`)

Measured in the rig at three banks and forty-eight firms over sixty periods: 11.5 moved every listed
line's float off the dealing desks and onto the household cells that save, and nothing moves a share
back — at period 56 the only named holder of `equity.firm.30` is the fund itself. A desk that has not
got the basket cannot create (`Fund Shares F1`), so `E3` runs one way only and `E3.a`'s premium —
measured at **0.28 of NAV, twenty-six times what a period of carry costs** — has nobody able to close
it. `etf.test.ts` stands (1) in with a fixture that opens the participants holding the basket. The
mechanism that answers it is in this item: **securities lending**, where a desk borrows the units it
has to deliver and gives the lender collateral for them. Seen at
`packages/engine/src/mechanisms/funds/etf.ts`.

### Every bank posts the same deposit board, so no depositor ever moves (`12d-11`)

Measured over 52 periods of two rig seeds: `deposit.moved` fires **zero** times, no bank draws the
window, no session refuses one. The three banks' boards are identical to the last digit
(`retail 0.012500073062255977` at every one of them). `defended` (`banks/treasury.ts:334`) matches the
keenest rival exactly whenever that rival pays more than this bank's own offer and less than what the
money is worth to it. Matching is free, perfect and instant, so every board converges on the keenest
bank's own offer every period and the `depositMargin` each bank was drawn with never shows. That is
`Seed B4`'s failure mode stated the other way round — a sector of equals never produces a market —
and `§46 A3`'s disagreement has been matched away. Six tests are named on it and stay red until it is
answered (`deposits.test.ts` 3, `run.test.ts` 3: the whole E1 → E3.a → B7 → C4.b → failure chain
cannot start). `A1.d`'s stickiness is deliberately NOT the answer (`setBoard` refuses to subtract a
depositor's cost from a rate, and is right to). What IS the answer is in this item: a bank with a
cheaper wholesale alternative — its own paper, a committed facility, a note it can roll — does not
match a deposit it can replace, so what makes matching imperfect is the **price of the other
funding** rather than a friction anybody wrote down. Seen at
`packages/engine/src/mechanisms/banks/treasury.ts:334`.

---

### An issuer books a PROFIT when its own debt falls in price (from the review — a decision the owner must take)

`ledger/settlement.ts`, on every holder-to-holder trade of a claim:

```ts
if (this.d.registry.instrumentKind(inst.kind).liabilityOfIssuer) {
  bump(issuerOf(inst), mul(op.totalQty, carryingOf(op.fromDebit) - basis, 'issuer re-mark'), inst.id);
}
```

The issuer's equity moves by `qty × (what the seller carried it at − what the buyer paid)`. So when a
firm's bonds trade DOWN — which is what happens as it approaches distress — `carrying − basis` is
positive and **the issuer's equity RISES**. `world/revalue.ts` does the same in `issuerMoves` for the
period's re-marking, so it is one modelling choice made consistently in two places, which means
fixing it is one decision and not two.

This is the own-credit gain, and it is famous because it is perverse: a firm on its way to insolvency
books profits from the market's growing doubt that it will pay. It fights XI-3 directly — a firm
cannot become insolvent if its own distress is a revenue line — and it fights §31/Banks Capital the
same way for a bank whose subordinated paper is falling. The citation given is Register B3, "a
liability is the same number read from the other side", which is true about the BALANCE SHEET and
does not settle where the change GOES.

**It belongs here** because this is the item where an issuer's own paper trades in volume against a
real credit assessment, and where a name walking towards default is the subject rather than an edge
case. Real accounting sends own-credit movements to other comprehensive income precisely so they do
not flow through profit, and this world has the machinery: the revaluation account
(`Register.moveRevaluation`) already demonstrates the pattern of a second equity-like account moved
by exactly one thing, built for the central bank's foreign reserves (Currency D2.a, §31 A2.c), and
§48's reporting can hold the distinction.

**What the owner decides** is whether the own-credit component lands in a named account of its own or
stays in the line that says what the issuer earned. The spec reserves neither answer, so it is stated
in the record either way and the code carries one citation for it.


## Design

### Sub-item 13f.1 Kernel: commitment markets

A syndicate is a **clearing of commitments**: the demand is one deal's size (units of face, or money
for a loan) and the supply is the commitment schedules of the banks willing to carry it, each a size
at a fee (underwriting) or a margin (a loan). The kernel's `MarketDecl` gains a variant `{ kind:
'commitment', deal: { requester, size, unit, quotedAs: 'fee' | 'margin' } }`; the solver is
unchanged (one side is a single point of demand; the other is schedules); the outcome fills shares
at one clearing fee/margin with pro-rata rationing at the marginal level, and reports `filled <
size` explicitly (a partial syndicate is an outcome, never a silent shrink). The market runs in the
`markets` phase **before** the primary market of the same deal (C10.a: shares are struck before the
book opens); a participant's commitment schedule is built by its own module from its own remaining
limit, so no share can exceed a member's limit by construction (C10.b), and the kernel never assigns
one (the fill is what the member posted). Both the underwriting syndicate (Corporate Credit C10) and
the loan syndicate (Banks Lending D4.a) are this one market kind.

### Module `corporate-credit`

- `requires: ['firms', 'credit-events', 'estate', 'dealers', 'ratings', 'sovereign-curve']`.
- **Kind** `bond` (corporate): terms `{ issuer, coupon: fixed | floating(benchmark, spread),
maturity, rank: seniority (honoured by item 7's waterfall: B3), callRegime (B1: stamped at issuance
from what the issue is: `makeWhole`|`nonCall(periods)`|`softCall`: a profile dispatch on the
regime, data per issue), covenants: [{ metric: 'leverage' | 'coverage' | 'restrictedPayments',
limit }] }`; display name issuer + coupon + maturity (Law 9); `due`: coupons and principal (bullet
  or amortising: F3) through the kernel's corporate actions; `defaultOn`: a missed payment or a
  breached covenant (B2, N12); `ranking` from `rank`.
- **The issuer's decision** (A2, A2.b): in `firms.decide`: management's target is the tightest
  covenant line on its existing debt moderated by its own risk aversion (preference `firm.
riskAversion`, dispersed) approached at its own pace (`firm.horizon`, item 10); issuing is a
  decision from its programme (item 10) and its refinancing need (F5: a maturity within its horizon
  is refinanced by a new issue at the market's price on the day); the structure that results is an
  outcome (A2.c: measured).
- **Coverage** (A3): a read: operating cash flow over interest plus scheduled principal, from the
  ledger (A3.a, A3.b).
- **Assessment** (A4): every holder's own PD model of the issuer (item 5's shape per assessor) and
  the ratings agency's grade (item 12): opinions held by somebody, disagreeing (A4.b), changing
  (A4.c: events).
- **Covenants** (B2.a, B2.b): tested each period in `credit.events` against the issuer's **published**
  accounts (Observer A5: the previous period's statement, public with a lag: the module's phase reads
  `view.published(issuer)`); a breach is a journaled event; the issuer may propose a **waiver** (a fee
  per unit of face) which holders accept or refuse by the same vote as restructuring (below); a
  cured breach is an event too.
- **The primary market** (C): the issuer appoints a bank (C1) on a **stated basis** (C11): every
  desk quotes **two** fees — a best-effort fee for placing as agent with no commitment (C11.a), and
  a backstop fee with a commitment from its remaining underwriting limit (C11.b; Dealer Desks D1, D2:
  `desk.limit.underwriting` preference); the issuer chooses basis and bank from its own outlook of
  the book against the fee gap (C11.c: a confident issuer saves the fee; one that must have the money
  buys the backstop; the deal's `basis` term is stamped and public). The fee is an instruction issuer →
  bank at pricing (C6). On a **backstopped** deal the underwriter **commits** to take what the book
  does not (C7) and its unplaced share lands in its inventory (C7.a: an ordinary holding it must fund
  and rent, item 9). On a **best-effort** deal the unplaced remainder is **not issued** (C11.d: the
  issuer's `issued` grows only by what the book took; a selling group is a fee-sharing list, never a
  commitment). C7.b and C11.e (fee vs risk; who buys the backstop) are measured. **Bookbuilding** (C2): the primary block of item 3.1 opened with the
  underwriter as the seller of record; indications are schedules (C2.a); the book is information the
  observer shows after pricing (C2.b); it prices at one level (C3: the solver); **walk-away** (C4):
  the issuer's reservation is a spread over its own cost expectation from its outlook: below it the
  primary returns `noOverlap` and the deal never existed (no instrument is created; the journal
  records `deal.pulled`); allocation is the solver's pro-rata at the marginal level (C5).
- **The syndicate** (C10): when the deal's size exceeds the chosen lead's commitment, the lead opens
  a **commitment market** for the deal (13f.1): each other desk posts a commitment schedule from its
  own remaining limit and its own capital headroom (item 11's read: a share sits against its own
  capital); the lead's share is what it committed itself (C10.a: not the remainder); the fee is split
  pro rata to shares (C6, C7); if `filled < size`, the issuer decides: **downsize** to `filled` or
  **pull** (C4); the outcome `syndicate.failed { issuer, size, filled }` is journaled publicly when
  the issuer pulls (C10.c: an observable event with a named issuer; the largest deal the market can
  bring is Σ willing members' limits, and that figure is a read at 16); after the book, the unplaced
  paper is split among members **by their shares** into each member's inventory (C10.a, C7.a), each
  against its own limit (C10.b: a member at its limit posted nothing, so it holds nothing).
- **Taps** (C8): a primary block on an existing instrument, cleared in the same solve as its
  secondary market (the primary block's supply schedule is added to the sellers' side of the
  existing market with the issuer's reservation; a fill credits the issuer's `issued`); buyers hold
  the existing instrument; the issuer's decision: tap when it has printed paper near the tenor,
  debut otherwise.
- **Committed facilities** (C9, Short-Term Debt B4): item 6's `loan` kind gains a `facility` variant:
  one line per (lender, borrower) with a limit, a struck margin, a term and a **commitment fee on
  undrawn headroom** paid each period by instruction; a draw is a row increase on the existing line
  at its margin; a new line opens only when none is live.
- **Syndicated loans** (Banks Lending D4, D4.a): a borrower whose request exceeds any single
  lender's large-exposure headroom (F3) is offered by the keenest quoting lender (the **lead**) into a
  commitment market quoted as margin; each lender's fill is its **own row** per (lender, borrower)
  with the syndicate id, at the one struck margin; the lead takes an arrangement fee by instruction;
  each share sits against its own capital (B2.a) and its own large-exposure limit (F3); a sale of a
  share afterwards is item 6's D4 (a reseat with a money leg at a cleared price in a loan market).
- **Secondary** (D): exists (items 3, 9); D7 accrued from item 3.2; D5 measured; D8 a lint.
- **Holders** (E): E6: a leveraged holder funds the position from a named lender (a repo from item
  11 or a margin loan from the prime broker below: E6.a); withdrawal (the lender's decision not to
  roll) forces a sale (E6.b, XI-2 door 4); E8: pledge through liens (exists); E9: the observer's
  statement per holder: position, price, income, realised and unrealised P&L (E4.a: from lots).
- **Life** (F): F4: a call is the issuer's decision when refinancing is cheaper by more than the
  regime's cost (make-whole price computed from the curve as the regime states: a **contractual
  formula stated in the terms**, not a market price: allowed because the regime is the contract's
  own promise); prepayment on amortisers likewise.
- **Restructuring** (G7): on a default or a breach, the issuer proposes (from its own outlook of
  what it can pay): amended terms, or debt exchanged for equity; every holder decides from its own
  expected recovery in liquidation (its estimate: its outlook of what the estate would fetch, read
  from its own view of the firm's published assets and the market prints for comparable assets)
  against the proposal's value at its own reservation; the vote is by face; a majority above the
  contractual threshold (`bond.restructuring.majority`, a term of the issue: data) binds all; the
  exchange is instructions (redeem the old rows, issue the new); a rejected proposal opens the
  estate (item 7). Waivers (B2.b) use the same vote at a fee.
- **Information** (G8): a default is a public event; every holder's PD model and the agency's grades
  update from public events (existing outlooks); a test asserts other issuers' reservations move
  after an event without any correlation parameter.
- **Aggregate** (H): H1 is 12's credit index; H2, H3 are reads at 16.

### Module `short-term-debt`

- `requires: ['corporate-credit', 'money-market', 'funds', 'treasury']`.
- **Kind** `discountNote`: any issuer (A3: the state's bill from item 3 is this kind with the state
  as issuer: item 3's `bill` kind is **renamed and generalised** here in one change; the record names
  the read that replaces the old id), no coupon, issued at a discount, redeemed at par (A1.a), tenor
  under a year (A1.b), senior unsecured (A1.c), no call (A1.d); yield derived from price and days on
  the stated convention (A2, A2.a: `dayCount` a term).
- **Issuing** (B): the issuer's cash plan (item 4) funds a short known need (B1) with paper when
  cheaper than its term quote (B2); **rolling** (B3): a maturity is redeemed by the kernel; the
  refinancing is a new primary that can fail (B3.a: `noDemand` leaves the issuer to pay from cash it
  may not have: item 5's event: B3.b: a run); the **backstop** (B4): a committed facility (above)
  with its commitment fee, or a liquid buffer; the maturity profile is a read (B5); E1: no automatic
  roll: a test asserts every roll is a primary market outcome.
- **Buyers** (C): money funds (item 8), corporate treasurers (firms' cash plans), bank liquidity
  books (item 11), each comparing yield against its alternatives (C2: deposit, repo, the floor) and
  with a **limit per issuer** from its own PD model (C3); C4 measured.
- **Trading** (D): a secondary market per note (D1); D3: eligible collateral in repo per the central
  bank's data; D4: the spread over the equivalent-tenor sovereign note is a read.

### Module `securities-lending`

- `requires: ['funds', 'insurers' (13h; until then the module's lenders are funds), 'dealers',
'derivative-layer']`.
- **The transaction** (A1, A2): a `secLoan` row (module-private state referencing register rows):
  the lender's units move to the borrower's holding (title passes: the register shows the borrower as
  holder; holdings still sum to issued: E2) and the borrower delivers collateral: cash (a
  `collateral.claim` from 13a issued by the lender) or securities (a lien in the lender's favour)
  worth the loan plus a **haircut** (C1: the lender's own decision from the security's measured move
  in its view: no table); the lender holds a `secLoan.claim` instrument (issued by the borrower;
  `liabilityOfIssuer: true`; `pricing: 'derived'`: units × the security's print: a claim to return
  of the units) so its economic exposure is a read of one security (E2).
- **Economics stay** (A3): the kernel pays coupons and dividends to the registered holder (the
  borrower); the `secLoan.claim`'s `due` is the **manufactured payment** borrower → lender of the same
  amount on the same date.
- **Marks** (C2): each period the collateral requirement is re-measured at the security's print; the
  difference moves as cash or lien adjustment in `margin.calls` (C2.a); **cash collateral is
  reinvested** by the lender (C3: its own portfolio decision; the reinvestment's loss is the
  lender's: the claim to the borrower is at par, the reinvested asset marks).
- **The fee** (A5): a market per instrument for borrow: lenders post size at fee from their mandates
  (B2, B2.a: a fund lends within its mandate with a limit; Fund Shares B2), borrowers post size at
  fee from their reasons (B1: to deliver a short, cover a fail, deliver on a derivative); the fee
  clears (A5.a); with cash collateral it is expressed as a rebate (A5.b: the same number: the module
  books one number and the observer shows both forms); B3: an agent (a desk) may intermediate for a
  share of the fee; B4: the lendable pool is a read of willing holders' posted sizes and it caps shorts.
- **Termination** (A4, D3): recall by the lender (its decision: it wants to sell or vote) forces the
  borrower to return or find another lender in the next borrow session or close its short (a forced
  buy: XI-2); a **failed return** (D1) terminates: the lender keeps the collateral and buys the
  security back in the market at whatever it costs; the difference is the borrower's loss or the
  lender's; D2: a squeeze is emergent (measured at 16).
- **Re-pledging** (C5): a security received as collateral may be re-pledged by the receiver where
  the agreement permits (a term): the register's lien references the prior lien, so the chain is
  traceable by the names family.
- **Shorts** (E1, Equity C7): a sale of borrowed units is a sale of held units: the kernel's register
  already refuses a sale of units not held, so a short without a borrow is impossible; the short's
  economic position is the `secLoan.claim` liability.

### Module `prime-brokerage`

- `requires: ['bank-lending', 'securities-lending', 'derivative-layer', 'funds']`.
- **The relationship** (A1): a row (broker bank, client) with terms; the broker **holds** the
  client's assets (A2: the client's holdings carry a lien in the broker's favour as portfolio
  collateral; the broker's view reads the client's positions **at that broker** only: A3: a client
  with two brokers is seen whole by neither: Observer A4 by construction).
- **Financing** (B): a `loan` row broker → client (B1, B1.a: leverage is this row) at a rate above the
  broker's cost of funds (B2: the broker's quote, item 6's shape), growing its balance sheet and
  consuming its capital and liquidity (B3: item 11's reads); the short side is financed (B4: short
  proceeds are held at the broker under the lien; the borrow comes through securities lending);
  B5: the client's leverage is a read of the loan against its equity and equals what the broker lent
  (one row: a test, not a check against itself).
- **Margin** (C): the broker's **requirement** on the whole portfolio it sees from its own measured
  move of that portfolio (C1: the net move with offsets: C1.a; C1.b: a decision of the broker's risk
  function: its own view, its own horizon preference, raised when it likes what it sees less: C4:
  worse markets (its surprise width), worse client (its PD of the client), worse own position (its
  capital headroom)); re-measured each period in `margin.calls` (C2); the **available line** = limit −
  drawn − requirement, **never floored** (C3.b: a negative available line is the shortfall and the
  call); the call is met from the client's cash, or by sales it posts (C3.a: `forced.sale`), or the
  broker **liquidates** (D1: closes positions by forced sales from the client's holdings under its
  lien into the next session at cleared prices: D3); C5: an unmet call has this consequence and a met
  call moved cash.
- **Default** (D): the liquidation's proceeds below the loan are the broker's loss against its
  capital (D2); the loss chain is traceable through the journal (D4; measured at 16).
- **Concentration** (E): exposure per client is a read (E1); the broker's own liquidation-value read
  of a concentrated book (E2: the measured move of a large position is wider: its own view); E4: a
  limit per client (preference of the broker's risk function); Equity C5 (leverage on shares) is met
  by this row.

### The schedule shapes the bond contract does not yet admit

`Bond N5` admits **exactly three** coupon shapes — fixed, floating, zero — and says so deliberately;
`Corp Credit F3` admits bullet or amortising principal. So there is no indexed coupon and no indexed
principal anywhere, and no step-up, sinking fund, payment holiday, or coupon contingent on a declared
event. The shapes land here because this is the item that opens the corporate bond's terms; each is a
**term of the paper** read by the kernel's corporate actions through the kind's `due` and
`cashFlows`, and none of them is a mechanism of its own.

- **The index-linked obligation** is the one that matters, because it is the only instrument in which
  **inflation itself is priced**. This world has a PPI and a CPI (`Goods G1`, `Indices D4`, completed
  at 13c) and gives every party its own inflation outlook (`§46`), and there is nowhere those outlooks
  **meet and clear**: no breakeven, therefore no market-priced expectation of inflation anywhere, and
  no **real** cost of funds for the sovereign whose paper the rest of the model treats so carefully.
  `XI-13` is what makes it bite — the market must be able to disagree with the model — and on
  inflation it cannot, because there is no instrument to disagree in.
  - The reference is **this world's own CPI print**, at a stated publication lag, applied to the
    **principal** as well as the coupon, with the uplift a real claim that settles. `Law 2` admits
    imported primitives and forbids imported equilibria: an imported inflation series would be the
    second kind. `Law 8` applies at the writer — the lag, the base period and the periodicity are
    part of the number and are named in the terms.
  - The **treasury** issues linkers alongside its nominal grid (item 3's programme decides the mix
    from its own quotes), so `XI-9`'s funding constraint is faced in real terms too, and the
    **breakeven** is a READ of two cleared prices at the same tenor — never a stored series, never an
    input to anything that prices either leg.
- **Step-up** (a coupon that rises on a stated date or on a stated rating event) and **sinking fund**
  (principal retired to a schedule) are the two commonest ways a real issuer shapes its own ladder,
  which `Sovereign A2` asks to be a programme rather than a calendar; both are terms, and the sinking
  fund's retirements are ordinary buybacks at the schedule's dates.
- **Payment holiday** is the restructuring-short-of-default that `Corp Credit B2.b` (waiver and cure)
  and `G7` (restructuring) already imply and that nothing can express as a *term* of the paper: the
  holder's vote grants it, the schedule moves, and no default event fires.
- **Contingent coupon**: a coupon that pays only when a stated, observable condition holds (a
  published metric of the issuer's own accounts, a declared event) — the condition is read from a
  public event, never from a probability.

### The credit index in two grades, and the loan index

`Indices D2` declares one credit index per currency, so **there is no quality dimension in credit as
a market**: nothing separates investment grade from high yield, and the single most recognisable
shape of a risk-off move — high yield widening while investment grade holds — is not expressible in
the cash market at all. The **fallen angel** does not exist either: a name crossing the grade boundary
should be a forced sale by every tracker on one side and a forced purchase of the same line by every
tracker on the other, in the same period, on top of `Insurers C5`'s forced sale on a downgrade and
`Ratings C1`'s mandate — and there is no boundary for a downgrade to cross. (13b builds the equity
size split, the global line and the two-grade DEFAULT index; the cash credit split is here, where
corporate paper starts to clear and the credit index's basket stops being empty.)

- **IG and HY per currency**: a stated rule struck on the assessors' grades, and this world has
  **three assessors that disagree** (`Ratings`), so the rule says whose grade counts or how they
  combine, publicly and in advance (`A1.a`). That disagreement is the point: an index boundary that
  depends on whose opinion you take is what makes a downgrade contestable rather than arithmetic.
- **The leveraged loan index waits on this item and not on a rule**: `A2` says an index reads cleared
  prices and nothing else, and a loan is carried at cost and names no market at all
  (`Banks Lending D1`). The secondary loan market this item opens (`D4`: a share sold by reseat with
  a money leg at a cleared price) is what makes the line possible, so the index and its vehicle come
  **after** it, last of the four.
- Its vehicle is then the one place a real **liquidity mismatch** lives — a claim redeemable on demand
  over an instrument that settles slowly — with the cost of a late sale landing on the holders who
  stayed (`Fund Shares C4.a`) and `XI-2`'s forced seller behind it.

### Parameters

`bond.indexation.lag` (a term per issue: the publication lag of the CPI print it references),
`bond.stepUp.*`, `bond.sinkingFund.*`, `bond.contingentCoupon.*` (terms per issue: data);
`index.rules.credit.*` (data: the grade boundary and whose grade counts); no `inflation.*` series and
no breakeven store exist. `desk.limit.underwriting` (preference per desk); `firm.riskAversion` (preference, dispersed);
`bond.restructuring.majority` (a term per issue: data); `facility.commitmentFee` is a **quote**, not
a parameter; `secLoan.haircut` does not exist (a decision); `broker.limit.perClient`, `broker.
horizon` (preferences per broker); `discountNote.dayCount` (a term: data). No `recovery`, no
`fee.table`, no `margin.rate` exist.

### Audit contributions

- `ownership`: securities on loan: holdings sum to issued with the borrower as holder; every
  `secLoan.claim` matches a borrower's holding of the units (E2).
- `flows`: fee, manufactured payments, commitment fees and arrangement fees each two-sided per period.
- `accounts`: the underwriter's and each syndicate member's unplaced paper appears in its inventory
  in the same period as the book closed.
- `names`: every syndicate share names a member that posted it; every facility names both sides;
  every re-pledge chain resolves to a live lien.
- `flows` again for the shapes: an indexed coupon's uplift is a claim that SETTLES, so what a linker
  pays is two-sided like every other payment and no uplift is ever a re-mark with nobody on the other
  end; a sinking fund's retirement moves issued amount and cash together.

### Files

```
packages/engine/src/clearing/commitmentMarket.ts, clearing/market.ts (13f.1)
packages/engine/src/mechanisms/corporate-credit/{index.ts,bond.ts,schedules.ts,structure.ts,covenants.ts,primary.ts,underwriter.ts,syndicate.ts,tap.ts,facility.ts,restructuring.ts,holders.ts}
packages/engine/src/mechanisms/sovereign-instruments/linker.ts (the treasury's indexed line)
packages/engine/src/mechanisms/indices/data.ts (the credit grade boundary; the loan line)
packages/engine/src/mechanisms/banks/{index.ts,treasury.ts} (one written rate; a board that is not matched away)
packages/engine/src/mechanisms/bank-lending/syndicated.ts (D4.a: the loan syndicate on the commitment market)
packages/engine/src/mechanisms/short-term-debt/{index.ts,note.ts,roll.ts,buyers.ts}
packages/engine/src/mechanisms/securities-lending/{index.ts,loan.ts,collateral.ts,fee.ts,manufactured.ts,recall.ts}
packages/engine/src/mechanisms/prime-brokerage/{index.ts,relationship.ts,financing.ts,margin.ts,liquidation.ts}
packages/engine/src/mechanisms/sovereign-instruments/… (bill → discountNote with the state as issuer)
packages/engine/test/{commitment-market,corporate-bond,capital-structure,covenants,underwriter,syndicate,bookbuilding,tap,facility,syndicated-loan,leveraged-holder,call-refinance,restructuring,default-information,discount-note,roll-run,note-buyers,sec-lending,manufactured-payment,borrow-fee,recall-fail,short,prime-financing,portfolio-margin,liquidation}.test.ts
```

---

## Steps

- [ ] The own-credit decision taken and implemented: the component of an issuer's re-mark that comes from its own credit lands where the decision says — a named account of its own, on the revaluation account's pattern, or the equity line — in `ledger/settlement.ts` and `world/revalue.ts` together, one citation, one writer; tests: a name walking towards default does not book a profit on the way down unless the decision says it does (Register B3, XI-3, §48)
- [ ] 13f.1 Kernel: commitment markets (one point of demand, commitment schedules, one clearing fee/margin, pro-rata at the margin, `filled < size` reported, ordered before the deal's primary); tests
- [ ] `bond` corporate kind: coupon forms, seniority honoured by the waterfall, call regime stamped at issuance by dispatch, covenants as terms, display name; tests (A1, A2.a, B1, B3, B4, N11–N13.a)
- [ ] The capital-structure decision: management's own target from the covenant line and its risk aversion at its own pace; issuance and refinancing as decisions; the structure as an outcome; tests (A2, A2.b, A2.c, A3, F5)
- [ ] Covenants tested on published accounts with the lag; breach and cure as events; waiver at a fee by the holders' vote; tests (B2, B2.a, B2.b, G1)
- [ ] Placement basis and the bank: desks quote a best-effort fee and a backstop fee with a commitment from their own limit; the issuer chooses basis and bank from its own outlook against the fee gap; fee by instruction; on a backstopped deal the unplaced paper is the underwriter's inventory, funded and rented; on a best-effort deal it is never issued; tests: an agent can never be left holding paper (C1, C6, C7, C7.a, C11, C11.a–C11.d)
- [ ] The syndicate: a commitment market opened by the lead when the deal exceeds its own commitment; shares struck before the book from each member's own limit and capital; the lead's share its own; fee split by shares; `filled < size` → the issuer downsizes or pulls; `syndicate.failed` journaled with issuer and size; unplaced paper split by shares; tests: no member ever holds above what it posted (C10, C10.a, C10.b, C10.c)
- [ ] Bookbuilding: indications as schedules, one price, allocation from the book, walk-away with no instrument created; tests (C2, C2.a, C2.b, C3, C4, C5)
- [ ] Taps cleared in the same solve as the existing paper; debut vs tap as the issuer's decision; tests (C8)
- [ ] Committed facilities: one line per pair, draws at the struck margin, commitment fee on undrawn headroom every period; tests (C9, Short-Term Debt B4)
- [ ] Syndicated loans on the commitment market: one loan, several lenders of record, a row per (lender, borrower), one margin, the lead's arrangement fee, each share against own capital and large-exposure limit; sale of a share as item 6's D4; tests (Banks Lending D4, D4.a, F3)
- [ ] Leveraged holders: funding from a named lender (repo or margin loan), withdrawal forces a sale; pledge; the holder's statement with realised and unrealised P&L; tests (E4.a, E6, E6.a, E6.b, E8, E9)
- [ ] Calls under the regime and prepayment as the issuer's decisions; tests (F4)
- [ ] Restructuring: the issuer's proposal, each holder's decision at its own expected recovery, vote by face, contractual majority binds, exchange by instructions, rejection opens the estate; tests (G7)
- [ ] Default as information: other issuers' reservations and grades move after an event with no correlation parameter; test (G8)
- [ ] `discountNote` kind for any issuer, item 3's bill generalised in one change with the record naming the replacing read; discount pricing on a stated convention; types by issuer; tests (§9 A)
- [ ] Rolls as new primaries that can fail; a run reachable; the backstop costs money every period; maturity profile as a read; no automatic roll; tests (§9 B, E1, E3)
- [ ] Note buyers with horizons, alternatives and a limit per issuer from their own PD; secondary trading; repo eligibility; spread over the sovereign note as a read; tests (§9 C, D, E2)
- [ ] Securities lending: title passes with holdings summing to issued, collateral at the lender's own haircut, marks each period, manufactured payments on the issuer's dates, fee clears with the rebate as the same number, cash collateral reinvested at the lender's risk, an agent's share; tests (§14 A, B, C)
- [ ] Recall, failed return with buy-in, re-pledge chains traceable, lendable pool capping shorts, no short without a borrow (Equity C7); tests (§14 D, E)
- [ ] Prime brokerage: the relationship and its lien, the margin loan above cost of funds consuming capital and liquidity, the short side financed, portfolio margin from the broker's own measured move with offsets, calls in `margin.calls`, the line never floored, requirements raised on the broker's view, liquidation into the market with the shortfall on the broker's capital, exposure and limit per client, the multi-broker blind spot (Equity C5); tests (§15)
- [ ] The schedule shapes as terms of the paper: an **indexed** coupon and principal off this world's own CPI print at a stated lag and base period, a step-up, a sinking fund, a payment holiday granted by the holders' vote, a contingent coupon read off a public event; the treasury issues a linker beside its nominal grid and the **breakeven** is a read of two cleared prices stored nowhere; tests: no imported inflation series, no uplift without a settlement (Bond N5, Law 2, Law 8, XI-9, XI-13)
- [ ] The cash credit index in two grades per currency, struck on the assessors' published grades with the rule saying whose counts, publicly and in advance; a fallen angel is a forced sale by every tracker on one side and a purchase by every tracker on the other in the same period; tests (Indices A1.a, A3, B2, C2, C2.a; Ratings C1; Insurers C5)
- [ ] The leveraged loan index and its vehicle, after the secondary loan market this item opens: a claim redeemable on demand over an instrument that settles slowly, with the cost of a late sale landing on the holders who stayed; tests (Indices A2, Fund Shares C4.a, XI-2)
- [ ] One rate per loan (`12d-3`): what is written is the quote that was published, and a bank that cannot lend after all records a refusal rather than a different price; the second derivation is deleted and the record names the read that replaces it; tests (Law 4, Law 19, Banks Lending C3.a)
- [ ] A desk that must deliver a basket can BORROW it (`12d-8`): creation runs both ways once the borrow market exists, and the exchange-traded fund's premium has somebody able to close it; test (Fund Shares E3, E3.a, F1)
- [ ] A deposit board that is not matched away (`12d-11`): a bank with a cheaper wholesale alternative — its own paper, a facility, a note it can roll — does not match a deposit it can replace, so the drawn `depositMargin` shows and a depositor has a gap to move on; tests: `deposit.moved` fires, and the failure chain E1 → E3.a → B7 → C4.b can start (Banks Funding A1.d, B1, D1; Seed B4; §46 A3)
- [ ] **`13b-4`: the cross-currency swap has no number of its own, and its book's natural level cannot be posted.** The other four bilateral classes price from their own value first (13b); this one reads `view.print(t.book)` and returns nothing when the book has never printed, so it is the one class that cannot open its own book and, once open, posts where the market already is for ever. What an xccy book quotes is a BASIS — a residual whose covered-parity value is zero — so naming a level of one's own needs what a borrower would pay to raise the money DIRECTLY in each money, which is this item's corporate funding curve per currency. `carryOf` is the forward exchange rate and is the wrong unit entirely. **And a kernel decision travels with it**: `clearing/market.ts`'s `onTheGrid` drops any limit at or below zero, and a basis of zero — or a negative one, which is the normal state of a real cross-currency basis — is a level a party genuinely means. Either a contract kind declares that its levels may be SIGNED (a basis, a spread, anything quoted as a deviation), or a book quoted as a rate posts an offset from a stated reference so what reaches the grid is positive. State which, in the record, where the class that needs it is built
- [ ] **`13b-8`: the treasury shares out ONE pool and its lines ask for it in a different quantity.** Measured at 13b, and it predates this session: `headroom` is in units of the asset a line would add (already levered by the required ratio and divided by one asset's weight), `r.capital` is the line's risk-weighted assets, and its ask is `capital × appetite` — three quantities, and `give = min(left, asks)` treats them as one. Nothing throws, because every term is money in the bank's own money and they stay the same order of magnitude, which is why it has survived. It is Law 4: one pool with three statements of what it is. The pool should be CAPITAL, which is the only quantity every line's appetite is stated in and the only one that does not depend on which asset the line would add: a line asks in capital (`capital × appetite − r.capital × askedWeighted`), the treasury allots capital, and each line converts its allotment into assets AT ITS OWN WEIGHT — which each consumer already knows and neither can read off the other (`quote.ts` weights a loan, `dealing.ts` a trading position, and `capitalOf` computes both). That also deletes `inUnits` from the shared path, where it is one line's weight applied to every line's room. Tests: `bank-capital.test.ts` and `lines.test.ts`; a bank's lending room is the same number whether it is asked in capital or in loans
- [ ] **`13b-12`: a bank thirty-one billion insolvent was still quoting, and nothing had resolved it.** Measured at 13b, `rigWorld('seed-C')` period 23: `bank.a` published `capital −31,237,415,456` and went on making a market for twenty-three periods. What 13b fixed is only the arithmetic that stopped the run — a hole was being blended in as a source of funds with a required return, which made the cost of funds negative, the dealer's edge negative, and the desk's bid rise above its own offer (`[Clearing A2] ... on both sides ... at crossing prices`). What is NOT fixed is the trigger: XI-3 says nothing is immortal and Banks Capital C1.a says both triggers must exist and the resolution must say which one fired. Whether the solvency trigger is not reading the published position, or is reading it and the resolution is not being run, is unmeasured — and it is the question, not the quote. It is `bank-resolution.test.ts`'s seven reds. Tests: a bank whose published capital is negative is resolved, and the resolution says it was solvency
- [ ] **`13b-1`'s remainder: a bank closes overdrawn at the central bank with nothing lent to it.** Mostly closed at 13b — a treasury can now see what its own contracts will take out of its account next period (`OwnContracts.cashDue`) and the year-long run went green — but `deposits.test.ts` and `resolution/cells.test.ts` still report it in their own trimmed worlds. Whether that is the same shortfall reaching a world with fewer mechanisms to fund it in, or a second cause, is not measured. A bank short of reserves after a margin call BORROWS them, and that is this item's money-market response (Money Market C, Central Bank D1)
- [ ] Observer: books, syndicates and their fills, facilities, restructurings, borrow fees, margin per client (own view only), the breakeven and the grade boundary as reads; year-long run green; determinism; H2/H3 as reads; coverage re-marked; record entry
- [ ] Delete this file; worklist row 13f → done; commit and push


## Carried in from 13e (positioned when 13e closed)

- [ ] `smallFirm` cell kind with declared key dimensions and per-member state; the firms profile runs per member through `integrate`; a mean-preserving spread of member cash causes defaults among the members below the threshold (Small-Business Pools A2.a, A6, A6.a)
- [ ] Small-firm loans per (lender, cell) with member-level default and split; security on the owner's dwelling; promotion across the size boundary as a split into a named firm with its rows reseated (B1–B4, A6.b, A6.c, E4–E6)
- [ ] Financing a receivable: a lien on the invoice row, and factoring as a reseat with a money leg (Trade Credit C2)
- [ ] The covered bond (**M7**): a pool of LIENS on loan rows the bank still owns and still collects on; a substitution is a release and a pledge in one instruction, and a release that breaks the cover ratio is REFUSED AT THE CALL — the ratio is a term of the issue, not a bound on a number afterwards (Register D5.a, Banks Funding A2, Law 6); dual recourse read off the instrument in the one waterfall (XI-8)
- [ ] Senior tranches as eligible repo collateral (Money Market B3.a)

## Exit criteria

A deal larger than any one desk's limit is carried by a syndicate of named banks by shares each
posted from its own limit, or it fails visibly; a covenant is observed before a default; holders can
restructure; paper must be rolled and can fail to; a short has a borrow behind it; a prime broker's
call moves cash or closes positions and its line can go negative.

## Guard

Corporate Credit C10.b, C11.d, D5, D8, E5.d; Short-Term Debt E1–E3; Securities Lending C4, E1–E3; Prime
Brokerage C3.b, C5, E4; Banks Lending F3; Dealer Desks D1, D2 (a share sits against the member's
own limit); Bond N7.b (a linker's price is cleared and its real yield derived, never the reverse);
Law 2 (no imported inflation series); Indices A3 (the grade boundary is read off the assessors'
opinions, never off the index).
