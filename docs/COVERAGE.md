# Requirement coverage

One row per REASON, VERIFY and FORBID in the specification. `MET at <path>` means the cited module
implements the clause; `PARTIAL` says what is still missing; `MISSING` is work; `OUT OF SCOPE` states a
reason and keeps the clause (Appendix C). Re-mark in the same change that meets a requirement, and
recount with `npm run coverage:spec` rather than adjusting a tally.


## Money

| requirement | status | where / why |
|---|---|---|
| `Money A1` | MET | packages/engine/src/world/seed.ts |
| `Money A1.d` | MET | packages/engine/src/audit/families/money.ts |
| `Money A2` | MET | packages/engine/src/core/money.ts |
| `Money A2.b` | MET | packages/engine/src/core/money.ts |
| `Money A3` | MISSING |  |
| `Money A4` | MISSING |  |
| `Money B1` | MISSING |  |
| `Money B1.b` | MISSING |  |
| `Money B2` | MISSING |  |
| `Money B3` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/registry/profiles.ts |
| `Money B3.c` | PARTIAL | refusal is recorded; a lender row for an allowed overdraft arrives with the corridor (worklist 11) |
| `Money C1` | MET | packages/engine/src/ledger/instruction.ts |
| `Money C2` | MET | packages/engine/src/ledger/settlement.ts |
| `Money C2.c` | MET | packages/engine/src/audit/families/money.ts, packages/engine/src/ledger/settlement.ts |
| `Money C3` | MISSING |  |
| `Money C4` | MET | packages/engine/src/audit/families/money.ts, packages/engine/src/ledger/settlement.ts |
| `Money C4.c` | MET | packages/engine/src/audit/families/money.ts, packages/engine/src/audit/memory.ts |
| `Money D1` | MET | packages/engine/src/ledger/instruction.ts, packages/engine/src/ledger/ledger.ts, packages/engine/src/ledger/settlement.ts |
| `Money D2` | MET | packages/engine/src/ledger/instruction.ts, packages/engine/src/prices/value.ts, packages/engine/src/register/instruments.ts, packages/engine/src/world/seed.ts |
| `Money D3` | MET | packages/engine/src/audit/families/flows.ts, packages/engine/src/audit/memory.ts, packages/engine/src/ledger/settlement.ts |
| `Money D4` | MET | packages/engine/src/audit/families/flows.ts, packages/engine/src/ledger/settlement.ts |
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
| `Register E1` | MET | packages/engine/src/world/actions.ts |
| `Register E2` | MET | packages/engine/src/world/actions.ts |
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
| `Clearing A3` | MISSING |  |
| `Clearing A4` | MET | packages/engine/src/clearing/solver.ts |
| `Clearing B1` | MET | packages/engine/src/clearing/market.ts |
| `Clearing B2` | MET | packages/engine/src/clearing/market.ts |
| `Clearing B3` | PARTIAL | no dealer exists yet (worklist 9) |
| `Clearing B4` | MISSING |  |
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
| `Seed A1` | MET | packages/engine/src/world/seed.ts |
| `Seed A2` | MET | packages/engine/src/world/seed.ts |
| `Seed A3` | MET | packages/engine/src/world/seed.ts |
| `Seed A4` | MET | packages/engine/src/world/seed.ts |
| `Seed A5` | MET | packages/engine/src/rng/prng.ts, packages/engine/src/world/seed.ts, packages/engine/src/world/world.ts |
| `Seed B1` | PARTIAL | the foundation seed has one instance of several kinds; populations are cells with weights |
| `Seed B2` | MET | packages/engine/src/parties/party.ts, packages/engine/src/world/seed.ts |
| `Seed B3` | MET | packages/engine/src/parties/party.ts, packages/engine/src/registry/registry.ts, packages/engine/src/world/seed.ts |
| `Seed B4` | PARTIAL | sizes are dispersed by hand in the foundation seed; nothing draws them |
| `Seed B5` | MISSING |  |
| `Seed C1` | MET | packages/engine/src/world/seed.ts |
| `Seed C2` | MET | packages/engine/src/world/seed.ts |
| `Seed C3` | MET | packages/engine/src/world/seed.ts |
| `Seed C4` | MET | packages/engine/src/world/seed.ts |
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
| `Bond N4` | MET | packages/engine/src/register/instruments.ts, packages/engine/src/world/actions.ts |
| `Bond N5` | MET | packages/engine/src/register/instruments.ts |
| `Bond N6` | PARTIAL | periodicity and day count are on the instrument; accrual between payments is not yet read |
| `Bond N7` | MISSING |  |
| `Bond N7.b` | MISSING |  |
| `Bond N8` | MISSING |  |
| `Bond N8.a` | MET | packages/engine/src/audit/families/ownership.ts |
| `Bond N9` | MISSING |  |
| `Bond N10` | MET | packages/engine/src/register/instruments.ts, packages/engine/src/world/actions.ts |
| `Bond N11` | PARTIAL | the sovereign answers none; the corporate regime arrives with Corporate Credit |
| `Bond N12` | MISSING |  |
| `Bond N13` | MISSING |  |
| `Bond N14` | MET | packages/engine/src/register/instruments.ts |

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
| `Sovereign A1` | MISSING |  |
| `Sovereign A1.c` | MISSING |  |
| `Sovereign A2` | MISSING |  |
| `Sovereign A3` | MISSING |  |
| `Sovereign A3.b` | MISSING |  |
| `Sovereign A4` | MISSING |  |
| `Sovereign B1` | MET | packages/engine/src/register/instruments.ts, packages/engine/src/world/actions.ts |
| `Sovereign B2` | MISSING |  |
| `Sovereign B3` | MISSING |  |
| `Sovereign B4` | MISSING |  |
| `Sovereign B5` | MISSING |  |
| `Sovereign B6` | MISSING |  |
| `Sovereign B7` | MISSING |  |
| `Sovereign C1` | MISSING |  |
| `Sovereign C2` | MET | packages/engine/src/clearing/solver.ts |
| `Sovereign C3` | MISSING |  |
| `Sovereign C4` | MISSING |  |
| `Sovereign C5` | MISSING |  |
| `Sovereign C6` | MISSING |  |
| `Sovereign C7` | MISSING |  |
| `Sovereign D1` | MISSING |  |
| `Sovereign D2` | MISSING |  |
| `Sovereign D3` | MISSING |  |
| `Sovereign D4` | MISSING |  |
| `Sovereign D5` | MISSING |  |
| `Sovereign D6` | MISSING |  |
| `Sovereign E1` | MISSING |  |
| `Sovereign E1.a` | MISSING |  |
| `Sovereign E2` | MISSING |  |
| `Sovereign E3` | MISSING |  |
| `Sovereign E4` | MISSING |  |
| `Sovereign E5` | MISSING |  |
| `Sovereign F1` | PARTIAL | coupon paid to the holder on the date; accrual to the holder of record between dates is not read |
| `Sovereign F2` | MISSING |  |
| `Sovereign F3` | MET | packages/engine/src/world/actions.ts |
| `Sovereign F4` | MISSING |  |
| `Sovereign F5` | MISSING |  |
| `Sovereign G1` | MISSING |  |
| `Sovereign G2` | MISSING |  |
| `Sovereign G3` | MISSING |  |
| `Sovereign G4` | MISSING |  |
| `Sovereign G5` | MISSING |  |
| `Sovereign H1` | MISSING |  |
| `Sovereign H2` | MISSING |  |
| `Sovereign H3` | MISSING |  |
| `Sovereign H4` | MISSING |  |
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
| `Equity F4` | MET | packages/engine/src/ledger/settlement.ts, packages/engine/src/registry/profiles.ts |
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
| `Fund Shares A3` | MET | packages/engine/src/audit/families/accounts.ts, packages/engine/src/registry/profiles.ts |
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
| `Commodities Spot D5` | MISSING |  |
| `Commodities Spot E1` | MISSING |  |
| `Commodities Spot E2` | MISSING |  |
| `Commodities Spot E3` | MISSING |  |
| `Commodities Spot E4` | MISSING |  |
| `Commodities Spot F1` | MISSING |  |
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
| `Dealer Desks D4` | MISSING |  |
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
| `Treasury A1` | MISSING |  |
| `Treasury A2` | MISSING |  |
| `Treasury A3` | MISSING |  |
| `Treasury B1` | MISSING |  |
| `Treasury B2` | MISSING |  |
| `Treasury B3` | MISSING |  |
| `Treasury B4` | MISSING |  |
| `Treasury C1` | MISSING |  |
| `Treasury C2` | MISSING |  |
| `Treasury C3` | MISSING |  |
| `Treasury D1` | MISSING |  |
| `Treasury D2` | MISSING |  |
| `Treasury D3` | MISSING |  |
| `Treasury D4` | MISSING |  |
| `Treasury D5` | MISSING |  |
| `Treasury D5.a` | MISSING |  |
| `Treasury D6` | MISSING |  |
| `Treasury E1` | MISSING |  |
| `Treasury E2` | MISSING |  |
| `Treasury E3` | MISSING |  |
| `Treasury E4` | MISSING |  |
| `Treasury F1` | MISSING |  |
| `Treasury F2` | MISSING |  |
| `Treasury F3` | MISSING |  |
| `Treasury F4` | MISSING |  |

