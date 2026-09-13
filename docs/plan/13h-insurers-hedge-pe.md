# Item 13h — Insurers and pensions, hedge funds, private equity

**Objective.** Three institutions with three different relationships to loss. An insurer or pension
owes a scheduled liability to named beneficiaries whose present value is read from a market curve,
so a rate move is a solvency event for it and it is the structural holder of duration; it writes
cover at a price that answers its own experience and capital, draws its own claims, matches, hedges
with swaps that call margin, and can fail. A hedge fund's investors bear the result through
redeemable equity; its leverage is a loan from a prime broker; it is the speculative side every
derivative book needs, the buyer when others are forced, and the first link of the loss loop. A
private-equity fund calls committed capital on dates investors cannot refuse, buys firms with debt
raised against the target by lenders that decide, holds them at a mark that is honestly not a
price, and exits only when a market is open. This item also closes what earlier items declared
PARTIAL for these holders: the pension's swap demand, the household's pension claim, fund fee
competition, portfolio shifts across currencies.

**Read first.** §27 Insurers and Pensions (all); §28 Hedge Funds (all); §29 Private Equity (all);
IRS B2, B2.a; Households F3; Fund Shares B3 (the fee shape from item 8); Currency E2; §35 (the
control market from 13g is what a buyout bids through, which is why 13g precedes this item); XI-2, XI-3; §15 (the broker). Code: item 8's funds and gates,
13f's prime brokerage and securities lending, 13b's swaps, 13g's tender market, item 7's estate.

**Clauses this item meets.** Insurers A1, A2, A2.a, A2.b, A3, A4, A4.a, A4.b, A4.c, B1, B2, B2.a,
B2.b, B3, B4, C1, C2, C2.a, C3, C4, C5, D1–D5, D4.a, E1–E4; Hedge Funds A1–A5, B1, B1.a, B2, B3,
B4, B5, C1–C4, D1–D7, D4.a, D5.a, E1–E3; Private Equity A1, A2, A2.a, A2.b, A3, A4, A5, B1, B2,
B2.a, B2.b, B3, B4, B5, C1–C5, C5.a, D1–D5, D4.a, E1–E3; IRS B2, B2.a; Households F3, F3.a (fully);
Currency E2 (fully); Fund Shares B3 (the shape's death); CDS B5 and IRS B4 (the view on both sides,
fully).

---

## Findings this item carries

Both are one absence: **nothing in this world holds a claim because of what it expects of it.** A
portfolio decision — a holder that takes a POSITION, for its own reasons, at its own horizon — is
what this item brings, and until it exists a long-dated or foreign claim has no holder with a reason.

### The long end has no holder (item 12's finding **12-3**; `docs/RECORD.md`)

**Twice mis-filed before it was read correctly** — first as "a bank expects to lose a third to a half
of what it lends the sovereign", then as "the treasury cannot place its paper". Both are Law 11: a
misbehaving number is not a work item; the missing mechanism is. Neither number is wrong.

**What is true.** Households will not tie money up past their own horizon (52 periods,
`households.horizon.periods`, a PREFERENCE with a reason — Households D5). The treasury issues at
two, five and ten years. So the only bidders at a long auction are the primary dealers meeting their
obligation: cover comes in at exactly `dealershipShare` (0.34) and the issuer withdraws at its stated
patience of 40bp over the curve (`treasury.concession`). Everything downstream is the model working —
the treasury funds nothing at the long end and misses payments (XI-9 biting); the banks then require
0.0490 and 0.0352 of it against 0.0833 and 0.125 of each other (a lender that has watched an issuer
miss payment after payment asking more); 748 `overdraftRefused` at the central bank.

**Do not "fix" any of it.** An attempt was made and reverted: making the issuer read its own auction
history and stop bringing a tenor the market refused. It is a symptom patch (Law 12), and worse — it
would delete the one signal that says this item is missing. The deliberately-failing auction is an
incomplete-model check working as designed.

**What closes it here.** An insurer or pension owes a scheduled liability whose present value is read
from the curve, so it is the structural holder of duration: it wants the long end because of what it
owes, not because of what the last auction printed. When that holder exists the long auction covers,
or it does not and the reason is a price.

### The banks sell the seed's cross holdings in week one (item 12's finding **12-13**; `docs/RECORD.md`)

