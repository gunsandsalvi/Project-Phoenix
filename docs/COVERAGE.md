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
| `Money A2` | MET | packages/engine/src/core/money.ts |
| `Money A2.b` | MET | packages/engine/src/core/money.ts |
| `Money A3` | MISSING |  |
| `Money A4` | MISSING |  |
| `Money B1` | MISSING |  |
| `Money B1.b` | MISSING |  |
| `Money B2` | MISSING |  |
| `Money B3` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/registry/kinds.ts |
| `Money B3.c` | PARTIAL | refusal is recorded; a lender row for an allowed overdraft arrives with the corridor (worklist 11) |
| `Money C1` | MET | packages/engine/src/ledger/instruction.ts |
| `Money C2` | MET | packages/engine/src/ledger/settlement.ts |
| `Money C2.c` | MET | packages/engine/src/audit/families/money.ts, packages/engine/src/ledger/settlement.ts |
| `Money C3` | MISSING |  |
| `Money C4` | MET | packages/engine/src/audit/families/money.ts, packages/engine/src/ledger/settlement.ts |
| `Money C4.c` | MET | packages/engine/src/audit/families/money.ts, packages/engine/src/audit/memory.ts |
| `Money D1` | MET | packages/engine/src/ledger/instruction.ts, packages/engine/src/ledger/ledger.ts, packages/engine/src/ledger/settlement.ts |
| `Money D2` | MET | packages/engine/src/ledger/instruction.ts, packages/engine/src/prices/value.ts, packages/engine/src/register/instruments.ts, packages/engine/src/registry/profiles.ts, packages/engine/src/seeds/foundation.ts |
| `Money D3` | MET | packages/engine/src/audit/families/flows.ts, packages/engine/src/audit/memory.ts, packages/engine/src/ledger/settlement.ts |
| `Money D4` | MET | packages/engine/src/audit/families/flows.ts, packages/engine/src/ledger/settlement.ts, packages/engine/src/world/context.ts |
| `Money E1` | PARTIAL | a fail is recorded; the default state and its downstream consequences arrive with XI-1 (worklist 5) |
| `Money E2` | MET | packages/engine/src/ledger/ledger.ts, packages/engine/src/ledger/settlement.ts |
| `Money E3` | MET | packages/engine/src/ledger/settlement.ts |
| `Money E4` | PARTIAL | a ceased party is refused by name; re-seating on the estate arrives with XI-8 (worklist 7) |
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
| `Register E3` | PARTIAL | a default converts nothing yet; XI-1 (worklist 5) |
| `Register E4` | PARTIAL | split, buyback and new issue apply through issuance legs; no corporate-action driver yet |
| `Register E5` | PARTIAL | every register event so far moves money; no explicit why-not record for the exceptions |
| `Register F1` | MET | packages/engine/src/register/instruments.ts |
| `Register F2` | MET | packages/engine/src/audit/families/names.ts, packages/engine/src/parties/party.ts |
| `Register F3` | MET | packages/engine/src/audit/families/flows.ts |

## Clearing

| requirement | status | where / why |
|---|---|---|
| `Clearing A1` | MET | packages/engine/src/clearing/market.ts |
| `Clearing A2` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing A3` | MET | packages/engine/src/mechanisms/sovereign-auction/index.ts, packages/engine/src/mechanisms/sovereign-curve/index.ts, packages/engine/src/world/context.ts, packages/engine/src/world/module.ts |
| `Clearing A4` | MET | packages/engine/src/clearing/solver.ts, packages/engine/src/world/context.ts |
| `Clearing B1` | MET | packages/engine/src/clearing/market.ts |
| `Clearing B2` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/mechanisms/sovereign-auction/index.ts, packages/engine/src/mechanisms/sovereign-curve/index.ts, packages/engine/src/world/module.ts |
| `Clearing B3` | PARTIAL | no dealer exists yet (worklist 9) |
| `Clearing B4` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts, packages/engine/src/mechanisms/sovereign-auction/index.ts |
| `Clearing B5` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing C1` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing C2` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing C3` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing C4` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing C4.c` | MET | packages/engine/src/clearing/solver.ts |
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
| `Audit B4` | PARTIAL | family declared and reported as not built |
| `Audit B5` | MET | packages/engine/src/audit/families/accounts.ts, packages/engine/src/ledger/settlement.ts, packages/engine/src/register/register.ts, packages/engine/src/world/revalue.ts |
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
| `Seed B1` | PARTIAL | the foundation seed has one instance of several kinds; populations are cells with weights |
| `Seed B2` | MET | packages/engine/src/parties/party.ts, packages/engine/src/seeds/foundation.ts |
| `Seed B3` | MET | packages/engine/src/parties/party.ts, packages/engine/src/registry/registry.ts, packages/engine/src/seeds/foundation.ts |
| `Seed B4` | PARTIAL | sizes are dispersed by hand in the foundation seed; nothing draws them |
| `Seed B5` | MISSING |  |
| `Seed C1` | MET | packages/engine/src/world/assemble.ts |
| `Seed C2` | MET | packages/engine/src/seeds/foundation.ts |
| `Seed C3` | MET | packages/engine/src/seeds/foundation.ts |
| `Seed C4` | MET | packages/engine/src/seeds/foundation.ts |
| `Seed C5` | MISSING |  |
| `Seed D1` | PARTIAL | coupons are payable from the treasury account; wages and work in progress arrive with worklist 4 |
| `Seed D2` | MISSING |  |
| `Seed D3` | MISSING |  |
| `Seed D4` | MISSING |  |
| `Seed E1` | MISSING |  |
| `Seed E2` | PARTIAL | no reasons exist yet; the seed sets endowments only |
| `Seed E3` | MISSING |  |

