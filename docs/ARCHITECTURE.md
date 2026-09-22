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
Those are future work. GitHub Pages and Android workflows return with that application rather than
failing against an absent build target.

## 2. Kernel stores and ownership

`assembly::World` owns exactly one instance of each kernel store:

- parties and their named/cell representation;
- instruments and the issued terms of money, claims, shares, goods and plant;
- the holdings register and liens;
- cleared price prints and the append-only journal;
- the settlement wire and its payment queue;
- parameters, agreements, schedules, outlooks, processes, estate claims, standing terms and work in
  progress;
- the equity accounts;
- the economic registry, ontology declarations, phases, books, calendar and audit runner.

The ontology declarations are a rule, not a list. `wire_up` passes every one of those stores, every
`standing` kind and every journal kind through `Nouns::sort_of`, which throws for a name nobody
declared — so a module that keeps a list as an event kind has to say what it holds and why, and a
new kernel store fails to compile until it is named there (the check destructures `World` with no
`..`). A module's kinds are declared where the module is wired, in `systems.rs`, and the kernel's
own in `assembly.rs`.

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

Index definitions are registry data, typed by both market family and scope. Equity definitions
distinguish all-, small- and large-cap universes; fixed bonds, CDS and tradable term loans remain
distinct credit families with explicit quality bands; a scope is either one named currency or
global. The benchmark mechanism computes dated levels from those definitions and price prints, so
the registry never stores a second copy of a level.

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

How a position is carried is the holder's own declaration and the register has no default: an
undeclared row is `None`, and `credit` refuses units into one. A holder declares before it
acquires, and `Register::carry` is the kernel's to call: a module says it through
`MechanismContext::carries`, a party about to bid through `Participant::carries`, the first holder
of new paper through `Brings::carried_as`, and the seed through `OpeningHolding` and
`OpeningLeg::Asset`. Restating a live position's treatment throws: one position, one treatment,
and a reclassification is an event nobody has built. Where units are handed on rather than
acquired — a cell split, an estate to its heir — the position travels with the treatment it
arrives under unless the receiver has already declared its own.

The current valuation API contains two deliberately different reads:

- `instruments::worth` reads `units × latest cleared price`, or a hard-coded price only where the
  instrument contract permits one. Missing market value remains missing even when an accounting
  basis exists; it never falls back to that basis;
- `instruments::carrying_value` reads the treatment declared on the holder's position: current
  market value for a market-carried position or its surviving lot basis for a cost-carried one;
- `instruments::booked_equity` reads every holding through its declared carrying treatment, adds
  estate receivables and subtracts issued money/claims and estate liabilities;
- `instruments::unrealised_difference` exposes market value less carrying value without changing a
  cost-carried position's booked amount.

Market-sensitive consumers use `worth`/market book value and prudential capital reads carrying
value. The price audit reports market-carried positions without a price. Principal servicing
atomically couples cash payment with destruction of the redeemed holder claim, so outstanding
issuance falls through settlement's single writer.

`booked_equity` is the residual, and it is one of the two records Audit B5 compares. The other is
`stores::Equity`, the equity account: a party's movements, each with the `Moved` that says why, and
never a stored sum — `balance_of` adds them up at read, exactly as the residual is added up at read.
Settlement moves it as the flows settle, because settlement is where both sides of a flow are:
money received under `Receipt::Capital` is capital paid in, a wage, tax, interest or dividend leg is
income to its payee and a cost to its payer, and what a disposal realised against the basis its lots
carried is a gain or loss landed. Assembly moves it for what an exhausted estate did not pay. The
seed states each account once, as the residual its opening holdings left, so the first week is the
first week the two can disagree — and `mechanisms/reporting.rs` publishes the account rather than
the residual, so a published result is not the number it is checked against.

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

Instrument dues identify their instrument. Bilateral dues may additionally retain the `AgreementId`
that created them; assembly admits that linkage only when payer and payee are the agreement's two
parties. Private-capital calls and derivative variation margin use this path. Central-bank advances
strike a typed facility agreement and its principal and interest schedules together, so runtime
mechanisms no longer create an uncontracted bilateral due. Wire failures record a durable breach on
the owning agreement, later successful performance records cure, full performance at contractual
term records discharge, and an explicit early end records termination. The event history remains
readable after the current agreement state changes.

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

## 5. Fixed weekly time, phases and execution

