# PROJECT PHOENIX — V2

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
order is therefore also a dependency order, and Part O turns it into build stages.

| Part | Layer                                                       |
| ---- | ----------------------------------------------------------- |
| I    | The laws of the model                                       |
| II   | How requirements are written                                |
| A    | Foundations: time, parties, numbers, chance, the physical world, how populations are represented |
| B    | Money and ownership: money, settlement, instruments, accounting |
| C    | Price formation, expectations and valuation                 |
| D    | People: population and households                           |
| E    | Production: technology, firms, capital                      |
| F    | Real markets: goods, services, freight, labour, housing, trade credit |
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

In scope: a world of three fictional countries on a physical map, each with its own currency, central bank,
treasury, tax system, social insurance, parliament and banking system; a population of hundreds of millions of
people living in households that are born, age, work, consume, save, borrow, migrate and die; millions of firms
that are born, produce goods and services with technologies that improve, invest, trade, borrow, merge and die;
the markets for goods, services, labour, housing, land, commodities and freight; and the financial system of
money, payments, cash, banks with term loans, credit lines and mortgages, securitisation, money markets,
sovereign and corporate debt, equity, funds, dealers, derivatives including options, insurance and pensions.
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
| **RESOLUTION** | a numerical choice about representation, tested by invariance                   | cohort budget, merge tolerance, map grid, preference types |
| **SHAPE**      | a claim about the answer standing in for a mechanism not yet built              | a placeholder for an unbuilt decision                         |

Everything else — ownership, prices, quantities, shares, capacities, allocations, distributions — is an
**outcome**. A SHAPE names the mechanism it stands in for and dies in the change that builds it; the count
of SHAPEs is the honest measure of how much model is missing and may only fall.

**A real-world primitive may be imported; a real-world outcome may not.** A statutory tax rate, a life
table or a recipe may come from data. A market share, a spread, a leverage ratio or a growth rate may not:
those are answers, and importing one means the model can never tell you anything about it. Outcomes may be
**compared** with data (§N3); they are never **tuned** to it.

**A residual with no holder is a defect**: a quantity computed as "everything minus the parts we know" must
have a named owner, or the computation is wrong.

### Law 3 — Every price is formed by a real mechanism between parties with reasons

A price is legitimate when it comes out of one of the price-forming mechanisms real economies use (§C1):
an auction or order book, a dealer's two-way quote, a posted price facing buyers who may walk away, a
bilateral quote a counterparty may refuse, or a negotiation between named parties. It is never computed by
the model on anybody's behalf.

- Yield, spread, discount margin, implied volatility, a price/earnings ratio and a cap rate are
  **statistics derived from a price**, never the mechanism that sets it.
- **The one exception** is an **administered** rate — a central-bank facility rate, a statutory benefit, a
  regulated tariff — and it qualifies only when a real quantity responds to it on both parties' books.
- A price that no mechanism formed is **absent**, and absence is visible (Law 8).

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
currencies is two legs that settle together. **A one-sided flow is a defect even when nothing fails.**

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
that only passes with one is reporting a defect.

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
(a bucket, a band, a tier) ever stands in for the thing that was actually bought.

### Law 10 — Mechanisms do not know what kind of thing they act on

A mechanism behaves the same for every party and instrument of the kinds it applies to. What differs
between kinds — an industry's recipe, a fund's mandate, an instrument's terms, a party's legal form — is
**declared data** about the kind, which the mechanism reads. Adding a product, an industry, an instrument
type or a fund type is a declaration, never a new special case in a mechanism.

### Law 11 — Heterogeneity is load-bearing, and nothing is decided at an average

Parties differ — in endowment, preference, history, information and position — and those differences are
what give a market two sides, make a distribution have tails, and let a shock transmit. Every decision is
taken **by a party from its own state**; an aggregate is always `Σ f(xᵢ)`, never `f(Σ xᵢ)`. There is no
representative agent anywhere a decision has a threshold. Members who are **identical in every respect** may be
carried together as one cohort (A6): its decision is exactly each member's decision, applied to every member,
which is a count and not an average.

### Law 12 — Causality runs forward, and nobody knows more than they could

A decision reads only what its decider could have observed **before** it decided: its own state, its own
history, public information as published (with its lags), and the terms of its own contracts. No party
reads the model's forecast of the future, another party's private state, or the result of the period it is
deciding in. **There is no global expectation**; expectations are personal and they disagree.

### Law 13 — Nothing is immortal, and nothing vanishes

Every kind of party can end — a person, a household, a firm, a bank, a fund, an insurer, a clearing house,
a sovereign's access to markets — each by its own real trigger. Every ending has a **destination** for
every asset, liability, contract, employee and obligation. The one party that cannot fail in its own money
is its issuer, and the reason is its balance sheet, not an exemption.

### Law 14 — Nothing is instant and nothing is free

Moving goods, people or information takes time and costs something; building takes time; settlement has a
date; a decision takes effect no earlier than the next opportunity to act on it. Transport, search, hiring,
switching, issuing, borrowing and failing each have a real cost borne by a named party.

### Law 15 — Chance is declared, seeded and dated

Randomness enters only through **declared hazard processes** — a death, an illness, an accident, a
catastrophe, a breakdown, a discovery, a meeting between a searcher and an opportunity — each with a
declared rate that is a primitive, drawn from a single seeded source so that the same seed reproduces the
same world. Each draw is an **event with a date and named subjects**. **Outcomes are never drawn**: a
default, a price, a vote, a merger or an unemployment rate is always a consequence, never a draw.

### Law 16 — No outcome is imposed

Requirements state reasons; outcomes emerge. No price path, earnings path, growth rate, unemployment rate,
trade balance, election result or narrative is ever written into the world, and no opening state is fitted
to an answer. The only legitimate way to change a primitive during a run is a **declared intervention**
(§N6) or a decision by the party that owns it.

### Law 17 — Looking never changes the world

Observing, measuring, auditing or displaying the world moves nothing: not a balance, not a price, not a
decision. An observer that acts does so as a party, through markets, with its own means, and is checked
like any other. **The audit never repairs**; it reports.

---

# PART II — HOW REQUIREMENTS ARE WRITTEN

### II.1 Requirement types

Every requirement in Parts A–M has an identifier, a type and one sentence of substance, with at most a few
lines of reason. There are seven types:

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

Every system section has the same parts, in the same order:

- **Purpose** — what the system is for, in a paragraph.
- **Depends on** — the systems that must exist before it can work.
- **State, Decisions, Processes** — what exists, who chooses what from what, and what happens by itself.
- **Invariants, Measures, Forbids** — what must hold, what to observe, what must be absent.
- **Primitives** — the declared numbers it needs.
- **Out of scope** — what it deliberately leaves out, with the reason (also listed in Appendix D).
- **Done when** — the acceptance test: the state, behaviour and evidence that show the system exists.

### II.4 Missing and out of scope are different answers

A requirement that is not yet met stays in the document and is **missing**. A thing the model
deliberately does not have is **out of scope**, stated with its reason. A requirement is never deleted or
softened to make a comparison look better.

### II.5 Contract violations and findings

Two kinds of wrongness exist and are treated differently:

- A **contract violation** is an impossible state — a one-sided flow, a negative count of physical units,
  a currency added to another, a decision reading the future, a weight or identity changed by nothing. It
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
- **TIME.2 STATE** — **One calendar**: an epoch, a mapping from days to dates, and per country a declared
  set of **business days** (weekends and holidays are ENDOWMENT). Markets and settlement run on business
  days; hazards, births, deaths and accruals run on every day.
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
  2. **Resolve the past** — accruals post; dues are paid or become arrears; fails from earlier settlement
     are recorded; recognised losses land on named holders; parties that cannot go on cease; estates
     distribute.
  3. **Nature and population** — the day's hazard events are drawn (A4); people are born, die, fall ill,
     move, form and dissolve households; new firms are founded.
  4. **Real work** — production runs and finishes; services are delivered; shipments move and arrive;
     construction progresses; jobs start and end.
  5. **Decide** — every party scheduled or woken today forms its outlook (C2) and posts what it wants:
     orders, quotes, offers, applications, bids, votes.
  6. **Form prices** — every market that meets today forms its prices and matches (C1).
  7. **Settle** — every instruction due today settles or fails (B2).
  8. **Value and judge** — positions are valued; accounts and ratios are read; covenants, margins and
     capital rules are tested; reports and ratings due today are published; calls and demands are issued.
  9. **Close** — the day's remaining payments are settled together where they can be; the audit runs over
     what the day left behind.
- **TIME.7 PROCESS** — **Nothing is demanded and paid in the same stage.** A margin call, a covenant
  demand, a redemption request, a capital-call notice or a policy decision issued in stage 8 is due no
  earlier than the **next business day**; if unmet then, its consequence (a forced sale, a default, a
  close-out) happens in that day's stages. This one-day lag is the clock's, and it is what gives every
  feedback loop its speed.
- **TIME.8 PROCESS** — Non-business days run stages 2–4 only (accruals, nature, population, physical
  processes that do not stop), and no market meets.

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
  across its countries. Every party is either an **individual** or a **cohort** of identical members (A6).
  Institutions, issuers, every firm above small size, and any person, household or small firm in an
  individually significant situation are individuals; the rest of the household and small-firm population is
  carried in cohorts. A cohort is a named party whose weight is the exact count of the real parties it is.
- **PTY.3 STATE** — A **person** has an age, a household, a region of residence, skills, a health state and a
  labour-market state. A **household** is one or more persons who share a budget and a dwelling; it is the
  unit that owns, consumes, saves and borrows. Legal ownership sits with the household; labour, age and
  mortality with the person. A person or household carried in a cohort has exactly its cohort's state.
- **PTY.4 STATE** — Every party has a **legal form**, and the legal form is declared data (Law 10): what it
  may hold, whether its owners have limited liability, whether it may take deposits, how it can end, and
  who its owners are.
- **PTY.5 STATE** — Every party has a **site** on the map (A5), from which its region and country are read;
  a party with several establishments has a site for each.
- **PTY.6 STATE** — Every party has a **home currency** — its country's — and keeps its books in it.
- **PTY.7 STATE** — **Ownership and control are relations between parties**, recorded as holdings of the
  owned party's equity (B3), never as attributes. A **group** is a parent and the subsidiaries it controls
  through those holdings.
- **PTY.8 STATE** — A party's **private state** (positions, limits, intentions, outlooks) is its own; its
  **public state** is what it has published or what is visible by law (M1).

**Processes**

- **PTY.9 PROCESS** — A party **begins** by a named event with a cause (a birth, a founding, a
  registration, a spin-off) and **ends** by a named event with a cause (a death, a dissolution, an
  insolvency, a merger, a resolution). Every ending opens an **estate** or names a **successor** (L3).

**Invariants**

- **PTY.10 INVARIANT** — Every party referenced by any holding, contract, instruction or event exists, or
  has an estate or successor that does.
- **PTY.11 INVARIANT** — Every person belongs to exactly one household; every household has at least one
  living member or is an estate.

**Measures**

- **PTY.12 MEASURE** — **Resolution invariance**: the same world at several resolutions — the cohort budget
  halved and doubled, the merge tolerance halved and doubled, the number of preference types changed (A6) —
  produces the same per-person and per-unit outcomes and the same distributions, within their measured sampling
  error. The size of the difference is the honest error bar on every number the world produces, and a
  difference that grows as resolution is refined is a finding.

**Forbids**

- **PTY.13 FORBID** — No party without an identity, no identity reused, no party that exists only to absorb
  a residual, and no party that cannot end (Law 13) except an issuer in its own money.
- **PTY.14 FORBID** — No weight, share or scale factor applied to a party's decisions or holdings, other than
  a cohort's count of identical members (A6). A party holds what it holds.

**Primitives**

- **PTY.15 PRIMITIVE** — The opening population and its parties (ENDOWMENT); legal forms and what each
  permits (POLICY of the country that defines them); the cohort budget and merge tolerance (RESOLUTION, A6).

**Out of scope**

- Informal and illegal activity; the internal organisation of a household beyond a shared budget.

**Done when**

- Every party in the world has a site, a legal form, a home currency and an owner or members; ending any
  party leaves no reference dangling; the world can be generated at three scales from one seed.

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
- **NUM.4 STATE** — **Differences are primitives too.** Where parties of one kind differ in a preference or
  a technology (patience, risk aversion, tastes, skill, memory), the declared primitive is a **finite set of
  types** with the share of each, from which a party's type is drawn once, at its creation, from the seeded
  source (A4). The number of types is a RESOLUTION, tested by invariance (PTY.12). Finite types are what let
  identical members be carried together (A6); a modest number of types is known to reproduce real wealth
  inequality and spending behaviour.

**Invariants**

- **NUM.5 INVARIANT** — No arithmetic combines two currencies, or two units, except an exchange at a stated
  price or a declared conversion (a recipe, a technology).
- **NUM.6 INVARIANT** — No number is invalid (not-a-number or infinite) anywhere in the world's state.

**Measures**