**Measured** with `foundationSpec('probe', drawBanks(3), drawFirms(24))` at period 1, before item 12
moved the cross holdings onto the central banks: every bank's holding of `bund`/`gilt`/`jgb` went out
in the first session — three `trade` instructions, 426,231,562,134 pieces of each foreign money paid
by the issuing central bank to `bank.a` — and the seed's cross holding was gone by period 2.

**Why it is here.** A bank's LIQUIDITY portfolio is correctly its own money only: a foreign bond
raises nothing at its own central bank's window (item 12 fixed that half, and then moved the cross
holdings to the central banks, for whom foreign paper is what reserves ARE — Central Bank F4). What
is still missing is the other reason to hold paper abroad, a POSITION taken because of what the
holder expects of the rate and the yield. `seed.crossHoldingShare` is a PLACEHOLDER whose named death
is this item (Law 2), and the portfolio decision is what kills it.

### A treasury bill prints at five times what it redeems for (`12d-2`)

The sovereign secondary book, seen with two rig seeds: `ust.bill.2026-06-15` carried at `1.0042` from
period 12 in one world and reaching **`4.949`** in another — a bill that pays one at maturity, priced
at nearly five. The engine found it by dying on it: `yield of ust.bill.2026-06-15 is Infinity`,
because the yield that discounts a payment of one to a price of five, days from redemption, is below
minus a hundred per cent and the discount factor underflows to nothing.

**It is 12c's finding in the bill book.** Both sides of a dealer's quote come from its own view, its
view follows the last print, and a book with nothing else in it is a fixed point — which is exactly
what 12c fixed for shares by giving a saver a reason off the issuer's own published accounts. The
sovereign book has no such party: the households' ladder prices paper off a public curve at their own
required yield, and the curve is read off the prints, so the anchor is the print again. 12d fixed the
ARITHMETIC only — a search that walks outward stops at the edge of what the function answers, a
present value says it diverges as the rate approaches minus one rather than throwing from inside the
walk, and a curve is built from the lines that HAVE a yield — so the world no longer dies on it and
the read is honest. **The price is not fixed**, and the second reason that fixes it is this item's
holder: a party that wants a dated claim because of what it owes at that date.

### The deposit guarantee can only pay for a loss no payment can cause (`12d-13`)

`bank-resolution.test.ts` "pays out of the fund the banks paid into, and the purse only after it"
(`Banks Capital D4`, `D5`) is red. The arithmetic says exactly when the insurer pays: `unmet = hole −
holders`, `holders` reaches at most the exposed pool, and the pool is every UNINSURED claim — so the
insurer pays **iff the bank's assets are worth less than its insured deposits**. That is the right
condition and the file's shock cannot reach it, because the loss it applies is a PAYMENT and a bank
can only pay what it holds in money: its loans and its paper are untouched, so assets never fall
below a few hundred million of insured deposits. Measured across penalties of 1, 8, 20, 40 and 80
times the bank's capital: at 8 the hole is 22bn against a 120bn pool and `insurerPaid` is **8,272** —
a rounding remainder, not the guarantee; past 20 the penalty cannot be paid at all, the bank fails on
CASH instead, and the hole goes negative. What the clause needs is a **valuation** loss — a mark
collapsing on assets the bank holds — and that is a market event rather than a fixture. This item is
where one becomes reachable: the failure loop below is a forced sale that moves prints, and the
prints reach every other holder's marks. Seen at
`packages/engine/src/mechanisms/money-market/resolution.ts:341`.

### A fund whose whole float is redeemed lives on as an empty vehicle (`12b.1-1`)

The exchange-traded fund's shares are redeemed in kind down to nothing at period 10, and then
`issued 0` with `noDemand` in every session from period 11 to 52. Nothing immortal (`XI-3`) and no
death without a destination (`XI-8`): a vehicle with no shares outstanding holds nothing and is owed
nothing by anybody, and it should wind up — or creation should be able to restart it, which is what
an authorised participant's other half is for (`Fund Shares E3`, `G1.a`). Instead it sits in the world
for ever with a market nobody can be on either side of. One half was fixed on the spot because it
STOPPED THE BUILD (`navOf` threw at a dealer asking `view.mark()` for a line with nothing
outstanding; `world.markOf` now answers `none`, which is what an optional read owes a caller). The
empty vehicle itself is this item's, beside the private-equity fund's own wind-up, which is the same
sentence: claims resolve, never freeze.