## Currency

| requirement | status | where / why |
|---|---|---|
| `Currency A1` | MISSING |  |
| `Currency A2` | MET | packages/engine/src/audit/families/names.ts, packages/engine/src/registry/registry.ts |
| `Currency A3` | MET | packages/engine/src/core/money.ts |
| `Currency A4` | MET | packages/engine/src/core/money.ts |
| `Currency A5` | PARTIAL | one currency in the foundation registry; several are data |
| `Currency B1` | MISSING |  |
| `Currency B2` | MISSING |  |
| `Currency B3` | MISSING |  |
| `Currency B4` | MISSING |  |
| `Currency B5` | MISSING |  |
| `Currency C1` | MISSING |  |
| `Currency C2` | MISSING |  |
| `Currency C3` | MISSING |  |
| `Currency C4` | MISSING |  |
| `Currency C4.a` | MISSING |  |
| `Currency C5` | MISSING |  |
| `Currency D1` | MISSING |  |
| `Currency D2` | MISSING |  |
| `Currency D2.b` | MISSING |  |
| `Currency D3` | PARTIAL | ordering enforced for marks; foreign positions cannot exist until the currency layer |
| `Currency D4` | MISSING |  |
| `Currency E1` | MISSING |  |
| `Currency E2` | MISSING |  |
| `Currency E3` | MISSING |  |
| `Currency E4` | MISSING |  |

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
| `Bond N11` | PARTIAL | the sovereign answers none; the corporate regime arrives with Corporate Credit |
| `Bond N12` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Bond N13` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Bond N14` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts, packages/engine/src/register/instruments.ts |

## Derivative

| requirement | status | where / why |
|---|---|---|
| `Derivative D1` | MISSING |  |
| `Derivative D1.a` | MISSING |  |
| `Derivative D1.b` | MET | packages/engine/src/audit/families/unbuilt.ts |
| `Derivative D2` | MISSING |  |
| `Derivative D3` | MISSING |  |
| `Derivative D3.a` | MISSING |  |
| `Derivative D4` | MISSING |  |
| `Derivative D5` | MISSING |  |
| `Derivative D6` | MISSING |  |
| `Derivative D7` | MISSING |  |
| `Derivative D8` | MISSING |  |
| `Derivative D9` | MISSING |  |
| `Derivative D10` | MISSING |  |
| `Derivative D11` | MISSING |  |
| `Derivative D12` | MISSING |  |
| `Derivative X1` | MISSING |  |
| `Derivative X2` | MISSING |  |
| `Derivative X3` | MISSING |  |

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
| `Corporate Credit E5` | MISSING |  |
| `Corporate Credit E5.d` | MISSING |  |
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
| `Corporate Credit G1` | MISSING |  |
| `Corporate Credit G2` | MISSING |  |
| `Corporate Credit G3` | MISSING |  |
| `Corporate Credit G4` | MISSING |  |
| `Corporate Credit G5` | MISSING |  |
| `Corporate Credit G6` | MISSING |  |
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
| `Sovereign C2` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/clearing/solver.ts, packages/engine/src/mechanisms/sovereign-auction/index.ts |
| `Sovereign C3` | MET | packages/engine/src/mechanisms/sovereign-auction/index.ts |
| `Sovereign C4` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/mechanisms/sovereign-auction/index.ts |
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
| `Sovereign E5` | MET | packages/engine/src/mechanisms/sovereign-auction/index.ts, packages/engine/src/mechanisms/sovereign-curve/index.ts |
| `Sovereign F1` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign F2` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign F3` | MET | packages/engine/src/mechanisms/sovereign-instruments/index.ts |
| `Sovereign F4` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Sovereign F5` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Sovereign G1` | MISSING |  |
| `Sovereign G2` | MISSING |  |
| `Sovereign G3` | MISSING |  |
| `Sovereign G4` | MISSING |  |
| `Sovereign G5` | MISSING |  |
| `Sovereign H1` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Sovereign H2` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Sovereign H3` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Sovereign H4` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Sovereign H5` | MISSING |  |
| `Sovereign I1` | MISSING |  |
| `Sovereign I2` | MISSING |  |
| `Sovereign I3` | MISSING |  |
| `Sovereign I3.a` | MISSING |  |

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
| `Equity A1` | MISSING |  |
| `Equity A2` | MISSING |  |
| `Equity A3` | MISSING |  |
| `Equity A4` | MISSING |  |
| `Equity A5` | MISSING |  |
| `Equity A5.b` | MISSING |  |
| `Equity A6` | MISSING |  |
| `Equity B1` | MISSING |  |
| `Equity B2` | MISSING |  |
| `Equity B3` | MISSING |  |
| `Equity B4` | MISSING |  |
| `Equity B4.a` | MISSING |  |
| `Equity B5` | MISSING |  |
| `Equity B6` | MISSING |  |
| `Equity C1` | MISSING |  |
| `Equity C1.a` | MET | packages/engine/src/audit/families/ownership.ts |
| `Equity C2` | MISSING |  |
| `Equity C3` | MET | packages/engine/src/prices/value.ts |
| `Equity C4` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/world/revalue.ts |
| `Equity C5` | MISSING |  |
| `Equity C6` | MISSING |  |
| `Equity C7` | MISSING |  |
| `Equity D1` | MISSING |  |
| `Equity D2` | MISSING |  |
| `Equity D3` | MISSING |  |
| `Equity D4` | MISSING |  |
| `Equity E1` | MISSING |  |
| `Equity E2` | MISSING |  |
| `Equity E3` | MISSING |  |
| `Equity E4` | MISSING |  |
| `Equity F1` | MISSING |  |
| `Equity F2` | MISSING |  |
| `Equity F3` | MISSING |  |
| `Equity F4` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/registry/kinds.ts |
| `Equity G1` | MISSING |  |
| `Equity G2` | MISSING |  |
| `Equity G3` | MISSING |  |