- **NUM.7 MEASURE** — The **SHAPE count**, over the life of the build, only falls.

**Forbids**

- **NUM.8 FORBID** — No number shapes behaviour without being in the register; no default value standing
  in for a missing one; no absent value read as zero.

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
  existing one, and the same seed reproduces the same world exactly.
- **CHN.2 STATE** — A **hazard process** is declared with: what it acts on (a person, a vehicle, a plant, a
  tile, a policy, a search), its **rate** as a function of declared state (age, health, wear, exposure,
  effort), what an occurrence does, and its source. The rate is a TECHNOLOGY primitive (a life table, a
  failure curve, a catastrophe frequency, a discovery rate) and may be imported from data.

**Processes**

- **CHN.3 PROCESS** — The processes this world has: **mortality** and **illness** of persons;
  **conception** given a household's decision to have a child; **accidents** and **damage** to dwellings,
  plant, vehicles and cargo; **natural catastrophes** on tiles (flood, storm, earthquake, drought, crop
  failure), which can hit many parties at once; **equipment failure**; **discovery** in research;
  **meetings** in search (a job seeker and a vacancy, a buyer and a dwelling for sale, two adults forming a
  household); and **heterogeneity at birth** (drawing a new party's preferences and skills from their
  declared distributions).
- **CHN.4 PROCESS** — Each occurrence is an **event** with a day, named subjects and a size, recorded before
  any party reacts to it.

**Forbids**

- **CHN.5 FORBID** — **No drawn outcome.** No default, price, vote, merger, bank run, hiring total or
  growth rate is ever drawn. Chance decides what happens to a party's circumstances; the party and the
  markets decide what follows.
- **CHN.6 FORBID** — No unseeded randomness and no dependence on iteration order of anything unordered.

**Measures**

- **CHN.7 MEASURE** — The realised frequency of each hazard matches its declared rate over long runs; a
  catastrophe's losses are clustered in place and time, which is the point of having one.

**Done when**

- Two runs from one seed are identical; a new hazard process leaves every existing draw unchanged; every
  hazard occurrence can be listed as a dated event.

---

## A5. GEO — The physical world

**Purpose.** The map: where parties, plant, dwellings, deposits and routes are; what separates them; what
the ground holds; and what nature can do to a place. Geography creates distance, barriers, resources and
exposure; it never writes an economic outcome.

**Depends on:** TIME, PTY, NUM, CHN.

**State**

- **GEO.1 STATE** — The world is a finite grid of **tiles**, each with a stable identity, a coordinate on one
  declared projection, a surface (land or water), an elevation and a terrain class. Adjacency is read from
  the grid, one way, for everybody.
- **GEO.2 STATE** — **Distance** is a physical length derived from coordinates and the projection; a path's
  length is the sum of its legs. Grid steps and labels are not distance.
- **GEO.3 STATE** — A **country** is a jurisdiction over a set of tiles, with a currency, laws and a state
  (Part J). A **region** is a set of tiles within one country and is where local markets (labour, housing,
  services, retail) meet. A **site** is the exact tile on which a party, plant, dwelling, warehouse, port or
  piece of infrastructure stands; country and region are read through the site.
- **GEO.4 STATE** — **Infrastructure** — roads, rail, bridges, tunnels, ports, pipelines, power lines — is
  owned capital (E3) with a site or a path, a capacity shared by everything using it in a day, a life, a
  maintenance need and a condition.
- **GEO.5 STATE** — **Land** is a tile's area, owned by a named party as a holding, with what stands on it.
  A dwelling or a plant occupies land.
- **GEO.6 STATE** — A tile may hold a **deposit**: a named resource at a grade, in a declared finite
  quantity or declared unbounded. The right to extract it is a holding like any other (F1).
- **GEO.7 STATE** — Every tile has an **exposure** to each natural hazard (A4), from its terrain, elevation,
  water and climate class: what a flood, storm or drought there does to what stands on it.

**Processes**

- **GEO.8 PROCESS** — A catastrophe on a tile damages or destroys what stands there — dwellings, plant,
  stock, infrastructure, crops — as real losses of units at their owners, and can close routes (F3).
- **GEO.9 PROCESS** — A finite deposit **depletes** by what is extracted and never refills; where its nature
  is that the richest part goes first, its grade falls with what has been taken.
- **GEO.10 PROCESS** — The opening map is **generated** from the seed by a recorded procedure with declared
  SHAPE parameters (world size, sea level, terrain roughness); a generated world that fails a declared
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

- **GEO.18 PRIMITIVE** — Projection and world size (RESOLUTION); terrain generation parameters (SHAPE,
  declared as such, with the reason no mechanism replaces them); deposits and opening infrastructure
  (ENDOWMENT); hazard exposure by terrain (TECHNOLOGY).

**Done when**

- Parties, plant and dwellings stand on sites; distances between any two sites are paths over real terrain;
  deposits deplete exactly; a catastrophe destroys named units at named owners.

---
## A6. REP — How populations are represented

**Purpose.** How a world of hundreds of millions of people and millions of small firms is carried exactly
enough to be true and compactly enough to run on a phone: identical members are counted, not copied; chance
acts on members, not on groups; a member becomes an individual when something individual happens to it; and
the cost of the world follows the number of distinct situations, not the number of people.

**Depends on:** PTY, NUM, CHN.

**State**

- **REP.1 STATE** — A **cohort** is a named party standing for a **count** of real households or small firms
  that are **identical in every respect**: every discrete attribute (type, age, region, bank, job state,
  health, household composition), every balance, every holding and every contract. Its **weight** is that
  count. Everything it holds is its weight times what one member holds, and every movement to or from it is
  its weight times one member's movement.
- **REP.2 STATE** — An **individual** is a party of weight one. Every institution, issuer and firm above small
  size is always an individual. A household or small firm is an individual while it is **materialised**
  (REP.6).
- **REP.3 STATE** — A cohort's contracts are **contract lines**: one record standing for its weight of
  identical contracts with the same named counterparty — a line of identical deposits, identical loans,
  identical tenancies, identical employment contracts. The counterparty holds the other side of the same line.
- **REP.4 STATE** — The **cohort budget** is the number of cohorts and individuals the world may carry at once,
  and the **merge tolerance** is how close two cohorts' continuous balances must be to be carried together
  (REP.9). Both are RESOLUTION primitives, tested by invariance (PTY.12).

**Decisions**

- **REP.5 DECISION** — A cohort decides **once, as each of its members would**, and the decision applies to
  every member. Where members' situations would lead to different answers, they are not identical and are not
  in one cohort.

**Processes**

- **REP.6 PROCESS** — **Materialisation.** A member is split out as an individual, with exactly its cohort's
  state, when something happens to it alone: it enters a negotiation with a named counterparty (a loan, a
  dwelling sale, a takeover), falls into arrears or default, founds or closes a business, is hired or separated
  in a way its cohort is not, or is watched by a player. Its cohort's weight falls by one in the same step.
- **REP.7 PROCESS** — **Chance acts on members.** A hazard (A4) acting on a cohort of weight _w_ draws **how many
  of its members** it hits that day, from the binomial distribution with that hazard's probability, using the
  cohort's own seeded stream for that event and day; those members split into a new cohort (or materialise).
  Counts can never go negative, events are never lumps of the whole weight, and the result does not depend on
  the order anything else runs in.
- **REP.8 PROCESS** — **Rejoining.** Two cohorts, or an individual and a cohort, whose states become identical
  combine into one, with their weights added.
- **REP.9 PROCESS** — **Controlled merging.** When the cohort budget binds, cohorts that are identical in every
  discrete attribute and every contract, and whose continuous balances differ by no more than the merge
  tolerance, are merged. A merge **conserves every total exactly**: the members keep their total count, and
  each balance's total `T` is spread over the `W` members in whole units, `q = T div W` each with the remainder
  `r = T mod W` members holding `q + 1` — at most two cohorts, and not one unit created or lost.
- **REP.10 PROCESS** — **The tails are protected.** Members near a threshold that matters — near default, near
  a covenant, near a mandate boundary, at the top of the wealth or firm-size distribution — are never merged;
  they are carried as individuals or small cohorts, because the tails are where the world's crises and
  inequality live.
- **REP.11 PROCESS** — **The shadow sample.** A small random set of members, drawn from the seeded source, is
  carried at full resolution beside the cohorts they came from — never merged — and the difference between
  their paths and their cohorts' paths is the measured cost of merging (REP.15).
- **REP.12 PROCESS** — **Only what is active is touched.** A cohort with nothing scheduled and no event does
  nothing on a day, so the work of a day follows the number of cohorts that act, not the population.

**Invariants**

- **REP.13 INVARIANT** — The sum of weights of every population equals its population, exactly, every day, and
  every real member is in exactly one cohort or is one individual.
- **REP.14 INVARIANT** — Every cohort's holdings and contract lines are divisible by its weight; no merge,
  split or materialisation changes any total of money or units.

**Measures**

- **REP.15 MEASURE** — The shadow sample's divergence from its cohorts, the number of cohorts and individuals,
  merges and materialisations per day, and the share of the population carried as individuals.

**Forbids**

- **REP.16 FORBID** — No cohort whose members differ in any discrete attribute or contract; no merge outside
  the declared tolerance; no merge that creates or destroys a unit; no cohort holding what no member could
  hold; no event applied to a whole cohort when it hits only some of its members.
- **REP.17 FORBID** — No weight that is a share, a scale factor or a probability: a weight is a count, changed
  only by entry, death, split, materialisation, rejoining or merging.

**Primitives**

- **REP.18 PRIMITIVE** — The cohort budget and merge tolerance (RESOLUTION); the thresholds that protect the
  tails (RESOLUTION); the size of the shadow sample (RESOLUTION).

**Done when**

- A world of hundreds of millions of people carries its population exactly; a hazard hits some members of a
  cohort and not others; a member who takes out a mortgage is an individual for as long as it matters; merging
  keeps every total exact, and the shadow sample reports what merging costs.

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
- **MON.4 STATE** — **Cash** is banknotes: issued by the central bank to banks against reserves, withdrawn
  by depositors against deposits, used in payments between parties, and deposited back. A party holding
  cash holds a claim on the central bank that no bank failure touches.

**Processes**

- **MON.5 PROCESS** — A **payment** is an instruction: payer, payee, amount, currency, reason, date. It
  settles by one rule (B2): payer minus, payee plus.
  - Between depositors of **one** bank it is a transfer of that bank's liability and moves no reserves.
  - Between depositors of **different** banks it moves the same amount of reserves between the two banks.
  - A payment in cash moves banknotes between the two parties.
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
  change by source are reads, published with a lag (M2).

**Forbids**

- **MON.11 FORBID** — No money without an issuer; no balance that is nobody's liability.
- **MON.12 FORBID** — No silent negative balance: a shortfall is a loan somebody agreed or a failed payment
  somebody can see.
- **MON.13 FORBID** — No conversion at the ledger: money paid in one currency arrives in that currency, and
  changing it is a separate trade with a counterparty (K1).
- **MON.14 FORBID** — No bearer money held by nobody: every banknote has a named holder in the world, even
  though its real-world counterpart would not.

**Primitives**

- **MON.15 PRIMITIVE** — Overdraft terms are contract terms set by the lending bank; the central bank's
  intraday and overnight credit terms are its POLICY.

**Out of scope**

- Private currencies, crypto-assets and payment-system outages. Banknotes carry no theft or loss hazard.

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
- **SET.9 INVARIANT** — Every settled trade has both legs; every leg names both parties.

**Measures**

- **SET.10 MEASURE** — Gross and net settlement values, fails by cause, and the size of the closing ring are
  published per day.

**Forbids**

- **SET.11 FORBID** — No move of anything without an instruction; no instruction with one side; no partial
  settlement of a trade; no instruction applied twice.

**Memory of the world**

- **SET.12 STATE** — **Snapshots.** The complete state of the world — every party, holding, lot, lien, contract,
  commitment, outlook, pending event and the seed streams' positions — is recorded as a **snapshot** at a declared
  interval (initially every simulated month-end) and at every save. A snapshot is the state itself, not a summary
  of it.
- **SET.13 STATE** — **Retention.** Individual instructions are kept for a declared **retention window**
  (initially the current and the previous snapshot interval). Older instructions are released once a snapshot
  after them exists. What outlives the window is what the world itself keeps: published statistics (M2), published
  statements and reports (ACC.9, RAT.2), prints, dated events, and each party's own bounded memory of what it
  observed (VAL).
- **SET.14 PROCESS** — **Replay.** Any day inside the retention window can be rebuilt exactly by loading the
  nearest earlier snapshot and re-applying the retained instructions with the same seed streams.
- **SET.15 INVARIANT** — A world restored from a snapshot and stepped forward is identical, day for day, to the
  world that was never interrupted.
- **SET.16 FORBID** — No fact that a decision reads may exist only in a released instruction: anything a party
  needs later — a basis, a credit record, a contract's history of arrears — is part of the state, and so is in
  every snapshot.

