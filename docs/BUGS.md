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
