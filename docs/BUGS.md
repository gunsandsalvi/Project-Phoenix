# Open findings — the holding pen

**What this file is.** A bug found while working an item does not derail the item and does not get
chased on the spot. It is written down here, with what was measured and where it was seen, and the
item carries on (Law 10: one ordered list, one item at a time; Law 11: a misbehaving number is not a
work item, the missing mechanism is).

**What happens to it.** When the item closes, every finding here is POSITIONED: moved into the
`docs/plan/<item>.md` of the item that should fix it, or inserted as an item of its own at its
dependency position, with the record saying where it landed and why. A finding is deleted from this
file only by being placed — never by being decided against without a line in `docs/RECORD.md`.

**This file is temporary.** When it is empty it goes.

---

## Found while working item 12

### 12-1 — A bank's opening funding cannot see the assets other modules endow it

**Where.** `packages/engine/src/seeds/foundation.ts`, the opening-balance-sheet block.

**Measured.** At period zero `bank.c` opens at exactly 3.0% equity — its own leverage rule — but
`bank.a` opens at 29.7% and `bank.b` at 23.8%.

**Why.** Those two also hold firm shares and ETF shares, which the `equity` and `funds` modules
endow. Those modules seed **after** `seed.foundation` (assembly order: `… households →
sovereign-instruments → seed.foundation → equity → funds → …`), so the foundation cannot read what
they will hand out and funds only the assets it endows itself.

**Not a slip — an ordering constraint.** `equity` and `funds` need the parties the foundation
creates, so the foundation cannot simply run last. Either the funding derivation moves somewhere
that sees the whole opening state (the kernel's seal already computes assets and liabilities per
party in `stateEquityAsRead`), or each module funds what it hands out. Undecided.

### 12-2 — A bill prints above par: somebody bids a guaranteed loss — **RESOLVED in 12's anchor step**

**Where.** The sovereign secondary market, seen after 12's opening-balance-sheet change.

**Measured.** Over a year `gov.north.bill.2026-06-15` printed to **1.1651** and
`gov.north.bill.2027-03-15` to 1.0946. A bill pays exactly 1.00 at maturity and nothing before it,
so any price above par is a certain loss to whoever paid it.

**Ruled out.** Not the households: their required yield is a positive PREFERENCE
(`HOUSEHOLD_PARAMS.liquidityPremium`) and `savingLines` prices with
`priceAt(flows, required, …)` over the instrument's own cash flows, which cannot exceed par at a
positive yield. The bidder is unidentified — the bank dealing quote and the money fund are the
candidates.

**It was worse than 1.1651 and it stopped the world.** On seed `review` the same bill printed
**3.3337** in period 22 — 3.33 paid for 1.00 due in seven days. No representable yield discounts
those flows to that price, so `yieldOf` drove `invertDecreasing`'s bracket to a discount base of
zero and the curve threw `NonFinite`. Two of seven seeds crashed mid-year.

**Cause, found and fixed.** `viewOf` in `banks/dealing-quote.ts` asked
`outlook('price.<instrument>')` FIRST — the desk's own adaptive expectation _of the price_, formed
from the prints — and fell back to the cash-flow reservation only when it had neither outlook nor
mark. That is XI-13's fixed point written out: print moves outlook moves view moves quote moves
print. For a dated claim the reservation valuation is now asked first, and the outlook is what a
share falls back to, because a share has no payments to discount. All seven seeds run; the audit is
green over a year; no line prints away.

**Placed at 12's close**, in the record, as the anchor step's outcome.

### 12-3 — A bank expects to lose a third to a half of what it lends the sovereign

**Where.** `bank.reservation` journal events; the expected-loss term behind
`requiredYieldOf` (`packages/engine/src/mechanisms/banks/`).

**Measured**, period 12, after 12's balance-sheet change:

```
bank.a  expectedLoss on treasury.north = 0.3333   required 0.0490
bank.b  expectedLoss on treasury.north = 0.5000   required 0.0352
```

A third to a half of the principal, expected to be lost, on **the issuer of the money the loan is
denominated in**. Against the same banks' expected loss on each other — 0.0833 and 0.125 — and on
the funds, zero.

**Why it matters.** This term is most of what holds the banks' required yield near 5% while sovereign
paper yields 2%, so it is most of why the sovereign book has demand in 25 of 312 sessions rather than
in most of them. The anchor step removed the runaway; this is what is left holding the two sides
apart.

**Suspected.** The PD read counts something that is not a default of the sovereign — the treasury
misses payments in this world (`treasury.test.ts` reports twelve over a year) and a model that reads
a missed instruction as a default event would produce exactly this. Not confirmed.

### 12-4 — An audit violation in the bank-failure scenario

**Where.** `packages/engine/test/bank-resolution.test.ts`, at module scope — the file's own setup
builds a world in which a bank fails and asserts the audit is clean before any test runs.

**Measured.** After 12's balance-sheet and anchor changes the whole file fails to collect:
`AssertionError: expected [ Array(1) ] to deeply equal []` — one unexpected audit violation. Eleven
tests do not run at all, which is why the suite total moved from 339 to 328.

**Not reached by the ordinary run.** `foundationWorld` is green in every built family over 52
periods on seven seeds; this is a scenario the foundation does not enter. The violation's family,
owner and size are not yet read — the assertion reports the array, not its contents.

---

## Carried in from item pre12

These two were named in `docs/RECORD.md`'s `pre12` entry and handed to item 12 rather than fixed,
because a documents item may not fix a red test. They are findings, not expected reds.

### pre12-1 — The resolution invariance does not converge

**Where.** `packages/engine/test/tick.test.ts`, "converges as the piece gets finer, in what it made
and in the money it holds".

**Measured.** 0.00928 against 0.00898 — a miss of about three per cent. This is Law 2's resolution
invariance for the whole grid: declare the same world in finer pieces and its path must not move.

**Open question.** Whether the opening balance sheets moved it. If they did, 12's record says so; if
they did not, it is a finding of its own about the grid and wants an item.

### pre12-2 — A matching-rule test depends on its employer's solvency

**Where.** `packages/engine/test/labour.test.ts`, "fills the higher offer first when there are not
enough hours for both (D1.a)".

**Measured and traced.** The venue is correct: it prints 7000, allots all 2100 hours to the higher
bidder, writes six rows of ten, and the wages settle in full (14,700,000 due, 14,700,000 paid, no
failed instruction). Then `firm.1` **ceases inside the same period** — all six rows separate with
`cause: "firm.1 ceased"` and unpaid severance — so both sides of the assertion read zero.

**The finding is about the test.** It builds an employer with no revenue whose only act is to hire
sixty people, and depends on that employer surviving the period it hired in. Whether what starves it
is the opening balance sheet is 12's question; whether a test of a matching rule should depend on
its employer's solvency is a question about the test.