## Central Bank

| requirement | status | where / why |
|---|---|---|
| `Central Bank A1` | MISSING |  |
| `Central Bank A2` | MISSING |  |
| `Central Bank A2.c` | MET | packages/engine/src/audit/families/accounts.ts |
| `Central Bank A3` | MISSING |  |
| `Central Bank A4` | MISSING |  |
| `Central Bank B1` | MISSING |  |
| `Central Bank B2` | MISSING |  |
| `Central Bank B3` | MISSING |  |
| `Central Bank B3.a` | MISSING |  |
| `Central Bank B4` | MISSING |  |
| `Central Bank C1` | MISSING |  |
| `Central Bank C1.b` | MISSING |  |
| `Central Bank C2` | MISSING |  |
| `Central Bank C3` | MISSING |  |
| `Central Bank C4` | MISSING |  |
| `Central Bank D1` | MISSING |  |
| `Central Bank D2` | MISSING |  |
| `Central Bank D3` | MISSING |  |
| `Central Bank D3.a` | MISSING |  |
| `Central Bank D4` | MISSING |  |
| `Central Bank E1` | MISSING |  |
| `Central Bank E2` | MISSING |  |
| `Central Bank E3` | MISSING |  |
| `Central Bank E4` | MISSING |  |
| `Central Bank E5` | MISSING |  |
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
| `Firm A1` | MISSING |  |
| `Firm A2` | MISSING |  |
| `Firm A3` | MISSING |  |
| `Firm A4` | MISSING |  |
| `Firm B1` | MISSING |  |
| `Firm B2` | MISSING |  |
| `Firm B3` | MISSING |  |
| `Firm B4` | MISSING |  |
| `Firm B5` | MISSING |  |
| `Firm B6` | MISSING |  |
| `Firm C1` | MISSING |  |
| `Firm C2` | MISSING |  |
| `Firm C3` | MET | packages/engine/src/audit/families/accounts.ts |
| `Firm C4` | MISSING |  |
| `Firm D1` | MISSING |  |
| `Firm D2` | MISSING |  |
| `Firm D3` | MISSING |  |
| `Firm D4` | MISSING |  |
| `Firm D5` | MISSING |  |
| `Firm E1` | MISSING |  |
| `Firm E2` | MISSING |  |
| `Firm E3` | MISSING |  |
| `Firm E4` | MISSING |  |
| `Firm E5` | MISSING |  |
| `Firm E6` | MISSING |  |
| `Firm E7` | MISSING |  |
| `Firm F1` | MISSING |  |
| `Firm F2` | MISSING |  |
| `Firm F3` | MISSING |  |
| `Firm F4` | MISSING |  |

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
| `Goods A1` | MISSING |  |
| `Goods A2` | MISSING |  |
| `Goods A2.b` | MISSING |  |
| `Goods A3` | MISSING |  |
| `Goods A4` | MISSING |  |
| `Goods B1` | MISSING |  |
| `Goods B1.d` | MISSING |  |
| `Goods B2` | MISSING |  |
| `Goods B3` | MISSING |  |
| `Goods B4` | MISSING |  |
| `Goods B5` | MISSING |  |
| `Goods C1` | MISSING |  |
| `Goods C2` | MET | packages/engine/src/clearing/market.ts, packages/engine/src/prices/price-store.ts |
| `Goods C3` | MISSING |  |
| `Goods C4` | MISSING |  |
| `Goods C5` | MISSING |  |
| `Goods C6` | MISSING |  |
| `Goods D1` | MISSING |  |
| `Goods D2` | MISSING |  |
| `Goods D3` | MISSING |  |
| `Goods D4` | MISSING |  |
| `Goods D5` | MISSING |  |
| `Goods E1` | MISSING |  |
| `Goods E2` | MISSING |  |
| `Goods E2.c` | MISSING |  |
| `Goods E3` | MISSING |  |
| `Goods E4` | MISSING |  |
| `Goods E5` | MISSING |  |
| `Goods F1` | MISSING |  |
| `Goods F2` | MISSING |  |
| `Goods F3` | MISSING |  |
| `Goods F4` | MISSING |  |
| `Goods F5` | MISSING |  |
| `Goods F5.b` | MISSING |  |
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
| `Labour A1` | MISSING |  |
| `Labour A2` | MISSING |  |
| `Labour A3` | MISSING |  |
| `Labour A4` | MISSING |  |
| `Labour B1` | MISSING |  |
| `Labour B2` | MISSING |  |
| `Labour B3` | MISSING |  |
| `Labour B4` | MISSING |  |
| `Labour B5` | MISSING |  |
| `Labour C1` | MISSING |  |
| `Labour C2` | MISSING |  |
| `Labour C3` | MISSING |  |
| `Labour C4` | MISSING |  |
| `Labour C5` | MISSING |  |
| `Labour D1` | MISSING |  |
| `Labour D2` | MISSING |  |
| `Labour D2.b` | MISSING |  |
| `Labour D3` | MISSING |  |
| `Labour D4` | MISSING |  |
| `Labour D5` | MISSING |  |
| `Labour E1` | MISSING |  |
| `Labour E2` | MISSING |  |
| `Labour E3` | MISSING |  |
| `Labour E4` | MISSING |  |
| `Labour F1` | MISSING |  |
| `Labour F2` | MISSING |  |
| `Labour F3` | MISSING |  |

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
| `Households A1` | MISSING |  |
| `Households A2` | MISSING |  |
| `Households A2.d` | MISSING |  |
| `Households A2.e` | MET | packages/engine/src/parties/party.ts |
| `Households A2.f` | MET | packages/engine/src/parties/party.ts |
| `Households A2.g` | MISSING |  |
| `Households A3` | MISSING |  |
| `Households B1` | MISSING |  |
| `Households B2` | MISSING |  |
| `Households B3` | MISSING |  |
| `Households B3.a` | MISSING |  |
| `Households B4` | MISSING |  |
| `Households B5` | MISSING |  |
| `Households C1` | MISSING |  |
| `Households C2` | MISSING |  |
| `Households C3` | MISSING |  |
| `Households C4` | MISSING |  |
| `Households C5` | MISSING |  |
| `Households D1` | MISSING |  |
| `Households D2` | MISSING |  |
| `Households D3` | MET | packages/engine/src/audit/families/accounts.ts |
| `Households D4` | MISSING |  |
| `Households D5` | MISSING |  |
| `Households D6` | MISSING |  |
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
| `Small-Business Pools A6` | MET | packages/engine/src/parties/party.ts |
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
| `Observer A1` | MET | packages/engine/src/observer/observer.ts, packages/engine/src/prices/price-store.ts |
| `Observer A2` | MET | packages/engine/src/observer/observer.ts |
| `Observer A3` | PARTIAL | public state is instrument terms and prints; issuer publications arrive with firms |
| `Observer A4` | MET | packages/engine/src/observer/observer.ts |
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
| `Observer F2` | PARTIAL | prices shown; derived spread and yield arrive with the curve (worklist 3) |
| `Observer F3` | MISSING |  |
| `Observer F4` | MET | packages/engine/src/observer/observer.ts |