Executable time has exactly one representation: `calendar::Week`, a monotonically increasing weekly tick from the fixed epoch of 1 January 2000. The clock advances by one `Week` per world step. Schedules, agreements, payments, settlements, instruments, journals, sessions, mechanisms and diagnostic binaries all store this type (or the corresponding journal column); there is no configurable epoch, tick length, day clock or period clock. `RunConfig` therefore contains no calendar resolution. Short-term and formerly overnight funding execute at a one-week tenor.

`calendar::CivilDate` is an input/output boundary value only. `Calendar::week_on_or_after` maps an external Gregorian date deterministically to the first weekly tick on or after it, and `Calendar::civil_date` presents a weekly boundary. Production mechanisms and stores never carry a `CivilDate`. Recurring executable schedules use fixed numbers of weekly ticks.

Financial measurement is separate from scheduling. `Convention::{Actual360, Actual365}` computes a year fraction from the number of seven-day intervals between two `Week` boundaries. Yield, coupon and curve calculations retain their declared convention; the resulting civil-day count does not create a schedulable daily clock.

A week is a sealed, single pass over nine kernel stages:

1. opens;
2. obligations due;
3. population;
4. work;
5. views/orders;
6. books;
7. judgments and marks;
8. scheduled decisions;
9. closes and audits.

Stages express causal order inside the atomic weekly tick, never intraday timestamps. Stage 5 is two
things, and the kernel runs them in that order: the module phases anchored there form each party's
view, and then `POSTS` — a kernel phase added after all of them — asks every participant what it
wants and takes its pulls. Nothing clears there; the books clear a stage later on exactly what was
posted into them, so a party reads everything above it and nothing below. When a party may post is
therefore not a thing a system declares: a row whose only act is to post declares no phase at all,
and a row that also runs a mechanism declares that mechanism's stage.

Systems declare phases anchored to those stages, and each phase declares the journal kinds its own mechanism says and the kinds it needs of the week it runs in. A kind a phase wants of an earlier week is ordered by the calendar and declares nothing. `Phases` rejects duplicate declarations, phases outside the nine stages, mutation after sealing, two phases writing one kind, and a same-week read placed before its writer. A print is not declared: it is written at `BOOKS` by the one solver, and every phase that reads one is in a later stage by the stage order itself.

`phoenix-check` enforces the boundary: legacy `Day`/`Period`, configurable tick fields, daily durations, overnight names and `CivilDate` outside `calendar.rs` are findings. Its tests cover both the boundary allowlist and forbidden production examples.

## 6. Clearing and prices

The kernel supports three venue protocols:

- call clearing at a price selected from posted levels;
- posted-price search with bounded visibility;
- an order book with resting and arriving orders.

Orders name a party, side, quantity and optionally a limit. An absent limit represents a forced or
market order: it can take only a finite level named by a different party on the other side. Two
unpriced orders do not clear, an order cannot trade with its owner's resting order, and posted sellers
must name the level they stand behind. Price rules and rationing live in the clearing layer, while the
session layer owns the mapping from a fill to delivery and payment.

`Prints` keys an observation by `(market, instrument, week)` and carries the currency, the quote kind
and the provenance, so the same grade in two places keeps two runs and one book cannot print twice in
a week. There are two reads: `latest` answers what a named book printed, and `of_line` answers what a
line printed for a holder that has units rather than a seat — and refuses where the line has printed
in more than one market, because which level its units mark at turns on where they are. Every print
also carries the week its level was struck in, so a carried mark's age is `week - struck` rather than
a walk back through the run. The current provenance distinguishes cleared trades, carried earlier
levels and seeded opening levels.
Mechanisms that require a transacted value must check the provenance appropriate to their decision;
a print is an observation, not permission to treat an outcome as a primitive.

Treasury funding reads the same dated schedule as settlement: bilateral receipts follow their named
payee and instrument receipts are allocated pro rata to current holders. Its cash-buffer target is a
durable mandate row held by the treasury, seeded once from Parliament's declaration; the auction
participant reads that party-owned row rather than a universal behavioural preference.
Completed book sessions are durable kernel facts, including the submitted orders and non-clearing
outcomes. This lets a sovereign auction retain requested units, filled units and actual proceeds even
when it partially clears or attracts no demand; the later inability to service a due remains a
separate event rather than being inferred from the auction result.
A sovereign's willingness to pay is likewise durable party-owned mandate state. A missed due in its
home currency is therefore recorded as refusal; a missed foreign-currency due is inability only when
that mandate still says it is willing to pay, and otherwise is refusal. The treasury-account funding
constraint remains separate: there is still no automatic central-bank overdraft.
Issuance does not rely on a single paper id captured during wiring: the treasury participant finds
every book for a line it issued. Every dated line produces its schedule from its contract, including
the principal-only schedule of a zero-coupon bill, so a newly auctioned line cannot become debt that
never falls due.

