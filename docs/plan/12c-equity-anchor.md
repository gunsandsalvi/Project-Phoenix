# Item 12c — The equity book walks away

**Read this part first.** This is 11.3's finding in the one book 11.3 did not reach. Item 12's own
preamble described it for the sovereign market — "its only participants are dealers, both sides of a
dealer's quote come from its own view, and its own view follows the last print" — and item 12 fixed
it there. The equity book has the same shape and the same defect, and `docs/BUGS.md` **12-15** is the
measurement.

**The finding.** With `foundationSpec('probe', drawBanks(4), drawFirms(40))`, `equity.firm.13` prints

```
100.00, 99.91, 99.76, 99.73, 127.13, 9613.36
```

and then the market stops trading at all. Every one of those is a session with real settled volume
behind it (1.2e9, 6.8e8, 1.9e10, 7.9e9, 2.3e6 units), so it is not a print with nobody on the other
side — it is desks crossing each other and carrying the line with them. Confirmed pre-existing: the
series is identical with item 12's index tracker removed.

**Why the tracker is not enough.** Item 12 built a fund whose mandate is an index's rule, and that
IS a second reason in the book — but a weak one: its target is its holding, so most periods it has
nothing to trade, and it pays what the line last printed rather than whatever is asked (Appendix B:
no forced buyer). A second reason that is silent most weeks does not anchor a book.

**Where it lands and why.** Before 13g. A tender offer is priced off a listed share and the
acquirer's own view of it, and a price that walks is not one. It is also what 12a's earnings surprise
is measured against (§48), so the closer it sits to 12a the less of 12a has to be re-done.

**Objective.** A listed line's print over a year stays inside what the claim on that firm is worth to
somebody who would hold it, with **no bound anywhere in any quote**. XI-13 in the book that prices
corporate control.

**Read first.** XI-13; §10 Equity B1, B3, C1, C2, C2.c; §24 Dealer Desks (all); §3 Clearing A2, C3,
E1, E4; §46 A3; Law 3, Law 6, Law 11, Law 13; Appendix B (the price and bound lists). Code:
`mechanisms/banks/dealing.ts` and `dealing-quote.ts`, `mechanisms/equity/`,
`mechanisms/households/portfolio.ts` (the saver's own ladder), `mechanisms/funds/tracker.ts`. The
record entries for `11.2`, `11.3` and `12` — 11.3 is an attempt that was thrown away for writing a
bound, and it is the cheapest way to learn what not to do here.

## Steps

- [ ] Name the party whose reason to be in an equity book is not the last print. The candidates this
      world has are a saver holding a claim on a firm's earnings (Equity B3) and 13h's hedge fund;
      say which, and why the other is not it
- [ ] Build that reason. No bound, no floor under a bid, no cap over an offer — 11.3 was thrown away
      for exactly that and the record says so
- [ ] Test: a world of dealers only is the FAILING case and says so; a world with the reason in it
      keeps a listed line inside what its own cash flows are worth at levels anybody would name
- [ ] `market.noView` no longer fires for the equity books, and that is a read rather than an
      assertion — item 12 built it to be exactly this measurement
- [ ] `docs/BUGS.md` 12-15 removed by being FIXED
- [ ] Coverage re-marked; record entry; delete this file; worklist row 12c → done