**Done when**

- Every holding can be replayed from the nearest snapshot and the retained instructions; a restored world runs
  identically to an uninterrupted one; a fail has a cause and a consequence; securities trades settle on their
  convention's date.

---

## B3. REG — Holdings and instruments

**Purpose.** What can be held, who holds it, what it is a claim on, and what binds it.

**Depends on:** SET.

**State**

- **REG.1 STATE** — A **holding** is (holder, instrument or asset, quantity in its own unit), and the holding
  is a chain of **lots**, each with the date and price at which those units were acquired (its basis).
- **REG.2 STATE** — A **lien** marks units of a holding as pledged to a named party; **free units** are held
  units minus pledged units, and only free units can move or be pledged again. A re-pledge is a traceable
  chain.
- **REG.3 STATE** — Every instrument has an **issuer** (or, for a real asset, none), an **issued amount**
  changed only by issuance, reopening, buyback, conversion, amortisation, maturity or default, and **terms**
  that belong to its **instrument family** (REG.5–REG.10).
- **REG.4 STATE** — The register answers both directions at once: what a party holds, and who holds an
  instrument.
- **REG.5 STATE** — **Debt claims.** Any instrument that promises money has: an issuer; a principal in units
  of face; a currency; a maturity (or perpetuity); a coupon of a stated form — fixed, floating over a named
  reference rate that is itself transacted, zero, **indexed** to a published index, **step-up**, or
  **payable in kind**; a payment schedule with a day-count convention; an early-termination regime (none,
  callable, putable, make-whole, convertible); a definition of default observable by a holder; a claim on
  failure and its **seniority**; and any **conversion or write-down** term (a convertible into shares at a
  ratio, a contingent instrument that converts or writes down when a named ratio crosses a stated level).
- **REG.6 STATE** — **Equity.** A share is a residual claim on its issuer, in a class (ordinary,
  preferred), counted in shares, perpetual, carrying votes by class and, for preferred, a stated dividend
  that ranks ahead of ordinary shares. Its value to a holder is never below zero (limited liability), which
  is a property of the legal form (PTY.4), not a bound.
- **REG.7 STATE** — **Fund units.** A claim on a fund's assets, counted in units, redeemable or tradeable by
  the fund's terms (H6).
- **REG.8 STATE** — **Contracts between two parties** — loans, deposits, leases, tenancies, employment,
  insurance policies, derivative contracts, trade-credit invoices, guarantees — are recorded as a contract
  with both parties, its terms and its schedule, an asset to one side and a liability to the other.
- **REG.9 STATE** — **Real assets** — goods, plant, vehicles, dwellings, land, infrastructure, extraction
  rights — are holdings in physical units with no issuer, and plant and dwellings are **individual units**
  that keep their identity, site and condition through every sale.
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
- **REG.14 INVARIANT** — For every two-party contract, the asset on one side and the liability on the other
  are one record, read from two sides.
- **REG.15 INVARIANT** — No holding of physical units is negative; free units are never negative.

**Forbids**

- **REG.16 FORBID** — No holding without a holder; no claim without an issuer; no short position without a
  borrow from a named lender (H7); no unit pledged twice.
- **REG.17 FORBID** — No invented grouping held in place of an instrument: a party holds the issue it bought.

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
  disclosed and applied consistently; last-in-first-out is not permitted.

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
  period **minus capital transactions with owners** (paid in, returned, distributed).
- **ACC.12 INVARIANT** — Receivables equal payables, contract by contract, across the whole world.

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
  **form** (MKT.3–MKT.7), **meeting days**, a **settlement convention**, and **who may take part**.
- **MKT.2 STATE** — A **print** is a formed price with its market, instrument, day, unit, currency, quantity
  and form. A print is public (M1). A market that formed no price has **no new print**, and its last print
  shows its age.

**The forms** — every market is one of these, declared as data (Law 10):

- **MKT.3 PROCESS** — **Call auction.** Participants post **schedules** (how much at each price); one price
  is found where posted supply meets posted demand, and quantity is rationed at that price by a stated
  rule (pro rata or priority). Used where a market meets at a moment: a sovereign or corporate primary
  issue, an opening, a commodity session, an interbank session.
- **MKT.4 PROCESS** — **Continuous book.** Limit orders rest; an incoming order that crosses a resting one
  trades at the resting price, in price-then-time priority. Used for listed shares, funds that trade, and
  futures. The day's closing print is the last trade.
- **MKT.5 PROCESS** — **Dealer market.** Dealers post two-way quotes from their own inventory, funding and
  risk (H7); clients trade on those quotes or ask several dealers and take the best; dealers trade with each
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
  quantity that comes to it. It is the exception of Law 3 and exists only with that quantity response.

**Processes common to every form**

- **MKT.9 PROCESS** — Every participant's order, quote or offer comes from its **own state**: its position,
  its cost of funds, its constraints, its outlook and its value of the thing (C2). A participant who does
  not want to trade does not post, and that is an outcome.
- **MKT.10 PROCESS** — **A market can fail.** No overlap, no bid, no seller, a dealer that stepped back: each
  is a recorded outcome with consequences that propagate — the issuer is not funded, the seller keeps its
  stock, the borrower is refused, the vacancy stays open.
- **MKT.11 PROCESS** — Every match becomes an **instruction** (B2) with two named sides, settling on the
  market's convention.
- **MKT.12 PROCESS** — A print becomes the **mark** for everybody whose carrying basis marks to it (ACC.2),
  in the market where the holder's units are.

**Invariants**

- **MKT.13 INVARIANT** — In every market, quantity bought equals quantity sold, and money paid equals money
  received, per match.
- **MKT.14 INVARIANT** — A print always came out of a match; a quote, an order, a reservation or a search
  bound is never a print.

**Measures**

- **MKT.15 MEASURE** — Depth, bid–offer width, turnover, the share of meetings that failed to form a price,
  and the age of last prints, per market.

**Forbids**

- **MKT.16 FORBID** — **No price-taker of a price not yet formed**: no order written against the price the
  mechanism is about to produce. A party that wants to trade "at market" posts a limit it chose.
- **MKT.17 FORBID** — **No buyer or seller of last resort by construction**: no participant whose order is
  "whatever is left, at any price", and the mechanism never adds demand or supply to make itself clear.
- **MKT.18 FORBID** — No price from a written path, a formula, a target, a parity condition or another
  price's statistic.
- **MKT.19 FORBID** — No spread applied to a mid, and no stated spread table: a bid–offer is what dealers
  posted.

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

**Processes**

- **VAL.5 PROCESS** — **Outlooks are formed from what the party observed**: its own receipts, payments,
  fills and holdings; public prints and published statistics as published, with their lags; announcements
  (an auction calendar, a policy decision, a new tax rate, a firm's guidance); and the terms of its own
  contracts. Nothing else.
- **VAL.6 PROCESS** — **The heuristic menu.** Each outlook is produced by one or more simple, fallible rules
  a real decider uses:
  - **adaptive** — last outlook corrected toward what happened, at the party's own speed (its memory);
  - **trend-following** — extrapolate the recent change;
  - **anchoring to a reference** — expect a return toward a level the party has observed over a long
    window, or toward a published target;
  - **announcement** — take a published, dated change (a tax rate, a contract reset, a policy rate
    decision) into the outlook from the date it applies.
- **VAL.7 PROCESS** — **Heuristic switching.** A party weights its heuristics by their recent forecasting
  performance **for that party**, updating on its own schedule; the intensity with which it switches is a
  PREFERENCE. Parties that have seen different histories therefore hold different outlooks, and the mix of
  heuristics across a market shifts with what has recently worked — which is a known source of boom,
  bust, fat tails and volatility clustering.
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
  last dwelling sold nearby, published statistics), and learns from there.

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
- **VAL.20 FORBID** — **A value never becomes a price**: it does not print, mark a position, enter an index,
  or stand in for a price anywhere one is required.
- **VAL.21 FORBID** — **No common value**: no single valuation shared by every party of a kind. A world where
  everybody values alike trades once and stops.

**Primitives**

- **VAL.22 PRIMITIVE** — Finite type sets across parties of memory, switching intensity, required return
  (patience) and risk aversion (PREFERENCE, NUM.4); the heuristic menu (SHAPE, declared as the accepted stand-in for
  how people actually forecast, with its source in the experimental and behavioural literature).

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

**Depends on:** PTY, CHN, GEO.

**State**

- **POP.1 STATE** — Every **person** has a birth date, a household, a region, a health state, a skill
  profile (per occupation family), an education record, a labour-market state (A person is in exactly one
  of: in education, employed, unemployed and searching, out of the labour force, retired) and an
  employment history. A person in a cohort has these as its cohort does (A6).
- **POP.2 STATE** — A **household** has members, a dwelling (owned, rented, or shared with another
  household), a budget, holdings and debts, and its own preferences drawn at its formation (NUM.4).

**Processes**

- **POP.3 PROCESS** — **Death**: each person faces a mortality hazard by age and health (A4), from a declared
  life table. A death is an event; the person's share of the household's claims passes by the household's
  rules and by inheritance law (POP.9), and a household with no surviving member becomes an estate.
- **POP.4 PROCESS** — **Illness and disability**: a health hazard by age can make a person unable to work for
  a spell or permanently, which is what sickness and disability insurance and benefits respond to.
- **POP.5 PROCESS** — **Birth**: a household **decides** whether to try for a child (POP.10), and the
  realisation follows a declared conception hazard (A4). A child joins its parents' household, costs
  consumption, and is in education until it leaves it.
- **POP.6 PROCESS** — **Education and skill**: a young person's education continues while its household
  chooses and pays for it (or the state provides it, J3); schooling raises skill in an occupation family by
  a declared technology; experience on the job raises it further and unemployment erodes it.
- **POP.7 PROCESS** — **Household formation and dissolution**: an adult leaves its parents' household when it
  can afford a dwelling of its own (a decision, HH.8); two adults form a household through a search-and-
  meeting process within a region (A4); a household dissolves by separation or by the death of its last
  member. Each is a dated event, and the household's holdings and debts are divided by declared law.
- **POP.8 PROCESS** — **Migration**: a household (or an adult leaving one) **decides** to move to another
  region or country when its own expected income, housing cost and prospects there, less the cost of
  moving, beat staying (HH.9); a move across a border needs the destination's admission (POLICY, K2).
- **POP.9 PROCESS** — **Inheritance**: an estate distributes to named heirs by declared law (POLICY), after
  debts, taxes and the estate's costs.

**Decisions**

- **POP.10 DECISION** — A household's decision to have a child reads its income, its dwelling, its members'
  ages, its outlook and its own preference for children; it is never a birth rate.

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
  (TECHNOLOGY); preference for children (PREFERENCE distribution); schooling-to-skill technology;
  inheritance, education and migration law (POLICY).

**Out of scope**

- Sex-specific demography beyond what the life tables carry; partnership preferences beyond region and
  age; crime; intra-household bargaining.

**Done when**

- A world run for decades has a population whose size, age structure and regional spread are outcomes of
  births, deaths, formation and migration; a region that loses its jobs loses people.

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

- **HH.4 DECISION** — **How much to spend** this period, from: its current income, its wealth and how
  liquid it is, its outlook for income and prices, its confidence, its patience, the rate it earns on
  saving, and what it can borrow. A household that cannot borrow spends only what it has.
- **HH.5 DECISION** — **What to buy**: spending is divided across goods and services by its tastes and the
  prices it faces where it can shop, substituting as relative prices move; it buys from sellers it can
  reach, comparing the posted prices it sees, and it pays consumption tax at the till.
