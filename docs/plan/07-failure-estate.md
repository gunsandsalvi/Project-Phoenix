# Item 7 — The forced seller, nothing is immortal, the estate

**Objective.** Every kind of party can cease, each with its own trigger and consequence; a chain of
losses has a push (a forced sale), a place to terminate (a failure), and something at the end that
distributes what is left (an estate with a waterfall). Losses land on named holders in proportion.
Money is not destroyed by a death; it is transferred and revalued.

**Read first.** XI-2, XI-3, XI-8 in full; §34 Firm Birth B, D, E; §25 Banks Capital C, D, E; §24
Banks Funding D6; §15 Prime Brokerage C3.b (the floor-at-zero FORBID, guarded here in advance); §13
Fund Shares C2.b (the redemption door: item 8); Register F2, E3; Money E4; Sovereign G3, G5.
Code: `parties/party.ts` (`cease`, `resolve`), `world/context.ts` (`cease`), `register/register.ts`
(liens), item 5's provisions.

**Clauses this item meets.** Firm Birth B1, B2, B3, B4, D1, D2, D2.a, D2.b, D2.c, D3, D4, D4.a, D5,
D6, D6.a, E1, E2; Banks Capital A1, A1.a, A2, A2.a, A2.b, A2.c, A4, C1, C1.a, C2 (the attempt; the
raise itself at 11), C2.b, C3, C3.a, C3.b, D1, D1.a, D2, D2.a, D3, D3.a, D3.b, D4, D5, D6, E1, E2,
E3; Banks Funding D6, D6.a; Money E4; Register F2, E3; Corporate Credit G4, G5, G5.a; XI-2 (the
doors that exist: funding line withdrawn (6), the others as each arrives), XI-3, XI-8. Remain
PARTIAL: XI-2 doors 1, 2, 4 (13a/13f, 8, 12), Banks Capital B (11).

---

## Design

### Module `estate` (sub-items 7.1–7.3)

- `requires: ['credit-events']`; owns party kind `estate` (named) and no instrument kinds.
- **Opening an estate** (D1, XI-8): a door `ctx.cease(party, successor)` exists; this item adds the
  estate module's `open(dead: PartyId): PartyId`: creates a party of kind `estate` banking where the
  dead party banked, moves every holding and every account of the dead party to it by **transfer
  instructions at carrying value** (Register F2: to a named successor; one instruction per holding
  so the ledger shows the move), re-seats every instrument the dead party **issued** on the estate
  as issuer (an instrument event journaled `instrument.reseated`; the kernel gains `instruments.
reseat(id, newIssuer)` as an estate-only door with the estate module as its one caller: sub-item
  7.1), calls `ctx.cease(dead, estateId)`, journals `estate.opened` (public).
- **The programme** (D1, XI-8): estate state `{ dead, opened, claims: Claim[], programme: { periods,
remaining } }`. Each period, phase `estates.sell` (cycle 2, after `firms.invoice`): the estate
  posts sell orders for everything it holds into the markets those instruments trade in (goods into
  the goods market, paper into its market), at a reservation that falls over the programme's length
  (the estate's patience is the policy `estate.programme.periods`, owner standardSetter: an
  insolvency regime is policy); plant to bidders who can use it (item 10: kinds of capital); what
  nobody buys by the last period is **abandoned** (a `destroy` leg for destructible kinds; for paper,
  it stays held by the estate and the estate closes with it at zero: an outcome, journaled).
- **The claims register**: at opening, every instrument the dead party issued and every payable it
  owed (invoices, item 4; trade creditors rank: D2.b) is a claim with a rank read from the
  instrument profile's `ranking` (item 5) and, for invoices, unsecured rank (D2.b); a counterparty's
  close-out claim (13a) ranks unsecured (D2.c). Equity (9) is last (Equity E4).