## Money Market

| requirement | status | where / why |
|---|---|---|
| `Money Market A1` | MISSING |  |
| `Money Market A2` | MISSING |  |
| `Money Market A2.b` | MISSING |  |
| `Money Market A3` | MISSING |  |
| `Money Market B1` | MISSING |  |
| `Money Market B2` | MISSING |  |
| `Money Market B2.b` | MISSING |  |
| `Money Market B3` | MISSING |  |
| `Money Market B4` | MISSING |  |
| `Money Market B5` | MISSING |  |
| `Money Market B6` | MISSING |  |
| `Money Market B6.a` | MISSING |  |
| `Money Market B7` | MISSING |  |
| `Money Market C1` | MISSING |  |
| `Money Market C2` | MISSING |  |
| `Money Market C3` | MISSING |  |
| `Money Market C4` | MISSING |  |
| `Money Market C5` | MISSING |  |
| `Money Market D1` | MISSING |  |
| `Money Market D2` | MISSING |  |
| `Money Market D3` | MISSING |  |
| `Money Market D4` | MISSING |  |
| `Money Market D5` | MISSING |  |
| `Money Market D5.b` | MISSING |  |
| `Money Market D6` | MISSING |  |
| `Money Market E1` | MISSING |  |
| `Money Market E2` | MISSING |  |
| `Money Market E3` | MISSING |  |

## Spot FX

| requirement | status | where / why |
|---|---|---|
| `Spot FX A1` | MISSING |  |
| `Spot FX A2` | MISSING |  |
| `Spot FX A3` | MISSING |  |
| `Spot FX B1` | MISSING |  |
| `Spot FX B2` | MISSING |  |
| `Spot FX B3` | MISSING |  |
| `Spot FX B4` | MISSING |  |
| `Spot FX B5` | MISSING |  |
| `Spot FX B6` | MISSING |  |
| `Spot FX C1` | MISSING |  |
| `Spot FX C2` | MISSING |  |
| `Spot FX C3` | MISSING |  |
| `Spot FX C4` | MISSING |  |
| `Spot FX C5` | MISSING |  |
| `Spot FX C6` | MISSING |  |
| `Spot FX D1` | MISSING |  |
| `Spot FX D2` | MISSING |  |
| `Spot FX D3` | MISSING |  |
| `Spot FX D4` | MISSING |  |
| `Spot FX D5` | MISSING |  |
| `Spot FX E1` | MISSING |  |
| `Spot FX E2` | MISSING |  |
| `Spot FX E3` | MISSING |  |
| `Spot FX E4` | MISSING |  |
| `Spot FX F1` | MISSING |  |
| `Spot FX F1.a` | MISSING |  |
| `Spot FX F1.b` | MISSING |  |

## Fund Shares

