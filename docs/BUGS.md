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

---

## 13b.1-5 — A resolution bid charges a full year of required return for one week

**Measured.** `doors.test.ts`, the first world the typed parameter reads ran through:

```
InvalidRegistry: [Law 8] parameter bank.returnOnCapital.bank.b is declared in perAnnum
                 ("per annum") and was read as ratio
 ❯ bidFor  src/mechanisms/money-market/resolution.ts:149
 ❯ resolve → Money-market run → World.step
```

**What it is.** `bank.returnOnCapital.<bank>` is a rate per annum, and every one of the four sites
that reads it says so — `costOfFunds` divides by the fraction of a year the period was, the dealing
desk passes `periodOfYear` into `rateOf`, both quotes build a rate that is itself per annum. The
fifth, the bid a surviving bank makes for a failed bank's book (Money-market D3), read the same
number as a dimensionless `ratio` and multiplied it straight onto the assets: `mul(v.assets,
required)`. Its own comment said "for one period". Twelve months of required return, charged against
one week of holding the book — the bid is roughly fifty times too low, so a resolution that should
place a book is a resolution that declines it (D3.b), and the bank that would have taken it looks
like a bank whose equity could not stand it.

**The read is fixed, the arithmetic is not.** The read now names `perAnnum`, which is what the
number is; the missing multiplication by the fraction of a year is the defect, and it is exactly
what the closed vocabulary was built to make visible — the dimension is part of the number (Law 8),
so a rate that never meets a length of time is a rate that has not been used yet.

**Where it goes.** Not this item's: 13b.1 changes no economic outcome (its own guard), and this
changes the price at which a failed bank's book is placed. It is Money-market D3's arithmetic, and
the item that owns a resolution reaching its own price is 13h.

---

## 13b.1-6 — A mean-preserving spread moves what the sector decides, by nineteen pieces a member

**Measured.** `households.test.ts`, "moves cells across a threshold under a mean-preserving spread
while the mean stands (A2.g)":

```
AssertionError: expected 18.99841727653984 to be less than or equal to 1
  test/households.test.ts:547  Math.abs(meanSpend(spread) - meanSpend(flat))
```

Confirmed at `8b562e9` as well, with the same figure to every digit: it arrived with one of the
three redraws, not with the change being made when it was seen.

**What it is.** Two worlds are seeded with the SAME total money reaching the SAME people, spread
differently between cells. The mean of what the sector then decides to spend should be the same in
both to within **one piece per member** — the test's own bound, and it is arithmetic and not a band
(Law 7): moving the same total between cells moves which member holds which cent, and that is all
it can move. Nineteen pieces is not that.

**What it is NOT.** It is not "the sector decides at an average" — the same test asserts, and the
assertion still passes, that not one cell decided at the mean and that the decisions are many. So
A2.f holds. What has moved is the SUM of a nonlinear decision under a spread, which is what
`Σ f(xᵢ)·wᵢ` is supposed to do — but only when a cell crosses a threshold, and the test's own
comment says this world has no threshold its cells straddle. So either a threshold appeared with
the redraw and the test's comment is now out of date, or something in a household's plan is not
homogeneous of degree one in what it holds when it should be. Which of the two it is, is the
measurement to take.

**Where it goes.** Not this item's: 13b.1 changes no economic outcome, and the dispersion of what
households hold is 13d's (Households E/F, the life cycle) or 16's (Part XII, measure). Positioned
when the item closes.

---

## 13b.1-7 — No spot FX trade happens at all in the rig's world

**Measured.** `spot-fx.test.ts`, "moves one money against another, both on their own grids, or
neither moves": `expected 0 to be greater than 0` at the count of two-legged money instructions
seen. Confirmed identical at `7cb266c`, before the market-shape change that was being made when it
was seen.

**What it is.** The test walks the journal for an instruction with two money legs in two currencies
and finds none: the pair market exists and nobody trades in it. Every other assertion in the file
passes, so the venue, the pair naming and the triangle read are all there — what is missing is a
party with a reason to convert. A world where nobody needs a foreign money is a world whose FX
market prints nothing, and that is a mechanism nobody has built rather than a number behaving badly
(Law 11): what makes somebody buy a currency is an import, a foreign asset or a foreign liability,
and item 12's currency layer is the first of those.

**Where it goes.** 12 or 13c (trade in goods across a border gives the first real reason). Not
this item's: 13b.1 changes no economic outcome.

---

## 13b.1-8 — The clearing house is not flat by nine hundredths

**Measured.** `derivative-layer.test.ts`, "the house is flat by construction: what it holds on one
side it owes on the other": `expected -0.09600000000000719 to be +0`. Confirmed identical at
`7cb266c`.

**What it is, and it is not dust.** C2 says the house is buyer to the seller and seller to the
buyer at the same level in the same instruction, so its own position is nothing BY CONSTRUCTION —
not to within a tolerance, exactly. Nine hundredths on a position of zero is not a last-bit
disagreement: something is writing one side of a cleared pair at a level or a size the other side
does not have. The two candidates are the margin legs (posted per side, against different
counterparties) and a contract cut by `admits` after one of the two legs was already priced.