---

### The estate's reservation is a formula discount off book (from the review)

`estate/index.ts`:

```ts
const total = sub(closesAfter, view.period, 'left') + 1;
price: mul(print.value.price, div(left, total, 'how much of its patience is left'), 'reservation')
```

`print × left/(left+1)` — 1/2, 2/3, 3/4, 4/5 as the programme runs down. The inline comment says
"There is no discount curve here — the number is how long is left", and the module's own header says
"ASSETS ARE SOLD, not valued... **a formula discount off book is a stated price with no buyer**". It
is a discount curve, and it is derived from nothing the estate knows about the asset, the bidders, or
what it has to raise — only from the calendar and a parameter. It is also a written price path
(Appendix B), in the one module whose whole purpose is that a forced sale realises what a market
pays, and the schedule is PUBLIC (`estate.opened` journals `closesAfter`), so any bidder can compute
the period the estate capitulates and wait.

**The codebase already contains the right answer**, one module over: `funds/index.ts:861`, the other
forced seller, posts `price: 'market'` with "XI-2: at whatever the market gives. A forced seller that
named a price would not be one." Two modules, one problem — a seller under time pressure — and two
answers, one of which is a reason and one of which is a curve.

**It belongs here** because this is the item that carries the vehicle that must wind up
(`12b.1-1`) and where marks that are not prices are the subject: a liquidator's reservation, a
gate, and a wind-up are one question asked three ways. The fix is either the fund's answer (an estate
that must sell names no price) or a reason: what it expects to get by waiting — its own outlook of
that line, which XI-16 already gives every party — against the chance its programme ends first.

### An asset manager that takes a fee and decides nothing (from the review)

`FUND_MANAGER` is a registered party kind. It is seeded, it chooses a bank, it receives the fee, and
it makes no decision anywhere. So "a household gives its money to somebody to manage" has a name in
this world and no mechanism, which matters because delegation is how a household gets a professional
view without having one (13d gives the household its own naive reasons, and the two together are the
real distribution). Wealth is the other half and is a cell key dimension at 13d, itself blocked on
13b.1: a manager for rich households needs rich households to exist.


## Design

### Module `insurers`

- `requires: ['households', 'funds', 'irs', 'securities-lending', 'ratings', 'estate']`.
- **Party kinds** `insurer`, `pension` (A1: named, with accounts and registers); the sponsor of a
  pension is a firm (a term).
