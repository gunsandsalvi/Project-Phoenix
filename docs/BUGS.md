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

### 12b-1 — A price is not on any grid, so every value is finer than money goes

**Where.** `prices/price-store.ts` (`PricePrint.price` is a bare `number`), `clearing/solver.ts`
(a fill is `at: g.o.price` — whatever the poster computed), and every read that multiplies by one:
`prices/value.ts` `valueOfLots`, `markPerUnit`, `carryingPerUnit`.

**Measured.** Prints carry sixteen significant figures: `share.etf.us` at `511.12891311042875`,
`equity.firm.11` at `49.7938868935811`, `jgb.2036-03-15` at `0.8887345371057928`. So every
`units × price` number is off the money grid by construction — a balance sheet at
`15901695062647.4`, an equity account at `2407881467552.351`, a revaluation delta at
`-6097242077.790232`. A money BALANCE is a whole count of pieces (`money:ecb:EUR 306717802678`)
because a quantity has a type and five doors (`core/tick.ts`); a price has neither.

**It is a gap, not a decision.** The spec has no clause on price resolution and no parameter
declares one, and nothing in the code states that a price is deliberately continuous. `core/tick.ts`
argues for the quantity grid from the real mechanism — "a unit of anything real has a smallest piece
and nothing finer exists" — and that argument is word for word the argument for a price: a market
has a minimum increment and prints on it. `clearing/market.ts` (fxTrade, assetTrade) already names
the consequence it lives with: cash lands on its own grain, so "the rate a trade REALISES can differ
from the print by less than one piece".

**Shape of the fix, when it is taken.** The increment is a RESOLUTION (Law 2), declared per market
beside `UnitDecl.perUnit` and shifted with it so invariance can be run. It belongs at the POSTING
door — an order posts at a tick of its market, the way a size posts on the unit's grid — and never
on the print: a cleared price is an outcome, and rounding the print would be a bound on it (Law 6).

**Where it belongs.** Not 12b, which is one wrong conversion (12b-2) and nothing to do with
resolution; a price grid changes every cleared number in the world and is measured, not guessed.
To be positioned when 12b closes.

### 12b-2 — RESOLVED here: the revaluation mark is booked in the instrument's money

Named while reproducing 12b at `410bf16`: `world/revalue.ts` computes a mark move in the
INSTRUMENT's currency and writes it to an equity account kept in the HOLDER's. Gap =
`delta × (1 − rate)` = `6097242077.790232 × 0.0008437280310877` = `5144414.05`, which is the
reported size to four decimal places. Still live in today's tree and silent only because every
rate is exactly 1. Fixed in 12b.
