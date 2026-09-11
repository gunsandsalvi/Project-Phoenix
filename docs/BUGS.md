# Open findings — the holding pen

**What this file is.** EVERY bug goes here. A bug found while working an item does not derail the
item and does not get chased on the spot: it is written down, with what was measured and where it
was seen, and the item carries on (Law 10: one ordered list, one item at a time; Law 11: a
misbehaving number is not a work item, the missing mechanism is).

**The one thing fixed on the spot** is a violation that STOPS THE BUILD: an impossible quantity, a
one-sided flow, a fact with two writers. The engine refuses to run past those, so they are fixed
where they are — and written down here as well, marked RESOLVED.

**What happens to it.** When the item closes, every finding here is POSITIONED: moved into the
`docs/plan/<item>.md` of the item that should fix it, or inserted as an item of its own at its
dependency position, with the record saying where it landed and why. A finding is deleted from this
file only by being placed — never by being decided against without a line in `docs/RECORD.md`.

**This file is temporary.** When it is empty it goes.

---

## Found while working item 12a

### 12a-1 — The equity index reads a level its own prints do not make

**Where.** `audit/families/cross-market.ts` (the `crossMarket` family), against the `indices`
module's `equity.us`. Seen with `rigWorld('reporting', 4, 40)`.

**Measured.** Green for twenty-four periods, then from period 25 on, every period:

```
crossMarket: equity.us reads 7860.8928192012245 where its own prints make 7864.660507830988
p26 onwards: 7837.44881897073 against 7841.20527100242
```

A gap of about five parts in ten thousand, appearing at one period and then carried unchanged —
which is the same SHAPE as 12-18 (a step, not a drift) and rules out accumulating dust.

**Not this item's, and not caused by it.** Item 12a adds an append-only ledger of equity moves, one
extracted balance-sheet read, and a module that writes journal events. None of them touches a price,
a print or an index; the world runs identically with the reporting phase removed. The likeliest
cause is 11.5: the equity float moved from the desks to the savers, and a free-float weighting is a
read of who holds what — so an index that weighs by float now weighs by a different set of holders,
and something in the level and something in the constituents disagree about which set.

**Where it belongs.** With the equity book, which is worklist **12c**: the index is a read of the
same book that item is about, and a level that disagrees with its own constituents is the same class
of defect as a print that disagrees with what was paid for it. To be positioned when 12a closes.

### 12a-2 — Every issuer misses payments in every window, so every grade is the worst one

**Where.** `mechanisms/ratings/assess.ts`, measured directly through the assessor's own blind view
in `rigWorld('ratings', 4, 40)` at period 30, with a seven-period window:

```
treasury.us   missed 1129   takesIn  2.995e+10   owedIn -5.734e+10
bank.a        missed    3   takesIn -4.079e+10   owedIn  0
firm.8        missed   24   takesIn -9.894e+07   owedIn -2.938e+09
firm.18       missed   24   takesIn -1.403e+08   owedIn -5.887e+09
```

**What item 12a fixed, and what it did not.** 12-17 said every sovereign grades `c` because the
measure was what falls due against what the issuer is WORTH, and a state's book equity is deeply
negative by construction. That is fixed: the measure is a coverage ratio against what the issuer
TAKES IN, read off the equity ledger with the marks excluded, and `treasury.us` takes in +3.0e10. A
second defect was fixed on the way — `failedPayments` took a COUNT of failures rather than a horizon,
so an issuer that missed one payment in its first week was graded the worst there is for ever.

**And the grades are still one grade**, because the branch that binds is not the ratio: every issuer
in this world misses payments in every seven-period window, and an issuer that cannot pay what falls
due is what the worst grade is FOR (§44 A2). The measure is reporting the world correctly.

**Where it belongs.** With **12-15's level** at worklist **16**: a world whose firms produce a
hundred and fiftieth of the scale their plant is sized for, whose treasury is refused at the window
748 times a run (12-3), and whose banks have only just been given funding room (11.5) is a world
where nobody can pay. The grade distribution is a MEASUREMENT of that and never a target — Part XII
is where a level is judged, and forcing a spread here would be tuning the assessor to make the world
look solvent. To be positioned when 12a closes.

---

## Found while working item 12b

### 12b-3 — No pair has ever traded: every FX session is `noDemand`, for ever

**Where.** `mechanisms/spot-fx/`, against `foundationSpec('year', drawBanks(4,'year'), drawFirms(40,'year'))`.

**Measured.** All six pairs, every period from 1 to 30:

```
p 1 mkt.fx.USD/EUR noDemand price 1 vol 0      p 30 mkt.fx.USD/EUR noDemand price 1 vol 0
p 1 mkt.fx.USD/JPY noDemand price 1 vol 0      p 30 mkt.fx.USD/JPY noDemand price 1 vol 0
```

Not one bid, not one trade, in any pair, ever. So the rate never leaves the level the seed claimed
and `seed.openingRate`'s own justification is now false: it says "each pair's own first session
replaces it" and "where those meet is the rate from period one", and there is no first session.

**Why it is probably here.** At `410bf16` the pairs DID print (0.9991562719689123) because the
commercial banks held foreign government paper and a bank with a foreign position has an FX reason.
`a6b2922` moved every cross holding to the central banks — correctly, for the liquidity reason it
gives — and the demand side went with it. A central bank holding reserves has no reason to trade
them; the banks that had one no longer hold anything foreign.