## 7. Parties, cells and cessation

A party has a kind, region, bank, representation, lattice key, entry period and live/dead state. A
kind selects system eligibility only. Its registry profile separately declares operational
capabilities—including banking location, money or paper issuance, and the accumulated state that
can end its legal life—so mortality, risk weights and facilities do not branch on kind IDs.
A representation is either named or a homogeneous cell with a non-zero member count. Household
cells use a joint age × composition × employment × income × tenure × liquid-wealth × debt-service
lattice. Small-business cells use sector × age × size × productivity × leverage × coverage ×
credit-access. These joint coordinates follow the state dimensions used by household
microsimulation/HANK work (HFCS and distributional national accounts) and firm-demography/firm-
dynamics work (OECD-Eurostat business demography and Census BDS), rather than independently sampled
margins or a representative household/firm. A weight moves by one of five events and by nothing else, and each one is recorded: `reweigh` is
the only writer of a weight and it appends a `PopulationEvent` saying what the weight was, what it
became, which event it was, and the journal row that says it happened. Entry, split and merge each
record both sides, so the standing cells and the history are two records of one population, and
`weight_conservation_gaps` is their comparison. `World::admit` is the entry door: a cell is admitted
through `enter_household` or `enter_small_business` against a journal row, never by a counter edit.
Cell splitting is implemented by creating a child and moving the leaving members' share of everything they held over
the ordinary wire — encumbered units included, each lien released on the parent for that share and
re-created on the child once the units are there, because a claim is over units and follows them —
then copying the parent's entry date, memory and outlook history and moving the applicable
agreement. Both cells stay homogeneous, and the units family is what says so. Duplicate rejection, transition-specific child coordinates and the inverse merge
transition remain implementation-plan work. The other population
transitions remain incomplete and are tracked in the implementation plan.

The mortality module consumes payment, funding, capital, waterfall and dissolution states and opens
an estate, heir or resolution destination before ordinary discretion ends. Assembly converts unpaid
schedules and bank deposits to ranked claims, moves relations to the legal authority, opens
instrument-specific liquidation workouts and transfers an inheritance over the wire. The estate
mechanism distributes realised cash by rank and records the unpaid residual as creditor loss.

That flow is implemented, but its selection logic still branches directly on party-kind constants.
Appendix B #48 requires the failure capabilities and legal destination to be declared attributes or
contract terms instead. Derivative agreements now use class-specific typed terms, preventing a CDS reference,
price-forward instrument or FX currency from being interpreted as another class's numeric field.
The active engagement, mortgage, tenancy, mandate, fund-subscription, private-commitment,
prime-brokerage, securities-loan and trade-credit paths also use typed terms. Opening-state
agreements use a corresponding symbolic typed schema that resolves names only after validation.
Unused policy, supply, carriage and committed-credit numeric placeholders were removed rather than
being mistaken for implemented contracts.

## 8. Systems and causal wiring

`systems::all` is the authoritative assembly table for what currently runs. It activates many
real-economy, funding, capital, market, reporting, expectations, loss, forced-sale, mortality and
estate mechanisms and attaches participants where a row posts into books. A wired row may contain a
mechanism, a participant, or both. `World::wire_up` rejects duplicate system names and rows with
neither behavior. It is not evidence that every helper in a wired module is on the production path.
For example, `irs.rs` still contains no `Mechanism`; commodity-futures order, margin and delivery
types are helpers inside the wired spot-commodities module but are not consumed by `Storing`; and the
benchmark row now publishes consumer-price, broad-price and constituent families as well as the
weekly funding fixing.

The spot-commodities row also owns the physical-units audit for registry-declared goods. It reads
settled creation and destruction from the wire and closing stock from the register, retaining only
the prior week’s stock for the next comparison; it does not introduce a second inventory ledger.
Its public observations carry physical tightness and named production available at the cleared price;
settled consumption, stock and the print remain authoritative in their owner stores. Location-specific books, production disruptions and the
downstream currency and policy links remain explicit backlog rather than inferred coverage.

