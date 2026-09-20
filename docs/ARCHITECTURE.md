# Project Phoenix — Architecture

This document describes the architecture that exists in the repository. The specification in
`docs/spec/PROJECT_PHOENIX.md` defines the required economy; `docs/IMPLEMENTATION.md` owns the
remaining work and its dependency order. Architecture statements must describe code that is present,
not a design from an earlier implementation or a planned application.

## 1. Repository and runtime

The engine is the Rust 2021 crate in `packages/kernel-rs`. It is an in-process library with no DOM,
network, storage, system clock or ambient randomness. The binaries in `src/bin` construct synthetic
worlds and exercise the same library at scale; they are benchmarks and diagnostics, not a calibrated
seed or a second engine.

The Node/TypeScript code under `tools` indexes the specification, verifies coverage and regenerates
the coverage backlog. It does not implement the economy. There is currently no web application,
worker/WASM bridge, Android wrapper, snapshot persistence format or APK build in the repository.
Those are future work under implementation item 24. The GitHub Pages and Android workflows are
therefore aspirational until that application exists.

## 2. Kernel stores and ownership

`assembly::World` owns exactly one instance of each kernel store:

- parties and their named/cell representation;
- instruments and the issued terms of money, claims, shares, goods and plant;
- the holdings register and liens;
- cleared price prints and the append-only journal;
- the settlement wire and its payment queue;
- parameters, agreements, schedules, outlooks, processes, estate claims, standing terms and work in
  progress;
- the economic registry, ontology declarations, phases, books, calendar and audit runner.

IDs are small typed row identifiers (`PartyId`, `InstrumentId`, `MarketId`, `CurrencyCode`, and
others), not display names. `ids::Names` is the explicit name-to-row boundary. Amounts and quantities
inside the stores are presently `f64`; positive settlement quantities are wrapped in `ledger::Units`.
Currency is carried structurally by the money instrument or by the row that owes the amount—for
example, a scheduled payment stores its `CurrencyCode`, and a price print stores its currency. There
is not yet a universal `Cash { amount, currency }` value type, so callers must preserve these
structural associations rather than claim compile-time dimensional safety that the code does not
provide.

The parameter register already exists. `RunConfig` records the run seed, epoch, period/payment-system
resolution and unit grids before state is constructed; `World::with_parameters` builds the single
`Params` instance from that configuration and an explicit declaration function. `Params` declares a
value with its dimension, denomination, kind, owner, reason and placeholder metadata, rejects
duplicate or non-finite declarations, and refuses undeclared or dimensionally incorrect reads. It
also exposes a stable snapshot of the declarations actually consumed by a run. Mechanisms receive
the register through their read facade, and `systems::declare` supplies the current declarations.

`opening::OpeningState` is the inert, validated description of period zero. It names parties and
representations, reporting currencies and banks, instruments and issuance, holdings and bases,
obligations, agreements, opening instructions and the parameters construction requires. Validation is read-only
and reports all discovered faults together: duplicate or unresolved identities, incompatible bank
accounts and currencies, non-finite or non-positive amounts, holdings that do not reconcile to
issuance, malformed instruction legs and undeclared construction parameters. `OpeningState::build`
admits its currency topology, parties and instruments, creates opening stocks through issuance and
the wire rather than direct register writes, settles explicitly declared DvP or FoP instructions,
and records obligations and agreements through their single-writer stores. It returns the stable
name-to-row mapping used during construction. `OpeningState::generate` gives the generator the
world's one parameter register and one fixed-algorithm `OpeningDraw` stream seeded from `RunConfig`;
the reproducibility test compares the generated description, consumed parameters, wire event stream
and period-zero audit result across identical runs, then separately varies the seed and an owned
parameter while holding declarations and topology fixed.

## 3. Instruments, holdings and value

Money is an instrument issued by a named party. A customer's account is its holding of its bank's
money; a bank's reserve account is its holding of central-bank money. `Instruments` is the one store
for issuer, currency, class, unit, coupon and maturity. `Register` is the one holdings store and is
indexed by both holder and instrument. Non-money holdings retain lots and acquisition basis; liens
reduce the free quantity that settlement may deliver.

Issuance and outstanding amounts are distinct from holdings. Settlement moves issued amounts for
`Create`, `Mint` and `Destroy` legs, while the ownership and money audit contributions compare the
resulting stocks. Direct register mutation remains public for construction and scale fixtures, so a
supported seed must use validated construction doors and audits rather than assume the Rust type
system makes bypasses impossible.

The current valuation API contains two deliberately different reads:

- `instruments::worth` reads `units × latest cleared price`, a hard-coded price where the instrument
  contract permits one, or lot basis for an instrument declared carried at cost;
- `instruments::equity` reads holdings at lot basis, adds estate receivables and subtracts issued
  money/claims and estate liabilities.

This is not a complete accounting architecture. Carrying treatment is currently declared per
instrument rather than per holder position, and the independent accounts/price audit families are
not complete. Implementation item 0n owns the separation of market value, position carrying value
and booked equity and the routing of those reads to margin, NAV, mandates, prudential capital and
published accounts.