- **The waterfall** (D2, XI-8): phase `estates.distribute` (same cycle, after `estates.sell`): cash
  realised this period is distributed in rank order, pro rata within a rank, by money instructions to
  the holders of each claim; a secured claim is paid from its security's proceeds first; when a claim
  is paid in part, its instrument is redeemed at the paid fraction of its face (a redemption leg to
  the estate as issuer at a price equal to the fraction) and the remainder stays outstanding until
  the estate closes, when it is written off (item 5's write-off). D6: recoveries plus losses equal
  the assets at realisation: an audit contribution, per estate, from the ledger.
- **Closing** (D5, D6.a): when the programme ends and every asset is sold or abandoned, the estate
  writes off every remaining claim, pays away every currency it holds (a residual on a dead party is
  a defect: the last distribution takes all cash, dust included, to the last-ranked claim), journals
  `estate.closed` (public) and ceases with successor `sink`? No: a ceased party must resolve to a
  successor that exists; the estate's successor is **the claims' holders**, which is many. Design: an
  estate that has distributed everything ceases with successor = itself (allowed for an estate kind
  only: the kernel's `cease` gains `successor: PartyId | 'terminal'` for parties whose kind profile
  declares `terminal: true`; the names family accepts a terminal cessation only if the party holds
  nothing in any currency: D6.a).

### Triggers (sub-item 7.4: each party kind's module owns its own trigger)

| kind                                   | fails when (XI-3)                                                                                                                                                                                                                                                               | who decides                                                                                                                  | then                                                                                                                                                                                                 |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| firm                                   | cannot pay (a failed instruction it cannot cure this period), or liabilities exceed assets at marks (Firm D4)                                                                                                                                                                   | `firms` phase `firms.solvency` (cycle 3, `resolution` slot) from the firm's own view: `view.failedPayments`, `view.equity()` | `estate.open(firm)`; employees released through the labour module's separation path (D4.a: the estate journals `estate.opened`, the labour module reads it and separates every row of the dead firm) |
| bank                                   | cannot fund itself (Banks Funding D6: its reserve account below zero after the market and the facility: the money market at 11 makes this reachable; until then: after the corridor placeholder, a negative reserve at the close) **or** its capital is gone (Banks Capital C1) | `banks-capital` phase `banks.resolution` (cycle 3), split at 11: here the solvency trigger and the resolution mechanics      | resolution (below)                                                                                                                                                                                   |
| household cell                         | a cell whose members cannot pay is split and the crossing members default on their claims (13d with consumer credit; here: the split and the failed payment as a state)                                                                                                         | households                                                                                                                   | no estate: a household's default resolves through its lender's enforcement (13d)                                                                                                                     |
| treasury                               | never in its own money (Sovereign G1: the failure mode is inflation); in a foreign money at 12                                                                                                                                                                                  | —                                                                                                                            | —                                                                                                                                                                                                    |
| central bank                           | never (XI-3's one exception)                                                                                                                                                                                                                                                    | —                                                                                                                            | a loss reduces equity and the deferred asset is a row (item 3)                                                                                                                                       |
| fund, insurer, sponsor, clearing house | items 8, 13h, 13a                                                                                                                                                                                                                                                               | those modules                                                                                                                | those modules' triggers, all through this item's estate                                                                                                                                              |

### Bank resolution (sub-item 7.5, `banks-capital` first half)

- Valuation (D1): the book at marks and, for loans, at the bank's own carrying values written down
  by item 5; the hole is liabilities minus that.
- Hierarchy (D2): equity wiped (shares at 9; until then the equity account to zero), subordinated
  debt (a `loan`/bond row with a subordinated rank: exists from 6/13f) bailed in by a redemption at
  the fraction the valuation supports, then senior and depositors only in liquidation (A2.c).
- Acquirer (D3): every other bank is asked for a **bid** for the book (assets and liabilities
  together) from its own view (its capital room and its valuation of the book with its own PD model);
  the highest bid above the public path's cost wins; the bidder pays or is paid the difference by an
  instruction; deposits and rows re-seat on the acquirer (D6: `instruments.reseat` for the bank's
  money instrument: every depositor's holding of `money:<bank>` becomes a holding of `money:<acquirer>`
  by one journaled re-kinding; reserves move with them). No bid: the public path (D4, D5): deposit
  insurance pays per member up to the limit (`regulation.depositInsurance.limit`, policy; Banks
  Funding A1.a), the insurer (the treasury until an insurer party exists at 13h) becomes a creditor
  of the estate, the rest goes through the estate.
- E3: the resolution conserves: acquirer paid + insurer paid + estate realised + holders lost = the
  hole; audit contribution per resolution.

### Forced sales (sub-item 7.6)

The doors that exist now: a **funding line withdrawn** (item 6: a bank cutting a facility limit
forces the borrower's phase to post sells to repay within the term; the borrower's module posts
them, reason journaled `forced.sale` public? private to the seller, with the count public). The
other three doors are wired by their items with the same event kind. The FORBID guard Prime
Brokerage C3.b (no floor at zero on the available line) is a kernel invariant: the facility row's
`available` is a read `limit − drawn` that may be negative, and the lint's no-bounds rule already
refuses a floor.

### Parameters

`estate.programme.periods` (policy, standardSetter); `regulation.depositInsurance.limit` (policy,
parliament); `resolution.trigger.capitalRatio` (policy, standardSetter: the ratio at which the
supervisor applies C3.a).

### Files

```
packages/engine/src/register/instruments.ts, world/context.ts (7.1: reseat; terminal cessation)
packages/engine/src/mechanisms/estate/{index.ts,open.ts,programme.ts,waterfall.ts,close.ts}
packages/engine/src/mechanisms/firms/solvency.ts
packages/engine/src/mechanisms/banks-capital/{index.ts,resolution.ts,valuation.ts,bids.ts}
packages/engine/src/mechanisms/labour/… (separations on estate.opened)
packages/engine/test/{estate,waterfall,resolution,forced-sale,triggers}.test.ts
```

---

## Steps

- [ ] 7.1 Kernel: `instruments.reseat` as an estate-only door; terminal cessation for kinds that declare it; names family accepts terminal only with nothing held in any currency; tests
- [ ] 7.1 Kernel: `cease` journals `party.ceased` with the successor; `parties.resolve` follows chains; test with a two-hop chain
- [ ] 7.2 `estate` module: opening moves every holding by transfer instructions at carrying value and re-seats issued instruments; test: the ledger shows every move; the audit is green the period a firm dies
- [ ] 7.2 Claims register with ranks from profiles; invoices and trade creditors rank unsecured (D2.b); close-out claims unsecured (D2.c); tests
- [ ] 7.2 The programme: sells into the existing markets with a falling reservation over the policy length; abandonment as a destroy leg or a zero close; tests
- [ ] 7.3 The waterfall: secured from its security first, then rank order pro rata; partial redemptions; tests: a junior claim can recover nothing (G5.a)
- [ ] 7.3 Closing: write-offs of the remainder, every currency paid away, terminal cessation; audit D6 and D6.a contributions; tests
- [ ] 7.4 Firm trigger from the firm's own view (cash failure it cannot cure; equity below zero at marks); test: a firm that cannot pay wages dies, its rows separate through the labour module
- [ ] 7.4 Labour reads `estate.opened` and separates every row of the dead firm through its own path (D4.a); test: headcount falls only by separations
- [ ] 7.4 Household cell crossings split and record the failed payment; no estate for a cell; test
- [ ] 7.5 Bank valuation at marks and carrying values; the hole; test
- [ ] 7.5 Hierarchy: equity to zero, subordinated rows bailed in by partial redemption, senior and depositors untouched outside liquidation; tests
- [ ] 7.5 Acquirer bids from each bank's own view; the winner pays or is paid the difference; deposits and rows re-seat; test: a resolution with no bid falls to the public path
- [ ] 7.5 Public path: deposit insurance per member up to the limit, the insurer as an estate creditor; test with a cell of small and a cell of large depositors (Banks Capital D4)
- [ ] 7.5 E3 conservation audit contribution; test
- [ ] 7.6 Funding line withdrawn forces sales posted by the borrower's own module with the reason journaled; test: the sale moves the print and the print reaches other holders (XI-2's consequence)
- [ ] 7.6 The `available = limit − drawn` read may go negative; a test asserts no floor exists (grep-free: the lint rule plus a behavioural test where a drawn line exceeds a cut limit)
- [ ] Observer: estates with their programmes and waterfalls (a story that develops: Observer B5), resolutions, forced sales counts
- [ ] Year-long run with a scenario seed in which one firm and one bank die; families green; determinism
- [ ] Coverage re-marked; PARTIAL rows for the remaining XI-2 doors and Banks Capital B named
- [ ] Record entry
- [ ] Delete this file; worklist row 7 → done; commit and push

## Exit criteria

A firm and a bank can die in a run; every reference resolves; recoveries plus losses equal assets at
realisation; the resolution conserves; no residual on any dead party in any currency; a year green.

## Guard

Firm Birth E1, E2, E3 (population not constant by construction: births at 13g); Banks Capital A1.a
(capital is never a pot), D2.a (no creditor worse off than in liquidation: a VERIFY, measured at 16);
Prime Brokerage C3.b; XI-3 (the central bank's immortality is a consequence, not an oversight);
Register A3 (no leg to nobody: abandonment is a destroy leg or a zero close).
