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
