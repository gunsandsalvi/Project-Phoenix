# PROJECT PHOENIX

## The specification of a bottom-up economic and financial world

---

## 0. About this document

### 0.1 What it is

This is the specification of a simulated economy built from the bottom up: people, households, firms,
banks, markets and a state, each a named party acting for its own reasons, whose interactions produce
every price, quantity and aggregate the world shows.

It says **what exists, what each party decides and from what, what must always be true, what must be
measured, and what must never exist.** It says nothing about how any of it is built. There are no
languages, no data layouts, no program structure and no assessment of any existing build. Every one of
those is a decision for whoever implements this, and the specification is written so that a developer
who has never seen any earlier work can implement it from this text alone.

The one requirement about the delivered world rather than the modelled one is the **performance budget**
(N8): the world must run on a stated device within a stated time per turn. It constrains what any build
must achieve; it chooses nothing about how. Its companion is **REP**, which states what any representation of a
real-sized population must make true — what is exact, what is approximated and how that is measured — and
likewise chooses no method.

### 0.2 The objective

**A realistic, bottom-up economy in which every outcome is caused.**

- **Realistic**: the mechanisms are the ones real economies run on, with real named counterparties,
  intermediaries, lags, fees, refusals and failures, and the world it produces is judged against the
  documented facts of real economies (§N3).
- **Bottom-up**: nothing above the level of a party is set. Prices, quantities, distributions, cycles,
  crises and growth are outcomes of individual decisions meeting in markets.
- **Every outcome is caused**: every unit of money and every unit of anything held has a named holder and
  a named counterparty at every moment; every price was formed by a real mechanism; nothing is bounded,
  plugged or invented; and the instruments that check this are themselves true.

### 0.3 How it is organised

The document is ordered **by causal layer**: each thing appears after everything it depends on. The
order is mostly a dependency order. Where systems depend on each other — a bank's funding and the money market,
the treasury and its auctions, the central bank and the market it frames — each names the other in **Depends
on**, and Part O builds them in the same stage or names the placeholder that stands in until the later one exists.

| Part | Layer                                                       |
| ---- | ----------------------------------------------------------- |
| I    | The laws of the model                                       |
| II   | How requirements are written                                |
| A    | Foundations: time, parties, numbers, chance, the physical world, how populations are represented, the opening world |
| B    | Money and ownership: money, settlement, instruments, accounting |
| C    | Price formation, expectations and valuation                 |
| D    | People: population and households                           |
| E    | Production: technology, firms, capital                      |
| F    | Real markets: goods, services, freight, labour, housing and land, trade credit, energy |
| G    | Banking and credit                                          |
| H    | Capital markets                                             |
| I    | Risk transfer: derivatives, insurance, pensions             |
| J    | The state                                                   |
| K    | The open world: currencies and cross-border activity        |
| L    | Transmission: the causal chains that run across systems     |
| M    | Observation: what can be seen and published                 |
| N    | Measurement and acceptance                                  |
| O    | Build stages                                                |
| App. | Glossary, prohibitions, primitive catalogue, scope, decisions |

### 0.4 Scope in one paragraph

In scope: a world of three generated countries on a physical map, sized and shaped by a short setup, each with its own
currency, central bank, treasury, tax system, social insurance, parliament and banking system; a population of
hundreds of millions of people living in households that are born, age, work, consume, save, borrow, migrate and die;
millions of firms that are born, produce goods and services with technologies that improve, invest, trade, borrow,
merge and die; the markets for goods, services, labour, housing, land, commodities, energy and freight; and the
financial system of money, payments, cash, banks with term loans, credit lines and mortgages, securitisation, money
markets, sovereign and corporate debt, equity, funds, dealers, derivatives including options, insurance and pensions.
Out of scope is listed with reasons in Appendix D.

### 0.5 How to use it

- **Implementing a system**: read Part I, Part II and the system's own section, then every section its
  **Depends on** line names. Its **Done when** list is the acceptance test.
- **Deciding a question the text does not settle**: apply Part I. If Part I does not settle it, the
  question is recorded in Appendix E as an open decision, and nothing is invented in its place.
- **Changing this document**: see §II.6. Identifiers never move and never get reused.

---

# PART I — THE LAWS OF THE MODEL

Seventeen laws. They govern every other part, none restates another, and a design that breaks one is wrong
however well it works. They are about **the world**, not about how the work is organised.

### Law 1 — Reflect the real mechanism

When in doubt, the answer is **how it actually works**, with real named parties. Where a real economy has
an intermediary, a lag, a fee, a contract, a refusal or a failure, the model has one. A simplification is
allowed only when it is **declared**: stated where it applies, with what it loses, and listed in
Appendix D or E.

### Law 2 — Every declared number is a primitive, and primitives are few

A number the model is given, rather than one it produces, is exactly one of six kinds:

| Kind           | What it is                                                                      | Examples                                                      |
| -------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| **TECHNOLOGY** | what a physical or biological process takes or yields                           | a recipe, a lead time, a useful life, a mortality schedule    |
| **PREFERENCE** | what a party wants, and how it weighs time, risk and memory                      | time preference, risk aversion, tastes, memory, leisure       |
| **POLICY**     | what an institution chooses, with a named owner                                 | a tax rate, a capital ratio, a haircut, a financing regime    |
| **ENDOWMENT**  | the state the world opens with                                                  | terrain, deposits, the opening population and balance sheets  |
| **RESOLUTION** | a numerical choice about representation, set for play by measuring the budget (N8.5) | the representation's factor, zones, map grid, preference types |
| **SHAPE**      | a claim about the answer: a placeholder for an unbuilt mechanism, or a standing assumption with its reason | a placeholder decision; the forecasting heuristics            |

Everything else — ownership, prices, quantities, shares, capacities, allocations, distributions — is an
**outcome**. A SHAPE is one of two things, and says which. A **placeholder** names the mechanism it stands
in for and dies in the change that builds it; the count of placeholders is the honest measure of how much
model is missing. It only falls, except by the placeholders a build stage introduces for systems of later stages,
each naming the system that retires it. A **standing SHAPE** is a claim about the answer that no mechanism in
scope can replace — how people forecast, how terrain is generated — declared with its reason and its source;
standing SHAPEs are listed, and a new one needs a reason no mechanism can serve.

**A real-world primitive may be imported; a real-world outcome may not** — except as the opening world's snapshot of
the present (GEN), which is where the world starts: state, never a rule, and the opening day alone counts as no
evidence of what the world does (GEN.10). A statutory tax rate, a life table or a recipe may come from data. A market
share, a spread, a leverage ratio or a growth rate may not: those are answers, and importing one means the model can
never tell you anything about it. Outcomes may be **compared** with data (§N3); they are never **tuned** to it.

**A residual with no holder is a defect**: a quantity computed as "everything minus the parts we know" must
have a named owner, or the computation is wrong.

### Law 3 — Every price is formed by a real mechanism between parties with reasons

A price is legitimate when it comes out of one of the price-forming mechanisms real economies use (§MKT):
an auction or order book, a dealer's two-way quote, a posted price facing buyers who may walk away, a
bilateral quote a counterparty may refuse, or a negotiation between named parties. It is never computed by
the model on anybody's behalf.

- Yield, spread, discount margin, implied volatility, a price/earnings ratio and a cap rate are
  **statistics derived from a price**, never the mechanism that sets it.
- **The one exception** is an **administered** rate — a central-bank facility rate, a statutory benefit, a
  regulated tariff — and it qualifies only when a real quantity responds to it on both parties' books.
- A price that no mechanism formed is **absent**, and absence is visible (Law 8).
- The **opening world's** prints are its snapshot's present values (GEN.5), one per market, each replaced as its
  market meets; its first posted prices are its sellers' own day-zero decisions (GEN.13).

### Law 4 — One representation per real thing, one writer per fact, and read rather than re-derive

Every real thing has exactly one representation, and every fact has exactly one writer. A reader **reads**
the fact where it is held; it does not recompute it, keep a second copy, infer it by subtraction, or model
a price a market already formed. Two disconnected representations of one real thing — a ledger and a
parallel formula, a stated ratio beside a real book, two index systems — are the defect this law exists to
kill. A party's **own opinion** (a reservation, a valuation) is not a re-derivation: it is a different fact
with a different owner.

### Law 5 — Every flow has two sides, and both move together

Every movement of money or of anything held has a giver and a receiver, both named, and both legs move in
the same settlement or neither does. Each leg is in exactly one currency or unit; an exchange of two
currencies is two legs that settle together. An instruction may have many legs — a payroll, a market's
settlement — and then every giver and every receiver is named and all legs settle together; money is fungible
within one instruction, so no leg is paired with another. A **physical transformation** — production by a way,
extraction from a deposit, consumption, spoilage, destruction by a hazard — is not a flow between parties: its
legs name the party whose units change and the recipe, deposit or event that accounts for them (SET.9). **A
one-sided flow is a defect even when nothing fails.**

### Law 6 — No invented bounds; real limits are declared

The model never caps, floors, clamps, damps, rescales or truncates a number to keep it in a range. If a
number explodes, the mechanism that should hold it is missing: build it.

Real limits exist and are **declared**, each as a primitive with an owner: physical capacity (TECHNOLOGY),
limited liability and contract terms (the contract), credit lines, position limits and regulatory ratios
(POLICY). A declared limit binds only the party it belongs to, and binding is an event that party sees and
reacts to. **Arithmetic impossibility** — a negative count of physical units, a share above its whole — is
not a bound but a contract violation (§II.5). A price may be negative wherever getting rid of something
costs money.

### Law 7 — An identity holds exactly

A conservation identity holds or it does not. A check may forgive only the error the arithmetic itself
introduces — on the order of *(number of terms) × (machine precision) × (sum of absolute magnitudes)*,
derived per check. **A percentage tolerance is a business judgement in a numerical costume**, and a check
that only passes with one is reporting a defect. **Money is counted in whole smallest units** of its currency
(MON.16), so every identity over money holds with no tolerance at all; the arithmetic allowance applies only
to quantities measured on a continuous scale. An identity that passes through a price (units times a net asset
value, face times a mark) holds to the rounding that price's convention states, and the residue lands on a named
party.

### Law 8 — Every number carries its meaning

Every quantity carries its **unit** (money in a named currency, shares, face, tonnes, hours, dwellings,
persons), and every rate, flow and index carries its **period** and its **date**. Two currencies are never
added without an exchange at a stated rate. A value that does not exist is **absent**, never zero; a price
of zero is a price and it propagates. A number older than its reader expects says so, and a legitimate lag
is distinguishable from staleness. **A displayed change with no history behind it is a lie; show the
level.**

### Law 9 — Things are named and keyed as their market names them

A bond is issuer + coupon + maturity; a loan is lender + borrower + terms; a bill is issuer + tenor; a
share is its issuer and class; a fund share is its fund; a good is its grade at its place; a contract is
its counterparties and terms. An internal identifier is never a display name, and **no invented grouping**
(a bucket, a band, a tier) ever stands in for the thing that was actually bought. The classes a representation
declares (REP) — a dwelling's condition band, a contract's start band — are RESOLUTION, set for play by measurement
(N8.5); they say how finely a thing is carried, never what it is.

### Law 10 — Mechanisms do not know what kind of thing they act on

A mechanism behaves the same for every party and instrument of the kinds it applies to. What differs
between kinds — an industry's recipe, a fund's mandate, an instrument's terms, a party's legal form — is
**declared data** about the kind, which the mechanism reads. Adding a product, an industry, an instrument
type or a fund type is a declaration, never a new special case in a mechanism.

### Law 11 — Heterogeneity is load-bearing, and nothing is decided at an average

Parties differ — in endowment, preference, history, information and position — and those differences are
what give a market two sides, make a distribution have tails, and let a shock transmit. Every decision is
taken **by a party from its own state**; an aggregate is always `Σ f(xᵢ)`, never `f(Σ xᵢ)`. There is no
representative agent anywhere a decision has a threshold. Identical twins may be carried together as one
**agent** of their count (REP): its decision is exactly each twin's decision, taken from its own state and applied
to every twin, which is a count and not an average. No state is ever averaged across parties.

### Law 12 — Causality runs forward, and nobody knows more than they could

A decision reads only what its decider could have observed **before** it decided: its own state, its own history,
public information as published (with its lags), and the terms of its own contracts. No party reads the model's
forecast of the future, another party's private state, or anything produced after the stage of the day in which it
decides (TIME.6). **There is no global expectation**; expectations are personal and they disagree.

### Law 13 — Nothing is immortal, and nothing vanishes

Every kind of party can end — a person, a household, a firm, a bank, a fund, an insurer, a clearing house,
a sovereign's access to markets — each by its own real trigger. Every ending has a **destination** for
every asset, liability, contract, employee and obligation. The one party that cannot fail in its own money
is a central bank in its own currency, and the reason is its balance sheet, not an exemption.

### Law 14 — Nothing is instant and nothing is free

Moving goods, people or information takes time and costs something; building takes time; settlement has a
date; a decision takes effect no earlier than the next opportunity to act on it. Transport, search, hiring,
switching, issuing, borrowing and failing each have a real cost borne by a named party.

### Law 15 — Chance is declared, seeded and dated

Randomness enters only through **declared processes of chance** (CHN.3): hazards — a death, an illness, an accident,
a catastrophe, a breakdown, a discovery, a meeting between a searcher and an opportunity — each with a declared
rate; a party's own taste on a choice, from a declared distribution; and the lots, samples and draws of which
parties an event concerns that the representation needs. Every rate and distribution is a primitive, and every draw
comes from a single seeded source, so that the same seed reproduces the same world. Every hazard occurrence is an
**event with a date and named subjects**; tastes, lots, samples and pairing draws are reproducible from the seed and
are not kept. **Outcomes are never drawn**: a default, a price, a vote, a merger or an unemployment rate is always a
consequence of what parties decided, never a draw.

### Law 16 — No outcome is imposed

Requirements state reasons; outcomes emerge. No price path, earnings path, growth rate, unemployment rate,
trade balance, election result or narrative is ever written into the world; the opening state is drawn from
declared distributions before any run (GEN) and never adjusted after seeing one, and no opening state is fitted
to an answer. The only legitimate way to change a primitive during a run is a decision by the party that
owns it.

### Law 17 — Looking never changes the world

Observing, measuring, auditing or displaying the world moves nothing: not a balance, not a price, not a
decision. An observer that acts does so as a party, through markets, with its own means, and is checked
like any other. **The audit never repairs**; it reports.

---

# PART II — HOW REQUIREMENTS ARE WRITTEN

### II.1 Requirement types

Every requirement in Parts A–M has an identifier, a type and one sentence of substance, with at most a few
lines of reason. Part L's chains are not requirements: each describes how requirements combine, and each is held
to its test (N4). There are seven types:

| Type          | Meaning                                                                                              |
| ------------- | ---------------------------------------------------------------------------------------------------- |
| **STATE**     | something that exists and is recorded: a party, a holding, a contract, a stock                        |
| **DECISION**  | a choice made by a named party, and **the inputs it is made from** — never the answer it must reach  |
| **PROCESS**   | a causal process that is not a choice: physics, biology, a contract executing, settlement, a hazard   |
| **INVARIANT** | an identity that must hold **exactly** at every day's close and is checked by the audit             |
| **MEASURE**   | something to observe and report, **never enforced**; a failure is a finding about a mechanism        |
| **FORBID**    | something that must be **absent**; the requirement no review of an existing build can produce        |
| **PRIMITIVE** | a class of declared number the system needs, with its Law 2 kind and its owner                       |

**An outcome written as a rule is a defect.** "Surplus banks lend and deficit banks borrow" is not a
requirement; it is what happens. The requirement is that each bank posts terms from its own position,
and who lends is the result. If a clause is none of the seven types, it does not belong here.

### II.2 Identifiers

Every system has a short code (`HH`, `BNK`, `FX` …). Requirements are numbered within their system:
`HH.12`. An identifier is **permanent**: it is never renumbered, never reused, and a retired requirement
keeps its number with the word *Retired* and the reason. New requirements take the next free number and
are placed where they belong in the text; the number records order of creation, not position.

### II.3 The shape of a system section

Every system section has these parts, in this order. **Primitives** and **Out of scope** are always present,
saying so where there are none; any other part with nothing to say is left out.

- **Purpose** — what the system is for, in a paragraph.
- **Depends on** — the systems that must exist before it can work.
- **State, Decisions, Processes** — what exists, who chooses what from what, and what happens by itself.
- **Invariants, Measures, Forbids** — what must hold, what to observe, what must be absent.
- **Primitives** — the declared numbers it needs.
- **Out of scope** — what it deliberately leaves out, with the reason (the larger items also in Appendix D).
- **Done when** — the acceptance test: the state, behaviour and evidence that show the system exists.

### II.4 Missing and out of scope are different answers

A requirement that is not yet met stays in the document and is **missing**. A thing the model
deliberately does not have is **out of scope**, stated with its reason. A requirement is never deleted or
softened to make a comparison look better.

### II.5 Contract violations and findings

Two kinds of wrongness exist and are treated differently:

- A **contract violation** is an impossible state — a one-sided flow, a negative count of physical units,
  a currency added to another, a decision reading the future, a multiplicity or identity changed by nothing. It
  **stops the run** at the site, with the requirement it breaks. It is never caught and continued.
- A **finding** is a state that is possible but wrong — an identity off by a named amount, a measure far
  from what real economies show. It is **reported** with its owner, size, date and requirement, and the
  run continues. The world is never repaired to hide it.

### II.6 Keeping the document true

- The document is updated in the **same change** as the thing it describes.
- It says what is **true**, never what a change found or did: it is not a diary.
- Where a requirement is met, the record of the build says where; this document does not.
- A rule that can be checked by a machine should be.

---

# PART A — FOUNDATIONS

The things every other system stands on: when things happen, who can act, what numbers mean, where
chance comes from, where everything is, and how a real-sized population is carried.

---

## A1. TIME — The calendar and the day

**Purpose.** One clock and one calendar that every event, obligation, price and number is placed on, and a
fixed causal order inside each day so that nothing reads what has not happened yet.

**Depends on:** nothing.

**State**

- **TIME.1 STATE** — The **day** is the atom of time. Nothing happens between two points inside a day
  except in the day's fixed order (TIME.6); every event, instruction, price and claim carries the day it
  belongs to.
- **TIME.2 STATE** — **One calendar**: an epoch, a mapping from days to dates, and per country a declared set of
  **business days** (weekends and holidays are ENDOWMENT). The epoch lies early enough that every opening contract's
  start date is a day; **day zero** is the day before the first (GEN.13). Markets and settlement run on business days;
  hazards, births, deaths and accruals run on every day.
- **TIME.3 STATE** — Every **periodicity** (a monthly payroll, a quarterly report, a semi-annual coupon, an
  annual tax return, a four-year term) is placed **by advancing a date**, never by counting days. A
  payment falls on the first business day on or after its date, by a declared business-day convention.
- **TIME.4 STATE** — Every accrual uses a **day count read from the calendar's dates**, by the convention
  the contract states. A convention computed from a count of periods is a second calendar.
- **TIME.5 STATE** — Every party has its own **decision schedule**: the days on which it reviews each kind
  of decision (a household shops on its own days, reviews its savings monthly; a firm sets prices weekly
  and plans investment quarterly; a desk quotes daily). The schedule is a PREFERENCE or a TECHNOLOGY of the
  party's kind, and a party can also be **woken** by an event that concerns it (a call, a job loss, a
  default, a price crossing its own trigger).

**Processes**

- **TIME.6 PROCESS** — **The order of a day.** Each business day runs these stages, each reading only what
  exists when it runs:
  1. **Open** — standing orders and offers that have lapsed expire; dated obligations falling due today
     are listed.
  2. **Resolve the past** — accruals post; calls and demands due today are paid or fail (TIME.7); dues unpaid from
     earlier days become arrears; fails from earlier settlement are recorded; recognised losses land on named holders;
     parties that cannot go on cease; estates distribute.
  3. **Nature and population** — the day's hazard events are drawn (CHN); people are born, die, fall ill,
     move, form and dissolve households; new firms are founded.
  4. **Real work** — production runs and finishes; services are delivered; shipments move and arrive;
     construction progresses; jobs start and end.
  5. **Decide** — every party scheduled or woken today forms its outlook (VAL) and posts what it wants:
     orders, quotes, offers, applications, bids, votes.
  6. **Form prices** — every market that meets today forms its prices and matches (MKT).
  7. **Settle** — every instruction due today, including today's dues and batches, settles or fails (SET), and a
     final pass lets a ring of payments that can settle together do so.
  8. **Fund** — the money market, the central bank's liquidity operations and its facilities meet over the reserve
     positions that settlement left, reading them because they exist when this stage runs, and their trades settle
     in this stage.
  9. **Value and judge** — positions are valued; accounts and ratios are read; covenants, margins and
     capital rules are tested; reports and ratings due today are published; calls and demands are issued.
  10. **Close** — public events are produced (OBS.3); the audit runs over what the day
     left behind.
- **TIME.7 PROCESS** — **Nothing is demanded and paid in the same stage.** A margin call, a covenant
  demand, a redemption request, a capital-call notice or a policy decision issued in stage 9 is due no
  earlier than the **next business day**; if unmet then, its consequence (a forced sale, a default, a
  close-out) happens in that day's stages. This one-day lag is the clock's, and it is what gives every
  feedback loop its speed.
- **TIME.8 PROCESS** — **Non-business days** run only what does not wait for markets and settlement:
  - stage 1's lapses, and the player's queued intents;
  - stage 2's accruals, and nothing else of stage 2;
  - stages 3 and 4: nature, population, and the physical processes that do not stop;
  - stage 5 for the decisions the day's meetings need (buyers' spending, the posted prices of sellers who open that
    day, generators' offers) and the occasions of those decisions; any other occasion waits for the first day its
    decision is taken;
  - stage 6 for the retail and service markets whose sellers open that day and for the electricity market (ENE.8);
  - stage 10: public events and the audit.

  No financial market meets and nothing settles. What is bought on a non-business day is paid in banknotes, which
  change hands at the purchase, or by card; card payments and the electricity market's trades settle on the next
  business day, as commitments until then (SET.2).

**Invariants**

- **TIME.9 INVARIANT** — Every recorded event, instruction and price has a day; there is no default day and
  no "unset" date.
- **TIME.10 INVARIANT** — No stage reads a fact written by a later stage of the same day or by a later day.

**Forbids**

- **TIME.11 FORBID** — No second clock: no system keeps its own count of periods or dates.
- **TIME.12 FORBID** — No periodicity finer than a day, and none placed by a count of days where a date is
  meant.

**Primitives**

- **TIME.13 PRIMITIVE** — The epoch, business-day calendars per country (ENDOWMENT); business-day and
  day-count conventions per contract type (POLICY of the market that declares them); decision schedules per
  kind (PREFERENCE/TECHNOLOGY).

**Out of scope**

- Time inside a day: the day's stages are its only order (TIME.6).

**Done when**

- Every other system can place a dated obligation and have it fall due on the right business day; a
  reordering of stages that lets a stage read later output is refused.

---

## A2. PTY — Parties and identity

**Purpose.** Who can own, owe, decide and act. Every holder, payer, decider and counterparty in the world
is a party with a permanent identity.

**Depends on:** TIME.

**State**

- **PTY.1 STATE** — A **party** is anything that can hold, owe, decide or be paid: a **person**, a
  **household**, a **firm**, a **bank**, a **fund**, an **insurer**, a **pension scheme**, a **clearing
  house**, a **treasury**, a **central bank**, a **public agency**, a **parliament**, a **political party**,
  an **estate**. Each has an identity that is never reused.
- **PTY.2 STATE** — **The world is real-sized**: hundreds of millions of people and millions of small firms
  across its countries, or one declared share of them in a small world (REP.40). Every party is either an
  **individual** or an **agent** of its kind's population (REP). Institutions, issuers and every firm or household
  within its rank at the opening are individuals; the rest of the households and small firms, the player's own among
  them, are agents. An agent is a named party whose multiplicity is the exact count of identical real parties it is.
- **PTY.3 STATE** — A **person** has an age, a household, a region of residence, skills, a health state and a
  labour-market state. A **household** is one or more persons who share a budget and a dwelling; it is the
  unit that owns, consumes, saves and borrows. Legal ownership sits with the household; labour, age and
  mortality with the person. A household holds its persons, each with its role in it (REP.26).
- **PTY.4 STATE** — Every party has a **legal form**, and the legal form is declared data (Law 10): what it
  may hold, whether it is a party separate from its owners, whether its owners have limited liability,
  whether it may take deposits, how it can end, and who its owners are.
- **PTY.5 STATE** — Every party has a **site** on the map (GEO), from which its region and country are read;
  a party with several establishments has a site for each. An agent has a **zone** among its attributes, and its
  site is the tile of what it holds there, drawn when something depends on it (REP.24).
- **PTY.6 STATE** — Every party has a **home currency** — its country's — and keeps its books in it.
- **PTY.7 STATE** — **Ownership and control are relations between parties**, recorded as holdings of the
  owned party's equity (REG), never as attributes. A **group** is a parent and the subsidiaries it controls
  through those holdings.
- **PTY.8 STATE** — A party's **private state** (positions, limits, intentions, outlooks) is its own; its
  **public state** is what it has published or what is visible by law (OBS).

**Processes**

- **PTY.9 PROCESS** — A party **begins** by a named event with a cause (a birth, a founding, a
  registration, a spin-off) and **ends** by a named event with a cause (a death, a dissolution, an
  insolvency, a merger, a resolution). Every ending opens an **estate** or names a **successor** (L3).

**Invariants**

- **PTY.10 INVARIANT** — Every party referenced by any holding, contract, instruction or event exists, or
  has an estate or successor that does; a kept record older than its horizon may name a party that reads as ended
  (SET.13).
- **PTY.11 INVARIANT** — Every person belongs to exactly one household; every household has at least one
  living member or is an estate.

**Measures**

- **PTY.12** — _Retired_: comparing the same world across a ladder of resolutions needs a second run, and the world
  runs once. Its representation is judged by its own macro results against real economies' (N3, N4).

**Forbids**

- **PTY.13 FORBID** — No party without an identity, no identity reused, no party that exists only to absorb
  a residual, and no party that cannot end (Law 13) except a central bank in its own currency.
- **PTY.14 FORBID** — No weight, share or scale factor applied to a party's decisions or holdings, other than
  an agent's multiplicity (REP). A party holds what it holds.

**Primitives**

- **PTY.15 PRIMITIVE** — The opening population and its parties (ENDOWMENT); legal forms and what each
  permits (POLICY of the country that defines them); the representation, its factor and zones (RESOLUTION, REP).

**Out of scope**

- Informal and illegal activity; the internal organisation of a household beyond a shared budget.

**Done when**

- Every party in the world has a site, a legal form, a home currency and an owner or members; ending any
  party leaves no reference dangling; the full population is generated from one seed.

---

## A3. NUM — Numbers, units and primitives

**Purpose.** What a number means, and where every number the world is given comes from.

**Depends on:** TIME, PTY.

**State**

- **NUM.1 STATE** — Every quantity carries its **unit**: money of a named currency, face, shares, fund
  units, contracts, physical units of a named good, hours, persons, dwellings, square metres, kilometres.
  The only route from a quantity to a value is **quantity × price**.
- **NUM.2 STATE** — A figure in money says **whose money**: the owner's home currency, a currency named
  beside it, the reporting numéraire, or another named party's money. The four are different numbers.
- **NUM.3 STATE** — **The primitive register.** Every declared number (Law 2) is registered with its value,
  unit, period, kind, owner, source (measured from data, estimated, assumed, placeholder) and, for a SHAPE,
  the mechanism whose absence it stands in for. A system reads declared numbers only from the register.
- **NUM.4 STATE** — **Differences are primitives too.** Where parties of one kind differ in a preference or a
  technology (patience, risk aversion, tastes, skill, memory), the declared primitive is the **distribution** across the
  kind, carried as a **finite set of types** with the share of each — the distribution's declared discretisation — from
  which a party's type is drawn once, at its creation, from the seeded source (CHN). The number of types is a RESOLUTION
  (N8.5): every number discretises the same distribution. A modest number of types reproduces much of real wealth
  inequality and spending behaviour, while the top of the wealth distribution comes from returns and business ownership.
  What varies within a type from one occasion to the next — a taste for one seller over another today — is a draw from
  the type's declared taste distribution (REP.22), not a new type.

**Invariants**

- **NUM.5 INVARIANT** — No arithmetic combines two currencies, or two units, except an exchange at a stated
  price or a declared conversion (a recipe, a technology).
- **NUM.6 INVARIANT** — No number is invalid (not-a-number or infinite) anywhere in the world's state.

**Measures**

- **NUM.7 MEASURE** — The **placeholder count**, over the life of the build, only falls, except by the
  placeholders a stage introduces for later systems, each naming what retires it; the standing SHAPEs are listed
  with their reasons.

**Forbids**

- **NUM.8 FORBID** — No number shapes behaviour without being in the register; no default value standing
  in for a missing one; no absent value read as zero.

**Primitives**

- **NUM.9 PRIMITIVE** — None of its own: it says how every other system's primitives are declared and registered.

**Out of scope**

- Nothing.

**Done when**

- Every behaviour-shaping number in the world can be listed with its kind, owner and source, and the
  SHAPE count is reported.

---

## A4. CHN — Chance

**Purpose.** The one source of randomness, and the rule that every random thing is a real, dated event.

**Depends on:** TIME, PTY, NUM.

**State**

- **CHN.1 STATE** — **One seed** per run. Every random draw comes from a named **stream** derived from that
  seed, one per hazard process and purpose, so that adding a new process never changes the draws of an
  existing one, and the same seed reproduces the same world on the same build and device (N5).
- **CHN.2 STATE** — A **hazard process** is declared with: what it acts on (a person, a vehicle, a plant, a
  tile, a policy, a search), its **rate** as a function of declared state (age, health, wear, exposure,
  effort), what an occurrence does, and its source. The rate is a TECHNOLOGY primitive (a life table, a
  failure curve, a catastrophe frequency, a discovery rate) and may be imported from data.

**Processes**