| requirement | status | where / why |
|---|---|---|
| `Fund Shares A1` | MISSING |  |
| `Fund Shares A2` | MISSING |  |
| `Fund Shares A3` | MET | packages/engine/src/audit/families/accounts.ts, packages/engine/src/registry/kinds.ts |
| `Fund Shares A4` | MISSING |  |
| `Fund Shares B1` | MET | packages/engine/src/prices/value.ts |
| `Fund Shares B2` | MISSING |  |
| `Fund Shares B3` | MISSING |  |
| `Fund Shares B4` | MISSING |  |
| `Fund Shares C1` | MISSING |  |
| `Fund Shares C2` | MISSING |  |
| `Fund Shares C3` | MISSING |  |
| `Fund Shares C4` | MISSING |  |
| `Fund Shares C5` | MISSING |  |
| `Fund Shares D1` | MISSING |  |
| `Fund Shares D2` | MISSING |  |
| `Fund Shares D3` | MISSING |  |
| `Fund Shares D4` | MISSING |  |
| `Fund Shares D5` | MISSING |  |
| `Fund Shares E1` | MISSING |  |
| `Fund Shares E2` | MISSING |  |
| `Fund Shares E3` | MISSING |  |
| `Fund Shares E4` | MISSING |  |
| `Fund Shares F1` | MISSING |  |
| `Fund Shares F2` | MISSING |  |
| `Fund Shares F3` | MISSING |  |
| `Fund Shares G1` | MISSING |  |

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
| `Derivative Layer A1` | MISSING |  |
| `Derivative Layer A2` | MISSING |  |
| `Derivative Layer A3` | MISSING |  |
| `Derivative Layer A4` | MET | packages/engine/src/audit/families/unbuilt.ts |
| `Derivative Layer B1` | MISSING |  |
| `Derivative Layer B2` | MISSING |  |
| `Derivative Layer B3` | MISSING |  |
| `Derivative Layer B4` | MISSING |  |
| `Derivative Layer C1` | MISSING |  |
| `Derivative Layer C2` | MISSING |  |
| `Derivative Layer C3` | MISSING |  |
| `Derivative Layer C4` | MISSING |  |
| `Derivative Layer C5` | MISSING |  |
| `Derivative Layer D1` | MISSING |  |
| `Derivative Layer D2` | MISSING |  |
| `Derivative Layer D2.b` | MET | packages/engine/src/audit/families/unbuilt.ts |
| `Derivative Layer D2.c` | MISSING |  |
| `Derivative Layer D3` | MISSING |  |
| `Derivative Layer D4` | MISSING |  |
| `Derivative Layer D5` | MISSING |  |
| `Derivative Layer E1` | MISSING |  |
| `Derivative Layer E2` | MISSING |  |
| `Derivative Layer E3` | MISSING |  |
| `Derivative Layer E4` | MISSING |  |
| `Derivative Layer F1` | MISSING |  |
| `Derivative Layer F2` | MISSING |  |
| `Derivative Layer F3` | MISSING |  |
| `Derivative Layer F4` | MISSING |  |
| `Derivative Layer G1` | MISSING |  |
| `Derivative Layer G2` | MISSING |  |
| `Derivative Layer G3` | MISSING |  |
| `Derivative Layer G4` | MISSING |  |

## CDS

| requirement | status | where / why |
|---|---|---|
| `CDS A1` | MISSING |  |
| `CDS A2` | MISSING |  |
| `CDS A3` | MISSING |  |
| `CDS A4` | MISSING |  |
| `CDS A4.a` | MISSING |  |
| `CDS A5` | MISSING |  |
| `CDS B1` | MISSING |  |
| `CDS B2` | MISSING |  |
| `CDS B3` | MISSING |  |
| `CDS B4` | MISSING |  |
| `CDS B5` | MISSING |  |
| `CDS C1` | MISSING |  |
| `CDS C2` | MISSING |  |
| `CDS C3` | MISSING |  |
| `CDS C4` | MISSING |  |
| `CDS D1` | MISSING |  |
| `CDS D2` | MISSING |  |
| `CDS D2.a` | MISSING |  |
| `CDS D3` | MISSING |  |
| `CDS D4` | MISSING |  |
| `CDS D5` | MISSING |  |
| `CDS E1` | MISSING |  |
| `CDS E2` | MISSING |  |
| `CDS E3` | MISSING |  |
| `CDS E4` | MISSING |  |

## IRS

| requirement | status | where / why |
|---|---|---|
| `IRS A1` | MISSING |  |
| `IRS A2` | MISSING |  |
| `IRS A3` | MISSING |  |
| `IRS A4` | MISSING |  |
| `IRS B1` | MISSING |  |
| `IRS B2` | MISSING |  |
| `IRS B3` | MISSING |  |
| `IRS B4` | MISSING |  |
| `IRS B5` | MISSING |  |
| `IRS C1` | MISSING |  |
| `IRS C2` | MISSING |  |
| `IRS C3` | MISSING |  |
| `IRS C4` | MISSING |  |
| `IRS D1` | MISSING |  |
| `IRS D2` | MISSING |  |
| `IRS D3` | MISSING |  |
| `IRS D4` | MISSING |  |
| `IRS E1` | MISSING |  |
| `IRS E2` | MISSING |  |
| `IRS E3` | MISSING |  |