- **HH.6 DECISION** — **Whether and how much to work**: each adult chooses to search, accept an offer, stay,
  quit, reduce hours, retire, or leave the labour force, from the wage it can get, its outside options
  (benefits, other income, its partner's income), its value of leisure and its outlook (F4).
- **HH.7 DECISION** — **How to hold its savings**: across deposits, cash, bills held directly, money funds,
  bond and equity funds, shares, dwellings and pensions, by yield, risk, liquidity and what it trusts,
  rebalancing on its own schedule — which is how a policy rate reaches a saver who is not a borrower.
- **HH.8 DECISION** — **Where to live**: rent or buy, where, and how large, from its income, its savings for
  a deposit, what lenders will offer it, rents and prices in reach, and its outlook for both (F5).
- **HH.9 DECISION** — **Whether to move** region or country (POP.8).
- **HH.10 DECISION** — **Whether to borrow**, for a dwelling, a vehicle, consumption or a shortfall, and from
  whom, among the offers it can get (G1).
- **HH.11 DECISION** — **Whether to insure**, against what, and with whom, at the premiums quoted (I3).
- **HH.12 DECISION** — **How to vote** (J6).

**Processes**

- **HH.13 PROCESS** — Debt service is paid from the household's accounts on its dates; a household that
  cannot pay falls into **arrears**, and continued arrears are a **default** with its contract's
  consequences: collection, repossession, a credit record that later lenders read.
- **HH.14 PROCESS** — A household in arrears **acts**: cuts spending, draws savings, sells assets, borrows
  elsewhere, moves, sends another member to work.

**Invariants**

- **HH.15 INVARIANT** — Every household's spending reaches named sellers; every unit of income came from a
  named payer.

**Measures**

- **HH.16 MEASURE** — The marginal propensity to consume differs across the wealth and liquidity
  distribution; the saving rate, wealth and income distributions, household leverage and debt-service
  burden; defaults by household type — all reads.
- **HH.17 MEASURE** — A mean-preserving spread of household incomes raises defaults and changes aggregate
  spending while the mean income does not move.

**Forbids**

- **HH.18 FORBID** — No representative household; no consumption function applied to an aggregate; no
  decision evaluated at an average (Law 11).
- **HH.19 FORBID** — No household as a residual holder of what nobody else took: it holds only what it chose
  to buy.

**Primitives**

- **HH.20 PRIMITIVE** — Preference distributions (PREFERENCE, estimated from data where possible); decision
  schedules (PREFERENCE); minimum consumption needs by household composition (TECHNOLOGY of living).

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

**Depends on:** NUM, CHN, GEO.

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
- **TEC.4 STATE** — Each firm holds the **ways it knows**. Knowing a way is a firm's asset: it can be
  discovered, licensed, imitated, or lost when a firm dies without a successor.

**Processes**

- **TEC.5 PROCESS** — **Research**: a firm that spends on research (employing researchers and buying
  services) faces a **discovery hazard** rising with that effort (A4); a discovery is a new way, or an
  improvement to one it knows, drawn from a declared distribution of improvements around its current
  best. Discovery is dated, owned and uncertain.
- **TEC.6 PROCESS** — **Imitation and diffusion**: a firm can learn a way another firm uses, by licensing it
  (a contract with a price, MKT.7) or by imitation effort facing its own hazard, which rises with how close
  and how visible the other firm is. Patents are a declared POLICY that sets how long imitation is barred.
- **TEC.7 PROCESS** — **Learning by doing**: the labour a way needs per unit falls with the firm's
  cumulative output on it, by a declared curve.
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
  (TECHNOLOGY); patent life (POLICY).

**Done when**

- A firm can switch ways when relative prices change; research spending produces dated discoveries; a
  productive way spreads across firms over time; long runs can grow.

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
- **FRM.3 STATE** — A firm may be a **parent or subsidiary** in a group: it controls another through a
  majority of its votes; intra-group loans, sales and guarantees are real contracts; each member keeps
  limited liability unless it has guaranteed another's debts.

**Decisions** — each from the firm's own state, outlook and the prices it faces:

- **FRM.4 DECISION** — **What to produce and how much**: expected demand, stock on hand, capacity, inputs
  available, labour available and the margin at expected prices. The quantity is the outcome.
- **FRM.5 DECISION** — **What to charge**: a firm selling by posted price revises its price on its own
  schedule from its costs, its stock, how fast it has been selling, what it sees competitors charge and its
  own outlook; it prices above unit cost when it can and below when it must clear stock.
- **FRM.6 DECISION** — **Which way to run**, among the ways its plant supports, at the input and labour prices
  it faces.
- **FRM.7 DECISION** — **Whom to employ** and at what wage (F4); **what to buy**, from whom and on what terms
  (F1, F6).
- **FRM.8 DECISION** — **Whether to invest**, in what, and how to fund it (E3).
- **FRM.9 DECISION** — **How to fund itself**: retained cash, trade credit, a bank loan, a bond, commercial
  paper, new shares; by what each costs it now and how close it is to the leverage its management will
  tolerate.
- **FRM.10 DECISION** — **What to pay out**: dividends and buybacks from cash after what is due, by its
  management's policy and what the owners expect.
- **FRM.11 DECISION** — **Which lines to be in**: enter a product by investing in the plant and knowing a way
  (E3), and exit one it cannot make pay.
- **FRM.12 DECISION** — **What to do in distress**: cut costs and staff, sell assets, draw lines, raise
  expensive money, ask suppliers for time, negotiate with lenders, seek a buyer.

**Processes**

- **FRM.13 PROCESS** — **Revenue** is quantity sold times price achieved, recognised on delivery, from named
  buyers. **Costs** are named lines with named payees: inputs, wages, energy, rent, services, interest,
  taxes, depreciation. **Operating profit** is a read, and can be negative.
- **FRM.14 PROCESS** — **Unit cost** is the inputs a batch consumed at their own cost, plus its labour, plus
  the capital charge its plant's week carries, over the units that survive. A line run below its rate
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

- **CAP.1 STATE** — A **capital good** is an individual unit with a kind, a site, a service date, a
  capacity, a condition and a remaining life. Kinds differ, and a use needing several kinds is limited by
  the scarcest.
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
  and to the unit; it can fail (A4), be damaged by a hazard (GEO.8), or be repaired.
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

**Depends on:** FRM, TEC, MKT, GEO.

**State**

- **GDS.1 STATE** — A good is keyed by **grade and place**: the same grade in two places is two things with
  two prices, and the gap between them is what carrying it costs (F3).
- **GDS.2 STATE** — Stocks are holdings of lots at a site, each with its cost (ACC.6); goods in transit are
  their owner's, pledged to the carrier until they arrive (F3).
- **GDS.3 STATE** — A **commodity** is a standardised good extracted from a deposit (GEO.6) or grown on land,
  whose grade is the deposit's or the land's. The **right to extract** is a holding over a named tile.

**Decisions**

- **GDS.4 DECISION** — **Producers** decide output (FRM.4); an extractor decides how much of its deposit to
  work, weighing today's price against its outlook (depletion makes waiting a real choice).
- **GDS.5 DECISION** — **Buyers** buy inputs from their own needs for planned production and the stock they
  want to hold, up to what the input is worth to them in use (VAL.8).
- **GDS.6 DECISION** — **Stockists and merchants** buy, hold and sell for profit, pay for storage, bear
  spoilage, and move goods where they fetch more (F3).

**Processes**

- **GDS.7 PROCESS** — **Wholesale markets** for goods between firms are call auctions or dealer markets at
  each place (MKT.3, MKT.5); **retail** reaches households through distributors (F2).
- **GDS.8 PROCESS** — **Spoilage and shrinkage** remove units without a sale at their own cost; **storage**
  is a service bought from whoever owns the room. The two are different things and never one number.
- **GDS.9 PROCESS** — Weather and catastrophes (A4, GEO.8) destroy crops and stocks at named places, which is
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

**Done when**

- Services are produced and consumed the same day; unused capacity perishes; retail margins move when
  wholesale costs change faster than retail prices.

---

## F3. FRT — Freight and logistics

**Purpose.** Moving goods between places costs time and money and needs a vehicle, a route and capacity;
this is what makes location matter.

**Depends on:** GEO, CAP, MKT, SET.

**State**

- **FRT.1 STATE** — A **vehicle** is an individual capital unit with a site, a capacity, a speed, a running
  cost per voyage and a keeping cost per day. Its room is available only on routes starting where it is.
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

- **LAB.1 STATE** — An **employment contract** is a row: employer, employee (a person), occupation, hours,
  wage, start date, notice and severance terms. The wage bill, headcount, unemployment and flows between
  states are reads of these rows.
- **LAB.2 STATE** — A **vacancy** is an employer's posted offer: occupation, skill required, hours, wage,
  region; it is open until filled or withdrawn.
- **LAB.3 STATE** — Labour is **heterogeneous** by occupation, skill and region, and a job in one is not a
  job in another.

**Decisions**

- **LAB.4 DECISION** — An **employer** posts vacancies when an extra worker is worth more to it than the wage
  (its output price, its capacity, its outlook), choosing the wage it offers from what it has been able to
  fill before, and **lays off** when it is sure it cannot use the work, paying what the contract owes.
- **LAB.5 DECISION** — A **searcher** applies to vacancies it can see (within its region and search reach)
  up to its search effort, and **accepts** the best offer that beats its own reservation — built from its
  outside option (benefits, other household income), its value of leisure and its outlook. It may also
  move region (POP.8) or retrain (POP.6).
- **LAB.6 DECISION** — An **employee** quits for a better offer, retires, or reduces hours when that is better
  for its household.
- **LAB.7 DECISION** — An employer **selects** among its applicants by skill and experience; a worker who is
  not chosen keeps searching.

**Processes**

- **LAB.8 PROCESS** — **Search is individual**: each application is a real meeting between a named person and
  a named vacancy, arriving through a declared search process (A4), and the number of matches is the sum of
  individual acceptances — never an aggregate matching function.
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

- **LAB.16 PRIMITIVE** — Search effort and reach (TECHNOLOGY/PREFERENCE); the meeting hazard per application
  (TECHNOLOGY); notice, severance and minimum-wage law (POLICY); union coverage (ENDOWMENT).

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

**Depends on:** HH, GEO, CAP, G1 (mortgages).

**State**

- **HSG.1 STATE** — A **dwelling** is an individual unit on a site with a size, a quality and a condition,
  owned by a named party, occupied by at most one household.
- **HSG.2 STATE** — A **tenancy** is a contract between an owner and a household: rent, term, notice, deposit.
- **HSG.3 STATE** — **Land** is owned, zoned (POLICY: what may be built on it) and traded; its price is formed
  in its own market.

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
  own book, its funding and its outlook, and tightens when worried (G1).

**Processes**

- **HSG.10 PROCESS** — **Sales are matched by search and negotiation**: a listed dwelling meets buyers
  through a declared search process (A4); an offer is accepted, countered or refused; a listing that finds no
  buyer stays listed or is withdrawn — so in a falling market volumes fall before prices.
- **HSG.11 PROCESS** — **Foreclosure** moves a dwelling from a defaulting owner to the lender or its agent,
  who sells it; the forced sale adds supply, which pushes prices down further.
- **HSG.12 PROCESS** — Dwellings depreciate, need maintenance, and can be destroyed by hazards (GEO.8).

**Invariants**

- **HSG.13 INVARIANT** — Every dwelling has one owner and at most one occupying household; every household
  is housed or recorded homeless.
- **HSG.14 INVARIANT** — Mortgage debt owed by households equals mortgage assets held by lenders and
  securitisation vehicles.

**Measures**

- **HSG.15 MEASURE** — House prices, rents and their ratio; transactions and time on market; the response of
  prices to mortgage rates and standards; boom–bust cycles with volumes leading prices.

**Forbids**

- **HSG.16 FORBID** — No house price path; no reservation floor; no mortgage without a lender's balance sheet
  on the other side; no dwelling without an owner.

**Primitives**

- **HSG.17 PRIMITIVE** — Construction technology and lead times (TECHNOLOGY); zoning and property law
  (POLICY); the opening housing stock (ENDOWMENT).

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
  the seller's receivable and the buyer's payable, one record.

**Decisions**

- **TCR.2 DECISION** — A seller decides **whether to give terms, and how long**, per buyer, from what it has
  seen of that buyer, and tightens when worried.
- **TCR.3 DECISION** — A buyer decides whether to take the discount or use the credit; a seller may sell
  its receivables to a bank (factoring, MKT.7).

**Processes**

- **TCR.4 PROCESS** — Late payment is a real state that stresses the seller's cash; a buyer's default makes
  the receivable an unsecured claim in its estate, and the seller's loss can push it into distress.

**Invariants**

- **TCR.5 INVARIANT** — Receivables equal payables, invoice by invoice, across the world.

**Measures**

- **TCR.6 MEASURE** — Days of payment, their lengthening in downturns, and chains of distress running from
  buyer to supplier.

**Forbids**

- **TCR.7 FORBID** — No sale that settles instantly by construction; no receivable that survives its debtor.

**Done when**

- A buyer's failure propagates losses to its suppliers by name; suppliers withdraw terms from buyers they
  doubt.

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
  consumer purposes) and, as mortgages, against dwellings; to a cohort they are a contract line (REP.3).
- **BNK.18 STATE** — A **revolving facility** or **credit line** (to a firm, or a household's credit card or
  overdraft) is drawn and repaid at the borrower's choice up to a limit; the lender can cut the undrawn limit
  where the contract allows, which is how credit tightens for borrowers who already have lines.

**Decisions**

- **BNK.4 DECISION** — A bank **quotes** a borrower a rate built from: its own marginal cost of funds (G2),
  its own assessment of the borrower's default probability and loss given default, the capital the loan
  consumes times the return it requires on that capital, and its cost of making the loan. Its assessment is
  its own opinion, formed from what it has seen of the borrower.
