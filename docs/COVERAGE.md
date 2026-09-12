# Requirement coverage

One row per REASON, VERIFY and FORBID in the specification. `MET at <path>` means the cited module
implements the clause; `PARTIAL` says what is still missing; `MISSING` is work; `OUT OF SCOPE` states a
reason and keeps the clause (Appendix C). Re-mark in the same change that meets a requirement, and
recount with `npm run coverage:spec` rather than adjusting a tally.


## Money

| requirement | status | where / why |
|---|---|---|
| `Money A1` | MET | packages/engine/src/registry/profiles.ts, packages/engine/src/seeds/foundation.ts |
| `Money A1.d` | MET | packages/engine/src/audit/families/money.ts |
| `Money A2` | MET | packages/engine/src/core/tick.ts (money is a COUNT of indivisible pieces and the count is a branded Qty, so an amount below a piece cannot be constructed anywhere; and a PRICE has a smallest increment for the same reason — `downToTick`, `upToTick`, `toTickOf`), packages/engine/src/registry/grid.ts, packages/engine/src/registry/registry.ts (the one boundary between a person’s number and the state’s), packages/engine/test/tick.test.ts |
| `Money A2.b` | MET | packages/engine/src/ledger/settlement.ts (a money leg names one currency and settles in it; what an amount in another comes to is a conversion at a rate somebody traded, never an addition), packages/engine/src/prices/value.ts, packages/engine/src/world/revalue.ts (a mark is in the instrument’s money and an account in its party’s: the move between them converts) |
| `Money A3` | MET | packages/engine/src/world/actions.ts (accountResolver: which account a party holds a money in — its own bank for its own money, that money’s own central bank for a foreign one), packages/engine/src/world/world.ts |
| `Money A4` | MISSING |  |
| `Money B1` | MISSING |  |
| `Money B1.b` | MISSING |  |
| `Money B2` | MISSING |  |
| `Money B3` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/registry/kinds.ts |
| `Money B3.c` | MET | packages/engine/src/mechanisms/banks/index.ts (a customer overdrawn at its bank has a lender, a rate and a date by the close), packages/engine/src/mechanisms/money-market/index.ts (and so does a BANK overdrawn at the central bank: what it drew is a repo row at the ceiling plus the penalty, and the unpriced path is gone) |
| `Money C1` | MET | packages/engine/src/ledger/instruction.ts, packages/engine/src/ledger/settlement.ts (a leg carrying a quantity that is not a whole number of its unit's pieces is refused at the site, including the per-member side of a cell leg) |
| `Money C2` | MET | packages/engine/src/ledger/settlement.ts |
| `Money C2.c` | MET | packages/engine/src/audit/families/money.ts, packages/engine/src/ledger/settlement.ts |
| `Money C3` | MISSING |  |
| `Money C4` | MET | packages/engine/src/audit/families/money.ts, packages/engine/src/ledger/settlement.ts |
| `Money C4.c` | MET | packages/engine/src/audit/families/money.ts, packages/engine/src/audit/memory.ts |
| `Money D1` | MET | packages/engine/src/ledger/instruction.ts, packages/engine/src/ledger/ledger.ts, packages/engine/src/ledger/settlement.ts |
| `Money D2` | MET | packages/engine/src/ledger/instruction.ts, packages/engine/src/prices/value.ts, packages/engine/src/register/instruments.ts, packages/engine/src/registry/profiles.ts, packages/engine/src/seeds/foundation.ts |
| `Money D3` | MET | packages/engine/src/audit/families/flows.ts, packages/engine/src/audit/memory.ts, packages/engine/src/ledger/settlement.ts |
| `Money D4` | MET | packages/engine/src/audit/families/flows.ts, packages/engine/src/ledger/settlement.ts, packages/engine/src/world/context.ts |
| `Money E1` | MET | packages/engine/src/ledger/settlement.ts (a payer that cannot pay does not pay, and nothing half-settles), packages/engine/src/mechanisms/credit-events/index.ts (and there is a named thing it then IS: in default of payment, publicly, with the payee and the amount that did not arrive). What that state then costs it — a lender's reaction, a rating, its death — arrives with lending (worklist 6) and the estate (worklist 7) |
| `Money E2` | MET | packages/engine/src/ledger/ledger.ts, packages/engine/src/ledger/settlement.ts |
| `Money E3` | MET | packages/engine/src/ledger/settlement.ts |
| `Money E4` | MET | packages/engine/src/ledger/settlement.ts (a leg naming a ceased party throws at the site), packages/engine/src/mechanisms/estate/index.ts (and its estate assumes what it issued, so a holder's claim names somebody who exists) |
| `Money F1` | MISSING |  |
| `Money F1.a` | MISSING |  |
| `Money F2` | MISSING |  |
| `Money F3` | MET | packages/engine/src/audit/families/money.ts |
| `Money F4` | MET | packages/engine/src/audit/families/money.ts |
| `Money G1` | MET | packages/engine/src/calendar/calendar.ts, packages/engine/src/world/world.ts |
| `Money G2` | MET | packages/engine/src/calendar/calendar.ts, packages/engine/src/world/world.ts |
| `Money G3` | MET | packages/engine/src/calendar/calendar.ts |
| `Money G3.b` | MET | packages/engine/src/calendar/calendar.ts |
| `Money G4` | MET | packages/engine/src/calendar/calendar.ts, packages/engine/src/journal/journal.ts, packages/engine/src/ledger/instruction.ts, packages/engine/src/world/world.ts |
| `Money G4.a` | MET | packages/engine/src/calendar/calendar.ts |

## Register

| requirement | status | where / why |
|---|---|---|
| `Register A1` | MET | packages/engine/src/register/register.ts |
| `Register A2` | MISSING |  |
| `Register A3` | MET | packages/engine/src/audit/families/names.ts, packages/engine/src/audit/families/ownership.ts, packages/engine/src/register/register.ts |
| `Register A4` | MET | packages/engine/src/audit/families/names.ts, packages/engine/src/register/instruments.ts |
| `Register B1` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/register/instruments.ts |
| `Register B2` | MET | packages/engine/src/audit/families/ownership.ts, packages/engine/src/register/register.ts |
| `Register B3` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/register/register.ts |
| `Register B4` | PARTIAL | maturity ceases an instrument; default-into-recovery arrives with XI-1 |
| `Register C1` | MET | packages/engine/src/ledger/instruction.ts, packages/engine/src/ledger/settlement.ts, packages/engine/src/register/register.ts |
| `Register C2` | MET | packages/engine/src/ledger/instruction.ts |
| `Register C3` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/ledger/instruction.ts, packages/engine/src/ledger/settlement.ts |
| `Register C4` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/register/register.ts |
| `Register C5` | MISSING |  |
| `Register D1` | MET | packages/engine/src/register/register.ts |
| `Register D2` | MET | packages/engine/src/register/register.ts |
| `Register D3` | MET | packages/engine/src/prices/value.ts |
| `Register D4` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/register/register.ts |
| `Register D5` | MET | packages/engine/src/register/register.ts |
| `Register E1` | MET | packages/engine/src/registry/kinds.ts, packages/engine/src/world/actions.ts |
| `Register E2` | MET | packages/engine/src/registry/kinds.ts, packages/engine/src/world/actions.ts |
| `Register E3` | PARTIAL | packages/engine/src/mechanisms/credit-events/index.ts names every holder of a claim that stopped performing and how much of it they are carrying. The loss now LANDS: a firm's estate realises what it holds, pays in rank order and writes off the rest, and the write-off is what each holder was carrying (packages/engine/src/mechanisms/estate/index.ts). A sovereign still has nothing seizable (G3): its loss takes a negotiated exchange (worklist 13f) |
| `Register E4` | PARTIAL | split, buyback and new issue apply through issuance legs, and the split has its door (item 9). The driver that ISSUES the other two — a board buying its own shares back, a board selling new ones — arrives with corporate control (worklist 13g), which is where item 11 placed raising equity and restricting distributions |
| `Register E5` | PARTIAL | every register event so far moves money, so the clause holds by having no exceptions to explain. The exceptions arrive with the corporate-action driver (worklist 13g): a split moves quantities and no money, and what it must then record is the why-not. Measuring the VERIFY over a window is Part XII (worklist 16) |
| `Register F1` | MET | packages/engine/src/register/instruments.ts |
| `Register F2` | MET | packages/engine/src/audit/families/names.ts, packages/engine/src/parties/party.ts |
| `Register F3` | MET | packages/engine/src/audit/families/flows.ts |

## Clearing

| requirement | status | where / why |
|---|---|---|
| `Clearing A1` | MET | packages/engine/src/clearing/market.ts |
| `Clearing A2` | PARTIAL | packages/engine/src/clearing/solver.ts (the solver takes schedules and every venue posts them). What is missing is on the PARTICIPANT side: A2.a says a market expressed as "here is the quantity I want" has no level, only a shape, and forces every venue to invent its own rule — and seventeen sites across nine modules post exactly that. `resolveMarketOrders` IS the invented rule and now says so where it lives (13b.1-10, positioned to 14 and 16) |
| `Clearing A3` | MET | packages/engine/src/mechanisms/banks/dealing.ts, packages/engine/src/mechanisms/sovereign-curve/index.ts, packages/engine/src/world/context.ts, packages/engine/src/world/module.ts |
| `Clearing A4` | PARTIAL | packages/engine/src/clearing/solver.ts, packages/engine/src/world/context.ts (no schedule is written against the clearing price, and a phase reading a print not yet produced throws). But an order with NO level is a price-taker of a price this mechanism has not yet produced, and a big enough one is the marginal order that sets it — the central bank bidding at the top of a sovereign book for a quarter of the line (13b.1-10, positioned to 14) |
| `Clearing B1` | MET | packages/engine/src/clearing/market.ts |
| `Clearing B2` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/mechanisms/banks/dealing.ts, packages/engine/src/mechanisms/sovereign-curve/index.ts, packages/engine/src/world/module.ts |
| `Clearing B3` | PARTIAL | no dealer exists yet (worklist 9) |
| `Clearing B4` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts, packages/engine/src/mechanisms/banks/dealing.ts |
| `Clearing B5` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing C1` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing C2` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing C3` | MET | packages/engine/src/clearing/solver.ts, packages/engine/src/core/tick.ts (a pro-rata split is in whole pieces, summing to exactly the whole, with the odd piece to a named claimant), packages/engine/src/clearing/market.ts (two parties exchange at the grain both can hold) |
| `Clearing C4` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing C4.c` | MET | packages/engine/src/clearing/solver.ts, packages/engine/src/clearing/market.ts (every limit reaching the book is on that market's own tick, so the level the solver picks is a posted level that is on the grid by construction — nothing rounds the print) |
| `Clearing C5` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing D1` | MET | packages/engine/src/clearing/solver.ts, packages/engine/src/prices/price-store.ts |
| `Clearing D2` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/clearing/solver.ts |
| `Clearing D3` | MET | packages/engine/src/clearing/market.ts |
| `Clearing D4` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/prices/price-store.ts, packages/engine/src/world/revalue.ts |
| `Clearing D5` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing E1` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/prices/price-store.ts |
| `Clearing E2` | MISSING |  |
| `Clearing E3` | PARTIAL | no dealer schedules yet (worklist 9) |
| `Clearing E4` | MET | packages/engine/src/audit/families/prices.ts, packages/engine/src/clearing/market.ts, packages/engine/src/prices/price-store.ts |
| `Clearing F1` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/prices/price-store.ts, packages/engine/src/world/world.ts |
| `Clearing F2` | MET | packages/engine/src/audit/families/prices.ts, packages/engine/src/clearing/market.ts, packages/engine/src/prices/price-store.ts |
| `Clearing F3` | PARTIAL | to be measured once markets read period state (Part XII) |

## Audit

| requirement | status | where / why |
|---|---|---|
| `Audit A1` | MET | packages/engine/src/audit/audit.ts, packages/engine/src/audit/view.ts |
| `Audit A2` | MET | packages/engine/src/audit/audit.ts |
| `Audit A3` | MET | packages/engine/src/audit/audit.ts |
| `Audit A4` | MET | packages/engine/src/audit/audit.ts, packages/engine/src/core/num.ts |
| `Audit B1` | MET | packages/engine/src/audit/families/money.ts |
| `Audit B2` | MET | packages/engine/src/audit/families/ownership.ts |
| `Audit B3` | MET | packages/engine/src/audit/families/prices.ts, packages/engine/src/prices/value.ts |
| `Audit B4` | PARTIAL | the crossMarket family is declared and reports NOT BUILT, never green. It becomes buildable when one economic thing is reached two ways: a share and the index containing it, at worklist 12 (Indices); a future and its underlying, at worklist 13c; a bond and its derived spread is already the curve's own read |
| `Audit B5` | MET | packages/engine/src/audit/families/accounts.ts (`balanceSheet` is the ONE read: the audit checks it, a report publishes it and the seed states the opening account as it), packages/engine/src/ledger/settlement.ts, packages/engine/src/register/register.ts, packages/engine/src/world/revalue.ts, packages/engine/test/balance-identity.test.ts (every party, a year, four moneys that are not the same size) |
| `Audit B6` | MET | packages/engine/src/audit/families/names.ts |
| `Audit B7` | MET | packages/engine/src/audit/families/flows.ts |
| `Audit B8` | PARTIAL | independence is measured once a defect can light families (Part XII) |
| `Audit C1` | MET | packages/engine/src/audit/audit.ts, packages/engine/src/world/world.ts |
| `Audit C2` | MET | packages/engine/src/audit/audit.ts, packages/engine/src/audit/memory.ts, packages/engine/src/world/world.ts |
| `Audit C3` | MET | packages/engine/src/audit/audit.ts, packages/engine/src/world/world.ts |
| `Audit C4` | MET | packages/engine/src/audit/audit.ts, packages/engine/src/audit/view.ts |
| `Audit D1` | MET | packages/engine/src/audit/audit.ts |
| `Audit D2` | MET | packages/engine/src/audit/audit.ts |
| `Audit D3` | MET | packages/engine/src/audit/audit.ts, packages/engine/src/rng/prng.ts |
| `Audit D4` | PARTIAL | run-length comparison is a Part XII measurement |
| `Audit E1` | MET | packages/engine/src/audit/audit.ts, packages/engine/src/audit/families/unbuilt.ts |
| `Audit E2` | MET | packages/engine/src/audit/audit.ts |
| `Audit E3` | MET | packages/engine/src/audit/audit.ts, packages/engine/src/audit/families/unbuilt.ts |

## Seed

| requirement | status | where / why |
|---|---|---|
| `Seed A1` | MET | packages/engine/src/seeds/foundation.ts, packages/engine/src/world/assemble.ts, packages/engine/src/world/context.ts |
| `Seed A2` | MET | packages/engine/src/world/assemble.ts |
| `Seed A3` | MET | packages/engine/src/seeds/foundation.ts |
| `Seed A4` | MET | packages/engine/src/seeds/foundation.ts, packages/engine/src/world/context.ts |
| `Seed A5` | MET | packages/engine/src/rng/prng.ts, packages/engine/src/seeds/foundation.ts, packages/engine/src/world/assemble.ts, packages/engine/src/world/world.ts |
| `Seed B1` | MET | packages/engine/src/seeds/foundation.ts, packages/engine/src/mechanisms/firms/data.ts (three firms in each of the three lines, two banks, and populations as cells whose weights sum to what they stand for). The central bank and the treasury are one each because that is what they are, not a sample of one |
| `Seed B2` | MET | packages/engine/src/parties/party.ts, packages/engine/src/seeds/foundation.ts |
| `Seed B3` | MET | packages/engine/src/parties/party.ts, packages/engine/src/registry/registry.ts, packages/engine/src/seeds/foundation.ts |
| `Seed B4` | MET | packages/engine/src/seeds/foundation.ts (no two firms in a line open with the same stock or the same cash), packages/engine/src/mechanisms/firms/data.ts (and none of them takes the same hours to a tonne, so no two bid the same wage). The household cells open IDENTICAL on purpose and their dispersion is produced rather than stated — the memory each draws at entry, and who was hired at what wage (packages/engine/test/world.test.ts): stating a size distribution for them would be seeding an outcome (E1) |
| `Seed B5` | MISSING |  |
| `Seed C1` | MET | packages/engine/src/world/assemble.ts (the opening equity IS the balance sheet the audit checks it against, not a second copy of it) |
| `Seed C2` | MET | packages/engine/src/seeds/foundation.ts |
| `Seed C3` | MET | packages/engine/src/seeds/foundation.ts |
| `Seed C4` | MET | packages/engine/src/seeds/foundation.ts (a stated opening level is a price and sits on the same grid as one, so the level the world opens at is one its market could print again) |
| `Seed C5` | MISSING |  |
| `Seed D1` | MET | packages/engine/src/seeds/foundation.ts (a coupon the treasury can pay, a stock of every good at what it cost, and one lead time of work in progress on every line, so period one is the line running and not the line starting). Employment is deliberately NOT seeded: a seeded row needs a seeded wage, which is a price nobody cleared (Law 3). The venue strikes the first rows in period two and the seeded stock is what carries the lines until it does |
| `Seed D2` | MISSING |  |
| `Seed D3` | MISSING |  |
| `Seed D4` | MET | packages/engine/test/world.test.ts (with nothing shocking it the world still moves: cells that opened identical hold different cash within eight periods, because each remembers at its own speed and they were not all hired) |
| `Seed E1` | MISSING |  |
| `Seed E2` | MET | packages/engine/src/seeds/foundation.ts (it states technology — the recipes, the lead times, the yields — and endowments of stock; what is produced, what is paid and who holds what afterwards are all outcomes) |
| `Seed E3` | MISSING |  |

## Currency

| requirement | status | where / why |
|---|---|---|
| `Currency A1` | MET | packages/engine/src/seeds/foundation.ts (four moneys, each a named central bank’s liability), packages/engine/src/registry/registry.ts |
| `Currency A2` | MET | packages/engine/src/audit/families/names.ts, packages/engine/src/registry/registry.ts |
| `Currency A3` | MET | packages/engine/src/seeds/foundation.ts (each money is its own unit with its own smallest piece), packages/engine/src/registry/grid.ts |
| `Currency A4` | MET | packages/engine/src/registry/registry.ts, packages/engine/src/seeds/foundation.ts |
| `Currency A5` | MET | packages/engine/src/seeds/foundation.ts (the closed named set: USD, EUR, GBP, JPY, each with its issuer, its region and its unit) |
| `Currency B1` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts (a party short of a money it does not issue buys it in the pair between that money and its own) |
| `Currency B2` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts (and a party holding one it has no use for sells it there, which is the same read from the other end) |
| `Currency B3` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts (the dealer: its position in a pair is what it holds of the money that is not its own) |
| `Currency B4` | MET | packages/engine/src/mechanisms/money-market/index.ts (a central bank lends to its own system; a bank short of a foreign money has no window and must buy it) |
| `Currency B5` | MET | packages/engine/src/mechanisms/spot-fx/index.ts (every pair is a market, opened at the seed from the currencies themselves) |
| `Currency C1` | MET | packages/engine/src/clearing/market.ts (an fx trade is two money legs in two currencies at the rate the session struck) |
| `Currency C2` | MET | packages/engine/src/clearing/market.ts (both legs settle or neither does: an instruction applies whole or not at all) |
| `Currency C3` | MET | packages/engine/src/mechanisms/spot-fx/arbitrage.ts (the triangle closes because a desk takes the round trip, never because anything enforces an identity), packages/engine/src/mechanisms/spot-fx/family.ts |
| `Currency C4` | MET | packages/engine/src/seeds/foundation.ts (a coupon in a money its receiver does not book in is a real payment: the foreign lines pay to holders abroad) |
| `Currency C4.a` | MET | packages/engine/src/ledger/settlement.ts (what the payment settles in is the leg’s own currency, at the account that money sits in for each side) |
| `Currency C5` | MET | packages/engine/src/prices/value.ts (rateInForce: ONE rate for a period — what a payment settles at and what a balance sheet is valued at are the same number), packages/engine/src/world/context.ts |
| `Currency D1` | MET | packages/engine/src/prices/value.ts (everything inside a period values at the rate the period opened with; the revaluation at the close brings the books to the rate this period’s session struck) |
| `Currency D2` | MET | packages/engine/src/world/revalue.ts (every position in a money that is not its holder’s own is revalued to the holder’s equity, and the mark is converted into that money on the way — `intoOwnMoney`, the same rate the balance sheet reads at) |
| `Currency D2.b` | MET | packages/engine/src/world/revalue.ts, packages/engine/src/register/register.ts (a central bank’s foreign reserves go to its revaluation account, never to what it earned) |
| `Currency D3` | MET | packages/engine/src/world/revalue.ts (the FX revaluation runs before the marks, so the two do not each claim the other’s move) |
| `Currency D4` | MET | packages/engine/src/audit/families/currency.ts (what every revaluation booked against what the period’s rate move on the positions revalued comes to, reached from the events rather than the register) |
| `Currency E1` | MET | packages/engine/src/observer/observer.ts (the rate of every pair, with whether the print is this period’s) |
| `Currency E2` | MISSING |  |
| `Currency E3` | MET | packages/engine/src/mechanisms/spot-fx/family.ts (the triangular gap per triple, reported only when it is bigger than the cheapest desk’s round trip) |
| `Currency E4` | MET | packages/engine/src/observer/observer.ts, packages/engine/src/world/revalue.ts (what the rate did to a holder is a journalled event with its own size) |

## Bond

| requirement | status | where / why |
|---|---|---|
| `Bond N1` | MET | packages/engine/src/register/instruments.ts |
| `Bond N2` | MET | packages/engine/src/register/instruments.ts |
| `Bond N3` | MET | packages/engine/src/register/instruments.ts |
| `Bond N4` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Bond N5` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Bond N6` | MET | packages/engine/src/calendar/daycount.ts, packages/engine/src/core/rate.ts, packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Bond N7` | MET | packages/engine/src/clearing/market.ts |
| `Bond N7.b` | MET | packages/engine/src/mechanisms/sovereign-curve/index.ts, packages/engine/src/prices/curve.ts |
| `Bond N8` | MISSING |  |
| `Bond N8.a` | MET | packages/engine/src/audit/families/ownership.ts |
| `Bond N9` | MET | packages/engine/src/clearing/market.ts |
| `Bond N10` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts, packages/engine/src/world/actions.ts |
| `Bond N11` | PARTIAL | the sovereign answers none, which is the right answer for it; the corporate regime — covenants, events of default, the trustee — arrives with Corporate Credit (worklist 13f) |
| `Bond N12` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts (missed payment only: there are no covenants to breach), packages/engine/src/world/actions.ts (the kernel asks the instrument's own profile after a payment it applied did not settle, and journals what it answers, publicly) |
| `Bond N13` | MET | packages/engine/src/registry/kinds.ts (every kind states what a holder is entitled to on failure — required, because the clause is about stating it even when the answer is nothing), packages/engine/src/mechanisms/sovereign-instruments/index.ts (nothing seizable: a negotiated exchange, and exclusion from the market) |
| `Bond N14` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts, packages/engine/src/register/instruments.ts |

## Derivative

| requirement | status | where / why |
|---|---|---|
| `Derivative D1` | MET | packages/engine/src/register/contracts.ts, packages/engine/src/registry/derivatives.ts, packages/engine/src/ledger/settlement.ts (a contract opens on two named books in one numbered instruction), packages/engine/src/audit/families/accounts.ts (an asset to one side and a liability to the other, at every instant), packages/engine/src/prices/contract-value.ts (a contract has exactly two sides, so a third party asking what it is worth to it is refused at the site — it used to be answered with 0, which is indistinguishable from a contract at par) |
| `Derivative D1.a` | MET | packages/engine/src/ledger/settlement.ts (a leg naming one side, or the same party twice, is refused at the site), packages/engine/src/audit/families/zero-sum.ts |
| `Derivative D1.b` | MET | packages/engine/src/audit/families/zero-sum.ts (the profile is asked for the contract as EACH side states it — `flip` — and the two must negate exactly; a negation the kernel performed itself would be checking a minus sign) |
| `Derivative D2` | MET | packages/engine/src/register/contracts.ts, packages/engine/src/registry/derivatives.ts (a notional in the kind’s own unit, positive, on the grid) |
| `Derivative D3` | MET | packages/engine/src/registry/derivatives.ts (a print this world clears, an index it reads, or a public event it records), packages/engine/src/world/world.ts (`missingUnderlying`) |
| `Derivative D3.a` | MET | packages/engine/src/world/world.ts, packages/engine/src/ledger/settlement.ts (a contract on something this world does not produce is refused when it is written) |
| `Derivative D4` | MET | packages/engine/src/registry/derivatives.ts (`legs`: what the terms put in this period, both directions) |
| `Derivative D5` | MET | packages/engine/src/register/contracts.ts, packages/engine/src/registry/derivatives.ts (the money its legs move in is the contract’s own; the equity effect converts at the rate in force) |
| `Derivative D6` | MET | packages/engine/src/registry/derivatives.ts (`expires`: the term runs out and the kind says when) |
| `Derivative D7` | MET | packages/engine/src/register/contracts.ts, packages/engine/src/registry/derivatives.ts (`struckAt`: the level the session cleared at), packages/engine/src/clearing/market.ts |
| `Derivative D8` | MET | packages/engine/src/prices/contract-value.ts (read at every ask, stored nowhere) |
| `Derivative D9` | MET | packages/engine/src/mechanisms/derivative-layer/margin.ts (posted as a claim, not spent) |
| `Derivative D10` | MET | packages/engine/src/mechanisms/derivative-layer/index.ts (a side can fail before the term ends; what the survivor then has is a claim on an estate, not a payoff) |
| `Derivative D11` | MET | packages/engine/src/mechanisms/derivative-layer/index.ts (`settleAndTearUp`: it ceases on both books at once), packages/engine/src/ledger/settlement.ts |
| `Derivative D12` | MET | packages/engine/src/register/contracts.ts, packages/engine/src/registry/derivatives.ts (counterparties + underlying + term + strike; two strikes are two rows and an offsetting trade with somebody else is a third) |
| `Derivative X1` | MET | packages/engine/src/register/contracts.ts, packages/engine/src/registry/derivatives.ts (a second register: no issuer, no issued amount, no ownership check) |
| `Derivative X2` | MET | packages/engine/src/mechanisms/derivative-layer/margin.ts (the premium and the margin are real money out of a real account, and a payment that cannot be made fails) |
| `Derivative X3` | MET | packages/engine/src/registry/derivatives.ts (a mark reads prints the rest of the world cleared; nothing prices the underlying off the contract) |

## Corporate Credit

| requirement | status | where / why |
|---|---|---|
| `Corporate Credit A1` | MISSING |  |
| `Corporate Credit A2` | MISSING |  |
| `Corporate Credit A2.c` | MISSING |  |
| `Corporate Credit A3` | MISSING |  |
| `Corporate Credit A4` | MISSING |  |
| `Corporate Credit B1` | MISSING |  |
| `Corporate Credit B2` | MISSING |  |
| `Corporate Credit B3` | MISSING |  |
| `Corporate Credit B4` | MISSING |  |
| `Corporate Credit C1` | MISSING |  |
| `Corporate Credit C2` | MISSING |  |
| `Corporate Credit C3` | MISSING |  |
| `Corporate Credit C4` | MISSING |  |
| `Corporate Credit C5` | MISSING |  |
| `Corporate Credit C6` | MISSING |  |
| `Corporate Credit C7` | MISSING |  |
| `Corporate Credit C7.b` | MISSING |  |
| `Corporate Credit C8` | MISSING |  |
| `Corporate Credit C9` | MISSING |  |
| `Corporate Credit C10` | MISSING |  |
| `Corporate Credit C10.b` | MISSING |  |
| `Corporate Credit C10.c` | MISSING |  |
| `Corporate Credit C11` | MISSING |  |
| `Corporate Credit C11.d` | MISSING |  |
| `Corporate Credit C11.e` | MISSING |  |
| `Corporate Credit D1` | MISSING |  |
| `Corporate Credit D2` | MISSING |  |
| `Corporate Credit D3` | MISSING |  |
| `Corporate Credit D4` | MISSING |  |
| `Corporate Credit D5` | MISSING |  |
| `Corporate Credit D6` | MISSING |  |
| `Corporate Credit D7` | MISSING |  |
| `Corporate Credit D8` | MISSING |  |
| `Corporate Credit E1` | MISSING |  |
| `Corporate Credit E2` | MISSING |  |
| `Corporate Credit E3` | MISSING |  |
| `Corporate Credit E4` | MET | packages/engine/src/world/revalue.ts |
| `Corporate Credit E5` | PARTIAL | packages/engine/src/mechanisms/banks/quote.ts (a bank's reservation is its own blended cost of funds and the capital the position consumes, published under its own name and read by its schedules), packages/engine/src/mechanisms/funds/index.ts (a fund's is what its own investors require of it, under its mandate), packages/engine/src/mechanisms/households/portfolio.ts (a cell's is its own patience). E5.b's EXPECTED LOSS is the one term still missing for a holder: it needs A4's assessment, and an assessment is an opinion somebody holds — the ratings system and the second opinion, which is worklist 12 |
| `Corporate Credit E5.d` | PARTIAL | every schedule in a bond market is now built from what its own poster requires, so the level a book clears at IS where the marginal holder sits and nothing floors it (packages/engine/src/mechanisms/sovereign-curve/index.ts). Measuring that it is, over a run, is Part XII (worklist 16) |
| `Corporate Credit E6` | MISSING |  |
| `Corporate Credit E7` | MISSING |  |
| `Corporate Credit E8` | MISSING |  |
| `Corporate Credit E9` | MISSING |  |
| `Corporate Credit F1` | MISSING |  |
| `Corporate Credit F2` | MISSING |  |
| `Corporate Credit F3` | MISSING |  |
| `Corporate Credit F4` | MISSING |  |
| `Corporate Credit F5` | MISSING |  |
| `Corporate Credit F6` | MISSING |  |
| `Corporate Credit G1` | PARTIAL | packages/engine/src/world/actions.ts (a missed payment is an event, public, so a holder observes it rather than inferring it from the issuer's accounts). A breached covenant needs covenants, which arrive with corporate paper (worklist 13f) |
| `Corporate Credit G2` | MET | packages/engine/src/world/actions.ts (a default on one line makes the issuer's others due where their own terms say so, through the same path a maturity takes, so an issuer that cannot pay the accelerated face fails that too), packages/engine/test/credit-events.test.ts |
| `Corporate Credit G3` | MISSING |  |
| `Corporate Credit G4` | MET | packages/engine/src/mechanisms/estate/index.ts (the estate is realised into the markets those things trade in, at what bidders pay: there is no formula discount to book anywhere in the path) |
| `Corporate Credit G5` | MET | packages/engine/src/mechanisms/estate/index.ts (senior in full first, then the next rank, by the instrument's own ranking; G5.a: a junior claim recovers nothing when the senior rank exhausts the proceeds, and packages/engine/test/estate.test.ts asserts it) |
| `Corporate Credit G6` | PARTIAL | packages/engine/test/credit-events.test.ts holds the identity — a claim leaving the book at what it fetched moves the holder's equity by exactly what it was carrying, and the issuer's by the same the other way. What a recovery IS needs an estate that realises something (worklist 7) |
| `Corporate Credit G7` | MISSING |  |
| `Corporate Credit G8` | MISSING |  |
| `Corporate Credit H1` | MISSING |  |
| `Corporate Credit H2` | MISSING |  |
| `Corporate Credit H3` | MISSING |  |
| `Corporate Credit H4` | MISSING |  |
| `Corporate Credit H4.a` | MISSING |  |

## Sovereign

| requirement | status | where / why |
|---|---|---|
| `Sovereign A1` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Sovereign A1.c` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Sovereign A2` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Sovereign A3` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Sovereign A3.b` | MET | packages/engine/src/mechanisms/treasury/index.ts, packages/engine/src/registry/profiles.ts |
| `Sovereign A4` | MISSING |  |
| `Sovereign B1` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign B2` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign B3` | MISSING |  |
| `Sovereign B4` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign B5` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign B6` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign B7` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign C1` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/mechanisms/treasury/index.ts |
| `Sovereign C2` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/clearing/solver.ts, packages/engine/src/mechanisms/banks/dealing.ts |
| `Sovereign C3` | MET | packages/engine/src/mechanisms/banks/dealing.ts |
| `Sovereign C4` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/mechanisms/banks/dealing.ts |
| `Sovereign C5` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/mechanisms/treasury/index.ts |
| `Sovereign C6` | MET | packages/engine/src/clearing/market.ts |
| `Sovereign C7` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/mechanisms/treasury/index.ts |
| `Sovereign D1` | MET | packages/engine/src/mechanisms/sovereign-curve/index.ts |
| `Sovereign D2` | MET | packages/engine/src/mechanisms/sovereign-curve/index.ts, packages/engine/src/prices/curve.ts |
| `Sovereign D3` | MET | packages/engine/src/mechanisms/sovereign-curve/index.ts, packages/engine/src/observer/observer.ts, packages/engine/src/prices/curve.ts |
| `Sovereign D4` | MET | packages/engine/src/mechanisms/sovereign-curve/index.ts, packages/engine/src/prices/curve.ts |
| `Sovereign D5` | MET | packages/engine/src/mechanisms/sovereign-curve/index.ts |
| `Sovereign D6` | PARTIAL | the bid-offer is whatever the schedules produce; there are no dealers to produce one until worklist 9 |
| `Sovereign E1` | MET | packages/engine/src/mechanisms/sovereign-curve/index.ts |
| `Sovereign E1.a` | MET | packages/engine/src/mechanisms/sovereign-curve/index.ts |
| `Sovereign E2` | PARTIAL | banks hold for the liquidity buffer, the central bank for policy and households directly out of what they save (packages/engine/src/mechanisms/households/portfolio.ts); funds arrive at worklist 8 and foreign holders at 12 |
| `Sovereign E3` | MET | packages/engine/src/mechanisms/sovereign-curve/index.ts |
| `Sovereign E4` | MET | packages/engine/src/mechanisms/sovereign-curve/index.ts |
| `Sovereign E5` | MET | packages/engine/src/mechanisms/banks/dealing.ts, packages/engine/src/mechanisms/sovereign-curve/index.ts |
| `Sovereign F1` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign F2` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign F3` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign F4` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Sovereign F5` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Sovereign G1` | MISSING |  |
| `Sovereign G2` | MISSING |  |
| `Sovereign G3` | PARTIAL | packages/engine/src/mechanisms/sovereign-instruments/index.ts states both halves — there is no estate (the claim is a negotiated exchange, nothing seizable) and a missed payment on one line does not accelerate the others. The negotiation itself is an exchange offer with holdouts (G4, worklist 13f) |
| `Sovereign G4` | MISSING |  |
| `Sovereign G5` | MISSING |  |
| `Sovereign H1` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Sovereign H2` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Sovereign H3` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Sovereign H4` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Sovereign H5` | MISSING |  |
| `Sovereign I1` | MET | packages/engine/src/mechanisms/bond-futures/index.ts (a named deliverable line, per unit of face, delivered against cash in one instruction) |
| `Sovereign I1.a` | MET | packages/engine/src/mechanisms/bond-futures/index.ts (`bondCarryOf`, `netBasis`: the coupon and the financing, both read; measured, never set; published through the kind's own `measures` so a reader sees it beside every other basis) |
| `Sovereign I2` | MET | packages/engine/src/mechanisms/bond-futures/index.ts (`futureOrders`: a holder short of the future, a party without duration long of it) |
| `Sovereign I3` | MISSING |  |
| `Sovereign I3.a` | MET | packages/engine/src/mechanisms/bond-futures/index.ts (cut on a drawdown against its own tolerance, with nothing making it whole) |

## Short-Term Debt

| requirement | status | where / why |
|---|---|---|
| `Short-Term Debt A1` | MISSING |  |
| `Short-Term Debt A2` | MISSING |  |
| `Short-Term Debt A3` | MISSING |  |
| `Short-Term Debt B1` | MISSING |  |
| `Short-Term Debt B2` | MISSING |  |
| `Short-Term Debt B3` | MISSING |  |
| `Short-Term Debt B4` | MISSING |  |
| `Short-Term Debt B5` | MISSING |  |
| `Short-Term Debt C1` | MISSING |  |
| `Short-Term Debt C2` | MISSING |  |
| `Short-Term Debt C3` | MISSING |  |
| `Short-Term Debt C4` | MISSING |  |
| `Short-Term Debt D1` | MISSING |  |
| `Short-Term Debt D2` | MISSING |  |
| `Short-Term Debt D3` | MISSING |  |
| `Short-Term Debt D4` | MISSING |  |
| `Short-Term Debt E1` | MISSING |  |
| `Short-Term Debt E2` | MISSING |  |
| `Short-Term Debt E3` | MISSING |  |

## Equity

| requirement | status | where / why |
|---|---|---|
| `Equity A1` | MET | packages/engine/src/mechanisms/equity/share.ts (a residual claim and not a liability of its issuer, ranking below everything it owes; A1.b: it is written off at zero and never below, because limited liability means a holder cannot be asked for more) |
| `Equity A2` | MET | packages/engine/src/mechanisms/equity/share.ts, packages/engine/src/registry/profiles.ts (counted in `shares`, a unit that is not money and is never added to one); A2.a: the count moves only by an issuance, a buyback the kernel extinguishes on arrival, or a split (packages/engine/src/register/instruments.ts) |
| `Equity A3` | MET | packages/engine/src/mechanisms/equity/index.ts (quoted in the issuer's own money, and its market clears in that currency) |
| `Equity A4` | MET | packages/engine/src/mechanisms/equity/share.ts (perpetual: no maturity, nothing due, no cash flows — so there is nothing in this world that COULD discount it, which is how B3 is kept rather than policed) |
| `Equity A5` | MET | packages/engine/src/mechanisms/equity/share.ts, packages/engine/test/equity.test.ts (a vote per share, and a cell casts weight × its member's votes, so a represented holder is not disenfranchised by its representation) |
| `Equity A5.b` | MISSING | control has a value distinct from the cash flows only where a takeover pays for it, and there is no tender market yet: worklist 13g |
| `Equity A6` | MET | packages/engine/src/mechanisms/equity/index.ts, packages/engine/src/mechanisms/equity/share.ts (the issuer names the line, as a market names a share; the internal id is never the display name, Law 9) |
| `Equity B1` | MET | packages/engine/src/mechanisms/households/portfolio.ts (a holder or buyer posts its own schedule, off its own opinion and its own budget, and who trades is the outcome), packages/engine/src/mechanisms/banks/dealing-quote.ts (and a desk posts two out of its own state) |
| `Equity B2` | MET | packages/engine/src/mechanisms/equity/index.ts, packages/engine/src/clearing/market.ts (one cleared price per line per period, out of the same solver as every other market) |
| `Equity B3` | MET | packages/engine/src/mechanisms/equity/share.ts (there is no multiple, discounted cash flow or target anywhere: the kind promises nothing dated, so nothing could discount it, and the price is what the session made of the schedules). An opinion is a participant's own and enters its schedule (packages/engine/src/mechanisms/households/portfolio.ts: a saver prices the claim off what the company itself published it owns net of what it owes, plus what it earns on that, at what THAT cell requires — so two cells want different prices for one firm), packages/engine/test/equity-anchor.test.ts |
| `Equity B4` | MET | packages/engine/src/mechanisms/equity/opinion.ts (shares times price, read in one place so nobody derives it a second way); B4.a: nothing compares it against shares times price, because the read is the only writer of it |
| `Equity B4.a` | MISSING |  |
| `Equity B5` | MET | packages/engine/src/mechanisms/banks/dealing.ts, packages/engine/src/mechanisms/banks/dealing-quote.ts (a desk quotes both sides out of its own inventory and its own capital, and what it earns is the width it quoted less what the inventory did) |
| `Equity B6` | MET | packages/engine/test/equity.test.ts (a seller with no buyer keeps its shares: the session prints `noOverlap` and nothing moves. There is no invisible bid because the only bids are the ones somebody posted) |
| `Equity C1` | MET | packages/engine/src/register/register.ts (who holds how many, indexed both ways, with lots and liens like any other holding). C1.b: the free float is a READ of what is bound — units under a lien cannot move and packages/engine/src/mechanisms/equity/index.ts reads them off the register rather than storing a float; insiders and strategic holders whose stake is not for sale are worklist 13g, so until there is one the read is honestly zero |
| `Equity C1.a` | MET | packages/engine/src/audit/families/ownership.ts |
| `Equity C2` | PARTIAL | packages/engine/src/mechanisms/households/portfolio.ts (C2.a: households directly, out of one budget and their own opinion), packages/engine/src/mechanisms/funds/etf.ts (C2.c: an index fund that does not price at all — it holds weight, whatever it costs). C2.b's institutions with mandates is so far that one fund (insurers and the rest: 13h); C2.d's treasury shares are not representable while a share arriving at its issuer is extinguished (packages/engine/src/ledger/settlement.ts), which is what makes D2.a's count fall; C2.e's insiders are 13g |
| `Equity C3` | MET | packages/engine/src/prices/value.ts |
| `Equity C4` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/world/revalue.ts |
| `Equity C5` | MISSING | a leveraged holder that funds its position and can be forced to sell needs margin and somebody lending against shares: worklist 13f |
| `Equity C6` | PARTIAL | packages/engine/src/register/register.ts (a lien binds units, only free units move, and packages/engine/src/mechanisms/equity/index.ts reads the bound ones). Nothing yet pledges a share or lends one: collateral against a loan and securities lending are worklist 13f |
| `Equity C7` | MISSING | a short is a borrow with a real cost and a real squeeze: worklist 13f |
| `Equity D1` | MET | packages/engine/src/mechanisms/equity/decide.ts, packages/engine/src/mechanisms/equity/index.ts (a primary offer with a size and a reservation; D1.a: the count rises and each claim shrinks; D1.b: the reason is a funding need it prefers to meet with equity, read off its own funding gap; D1.c: the session prices it and it can fail), packages/engine/test/equity.test.ts |
| `Equity D2` | MET | packages/engine/src/mechanisms/equity/decide.ts, packages/engine/src/mechanisms/equity/index.ts (it bids in its own line at its own reservation and takes what the session gives it, which may be nothing; D2.a: what it buys is extinguished on arrival, so the count falls and each remaining claim grows; D2.b: the cash is gone; D2.c: it is the same money the dividend would have been, and never both in one period) |
| `Equity D3` | MET | packages/engine/src/mechanisms/equity/decide.ts, packages/engine/src/mechanisms/equity/index.ts (cash per share to whoever the register says holds one; D3.a: it leaves the firm and arrives at named holders, per member for a cell; D3.b: the number is the decision, and a firm with less to spare declares less — which is the cut others read) |
| `Equity D4` | MET | packages/engine/src/register/register.ts, packages/engine/src/register/instruments.ts, packages/engine/src/prices/price-store.ts (the count, the lots' basis, the liens and the print all rebase in one door, and nothing else moves), packages/engine/test/equity.test.ts |
| `Equity E1` | MISSING | worklist 13g |
| `Equity E2` | MISSING | worklist 13g |
| `Equity E3` | MISSING | worklist 13g |
| `Equity E4` | MET | packages/engine/src/mechanisms/estate/index.ts (a share ranks last, so the waterfall reaches it only if every other claim was paid in full; when it does not, the register goes to zero rather than to a recovery — and it is the ranking that does it, not a special case), packages/engine/test/equity.test.ts |
| `Equity F1` | MET | packages/engine/src/mechanisms/equity/index.ts (the declared dividend, to the holders the register says held on the day) |
| `Equity F2` | MET | packages/engine/src/mechanisms/estate/index.ts, packages/engine/test/equity.test.ts (the residual on wind-up, after every other claim, through the same waterfall as everything else) |
| `Equity F3` | MET | packages/engine/src/mechanisms/equity/share.ts (a vote, per share, and a cell's is its whole weight) |
| `Equity F4` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/registry/kinds.ts |
| `Equity G1` | MISSING | an index of real prices and real free-float weights is worklist 12 |
| `Equity G2` | MISSING | worklist 12 |
| `Equity G3` | MISSING |  |

## Money Market

| requirement | status | where / why |
|---|---|---|
| `Money Market A1` | MET | packages/engine/src/mechanisms/money-market/session.ts (a bank position is the sum of the wire own reserve legs for the period: nobody chose it) |
| `Money Market A2` | MET | packages/engine/src/mechanisms/money-market/deposits.ts (`bufferOf`: the worst week this bank own account has had, over the memory it keeps — derived from what it saw, never a ratio) |
| `Money Market A2.b` | MET | packages/engine/src/mechanisms/money-market/index.ts, packages/engine/src/mechanisms/banks/dealing.ts (missing the buffer costs it what the window charges, or what its paper fetches when it has to sell) |
| `Money Market A3` | MET | packages/engine/src/mechanisms/money-market/index.ts (the session is anchored after every market and every payment: the need is not knowable before them) |
| `Money Market B1` | MET | packages/engine/src/mechanisms/money-market/session.ts (every bank posts a schedule out of its own position; who lends and who borrows is the fill) |
| `Money Market B2` | MET | packages/engine/src/mechanisms/money-market/collateral.ts (`nameCost`: what THIS lender believes an unsecured claim on that name costs it, published by its own module) |
| `Money Market B2.b` | PARTIAL | the prints are per NAME per book (packages/engine/src/mechanisms/money-market/index.ts), so the spread between the strongest and the weakest is readable; measuring it is Part XII (worklist 16) |
| `Money Market B3` | MET | packages/engine/src/mechanisms/money-market/collateral.ts, packages/engine/src/mechanisms/money-market/rows.ts (secured lending prices the paper at the lender own required yield, and the lien is real) |
| `Money Market B4` | MET | packages/engine/src/mechanisms/money-market/session.ts (`strike`: the rate is what cleared, lenders undercutting each other) |
| `Money Market B5` | MET | packages/engine/src/mechanisms/money-market/index.ts (a money fund with spare cash is in the same session, with the floor as its alternative) |
| `Money Market B6` | MET | packages/engine/src/mechanisms/money-market/data.ts (overnight and term, secured and unsecured: four books) |
| `Money Market B6.a` | PARTIAL | both books print, so the gap between them exists to be read; reading it as a measure of expected stress is Part XII (worklist 16) |
| `Money Market B7` | MET | packages/engine/src/mechanisms/money-market/index.ts (`moneyMarket.refused`, per name, public, with what it was short of) |
| `Money Market C1` | MET | packages/engine/src/mechanisms/money-market/index.ts (`parkTheRest`: the money leg pays the issuer of the money, so the reserves are destroyed where they land — C1.a) |
| `Money Market C2` | MET | packages/engine/src/mechanisms/money-market/session.ts (`windowOffer`: the standing facility takes its seat in every session at the ceiling) |
| `Money Market C3` | MET | packages/engine/test/money-market.test.ts (every rate the session printed sits between the two administered levels, in worlds three points of policy apart) |
| `Money Market C4` | MET | packages/engine/src/mechanisms/money-market/collateral.ts (`windowAdvances`: the window lends against unencumbered eligible paper at the haircut it declared, and a bank out of it cannot draw) |
| `Money Market C5` | MET | packages/engine/src/mechanisms/money-market/index.ts (`reserveOverdraft`: collateralised, priced at the ceiling plus a penalty, refused to the insolvent — the FORBID holds because all four conditions are there) |
| `Money Market D1` | MET | packages/engine/src/mechanisms/banks/dealing.ts (it sells its free eligible paper at whatever the book gives), packages/engine/src/mechanisms/banks/quote.ts (and writes no new business while it has no room) |
| `Money Market D2` | MET | packages/engine/src/mechanisms/money-market/deposits.ts (`setRates`, `worthOfMoney`: it pays up when paying up is the cheaper answer, and its depositors answer the rate) |
| `Money Market D3` | MET | packages/engine/src/mechanisms/money-market/index.ts (`bookOverdrafts`: what it drew is a row at the window rate plus the penalty by the close) |
| `Money Market D4` | MET | packages/engine/src/world/failure.ts (two triggers, asked of the kind own profile), packages/engine/src/mechanisms/money-market/resolution.ts (and the resolution says which one fired) |
| `Money Market D5` | MET | packages/engine/src/mechanisms/money-market/deposits.ts (`looksInTrouble`, `moveDeposits`: depositors move on what was published about the bank, and only on that) |
| `Money Market D5.b` | MET | packages/engine/test/run.test.ts (the deposit leaves with the reserves behind it, in the same instruction, and the bank is shorter at the next close) |
| `Money Market D6` | MET | packages/engine/src/mechanisms/money-market/index.ts (`reserveOverdraft`: freely, against good collateral, at a penalty, to the solvent) |
| `Money Market E1` | MET | packages/engine/test/money-market.test.ts (three points of policy moves what the session strikes and what a bank charges a borrower, and no cleared rate is the policy rate) |
| `Money Market E2` | PARTIAL | the channel is there and tested one step at a time (a squeeze raises what a bank pays for deposits and what it quotes a borrower); the SCALAR-versus-channel measurement across a shock is Part XII (worklist 16) |
| `Money Market E3` | MET | packages/engine/src/mechanisms/money-market/resolution.ts, packages/engine/test/bank-resolution.test.ts (a failed bank losses land on the banks that funded it, by name, junior money first) |

## Spot FX

| requirement | status | where / why |
|---|---|---|
| `Spot FX A1` | MET | packages/engine/src/clearing/market.ts (a spot trade is two money legs at the rate the session struck; there is no asset and no issuer) |
| `Spot FX A2` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/ledger/settlement.ts (both legs settle or neither does — the atomicity an instruction already has, which is Herstatt) |
| `Spot FX A3` | MET | packages/engine/src/core/ids.ts (a pair is named as a market names one), packages/engine/src/world/world.ts (a pair against itself, or priced in a third money, is refused at assembly) |
| `Spot FX B1` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts (a party short of a money it has to pay posts a size and no level) |
| `Spot FX B2` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts (and one holding a money nothing it owes is in sells it — the same signed read, `owedIn`) |
| `Spot FX B3` | MET | packages/engine/src/world/world.ts (owedIn: what a party’s own liabilities say falls due in a money, less what it holds of it) |
| `Spot FX B4` | PARTIAL | packages/engine/src/mechanisms/spot-fx/index.ts — the participants are the kernel’s kinds plus the banks’ desks. A party whose reason to be in a pair is a VIEW of the rate is 13h’s hedge fund; nothing here speculates on a currency |
| `Spot FX B5` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts (the desk quotes both ways, its edge is its own, and its spread is twice that rather than a width anybody stated) |
| `Spot FX B6` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts (a desk cannot sell a money it has not got and cannot pay with one it has not got — arithmetic, not a limit) |
| `Spot FX C1` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/prices/price-store.ts (the rate is a PRINT of the pair’s own session, in the one price store), packages/engine/src/registry/registry.ts (`rateTickFor`: a rate moves in pips, and a pip belongs to the money it is quoted in) |
| `Spot FX C2` | MET | packages/engine/src/mechanisms/spot-fx/arbitrage.ts (the desk decides once and its three books read the decision back as ordinary orders) |
| `Spot FX C3` | MET | packages/engine/src/mechanisms/spot-fx/arbitrage.ts, packages/engine/src/mechanisms/spot-fx/family.ts (the gap closes to a desk’s own cost and no further, and a standing gap is measured rather than closed) |
| `Spot FX C4` | MET | packages/engine/src/clearing/market.ts (each leg lands on the smallest piece of its OWN money, and the two pieces are different sizes) |
| `Spot FX C5` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts (the spread is the desk’s edge either side, never a stated width) |
| `Spot FX C6` | MET | packages/engine/src/ledger/settlement.ts (neither side can be left having paid for money it did not get) |
| `Spot FX D1` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts, packages/engine/src/mechanisms/spot-fx/data.ts (what a desk will risk in a pair is its own share of its own capital) |
| `Spot FX D2` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts (past its own limit it stops quoting and takes the market: the rung its paper book already had) |
| `Spot FX D3` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts (what it can deliver and what it can pay with, both read off its own balances) |
| `Spot FX D4` | MET | packages/engine/src/mechanisms/spot-fx/participants.ts (its own position skews both sides, which is how order flow moves a rate with nobody deciding that it should) |
| `Spot FX D5` | MET | packages/engine/src/mechanisms/spot-fx/data.ts (every number in a desk’s quote is declared under that desk’s own name) |
| `Spot FX E1` | MET | packages/engine/src/observer/observer.ts (every pair’s rate, and whether it is this period’s print) |
| `Spot FX E2` | MET | packages/engine/src/journal/journal.ts, packages/engine/src/clearing/market.ts (a session that produced no trade says so and carries the last real price, visibly stale) |
| `Spot FX E3` | MET | packages/engine/src/mechanisms/spot-fx/family.ts, packages/engine/src/observer/observer.ts (the triangular gap as a standing measurement with an owner and a size) |
| `Spot FX E4` | MET | packages/engine/src/world/revalue.ts (what a rate move did to a holder is an event with its own size) |
| `Spot FX F1` | MET | packages/engine/src/world/world.ts (markets clear in declared order; a pair runs first) |
| `Spot FX F1.a` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/mechanisms/spot-fx/index.ts (the conversion is its own session with its own counterparty — never inside another trade) |
| `Spot FX F1.b` | MET | packages/engine/src/clearing/market.ts (a pair’s market order is the same order every other book takes) |

## Fund Shares

| requirement | status | where / why |
|---|---|---|
| `Fund Shares A1` | MET | packages/engine/src/mechanisms/funds/index.ts (a named party with an account and a register of holdings, like anything else) |
| `Fund Shares A2` | MET | packages/engine/src/mechanisms/funds/index.ts (its liability is its shares, counted in shares, held by named holders) |
| `Fund Shares A3` | MET | packages/engine/src/mechanisms/funds/index.ts (its equity is nothing, and nothing in the module enforces it: it falls out of the wire, and the audit family says whether the wire did it), packages/engine/test/funds.test.ts |
| `Fund Shares A4` | MET | packages/engine/src/mechanisms/funds/data.ts, packages/engine/src/mechanisms/funds/index.ts (the mandate is what it may hold and how long for, and the module never posts an order outside it — so a flow into it becomes a purchase of exactly what the mandate allows) |
| `Fund Shares B1` | MET | packages/engine/src/mechanisms/funds/nav.ts, packages/engine/src/prices/value.ts (assets at market minus liabilities over shares outstanding, read every time it is asked; there is no NAV series anywhere and nothing stores one) |
| `Fund Shares B2` | MET | packages/engine/src/mechanisms/funds/nav.ts (marked at cleared prices, and a holding nothing has ever priced makes the read fail rather than be guessed at); B2.a: the oldest mark travels with the value and a stale one is journalled as stale (packages/engine/src/mechanisms/funds/index.ts) |
| `Fund Shares B3` | MET | packages/engine/src/mechanisms/funds/index.ts (the fee is a real payment to the manager each period, for the days the period has, and the NAV read after it is lower by exactly that) |
| `Fund Shares B4` | MET | packages/engine/test/funds.test.ts (holders' share value against the fund's assets minus liabilities, asserted as arithmetic rather than checked against itself in the audit) |
| `Fund Shares C1` | MET | packages/engine/src/mechanisms/funds/index.ts (cash in and shares out in ONE instruction at the NAV struck; C1.a: the cash above its buffer is then posted per its mandate) |
| `Fund Shares C2` | MET | packages/engine/src/mechanisms/funds/index.ts (shares back and cash out at the NAV struck when it asked; C2.a: from its buffer or by selling; C2.b: what it cannot pay stays on the book and the sale is into the market that thing trades in, at whatever it gives) |
| `Fund Shares C3` | MET | packages/engine/src/mechanisms/funds/index.ts (shares outstanding move with every subscription and redemption; a fund is not fixed-size) |
| `Fund Shares C4` | MET | packages/engine/src/mechanisms/funds/index.ts (a queued redeemer is paid at the NAV it struck, and what the sales fetched is what the remaining holders carry through the same read — C4.a: which is why a redemption is a cost to those who stay) |
| `Fund Shares C5` | PARTIAL | packages/engine/src/mechanisms/funds/index.ts (the flows family checks that everything ever asked for is paid or still on the book, which is the half that breaks in silence). Shares created minus redeemed against outstanding is the kernel's ownership family already; cash in and out against the shares is the two legs of one instruction and is asserted in packages/engine/test/funds.test.ts rather than audited against itself. BOTH HALVES ARE CHECKED, by two families and a test rather than by one family, so what is open is not a mechanism but whether that counts: the measurement programme (worklist 16) decides whether a clause checked in two places needs a family of its own, and this row closes either way there |
| `Fund Shares D1` | MET | packages/engine/src/mechanisms/funds/data.ts (a mandate of bills, and a maximum tenor: short, high-quality paper) |
| `Fund Shares D2` | PARTIAL | packages/engine/src/mechanisms/households/portfolio.ts (a saver holds it instead of a deposit and asks the money back when it cannot cover what it means to spend; its account falls to what it is about to spend, because a deposit pays it nothing). D2.a is only half a competition until a bank BIDS for a deposit (Banks Funding B1, worklist 11): the fund publishes what it offers, and nothing yet answers |
| `Fund Shares D3` | MET | packages/engine/src/mechanisms/funds/index.ts (it is a buyer in the bill market, and its size is what determines how much paper it can place: the savers' money reaches the state's paper through it) |
| `Fund Shares D4` | MET | packages/engine/src/mechanisms/funds/nav.ts (nothing can hold the number at one because there is nothing to hold it WITH: the value is the division, and if the assets fall it falls), packages/engine/test/funds.test.ts |
| `Fund Shares D5` | PARTIAL | packages/engine/src/mechanisms/households/portfolio.ts (the flow into the fund is a consequence of the cell's own budget and stops when what the fund offers stops clearing what the cell requires). Whether flows rise when its yield beats DEPOSITS cannot be measured until a deposit has a rate (worklist 11); it is a VERIFY for Part XII |
| `Fund Shares E1` | MET | packages/engine/src/mechanisms/funds/index.ts (its shares have a market and a session prices them, which is the whole of what makes it exchange-traded: it is the same claim on the same kind of book as any other fund's), packages/engine/test/etf.test.ts |
| `Fund Shares E2` | MET | packages/engine/src/mechanisms/funds/etf.ts, packages/engine/src/mechanisms/funds/index.ts (the NAV read off its own book and the print the session made, published together and different numbers; neither is the other's approximation — and only the POSTED one is on a price grid, because rounding the other leaves the fund holding a residue of its holders' money), packages/engine/test/etf.test.ts |
| `Fund Shares E3` | MET | packages/engine/src/mechanisms/funds/etf.ts (a creation unit is a pro-rata slice of what the fund ACTUALLY holds, so a creation cannot change what the fund is; delivered and taken back in one instruction, every leg or none), packages/engine/src/mechanisms/banks/dealing.ts (E3.a: a desk does it because the gap is worth more than a period of carrying the position costs it, and does nothing when it is not — so a gap nobody will close stays open) |
| `Fund Shares E4` | MET | packages/engine/src/mechanisms/funds/index.ts (the premium is a READ of the two prices published beside them; nothing anywhere clamps it and nothing tries to close it), packages/engine/test/etf.test.ts (a large one persists, which is E4's finding about liquidity rather than a defect in the arithmetic) |
| `Fund Shares F1` | MET | packages/engine/src/mechanisms/funds/index.ts (it does not create its assets: every unit it holds it bought from a named seller in a market that cleared), packages/engine/test/funds.test.ts |
| `Fund Shares F2` | MET | packages/engine/src/mechanisms/funds/index.ts, packages/engine/src/registry/profiles.ts (nobody lends to it, stated on the kind, so it cannot hold more than it raised; and the kernel refuses to value a book that holds its own claim) |
| `Fund Shares F3` | MET | packages/engine/src/mechanisms/funds/index.ts (the manager is a separate party and the fee is its income and the fund's cost) |
| `Fund Shares G1` | MET | packages/engine/src/mechanisms/funds/index.ts (a share count, a redemption request, a sale in the same period's books, and the cost of a late sale landing on the holders who stayed), packages/engine/src/mechanisms/funds/etf.ts (G1.a: an exchange-traded fund redeems IN KIND against a pro-rata slice of its own book — nothing is sold and no market is touched, which is exactly why this vehicle is not the forced seller and why XI-2 runs through the money fund instead) |

## Securities Lending

| requirement | status | where / why |
|---|---|---|
| `Securities Lending A1` | MISSING |  |
| `Securities Lending A2` | MISSING |  |
| `Securities Lending A3` | MISSING |  |
| `Securities Lending A4` | MISSING |  |
| `Securities Lending A5` | MISSING |  |
| `Securities Lending B1` | MISSING |  |
| `Securities Lending B2` | MISSING |  |
| `Securities Lending B3` | MISSING |  |
| `Securities Lending B4` | MISSING |  |
| `Securities Lending C1` | MISSING |  |
| `Securities Lending C2` | MISSING |  |
| `Securities Lending C3` | MISSING |  |
| `Securities Lending C4` | MISSING |  |
| `Securities Lending C5` | MISSING |  |
| `Securities Lending D1` | MISSING |  |
| `Securities Lending D2` | MISSING |  |
| `Securities Lending D2.a` | MISSING |  |
| `Securities Lending D3` | MISSING |  |
| `Securities Lending E1` | MISSING |  |
| `Securities Lending E2` | MISSING |  |
| `Securities Lending E3` | MISSING |  |

## Prime Brokerage

| requirement | status | where / why |
|---|---|---|
| `Prime Brokerage A1` | MISSING |  |
| `Prime Brokerage A2` | MISSING |  |
| `Prime Brokerage A3` | MISSING |  |
| `Prime Brokerage A4` | MISSING |  |
| `Prime Brokerage B1` | MISSING |  |
| `Prime Brokerage B2` | MISSING |  |
| `Prime Brokerage B3` | MISSING |  |
| `Prime Brokerage B4` | MISSING |  |
| `Prime Brokerage B5` | MISSING |  |
| `Prime Brokerage C1` | MISSING |  |
| `Prime Brokerage C2` | MISSING |  |
| `Prime Brokerage C3` | MISSING |  |
| `Prime Brokerage C3.b` | MISSING |  |
| `Prime Brokerage C4` | MISSING |  |
| `Prime Brokerage C4.a` | MISSING |  |
| `Prime Brokerage C5` | MISSING |  |
| `Prime Brokerage D1` | MISSING |  |
| `Prime Brokerage D2` | MISSING |  |
| `Prime Brokerage D3` | MISSING |  |
| `Prime Brokerage D4` | MISSING |  |
| `Prime Brokerage E1` | MISSING |  |
| `Prime Brokerage E2` | MISSING |  |
| `Prime Brokerage E3` | MISSING |  |
| `Prime Brokerage E4` | MISSING |  |

## Derivative Layer

| requirement | status | where / why |
|---|---|---|
| `Derivative Layer A1` | MET | packages/engine/src/register/contracts.ts, packages/engine/src/registry/derivatives.ts (a row lives until it is terminated, and both sides stay exposed to each other meanwhile) |
| `Derivative Layer A2` | MET | packages/engine/src/mechanisms/derivative-layer/margin.ts |
| `Derivative Layer A3` | MET | packages/engine/src/prices/contract-value.ts (one number, read from two sides) |
| `Derivative Layer A4` | MET | packages/engine/src/audit/families/zero-sum.ts (per contract, exactly; in aggregate per money, at the dust of the sum) |
| `Derivative Layer B1` | MET | packages/engine/src/clearing/market.ts (a `contract` book: the solver over posted schedules, and the fill becomes a row) |
| `Derivative Layer B2` | MET | packages/engine/src/ledger/settlement.ts (one contract, recorded on both books in one instruction) |
| `Derivative Layer B3` | MET | packages/engine/src/mechanisms/derivative-layer/index.ts (closed by an offsetting close, an early termination or expiry) |
| `Derivative Layer B4` | MET | packages/engine/src/register/contracts.ts, packages/engine/src/registry/derivatives.ts (`novate`), packages/engine/src/ledger/settlement.ts (the obligation leaves one balance sheet at what it was carried at and lands on another) |
| `Derivative Layer C1` | MET | packages/engine/src/register/contracts.ts, packages/engine/src/registry/derivatives.ts (`between`: the rows two named parties have with each other) |
| `Derivative Layer C2` | MET | packages/engine/src/clearing/market.ts (one trade becomes two rows, member to house and house to member, so no member pays another) |
| `Derivative Layer C3` | MET | packages/engine/src/mechanisms/derivative-layer/house.ts (the margin it holds, the fund it owes, and its own capital as the residual read) |
| `Derivative Layer C4` | MET | packages/engine/src/mechanisms/derivative-layer/house.ts (`runWaterfall`, in order) |
| `Derivative Layer C5` | MET | packages/engine/src/mechanisms/derivative-layer/house.ts (`waterfall.unfunded`: past the end is an event and the house’s own equity carries it; nothing tops it up) |
| `Derivative Layer D1` | MET | packages/engine/src/world/world.ts (`measuredMove`: the underlying’s own record), packages/engine/src/registry/derivatives.ts (and no rate per class exists) |
| `Derivative Layer D2` | MET | packages/engine/src/mechanisms/derivative-layer/margin.ts (the requirement re-measured after the marks moved, met in cash) |
| `Derivative Layer D2.b` | MET | packages/engine/src/mechanisms/derivative-layer/index.ts (`marginIsHeld`: what every poster holds against what every holder issued, read from both ends) |
| `Derivative Layer D2.c` | MET | packages/engine/src/mechanisms/derivative-layer/margin.ts (it goes through settlement; a party that cannot pay fails the instruction and is in Money E1’s state) |
| `Derivative Layer D3` | MET | packages/engine/src/mechanisms/derivative-layer/margin.ts (held, not consumed: when the requirement falls the claim is redeemed and the cash comes back) |
| `Derivative Layer D4` | MET | packages/engine/src/mechanisms/derivative-layer/index.ts (`margin.call`, journaled with what was asked and whether it was met) |
| `Derivative Layer D5` | MET | packages/engine/src/mechanisms/derivative-layer/margin.ts (it rises with the measured move, which is procyclical by construction and is measured rather than smoothed) |
| `Derivative Layer E1` | MET | packages/engine/src/mechanisms/derivative-layer/index.ts (own liquid cash net of what it has already committed and of its own buffer) |
| `Derivative Layer E2` | MET | packages/engine/src/clearing/market.ts (cut to the smaller of the two sides’ admitted shares, at the strike, in the same pass as the margin) |
| `Derivative Layer E3` | MET | packages/engine/src/mechanisms/derivative-layer/margin.ts (`committed`: capacity is drawn down as it is consumed) |
| `Derivative Layer E4` | MET | packages/engine/src/clearing/market.ts (`derivatives.refused`, with the size), packages/engine/src/mechanisms/derivative-layer/index.ts (`refusedThisPeriod`) |
| `Derivative Layer F1` | MET | packages/engine/src/mechanisms/derivative-layer/index.ts (a party fails with open rows and they are resolved rather than forgotten) |
| `Derivative Layer F2` | MET | packages/engine/src/mechanisms/derivative-layer/index.ts (closed out at the stated value; the in-the-money side has a claim on the estate) |
| `Derivative Layer F3` | MET | packages/engine/src/mechanisms/derivative-layer/index.ts (the loss is the mark less the collateral held, and it lands on named survivors) |
| `Derivative Layer F4` | MET | packages/engine/src/mechanisms/derivative-layer/index.ts (`contractsNameTheLiving`) and the journal’s `waterfall.round` and `contract.closed` |
| `Derivative Layer G1` | MET | packages/engine/src/audit/families/zero-sum.ts, packages/engine/src/ledger/settlement.ts |
| `Derivative Layer G2` | MET | packages/engine/src/registry/derivatives.ts (a kind that cannot say what a position could do answers Missing and the layer refuses the trade) |
| `Derivative Layer G3` | MET | packages/engine/src/world/context.ts (there is no door that nets across counterparties) |
| `Derivative Layer G4` | MET | packages/engine/src/world/world.ts (`missingUnderlying`, asked when the book opens and again when a row is written) |

## CDS

| requirement | status | where / why |
|---|---|---|
| `CDS A1` | MET | packages/engine/src/mechanisms/cds/index.ts (a book per reference per tenor, cleared through the house) |
| `CDS A2` | MET | packages/engine/src/mechanisms/cds/contract.ts (`legs`: the premium in cash every period, stopping the period the reference defaults) |
| `CDS A3` | MET | packages/engine/src/mechanisms/cds/contract.ts (`markOf`: the spread now against the spread struck, and after the event the payoff — two reads, no model) |
| `CDS A4` | MET | packages/engine/src/mechanisms/cds/index.ts (`openBooks`: a reference is a party with live paper that can default) |
| `CDS A4.a` | MET | packages/engine/src/mechanisms/cds/index.ts (`referencesAreObservable`: a row on a reference nobody can watch fail is reported, never silent) |
| `CDS A5` | MET | packages/engine/src/mechanisms/cds/series.ts (names fixed at the roll; a name’s event settles its weight once per contract; the line runs on over survivors) |
| `CDS B1` | MET | packages/engine/src/mechanisms/cds/participants.ts (`ownView`: a party whose outlook of this book differs from it) |
| `CDS B2` | MET | packages/engine/src/mechanisms/cds/participants.ts (the seller: the same credit with no bond behind it) |
| `CDS B3` | MET | packages/engine/src/mechanisms/cds/index.ts (`regulation.riskWeight.cds.sold`, policy, parliament) |
| `CDS B4` | MISSING |  |
| `CDS B5` | MISSING |  |
| `CDS C1` | MISSING |  |
| `CDS C2` | MET | packages/engine/src/mechanisms/cds/contract.ts (`impliedDefaultRate`: derived from the spread and the recovery, stored nowhere) |
| `CDS C3` | MET | packages/engine/src/mechanisms/cds/measures.ts (`basisFor`: protection against the same name's own cash spread over the sovereign curve, a read at the read; surfaced through `DerivativeKindProfile.measures`, so the class that knows it is the one that computes it) |
| `CDS C4` | MISSING |  |
| `CDS D1` | MET | packages/engine/src/mechanisms/cds/contract.ts (`creditState`: item 5’s `credit.default` for the reference) |
| `CDS D2` | MET | packages/engine/src/mechanisms/cds/index.ts (`settleEvents`: settled at the estate’s realised recovery) |
| `CDS D2.a` | MET | packages/engine/src/mechanisms/cds/contract.ts (`payoff` reads the defaulted line’s own mark; no recovery rate exists anywhere) |
| `CDS D3` | MET | packages/engine/src/mechanisms/cds/index.ts (cash from a named seller to a named buyer, and it can fail) |
| `CDS D4` | MET | packages/engine/src/mechanisms/cds/index.ts (`settleEvents` closes the row when it settles) |
| `CDS D5` | MISSING |  |
| `CDS E1` | MISSING |  |
| `CDS E2` | MISSING |  |
| `CDS E3` | MET | packages/engine/src/mechanisms/cds/measures.ts (`netNotionalOn`: one question about one name, never a netting across counterparties; shown once per name however many tenors carry a book on it) |
| `CDS E4` | MISSING |  |

## IRS

| requirement | status | where / why |
|---|---|---|
| `IRS A1` | MET | packages/engine/src/mechanisms/irs/contract.ts (two legs on one notional) |
| `IRS A2` | MET | packages/engine/src/mechanisms/irs/contract.ts (`fixedEvery`, `floatEvery`: each leg its own periodicity) |
| `IRS A3` | MET | packages/engine/src/mechanisms/irs/contract.ts (the floating leg fixes on what the book actually paid) |
| `IRS A4` | MET | packages/engine/src/mechanisms/irs/contract.ts (only the net moves, so only the net is in anybody’s cash) |
| `IRS B1` | MET | packages/engine/src/mechanisms/irs/participants.ts (`fixedDebtOf`: what it owes at a rate its terms fixed) |
| `IRS B2` | PARTIAL | The reason is stated and no party in this world has it: a pension whose liabilities are long and whose assets are not does not exist until 13h. Measured — every schedule in every contract book is on the same side, because every party the layer admits is a bank or a firm (13b's record; carried into `docs/plan/13h-insurers-hedge-pe.md` as `13b-10`). B2.a's one-way demand is what 13h's parties bring |
| `IRS B3` | MET | packages/engine/src/mechanisms/irs/participants.ts (a bank managing its own gap) |
| `IRS B4` | MET | packages/engine/src/mechanisms/irs/participants.ts (a view on the rate path, from its own outlook) |
| `IRS B5` | MET | packages/engine/src/mechanisms/banks/dealing.ts (a bank quotes out of its own inventory against its own capital and funding), packages/engine/src/mechanisms/irs/participants.ts (`swapped`: its net position in the book is what it is hedging) |
| `IRS C1` | MET | packages/engine/src/mechanisms/irs/index.ts (`swapCurve`: the set of cleared fixed rates) |
| `IRS C2` | MET | packages/engine/src/mechanisms/irs/index.ts (`forwardRate`: derived from two cleared points) |
| `IRS C3` | MET | packages/engine/src/mechanisms/irs/measures.ts (`swapSpread`: the cleared rate against the sovereign’s own yield, a read at the read; the sovereign is `WorldReads.sovereignCurveIn`, never a party named at assembly) |
| `IRS C4` | MISSING |  |
| `IRS D1` | MISSING |  |
| `IRS D2` | MISSING |  |
| `IRS D3` | MISSING |  |
| `IRS D4` | MISSING |  |
| `IRS E1` | MET | packages/engine/src/mechanisms/irs/contract.ts (no path through `legs` moves the notional; `irs.test.ts` reads every leg of a life) |
| `IRS E2` | MET | packages/engine/src/mechanisms/irs/index.ts (there is no `parRate` and no discount curve in this module to run backwards) |
| `IRS E3` | MET | packages/engine/src/mechanisms/irs/contract.ts (a period the overnight book did not trade has no fixing and nothing accrues) |

## FX Forwards

| requirement | status | where / why |
|---|---|---|
| `FX Forwards A1` | MET | packages/engine/src/mechanisms/fx-derivatives/contract.ts (`fxForwardKind`) |
| `FX Forwards A2` | MET | packages/engine/src/mechanisms/fx-derivatives/contract.ts (nothing falls due before maturity) |
| `FX Forwards A3` | MET | packages/engine/src/mechanisms/fx-derivatives/contract.ts (`forwardMark`: against the forward for the tenor left, never against spot) |
| `FX Forwards A4` | MET | packages/engine/src/mechanisms/fx-derivatives/index.ts (an FX swap is a spot trade and a forward row; no kind is invented for it) |
| `FX Forwards B1` | MET | packages/engine/src/mechanisms/fx-derivatives/participants.ts (the forward rate clears, from schedules on both sides) |
| `FX Forwards B2` | MET | packages/engine/src/mechanisms/fx-derivatives/participants.ts (`carryOf`: the arbitrage as a bank’s own reservation) |
| `FX Forwards B3` | MET | packages/engine/src/mechanisms/fx-derivatives/participants.ts (`basisOf`: one basis, read from prints) |
| `FX Forwards B3.b` | MET | packages/engine/src/mechanisms/fx-derivatives/participants.ts (no parity formula anywhere sets a level; `derivative-classes.test.ts`) |
| `FX Forwards B4` | MISSING |  |
| `FX Forwards C1` | MET | packages/engine/src/mechanisms/fx-derivatives/contract.ts (`xccyKind`: the notionals exchanged at the start) |
| `FX Forwards C2` | MISSING |  |
| `FX Forwards C3` | MET | packages/engine/src/mechanisms/fx-derivatives/contract.ts (returned at the ORIGINAL rate) |
| `FX Forwards C4` | MET | packages/engine/src/mechanisms/fx-derivatives/contract.ts (its price is the basis on one leg, cleared) |
| `FX Forwards D1` | MISSING |  |
| `FX Forwards D2` | MISSING |  |
| `FX Forwards D3` | MET | packages/engine/src/mechanisms/banks/dealing.ts (`carryRate`: a foreign line is funded in its own money, at what the bank published for it) |
| `FX Forwards D4` | MISSING |  |
| `FX Forwards E1` | MISSING |  |
| `FX Forwards E2` | MISSING |  |
| `FX Forwards E3` | MET | packages/engine/src/mechanisms/fx-derivatives/contract.ts (both legs in one instruction, so both settle or neither does) |
| `FX Forwards E4` | MISSING |  |

## Commodity Futures

| requirement | status | where / why |
|---|---|---|
| `Commodity Futures A1` | MISSING |  |
| `Commodity Futures A2` | MISSING |  |
| `Commodity Futures A3` | MISSING |  |
| `Commodity Futures A4` | MISSING |  |
| `Commodity Futures B1` | MISSING |  |
| `Commodity Futures B2` | MISSING |  |
| `Commodity Futures B3` | MISSING |  |
| `Commodity Futures B4` | MISSING |  |
| `Commodity Futures B5` | MISSING |  |
| `Commodity Futures C1` | MISSING |  |
| `Commodity Futures C2` | MISSING |  |
| `Commodity Futures C3` | MISSING |  |
| `Commodity Futures C4` | MISSING |  |
| `Commodity Futures D1` | MISSING |  |
| `Commodity Futures D2` | MISSING |  |
| `Commodity Futures D3` | MISSING |  |
| `Commodity Futures D4` | MISSING |  |
| `Commodity Futures E1` | MISSING |  |
| `Commodity Futures E2` | MISSING |  |
| `Commodity Futures E3` | MISSING |  |

## Commodities Spot

| requirement | status | where / why |
|---|---|---|
| `Commodities Spot A1` | MISSING |  |
| `Commodities Spot A2` | MISSING |  |
| `Commodities Spot A3` | MISSING |  |
| `Commodities Spot A4` | MISSING |  |
| `Commodities Spot B1` | MISSING |  |
| `Commodities Spot B2` | MISSING |  |
| `Commodities Spot B3` | MISSING |  |
| `Commodities Spot B4` | MISSING |  |
| `Commodities Spot C1` | MISSING |  |
| `Commodities Spot C2` | MISSING |  |
| `Commodities Spot C3` | MISSING |  |
| `Commodities Spot C4` | MISSING |  |
| `Commodities Spot D1` | MET | packages/engine/src/mechanisms/goods/index.ts (the same grade in two places is two instruments with two prints; 13c.1 made the places real) |
| `Commodities Spot D2` | MISSING |  |
| `Commodities Spot D3` | MISSING |  |
| `Commodities Spot D4` | MISSING |  |
| `Commodities Spot D5` | PARTIAL | the identity holds for every physical kind the world has, checked from two independent records (packages/engine/src/audit/families/units.ts, packages/engine/src/mechanisms/goods/index.ts); commodities as a system arrive at worklist 13c |
| `Commodities Spot E1` | MISSING |  |
| `Commodities Spot E2` | MISSING |  |
| `Commodities Spot E3` | MISSING |  |
| `Commodities Spot E4` | MISSING |  |
| `Commodities Spot F1` | MET | packages/engine/src/ledger/settlement.ts (units enter the world only through a production event; a destroy beyond what is held fails), packages/engine/src/mechanisms/goods/index.ts (and what a batch consumed is checked against its recipe) |
| `Commodities Spot F2` | MISSING |  |
| `Commodities Spot F3` | MISSING |  |

## Indices

| requirement | status | where / why |
|---|---|---|
| `Indices A1` | MET | packages/engine/src/mechanisms/indices/baskets.ts, packages/engine/src/prices/index-read.ts (an index is a stated rule over constituents, and the rule is data) |
| `Indices A2` | MET | packages/engine/src/prices/index-read.ts (the level is applied where it is asked for; nothing stores one), packages/engine/src/world/world.ts (`walkIndices`: the step is taken ONCE, at the close of the period, so no reader ever re-derives a past step against a basket that period never had), packages/engine/src/observer/observer.ts (`IndexView.from`: what each rule was read from, so a size boundary is where the list stops rather than a number anybody keeps) |
| `Indices A3` | MET | packages/engine/src/prices/index-read.ts (the level is chained, so a rebalance moves nothing) |
| `Indices A4` | MET | packages/engine/src/mechanisms/indices/index.ts (the base is a declared resolution: doubling it doubles every level and changes nothing) |
| `Indices B1` | MET | packages/engine/src/mechanisms/indices/baskets.ts (a weight is a COUNT of the line — shares in issue, par outstanding, units bought — never a share of the index) |
| `Indices B2` | MET | packages/engine/src/prices/index-read.ts (each step compares this period’s basket against itself a period ago) |
| `Indices B3` | MET | packages/engine/src/world/world.ts (a split multiplies the count and divides the price in one event, so the basket is worth what it was) |
| `Indices B4` | MET | packages/engine/src/prices/index-read.ts (a line that did not print is not in the step: an index cannot report a price nobody made) |
| `Indices C1` | MET | packages/engine/src/mechanisms/funds/tracker.ts (a mandate refers to the index’s own basket, not to what happened to print) |
| `Indices C2` | MET | packages/engine/src/mechanisms/funds/tracker.ts, packages/engine/src/mechanisms/funds/data.ts (a tracker holds the index at the index’s weights and trades the difference) |
| `Indices C2.a` | MET | packages/engine/src/mechanisms/funds/tracker.ts (a rebalance is a real trade in the same session; a line that left the index goes out at market) |
| `Indices C3` | MET | packages/engine/src/mechanisms/index-futures/index.ts (cash-settled against the index READ at expiry) |
| `Indices C4` | PARTIAL | packages/engine/src/mechanisms/indices/benchmark.ts — the benchmark is published and transacted; a floating coupon that FIXES on it is 13f’s |
| `Indices D1` | MET | packages/engine/src/mechanisms/indices/baskets.ts (an equity index per region, over the listed shares of the companies that book there) |
| `Indices D2` | MET | packages/engine/src/mechanisms/indices/baskets.ts (a credit index per currency, over the dated claims somebody other than the state promised — empty, and therefore Missing, until 13f issues some) |
| `Indices D3` | MET | packages/engine/src/mechanisms/indices/benchmark.ts (the volume-weighted rate of the overnight lending that SETTLED, per book; a book that did not trade has no fixing) |
| `Indices D3.b` | MET | packages/engine/src/mechanisms/indices/benchmark.ts (nothing here reads the corridor: what the central bank administers is not what the market paid) |
| `Indices D4` | PARTIAL | packages/engine/src/mechanisms/indices/baskets.ts — producer and consumer indices over the same goods weighed by what this region’s sellers sold and what its cells bought. What will part the two levels is the wedge between the gate and the counter, which is 13c’s freight and distribution margin |
| `Indices D5` | MET | packages/engine/src/world/world.ts (one system: a second module declaring the same id is refused at assembly) |
| `Indices E1` | MET | packages/engine/src/mechanisms/indices/index.ts, packages/engine/src/observer/observer.ts (published as an observation, with what it was read from) |
| `Indices E2` | MET | packages/engine/src/prices/index-read.ts (no stored level, so it cannot be stale and cannot be revised) |
| `Indices E3` | MET | packages/engine/src/audit/families/cross-market.ts (the audit reads every index a second time from the prints and puts the two against each other), packages/engine/test/equity-anchor.test.ts (the two agree every period of a year) |

## Banks Lending

| requirement | status | where / why |
|---|---|---|
| `Banks Lending A1` | MET | packages/engine/src/mechanisms/banks/loan.ts (a bilateral contract between a named bank and a named borrower: the lender is on the terms, the borrower is the issuer, and neither is a class of anything) |
| `Banks Lending A2` | MET | packages/engine/src/mechanisms/banks/loan.ts (principal, maturity, rate and currency fixed at origination; the maturity is placed by date like every other in this world) |
| `Banks Lending A3` | PARTIAL | packages/engine/src/mechanisms/banks/index.ts drawn and repaid on ONE line per lender and borrower, at the margin struck when the line was agreed. A committed UNDRAWN limit needs a borrower that asks for a limit rather than an amount, and that is a firm with an investment programme (worklist 10) or corporate paper (13f) |
| `Banks Lending A3.b` | MISSING | undrawn commitments need a committed limit to be undrawn against (worklist 10, 13f) |
| `Banks Lending A4` | MET | packages/engine/src/mechanisms/banks/loan.ts (secured on named collateral or unsecured, and it says which — every loan here is unsecured because there is nothing a bank could realise until an estate exists, worklist 7) |
| `Banks Lending A5` | MISSING | covenants need a borrower with accounts to test against; the first are corporate paper's (worklist 13f) |
| `Banks Lending B1` | MET | packages/engine/src/mechanisms/banks/index.ts (one instruction: the borrower issues the loan and the bank creates its own money into the borrower's account, at the same instant) |
| `Banks Lending B1.c` | MET | packages/engine/src/mechanisms/banks/index.ts, packages/engine/test/loans.test.ts (there is nowhere a deposit or a reserve is consumed to fund a loan, and the test asserts the instruction carries no interbank leg at all) |
| `Banks Lending B2` | MET | packages/engine/src/mechanisms/banks/quote.ts (`room`: capital, its own appetite for the name, and now LIQUIDITY too — what its own funding leaves it, read from the position it published (Banks Funding C1) — with which of the three bound recorded on every decline) |
| `Banks Lending B2.d` | MET | packages/engine/src/mechanisms/banks/quote.ts (which constraint binds is computed from the bank's own state and recorded on the decline, so it differs by bank and by period rather than being decided once) |
| `Banks Lending C1` | MET | packages/engine/src/mechanisms/banks/quote.ts (four named terms and their sum: what its funding costs it, what it expects to lose on this borrower, the capital the loan consumes times what it needs on it, and what it costs to run) |
| `Banks Lending C2` | MET | packages/engine/src/mechanisms/banks/index.ts (every bank quotes from its own state and the borrower takes the keenest that will have it; a bank with no room does not quote) |
| `Banks Lending C3` | MET | packages/engine/src/mechanisms/banks/index.ts (declining IS the credit decision: a bank with no room writes nothing and says so) |
| `Banks Lending C3.a` | MET | packages/engine/src/mechanisms/banks/index.ts (what was declined and what was written is published every period as a count and a volume; who was refused stays between the two of them, that it happened does not) |
| `Banks Lending C4` | MET | packages/engine/src/mechanisms/banks/quote.ts (one function of one set of observations answers both the price and the provision, so they cannot be struck against different beliefs), packages/engine/test/loans.test.ts |
| `Banks Lending D1` | MET | packages/engine/src/mechanisms/banks/loan.ts (carried at cost, and it names no market at all — the kernel refuses a cleared kind without one, so a loan cannot be marked to a market that does not exist) |
| `Banks Lending D2` | MET | packages/engine/src/mechanisms/banks/index.ts (what a unit is worth to the bank holding it is that bank's own expected recovery), packages/engine/src/world/revalue.ts (and the kernel writes the lot down to it and moves the equity account by the same, which is the charge to income) |
| `Banks Lending D2.b` | MET | packages/engine/src/world/revalue.ts (the provision is ON the lot the book carries the loan at; there is no reserve anywhere for a loss to be absorbed into, and every movement is journalled with its size) |
| `Banks Lending D3` | MET | packages/engine/src/mechanisms/banks/loan.ts (interest accrues by day count on what is outstanding and falls due every period the calendar places), packages/engine/src/world/actions.ts (paid by the kernel to the lender of record, and observably not paid when it is not) |
| `Banks Lending D4` | MISSING | a loan row is transferable like any instrument; a buyer for one, and the syndicate of D4.a, arrive with corporate credit (worklist 13f) |
| `Banks Lending D5` | MISSING | pledging needs a lender to pledge to, which is the money market (worklist 11) |
| `Banks Lending E1` | PARTIAL | packages/engine/src/world/actions.ts (a missed payment IS an event, dated, named and public, for every instrument whose profile defines one). A covenant breach needs covenants, and those need a borrower with accounts to test against (A5, worklist 13f) |
| `Banks Lending E2` | PARTIAL | packages/engine/src/register/instruments.ts (performing is written by exactly one path, the kernel's, on the instrument's own definition being met, and nothing restores it), packages/engine/src/mechanisms/credit-events/index.ts (and every holder of a claim that stopped performing carries a stated exposure from then on). A bigger provision against it is now real for a loan (packages/engine/src/mechanisms/banks/index.ts: it is carried at what its lender expects to recover). IMPAIRED as a third state of its own, distinct from not-performing, arrives with the workout (worklist 7) |
| `Banks Lending E3` | MISSING | restructure, extend and enforce are decisions between what each path would bring, and what a defaulted claim brings is the recovery that needs an estate (worklist 7) |
| `Banks Lending E4` | MISSING | there is nothing a bank can take security over and realise until an estate exists (worklist 7) |
| `Banks Lending E5` | PARTIAL | a write-off is a redemption at whatever the claim fetched — a real leg back to whoever promised it, never a number vanishing (packages/engine/test/credit-events.test.ts). Nothing in this world writes one off yet: a sovereign default is negotiated and has no estate (Sovereign G3), so the first is a firm's (worklist 7) |
| `Banks Lending E5.a` | MET | packages/engine/test/credit-events.test.ts (the loss reaching capital is principal minus recovery minus provisions already taken, which with nothing recovered and nothing provisioned is exactly the carrying value — and it is settlement's own arithmetic, not a second sum) |
| `Banks Lending E6` | MISSING | correlated losses need borrowers that share a cause, which needs enough borrowers to correlate (worklist 13e) |
| `Banks Lending F1` | MET | packages/engine/src/mechanisms/banks/index.ts, packages/engine/src/mechanisms/banks/loan.ts (a bank's book is the rows it holds and there is no book number anywhere) |
| `Banks Lending F1.a` | MET | packages/engine/src/mechanisms/banks/index.ts (the audit contribution: every unit outstanding is held by the lender of record, so a book with no loans in it cannot exist) |
| `Banks Lending F2` | MET | packages/engine/src/audit/families/flows.ts (the kernel already holds, for every holder and instrument, that the change equals the legs; a bank's book is the sum of its rows, so what accounts for the change in the book is checked row by row. A second sum over the same legs would be the parallel formula Law 4 hunts) |
| `Banks Lending F3` | MET | packages/engine/src/mechanisms/banks/quote.ts (the most it will have out to one name, as a share of its own capital — a limit that binds and changes what it writes), packages/engine/test/loans.test.ts |

## Banks Funding

| requirement | status | where / why |
|---|---|---|
| `Banks Funding A1` | MET | packages/engine/src/mechanisms/money-market/data.ts (retail, corporate and wholesale, as a read of what kind of party the depositor is) |
| `Banks Funding A1.d` | MET | packages/engine/src/mechanisms/money-market/deposits.ts (stickiness is a COST the depositor bears, per class, and it is what decides both whether it moves for a rate and whether it runs) |
| `Banks Funding A2` | MET | packages/engine/src/mechanisms/money-market/rows.ts (interbank and repo rows, short and rolling) |
| `Banks Funding A3` | PARTIAL | packages/engine/src/mechanisms/banks/subordinated.ts (the subordinated layer is real, issued into a market that can refuse). Equity that a bank RAISES needs a bank share line and owners, which is worklist 13g |
| `Banks Funding A4` | MET | packages/engine/src/mechanisms/money-market/session.ts (the central bank funds it on the corridor terms and no others) |
| `Banks Funding A5` | MET | packages/engine/src/mechanisms/money-market/index.ts, packages/engine/src/mechanisms/money-market/deposits.ts (each source has a price, the prices differ, and what it takes from each is what cleared) |
| `Banks Funding B1` | MET | packages/engine/src/mechanisms/money-market/deposits.ts (`payDepositInterest`: a real payment to the holder, at the rate the bank set) |
| `Banks Funding B2` | MET | packages/engine/src/mechanisms/banks/index.ts (`costOfFunds`: a read of what it actually paid on what it owes, blended with what its own capital costs it) |
| `Banks Funding B2.b` | MET | packages/engine/src/mechanisms/money-market/deposits.ts, packages/engine/src/mechanisms/banks/index.ts (the rate that leaves and the rate the quote is built on are the same number, read once) |
| `Banks Funding B3` | PARTIAL | both sides of the margin exist as published numbers (what it pays, what it charges) and it can be negative; the margin AS A READ is Part XII (worklist 16) |
| `Banks Funding C1` | MET | packages/engine/src/mechanisms/money-market/index.ts (`publishFunding`: the account, what comes back tomorrow, and what its unencumbered paper would raise at the declared haircut — C1.a as a number) |
| `Banks Funding C2` | MET | packages/engine/src/mechanisms/money-market/deposits.ts (`couldLeave`: the part of its own base nobody insures, holder by holder — C2.a derived from its liabilities) |
| `Banks Funding C3` | MET | packages/engine/src/mechanisms/money-market/rows.ts, packages/engine/src/mechanisms/banks/loan.ts (it funds dated assets with overnight and term money, and that is the book) |
| `Banks Funding C3.a` | PARTIAL | the two sides are dated in the register, so the gap is readable; measuring it is Part XII (worklist 16) |
| `Banks Funding C4` | MET | packages/engine/src/mechanisms/money-market/session.ts (`positionOf`: the residue of everybody else period, read off the wire) |
| `Banks Funding D1` | MET | packages/engine/src/mechanisms/money-market/session.ts (it borrows in the market, secured or unsecured) |
| `Banks Funding D2` | MET | packages/engine/src/mechanisms/banks/dealing.ts (it sells free eligible paper with a size and no level, so what it fetches is what somebody posted) |
| `Banks Funding D3` | MET | packages/engine/src/mechanisms/money-market/index.ts (`worthOfMoney`: it values money at the window when paying up is the cheaper answer, and pays it) |
| `Banks Funding D4` | MET | packages/engine/src/mechanisms/banks/quote.ts (`room`: no room, no new business — the credit crunch, read from its own published position) |
| `Banks Funding D5` | MET | packages/engine/src/mechanisms/money-market/index.ts (the window, collateralised and at a penalty) |
| `Banks Funding D6` | MET | packages/engine/src/world/failure.ts (a funding failure with its own trigger, distinct from insolvency and named as such) |
| `Banks Funding D6.a` | MET | packages/engine/src/mechanisms/money-market/index.ts (the only central-bank credit is collateralised, priced and refused to the insolvent, so D6 stays reachable) |
| `Banks Funding E1` | MET | packages/engine/src/mechanisms/money-market/deposits.ts (`moveDeposits`: they leave, and the payment can fail) |
| `Banks Funding E2` | MET | packages/engine/src/mechanisms/money-market/deposits.ts (`looksInTrouble`: only from what was published — a refusal, a draw, a short close) |
| `Banks Funding E3` | MET | packages/engine/test/run.test.ts (leaving forces the sale and the shrinking, and both produce more of E2.a) |
| `Banks Funding E3.a` | MET | packages/engine/test/run.test.ts (the reserves leave in the same instruction as the deposits, so the bank is shorter at the next close) |
| `Banks Funding E4` | MET | packages/engine/src/mechanisms/money-market/deposits.ts, packages/engine/test/run.test.ts (insurance is per member, so raising it past what a member holds takes a household cell out of what can run) |
| `Banks Funding E5` | PARTIAL | the second half is answered: a bank whose only capital is retained earnings can now raise a layer instead (bank-lending/subordinated.ts). A run at one bank being information about OTHERS is not built — every depositor reads its own bank only — and it arrives with the depositor that decides from its own balance (worklist 12) |
| `Banks Funding F1` | MET | packages/engine/src/mechanisms/money-market/index.ts, packages/engine/src/observer/observer.ts (deposit lines by class, published and shown) |
| `Banks Funding F2` | MET | packages/engine/src/mechanisms/money-market/index.ts (the reserve balance as a read of the one account, never a mirrored copy) |
| `Banks Funding F3` | MET | packages/engine/src/audit/families/accounts.ts |
| `Banks Funding F4` | MET | packages/engine/src/mechanisms/money-market/deposits.ts (`liquidityMetric`), packages/engine/src/observer/observer.ts (with how old it is beside it) |

## Banks Capital

| requirement | status | where / why |
|---|---|---|
| `Banks Capital A1` | MET | packages/engine/src/mechanisms/banks/capital.ts (capital is the equity account plus the layer behind it: a residual, read each period) |
| `Banks Capital A1.a` | MET | packages/engine/src/mechanisms/banks/capital.ts (nothing is spent from it; it falls when a loss is booked) |
| `Banks Capital A2` | MET | packages/engine/src/mechanisms/banks/subordinated.ts, packages/engine/src/mechanisms/money-market/resolution.ts (the layers absorb in rank order, most junior first, pari passu within a rank) |
| `Banks Capital A3` | PARTIAL | packages/engine/src/mechanisms/banks/subordinated.ts (it grows by retained earnings and by ISSUING the subordinated layer). An equity issue needs a bank share line and owners: worklist 13g |
| `Banks Capital A4` | MET | packages/engine/src/mechanisms/banks/capital.ts, packages/engine/src/mechanisms/money-market/resolution.ts (it falls by losses, and every one of them is an event with a date) |
| `Banks Capital B1` | MET | packages/engine/src/mechanisms/banks/capital.ts (a requirement against risk-weighted assets, with the weight asked of what the asset IS) |
| `Banks Capital B1.c` | MET | packages/engine/src/mechanisms/banks/capital.ts, packages/engine/test/bank-capital.test.ts (which of the two rules binds is a read of what the bank holds, and it moves when the rules move) |
| `Banks Capital B2` | MET | packages/engine/src/mechanisms/banks/data.ts, packages/engine/src/mechanisms/banks/capital.ts (the buffer above the line is each bank own caution) |
| `Banks Capital B3` | MET | packages/engine/src/mechanisms/banks/capital.ts (a breach demands a plan in public and leaves no room to lend). Distributions restricted needs shares to distribute on: worklist 13g |
| `Banks Capital B3.a` | MET | packages/engine/test/bank-capital.test.ts (a bank near the line writes no new business and says what it is short of) |
| `Banks Capital C1` | MET | packages/engine/src/world/failure.ts (both triggers exist and are asked of the kind own profile), packages/engine/src/mechanisms/money-market/resolution.ts (and the resolution names which one fired) |
| `Banks Capital C2` | PARTIAL | packages/engine/src/mechanisms/banks/subordinated.ts (recapitalisation is tried first and can fail — C2.b is tested). C2.a existing holders diluted needs a bank share line: worklist 13g |
| `Banks Capital C3` | MET | packages/engine/src/mechanisms/money-market/resolution.ts (the bank stops being a going concern on a trigger somebody applies, and its deposits keep working at the acquirer) |
| `Banks Capital D1` | MET | packages/engine/src/mechanisms/money-market/resolution.ts (`valueBook`: the book at marks, and the hole is what it owes less that) |
| `Banks Capital D2` | MET | packages/engine/src/mechanisms/money-market/resolution.ts (rank by rank, most junior first, pari passu within a rank, and a secured lender only for what its paper does not cover) |
| `Banks Capital D2.a` | MET | packages/engine/test/bank-resolution.test.ts (every claim in a rank takes the same share of the same loss) |
| `Banks Capital D3` | MET | packages/engine/src/mechanisms/money-market/resolution.ts (every other bank values the book from its own view and bids, and it may decline) |
| `Banks Capital D4` | MET | packages/engine/src/mechanisms/money-market/insurer.ts (a fund the banks pay into every period, that pays what the hierarchy could not reach, per member) |
| `Banks Capital D5` | MET | packages/engine/src/mechanisms/money-market/resolution.ts, packages/engine/test/bank-resolution.test.ts (the purse pays last, and with no premium collected there is no fund and it pays instead) |
| `Banks Capital D6` | MET | packages/engine/src/mechanisms/money-market/resolution.ts (the book and the deposits move to the acquirer over the wire, and a `names` family says nothing was left behind) |
| `Banks Capital E1` | MET | packages/engine/src/mechanisms/estate/index.ts (an estate is realised over time and creditors are paid from it) |
| `Banks Capital E2` | PARTIAL | the surviving system IS more concentrated after a resolution (one bank holds both books) and the register says so; measuring the consequence is Part XII (worklist 16) |
| `Banks Capital E3` | MISSING |  |

## Dealer Desks

| requirement | status | where / why |
|---|---|---|
| `Dealer Desks A1` | MET | packages/engine/src/mechanisms/banks/data.ts, packages/engine/src/mechanisms/banks/dealing.ts (A1's "its own balance sheet INSIDE a bank's" is a SUB-LEDGER and it is now structurally one: there is no desk party, the bank quotes out of its own inventory, and what a line of business is, is data about the bank) |
| `Dealer Desks A2` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts (a price it will buy at and a price it will sell at, both posted, both real sizes it will do) |
| `Dealer Desks A3` | MET | packages/engine/src/mechanisms/banks/dealing.ts (a desk quotes only the lines the listing drew it as a maker of) |
| `Dealer Desks A4` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts, packages/engine/src/world/revalue.ts (it earns the width it quoted and its inventory is marked every period like anybody's, and the two together are its whole result) |
| `Dealer Desks B1` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts (it quotes both sides because buyers and sellers arrive at different times; nothing tells it to be there) |
| `Dealer Desks B2` | PARTIAL | packages/engine/src/mechanisms/banks/dealing-quote.ts (it prices off the flow it FACED — how one-sided its own fills were — which is information it has and nobody else does). Whom it faced is named on every fill in the ledger, but the quote does not read the name yet: a counterparty expensive to face needs clients that repeat, worklist 13f |
| `Dealer Desks B3` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts (what a taker pays for immediacy is the desk's own edge; the alternative is waiting for a natural counterparty, and the `noOverlap` sessions are what that waiting looks like) |
| `Dealer Desks B4` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts, packages/engine/test/dealing.test.ts (the schedule is a function of the desk's own state and of nothing in the book: the XI-13 test posts the same quote with the book empty and with it full of other people's orders) |
| `Dealer Desks C1` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts (the quote comes from its inventory, its cost of funds, its limit and its own view — every one of them read off itself) |
| `Dealer Desks C2` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts (the further into its limit it is, the more the next unit costs it, so both sides come down together: long, it bids lower AND offers lower; C2.a: the book mean-reverts with nobody telling it to, which is why order flow moves prices) |
| `Dealer Desks C3` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts (risk is the width of the desk's OWN recent surprises about that line, in the line's own money — a read, not a number) |
| `Dealer Desks C4` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts (adverse selection is how one-sided the flow it faced was, priced at its own uncertainty on the share of the flow that went one way) |
| `Dealer Desks C5` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts (the two sides are derived separately and the width is what is left of them; the same posted quote goes into every book the desk makes a market in) |
| `Dealer Desks C5.a` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts (there is no mid in the file: what it wants for a unit and what it will pay for one are two different questions answered separately, so each can skew, widen and refuse on its own) |
| `Dealer Desks C5.b` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts, tools/eslint-rules (there is no width to state: the only declared numbers a desk reads are its limits, the return it needs on capital and the capital charge, and the lint refuses any other literal) |
| `Dealer Desks D1` | MET | packages/engine/src/mechanisms/banks/data.ts, packages/engine/src/mechanisms/banks/dealing-quote.ts (what it will have standing behind its book is a share of its own capital and what it will have in one line is a share of that book; the binding one is what shrinks the size and it is recorded on the quote) |
| `Dealer Desks D2` | MET | packages/engine/src/mechanisms/banks/dealing.ts (a capital charge on what it holds, at a risk weight and a ratio somebody wrote, and it pays for that capital every period) |
| `Dealer Desks D3` | MET | packages/engine/src/mechanisms/banks/dealing.ts, packages/engine/test/dealing.test.ts (the RENT is gone with the desk that paid it — a transfer price between two parties that were economically one. What funds the inventory is what the bank pays its depositors and its lenders, every period, to holders with names, and the quote's carry reads that blended cost plus what the capital the position consumes has to earn) |
| `Dealer Desks D4` | MET | packages/engine/src/mechanisms/banks/dealing.ts, packages/engine/src/mechanisms/banks/dealing-quote.ts (a desk at its limit widens, shrinks its size, and at the limit stops quoting altogether — a state, not an error); D4.a: packages/engine/test/dealing.test.ts (a market whose only liquidity was a desk that stepped back prints stale with the reason) |
| `Dealer Desks D5` | MET | packages/engine/src/mechanisms/banks/dealing.ts (`dealers.book` publishes inventory, the width, the skew and the room left together, per line, so a period in which spreads widened and inventory did not is visible rather than inferred), packages/engine/test/dealing.test.ts |
| `Dealer Desks E1` | MET | packages/engine/src/mechanisms/index-futures/index.ts (a desk lays its book off into a contract sized from its own inventory) |
| `Dealer Desks E2` | MET | packages/engine/src/mechanisms/index-futures/index.ts (the hedge is a contract with a counterparty and its own margin) |
| `Dealer Desks E3` | MET | packages/engine/src/mechanisms/banks/dealing.ts (two desks quoting into the same session, so inventory is redistributed between them through the book everybody else trades in — not a separate venue) |
| `Dealer Desks E4` | MET | packages/engine/src/audit/families/ownership.ts (held equals issued is the identity, and dealer inventory is the part of it the rest of the world does not hold), packages/engine/src/mechanisms/banks/dealing.ts (and what each desk is carrying is published every period, so it can be watched against client flow) |
| `Dealer Desks F1` | MET | packages/engine/src/mechanisms/banks/dealing-quote.ts (three real constraints — the line's limit, the book's, and the money it actually has — and the binding one is named on the quote; nothing here is unbounded) |
| `Dealer Desks F2` | MET | packages/engine/src/mechanisms/banks/capital.ts, packages/engine/src/mechanisms/banks/index.ts (NOTHING TO EXEMPT: one balance sheet, and a holding weighs by INTENT — up to the treasury's target it weighs what a claim on that issuer weighs, above it what a trading position weighs. The `accounts` family measures that the published requirement covers the book) |
| `Dealer Desks F3` | MET | packages/engine/src/world/revalue.ts, packages/engine/src/mechanisms/banks/dealing.ts (its P&L is the width it earned MINUS what the inventory did: the marks reach its equity like any other holder's, so a desk that is wrong loses money) |

## Insurers

| requirement | status | where / why |
|---|---|---|
| `Insurers A1` | MISSING |  |
| `Insurers A2` | MISSING |  |
| `Insurers A3` | MISSING |  |
| `Insurers A4` | MISSING |  |
| `Insurers B1` | MISSING |  |
| `Insurers B2` | MISSING |  |
| `Insurers B2.b` | MISSING |  |
| `Insurers B3` | MISSING |  |
| `Insurers B4` | MISSING |  |
| `Insurers C1` | MISSING |  |
| `Insurers C2` | MISSING |  |
| `Insurers C3` | MISSING |  |
| `Insurers C4` | MISSING |  |
| `Insurers C5` | MISSING |  |
| `Insurers D1` | MISSING |  |
| `Insurers D2` | MISSING |  |
| `Insurers D3` | MISSING |  |
| `Insurers D4` | MISSING |  |
| `Insurers D5` | MISSING |  |
| `Insurers E1` | MISSING |  |
| `Insurers E2` | MISSING |  |
| `Insurers E3` | MISSING |  |
| `Insurers E4` | MISSING |  |

## Hedge Funds

| requirement | status | where / why |
|---|---|---|
| `Hedge Funds A1` | MISSING |  |
| `Hedge Funds A2` | MISSING |  |
| `Hedge Funds A3` | MISSING |  |
| `Hedge Funds A4` | MISSING |  |
| `Hedge Funds A5` | MISSING |  |
| `Hedge Funds B1` | MISSING |  |
| `Hedge Funds B2` | MISSING |  |
| `Hedge Funds B3` | MISSING |  |
| `Hedge Funds B4` | MISSING |  |
| `Hedge Funds B5` | MISSING |  |
| `Hedge Funds C1` | MISSING |  |
| `Hedge Funds C2` | MISSING |  |
| `Hedge Funds C3` | MISSING |  |
| `Hedge Funds C4` | MISSING |  |
| `Hedge Funds D1` | MISSING |  |
| `Hedge Funds D2` | MISSING |  |
| `Hedge Funds D3` | MISSING |  |
| `Hedge Funds D4` | MISSING |  |
| `Hedge Funds D5` | MISSING |  |
| `Hedge Funds D6` | MISSING |  |
| `Hedge Funds D7` | MISSING |  |
| `Hedge Funds E1` | MISSING |  |
| `Hedge Funds E2` | MISSING |  |
| `Hedge Funds E3` | MISSING |  |

## Private Equity

| requirement | status | where / why |
|---|---|---|
| `Private Equity A1` | MISSING |  |
| `Private Equity A2` | MISSING |  |
| `Private Equity A2.b` | MISSING |  |
| `Private Equity A3` | MISSING |  |
| `Private Equity A4` | MISSING |  |
| `Private Equity A5` | MISSING |  |
| `Private Equity B1` | MISSING |  |
| `Private Equity B2` | MISSING |  |
| `Private Equity B3` | MISSING |  |
| `Private Equity B4` | MISSING |  |
| `Private Equity B5` | MISSING |  |
| `Private Equity C1` | MISSING |  |
| `Private Equity C2` | MISSING |  |
| `Private Equity C3` | MISSING |  |
| `Private Equity C4` | MISSING |  |
| `Private Equity C5` | MISSING |  |
| `Private Equity C5.a` | MISSING |  |
| `Private Equity D1` | MISSING |  |
| `Private Equity D2` | MISSING |  |
| `Private Equity D3` | MISSING |  |
| `Private Equity D4` | MISSING |  |
| `Private Equity D5` | MISSING |  |
| `Private Equity E1` | MISSING |  |
| `Private Equity E2` | MISSING |  |
| `Private Equity E3` | MISSING |  |

## Treasury

| requirement | status | where / why |
|---|---|---|
| `Treasury A1` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury A2` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury A3` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury B1` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury B2` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury B3` | PARTIAL | outlays now vary with what the state actually faces: its wage bill is a read of its own employment rows at the wage the venue cleared at, and its purchases are a budget the goods market rations (packages/engine/src/mechanisms/treasury/index.ts). Nothing in them rises with unemployment, because a benefit somebody draws on losing a job needs the loss to be an event (worklist 5), and policy that varies them is the polity (worklist 14) |
| `Treasury B4` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury C1` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury C2` | MET | packages/engine/src/mechanisms/treasury/index.ts (three bases read off what payers actually did: interest received, what households were paid, what they paid for real things — so receipts fall when income and spending fall) |
| `Treasury C3` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury D1` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury D2` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury D3` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/mechanisms/treasury/index.ts, packages/engine/src/registry/profiles.ts |
| `Treasury D4` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury D5` | MET | packages/engine/src/mechanisms/banks/dealing.ts, packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury D5.a` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/mechanisms/banks/dealing.ts, packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury D6` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury E1` | MET | packages/engine/src/mechanisms/treasury/data.ts, packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury E2` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury E3` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury E4` | MET | packages/engine/src/mechanisms/sovereign-curve/index.ts |
| `Treasury F1` | MISSING |  |
| `Treasury F2` | MISSING |  |
| `Treasury F3` | MISSING |  |
| `Treasury F4` | MISSING |  |

## Central Bank

| requirement | status | where / why |
|---|---|---|
| `Central Bank A1` | MET | packages/engine/src/registry/profiles.ts |
| `Central Bank A2` | MET | packages/engine/src/audit/families/accounts.ts |
| `Central Bank A2.c` | MET | packages/engine/src/audit/families/accounts.ts |
| `Central Bank A3` | PARTIAL | the mandate exists as a stated objective in the register; parliament owns its text and its target from worklist 14 |
| `Central Bank A4` | PARTIAL | financially owned by the treasury through remittance; operational independence has no rate to be independent about until the corridor (worklist 11) |
| `Central Bank B1` | MET | packages/engine/src/mechanisms/money-market/index.ts (`centralBank.policyRate`, declared by the central bank as a POLICY primitive with an owner). The rule it sets it BY — its own outlook of inflation and activity — arrives with the ratings and the measurement (worklist 12, 16) |
| `Central Bank B2` | MET | packages/engine/src/mechanisms/money-market/index.ts (`publishCorridor`: two administered levels, said out loud to be administered) |
| `Central Bank B3` | MET | packages/engine/test/money-market.test.ts (it is effective THROUGH the corridor: three points of policy moves what the session strikes and what a borrower is charged) |
| `Central Bank B3.a` | MET | packages/engine/test/money-market.test.ts (the FORBID holds: every printed rate sits between the two levels and none of them is the policy rate) |
| `Central Bank B4` | MET | packages/engine/test/money-market.test.ts (the market rate tracks it because of the corridor, and the gap is a read) |
| `Central Bank C1` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts (sovereign paper only: it stands in no other market) |
| `Central Bank C1.b` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Central Bank C2` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Central Bank C3` | MET | packages/engine/src/clearing/solver.ts, packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Central Bank C4` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Central Bank D1` | MET | packages/engine/src/mechanisms/money-market/session.ts (`windowOffer`: a seat in the money market own session at the top of the corridor, bounded by unencumbered eligible paper) |
| `Central Bank D2` | MET | packages/engine/src/mechanisms/money-market/collateral.ts (`windowAdvances`: eligibility and the haircut are the central bank own choice and a policy instrument) |
| `Central Bank D3` | MET | packages/engine/src/mechanisms/money-market/index.ts (`reserveOverdraft`: freely, against good collateral, at a penalty, to the solvent) |
| `Central Bank D3.a` | MET | packages/engine/src/mechanisms/money-market/index.ts, packages/engine/test/bank-resolution.test.ts (it refuses an insolvent bank for CAPITAL and not for collateral, and that bank goes to resolution) |
| `Central Bank D4` | MET | packages/engine/src/mechanisms/money-market/index.ts (`centralBank.refused`: refusal is a real, public outcome with what it was short of and why) |
| `Central Bank E1` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Central Bank E2` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/registry/profiles.ts |
| `Central Bank E3` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Central Bank E4` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Central Bank E5` | PARTIAL | the two statements are both true of the books; measuring them is the measurement programme (worklist 16) |
| `Central Bank F1` | MISSING |  |
| `Central Bank F2` | MISSING |  |
| `Central Bank F3` | MISSING |  |
| `Central Bank F4` | MISSING |  |

## Polity

| requirement | status | where / why |
|---|---|---|
| `Polity A1` | MISSING |  |
| `Polity A2` | MISSING |  |
| `Polity A3` | MISSING |  |
| `Polity A4` | MISSING |  |
| `Polity B1` | MISSING |  |
| `Polity B1.a` | MISSING |  |
| `Polity B2` | MISSING |  |
| `Polity B2.a` | MISSING |  |
| `Polity B2.b` | MISSING |  |
| `Polity B3` | MISSING |  |
| `Polity B4` | MISSING |  |
| `Polity C1` | MISSING |  |
| `Polity C2` | MISSING |  |
| `Polity C3` | MISSING |  |
| `Polity C3.a` | MISSING |  |
| `Polity C3.b` | MISSING |  |
| `Polity C4` | MISSING |  |
| `Polity D1` | MISSING |  |
| `Polity D2` | MISSING |  |
| `Polity D3` | MISSING |  |
| `Polity D3.a` | MISSING |  |
| `Polity D4` | MISSING |  |
| `Polity D5` | MISSING |  |
| `Polity E1` | MISSING |  |
| `Polity E2` | MISSING |  |
| `Polity E3` | MISSING |  |
| `Polity E4` | MISSING |  |
| `Polity F1` | MISSING |  |
| `Polity F2` | MISSING |  |
| `Polity F3` | MISSING |  |
| `Polity F4` | MISSING |  |
| `Polity F5` | MISSING |  |

## Firm

| requirement | status | where / why |
|---|---|---|
| `Firm A1` | MET | packages/engine/src/mechanisms/firms/data.ts, packages/engine/src/seeds/foundation.ts (a named party with an account and a register of what it holds) |
| `Firm A2` | MET | packages/engine/src/mechanisms/firms/data.ts (the region fixes its money and the line fixes what it buys, sells and employs) |
| `Firm A3` | PARTIAL | size and COST both: firms differ in stock, work in progress and cash, and each has its own labour productivity, so what it will pay for an hour differs from its neighbour's and one of them is the marginal employer (packages/engine/src/mechanisms/firms/data.ts, packages/engine/src/mechanisms/firms/decide.ts). That dispersion is what makes the venue a market rather than one bid. Leverage arrives with loans (worklist 6) |
| `Firm A4` | PARTIAL | the residual is the firm own equity account (packages/engine/src/audit/families/accounts.ts); owners of record are a share register (worklist 9) |
| `Firm B1` | MET | packages/engine/src/mechanisms/firms/decide.ts (what it sells is what its own offers cleared at, against named buyers; never a rate applied to last period) |
| `Firm B2` | MET | packages/engine/src/mechanisms/firms/produce.ts (what it bought, drawn at what it cost it: packages/engine/src/mechanisms/goods/inventory.ts) |
| `Firm B3` | MET | packages/engine/src/mechanisms/labour/matching.ts (headcount times wage, paid to named cells), packages/engine/src/mechanisms/firms/produce.ts (and absorbed into what it made) |
| `Firm B4` | MET | packages/engine/src/mechanisms/expectations/index.ts (its result is the sum of what every instruction and every mark did to its own equity, and it is often negative) |
| `Firm B5` | MET | packages/engine/src/mechanisms/firms/produce.ts (the period wage bill lands whole on whatever batch was started, and the inputs scale with it: a smaller batch is a higher unit cost) |
| `Firm B6` | MET | packages/engine/src/mechanisms/firms/index.ts (the audit contribution: nothing is on a batch that nobody was paid), packages/engine/src/audit/families/flows.ts |
| `Firm C1` | PARTIAL | cash and inventory, in lots at what they cost (packages/engine/src/mechanisms/goods/inventory.ts); receivables arrive with trade credit (worklist 13e) and fixed capital with the capital programme (worklist 10) |
| `Firm C2` | MISSING | payables arrive with trade credit (worklist 13e), bank debt with loans (worklist 6), bonds with corporate credit (worklist 13f) |
| `Firm C3` | MET | packages/engine/src/audit/families/accounts.ts |
| `Firm C4` | PARTIAL | inventory bought and not sold is a real use of cash and it is carried in lots; invoices sent and not paid arrive with trade credit (worklist 13e) |
| `Firm D1` | MET | packages/engine/src/mechanisms/labour/matching.ts (a wage that does not settle is recorded and the people are not paid), packages/engine/src/ledger/settlement.ts |
| `Firm D2` | MISSING |  |
| `Firm D3` | MISSING |  |
| `Firm D4` | MET | packages/engine/src/mechanisms/estate/index.ts (both failures are asked of every party whose kind names them, and the answer says which one fired: what fell due out of its own balance and it still cannot pay, or its liabilities past its assets at marks), packages/engine/test/estate.test.ts |
| `Firm D5` | MET | packages/engine/src/mechanisms/credit-events/index.ts (when it cannot pay, it is in default of payment — publicly, by name, with the payee that did not get paid and the amount that did not arrive) |
| `Firm E1` | PARTIAL | packages/engine/src/mechanisms/firms/decide.ts prices and sizes what it offers from its own outlook and what holding is worth, and how much to invest in the line it is in; entering and leaving a line is firm birth (worklist 13g) |
| `Firm E2` | MET | packages/engine/src/mechanisms/firms/decide.ts (the employment it wants is the labour that makes what it expects to sell, at what an hour is worth to it) |
| `Firm E3` | MET | packages/engine/src/mechanisms/firms/invest.ts (how much to invest, decided from its own view against its own cost of capital) |
| `Firm E4` | MET | packages/engine/src/mechanisms/firms/invest.ts (what its debt costs at the margin and what its equity costs, weighted by its own balance sheet, are what a project is measured against), packages/engine/src/mechanisms/firms/index.ts (what it cannot fund out of cash is a programme, and E4.a is that a firm with no programme is short of nothing on that account and raises nothing) |
| `Firm E5` | PARTIAL | what it does not pay out stays in its own account; a dividend needs owners of record, which is a share register (worklist 9) |
| `Firm E6` | MET | packages/engine/src/mechanisms/firms/decide.ts (every decision is a function of its own state, its own outlook and the prices it faces, and of nothing else) |
| `Firm E7` | MET | packages/engine/src/mechanisms/firms/produce.ts (it publishes its own outlook of its own earnings), packages/engine/src/mechanisms/expectations/index.ts (and the surprise against it is a recorded event) |
| `Firm F1` | MET | packages/engine/src/mechanisms/firms/index.ts (the audit contribution), packages/engine/src/ledger/settlement.ts (every leg has two named sides) |
| `Firm F2` | MET | packages/engine/src/mechanisms/firms/index.ts (the module declares no number at all: there is no earnings path, no target margin and no adjustment speed in it) |
| `Firm F3` | MET | packages/engine/src/mechanisms/labour/matching.ts, packages/engine/src/ledger/settlement.ts (nothing advances a firm that cannot pay; the payment fails and is recorded) |
| `Firm F4` | PARTIAL | no aggregate is stored anywhere, so a sector total can only be a sum over its firms; measuring it is Part XII (worklist 16) |

## Capital Programme

| requirement | status | where / why |
|---|---|---|
| `Capital Programme A1` | MET | packages/engine/src/mechanisms/capital-programme/plant.ts (a dated vintage of productive assets, held by a named firm, owned outright and issued by nobody) |
| `Capital Programme A2` | MET | packages/engine/src/mechanisms/capital-programme/capacity.ts (capacity is a function of the stock, per kind), packages/engine/src/mechanisms/firms/produce.ts (and output is limited by it) |
| `Capital Programme A3` | MET | packages/engine/src/mechanisms/capital-programme/plant.ts, packages/engine/src/world/revalue.ts (one schedule: the kernel asks the kind what a lot is carried at now and books the difference against the stock and against income together), packages/engine/src/mechanisms/capital-programme/capacity.ts (and the same wear is the capital charge in unit cost) |
| `Capital Programme A4` | MET | packages/engine/src/mechanisms/capital-programme/data.ts (a registry of kinds, each with its own unit, its own good and its own life), packages/engine/src/mechanisms/capital-programme/capacity.ts (a use that needs several is limited by the scarcest of them) |
| `Capital Programme A5` | MET | packages/engine/src/mechanisms/capital-programme/plant.ts (its value is what it can still produce: the carrying value is what it cost over the service it has left, so a vintage with a third of its life left is carried at a third) |
| `Capital Programme A6` | MET | packages/engine/src/mechanisms/capital-programme/plant.ts (dated vintages, each with its own cost, service date, life and kind; it leaves the register when fully worn), packages/engine/src/mechanisms/capital-programme/capacity.ts (gross, net, accumulated and the period charge are reads over the vintages) |
| `Capital Programme A6.b` | MET | packages/engine/src/mechanisms/capital-programme/index.ts (the units family: plant and capital in transit move only by a leg that says why, per firm per kind, every period) |
| `Capital Programme B1` | MET | packages/engine/src/mechanisms/firms/invest.ts (it invests when a unit of capacity is worth more to it than the plant that makes one costs, which is the return exceeding its cost of capital) |
| `Capital Programme B2` | MET | packages/engine/src/mechanisms/firms/invest.ts (what it posts is what it can pay for; what it cannot is a programme it publishes), packages/engine/src/mechanisms/firms/index.ts (and a bank lends against it and a share issue is raised into it) |
| `Capital Programme B3` | MET | packages/engine/src/mechanisms/firms/invest.ts (the gap is the rate it would run at against what its plant will still let it run at next period; a firm running empty has none) |
| `Capital Programme B4` | MET | packages/engine/src/mechanisms/firms/invest.ts (what it commits to is what it would run at LESS the width of its own recent surprises, so a firm whose expectation is inside its own dispersion waits) |
| `Capital Programme B5` | MET | packages/engine/src/mechanisms/firms/invest.ts (no fraction of profit or output anywhere: the only reason anything is bought is that a unit of capacity was worth more than the plant that makes one), packages/engine/test/capital.test.ts (asserted over the module’s declared numbers) |
| `Capital Programme C1` | MET | packages/engine/src/mechanisms/capital-programme/index.ts (what a firm BOUGHT of a capital good becomes plant; the seller is a named capital-goods producer and it is that seller’s revenue). 13c.1 adds `buildOn`: what erecting plant on a place takes, share-weighted over its ground — read and waiting for 13c.2's project to ask it |
| `Capital Programme C2` | MET | packages/engine/src/mechanisms/firms/invest.ts (a purchase order in the capital good’s market, paid out of its account in a currency like any other trade) |
| `Capital Programme C3` | MET | packages/engine/src/mechanisms/capital-programme/index.ts (a build lag between the machines arriving and the vintage going into service, declared as the capital kind’s own technology) |
| `Capital Programme C4` | MET | packages/engine/src/mechanisms/capital-programme/index.ts (the good is destroyed into the plant in the same instruction: the money went to the producer and nothing can turn it back) |
| `Capital Programme D1` | MET | packages/engine/src/mechanisms/capital-programme/plant.ts, packages/engine/src/mechanisms/firms/decide.ts (capital next period is capital now plus what was commissioned less what wears out, and the decision is measured against it) |
| `Capital Programme D2` | MET | packages/engine/src/mechanisms/capital-programme/capacity.ts (the aggregate is a sum over the vintages firms hold; nothing stores one) |
| `Capital Programme D3` | PARTIAL | a dead firm’s plant goes to its estate and is offered into the vintage’s own market (packages/engine/src/mechanisms/estate/index.ts), and a firm that can use it bids what the service LEFT in it is worth to it, from its own view (packages/engine/src/mechanisms/firms/invest.ts). What is not shown is a completed sale out of an estate in this world’s own run: a bidder needs a gap at the moment the estate is selling, and whether the two coincide is an outcome. Measuring it is Part XII (worklist 16) |
| `Capital Programme D4` | MET | packages/engine/src/mechanisms/capital-programme/capacity.ts, packages/engine/src/mechanisms/firms/produce.ts (utilisation is a read of the outcome against capacity, taken where the outcome is and used by nothing) |
| `Capital Programme E1` | MET | packages/engine/src/mechanisms/firms/invest.ts (the order is demand in the capital good’s market in the period it is placed, and the capacity arrives after the build lag) |
| `Capital Programme E2` | MET | packages/engine/src/mechanisms/firms/data.ts, packages/engine/src/mechanisms/labour/data.ts (the capital-goods line employs people in a venue of its own, and its hours are bought by somebody else’s decision to expand) |
| `Capital Programme E3` | MET | packages/engine/src/mechanisms/firms/index.ts (what it cannot fund out of cash is a programme, and a programme is what a bank lends into: investment drives the credit it is funded by) |
| `Capital Programme E4` | PARTIAL | the chain is built and asserted by direction in packages/engine/test/capital.test.ts (a dearer cost of capital means a dearer quote, less investment, less capacity and less output after the build lag). Measuring the size of it, and over the run ladder, is Part XII (worklist 16) |
| `Capital Programme F1` | PARTIAL | a project for a line the firm does not yet make is the same arithmetic and the same market, but a firm’s line-up is registry data until something can change it: entering a line is a decision that needs firm birth and corporate control (worklist 13g). There is no opening-stock stand-in to delete: the work in progress the seed states is Seed D1’s stock for the flows that act on it, not F1.b’s stand-in for entry |

## Firm Birth

| requirement | status | where / why |
|---|---|---|
| `Firm Birth A1` | MISSING |  |
| `Firm Birth A2` | MISSING |  |
| `Firm Birth A2.a` | MISSING |  |
| `Firm Birth A3` | MISSING |  |
| `Firm Birth A4` | MISSING |  |
| `Firm Birth A5` | MISSING |  |
| `Firm Birth B1` | MISSING |  |
| `Firm Birth B2` | MISSING |  |
| `Firm Birth B3` | MISSING |  |
| `Firm Birth B4` | MISSING |  |
| `Firm Birth C1` | PARTIAL | packages/engine/src/registry/kinds.ts (the definition is the instrument's own and is observable by a holder: it is journalled publicly with the words it met). A firm's own definition needs a firm with debt (worklist 6) |
| `Firm Birth C2` | MET | packages/engine/src/world/actions.ts, packages/engine/src/mechanisms/credit-events/index.ts (a default is what is left when a payment out of a party's own balance did not go through; there is no other way to produce one) |
| `Firm Birth C2.a` | MET | packages/engine/src/world/actions.ts (a default exists only where a payment was applied and failed; there is no hazard rate, no draw and no assignment anywhere in the path), packages/engine/test/default-events.test.ts (and a world whose payments all settle produces none) |
| `Firm Birth C3` | PARTIAL | packages/engine/src/world/actions.ts, packages/engine/src/mechanisms/credit-events/index.ts (the event is public, so anybody may react to it, and it names who failed, on what, to whom and for how much). What it triggers — a lender's loss, a rating action — arrives with lending (worklist 6) and ratings (worklist 12) |
| `Firm Birth C4` | PARTIAL | packages/engine/src/mechanisms/credit-events/index.ts (every default carries the instruction that failed and what it was for, so the cash failure behind it is traceable from the event). A default caused by solvency rather than cash needs the other failure (D4, worklist 6) |
| `Firm Birth D1` | MET | packages/engine/src/mechanisms/estate/index.ts (the estate sells what it holds into the market that thing always traded in, at a reservation that falls as its programme runs out and takes what the book gives on the last period; what nobody bought is abandoned by a destroy leg) |
| `Firm Birth D2` | PARTIAL | packages/engine/src/mechanisms/estate/index.ts (proceeds distributed in rank order, pro rata within a rank, by the instrument's own stated seniority; a partial payment redeems that much at par and the rest stays outstanding). D2.b's trade creditors need payables (worklist 13e) and D2.c's close-out claims need the derivative layer (13a); severance owed by a dead employer is recorded owed and unranked for the same reason (packages/engine/src/mechanisms/labour/matching.ts) |
| `Firm Birth D3` | MET | packages/engine/src/mechanisms/estate/index.ts (what is never paid is written off against the holder at what it fetched, which was nothing, so the loss lands on the named holders in proportion to what each was owed), packages/engine/test/estate.test.ts |
| `Firm Birth D4` | PARTIAL | packages/engine/src/mechanisms/labour/matching.ts (its employees lose their jobs, through the labour market's own separation path and never by a headcount going down — D4.a). Its suppliers' receivables need payables (worklist 13e) and its capital going to a buyer needs kinds of capital (worklist 10) |
| `Firm Birth D5` | MET | packages/engine/src/mechanisms/estate/index.ts (everything it held moves to the estate by instruction, everything it issued is assumed by the estate over the wire, and then it ceases naming the estate; the estate ends the chain and resolving still terminates), packages/engine/src/parties/party.ts |
| `Firm Birth D6` | MET | packages/engine/src/mechanisms/estate/index.ts (nothing leaves an estate except to somebody holding a claim on it, so what the assets fetched is what the claimants got and the rest is the loss; and no dead party keeps anything, in any account it held — D6.a), packages/engine/test/estate.test.ts |
| `Firm Birth E1` | MET | packages/engine/src/registry/profiles.ts, packages/engine/src/mechanisms/estate/index.ts (every kind states what it can fail on, and a firm states both; the two exceptions — the central bank and a treasury in its own money — are named consequences and not omissions) |
| `Firm Birth E2` | PARTIAL | packages/engine/src/mechanisms/estate/index.ts (every asset moves to the estate, every liability is assumed by it, and every employee is separated through the labour market). A contract that is not an instrument has no destination yet, because there are none: payables are worklist 13e and derivatives 13a |
| `Firm Birth E3` | MISSING |  |
| `Firm Birth E4` | MISSING |  |

## M&A

| requirement | status | where / why |
|---|---|---|
| `M&A A1` | MISSING |  |
| `M&A A2` | MISSING |  |
| `M&A A3` | MISSING |  |
| `M&A A4` | MISSING |  |
| `M&A A5` | MISSING |  |
| `M&A B1` | MISSING |  |
| `M&A B2` | MISSING |  |
| `M&A B3` | MISSING |  |
| `M&A B4` | MISSING |  |
| `M&A B5` | MISSING |  |
| `M&A C1` | MISSING |  |
| `M&A C2` | MISSING |  |
| `M&A C3` | MISSING |  |
| `M&A C4` | MISSING |  |
| `M&A D1` | MISSING |  |
| `M&A D2` | MISSING |  |
| `M&A D3` | MISSING |  |
| `M&A D4` | MISSING |  |
| `M&A D5` | MISSING |  |
| `M&A E1` | MISSING |  |
| `M&A E2` | MISSING |  |
| `M&A E3` | MISSING |  |

## Trade Credit

| requirement | status | where / why |
|---|---|---|
| `Trade Credit A1` | MISSING |  |
| `Trade Credit A2` | MISSING |  |
| `Trade Credit A3` | MISSING |  |
| `Trade Credit A4` | MISSING |  |
| `Trade Credit B1` | MISSING |  |
| `Trade Credit B2` | MISSING |  |
| `Trade Credit B3` | MISSING |  |
| `Trade Credit B4` | MISSING |  |
| `Trade Credit B5` | MISSING |  |
| `Trade Credit C1` | MISSING |  |
| `Trade Credit C2` | MISSING |  |
| `Trade Credit C3` | MISSING |  |
| `Trade Credit C4` | MISSING |  |
| `Trade Credit D1` | MISSING |  |
| `Trade Credit D2` | MISSING |  |
| `Trade Credit D3` | MISSING |  |
| `Trade Credit D3.a` | MISSING |  |
| `Trade Credit D4` | MISSING |  |
| `Trade Credit D5` | MISSING |  |
| `Trade Credit E1` | MISSING |  |
| `Trade Credit E2` | MISSING |  |
| `Trade Credit E3` | MISSING |  |

## Goods

| requirement | status | where / why |
|---|---|---|
| `Goods A1` | MET | packages/engine/src/mechanisms/goods/data.ts, packages/engine/src/mechanisms/goods/recipes.ts, packages/engine/src/mechanisms/goods/inventory.ts |
| `Goods A2` | MET | packages/engine/src/mechanisms/goods/recipes.ts (the recipe is the good own public technology in physical quantities), packages/engine/src/mechanisms/firms/produce.ts (and the firm draws exactly it) |
| `Goods A2.b` | MET | packages/engine/src/mechanisms/goods/index.ts (refused at assembly by the declared unit), tools/eslint-rules/index.js (phoenix/no-value-recipe) |
| `Goods A3` | MET | packages/engine/src/mechanisms/goods/data.ts, packages/engine/src/mechanisms/goods/inventory.ts |
| `Goods A4` | MET | packages/engine/src/mechanisms/goods/index.ts (one instrument per region and sub-unit) |
| `Goods B1` | MET | packages/engine/src/mechanisms/firms/decide.ts (expected demand, margin, inputs and labour are the reasons; the quantity is what comes out of them) |
| `Goods B1.d` | MET | packages/engine/src/mechanisms/capital-programme/capacity.ts, packages/engine/src/mechanisms/firms/produce.ts (utilisation is a read of the outcome against capacity, taken where the outcome is; no decision reads it) |
| `Goods B2` | MET | packages/engine/src/mechanisms/firms/produce.ts, packages/engine/src/mechanisms/goods/index.ts (the audit contribution: what a batch consumed IS its recipe) |
| `Goods B3` | MET | packages/engine/src/mechanisms/goods/inventory.ts (work in progress is a kind of its own, carried at what it has cost), packages/engine/src/mechanisms/firms/produce.ts |
| `Goods B4` | MET | packages/engine/src/mechanisms/firms/produce.ts (what is started is not what is finished; the scrap is units and the whole batch cost lands on the survivors — and 13c.1 put the GROUND into the same exponent the season enters, so where a line stands is part of what it yields) |
| `Goods B5` | MET | packages/engine/src/mechanisms/firms/produce.ts (inputs consumed plus the period wage bill), packages/engine/src/mechanisms/capital-programme/capacity.ts (plus a capital charge: the plant a unit takes times what a unit of that plant wears out by, which is the same schedule the stock is written down on) |
| `Goods C1` | MET | packages/engine/src/mechanisms/firms/decide.ts (a seller offers what it holds in steps with a reason behind each; a buyer posts what the thing is worth to it) |
| `Goods C2` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/prices/price-store.ts, packages/engine/src/mechanisms/goods/index.ts |
| `Goods C3` | MET | packages/engine/src/mechanisms/firms/decide.ts (firms bid what an input is worth to them), packages/engine/src/mechanisms/households/consume.ts (households bid a curve out of what they decided to spend), packages/engine/src/mechanisms/treasury/index.ts (and the state buys with a budget and is rationed with everybody else) |
| `Goods C4` | MET | packages/engine/src/mechanisms/goods/index.ts (pro rata, stated once for every goods market), packages/engine/src/clearing/solver.ts |
| `Goods C5` | MET | packages/engine/src/clearing/solver.ts (nothing is added to either side, so what nobody bought stays where it was) |
| `Goods C6` | PARTIAL | packages/engine/src/mechanisms/goods/index.ts clears each good in the money of the region it is in; a foreign buyer buying that money arrives with the currency layer (worklist 12) |
| `Goods D1` | MISSING |  |
| `Goods D2` | MISSING |  |
| `Goods D3` | MISSING |  |
| `Goods D4` | MISSING |  |
| `Goods D5` | MISSING |  |
| `Goods E1` | MET | packages/engine/src/register/register.ts (lots), packages/engine/src/prices/value.ts |
| `Goods E2` | MET | packages/engine/src/mechanisms/goods/inventory.ts, packages/engine/src/world/revalue.ts |
| `Goods E2.c` | MET | packages/engine/src/world/revalue.ts (the kernel refuses an upward move for a kind that is not marked both ways) |
| `Goods E3` | MET | packages/engine/src/world/revalue.ts, packages/engine/src/mechanisms/goods/inventory.ts (the write-down and the spoilage are journalled events with a size) |
| `Goods E4` | MET | packages/engine/src/mechanisms/goods/inventory.ts, packages/engine/src/ledger/instruction.ts |
| `Goods E5` | MET | packages/engine/src/register/register.ts, packages/engine/src/registry/registry.ts (the one stated lot flow is FIFO; the type admits no LIFO) |
| `Goods F1` | MET | packages/engine/src/clearing/market.ts (the buyer pays the seller by name, in the same instruction as the goods) |
| `Goods F2` | PARTIAL | immediate: the goods and the money settle together; terms arrive with trade credit (worklist 13e) |
| `Goods F3` | MISSING |  |
| `Goods F4` | MISSING |  |
| `Goods F5` | MET | packages/engine/src/clearing/market.ts (recognised on delivery), packages/engine/src/mechanisms/goods/inventory.ts (cost of what left, at what it cost) |
| `Goods F5.b` | MET | packages/engine/src/mechanisms/firms/produce.ts (the wage is paid once and capitalised once: a period that starts nothing capitalises nothing) |
| `Goods G1` | MISSING |  |
| `Goods G1.c` | MISSING |  |
| `Goods G2` | MISSING |  |
| `Goods G3` | MISSING |  |
| `Goods G4` | MET | packages/engine/src/mechanisms/capital-programme/capacity.ts (a read of the outcome against capacity), packages/engine/src/mechanisms/firms/produce.ts (published where the outcome is) |

## Freight

| requirement | status | where / why |
|---|---|---|
| `Freight A1` | MET | packages/engine/src/mechanisms/freight/index.ts (a service: moving a quantity from one place to another over a time; the leg is read off the map, packages/engine/src/registry/geography.ts) |
| `Freight A2` | MET | packages/engine/src/mechanisms/freight/index.ts (bought by a named shipper from a named carrier, at a price, in a currency) |
| `Freight A3` | MET | packages/engine/src/register/voyages.ts (a voyage is a row with a position; the cargo is on the shipper’s own book the whole way — A3.a’s working capital) |
| `Freight A4` | MET | packages/engine/src/registry/geography.ts (legsBetween: every leg is a different length over different ground, so capacity on one is not capacity on another — the four declared route numbers are deleted) |
| `Freight B1` | MET | packages/engine/src/mechanisms/freight/index.ts (a carrier is a named FIRM owning hulls, which are plant with a life) |
| `Freight B2` | MET | packages/engine/src/mechanisms/freight/index.ts (roomOf: what a carrier offers is the hulls it has FREE — a hull on a voyage is liened and the register refuses to move encumbered units) |
| `Freight B3` | MET | packages/engine/src/mechanisms/freight/index.ts (costOf: the hull used up per unit-kilometre and the crew paid per day, out of the leg the map gave) |
| `Freight B4` | MET | packages/engine/src/mechanisms/freight/index.ts (the sail phase: a voyage comes as far as the weather AT THE PLACE IT IS IN let it, exp(-(wind/what that ground stands)^hardness), never a threshold) |
| `Freight C1` | MET | packages/engine/src/mechanisms/freight/index.ts (demand is derived: a shipper wants room because it holds something worth more elsewhere) |
| `Freight C2` | PARTIAL | the shipper holds or does not trade; SOURCE LOCALLY needs a second place that makes the thing — 13c step 10 |
| `Freight C3` | MET | packages/engine/src/mechanisms/freight/index.ts (toShip/shippers: read off two prints both already made, never a series) |
| `Freight D1` | MET | packages/engine/src/mechanisms/freight/index.ts (it clears per leg, one solver, like every market) |
| `Freight D2` | MET | packages/engine/src/mechanisms/freight/index.ts (the freight is part of what the cargo lands costing: the create leg carries it in the basis) |
| `Freight D3` | PARTIAL | the mechanism is here — two places, two prints, a real cost to move between them — but the BASIS is measured at 13c step 9 |
| `Freight D4` | MET | packages/engine/src/register/voyages.ts (transit is a real lag and it is not a number: kmTravelled against the weather the voyage met) |
| `Freight D5` | PARTIAL | measured at 13c step 9, which is what this item was inserted before |
| `Freight D6` | MET | packages/engine/src/mechanisms/freight/index.ts (capacity rations quantity: what is not carried stays where it is, and the session says so) |
| `Freight E1` | MET | packages/engine/src/register/voyages.ts (a voyage covers a real distance and takes real time; settlement refuses one that does not) |
| `Freight E2` | MET | packages/engine/src/register/voyages.ts (no shipment without capacity, and none without a carrier that owns it: the lien is what enforces it) |
| `Freight E3` | MET | packages/engine/src/mechanisms/freight/index.ts (every unit in transit is the shipper’s, the whole way; what a storm takes leaves its book by the same instruction) |

## Labour

| requirement | status | where / why |
|---|---|---|
| `Labour A1` | MET | packages/engine/src/mechanisms/labour/index.ts, packages/engine/src/mechanisms/labour/matching.ts (hours of a person time, supplied by a named cell to a named firm) |
| `Labour A2` | MET | packages/engine/src/mechanisms/labour/index.ts (the venue prices hours in the money of its region) |
| `Labour A3` | MET | packages/engine/src/mechanisms/labour/data.ts, packages/engine/src/mechanisms/labour/index.ts (one venue per region and occupation; a trade is what a seeker looks for) |
| `Labour A4` | MET | packages/engine/src/mechanisms/labour/register.ts (a relationship with a firm, a worker, a wage and a start date) |
| `Labour B1` | MET | packages/engine/src/mechanisms/labour/matching.ts (each cell decides from its own view whether to offer its hours) |
| `Labour B2` | MET | packages/engine/src/mechanisms/labour/matching.ts (supply is the cells weights times the hours a person has) |
| `Labour B3` | MET | packages/engine/src/mechanisms/labour/index.ts (employed, unemployed or inactive, one state each, read from the rows and the cohort) |
| `Labour B4` | MET | packages/engine/src/mechanisms/labour/matching.ts (an unemployed cell posts every period; what it meets is finite) |
| `Labour B5` | MET | packages/engine/src/mechanisms/labour/index.ts (the audit contribution: the three states against the population) |
| `Labour C1` | MET | packages/engine/src/mechanisms/firms/decide.ts (it hires when an hour adds more than an hour costs; what it offers is what an hour is worth to it, from its own outlook of its output price) |
| `Labour C2` | PARTIAL | packages/engine/src/mechanisms/labour/register.ts carries the lag from the match to the day the person is productive, and the firm plans on the hours it already has (packages/engine/src/mechanisms/firms/decide.ts); the cost of finding somebody arrives with the search side (worklist 13d) |
| `Labour C3` | MET | packages/engine/src/mechanisms/labour/matching.ts (severance paid to the people separated, out of the employer account) |
| `Labour C4` | MET | packages/engine/src/mechanisms/labour/matching.ts (an employer that has ceased releases its workers at once, through the same separation path as any other separation; a merger that removes jobs is worklist 13g) |
| `Labour C5` | MET | packages/engine/src/mechanisms/labour/matching.ts (a vacancy is a posting the employer owns, for the period it posts it) |
| `Labour D1` | MET | packages/engine/src/mechanisms/labour/matching.ts (every posting is a bid, the highest fill first, and THE BID THAT TOOK THE LAST MATCH IS THE PRINT — read off the book rather than taken from the crossing, which in a slack market sits on a seeker's reservation and is a level no employer offered), packages/engine/src/mechanisms/labour/index.ts (the prices family checks every print against the bids that session) |
| `Labour D2` | MET | packages/engine/src/mechanisms/labour/register.ts (the wage is the contract and does not move with the print), packages/engine/src/mechanisms/firms/decide.ts (a firm that wants fewer hours than it has sheds them and pays severance) |
| `Labour D2.b` | MET | packages/engine/src/mechanisms/labour/matching.ts, packages/engine/src/mechanisms/firms/decide.ts (stickiness is the contract and the severance a change costs; nothing damps a series) |
| `Labour D3` | MET | packages/engine/src/mechanisms/labour/matching.ts (whole people are matched from a queue of seekers; hours that do not make a person are no hire) |
| `Labour D4` | MISSING |  |
| `Labour D5` | MISSING |  |
| `Labour E1` | MET | packages/engine/src/mechanisms/labour/matching.ts (the wage reaches the household cell every period) |
| `Labour E2` | MET | packages/engine/src/mechanisms/firms/decide.ts (the wage is in what a unit costs and therefore in what the firm will make and offer) |
| `Labour E3` | MET | packages/engine/src/mechanisms/treasury/index.ts (the income base is what named payers actually paid a household, wages included, remitted out of the household's own account) |
| `Labour E4` | MISSING |  |
| `Labour F1` | MET | packages/engine/src/mechanisms/labour/index.ts (the audit contribution: every row is a job at a named employer that exists), packages/engine/src/mechanisms/treasury/index.ts (the state employs on the same rows, in the same venue, and its wage leaves its own account like anybody else's) |
| `Labour F2` | MET | packages/engine/src/mechanisms/labour/index.ts (headcount is a count of people and never exceeds the population) |
| `Labour F3` | MET | packages/engine/src/mechanisms/labour/index.ts (unemployment is a read of the cells with no row; no rate exists anywhere) |

## Housing

| requirement | status | where / why |
|---|---|---|
| `Housing A1` | MISSING |  |
| `Housing A2` | MISSING |  |
| `Housing A3` | MISSING |  |
| `Housing A4` | MISSING |  |
| `Housing A5` | MISSING |  |
| `Housing B1` | MISSING |  |
| `Housing B2` | MISSING |  |
| `Housing B3` | MISSING |  |
| `Housing B4` | MISSING |  |
| `Housing B4.a` | MISSING |  |
| `Housing B5` | MISSING |  |
| `Housing C1` | MISSING |  |
| `Housing C2` | MISSING |  |
| `Housing C3` | MISSING |  |
| `Housing C4` | MISSING |  |
| `Housing C5` | MISSING |  |
| `Housing C6` | MISSING |  |
| `Housing D1` | MISSING |  |
| `Housing D2` | MISSING |  |
| `Housing D3` | MISSING |  |
| `Housing D4` | MISSING |  |
| `Housing D5` | MISSING |  |
| `Housing E1` | MISSING |  |
| `Housing E2` | MISSING |  |
| `Housing E3` | MISSING |  |
| `Housing E4` | MISSING |  |

## Households

| requirement | status | where / why |
|---|---|---|
| `Households A1` | MET | packages/engine/src/mechanisms/households/index.ts (it earns, decides what to spend, saves what is left and owns what it bought) |
| `Households A2` | PARTIAL | packages/engine/src/mechanisms/households/consume.ts decides per cell from that cell own income, cash, holdings and surprises, so cells with different histories decide differently; life stage is a cohort and employment state is a row, and a cell that borrows arrives with credit (worklist 6) |
| `Households A2.d` | MET | packages/engine/src/mechanisms/households/index.ts (every decision is one cell own, and there is no sector anywhere for one to be taken at) |
| `Households A2.e` | MET | packages/engine/src/parties/party.ts, packages/engine/src/registry/profiles.ts |
| `Households A2.f` | MET | packages/engine/src/parties/party.ts |
| `Households A2.g` | PARTIAL | packages/engine/test/households.test.ts shows the half that can be shown: a mean-preserving spread widens what the cells decide while the mean of what they were paid does not move, and NOT ONE CELL decides at that mean. The COUNT OF CROSSINGS still cannot be measured — not because the sector is an average but because it is wholly on one side of every threshold it has. Item 8 found one in the lumpiness of the small paper holding a cell could afford; item 10 put a bank's own capital into what it requires to hold paper, the money fund became where a cell's spare money actually goes, and the sector is comfortably on one side of that. The first threshold that will straddle is a default (worklist 5) or a household that borrows (13d) |
| `Households A3` | MET | packages/engine/src/seeds/foundation.ts, packages/engine/src/parties/party.ts (a cell is a named party with an account and a register) |
| `Households B1` | MET | packages/engine/src/mechanisms/labour/matching.ts (the wage leaves a named employer account and reaches the cell) |
| `Households B2` | MET | packages/engine/src/mechanisms/treasury/index.ts (the standing mandate reaches each cell by name) |
| `Households B3` | PARTIAL | coupons on the paper it holds reach it through corporate actions (packages/engine/src/world/actions.ts); dividends arrive with equity (worklist 9) |
| `Households B3.a` | MET | packages/engine/src/mechanisms/households/consume.ts (what it acts on is what reached its account and what its holdings are worth, never anything retained on its behalf) |
| `Households B4` | MET | packages/engine/src/mechanisms/treasury/index.ts (income and consumption are taxed on what the payer itself did, and remitted out of its own account) |
| `Households B5` | MET | packages/engine/src/mechanisms/households/index.ts (the sector income is published as a sum of what named payers paid, read from the ledger and causing nothing) |
| `Households C1` | MET | packages/engine/src/mechanisms/households/consume.ts (its own expected income, what it owns, its own recent surprises and the cash it can actually pay with) |
| `Households C2` | MET | packages/engine/src/mechanisms/households/portfolio.ts (what it neither spends nor puts into paper stays in its account) |
| `Households C3` | MET | packages/engine/src/mechanisms/households/data.ts (the shares are a cohort preference), packages/engine/src/mechanisms/households/consume.ts (what that buys is the price business) |
| `Households C4` | MET | packages/engine/src/mechanisms/households/consume.ts (it finds the consumption tax on top of the price when it decides what to spend) |
| `Households C5` | MET | packages/engine/src/mechanisms/households/index.ts (the audit contribution: what a household took is what it paid a named seller for) |
| `Households D1` | PARTIAL | deposits and securities held directly, in the register (packages/engine/src/mechanisms/households/portfolio.ts); fund shares arrive at worklist 8, pensions at 13h and housing at 13d |
| `Households D2` | MISSING |  |
| `Households D3` | MET | packages/engine/src/audit/families/accounts.ts |
| `Households D4` | MISSING |  |
| `Households D5` | PARTIAL | packages/engine/src/mechanisms/households/portfolio.ts weighs yield against liquidity — what a saver requires of paper for giving up access, against what a deposit returns; risk needs something that prices it (worklist 9) |
| `Households D6` | MET | packages/engine/src/mechanisms/households/index.ts (a cell bids in the markets it is in and is nobody residual holder) |
| `Households E1` | MISSING |  |
| `Households E2` | MISSING |  |
| `Households E3` | MISSING |  |
| `Households E4` | MISSING |  |
| `Households E5` | MISSING |  |
| `Households F1` | MISSING |  |
| `Households F2` | MISSING |  |
| `Households F3` | MISSING |  |
| `Households F4` | MISSING |  |

## Small-Business Pools

| requirement | status | where / why |
|---|---|---|
| `Small-Business Pools A1` | MISSING |  |
| `Small-Business Pools A2` | MISSING |  |
| `Small-Business Pools A2.a` | MISSING |  |
| `Small-Business Pools A3` | MISSING |  |
| `Small-Business Pools A4` | MISSING |  |
| `Small-Business Pools A5` | MISSING |  |
| `Small-Business Pools A6` | MET | packages/engine/src/parties/party.ts, packages/engine/src/registry/profiles.ts |
| `Small-Business Pools B1` | MISSING |  |
| `Small-Business Pools B2` | MISSING |  |
| `Small-Business Pools B3` | MISSING |  |
| `Small-Business Pools B4` | MISSING |  |
| `Small-Business Pools C1` | MISSING |  |
| `Small-Business Pools C2` | MISSING |  |
| `Small-Business Pools C3` | MISSING |  |
| `Small-Business Pools C4` | MISSING |  |
| `Small-Business Pools C5` | MISSING |  |
| `Small-Business Pools C6` | MISSING |  |
| `Small-Business Pools D1` | MISSING |  |
| `Small-Business Pools D2` | MISSING |  |
| `Small-Business Pools D3` | MISSING |  |
| `Small-Business Pools D4` | MISSING |  |
| `Small-Business Pools D4.a` | MISSING |  |
| `Small-Business Pools E1` | MISSING |  |
| `Small-Business Pools E2` | MISSING |  |
| `Small-Business Pools E3` | MISSING |  |
| `Small-Business Pools E4` | MISSING |  |
| `Small-Business Pools E5` | MET | packages/engine/src/audit/families/units.ts, packages/engine/src/world/cells.ts |
| `Small-Business Pools E6` | MISSING |  |

## Cross-Border

| requirement | status | where / why |
|---|---|---|
| `Cross-Border A1` | PARTIAL | packages/engine/src/seeds/foundation.ts — four regions, each with its own money, its own central bank and its own sovereign borrower. The three abroad have no firms, no households and no labour market: a real economy there is 13i’s |
| `Cross-Border A2` | MET | packages/engine/src/seeds/foundation.ts (a holder in one region holds another’s paper, and the coupon crosses the border in the issuer’s money) |
| `Cross-Border A3` | MET | packages/engine/src/world/revalue.ts, packages/engine/src/audit/families/accounts.ts (what a holder abroad is worth at home moves with the rate, and the balance sheet reads it in the holder’s own money) |
| `Cross-Border A4` | PARTIAL | packages/engine/src/mechanisms/spot-fx — the capital flow that exists is the one the cross holdings and the coupons produce. Trade invoiced in another country’s money, and a portfolio decision to hold abroad, are 13i and 13h |
| `Cross-Border B1` | MISSING |  |
| `Cross-Border B2` | MISSING |  |
| `Cross-Border B3` | MISSING |  |
| `Cross-Border B4` | MISSING |  |
| `Cross-Border C1` | MISSING |  |
| `Cross-Border C2` | MISSING |  |
| `Cross-Border C3` | MISSING |  |
| `Cross-Border C4` | MISSING |  |
| `Cross-Border C5` | MISSING |  |
| `Cross-Border D1` | MISSING |  |
| `Cross-Border D2` | MISSING |  |
| `Cross-Border D3` | MISSING |  |
| `Cross-Border D4` | MISSING |  |
| `Cross-Border D5` | MISSING |  |
| `Cross-Border D6` | MISSING |  |
| `Cross-Border E1` | MISSING |  |
| `Cross-Border E2` | MISSING |  |
| `Cross-Border E3` | MISSING |  |
| `Cross-Border E4` | MISSING |  |
| `Cross-Border F1` | MISSING |  |
| `Cross-Border F2` | MISSING |  |
| `Cross-Border F3` | MISSING |  |

## Ratings

| requirement | status | where / why |
|---|---|---|
| `Ratings A1` | MET | packages/engine/src/mechanisms/ratings/index.ts (a grade is a named assessor’s opinion, published as its own action) |
| `Ratings A2` | MET | packages/engine/src/mechanisms/ratings/assess.ts (from the issuer’s own public state: what falls due against what it is worth, and whether it has missed a payment) |
| `Ratings A2.a` | MET | packages/engine/src/world/world.ts (blindView: the assessor decides from a view whose prints, marks, indices and curves are all closed — structurally, not by convention) |
| `Ratings A3` | MET | packages/engine/src/mechanisms/ratings/assess.ts, packages/engine/src/mechanisms/ratings/index.ts (seven coarse bands, and the state must stay across a boundary for the assessor’s own patience) |
| `Ratings A4` | MET | packages/engine/src/mechanisms/ratings/index.ts (every move is published with what it was, and with the measure behind it) |
| `Ratings A5` | MET | packages/engine/src/mechanisms/ratings/index.ts (the rated party pays the assessor every period, by instruction, from its own account — the conflict kept and priced) |
| `Ratings B1` | MET | packages/engine/src/mechanisms/ratings/assess.ts (the measure, and the band it falls in on this assessor’s own geometrically widening scale) |
| `Ratings B2` | MET | packages/engine/src/mechanisms/ratings/assess.ts (an instrument’s grade is its issuer’s, moved by where the line stands in the queue) |
| `Ratings B3` | MET | packages/engine/src/mechanisms/ratings/data.ts (each assessor’s thresholds and patience are its own, drawn and declared under its own name) |
| `Ratings C1` | PARTIAL | packages/engine/src/mechanisms/funds/tracker.ts — a mandate that refers to an INDEX is built; one written in grades wants a holder of rated paper, which is 13f’s |
| `Ratings C2` | PARTIAL | packages/engine/src/mechanisms/ratings/index.ts declares a risk weight per grade as policy; the bank that reads it into its own capital rule is 13f’s rated exposure |
| `Ratings C3` | MISSING |  |
| `Ratings C4` | MISSING |  |
| `Ratings C5` | MET | packages/engine/src/mechanisms/ratings/index.ts (a rating action is public, so a participant’s outlook may observe it like anything else) |
| `Ratings D1` | MET | packages/engine/src/mechanisms/ratings/index.ts (everybody who borrows on its own credit is rated, the state included) |
| `Ratings D2` | MET | packages/engine/src/mechanisms/ratings/index.ts (the subjects are the party kinds that borrow, asked of the registry rather than listed) |
| `Ratings D3` | MISSING |  |
| `Ratings D4` | MISSING |  |
| `Ratings D5` | MET | packages/engine/src/mechanisms/ratings/data.ts (three assessors, so a world has a spread of opinions rather than an opinion) |
| `Ratings E1` | MET | packages/engine/src/mechanisms/ratings/index.ts, packages/engine/src/observer/observer.ts (published where anybody may read it) |
| `Ratings E2` | MET | packages/engine/src/mechanisms/ratings/index.ts (what moved a grade is in the action: the measure, and what it was before) |
| `Ratings E3` | PARTIAL | the grades and the defaults that follow them are both public and both journalled; the scenario that shows a rated-safe issuer defaulting is a Part XII measurement |
| `Ratings E4` | MET | packages/engine/src/observer/observer.ts (the distribution of grades is a read of the published actions) |

## Reporting

| requirement | status | where / why |
|---|---|---|
| `Reporting A1` | MET | packages/engine/src/mechanisms/reporting/index.ts (`publish`: every public company, on its own fiscal calendar) with report.ts (`isPublic`, `listedLineOf` — a read of the register every period, no flag and no kind: A1.a) |
| `Reporting A2` | MET | packages/engine/src/mechanisms/reporting/report.ts and index.ts (income from the equity ledger, the balance sheet from the SAME `balanceSheet()` the accounts family checks, the cash from the wire own money legs) |
| `Reporting A2.a` | MET | packages/engine/src/audit/families/accounts.ts (`equityLedgerFamily`: the itemisation against the balance, and the COUNT of entries against the count of moves — a sum can agree by two errors, a count cannot) |
| `Reporting A3` | MET | packages/engine/src/mechanisms/reporting/fiscal.ts (a quarter is a pair of dates; the periods in it are asked of the one calendar and are a whole number only by accident) |
| `Reporting A4` | MET | packages/engine/src/mechanisms/reporting/data.ts (`reporting.lag.days`, POLICY, owner parliament) with fiscal.ts (`publishableOn`); the report is a public journal event and there is no other way out, so the gap is the only asymmetry there is (A4.a) |
| `Reporting A5` | MET | packages/engine/src/mechanisms/reporting/index.ts (`restate`: the entries are append-only, so a published figure changes only by a later entry dated into a reported span — re-read every period, with the original standing) |
| `Reporting B1` | MET | packages/engine/src/mechanisms/reporting/guidance.ts (`guidanceOf`: the firm own `income` outlook, for the coming quarter, in the report own lines) |
| `Reporting B2` | MET | packages/engine/src/mechanisms/reporting/index.ts (`revise`, `guide`: a revision on a move past the arithmetic dust, a withdrawal when the management loses its view — both events with dates, neither on a schedule) |
| `Reporting B3` | MET | packages/engine/src/mechanisms/reporting/guidance.ts (`guidanceRecord`: a read of the journal pairing what a management said with what it then reported; nothing stored) |
| `Reporting B4` | MET | packages/engine/src/mechanisms/reporting/guidance.ts (the published figure IS `view.outlook(income)` — the object the firm own decisions read — with the periodicity stated beside it rather than a second number) |
| `Reporting C1` | MET | packages/engine/src/mechanisms/research/estimate.ts (`estimateFrom`, `seenOf`: an adaptive outlook over what THAT bank observed of THAT company, at its own drawn memory) |
| `Reporting C2` | MET | packages/engine/src/mechanisms/research/index.ts (`research.estimate`, public, naming the bank and the company) |
| `Reporting C3` | MET | packages/engine/src/mechanisms/research/data.ts (`memoryOf`, drawn per bank) — measured: consensus counts of 1 to 3 with real spreads, nobody dispersing them |
| `Reporting C4` | MET | packages/engine/src/mechanisms/research/index.ts (a desk reads only what was published since it last spoke, so a view that has seen nothing new does not move) |
| `Reporting C5` | MET | packages/engine/src/mechanisms/research/estimate.ts (the management guidance is one more observation, weighed by that management own record, never the answer) |
| `Reporting C6` | MET | tools/check-forbids.ts (no read of a print, a mark or the price store anywhere in mechanisms/research — a CHECK, because a test cannot see this break) |
| `Reporting D1` | MET | packages/engine/src/mechanisms/research/index.ts (`needsTheView`: it holds something the company issued, or its own desk makes a market in the line) |
| `Reporting D2` | MET | packages/engine/src/mechanisms/research/index.ts (`pay`) with data.ts (`research.hoursPerName`, TECHNOLOGY) — costed at what the labour venue cleared at and paid per member to named cells |
| `Reporting D3` | MET | packages/engine/src/mechanisms/research/index.ts (the count per name is an outcome of what banks books look like; nothing assigns coverage) |
| `Reporting D3.a` | MET | packages/engine/test/research.test.ts (the counts are not all equal and no name is covered by every bank) |
| `Reporting E1` | MET | packages/engine/src/mechanisms/research/index.ts (`consensusOf`: count, mean, spread and the age of the oldest estimate in it, computed at the moment of reading) |
| `Reporting E2` | MET | tools/check-forbids.ts (nothing outside research and the observer calls `consensusOf` at all) |
| `Reporting E3` | MET | packages/engine/src/mechanisms/research/index.ts (`consensusOf` reads the journal and stores nothing) |
| `Reporting F1` | MET | packages/engine/src/mechanisms/research/index.ts (`settle`, anchored BEFORE `cover`, so a surprise is measured against what the bank said before the report) |
| `Reporting F2` | MET | packages/engine/src/mechanisms/research/index.ts (the surprise is an observation and the module writes no price; the chain to a print is the party own outlook and the book) |
| `Reporting F2.a` | MET | tools/check-forbids.ts (no module outside research names `research.surprise`; there is no parameter anywhere whose unit is a price move) |
| `Reporting F3` | MET | packages/engine/src/mechanisms/research/estimate.ts (`missedBy`: a read of this management own past guidance against its own past reports, weighed into what its next guidance is worth) |
| `Reporting F4` | MISSING |  |
| `Reporting G1` | MET | packages/engine/src/mechanisms/research/index.ts and mechanisms/ratings/assess.ts (the reports are read: an estimate is formed from them and an assessor measures coverage against what the issuer takes in) |
| `Reporting G2` | MET | packages/engine/src/register/register.ts (`EquityEntry`) with mechanisms/reporting/report.ts (`incomeOf`: the account movement decomposed into what the instructions and the marks did) |
| `Reporting G3` | PARTIAL | no analyst always right and none always wrong by a fixed amount: the estimates are formed from drawn memories and observed reports, so neither is written anywhere — but whether it HOLDS is a measurement over a long run, which is worklist 16 |
| `Reporting G4` | MET | packages/engine/src/mechanisms/reporting/report.ts (`isPublic`) and mechanisms/research/index.ts (`reported`: there is nothing to estimate about a company that has not published) |
| `Reporting G5` | MET | packages/engine/src/mechanisms/reporting/index.ts (the report carries shares outstanding and income; no per-share figure is stored anywhere) |
| `Reporting G6` | MET | packages/engine/src/mechanisms/reporting/fiscal.ts (nothing counts periods; a quarter is placed by date and nothing in it is finer than a period) |
| `Reporting H1` | PARTIAL | the dispersion of estimates on a name is a read (`consensusOf().spread`); whether it widens after volatile results is a Part XII measurement — worklist 16 |
| `Reporting H2` | PARTIAL | the surprise and the print are both recorded; whether the price moves more on a large surprise is a Part XII measurement — worklist 16 |
| `Reporting H3` | PARTIAL | the count of estimates per name and their dispersion are reads; whether they respond to a company record is a Part XII measurement — worklist 16 |
| `Reporting H4` | PARTIAL | the consensus carries the age of the oldest estimate in it; whether it lags the information that produced it is a Part XII measurement — worklist 16 |

## Observer

| requirement | status | where / why |
|---|---|---|
| `Observer A1` | MET | packages/engine/src/observer/observer.ts, packages/engine/src/prices/price-store.ts, packages/engine/src/world/context.ts |
| `Observer A2` | MET | packages/engine/src/observer/observer.ts, packages/engine/src/world/context.ts |
| `Observer A3` | MET | packages/engine/src/observer/observer.ts (terms, prints and curves), packages/engine/src/mechanisms/treasury/index.ts (the programme it announces), packages/engine/src/mechanisms/firms/produce.ts (what a firm says it expects to earn) |
| `Observer A4` | MET | packages/engine/src/journal/journal.ts, packages/engine/src/observer/observer.ts, packages/engine/src/world/context.ts, packages/engine/src/world/world.ts |
| `Observer A5` | MET | packages/engine/src/mechanisms/households/index.ts (the sector's income, published a period late as a sum of what named payers paid), packages/engine/src/mechanisms/expectations/index.ts (and the dispersion of outlooks, likewise lagged); both are reads and nothing in the engine acts on them |
| `Observer B1` | MET | packages/engine/src/journal/journal.ts, packages/engine/src/observer/observer.ts (a viewer asks for the recent events of the kinds it follows, not only the last N of everything) |
| `Observer B2` | MET | packages/engine/src/journal/journal.ts |
| `Observer B2.a` | MET | packages/engine/src/journal/journal.ts |
| `Observer B3` | MET | packages/engine/src/journal/journal.ts |
| `Observer B4` | MISSING |  |
| `Observer B5` | MISSING |  |
| `Observer C1` | MISSING |  |
| `Observer C2` | MISSING |  |
| `Observer C2.a` | MISSING |  |
| `Observer C3` | MISSING |  |
| `Observer C4` | MISSING |  |
| `Observer D1` | MET | packages/engine/src/journal/journal.ts, packages/engine/src/observer/observer.ts |
| `Observer D2` | MISSING |  |
| `Observer D3` | MET | packages/engine/src/observer/observer.ts |
| `Observer E1` | MET | packages/engine/src/observer/observer.ts |
| `Observer E2` | MISSING |  |
| `Observer E3` | MET | packages/engine/src/observer/observer.ts, packages/engine/src/world/world.ts, packages/app/src/ui/render.ts (the snapshot hands over how many pieces one named unit is and the surface divides where it prints; nothing is converted on the way out of the engine) |
| `Observer F1` | MET | packages/engine/src/observer/observer.ts, packages/engine/src/registry/naming.ts |
| `Observer F2` | MET | packages/engine/src/observer/observer.ts |
| `Observer F3` | MISSING |  |
| `Observer F4` | MET | packages/engine/src/observer/observer.ts |

## Expectations

| requirement | status | where / why |
|---|---|---|
| `Expectations A1` | MET | packages/engine/src/world/context.ts (an outlook carries its number, its unit, its periodicity and how much the party trusts it), packages/engine/src/mechanisms/expectations/index.ts |
| `Expectations A2` | MET | packages/engine/src/mechanisms/expectations/index.ts (formed only from legs the party was a side of and from what happened to its own account) |
| `Expectations A2.b` | MET | packages/engine/src/world/context.ts (the only door answers about self), packages/engine/src/mechanisms/expectations/index.ts (the published aggregate is a lagged read no decision can consult) |
| `Expectations A3` | MET | packages/engine/src/mechanisms/expectations/index.ts (memory is drawn per party and dispersed, so parties with the same history still move differently) |
| `Expectations A4` | MET | packages/engine/src/mechanisms/expectations/index.ts (an outlook is last period outlook corrected towards what happened; nothing runs the world forward) |
| `Expectations A5` | MET | packages/engine/src/world/context.ts (unit and periodicity are part of an outlook) |
| `Expectations B1` | MET | packages/engine/src/mechanisms/expectations/index.ts (corrected towards what it observed, at its own speed) |
| `Expectations B1.b` | MET | packages/engine/src/mechanisms/expectations/index.ts (one preference — memory — and nothing else; confidence is a read of its own surprises) |
| `Expectations B2` | MET | packages/engine/src/mechanisms/expectations/index.ts (observed minus expected, recorded per party and variable, and the only thing that moves an outlook) |
| `Expectations B2.a` | MET | packages/engine/src/mechanisms/expectations/index.ts (the update reads the surprise and nothing else) |
| `Expectations B3` | MET | packages/engine/src/mechanisms/expectations/index.ts (confidence is the width of that party own recent surprises) |
| `Expectations B4` | MET | packages/engine/src/mechanisms/expectations/index.ts (form runs at the top of the period on what the close of the last one recorded) |
| `Expectations B5` | MISSING |  |
| `Expectations C1` | MISSING |  |
| `Expectations C2` | PARTIAL | packages/engine/src/mechanisms/firms/decide.ts (output and hiring read the firm own outlook of what it sells and what it fetches), packages/engine/src/mechanisms/firms/produce.ts (and it publishes it); investment arrives with the capital programme (worklist 10) |
| `Expectations C3` | PARTIAL | packages/engine/src/mechanisms/firms/decide.ts (what a firm offers and what it will pay are its own expectation of the price); the sovereign holders required yield is still a placeholder (worklist 10) |
| `Expectations C4` | MISSING |  |
| `Expectations C5` | MISSING |  |
| `Expectations C6` | MISSING |  |
| `Expectations D1` | MET | packages/engine/src/world/context.ts |
| `Expectations D2` | MET | packages/engine/src/mechanisms/expectations/index.ts (every observation is per member of the party that made it, never a cell total) |
| `Expectations D3` | MET | packages/engine/src/mechanisms/expectations/index.ts (every outlook is scored against what happened, every period it is observed) |
| `Expectations D4` | MET | packages/engine/src/mechanisms/expectations/index.ts (the dispersion aggregate is published with a lag and nothing can read it back) |
| `Expectations E1` | MISSING |  |
| `Expectations E2` | MET | packages/engine/src/mechanisms/expectations/index.ts (the aggregate is a read of outlooks already formed, published about the period that closed) |
| `Expectations E3` | MISSING |  |
| `Expectations E4` | MISSING |  |
