# The worklist

One ordered list, worked strictly in order, one item at a time (Law 10). Take the first open item.
A new item is **inserted** at the position its dependencies put it, never appended; the record says
where it landed and why. An item is done when its checks are green, `docs/RECORD.md` has its entry,
and `docs/COVERAGE.md` is re-marked, all in one commit (Law 14).

The order follows Part XIII. Items are deliberately coarse-grained at the far end: each is split into
bounded items when it becomes the item in hand, never earlier (Law 11: do not plan a world that has
not arrived).

| # | item | state |
|---|---|---|
| 0 | **Foundation.** Rules digest, architecture, toolchain, lint rules, CI, Pages, APK pipeline, spec index and citation check. | done |
| 1 | **Money and settlement, one calendar** (Money A–G). Money as an instrument; the wire; settlement with the interbank leg; fails; overdraft decisions recorded; calendar and day counts. | done |
| 2 | **Register, clearing, cells, DvP, value, parameter register** (Register A–F, Clearing A–F, XI-15, XI-5, XI-6, XI-14). Holdings with lots and liens; solver; markets; prints with provenance; revaluation; equity accounts; split/merge; audit families; seed at period zero; observer and worker. | done |
| 2a | **Kernel/module boundary.** Runtime kind registry with profiles; `SystemModule` contract and `assemble()`; `ParticipantView`, `MechanismContext`, `SeedContext`; register write access private to the kernel; sovereign instruments as the first module; seed as a module with PRNG-drawn dispersion; cross-module import lint. Inserted before 3 because every mechanism from 3 on is a module. | done |
| 3 | **The sovereign's funding constraint** (Treasury, Sovereign, XI-9). Treasury outlays and receipts with named payees and payers; the forward-looking programme; the uniform-price auction with primary dealers' obligation; bills and bonds as two instruments; the secondary market and the curve with provenance; accrued interest travelling with the trade (Bond N9.b); buybacks; the central bank's open-market purchases; no overdraft. Deletes placeholder `seed.openingPrice.gov.north.2036`. | open |
| 4 | **A firm's real cost base, the households and labour it pays and sells to, and the outlooks they act on** (Firm, Goods A–F, Labour A–D, Households A–E, Expectations, XI-16, XI-10 first half). Named cost lines with real payees; invoice book; production decisions with a recipe; employment as a register; wages to household cells; consumption from the cell's own outlook; the memory preference. | open |
| 5 | **A loss is an event** (XI-1; Banks Lending E; Firm D; Firm Birth C). Threshold crossings, statuses that are written, provisions booked, write-offs dated. | open |
| 6 | **Loans are rows** (Banks Lending A–D, F; Money B3.a). The credit decision; the bank's cost of funds; customer overdrafts as decisions. | open |
| 7 | **The forced seller, nothing is immortal, the estate** (XI-2, XI-3, XI-8; Firm Birth D; Banks Capital C–E). Estates that sell into real markets; waterfalls; successors for every reference. | open |
| 8 | **Redeemable claims** (Fund Shares). NAV as a read; subscriptions and redemptions that sell. | open |
| 9 | **Equity, and dealers that carry inventory** (Equity, Dealer Desks). Shares as a non-liability instrument; a desk with a limit, a funding cost and a skew. | open |
| 10 | **The cost of capital** (XI-4, Capital Programme). Investment as a project against a hurdle; the bank's blended cost of funds in the loan price; the desk's rent. | open |
| 11 | **Bank capital that can be raised, and the money market with its corridor** (Banks Funding, Banks Capital A–B, Money Market, Central Bank B–D). Prices the reserve overdraft; deletes the recorded-but-unpriced overdraft path. | open |
| 12 | **The currency layer, benchmarks, the second opinion** (Currency, Spot FX, FX Forwards, Indices, Ratings, XI-12, XI-7, XI-13). Removes the single-currency guard in settlement. | open |
| 13 | **Employment's other half, housing, securitisation, corporate control** (Labour E–F, Housing, Small-Business Pools, M&A, Trade Credit, Securities Lending, Prime Brokerage, Derivative Layer and the classes, Insurers, Hedge Funds, Private Equity, Freight, Commodities, Cross-Border, XI-10, XI-11). Split into bounded items when reached. | open |
| 14 | **The polity** (Polity, XI-17). | open |
| 15 | **The recipe** (Goods A2). Last, deliberately. | open |
| 16 | **Measure** (Part XII, in full). Only then. | open |

## Standing decisions the list depends on

- The observer surface ships as the **inspector** during development (Observer A4); which product the
  APK is remains the owner's decision and is not on this list until taken.
- Performance campaigns (Law 18) are inserted where cost growth is observed, never scheduled.