- **BNK.5 DECISION** — A bank **declines** when the loan is not worth it, when the borrower fails its
  standards (loan-to-value, income multiple, coverage), or when its own capital or liquidity will not carry
  it; it **tightens** its standards when its own losses, funding or outlook worsen.
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
  (PREFERENCE of each bank's management); standards set by each bank (outcomes of BNK.5).

**Done when**

- Every loan is a contract with two named sides; a bank refuses as well as lends; a borrower's default
  follows its own cash failure and is worked out or written off as dated events.

---

## G2. BFL — Bank funding and liquidity

**Purpose.** A bank's funding is a mix of liabilities that can leave at different speeds, and a bank can
fail for want of cash while still solvent.

**Depends on:** BNK, MON, H1 (money market).

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
  what the market prints, and what other depositors do visibly.

**Processes**

- **BFL.8 PROCESS** — A bank's reserve position each day is the **residue of everybody's payments**; nobody
  chose it.
- **BFL.9 PROCESS** — **A run**: withdrawals take reserves with them, which forces sales and dearer funding,
  which is observable and prompts further withdrawals. Deposit insurance (J5) removes the reason for insured
  depositors to run but not for uninsured ones.
- **BFL.10 PROCESS** — **A bank that cannot pay** — its reserve account short after the market and the central
  bank's facility have both run — has failed for liquidity, whatever its capital, and goes to resolution
  (J5).

**Invariants**

- **BFL.11 INVARIANT** — A bank's reserve balance is its account at the central bank, moved only by settlement.

**Measures**

- **BFL.12 MEASURE** — Deposit stickiness by class; the maturity gap; funding costs by bank; runs start with
  uninsured and wholesale funding; failure at one bank raises withdrawals at banks that look like it.

**Forbids**

- **BFL.13 FORBID** — No single deposit type; no unlimited, unpriced or uncollateralised central-bank credit
  (J4); no bank whose funding cannot leave.

**Done when**

- Banks can be solvent and fail for liquidity; a run can be self-reinforcing and can be stopped by insurance
  or lender-of-last-resort lending.

---

## G3. BCP — Bank capital

**Purpose.** Capital is what absorbs a bank's losses, in layers; the bank chooses how much to hold above the
rule, can raise more if investors will pay for it, and restricts itself as it approaches the line.

**Depends on:** BNK, BFL, ACC, H4 (equity), H3 (bank debt).

**State**

- **BCP.1 STATE** — Capital is **layered**: ordinary equity, then contingent capital that converts or writes
  down when a named ratio falls below a stated level, then subordinated debt, then senior creditors and
  uninsured depositors. The layers absorb losses in that order, in resolution (J5).
- **BCP.2 STATE** — **Requirements** (POLICY of the supervisor): a risk-weighted ratio, a leverage ratio, a
  large-exposure limit and a liquidity requirement; risk weights differ by asset.

**Decisions**

- **BCP.3 DECISION** — A bank chooses a **buffer** above its requirements, from its own caution and the cost
  of equity; near the line it restricts dividends, sheds risk-weighted assets and slows lending.
- **BCP.4 DECISION** — A bank **raises capital** by issuing shares or contingent capital when it judges the
  price worth paying; the issue can fail if investors will not buy (H4).

**Processes**

- **BCP.5 PROCESS** — Capital falls by booked losses and distributions and rises by retained income and
  issuance; it is the equity account (ACC.4), never a pot that is spent.
- **BCP.6 PROCESS** — A breach of the requirement triggers the supervisor's consequences (J5): a restriction
  on distributions, a demanded plan, and at worst resolution.

**Measures**

- **BCP.7 MEASURE** — Banks near their line lend and pay out less; which requirement binds differs by bank;
  capital ratios are procyclical.

**Forbids**

- **BCP.8 FORBID** — No capital pot; no loss that skips a layer; no bank exempt from its requirement.

**Done when**

- A loss that reaches a bank's capital changes its lending through its own buffer decision; a bank that
  cannot raise capital shrinks or is resolved.

---

## G4. SEC — Securitisation

**Purpose.** Pools of named loans moved into a vehicle and funded by tranches, so credit risk moves to named
investors and banks can lend again — and so correlated losses can reach senior holders.

**Depends on:** BNK, REP, H3, H6, RAT.

**State**

- **SEC.1 STATE** — A **vehicle** is a party holding named loans, funded by issuing **tranches** with stated
  attachment points and seniority; a **servicer** collects and passes cash through a stated waterfall.
- **SEC.7 STATE** — **Pool kinds** are declared data (Law 10): residential **mortgage-backed** securities;
  **consumer** asset-backed securities (vehicle loans, credit cards); **small-business** loan securities; and
  **collateralised loan obligations** of corporate term loans. A pool's loans may be lines to cohorts (REP.3),
  each line standing for its weight of identical loans to named members.
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
- **SEC.3 PROCESS** — Losses on the underlying loans — each a borrower's own default, drawn member by member in a
  cohort's line (REP.7) — are allocated bottom-up; when defaults are more correlated than the tranches assumed,
  senior holders lose.
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

**Depends on:** MON, SET, REG, MKT, BFL, J4 (central bank).

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

- **MMK.6 PROCESS** — The money market meets **after** the day's payments (stage 6 after the real work and
  decisions), and its rates are formed in call auctions or dealer markets (MKT.3, MKT.5).
- **MMK.7 PROCESS** — A name the market doubts pays more **or finds no lender**; that refusal is a real outcome.
- **MMK.8 PROCESS** — The central bank's **corridor** frames the market: a deposit facility below and a lending
  facility above (J4). The market rate sits where banks' own schedules meet, and banks prefer the market to the
  facilities while it is open to them.

**Measures**

- **MMK.9 MEASURE** — The spread between the strongest and weakest names; repo haircuts through the cycle; the
  term premium; the transmission of policy-rate changes to market rates.

**Forbids**

- **MMK.10 FORBID** — No rule assigning surplus banks to lend and deficit banks to borrow; no market rate that
  equals the policy rate by construction; no collateral counted as available by both sides.

**Done when**

- Banks' daily reserve positions are funded or not in the market; a doubted bank pays more or is refused; a
  policy-rate change reaches the market through the corridor and banks' own schedules.

---

## H2. SOV — Sovereign debt

**Purpose.** Government bills and bonds: issued by a treasury that must fund itself before it spends, sold in
auctions whose failure is possible and costly, and traded to form the curve other credit is priced against.

**Depends on:** REG, MKT, J1 (treasury), J4 (central bank), H7 (dealers).

**State**

- **SOV.1 STATE** — Bills (discount), fixed-coupon bonds and **inflation-linked bonds** (principal indexed to
  the published consumer price index with its publication lag), each a line that can be **reopened**; the
  line and each tranche added to it are distinct records.
- **SOV.2 STATE** — Sovereign debt ranks **pari passu**, has no covenants, and is not callable; the treasury
  manages its curve by buybacks and switches.

**Decisions**

- **SOV.3 DECISION** — The treasury decides **size, maturity and timing** of each auction from its funding plan
  (J1) and announces the calendar in advance.
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

**Done when**

- A treasury can fail to sell what it offered, and that failure has consequences it must handle.

---

## H3. CRD — Corporate and bank debt

**Purpose.** Bonds, notes, commercial paper and hybrids issued by firms and banks: brought to market by named
underwriters, priced by investors with their own views, traded by dealers, and defaulted on as events.

**Depends on:** REG, MKT, VAL, H7, H9.

**State**

- **CRD.1 STATE** — Debt instruments of every form in REG.5: senior and subordinated bonds, floating-rate notes,
  **commercial paper**, **convertibles**, **preferred shares** (H4), and **contingent capital** of banks. Each
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
  **tap** adds to an existing line; commercial paper is issued continuously to cash investors and must be
  rolled.
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

**Done when**

- Firms and banks issue, roll and default on debt; a default moves other issuers' spreads through investors'
  own reassessments.

---

## H4. EQY — Equity

**Purpose.** Shares as residual claims with votes, issued and bought back by firms, priced continuously by
investors with different views, and wiped out first when a firm fails.

**Depends on:** REG, MKT, VAL, ACC, H9.

**State**

- **EQY.1 STATE** — Ordinary and preferred shares per issuer; a register of holders; a **free float** (what
  insiders, parents and strategic holders do not hold).
- **EQY.2 STATE** — A firm is **listed** when its shares trade on an exchange book; listing requires published
  reports (H9).

**Decisions**

- **EQY.3 DECISION** — Investors buy below and sell above their own value of the share (VAL.8), from their own
  outlooks of its distributions or earnings and their required return; index funds track their index within
  their own tolerance for tracking error, posting limits, never "at any price".
- **EQY.4 DECISION** — A firm **issues shares** (an initial public offering or a secondary issue, book-built)
  when equity is its cheapest way to fund a programme it has; it **buys back** or **pays dividends** from cash
  it does not need.
- **EQY.5 DECISION** — A shareholder **votes** its shares at meetings and on takeover offers (H5).

**Processes**

- **EQY.6 PROCESS** — Listed shares trade on a continuous book (MKT.4) with dealers and market makers.
- **EQY.7 PROCESS** — Dividends are paid to holders of record; issuance dilutes; buybacks concentrate; a split
  changes the count and nothing else.
- **EQY.8 PROCESS** — In insolvency, shareholders are paid only after every creditor; their holding goes to zero,
  never below.
- **EQY.9 PROCESS** — **Short selling** needs a borrowed share (H7) and pays for the borrow; a short squeeze is
  a consequence of limited lendable supply.

**Measures**

- **EQY.10 MEASURE** — Returns are fat-tailed, have little autocorrelation, and their volatility clusters;
  volatility rises when prices fall; prices move on earnings surprises through revised views; market
  capitalisation is a read.

**Forbids**

- **EQY.11 FORBID** — No price from a multiple, a book value, a discounted cash flow or a target; no income to a
  shareholder from earnings not distributed; no short without a borrow.

**Done when**

- Share prices form from investors' own values; firms raise equity when it is cheap to them and can fail to
  raise it; returns show the documented statistical facts without anything imposing them.

---

## H5. MNA — Corporate control and private equity

**Purpose.** Firms bought and sold whole: mergers, takeovers, buyouts and private-equity ownership, each a
priced, funded, refusable transaction.

**Depends on:** EQY, CRD, BNK, FND.

**Decisions**

- **MNA.1 DECISION** — An **acquirer** bids when its own value of the target (its outlook of the target's
  earnings, including changes it believes it can make, discounted at its own required return) exceeds the
  price it must pay, and when it can fund the bid.
- **MNA.2 DECISION** — **Target shareholders** each accept or refuse the offer from their own value; the bid
  succeeds only with the acceptances it requires; management may resist; a rival may bid.
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

**Done when**

- Takeovers happen when bidders value targets above their price and can fund it, and fail when shareholders
  or lenders refuse.

---
## H6. FND — Funds

**Purpose.** Pooled vehicles whose investors own the result: money funds, bond and equity funds,
exchange-traded funds, hedge funds and private-equity funds, each a channel that turns flows into purchases
and redemptions into forced sales.

**Depends on:** REG, ACC, MKT, H1–H5.

**State**

- **FND.1 STATE** — A fund is a named party whose liabilities are its **units**, held by named investors; its
  assets are marked at formed prices (fair value); it has a **mandate** (what it may hold, how much leverage,
  how it can be redeemed) declared as data; its **manager** is a separate party paid a fee.
- **FND.2 STATE** — Fund kinds (Law 10): **money funds** (short, high-quality paper; a substitute for deposits;
  units at a floating value); **open-ended funds** (daily or periodic redemption at net asset value);
  **exchange-traded funds** (units trade on a book; authorised dealers create and redeem against the basket);
  **hedge funds** (wide mandate, leverage from named lenders, redemption with notice and gates);
  **private-equity funds** (committed capital called on demand, a fixed life, distributions on exit).

**Decisions**

- **FND.3 DECISION** — Investors subscribe and redeem from their own reasons: yield against alternatives, past
  performance they observed, their own needs for cash.
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

**Invariants**

- **FND.9 INVARIANT** — A fund's units times net asset value equals its assets minus liabilities; its equity
  beyond that is zero.

**Measures**

- **FND.10 MEASURE** — Flows chase recent performance; money-fund flows rise when their yield beats deposits;
  exchange-traded fund discounts widen in stress; hedge-fund deleveraging moves prices.

**Forbids**

- **FND.11 FORBID** — No guaranteed constant value; no leverage without a lender; no redemption rationed by the
  fund's cash with the rest dropped; no fund that cannot fail.

**Done when**

- Fund flows become purchases and sales in named markets; a wave of redemptions forces sales and moves
  prices; a leveraged fund can be closed out by its lenders.

---

## H7. DLR — Dealers, securities lending and prime brokerage

**Purpose.** The intermediaries that make markets possible when natural buyers and sellers do not arrive at
the same time — and that step back when their own limits bind, which is when markets fail.

**Depends on:** MKT, REG, H1, BCP.

**State**

- **DLR.1 STATE** — A **dealer** is a named desk, usually inside a bank, with inventory, a funding cost on that
  inventory paid every day, a capital charge, position limits per instrument and in total, and a view.
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

**Done when**

- Markets have depth only where dealers have capacity; a dealer at its limit steps back and a market can fail;
  a margin spiral can be seen from the first call to the last sale.

---

## H8. IDX — Indices and benchmarks

**Purpose.** The reference numbers everything else prices off must themselves be reads of formed prices.