## FX Forwards

| requirement | status | where / why |
|---|---|---|
| `FX Forwards A1` | MISSING |  |
| `FX Forwards A2` | MISSING |  |
| `FX Forwards A3` | MISSING |  |
| `FX Forwards A4` | MISSING |  |
| `FX Forwards B1` | MISSING |  |
| `FX Forwards B2` | MISSING |  |
| `FX Forwards B3` | MISSING |  |
| `FX Forwards B3.b` | MISSING |  |
| `FX Forwards B4` | MISSING |  |
| `FX Forwards C1` | MISSING |  |
| `FX Forwards C2` | MISSING |  |
| `FX Forwards C3` | MISSING |  |
| `FX Forwards C4` | MISSING |  |
| `FX Forwards D1` | MISSING |  |
| `FX Forwards D2` | MISSING |  |
| `FX Forwards D3` | MISSING |  |
| `FX Forwards D4` | MISSING |  |
| `FX Forwards E1` | MISSING |  |
| `FX Forwards E2` | MISSING |  |
| `FX Forwards E3` | MISSING |  |
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
| `Commodities Spot D1` | MISSING |  |
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
| `Indices A1` | MISSING |  |
| `Indices A2` | MISSING |  |
| `Indices A3` | MISSING |  |
| `Indices A4` | MISSING |  |
| `Indices B1` | MISSING |  |
| `Indices B2` | MISSING |  |
| `Indices B3` | MISSING |  |
| `Indices B4` | MISSING |  |
| `Indices C1` | MISSING |  |
| `Indices C2` | MISSING |  |
| `Indices C2.a` | MISSING |  |
| `Indices C3` | MISSING |  |
| `Indices C4` | MISSING |  |
| `Indices D1` | MISSING |  |
| `Indices D2` | MISSING |  |
| `Indices D3` | MISSING |  |
| `Indices D3.b` | MISSING |  |
| `Indices D4` | MISSING |  |
| `Indices D5` | MISSING |  |
| `Indices E1` | MISSING |  |
| `Indices E2` | MISSING |  |
| `Indices E3` | MISSING |  |

## Banks Lending

| requirement | status | where / why |
|---|---|---|
| `Banks Lending A1` | MISSING |  |
| `Banks Lending A2` | MISSING |  |
| `Banks Lending A3` | MISSING |  |
| `Banks Lending A3.b` | MISSING |  |
| `Banks Lending A4` | MISSING |  |
| `Banks Lending A5` | MISSING |  |
| `Banks Lending B1` | MISSING |  |
| `Banks Lending B1.c` | MISSING |  |
| `Banks Lending B2` | MISSING |  |
| `Banks Lending B2.d` | MISSING |  |
| `Banks Lending C1` | MISSING |  |
| `Banks Lending C2` | MISSING |  |
| `Banks Lending C3` | MISSING |  |
| `Banks Lending C3.a` | MISSING |  |
| `Banks Lending C4` | MISSING |  |
| `Banks Lending D1` | MISSING |  |
| `Banks Lending D2` | MISSING |  |
| `Banks Lending D2.b` | MISSING |  |
| `Banks Lending D3` | MISSING |  |
| `Banks Lending D4` | MISSING |  |
| `Banks Lending D5` | MISSING |  |
| `Banks Lending E1` | MISSING |  |
| `Banks Lending E2` | MISSING |  |
| `Banks Lending E3` | MISSING |  |
| `Banks Lending E4` | MISSING |  |
| `Banks Lending E5` | MISSING |  |
| `Banks Lending E5.a` | MISSING |  |
| `Banks Lending E6` | MISSING |  |
| `Banks Lending F1` | MISSING |  |
| `Banks Lending F1.a` | MISSING |  |
| `Banks Lending F2` | MISSING |  |
| `Banks Lending F3` | MISSING |  |

## Banks Funding