## 4. The settlement wire

`ledger::Settlement` is the single production path by which mechanism proposals and market fills
change holdings. An instruction contains one or more typed legs:

- `Money`, naming payer, payee, money instrument, positive amount and receipt class;
- `Asset`, naming deliverer, receiver, instrument, positive quantity and optional trade price;
- `Create`, `Mint`, `Destroy`, and `Pledge`.

Every instruction declares `Delivery::AgainstPayment`, `Free` or `Nothing`. The wire derives the
shape from its legs and rejects a false declaration. It pre-checks all legs and applies all or none,
so DvP is atomic. FOP is explicit and recorded as an exposure rather than inferred from a missing
cash leg. There is no reversal operation; corrections require another instruction.

For a payment across two commercial banks, the wire:

1. identifies the payer's money issuer and payee's bank;
2. requires the payee account to be in the same currency;
3. requires both banks to settle in the same reserve instrument;
4. moves the payer's deposit, credits the payee's deposit, and transfers reserves between the banks
   in the same atomic instruction.

It records distinct outcomes for settlement, queuing, short money, bank settlement failure, missing
same-currency account, encumbered assets and short units. A fresh short-money payment can enter the
queue; a later receipt retries it. The close stage finds pure-payment gridlock cycles and attempts
them together, then expires payments whose waiting period ended. This wire, queue and currency
routing already exist and are prerequisites to use, not milestones to rebuild.

A servicing instruction carries the `DueId` it attempts to perform. Settlement emits the matching
due update on initial settlement, queueing, retry, gridlock completion or expiry, and assembly
applies that outcome to `Schedules`. A proposal can therefore never mark an obligation paid: it
remains dated and outstanding while queued, becomes paid only after settlement, and retains the
grace date or typed final failure when performance does not occur.

`loss::Losses` reads those contractual states rather than re-estimating default from current cash.
A final failure moves the named borrower and claim to non-performing; declared policy clocks advance
an unresolved claim to impaired and written-off, while a later settled attempt cures it to
performing. Write-off publishes the remaining scheduled loss to each named holder in proportion to
its current units. Any money actually recovered through a due-associated settlement accumulates on
the schedule first, so partial collateral or estate proceeds reduce the residual before allocation.

`mortality::Failing` consumes accumulated kind-specific states rather than applying one universal
negative-equity rule. Firms require a written-off claim, banks distinguish final funding failure
from capital exhaustion, funds and insurers use liabilities exceeding assets, sovereigns consume
final payment failure, and households require an explicit dissolution event. Central banks do not
cease merely for negative equity. Every cessation request carries a typed trigger and an estate,
heir or resolution destination path, and the same facts are published in the journal for the later
estate/resolution mechanism.

## 5. Calendar, phases and execution

There is one `Calendar`, currently configured by `World::empty` with seven-day periods. Contractual
payments use civil `Day` values and day-count conventions; period execution uses a monotonically
increasing integer period.

A period is a sealed, single pass over nine kernel stages:

1. opens;
2. obligations due;
3. population;
4. work;
5. views/orders;
6. books;
7. judgments and marks;
8. scheduled decisions;
9. closes and audits.

Systems declare phases anchored to those stages. `Phases` rejects duplicate declarations, phases
outside the nine stages, mutation after sealing, and a same-period read placed before its declared
writer. Within a stage, assembly order is model order. The kernel owns opening/expiry work, book
sessions and the closing gridlock/audit pass; system mechanisms run in their declared slots.

A mechanism receives `MechanismContext`, a read facade over the stores plus an accumulator of
requests. It cannot directly mutate the register. It can propose instructions and request declared
changes such as obligations, agreements, processes, outlooks, claims, standing terms, cell splits and
cessation. `World::run_phase` applies those requests through the owning stores and submits proposed
legs to settlement.

A market participant receives `ParticipantView` for one party. It selects markets and posts orders
from that party's holdings, funds, terms and public/subject-visible observations. The facade does not
currently expose the party's stored outlook, which is part of item 0p's missing decision handoff.
`session::run_book` clears the book using its declared venue protocol, converts fills to DvP
instructions, settles them, updates resting orders and writes a price only for settled volume.

## 6. Clearing and prices

The kernel supports three venue protocols:

- call clearing at a price selected from posted levels;
- posted-price search with bounded visibility;
- an order book with resting and arriving orders.

Orders name a party, side, quantity and optionally a limit. An absent limit represents a forced or
market order and must not manufacture a price by itself. Price rules and rationing live in the
clearing layer, while the session layer owns the mapping from a fill to delivery and payment.

`Prints` stores `(market, instrument, period)` observations with currency, quote kind and provenance.
The current provenance distinguishes cleared trades, carried earlier levels and seeded opening
levels.
Mechanisms that require a transacted value must check the provenance appropriate to their decision;
a print is an observation, not permission to treat an outcome as a primitive.

## 7. Parties, cells and cessation