**Depends on:** MKT, REG, M2 (published statistics).

**State**

- **IDX.1 STATE** — An **index** is a published rule over a set of constituents and their weights (market
  capitalisation, amount outstanding, equal, production, or household expenditure), with a base; its level is
  recomputed from constituents' prints and never stored as a number of its own.
- **IDX.2 STATE** — **Reference rates**: the floating-rate benchmark is a read of actual transactions in the
  money market (H1); a policy rate is never a benchmark.
- **IDX.3 STATE** — **Price indices** (published by the statistics agency, M2): a **producer** index over
  factory-gate transactions weighted by production, and a **consumer** index over what households actually
  pay at retail, weighted by their spending, including rents and consumption tax and excluding intermediate
  goods.

**Processes**

- **IDX.4 PROCESS** — A change of constituents does not jump the level: the index is chained. A tracked index
  change is a real, simultaneous trade by every fund that tracks it.

**Invariants**

- **IDX.5 INVARIANT** — An index's return equals the weighted return of its constituents.

**Forbids**

- **IDX.6 FORBID** — No index that is an input to its own constituents; no index without constituents; no posted
  benchmark; no single index wearing two names.

**Done when**

- Floating rates fix on transacted rates; producer and consumer prices are separate indices and can diverge.

---

## H9. RAT — Ratings, reporting and estimates

**Purpose.** Published opinions and published accounts, and the rules and decisions that refer to them — so
that information and its errors move markets.

**Depends on:** ACC, VAL, H3, H4, H6.

**State**

- **RAT.1 STATE** — A **rating** is a named agency's published, ordinal opinion of an issuer or instrument on
  the market's scale, formed from the issuer's published state — never from its price — coarse, sticky, and
  sometimes wrong. Several agencies can disagree.
- **RAT.2 STATE** — A **report** is a listed company's published statement for a fiscal period, a read of its
  books (ACC.9), published after a lag on a dated calendar, and restated if corrected.
- **RAT.3 STATE** — **Guidance** is management's published expectation; an **estimate** is a named analyst
  bank's published expectation, formed like any outlook from what that bank observed.

**Processes**

- **RAT.4 PROCESS** — **Rules refer to ratings**: fund and insurer mandates, capital risk weights, collateral
  haircuts and contract triggers. A downgrade across a boundary forces every bound holder to act at once.
- **RAT.5 PROCESS** — A report **settles** every expectation standing against it; each holder's surprise
  revises its own outlook, and prices move because schedules moved.

**Measures**

- **RAT.6 MEASURE** — The downgrade loop (selling, capital pressure, dearer funding, weaker state, further
  downgrade) is traceable step by step; prices move more on large surprises; estimates disagree more after
  volatile results.

**Forbids**

- **RAT.7 FORBID** — No rating derived from a price; no rating nothing refers to; no estimate from the share
  price or the model's forecast; no price-reaction rule; no reported number the books do not produce.

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

**Processes**

- **DRV.4 PROCESS** — **Variation margin** moves the change in mark in cash every business day; **initial
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

**Done when**

- A derivative's mark moves cash every day through margin; a member's default runs through a waterfall and
  can reach surviving members' capital.

---

## I2. DRX — Derivative classes

**Purpose.** The derivative markets this world has, each with its own reasons for each side.

**Depends on:** DRV, and the market of each underlying.

**State and processes, by class** — each class is declared data over DRV (Law 10):

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
  index). A commodity future converges to spot because delivery is possible; its curve is in contango bounded
  by storage and financing cost, or in backwardation when stocks are short.
- **DRX.5 STATE** — **Options**: the right to buy or sell an underlying at a strike, on or before expiry, for a
  **premium formed in the market** — on rates (caps, floors, swaptions), currencies, equities and indices, and
  commodity futures. Users: hedgers buying protection, writers selling it for income, dealers who hedge their
  books by trading the underlying. **Implied volatility is a statistic derived from the premium.** Exercise is
  the holder's decision at expiry (or before, for American style) and settles by delivery or cash.

**Measures**

- **DRX.6 MEASURE** — The swap spread and the credit basis are outcomes that differ by state; implied
  volatility rises in falling markets and has a skew; futures curves read inventories; a hedged firm suffers a
  smaller shock than an unhedged one.

**Forbids**

- **DRX.7 FORBID** — No forward from a parity formula; no fixed swap rate solved from a curve; no option premium
  from a pricing formula; no fixed recovery; no future forced to converge; **no derivative market whose only
  participants are hedgers**: every class has participants with views on both sides.

**Done when**

- Each class has participants with their own reasons on both sides and forms its own prices; option premiums,
  swap rates, forward rates and credit premiums are all formed, and their implied statistics are reads.

---

## I3. INS — Insurance and reinsurance

**Purpose.** Insurers pool the hazards of this world — deaths, illness, accidents, damage, catastrophes — and
can fail when correlated losses exceed what they priced and hold.

**Depends on:** CHN, REG, ACC, MKT, H3, H4, H6.

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

- **INS.6 PROCESS** — **Claims come from the hazard events of A4**: a death, an illness, a fire, a flood. A
  catastrophe hits many policies at once in one place, which is the risk reinsurance exists for.
- **INS.7 PROCESS** — Falling rates raise the present value of long liabilities; an insurer whose assets fall
  below its liabilities is insolvent and is resolved (J5); policyholders then have claims on its estate, up to
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

**Done when**

- Hazard events turn into claims at the right insurers; a catastrophe can break an under-reinsured insurer.

---

## I4. PEN — Pensions

**Purpose.** How people provide for old age: pay-as-you-go state pensions, funded employer schemes that
promise a benefit, and personal accounts that bear their own investment risk.

**Depends on:** POP, INS, FND, J3.

**State**

- **PEN.1 STATE** — A **state pension** is a statutory benefit paid by the social-insurance system (J3) from
  current contributions and taxes, by rules that are POLICY.
- **PEN.2 STATE** — A **defined-benefit scheme** is a party sponsored by an employer: it owes each member a
  benefit schedule; its liability is that schedule discounted at market rates; the sponsor must fill any
  deficit on a schedule; members' claims survive the sponsor as claims on the scheme and any guarantee fund.
- **PEN.3 STATE** — A **defined-contribution account** is a member's holding of fund units, fed by
  contributions; the member bears the result and draws down or buys an annuity on retirement.

**Processes**

- **PEN.4 PROCESS** — Contributions are deducted from wages at payroll; benefits are paid on schedule to living
  members; deaths end benefits (A4).
- **PEN.5 PROCESS** — A falling rate raises a defined-benefit liability and can open a deficit the sponsor must
  pay; a scheme that hedged with leveraged swaps faces margin calls in cash when rates rise.

**Measures**

- **PEN.6 MEASURE** — Funding ratios move with rates; sponsor contributions compete with investment; state
  pension spending rises as the population ages.

**Forbids**

- **PEN.7 FORBID** — No pension liability without members; no promise that is a cash balance; no scheme that
  cannot be underfunded.

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

**Depends on:** MON, SOV, TAX, SOC, CB.

**State**

- **TRS.1 STATE** — The treasury is a party with an account at its central bank, a cash buffer, the debt it has
  issued (read from the register) and the assets it owns.

**Decisions**

- **TRS.2 DECISION** — The treasury runs a **forward funding plan**: it forecasts its own outlays, receipts and
  redemptions (its own outlook, VAL), and schedules auctions of chosen sizes and maturities ahead of the money
  leaving, keeping a buffer it chooses (under a mandate, J6).
- **TRS.3 DECISION** — When an auction falls short, the treasury pays from its buffer, cuts or defers outlays,
  returns at another size or maturity, or — where its country's financing regime allows — uses central-bank
  financing (CB.9).

**Processes**

- **TRS.4 PROCESS** — Outlays go to named recipients: purchases of goods and services, public wages, transfers
  and benefits (J3), interest and redemptions read from the register. Receipts come from TAX.
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
  in auction prices and later in interest cost; the fiscal balance and private net saving mirror each other as
  an accounting consequence.

**Forbids**

- **TRS.8 FORBID** — No automatic central-bank overdraft; no forced buyer of its debt; no deficit target that
  sets anything; no outlay or receipt applied to an aggregate instead of named parties.

**Done when**

- A treasury issues ahead of spending, can have an auction fail, and handles the failure; own-currency default
  is possible exactly where the financing regime makes it so.

---

## J2. TAX — The tax system

**Purpose.** Taxes levied on real bases, collected from named payers at the point the base arises, remitted
on a calendar, so that the state's receipts follow the economy and taxes change what parties do.

**Depends on:** ACC, MON, TIME.

**State**

- **TAX.1 STATE** — Tax bases and rates are POLICY (owned by the parliament, J6): **income tax** on persons with
  a schedule of bands and allowances; **payroll contributions** from employers and employees; **consumption
  tax** added at retail sale; **corporate tax** on reported profit with loss carry-forward; **capital-gains
  tax** on realised gains; **property tax** on dwellings and land; **tariffs** on imports at the border;
  **inheritance tax** on estates.

**Processes**

- **TAX.2 PROCESS** — Each tax arises at its base's event: **withheld** by the employer at payroll, **charged**
  by the seller at the till and **remitted** periodically, **assessed** on the annual return from the payer's
  own statement, **collected** at the border by customs. Between arising and remitting it is a liability of the
  collector to the treasury.
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

**Done when**

- Every tax is collected from a named payer when its base arises; receipts fall in a downturn through the
  bases, not through a rule.

---

## J3. SOC — Social insurance and public services

**Purpose.** The state's transfers and services: unemployment and sickness benefits, state pensions, child
benefits, public education, health care and administration — paid to or provided for named people, and
employing named people.

**Depends on:** POP, LAB, TRS, TAX.

**State**

- **SOC.1 STATE** — **Benefit rules** are POLICY: eligibility, replacement rates, durations, means tests.
- **SOC.2 STATE** — **Public services** are produced by public agencies employing public staff and buying
  inputs; they are provided free or at a charge to eligible people.

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

**Done when**

- Benefits follow individual eligibility events; public employment and purchases are real contracts.

---

## J4. CB — The central bank

**Purpose.** The issuer of reserves and banknotes in its currency, which sets an administered rate and makes
it effective through the corridor and its operations, lends to banks on classical terms, may hold foreign
reserves, and funds its treasury only as its country's financing regime allows.

**Depends on:** MON, MMK, SOV, J6.

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

**Processes**

- **CB.7 PROCESS** — **The corridor**: a deposit facility pays its rate on reserves placed with it; a lending
  facility lends overnight against eligible collateral at its rate; both are administered prices with real
  quantity responses (MKT.8).
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
  weakness; no participation in primary sovereign auctions; no uncollateralised, unpriced, unlimited lending; no
  financing of the treasury beyond its country's declared regime.

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

**Processes**

- **SUP.4 PROCESS** — The supervisor tests institutions on their reporting dates; a breach triggers its declared
  consequences in the next business day.
- **SUP.5 PROCESS** — **Resolution**: triggered by a bank's failure for liquidity or its insolvency (which is
  stated); the authority **values** the book at what it is worth, writes down equity, converts or writes down
  contingent capital and subordinated debt, then senior debt as needed; seeks an **acquirer** who bids and may
  decline; transfers insured deposits; and the rest goes to an estate. No creditor ends worse off than in
  liquidation.
- **SUP.6 PROCESS** — The deposit insurer pays insured depositors what the estate cannot, and becomes a creditor
  of the estate; a fund that runs short draws its treasury backstop — a fiscal cost with a payer.

**Invariants**

- **SUP.7 INVARIANT** — In every resolution, what the acquirer took, what the insurer paid, what the estate
  realised and what holders lost sum to the hole the valuation found.

**Forbids**

- **SUP.8 FORBID** — No failed institution whose positions vanish; no bail-out without a named payer; no
  deposit insurance without a fund and a limit.

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
  change only by a mandate.

**Decisions**

- **POL.4 DECISION** — **Each adult person votes** for the platform that it expects to leave its household
  best off, by applying each platform to its own household's position and outlooks (VAL.8), and abstains if no
  platform differs for it enough.
- **POL.5 DECISION** — **Parties adapt**: between elections each party revises its platform toward positions it
  judges would have won it more votes — from published results and published polls, with their lags — as far
  as its ideology preference allows. A party that loses repeatedly can split, merge or disappear, and new parties
  can form when a large group of voters is far from every platform.
- **POL.6 DECISION** — **Coalitions** form by negotiation among parties by a declared procedure; a parliament
  with no majority keeps the standing mandate.

**Processes**

- **POL.7 PROCESS** — An election is held on its date; seats are allotted; a government forms; the new mandate
  takes effect from a declared date and is announced before then, so parties can anticipate it (VAL.6).
- **POL.8 PROCESS** — The mandate reaches the economy only through the systems that read policy: taxes,
  benefits, outlays, regulation, the central bank's target.

**Measures**

- **POL.9 MEASURE** — Economic downturns change votes; a mean-preserving spread of incomes changes the seat
  count while mean income does not move; platforms converge or polarise as outcomes.