| requirement | status | where / why |
|---|---|---|
| `Banks Funding A1` | MISSING |  |
| `Banks Funding A1.d` | MISSING |  |
| `Banks Funding A2` | MISSING |  |
| `Banks Funding A3` | MISSING |  |
| `Banks Funding A4` | MISSING |  |
| `Banks Funding A5` | MISSING |  |
| `Banks Funding B1` | MISSING |  |
| `Banks Funding B2` | MISSING |  |
| `Banks Funding B2.b` | MISSING |  |
| `Banks Funding B3` | MISSING |  |
| `Banks Funding C1` | MISSING |  |
| `Banks Funding C2` | MISSING |  |
| `Banks Funding C3` | MISSING |  |
| `Banks Funding C3.a` | MISSING |  |
| `Banks Funding C4` | MISSING |  |
| `Banks Funding D1` | MISSING |  |
| `Banks Funding D2` | MISSING |  |
| `Banks Funding D3` | MISSING |  |
| `Banks Funding D4` | MISSING |  |
| `Banks Funding D5` | MISSING |  |
| `Banks Funding D6` | MISSING |  |
| `Banks Funding D6.a` | MISSING |  |
| `Banks Funding E1` | MISSING |  |
| `Banks Funding E2` | MISSING |  |
| `Banks Funding E3` | MISSING |  |
| `Banks Funding E3.a` | MISSING |  |
| `Banks Funding E4` | MISSING |  |
| `Banks Funding E5` | MISSING |  |
| `Banks Funding F1` | MISSING |  |
| `Banks Funding F2` | MISSING |  |
| `Banks Funding F3` | MET | packages/engine/src/audit/families/accounts.ts |
| `Banks Funding F4` | MISSING |  |

## Banks Capital

| requirement | status | where / why |
|---|---|---|
| `Banks Capital A1` | MISSING |  |
| `Banks Capital A1.a` | MISSING |  |
| `Banks Capital A2` | MISSING |  |
| `Banks Capital A3` | MISSING |  |
| `Banks Capital A4` | MISSING |  |
| `Banks Capital B1` | MISSING |  |
| `Banks Capital B1.c` | MISSING |  |
| `Banks Capital B2` | MISSING |  |
| `Banks Capital B3` | MISSING |  |
| `Banks Capital B3.a` | MISSING |  |
| `Banks Capital C1` | MISSING |  |
| `Banks Capital C2` | MISSING |  |
| `Banks Capital C3` | MISSING |  |
| `Banks Capital D1` | MISSING |  |
| `Banks Capital D2` | MISSING |  |
| `Banks Capital D2.a` | MISSING |  |
| `Banks Capital D3` | MISSING |  |
| `Banks Capital D4` | MISSING |  |
| `Banks Capital D5` | MISSING |  |
| `Banks Capital D6` | MISSING |  |
| `Banks Capital E1` | MISSING |  |
| `Banks Capital E2` | MISSING |  |
| `Banks Capital E3` | MISSING |  |

## Dealer Desks