- **Liabilities** (A2, B1): instrument kinds `policy` (an insurer's liability to a beneficiary cell:
  terms `{ line, cover, term, premiumPerPeriod }`) and `pensionClaim` (a pension's liability to a
  cohort cell: terms `{ schedule: [{ period, amount }] }` accrued from contributions); both
  `liabilityOfIssuer: true`, `pricing: 'derived'` (item 8.1) with `value` = the schedule discounted at
  the **swap curve read** for each cash flow's tenor (B2; B2.b: no fixed rate, no cash balance: the
  liability never stores its value; a stored liability is impossible by construction because the
  profile has no store), so falling rates raise it (B2.a). The beneficiary does not absorb the
  investment result (A2.a); a unit-linked contract is item 8's `fund.share` and is declared so
  (A2.b). E1: every liability row names its beneficiary (the register's holder).
- **Cover** (A4): a **cover market** per line in `markets`: household cells (and firms for
  commercial lines) post demand for cover from their own decision (a line in the cell's basket:
  item 4's consumption decision gains insurance as a good whose price is the premium); insurers post
  supply at their **quote** = expected claims per unit of cover from their **own** experience (A4.c:
  a read of their own claims history per line) plus the return required on the capital held against
  the premium (A4.b: `insurer.requiredReturn` preference × the capital charge read from item 11's
  risk weights: `regulation.riskWeight.insurance.<line>`), sized by what its **surplus** can stand
  behind (A4.a: cover written ≤ surplus / the line's measured loss width: a read of its own history,
  not a ratio parameter); the solver gives the policy to the lower quote; an insurer with no surplus
  posts nothing and loses renewals (it loses book before licence); unplaced cover pays no premium.
- **Claims** (A4.c, B3, B4): per member, an event from the line's frequency and severity
  (technology primitives `claims.<line>.frequency`, `claims.<line>.severity`, declared as such: they
  are the world's physical hazard, not a credit rate: XI-1 concerns defaults, which stay events of
  state); a **catastrophe** is 13c's `environment` event reaching every policy of a line in a region
  at once (B4) — the SAME event that destroyed a producer's units and took a route's capacity away,
  read here rather than drawn again, because an insurer's loss and a producer's loss from one storm
  are one representation of one thing (Law 4); claims are instructions insurer → beneficiary.
- **Assets** (C): the portfolio decision (C1) matches the schedule's tenors (C2: the participant's
  bond and swap demand is built from the liability schedule's cash flows by tenor: C2.a: a one-way
  demand for long bonds and receiver swaps: IRS B2 → MET); illiquid assets: a limited partner in PE
  (C3: below); lends securities (C4: 13f's market); mandate limits credit by grade (C5: item 12's
  forced sale on downgrade applies).
- **The gap** (D): duration and cash-flow mismatch reads (D1); equity (A3: a read) moves opposite to
  a bank's (D2: measured); a **funding shortfall** (assets < liabilities at the read) triggers a named
  action in the party's `decide`: the sponsor contributes (an instruction sponsor → pension from the
  sponsor's cash plan: a decision the sponsor can refuse), the fund de-risks (sells), or benefits
  are cut (a journaled amendment of the schedules: a contractual term says whether allowed) (D3);
  hedging with receiver swaps (D4) creates margin calls through 13a (D4.a: the sector's failure mode:
  a rate rise improves solvency and demands cash); D5 measured at 16.
- **Failure** (A3, XI-3): negative equity → `resolution`: policies and claims are transferred to a
  successor insurer that bids for the book (a tender in the cover market) or into the estate, where
  beneficiaries rank as unsecured creditors (E1: nobody's claim vanishes).
- **Pensions and households** (Households F3): contributions from wages (an instruction cell →
  pension per period from item 4's income) accrue the claim's schedule; at retirement (13d's cohort
  crossing) the claim pays its schedule: the retired cell's income (F3 PARTIAL → MET).

### Module `hedge-funds`

- `requires: ['funds', 'prime-brokerage', 'securities-lending', 'derivative-layer', 'cds', 'irs',
'fx-derivatives', 'index-futures', 'commodity-futures']` (C1: it participates in every book).
- **Party kind** `hedgeFund`: item 8's fund with a **wide mandate** (A4: eligible: every kind; long,
  short, levered), investors' equity as redeemable shares (A2) with a **notice period and a gate**
  (D5.a: terms; item 8's gate mechanism), a manager party earning a management fee and a
  **performance fee** on gains above a high-water mark (A3: an instruction fund → manager when NAV
  exceeds the mark: a term), everything marked at cleared prices (A5, E2: an unmarked position
  is `Unpriced` and throws).
- **Leverage** (B): a margin loan from its prime broker (B1, B1.a, E1), derivatives (B2: the notional
  exceeds the margin), repo (B3: item 11) — the amount is the lender's decision (B4); gross, net and
  equity are three reads on the observer (B5).
- **What it does** (C): positions for reasons (C1): the `speculative` participant in every derivative
  and credit book — including 13b's **option** book, where a fund whose strategy is to be short
  dispersion writes cover out of the same balance-sheet budget every other position comes out of, and
  quotes from the move it expects the underlying to realise plus the return its capital requires on
  what the position consumes (this is the second side `§46 A3` has been missing: parties could
  disagree about a level and had no way to disagree about how far it would move) — (its outlook vs the print: this is the party that lets the market disagree with
  the model; XI-13's assembly check is satisfied by hedge funds on both sides of every book, so the
  desks' own view is no longer the only one); a buyer of forced sales when it has capacity (C2: its
  participant reads the session's `forced.sale` orders as an opportunity and bids at its own
  valuation within its remaining line); shorts with borrows (C3: 13f); every trade is a cleared
  trade (C4).
- **The failure loop** (D): loss → equity down, leverage up (D1: reads) → the broker's call (D2:
  13f's margin) → sales that move prints (D3) → other holders' marks and their calls (D4: emergent,
  D4.a) → redemptions (D5: item 8) → failure (D6: NAV ≤ 0 or an unmet call: the broker eats the
  shortfall against its capital, the investors lose their equity; E3: it can fail); D7 traceable.

### Module `private-equity`

- `requires: ['funds', 'corporate-control', 'corporate-credit', 'bank-lending', 'equity', 'insurers']`.
- **Party kinds** `peFund` (A1: committed capital from named investors: insurers, pensions, wealthy
  cells; a `commitment` row per investor with an amount and the fund's life: A4), `holdco` (A5: a
  vehicle per deal, a party with its own balance sheet).
- **Calls** (A2): the fund's `decide` posts a **capital call** when a deal needs equity: an
  instruction investor → fund on the date, which the investor's cash plan must meet from its
  liquidity ladder (selling if it must: A2.a); an unmet call **fails** the instruction (item 5's
  Failed state: A2.b: a default on the call, journaled, with the contractual consequence: the
  defaulting investor's commitment is forfeited pro rata (a term)); nothing bounds a call to spare
  cash.
- **The buyout** (B): the fund bids for a firm through 13g's tender market (B1); the price is
  funded by **debt raised against the target** (B2: a loan or bond issued by the holdco/target
  through 13f's markets, secured on the target's assets; B2.a: the target's liability; B2.b: lenders
  decide, so the deal happens only if the debt clears: the tender bid is conditional on the financing
  clearing in the same session: the commitment market for the acquisition loan runs before the
  tender) and the **equity cheque** from calls (B3); at completion the target's register shows the
  holdco as owner and the target's balance sheet carries the new debt (B4); B5: sources = uses
  exactly: an audit contribution per deal from the ledger.
- **The hold** (C): the target operates and services its debt (C1); the owner influences investment,
  costs and distributions (C2: the holdco's participant sets the firm's management preferences: the
  firm module's decision reads its owner's preferences through its view when the owner holds
  control: 13g's control read); recapitalisation (C3: new debt at the target paying a distribution
  to the holdco: a decision the lenders can refuse); failure (C4: item 7: the lenders lose, the
  equity is wiped, the fund's other holdings are unaffected: B2.a); the **mark** (C5): the holding's
  value in the fund's NAV is a mark from the fund's own model (its outlook of the target's earnings
  at its own hurdle), carried with provenance `marked` (C5.a: the prices family shows "marked, not
  cleared"; it never enters an index or a collateral haircut as a cleared price: a kind with
  `pricing: 'derived'` whose read reports `marked`).
- **The exit** (D): a sale to a corporate or another fund through 13g's tender market, or an IPO
  through item 9's issuance (D1); the exit price is the first cleared price (D2); proceeds
  distributed to investors in cash by instructions (D3); a closed market (a failed primary or no bid
  above the fund's reservation) extends the hold (D4) and investors owe calls while receiving no
  distributions (D4.a: emergent); D5: returns are a read of distributions against calls.
- **Wind-up** (A4): at the end of the life the fund sells what it holds into whatever market exists
  and distributes; an unsold holding at wind-up is distributed in kind (the shares themselves) to
  investors pro rata: claims resolve, never freeze.

### Fee competition and portfolio shifts

- Item 8's `fund.managementFee` shape dies: each manager quotes a fee (its decision from its own
  costs and the flows it observed); each investor's subscription decision (Households D5) compares
  funds on net expected return from its own outlook; the fee is a term struck at subscription.
- Currency E2 (portfolio shifts across currencies) is complete once insurers, hedge funds and PE
  hold foreign assets from their own decisions (12's revaluation applies to them unchanged).

### Parameters

`insurer.requiredReturn` (preference per insurer); `regulation.riskWeight.insurance.<line>`
(policy, parliament); `claims.<line>.frequency`, `claims.<line>.severity` (technology); `hedgeFund.
noticePeriods`, `hedgeFund.gate` (terms per fund: data); `peFund.life` (a term); `pe.callForfeit`
(a term). Deleted: `fund.managementFee` (item 8's shape). No `catastrophe.probability` as a
mechanism input (a scenario event), no `contagion.*`, no `hurdle` other than the party's own.

### Audit contributions

- `accounts`: an insurer's equity is assets at marks minus liabilities at the curve read, both from
  the register (E3: no stored liability exists); Insurers E4: sector holdings enter the ownership
  family unchanged.
- `flows`: PE B5 sources = uses per deal; calls and distributions two-sided.
- `prices`: a `marked` provenance is reported distinctly and never counted as cleared.
- `names`: every liability has a beneficiary (E1); every holdco has an owner and a target.

### Files

```
packages/engine/src/mechanisms/insurers/{index.ts,liability.ts,cover.ts,claims.ts,matching.ts,gap.ts,pension.ts,resolution.ts}
packages/engine/src/mechanisms/hedge-funds/{index.ts,mandate.ts,leverage.ts,participants.ts,fees.ts}
packages/engine/src/mechanisms/private-equity/{index.ts,commitments.ts,buyout.ts,hold.ts,exit.ts,mark.ts}
packages/engine/src/mechanisms/funds/{fees.ts,windup.ts} (competition; a vehicle with nothing outstanding), households/portfolio.ts (fund choice, pension contributions)
packages/engine/test/{insurer-liability,cover-market,claims,catastrophe,matching,gap-hedge,insurer-failure,pension-claim,hedge-fund,hf-leverage,hf-view,hf-loop,pe-calls,buyout,pe-hold,pe-exit,fund-fees}.test.ts
```

---

## Steps

- [ ] The estate names no price it cannot defend: either the fund's answer (a forced seller posts size and no level) or its own outlook of that line against the chance its programme ends first; the `left/(left+1)` curve deleted; tests: what an estate realises is what a market paid, and a bidder cannot compute the period it capitulates (XI-2, XI-8, Appendix B)
- [ ] `FUND_MANAGER` decides: a household with wealth above its own threshold delegates, the manager runs a mandate on its behalf and is paid for it, and what the household holds is then the manager's decisions and not its own; depends on 13d's wealth dimension; tests (Fund Shares A4, Households D5)
- [ ] **From item 11 (XI-2 door three, Prime Brokerage C3.b)**: a broker cuts a leveraged client's line below what it has drawn, and the client's own module posts the sales that repay it, at whatever the book gives. Item 11 built this door for a BANK (a bank refused by the session sells its own paper) and published the line a bank will fund for one name (Banks Lending F3), but it has no client to cut: this world's only leveraged holder of marketable assets is a desk, and a desk is its own bank's arm. A hedge fund with a prime broker is the party. Test: `limit − exposure` is negative with no floor in the path, the sale moves the print, and the print reaches other holders
- [ ] `insurer` and `pension` kinds; `policy` and `pensionClaim` liabilities to named beneficiaries valued at the swap curve read each time; no stored or fixed-rate value can exist; tests (A1–A3, B1, B2, B2.a, B2.b, E1, E3)
- [ ] The cover market: quotes from own experience and capital, sized by surplus; policy to the lower quote; unplaced cover; premiums and claims as instructions; an insurer with no surplus writes nothing; tests (A4, A4.a–A4.c)
- [ ] Claims as per-member events from declared technology primitives; a catastrophe as one event on many policies; tests (B3, B4)
- [ ] Matching: bond and swap demand built from the liability schedule by tenor (IRS B2 → MET); LP in PE; securities lending; downgrade-forced sales; tests (C1–C5)
- [ ] **The long end has a holder** (the findings above): the two-, five- and ten-year auctions are bid by a party that wants the tenor because of what it owes at it, not by dealers meeting an obligation — and the bill and bond books gain the second reason their prints have never had (`12d-2`). Tests: cover at a long auction exceeds `dealershipShare`, or it does not and the issuer's withdrawal is priced rather than structural; a dated claim's print is not a fixed point of the desks' own view
- [ ] **A position is held for a reason** (the finding above): a holder decides what foreign and long-dated paper to carry from its own outlook on the rate and the yield, so `seed.crossHoldingShare` dies as a PLACEHOLDER (Law 2) and a cross holding survives period 1 when its holder still wants it
- [ ] The gap: duration and cash-flow reads; equity moving opposite to a bank's; shortfall actions by the sponsor, the fund or the schedule; swap hedges that call margin; tests (D1–D4, D4.a)
- [ ] Failure: negative equity resolves to a successor's tender or the estate with beneficiaries ranking; tests (A3, XI-3)
- [ ] Pension contributions from wages accrue the schedule; drawdown at retirement is the retired cell's income (Households F3 → MET); tests
- [ ] `hedgeFund` kind: wide mandate, redeemable equity with notice and gate, manager with performance fee above a high-water mark, every position marked or `Unpriced`; tests (A1–A5, D5.a, E2)
- [ ] Leverage from the prime broker, derivatives and repo at the lender's decision; gross, net and equity as three reads; tests (B1–B5, E1)
- [ ] Positions for reasons: the speculative participant on both sides of every derivative and credit book (XI-13 check now satisfied by hedge funds); buyer of forced sales within its line; shorts with borrows; tests (C1–C4, CDS B5, IRS B4)
- [ ] The loop: loss, call, sale, price, other funds' calls, redemptions, failure landing on the broker and the investors; scenario test traceable party by party; tests (D1–D7, E3)
- [ ] `peFund` and `holdco` kinds; commitments; calls as instructions on dates that fail when unmet with the contractual forfeit; manager fees; life and wind-up resolving to cash or in-kind; tests (A1–A5, E2)
- [ ] The buyout: a conditional tender funded by target debt that lenders clear and an equity cheque from calls; the target's balance sheet transformed; sources = uses contribution; tests (B1–B5, E1)
- [ ] The hold and the exit: owner influence through preferences, recapitalisation lenders can refuse, failure wiping the equity only, the `marked` provenance shown as such, exit by tender or IPO as the first cleared price, a closed market extending the hold, distributions in cash, returns as a read; tests (C1–C5, D1–D5, E3)
- [ ] The short-dispersion side of the option book: a fund writes 13b's options from the same balance-sheet budget as every other position, so the premium has two sides with reasons and parties can disagree about dispersion as well as level; tests (§46 A3, XI-13, Derivative D7.a)
- [ ] A valuation loss a bank can actually take (`12d-13`): the loop's forced sales move prints, the prints reach the marks of every other holder of that paper, and a bank's assets can fall below its insured deposits — so the guarantee's condition is reachable and `bank-resolution.test.ts`'s D4/D5 test has a world to run in; tests
- [ ] A vehicle with nothing outstanding winds up or can be restarted (`12b.1-1`): no empty immortal claim with a market nobody can be on either side of; the same sentence as the private-equity wind-up; tests (XI-3, XI-8, Fund Shares E3, G1.a)
- [ ] Fund fee competition: managers quote, investors compare on their own outlook; item 8's fee shape deleted; Currency E2 complete; observer: liabilities and gaps, cover books, hedge-fund reads, PE deals with marks flagged; year-long run green with a rate-move scenario; determinism
- [ ] **`13b-10`: EVERY CONTRACT BOOK IN THIS WORLD HAS ONE SIDE, and this item is the other one.** Measured at 13b: forty books open in the classes rig and not one contract is written in sixteen periods. It is not the levels — three banks name three different prices for the same bond future, so their outlooks do disagree — it is that every party the layer admits is a bank or a firm, and in this world both want the same thing (banks long duration and short of fixed, firms short of foreign money). Asking every party rather than the eligible ones, the bond-future books have forty-two to fifty-two willing schedules. So: the insurer and the pension are admitted to contract books (`derivativeLayer([...])` takes their kinds), and IRS B2/B2.a's structural one-way demand — a pension whose liabilities are long and whose assets are not — is what puts a receiver of fixed opposite every payer. Tests: a swap book, a bond-future book and an option book each clear with a named party on each side, and the two sides are two different KINDS of party with two different books (§46 A3, IRS B2, B2.a, Derivative Layer B1). **Letting the households in is the wrong fix and is refused in the test**: a household does not sell a bond future, it owns a fund that does (Law 1)
- [ ] **`13b-2`, `13b-9`, `13b-11`: what a fund holds, and what a fund IS.** Funds re-enter the layer's contract books with the read that keeps Fund Shares A3 true — a fund's equity is zero by construction, and the pass that re-marks its claim on itself runs in the same pass as the mark that moved its assets (measured at 83,247,864 of equity on `etf.us` the one time they were let in; and 1.03e-6 on a book of 8e3 in `resolution/cells.test.ts`). With it: a fund whose shares are marked at NOTHING while shares are outstanding is explained or cannot happen — 13b found one and could only stop it publishing `0/0` as what it returned. And the tracker's provenance: a vehicle exists by publishing `etf.launched`, whichever path made it, so "which trackers are on this rule" has one answer (Law 4) — preferably by the seed no longer seeding a vehicle at all, because a seeded fund is a seeded outcome (Law 2)
- [ ] **`13b.1-5`: a resolution bid charges a YEAR of required return for one week.** The bid a surviving bank makes for a failed bank's book (Money-market D3) multiplies `bank.returnOnCapital` — a per-annum rate, and the other four readers of it carry the year through — straight onto the assets, with no fraction of a year anywhere. The bid is roughly fifty times too low, so a resolution that should place a book declines it (D3.b) and the bank that would have taken it looks like one whose equity could not stand it. The read now names `perAnnum`, which is what made it visible; the missing multiplication is here, with the rest of what a resolution reaches its own price by
- [ ] **`13b.1-8`: the clearing house is not flat, by nine hundredths.** `derivative-layer.test.ts` measures the house's own position at -0.096 where C2 says it is nothing BY CONSTRUCTION — buyer to the seller and seller to the buyer, at one level in one instruction, so not to within a tolerance but exactly. Two candidates: the margin legs, posted per side against different counterparties, and a contract cut by `admits` after one of its two legs was already priced. Being flat is the property that makes a central counterparty one, so this is a mechanism finding and not a measurement
- [ ] **`13b.1-9`, and it is `13b-10` one class over and stronger**: not one living party posts a schedule into any option book, either side. The books are open and the class has its own `orders`; what comes back is empty for everybody. Covered by the item above — the counterparty that takes the other side of an option is the insurer and the pension — and named separately so that the test which measured it (`derivative-classes.test.ts`, "opens an option book") is one of the ones that has to go green
- [ ] **`13b.1-1`: A FUND SHARE WORTH NEARLY NOTHING MAKES EVERY QUANTITY DIVIDED BY IT EXPLODE, and at 13b.1's close it is the largest single cause of red in the suite.** It is `13b-9`'s open half — why a share is worth less than a piece of money — and the guards 13b.1 put on the rounding doors turned it from a wrong number into a refusal at the site, in three places: a redemption whose cash rounds to nothing (a one-sided money leg, now refused at the wire and the request left on the book, which is C2.b's own answer); a household cell asking for `scaleQty(sharesPerMember, weight)` = **1.33e16 shares**, past the safe-integer range (`households/index.ts:374`); and a dealer's creation-unit arithmetic, `money / nav` = **1.72e16** (`banks/dealing.ts:415`), which takes `omo`, `raise` and both `equity-anchor` tests with it. Law 6: a number that explodes means the compensating mechanism is missing, and here the missing one is whatever holds a fund's NAV per share off zero — which is this item's, because what a fund IS and what stands behind its shares is. Tests: the three sites above, and a year-long run in which no share count exceeds what its holder could hold
- [ ] Coverage re-marked, including IRS B2 and B2.a (PARTIAL from 13b: the reason is stated and no party in this world has it) and `Fund Shares A3`; record entry with both shapes' deaths (`funds.managementFee` and `seed.crossHoldingShare`); delete this file; worklist row 13h → done; commit and push


## Carried in from 13e (positioned when 13e closed)

- [ ] Mortgage pools through the same vehicle, with the foreclosure path running with the pool as lender of record and the servicing agreement as a fee row (Housing C6, XI-11)
- [ ] Insurers as note holders: the party kind whose liabilities make it the natural buyer of a senior layer, so a tranche has holders that are not other banks (XI-11's "named holders")


- [ ] Prime brokerage (from 13f): the relationship and its lien, the margin loan above cost of funds, portfolio margin from the broker's own measured move, calls in `margin.calls`, the line NEVER floored, liquidation with the shortfall on the broker's capital, and the multi-broker blind spot (§15, Equity C5)
- [ ] Restructuring and the holders' vote (from 13f): the issuer's proposal, each holder deciding at its own expected recovery, vote by face, contractual majority binds, rejection opens the estate (Corporate Credit G7)

## Exit criteria

A rate move revalues a pension's liability, its hedge and its cash in three different places; an
insurer can lose its book before its licence and can fail; a hedge fund's loss reaches another fund
through a broker and a price; a buyout happens only when lenders lend and its sources equal its
uses; a PE mark is never mistaken for a price.

## Guard

Insurers B2.b, E1–E3; Hedge Funds E1–E3; Private Equity A2.b, C5.a, E1–E3; XI-13 (a view on both
sides of every book).