**Forbids**

- **POL.10 FORBID** — No exogenous election result; no policy path; no turnout, swing, loyalty or bloc
  parameter; no vote from an aggregate statistic; the parliament never sets a price, a quantity, an outcome or
  the central bank's rate.

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

**Depends on:** MON, MKT, H7, CB.

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
- **FX.5 PROCESS** — One rate is in force per pair per day for valuation, the day's closing print, and every
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

**Done when**

- Exchange rates move with the flows that cross them; a peg can break when reserves run out.

---

## K2. XB — Cross-border trade, finance and migration

**Purpose.** What crosses borders — goods, services, capital, income and people — each as transactions
between named parties, so the balance of payments is a read and its imbalances are financed by someone who
chooses to.

**Depends on:** FX, GDS, SRV, FRT, H3, H4, H6, POP, TAX.

**State**

- **XB.1 STATE** — A cross-border transaction is one between parties sited in different countries; it is
  **invoiced in the seller's currency** unless the contract states another, and the party not invoicing in its
  own money holds the currency exposure.
- **XB.2 STATE** — Border policies (POLICY of each country): tariffs by good, capital-flow rules, and
  admission rules for migrants.

**Processes**

- **XB.3 PROCESS** — **Trade**: a buyer imports when the delivered price (price abroad, converted, plus freight,
  tariff and time) beats buying at home; the goods cross a border on a route, pay duty at customs, and the buyer
  buys the seller's currency to pay.
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
  account and the financial account, read from transactions, sum to zero; across all countries every category
  sums to zero.

**Measures**

- **XB.9 MEASURE** — Trade falls with distance (the gravity pattern); exchange-rate moves switch expenditure with
  a lag; foreign-currency debt turns depreciations into defaults.

**Forbids**

- **XB.10 FORBID** — No exogenous trade or capital-flow series; no country that is a closed box; no netting of
  cross-border flows into a regional aggregate.

**Done when**

- Imports and exports are transactions between named firms; the balance of payments is a read that balances;
  a sudden stop can happen.

---

# PART L — TRANSMISSION

The causal chains that run across systems. Each system above can meet its own requirements and the world can
still fail to transmit, because transmission lives in the joints. Every chain here is stated as: the
mechanism, the systems it runs through, what silently breaks it, and the test that would kill the claim that it
works (N4).

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

**Mechanism.** A party ends by its own trigger; an estate opens; its assets are **sold** into real markets to real
bidders (plant to buyers who can use it, stock where it always sold); its claims rank — secured, then
senior, then trade creditors and other unsecured including close-out claims, then subordinated, then equity; the
proceeds are paid in that order in every currency the party held; employees are released through the labour
market; suppliers lose receivables; references resolve to the estate.

**Who ends and how:**

| Party                     | Ends when                                                  | Then                                                        |
| ------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------- |
| Person                    | a mortality event                                          | household continues or becomes an estate; heirs inherit     |
| Firm                      | cannot pay, or liabilities exceed assets, under its law    | restructuring or estate                                     |
| Bank                      | fails for liquidity, or is insolvent                       | resolution (J5)                                             |
| Fund                      | its equity is gone or its lenders close it                 | liquidation; lenders take shortfalls, investors lose        |
| Insurer, pension scheme   | assets below the value of liabilities                      | sponsor contribution, benefit cuts, resolution or guarantee |
| Clearing house            | its waterfall is exhausted                                 | resolution; members' further losses                         |
| Sovereign                 | cannot or will not pay (TRS.5)                             | default, exchange offer, market exclusion                   |
| Central bank              | cannot fail in its own money; can lose equity              | losses carried, may be made good by the treasury            |

**Silently broken by:** any party that cannot fail; an estate that values rather than sells; a claim ranked by
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

**Runs through:** BFL, MMK, FND, DRV, TCR, SEC, J5.

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

**Runs through:** GEO, CHN, GDS, FRT, TEC, FRM, SRV, IDX, CB.

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

- **OBS.1 STATE** — **Public** information: prints with their age; published statistics (M2) with their lags
  and revisions; reports, guidance and estimates; ratings; policy decisions and mandates; auction results;
  election results; events (below).
- **OBS.2 STATE** — **Private** information: a party's own positions, limits, intentions and outlooks. The
  observer surface declares which view it gives — an **inspector's** full view of the world, or a
  **participant's** view of its own party plus public information. **Both views exist**, each labelled on every
  screen, and they are different products: the inspector's view is for building and research, the
  participant's for playing.
- **OBS.3 STATE** — An **event** is a change of state somebody would notice — a default, a failed auction, a
  downgrade, a run, a catastrophe, an election, a policy change, a large print, an estate's distribution —
  generated **from** the state, with a date and named subjects, and it can develop over days.

**Processes**

- **OBS.4 PROCESS** — A human **player** acts as a named party in the world, with its own means, through the same
  markets and contracts as everybody else, and appears in every check.

**Forbids**

- **OBS.5 FORBID** — No news that causes anything: prices move because parties act, and the story reports it.
- **OBS.6 FORBID** — No display-only number, no invented story, no scripted narrative, no privileged actor, and
  no surface that changes the world by looking at it.
- **OBS.7 FORBID** — A missing number is shown as missing, a stale price as stale, and every instrument by the
  name its market uses.

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
  vacancies, wages, the money stock, credit, house prices, the balance of payments and the fiscal balance.

**Processes**

- **STA.2 PROCESS** — Each statistic is computed from a **sample** or from reported records of the period, published
  after a declared lag, and **revised** as more records arrive. Inflation is always stated as the change in a
  named index over a named period.

**Invariants**

- **STA.3 INVARIANT** — Output measured by expenditure, by income and by production agree up to the
  statistical discrepancy the sampling produces, and that discrepancy is itself published.

**Forbids**

- **STA.4 FORBID** — No statistic is an input to the world except as a published number a party chose to read;
  no statistic available before its publication date.

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
| **Units**         | goods, dwellings, plant, deposits and people reconcile: opening plus in equals out plus closing          |
| **Contracts**     | receivables equal payables; mortgages owed equal mortgages held; derivative marks sum to zero             |
| **Prices**        | every mark came from a print; no print without a match                                                    |
| **Names**         | every referenced party exists or has a successor                                                          |
| **Cross-border**  | exports equal imports party to party; each country's accounts balance                                     |

**Independence is measured**: a single injected discrepancy must light exactly one family.

## N2. Liveness — allowed at every stage of the build

Liveness reads are cheap, never tuned to, and may be taken at any time while building. They answer "is the
world alive?", not "is it right?":

- Books form prices, and the share of meetings that fail is reported.
- Money circulates: payments per day, velocity, no stock of money stuck on dead parties.
- Parties are born and die in every sector that has them.
- Production, employment, lending and investment are non-zero and respond when a primitive is moved.
- No quantity grows without bound for a reason nobody can name; no state repeats identically for ever with
  nothing changing (a dead fixed point).
- Every system's **Done when** evidence exists.

A liveness failure is a missing mechanism; it is recorded and built, never patched.

## N3. Realism — the stylised facts

The world is judged **realistic** when, in long runs, it reproduces the documented regularities of real
economies **without any of them being imposed**. Each is a measurement; a miss is a finding about a mechanism,
never a reason to tune a number.

Each fact has a **statistic** and a **benchmark range cited from published empirical work**. Because the world's
countries are fictional, a benchmark is the range real economies show, not one country's number. Before a fact
is first measured, its statistic is fixed exactly in the measurement record (series, filter, window, sample),
together with its sources; a benchmark marked *to be cited* gets its range and source at that point.

| #   | Fact                                                         | Statistic                                                                  | Benchmark and source                                                                                          |
| --- | ------------------------------------------------------------ | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| 1   | Output grows, with irregular recessions                       | long-run growth; recession and expansion durations                          | expansions last several times longer than recessions (NBER business-cycle chronology)                         |
| 2   | Investment volatile, consumption smooth                       | standard deviation of cyclical components relative to output                | investment ≈ 2.5–3.5× output, consumption ≈ 0.5–0.8× (Stock and Watson, 1999)                                 |
| 3   | Co-movement with output                                       | correlation of cyclical components with output                              | consumption, investment, employment positive; unemployment strongly negative (Stock and Watson, 1999)         |
| 4   | Okun's relation                                               | regression of the change in unemployment on output growth                   | coefficient ≈ −0.3 to −0.5 (Ball, Leigh and Loungani, 2017)                                                   |
| 5   | Beveridge curve                                               | correlation of vacancy and unemployment rates                               | strongly negative, around −0.9 (Shimer, 2005)                                                                 |
| 6   | Flat, shifting Phillips relation                              | slope of wage or price inflation on slack, over rolling windows             | small and unstable slope (Hooper, Mishkin and Sufi, 2020)                                                     |
| 7   | Output growth is fat-tailed                                   | shape of the growth-rate distribution                                       | close to Laplace, far from normal (Fagiolo, Napoletano and Roventini, 2008)                                   |
| 8   | Firm sizes are skewed                                         | tail exponent of the firm-size distribution                                 | ≈ 1, Zipf (Axtell, 2001)                                                                                       |
| 9   | Firm growth is fat-tailed and falls in variance with size     | growth-rate distribution; slope of log volatility on log size               | Laplace-shaped; slope ≈ −0.15 to −0.2 (Stanley et al., 1996; Bottazzi and Secchi, 2003)                       |
| 10  | Productivity dispersion                                       | ratio of the 90th to the 10th percentile of productivity within an industry | ≈ 2:1 for total factor productivity in narrow industries (Syverson, 2011)                                     |
| 11  | Frequent entry and exit; young firms fail more                | exit rates by firm age; five-year survival                                  | roughly half of new firms gone within five years (Haltiwanger, Jarmin and Miranda, 2013)                      |
| 12  | Sticky, lumpy prices                                          | median duration of individual regular prices; mean size of changes          | median ≈ 8–11 months; changes ≈ 10% on average (Nakamura and Steinsson, 2008; Klenow and Kryvtsov, 2008)     |
| 13  | Income and wealth distributions                               | Pareto exponent of top incomes and of top wealth                            | incomes ≈ 1.5–3; wealth lower, ≈ 1.3–1.8, so more unequal (Atkinson, Piketty and Saez, 2011; Vermeulen, 2018) |
| 14  | Spending responses differ with liquidity                      | consumption response to a transfer by liquid wealth                         | markedly higher for households with little liquid wealth (Jappelli and Pistaferri, 2014; Kaplan and Violante, 2014) |
| 15  | Long unemployment spells                                      | distribution of spell lengths; long-term share over the cycle               | right-skewed; the long-term share rises in and after recessions (*to be cited*)                              |
| 16  | Credit is procyclical and leverage builds in booms            | correlation of credit growth with output; leverage over the cycle           | credit booms precede downturns (Schularick and Taylor, 2012)                                                  |
| 17  | Defaults cluster                                              | excess dispersion of default counts over independent defaults               | defaults more clustered than common factors alone explain (Das, Duffie, Kapadia and Saita, 2007)              |
| 18  | Crises follow credit booms                                    | probability of a banking crisis given past credit growth                    | rises with past credit growth (Schularick and Taylor, 2012; Reinhart and Rogoff, 2009)                        |
| 19  | Partial, lagged pass-through of policy rates                  | response of loan rates to a policy-rate change over time                    | incomplete on impact, larger with time (*to be cited*)                                                        |
| 20  | Returns fat-tailed, nearly uncorrelated                       | tail index of daily returns; autocorrelation of returns                     | tail index ≈ 3; autocorrelation insignificant at daily horizons (Cont, 2001)                                  |
| 21  | Volatility clusters, rises as prices fall                     | autocorrelation of absolute returns; correlation of returns with future volatility | slowly decaying autocorrelation; negative return–volatility correlation (Cont, 2001; Black, 1976)       |
| 22  | Housing booms and busts, volumes lead prices                  | lead–lag of transaction volume and price                                    | volumes turn before prices (Stein, 1995)                                                                       |
| 23  | Yield curve slopes up; inversion precedes recessions          | average term spread; its predictive power for downturns                     | positive on average; inversion predicts recessions (Estrella and Mishkin, 1998)                               |
| 24  | Credit spreads are countercyclical                            | correlation of corporate spreads with output; response to defaults          | widen in downturns and predict them (Gilchrist and Zakrajšek, 2012)                                           |
| 25  | Trade falls with distance                                     | elasticity of bilateral trade with respect to distance                      | ≈ −0.9 (Disdier and Head, 2008)                                                                                |
| 26  | Exchange rates disconnected over short horizons               | forecastability of exchange rates by fundamentals                           | a random walk forecasts about as well (Meese and Rogoff, 1983)                                                |
| 27  | Sudden stops                                                  | frequency and size of current-account reversals                             | abrupt reversals occur, especially with foreign-currency debt (Calvo, 1998)                                   |
| 28  | Fertility and migration respond to conditions                 | response of births and moves to income, housing cost and employment        | *to be cited*                                                                                                  |

