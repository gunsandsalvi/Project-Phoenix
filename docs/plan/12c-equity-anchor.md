# Item 12c — The equity book walks away

**Read this part first.** This is 11.3's finding in the one book 11.3 did not reach. Item 12's own
preamble described it for the sovereign market — "its only participants are dealers, both sides of a
dealer's quote come from its own view, and its own view follows the last print" — and item 12 fixed
it there. The equity book has the same shape and the same defect. What item 12 measured is repeated
below in full, so this file stands on its own.

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

**And the second finding, which is the same absence read from the other end: EVERY DEALER OPENS
ABOVE ITS OWN LIMIT, BECAUSE THE FLOAT IS THE FIRM'S WHOLE BOOK.** In a rig world of six banks and
twelve firms at period 4, the desk making the listed line publishes `roomLeft: -630,738,745` — six
hundred million more than the limit it set itself, on the first morning, and every desk in every seed
is in that state. Item 12 made the float a READ rather than a stated table (a share is a claim on the
residual, and at period zero a firm's residual is its whole opening book, so the line is that book in
shares of one PHX, as the money was called before item 12 renamed it — Law 19), and that part is right: a larger firm has a larger line without a number
being written beside its name. What is wrong is WHO OPENS HOLDING IT. All of it goes to the banks
that make its market, and a dealer's inventory is a working stock, not the whole float of every name
it quotes. **A saver holds the float; a dealer holds what it can carry** — which is the same sentence
as this item's first step, and is why the two are one item. It is not impossible and it resolves
itself (a desk over its limit stops bidding and sells, D4, and the world is green in every family
with it), but it costs the book its other side on the one morning that decides where the line starts,
and it means no desk in any seed is ever seen with room to GROW: D4's "shrinks the bid to whichever
limit binds" can only ever report the aggregate one, and `test/dealing.test.ts` has to construct a
state to see the other two.

The obstacle is an ordering one: a bank's limit is a share of its capital, and its capital is set by
`seed.funding`, which runs after `equity` because it has to see every asset. Two candidates, and this
item picks one and says why — read the limit against the assets the foundation endowed (a
re-derivation of `seed.funding`'s own rule, so Law 4 says no), or move the float to the households
entirely and let the desks acquire inventory in period one's session (which overturns the recorded
reason that a maker opening with nothing can only ever bid).

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
- [ ] The float opens where that party is, not all of it on the desks that make the line: a desk
      opens inside its own limit and can be seen with room to GROW, and `test/dealing.test.ts` stops
      constructing a state to see the two limits that never bind. One of the two candidates above is
      taken and the record says which and why (Law 4 decides against the first)
- [ ] Build that reason. No bound, no floor under a bid, no cap over an offer — 11.3 was thrown away
      for exactly that and the record says so
- [ ] Test: a world of dealers only is the FAILING case and says so; a world with the reason in it
      keeps a listed line inside what its own cash flows are worth at levels anybody would name
- [ ] `market.noView` no longer fires for the equity books, and that is a read rather than an
      assertion — item 12 built it to be exactly this measurement
- [ ] Coverage re-marked; record entry; delete this file; worklist row 12c → done
