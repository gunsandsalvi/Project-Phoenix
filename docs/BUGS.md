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