| requirement | status | where / why |
|---|---|---|
| `Dealer Desks A1` | MISSING |  |
| `Dealer Desks A2` | MISSING |  |
| `Dealer Desks A3` | MISSING |  |
| `Dealer Desks A4` | MISSING |  |
| `Dealer Desks B1` | MISSING |  |
| `Dealer Desks B2` | MISSING |  |
| `Dealer Desks B3` | MISSING |  |
| `Dealer Desks B4` | MISSING |  |
| `Dealer Desks C1` | MISSING |  |
| `Dealer Desks C2` | MISSING |  |
| `Dealer Desks C3` | MISSING |  |
| `Dealer Desks C4` | MISSING |  |
| `Dealer Desks C5` | MISSING |  |
| `Dealer Desks C5.a` | MISSING |  |
| `Dealer Desks C5.b` | MISSING |  |
| `Dealer Desks D1` | MISSING |  |
| `Dealer Desks D2` | MISSING |  |
| `Dealer Desks D3` | MISSING |  |
| `Dealer Desks D4` | MET | packages/engine/src/mechanisms/sovereign-auction/index.ts |
| `Dealer Desks D5` | MISSING |  |
| `Dealer Desks E1` | MISSING |  |
| `Dealer Desks E2` | MISSING |  |
| `Dealer Desks E3` | MISSING |  |
| `Dealer Desks E4` | MISSING |  |
| `Dealer Desks F1` | MISSING |  |
| `Dealer Desks F2` | MISSING |  |
| `Dealer Desks F3` | MISSING |  |

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
| `Treasury B3` | PARTIAL | outlays vary with the standing mandate; the cycle and unemployment arrive with the real economy in full (worklist 4.7) and policy with the polity (worklist 14) |
| `Treasury B4` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury C1` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury C2` | MET | packages/engine/src/mechanisms/treasury/index.ts (three bases read off what payers actually did: interest received, what households were paid, what they paid for real things — so receipts fall when income and spending fall) |
| `Treasury C3` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury D1` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury D2` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury D3` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/mechanisms/treasury/index.ts, packages/engine/src/registry/profiles.ts |
| `Treasury D4` | MET | packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury D5` | MET | packages/engine/src/mechanisms/sovereign-auction/index.ts, packages/engine/src/mechanisms/treasury/index.ts |
| `Treasury D5.a` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/mechanisms/sovereign-auction/index.ts, packages/engine/src/mechanisms/treasury/index.ts |
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
| `Central Bank B1` | MISSING |  |
| `Central Bank B2` | MISSING |  |
| `Central Bank B3` | MISSING |  |
| `Central Bank B3.a` | MISSING |  |
| `Central Bank B4` | MISSING |  |
| `Central Bank C1` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts (sovereign paper only: it stands in no other market) |
| `Central Bank C1.b` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Central Bank C2` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Central Bank C3` | MET | packages/engine/src/clearing/solver.ts, packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Central Bank C4` | MET | packages/engine/src/mechanisms/central-bank-omo/index.ts |
| `Central Bank D1` | MISSING |  |
| `Central Bank D2` | MISSING |  |
| `Central Bank D3` | MISSING |  |
| `Central Bank D3.a` | MISSING |  |
| `Central Bank D4` | MISSING |  |
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
| `Firm A3` | PARTIAL | firms differ in line, stock and cost (packages/engine/src/mechanisms/firms/data.ts); leverage arrives with loans (worklist 6) and the dispersion of size with the seed (worklist 4.7) |
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
| `Firm D4` | MISSING |  |
| `Firm D5` | MISSING |  |
| `Firm E1` | PARTIAL | packages/engine/src/mechanisms/firms/decide.ts prices and sizes what it offers from its own outlook and what holding is worth; entering and leaving a line is firm birth (worklist 13g) |
| `Firm E2` | MET | packages/engine/src/mechanisms/firms/decide.ts (the employment it wants is the labour that makes what it expects to sell, at what an hour is worth to it) |
| `Firm E3` | MISSING |  |
| `Firm E4` | MISSING |  |
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
| `Capital Programme A1` | MISSING |  |
| `Capital Programme A2` | MISSING |  |
| `Capital Programme A3` | MISSING |  |
| `Capital Programme A4` | MISSING |  |
| `Capital Programme A5` | MISSING |  |
| `Capital Programme A6` | MISSING |  |
| `Capital Programme A6.b` | MISSING |  |
| `Capital Programme B1` | MISSING |  |
| `Capital Programme B2` | MISSING |  |
| `Capital Programme B3` | MISSING |  |
| `Capital Programme B4` | MISSING |  |
| `Capital Programme B5` | MISSING |  |
| `Capital Programme C1` | MISSING |  |
| `Capital Programme C2` | MISSING |  |
| `Capital Programme C3` | MISSING |  |
| `Capital Programme C4` | MISSING |  |
| `Capital Programme D1` | MISSING |  |
| `Capital Programme D2` | MISSING |  |
| `Capital Programme D3` | MISSING |  |
| `Capital Programme D4` | MISSING |  |
| `Capital Programme E1` | MISSING |  |
| `Capital Programme E2` | MISSING |  |
| `Capital Programme E3` | MISSING |  |
| `Capital Programme E4` | MISSING |  |
| `Capital Programme F1` | MISSING |  |

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
| `Firm Birth C1` | MISSING |  |
| `Firm Birth C2` | MISSING |  |
| `Firm Birth C2.a` | MISSING |  |
| `Firm Birth C3` | MISSING |  |
| `Firm Birth C4` | MISSING |  |
| `Firm Birth D1` | MISSING |  |
| `Firm Birth D2` | MISSING |  |
| `Firm Birth D3` | MISSING |  |
| `Firm Birth D4` | MISSING |  |
| `Firm Birth D5` | MISSING |  |
| `Firm Birth D6` | MISSING |  |
| `Firm Birth E1` | MISSING |  |
| `Firm Birth E2` | MISSING |  |
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
| `Goods B1.d` | MISSING | utilisation is a read against capacity, and capacity is plant (worklist 10) |
| `Goods B2` | MET | packages/engine/src/mechanisms/firms/produce.ts, packages/engine/src/mechanisms/goods/index.ts (the audit contribution: what a batch consumed IS its recipe) |
| `Goods B3` | MET | packages/engine/src/mechanisms/goods/inventory.ts (work in progress is a kind of its own, carried at what it has cost), packages/engine/src/mechanisms/firms/produce.ts |
| `Goods B4` | MET | packages/engine/src/mechanisms/firms/produce.ts (what is started is not what is finished; the scrap is units and the whole batch cost lands on the survivors) |
| `Goods B5` | MET | packages/engine/src/mechanisms/firms/produce.ts (inputs consumed plus the period wage bill; the capital charge arrives with the capital programme, worklist 10) |
| `Goods C1` | MET | packages/engine/src/mechanisms/firms/decide.ts (a seller offers what it holds in steps with a reason behind each; a buyer posts what the thing is worth to it) |
| `Goods C2` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/prices/price-store.ts, packages/engine/src/mechanisms/goods/index.ts |
| `Goods C3` | PARTIAL | firms buying inputs bid what the input is worth to them (packages/engine/src/mechanisms/firms/decide.ts); households arrive at worklist 4.6 and estates at worklist 7 |
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
| `Goods G4` | MISSING |  |

## Freight

| requirement | status | where / why |
|---|---|---|
| `Freight A1` | MISSING |  |
| `Freight A2` | MISSING |  |
| `Freight A3` | MISSING |  |
| `Freight A4` | MISSING |  |
| `Freight B1` | MISSING |  |
| `Freight B2` | MISSING |  |
| `Freight B3` | MISSING |  |
| `Freight B4` | MISSING |  |
| `Freight C1` | MISSING |  |
| `Freight C2` | MISSING |  |
| `Freight C3` | MISSING |  |
| `Freight D1` | MISSING |  |
| `Freight D2` | MISSING |  |
| `Freight D3` | MISSING |  |
| `Freight D4` | MISSING |  |
| `Freight D5` | MISSING |  |
| `Freight D6` | MISSING |  |
| `Freight E1` | MISSING |  |
| `Freight E2` | MISSING |  |
| `Freight E3` | MISSING |  |

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
| `Labour C4` | MISSING | a firm that fails releases its workers; firm death is worklist 7 |
| `Labour C5` | MET | packages/engine/src/mechanisms/labour/matching.ts (a vacancy is a posting the employer owns, for the period it posts it) |
| `Labour D1` | MET | packages/engine/src/mechanisms/labour/matching.ts (every posting is a bid, the highest fill first, and the level is where the book cleared: in a slack market it falls to the seekers own option and no further, in a tight one the bids set it) |
| `Labour D2` | MET | packages/engine/src/mechanisms/labour/register.ts (the wage is the contract and does not move with the print), packages/engine/src/mechanisms/firms/decide.ts (a firm that wants fewer hours than it has sheds them and pays severance) |
| `Labour D2.b` | MET | packages/engine/src/mechanisms/labour/matching.ts, packages/engine/src/mechanisms/firms/decide.ts (stickiness is the contract and the severance a change costs; nothing damps a series) |
| `Labour D3` | MET | packages/engine/src/mechanisms/labour/matching.ts (whole people are matched from a queue of seekers; hours that do not make a person are no hire) |
| `Labour D4` | MISSING |  |
| `Labour D5` | MISSING |  |
| `Labour E1` | MET | packages/engine/src/mechanisms/labour/matching.ts (the wage reaches the household cell every period) |
| `Labour E2` | MET | packages/engine/src/mechanisms/firms/decide.ts (the wage is in what a unit costs and therefore in what the firm will make and offer) |
| `Labour E3` | MISSING |  |
| `Labour E4` | MISSING |  |
| `Labour F1` | MET | packages/engine/src/mechanisms/labour/index.ts (the audit contribution: every row is a job at a named firm that exists) |
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
| `Households A2.g` | MET | packages/engine/test/households.test.ts (a mean-preserving spread over cells moves more of them below the cushion they want while what they were paid in total does not move) |
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
| `Cross-Border A1` | MISSING |  |
| `Cross-Border A2` | MISSING |  |
| `Cross-Border A3` | MISSING |  |
| `Cross-Border A4` | MISSING |  |
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
| `Ratings A1` | MISSING |  |
| `Ratings A2` | MISSING |  |
| `Ratings A2.a` | MISSING |  |
| `Ratings A3` | MISSING |  |
| `Ratings A4` | MISSING |  |
| `Ratings A5` | MISSING |  |
| `Ratings B1` | MISSING |  |
| `Ratings B2` | MISSING |  |
| `Ratings B3` | MISSING |  |
| `Ratings C1` | MISSING |  |
| `Ratings C2` | MISSING |  |
| `Ratings C3` | MISSING |  |
| `Ratings C4` | MISSING |  |
| `Ratings C5` | MISSING |  |
| `Ratings D1` | MISSING |  |
| `Ratings D2` | MISSING |  |
| `Ratings D3` | MISSING |  |
| `Ratings D4` | MISSING |  |
| `Ratings D5` | MISSING |  |
| `Ratings E1` | MISSING |  |
| `Ratings E2` | MISSING |  |
| `Ratings E3` | MISSING |  |
| `Ratings E4` | MISSING |  |

## Observer

| requirement | status | where / why |
|---|---|---|
| `Observer A1` | MET | packages/engine/src/observer/observer.ts, packages/engine/src/prices/price-store.ts, packages/engine/src/world/context.ts |
| `Observer A2` | MET | packages/engine/src/observer/observer.ts, packages/engine/src/world/context.ts |
| `Observer A3` | PARTIAL | public state is instrument terms and prints; issuer publications arrive with firms |
| `Observer A4` | MET | packages/engine/src/journal/journal.ts, packages/engine/src/observer/observer.ts, packages/engine/src/world/context.ts, packages/engine/src/world/world.ts |
| `Observer A5` | PARTIAL | no published aggregates with a lag yet |
| `Observer B1` | MET | packages/engine/src/journal/journal.ts |
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
| `Observer E3` | MET | packages/engine/src/observer/observer.ts, packages/engine/src/world/world.ts |
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
