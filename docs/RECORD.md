# The record

A ledger of outcomes, not a diary (Law 16). One entry per worklist item, written when the item
closes: what changed, why, what was found, what it deleted. A forecast appears only next to the
measurement that would kill it (Law 17).

---

## 0 — Foundation

**What.** Repository scaffold: TypeScript strict, npm workspaces (`packages/engine`, `packages/app`,
`tools`), ESLint with five project rules (no bounds, no numeric defaults, no magic numbers, no kind
branches, no clock or randomness), Vitest with fast-check, Playwright smoke test, GitHub Actions for
CI, Pages and a Capacitor Android APK. `docs/ARCHITECTURE.md` records every implementation decision
with the clause it derives from. `CLAUDE.md` carries the rules digest into every session. The spec
moved to `docs/spec/PROJECT_PHOENIX.md`; `tools/spec-index.ts` parses it (1,739 nodes, 1,317 of them
REASON/VERIFY/FORBID) and `tools/check-citations.ts` fails the build on a citation that does not
resolve.

**Why.** Law 14 and Appendix C need a place where every change says what and why; Law 2 and XI-14
need every number declared; Law 6 and Appendix A need bounds and defaults to be impossible to write
by accident.

**Decisions taken here** (all in ARCHITECTURE.md): doubles with derived dust, not integer minor
units; money as an instrument; the worker boundary as the observer surface's physical form.

## 1 — Money and settlement, one calendar

**What.** `core/`, `calendar/`, `ledger/`. Money is an instrument issued by a bank or central bank;
an account is a holding of it. Every state change is a numbered instruction with two named sides per
leg; settlement applies all legs or none and generates the interbank reserve leg itself. A payment
across issuers is that money's redemption at the first issuer and issuance at the second; money whose
issuer is the central bank changes holder and is never redeemed by a transfer. Creation and
destruction happen only through an issuer's own account. Fails are recorded, never half-settled. One
calendar with 7-day periods and 5 cycles; periodicities placed by date; four day counts.

**Found.** The audit found two settlement defects before any test did: a redeem leg emitted on the
central bank's money when the payer's issuer was the central bank (money "vanished" from the CB's
stock while its holdings stood), and the issuer's liability not re-marked when two holders traded at
a price away from the previous mark (world equity was not zero-sum by the trade's realised loss).
Both fixed at the cause: the routing rule and the issuer-side effect of a holder-to-holder credit.

## 2 — Register, clearing, cells, delivery-versus-payment, value, parameter register

**What.** `register/`, `clearing/`, `prices/`, `parties/`, `world/`, `audit/`, `observer/`. Holdings
with FIFO lots and liens, both directions indexed. Cells store per-member state; a leg on a cell is
denominated per member at the weight it was struck; split and merge are exact; weights move by five
events only. One solver: uniform price at the posted price executing the most volume, pro rata at
the margin, outcomes `cleared | noDemand | noSupply | noOverlap`, never a bracket. Markets settle each
trade as one instruction (paper one way, cash the other) and print with provenance; a market nobody
posts into prints stale, visibly. Value is units × price at read; carrying value is derived from lot
basis, acquisition period and the price store (nothing stored beside units). Revaluation books mark
changes to holders and the reverse to issuers. Equity is a stated account per party moved only by
settlement effects and revaluation; the accounts family compares it to assets minus liabilities.
Seven audit families built, two report themselves as not built. The foundation seed (one currency,
one region, a central bank, a treasury, two banks, three firms, four household cells, one benchmark
line) passes the audit at period zero and stays green over a year with coupons flowing.

**Found.** Per-member and total quantities were mixed in the seed's equity read and the accounts
family (both divided a per-member figure by the weight); fixed by making every party's accounting
per member of that party. The single-currency guard in settlement (`Money A2.b`) stands until the
currency layer exists (worklist 12) and is declared there.

**Placeholders.** One: `seed.openingPrice.gov.north.2036`, standing in for the auction (worklist 3).

**Forecast, with its killer.** The `flows` and `money` families will fail the period a mechanism
writes a holding outside settlement; if a year of stepping with coupons ever passes with such a write
unreported, the families are not independent reads and must be rebuilt.

## 2a — Kernel/module boundary

**What.** The engine is now a kernel and modules (`docs/ARCHITECTURE.md` 4.9b). Kinds are registered
at assembly with their profiles (`registry/kinds.ts`); the kernel asks a profile how an instrument
prices, whether it is a liability, what unit it takes, how its terms validate, how it is named, and
what falls due. `SystemModule` (`world/module.ts`) declares kinds, units, params, phases anchored to
the kernel's, participants per party kind, audit contributions and a seed contribution;
`assemble()` orders modules by `requires`, seeds, states equity as the read and seals. Modules act
through `ParticipantView`, `MechanismContext` and `SeedContext` only; `World.register` is a frozen
read facade and the store with writes is handed to settlement, the seed and the cell events. A lint
rule forbids a module importing a sibling or the world container. The audit takes contributions per
family, so a module can build or extend a family. Journal events carry public/private visibility;
a participant sees public events and its own. The sovereign bond and bill are the first module; the
foundation seed is a module that draws dispersed household endowments from the seed's own stream
with cells per key and members per key as RESOLUTION parameters and the dispersion width as a
declared SHAPE.

**Why.** Before this, an order provider and a phase received the whole world, including every
party's private state and the register's writes; a mechanism could have bypassed settlement or
read another party's book and only the audit would have noticed afterwards. Making the contexts
the only door, and the single-writer rule a runtime fact, is cheaper now than after fifteen
mechanisms are written against the wrong surface. Inserted before item 3 for that reason (Law 10).

**Found.** Two tests had been settling instructions directly at period zero after the seal; they
were exercising a path no mechanism has. Rewritten to act inside a phase, which is the only path.

**Placeholders and shapes.** One placeholder (the opening price); three shapes (mean deposit, mean
bond holding, dispersion width of the seed), all scheduled for worklist 4.

## Between 2a and 3 — the plan in full, and the syndicate

**What.** `docs/PLAN.md` rewritten as the standing plan (what Phoenix is, the rules, the
architecture, the module contract, the build loop, the strategies, the canonical period, dos and
don'ts, risks, glossary) for a reader with no context; one detailed file per open item under
`docs/plan/` (objective, clauses met, design with kernel doors as sub-items, parameters, audit
contributions, files, a step checklist, tests, exit criteria, guard); `docs/plan/manifest.json`
with every item's step count; `tools/plan-progress.ts` recounts the completion figure into
`docs/PLAN.md` and `npm run check` fails when the figure is stale. Item 13 split into 13a–13i;
13g (corporate control) placed before 13h (insurers, hedge funds, private equity) because a buyout
bids through the tender market.

**Spec amendment.** The specification had no syndicate: an issue larger than one underwriter's own
limit could only be downsized or carried past a limit. Added Corporate Credit C10, C10.a, C10.b
(FORBID), C10.c (VERIFY): a lead forms a syndicate of named banks each taking a stated share of the
underwriting risk and fee against its own limit and capital; shares struck before the book opens;
a syndicate that cannot be filled is a deal downsized or pulled, an observable event. Added Banks
Lending D4.a: a syndicated loan is one loan with several lenders of record, a row per (lender,
borrower), one margin struck by a lead that takes a fee, each share against its own capital and
large-exposure limit. Both are built at 13f on one kernel mechanism, the commitment market
(demand is a deal's size; supply is commitment schedules from members' own limits). Coverage
regenerated: 1320 clauses; the two FORBID/VERIFY rows are MISSING until 13f.
