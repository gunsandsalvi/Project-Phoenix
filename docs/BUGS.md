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

### 12-2 — A bill prints above par: somebody bids a guaranteed loss

**Where.** The sovereign secondary market, seen after 12's opening-balance-sheet change.

**Measured.** Over a year `gov.north.bill.2026-06-15` printed to **1.1651** and
`gov.north.bill.2027-03-15` to 1.0946. A bill pays exactly 1.00 at maturity and nothing before it,
so any price above par is a certain loss to whoever paid it.

**Ruled out.** Not the households: their required yield is a positive PREFERENCE
(`HOUSEHOLD_PARAMS.liquidityPremium`) and `savingLines` prices with
`priceAt(flows, required, …)` over the instrument's own cash flows, which cannot exceed par at a
positive yield. The bidder is unidentified — the bank dealing quote and the money fund are the
candidates.

**Why it matters here.** This is XI-13's defect in the one market the model funds itself through,
and item 12's own exit criterion is that a line's price stays where its own cash flows put it. In
scope for 12's anchor step rather than for this file — listed so it is not lost if the anchor step
closes without reaching it.

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