A party has a kind, region, bank, representation, key, entry period and live/dead state. A
representation is either named or a homogeneous cell with a non-zero member count. Cell splitting is
implemented by creating a child, moving a proportional share of every free holding over the ordinary
wire, copying the parent's outlook history and moving the applicable agreement. The other population
transitions remain incomplete and are tracked in the implementation plan.

The mortality module defines distinct triggers for payment failure, balance-sheet failure, bank
funding and capital failure, clearing-waterfall exhaustion, sovereign payment failure and household
dissolution. It also defines estate, heir and resolution destinations. The production `Failing`
mechanism does not yet consume that representation: it currently ceases every eligible party whose
basis-read equity is negative. Item 0v replaces that universal rule with kind-specific state and
triggers.

The estate module already contains legal ranks, pro-rata treatment within rank and a mechanism that
pays existing claims from a ceased party's cash. The forced-sale module already opens mandate
workouts and supplies a fund participant that submits unpriced orders to actual books. What is not
complete is the connection: cessation does not generally create the destination and full claim set,
transfer every relationship, place estate assets into forced sale or route realized proceeds into
the waterfall. Item 0q owns that integration.

## 8. Systems and causal wiring

`systems::all` is the authoritative assembly table. It activates the real-economy, funding, capital,
market, derivatives, reporting, expectations, loss, forced-sale, mortality and estate mechanisms,
and attaches participants where a system posts into books. A wired row may contain a mechanism, a
participant, or both. `World::wire_up` rejects duplicate system names and rows with neither behavior.

Many mechanism files also contain richer pure economic functions than their current production
`Mechanism` implementation consumes. That does not make the modules absent or dormant: the period
runner calls their production implementations. The remaining work is to close causal handoffs—an
output must reach the next bounded decision, settlement or explicit failure state and an independent
audit—not to create duplicate sector modules. Implementation items 0r and 0u own those vertical
slices and remaining cross-system arcs.

Expectations are party-specific store rows. `expectations::Forming` derives outlooks from observations
available to each party and writes them after a lag. The market participants do not yet consistently
use those outlooks to form mechanism-specific reservations; several still use shared parameterized
multiples or widths. Item 0p replaces those outcome-like primitives with decisions from each party's
legal observations, constraints, alternatives and outlook.

## 9. Journal, claims and other persistent relations

`Journal` is append-only and indexes events by period and kind. Each row records subjects, typed
numeric/identifier values and whether it is public. `ParticipantView` enforces the basic observation
boundary: a party can read a public event or one that names it as a subject.

`Schedules` records who owes a payment, what instrument or counterparty it is owed on, its currency,
covered interval, due day, amount, kind and paid state. `Agreements` records bilateral relations and
terms. `Standing` records replaceable public or private terms such as grades. `Processes` records
work that spans periods. `Claims` records holder, estate, amount paid and legal rank. `InProgress`
records owned production batches. These stores are the persistent facts mechanisms share; journal
messages do not substitute for updating the owning store.

## 10. Audits

The audit runner is assembled with the world and executes every period after settlement's gridlock
pass. A contribution receives read-only sources and reports violations with family, specification
citation, owner, size, unit, period and message. It cannot repair state.

There are ten declared families: money, ownership, prices, cross-market, accounts, names, flows,
zero-sum, units and liveness. A family with no contribution reports `not built` rather than passing.
The currently assembled core contributions check total-versus-lot representation, collateral reuse,
holdings against issuance, money conservation, flow completeness and name resolution. Mechanisms may
add family contributions during system assembly. The scale world's violations are findings about its
arbitrary construction, not values to normalize away.

## 11. Validation and generated documentation

The repository gate is `npm run check`:

- `phoenix-check` enforces source-level project laws and its own tests;
- Cargo runs the Rust kernel test suite;
- TypeScript type-checking and tool tests validate the documentation tools;
- the coverage existence checker verifies source citations, partial-item ownership and the measured
  coverage table.

`npm run plan:gaps` regenerates Part 4 of `docs/IMPLEMENTATION.md` from the specification index and
`docs/COVERAGE.md`. Part 4 is a coverage ledger, not execution order. Parts 1–2 are the maintained
dependency order and must remain grounded in the production source.

Clippy is available as `npm run check:clippy` but is not currently part of `npm run check`. The CI
workflow still contains obsolete `npm run build` and `npm run e2e` steps from the removed application;
those commands do not exist in `package.json` and must be replaced when item 24 introduces the actual
application toolchain.

## 12. Deployment boundary

No observer UI or deployment bridge is implemented today. When item 24 is built, it must preserve
these boundaries:

- the Rust kernel remains the one engine;
- the observer receives immutable, serializable snapshots or query results and cannot mutate kernel
  stores;
- user controls submit declared inputs or actions through a narrow API, never references to internal
  state;
- browser and Android packages share the same engine artifact and model semantics;
- CI commands describe packages that actually exist.

Until then, architecture and implementation documentation must not cite TypeScript engine files,
application packages, worker APIs, persistence formats or lint rules that are absent from the tree.