**What it costs.** The whole currency layer's price discovery never runs, so every conversion in the
world is a multiplication by one and every defect that lives in a conversion is invisible. 12b was
exactly such a defect and could only be measured by stating a different opening rate.

**Not chased.** Law 11: the misbehaving number is not the work item, the missing mechanism is — and
the missing mechanism here is a reason to hold another country's money. To be positioned when 12b
closes; it is a candidate for **13i (cross-border)**, which is where sourcing across regions and
foreign-currency issuance give somebody that reason.

### 12b-4 — The seed adds two currencies, and one parameter cannot state a consistent triangle

**Where.** `seeds/foundation.ts`: `centralBankAssets` adds `inNamedUnits(ctx, line, drawn) * price`
for each foreign reserve line, where `price` is that line's price in ITS money and the sum is in USD;
and the foreign treasuries' buffer is `mul(reserveUnits, reservePrice, 'what it holds abroad')` — a
USD value — endowed as `c.ccy` money.

**Measured.** Invisible at the delivered opening rate of one. It is Money A2.b ("two currencies are
never added") holding only because the two are the same size.

**And the rate cannot be anything else.** `seed.openingRate` is ONE number for all six pairs, so the
only value that leaves the triangle consistent is 1: at 0.8 the `crossMarket` family reports
`USD through EUR into GBP costs 0.64 against 0.8 direct`. A world cannot currently be opened with
realistic rates at all — 150 JPY to the dollar is not expressible.

**Not chased.** It changes no number in the delivered world and 12b is the identity, not the
opening. To be positioned when 12b closes, with 12b-3: the same item that gives somebody a reason to
hold another money is the one that needs the openings to be real.

### 12b-5 — The trading-book check's dust counts its own terms and not the other side's

**Where.** `mechanisms/banks/index.ts`, `tradingBookIsCapitalised`, contributing to `accounts`.

**Measured.** From period 41 of a 52-period run, every period:

```
bank.a: its dealing book weighs 50408462604.28294 and it published 50408462604.2827
```

A gap of 0.000244140625 — 2^-12, pure binary dust — on numbers of 5.0e10.

**The derivation is short, not absent.** It allows `dustOf(terms.length + 2, |rwa| + |asked|)`, where
`terms` are the dealing lines above target. But `rwa` is the bank's published weighting of its WHOLE
book, a sum over far more terms than this check can see, and Law 7 says the tolerance is what the
arithmetic did — so the terms that went into the other side belong in it. The honest fix is for the
`bank.capital` event to carry the count its own sum had, not for this check to widen a band.

**Not chased, and not widened.** 12b's own step says a check that only passes with a band is
reporting the defect a second time; this one is a different check with an under-derived tolerance.
To be positioned when 12b closes.

### 12b-6 — A research desk keeps covering a company that has ceased

**Where.** `mechanisms/research/` (12a's module), reported by the `names` family.

**Measured.** `p52 names Reporting C2: an estimate names firm.17, which has ceased`, and the same
for `firm.11` at p53.

**What is missing.** Coverage is initiated and dropped for reasons the desk has (D1, D3), and death
is not one of them: nothing in `cover` asks whether the name is still alive, so a desk goes on
publishing a view of a company that no longer exists. The estate item (XI-8) gives death a
destination; being dropped by the analysts who covered it is part of what happens to a name.

**Not chased.** It is 12a's module and 12a is closed; the fix is one read, and it belongs with an
item that is in that module. To be positioned when 12b closes.

## Found while working item 12b.1

### 12b.1-1 — A fund whose whole float is redeemed lives on as an empty vehicle

**Where.** `mechanisms/funds/` (`redeemInKind`, and whatever should notice). Seen with
`foundationSpec('year', drawBanks(4,'year'), drawFirms(40,'year'))`.

**Measured.** The exchange-traded fund's shares are redeemed in kind down to nothing at period 10,
and then:

```
p  9 issued 20000  mkt.share.etf.us noOverlap 985 vol 0
p 10 issued 0      mkt.share.etf.us noDemand 985 vol 0
p 11..52 issued 0  noDemand
```

**What is missing.** Nothing immortal (XI-3) and no death without a destination (XI-8): a vehicle
with no shares outstanding holds nothing and is owed nothing by anybody, and it should wind up — or
creation should be able to restart it, which is what an authorised participant's other half is for
(E3, G1.a). Instead it sits in the world for ever with a market nobody can be on either side of.

**Fixed on the spot, because it STOPPED THE BUILD.** `navOf` threw `Fund Shares B1` — "no shares
outstanding to divide by" — at a dealer asking `view.mark()` for a line it holds none of.
`world.markOf` now answers `none` for a derived line with nothing outstanding, which is what an
OPTIONAL read owes a caller: a claim on a book, per share, has no answer when there are no shares,
and that is XI-6's "unpriced" rather than a crash. A reader that REQUIRES a price still throws at
the site that requires it. The empty vehicle itself is the finding and is not fixed.

**Not chased.** To be positioned when 12b.1 closes.

### 12b.1-2 — A share in this world is worth a fraction of a cent

**Where.** `mechanisms/equity/`, exposed by the price grid.

**Measured.** At period 52 of a four-bank, forty-firm year, WITHOUT a price grid:

```
equity.firm.11  0.00026 USD/share   23,991,042,300 shares
equity.firm.13  0.00030 USD/share    9,579,949,734 shares
equity.firm.17  0.00465 USD/share   48,375,963,805 shares
```

and WITH the grid, the same lines sit on the smallest thing that exists — one cent — because there
is nothing below it to drift to.

**What it says.** The grid did not break the share market; it made the break visible. A firm with
twenty-four BILLION shares outstanding at a hundredth of a cent each is not a share register anybody
would recognise, and a real market's cent tick is a one per cent grid on a price like that. Two
numbers are wrong together — the float and the level — and the level is 12c's finding 12-15 walking
downwards instead of upwards.

**Not chased**, and the tick is NOT loosened to accommodate it: a cent a share is what a share market
quotes in, and a grid widened to fit a broken price would be the price deciding the resolution. To be
positioned with 12c, which owns the share book.

## Found while working item 12c

### 12c-1 — A ceased issuer's share line stays live and keeps printing

**Where.** `mechanisms/equity/`, `mechanisms/estate/`. Seen with
`foundationSpec('probe', drawBanks(4,'probe'), drawFirms(40,'probe'))`.

**Measured.** `firm.13` ceases; its share line `equity.firm.13` has `issued === 0` from then on, so
its published book per share reads `Infinity` — and the market still prints 11 every period, with
`market.noView` firing on it because the only parties left in the book are the desks.

**What is missing.** XI-8: no death without a destination. A company that has ceased has no residual
for a share to be a claim on, so its line should stop trading and its market should close — the
estate settles what is left and the claim is extinguished. Instead there is a live market in the
shares of a company that does not exist, priced by desks alone.

**Not chased.** It is the estate's business rather than the share book's, and 12c is the anchor. To
be positioned when 12c closes.

### 12c-2 — RESOLVED here: 12a-1, the index read a level its own prints did not make

12a's finding, positioned into this item and fixed: the two readers of an index — the kernel's and
the audit's independent one — walked the SAME period's step at different moments in it. The tracker
fund asked during the markets phase, so the kernel chained period t with the companies alive then;
the audit chained it at the close with the ones still alive. A basket is not a function of the
period alone (a constituent's shares outstanding is the count there is NOW, and a delisting takes a
company out of baskets it used to be in), so the two parted by exactly one step and carried the gap
unchanged for ever — which is why it looked like a step and not a drift. The step is now taken once,
at the close of the period, and a reader inside a period gets the last level that is finished.

## Found while working item 12c.1

### 12c.1-1 — The cell partition refines every period and never coarsens

**Where.** `world/cells.ts` (`mergeCells`, which nothing has ever called), `mechanisms/labour/
matching.ts` (which splits on every hire and every release). Seen with `rigWorld('perf', 4, 40)`.

**Measured.** Household cells, by period:

```
at seed 16      p15 260      p30 546      p45 844        — nineteen new cells a period
```

and every one alive. So the world itself grows: parties 318 → 604 → 902, holdings 3394 → 10433,
ledger rows in a period 3188 → 9829 — and the cost of a period goes from 146ms at period 5 to
1755ms at period 55, which makes a run quadratic in the periods it takes.

**AND MERGING WOULD RECLAIM NOTHING.** At period 45, grouped by (cell key + exact per-member state),
844 cells make **843 distinct groups**; ignoring lot acquisition periods too, still 843. The cells are
genuinely different: a member who was hired has been paid and one who was not has not, so every
partial event partitions the population a little finer and nothing ever makes two groups identical
again. The representation degenerates towards one cell per person, which is the one thing a cell
exists to avoid (XI-15: per-member state, no mean).

**What is missing, and it is a question rather than a fix.** XI-15 gives five events that change a
weight and `merge` is one of them, but a merge needs two cells that are the SAME — and in a world
where every wage payment distinguishes its recipient, sameness never comes back. Either the state a
cell carries is coarser than the register (members of a cell share a bank and a cohort, so why not a
balance?), or a cell needs a rule for when two nearly-identical groups become one, and that rule is a
modelling decision with an owner. Neither is a performance question.

**Not chased.** 12c.1 is a Law 18 item and Law 18 says mechanisms, economics and boundaries never
change in one. To be positioned when 12c.1 closes — a candidate for **worklist 16**, where the
representation is measured, or its own item before the scale runs of Part XII.

## Found while working item 12d

### 12d-2 — A treasury bill prints at five times what it redeems for

**Where.** The sovereign secondary book. Seen with `rigWorld('seed-A')` and `rigWorld('seed-B')`.

**Measured.** `ust.bill.2026-06-15` carried at `1.0042` from period 12 in one world and reaching
**`4.949`** in another — a bill that pays one at maturity, priced at nearly five. The engine found it
by dying on it: `yield of ust.bill.2026-06-15 is Infinity`, because the yield that discounts a
payment of one to a price of five, days from redemption, is below minus a hundred per cent and the
discount factor underflows to nothing.

**It is 12c's finding in the bill book.** Both sides of a dealer's quote come from its own view, its
view follows the last print, and a book with nothing else in it is a fixed point — which is exactly
what 12c fixed for shares by giving a saver a reason off the issuer's own published accounts. The
sovereign book has no such party: the households' ladder prices paper off a public curve at their own
required yield (`portfolio.ts`), and the curve is read off the prints, so the anchor is the print
again.

**What was fixed here, and it is only the arithmetic.** A search that walks outward now STOPS at the
edge of what the function answers instead of marching past it (`invertDecreasing`), a present value
says it diverges as the rate approaches minus one rather than throwing from inside the walk
(`curve.ts`), and a curve is built from the lines that HAVE a yield — a print with none is not a
point on one. So the world no longer dies on it and the read is honest. **The price is not fixed.**

**Not chased.** A second reason in the sovereign book is the same item 12c was, in a different market,
and it is not a test migration. To be positioned when 12d closes.

### 12d-3 — A loan's rate is derived twice, and the two do not agree

**Where.** `mechanisms/banks/index.ts`: `publishQuotes` calls `quote(...)` per bank and records the
keenest as `credit.quoted`; `runRequests` calls `shop(...)` — which calls `quote(...)` again — and
writes the loan at `best.rate`.

**Measured.** Same borrower, same period, same bank; quoted `0.013676115348016367` and written
`0.013676161104839884`. Three parts in a million apart, which is nowhere near the dust of either
derivation: the inputs move between the two phases (a bank's cost of funds is read afresh, and so is
what it has seen default).

**Law 4.** One fact, two writers. A borrower took a loan at a rate it was not quoted, and both
numbers are published under its name. The fix is a read, not a second derivation: what is written is
what was QUOTED — `runRequests` should take the published quote rather than re-deriving one — and
where the quoting bank cannot lend after all, that is a refusal to record rather than a silently
different price (C3.a: declined volume is visible).

**Not chased.** It is the lending mechanism and 12d is the tests. To be positioned when 12d closes.

### 12d-4 — An exchange-traded fund's own market clears nothing

**Where.** `mechanisms/funds/`, the ETF's secondary market. Seen with the world `etf.test.ts` builds
(`rigShapeFor(seed, { etfs: 1 })`, households with no buffer).

**Measured.** Before this item, `noDemand` every period — nobody was in the book at all. That half
was a regression 12c introduced and it is fixed here: a saver's reason became the issuer's PUBLISHED
ACCOUNTS, and a fund publishes none, so no saver would hold a fund share. A claim on a book is worth
the book (Fund Shares B1), so a claim whose kind says its value is DERIVED is valued at that — the
kernel's own read, through the same dispatch key everything else is valued by.

**What is left.** `nothingSettled` every period: there are bids now and no trade comes of them, and
no failed instruction names the line either — so the trades are not being drafted rather than being
refused. Four of `etf.test.ts`'s reds are this one thing (a market that clears, a slice taken against
shares, a mark nobody traded at, a book somebody would close).

**Not chased.** It is the funds module and 12d is the tests. To be positioned when 12d closes.

### 12d-1 — No firm in this world ever wants plant, so nothing is ever built

**Where.** `mechanisms/firms/invest.ts` (`project`), `mechanisms/capital-programme/`. Seen with
`rigWorld('capital', 4, 40)` and with the tight world `capital.test.ts` builds (four machines to the
tonne instead of one).

**Measured.** Twelve periods, with the plant requirement turned up four times:

```
capital.commissioned 0      capital.retired 0      machine market: 48 sessions, 0 cleared
firms.funding: {"short":-4862120749,"owed":0,"programme":0,"ccy":"USD"}     — programme 0, every firm
credit.quoted 288                                                          — so it is NOT 12-19
```

**It is not the funding.** 11.5 gave the banks room and they quote: 288 credit quotes in twelve
periods. The chain stops earlier than that — `project()` returns `none` at `if (wanted.length === 0)`,
so no firm ever WANTS a machine, so nothing is bid for, so the machine market never clears in
forty-eight consecutive sessions, so `commission` (which reads settled trades) has nothing to read.

**What it costs.** Ten of the eleven reds in `capital.test.ts` are this one thing — every assertion
of the shape `expected 0 to be greater than 0`. They are NOT migration work and must not be made
green by editing them: a test that asserts plant is bought, built, bid for at what its remaining
service is worth, and retired into a new vintage is asserting the mechanism, and the mechanism does
not run (Law 11, and 12d's own guard names this temptation by name).

**WHY `wanted` IS EMPTY, measured by instrumenting every gate in `project()` and reverting it.** It is
the capacity gate, first try, every firm, every period:

```
GATE gap | gap=-20877525277.3  cautious=4474722.7  capacityNext=20882000000
```

A farm's plant lets it run at **4,670 times** the rate it is sure enough of to build for. A second
firm, from its own start event: `{"planned":5198475,"bound":"plan","capacity":109022000000}` — bound
by its own PLAN at twenty-one thousandths of one per cent of what its plant allows.

**SO THE MECHANISM IS RIGHT.** A firm with four thousand times the plant it needs declining to buy
more is Capital Programme B3 working exactly as written, and `project()` is correct to return none.
There is nothing to fix in `firms/invest.ts` or `capital-programme/`.

**And the test's lever cannot reach it either.** `capital.test.ts` turns `goods.grain.plant.machinery`
up four times to put the farms at their ceiling — but the SEED sizes a firm's plant from that same
recipe figure (`plantOf` = what the line starts x the plant it takes x 1.5 headroom), so raising the
requirement raises the endowment with it and the ratio does not move. Measured at 64x, 1024x, 4096x
and 16384x: `capital.commissioned 0` at every one of them. The knob is not a knob.

**What it actually is.** The seed sizes a firm for `startsOf(f)`, derived from the hours this world's
people offer against the hours its chain needs, and gives it 1.5x the plant that needs. The firm then
plans from what it expects to SELL — and that is about fourteen thousand times smaller. The world can
make far more than anybody in it buys, and every consequence of that follows: no firm is near its
ceiling, no machine is ever bought, no plant is ever built.

**Which is 12-15, and it is not this item's and not the capital mechanism's.** "A world whose firms
produce a hundred and fiftieth of the scale their plant is sized for" is already recorded and already
positioned at **worklist 16**, where a level is judged — and closing a gap between what a world can
make and what it buys reaches the seed's sizing, the recipes, wages, prices and the household
consumption rule all at once. Tuning any of them here to make ten tests pass is exactly what Part XII
exists to prevent.

**So the ten reds stay red and are NAMED.** They assert a mechanism that is correct and never fires,
and 12d may not edit them into agreement with a world that does not invest (Law 11; 12d's own guard).
What 12d owes them is this entry and a worklist row saying which item makes them green: **16**.

### 12b-2 — RESOLVED here: a value in one money written into an account kept in another

Named while reproducing 12b at `410bf16`, and it had two sites, both the same cause:

1. `world/revalue.ts` computed a mark move in the INSTRUMENT's currency and wrote it to an equity
   account kept in the HOLDER's. Gap = `delta x (1 - rate)` =
   `6097242077.790232 x 0.0008437280310877` = `5144414.05`, the reported size to four decimals.
2. `world/assemble.ts` `stateEquityAsRead` had its own copy of the balance sheet that summed
   `valueOfLots` across four moneys without converting any of them, so every central bank OPENED
   contradicting the sheet the audit checks it against.

Both silent at a rate of one, which is why the world never showed it after `a6b2922`. Fixed in 12b:
the mark converts through `intoOwnMoney`, and the seed's copy of the balance sheet is deleted in
favour of `balanceSheet`, the one read.

### 12d-5 — a levy that fails is recorded and then forgotten: no arrears

Measured in `estate.test.ts` at 14 periods of the rig world: **855 failed tax legs** from household
cells, and nothing else a cell posted ever failed. `treasury/index.ts:728` settles the levy and, when
it fails, adds it to `unpaid` on `treasury.receipts` — which is right (Money E1, D3: a payer that
cannot pay has not paid). But `unpaid` is a number in an event and nothing carries it: the cell does
not owe it next period, the treasury does not chase it, and the receipt is not short by it in any
account. So a tax that failed is a hole between two balance sheets that only the journal knows about.

The mechanism is arrears — a levy that is not paid becomes a claim the treasury holds on the payer,
ranking where the law says. There is no instrument for it (worklist 13 gives trade payables their
first one), and the fiscal state is **worklist 14**.

Seen at: `packages/engine/src/mechanisms/treasury/index.ts:728`.

### 12d-6 — twelve firms, eighteen periods, one loan

Counted off `w.instruments.all()` in the estate rig at period 18: `loan 1`, against 34 interbank, 21
repo and 10 subordinated. Firms in this world do not borrow, so nothing about a firm's leverage, its
cost of debt or a bank's corporate book is being exercised by any test that builds this world — and
the dead firm in `estate.test.ts` had issued no paper at all, which is what sent that test looking
for it elsewhere.

Probably the same cause as 12-15 (a world that produces a fraction of what its plant is sized for
and therefore never invests, hence never needs funding), and if so it is **worklist 16**. Recorded
separately because it is a different measurement and may have a different cause.

Seen at: `packages/engine/test/estate.test.ts` (`worldWithADeathInIt`, 18 periods).

### 12d-7 — a saver has no reason to hold a share until the issuer's first published quarter

12c anchored the equity book on published accounts (Reporting A1, A2, §48, `households/portfolio.ts`
`publishedBy`): a cell values a share at the residual the company published plus what it published
earning on it. A company publishes on a fiscal calendar, and the first report is for the first
quarter that OPENS on or after the epoch — measured across five rig seeds, **period 25 to 29**, and
one seed (`etf-stale`, twelve firms) publishes nothing in forty.

So for the first half-year of every run no saver names a price for any share, the equity books have
one side, and nothing about what a share is worth to a holder (Equity B1, B3, XI-13, §46 A3) can be
shown by a test that runs fewer than about thirty periods. From period 25 the books do work: twelve
of the next thirty-one sessions in `equity.firm.30` traded.

The mechanism is not wrong — a saver that invented a figure for an unreported company would be
holding a second set of its books (Law 4) — but a saver with NO read at all is not the only honest
answer either: what a holder can see before the first report is its own basis, the market's own
print, and the company's prospectus. Whether that is a reason is §46's question, and it is worklist
**16** with 12-15.

Seen at: `packages/engine/test/equity.test.ts`, `packages/engine/test/etf.test.ts`.

### 12d-8 — an exchange-traded fund whose desks cannot create, and whose market trades once

Two measurements of the same world (`etf-arb`, the rig at three banks and forty-eight firms, sixty
periods):

1. **No authorised participant ever holds the basket.** 11.5 moved every listed line's float off the
   dealing desks and onto the household cells that save, and nothing moves a share back: at period
   56 the only named holder of `equity.firm.30` is the fund itself. A desk that has not got the
   basket cannot create (Fund Shares F1), so E3 runs one way only and E3.a's premium — measured at
   0.28 of NAV, twenty-six times what a period of carry costs — has nobody able to close it.
2. **The fund's own market trades once in sixty sessions.** Every other session is `noOverlap`, so
   the price is a carried mark in fifty-nine of them and no desk may act on one (Clearing E4). Every
   act of arbitrage in the run therefore lands in the one period after the one session that traded,
   and no run reaches both sides of E3.

(1) is stood in for in `etf.test.ts` by a fixture that opens the participants holding the basket, as
a desk that makes this market would. (2) is not stood in for: the test asserts what the decision
does and names this entry for what the run cannot reach.

Also measured on the way: the fund's own basket holding falls from 6,948,844 to 14,904 over the run
while its shares outstanding do not, so its NAV per share collapses. Not chased (Law 11) — recorded.

Seen at: `packages/engine/src/mechanisms/funds/etf.ts`, `packages/engine/test/etf.test.ts`.

### 12d-9 — no listed firm is ever short, so D1 never fires

`equity.test.ts` "sells new shares when it is short and the market is dear" is red and stays red.
`decideEquity` sells shares only when `spare < 0` — the firm's own funding read says it is short of
what it is about to have to pay — and over forty periods of the rig no listed firm ever is: every
`equity.plan` in the run reads `issue: 0`, and the last of them are BUYBACKS, which is the opposite
corner of the same decision. The mechanism is correct and declines correctly.

Same cause as 12d-1 and 12-15: a world whose firms hold far more than their own plans need, never
invest, never borrow (12d-6) and therefore never raise. Positioned at worklist **16**, where a level
is judged.

Seen at: `packages/engine/src/mechanisms/equity/decide.ts:76`.

### 12d-10 — a line's makers are drawn and nobody reads them

`ListedDecl.makers` (`equity/data.ts:44`) says which of this world's banks make a market in a given
share line — "a market in one name has a few makers, not all of them and not one" (Dealer Desks A3),
drawn `MAKERS_PER_LINE` at a time. Nothing reads it. `dealingOrders` asks the BANK's own `makes`
list, which is by instrument KIND, so every bank that deals shares at all quotes every line in the
world — which is the "every bank has a view of every firm" that A3 says a dealer is not.

It went unread when 11.5 moved the equity float off the desks and onto the household cells that
save: the comment at `foundation.ts:491` records why (the float on the desks raises nothing at the
window and put every bank under the liquidity standard). Moving it was right; what it left behind is
a datum with no reader.

Two things follow, and neither is 12d's:
- A3 is MISSING, not out of scope: quoting should be per line and is per kind.
- A desk therefore opens holding no inventory in any line and builds one only by trading, which is
  what `dealing.test.ts` measures — four of its tests are about a desk with a position (how it skews
  when it is long, which limit binds, what it publishes) and there is never one to measure.

Belongs with the dealer desks, so it is an inserted item after the equity float is settled — the
same place 12d-8's authorised participants are answered.

Seen at: `packages/engine/src/mechanisms/equity/data.ts:44`,
`packages/engine/src/mechanisms/banks/dealing.ts:203`.

### 12d-11 — every bank posts the same deposit board, so no depositor ever moves

Measured over 52 periods of two rig seeds (`deposits-why`, `run-a`): `deposit.moved` fires **zero**
times, no bank draws the window, no session refuses one — and the reason is one number. The three
banks' boards are identical to the last digit:

```
bank.a {"retail":0.012500073062255977,"corporate":0.014500073062255977, ...}
bank.b {"retail":0.012500073062255977, ...}
bank.c {"retail":0.012500073062255977, ...}
```

`defended` (`banks/treasury.ts:334`) matches the keenest rival exactly whenever that rival pays more
than this bank's own offer and less than what the money is worth to it. Matching is free, perfect
and instant, so every board converges on the keenest bank's own offer every period, and the
`depositMargin` each bank was drawn with never shows. A household moves on a GAP (`households/bank.ts`)
and there is never one.

That is Seed B4's failure mode stated the other way round: a sector of equals never produces a
market, and §46 A3's disagreement — the thing that gives a market two sides — has been matched away.
The missing mechanism is whatever makes matching imperfect; A1.d's stickiness is deliberately NOT
it (`setBoard` refuses to subtract a depositor's cost from a rate, and is right to).

**Six tests are named on this and stay red**: `deposits.test.ts` 3 (all three doors, the class that
drains, the class that splits) and `run.test.ts` 3 (the whole E1 → E3.a → B7 → C4.b → failure chain,
which cannot start). It belongs with deposit competition — Banks Funding B1/D1 — and is an inserted
item there, not 12d's.

Seen at: `packages/engine/src/mechanisms/banks/treasury.ts:334`.

### 12d-12 — a desk's limit is measured against its bank's liquidity portfolio

`opening-liquidity.test.ts` asks that every desk opens inside its own limit (Dealer Desks D1, D4).
Measured at the rig's six banks: `roomLeft -58,287,103,191` against a book of 53,417,676,666, and
the book is almost entirely SOVEREIGN PAPER — tens of billions of `ust.bill.*` and `ust.*` per bank,
against 30,531 of the one fund share on it.

That paper is the bank treasury's liquidity buffer, not a position its desk took. `quoteFor` reads
inventory as `view.free(instrument)` and the aggregate book as the whole of it, so a bank holding
what the liquidity standard requires is a desk over its dealing limit before it has quoted anything.
The per-line arithmetic already knows the difference — `state.targetIn(instrument)` is where the
treasury wants the line, and `away = inventory − target` — and the AGGREGATE does not.

One holding, two purposes, one of them measured with the other's ruler (Law 4). Belongs with the
dealer desks, beside 12d-10 and 12d-8. The test is named and stays red.

Seen at: `packages/engine/src/mechanisms/banks/dealing-quote.ts:208`.

### 12d-13 — the deposit guarantee can only pay for a loss no payment can cause

`bank-resolution.test.ts` "pays out of the fund the banks paid into, and the purse only after it"
(D4, D5) is red and stays red.

The arithmetic says exactly when the insurer pays. `unmet = hole − holders`, `holders` reaches at
most the exposed pool, and the pool is every UNINSURED claim (`resolution.ts:283`, `uninsuredAt`).
So with `hole = owes − assets` and `pool ≈ owes − insured`:

> the insurer pays **iff the bank's assets are worth less than its insured deposits**.

That is the right condition — it is what a deposit guarantee is for — and this file's shock cannot
reach it. The loss is a PAYMENT, and a bank can only pay what it holds in money: its loans and its
paper are untouched, so assets never fall below a few hundred million of insured deposits. Measured
across penalties of 1, 8, 20, 40 and 80 times the bank's capital: at 8 the hole is 22bn against a
120bn pool and `insurerPaid` is **8,272** — a rounding remainder, not the guarantee; past 20 the
penalty simply cannot be paid, the bank fails on CASH instead, and the hole goes NEGATIVE (assets
above liabilities, which is D6's funding failure and has no hole at all).

The file's own note saw the shape of this: "the only loss big enough to eat its capital was a
payment, and paying it away took the cash with it." What the clause needs is a VALUATION loss — a
mark collapsing on assets the bank holds — and that is a market event, not a fixture. The test
passed before by asserting a rounding remainder was positive.

Its companion is green and covers the other half: raising the cover stops households being written
down, and lowering it does not (A1.a, D4).

Seen at: `packages/engine/src/mechanisms/money-market/resolution.ts:341`.

### 12d-14 — a bank lends a money it has not funded, and a desk carries one in a money nobody named

Two halves of the same gap, found while closing `money-market.test.ts`'s Law 4 assertion.

**The quote.** `publishQuotes` asks every bank in the world what it would lend a borrower, and
prices the answer off `costOfFunds(bank, the BORROWER's currency)`. A US bank quoting a European
firm is therefore quoting a euro loan — with `owedBy(bank, EUR) = 0`. `costOfFunds` then blends an
interest cost of nothing with the bank's whole equity, so the quote comes out at the bank's required
return on capital and nothing else: measured at 0.0887 against 0.0044 for the same bank's own money.
A bank that has not funded a currency has to borrow or swap it, and nothing here does either.

Worse inside `costOfFunds` itself: `owed` and `couponsPaid` are per currency, but `capital` is
`equity()` — the bank's whole capital in its own money — so the blend adds two currencies (Appendix
B: two currencies are never added).

**The desk.** `carryRate` reads what funding costs the bank and applies it to every line it quotes,
including a line denominated in somebody else's money. Which money a desk funds a foreign position
in is not asked.

Fixed here only as far as the publication: the event now carries what funding costs the bank in
every other money beside its own (`alsoIn`), so the number a quote is priced off is one somebody
published, and `carryRate` names the currency it is taking. The MECHANISM — a bank funding a foreign
loan — is the currency layer's, worklist **12/13h**.

Seen at: `packages/engine/src/mechanisms/banks/index.ts:248` (`costOfFunds`), `:1082`
(`publishQuotes`), `packages/engine/src/mechanisms/banks/dealing.ts:118`.

### 12d-15 — four countries, one population: no FX book ever has a bid

Measured in the rig (`fx`, 12 periods): every one of the six currency markets prints `noDemand` in
every session and the rate stands at its opening 1 for ever. No spot trade has ever settled, so
`spot-fx.test.ts`'s "moves one money against another, both on their own grids, or neither moves"
has nothing to read. It is red and stays red.

The cause is one line of measurement:

```
banks              [ 'bank.a:us', 'bank.b:us', 'bank.c:us' ]
firm regions       [ 'us' ]
household regions  [ 'us' ]
```

The world gained Europe, the United Kingdom and Japan as SOVEREIGNS — a treasury, a central bank, a
curve and a currency each — and no population. Every bank, firm and household in it lives in the
United States and holds nothing but dollars.

From there the FX book cannot have two sides, and the arithmetic says so exactly. A dealer's bid in
`USD/EUR` is `least(room − held, cash(EUR) / rate)` — to BID for the pair it must hold the QUOTE
money — and no party in this world holds a euro. So every desk offers and none bids, in every pair,
for ever. Buying euros would be selling the pair, and the parties who would want to are European,
and there are none.

It also explains two things measured elsewhere: banks are refused at the foreign windows (`ccy: EUR,
foreign: true`) with no market to go to instead, and the foreign treasuries' auctions are covered
many times over and place nothing.

This is the currency layer's (XI-12, worklist **12/13h**): the sovereigns exist and the economies
behind them do not.

Seen at: `packages/engine/src/mechanisms/spot-fx/participants.ts:168`,
`packages/engine/src/seeds/foundation.ts`.

### 12d-16 — the cell grain moves the money by more than whole people are worth

`resolution/cells.test.ts` runs the same world at three cell grains and asks that the aggregates
differ only by what whole people do. The cash differs by **60,119,516** against a bar of
**51,247,510** — seventeen per cent over, and the bar is the mechanism's own: the hours a whole
person supplies over the run, across every labour venue, at the most any venue is paying.

(The bar was read off the LAST `labour.goingRate` event, which with four countries is as likely to
be one where nobody works; it is now the most any venue pays, which is what its own comment said.
The numbers did not move — the United States is the only place in this world with a wage — so the
bar was already right and the excess is real.)

XI-15's own instruction for this is not to widen it: *"if the aggregates move, the resolution is too
coarse and the finding is the resolution."* So the test stays red and this is the finding. What
moves more than people do is what an INSTITUTION does — the test says so itself one line further
down, where the deposit bar is a whole crossing — and which whole cells crossed which bank in which
week is exactly what the grain changes. Whether the answer is a finer opening grain or a mechanism
that stops a cell's whole account moving at once is worklist **16**, with the rest of the level.

Seen at: `packages/engine/test/resolution/cells.test.ts:169`.

### 12d-17 — the bank display names run past Z into punctuation

`BANK_COUNT` is 30 and the seed's display name is `Bank ${String.fromCharCode(65 + n)}`, so banks
27 to 30 are called **`Bank [`**, **`Bank \`**, **`Bank ]`** and **`Bank ^`**. The *id* generator
beside it (`bankName`, banks/data.ts:260) carries to `bank.aa` correctly; the display name is a
second, worse copy of the same rule written at the other end of the codebase (Law 4), and it was
right only while the count was under 27.

Law 9: a display name is what a market calls the thing. `Bank ^` is not a name, and the count that
breaks it is a RESOLUTION nobody may be afraid to raise (banks/data.ts:211).

Seen at: `packages/engine/src/seeds/foundation.ts:634`.

### 12d-18 — the countries are the one population in this world that was typed, not drawn

Every other population is drawn from stated spreads and is deterministic in the world's seed value
(`drawBanks`, `drawFirms`, `drawListed`, `drawFunds`, `drawEtfs`, `drawAssessors` — Seed B1.a, B4).
There is no `drawCountries`. `ABROAD` (foundation.ts:154) is a hand-written table of three rows
carrying `'Europe' / 'European Central Bank' / 'European Treasury' / 'bund'`,
`'United Kingdom' / 'Bank of England' / 'HM Treasury' / 'gilt'`,
`'Japan' / 'Bank of Japan' / 'Japanese Treasury' / 'jgb'`, and the home region is
`'United States' / 'Federal Reserve' / 'US Treasury'` written the same way.

The consequence is not the names. It is that a typed population gets exactly the attributes somebody
typed, and what was typed is a central bank, a treasury and a ten-year line — so the three rows are
**structurally identical** (one tenor, one opening yield, one `crossHoldingShare` split evenly, one
opening rate) and Seed B4's *"a sector of equals never produces a market"* is broken by construction
in the one sector where it was never noticed, because nobody thought of a country as a member of a
population. It is also why every attribute a country needs and did not get — banks, firms,
households, a labour market, a goods market — is absent rather than declared missing: the row has no
field for it, so nothing counts it.

This is the structural half of 12d-15 (which measures the effect: no FX book ever has a bid).

Seen at: `packages/engine/src/seeds/foundation.ts:154`.

### 12d-19 — `region` is a declared cell key dimension with one value

XI-15 and Appendix A: the cell key is what the registry declares, at present `region, cohort, bank`.
Only the United States has households, so `region` stratifies nothing and every cell in the world
carries the same value for a third of its key. A key dimension that cannot cut the ensemble is a
dimension that costs a cross product and buys nothing, and it hides the fact that the represented
sector is one country's.

Not a defect to fix by deleting the dimension — the fix is populations abroad (the same place
12d-18 goes). Recorded so that the resolution measurement in `resolution/cells.test.ts` is read
knowing what it is and is not varying over.

Seen at: `packages/engine/src/seeds/foundation.ts:1692`.

### 12d-20 — `North Asset Management` manages a fund in a world with no north

`funds/data.ts:241` names every bank's money-fund manager `North Asset Management ${at}`. The world
it manages in is the United States; the name is left over from a one-region world called North and
is now a stale fact with a reader (Law 16). Beside it, `'American Index Managers'` and
`'US Listed Equity Fund'` (data.ts:185-187) are typed the same way, while the FIRMS in the same
world are drawn. Fund managers are a population like any other (Seed B1, B1.a) and there is no
`drawManagers`.

Seen at: `packages/engine/src/mechanisms/funds/data.ts:185`, `:241`.