- **CHN.3 PROCESS** — The processes this world has: **mortality** and **illness** of persons; **conception** given a
  household's decision to have a child; **accidents** and **damage** to dwellings, plant, vehicles and cargo, and
  **harm to third parties** by a party's vehicle, premises or work, which is what liability cover answers (INS.1);
  **natural catastrophes** on tiles (flood, storm, earthquake, drought, crop failure), which can hit many parties at
  once; **equipment failure**; **discovery** in research and in imitation (TEC.5, TEC.6); **meetings** in search (a
  job seeker and a vacancy, a buyer and a dwelling for sale, two adults forming a household); **weather** — wind,
  sunshine, temperature and rainfall per region per day, drawn from each region's declared climate — which drives
  renewable power, heating and cooling demand, crop yields and river levels; **heterogeneity at birth** (drawing a new
  party's type from its declared set of types); **occasions** — the reviews and needs on which an agent reconsiders a
  lumpy decision (REP.21); and **tastes** — a party's idiosyncratic taste for each alternative on a choice occasion
  (REP.22); **pairing draws** — which parties on a line an event concerns (REP.23); **samples** — the records a
  statistics agency's survey or a polling firm's poll reads (STA.2, POL.4); the **opening draws** of the world and its
  map (GEN.3, GEO.10); **schedule phases** — where in its period a new party's decision schedule falls (TIME.5); and
  **lots** — any order or choice that matters and that no rule fixes (CHN.6): the order in which buyers reach a
  seller whose capacity runs out and orders reach a book (MKT.4), and the choice among applicants or bidders a rule
  leaves equal.
- **CHN.4 PROCESS** — Each occurrence is an **event** with a day, named subjects and a size, recorded before
  any party reacts to it.

**Measures**

- **CHN.7 MEASURE** — The realised frequency of each hazard matches its declared rate over the run; a
  catastrophe's losses are clustered in place and time, which is the point of having one.

**Forbids**

- **CHN.5 FORBID** — **No drawn outcome.** No default, price, vote, merger, bank run, hiring total or
  growth rate is ever drawn. Chance decides what happens to a party's circumstances; the party and the
  markets decide what follows.
- **CHN.6 FORBID** — No unseeded randomness and no dependence on iteration order of anything unordered: where an
  order matters and no rule fixes it, it is a lot (CHN.3).

**Primitives**

- **CHN.8 PRIMITIVE** — The seed, chosen per run; every hazard rate and taste distribution is declared by the system
  that owns it (CHN.2).

**Out of scope**

- Nothing.

**Done when**

- Every draw comes from a named stream derived from the run's seed (CHN.1, CHN.6), and no stream's draws depend on
  which other processes exist; every hazard occurrence can be listed as a dated event.

---

## A5. GEO — The physical world

**Purpose.** The map: where parties, plant, dwellings, deposits and routes are; what separates them; what
the ground holds; and what nature can do to a place. Geography creates distance, barriers, resources and
exposure; it never writes an economic outcome.

**Depends on:** TIME, PTY, NUM, CHN.

**State**

- **GEO.1 STATE** — The world is a finite grid of **tiles** covering a **closed surface**: it wraps east to west and
  north to south, so it has no edge and no pole, and whatever travels far enough in one direction comes back to where
  it started. Each tile has a stable identity, a coordinate on the surface, a surface (land or water), an elevation
  and a terrain class. Every tile has the same eight neighbours around it; adjacency is read from the grid, one way,
  for everybody.
- **GEO.2 STATE** — **Distance** is a physical length derived from coordinates on the closed surface: between two
  places it is the shortest of the ways around; a path's length is the sum of its legs. Grid steps and labels are
  not distance.
- **GEO.3 STATE** — A **country** is a jurisdiction over a set of tiles, with a currency, laws and a state (Part J). A
  **region** is a set of tiles within one country and is where local markets (labour, housing, services, retail) meet.
  A country's land and its number of regions follow its population share (GEN.14): the regions, a world constant in
  number, are allotted by largest remainder with at least three per country, and each country's land is its share of
  the map, so regions are of like size. A **zone** is a declared set of neighbouring tiles within one region
  (RESOLUTION), the place at which agents are carried (REP.24). A **site** is the exact tile on which a
  party, plant, dwelling, warehouse, port or piece of infrastructure stands; country, region and zone are read through
  the site.
- **GEO.4 STATE** — **Infrastructure** — roads, rail, bridges, tunnels, ports, pipelines, power lines — is
  owned capital (CAP) with a site or a path, a capacity shared by everything using it in a day, a life, a
  maintenance need and a condition.
- **GEO.5 STATE** — **Land** is a tile's area, owned by a named party as a holding, with what stands on it.
  A dwelling or a plant occupies land.
- **GEO.6 STATE** — A tile may hold a **deposit**: a named resource at a grade, in a declared finite
  quantity or declared unbounded. The right to extract it is a holding like any other (GDS).
- **GEO.7 STATE** — Every tile has an **exposure** to each natural hazard (CHN), from its terrain, elevation,
  water and climate class: what a flood, storm or drought there does to what stands on it.

**Processes**

- **GEO.8 PROCESS** — A catastrophe on a tile damages or destroys what stands there — dwellings, plant,
  stock, infrastructure, crops — as real losses of units at their owners, and can close routes (FRT).
- **GEO.9 PROCESS** — A finite deposit **depletes** by what is extracted and never refills; where its nature
  is that the richest part goes first, its grade falls with what has been taken.
- **GEO.10 PROCESS** — The opening map is **generated** from the seed by a recorded procedure with declared
  SHAPE parameters (sea level, terrain roughness); a generated world that fails a declared
  construction condition is rejected and recorded, never nudged toward a desired economy.

**Invariants**

- **GEO.11 INVARIANT** — Every site lies in its declared country and region; every inhabited region has land.
- **GEO.12 INVARIANT** — For every finite deposit, extracted plus remaining equals its opening quantity.
- **GEO.13 INVARIANT** — A network segment carries no more in a day than its capacity.

**Forbids**

- **GEO.14 FORBID** — No decorative map: nothing keeps a second map for display.
- **GEO.15 FORBID** — Ownership never moves a site: buying a factory changes title, not location.
- **GEO.16 FORBID** — No resource available everywhere: what a place can extract is a fact of its ground.
- **GEO.17 FORBID** — No border, deposit or coastline placed to manufacture a later result.

**Primitives**

- **GEO.18 PRIMITIVE** — The world's size and its **latitude cycle** (ENDOWMENT, chosen by the owner): with no pole,
  the latitude a row's climate is read at runs evenly from a warm belt to a cool belt over half the world's height
  and back over the other half, so every latitude between them is present twice, and it changes along the way
  faster than on the Earth; tile size (RESOLUTION: the same map subdivided); terrain generation parameters (SHAPE,
  declared as such, with the reason no mechanism replaces them); deposits and opening infrastructure
  (ENDOWMENT); hazard exposure by terrain (TECHNOLOGY).

**Out of scope**

- A changing climate (Appendix D); slow change of terrain such as erosion.

**Done when**

- Parties, plant and dwellings stand on sites; distances between any two sites are paths over real terrain;
  deposits deplete exactly; a catastrophe destroys named units at named owners.

---

## A6. REP — How populations are represented

**Purpose.** How a world of hundreds of millions of people and millions of small firms is carried exactly
enough to be true and compactly enough to run on a phone. This section says what any representation must make
true — what is exact, what is approximated, and how the approximation is measured — and chooses no method.

1. Every household and small firm is an **agent** with its own state, its own persons and its own contracts. No
   agent is ever split, joined or averaged with another.
2. An agent may stand for several **identical twins** — its **multiplicity** — which share one state and one fate.
   The multiplicity is a count fixed when the agent begins.
3. The world is held in one of **two representations**, chosen per build (decision 44): the full population as
   agents of one multiplicity, or a smaller world whose every agent is one real party.
4. Chance, choice and the **occasions** to act reach agents, each by its own state.
5. A relationship between many parties is **one record**, and who in it is paired with whom is drawn only when
   something depends on it. That rests on **one declared assumption**: inside a line, nothing matters beyond what
   is counted.
6. **Looking changes nothing.**

The work of a day follows the number of agents something happens to, not the number of people.

**Depends on:** PTY, NUM, CHN, GEO.

**State**

- **REP.1 STATE** — An **agent** is a named household or small firm held in its kind's population. It holds its
  own attributes (REP.41), its own positions (REP.20), its persons (REP.26), its contracts and holdings.
  - Its **multiplicity** is the count of real parties it stands for: its **twins**, identical in every attribute,
    position, person and contract, each holding its share of everything the agent holds (REP.9).
  - Everything a twin does, every twin does; everything that happens to one happens to all. So an agent's state
    is exactly each twin's, and nothing is ever averaged.
  - Agents of one kind share one multiplicity, except the player's own, which is one (OBS.4), and the agent the
    player's household was drawn from, which keeps one twin fewer.
  - At the opening, as the player's twin is taken from its donor, each agent on the other side of a line the player
    holds, one drawn by its contracts on the line where several are, gives one twin too, seated the same way, so every
    such line holds a party of the player's multiplicity and one of the donor's on its other side.
  - An agent that ends leaves one estate for each twin, alike: one estate party standing for them all, of the
    agent's multiplicity, whose every amount is a whole share for each twin (REP.9) and whose waterfall runs on one
    twin's estate.
- **REP.2 STATE** — An **individual** is a party never held in a population:
  - every institution and issuer, and every party with a public instrument (a listed share, a bond, a rating);
  - the **top-ranked** parties of each kind by size (employees, turnover, net worth) at the opening, down to a
    declared **rank**. The largest parties move aggregates on their own. A rank, unlike a size in money, keeps the
    number of individuals bounded and does not drift with the price level.

  An individual's multiplicity is one.
- **REP.40 STATE** — **The representation.** A build holds its world in one of two representations, with one
  **factor** _k_ (RESOLUTION):
  - **Twins.** The countries hold the whole population the setup gives (GEN.14). Its households and small firms
    are drawn as agents of multiplicity _k_, one agent for every _k_ real ones; individuals are drawn as they are.
  - **Small world.** The countries hold one _k_-th of the population the setup gives, and every agent is one real
    party of multiplicity one. Everything the opening derives from the population — the employed, the firms, the
    lines — follows from the smaller population, and the individuals' rank is one _k_-th of the declared rank.

  Both draw the same number of agents for one _k_, by the same mechanisms; they differ only in what one agent
  stands for. The factor is set by measuring the budget (N8.5).
- **REP.41 STATE** — **An agent's attributes** are what it holds exactly and is never averaged on: for a household
  its region and zone, its bank, its tenure, its credit-record stage, its preference type (NUM.4) and heuristic
  stance (VAL.7), and every **clock a rule reads** (months unemployed toward a benefit's end, months in arrears
  toward default); for a small firm its zone, its ways, legal form, productivity and management types, its
  **posted price and wage offers** (REP.34), its headcount by occupation family and its plant. Each kind declares
  its attributes (Law 10).
- **REP.20 STATE** — **Positions** are an agent's continuous amounts. A position is **read** where a fact already
  lives — cash from its accounts, wealth from its holdings at their marks — and only a record that lives nowhere
  else (recent income, accrued rights, own outlooks) is held by the agent, as an exact total over its twins.
  Examples: recent income, debt service, the mark of illiquid wealth, the risky share of savings, accrued pension
  rights and contribution records, inventory, cumulative output, and its outlooks of its own variables (VAL.23).
- **REP.3 STATE** — A **line** is one record of identical contracts, with the same terms, between the parties on
  its two sides.
  - Each side is one or more named parties, individuals or agents, each with an exact **count** of contracts.
  - A line with one party per side is an ordinary contract line. A line with many is how a relationship between many
    parties is recorded: employment in one occupation family and skill at one wage offer and start band in one region;
    tenancies of one class at one rent in one zone; deposits of one kind at one bank; invoices on one market's terms
    that fall due together (TCR.1); kinship between parents and the households their children formed.
  - An agent's count on a line is its multiplicity times the contracts each twin holds there.
  - **Terms belong to the line**, and nothing but the parties' own decisions and the contract's own events changes
    them. An amount that differs by holder, such as a deposit's balance or an accrued pension, is the holder's own.
- **REP.34 STATE** — **Price points.** Posted prices, wage offers and lenders' rates lie on the poster's **price
  points**: the round and conventional numbers of its trade. The set of points is a convention of the trade, a
  primitive like a market's tick size; which point is posted is the poster's own decision (FRM.5, LAB.4, BNK.4), and
  how often prices sit on each point, and how long, are outcomes.
  - A posted price is never averaged: every price a buyer meets is one a seller chose.

**Decisions**

- **REP.5 DECISION** — **An agent decides as itself.** Every decision is taken from the agent's own state, and an
  agent of multiplicity _k_ takes it once for its twins, which is exactly each twin's decision.
  - A **continuous** decision is taken on its schedule. Examples: this week's spending, the level of a buffer,
    where savings go. Hours and wages are contract terms, changed only on occasions.
  - A **lumpy** decision is taken only on an **occasion** (REP.21). Examples: to quit, move, buy a dwelling or a
    vehicle, borrow, found or close a business, switch bank, reconsider a heuristic, **change a posted price or
    wage**.
- **REP.38 DECISION** — **Attention.** How often an agent reviews each kind of lumpy decision is its own choice, a
  continuous decision, from what reviewing and changing that decision costs it (REP.18) and what is at stake: the
  loss it expects from leaving the decision as it stands, which a surprise raises (REP.35). Attention is therefore
  an outcome, never a review rate.

**Processes**

- **REP.21 PROCESS** — **Occasions reach agents.** Every lumpy decision is triggered by an occasion:
  - a **review**, at the agent's **attention** rate (REP.38). Reviewing and changing a decision costs its decider
    something real (Law 14) — time, a fee, a menu cost paid to a named party — and the cost is the primitive;
  - a **need**: a breakdown, a birth, a notice to leave, a contract ending;
  - a **meeting**: an offer, a listing, a vacancy, an opportunity (CHN);
  - a **notice** addressed to it: a layoff, a margin call, a demand.

  An occasion reaches an agent with the chance it would reach each of its twins, and reaches all of them together.
- **REP.35 PROCESS** — **A surprise raises attention.** A surprise (VAL.4) raises what is at stake in the decisions
  it bears on, and so the rate at which agents choose to review them (REP.38). A shock therefore reaches agents over
  days as their reviews arrive: fast when it is large, and never all at once by construction.
- **REP.7 PROCESS** — **Chance acts on persons and agents.** A hazard acts on each person or agent at its own
  rate, read from its own state on the day, from the agent's own stream for that process (CHN). A hit on a person
  of an agent of multiplicity _k_ is a hit on that person in every twin.
  - The **draw scheme** is declared: the day of an agent's next hit is drawn ahead, and drawn again when anything
    its rate reads changes or on the next day its rate may change. Which of its persons the hit reaches is drawn
    on that day, each as its own rate gives, at least one. The world is exactly reproducible under the scheme.
  - An agent nothing hits is not visited, and no result depends on the order anything else runs in.
- **REP.22 PROCESS** — **A choice among alternatives.** When an agent chooses among sellers, vacancies, dwellings,
  lenders or heuristics, it has its own **taste** for each alternative on that occasion. For a job offer, that
  taste is the match's quality. It is drawn from its type's declared distribution (random utility), and the agent
  chooses the alternative its tastes and state make best; its twins choose the same.
  - Because tastes are drawn, identical agents spread over alternatives as independent choosers would, and every
    discrete choice is a **smooth** function of the shared state, not a kink.
  - An alternative with limited capacity serves those who reach it in an order drawn by lot, and the rest choose
    again.
- **REP.23 PROCESS** — **Pairings are drawn when they matter.** Within a line, which party on one side is paired
  with which on the other is **not recorded**. The pairing is taken to be a uniform matching consistent with the
  counts: nothing about who is paired with whom matters beyond the line's terms and the counts. So **whatever a
  counterparty chose on is in the line's terms**: an employer that selects by skill hires onto a line whose terms
  include the skill it hired.
  - When an event concerns some contracts on one side, the parties on the other side it reaches are drawn from
    the counts at that moment. Examples: a firm on one side closes, a worker on the other side quits, a flood
    reaches one tile, a parent dies and its heirs must be found.
  - Under that uniform matching, drawing at the moment is exactly what a pairing recorded from the start would have
    given (the principle of deferred decisions). Nothing drawn is contradicted afterwards.
  - A party drawn on an agent's side is drawn whole: its twins' contracts leave, or lose, together (REP.1). Contracts
    leaving are drawn like with like: among the other side's parties of the leaving party's multiplicity while they
    hold what is left, and otherwise among those whose multiplicity fits in what is left to draw.
  - A line whose payments settle through the issuer of their money, each against the line (MON.5), draws its losers
    until at least the failed count is reached, the last drawn party's twins losing together. That issuer is owed
    the failed payers' dues and owes the losers theirs, each recorded as failed; what the losers drawn past the
    failed count were owed stays with it, owed to them as failed dues, so no amount has one side (Law 5).
- **REP.24 PROCESS** — **Where agents are.**
  - A household's zone is an attribute. Its dwelling, plant and vehicles are held by zone and **class** (kind,
    size, quality band, condition band). Wear, damage and repair move units between condition classes.
  - The tile each unit stands on is drawn when something depends on it (REP.23) from the zone's stock of that
    class counted per tile.
  - Distance for agents is measured between zones. The zone is the spatial resolution at which they are carried.
- **REP.25 PROCESS** — **Age.** Every person holds its **birth date**. Its age on any day, and the day it reaches a
  statutory age or an entitlement, are read from it exactly.
- **REP.26 PROCESS** — **Persons are held in their household.** A household holds each of its persons, with its
  role (head, partner, another adult, a child), its birth date, sex, health, education and its own contracts, such
  as employment. A person's event changes that person: a death, an illness, a job lost or taken, a new skill
  level, a child reaching adulthood.
- **REP.9 PROCESS** — **Exact totals in whole units.** An agent's money is a total in whole smallest units
  (MON.16), and every holding counted in whole units — shares, face, fund units — is held the same way. A rule's
  amount for an agent is its amount for one twin, rounded by its convention, times the multiplicity, so each twin's
  share is always whole. Indivisible physical units are held per twin and never shared.
- **REP.12 PROCESS** — **Only what is active is touched.** An agent is visited on a day only if it has an
  occasion, a hazard hit, a scheduled payment or a flow that day. Accruals post on the dates that need them.

**Invariants**

- **REP.13 INVARIANT** — Every population's agents' multiplicities sum to its population, exactly, every day:
  households and small firms, and persons counted by their households' multiplicities. Every real household and
  small firm is a twin of exactly one agent or is one individual.
- **REP.14 INVARIANT** — No event, pairing draw or ending changes any total of money or units, and every holding
  and count an agent holds is a whole multiple of its multiplicity.
- **REP.31 INVARIANT** — On every line, the two sides hold equal counts. An agent's count on a line is its
  multiplicity times the contracts its twin holds, and never more than its persons in the role that holds them.

**Measures**

- **REP.15 MEASURE** — **What the representation is, reported every day**, per kind: its representation and factor,
  its agents, individuals and lines, the persons they hold, the agents something happened to that day and the
  events per agent, and — in twins — the agents whose size would rank them individuals but that stand for _k_
  identical twins. These are what every distributional number the world shows is drawn from.

**Forbids**

- **REP.16 FORBID** — The representation must never:
  - average an attribute, a position, a posted price, a line's terms or a person;
  - split an agent, join two, or let a twin differ from another;
  - create or destroy a unit;
  - apply an event to an agent that did not reach it, or to some of an agent's twins and not the others;
  - let an agent read an experience it did not have;
  - record a pairing, or contradict one that was drawn;
  - observe with a world stream, or act on the observer's.
- **REP.17 FORBID** — No multiplicity that is a share, a scale factor or a probability. A multiplicity is a count
  of real parties, one or the factor, fixed when its agent begins and never changed, but the donor's, which gives
  the player's twin at the opening (REP.1). An agent's estate takes the agent's multiplicity.

**Primitives**

- **REP.18 PRIMITIVE** — RESOLUTION:
  - the representation's factor (REP.40), and which representation a build holds (decision 44);
  - each kind's attribute classes;
  - zones;
  - the individuals' ranks;
  - the declared payment order and the draw scheme (REP.7).

  PREFERENCE: taste distributions per type. TECHNOLOGY: what reviewing and changing each kind of decision
  costs. POLICY of each trade: its price points.

**Out of scope**

- Nothing: what the representation gives up is stated with its reasons in Appendix E, decisions 14 and 44.

**Retired**

- **REP.4** — _Retired_: cell budgets, tolerances and steps belonged to cells joining members within a tolerance;
  agents are never joined. The factor (REP.40) is the representation's one valve.
- **REP.6** — _Retired_: splitting a member out when something happens to it alone, or when it is watched,
  changed the world by looking at it. Replaced by occasions (REP.21) and drawn pairings (REP.23).
- **REP.8** — _Retired_: landing joined parts of cells into cells of the same key within a tolerance, and pooled a
  flow reaching some of a cell's members into its total. An agent is never split, so nothing lands and no flow is
  pooled across parties (decision 44).
- **REP.10** — _Retired_: tails were carried finely by narrowing tolerances; every agent is exact.
- **REP.11** — _Retired_: a shadow sample either diverges by its own chance or double-counts.
- **REP.19** — _Retired_: the key was what every member of a cell shared exactly. Replaced by the agent's own
  attributes (REP.41).
- **REP.27** — _Retired_: outlooks held by experience group let a member read experience it did not have.
  Replaced by public outlooks computed once per method, and own outlooks as positions (VAL.23).
- **REP.28** — _Retired_: tolerance control widened and narrowed cells' tolerances; there are none.
- **REP.29** — _Retired_: promotion and demotion moved parties between cells and individuals by rank. The
  individuals are those ranked at the opening (REP.2); an agent stays an agent, and a small firm sold whole is
  named as the agent it is (decision 35).
- **REP.30** — _Retired_: tracers followed members of cells through their splits. Every person is held in its
  household (REP.26), and the observer reads agents themselves (OBS.8).
- **REP.32** — _Retired_: profiles counted the attributes members of a cell did not share. Persons are held in
  their households (REP.26).
- **REP.33** — _Retired_: which way an attribute was carried followed from cells' four ways; an agent holds
  everything as its own.
- **REP.36** — _Retired_: exactness at a landing; nothing lands.
- **REP.37** — _Retired_: choice groups let cells within a tolerance meet a market as one; every agent meets it as
  itself.
- **REP.39** — _Retired_: the decision gap of a landing; nothing lands.

**Done when**

- A world of hundreds of millions of people and millions of small firms carries its population exactly in either
  representation, within the budget, on the target device.
- A firm closing releases workers drawn from its lines, and a flood destroys dwellings of owners drawn from the
  zone's counts.
- Identical agents spread across sellers, vacancies and occasions by their own tastes and chances.
- No attribute, posted price or contract term was ever averaged, and every agent's holdings are whole multiples of
  its multiplicity.
- The representation and its counts are reported every day.

---

## A7. GEN — The opening world

**Purpose.** How a run's first day is made. The opening world is a **snapshot of the present**: the state a
statistician would find in a real-shaped economy on one date — its parties, holdings and contracts, and the single
latest value of everything observed on that date — derived from a short setup, drawn from declared distributions, made
consistent in its accounts and nothing else, given its first decisions by its own parties on day zero, and settled by
the world's own mechanisms for a short period before play. It holds the present, never a past: no series of prices or
statistics is drawn, and the settling period is the world's only history.

**Depends on:** TIME, PTY, NUM, CHN, GEO, REP, VAL, and every system whose state it opens or whose decisions it
takes on day zero.

**State**

- **GEN.1 STATE** — The **opening world** is the state on a run's first day. It is an ENDOWMENT made by the
  generator below from the run's seed and its setup (GEN.14), and no country in it copies a real one: a country may
  bear a real country's name, which names its institutions and currency, but its economy is always generated.
- **GEN.14 STATE** — **The setup.** Every run starts from a setup, fixed before it and recorded with it:
  - **world constants**, the same for every run of a build: the total population, the map's size (GEO.18), three
    countries, the total number of regions and the settling length (GEN.6);
  - **the representation** with its factor (REP.40), which the setup does not choose: it is the build's RESOLUTION
    (REP.18), set by measuring the budget and recorded with the run and its saves, and under a small world the
    countries hold one factor-th of the total population;
  - **the population split** among the three countries, each between 10% and 70% of the total, so none is too small
    to have its markets or so large that it swamps the others; land and regions follow it (GEO.3);
  - **per country, six choices of three levels each**: development (developed, emerging, developing); public debt
    (low, medium, high); private debt, of households and firms (low, medium, high); risk appetite (cautious,
    balanced, bold); inequality (low, medium, high); openness (low, medium, high);
  - **per country, a name**: a real country's, which names its institutions and currency and pre-fills its choices
    with that country's levels, or a generated one.

  Each choice has a default and may instead be drawn from the seed. The setup also states **the player's**: the
  country the player lives in, and whether the rules decide for the player's household on a day the player has
  queued nothing (OBS.4).
- **GEN.2 STATE** — **Declared distributions.** For each country, the generator derives distributions from its
  derived values (GEN.15), their shapes from published work and their parameters from those values, each registered
  (NUM.3) with its sources:
  - population by age, household composition and region, from life tables and censuses, with education and
    skills by age, region and occupation family;
  - incomes (a log-normal body with a Pareto top), drawn as the contracts and holdings that pay them (Law 4), and
    wealth, by preference type;
  - firms by industry and size (with a power-law top), their plant, stocks, debts and owners, the ways they know
    and the patents they hold;
  - the housing stock, its tenure and its mortgages;
  - banks' balance sheets; the sovereign's debt and its maturities; holdings of funds, pensions and insurance;
  - the start dates of each kind of contract (its vintages), from which GEN.5 computes what each has paid;
  - the **present values** of the snapshot date, derived from the derived values by declared mappings and the
    steady-path convention (GEN.5), one each: each market's latest price and each currency
    pair's fixing; each reference rate and index level; each administered rate in force (a policy or facility rate);
    each published statistic's latest release; each rated issuer's rating; households' expectations of the variables
    they forecast, by age and income, from survey cross-sections, with their dispersion.
- **GEN.5 STATE** — **The snapshot.** The opening is one date's state, and each fact in it is drawn once, on one
  side; every other side is derived from it, so counterparties agree by construction:
  - **Present values** (GEN.2) are the opening's prints, fixings, published statistics and ratings (Law 3), one
    each and dated the snapshot date; a series that has one value is a series of one.
  - **Contracts** are drawn with their terms and start dates. Whatever a contract's balance or current amount
    depends on from before the snapshot — an amortised principal, accrued interest, a floating coupon's current
    fixing, an indexed amount's accrued ratio, a revalued pension slice — follows the **steady-path convention**:
    it is computed as if the snapshot's present values had held since the contract's start. The convention is one,
    declared, and the same for every contract, so contracts on one reference agree; it stores no series and no
    party reads it as an observation.
  - **Carrying values** are the snapshot's prints, or a valuer's published method applied to them (MKT.20), so every
    position has one on the first day.
  - **Each firm's latest filed accounts** are derived from its drawn books and lines (ACC), as its last report.
  - **Outlooks** start from the snapshot (VAL.10): each at the latest present value it reads, households' at their
    surveyed expectations, each width at the dispersion observed; records of heuristics' performance and of
    surprises start empty.

**Processes**

- **GEN.3 PROCESS** — **Drawing.** Parties, holdings, contracts and present values are drawn from the distributions
  by a recorded procedure, from the run's seed (CHN).
- **GEN.4 PROCESS** — **Balancing.** A declared procedure makes the drawn world consistent in its accounts, and in
  nothing else:
  - every liability has a holder and every holding an issuer, and holdings sum to what was issued;
  - deposits equal banks' liabilities, and reserves the central bank's;
  - every loan has a lender whose balance sheet carries it, and every party's books close at the snapshot's
    carrying values;
  - every contract's payments fall on dates the calendar places.

  It changes drawn amounts only as far as the accounts need, reports each change, never changes a contract's terms
  or start date, so what the steady-path convention computes from them stands, and never solves for an economic
  equilibrium, a price or a behaviour.
- **GEN.6 PROCESS** — **Settling.** Before play, the world runs for a **settling period** by its own mechanisms.
  Its length is the owner's setting, adjustable, one simulated year by default. Its history is the world's only
  history: it is kept as the parties' real experience, their outlooks learn from it, and play begins on the day
  after it.
- **GEN.15 PROCESS** — **Derivation.** From the setup, the generator derives each country's **derived values**: its
  population and age structure (life expectancy, fertility), GDP per head, the income Gini, household wealth to
  income, the top tenth's wealth share, the employment and unemployment rates, the labour share, inflation, the
  policy rate, household debt to income, firm debt to value added, public debt to GDP, banks' capital ratio, home
  ownership, tax revenue and social spending to GDP, sector shares and trade to GDP, the banks' concentration, assets,
  deposits and liquid reserves, the central bank's assets, the lending and deposit rates, fixed investment to GDP, GDP
  growth, and enterprises per person employed. The development level draws one
  joint profile from a declared table of country groups' published profiles, perturbed within the group's declared
  dispersion from the seed, so values that go together stay together; each other choice sets its values within the
  profile's declared range for its level; risk appetite sets the distribution of risk aversion (a PREFERENCE). From
  the derived values, GEN.2's distributions and present values follow by declared mappings and accounting identities
  only; nothing is solved for an equilibrium (GEN.4).
- **GEN.13 PROCESS** — **Day zero.** The calendar day before the first day runs only its decision stage: every party
  takes each decision kind declared as an opening decision once, by its own rule, from its own drawn state and the
  snapshot — sellers post prices, employers post wage offers, banks set rates and standards, the central bank
  applies its rule, agencies confirm ratings, holders place orders — as simultaneous decisions on the same inputs
  (TIME.10). Nothing meets, settles or is repeated to agree; orders placed stand into the first day. Nothing a party
  decides is drawn.

**Invariants**

- **GEN.7 INVARIANT** — The opening world passes every family of the audit on its first day.

**Measures**

- **GEN.8 MEASURE** — **What settling changed**: for each opening distribution, its distance from the world's own
  at the end of settling and at later dates, and how far its members move within it. A distribution the world's
  dynamics carry far away, and one whose members never move, are both findings about mechanisms.
- **GEN.9** — _Retired_: comparing settling lengths needs a second run, and the world runs once. What settling
  changed is read in the run itself (GEN.8).

**Forbids**

- **GEN.10 FORBID** — **No credit for what the world does not do.** A stylised fact (N3) about a slow distribution —
  income, wealth, firm sizes — counts as reproduced only while the world holds it and moves it: its statistic stays
  in its benchmark range over the run, its distance from the opening (GEN.8) shows no drift away, and its members
  move within it by the world's own flows. A fact about behaviour — cycles, prices, markets, responses — counts only
  once the run has produced it after settling. Nothing counts from the opening day alone, and a fact the run has not
  yet had time to produce is reported as not yet credited.
- **GEN.11 FORBID** — No opening distribution, present value or parameter is changed because of what a run's results
  show about the world: each changes only with its source, recorded with it; the resolution, which represents the
  world rather than describing it, is set by measurement (N8.5). No setup is chosen or changed because of what a
  run's results show, and every realism report names the setup it ran under. No opening copied from a real
  country; no balancing
  that sets a price, a rate or a quantity for any reason but the accounts; no opening decision drawn instead of
  decided.

**Primitives**

