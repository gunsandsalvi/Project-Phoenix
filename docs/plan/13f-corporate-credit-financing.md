# Item 13f — Corporate credit, short-term debt, securities lending, prime brokerage, syndication

**Objective.** The corporate bond as a full instrument: a capital-structure decision by management,
covenants that are observed before default and can be waived at a price, a primary market that is
brought by an underwriter, built from indications, priced at one level, allocated, walked away from,
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
D1–D8, D3.a, D3.b, D3.c, E1–E9, E4.a, E5.d, E6.a, E6.b, F1–F6, G4, G5, G5.a, G7, G8, H1, H2, H3
(H4 at 13b); Short-Term Debt A1, A1.a–A1.d, A2, A2.a, A3, B1–B5, B3.a, B3.b, C1–C4, C2.a, D1–D4,
E1–E3; Securities Lending A1–A5, A5.a, A5.b, B1–B4, B2.a, C1–C5, C2.a, D1–D3, D2.a, E1–E3; Prime
Brokerage A1–A4, B1–B5, B1.a, C1–C5, C1.a, C1.b, C3.a, C3.b, C4.a, D1–D4, E1–E4; Banks Lending D4,
D4.a, F3 (fully); Equity C5, C7; Fund Shares B2.

---

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
- **The primary market** (C): the issuer appoints an **underwriter** (C1): every desk quotes a fee
  and a commitment from its remaining underwriting limit (Dealer Desks D1, D2: `desk.limit.
underwriting` preference); the issuer takes the keenest; the fee is an instruction issuer →
  underwriter at pricing (C6); the underwriter **commits** to take what the book does not (C7); its
  unplaced share lands in its inventory (C7.a: an ordinary holding it must fund and rent, item 9);
  C7.b (fee vs risk) is measured. **Bookbuilding** (C2): the primary block of item 3.1 opened with the
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

### Parameters

`desk.limit.underwriting` (preference per desk); `firm.riskAversion` (preference, dispersed);
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

### Files

```
packages/engine/src/clearing/commitmentMarket.ts, clearing/market.ts (13f.1)
packages/engine/src/mechanisms/corporate-credit/{index.ts,bond.ts,structure.ts,covenants.ts,primary.ts,underwriter.ts,syndicate.ts,tap.ts,facility.ts,restructuring.ts,holders.ts}
packages/engine/src/mechanisms/bank-lending/syndicated.ts (D4.a: the loan syndicate on the commitment market)
packages/engine/src/mechanisms/short-term-debt/{index.ts,note.ts,roll.ts,buyers.ts}
packages/engine/src/mechanisms/securities-lending/{index.ts,loan.ts,collateral.ts,fee.ts,manufactured.ts,recall.ts}
packages/engine/src/mechanisms/prime-brokerage/{index.ts,relationship.ts,financing.ts,margin.ts,liquidation.ts}
packages/engine/src/mechanisms/sovereign-instruments/… (bill → discountNote with the state as issuer)
packages/engine/test/{commitment-market,corporate-bond,capital-structure,covenants,underwriter,syndicate,bookbuilding,tap,facility,syndicated-loan,leveraged-holder,call-refinance,restructuring,default-information,discount-note,roll-run,note-buyers,sec-lending,manufactured-payment,borrow-fee,recall-fail,short,prime-financing,portfolio-margin,liquidation}.test.ts
```

---

## Steps

- [ ] 13f.1 Kernel: commitment markets (one point of demand, commitment schedules, one clearing fee/margin, pro-rata at the margin, `filled < size` reported, ordered before the deal's primary); tests
- [ ] `bond` corporate kind: coupon forms, seniority honoured by the waterfall, call regime stamped at issuance by dispatch, covenants as terms, display name; tests (A1, A2.a, B1, B3, B4, N11–N13.a)
- [ ] The capital-structure decision: management's own target from the covenant line and its risk aversion at its own pace; issuance and refinancing as decisions; the structure as an outcome; tests (A2, A2.b, A2.c, A3, F5)
- [ ] Covenants tested on published accounts with the lag; breach and cure as events; waiver at a fee by the holders' vote; tests (B2, B2.a, B2.b, G1)
- [ ] Underwriter: desks quote fee and commitment from their own limit; appointment; fee by instruction; unplaced paper in the underwriter's inventory funded and rented; tests (C1, C6, C7, C7.a)
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
- [ ] Observer: books, syndicates and their fills, facilities, restructurings, borrow fees, margin per client (own view only); year-long run green; determinism; H2/H3 as reads; coverage re-marked; record entry
- [ ] Delete this file; worklist row 13f → done; commit and push

## Exit criteria

A deal larger than any one desk's limit is carried by a syndicate of named banks by shares each
posted from its own limit, or it fails visibly; a covenant is observed before a default; holders can
restructure; paper must be rolled and can fail to; a short has a borrow behind it; a prime broker's
call moves cash or closes positions and its line can go negative.

## Guard

Corporate Credit C10.b, D5, D8, E5.d; Short-Term Debt E1–E3; Securities Lending C4, E1–E3; Prime
Brokerage C3.b, C5, E4; Banks Lending F3; Dealer Desks D1, D2 (a share sits against the member's
own limit).