**Where it goes.** Derivative Layer C2 is item 13a's clause and the layer is its module; the house
being flat is the property that makes a central counterparty a central counterparty, so this is a
mechanism finding and not a measurement one. Positioned to 13h, which owns what is left of the
derivative layer's counterparties.

---

## 13b.1-9 — Nobody posts a schedule into an option book

**Measured.** `derivative-classes.test.ts`, "opens an option book, which a bootstrap of one tick
could not (D7.b, D4)": `expected 0 to be greater than 0` at the count of orders any living party
posts into any option book. Confirmed identical at `7cb266c`.

**What it is.** The books are there and the class has an `orders` of its own; what comes back is
empty for every party. The test's own comment already names the family this belongs to: `13b-10` —
every party this world admits to a contract book is on the same side of it, because
`TRADES_CONTRACTS` is banks and firms and in this world both want the same thing. This is the same
finding one class over, and it is stronger: here not one party wants either side.

**Where it goes.** 13h, with `13b-10`: the counterparty that would take the other side of an
option — an insurer, a pension fund — is that item's to build.

---

## 13b.1-10 — Seventeen participants post a quantity with no level, and one of them is the central bank

**Measured.** Read, not run: `grep "price: 'market'"` over `src/mechanisms` gives seventeen sites in
nine modules — funds, the tracker, the central bank's open-market desk, estates, subordinated debt,
the dealing desk, spot FX, firms and households.

**What it is, and Clearing A2.a said it in advance.** "Each participant posts a **schedule**, not a
point... A market expressed as 'here is the quantity I want' has no level, only a shape, and forces
every venue to invent its own rule." `resolveMarketOrders` is that invented rule. It now says so
where it lives and states the choice deliberately — the worst level the other side actually posted,
because the level taken is then one somebody really asked for and the price still comes out of
posted supply meeting posted demand (Law 3).

**The consequential one is the central bank.** A bidder with no level is a price-taker of a price
this mechanism has not yet produced (Clearing A4), and a big enough one is the marginal order that
sets it. `central-bank-omo` closes a gap towards a 25%-of-line target, so in every sovereign session
where it has a gap it bids at the top of the book for a quarter of the line. The quantity limit is
real policy (Central Bank C1) and the module correctly refuses to stand in any other market, so this
is not Appendix B's buyer of last resort — but it makes the central bank the marginal price-setter
in the sovereign book rather than a large participant in it, which is more aggressive than any real
open-market operation and is a price the polity is setting by another route.

**Where it goes.** Each poster needs the level at which its own reason stops, which is a mechanism
per module and not one change. The central bank's is **14** (the polity: what the central bank may
and may not do to a price is that item's subject, and its administered rate arrives there). The
other eight are read together at **16**, where Part XII measures what the market shapes actually
are. Not this item's: 13b.1 changes no economic outcome, and every one of these changes a price.

---

## 13b.1-11 — An index publishes a level and then cannot answer for it

**Measured.** `indices.test.ts`, "records what each rule came to, and the rule still answers for
itself": `expected false to be true` at `w.index(id).some`, for an id read off the module's own
published `indices.level` event. Confirmed identical at `0ae9b7f`.

**What it is.** Indices E1/E2: what is published is an OBSERVATION of a rule, never a stored level
(Appendix B: no stored index level), so the published number and the read must be the same rule
asked twice. Here a rule published a level in a period and the read of that same id came back
Missing — so either the rule's constituents stopped answering between the publication and the read,
or the two are not asking the same rule. The first is the likely one and is the interesting one: a
constituent that de-listed, or a line whose print was not there when the read ran, leaves the index
with nothing to compute from, and "the index says nothing" is then the honest answer while
"something published a number for it this period" is not.

**Where it goes.** Indices is item 12's, and what makes a constituent stop answering mid-period is
a phase-order question (Clearing F1.a). Positioned to 16 with the other measurements, unless the
read turns out to be asking a different rule — in which case it is Law 4 and belongs where the
publication is written.

---

## 13b.1-12 — Only some of the three deposit classes ever moves bank

**Measured.** `deposits.test.ts`, "gives each kind a reason of its own, and all three of them fire":
`expected false to be true` on one of the three `kinds.has(...)` assertions. Confirmed identical at
`0ae9b7f`.

**What it is.** Banks Funding A1.a/A1.b/A1.c give three deposit classes three different reasons to
leave a bank: a household moves for the rate it is not being paid, a firm moves off a bank that drew
the window, a fund leaves one whose session refused it. The test asserts all three doors fire over
the run, because A1.d's whole point is that a bank's funding is not one substance — and a world
where only one class ever moves is a world with one deposit type in it, which is the thing A1.d
says a bank's liability side is not.

At least one of the three never fires. Which one, and whether its reason is unreachable or merely
never reached in this draw, is the measurement to take: a reason that cannot fire is a mechanism
finding (the door is wired to a condition nothing produces), and a reason that could fire but did
not in thirty periods of one seed is a draw.

**Where it goes.** Banks Funding A1 is item 11's and the deposit market is built; what is missing
is either a condition or the run length to reach it. Positioned to 16, where Part XII measures what
actually happens over a long run, unless the first look shows a door wired to nothing — in which
case it is 13d's, with the household life cycle that gives a household its reason.