## N4. Causal-chain tests

Every chain in Part L is tested by **falsification**: run the world with the chain's first link held fixed (the
shock absent, the price held, the rule removed) from the same state and seed; the effects the chain claims must
disappear, and effects that do not disappear were not caused by it. A claim about the model without such a test
is not made.

## N5. Reproducibility and resolution

- The same seed and primitives reproduce the same world exactly.
- **Resolution invariance** (PTY.12): per-person and distributional outcomes do not change materially with the
  cohort budget, the merge tolerance, the number of preference types or the map's grid; the measured change,
  together with the shadow sample's divergence (REP.15), is the error bar.
- **Seed dispersion**: key outcomes are reported across many seeds, so a result is never one draw of chance.

## N6. Interventions and experiments

The only way to change a primitive during a run other than by its owner's decision is a **declared
intervention**: a named change to a primitive or an endowment (a technology improves, a catastrophe is added, a
tax rate changes outside the polity, a bank is given more capital) at a stated date, applied to a copy of the
world from the same state and seed. The measured effect is the difference between the two worlds. Interventions
are experiments about the model, never part of the world a player sees.

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
- **N8.2** — **A turn** is one simulated business day by default. At the **play resolution** (N8.5), with the full population, a turn completes
  in **at most 1 second at the median and 2 seconds at the worst** (month-ends, quarter-ends, paydays and the days
  markets are busiest), measured over a full simulated year.
- **N8.3** — **Sustained**: the budget holds across a simulated year of consecutive turns with the phone's own
  thermal limits in force, not only for a first burst of turns while the device is cool.
- **N8.4** — **Memory**: the world, its retained instructions and its snapshots stay within a declared memory
  budget (initially 2 GB resident) and a declared storage budget for saves (initially 1 GB), and neither grows
  without bound over a run of decades — which is what SET.12–SET.16 exist for.
- **N8.5** — **The play resolution** is the largest cohort budget and finest merge tolerance (A6) that meet
  N8.2–N8.4 on the target device, always with the full population. The realism runs of Stage 7 may use finer
  resolutions on other machines. The resolution test (PTY.12) and the shadow sample (REP.11) then say whether the
  play resolution gives the same per-person results; if it does not, the difference is published beside every
  result the play resolution shows.
- **N8.6** — **Cost follows events, not size**: nothing in this specification requires every party to be visited
  every day. Parties act on their own schedules or when woken (TIME.5), accruals are applied on the dates that
  need them, and the daily audit checks what the day changed, with the full audit on a declared cycle.
- **N8.7** — **The budget never changes a mechanism.** When the budget is missed, the remedies are, in order: how
  the world is represented and traversed; then the play resolution (a smaller cohort budget or a wider merge
  tolerance). The population is never reduced, and no law, mechanism or requirement is weakened to meet it.
- **N8.8** — The budget is **measured on the device** at the end of every stage from Stage 1 on, and a stage does
  not end with the budget missed.

---

# PART O — BUILD STAGES

The layers of this document in the order they can be built, grouped into stages. Each stage ends with a
**living world**: everything built so far runs, the audit is clean for what exists, and the stage's liveness
reads (N2) pass. A stage is not a delivery date and says nothing about how to build.

**Stage 0 — Foundations.** TIME, PTY, NUM, CHN, GEO, REP, MON, SET, REG, ACC, MKT. *Exit:* a world of parties on a map
can pay each other, hold and transfer instruments and physical units, and form a price in each market form, with
every family of the audit that applies running clean.

**Stage 1 — The circular flow.** A single country: POP (births, deaths and ageing only), HH (spending, working,
saving in deposits), TEC (opening ways, no innovation), FRM, CAP (plant only), GDS, SRV, LAB, one tier of banks with
BNK and deposits, the central bank's settlement and a fixed policy rate, a treasury with income and consumption tax
and one benefit, published statistics, VAL (adaptive outlooks and values). *Exit:* households earn wages, spend them
at firms that pay wages, firms are born and die, banks lend and are repaid, the treasury taxes and spends — and the
world keeps doing so for decades without anything imposed — **and a simulated year of it, with the full population at
the play resolution, meets the performance budget (N8) on the target device.** This is the first go/no-go point: if the thin circular flow
cannot meet the budget, the representation is revisited before anything is built on top of it.

**Stage 2 — Credit and failure.** L1 (loss as event), L3 (estates), TCR, the full firm lifecycle, bank provisions and
write-offs, BFL, BCP, J5 (supervision, deposit insurance, resolution), HSG with mortgages. *Exit:* a borrower's own
cash failure produces a default, an estate, a loss on named holders and a housing foreclosure; a bank can fail for
liquidity or solvency and is resolved.

**Stage 3 — Money and capital markets.** MMK and repo, the full central bank (corridor, operations, lender of last
resort, financing regime), TRS with SOV auctions, CRD, EQY, DLR, FND, IDX, RAT, L2 (forced seller), L4 (cost of
capital). *Exit:* the policy rate reaches loan rates, asset prices and investment through markets; a margin spiral and
a fund run can happen.

**Stage 4 — Risk transfer.** DRV, DRX (swaps, credit, currencies, futures, options), INS, PEN, SEC, MNA. *Exit:* every
derivative class forms its price with views on both sides; hazard events become insurance claims; pension liabilities
move with rates.

**Stage 5 — The full state and the open world.** TAX in full, SOC, POL, a second and third country, FX, XB, FRT across
borders, migration. *Exit:* elections change policy; currencies float or break their pegs; trade and capital flows
balance as reads.

**Stage 6 — Growth and the full population.** TEC research and diffusion, POP in full (formation, education,
migration), HH in full. *Exit:* long runs grow through discovered improvements; the population's size and shape are
outcomes.

**Stage 7 — Realism.** The full measurement programme: N3 stylised facts, N4 chain tests, N5 resolution and seed
dispersion, N7 calibration of primitives from data. What misses is recorded against the mechanism suspected; the
build continues by adding mechanisms, never by tuning.

**Rules for the stages**

- A stage uses only systems from its own or earlier stages; a need discovered for a later system is met by
  **bringing that system forward**, and the move is recorded.
- Within a stage, a system is built to its **Done when** before the next is started.
- Every SHAPE introduced to let an earlier stage run names the later system that retires it.

---
# APPENDICES

---

## Appendix A — Glossary

| Term                        | Meaning in this document                                                                                     |
| --------------------------- | ------------------------------------------------------------------------------------------------------------ |
| **Party**                   | anything that can hold, owe, decide or be paid (PTY.1)                                                       |
| **Person / household**      | an individual human / the people who share a budget and dwelling and own jointly (PTY.3)                    |
| **Cohort**                  | a named party standing for an exact count of identical households or small firms (REP.1)                    |
| **Individual**              | a party of weight one: every institution, and any member materialised from its cohort (REP.2)              |
| **Materialisation**         | splitting a member out of its cohort when something happens to it alone (REP.6)                             |
| **Contract line**           | one record for a cohort's identical contracts with one counterparty (REP.3)                                 |
| **Cohort budget / merge tolerance** | how many cohorts the world carries, and how close balances must be to merge (REP.4)                  |
| **Shadow sample**           | members carried at full resolution to measure what merging costs (REP.11)                                  |
| **Primitive**               | a declared number of one of the six kinds of Law 2                                                           |
| **Outcome**                 | anything the world produces rather than is given                                                            |
| **Hazard process**          | a declared source of chance with a rate, acting on named subjects (CHN.2)                                   |
| **Way**                     | a fixed-proportion method of producing a product (TEC.2)                                                    |
| **Market form**             | one of the six price-forming mechanisms of C1                                                               |
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
| **Intervention**            | a declared change to a primitive in a copy of the world, for measurement (N6)                               |
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

17. No representative agent; no decision at an average; no cohort of members that differ; no weight that is not a
    count; no merge that creates or loses a unit (PTY.14, HH.18, REP.16, REP.17, Law 11).
18. No global expectation; no model forecast; no peeking; no sentiment parameter; no common value; no value
    printed as a price (VAL.16–VAL.21).
19. No aggregate matching function; no birth, migration, participation or investment rate (LAB.15, POP.14,
    CAP.11).
20. No instantaneous or costless transport; no goods in transit owned by nobody (FRT.11).
21. No mechanism that branches on the kind of thing it acts on (Law 10).

**Observation**

22. No news that causes anything; no display-only number; no privileged actor; no surface that changes the world
    (OBS.5, OBS.6, Law 17).
23. No statistic available before its publication (STA.4).

---

## Appendix C — The primitive catalogue

What the world must be **given**, by kind. Every item is registered (NUM.3) with its value, unit, owner and
source.

| Kind           | What                                                                                                   |
| -------------- | ------------------------------------------------------------------------------------------------------ |
| **TECHNOLOGY** | ways of making every product; capital kinds, lives and wear; construction and build lead times; vehicle speeds, capacities and running costs; storage and spoilage; life tables and health hazards; conception hazard; schooling-to-skill; learning curves; discovery and imitation hazards and improvement distributions; catastrophe frequencies and exposures; search meeting rates |
| **PREFERENCE** | finite type sets (with shares) of patience, risk aversion, tastes, leisure, dwelling and location preferences, preference for children, memory, heuristic-switching intensity; management risk appetite, hurdles and horizons; decision schedules; party ideology preferences |
| **POLICY**     | tax bases and rates; benefit rules; minimum wage and labour law; capital, liquidity and exposure rules; deposit-insurance limits and premiums; insolvency and inheritance law; zoning; tariffs, capital-flow rules and admission rules; patent life; the central bank's mandate, target and financing regime; the constitution's seats, term and allotment rule; accounting standards; market conventions (settlement cycles, day counts, auction formats) |
| **ENDOWMENT**  | the map, terrain, deposits and opening infrastructure; calendars; the opening population with its households, skills and holdings; the opening firms, banks, funds, insurers and their balance sheets; opening contracts and instruments with their terms and remaining lives |
| **RESOLUTION** | cohort budget, merge tolerance, tail-protection thresholds and shadow-sample size; number of preference types; map grid; the number of heuristics tracked per outlook                                                                          |
| **SHAPE**      | the heuristic menu (VAL.22); terrain-generation parameters (GEO.18); every placeholder introduced during building, each naming what retires it |

An opening world must pass the audit on its first day, must be consistent with the flows that will run on it
(debts with coupons somebody can pay, employment with a wage bill somebody can meet), and must contain **no
outcome** fitted to an answer.

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
11. **The document is ordered by causal layer**, and the order is the build order.
12. **Real limits are declared, invented bounds are forbidden**, and negative prices are possible.
13. **The polity stays, and parties adapt their platforms.**
14. **A real-sized population, carried as cohorts of identical members with individuals where it matters** (A6).
    The world holds hundreds of millions of people and millions of small firms; no phone can hold that many
    separate parties, so cost must follow the number of distinct situations rather than the headcount. The
    representation combines techniques proven where counts are far larger: exact counting of identical members
    (lumpability of agent-based Markov chains, Banisch); finite preference types, which suffice to reproduce real
    wealth inequality and spending (Carroll, Slacalek, Tokuoka and White, 2017); binomial draws of how many members
    an event hits (binomial tau-leaping from stochastic chemical kinetics); on-demand switching to full detail
    (adaptive-resolution molecular dynamics, level-of-detail simulation in games, hybrid agent–compartment
    epidemic models); budgeted merging that conserves every total exactly (particle merging in plasma simulation,
    population control in Monte Carlo transport); tail protection and a full-resolution shadow sample to measure the
    error (adaptive prototype simulation of population-scale agents); and a counter-based seeded generator so the
    result does not depend on run order.

15. **Three fictional countries.** Enough for cross rates, triangular arbitrage, trade and migration, and for a
    large and a small open economy. Their primitives may come from data (tax law, life tables, technology), but no
    country copies a real one, so results are never read as forecasts of a real economy.
16. **The population is never scaled down to fit the device**: the phone runs the full population at the play
    resolution (N8.5), and the resolution test and the shadow sample report what that resolution costs.
17. **Both observer views exist**, clearly labelled: an inspector's full view for building and research, and a
    participant's view for playing (OBS.2).
18. **Stylised facts have cited benchmark ranges** from published empirical work (N3); the exact statistic is
    fixed before each is first measured, and a miss is a finding, never a tuning target.
19. **A performance budget** (N8): one simulated business day in at most 1 s median and 2 s worst on the target
    phone, sustained over a simulated year, within declared memory; measured at the end of every stage from Stage 1.
20. **Snapshots and a retention window** (SET.12–SET.16) replace unbounded replay from the first instruction: the
    world is exactly restorable from its last snapshot, and nothing a decision reads lives only in released
    history.

**Open** — to be decided by the owner before the stage that needs them:

1. The number of regions per country and the size of the map, chosen when the opening world is first generated
   (Stage 0).
2. The benchmark ranges still marked *to be cited* in N3, before Stage 7.

---

*Project Phoenix V2 — the world, and nothing about how to build it.*