- **GEN.12 PRIMITIVE** — The world constants (the owner's); the country-group profile tables and each choice's
  ranges by level (ENDOWMENT, from published data by country group, with sources); the derivation's mappings, the
  balancing procedure and the steady-path convention (declared); the settling length (the owner's setting, changed
  only for reasons other than what a run shows, GEN.11).

**Out of scope**

- A world grown from nothing by its own history alone: the opening is drawn, and what follows it is caused.
- A drawn past: no series of prices or statistics before the snapshot.

**Done when**

- A world of the full population is generated from a seed and a setup as one date's snapshot, balances on its first
  day, takes its day-zero decisions, settles for the declared period and plays.
- Its opening distributions, present values and every balancing change are listed.
- Realism is credited only for what the world holds and moves, or produces.

---


# PART B — MONEY AND OWNERSHIP

What money is, how anything changes hands, what can be held, and how every party keeps its books. Every
other system's cash leg, transfer and valuation lands here.

---

## B1. MON — Money and payments

**Purpose.** Money as a liability of a named issuer, held by named holders, moved only by payments, and
created and destroyed only by its issuers.

**Depends on:** PTY, NUM.

**State**

- **MON.1 STATE** — Money is a **liability of a named issuer**, denominated in one currency:
  - **central-bank reserves** — the central bank's liability to a bank;
  - **banknotes** — the central bank's liability to whoever holds them, held by a named party;
  - **deposits** — a bank's liability to a named depositor;
  - **the treasury's account** — the central bank's liability to its own treasury.
- **MON.2 STATE** — An **account** is (holder, issuer, currency). A party may hold several; a holding in a
  foreign currency is a real position.
- **MON.3 STATE** — A balance may be **negative** only as a recorded **loan** from the issuer to the holder
  (an overdraft agreed in advance, or a central bank's intraday or overnight credit) at a rate, within a
  limit; otherwise a payment that would take it below zero **fails**.
- **MON.16 STATE** — Every currency has a **smallest unit** (ENDOWMENT of its country), and every amount of
  money is a whole number of it. Every calculation that produces money — interest, a tax, a fee, a price times a
  quantity, a currency conversion, a share of a total — **rounds** by the convention of the contract or law that
  governs it (to the nearest unit, down, or in the payee's favour), and the rounding lands on a named party as
  that convention says. No fraction of a unit is ever held or paid, and none is lost; an agent's total is a whole
  multiple of its multiplicity, so each twin's share is whole (REP.9).
- **MON.4 STATE** — **Cash** is banknotes: issued by the central bank to banks against reserves, withdrawn
  by depositors against deposits, used in payments between parties, and deposited back. A party holding
  cash holds a claim on the central bank that no bank failure touches.

**Processes**

- **MON.5 PROCESS** — A **payment** is an instruction: payer, payee, amount, currency, reason, date. It
  settles by one rule (SET): payer minus, payee plus.
  - Between depositors of **one** bank it is a transfer of that bank's liability and moves no reserves.
  - Between depositors of **different** banks it moves the same amount of reserves between the two banks, in
    the large-value system, one payment at a time.
  - **Batched payments** — payrolls, card and retail payments, direct debits, a market's settlement between many
    parties — clear through the country's **net settlement system**, a named operator under its POLICY. Each payment
    is a leg of a batch instruction, and at the batch's settlement each bank's reserves move by its net across the
    batch. A payer that cannot pay fails with its own payments, and the payees it was paying lose them, drawn as
    REP.23 draws them where the pairing was not recorded. A bank that cannot cover its net, after the central bank's
    intraday credit (MON.3), has its customers' legs removed and the rest resettles, as the operator's rules
    (POLICY) state; each removed payment fails visibly.
  - A payment in cash moves banknotes between the two parties.
  - A payment to or from a party that holds no money in its currency — no account and no banknotes — cannot settle,
    and fails visibly for that cause.
- **MON.6 PROCESS** — **Money is created and destroyed only by its issuer's own transactions.**
  - A bank creates deposits whenever it **pays** a non-bank: a loan disbursed, a security bought, a wage or
    supplier paid, interest or a dividend paid out.
  - A bank destroys deposits whenever a non-bank **pays it**: a loan repaid, interest or a fee charged, a
    bond or share the bank issued bought by a depositor.
  - A central bank creates reserves when it buys an asset, lends, or pays; it destroys them when it sells,
    is repaid, or is paid. Banknotes are issued and retired against reserves one for one.
  - A payment from a depositor to the treasury's account at the central bank destroys a deposit and
    reserves together, and a treasury payment out creates them.

**Invariants**

- **MON.7 INVARIANT** — The balances held at every issuer, per currency, equal that issuer's recorded money
  liability.
- **MON.8 INVARIANT** — Every change in the money stock of a currency is matched, leg for leg, by a
  transaction to which one of its issuers is a party (MON.6), and by nothing else.
- **MON.9 INVARIANT** — The banknotes held by all parties equal the banknotes the central bank has issued.

**Measures**

- **MON.10 MEASURE** — The money stock, its composition (reserves, notes, deposits by holder class) and its
  change by source are reads, published with a lag (STA).

**Forbids**

- **MON.11 FORBID** — No money without an issuer; no balance that is nobody's liability.
- **MON.12 FORBID** — No silent negative balance: a shortfall is a loan somebody agreed or a failed payment
  somebody can see.
- **MON.13 FORBID** — No conversion at the ledger: money paid in one currency arrives in that currency, and
  changing it is a separate trade with a counterparty (FX).
- **MON.14 FORBID** — No bearer money held by nobody: every banknote has a named holder in the world, even
  though its real-world counterpart would not.

**Primitives**

- **MON.15 PRIMITIVE** — Overdraft terms are contract terms set by the lending bank; the central bank's
  intraday and overnight credit terms are its POLICY; the net settlement system's rules (POLICY of its operator);
  what drawing and depositing banknotes costs a party (TECHNOLOGY).

**Out of scope**

- Private currencies, crypto-assets and payment-system outages. Banknotes carry no theft or loss hazard; holding
  them in quantity costs storage and security, a service bought from named providers (SRV).

**Done when**

- Every payment in the world moves money between named accounts; money stock changes only through an
  issuer's own transactions; a bank run can move deposits into banknotes.

---

## B2. SET — Settlement

**Purpose.** The single rule by which every state change is applied: a numbered, two-sided, dated
instruction, settled atomically or failed visibly.

**Depends on:** MON.

**State**

- **SET.1 STATE** — Every change to who holds what — money, securities, goods, dwellings, land, contracts —
  is a **numbered instruction** with its legs, its cause (trade, payment, production, consumption, corporate
  action, default, estate, hazard, tax), its **trade date** and its **settlement date**.
- **SET.2 STATE** — A **settlement convention** belongs to each market and instrument (same day for money
  and most payments, one or two business days for securities), declared by the market (POLICY). Between
  trade date and settlement date, both parties hold a **commitment**, recorded on both books.
- **SET.3 STATE** — A **fail** is a recorded state: an instruction due that did not settle, with its cause
  (the payer had no money, the seller had no free units, a lien blocked it), and what follows from it is
  what the contract says: a retry, a penalty, a buy-in, a default.

**Processes**

- **SET.4 PROCESS** — **All legs or none.** An instruction's legs settle together — delivery against
  payment for a trade, both currencies for an exchange — or none does.
- **SET.5 PROCESS** — **Finality.** A settled instruction is never reversed; an error is corrected by a new
  instruction in the other direction.
- **SET.6 PROCESS** — **Order within the settlement stage is declared** and the same every day, so that two
  instructions drawing on one balance cannot both succeed by luck; the close runs a final pass that lets a
  ring of payments that can settle together do so.
- **SET.7 PROCESS** — A party that ends during a day still has every instruction naming it settled or
  refused by name, against its estate.

**Invariants**

- **SET.8 INVARIANT** — For every asset, instructions in minus instructions out equals the change in
  holdings, per holder, per day.
- **SET.9 INVARIANT** — Every settled instruction's legs sum to zero in each currency and unit; every leg names
  its party; every settled trade has both its legs. The one exception is a **physical transformation** (Law 5):
  production (checked against its way, TEC.9), extraction (against its deposit, GEO.12), and consumption, spoilage
  and destruction (against the purchase, the storage or the hazard event, GDS.10), each naming its party and what
  accounts for it.

**Measures**

- **SET.10 MEASURE** — Gross and net settlement values, fails by cause, and the size of the closing ring are
  published per day.

**Forbids**

- **SET.11 FORBID** — No move of anything without an instruction; no instruction with one side; no partial
  settlement of a trade; no instruction applied twice.

**Memory of the world**

- **SET.12 STATE** — **Snapshots.** The complete state of the world — every party, holding, lot, lien, contract,
  commitment, outlook, pending event and the seed streams' positions — is recorded as a **full snapshot**: every
  save is full, so a restore reads one snapshot and nothing else. A snapshot is the state itself, not a summary of
  it. Snapshots are written at
  declared moments — when the player saves, when the world is set aside, and at the declared interval — and the
  world may pause while one is written, within the budget for it (N8.10).
- **SET.13 STATE** — **Retention.** An instruction lives until the day's close, when the audit has read it. What
  outlives the day is the state itself — holdings, lots, contracts, and each party's own records of what it will
  need (SET.16) — and what the world publishes or keeps as history: statistics (STA), statements and reports
  (ACC.9, RAT.2), each market's daily mark and volume, dated events, and each party's own bounded memory of what
  it observed (VAL). Each kind of history is kept for a declared horizon (RESOLUTION); a kept record naming a party
  that ended before the horizon reads as ended, on its date.
- **SET.14** — _Retired_: rebuilding past days is not required; a save restores the world as it was (SET.15).
- **SET.15 INVARIANT** — A world restored from a snapshot holds exactly the state that was saved, and continues
  from it.
- **SET.16 FORBID** — No fact that a decision reads may exist only in an instruction: anything a party
  needs later — a basis, a credit record, a contract's history of arrears — is part of the state, and so is in
  every snapshot.

**Primitives**

- **SET.17 PRIMITIVE** — Settlement conventions per market (POLICY of the market); snapshot intervals and history
  horizons (RESOLUTION).

**Out of scope**

- Payment-system outages (Appendix D); rebuilding past days (SET.14).

**Done when**

- A saved world restores exactly and continues; a fail has a cause and a consequence; securities trades settle on
  their convention's date.

---

## B3. REG — Holdings and instruments

**Purpose.** What can be held, who holds it, what it is a claim on, and what binds it.

**Depends on:** SET.

**State**

- **REG.1 STATE** — A **holding** is (holder, instrument or asset, quantity in its own unit), and the holding
  is a chain of **lots**, each with the date and price at which those units were acquired (its basis). An agent's
  holding is one lot at average cost, its quantity a whole multiple of its multiplicity (REP.9).
- **REG.2 STATE** — A **lien** marks units of a holding as pledged to a named party; **free units** are held
  units minus pledged units, and only free units can move or be pledged again. A re-pledge is a traceable
  chain.
- **REG.3 STATE** — Every instrument has an **issuer** (or, for a real asset, none), an **issued amount**
  changed only by issuance, reopening, buyback, conversion, amortisation, maturity or default, and **terms**
  that belong to its **instrument family** (REG.5–REG.10).
- **REG.4 STATE** — The register answers both directions at once: what a party holds, and who holds an
  instrument.
- **REG.5 STATE** — **Debt claims.** Any instrument that promises money has: an issuer; a principal in units of face;
  a currency; a maturity (or perpetuity); a coupon of a stated form — fixed, floating over a named reference rate that
  is itself transacted, zero, **indexed** to a published index, **step-up**, or **payable in kind**, each floating or
  indexed amount carrying its **current fixing** as a term, which stands until the reference is next fixed or
  published; a payment schedule with a day-count convention; an early-termination regime (none, callable, putable,
  make-whole, convertible); a definition of default observable by a holder; a claim on failure and its **seniority**;
  and any **conversion or write-down** term (a convertible into shares at a ratio, a contingent instrument that
  converts or writes down when a named ratio crosses a stated level).
- **REG.6 STATE** — **Equity.** A share is a residual claim on its issuer, in a class (ordinary,
  preferred), counted in shares, perpetual, carrying votes by class and, for preferred, a stated dividend
  that ranks ahead of ordinary shares. Its value to a holder is never below zero (limited liability), which
  is a property of the legal form (PTY.4), not a bound.
- **REG.7 STATE** — **Fund units.** A claim on a fund's assets, counted in units, redeemable or tradeable by
  the fund's terms (FND).
- **REG.8 STATE** — **Contracts between two parties** — loans, deposits, leases, tenancies, employment,
  insurance policies, derivative contracts, trade-credit invoices, guarantees — are recorded as a contract
  with both parties, its terms and its schedule, an asset to one side and a liability to the other.
- **REG.9 STATE** — **Real assets** — goods, plant, vehicles, dwellings, land, infrastructure, extraction
  rights — are holdings in physical units with no issuer. Plant and dwellings keep their site and condition
  through every sale: an individual's are named units; an agent's are counted by zone and class, with each unit's
  tile a pairing drawn when needed (REP.24).
- **REG.10 STATE** — **Commitments** — unsettled trades, undrawn credit lines, uncalled capital, pending
  orders — are recorded on both parties' books as what they are.

**Processes**

- **REG.11 PROCESS** — An instrument's **events** — coupon, dividend, amortisation, call, conversion,
  maturity, default, split, buyback, write-down — are applied on their dates to the **holders of record at
  the start of that day**, and each moves money or units, or says why not.
- **REG.12 PROCESS** — An instrument **ceases** — it matures, is redeemed, converts, or its issuer's estate
  resolves it — and every holding in it resolves to something named: cash, another instrument, a recovery
  claim, or a recorded loss.

**Invariants**

- **REG.13 INVARIANT** — For every issued instrument, holdings sum to the issued amount exactly.
- **REG.14 INVARIANT** — For every contract, and every line of identical contracts (REP.3), the asset on one
  side and the liability on the other are one record, read from its sides.
- **REG.15 INVARIANT** — No holding of physical units is negative; free units are never negative.

**Forbids**

- **REG.16 FORBID** — No holding without a holder; no claim without an issuer; no short position without a
  borrow from a named lender (DLR); no unit pledged twice.
- **REG.17 FORBID** — No invented grouping held in place of an instrument: a party holds the issue it bought.

**Primitives**

- **REG.18 PRIMITIVE** — None of its own: instrument families and their terms are declared data (Law 10), and each
  issue's terms are its issuer's.

**Out of scope**

- Nothing.

**Done when**

- Every instrument family can be issued, held, pledged, transferred and ended; holdings sum to issued for
  every instrument every day; every contract is one record with two sides.

---

## B4. ACC — Accounting

**Purpose.** How every party keeps its books: when income is recognised, how each position is carried,
what equity is, and what a statement shows. The books are a **read** of the register and the contracts,
under stated rules — never a separate set of numbers.

**Depends on:** REG.

**State**

- **ACC.1 STATE** — **Accrual basis.** Income and expense are recognised when **earned or incurred** —
  interest as it accrues, revenue on delivery, a wage as the work is done, a tax when its base arises — and
  every recognised item that has not been paid is a **receivable or payable** with a named counterparty.
  Cash and income are two records and both are kept.
- **ACC.2 STATE** — **Each position has a carrying basis**, declared by its holder when it is acquired,
  according to what it is held **for**, from the options its legal form permits:
  - **fair value through income** — a dealer's inventory, a trading book, a fund's assets;
  - **amortised cost** — a loan held to collect, a bond held to maturity;
  - **lower of cost and net realisable value** — a firm's inventory of goods;
  - **cost less depreciation and impairment** — plant, dwellings held for use, infrastructure.
- **ACC.3 STATE** — For every position, the **unrealised difference** — units × (latest price − carrying
  value) — is readable at any time wherever a price exists, even when the carrying basis keeps it out of
  income. A carrying basis hides a loss from income; it never hides it from a reader.
- **ACC.4 STATE** — Every party that has owners keeps an **equity account**, moved only by named events:
  capital paid in, capital returned, income recognised, dividends and buybacks, and revaluations the basis
  sends to equity. A party without owners (a household, an estate) has only **net worth**, which is a read.
- **ACC.5 STATE** — A **group** reports **consolidated** statements: its subsidiaries' positions combined,
  intra-group claims and trades eliminated, and minority holders shown. Each member still keeps its own
  books, and limited liability stays with each legal entity.
- **ACC.6 STATE** — **Inventory cost flows** first-in-first-out or by weighted average, chosen by the firm,
  disclosed and applied consistently; last-in-first-out is not permitted. A small firm's agent uses weighted average,
  as its holdings do (REG.1).

**Processes**

- **ACC.7 PROCESS** — A **write-down** (an inventory below net realisable value, an impaired loan, an
  impaired plant) is a charge to income on the day it is recognised; a later recovery reverses it only up to
  the original cost. A loan's **expected-loss provision** moves with its holder's own assessment, and every
  move is an income event.
- **ACC.8 PROCESS** — A **sale realises** the unrealised difference in full, on the day.
- **ACC.9 PROCESS** — A **statement** for a period — income, balance sheet, cash flow, in the party's own
  money — is a read of the recorded events and positions under ACC.1–ACC.6.

**Invariants**

- **ACC.10 INVARIANT** — For every party with an equity account: assets minus liabilities, read from the
  register and the contracts at their carrying values, equals the equity account. The two are **independent
  records**, which is what makes the equality a check.
- **ACC.11 INVARIANT** — Reported income for a period equals the change in the equity account over that
  period **minus capital transactions with owners** (paid in, returned, distributed) and minus revaluations the
  carrying basis sends straight to equity.
- **ACC.12 INVARIANT** — The contractual amount receivable equals the amount payable, contract by contract and
  line by line, across the whole world, whatever each side's carrying value.

**Forbids**

- **ACC.13 FORBID** — No income without a delivery, an accrual or a receipt; no income credited to a holder
  that neither received cash nor holds a receivable.
- **ACC.14 FORBID** — No cost recognised twice: a cost capitalised into stock or plant is not also expensed
  when incurred.
- **ACC.15 FORBID** — No goods inventory of a non-dealer carried above cost; no stored value beside units
  that is not a read.
- **ACC.16 FORBID** — No reported number the records do not produce, and no smoothing.

**Primitives**

- **ACC.17 PRIMITIVE** — The permitted carrying bases per legal form and holder purpose (POLICY of the
  accounting standard-setter); depreciation methods (a choice by the owner within the standard).

**Out of scope**

- Outside audit of accounts, and accounting fraud.

**Done when**

- Every party with owners passes ACC.10 every day; a bank holding one bond at amortised cost and one at fair
  value shows the price fall in income for one and as an unrealised difference for the other; a group's
  consolidated statement eliminates its intra-group loan.

---

# PART C — PRICE FORMATION, EXPECTATIONS AND VALUATION

How prices come to exist, and how every party forms the view of the future it acts on. Every market in
Parts F–K is one of the forms here, and every decision in Parts D–K reads an outlook or a value formed
here.

---

## C1. MKT — Market forms and price formation

**Purpose.** The small set of real mechanisms by which prices are formed, the one meaning of a price, and
what happens when a market does not clear.

**Depends on:** SET, REG.

**State**

- **MKT.1 STATE** — A **market** is a place where parties with **different reasons** meet over one
  thing, keyed as that thing is keyed (Law 9): an instrument, a good at a place, a service in a region,
  labour of an occupation in a region, dwellings in a location, a route's carriage. It has a declared
  **form** (MKT.3–MKT.8), **meeting days**, a **settlement convention**, and **who may take part**.
- **MKT.2 STATE** — A **print** is a formed price with its market, instrument, day, unit, currency, quantity
  and form. A print is public (OBS). A market that formed no price has **no new print**, and its last print
  shows its age.

**The forms** — every market is one of these, declared as data (Law 10):

- **MKT.3 PROCESS** — **Call auction.** Participants post **schedules** (how much at each price); one price
  is found where posted supply meets posted demand, and quantity is rationed at that price by a stated
  rule (pro rata or priority). Used where a market meets at a moment: a sovereign or corporate primary
  issue, an opening, a commodity session, an interbank session.
- **MKT.4 PROCESS** — **Continuous book.** Limit orders rest; an incoming order that crosses a resting one
  trades at the resting price, in price-then-time priority. Used for listed shares, funds that trade, and
  futures. Orders posted in the day's decide stage arrive at the book in an order drawn by lot, and nobody reacts
  within the day; the book then ends in a **closing call auction** (MKT.3), whose price is the day's close.
- **MKT.5 PROCESS** — **Dealer market.** Dealers post two-way quotes from their own inventory, funding and
  risk (DLR); clients trade on those quotes or ask several dealers and take the best; dealers trade with each
  other to redistribute inventory. Used for bonds, currencies, over-the-counter derivatives.
- **MKT.6 PROCESS** — **Posted price.** A seller posts a price and serves whoever comes at it until its
  capacity or stock runs out; buyers compare the posted prices they can see, choose, and may walk away; a
  seller revises its price on its own schedule from what it sold and what it holds. Used for retail goods,
  services, rents asked, wages offered in vacancies.
- **MKT.7 PROCESS** — **Bilateral quote and negotiation.** One named party asks one or several others for
  terms; each answers from its own position or declines; the asker accepts one, bargains, or walks away.
  Used for loans, insurance cover, trade credit, wages on hiring, dwelling sales, mergers, private deals.
- **MKT.8 PROCESS** — **Administered price.** A named institution declares a rate or a price for a facility
  it stands behind (a central-bank facility rate, a statutory benefit, a regulated tariff) and meets the
  quantity that comes to it. It is the exception of Law 3 and exists only with that quantity response. An
  open-ended fund's dealing is one: it issues and cancels its units, and its authorised dealers create and redeem
  them, at the **next** value it computes from the prices its holdings formed (forward pricing, FND.5), so the
  quantity responds on both books and the value is a read of formed prices, never a price the fund makes.

**Processes common to every form**

- **MKT.9 PROCESS** — Every participant's order, quote or offer comes from its **own state**: its position,
  its cost of funds, its constraints, its outlook and its value of the thing (VAL). A participant who does
  not want to trade does not post, and that is an outcome.
- **MKT.10 PROCESS** — **A market can fail.** No overlap, no bid, no seller, a dealer that stepped back: each
  is a recorded outcome with consequences that propagate — the issuer is not funded, the seller keeps its
  stock, the borrower is refused, the vacancy stays open.
- **MKT.11 PROCESS** — Every match becomes an **instruction** (SET) with two named sides, settling on the
  market's convention.
- **MKT.12 PROCESS** — A print becomes the **mark** for everybody whose carrying basis marks to it (ACC.2),
  in the market where the holder's units are. Each form declares which print is the day's mark: a call auction's
  price; a continuous book's closing auction; a dealer or bilateral market's **fixing** — the day's trades taken
  by a named publisher's declared method (MKT.20); a posted-price market's sales at the posted price.
- **MKT.20 PROCESS** — **Valuation.** Where a position has no print of its own — a seasoned swap or option, a bond
  that did not trade, a loan, a dwelling not sold, a liability to policyholders or members — a **named valuer**
  values it by a **published method** whose inputs are prints: a clearing house's settlement price from its
  members' trades and submissions, a pricing service's fixing, an appraiser's valuation from nearby sales, a
  statutory assessor's roll, an actuary's discounting at the day's traded rates. A valuation is labelled as such,
  shows the prints it rests on and their age, and is never a print: it does not enter an index or a market. Marks,
  margin, net asset values, tax bases, loan-to-value tests, provisions and resolution may read a valuation where
  their rule says so. A method that fits a curve through traded points marks every point it did not trade as
  interpolated, and every point beyond the longest traded one — as a pension's or an annuity's liability needs — as
  extrapolated.

**Invariants**

- **MKT.13 INVARIANT** — In every market, quantity bought equals quantity sold, and money paid equals money
  received, per match.
- **MKT.14 INVARIANT** — A print always came out of a match; a quote, an order, a reservation, a search bound or
  a valuation is never a print.

**Measures**

- **MKT.15 MEASURE** — Depth, bid–offer width, turnover, the share of meetings that failed to form a price,
  and the age of last prints, per market.

**Forbids**

- **MKT.16 FORBID** — **No price-taker of a price not yet formed**: no order written against the price the
  mechanism is about to produce. A party that wants to trade "at market" posts a limit it chose. The one exception
  is dealing in an open-ended fund at its next value (MKT.8), which no order in that fund's own dealing forms.
- **MKT.17 FORBID** — **No buyer or seller of last resort by construction**: no participant whose order is
  "whatever is left, at any price", and the mechanism never adds demand or supply to make itself clear.
- **MKT.18 FORBID** — No price from a written path, a formula, a target, a parity condition or another
  price's statistic.
- **MKT.19 FORBID** — No spread applied to a mid, and no stated spread table: a bid–offer is what dealers
  posted.

**Primitives**

- **MKT.21 PRIMITIVE** — Each market's form, meeting days, settlement convention and participants (POLICY of its
  operator); tick sizes and price points (POLICY of the trade); each valuer's published method (POLICY of the
  valuer).

**Out of scope**

- Trading within the day: orders posted in a day arrive by lot and nobody reacts until the next (MKT.4).

**Done when**

- Every market in the world is declared as one of the forms; a market with no overlap reports a failure and
  its consequences run; every print can be traced to the match that made it.

---

## C2. VAL — Expectations and valuation

**Purpose.** Every decision in this world looks forward, and what the decider sees is **its own view**,
formed from what it has observed, by its own methods. This system is the discipline that keeps that view
personal, fallible and heterogeneous — and able to value things that have never been observed.

**Depends on:** TIME, PTY, NUM, CHN, MKT.

**State**

- **VAL.1 STATE** — An **outlook** is a party's own forecast of a variable it acts on — its income, the
  prices it pays and gets, a rate it borrows at, the demand for what it sells, whether it keeps its job, a
  counterparty's ability to pay — with its **unit, currency, horizon and date**.
- **VAL.2 STATE** — A **value** is what a thing is worth **to that party**: its own estimate of what the thing
  will pay, earn, save or yield it, weighed by its own patience and caution. A value exists where no price
  does — a new issue, an unlisted firm, a plant not yet built, a dwelling not yet sold, an untried policy.
  A price is the market's answer; a value is each party's question.
- **VAL.3 STATE** — Each party holds a small set of **forecasting heuristics** for each outlook, from a
  declared menu (VAL.6), with a record of how well each has recently performed for that party.
- **VAL.4 STATE** — A **surprise** is observed minus expected, per party, variable and date, and it is
  recorded. **Confidence** is a read of the width of a party's recent surprises.
- **VAL.23 STATE** — For agents (REP.1), outlooks are of two kinds:
  - An outlook of a **public** variable (a price index, a rate, a house price, a return) is formed by a method — a
    heuristic, a memory, and the window of lived experience its age class weights (older people weight a longer
    history, and no window reaches back before the snapshot, GEN.5) — from the public series as published. Everyone
    using the same method observed the same series in the same way and holds the same outlook, so it is computed once
    for that method and read by each of them.
  - An outlook of the agent's **own** variables (its income, its job, its sales) is its own position (REP.20),
    formed from its own receipts and events.

  No agent reads an experience it did not have.

**Decisions**

- **VAL.7 DECISION** — **Heuristic switching.** A party weights its heuristics by their recent forecasting
  performance **for that party**, updating on its own schedule; the intensity with which it switches is a
  PREFERENCE. An agent relies on one heuristic at a time — its **stance** — and reconsiders it on its review
  occasions, choosing among the heuristics by their recent performance on the public series under its own method,
  with its own taste draw (REP.22); so alike agents split across stances, and the shares move with what has
  worked, as in the discrete-choice switching literature. Parties that have seen different histories therefore hold
  different outlooks, and the mix of heuristics across a market shifts with what has recently worked — which is a
  known source of boom, bust, fat tails and volatility clustering.

**Processes**

- **VAL.5 PROCESS** — **Outlooks are formed from what the party observed**: its own receipts, payments,
  fills and holdings; public prints and published statistics as published, with their lags; announcements
  (an auction calendar, a policy decision, a new tax rate, a firm's guidance); public events (OBS.3); and the
  terms of its own contracts. Nothing else.
- **VAL.6 PROCESS** — **The heuristic menu.** Each outlook is produced by one or more simple, fallible rules
  a real decider uses:
  - **adaptive** — last outlook corrected toward what happened, at the party's own speed (its memory);
  - **trend-following** — extrapolate the recent change;
  - **anchoring to a reference** — expect a return toward a level the party has observed over a long
    window, or toward a published target;
  - **announcement** — take a published, dated change (a tax rate, a contract reset, a policy rate
    decision) into the outlook from the date it applies.
- **VAL.8 PROCESS** — **A party's own simple model of value.** To value a thing, a party applies a method
  fitted to what the thing is, using its own outlooks:
  - a claim: its own outlook of the payments, discounted at its own required return for the risk it sees;
  - a share or a firm: its own outlook of the distributions or earnings, discounted likewise, or compared
    with what it has seen similar things fetch;
  - a plant or a project: the output it expects to sell, at the prices it expects, less what running it
    costs, over the plant's life, against what the money costs it;
  - a dwelling: the rent it saves or earns, and the price it expects, against its alternative;
  - a job offer: the wage against its outside option and the cost of moving;
  - an untried platform or policy: the policy applied to its own current position and outlooks.

  Each method is a **reason**, not a price. The resulting value enters the party's order as its reservation.
- **VAL.9 PROCESS** — **Confidence changes behaviour.** A party whose recent surprises were wide holds more
  liquidity, spends less of an expected windfall, bids more cautiously and demands more for risk, in the
  measure its own risk aversion sets.
- **VAL.10 PROCESS** — **No history, no silence.** A party valuing something it has never observed starts
  from the closest things it has observed (the same issuer's other debt, a comparable firm's price, the
  last dwelling sold nearby, published statistics, the terms and records of its own contracts), and learns from
  there. At the opening, what it has observed is the snapshot (GEN.5): the present values, its own drawn state and,
  for a household, its surveyed expectations. A record it has not yet had — a heuristic's performance, a surprise —
  is absent, and each decision that reads one states what it does while it is absent.

**Invariants**

- **VAL.11 INVARIANT** — Every outlook and value was formed on a day before the day it is used, or earlier in
  the same day's decision stage from facts that existed before that stage.

**Measures**

- **VAL.12 MEASURE** — Outlooks **disagree**: the dispersion of outlooks across parties, per variable, is
  reported, and widens after surprises.
- **VAL.13 MEASURE** — Outlooks **lag turning points**, and the lag differs by party with its memory and its
  heuristic mix.
- **VAL.14 MEASURE** — The parties most surprised by a shock change behaviour first; others follow as
  consequences reach them.
- **VAL.15 MEASURE** — The share of each heuristic in use moves over time, and its movements precede price
  swings.

**Forbids**

- **VAL.16 FORBID** — **No global expectation**: no variable anywhere is "the" expected inflation, growth or
  price that decisions read.
- **VAL.17 FORBID** — **No model forecast**: no outlook or value is computed by running the world forward
  and handing a party the answer.
- **VAL.18 FORBID** — No outlook reads another party's private state, the model's internals, or the result
  of the stage it is used in.
- **VAL.19 FORBID** — No sentiment, optimism or confidence parameter: confidence is a read of surprises, and
  a sentiment index is a published aggregate that causes nothing.
- **VAL.20 FORBID** — **A value never becomes a price**: it does not print, enter an index, or stand in for a
  price anywhere one is required. Where a rule needs an estimate — an impairment, an expected-loss provision, a
  liability discounted at traded rates — the estimate is the holder's own or a named valuer's (MKT.20), labelled as
  such and never a print.
- **VAL.21 FORBID** — **No common value**: no single valuation shared by every party of a kind. A world where
  everybody values alike trades once and stops.

**Primitives**

- **VAL.22 PRIMITIVE** — Finite type sets across parties of memory, switching intensity, required return
  (patience) and risk aversion (PREFERENCE, NUM.4); the heuristic menu and how many of its heuristics a party tracks
  per outlook (SHAPE, declared as the accepted stand-in for how people actually forecast, with its source in the
  experimental and behavioural literature).

**Out of scope**

- Forecasting by fitted statistical models beyond the heuristic menu.

**Done when**

- Every decision in the world reads an outlook or value of its own party; two parties with different
  histories value one thing differently; a party can value a first issue and an election platform; the
  heuristic mix is reported and moves.

---

# PART D — PEOPLE

The population and the decisions of the households it lives in. People are the source of labour, the end
point of consumption, the ultimate owners of every firm and fund, the voters, and the reason any of the
rest exists.

---

## D1. POP — Population and demography

**Purpose.** A population of hundreds of millions of people who are born, grow up, learn, work, form and leave
households, move, fall ill, retire and die, so that the size, age structure, skills and location of the
population are outcomes.

**Depends on:** PTY, CHN, GEO, REP, VAL, HH, LAB, HSG, BNK, SOC.

**State**

- **POP.1 STATE** — Every **person** has a birth date, a household, a region, a health state, a skill
  profile (per occupation family), an education record, a labour-market state (A person is in exactly one
  of: in education, employed, unemployed and searching, out of the labour force, retired), an
  employment history and its **kin** — its parents and children living in other households — which is whom
  inheritance law names as heirs (POP.9). A person of an agent is held in its household (REP.26): its birth date
  (REP.25), role and the person attributes its kind declares, zone as its household's attribute, employment in its
  attachments, its labour state a read of its attachments and participation, its employment history in its
  employment's start band, the start of its search and its contribution records, kin as lines between households
  (REP.3), and the clocks rules read as its household's attributes (REP.41).
- **POP.2 STATE** — A **household** has members, a dwelling (owned, rented, or a room let by another household
  under a tenancy), a budget, holdings and debts, and its own preferences drawn at its formation (NUM.4).

**Decisions**

- **POP.10 DECISION** — A household's decision to have a child reads its income, its dwelling, its members'
  ages, its outlook and its own preference for children; it is never a birth rate.
- **POP.17 DECISION** — **Forming and separating.** Two single adults who meet (POP.7) each decide whether to form a
  household with the other, and the household forms only if both accept; either adult of a couple may decide to
  separate. Each decides from its own income and outlook, the needs and dwelling costs of one household against two
  at the rents and prices in reach, what the declared law's division would give it, and its own taste; never a
  formation or separation rate.
- **POP.18 DECISION** — **Schooling and retraining.** A household decides where each of its young members is
  schooled (public or private), whether it continues beyond the compulsory stages and in which field, and whether
  an adult retrains, from the wages and employment it expects by occupation family and skill, the fees, waits and
  earnings forgone, its liquidity and what it can borrow, its risk aversion and its own taste; never an enrolment
  rate.

**Processes**

- **POP.3 PROCESS** — **Death**: each person faces a mortality hazard by age and health (CHN), from a declared
  life table. A death is an event; the person's share of the household's claims passes by the household's
  rules and by inheritance law (POP.9), and a household with no surviving member becomes an estate.
- **POP.4 PROCESS** — **Illness and disability**: a health hazard by age can make a person unable to work for
  a spell or permanently, which is what sickness and disability insurance and benefits respond to.
- **POP.5 PROCESS** — **Birth**: a household **decides** whether to try for a child (POP.10), and the
  realisation follows a declared conception hazard (CHN). A child joins its parents' household, costs
  consumption, and is in education until it leaves it.
- **POP.6 PROCESS** — **Education and skill**: a young person's education continues while its household
  chooses (POP.18) and pays for it (or the state provides it, SOC); schooling raises skill in an occupation family by
  a declared technology; experience on the job raises it further and unemployment erodes it. An adult who retrains
  (POP.18) spends the declared time in training, out of work or on fewer hours, pays its fees, and gains skill in
  the field it trains for by the same technology.
- **POP.7 PROCESS** — **Household formation and dissolution**: an adult leaves its parents' household when it
  can afford a dwelling of its own (a decision, HH.8); two adults who meet in a search-and-meeting process
  within a region (CHN) form a household by their decision (POP.17); a household dissolves by separation
  (POP.17) or by the death of its last member. Each is a dated event, and the household's holdings and debts
  are divided by declared law.
- **POP.8 PROCESS** — **Migration**: a household (or an adult leaving one) **decides** to move to another
  region or country when its own expected income, housing cost and prospects there, less the cost of
  moving, beat staying (HH.9); a move across a border needs the destination's admission (POLICY, XB).
- **POP.9 PROCESS** — **Inheritance**: a person's estate pays its debts, taxes and costs, selling what it must to
  do so, and distributes the rest to named heirs **in kind** by declared law (POLICY): a dwelling, holdings and a
  household business pass to the heirs, who keep, sell or run them as they choose.

**Invariants**

- **POP.11 INVARIANT** — The population equals births and arrivals minus deaths and departures, per region
  and in total, every day.
- **POP.12 INVARIANT** — Every person is in exactly one labour-market state; employed plus unemployed plus
  out of the labour force plus in education plus retired equals the population.

**Measures**

- **POP.13 MEASURE** — Age structure, fertility, dependency ratio, household size, regional population and
  net migration, and how each moves with incomes, housing costs and employment.

**Forbids**

- **POP.14 FORBID** — No birth rate, migration rate, participation rate or population path is ever stated:
  each is a read of individual events.
- **POP.15 FORBID** — No death without a cause and a destination for everything the person held and owed.

**Primitives**

- **POP.16 PRIMITIVE** — Life tables and health hazards by age (TECHNOLOGY, from data); conception hazard
  (TECHNOLOGY); preference for children (PREFERENCE distribution); the schooling, retraining and experience that
  raise skill and the unemployment that erodes it (TECHNOLOGY);
  meeting rates (TECHNOLOGY); inheritance, education, family and migration law (POLICY).

**Out of scope**

- Sex-specific demography beyond what the life tables carry; partnership preferences beyond region and
  age; crime; intra-household bargaining.

**Done when**

- In the run, the population's size, age structure and regional spread are outcomes of births, deaths, formation
  and migration; a region that loses its jobs loses people.

---

## D2. HH — Household decisions

**Purpose.** What a household does with its time, income and wealth: work, spend, save, invest, borrow,
house itself, insure itself, and vote. Every decision is its own, from its own state and outlook.

**Depends on:** POP, VAL, ACC.

**State**

- **HH.1 STATE** — A household's **balance sheet**: deposits, banknotes, securities and fund units held
  directly, pension claims, insurance policies, dwellings and land, business equity, less mortgages,
  consumer credit and other debts. Net worth is a read.
- **HH.2 STATE** — A household's **income**: wages, business income, interest, dividends and coupons,
  rents received, transfers and benefits, pensions, annuities, inheritances, each from a named payer. Income
  it has not received and is not owed is not income.
- **HH.3 STATE** — A household's **preferences** (drawn at formation from declared distributions): patience,
  risk aversion, tastes across categories of consumption, the value it places on leisure, on dwelling size
  and on location, and its memory.

**Decisions**

- **HH.4 DECISION** — **How much to spend** this period, from: its current income, its wealth and how liquid it is,
  its outlook for income and prices, its confidence, its patience, the rate it earns on saving, and what it can
  borrow. A household that cannot borrow spends only what it has; when that is less than its needs (HH.20) it claims
  what benefits it is eligible for (SOC) and otherwise goes without, which is recorded as an event.
- **HH.5 DECISION** — **What to buy**: spending is divided across goods and services by its tastes and the
  prices it faces where it can shop, substituting as relative prices move; it pays consumption tax at the till.
  On each shopping occasion it chooses a seller among those it can reach by price, distance and its own taste
  for each that day (REP.22), so households that are alike spread across sellers. It buys what it needs
  (HH.20) by quantity and spends a budget in money on the rest; what is used up is bought at market meetings.
- **HH.6 DECISION** — **Whether and how much to work**: each adult chooses to search, accept an offer, stay,
  quit, reduce hours, retire, or leave the labour force, from the wage it can get, its outside options
  (benefits, other income, its partner's income), its value of leisure and its outlook (LAB).
- **HH.7 DECISION** — **How to hold its savings**: across deposits, cash, bills held directly, money funds,
  bond and equity funds, shares, dwellings and pensions, by yield, risk, liquidity and what it trusts,
  rebalancing on its own schedule — which is how a policy rate reaches a saver who is not a borrower.
- **HH.8 DECISION** — **Where to live**: rent or buy, where, and how large, from its income, its savings for
  a deposit, what lenders will offer it, rents and prices in reach, and its outlook for both (HSG).
- **HH.9 DECISION** — **Whether to move** region or country (POP.8).
- **HH.10 DECISION** — **Whether to borrow**, for a dwelling, a vehicle, consumption or a shortfall, and from
  whom, among the offers it can get (BNK) — including borrowing against the equity in its dwelling, by refinancing
  or a home-equity line at the lender's valuation (MKT.20).
- **HH.11 DECISION** — **Whether to insure**, against what, and with whom, at the premiums quoted (INS).
- **HH.12 DECISION** — **How to vote** (POL).

**Processes**

- **HH.13 PROCESS** — Debt service is paid from the household's accounts on its dates; a household that
  cannot pay falls into **arrears**, and continued arrears are a **default** with its contract's
  consequences: collection, repossession, a credit record that later lenders read.
- **HH.14 PROCESS** — A household in arrears **acts**: cuts spending, draws savings, sells assets, borrows
  elsewhere, moves, sends another member to work.
- **HH.21 PROCESS** — **Personal insolvency.** A household that cannot pay its debts may enter, or its creditors
  may put it into, its country's personal insolvency procedure (POLICY): its assets beyond the law's exemptions
  are sold through an estate-like party, the proceeds paid in the law's order of claims; where the law provides, its
  income above a declared allowance is paid to its creditors for a declared period; the rest of its debts are
  discharged after a declared period, and its record is kept by the credit bureau for a declared time.

**Invariants**

- **HH.15 INVARIANT** — Every household's spending reaches named sellers; every unit of income came from a
  named payer.

**Measures**

- **HH.16 MEASURE** — The marginal propensity to consume differs across the wealth and liquidity
  distribution; the saving rate, wealth and income distributions, household leverage and debt-service
  burden; defaults by household type — all reads.
- **HH.17 MEASURE** — How household defaults and aggregate spending relate to the spread of household incomes,
  their mean held as a control, across the run's regions and years. Its statistic, the region-years it reads and its
  estimator are fixed in the measurement record before it is first read, as N3's are; the relationship is reported
  with its interval, and a sign is expected only where a cited benchmark states one.

**Forbids**

- **HH.18 FORBID** — No representative household; no consumption function applied to an aggregate; no
  decision evaluated at an average (Law 11).
- **HH.19 FORBID** — No household as a residual holder of what nobody else took: it holds only what it chose
  to buy.

**Primitives**

- **HH.20 PRIMITIVE** — Preference distributions (PREFERENCE, estimated from data where possible); decision
  schedules (PREFERENCE); minimum consumption needs by household composition (TECHNOLOGY of living); personal
  insolvency law (POLICY).

**Out of scope**

- Durable goods other than dwellings and vehicles are consumed when bought; household production (cooking,
  childcare) is not modelled beyond the value of leisure.

**Done when**

- Households spend, work, save, borrow, move and default from their own states; the consumption response to
  a transfer depends on who receives it; a saver switches from deposits to a money fund when the gap is wide
  enough for it.

---

# PART E — PRODUCTION

How goods and services are made, who makes them, how the capital behind them is built and worn, and how
the ways of making things improve.

---

## E1. TEC — Technology and innovation

**Purpose.** The ways things are made, and how better ways are discovered and spread, so that productivity
and growth are outcomes.

**Depends on:** NUM, CHN, GEO, VAL, MKT, LAB, FRM, STA.

**State**

- **TEC.1 STATE** — A **product** is a good (storable or perishable, with a physical unit) or a **service**
  (produced and consumed at once, measured in units of delivery: hours of care, trips, meals, transactions),
  each with a declared industry.
- **TEC.2 STATE** — A **way** of making a product states, per unit of output: the **inputs** it consumes (in
  physical units of named products), the **labour** by occupation family and skill, the **capital services**
  by kind of plant, the **land** or **deposit** it needs if any, its **lead time**, its **batch**, its
  **yield** (the share of what is started that is finished), and its by-products and wastes. A way is
  fixed-proportion: within one way, inputs do not substitute.
- **TEC.3 STATE** — **Substitution happens between ways**: a product can have several ways, differing in the
  mix of inputs, labour and capital, and a firm running a plant chooses among the ways that plant supports
  by their cost at the prices it faces (FRM.6). A new way is adopted by investment when it needs different
  plant.
- **TEC.4 STATE** — Each firm holds the **ways it knows**. Every firm of an industry knows its **public ways**: the
  ways no patent covers and no firm keeps to itself — the industry's standard ways and ways whose patents have
  expired. Knowing any other way — a discovered improvement, patented or kept private — is a firm's asset: it can be
  discovered, licensed, imitated, or lost when a firm dies without a successor (decision 42).
- **TEC.15 STATE** — A **patent** is a holding of the firm it is granted to: the way it covers, its grant date and
  its expiry. The country's **patent office**, a public agency, grants it on the firm's application and the fee
  paid to the office, and publishes it in its register (OBS.1). Until it expires, no other firm may run the way
  except under licence (TEC.6).

**Decisions**

- **TEC.14 DECISION** — **Research, imitation and licensing.** A firm decides its research and imitation effort,
  whether to patent a discovery, and whether to license a way, from its own value of a better way (the saving it
  expects on its own sales outlook over its horizon), its own outlook of discoveries and imitations per unit of effort
  from the published research and patent statistics (STA.1) and the patent register, the effort's cost at the wages
  and prices it faces and its marginal cost of funds, the patent fee, and the licence fees it expects; a licensor
  quotes from its own saving and the margin it would lose; never a research intensity.

**Processes**

- **TEC.5 PROCESS** — **Research**: a firm that spends on research (employing researchers and buying
  services) faces a **discovery hazard** rising with that effort (CHN); a discovery is a new way, or an
  improvement to one it knows, drawn from a declared distribution of improvements around its current
  best. Discovery is dated, owned and uncertain.
- **TEC.6 PROCESS** — **Imitation and diffusion**: a firm can learn a way another firm uses, by licensing it
  (a contract with a price, MKT.7) or by imitation effort facing its own hazard, which rises with how close
  and how visible the other firm is. Patents are a declared POLICY that sets how long imitation is barred.
- **TEC.7 PROCESS** — **Learning by doing**: the labour a way needs per unit falls with the firm's
  cumulative output on it, by a declared curve; for a small firm's agent, cumulative output is a position (REP.20).
- **TEC.8 PROCESS** — **Obsolescence**: a way stays usable, but a firm stuck with an old way loses sales to
  firms with better ones through prices, not through a rule.

**Invariants**

- **TEC.9 INVARIANT** — Every unit of output was made by a known way, from inputs actually consumed.

**Measures**

- **TEC.10 MEASURE** — Productivity by firm and industry, its dispersion, its growth, the diffusion lag of new
  ways, and research spending against growth — all reads.

**Forbids**

- **TEC.11 FORBID** — No exogenous productivity or growth path; no technology improvement that nobody
  discovered, paid for or imitated.
- **TEC.12 FORBID** — No recipe expressed as a value share: a way is in physical units, so a price change
  never silently changes a physical draw.

**Primitives**

- **TEC.13 PRIMITIVE** — The opening ways per product (TECHNOLOGY, from input–output and engineering data);
  the discovery and imitation hazards and the distribution of improvements (TECHNOLOGY); learning curves
  (TECHNOLOGY); patent law: its life, fees and what it covers (POLICY).

**Out of scope**

- Quality as a continuous attribute of a product: a better product is a new product with its own ways.

**Done when**

- A firm can switch ways when relative prices change; research spending produces dated discoveries; a
  productive way spreads across firms over time; the run grows through discovered improvements.

---

## E2. FRM — Firms

**Purpose.** The firm as a named party that buys, makes, sells, employs, invests, borrows, pays out, belongs
to owners, can belong to a group, and can be born and die.

**Depends on:** TEC, ACC, MKT, VAL.

**State**

- **FRM.1 STATE** — A firm has owners (holders of its shares, or a sole owner household), sites, plant,
  stocks of inputs, work in progress and output, the ways it knows, its contracts (employment, supply,
  credit, leases), its accounts, and a management whose preferences (patience, risk aversion, growth
  appetite) are drawn at founding.
- **FRM.2 STATE** — Firms differ in size, productivity, cost, leverage, age and location, and the
  differences are the reason they compete.
- **FRM.23 STATE** — **Small firms are agents** (REP.1): each holds its zone, ways, form, productivity and
  management types, posted price and wage offers, headcount by occupation family and plant units as attributes
  (REP.41); cash, debt, inventory and cumulative output as positions (REP.20); employment, ownership, loans and
  invoices as lines. A firm ranked among its kind's individuals at the opening, or with a public instrument, is an
  individual (REP.2). An
  **unincorporated** business (a sole trader or a partnership, PTY.4) is not a separate party: its stock, plant,
  receivables and debts are its owners' household's, as the law of such businesses has it, and a household running
  one is of a kind whose attributes and positions add the business's ways, posted price, plant, stock and sales.
- **FRM.3 STATE** — A firm may be a **parent or subsidiary** in a group: it controls another through a
  majority of its votes; intra-group loans, sales and guarantees are real contracts; each member keeps
  limited liability unless it has guaranteed another's debts.

**Decisions** — each from the firm's own state, outlook and the prices it faces:

- **FRM.4 DECISION** — **What to produce and how much**: expected demand, stock on hand, capacity, inputs
  available, labour available, the margin at expected prices and what financing the work in progress and stock
  costs it at the margin. The quantity is the outcome.
- **FRM.5 DECISION** — **What to charge**: a firm selling by posted price revises its price on its own
  schedule from its costs, its stock, how fast it has been selling, what it sees competitors charge and its
  own outlook; it prices above unit cost when it can and below when it must clear stock.
- **FRM.6 DECISION** — **Which way to run**, among the ways its plant supports, at the input and labour prices
  it faces.
- **FRM.7 DECISION** — **Whom to employ** and at what wage (LAB); **what to buy**, from whom and on what terms
  (GDS, TCR).
- **FRM.8 DECISION** — **Whether to invest**, in what, and how to fund it (CAP).
- **FRM.9 DECISION** — **How to fund itself**: retained cash, trade credit, a bank loan, a bond, commercial
  paper, new shares; by what each costs it now and how close it is to the leverage its management will
  tolerate.
- **FRM.10 DECISION** — **What to pay out**: dividends and buybacks from cash after what is due, by its
  management's policy and what the owners expect.
- **FRM.11 DECISION** — **Which lines to be in**: enter a product by investing in the plant and knowing a way
  (CAP), and exit one it cannot make pay.
- **FRM.12 DECISION** — **What to do in distress**: cut costs and staff, sell assets, draw lines, raise
  expensive money, ask suppliers for time, negotiate with lenders, seek a buyer.

**Processes**

- **FRM.13 PROCESS** — **Revenue** is quantity sold times price achieved, recognised on delivery, from named
  buyers. **Costs** are named lines with named payees: inputs, wages, energy, rent, services, interest,
  taxes, depreciation. **Operating profit** is a read, and can be negative.
- **FRM.14 PROCESS** — **Unit cost** is the inputs a batch consumed at their own cost, plus its labour, plus
  the capital charge its plant's period carries, over the units that survive. A line run below its rate
  carries its plant's whole cost over fewer units; a line that ran nothing charges its plant's cost to the
  period.
- **FRM.15 PROCESS** — **Two ways to fail**: a firm that cannot pay something due is in **default of
  payment**; a firm whose liabilities exceed its assets is **balance-sheet insolvent**. Either can come
  without the other. What follows is the insolvency law of its country (POLICY): a **restructuring** in
  which creditors decide, or a **liquidation** into an estate (L3).
- **FRM.16 PROCESS** — **Birth**: a firm is founded by a named founder (a household, a firm, a fund) with
  money from named accounts, when the founder expects the venture to pay; it starts small, buys its plant
  from producers, and is fragile.

**Invariants**

- **FRM.17 INVARIANT** — Every revenue is somebody's outlay and every cost somebody's income.
- **FRM.18 INVARIANT** — Receivables and payables equal the sum of the actual invoices.

**Measures**

- **FRM.19 MEASURE** — Firm size, growth, age and productivity distributions; entry and exit rates by
  industry and age; margins and their cyclicality; the share of firms in distress; markups.

**Forbids**

- **FRM.20 FORBID** — No revenue without a buyer; no cost without a payee; no earnings path; no cost line
  set as a share of revenue.
- **FRM.21 FORBID** — No firm that cannot run out of cash; no firm born with an endowment from nowhere; no
  birth rate; no constant population of firms.

**Primitives**

- **FRM.22 PRIMITIVE** — Management preference distributions (PREFERENCE); decision schedules (PREFERENCE);
  insolvency law (POLICY); the opening population of firms (ENDOWMENT).

**Out of scope**

- Internal organisation, principal–agent problems inside a firm beyond management preferences, and
  strategic interaction beyond what posted prices and markets carry.

**Done when**

- Firms are born, grow, shrink and die from their own cash and solvency; their prices and quantities move
  with their costs and demand; the size distribution of firms is an outcome.

---

## E3. CAP — Capital, investment and construction

**Purpose.** Plant, vehicles, dwellings and infrastructure: how they are decided on, built, paid for,
worn, maintained, repaired, sold and scrapped, so that investment is where finance reaches the real economy.

**Depends on:** FRM, TEC, GEO.

**State**

- **CAP.1 STATE** — A **capital good** is a unit with a kind, a site, a service date, a capacity, a condition
  and a remaining life; an agent's plant is counted by zone and class (REP.24). Kinds differ, and a use needing
  several kinds is limited by the scarcest.
- **CAP.2 STATE** — A **construction project** has an owner, a site, a builder, a budget, a schedule, and
  work done to date; while under construction it is an asset of its owner at cost.

**Decisions**

- **CAP.3 DECISION** — A firm **invests** when its own value of the project (VAL.8) — the output it expects
  to sell over the plant's life, at the prices it expects, less running costs — beats what the money costs
  it **at the margin now** (its borrowing rate for new debt and its shareholders' required return), by at
  least its management's own hurdle, and when it can fund it. A firm running full has a reason to expand;
  uncertainty about demand is a reason to wait.
- **CAP.4 DECISION** — An owner **maintains, repairs, sells or scraps** a unit when doing so is worth more to
  it than keeping it as it is.

**Processes**

- **CAP.5 PROCESS** — Investment is a **purchase from a named producer** of capital goods, paid in stages,
  built over a lead time, and in service only when complete: demand now, capacity later.
- **CAP.6 PROCESS** — **Wear**: a unit depreciates by its use and age, one schedule charged both to income
  and to the unit; it can fail (CHN), be damaged by a hazard (GEO.8), or be repaired.
- **CAP.7 PROCESS** — **Infrastructure** — roads, ports, power, water — is built the same way, by a public
  or private owner, and charges its users where its owner chooses to (tolls, fees, taxes).

**Invariants**

- **CAP.8 INVARIANT** — Per owner and kind: capital next day equals capital today plus completions minus
  retirements, scrappings and destructions plus transfers in minus transfers out.
- **CAP.9 INVARIANT** — No output exceeds the capacity of the plant, labour and inputs that made it.

**Measures**

- **CAP.10 MEASURE** — Investment's share and volatility relative to output; its response to borrowing costs
  and to capacity utilisation; the age of the capital stock.

**Forbids**

- **CAP.11 FORBID** — No investment rate: no fixed share of profit or output reinvested.
- **CAP.12 FORBID** — No capacity from nowhere, and no plant that moves when its owner changes.

**Primitives**

- **CAP.13 PRIMITIVE** — Capital kinds, lives, lead times and wear curves (TECHNOLOGY); hurdle and horizon
  distributions of managements (PREFERENCE).

**Out of scope**

- Nothing beyond Appendix D.

**Done when**

- A tightening of credit reduces investment through the firms' own comparisons and funding, and output
  follows with the build lag; capital goods are real units sold into real markets when a firm dies.

---

# PART F — REAL MARKETS

Where goods, services, carriage, labour, dwellings and land change hands, and where trade credit binds
buyers and sellers together.

---

## F1. GDS — Goods and commodities

**Purpose.** Physical goods and raw commodities: made or extracted at a place, stored, carried, sold,
consumed, spoiled.

**Depends on:** FRM, TEC, MKT, GEO, HSG.

**State**

- **GDS.1 STATE** — A good is keyed by **grade and place**: the same grade in two places is two things with
  two prices; the gap between them is a read (FRT.10), an outcome of shippers who carry goods when it exceeds what
  carrying costs (FRT.5).
- **GDS.2 STATE** — Stocks are holdings of lots at a site, each with its cost (ACC.6); goods in transit are
  their owner's, pledged to the carrier until they arrive (FRT).
- **GDS.3 STATE** — A **commodity** is a standardised good extracted from a deposit (GEO.6) or grown on land the
  grower owns or leases (HSG.19), whose grade is the deposit's or the land's. The **right to extract** is a holding
  over a named tile.

**Decisions**

- **GDS.4 DECISION** — **Producers** decide output (FRM.4); an extractor decides how much of its deposit to
  work, weighing today's price against its outlook (depletion makes waiting a real choice).
- **GDS.5 DECISION** — **Buyers** buy inputs from their own needs for planned production and the stock they
  want to hold, up to what the input is worth to them in use (VAL.8), counting what financing that stock costs
  them at the margin.
- **GDS.6 DECISION** — **Stockists and merchants** buy, hold and sell for profit, pay for storage, bear
  spoilage, pay for the money tied up in stock at their own marginal cost of funds, and move goods where they
  fetch more (FRT).

**Processes**

- **GDS.7 PROCESS** — **Standardised commodities** trade between firms in call auctions or dealer markets at each
  place (MKT.3, MKT.5); **other goods between firms** are sold at posted list prices and under bilateral supply
  contracts with terms, lead times and volumes (MKT.6, MKT.7); **retail** reaches households through distributors
  (SRV).
- **GDS.8 PROCESS** — **Spoilage and shrinkage** remove units without a sale at their own cost; **storage**
  is a service bought from whoever owns the room. The two are different things and never one number.
- **GDS.9 PROCESS** — Weather and catastrophes (CHN, GEO.8) destroy crops and stocks at named places, which is
  what a supply shock is.

**Invariants**

- **GDS.10 INVARIANT** — Per good and place: opening stock plus produced plus arrived equals consumed plus
  shipped out plus spoiled plus destroyed plus closing stock.

**Measures**

- **GDS.11 MEASURE** — Price volatility rises as stocks fall; the location basis between two places tracks
  the freight between them; commodity shocks reach producer prices before consumer prices.

**Forbids**

- **GDS.12 FORBID** — No consumption without production or stock; no negative inventory; no commodity price
  from a written path; no commodity extracted where there is no deposit.

**Primitives**

- **GDS.13 PRIMITIVE** — Grades, spoilage rates, storage technology (TECHNOLOGY); deposits (ENDOWMENT).

**Out of scope**

- Variation of quality within a grade.

**Done when**

- Every good's stock reconciles by place every day; a drought at a place raises the price there first and
  elsewhere as merchants ship.

---

## F2. SRV — Services and distribution

**Purpose.** Most of a real economy's output: non-storable services, and the distribution chain that
carries goods from factory gates to households and adds a margin on the way.

**Depends on:** FRM, TEC, MKT, LAB.

**State**

- **SRV.1 STATE** — A **service provider** has capacity per day (staff hours, seats, beds, vehicles), a
  region it serves and a posted price. Capacity not used on the day **is lost**.
- **SRV.2 STATE** — A **distributor** (wholesaler, retailer) holds goods bought at wholesale in its outlets,
  and sells them at posted retail prices; its margin pays its staff, rent, storage, spoilage and capital.

**Decisions**

- **SRV.3 DECISION** — A provider sets its price and staffing from its bookings, its costs and its outlook;
  a distributor sets its retail prices and stock levels from its wholesale costs, its sales and its
  competitors' prices.
- **SRV.4 DECISION** — A household chooses among the providers and outlets it can reach, by price, distance
  and whether they can serve it today.

**Processes**

- **SRV.5 PROCESS** — Services are sold by **posted price with rationing by capacity**: a provider that is
  full turns buyers away or makes them wait, and they go elsewhere or go without.
- **SRV.6 PROCESS** — The **retail price** a household pays includes the distributor's margin, the freight to
  the outlet, and consumption tax; the **factory-gate price** does not. Producer and consumer prices are
  therefore different prices of different transactions.

**Measures**

- **SRV.7 MEASURE** — The share of services in output and employment; the retail margin and its movement in a
  cost shock (margin compression); the frequency and size of retail price changes.

**Forbids**

- **SRV.8 FORBID** — No service stored; no retail price that is a factory price with a stated markup
  applied; no household buying at the factory gate unless it goes there.

**Primitives**

- **SRV.9 PRIMITIVE** — Service technologies (TECHNOLOGY); the reach of a household's shopping by distance
  (TECHNOLOGY of travel).

**Out of scope**

- Online selling as a separate channel: a seller's reach is declared by its technology (SRV.9).

**Done when**

- Services are produced and consumed the same day; unused capacity perishes; retail margins move when
  wholesale costs change faster than retail prices.

---

## F3. FRT — Freight and logistics

**Purpose.** Moving goods between places costs time and money and needs a vehicle, a route and capacity;
this is what makes location matter.

**Depends on:** GEO, CAP, MKT, SET.

**State**

- **FRT.1 STATE** — A **vehicle** is a capital unit with a site, a capacity, a speed, a running cost per voyage and
  a keeping cost per day; vehicles held by agents are counted by zone and class (REP.24). Its room is available only
  on routes starting where it is.
- **FRT.2 STATE** — A **route** is a path over real network segments between two sites, with a mode
  (road, rail, sea, pipeline) and every transfer between modes.
- **FRT.3 STATE** — A **shipment** is named goods owned by a named party, aboard a named vehicle, on a
  route, with a promised and an actual arrival; it shares its vehicle's fate.

**Decisions**

- **FRT.4 DECISION** — A **carrier** offers room on routes from where its vehicles stand, priced from what
  running them costs and what it expects to fill, and decides where to reposition empty vehicles.
- **FRT.5 DECISION** — A **shipper** — the seller who sells delivered, or the buyer who buys at origin —
  books room when the price gap between two places exceeds what the room costs, and otherwise holds, sells
  locally or does not trade.

**Processes**

- **FRT.6 PROCESS** — Carriage is sold on each route's market (dealer or posted price) and paid to the
  carrier; the goods are pledged to the carrier while in transit, cannot be used or sold, and are released
  on arrival. Freight enters the delivered price the buyer pays.
- **FRT.7 PROCESS** — A closure, a catastrophe or a breakdown removes real capacity: shipments are refused,
  delayed or rerouted, never repriced by a multiplier.
- **FRT.8 PROCESS** — If a carrier fails with cargo aboard, the goods remain the owner's and are recovered
  from the carrier's estate after a delay and at a cost; lost cargo is the carrier's liability as a claim
  in its estate.

**Invariants**

- **FRT.9 INVARIANT** — Every shipment has one owner, one carrier and one vehicle; no vehicle is in two places;
  no route carries more than its segments' capacity.

**Measures**

- **FRT.10 MEASURE** — Freight rates are inelastic in the short run and spike when demand meets fixed
  capacity; price gaps between places track freight costs and widen when capacity binds.

**Forbids**

- **FRT.11 FORBID** — No instantaneous or costless transport; no room without a vehicle; no goods in transit
  owned by nobody; no location gap closed by formula.

**Primitives**

- **FRT.12 PRIMITIVE** — Vehicle technologies, speeds and running costs (TECHNOLOGY); loading times
  (TECHNOLOGY).

**Out of scope**

- Passenger transport as its own market: travel is a service (SRV).

**Done when**

- Goods move between places only on booked vehicles over real routes; a blocked route widens the price gap
  and strands goods.

---

## F4. LAB — Labour

**Purpose.** People's time, sold to employers by people who search, choose and quit, for wages set by
employers who post vacancies and compete for workers — so that employment, unemployment, vacancies and
wages are outcomes.

**Depends on:** POP, HH, FRM, MKT.

**State**

- **LAB.1 STATE** — An **employment contract** is a row: employer, employee (a person), occupation family, hours,
  wage, start date, notice and severance terms, and the pension it earns (PEN): its kind and contribution rates
  are terms of the contract, and the scheme the employee belongs to is part of its attachment (REP.23). Contracts
  of one occupation family, skill level, wage offer, hours, notice and severance terms, pension kind and rates,
  begun in one start band, in one region, form a line (REP.3) whose sides are the
  employers and the households' adult roles, each with its count; who works for whom is drawn when it matters
  (REP.23). The wage bill, headcount, unemployment and flows between states are reads of these rows.
- **LAB.2 STATE** — A **vacancy** is an employer's posted offer: occupation family, skill required, hours, wage,
  notice and severance terms, region; it is open until filled or withdrawn.
- **LAB.3 STATE** — Labour is **heterogeneous** by occupation, skill and region, and a job in one is not a
  job in another.

**Decisions**

- **LAB.4 DECISION** — An **employer** posts vacancies when an extra worker is worth more to it than the wage
  (its output price, its capacity, its outlook, and what financing the wage bill before sales pay for it costs),
  choosing the wage it offers from what it has been able to
  fill before, and **lays off** when it is sure it cannot use the work, paying what the contract owes.
- **LAB.5 DECISION** — A **searcher** decides how many of the vacancies it can see (within its region and search
  reach) to apply to, from what each application costs it and what it expects a job to be worth, and **accepts**
  the best offer that beats its own reservation — built from its outside option (benefits, other household income),
  its value of leisure and its outlook. It may also move region (POP.8) or retrain (POP.18).
- **LAB.6 DECISION** — An **employee** quits for a better offer, retires, or reduces hours when that is better
  for its household.
- **LAB.7 DECISION** — An employer **selects** among its applicants by skill and experience; a worker who is
  not chosen keeps searching.
- **LAB.17 DECISION** — **Renegotiation.** At a contract's review date the employer offers terms from its output
  price and outlook, its profitability and how hard the job has been to fill; the employee (or its union) accepts,
  counters or quits from its own price outlook, the vacancies it can see and its reservation; a union reads its
  members' outlooks and the employer's published results. Nothing else moves a contract's wage.

**Processes**

- **LAB.8 PROCESS** — **Search is individual**: each application is a real meeting between a searcher and a
  named vacancy, arriving through a declared search process (CHN), and the number of matches is the sum of individual
  acceptances — never an aggregate matching function. A searcher sends its applications in rounds over its occasions, to
  the vacancies it can reach that its own tastes choose (REP.22); the employer chooses among its applicants by skill and
  experience, and among equals by lot; the searcher offered accepts if the offer, with the match's quality drawn as its
  taste, beats its reservation, and those not chosen apply again.
- **LAB.9 PROCESS** — **Wages are sticky because they are contracts**: an existing contract's wage changes only
  when renegotiated at its review date, by agreement or by collective agreement (LAB.10), or when the worker
  leaves. Adjustment in a downturn therefore falls first on hiring and layoffs.
- **LAB.10 PROCESS** — **Collective bargaining**: where a union represents workers of an occupation at an
  employer or industry, wage terms are negotiated between named parties and bind the contracts they cover
  until the next round; a strike is a real stoppage with real losses on both sides.
- **LAB.11 PROCESS** — A **minimum wage** (POLICY) is a declared limit on the contracts an employer may offer;
  a job worth less than it to the employer is not offered.
- **LAB.12 PROCESS** — A firm that fails releases its workers through the same separation path, with what the
  contract and insolvency law give them.

**Invariants**

- **LAB.13 INVARIANT** — No person holds more hours of employment than the day has; no employer pays wages
  to nobody; headcount is the count of contracts.

**Measures**

- **LAB.14 MEASURE** — Unemployment and vacancies move against each other over the cycle (the Beveridge
  curve); employment moves with output (Okun); unemployment durations, wage dispersion within occupations,
  job-to-job flows, and the wage response to slack are reads.

**Forbids**

- **LAB.15 FORBID** — No employment without an employer; no exogenous unemployment rate; no aggregate
  matching function; no second channel from prices to wages beside the contracts and their renegotiation.

**Primitives**

- **LAB.16 PRIMITIVE** — What an application costs and the reach of search by distance (TECHNOLOGY); the meeting
  hazard per application (TECHNOLOGY); notice, severance and minimum-wage law (POLICY); union coverage (ENDOWMENT).

**Out of scope**

- Discrimination, informal work, and self-employment beyond sole-owner firms.

**Done when**

- Unemployment, vacancies and wages are reads of individual contracts, applications and offers; a
  downturn raises unemployment through layoffs and fewer vacancies before wages move.

---

## F5. HSG — Housing and land

**Purpose.** Dwellings and land: an asset and a service at once, fixed in place, slow to build, bought
mostly with borrowed money, which makes housing the largest household asset, the largest household
liability and a main channel of monetary policy.

**Depends on:** HH, GEO, CAP, BNK (mortgages).

**State**

- **HSG.1 STATE** — A **dwelling** is a unit on a site with a size, a quality and a condition, owned by a named
  party, occupied by one household, which may let rooms to lodger households under tenancies. Dwellings held by
  agents are counted by zone and class, and each unit's tile is a pairing drawn when needed (REP.24).
- **HSG.2 STATE** — A **tenancy** is a contract between an owner and a household: rent, term, notice, deposit.
  Tenancies of one class, rent and term in one zone form a line between landlords and tenants (REP.3).
- **HSG.3 STATE** — **Land** is owned, zoned (POLICY: what may be built on it) and traded; its price is formed
  in its own market.
- **HSG.19 STATE** — A **land lease** is a contract: owner, tenant, parcel, rent, term; agricultural, commercial and
  industrial land is often leased rather than owned.

**Decisions**

- **HSG.4 DECISION** — A household decides to **rent or buy**, and what and where (HH.8).
- **HSG.5 DECISION** — An **owner who sells** sets its asking price from its own outlook, what it owes on the
  dwelling, and how urgently it must sell; a distressed or forced seller asks less. There is no floor.
- **HSG.6 DECISION** — A **buyer** bids up to its own value of the dwelling (the rent it saves, the price it
  expects), limited by its deposit and what a lender will lend it.
- **HSG.7 DECISION** — A **landlord** sets rents from its costs, vacancies and outlook, and buys or sells
  dwellings as investments.
- **HSG.8 DECISION** — A **builder** buys land, obtains permission and builds when the expected sale price
  exceeds land, construction and financing cost; completion takes time.
- **HSG.9 DECISION** — A **lender** sets its mortgage standards (loan-to-value, income multiple, rate) from its
  own book, its funding and its outlook, and tightens when worried (BNK).
- **HSG.18 DECISION** — A **landowner** sells, leases or holds its land from its own value of it — what it can
  earn, what it expects the land to fetch — and a buyer or tenant (a farmer, a builder, a firm, a public agency)
  bids up to what the land is worth to it for its own use.

**Processes**

- **HSG.10 PROCESS** — **Sales are matched by search and negotiation**: a listed dwelling meets buyers
  through a declared search process (CHN); an offer is accepted, countered or refused; a listing that finds no
  buyer stays listed or is withdrawn — so in a falling market volumes fall before prices.
- **HSG.11 PROCESS** — **Foreclosure** moves a dwelling from a defaulting owner to the lender or its agent,
  who sells it; the forced sale adds supply, which pushes prices down further.
- **HSG.12 PROCESS** — Dwellings depreciate, need maintenance, and can be destroyed by hazards (GEO.8).
- **HSG.20 PROCESS** — Land changes hands by negotiation or auction per region (MKT.7, MKT.3); its price is a read
  of those sales and, where none happened, is absent; a tax or a lender that needs a figure reads a valuation
  (MKT.20). Rezoning a parcel (POLICY) changes what it is worth to
  bidders, not its price directly.

**Invariants**

- **HSG.13 INVARIANT** — Every dwelling has one owner and at most one occupying household besides its lodgers
  under tenancies; every household is housed or recorded homeless.
- **HSG.14 INVARIANT** — Mortgage debt owed by households equals the contractual amount of mortgages held by
  their holders, whoever they are.

**Measures**

- **HSG.15 MEASURE** — House prices, rents and their ratio; transactions and time on market; the response of
  prices to mortgage rates and standards; boom–bust cycles with volumes leading prices.

**Forbids**

- **HSG.16 FORBID** — No house price path; no reservation floor; no mortgage without a lender's balance sheet
  on the other side; no dwelling without an owner.

**Primitives**

- **HSG.17 PRIMITIVE** — Construction technology and lead times (TECHNOLOGY); zoning and property law
  (POLICY); the opening housing stock and land ownership (ENDOWMENT).

**Out of scope**

- Commercial property as its own market: firms' premises are plant (CAP) on land (HSG.18–HSG.20).

**Done when**

- House prices and rents are outcomes of individual sales and tenancies; a rate rise reaches house prices
  through what buyers can borrow, and reaches spending through floating mortgage payments.

---

## F6. TCR — Trade credit

**Purpose.** Credit extended by sellers to buyers, the first and cheapest credit most firms use, and a
contagion path that runs along the supply chain rather than through banks.

**Depends on:** FRM, ACC, MKT.

**State**

- **TCR.1 STATE** — An **invoice** is a contract: seller, buyer, amount, due date, early-payment discount. It is
  the seller's receivable and the buyer's payable, one record. Invoices on one market's terms that fall due together
  form a line between its sellers and buyers (REP.3) — one day's, or, where the trade's customary terms bill by
  statement (TCR.8), one statement period's, due at its end plus the term; a buyer's default lands on sellers drawn
  from it (REP.23).

**Decisions**

- **TCR.2 DECISION** — A seller decides **whether to give terms, and how long**, per buyer, from what it has
  seen of that buyer, and tightens when worried.
- **TCR.3 DECISION** — A buyer decides whether to take the discount or use the credit; a seller may sell
  its receivables to a bank (factoring, MKT.7).

**Processes**

- **TCR.4 PROCESS** — Late payment is a real state that stresses the seller's cash; a buyer's default makes
  the receivable an unsecured claim in its estate, and the seller's loss can push it into distress.

**Invariants**

- **TCR.5 INVARIANT** — Receivables equal payables, invoice by invoice and line by line, across the world.

**Measures**

- **TCR.6 MEASURE** — Days of payment, their lengthening in downturns, and chains of distress running from
  buyer to supplier.

**Forbids**

- **TCR.7 FORBID** — No sale that settles instantly by construction; no receivable that survives its debtor.

**Primitives**

- **TCR.8 PRIMITIVE** — The payment terms customary in each trade, including whether it bills per invoice or by
  statement (POLICY of the trade); each seller's own terms are its decision (TCR.2).

**Out of scope**

- Trade finance and letters of credit (Appendix D).

**Done when**

- A buyer's failure propagates losses to its suppliers by name; suppliers withdraw terms from buyers they
  doubt.

---

## F7. ENE — Energy

**Purpose.** Electricity, which cannot be stored at scale and must be produced the moment it is used, and the
fuels that feed it and almost every other process — so that energy shocks are real losses of supply that travel
through costs to prices.

**Depends on:** GDS, CAP, GEO, CHN, MKT, FRT.

**State**

- **ENE.1 STATE** — A **power plant** is an individual capital unit (CAP) with a technology — fuel-fired, hydro,
  nuclear, wind, solar, or storage — a capacity, an efficiency, the fuel it burns, how fast it can change output,
  and a running cost. Wind, solar and hydro output depend on the day's weather (CHN.3); storage buys power when
  cheap and sells it when dear, within its capacity and losses.
- **ENE.2 STATE** — The **grid** is infrastructure (GEO.4): transmission lines between regions, each with a
  capacity, and distribution to consumers. A named **system operator** per country runs it, balances supply and
  demand, and settles imbalances.
- **ENE.3 STATE** — **Fuels** — coal, oil, gas, uranium — are commodities (GDS.3): extracted from deposits,
  stored, carried by pipeline, ship or rail, and traded like any good.
- **ENE.4 STATE** — Energy is an **input in every way** that uses it (TEC.2), in physical units, and a household
  consumption good; heating and cooling demand rise with the day's weather.

**Decisions**

- **ENE.5 DECISION** — A **generator** offers its available capacity into each day's wholesale market at prices
  from its own running costs, its fuel outlook and its contracts; an inflexible or subsidised plant may offer
  below zero rather than stop.
- **ENE.6 DECISION** — A **retail supplier** buys wholesale and sells to households and firms at posted tariffs,
  fixed or variable, bearing the difference; a **large consumer** buys wholesale directly or under a contract.
- **ENE.7 DECISION** — An **investor** builds a plant, a line or storage when its own value of it beats what the
  money costs it (CAP.3), under the country's energy policy.

**Processes**

- **ENE.8 PROCESS** — **The wholesale market** meets **every day**, for delivery the next day, as one call auction
  (MKT.3) across all regions of a grid, in declared **blocks** (peak and off-peak are two products delivered on the
  same day), finding a price per region subject to the lines' capacities: where no line binds, regions share a
  price, and a congested line leaves two regions with two prices. The order in which plants run is an outcome of
  their offers; the price is negative when output that cannot stop exceeds demand. Trades made on non-business days
  settle on the next business day.
- **ENE.9 PROCESS** — **Shortage**: when supply cannot meet demand, the system operator sheds load by the
  country's declared rule (POLICY); the named consumers cut off lose the output that needed the power, which is a
  real, dated event.
- **ENE.10 PROCESS** — **Imbalance**: what a supplier or generator delivered or took beyond its position is settled
  with the system operator at the day's imbalance price.

**Invariants**

- **ENE.11 INVARIANT** — Per region and day, electricity produced plus imported equals electricity consumed plus
  exported plus lost plus stored; no line carries more than its capacity.

**Measures**

- **ENE.12 MEASURE** — Price spikes when spare capacity is thin; renewable output lowers and destabilises prices;
  fuel and power shocks reach producer prices before consumer prices (L9).

**Forbids**

- **ENE.13 FORBID** — No electricity stored outside storage plants; no energy price from a path or formula; no
  outage without a named cause; no grid without an owner.

**Primitives**

- **ENE.14 PRIMITIVE** — Plant technologies, efficiencies and ramp limits (TECHNOLOGY); each region's climate
  (ENDOWMENT); market design, load-shedding rules and energy policy (POLICY); the opening fleet and grid
  (ENDOWMENT).

**Out of scope**

- District heating and water supply.

**Done when**

- Power is produced by named plants, priced daily per region, carried over a grid with limits, and rationed with
  named losses in a shortage; a fuel shock reaches consumer prices through producers' costs.

---

# PART G — BANKING AND CREDIT

Banks create most of the money, lend it, fund themselves in the money markets, hold capital against loss,
and can fail in two different ways. A bank is three concerns that can fail independently — its lending, its
funding, its capital — and this part treats them in that order.

---

## G1. BNK — Bank lending

**Purpose.** Loans as named contracts, written by a bank that creates the deposit it lends, priced from its
own economics, refused when it judges the risk not worth it, and carried, provisioned, worked out and
written off as events.

**Depends on:** MON, ACC, MKT, VAL.

**State**

- **BNK.1 STATE** — A **loan** is a contract between a named lender and a named borrower: principal, currency,
  rate (fixed, or a margin over a named transacted reference), amortisation schedule, maturity, collateral if
  secured, covenants, and a status (performing, in arrears, impaired, in workout, written off).
- **BNK.2 STATE** — Loan types are declared data (Law 10): corporate term loans and revolving facilities,
  small-business loans, mortgages, consumer loans and credit lines, loans to other banks. A **facility**
  is a commitment the borrower may draw; its undrawn part is a real obligation that consumes the bank's
  capital and liquidity and earns a commitment fee.
- **BNK.3 STATE** — A **syndicated loan** is one loan with several lenders of record, each holding its share
  against its own capital and limits, arranged by a lead that takes a fee.
- **BNK.17 STATE** — A **term loan** is disbursed once (or in stated tranches for a construction or investment
  programme) and repaid on a stated profile: **amortising** in equal instalments, **bullet** at maturity, or
  **balloon** (partly amortising, the rest at maturity), with any grace period on principal. Its terms state
  whether it may be **prepaid** and at what fee, its rate reset dates if floating, and its covenants. Term loans
  are written to firms of every size (investment, acquisitions, refinancing), to households (vehicles and other
  consumer purposes) and, as mortgages, against dwellings; to an agent they are a line (REP.3).
- **BNK.18 STATE** — A **revolving facility** or **credit line** (to a firm, or a household's credit card or
  overdraft) is drawn and repaid at the borrower's choice up to a limit; the lender can cut the undrawn limit
  where the contract allows, which is how credit tightens for borrowers who already have lines.
- **BNK.21 STATE** — A **credit bureau** is a named party to which lenders report each borrower's contracts,
  payments and defaults, as its law requires (POLICY); a lender to whom a borrower applies buys its record, and
  its assessment (BNK.4) reads it. For an agent, the record is its credit-record stage (REP.41).
- **BNK.22 STATE** — **Non-bank lenders** — finance companies funded by bonds, paper and bank lines, and private
  credit funds (FND) — write loans under the same contracts and decisions (BNK.4–BNK.7) without taking deposits, and
  fail when their funding does.

**Decisions**

- **BNK.4 DECISION** — A bank **quotes** a borrower a rate built from: its own marginal cost of funds (BFL),
  its own assessment of the borrower's default probability and loss given default, the capital the loan
  consumes times the return it requires on that capital, and its cost of making the loan. Its assessment is
  its own opinion, formed from what it has seen of the borrower.
- **BNK.5 DECISION** — A bank **declines** when the loan is not worth it, when the borrower fails its
  standards (loan-to-value, income multiple, coverage), or when its own capital or liquidity will not carry
  it; it **tightens** its standards when its own losses, funding or outlook worsen.
- **BNK.20 DECISION** — A bank assesses an applicant from what it can observe of it. For an agent, that is what it
  can observe of the agent's own state, which is each twin's: the bank offers all its twins the same terms, or
  declines them all (REP.5).
- **BNK.6 DECISION** — A **borrower shops**: it asks the lenders it can reach, takes the best quote its own
  value of the loan accepts, or goes without.
- **BNK.7 DECISION** — On arrears or a covenant breach, the bank decides to **wait, restructure, extend,
  waive for a fee, or enforce**, from what each is worth to it.

**Processes**

- **BNK.8 PROCESS** — **A bank lends by creating a deposit** for the borrower (MON.6); reserves move only when
  the borrower pays someone at another bank.
- **BNK.9 PROCESS** — A loan is carried at amortised cost with an **expected-loss provision** that moves with
  the bank's assessment; interest accrues and is received or falls into arrears; enforcement realises
  collateral for what it fetches; a **write-off** removes the loan and books the loss not already provided.
- **BNK.10 PROCESS** — A loan can be **sold** (to another bank, a fund, a securitisation vehicle) or used as
  collateral, and then it has a price and a holder.
- **BNK.19 PROCESS** — A term loan's instalments fall due on their dates and are paid from the borrower's account
  or become arrears; a borrower may **prepay** when refinancing elsewhere is worth its fee to it, which shortens the
  lender's book when rates fall; a balloon or bullet at maturity must be repaid or refinanced, and refinancing is a
  new loan priced today.

**Invariants**

- **BNK.11 INVARIANT** — A bank's loan book is the sum of its loan contracts; its change equals new lending
  minus repayments minus write-offs plus purchases minus sales.
- **BNK.12 INVARIANT** — The loss that reaches capital on a write-off equals principal minus recovery minus
  provisions already taken.

**Measures**

- **BNK.13 MEASURE** — Declined applications are visible; lending standards tighten when bank capital or
  funding weakens (the credit channel); loan rates pass policy-rate changes through incompletely and with a
  lag; which constraint binds differs by bank and by time.

**Forbids**

- **BNK.14 FORBID** — No lending "out of" deposits or reserves; no loan book that is a number rather than
  loans; no loss rate in place of a default; no fixed recovery rate.
- **BNK.15 FORBID** — No single assessment of a borrower shared by every lender: each lender holds its own
  (Law 11); within one lender, one assessment drives both the price and the provision.

**Primitives**

- **BNK.16 PRIMITIVE** — Operating cost per loan (TECHNOLOGY); required return on capital and risk appetite
  (PREFERENCE of each bank's management); credit-reporting law (POLICY, BNK.21). Each bank's standards are its
  decision (BNK.5), not a primitive.

**Out of scope**

- Lending outside interest-bearing contracts, and loans arranged through brokers.

**Done when**

- Every loan is a contract with two named sides; a bank refuses as well as lends; a borrower's default
  follows its own cash failure and is worked out or written off as dated events.

---

## G2. BFL — Bank funding and liquidity

**Purpose.** A bank's funding is a mix of liabilities that can leave at different speeds, and a bank can
fail for want of cash while still solvent.

**Depends on:** BNK, MON, MMK (money market).

**State**

- **BFL.1 STATE** — **Deposits** differ by holder: households (many, small, sticky, insured up to a limit),
  firms (operational, larger), institutions (few, large, fast). Each deposit is a contract with a rate and
  terms (sight or term).
- **BFL.2 STATE** — **Wholesale funding**: interbank loans, repo, certificates of deposit, commercial paper and
  bonds the bank issued, each maturing and needing to be rolled.
- **BFL.3 STATE** — **Liquid assets**: reserves, banknotes, and securities it can sell or pledge, each with a
  haircut and a market depth.

**Decisions**

- **BFL.4 DECISION** — A bank **sets its deposit rates** from its own need for funding and the rates its
  depositors could get elsewhere; a bank short of funding bids up.
- **BFL.5 DECISION** — A bank chooses its **liquidity buffer** from its own view of how fast its funding could
  leave (its depositor mix, its maturities) and the cost of holding liquid assets, above any regulatory
  minimum.
- **BFL.6 DECISION** — When short, a bank **borrows in the market, pledges or sells assets, bids for
  deposits, shrinks its new lending, or draws the central bank's facility** — in the order its costs and
  eligibility make best.
- **BFL.7 DECISION** — A **depositor** moves its money (to another bank, a money fund, banknotes, bills) when
  its own view of the bank worsens or a better rate appears; what it can observe is what the bank publishes,
  what the market prints, and the public events that report withdrawals and queues (OBS.3), never other
  depositors' accounts.

**Processes**

- **BFL.8 PROCESS** — A bank's reserve position each day is the **residue of everybody's payments**; nobody
  chose it.
- **BFL.9 PROCESS** — **A run**: withdrawals take reserves with them, which forces sales and dearer funding,
  which is observable and prompts further withdrawals. Deposit insurance (SUP) removes the reason for insured
  depositors to run but not for uninsured ones.
- **BFL.10 PROCESS** — **A bank that cannot pay** — its reserve account short after the market and the central
  bank's facility have both run — has failed for liquidity, whatever its capital, and goes to resolution
  (SUP).

**Invariants**

- **BFL.11 INVARIANT** — A bank's reserve balance is its account at the central bank, moved only by settlement.

**Measures**

- **BFL.12 MEASURE** — Deposit stickiness by class; the maturity gap; funding costs by bank; runs start with
  uninsured and wholesale funding; failure at one bank raises withdrawals at banks that look like it.

**Forbids**

- **BFL.13 FORBID** — No single deposit type; no unlimited, unpriced or uncollateralised central-bank credit
  (CB); no bank whose funding cannot leave.

**Primitives**

- **BFL.14 PRIMITIVE** — Regulatory liquidity minimums (POLICY of the supervisor); deposit-insurance limits (SUP);
  each bank's own buffer appetite (PREFERENCE of its management).

**Out of scope**

- Nothing beyond Appendix D.

**Done when**

- Banks can be solvent and fail for liquidity; a run can be self-reinforcing and can be stopped by insurance
  or lender-of-last-resort lending.

---

## G3. BCP — Bank capital

**Purpose.** Capital is what absorbs a bank's losses, in layers; the bank chooses how much to hold above the
rule, can raise more if investors will pay for it, and restricts itself as it approaches the line.

**Depends on:** BNK, BFL, ACC, EQY (equity), CRD (bank debt).

**State**

- **BCP.1 STATE** — Capital is **layered**: ordinary equity, then contingent capital that converts or writes
  down when a named ratio falls below a stated level, then subordinated debt, then senior creditors and
  uninsured depositors. The layers absorb losses in that order, in resolution (SUP).
- **BCP.2 STATE** — **Requirements** (POLICY of the supervisor): a risk-weighted ratio, a leverage ratio, a
  large-exposure limit and a liquidity requirement; risk weights differ by asset.

**Decisions**

- **BCP.3 DECISION** — A bank chooses a **buffer** above its requirements, from its own caution and the cost
  of equity; near the line it restricts dividends, sheds risk-weighted assets and slows lending.
- **BCP.4 DECISION** — A bank **raises capital** by issuing shares or contingent capital when it judges the
  price worth paying; the issue can fail if investors will not buy (EQY).

**Processes**

- **BCP.5 PROCESS** — Capital falls by booked losses and distributions and rises by retained income and
  issuance; it is the equity account (ACC.4), never a pot that is spent.
- **BCP.6 PROCESS** — A breach of the requirement triggers the supervisor's consequences (SUP): a restriction
  on distributions, a demanded plan, and at worst resolution.

**Measures**

- **BCP.7 MEASURE** — Banks near their line lend and pay out less; which requirement binds differs by bank;
  capital ratios are procyclical.

**Forbids**

- **BCP.8 FORBID** — No capital pot; no loss that skips a layer; no bank exempt from its requirement.

**Primitives**

- **BCP.9 PRIMITIVE** — Capital, leverage, large-exposure and liquidity requirements and risk weights (POLICY of the
  supervisor); each bank's buffer above them (PREFERENCE of its management).

**Out of scope**

- Banks' own risk models: risk weights are the supervisor's declared rule.

**Done when**

- A loss that reaches a bank's capital changes its lending through its own buffer decision; a bank that
  cannot raise capital shrinks or is resolved.

---

## G4. SEC — Securitisation

**Purpose.** Pools of named loans moved into a vehicle and funded by tranches, so credit risk moves to named
investors and banks can lend again — and so correlated losses can reach senior holders.

**Depends on:** BNK, REP, CRD, FND, RAT.

**State**

- **SEC.1 STATE** — A **vehicle** is a party holding named loans, funded by issuing **tranches** with stated
  attachment points and seniority; a **servicer** collects and passes cash through a stated waterfall.
- **SEC.7 STATE** — **Pool kinds** are declared data (Law 10): residential **mortgage-backed** securities;
  **consumer** asset-backed securities (vehicle loans, credit cards); **small-business** loan securities; and
  **collateralised loan obligations** of corporate term loans. A pool's loans may be lines to agents
  (REP.3), each standing for its count of identical loans to a named agent's twins.
- **SEC.8 STATE** — The **waterfall** states the order of interest and principal to each tranche, the
  **overcollateralisation and interest-coverage tests** that divert cash from junior to senior tranches when they
  fail, the **reserve account**, and the servicer's fee. Pass-through pools pay principal as it arrives,
  **prepayments** included.
- **SEC.9 STATE** — **Risk retention** (POLICY): the originator must keep a stated share of the pool's risk — a
  slice of every tranche or the first-loss piece — on its own books.

**Decisions**

- **SEC.10 DECISION** — A bank **securitises** when selling the pool frees capital or funding worth more to it than
  the loans' own income, at the price the tranches' buyers will pay.
- **SEC.11 DECISION** — **Investors** buy tranches from their own value of them (VAL.8): their own view of the
  pool's defaults, prepayments and recoveries, their cost of funds, their mandates and the ratings that bind them.

**Processes**

- **SEC.2 PROCESS** — A bank sells loans into a vehicle at a price the vehicle's funding supports, freeing its
  capital; it keeps what the retention rule and its own choice leave it.
- **SEC.3 PROCESS** — Losses on the underlying loans — each a borrower's own default after its own cash failure,
  an agent's counted by its count on the line — are allocated bottom-up; when defaults are more correlated than the
  tranches assumed, senior holders lose.
- **SEC.4 PROCESS** — Tranches trade and are pledged, so their prices matter to the funding system; a downgrade
  across a mandate boundary forces sales (L7).
- **SEC.12 PROCESS** — **Prepayment** reaches tranche holders as early principal, rising when rates fall and
  refinancing pays, which is the interest-rate risk of mortgage pools.

**Invariants**

- **SEC.5 INVARIANT** — Tranche losses sum to the pool's realised losses; cash out equals cash in by the
  waterfall; the pool's loans sum to the loans the vehicle holds.

**Measures**

- **SEC.13 MEASURE** — Issuance with credit conditions; tranche spreads through the cycle; senior losses in
  correlated downturns; the share of lending that is securitised.

**Forbids**

- **SEC.6 FORBID** — No pool without loans to named borrowers; no tranche without a holder; no risk transfer
  without a transferee; no pool whose losses are a rate rather than defaults.

**Primitives**

- **SEC.14 PRIMITIVE** — Risk-retention rules and capital treatment of tranches (POLICY); tranche structures and
  waterfall tests are the terms of each deal, chosen by its arranger.

**Out of scope**

- Synthetic securitisation, and securitisation of securitisations.

**Done when**

- Banks sell mortgage, consumer, small-business and corporate loan pools into vehicles and lend again; prepayments
  and defaults flow through the waterfall; a wave of correlated defaults reaches senior tranches.

---

# PART H — CAPITAL MARKETS

Where debt, equity and funds are issued and traded, where short-term money is lent and borrowed, where
intermediaries carry inventory, and where the prices everything else refers to are formed.

---

## H1. MMK — The money market and repo

**Purpose.** The market in which banks and cash-rich parties lend and borrow for days and weeks, unsecured
and against collateral; the market through which the policy rate reaches everything else; and the place a
funding squeeze first shows.

**Depends on:** MON, SET, REG, MKT, BFL, CB (central bank).

**State**

- **MMK.1 STATE** — **Unsecured** interbank loans: overnight and term, between named banks.
- **MMK.2 STATE** — **Repo**: a sale of securities with an agreement to repurchase, which is a secured loan.
  The cash lender takes title to the collateral, less a **haircut** set by the lender per security and per
  borrower; income on the collateral is passed back; the collateral may be re-used where the agreement
  allows, and every re-use is a traceable chain.
- **MMK.3 STATE** — Non-bank cash — money funds, firms, insurers — lends in repo and buys short paper.

**Decisions**

- **MMK.4 DECISION** — Every participant posts **from its own position** after the day's payments are known:
  a bank short of reserves bids, a bank long offers, at rates from its own cost of funds and its view of the
  counterparty. Who lends and who borrows is the outcome.
- **MMK.5 DECISION** — A lender sets **limits per counterparty** and **haircuts per collateral and borrower**,
  and widens or cuts them when its view worsens — which is how a solvent party loses funding before it
  loses solvency.

**Processes**

- **MMK.6 PROCESS** — The money market meets **after** the day's payments have settled, in the fund stage
  (TIME.6), and its rates are formed in call auctions or dealer markets (MKT.3, MKT.5).
- **MMK.7 PROCESS** — A name the market doubts pays more **or finds no lender**; that refusal is a real outcome.
- **MMK.8 PROCESS** — The central bank's **corridor** frames the market: a deposit facility below and a lending
  facility above (CB). The market rate sits where banks' own schedules meet, and banks prefer the market to the
  facilities while it is open to them.

**Measures**

- **MMK.9 MEASURE** — The spread between the strongest and weakest names; repo haircuts through the cycle; the
  term premium; the transmission of policy-rate changes to market rates.

**Forbids**

- **MMK.10 FORBID** — No rule assigning surplus banks to lend and deficit banks to borrow; no market rate that
  equals the policy rate by construction; no collateral counted as available by both sides.

**Primitives**

- **MMK.11 PRIMITIVE** — Repo conventions and eligible collateral per market (POLICY); each lender's limits and
  haircuts are its own decisions (MMK.5).

**Out of scope**

- Tri-party agents as separate parties: collateral moves between the two sides.

**Done when**

- Banks' daily reserve positions are funded or not in the market; a doubted bank pays more or is refused; a
  policy-rate change reaches the market through the corridor and banks' own schedules.

---

## H2. SOV — Sovereign debt

**Purpose.** Government bills and bonds: issued by a treasury that must fund itself before it spends, sold in
auctions whose failure is possible and costly, and traded to form the curve other credit is priced against.

**Depends on:** REG, MKT, TRS (treasury), CB (central bank), DLR (dealers).

**State**

- **SOV.1 STATE** — Bills (discount), fixed-coupon bonds and **inflation-linked bonds** (principal indexed to
  the published consumer price index with its publication lag), each a line that can be **reopened**; the
  line and each tranche added to it are distinct records.
- **SOV.2 STATE** — Sovereign debt ranks **pari passu**, has no covenants, and is not callable; the treasury
  manages its curve by buybacks and switches.

**Decisions**

- **SOV.3 DECISION** — The treasury decides **size, maturity and timing** of each auction from its funding plan
  (TRS) and announces the calendar in advance.
- **SOV.4 DECISION** — **Bidders** — banks for their liquidity buffers, insurers and pensions for duration,
  funds, dealers, foreign reserve managers, households and firms directly — each submit schedules from their
  own reasons and funding.
- **SOV.5 DECISION** — **Primary dealers** have a contract with the treasury: privileges in exchange for an
  undertaking to **submit a bid** in every auction. The bid's size and price are the dealer's own, limited by
  its own capital and inventory; a dealer at its limit bids low or small. Nobody is obliged to take any
  quantity at any price.

**Processes**

- **SOV.6 PROCESS** — Auctions are **uniform-price call auctions** (MKT.3); weak demand shows as a higher yield,
  a long tail and low cover, or as the treasury cutting the size; unsold paper is **not issued**, and that is an
  event.
- **SOV.7 PROCESS** — Secondary trading is a dealer market (MKT.5); the **curve** is a fit through traded
  prices, owned by one publisher, with every point labelled as traded or interpolated.

**Measures**

- **SOV.8 MEASURE** — Auction tails and cover; the level and slope of the curve; the response of yields to
  issuance size, to policy rates and to the fiscal position; the break-even inflation between nominal and
  indexed bonds.

**Forbids**

- **SOV.9 FORBID** — No forced buyer and no residual absorber in any auction; no yield that sets a price; no
  curve that uses its own previous output as an observation.

**Primitives**

- **SOV.10 PRIMITIVE** — Auction formats and the primary-dealer contract (POLICY of the treasury); the indexation
  lag of indexed bonds (POLICY).

**Out of scope**

- Debt of regional and supranational bodies.

**Done when**

- A treasury can fail to sell what it offered, and that failure has consequences it must handle.

---

## H3. CRD — Corporate and bank debt

**Purpose.** Bonds, notes, commercial paper and hybrids issued by firms and banks: brought to market by named
underwriters, priced by investors with their own views, traded by dealers, and defaulted on as events.

**Depends on:** REG, MKT, VAL, DLR, RAT, EQY.

**State**

- **CRD.1 STATE** — Debt instruments of every form in REG.5: senior and subordinated bonds, floating-rate notes,
  **commercial paper**, **convertibles**, **preferred shares** (EQY), and **contingent capital** of banks. Each
  carries covenants where its type has them, and a seniority honoured by the estate (L3).

**Decisions**

- **CRD.2 DECISION** — An **issuer** decides to issue, the size, the tenor and the form, from its funding need,
  its leverage tolerance and what each costs it now; it can **walk away** if the book prices too wide.
- **CRD.3 DECISION** — An **underwriter** chooses a basis: **best effort** (it builds the book and commits
  nothing) or **underwritten** (it commits to buy what the book does not take, within its own limit, for a
  larger fee); a large issue is shared by a **syndicate** of named banks, each within its own limit.
- **CRD.4 DECISION** — An **investor** bids from its own value (VAL.8): its own assessment of default and
  recovery, its cost of funds, the capital the holding consumes, its mandate.

**Processes**

- **CRD.5 PROCESS** — **Primary issues** are book-built call auctions (MKT.3) on the underwriter's book; a
  **tap** adds to an existing line; commercial paper is sold through dealers (MKT.5) or placed directly by
  bilateral quote (MKT.7) to cash investors, and must be rolled.
- **CRD.6 PROCESS** — **A missed payment or a breached covenant is an event** observable by holders; it can
  **accelerate** the debt; a waiver can be negotiated at a price; default makes the claim a claim on an estate
  or the subject of a restructuring the holders vote on.
- **CRD.7 PROCESS** — A **convertible** converts when its holder chooses and the terms allow; **contingent
  capital** converts or writes down automatically when its trigger ratio, read from the issuer's published
  accounts, crosses the stated level.
- **CRD.8 PROCESS** — **Refinancing** retires old debt with new at today's price, which is how a rate rise
  reaches a firm that borrowed years ago.

**Measures**

- **CRD.9 MEASURE** — Worse credit trades wider; junior wider than senior within an issuer; spreads widen in
  downturns and after defaults of similar names; defaults cluster; recoveries vary with the cycle.

**Forbids**

- **CRD.10 FORBID** — No price from a spread; no fixed recovery; no seniority that changes the price but not the
  payout; no underwritten deal whose underwriter carries no risk, and no best-effort deal that leaves the agent
  holding paper.

**Primitives**

- **CRD.11 PRIMITIVE** — None of its own: covenants, fees and terms are chosen by the parties to each issue.

**Out of scope**

- Nothing beyond Appendix D.

**Done when**

- Firms and banks issue, roll and default on debt; a default moves other issuers' spreads through investors'
  own reassessments.

---

## H4. EQY — Equity

**Purpose.** Shares as residual claims with votes, issued and bought back by firms, priced continuously by
investors with different views, and wiped out first when a firm fails.

**Depends on:** REG, MKT, VAL, ACC, RAT.

**State**

- **EQY.1 STATE** — Ordinary and preferred shares per issuer; a register of holders; a **free float** (what
  insiders, parents and strategic holders do not hold).
- **EQY.2 STATE** — A firm is **listed** when its shares trade on an exchange book; listing requires published
  reports (RAT).

**Decisions**

- **EQY.3 DECISION** — Investors buy below and sell above their own value of the share (VAL.8), from their own
  outlooks of its distributions or earnings and their required return; index funds track their index within
  their own tolerance for tracking error, posting limits, never "at any price".
- **EQY.4 DECISION** — A firm **issues shares** (an initial public offering or a secondary issue, book-built)
  when equity is its cheapest way to fund a programme it has; it **buys back** or **pays dividends** from cash
  it does not need.
- **EQY.5 DECISION** — A shareholder **votes** its shares at meetings and on takeover offers (MNA).

**Processes**

- **EQY.6 PROCESS** — Listed shares trade on a continuous book (MKT.4) with dealers and market makers.
- **EQY.7 PROCESS** — Dividends are paid to holders of record; issuance dilutes; buybacks concentrate; a split
  changes the count and nothing else.
- **EQY.8 PROCESS** — In insolvency, shareholders are paid only after every creditor; their holding goes to zero,
  never below.
- **EQY.9 PROCESS** — **Short selling** needs a borrowed share (DLR) and pays for the borrow; a short squeeze is
  a consequence of limited lendable supply.

**Measures**

- **EQY.10 MEASURE** — Returns are fat-tailed, have little autocorrelation, and their volatility clusters;
  volatility rises when prices fall; prices move on earnings surprises through revised views; market
  capitalisation is a read.

**Forbids**

- **EQY.11 FORBID** — No price from a multiple, a book value, a discounted cash flow or a target; no income to a
  shareholder from earnings not distributed; no short without a borrow.

**Primitives**

- **EQY.12 PRIMITIVE** — Listing and short-selling rules (POLICY of the exchange and the supervisor).

**Out of scope**

- Listings of one firm on several countries' exchanges.

**Done when**

- Share prices form from investors' own values; firms raise equity when it is cheap to them and can fail to
  raise it; returns show the documented statistical facts without anything imposing them.

---

## H5. MNA — Corporate control and private equity

**Purpose.** Firms bought and sold whole: mergers, takeovers, buyouts and private-equity ownership, each a
priced, funded, refusable transaction.

**Depends on:** EQY, CRD, BNK, FND.

**State**

- **MNA.9 STATE** — A **takeover offer** is a proposal to a target's shareholders: the acquirer, the price per share
  in cash or shares, its conditions (the acceptances it needs, its financing, approvals) and its expiry; acceptances
  are counted on the register.

**Decisions**

- **MNA.1 DECISION** — An **acquirer** bids when its own value of the target (its outlook of the target's
  earnings, including changes it believes it can make, discounted at its own required return) exceeds the
  price it must pay, and when it can fund the bid.
- **MNA.2 DECISION** — **Target shareholders** each accept or refuse the offer from their own value; the bid
  succeeds only with the acceptances it requires; management may resist, from its own value of the firm against
  the offer, by recommending refusal or by the defences the law allows; a rival may bid.
- **MNA.3 DECISION** — A **private-equity fund** buys a firm with its investors' committed capital and debt
  raised against the target, holds it, influences it, may pay itself a distribution funded by more debt, and
  sells it when an exit price beats holding.

**Processes**

- **MNA.4 PROCESS** — On completion the target's shares move to the acquirer and its shareholders are paid in
  cash, shares or both; the target becomes a **subsidiary** or is merged; its debts are assumed, repaid or
  triggered by change-of-control terms; its employees, suppliers and customers carry over.
- **MNA.5 PROCESS** — A buyout's debt is the target's own, so a failed buyout kills the firm and not the fund.

**Invariants**

- **MNA.6 INVARIANT** — Sources equal uses in every deal, in named accounts.

**Measures**

- **MNA.7 MEASURE** — Takeover premiums; deal volume with credit conditions; an acquirer's bonds fall when its
  leverage rises even as its shares may rise.

**Forbids**

- **MNA.8 FORBID** — No deal by assignment; no acquisition without payment; no synergy that appears without
  real revenue or cost; no headcount cut without separations.

**Primitives**

- **MNA.10 PRIMITIVE** — Takeover law: disclosure and mandatory-offer thresholds, squeeze-out rules (POLICY).

**Out of scope**

- Competition policy: no authority blocks a deal for its effect on markets.

**Done when**

- Takeovers happen when bidders value targets above their price and can fund it, and fail when shareholders
  or lenders refuse.

---

## H6. FND — Funds

**Purpose.** Pooled vehicles whose investors own the result: money funds, bond and equity funds,
exchange-traded funds, hedge funds and private-equity funds, each a channel that turns flows into purchases
and redemptions into forced sales.

**Depends on:** REG, ACC, MKT, MMK, SOV, CRD, EQY, MNA.

**State**

- **FND.1 STATE** — A fund is a named party whose liabilities are its **units**, held by named investors; its
  assets are marked at formed prices (fair value); it has a **mandate** (what it may hold, how much leverage,
  how it can be redeemed) declared as data; its **manager** is a separate party paid a fee.
- **FND.2 STATE** — Fund kinds (Law 10): **money funds** (short, high-quality paper; a substitute for deposits;
  units at a floating value); **open-ended funds** (daily or periodic redemption at net asset value);
  **exchange-traded funds** (units trade on a book; authorised dealers create and redeem against the basket);
  **hedge funds** (wide mandate, leverage from named lenders, redemption with notice and gates);
  **private-equity funds** (committed capital called on demand, a fixed life, distributions on exit); **private
  credit funds** (committed capital and their own borrowing, lent directly to firms under loan contracts, BNK.22).

**Decisions**

- **FND.3 DECISION** — Investors subscribe and redeem from their own reasons: yield against alternatives, past
  performance they observed, their own needs for cash.
- **FND.12 DECISION** — A manager **launches** a fund when it expects the fees to pay, seeding it from named
  accounts, and closes one that no longer pays.
- **FND.4 DECISION** — A manager invests within the mandate from its own views; a hedge fund takes positions on
  its views, including shorts and derivatives, and cuts them when its lenders call margin or its losses reach
  its own stop.

**Processes**

- **FND.5 PROCESS** — **Net asset value** is assets at formed prices minus liabilities, over units, read every
  day; a stale price makes a stale value, and dealing at it transfers value between investors.
- **FND.6 PROCESS** — A subscription is invested per the mandate; a **redemption** is paid from the buffer or by
  **selling into markets that must clear** — the forced-seller channel — and the cost of late sales falls on
  those who stay, which is why runs on funds happen.
- **FND.7 PROCESS** — An exchange-traded fund's price and its net asset value are two numbers; arbitrage by
  authorised dealers narrows the gap when they have the balance sheet to do it.
- **FND.8 PROCESS** — A private-equity **capital call** must be paid by its investor on the date, from its own
  liquidity, or the investor defaults on the call.
- **FND.14 PROCESS** — **Liquidation.** A fund ends when its manager closes it (FND.12), when its life or mandate
  ends it, or when its assets fall below what it owes its lenders: its assets are sold into markets, its lenders are
  paid first, and what remains is paid to its unitholders per unit; a shortfall to lenders is their loss, and the
  fund's estate distributes (L3).

**Invariants**

- **FND.9 INVARIANT** — A fund's units times net asset value equals its assets minus liabilities, to the rounding
  its unit price's convention states, and that rounding lands on the fund.

**Measures**

- **FND.10 MEASURE** — Flows chase recent performance; money-fund flows rise when their yield beats deposits;
  exchange-traded fund discounts widen in stress; hedge-fund deleveraging moves prices.

**Forbids**

- **FND.11 FORBID** — No constant value by construction: a money fund may promise a stable unit price as a term
  of its mandate, and failing to keep it is an event; no leverage without a lender; no redemption rationed by the
  fund's cash with the rest dropped; no fund that cannot fail.

**Primitives**

- **FND.13 PRIMITIVE** — Fund kinds' mandates and redemption terms (declared data, Law 10); fund regulation
  (POLICY).

**Out of scope**

- Funds of funds beyond one level.

**Done when**

- Fund flows become purchases and sales in named markets; a wave of redemptions forces sales and moves
  prices; a leveraged fund can be closed out by its lenders.

---

## H7. DLR — Dealers, securities lending and prime brokerage

**Purpose.** The intermediaries that make markets possible when natural buyers and sellers do not arrive at
the same time — and that step back when their own limits bind, which is when markets fail.

**Depends on:** MKT, REG, MMK, BCP.

**State**

- **DLR.1 STATE** — A **dealer** is a party of the dealer form — a subsidiary of a bank or an independent
  broker-dealer, its form declared — with its own equity, inventory, a funding cost on that inventory paid every
  day, a capital charge, position limits per instrument and in total, and a view.
- **DLR.2 STATE** — A **securities loan** transfers a security to a borrower against collateral worth more than
  it (a haircut), for a fee; title passes, the economics stay with the lender through manufactured payments;
  the loan can be recalled.
- **DLR.3 STATE** — A **prime broker** holds a client's assets, lends it cash and securities against them, sets
  a margin requirement on its whole portfolio, and can end the relationship.

**Decisions**

- **DLR.4 DECISION** — A dealer quotes both sides from its **own state**: inventory skews the quote (long
  inventory lowers both sides), risk and adverse selection widen it, and a binding limit **shrinks or stops**
  it. The bid–offer is the output, never an input.
- **DLR.5 DECISION** — A securities lender lends what it holds, within its mandate, against collateral it
  accepts; a borrower borrows to deliver a short or cover a fail.
- **DLR.6 DECISION** — A prime broker sets and **changes** margin from its own view of the portfolio, the market
  and the client, and calls for more when the requirement rises.

**Processes**

- **DLR.7 PROCESS** — Dealers trade with each other in an **interdealer market** to redistribute inventory.
- **DLR.8 PROCESS** — A client that cannot meet a margin call by the next business day is **closed out**: its
  positions are sold into markets that must clear, the proceeds repay the broker, and any shortfall is the
  broker's loss.
- **DLR.9 PROCESS** — A borrower that cannot return a security is bought in; the lender keeps the collateral.
  Cash collateral reinvested by the lender is a position with its own risk.

**Measures**

- **DLR.10 MEASURE** — In stress, dealer inventory, bid–offer widths and capital usage move together; raising
  margin into a falling market amplifies the fall; borrow fees spike when lendable supply is small.

**Forbids**

- **DLR.11 FORBID** — No dealer that quotes because a market needs someone to; no infinite balance sheet; no
  desk exempt from its bank's capital and funding; no dealer whose profit is the spread without the inventory's
  gains and losses; no short without a borrow; no margin that is only a number; no netting across
  counterparties.

**Primitives**

- **DLR.12 PRIMITIVE** — Position limits and capital charges (POLICY of each bank and of the supervisor).

**Out of scope**

- Market making within the day (MKT.4).

**Done when**

- Markets have depth only where dealers have capacity; a dealer at its limit steps back and a market can fail;
  a margin spiral can be seen from the first call to the last sale.

---

## H8. IDX — Indices and benchmarks

**Purpose.** The reference numbers everything else prices off must themselves be reads of formed prices.

**Depends on:** MKT, REG, STA (published statistics).

**State**

- **IDX.1 STATE** — An **index** is a published rule over a set of constituents and their weights (market
  capitalisation, amount outstanding, equal, production, or household expenditure), with a base. A **market
  index** is recomputed from its constituents' prints and never stored as a number of its own. A **published
  index** — a price index computed by the statistics agency from a sample (IDX.3, STA.2) — is a statistic: its
  published value and each revision are kept as published, with their dates, because the records it was computed
  from do not outlive their day and contracts pay on the published figure.
- **IDX.2 STATE** — **Reference rates**: the floating-rate benchmark is a read of actual transactions in the
  money market (MMK); a policy rate is never a benchmark.
- **IDX.3 STATE** — **Price indices** (published by the statistics agency, STA): a **producer** index over
  factory-gate transactions weighted by production, and a **consumer** index over what households actually
  pay at retail, weighted by their spending, including rents and consumption tax and excluding intermediate
  goods.

**Processes**

- **IDX.4 PROCESS** — A change of constituents does not jump the level: the index is chained, and the chain's
  link is stored by the publisher as its own record. A tracked index
  change is a real, simultaneous trade by every fund that tracks it.

**Invariants**

- **IDX.5 INVARIANT** — An index's return equals the weighted return of its constituents.

**Forbids**

- **IDX.6 FORBID** — No index that is an input to its own constituents; no index without constituents; no posted
  benchmark; no single index wearing two names.

**Primitives**

- **IDX.7 PRIMITIVE** — Each index's rule, constituents, weights and base (POLICY of its publisher).

**Out of scope**

- Nothing.

**Done when**

- Floating rates fix on transacted rates; producer and consumer prices are separate indices and can diverge.

---

## H9. RAT — Ratings, reporting and estimates

**Purpose.** Published opinions and published accounts, and the rules and decisions that refer to them — so
that information and its errors move markets.

**Depends on:** ACC, VAL, CRD, EQY, FND.

**State**

- **RAT.1 STATE** — A **rating** is a named agency's published, ordinal opinion of an issuer or instrument on
  the market's scale, formed from the issuer's published state — never from its price — coarse, sticky, and
  sometimes wrong. Several agencies can disagree.
- **RAT.2 STATE** — A **report** is a listed company's published statement for a fiscal period, a read of its
  books (ACC.9), published after a lag on a dated calendar, and restated if corrected. Each company chooses its
  fiscal year-end and its publication date within the legal window after it (POLICY), so reports spread across
  the calendar as they do in real markets.
- **RAT.3 STATE** — **Guidance** is management's published expectation; an **estimate** is a named analyst
  bank's published expectation, formed like any outlook from what that bank observed.
- **RAT.8 STATE** — Every incorporated firm **files** annual accounts with its country's companies registry by a
  legal deadline (POLICY); filed accounts are public, later and less detailed than a listed company's reports, and
  are what lenders and suppliers read of a firm that does not publish.

**Processes**

- **RAT.4 PROCESS** — **Rules refer to ratings**: fund and insurer mandates, capital risk weights, collateral
  haircuts and contract triggers. A downgrade across a boundary forces every bound holder to act at once. An issuer
  or instrument no agency rates is unrated, never a default grade, and each rule declares what it does with an
  unrated holding (Law 10).
- **RAT.5 PROCESS** — A report **settles** every expectation standing against it; each holder's surprise
  revises its own outlook, and prices move because schedules moved.

**Measures**

- **RAT.6 MEASURE** — The downgrade loop (selling, capital pressure, dearer funding, weaker state, further
  downgrade) is traceable step by step; prices move more on large surprises; estimates disagree more after
  volatile results.

**Forbids**

- **RAT.7 FORBID** — No rating derived from a price; no rating nothing refers to; no estimate from the share
  price or the model's forecast; no price-reaction rule; no reported number the books do not produce.

**Primitives**

- **RAT.9 PRIMITIVE** — Rating scales and methods (POLICY of each agency); reporting and filing deadlines (POLICY).

**Out of scope**

- Auditors' opinions and fraud.

**Done when**

- A downgrade across a mandate boundary produces dated forced sales; an earnings surprise moves a share price
  through investors' revised views.

---

# PART I — RISK TRANSFER

Contracts that move risk between parties: derivatives, insurance and pensions. In every one of them the
risk goes to somebody named who can pay for it or fail.

---

## I1. DRV — Derivative contracts and clearing houses

**Purpose.** What every derivative is — a bilateral obligation both sides can lose on — and how the exposure
between the two sides is collateralised, cleared and closed out.

**Depends on:** REG, SET, MKT, ACC.

**State**

- **DRV.1 STATE** — A derivative has **two named counterparties**, a **notional** in a unit (generally not
  exchanged), an **underlying** that is a price, rate, index or event this world forms, a **payoff**, a
  currency per leg, a **term**, a **price at inception** (formed, never solved for), and a **mark** after it.
- **DRV.2 STATE** — A derivative is either **bilateral** (the two parties face each other under an agreement
  that nets only the contracts between them and exchanges collateral) or **cleared** (a **clearing house**
  becomes buyer to every seller and seller to every buyer).
- **DRV.3 STATE** — A clearing house is a party with a balance sheet: the margin its members post, a **default
  fund** its members contribute to, and its own capital, used in a stated **waterfall**: the defaulter's margin,
  the defaulter's contribution, the house's capital, the survivors' contributions.
- **DRV.9 STATE** — A party that is not a clearing member **clears through a member**, which posts margin for it to
  the house, collects margin from it, and is exposed to it; when a member defaults, its clients' positions move to
  another member or are closed out.

**Processes**

- **DRV.4 PROCESS** — **Variation margin** moves the change in mark in cash every business day, the mark being the
  clearing house's settlement price or the valuation the bilateral agreement names (MKT.20); **initial
  margin** is posted up front, sized by the house or the counterparty from the underlying's own measured
  volatility and the position's remaining life, and rises when volatility rises. Posted margin is still the
  poster's asset but no longer free.
- **DRV.5 PROCESS** — A margin call unmet by the next business day **closes out** the position; the close-out
  value is a claim on the defaulter's estate, and a clearing house's shortfall runs down its waterfall;
  running past its end is a real event.
- **DRV.6 PROCESS** — A clearing member may carry no more margin than its own liquid resources can re-margin;
  a trade beyond that is cut or refused at the point of admission.

**Invariants**

- **DRV.7 INVARIANT** — Marks across the two sides of every contract sum to zero; variation margin paid equals
  received, every day.

**Forbids**

- **DRV.8 FORBID** — No derivative with one side; no underlying that exists only inside the contract; no
  netting across counterparties; no exposure without margin or a stated reason there is none; no clearing house
  that cannot run out.

**Primitives**

- **DRV.10 PRIMITIVE** — Each clearing house's margin method, default-fund sizing and waterfall (POLICY of the
  house); bilateral margin rules (POLICY).

**Out of scope**

- Compression services that tear up offsetting contracts.
- Derivatives held by households and small firms: only individuals hold derivatives (Appendix E 33).

**Done when**

- A derivative's mark moves cash every day through margin; a member's default runs through a waterfall and
  can reach surviving members' capital.

---

## I2. DRX — Derivative classes

**Purpose.** The derivative markets this world has, each with its own reasons for each side.

**Depends on:** DRV, and the market of each underlying.

**State**

Each class is declared data over DRV (Law 10), with its state and its processes:

- **DRX.1 STATE** — **Interest-rate swaps**: fixed against floating on a transacted reference rate; the fixed
  rate is formed at inception; the set of formed fixed rates across tenors **is** the swap curve. Users: firms
  switching their debt's rate, pensions and insurers extending duration (a structural one-way demand), banks
  managing their repricing gaps, speculators with a view, dealers.
- **DRX.2 STATE** — **Credit default swaps**: protection on a named reference entity's default, paid for by a
  running premium formed in the market; on the event, the seller pays par minus the recovery the defaulted debt
  actually fetches, and the premium stops. Single names and indices. Users: holders hedging, banks hedging loans
  they cannot sell, speculators on both sides, dealers. The implied default probability is derived from the
  premium, never fed into it.
- **DRX.3 STATE** — **FX forwards and swaps, and cross-currency swaps**: exchanges of two currencies at a future
  date at a formed rate; the forward sits near spot adjusted for the two currencies' funding costs only because
  somebody arbitrages it with limited balance sheet, and the residual is the **cross-currency basis**. Users:
  traders with foreign payments, investors hedging foreign assets, banks funding a foreign book, dealers.
- **DRX.4 STATE** — **Futures**: exchange-traded, standardised, margined daily, on commodities (a grade at a
  delivery place), on sovereign bonds (a deliverable basket), and on equity indices (cash-settled on the
  index). Delivery is possible at expiry, which is what ties a commodity future to spot.
- **DRX.5 STATE** — **Options**: the right to buy or sell an underlying at a strike, on or before expiry, for a
  **premium formed in the market** — on rates (caps, floors, swaptions), currencies, equities and indices, and
  commodity futures. Users: hedgers buying protection, writers selling it for income, dealers who hedge their
  books by trading the underlying. **Implied volatility is a statistic derived from the premium.** Exercise is
  the holder's decision at expiry (or before, for American style) and settles by delivery or cash.

**Decisions**

- **DRX.9 DECISION** — A user **takes a position** in a class from its own exposure (what it holds or owes that the
  class moves against), its own outlook of the underlying, the quotes it can get, its risk aversion, its mandate,
  the margin it can fund and its declared limits. A hedger and a speculator differ only in the exposure they hold.

**Measures**

- **DRX.6 MEASURE** — The swap spread and the credit basis are outcomes that differ by state; implied volatility
  rises in falling markets and has a skew; futures converge to spot at expiry, and their curves sit in contango
  within storage and financing cost when stocks are ample and in backwardation when they are short; a hedged firm
  suffers a smaller shock than an unhedged one.

**Forbids**

- **DRX.7 FORBID** — No forward from a parity formula; no fixed swap rate solved from a curve; no option premium
  from a pricing formula; no fixed recovery; no future forced to converge; **no derivative market whose only
  participants are hedgers**: every class has participants with views on both sides.

**Primitives**

- **DRX.8 PRIMITIVE** — Each class's contract specifications (declared data, Law 10).

**Out of scope**

- Exotic derivatives (Appendix D).
- Positions of households and small firms: every user is an individual (Appendix E 33).

**Done when**

- Each class has participants with their own reasons on both sides and forms its own prices; option premiums,
  swap rates, forward rates and credit premiums are all formed, and their implied statistics are reads.

---

## I3. INS — Insurance and reinsurance

**Purpose.** Insurers pool the hazards of this world — deaths, illness, accidents, damage, catastrophes — and
can fail when correlated losses exceed what they priced and hold.

**Depends on:** CHN, REG, ACC, MKT, CRD, EQY, FND.

**State**

- **INS.1 STATE** — An **insurance policy** is a contract: insurer, policyholder, what is covered (a life, a
  health state, a dwelling, a plant, a vehicle, cargo, liability), the sum insured, the premium, the term.
  Kinds: life and annuities, health and disability, property and casualty, and **reinsurance** between insurers.
- **INS.2 STATE** — An insurer's liabilities are its **reserves** for claims incurred and the present value of
  future claims on its policies, discounted at market rates; its assets are an investment portfolio.

**Decisions**

- **INS.3 DECISION** — An insurer **quotes** premiums from its own experience of claims, its own view of the
  hazard, the capital each policy consumes and the return it requires; it declines cover it cannot stand
  behind, and buys reinsurance to shed peak exposures.
- **INS.4 DECISION** — A policyholder buys cover from its own view of its exposure and its own risk aversion, at
  the best quote it can get, or goes without.
- **INS.5 DECISION** — An insurer invests to match its liabilities — a structural buyer of long bonds — within
  its mandate and its regulator's rules.

**Processes**

- **INS.6 PROCESS** — **Claims come from the hazard events of CHN**: a death, an illness, a fire, a flood. A
  catastrophe hits many policies at once in one place, which is the risk reinsurance exists for.
- **INS.7 PROCESS** — Falling rates raise the present value of long liabilities; an insurer whose assets fall
  below its liabilities is insolvent and is resolved (SUP); policyholders then have claims on its estate, up to
  any guarantee scheme.

**Invariants**

- **INS.8 INVARIANT** — Every claim paid follows a recorded hazard event on a covered subject; every premium
  comes from a named policyholder.

**Measures**

- **INS.9 MEASURE** — Premiums rise after catastrophes (an underwriting cycle); insurer solvency moves with
  rates; reinsurance concentrates catastrophe risk.

**Forbids**

- **INS.10 FORBID** — No claim computed as a share of premium; no fixed discount rate on a liability; no
  liability without beneficiaries; no insurer that cannot fail.

**Primitives**

- **INS.11 PRIMITIVE** — The hazards insured events come from (TECHNOLOGY, CHN); solvency and reserving rules
  (POLICY).

**Out of scope**

- Health care itself: insurers pay claims, and care is a service (SRV, SOC).

**Done when**

- Hazard events turn into claims at the right insurers; a catastrophe can break an under-reinsured insurer.

---

## I4. PEN — Pensions

**Purpose.** How people provide for old age: pay-as-you-go state pensions, funded employer schemes that
promise a benefit, and personal accounts that bear their own investment risk.

**Depends on:** POP, INS, FND, SOC, DRV, DRX, SUP.

**State**

- **PEN.1 STATE** — A **state pension** is a statutory benefit paid by the social-insurance system (SOC) from
  current contributions and taxes, by rules that are POLICY.
- **PEN.2 STATE** — A **defined-benefit scheme** is a party sponsored by an employer: it owes each member a benefit
  schedule, accrued as **career-average** amounts — each year's pay adds a revalued slice (Appendix E 32); its
  liability is that schedule discounted at market rates; the sponsor must fill any deficit on a schedule; members'
  claims survive the sponsor as claims on the scheme and any guarantee fund.
- **PEN.3 STATE** — A **defined-contribution account** is a member's holding of fund units, fed by
  contributions; the member bears the result and draws down or buys an annuity on retirement.

**Decisions**

- **PEN.8 DECISION** — A scheme's **trustees** choose its assets, its hedge of interest-rate and inflation risk,
  and the cash and collateral it keeps against margin calls, from its liabilities, its funding level, its mandate
  and the sponsor's strength.
- **PEN.9 DECISION** — A **sponsor** pays the contributions the scheme's funding rules require, and chooses how fast
  to repair a deficit within the law's schedule (POLICY), weighing it against its own investment and dividends.
- **PEN.10 DECISION** — A **member** of a defined-contribution scheme chooses, on its occasions, how much to
  contribute above the minimum and into which funds, and at retirement whether to draw down or buy an annuity (INS).

**Processes**

- **PEN.4 PROCESS** — Contributions are deducted from wages at payroll; benefits are paid on schedule to living
  members; deaths end benefits (CHN).
- **PEN.5 PROCESS** — A falling rate raises a defined-benefit liability and can open a deficit the sponsor must
  pay; a scheme that hedged with leveraged swaps faces margin calls in cash when rates rise.
- **PEN.12 PROCESS** — **Wind-up.** A defined-benefit scheme whose sponsor has ended with its deficit unfilled, or
  that its law's entry conditions send to a guarantee fund, winds up. Its assets pay members' rights in the law's
  order of priority, by buying annuities (INS) or paying them as they fall due; where the law provides a guarantee
  fund and its conditions are met, the fund pays what the assets cannot, up to its limits (SUP.13); every remaining
  shortfall is a dated cut to named members' rights.

**Measures**

- **PEN.6 MEASURE** — Funding ratios move with rates; sponsor contributions compete with investment; state
  pension spending rises as the population ages.

**Forbids**

- **PEN.7 FORBID** — No pension liability without members; no promise that is a cash balance; no scheme that
  cannot be underfunded.

**Primitives**

- **PEN.11 PRIMITIVE** — Pension law: funding rules, deficit-repair schedules, the guarantee fund and its entry
  conditions, and the order of members' rights on a wind-up (POLICY); state-pension rules (POLICY).

**Out of scope**

- Nothing beyond Appendix D.

**Done when**

- Retirement changes a household's income from wages to pensions of the kinds it holds; an ageing population
  raises state pension spending through named payments.

---

# PART J — THE STATE

Each country has a treasury that must fund itself, a tax system that collects from named payers, social
insurance and public services, a central bank, a supervisor with a deposit insurer and a resolution
authority, and a parliament that owns the policy choices. None of them sets a market price.

---

## J1. TRS — The treasury

**Purpose.** A government that spends on named things, collects from named payers, and must raise the
difference in markets before it spends — so fiscal policy has a funding constraint and a sovereign can fail.

**Depends on:** MON, SOV, TAX, SOC, CB, POL, VAL.

**State**

- **TRS.1 STATE** — The treasury is a party with an account at its central bank, a cash buffer, the debt it has
  issued (read from the register) and the assets it owns.

**Decisions**

- **TRS.2 DECISION** — The treasury runs a **forward funding plan**: it forecasts its own outlays, receipts and
  redemptions (its own outlook, VAL), and schedules auctions of chosen sizes and maturities ahead of the money
  leaving, keeping a buffer it chooses (under a mandate, POL).
- **TRS.3 DECISION** — When an auction falls short, the treasury pays from its buffer, cuts or defers outlays,
  returns at another size or maturity, or — where its country's financing regime allows — uses central-bank
  financing (CB.9).

**Processes**

- **TRS.4 PROCESS** — Outlays go to named recipients: purchases of goods and services, public wages, transfers
  and benefits (SOC), interest and redemptions read from the register. Receipts come from TAX.
- **TRS.5 PROCESS** — **Sovereign default**: a treasury that cannot or will not pay a debt service in full
  defaults. In a **foreign** currency it cannot create, this can happen whatever its central bank does. In its
  **own** currency it can happen only where its financing regime forbids the central bank to fund it and the
  market refuses — and where the regime allows funding, the failure mode is instead money creation and, through
  prices, inflation. A default is negotiated by exchange offer, with holdouts, and followed by market exclusion.

**Invariants**

- **TRS.6 INVARIANT** — Debt outstanding equals cumulative issuance minus redemptions and buybacks, read from
  the register; every outlay and receipt names its counterparty.

**Measures**

- **TRS.7 MEASURE** — Deficits rise in downturns through benefits and falling receipts; heavier issuance shows
  in auction prices and later in interest cost; the fiscal balance, private net saving and the current account are
  read side by side.

**Forbids**

- **TRS.8 FORBID** — No automatic central-bank overdraft; no forced buyer of its debt; no deficit target that
  sets anything; no outlay or receipt applied to an aggregate instead of named parties.

**Primitives**

- **TRS.9 PRIMITIVE** — The debt-management mandate and cash-buffer policy (POLICY of the treasury under its
  mandate).
- **TRS.10 PRIMITIVE** — The order in which the treasury's payments rank when its cash falls short — debt service,
  wages, benefits, purchases (POLICY of the parliament). A treasury that will not pay is one whose declared order
  leaves a payment unpaid; there is no other default decision (TRS.5).

**Out of scope**

- Sovereign wealth funds.

**Done when**

- A treasury issues ahead of spending, can have an auction fail, and handles the failure; own-currency default
  is possible exactly where the financing regime makes it so.

---

## J2. TAX — The tax system

**Purpose.** Taxes levied on real bases, collected from named payers at the point the base arises, remitted
on a calendar, so that the state's receipts follow the economy and taxes change what parties do.

**Depends on:** ACC, MON, TIME.

**State**

- **TAX.1 STATE** — Tax bases and rates are POLICY (owned by the parliament, POL): **income tax** on persons with a
  schedule of bands and allowances; **payroll contributions** from employers and employees; **consumption tax** in
  the form the country chooses — a **value-added tax** charged at every sale along the chain, each business
  remitting what it charged less what it paid on its own purchases, with exports zero-rated and imports taxed at the
  border, or a **retail sales tax** charged on final sales only; **corporate tax** on reported profit with loss
  carry-forward and interest deductible as the law states; **capital-gains tax** on realised gains (on average cost
  for agents' holdings, REG.1); **property tax** on dwellings and land; **tariffs** on imports at the
  border; **inheritance tax** on estates.

**Processes**

- **TAX.2 PROCESS** — Each tax arises at its base's event: **withheld** by the employer at payroll, **charged** by
  the seller at each sale and **remitted** periodically (net of tax paid on its own purchases, under a value-added
  tax), **assessed** on the annual return from the payer's own statement, filed on a day of the payer's choosing
  within the filing window (POLICY), capital income being attributed to a household's adults by the country's rule
  (POLICY), **collected** at the border by customs. Between arising and remitting it is a liability of the collector
  to the treasury.
- **TAX.3 PROCESS** — An unpaid tax is an arrear with penalties and, persisting, a claim that ranks in the
  payer's estate.
- **TAX.4 PROCESS** — Taxes change behaviour through the parties' own decisions: a consumption tax raises the
  retail price households see; corporate tax changes what a project is worth after tax; capital-gains tax
  changes when a holder sells.

**Invariants**

- **TAX.5 INVARIANT** — Tax received by the treasury equals tax remitted by named collectors; every tax payment
  has a named payer and base.

**Measures**

- **TAX.6 MEASURE** — Receipts by base and their cyclical elasticity; effective rates by income and wealth.

**Forbids**

- **TAX.7 FORBID** — No tax computed as a rate on an aggregate; no tax without a named payer; no tax rate set by
  anyone but its owner.

**Primitives**

- **TAX.8 PRIMITIVE** — Every tax's base, rates, bands, allowances, deadlines and collection rules (POLICY, owned by
  the parliament).

**Out of scope**

- Evasion and avoidance schemes (part of the informal economy, Appendix D).

**Done when**

- Every tax is collected from a named payer when its base arises; receipts fall in a downturn through the
  bases, not through a rule.

---

## J3. SOC — Social insurance and public services

**Purpose.** The state's transfers and services: unemployment and sickness benefits, state pensions, child
benefits, public education, health care and administration — paid to or provided for named people, and
employing named people.

**Depends on:** POP, LAB, TRS, TAX, POL.

**State**

- **SOC.1 STATE** — **Benefit rules** are POLICY: eligibility, replacement rates, durations, means tests.
- **SOC.2 STATE** — **Public services** are produced by public agencies employing public staff and buying
  inputs; they are provided free or at a charge to eligible people.

**Decisions**

- **SOC.8 DECISION** — A **public agency** decides its staff, wages and purchases within its **appropriation**
  (money voted in the budget, POL.11), from the demand for its service, its costs and its outlook. It cannot spend
  beyond its appropriation: demand it cannot meet is met by waits (SOC.4).

**Processes**

- **SOC.3 PROCESS** — A person who becomes eligible (loses a job, falls ill, retires, has a child) **claims**,
  and the benefit is paid on its dates to the named person's household until eligibility ends.
- **SOC.4 PROCESS** — Public education and health services are delivered to named people, with capacity: when
  demand exceeds capacity, there are waits.
- **SOC.5 PROCESS** — Public agencies hire in the labour market and buy in goods and services markets like any
  employer and buyer.

**Measures**

- **SOC.6 MEASURE** — Automatic stabilisers: benefit spending rises with unemployment; benefits' effect on
  search and participation.

**Forbids**

- **SOC.7 FORBID** — No transfer to a sector: every benefit is paid to a named household under a rule.

**Primitives**

- **SOC.9 PRIMITIVE** — Benefit and service rules (POLICY); the technologies of public services (TECHNOLOGY).

**Out of scope**

- Nothing beyond Appendix D.

**Done when**

- Benefits follow individual eligibility events; public employment and purchases are real contracts.

---

## J4. CB — The central bank

**Purpose.** The issuer of reserves and banknotes in its currency, which sets an administered rate and makes
it effective through the corridor and its operations, lends to banks on classical terms, may hold foreign
reserves, and funds its treasury only as its country's financing regime allows.

**Depends on:** MON, MMK, SOV, POL, TRS, IDX, FX.

**State**

- **CB.1 STATE** — A real balance sheet: reserves, banknotes and the treasury's account as liabilities;
  government securities, loans to banks, foreign reserves and claims on other central banks as assets; equity
  including a revaluation account for foreign positions.
- **CB.2 STATE** — A **mandate** (price stability, and where given, employment or financial stability) and its
  **target**, set by the parliament; the **rate decision** is the central bank's own (operational independence).
- **CB.3 STATE** — A **financing regime** (POLICY of the parliament, per country): the central bank may **never**
  fund the treasury; may buy its debt **only in the secondary market** for policy reasons; or may **lend to it
  directly** within stated limits and rates.

**Decisions**

- **CB.4 DECISION** — The policy committee sets its rates on its own schedule, from its own outlooks of
  inflation (on the published consumer index, with its lag) and activity against its mandate, by the rule or
  judgement it declares.
- **CB.5 DECISION** — It chooses asset purchases and sales, their size and whether to reinvest maturities, for
  policy reasons, posting limit orders in secondary markets like any participant.
- **CB.6 DECISION** — It chooses collateral eligibility and haircuts for its lending.
- **CB.14 DECISION** — **Liquidity operations.** It declares how it implements its rate — a corridor with scarce
  reserves, or a floor with ample ones (POLICY) — and runs regular repo tenders and fine-tuning operations sized
  from its own forecast of the treasury's and banknotes' flows, so that reserves are what its regime needs. Where
  a country has reserve requirements, they are held on average over a declared period (POLICY).
- **CB.15 DECISION** — Central banks may open **swap lines** with each other: reciprocal currency swaps on stated
  terms, each a contract with two sides, which each draws to lend the other's currency to its own banks against
  collateral.

**Processes**

- **CB.7 PROCESS** — **The corridor**: a deposit facility pays its rate on reserves placed with it; a lending
  facility lends overnight against eligible collateral at its rate; both are administered prices with real
  quantity responses (MKT.8), and both meet in the fund stage (TIME.6).
- **CB.8 PROCESS** — **Lender of last resort**: to a bank that is solvent in the supervisor's judgement, it
  lends freely against good collateral at a penalty rate; to an insolvent bank it does not, and the bank goes to
  resolution. These four conditions are the central bank's **declared policy**; a country may declare otherwise,
  and the declaration is visible.
- **CB.9 PROCESS** — Under a regime that allows it, direct lending to the treasury creates reserves and deposits
  as it is spent; under any regime, purchases create reserves and sales destroy them.
- **CB.10 PROCESS** — Its net income is remitted to the treasury; an unrealised revaluation gain is not; a loss is
  kept against its equity and may be made good by the treasury later.
- **CB.11 PROCESS** — It may intervene in currency markets with its foreign reserves, and cannot defend a rate
  once they are gone.

**Measures**

- **CB.12 MEASURE** — Market rates track the policy rate because of the corridor and banks' schedules; the
  transmission of rate changes to loan rates, asset prices, investment and inflation, with its lags.

**Forbids**

- **CB.13 FORBID** — No market rate equal to the policy rate by construction; no purchase sized by an auction's
  weakness; no participation in its own sovereign's primary auctions; no uncollateralised, unpriced, unlimited
  lending; no financing of the treasury beyond its country's declared regime.

**Primitives**

- **CB.16 PRIMITIVE** — Mandate, target and financing regime (POLICY of the parliament); the rate rule or judgement,
  collateral framework and implementation regime (POLICY of the central bank).

**Out of scope**

- Central-bank digital currency.

**Done when**

- Policy rates reach the economy through markets and balance sheets; a central bank can run out of foreign
  reserves but never of its own money.

---

## J5. SUP — Supervision, deposit insurance and resolution

**Purpose.** The named authorities that apply the rules to banks, insurers and clearing houses, insure small
depositors, and resolve failed institutions so that their positions do not vanish.

**Depends on:** BCP, BFL, INS, DRV, TRS.

**State**

- **SUP.1 STATE** — A **supervisor** per country applies capital, liquidity and conduct rules (POLICY) and holds
  the authority to demand plans, restrict distributions and start resolution.
- **SUP.2 STATE** — A **deposit insurer** is a party with a fund built from premiums charged to banks by risk,
  covering each depositor up to a limit per person (POLICY), with a backstop line from the treasury.
- **SUP.3 STATE** — A **resolution authority** is a named party that takes control of a failing institution.
- **SUP.13 STATE** — Where the law provides one, a **policyholder protection scheme** and a **pension guarantee
  fund** are parties like the deposit insurer: a fund built from levies on insurers or on schemes' sponsors, cover
  per person up to a limit (POLICY), and a backstop line from the treasury.

**Decisions**

- **SUP.10 DECISION** — The supervisor may set **macroprudential limits** from its own outlook of credit and asset
  prices: caps on loan-to-value and debt-to-income for new mortgages, a countercyclical capital buffer, sectoral
  risk weights (POLICY). Each binds lenders as a declared limit (Law 6), and a lender that meets one refuses or
  reprices.

**Processes**

- **SUP.4 PROCESS** — The supervisor tests institutions on their reporting dates; a breach triggers its declared
  consequences in the next business day.
- **SUP.5 PROCESS** — **Resolution**: triggered by a bank's failure for liquidity or its insolvency (which is
  stated); the authority **values** the book at what it is worth, writes down equity, converts or writes down
  contingent capital and subordinated debt, then senior debt as needed; seeks an **acquirer** who bids and may
  decline; transfers insured deposits; and the rest goes to an estate. A creditor that the authority's own
  valuation of a liquidation (MKT.20) shows worse off than it would have been there is compensated for the
  difference from the resolution fund or the treasury, named payers.
- **SUP.14 PROCESS** — **Resolution of insurers and clearing houses.** A failing insurer's book is transferred to
  an acquiring insurer that bids, or run off by the authority; what its assets cannot pay, the protection scheme
  covers per person to its limit (SUP.13). A clearing house whose waterfall is exhausted recovers by its rulebook
  (cash calls on members, loss allocation) and, failing that, has its service transferred or is wound down at the
  last settlement prices. In both, nothing vanishes, and every payer is named (SUP.8).
- **SUP.6 PROCESS** — The deposit insurer pays insured depositors what the estate cannot, and becomes a creditor
  of the estate; a fund that runs short draws its treasury backstop — a fiscal cost with a payer.
- **SUP.9 PROCESS** — **Licensing.** A bank, insurer, multi-employer pension trust, clearing house or dealer begins
  when a founder (a firm, a group, a fund, investors) subscribes its capital from named accounts because it expects
  the venture to pay (FRM.16), and the supervisor licenses it by declared criteria (POLICY). A single-employer
  pension scheme is established by its sponsor under pension law. So the financial sector grows,
  shrinks and changes as its parts are founded and fail.

**Invariants**

- **SUP.7 INVARIANT** — In every resolution, what the acquirer took, what the insurer, the protection scheme or the
  guarantee fund paid, what the estate realised and what holders lost sum to the hole the valuation found.

**Measures**

- **SUP.11 MEASURE** — Failures of banks, insurers and clearing houses and their clustering; the cost of each
  resolution and who bore it; the deposit-insurance, protection and guarantee funds through the cycle; how often
  macroprudential limits bind.

**Forbids**

- **SUP.8 FORBID** — No failed institution whose positions vanish; no bail-out without a named payer; no
  deposit insurance without a fund and a limit.

**Primitives**

- **SUP.12 PRIMITIVE** — Supervisory rules, licensing criteria, deposit-insurance limits and premiums, resolution
  tools and macroprudential tools (POLICY).

**Out of scope**

- Nothing beyond Appendix D.

**Done when**

- A failing bank is resolved through named parties and every one of its positions lands on a successor.

---

## J6. POL — The polity

**Purpose.** The owner of the fiscal and regulatory choices: a parliament elected by the people from their own
circumstances, whose parties compete and adapt, so that policy responds to the economy and the economy to
policy.

**Depends on:** POP, HH, VAL, TRS, TAX, SOC.

**State**

- **POL.1 STATE** — A **parliament** with a fixed number of seats, an electoral **term**, and an **allotment
  rule** from votes to seats (all POLICY of the constitution, declared once).
- **POL.2 STATE** — **Parties** are named parties with a **platform** (a position on each policy the parliament
  controls) and their own **ideology preference** (how far they are willing to move from their founding
  positions).
- **POL.3 STATE** — The **mandate** is the set of policy values the governing coalition enacts; every POLICY
  primitive names its owner (the parliament, the central bank, a standard-setter), and those the parliament owns
  change only by the parliament's votes: a new mandate after an election, or legislation within one (POL.11).

**Decisions**

- **POL.4 DECISION** — **Each adult person votes** for the platform that it expects to leave its household best off,
  by applying each platform to its own household's position and outlooks (VAL.8) and weighing the incumbents' record
  as it reads it in published statistics, together with its own taste for each platform drawn on each occasion from
  one distribution for all platforms (REP.22) — so that voters who are alike spread across parties, and nobody's
  loyalty is a parameter. It abstains when voting is worth less to it than its cost of voting; what its vote is worth
  rises with how close the latest published polls show the race its vote counts in, and before a campaign's first poll
  that closeness is absent. It forms its intention on its own occasions during a declared campaign period, which is
  what polls ask, and votes on the day from it.
- **POL.5 DECISION** — **Parties adapt**: between elections each party revises its platform toward positions it
  judges would have won it more votes — from published results and published polls, with their lags — as far
  as its ideology preference allows. A party that loses repeatedly can split, merge or disappear, and new parties
  can form when a large group of voters is far from every platform.
- **POL.6 DECISION** — **Coalitions** form by negotiation among parties by a declared procedure; a parliament
  with no majority keeps the standing mandate.
- **POL.11 DECISION** — **Legislating between elections.** The governing coalition brings to the parliament an
  **annual budget** on the fiscal calendar — tax rates, benefit rules, appropriations for public agencies, the
  borrowing plan — and **emergency measures** when its own outlook or public events call for them, each within its
  mandate's platform; the parliament passes or rejects it by vote, each party voting by its platform and its reading
  of its voters. A rejected budget leaves the last one standing.

**Processes**

- **POL.7 PROCESS** — An election is held on its date; seats are allotted; a government forms; the new mandate
  takes effect from a declared date and is announced before then, so parties can anticipate it (VAL.6).
- **POL.8 PROCESS** — The mandate reaches the economy only through the systems that read policy: taxes,
  benefits, outlays, regulation, the central bank's target.

**Measures**

- **POL.9 MEASURE** — Economic downturns change votes; platforms converge or polarise as outcomes; and how parties'
  vote shares relate to the spread of incomes, mean income held as a control, across the run's regions and
  elections. Each statistic, the observations it reads and its estimator are fixed in the measurement record before
  it is first read, as N3's are; each relationship is reported with its interval, and a sign is expected only where
  a cited benchmark states one.

**Forbids**

- **POL.10 FORBID** — No exogenous election result; no policy path; no turnout, swing, loyalty or bloc parameter; no
  vote computed from an aggregate statistic in place of the voter's own position (a voter reads published statistics
  and polls as any party does, VAL.5); the parliament never sets a price, a quantity, an outcome or the central bank's
  rate.

**Primitives**

- **POL.12 PRIMITIVE** — The constitution: seats, term, allotment rule, coalition procedure, budget calendar, the
  campaign period, and party funding — a payment per vote received and a registration deposit (POLICY, declared
  once); parties' ideology preferences and the cost of voting (PREFERENCE).

**Out of scope**

- Referendums, courts and lobbying.

**Done when**

- Elections are held on the calendar, decided by individual votes from individual circumstances; policy changes
  through mandates and reaches the economy through named payments.

---

# PART K — THE OPEN WORLD

Several countries, each with its own money, trading goods, services, capital and people across their
borders. The world as a whole is closed: every cross-border flow has a named party on each side.

---

## K1. FX — Currencies and exchange

**Purpose.** Each currency is a price in every other, formed by parties who need, hold or want to change
currencies, so exchange rates move with trade, rate differentials and portfolio choices.

**Depends on:** MON, MKT, DLR, CB.

**State**

- **FX.1 STATE** — Every currency is issued by its central bank; a party holds foreign currency only as an
  account at a bank in that currency's system (or at a correspondent), which is a real position.
- **FX.2 STATE** — Each country declares an **exchange-rate regime** (POLICY): floating, managed, or pegged
  with a stated rate and the reserves to defend it.

**Decisions**

- **FX.3 DECISION** — Parties buy and sell currencies for their own reasons: an importer paying a seller in the
  seller's currency, an exporter converting receipts, an investor buying a foreign asset or repatriating, a
  borrower servicing foreign debt, a hedger, a dealer managing inventory, a central bank intervening within its
  reserves.

**Processes**

- **FX.4 PROCESS** — Spot currency trades in a **dealer market** per pair (MKT.5); both currencies settle together
  (SET.4). Each pair forms its own price from its own flow; the cross through a third currency and the direct
  pair agree only as far as arbitrageurs with limited balance sheet make them.
- **FX.5 PROCESS** — One rate is in force per pair per day for valuation, the day's fixing (MKT.12), and every
  foreign position **revalues** at it as a real gain or loss to a named party.
- **FX.6 PROCESS** — A peg holds while the central bank's reserves and the market's belief hold; a peg whose
  reserves run out breaks, and the break is an event.

**Invariants**

- **FX.7 INVARIANT** — Every currency trade has two legs in two currencies, each landing in its own currency;
  dealer and client positions sum to zero per currency.

**Measures**

- **FX.8 MEASURE** — Persistent one-way flow moves the rate; rate differentials attract carry flows that unwind
  violently; pass-through of exchange-rate moves to import prices is incomplete and lagged; cross-rate gaps
  are small, bounded by arbitrage capacity.

**Forbids**

- **FX.9 FORBID** — No rate from purchasing-power parity or a rate-differential formula; no conversion without a
  counterparty; no vehicle currency by construction; no written rate path.

**Primitives**

- **FX.10 PRIMITIVE** — Each country's exchange-rate regime (POLICY).

**Out of scope**

- Currency unions.

**Done when**

- Exchange rates move with the flows that cross them; a peg can break when reserves run out.

---

## K2. XB — Cross-border trade, finance and migration

**Purpose.** What crosses borders — goods, services, capital, income and people — each as transactions
between named parties, so the balance of payments is a read and its imbalances are financed by someone who
chooses to.

**Depends on:** FX, GDS, SRV, FRT, CRD, EQY, FND, POP, TAX.

**State**

- **XB.1 STATE** — A cross-border transaction is one between parties sited in different countries; it is invoiced
  in the currency its contract states (XB.11), and the party not invoicing in its own money holds the currency
  exposure.
- **XB.2 STATE** — Border policies (POLICY of each country): tariffs by good, capital-flow rules, and
  admission rules for migrants.

**Decisions**

- **XB.11 DECISION** — The **seller chooses the currency it quotes in** — its own, the buyer's, or a third that
  is widely used — from its own costs, what its buyers will accept, and the hedging it can buy; the buyer accepts
  or goes elsewhere. Which currencies dominate trade is an outcome, never a rule.

**Processes**

- **XB.3 PROCESS** — **Trade**: a buyer imports when the delivered price (price abroad, converted, plus freight,
  tariff and time) beats buying at home; the goods cross a border on a route, pay duty and import value-added tax
  at customs, and the buyer pays in the invoice currency (XB.11), buying it if it does not hold it.
- **XB.4 PROCESS** — **Finance**: investors buy foreign assets for return and diversification; borrowers issue in
  foreign currencies when it is cheaper, and then owe a money they do not earn; banks fund in one currency and
  lend in another and must square it; direct investment buys foreign firms outright.
- **XB.5 PROCESS** — **Income** — coupons, dividends, interest, wages of cross-border workers, remittances — flows
  across borders to named holders in their currencies.
- **XB.6 PROCESS** — **Migration** moves people between countries under admission rules (POP.8), taking their
  skills and, by the rules, their savings.
- **XB.7 PROCESS** — **A deficit must be financed by somebody who chooses to**, at a price, and that financing can
  stop — which is a sudden stop, with its consequences for the currency and for foreign-currency borrowers.

**Invariants**

- **XB.8 INVARIANT** — One country's exports are another's imports, party to party; for each country the current
  account, the capital account (cross-border inheritances and migrants' transfers) and the financial account, read
  from transactions, sum to zero; across all countries every category
  sums to zero.

**Measures**

- **XB.9 MEASURE** — Trade falls with distance (the gravity pattern); exchange-rate moves switch expenditure with
  a lag; foreign-currency debt turns depreciations into defaults.

**Forbids**

- **XB.10 FORBID** — No exogenous trade or capital-flow series; no country that is a closed box; no netting of
  cross-border flows into a regional aggregate.

**Primitives**

- **XB.12 PRIMITIVE** — Tariffs, capital-flow rules and admission rules (POLICY of each country).

**Out of scope**

- Trade agreements beyond each country's declared tariffs.

**Done when**

- Imports and exports are transactions between named firms; the balance of payments is a read that balances;
  a sudden stop can happen.

---

# PART L — TRANSMISSION

The causal chains that run across systems. Each system above can meet its own requirements and the world can
still fail to transmit, because transmission lives in the joints. Every chain here is stated as: the
mechanism, the systems it runs through and what silently breaks it; the test that would kill the claim that it
works is its row of N4.

---

## L1. A loss is an event

**Mechanism.** A borrower's own cash or solvency fails; a payment is missed on a date; the claim goes into
arrears, is impaired, worked out or enforced; something is sold for what it fetches; the loss lands on named
holders in the order of their claims.

**Runs through:** FRM, HH, BNK, CRD, SEC, HSG, DRX (CDS), L3.

**Silently broken by:** a loss rate applied to a book; a default drawn from a probability; a recovery fixed at a
constant; a threshold tested on an average borrower.

---

## L2. The forced seller

**Mechanism.** A party must sell something it did not want to sell, at whatever the market pays, and the sale
moves the price, which reaches other holders. The doors in: a **margin call** unmet from cash (DLR, DRV); a
**redemption** a fund cannot meet from its buffer (FND); a **funding line withdrawn** (MMK, BFL); a **mandate
boundary crossed** by a downgrade (RAT); a **capital requirement** a bank can only meet by shrinking (BCP); an
**estate's** liquidation (L3).

**Silently broken by:** a floor at zero on available credit; an automatic loan of the shortfall from the same
lender; a margin payment with no cash test; a redemption rationed by cash with the rest dropped.

---

## L3. Nothing is immortal, and every estate distributes

**Mechanism.** A party ends by its own trigger; an estate opens; a firm's or an institution's assets are **sold**
into real markets to real bidders, and a person's estate sells only what its debts need (POP.9) (plant to buyers who
can use it, stock where it always sold); its claims rank in the order the country's insolvency law declares
(POLICY), which places every class — secured; preferential claims such as employees', taxes, and deposits and the
deposit insurer where the law prefers them; ordinary unsecured claims, including senior debt, trade creditors,
close-out claims and other deposits, ranking equally unless the law says otherwise; subordinated; equity — and the
proceeds are paid in that order in every currency the party held; employees are released through the labour market;
suppliers lose receivables; references resolve to the estate.

**Who ends and how:**

| Party                     | Ends when                                                  | Then                                                        |
| ------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------- |
| Person                    | a mortality event                                          | household continues or becomes an estate; heirs inherit     |
| Firm                      | cannot pay, or liabilities exceed assets, under its law    | restructuring or estate                                     |
| Bank                      | fails for liquidity, or is insolvent                       | resolution (SUP)                                             |
| Fund                      | assets fall below what it owes lenders, or units reach zero | liquidation; lenders take shortfalls, investors lose        |
| Insurer, pension scheme   | assets below the value of liabilities                      | sponsor contribution, benefit cuts, resolution or guarantee |
| Clearing house            | its waterfall is exhausted                                 | resolution; members' further losses                         |
| Sovereign                 | cannot or will not pay (TRS.5)                             | default, exchange offer, market exclusion                   |
| Central bank              | cannot fail in its own money; can lose equity              | losses carried, may be made good by the treasury            |

**Silently broken by:** any party that cannot fail; a firm's estate that values rather than sells; a claim ranked by
instrument type rather than its own seniority; an estate that collects receivables while its trade creditors rank
nowhere.

---

## L4. The cost of capital

**Mechanism.** A financial price moves; somebody's marginal cost of capital moves; a real decision — to invest,
hire, hold stock, buy a dwelling — changes; output and employment follow with the build and hiring lags.

**Runs through:** MMK → BFL (bank funding cost) → BNK (loan pricing and standards) → CAP, FRM, HSG, HH;
and SOV, CRD, EQY → CAP (firms' marginal debt and equity cost).

**Silently broken by:** a bank that prices from the policy rate instead of its own funding; investment as a rate
on revenue; a firm that uses its average old coupon instead of today's marginal cost; a dealer whose inventory
costs nothing to carry.

---

## L5. The credit cycle and the housing cycle

**Mechanism.** Rising asset prices raise collateral values and bank capital; lenders' own assessments improve
and standards loosen; credit grows and bids prices up further — until losses, a funding squeeze or a rate rise
reverse it, when standards tighten, forced sales and foreclosures add supply, and prices, credit and spending
fall together.

**Runs through:** HSG, BNK, BCP, BFL, HH, CAP, VAL (heuristic switching).

**Silently broken by:** a constant lending standard; a loss rate instead of foreclosure; house prices from a path;
outlooks shared by all.

---

## L6. Runs and contagion

**Mechanism.** A depositor, lender or investor who observes weakness withdraws; the withdrawal forces sales or
dearer funding, which is observable, which prompts more withdrawals — at the institution and at others that
look like it. Losses travel by name along exposures: interbank loans, derivative counterparties, clearing-house
waterfalls, trade credit, securitisation tranches.

**Runs through:** BFL, MMK, FND, DRV, TCR, SEC, SUP.

**Silently broken by:** one deposit type; unlimited central-bank credit; netting across counterparties;
anything observable only as an aggregate.

---

## L7. The downgrade loop

**Mechanism.** A rating falls across a boundary; mandated holders sell, capital charges rise, haircuts widen;
the issuer's funding costs more; its state worsens; the rating falls further.

**Runs through:** RAT, FND, INS, BCP, MMK, CRD.

**Silently broken by:** a rating read from the price; a haircut that is one number for every credit; a rating no
rule refers to.

---

## L8. The fiscal and political loop

**Mechanism.** A downturn raises benefits and lowers receipts; the treasury must borrow more; its cost of
borrowing moves; voters' circumstances change their votes; a new mandate changes taxes and spending; the change
reaches households and firms through named payments.

**Runs through:** LAB, HH, SOC, TAX, TRS, SOV, POL.

**Silently broken by:** a central-bank overdraft; a policy path; a vote from an aggregate.

---

## L9. Supply shocks and the cost chain

**Mechanism.** A catastrophe, a route closure or a depleting deposit removes real units; the price at that place
rises; merchants ship from elsewhere as far as freight capacity allows; producers' input costs rise before
their output prices; retail margins compress; consumer prices follow; the central bank reads its published
index and responds.

**Runs through:** GEO, CHN, GDS, ENE, FRT, TEC, FRM, SRV, IDX, CB.

**Silently broken by:** a price shock instead of lost units; one index wearing two names; a recipe expressed as
a value share.

---

## L10. The open-economy chain

**Mechanism.** A rate differential or a trade imbalance moves a currency; import prices move; expenditure
switches; foreign-currency borrowers' debts revalue and some fail; foreign investors reprice and can stop
financing a deficit.

**Runs through:** FX, XB, CRD, BNK, CB, TRS.

**Silently broken by:** a formula exchange rate; conversion without a counterparty; netted cross-border flows.

---

## L11. Expectations transmit

**Mechanism.** A shock surprises the parties it touches first; they revise their outlooks and act; their actions
are what others observe; outlooks and heuristic mixes shift across the population in sequence, amplifying or
damping the shock.

**Runs through:** VAL and every decision.

**Silently broken by:** a global expectation; a model forecast handed to parties; outlooks that do not differ.

---

## L12. Growth

**Mechanism.** Firms spend on research when they expect it to pay; discoveries improve ways; the improvements
lower costs and prices, win market share and spread by licensing and imitation; investment embodies them in
new plant; productivity, wages and output grow — unevenly, across firms, industries and regions.

**Runs through:** TEC, FRM, CAP, LAB, POP (skills).

**Silently broken by:** an exogenous productivity trend; improvements nobody paid for.

---

# PART M — OBSERVATION

What can be seen, by whom, and when — so that information has a real distribution and a real cost, and so that
nothing seen changes what it shows.

---

## M1. OBS — The observer surface and the news

**Purpose.** A view of the world for a human observer or player that shows only what is truly there, with its
age, and a stream of news generated from real events.

**Depends on:** every system it shows.

**State**

- **OBS.1 STATE** — **Public** information: prints with their age; published statistics (STA) with their lags
  and revisions; reports, filed accounts (RAT.8), guidance and estimates; ratings; the patent register (TEC.15);
  policy decisions and mandates; auction results; election results and published polls; events (below).
- **OBS.2 STATE** — **Private** information: a party's own positions, limits, intentions and outlooks. The
  observer surface declares which view it gives — an **inspector's** full view of the world, or a
  **participant's** view of its own party plus public information. **Both views exist**, each labelled on every
  screen, and they are different products: the inspector's view is for building and research, the
  participant's for playing.
- **OBS.3 STATE** — An **event** is a change of state somebody would notice — a default, a failed auction, a
  downgrade, a run, a catastrophe, an election, a policy change, a large print, an estate's distribution —
  generated **from** the state, with a date and named subjects, and it can develop over days. Events are part of
  the world's **public information**, which deciders may read (VAL.5): they are produced by the world, by a
  declared rule of what becomes public (a standing SHAPE, standing in for how news is selected), and the observer
  surface only shows them.

**Processes**

- **OBS.4 PROCESS** — A human **player** acts as a named party in the world, with its own means, through the same
  markets and contracts as everybody else, and appears in every check. The player's party is an agent of
  multiplicity one from the start (REP.1): its household, drawn at the opening from the households of the country the
  setup names, each equally likely, and under twins one twin of the agent drawn.
- **OBS.8 PROCESS** — **Looking at a household or small firm** shows its **agent** itself: its attributes, persons,
  positions and contracts, and the recorded events that name it. Under twins one agent stands for _k_ identical
  households or firms, and the view says so. Looking at one agent, or at a million, leaves the world exactly as it
  would have been.

**Forbids**

- **OBS.5 FORBID** — No news the state did not produce: an event reports what happened, and parties that read it
  react to what happened; no story, headline or sentiment moves anything by being written.
- **OBS.6 FORBID** — No display-only number, no invented story, no scripted narrative, no privileged actor, and
  no surface that changes the world by looking at it.
- **OBS.7 FORBID** — A missing number is shown as missing, a stale price as stale, and every instrument by the
  name its market uses.

**Primitives**

- **OBS.9 PRIMITIVE** — The rule of what becomes a public event (a standing SHAPE, OBS.3).

**Out of scope**

- Rumour and social media as mechanisms.

**Done when**

- Everything shown is reproducible from the state; a player's actions pass every check any party's would.

---

## M2. STA — Published statistics

**Purpose.** The official numbers of each country — national accounts, prices, labour, money, trade — computed
from the world's transactions by a statistics agency and published after a lag, with revisions, because that
is how real deciders see the aggregate economy.

**Depends on:** ACC, IDX, and every system it counts.

**State**

- **STA.1 STATE** — A **statistics agency** per country publishes, on a calendar: output (nominal and **real**,
  deflated by the appropriate price index), its components (consumption, investment, government, exports,
  imports), income and its distribution, the consumer and producer price indices, employment, unemployment,
  vacancies, wages, the money stock, credit, house prices, the balance of payments, the fiscal balance, research
  spending and patents granted by industry, and vital statistics (deaths and illness by age and health, as a period
  life table) — each on its own publication day, so releases spread across the month.

**Processes**

- **STA.2 PROCESS** — Each statistic is computed from a **sample** or from reported records of the period, published
  after a declared lag, and **revised** as more records arrive. Inflation is always stated as the change in a
  named index over a named period.

**Measures**

- **STA.3 MEASURE** — Output measured by expenditure, by income and by production agree up to the
  statistical discrepancy the sampling produces, and that discrepancy is itself published.

**Forbids**

- **STA.4 FORBID** — No statistic is an input to the world except as a published number a party chose to read;
  no statistic available before its publication date.

**Primitives**

- **STA.5 PRIMITIVE** — Survey designs, sample sizes, publication calendars and revision policies (POLICY of each
  agency).

**Out of scope**

- Nothing.

**Done when**

- Every country publishes its accounts on a calendar, and decisions that read aggregates read these, late and
  revised.

---

# PART N — MEASUREMENT AND ACCEPTANCE

What must hold every day, what must be checked at every stage of building, what "realistic" means, and how a
claim about the model is tested.

---

## N1. The audit

The audit runs at every day's close over what the day left behind. Each **family** is independent; a violation
names the family, the requirement, the owner, the size in its unit and the day; the audit never repairs.

| Family            | What must hold                                                                                            |
| ----------------- | --------------------------------------------------------------------------------------------------------- |
| **Money**         | balances at every issuer equal its liability; every change in money stock is an issuer's own transaction |
| **Ownership**     | holdings sum to issued per instrument; every contract is one record with two sides                        |
| **Flows**         | every instruction has both legs, in one currency each; instructions reconcile to holdings                 |
| **Accounts**      | assets minus liabilities equals the equity account for every party that has one; income reconciles      |
| **Units**         | goods, dwellings, plant and deposits reconcile: opening plus in equals out plus closing                  |
| **Contracts**     | receivables equal payables; mortgages owed equal mortgages held; derivative marks sum to zero             |
| **Prices**        | every mark came from a print or a declared valuation (MKT.20); no print without a match                   |
| **Names**         | every referenced party exists or has a successor                                                          |
| **Cross-border**  | exports equal imports party to party; each country's accounts balance                                     |
| **Representation** | people and small firms reconcile by entry, death and migration, and agents' multiplicities sum to them; every holding and count is a whole multiple of its agent's multiplicity; the sides of every line are equal; attachments reconcile with persons |

**Independence is measured**: for every family there is a single discrepancy that lights it and no other, injected
into values handed to the audit alone — never into the world that runs, and discarded unrun after the audit reads
them (decision 36) — and a discrepancy that several families can see is reported first by the family that owns the
fact.

## N2. Liveness — allowed at every stage of the build

Liveness reads are cheap, never tuned to, and may be taken at any time while building. They answer "is the
world alive?", not "is it right?":

- Books form prices, and the share of meetings that fail is reported.
- Money circulates: payments per day, velocity, no stock of money stuck on dead parties.
- Parties are born and die in every sector that has them.
- Production, employment, lending and investment are non-zero and respond when a primitive's owner changes it in
  the run (a policy rate, a tax rate, a standard).
- No quantity grows without bound for a reason nobody can name; no state repeats identically for ever with
  nothing changing (a dead fixed point).
- Every system's **Done when** evidence exists.

A liveness failure is a missing mechanism; it is recorded and built, never patched.

## N3. Realism — the stylised facts

The world is judged **realistic** when, in its run, it reproduces the documented regularities of real economies
**without any of them being imposed**. Each is a measurement; a miss is a finding about a mechanism, never a reason to
tune a number. A slow distribution counts while the world holds it, and a fact about behaviour once the run has
produced it (GEN.10).

Each fact has a **statistic** and a **benchmark range cited from published empirical work**. Because the world's
countries are generated, a benchmark is the range real economies of the country's development level (GEN.14) show,
not one country's number. Before a fact
is first measured, its statistic is fixed exactly in the measurement record (series, filter, window, sample),
together with its sources. A benchmark stated only in words gets its numerical range, from its source, at that
point.

| #   | Fact                                                         | Statistic                                                                  | Benchmark and source                                                                                          |
| --- | ------------------------------------------------------------ | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| 1   | Output grows, with irregular recessions                       | long-run growth; recession and expansion durations                          | expansions last several times longer than recessions (NBER business-cycle chronology)                         |
| 2   | Investment volatile, consumption smooth | standard deviation of cyclical components relative to output | investment about three times as volatile as output; nondurable consumption less volatile than output (Hodrick and Prescott, 1997; Rebelo, 2005) |
| 3   | Co-movement with output                                       | correlation of cyclical components with output                              | consumption, investment, employment positive; unemployment strongly negative (Stock and Watson, 1999)         |
| 4   | Okun's relation | regression of the change in unemployment on output growth | ≈ −0.4 on average across advanced economies, varying by country (Ball, Leigh and Loungani, 2017) |
| 5   | Beveridge curve | correlation of vacancy and unemployment rates | strongly negative (Shimer, 2005) |
| 6   | Flat, shifting Phillips relation                              | slope of wage or price inflation on slack, over rolling windows             | small and unstable slope (Hooper, Mishkin and Sufi, 2020)                                                     |
| 7   | Output growth is fat-tailed                                   | shape of the growth-rate distribution                                       | close to Laplace, far from normal (Fagiolo, Napoletano and Roventini, 2008)                                   |
| 8   | Firm sizes are skewed                                         | tail exponent of the firm-size distribution                                 | ≈ 1, Zipf (Axtell, 2001)                                                                                       |
| 9   | Firm growth is fat-tailed and falls in variance with size | growth-rate distribution; volatility against size | tent-shaped (Laplace) growth rates whose spread falls as a power of size (Stanley et al., 1996; Bottazzi and Secchi, 2003) |
| 10  | Productivity dispersion                                       | ratio of the 90th to the 10th percentile of productivity within an industry | ≈ 2:1 for total factor productivity in narrow industries (Syverson, 2011)                                     |
| 11  | Frequent entry and exit; young firms fail more | exit rates by firm age; five-year survival | most start-ups exit within five years, and survivors grow fast (Haltiwanger, Jarmin and Miranda, 2013) |
| 12  | Sticky, lumpy prices                                          | median duration of individual regular prices; mean size of changes          | median ≈ 8–11 months; changes ≈ 10% on average (Nakamura and Steinsson, 2008; Klenow and Kryvtsov, 2008)     |
| 13  | Income and wealth distributions | Pareto exponents of the top of income and of wealth | both tails are Pareto, and the wealth tail is fatter than the income tail (Atkinson, Piketty and Saez, 2011; Vermeulen, 2018) |
| 14  | Spending responses differ with liquidity                      | consumption response to a transfer by liquid wealth                         | markedly higher for households with little liquid wealth (Jappelli and Pistaferri, 2014; Kaplan and Violante, 2014) |
| 15  | Long unemployment spells | distribution of spell lengths; long-term share over the cycle | the long-term share rises sharply in and after recessions, and exit rates fall with duration (Kroft, Lange, Notowidigdo and Katz, 2016) |
| 16  | Credit is procyclical and leverage builds in booms            | correlation of credit growth with output; leverage over the cycle           | credit booms precede downturns (Schularick and Taylor, 2012)                                                  |
| 17  | Defaults cluster                                              | excess dispersion of default counts over independent defaults               | defaults more clustered than common factors alone explain (Das, Duffie, Kapadia and Saita, 2007)              |
| 18  | Crises follow credit booms                                    | probability of a banking crisis given past credit growth                    | rises with past credit growth (Schularick and Taylor, 2012; Reinhart and Rogoff, 2009)                        |
| 19  | Partial, lagged pass-through of policy rates | response of loan rates to a policy-rate change over time | incomplete on impact, larger over time, and asymmetric (de Bondt, 2002, and the euro-area pass-through literature) |
| 20  | Returns fat-tailed, nearly uncorrelated | tail index of daily returns; autocorrelation of returns | tail index ≈ 3; autocorrelation insignificant at daily horizons (Gopikrishnan et al., 1999; Cont, 2001) |
| 21  | Volatility clusters, rises as prices fall                     | autocorrelation of absolute returns; correlation of returns with future volatility | slowly decaying autocorrelation; negative return–volatility correlation (Cont, 2001; Black, 1976)       |
| 22  | Housing booms and busts, volumes lead prices                  | lead–lag of transaction volume and price                                    | volumes turn before prices (Stein, 1995)                                                                       |
| 23  | Yield curve slopes up; inversion precedes recessions          | average term spread; its predictive power for downturns                     | positive on average; inversion predicts recessions (Estrella and Mishkin, 1998)                               |
| 24  | Credit spreads are countercyclical                            | correlation of corporate spreads with output; response to defaults          | widen in downturns and predict them (Gilchrist and Zakrajšek, 2012)                                           |
| 25  | Trade falls with distance                                     | elasticity of trade between regions with respect to distance, within and across borders | ≈ −0.9 (Disdier and Head, 2008)                                                                                |
| 26  | Exchange rates disconnected over short horizons               | forecastability of exchange rates by fundamentals                           | a random walk forecasts about as well (Meese and Rogoff, 1983)                                                |
| 27  | Sudden stops                                                  | frequency and size of current-account reversals                             | abrupt reversals occur, especially with foreign-currency debt (Calvo, 1998)                                   |
| 28  | Fertility and migration respond to conditions | response of births and moves to income, housing cost and employment | rising house prices raise births among owners and lower them among renters (Dettling and Kearney, 2014); high unemployment delays births (Adsera, 2011); migrants move toward higher earnings (Grogger and Hanson, 2011) |

## N4. Causal-chain tests

The world is not run again to test a chain. What each chain in Part L implies for the relationships between macro
variables — their signs, leads and lags — is read from the world's run and compared with what real economies show,
each with a benchmark cited from published work, as N3's facts are: each statistic is fixed exactly in the
measurement record before it is first read, and a benchmark stated in words gets its range from its source then. A
chain whose implied relationships miss is a finding about its links. A claim about the model without such a test is
not made. L1 and L3 say where losses and endings land, not how macro variables relate: they are held to the audit
(N1) and to N3's facts 17 and 11.

| Chain | Relationships read from the run | Benchmark and source |
| ----- | ------------------------------- | -------------------- |
| L1 | defaults cluster beyond common factors (fact 17) | Das, Duffie, Kapadia and Saita (2007) |
| L2 | forced sales are followed by price falls in what was sold, partly reversed later; forced sales of dwellings fetch less | Coval and Stafford (2007); Campbell, Giglio and Pathak (2011) |
| L3 | young firms exit more (fact 11) | Haltiwanger, Jarmin and Miranda (2013) |
| L4 | after a policy-rate surprise, market rates move at once, loan rates partly and with a lag (fact 19), investment and output fall over the following quarters, prices later | Kuttner (2001); Christiano, Eichenbaum and Evans (1999) |
| L5 | credit and house prices rise together; a rise in household debt precedes slower output growth; volumes lead prices (fact 22) | Mian, Sufi and Verner (2017); Stein (1995) |
| L6 | withdrawals from a failing bank spread to banks that look like it; uninsured and wholesale funding leaves first, insured deposits largely stay | Iyer and Puri (2012); Iyer and Peydró (2011) |
| L7 | a downgrade across a regulatory boundary is followed by sales by bound holders and a price fall that partly reverses | Ellul, Jotikasthira and Lundblad (2011) |
| L8 | the fiscal balance falls in downturns through benefits and receipts; incumbents lose votes when the economy worsens | Girouard and André (2005); Lewis-Beck and Stegmaier (2000) |
| L9 | a cost shock passes to downstream prices partly and with a lag, as margins absorb part of it | Nakamura and Zerom (2010); Kilian (2009) |
| L10 | exchange-rate moves pass to import prices partly and with a lag; current-account reversals occur, larger with foreign-currency debt (fact 27) | Campa and Goldberg (2005); Calvo (1998) |
| L11 | the dispersion of outlooks widens after surprises; mean outlooks adjust to shocks with a lag | Mankiw, Reis and Wolfers (2003); Coibion and Gorodnichenko (2012) |
| L12 | a new way's use spreads across firms along an S-shaped path over years; productivity differs widely within an industry (fact 10) | Griliches (1957); Comin and Hobijn (2010) |

## N5. Reproducibility

- The same seed and primitives reproduce the same world on the same build and device, so a saved world restores
  and continues exactly (SET.15).
- **One run.** The world is judged on the one run it has: there is no second run at another resolution, seed or
  setting to compare it with. Its distributional numbers are read from its agents and individuals, with the
  representation they are held in reported beside them (REP.15).

## N6. Interventions and experiments

_Retired_: an experiment on a copy of the world is a second run, and the world runs once. A primitive changes during
the run only by its owner's decision (Law 16).

## N7. Calibration

- Primitives may be taken from real data, with the source recorded in the primitive register.
- Outcomes may be **compared** with real data (N3) and never **tuned** to match it. When an outcome misses, the
  finding names the mechanism suspected of missing, and nothing is adjusted to close the gap.
- A primitive that cannot be measured is **estimated** or **assumed** and says so; the share of assumed
  primitives is reported.

## N8. The performance budget

The world exists to be played on a phone, turn by turn. A world that is right but takes hours per turn does not
meet its purpose, so the budget is a requirement with the same standing as the audit and the realism test.

- **N8.1** — **The target device** is a current flagship phone (initially the Pixel 11 Pro), running the world
  on the device itself, with no server.
- **N8.2** — **A turn** is one simulated business day by default, together with any non-business days before
  it. At the **play resolution** (N8.5), with the population its representation holds (REP.40), a turn completes in **at
  most 1 second at the median and 2 seconds at the worst** (month-ends, quarter-ends, paydays, election days, the last
  days of a tax-filing window, and the days markets are busiest), measured over a full simulated year.
- **N8.3** — **Sustained**: the budget holds across a simulated year of consecutive turns with the phone's own
  thermal limits in force, not only for a first burst of turns while the device is cool.
- **N8.4** — **Memory**: the world, the day's instructions and its snapshots stay within a declared memory
  budget (initially 4.5 GB resident) and a declared storage budget for saves (initially 4 GB), and neither grows
  without bound — which is what SET.12–SET.16 exist for: each store's growth per simulated year, measured over the
  run, is what the population and the declared horizons (SET.13) explain.
- **N8.5** — **The play resolution** is the setting of every RESOLUTION primitive but the map's grid, which the owner
  fixed (decision 29) — among them the representation's factor (REP.40), the number of types (NUM.4), each kind's
  attribute classes, zones and age classes, the individuals' ranks, history horizons and snapshot intervals (SET.13,
  SET.17) — that is finest while meeting N8.2–N8.4 on the target device, in the representation the build holds
  (decision 44). Where refining one setting costs another,
  the order in which they are refined is the owner's (decision 41). The resolution is a valve: it is set and reset by
  measurement of the budget. Whether the world at that resolution makes sense is judged by its macro results against
  real economies' (N3, N4); a miss is a finding, and the representation's report (REP.15) is published beside the
  results.
- **N8.6** — **Cost follows events, not size**: nothing in this specification requires every party to be visited
  every day. Parties act on their own schedules or when woken (TIME.5), accruals are applied on the dates that
  need them, and the daily audit checks what the day changed, with the full audit on a declared cycle.
- **N8.7** — **The budget never changes a mechanism.** When the budget is missed, the remedies are, in order: how
  the world is represented and traversed; then the play resolution (a larger factor, REP.40). The population is
  fitted to the phone by the factor alone, and no law, mechanism or requirement is weakened to meet it.
- **N8.8** — The budget is **measured on the device** at the end of every stage from Stage 0 on, and a stage does
  not end with the budget missed. At each stage's end it is measured twice on the phone: on the stage's own world, and
  on the finished world's volumes — random data at every store's final size run through the stage's own kernels at
  the final daily counts — so a finished world too slow or too large is found at Stage 0, not at the end.
- **N8.10** — **Saving is budgeted apart from turns.** A snapshot is written at the moments SET.12 declares, within
  the memory and storage budgets, and its duration is measured and budgeted on the device separately from the
  turn's.
- **N8.9** — **Heavy days are spread as far as real calendars spread them**: companies' report dates differ
  across the reporting window (RAT.2), tax returns across the filing window (TAX.2), voting intentions across the
  campaign (POL.4), statistics are released on different days (STA.1), the full audit runs as a rolling
  cycle. No event whose date is causal is moved to meet
  the budget.

---

# PART O — BUILD STAGES

The layers of this document in the order they can be built, grouped into stages. Each stage ends with a
**living world**: everything built so far runs, the audit is clean for what exists, and the stage's liveness
reads (N2) pass. A stage is not a delivery date and says nothing about how to build.

**Stage 0 — Foundations.** TIME, PTY, NUM, CHN, GEO, REP, GEN for what exists, MON, SET, REG, ACC, MKT, POP's
mortality and illness, the household estates they need (L3), the firms, banks and central banks of the opening world as
parties with their opening balance sheets (FRM, BNK and CB brought forward without behaviour), and the opening world's
employment, tenancy, deposit and loan lines and its pensions in payment — the state pension and defined-benefit pensions
already being paid — paying as their terms say (LAB, HSG, BNK, SOC and PEN brought forward as contracts that execute,
with no decision), so that paydays and dues are measured before behaviour is built. *Exit:* a world of parties on a map
can pay each other and hold and transfer instruments and physical units, every market form is built and forms its prices
whenever its participants post, with every family of the audit that applies running clean — **and the opening population
and its small firms, carried as agents in the build's representation (REP.40) with their persons, holdings and the lines
of Stage 0's systems, live a simulated year of deaths, illness, ageing and catastrophes within the memory budget (N8.4)
and the time budget (N8.2) on the target device.** No order is placed for a party by anything but its own decision
(MKT.9), so each form forms live prices from the stage whose systems post in it. This bounds what the representation
holds before any behaviour is built on it. It does not bound the daily flows — shopping, pay, hiring — which are Stage
1's go/no-go.

**Stage 1 — The circular flow.** All three countries, each closed to the others: POP (births), HH (spending, working,
saving in deposits), TEC (opening ways, no innovation), FRM with its births and estates (L3), CAP (plant only), GDS,
SRV, FRT within each country, LAB, one tier of banks with BNK and deposits (their marginal cost of funds a placeholder
for BFL), the central bank's settlement and a fixed policy rate (a placeholder for CB), a treasury with income and
consumption tax, one benefit and bills sold at auction (SOV's bills), the consumer price index (IDX.3) and published
statistics, VAL (adaptive outlooks and values), and the opening dwellings held without a housing market (a placeholder
for HSG). *Exit:* households earn wages, spend them at firms that pay wages, firms are born and die, banks lend and are
repaid, the treasury taxes and spends — and the world keeps doing so without anything imposed — **and a simulated year
of it, with the population its representation holds at the play resolution, meets the performance budget (N8) on the
target device.** This is the first go/no-go point: if the thin circular flow cannot meet it, the representation is
revisited before anything is built on top of it.

**Stage 2 — Credit and failure.** L1 (loss as event), L3 (estates), TCR, the full firm lifecycle, bank provisions
and write-offs, BFL, BCP, SUP (supervision, deposit insurance, resolution, macroprudential limits), HSG with
mortgages and land, ENE, the credit bureau and filed accounts, personal insolvency, inheritance in kind.
*Exit:* a borrower's own cash failure produces a default, an estate, a loss on named holders and a housing
foreclosure; a bank can fail for liquidity or solvency and is resolved.

**Stage 3 — Money and capital markets.** MMK and repo, the full central bank (corridor, operations, lender of last
resort, financing regime, liquidity operations), TRS with SOV auctions, CRD, EQY, DLR, FND, non-bank lenders, IDX,
RAT, L2 (forced seller), L4 (cost of capital). *Exit:* the policy rate reaches loan rates, asset prices and
investment through markets; a margin spiral and a fund run can happen.

**Stage 4 — Risk transfer.** DRV with client clearing, DRX (swaps, credit, futures, options; currency derivatives with
FX at Stage 5), INS, PEN with its trustees', sponsors' and members' decisions, SEC, MNA. *Exit:* every derivative
class forms its price with views on both sides; hazard events become insurance claims; pension liabilities move with
rates.

**Stage 5 — The full state and the open world.** TAX in full, SOC with public agencies and their appropriations, POL
with budgets and emergency legislation, the three countries opened to each other, FX, XB, central-bank swap lines, FRT
across borders, migration between countries. *Exit:* elections change policy; currencies float or break their pegs;
trade and capital flows balance as reads.

**Stage 6 — Growth and the full population.** TEC research and diffusion, POP in full (formation, education and
retraining, migration between regions), HH in full. *Exit:* the run grows through discovered improvements; the
population's size and shape are outcomes.

**Stage 7 — Realism.** The measurement of the world's run completed: every N3 stylised fact and N4 chain
relationship, and N7 calibration of primitives from data. What misses is recorded against the mechanism suspected;
the build continues by adding mechanisms, never by tuning.

**Rules for the stages**

- A stage uses only systems from its own or earlier stages; a need discovered for a later system is met by
  **bringing that system forward**, and the move is recorded.
- Within a stage, a system is built to its **Done when** before the next is started, as far as the systems built so
  far can show it; what only a later system of the same stage can show (a firm's sales need households that spend)
  is shown when that system is done, and the stage does not end until every Done when is shown. An item that needs
  an event the run has not yet produced is shown by its mechanism's trigger and consequence at logic level and is
  listed as not yet seen in the run, with the run's length; it never blocks (decision 39).
- Every SHAPE introduced to let an earlier stage run names the later system that retires it.
- GEN grows with the stages: each stage's systems are opened by it, and each stage's exit is judged after the
  settling period.
- From Stage 1 on, each stage's exit also reads, from that stage's run, the N3 facts and N4 relationships whose
  systems exist by then, each with its verdict or as not yet credited with the run's length (GEN.10); they never
  block (decision 39).

---

# APPENDICES

---

## Appendix A — Glossary

| Term                        | Meaning in this document                                                                                     |
| --------------------------- | ------------------------------------------------------------------------------------------------------------ |
| **Party**                   | anything that can hold, owe, decide or be paid (PTY.1)                                                       |
| **Person / household**      | an individual human / the people who share a budget and dwelling and own jointly (PTY.3)                    |
| **Agent / multiplicity / twin** | a household or small firm with its own state, persons and contracts / the count of real parties it stands for / each of those identical parties (REP.1) |
| **Individual**              | a party never held in a population: institutions, issuers, public names, the top-ranked by size at the opening (REP.2) |
| **Representation**          | twins (the full population, agents of multiplicity _k_) or a small world (one _k_-th of it, agents of multiplicity one) (REP.40) |
| **Attribute / position**    | what an agent holds exactly as a declared class / a continuous amount it holds or reads (REP.41, REP.20) |
| **Line**                    | one record of identical contracts whose sides are named parties with exact counts (REP.3)                |
| **Kink**                    | a point where a rule, contract or constraint changes slope: a due payment, a limit, a tax band (REP.22)  |
| **Drawn pairing**           | who in a line is paired with whom, drawn only when something depends on it (REP.23)                       |
| **Occasion / attention**    | a review, need, meeting or notice on which an agent takes a lumpy decision / how often it reviews (REP.21, REP.38) |
| **Price point**             | a round or conventional number a seller posts at (REP.34)                                                 |
| **Setup**                   | the world constants and a new game's choices from which the opening is derived (GEN.14, GEN.15) |
| **Opening world / settling** | the first day's state, drawn and balanced (GEN) / the period the world runs by its own mechanisms before play (GEN.6) |
| **Valuation**               | a named valuer's figure for a position with no print of its own, from prints by a published method; never a print (MKT.20) |
| **Primitive**               | a declared number of one of the six kinds of Law 2                                                           |
| **Outcome**                 | anything the world produces rather than is given                                                            |
| **Hazard process**          | a declared source of chance with a rate, acting on named subjects (CHN.2)                                   |
| **Way**                     | a fixed-proportion method of producing a product (TEC.2)                                                    |
| **Market form**             | one of the six price-forming mechanisms of MKT                                                               |
| **Print**                   | a price formed by a match, with its market, day, unit and quantity (MKT.2)                                  |
| **Mark**                    | the print a holder values a position at, under its carrying basis                                           |
| **Outlook**                 | a party's own forecast of a variable it acts on (VAL.1)                                                     |
| **Value**                   | what a thing is worth to a party, by its own method (VAL.2); never a price                                  |
| **Surprise / confidence**   | observed minus expected / a read of the width of recent surprises (VAL.4)                                   |
| **Carrying basis**          | how a position enters its holder's books (ACC.2)                                                           |
| **Unrealised difference**   | units × (latest price − carrying value) (ACC.3)                                                             |
| **Equity account**          | a record moved only by named events, checked against assets minus liabilities (ACC.4, ACC.10)               |
| **Lien / free units**       | a claim over units / held units minus pledged units (REG.2)                                                 |
| **Estate**                  | the party that holds and distributes what an ended party left (L3)                                          |
| **Forced seller**           | a party made to sell by an obligation it cannot otherwise meet (L2)                                         |
| **Financing regime**        | a country's declared rule for central-bank funding of its treasury (CB.3)                                   |
| **Mandate**                 | the policy values a governing coalition enacts (POL.3)                                                      |
| **Liveness read**           | a cheap check that the world is alive, allowed at any stage (N2)                                            |
| **Stylised fact**           | a documented regularity of real economies the world should reproduce unimposed (N3)                         |

---

## Appendix B — The prohibitions, consolidated

A prohibition that holds is as valuable as a mechanism that works, and it is the easiest thing to break
silently. Each line cites the requirements that state it.

**Money and ownership**

1. No money without an issuer; no silent negative balance; no conversion at the ledger (MON.11–MON.14).
2. No move without an instruction; no one-sided or partial settlement (SET.11).
3. No holding without a holder; no claim without an issuer; no short without a borrow; no unit pledged twice
   (REG.16, DLR.11).
4. No income without delivery, accrual or receipt; no cost recognised twice; no stored value beside units
   (ACC.13–ACC.16).

**Prices and markets**

5. No price-taker of a price not yet formed; no buyer or seller of last resort; no price from a path, formula,
   target, parity or statistic; no spread on a mid (MKT.16–MKT.19).
6. No forward from parity, swap rate from a curve, option premium from a formula, or future forced to converge
   (DRX.7).
7. No posted benchmark; no index that feeds its constituents; no single index with two names (IDX.6).
8. No rating from a price; no estimate from a share price (RAT.7).

**Bounds, chance and imposition**

9. No invented bound (Law 6); no drawn outcome and no unseeded chance (CHN.5, CHN.6).
10. No exogenous path for any price, rate, earnings, productivity, unemployment, population, trade, capital flow or
    election (TEC.11, FRM.20, LAB.15, POP.14, XB.10, POL.10).
11. No fixed recovery; no fixed discount rate on a liability; no constant fund value; no loss rate in place of a
    default (BNK.14, INS.10, FND.11, L1).

**Institutions and failure**

12. No party that cannot end except an issuer in its own money; no ending without a destination (PTY.13, L3).
13. No automatic central-bank overdraft; no financing beyond the declared regime; no unlimited, unpriced or
    uncollateralised central-bank credit (TRS.8, CB.13).
14. No forced buyer in any auction (SOV.9); no dealer that must quote; no infinite balance sheet (DLR.11).
15. No netting across counterparties; no exposure without margin or a stated reason; no clearing house that cannot
    run out (DRV.8).
16. No leverage without a lender; no failed institution whose positions vanish (FND.11, SUP.8).

**Structure and representation**

17. No representative agent; no averaged attribute, position, posted price, contract term or person; no agent split
    or joined, and no twin that differs from another; no lumpy decision without an occasion; no experience read by an
    agent that did not have it; no recorded pairing; no multiplicity that is not a count; no event that creates or
    loses a unit (PTY.14, HH.18, REP.16, REP.17, Law 11).
18. No global expectation; no model forecast; no peeking; no sentiment parameter; no common value; no value
    printed as a price (VAL.16–VAL.21).
19. No aggregate matching function; no birth, migration, participation or investment rate (LAB.15, POP.14,
    CAP.11).
20. No instantaneous or costless transport; no goods in transit owned by nobody (FRT.11).
21. No mechanism that branches on the kind of thing it acts on (Law 10).

**Observation**

22. No news the state did not produce; no display-only number; no privileged actor; no surface that changes the
    world (OBS.5, OBS.6, Law 17).
23. No statistic available before its publication (STA.4).
24. No credit for the opening day alone; no drawn past; no opening adjusted after seeing a run; no balancing that
    sets a price; no opening decision drawn instead of decided (GEN.10, GEN.11, GEN.13).

---

## Appendix C — The primitive catalogue

What the world must be **given**, by kind. Every item is registered (NUM.3) with its value, unit, owner and
source.

| Kind           | What                                                                                                   |
| -------------- | ------------------------------------------------------------------------------------------------------ |
| **TECHNOLOGY** | ways of making every product; what reviewing and changing each kind of decision costs, and what drawing cash costs; power-plant technologies; capital kinds, lives and wear; construction and build lead times; vehicle speeds, capacities and running costs; storage and spoilage; life tables and health hazards; conception hazard; schooling, retraining and experience to skill, and its erosion in unemployment; what a job application costs; learning curves; discovery and imitation hazards and improvement distributions; catastrophe frequencies and exposures; search meeting rates |
| **PREFERENCE** | distributions, carried as finite type sets (with shares, NUM.4), of patience, risk aversion, tastes and taste dispersion, leisure, dwelling and location preferences, preference for children, memory, heuristic-switching intensity; management risk appetite, hurdles and horizons; banks' buffer appetites; decision schedules; party ideology preferences; the cost of voting |
| **POLICY**     | each trade's price points; macroprudential limits; personal insolvency law; credit-reporting and account-filing rules; budgets and appropriations; the treasury's payment priority; the central bank's implementation regime and any reserve requirement; tax bases and rates; benefit rules; minimum wage and labour law; capital, liquidity and exposure rules; deposit-insurance limits and premiums; insolvency and inheritance law; zoning; tariffs, capital-flow rules and admission rules; patent law; the net settlement system's rules; pension law; education and family law; the central bank's mandate, target and financing regime; the constitution's seats, term and allotment rule; accounting standards; market conventions (settlement cycles, day counts, auction formats) |
| **ENDOWMENT**  | the map, terrain, deposits and opening infrastructure; calendars; the opening population with its households, skills and holdings; the opening firms, banks, funds, insurers and their balance sheets; opening contracts and instruments with their terms and remaining lives |
| **RESOLUTION** | the representation and its factor; each kind's attribute classes, zones, age classes, the individuals' ranks, payment order, draw scheme; number of preference types; map grid; history horizons and snapshot intervals |
| **SHAPE**      | the heuristic menu and how many heuristics a party tracks per outlook (VAL.22); the **form of every decision rule** — how a household, firm, bank, fund, agency or party turns the inputs its DECISION clause lists into a choice — each listed with its reason (no mechanism in scope derives how people decide) and its source in the literature; terrain-generation parameters (GEO.18); the rule of what becomes a public event (OBS.3); every placeholder introduced during building, each naming what retires it |

The opening world is made by GEN: one date's snapshot of the present, drawn from declared, data-shaped
distributions, balanced in its accounts and nothing else, its contracts' pasts computed by the steady-path
convention, its first decisions taken by its own parties on day zero, and settled by the world's own mechanisms. It
must pass the audit on its first day, is never adjusted because of what a run shows, and the opening day alone earns
no credit as evidence (GEN.10).

---

## Appendix D — Out of scope

Each item is deliberately absent, with the reason. Out of scope is not missing: it is a decision, and it can be
revisited in Appendix E.

| Item                                                        | Reason                                                                                     |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Informal and illegal economies, crime                        | no observable mechanism to build from without inventing one                                |
| Intra-household bargaining and household production          | the household is the unit of choice; the value of leisure stands in for home production    |
| Discrimination, occupational licensing                       | would need behavioural primitives no source supports at this level                         |
| Environment, emissions and climate change as a process       | hazards on tiles are in scope; a changing climate is not, to keep hazard rates primitives  |
| Private and crypto currencies                                | the world's money is issued by banks and central banks                                     |
| Payment-system outages, cyber events                         | settlement is always available on business days                                            |
| Detailed corporate governance and executive incentives       | management preferences stand in for them                                                   |
| Exotic derivatives beyond options, swaps, forwards, futures and credit default swaps | the listed classes carry the transmission channels that matter               |
| Wars and political violence                                  | no mechanism consistent with Law 16 at this level                                          |
| Trade finance and letters of credit                          | the currency, freight and trade-credit channels already carry the transmission              |
| Local government                                             | regional offices of the national state collect local taxes; a second layer of government adds parties without a new mechanism |

---

## Appendix E — The decision record

Decisions taken in writing this version, and decisions still open.

**Taken**

1. **The laws are about the world only.** How the building work is organised is a separate document.
2. **The day is the atom of time**, with business-day calendars, so overnight funding, daily margin and fast
   runs exist. Parties act on their own schedules, so cost grows with events rather than with parties × days.
3. **Chance is allowed, seeded and dated**, for real hazards and search meetings, and never for outcomes.
4. **Expectations are adaptive plus each party's own simple models**, with heuristic switching by own
   performance (after the heuristic-switching literature, e.g. Brock and Hommes), so parties can value what they
   have never seen.
5. **A price is legitimate if a real mechanism formed it**: auctions, books, dealer quotes, posted prices,
   bilateral quotes and negotiations, plus administered rates with a quantity response.
6. **Central-bank financing of the treasury is a policy choice per country**; own-currency default and
   inflationary financing are both possible outcomes of it.
7. **Scope includes** services and distribution, growth and technology, a full tax and social-insurance system,
   demography and migration, options and more derivatives, richer debt instruments, physical cash and corporate
   groups.
8. **Realism has an acceptance test** — the stylised facts of N3 — measured, never tuned to; liveness reads are
   allowed at every stage.
9. **Primitives may come from data; outcomes are compared with data but never tuned.**
10. **The tiled map stays**, with deposits and hazards.
11. **The document is ordered by causal layer**, and the order is mostly the build order: Part O builds
    interdependent systems in one stage and brings a system forward where an earlier stage needs it.
12. **Real limits are declared, invented bounds are forbidden**, and negative prices are possible.
13. **The polity stays, and parties adapt their platforms.**
14. **A real-sized population, carried as agents** (REP). The world holds hundreds of millions of people and
    millions of small firms. No phone can hold them one by one, and no exact record can hold every pairing between
    them, because a household's employer, landlord, bank and shops are independent of one another. So every household
    and small firm the world holds is an agent with its own state, persons and contracts, never split, joined or
    averaged; lines record relationships between many parties as counts; and the population is fitted to the phone
    by one of two representations (decision 44), not by averaging anyone.

    It is a **hypothesis**, and it is tested on its macro results: from Stage 1 on, the world's run is judged by the
    relationships between its macro variables against those real economies show (N3, N4), while meeting the budget
    (decision 36).

    Its parts come from established work:
    - **Weighted agents of one fixed multiplicity that never split** are the robust form of super-individuals
      (Scheffer, Baveco, DeAngelis, Rose and van Nes, 1995; Parry and Evans, 2008, on the distortions of splitting
      and merging ones), as in Covasim's population scale (Kerr et al., 2021), the super-droplets of cloud physics
      (Shima et al., 2009) and dynamic microsimulation's households copied to equal weight (Li and O'Donoghue,
      2013); **a full-scale agent per real party** follows Poledna, Miess, Hommes and Rabitsch (2023).
    - **Next hits drawn ahead per agent** are the next-reaction method of stochastic kinetics (Gibson and Bruck).
    - **Occasions chosen against a real cost of reviewing** follow costly observation and staggered adjustment
      (Calvo; Reis; Alvarez, Guiso and Lippi, 2012), and **attention that rises with what is at stake** follows
      Sicherman, Loewenstein, Seppi and Utkus (2016).
    - **Tastes drawn per occasion** follow random utility (McFadden).
    - **Price points** follow the bunching of prices and wages at round numbers (Levy, Lee, Chen, Kauffman and
      Bergen, 2011; Dube, Manning and Naidu, 2018).
    - **Many-party lines with pairings drawn when needed** apply the principle of deferred decisions (Motwani and
      Raghavan) to the uniform matching of the configuration model (Bollobás).
    - **Batch settlement** is how real payrolls and card payments settle (deferred net settlement).
    - **Heuristic switching by discrete choice** follows Brock and Hommes, and **experience weighted by age**
      follows Malmendier and Nagel (2016).
    - **The largest firms kept individual** follows granular fluctuations (Gabaix, 2011).

    What it gives up, and says so:
    - persistence of who is paired with whom inside a line beyond its terms;
    - where within a region an agent works; the exact tile of what it holds until something depends on it;
    - under twins, that an agent's twins share one fate: a death, a default or a success happens to _k_ real parties
      at once, so counts move in steps of _k_ and a small firm's rise is _k_ identical rises;
    - under a small world, the scale of the real economy: fewer parties in every market;
    - final salary's back-loading of pension rights (decision 32);
    - derivatives held by households and small firms (decision 33).

15. **Three generated countries.** Enough for cross rates, triangular arbitrage, trade and migration, and for a
    large and a small open economy. Their primitives may come from data (tax law, life tables, technology), but no
    country copies a real one, so results are never read as forecasts of a real economy; a real name labels a
    country and pre-fills its setup, never its data (decision 43).
16. _Retired_: the full population carried in cells was never scaled down, but cells joined their members within
    tolerances, and a phone cannot hold the full population as agents. Decision 44 fits the population to the phone
    by its representation, and says which of the two a build holds.
17. **Both observer views exist**, clearly labelled: an inspector's full view for building and research, and a
    participant's view for playing (OBS.2).
18. **Stylised facts have cited benchmark ranges** from published empirical work (N3); the exact statistic is
    fixed before each is first measured, and a miss is a finding, never a tuning target.
19. **A performance budget** (N8): one simulated business day in at most 1 s median and 2 s worst on the target
    phone, sustained over a simulated year, within declared memory; measured at the end of every stage from Stage 0.
20. **Snapshots, not replay** (SET.12–SET.16): instructions live for their day, a saved world restores exactly and
    continues, and nothing a decision reads lives only in an instruction. Rebuilding past days and bit-identical
    results across machines are not required: the engineering around the economy keeps its slack.

21. **Energy is its own system** (ENE): plants, a grid with limits, daily wholesale markets per region with
    negative prices possible, retail tariffs, shortages as named losses, and fuels as commodities.
22. **Every save is full** (SET.12), within the 4 GB budget: the latest complete save and the one being written are
    the peak: a full save, an increment and the next full save would exceed 4 GB, and an increment could not be
    written within its time.
23. **This document is the specification**, not a version of one.
24. **Unincorporated businesses belong to their households** (FRM.23), as their law has it, so most of the world's
    firms by count are household activities, and the firms carried as separate parties are the incorporated ones.
25. **The representation is tested on its macro results**: Stage 0 bounds what it holds, and from Stage 1 on the
    world's run is judged by the relationships between its macro variables against real economies' (N3, N4) while
    meeting the budget (decision 36).
26. **Batched payments settle net between banks** (MON.5), as payrolls and card payments do, so one instruction
    carries a whole payroll or a market's settlement with every payer and payee named.
27. **Sellers post at price points** (REP.34), as real prices and wages bunch, so identical sellers share a price
    and no posted price is ever an average.
28. **The opening world is a snapshot of the present** (GEN): derived from a short setup (decision 43), drawn from
    declared distributions shaped like real economies' data and varied between countries, made consistent in its
    accounts and nothing else, given its first decisions by its parties on day zero, then settled by the world's own
    mechanisms for one year by default, a length the owner can adjust; that year is the world's only history (decision
    38).

29. **The map** is about 40,000 tiles of 10 km across the three countries, with 25 regions in all, allotted to the
    countries by their population shares with at least three each (GEO.3, decision 43).
30. _Retired_: there is no accuracy for play to set. The play resolution is set by the budget alone (N8.5), and
    whether the world makes sense is judged by its one run (decision 36).
31. **The representation is coarsened for the phone.** Independent estimates put the fully exact representation at
    several times the budget (N8), driven by what agents do rather than by how many there are. So: employment lines
    are by occupation family and region, with a start band (LAB.1, REP.3); and reviews reach an agent on its own
    review days (REP.21). What each loses is listed under decision 14.
32. **Defined-benefit rights accrue as career-average amounts** (PEN.2). A final-salary right needs each member's pay
    history, a record per person that the budget does not carry; career-average revalued slices are one exact total
    per right. Final-salary schemes of
    the opening world are carried as their career-average equivalents.
33. **Derivatives are held by individuals only** (DRX, DRV): banks, funds, insurers, pension schemes, dealers and the
    largest firms. Households and small firms carry their rate and price risks through the terms of their loans,
    deposits and supply contracts, as most do; daily margin per agent would cost the budget for few real users.
34. **Liability cover has its hazard**: harm to third parties is one of CHN.3's processes, so liability claims come
    from events like every other claim (INS.6).
35. **A small firm is sold whole as the agent it is**: when its owners seek a buyer, the agent itself is named as
    the target, and under twins each of its twins is sold to one of the buyer's (REP.1).
36. **One run.** The world runs once, on the phone: no reference run of the full population one party at a time, no
    second run at another resolution or seed, no copy for an experiment. A new build starts its run anew from day zero,
    since a save from another build is refused. Runs off the phone that test the code are never read as the world's.
    Whether it makes sense is judged by its macro outcomes against the relationships real economies show between macro
    variables (N3, N4). The resolution is a valve, set and reset by measurement of the budget.
37. **Parties are publicly funded** (POL.12): the constitution pays each party per vote received and requires a
    registration deposit to stand; parties employ staff and buy polls from polling firms, which are ordinary firms.
38. **The opening is a snapshot of the present** (GEN.2, GEN.5, GEN.6, GEN.10, GEN.13). One date's state, derived from
    the setup (decision 43) and drawn once per fact with every other side derived: stocks, contracts with their start
    dates, and the single latest value of everything observed on that date — each market's print and fixing, each
    reference rate and index, each published statistic, each rating, each firm's latest filed accounts, households'
    surveyed expectations. Whatever a contract needs from before the snapshot follows one steady-path convention: as
    if the present values had held since its start. On day zero every party decides once by its own rules; nothing a
    party decides is drawn. No series is drawn, and the one settling year is the world's only history. Slow
    distributions — income, wealth, firm sizes — are credited while the world holds them and moves their members
    within them, not after decades of regrowth.
39. **What the run has not produced never blocks** (Part O, N3, N4). A rare event, a possibility a Done-when item
    states ("can break", "can happen") or a fact that needs time is shown when the run produces it; until then it is
    listed as not yet seen, with the run's length, and its mechanism's trigger and consequence are shown at logic
    level on values handed to it. Only the budget blocks a stage (N8.8).
40. **The resolution is measured only at the play resolution.** The representation's counts and costs are read in the
    one run at the resolution in force; when the valve moves, its effect is measured in the running world.
41. **The order of refinement** (N8.5): when the budget allows a finer resolution, the representation's factor is
    refined first, then the number of preference types, then attribute classes and zones, and individuals' ranks last;
    when the budget calls for a coarser one, the same order runs backwards. If representation and traversal and this
    valve cannot meet the worst turn (N8.2), the answer is decided on the measured numbers at the first gates
    (N8.7, N8.8), under the standing rule that the specification is coarsened before the budget is relaxed.
42. **Public ways** (TEC.4): every firm knows its industry's ways that no patent covers and no firm keeps to itself,
    so a new firm can produce from its first day; only discovered improvements, patented or private, are assets to be
    licensed or imitated (TEC.6).
43. **The setup** (GEN.14, GEN.15). A world is shaped by a few choices, so a new game takes one screen or none. The
    world constants are fixed for the whole simulation: the total population, the map, three countries, 25 regions,
    the settling year. The player splits the population among the countries, each between 10% and 70%, and sets for
    each country six three-level choices — development, public debt, private debt, risk appetite, inequality,
    openness — and a name, real or generated; every choice has a default or may be drawn. The development level draws
    a joint profile from real country groups' published profiles, and the other choices move their values within it,
    so the ~20 numbers the opening needs are never typed. A real name labels the country's institutions and currency
    and pre-fills its choices with that country's levels; its economy is always generated. Because the total
    population is a constant, no split can break the budget (N8).
44. **Two representations, one switch** (REP.40). The phone cannot hold the full population one party at a time, so
    a build holds it in one of two ways with one factor _k_: **twins**, the full population with every household and
    small firm an agent of _k_ identical twins; or a **small world**, one _k_-th of the population with every agent
    one real party. Both use the same mechanisms and draw the same number of agents; the build's representation is a
    world constant recorded with the run, and a save of the other is refused. Twins is the default, for the depth
    of the full economy's banks, markets and largest firms; the two are compared on the finished world's macro
    results (N3, N4), and the factor is set by the budget (N8.5).

**Open** — none. A question the text does not settle and the laws do not settle is added here before the stage that
needs it.

---

*Project Phoenix — the world, and nothing about how to build it.*