## Expectations

| requirement | status | where / why |
|---|---|---|
| `Expectations A1` | MISSING |  |
| `Expectations A2` | MISSING |  |
| `Expectations A2.b` | MISSING |  |
| `Expectations A3` | MISSING |  |
| `Expectations A4` | MISSING |  |
| `Expectations A5` | MISSING |  |
| `Expectations B1` | MISSING |  |
| `Expectations B1.b` | MISSING |  |
| `Expectations B2` | MISSING |  |
| `Expectations B2.a` | MISSING |  |
| `Expectations B3` | MISSING |  |
| `Expectations B4` | MISSING |  |
| `Expectations B5` | MISSING |  |
| `Expectations C1` | MISSING |  |
| `Expectations C2` | MISSING |  |
| `Expectations C3` | MISSING |  |
| `Expectations C4` | MISSING |  |
| `Expectations C5` | MISSING |  |
| `Expectations C6` | MISSING |  |
| `Expectations D1` | MISSING |  |
| `Expectations D2` | MISSING |  |
| `Expectations D3` | MISSING |  |
| `Expectations D4` | MISSING |  |
| `Expectations E1` | MISSING |  |
| `Expectations E2` | MISSING |  |
| `Expectations E3` | MISSING |  |
| `Expectations E4` | MISSING |  |