Many mechanism files contain richer pure functions than their production `Mechanism` consumes, and
some have no production implementation at all. This distinction is material in the milestone 8–11
code: the reporting, ratings, control, polity, CDS, FX-forward, securities-lending, prime-brokerage,
private-equity and securitisation rows do run, but several contract helpers added beside them are
still test-only. Population entry, death, promotion and merge doors likewise exist in `Parties`
without a system that calls them, while agreement expiry and process completion are called by the
weekly opening path. Freight's wired `Carriage` remains a count of live agreements; its typed
`Dispatches` store is a field on `Settlement`, and nothing reads a delivery outcome out of it. A helper covered by unit tests is
not a wired economy. The remaining plan therefore distinguishes (a) kernel contract defects, (b) an
absent production mechanism, and (c) a broken causal handoff; it does not infer completion from
module existence, a declared journal kind, or a system row.

Expectations are party-specific store rows. `expectations::Forming` derives outlooks from observations
available to each party and writes them after a lag. Each party receives one reproducible, dispersed
memory draw when it enters; price outlooks are keyed by instrument, so unlike price units are never
averaged together. Each subsequent observation records expected and observed values as a durable
surprise, and confidence is computed from that party's own recent absolute errors. Market participants
use those outlooks to form
mechanism-specific reservations under their own cash, inventory, mandate and holding-cost constraints.
Reservation-price multiples and a declared
dealer spread are not run parameters. A dealer converts its money inventory limit to units at its own outlook and derives quote width from
its own recent forecast errors, falling back to disagreement with the last public print until that
history exists.

The treasury's fiscal programme is also a store-to-store causal path rather than a statement
calculation. Typed public-purchase, transfer and treasury-employment agreements create bilateral
scheduled obligations to their named recipients. Settled wage and sale legs create tax obligations
for the named statutory payer, while positive reported firm income creates the profit-tax
obligation. Because `WORK` follows the week's `OWED` pass, all obligations created by this path fall
due at the next weekly boundary; the funding programme can therefore see them before servicing can
present them to settlement. It reads the treasury's own cash, incoming schedules, outgoing
schedules and durable buffer mandate, brings a dated line for the uncovered amount, and the ordinary
call book determines how much sells and at what price. Unsold paper remains with the issuer: no
central-bank facility or synthetic residual buyer completes the auction.

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
zero-sum, units and liveness. A family with no contribution reports `not built` rather than passing,
and every one of the ten now has one. The population is the units family's: what a merged cell still
holds or is still named by, what the weights of a kind in a region come to, and that what a cell
holds divides by its weight — a cell being homogeneous, its holding is `weight × what one member
holds`, and a total that does not divide is the average the cell exists to avoid being. Only the
pieces are checked that way: a money account divides into what a money is divided into, which a
holding does not carry. Mechanisms may add family contributions during system assembly. The scale world's violations are findings about its
arbitrary construction, not values to normalize away.

## 11. Validation and generated documentation

The repository gate is `npm run check`:

- `phoenix-check` enforces source-level project laws and its own tests;
- Cargo runs the Rust kernel test suite;
- TypeScript type-checking and tool tests validate the documentation tools;
- the reach walk takes the transitive closure of the names reachable from `systems::all` and a
  built, wired, stepped `World`, excluding `#[cfg(test)]` blocks and `src/bin`, and ratchets the
  count of public items outside it;
- the coverage existence checker verifies source citations, the measured coverage table, that a
  `MET` row rests neither on an item outside that closure nor on a name no file in the tree
  contains, and that no fixed document points into the implementation plan.

`npm run plan:gaps` regenerates Part 4 of `docs/IMPLEMENTATION.md` from the specification index and
`docs/COVERAGE.md`. Part 4 is a coverage ledger, not execution order. Parts 1–2 are the maintained
dependency order and must remain grounded in the production source.

Clippy is available as `npm run check:clippy` and CI runs it after `npm run check`. No web, Pages,
Playwright or Android build runs while those packages are absent; application jobs return only when
there are real targets for them.

## 12. Deployment boundary

No observer UI or deployment bridge is implemented today. When one is built, it must preserve these
boundaries:

- the Rust kernel remains the one engine;
- the observer receives immutable, serializable snapshots or query results and cannot mutate kernel
  stores;
- user controls submit declared inputs or actions through a narrow API, never references to internal
  state;
- browser and Android packages share the same engine artifact and model semantics;
- CI commands describe packages that actually exist.

Until then, architecture and implementation documentation must not cite TypeScript engine files,
application packages, worker APIs, persistence formats or lint rules that are absent from the tree.
