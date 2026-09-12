# Findings, waiting to be positioned

Everything measured during an item that is not the item's own work (CLAUDE.md: *every bug is written
down — all of them*). A finding leaves this file only by being **positioned**: moved into the
`docs/plan/<item>.md` of the item that should fix it, or inserted as its own item at its dependency
position. The file is temporary and goes when it is empty.

Open at: **13b.1**, the checks that do not check.

---

## 13b.1-1 — A fund redeems shares whose cash value rounds to nothing — FIXED AT THE WIRE

**Measured.** `balance-identity.test.ts`, both year-long runs, after 13b.1 redrew the world:

```
Impossible: [Money C1] money leg amount must be positive, got 0
 ❯ Settlement.validateMoney src/ledger/settlement.ts:291
 ❯ redeem  src/mechanisms/funds/index.ts:450
 ❯ payQueue → strike
```

**What it is.** Shares come back in whole pieces and cash goes out in whole pieces, and the two
grids are not the same grid (Law 8). A share worth less than a piece of money gives a whole number
of shares whose cash value rounds to nothing, and the fund tried to take the shares and pay zero —
a one-sided flow (Law 5), which the wire refused at the site.

**Fixed where it is, because it stops the build.** A payment below one piece of money is not a
payment: the request stays on the book under its own name and the fund is gated, which is the
answer C2.b already has for a redemption it cannot pay.

**What is NOT fixed.** Why a fund's shares are worth less than a piece of money at all. That is
`13b-9`'s open half, positioned to 13h, and this is the second place it has surfaced.

---

## 13b.1-2 — Two walks of one bank's risk-weighted assets disagree in the last bit

**Measured.** `run.test.ts`, after 13b.1 redrew the world:

```
accounts: bank.a: its dealing book weighs 424622285.4494608
          and it published 424622285.4494532 of risk-weighted assets in total
```

A difference of 7.6e-9 on 4.2e8 — 1.8e-17 relative, which is one or two units in the last place.

**What it is.** An audit family comparing the dealing book's weight against the bank's published
total. Two sums of the same terms in different orders do not agree bit for bit, and the family's
derived dust does not cover the difference. Law 7 says a check that only passes with a band is
reporting a defect — and the defect here is that the answer is computed twice (Law 19: read the
source, do not re-derive it), not that the band is too narrow.

**Where it goes.** It is this item's own: the audit step re-states what `built` means and what the
flows family exempts, and this is the same question one family over. Whether the fix is that the
family reads the published figure instead of re-walking, or that the dust is derived from the walk
that produced it rather than from the comparison, is decided when that step is taken.

---

## 13b.1-3 — Two tests named an outcome of the draw — FIXED

**Measured.** `world.test.ts`, after 13b.1 redrew the world: `expected 2 to be 1` on
`gov?.failedTrades`, and `expected 7 to be 6` on the placeholder count.

**What it is, and it is the rule CLAUDE.md states.** A test never names a party, and neither of
these was naming one on purpose — but `failedTrades === 1` is a statement about how many sellers
the buyer's money ran out against, which is an outcome of which banks the draw gave which holdings.
And the placeholder count moved because WHICH bank the money fund is launched at is an outcome too,
so a redrawn world has a differently named fee.

**Fixed in the tests, not the engine.** The first now asserts what Register C3.b is about: at least
one trade failed, every failed one carries no deltas at all, and the buyer holds exactly the settled
volume and paid exactly for it. The second asserts the count the world actually declares, with the
reason it moves written beside it.

---

## 13b.1-4 — Two steps of this item were wrong on the arriving world's terms — BOTH CHANGED, PLAN §5.2

Written down because §5.2 says the reason goes in the record, and because both were tried, measured
and undone rather than reasoned away.

**"`namesAnItem` runs on every kind."** Turned on, the world would not assemble: eleven policies and
preferences cite a future worklist item in their reason, and `params.ts`'s own comment says why they
may — a rate parliament owns from 14, a comparison that becomes real at 11. They are still there
afterwards; what changes is who sets them. The guard now asks a SHAPE and a TECHNOLOGY, which is
where the real hole was: a technology is a fact about the world and a fact about the world has no
scheduled death.

**"`worthOf` and `markPerUnit` answer the same question the same way."** They do not answer the same
question. `worthOf` is "what is this worth and HOW OLD is that" — it hands back the print's own
period, which is what makes a stale mark visibly stale (Observer A1.a), and a line that never printed
is honestly nothing. `markPerUnit` is "what is a unit marked at NOW", asked inside a period whose
phases are ordered, where a print that is not there yet means a phase in the wrong place and throwing
is the point (Clearing F1.a). Merging them turned every read of a line before its first session —
including the seed's own valuation — into `NotYetProduced`, measured in `tick.test.ts` at four tests.
Both functions now state the difference where they are.
