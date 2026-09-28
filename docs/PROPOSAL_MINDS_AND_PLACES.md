# Proposal — Minds and places

*A proposal accepted by the owner on 2026-09-28, with the answers of Part 3; not yet the specification. What is
accepted moves into `PROJECT_PHOENIX.md` (what exists and why), `ARCHITECTURE.md` (how it is built) and
`IMPLEMENTATION.md` (the steps), and this file is then removed.*

Two ideas, one proposal each:

1. **A mind for every decider.** Every decision in the world is taken by a person — a household's adults, a chief
   executive, a board, a treasurer, a governor, a head of government — and today each is a mechanical rule of a few
   inputs with a threshold. The proposal gives every person a **mind**: one shared decision engine that weighs many
   inputs at once, coloured by the person's character, the life it has lived, and what it aims for. Two chief
   executives facing the same firm decide differently, and a new president changes policy because a different person
   is deciding.
2. **Places that grow.** Today the map is 40,000 tiles of 10 km, and households and firms are carried by zone. The
   proposal adds a finer layer where things are built: land parcels of 1 km, and buildings as real capital on them.
   Towns and cities **emerge** where building concentrates. The statistics agency recognises a settlement by the real
   statistical definition of one, and a recognised settlement may incorporate as a municipality, with a mayor who is a
   person.

Both follow from the owner's decisions of 2026-09-28: every person is a party with a lasting identity (Appendix E 46),
and office holders decide with their own minds (E 47).

---

## Part 1 — The mind

### 1.1 What we have today

- **Decision points** (`phx_core::decisions::DecisionPointDecl`): each is a pure function `fn(&I) -> O` with a
  schedule or wakes. Its form is a standing SHAPE listed in `SHAPES.toml`, and its parameters are primitives (plan
  §2.21). The `Decider` is either `Rule` or `Player`.
- **Outlooks and values** (VAL): each party forecasts with fallible heuristics (adaptive, trend, anchor,
  announcement). It switches heuristics by their performance for it (VAL.7), and values a thing by its own simple
  model (VAL.8). The window of lived experience is weighted by age class (VAL.23).
- **Heterogeneity** (NUM.4): patience, risk aversion, memory and switching intensity are finite type sets drawn once
  per party. A taste is drawn on each choice (REP.22, random utility), so choices are smooth rather than kinks.
- **What it lacks.**
  - A rule reads its listed inputs and compares a few of them: a firm posts vacancies when an hour's output is worth
    more than the wage, and a household buys when its value exceeds the price.
  - Nothing weighs **several concerns against each other**: security against income, career against family, a
    principle against a profit.
  - Nothing carries **what a person has lived through** beyond its forecasting windows, and nothing carries **what
    it is trying to become**.
  - Institutions decide by the institution's type, so a change of chief executive or president changes nothing.

### 1.2 What the mind must respect

The laws constrain the design more than any taste does:

- **Law 11 (heterogeneity), Law 12 (nobody knows more than they could), VAL.16–21 (no global expectation, no common
  value).** A mind reads only its person's own state, its own outlooks and what is published after its lag.
- **Law 15 (chance declared and seeded), Law 16 (no outcome imposed).** A mind never draws an outcome. Its only chance
  is the declared taste on a choice (REP.22). No career, election or crisis is scripted.
- **Law 2 (every number a primitive, few of them).** Every weight, speed and threshold is a primitive with a source.
  The mind should *reduce* the number of forms, not add to them.
- **Determinism.** The world runs once (Appendix E 36), and a save must reload to the same world. Every evaluation is
  pure arithmetic over `libm`, identical on every device.
- **The budget (N8).** 1 s median and 2 s worst per business day on the phone, 4.5 GB of memory, 4 GB of saves, at the
  design point of five million persons. The full-load bench stands at a 2.52 s median today, above the budget before
  any of this, so the mind must cost microseconds a decision and tens of bytes a person.

### 1.3 Alternatives considered

| Approach | Why not |
| --- | --- |
| **A language model deciding for each actor** | A small on-device model takes ~0.1–1 s an answer. The world takes ~10⁵ decisions a business day, so this is five orders of magnitude over budget. Its output is not reproducible across devices and versions, which breaks the one run and the saves. What it does cannot be stated as a clause, sourced or audited (Law 2, Part II). It stays outside the world (§1.10). |
| **Neural policies trained offline** | Training on the world's own outcomes is tuning (N7, GEN.11). Training on outside data needs sources we do not hold, and the weights are primitives nobody can state or check. |
| **Full optimisation (dynamic programming per person)** | Real people do not solve it. At five million persons the state space is out of reach, and it makes everyone rational in the same way, erasing the heterogeneity Law 11 needs. |
| **More special-case rules** | Every rule stays a threshold on its own inputs. Each new concern means a new form in every decision, and the SHAPE list grows with every system (Law 10's spirit). |
| **Chosen: a bounded-rational, multi-concern mind** | One declared form for every decision. Each option is scored on a few concerns relative to the person's own aspirations, with loss aversion and the person's own weights. The choice is by random utility over a considered set. The literature behind it is large and measured (§1.9). It costs a few hundred nanoseconds a decision. |

### 1.4 The design

A **mind** is what a person brings to any decision. It has four parts: character, experience, aspirations and goals.
It sits beside the person's word on the core, and only adults carry the parts that change.

**Character** (drawn once, at birth or arrival; NUM.4 types):
- Patience, risk aversion and memory (already primitives), plus the preferences the Global Preferences Survey
  measures: **altruism, trust, positive and negative reciprocity** (Falk et al., 2018). The country means are in hand
  (`people/gps_country.csv`); the within-country spread is the survey's published dispersion (§1.9).
- **Concern weights**: how much the person cares about each concern (below), a type from a declared table.
- Held as **one type index** (two bytes). The type table is a register primitive, so a person's character costs
  nothing more than its index.

**The concerns** (a finite list, declared once; each kind declares which apply to it, Law 10):

| Concern | What an option is scored on | Read from |
| --- | --- | --- |
| Means | the money it brings or costs over the person's horizon, discounted at its patience | outlooks and values (VAL.8) |
| Security | the chance and size of a loss: of job, home, firm or savings | outlook widths (VAL.4, VAL.9) |
| Wealth and legacy | net worth, and what passes to heirs | holdings at marks, kin |
| Standing | rank, title, office, income against the person's peers | offices held, peers' published pay |
| Family | partner, children, time at home, kin nearby | household, kin, hours |
| Leisure and health | hours worked, commute, illness | contracts, sites, health state |
| Place | staying near home and kin | zone, kin's zones |
| Principle | fairness and redistribution, economic freedom, the institution's mission | platforms, mandates |

**Experience** (what the person has lived, carried as a few decaying traces; adults only):
- The returns, inflation and unemployment the person lived through, weighted by how recent and how young it was then:
  the Malmendier–Nagel learning-from-experience weights, the same windows VAL.23 already reads.
- The shocks the person suffered itself: a spell of unemployment, a firm of its own failing, a home foreclosed, a
  crisis lived through.
- Experience moves two things:
  - its **outlooks** (already, through VAL);
  - its **effective risk aversion and concern weights**. A person who lived a crash takes less risk; one who lived
    long unemployment weighs security more; a chief executive who lived a depression borrows less (Malmendier, Tate
    and Yan, 2011).
- Carried as four traces of 16 bits each (8 bytes).

**Aspirations** (a reference level on each concern that has a level; adults only):
- A person judges an outcome **against what it aspired to**: a gain above the aspiration counts, and a shortfall
  counts more (loss aversion, Kahneman and Tversky; the coefficient is a primitive with its source).
- An aspiration adapts toward what the person achieves and what its peers achieve, at a declared speed (aspiration
  adaptation; Easterlin; Stutzer, 2004). A pay rise pleases, then becomes the new normal.
- Carried as three levels of 16 bits (income, wealth, standing) (6 bytes).

**Goals** (discrete aims, each with a target and, where it has one, a date):
- The menu: own a home, have children, found a firm, reach an office (a board seat, the chief executive's chair,
  parliament), retire by an age, move to a place.
- A goal is **formed at a life event**, from the person's character and circumstances: coming of age, partnering, a
  first child, a promotion.
- It raises the weight of the concerns it serves until it is **reached, or abandoned** when its aspiration has
  adapted below it after repeated failure.
- Carried as two slots (goal, target date) of 32 bits each (8 bytes).

A person's mind costs **2 bytes of character for everyone and 22 bytes for each adult**: about 80 MB at five million
persons, which the bench must measure before anything is built (§1.8).

### 1.5 How a decision is taken

Every decision point keeps its schedule, wakes and inputs. What changes is its body. A rule becomes an **option
generator**, and the mind chooses among the options:

1. **Generate options.** The mechanism lists the few alternatives the decision has:
   - posted prices on the nearby price points, or wage points for a wage offer;
   - staying or moving, taking or refusing an offer, borrowing or not;
   - for a platform, the policy values within the mandate's reach.

   Most decisions already work on discrete points (REP.34), so the options are there today.
2. **Score consequences.** For each option, the mechanism states what the person expects on each concern that applies,
   from the person's own outlooks and values (VAL.8). This is the arithmetic the rules already do, now reported per
   concern instead of compared once.
3. **Weigh.** The mind scores each option with its concern weights. Each consequence is read against the matching
   aspiration, with loss aversion and the risk aversion its experience has shaped. The office's **mandate** is added
   where there is one (§1.6).
4. **Consider and choose.**
   - The person considers as many options as its **attention** allows: a declared number by type, fewer under a
     costly review (REP.21).
   - It chooses by random utility: its taste on each option is drawn from its type's declared distribution (REP.22,
     already the rule).
   - Where options are ordered and time presses (a job search, a house hunt), it may instead **satisfice**: take the
     first option that meets its aspiration (Simon; Selten's aspiration adaptation). Each decision's form, one or the
     other, is declared.
5. **Learn.** The outcome, seen later, updates the person's experience and aspirations. Where a decision has a menu of
   **strategies** (a firm's pricing, growth or cash style; a household's saving style), the person switches among them
   by how each has worked **for it**, exactly as VAL.7 does for forecasting heuristics. This is the "smarter outcome":
   people get better at what they do, each by their own history, never by the world's.
6. **Explain.** The decision records which concern tipped it: one byte, kept only for persons being followed (§1.10).

**Households decide as one** (PTY.3). A household's choice sums its adults' scores, each weighted by the member's share
of the household's income: the collective model of Browning and Chiappori, whose weights are measured. When partners'
scores diverge far and long, that is an input to POP.17's decision to separate.

### 1.6 Offices: why a new leader changes things

Under Appendix E 47, an institution's decision reads its office holder's mind within the office's **mandate**:

- **Chief executive** (FRM, BNK): the mandate is the owners' interest, plus what the pay contract rewards; the mind is
  the person's. Two chief executives facing the same firm post different prices, hire and invest differently, and
  borrow differently. The literature measures this: manager effects explain a significant share of the variation in
  firms' investment, leverage and dividends (Bertrand and Schoar, 2003), and overconfident chief executives invest and
  merge more (Malmendier and Tate, 2005, 2008).
- **Board**: each member scores the candidates for chief executive on its own mind, over their records (PTY.17). The
  vote aggregates them. Boards choose people like themselves, or the opposite, as their minds dictate.
- **Governor and committee** (CB): the mandate is the statutory target; a governor's own patience, risk aversion and
  lived inflation colour how hard it leans, and each member votes.
- **Head of government and ministers** (POL): the mandate is the coalition's platform. The leader's mind chooses among
  the budget proposals and emergency measures within it (POL.11). Leaders measurably matter: growth changes when
  leaders die in office (Jones and Olken, 2005), and policy shifts with the party and person in power.
- **Candidates and parties**: a person's decision to stand, to join a party, or to lead one is a decision of its mind
  (Standing, Principle), so a political career is followed like any other.

### 1.7 What changes in the spec

A new system, **MND — Minds**, beside VAL in Part C. Its clauses, in outline:

- **STATE**: a person's character (types drawn at birth), experience traces, aspirations and goals. An office's mandate
  is declared data of the office (Law 10).
- **DECISION**: every decision of a person, or taken in an office by a person, is a choice among the options its
  mechanism generates, scored on the declared concerns against the person's aspirations, with its own weights and
  tastes. Satisficing or best choice is declared per decision.
- **PROCESS**:
  - experience accrues from lived events and published series;
  - aspirations adapt toward achievement and peers;
  - goals are formed at life events, reached or abandoned;
  - strategies are switched by their record for the person.
- **FORBID**: no opaque or learned-from-outcomes policy; no common mind; no career, success or failure drawn; no mind
  reading another's private state.
- **PRIMITIVES**:
  - the concern list and each kind's concerns (SHAPE, with its reason);
  - the character type table (PREFERENCE, from the GPS and the types NUM.4 already draws);
  - loss aversion; the experience weights; the aspiration adaptation speed;
  - attention by type; the goal menu and formation hazards; the household bargaining weights.
- **MEASURES** (N3 realism tests the world must pass, never targets):
  - manager effects in firms' policies;
  - leader effects on policy after transitions;
  - experience effects on risk-taking and inflation expectations (the "depression babies" pattern);
  - the Easterlin pattern: satisfaction and aspiration rising with income within a year, but not across years.

VAL.7 (heuristic switching), REP.22 (tastes) and NUM.4 (types) stay; the mind is built on them. Plan §2.21's rule that
every decision's form is a SHAPE becomes one SHAPE, the mind, with each decision declaring its options and concerns.

### 1.8 How it is built on today's architecture

- **A kernel crate, `phx-mind`**: pure functions over `libm`. `Options` holds per-option consequence vectors. `Mind`
  holds the character index, traces, aspirations and goals. `choose(mind, options, taste_draws) -> index` and
  `explain(...)`. No world, no stores, and unit tests at the logic level.
- **Storage on the core**:
  - the character index sits in the person's identity word's spare bits: an identity needs about 40 bits, so 16 of
    its 64 are free;
  - the adults' 22 bytes are a column keyed like `Persons`, beside the identity;
  - an office holder's mind is its person's. The institution holds the office as a contract naming the person, and
    reads the mind through it.
- **Decision points**: `DecisionPointDecl::rule` keeps its signature. A migrated rule builds `Options` and calls
  `phx_mind::choose`. `Decider::Player` still overrides for the player's own household.
- **Cost** at five million persons: about 70,000 household and 35,000 firm decisions a business day. Each scores ~8
  options on ~8 concerns (64 multiply-adds, one `exp` an option), about 0.2 µs, so **~20 ms a day in all**. Memory is
  ~80 MB, saves ~40 MB compressed. The bench gains a mind for every person and every decision before any system uses
  one; if it breaks the budget, the representation changes first (N8), never the mechanism.
- **Order**: after S1.24 and S1.25, when the world runs on the core:
  1. the kernel and households' decisions (spending, work, moving);
  2. firms' decisions, with the chief executive's mind once offices exist (S3.05);
  3. experience, aspirations and goals with the life events (POP.7, POP.17);
  4. the polity's minds (S5.03).

### 1.9 Sources

- **In hand:**
  - Global Preferences Survey country means (patience, risk taking, reciprocity, altruism, trust);
  - the NY Fed Survey of Consumer Expectations microdata, for individuals' revisions and experience effects.
- **Needed from the literature** (tables of published papers, which the data rule allows):
  - the GPS within-country dispersion;
  - Malmendier and Nagel (2011, 2016) experience weights;
  - Kahneman and Tversky (1992) loss aversion;
  - Stutzer (2004) aspiration adaptation;
  - Bertrand and Schoar (2003) and Malmendier and Tate (2005) for the measures;
  - Browning and Chiappori (1998), and later estimates, for household weights;
  - Jones and Olken (2005).
- **Not in hand:** a measured distribution of concern weights across people. Candidates are life-goal importance items
  (World Values Survey) or a declared simulation from the GPS dimensions. This is an owner decision (§3).

### 1.10 Following a life, and a narrator outside the world

- **A recorder** keeps, for the persons the player chooses to follow, every event of their life: birth, schooling,
  jobs and pay, partnering, children, homes, firms founded, offices held, elections, illness and death. It also keeps
  the concern that tipped each decision. It only reads (Law 17), and costs nothing for anyone not followed.
- **A narrator** (optional) turns a followed person's record into prose ("She left the bank after the crash, founded a
  builder in the river town, and at 52 ran for mayor"). It can use a language model, because it sits outside the
  world: it reads records after the fact and nothing it writes enters the world. It can run on the device or on a
  server, at the owner's choice.

---

## Part 2 — Places that grow

### 2.1 What we have today

- **Tiles**: 40,000 tiles of 10 km on a closed surface. Each has land or water, elevation, terrain, deposits and
  hazard exposure (GEO.1–7). Its size is a RESOLUTION: "the same map subdivided" (GEO.18).
- **Regions** (25) hold local markets. **Zones** carry households and firms (REP.24).
- **Units stand in zones**: dwellings, plant and vehicles are counted by zone and class, and a unit's tile is drawn
  only when something depends on it (REP.23).
- **Building**: builders build when the expected price exceeds land, construction and financing (HSG.8). Land is
  owned, zoned and traded per region (HSG.3, HSG.20). Infrastructure is owned capital with capacity (GEO.4).
- **What it lacks**: nowhere does building **concentrate** by a mechanism, nor is there anything to show as a town.
  Distance within a region is between zones, and commuting has no cost.

### 2.2 What the design must respect

- **Nothing is placed to manufacture a result** (GEO.17), and **no outcome is imposed** (Law 16). A town is never
  created by a rule "above N people, make a town". Building concentrates because of mechanisms, and a town is
  **recognised**.
- **No decorative map** (GEO.14). Every building shown is a real unit of capital owned by a named party.
- **Read, don't re-derive** (Law 4). A settlement's boundary and population are a read of what stands where.
- **The budget.** A finer grid must be sparse: only land in use is stored.

### 2.3 The design

**The parcel layer** (a finer grid, sparse):
- Each 10 km tile is subdivided into **1 km cells**, a hundred a tile. A cell's identity is its tile's identity and its
  place within the tile. Coordinates, distance and terrain are read from the tile and the cell's offset, so nothing is
  stored for empty land.
- A cell is **stored only once something is on it**: a building, a parcel sold or leased, a road. At five million
  persons that is on the order of 10⁵ cells, not the 4 million the map holds.
- 1 km is the size at which the real statistical definition of a settlement is written (below). A finer cell (500 m,
  250 m) is the same map subdivided again, a RESOLUTION the budget sets (N8.5).

**Buildings** (real units of capital, CAP and HSG):
- A **building** stands on a cell. It is owned by a named party and holds a declared kind: detached house, apartment
  block, shop, office, factory, warehouse, school, clinic, town hall.
- It has floors, a floor area, a condition and an age. Dwellings, plant and shops are **units within buildings**, so
  a flat is a unit in a block and a firm's plant fills a factory.
- The zone-and-class counting of REP.24 stays for markets. A unit's cell replaces the tile drawn when it matters
  (REP.23), because the cell is now recorded where there is a building.
- **Height is a decision**: a builder chooses floors where the extra floor's expected price covers its rising
  construction cost. Where land is dear, buildings rise, and towers appear in centres by the same arithmetic as real
  cities: the height elasticity of Ahlfeldt and McMillen (2018).

**The mechanisms that make building concentrate** (each a real one, each with its source):
1. **Commuting**: a job has a site and a dwelling has a cell. The trip costs time and money, read from distance and
   the roads' capacity (GEO.4, FRT). It enters the household's choice of home and job (HH.8, LAB) as Leisure and Means
   in its mind.
2. **Land prices by cell (bid-rent)**: land is sold and leased per cell (HSG.18, HSG.20 made finer). Each bidder bids
   what the parcel is worth to it: a household for the commute it saves, a firm for the customers and staff in reach,
   a builder for what it can build there. Prices fall with distance from where activity is, as in Alonso–Muth–Mills,
   because that is what the bids say, not because a gradient is declared.
3. **Agglomeration**: a firm's productivity rises with the density of jobs within reach. This is a declared
   TECHNOLOGY with its elasticity from the literature: about 0.02–0.05 (Ciccone and Hall, 1996; Combes et al., 2012).
   Firms therefore gain by clustering, which raises wages there and draws workers.
4. **Reach**: the retail logit's distance term (already declared) and firms' search for staff within commuting reach
   make a site near people worth more to a seller or an employer.
5. **Public services**: a school, clinic or station is built by a public agency or municipality where demand is,
   and it raises what nearby parcels are worth to households.
6. **Migration** (POP.8): people move to where their minds see better prospects. That feeds the growth of what grows.

Together these produce the self-reinforcing growth of the new economic geography (Krugman, 1991; Fujita, Krugman and
Venables, 1999), limited by land prices and congestion. Where the geography favours it, on a coast, a river, flat
land or a deposit, towns take hold; where it doesn't, they don't.

**Recognising a settlement** (a read, by the statistics agency):
- The agency applies the **Degree of Urbanisation**, the definition the UN Statistical Commission adopted in 2020 for
  international comparisons, to the 1 km cells:
  - an **urban centre** is contiguous cells of at least 1,500 persons a km² holding at least 50,000 persons;
  - an **urban cluster (town)** is contiguous cells of at least 300 a km² holding at least 5,000.
- These thresholds are the statistical definition. The world does not act on them: a settlement is **recognised**
  and published (named, bounded, counted). It exists as a fact about what stands where, and it goes when that thins
  out.

**Municipalities** (a real party, by law):
- Where a country's law provides (POLICY), a recognised settlement may **incorporate** as a municipality: a public
  party with offices held by persons (a mayor, a council, elected; PTY.16–17). It levies local taxes, zones its land,
  and builds local roads, schools and clinics.
- Zoning is where a municipality's mind meets the land market: permitting height, or protecting a centre.
- Incorporation is a legal event with a cause; the number of municipalities is never declared.

**The opening** (a snapshot of the present, GEN.5):
- The opening world already has towns. The group's **urbanisation share** and its **city-size distribution** place
  them.
- Sizes follow Zipf's law, as measured: the exponent near one (Gabaix, 1999; Rozenfeld et al., 2011). Density
  declines from each centre by Clark's negative exponential, with gradients measured by city size (Bertaud and
  Malpezzi).
- Centres go on the land the geography favours: coasts, rivers, flat ground, deposits.
- Buildings are opened to house the households and firms the opening draws, each on a cell.

### 2.4 What changes in the spec

- **GEO**: the cell layer as a RESOLUTION of the same map (GEO.1, GEO.18). Cells are stored when used.
- **CAP / HSG**:
  - buildings as capital with floors, units within them, and height as a builder's decision;
  - land per cell;
  - commuting as a cost of the dwelling–job pair.
- **REP.23–24**: the cell recorded where a building stands, replacing the tile drawn when it matters; the zone kept
  for markets.
- **TEC**: agglomeration as a declared technology.
- **STA**: the Degree of Urbanisation as a published read.
- **A new public party, the municipality** (with SOC, TAX, POL): offices, local taxes, zoning, local services.
- **MEASURES** (N3): Zipf's law in city sizes; density gradients; an urban wage premium of the measured size;
  land-price gradients; rising heights in centres; towns rising and declining with their economies.

### 2.5 How it is built, and what it costs

- **GEO kernel**: cell identity, coordinates and distance as arithmetic on the tile's. A sparse map from cell to
  what stands there (built cells only).
- **Buildings**: a holding of a declared kind, 32 bytes a building. At five million persons, ~1.6 million dwellings
  in ~0.5 million buildings, plus firms' and public buildings: **~20 MB**.
- **Land market per cell**: builders' and households' reviews already run monthly. Candidates are a few cells near
  the chooser, scored by its mind. A few milliseconds a day.
- **Commuting**: a dwelling–job distance read at the decisions that need it, with no daily cost beyond the pay-day
  and review reads.
- **The settlement read**: monthly, by the agency, over the built cells only (~10⁵). Well under a millisecond a day
  amortised.
- **Display**: the phone draws cells and buildings from the save's holdings, and a zoomed-out view draws recognised
  settlements. Nothing is kept for display alone (GEO.14).
- **Order**: with housing (S2.05) for buildings, land and commuting. Agglomeration comes with firms' site decisions,
  and municipalities with the state's later steps. The opening's cities come with the native opening (S1.24 i, or
  the step the owner names).

### 2.6 Sources

- **In hand:** internal migration by education (`people/typed_internal_migration.csv`); the map and its generator.
- **Needed from the literature** (tables of published papers):
  - Degree of Urbanisation thresholds (Eurostat–OECD–UN, 2020);
  - Zipf exponents (Gabaix, 1999; Rozenfeld et al., 2011);
  - density gradients (Clark, 1951; Bertaud and Malpezzi, 2003);
  - agglomeration elasticities (Ciccone and Hall, 1996; Combes et al., 2012);
  - the height elasticity (Ahlfeldt and McMillen, 2018);
  - commuting time costs (Small and Verhoef).
- **Not in hand:** urban population shares by group (WDI `SP.URB.TOTL.IN.ZS`) and city-size data. This is an owner
  decision to fetch (§3).

---

## Part 3 — The owner's answers (2026-09-28)

The direction is accepted, both parts as proposed, with these answers. Where an answer differs from the text above,
the answer stands.

**When and how**
- **The mind** is built in **its own stage after Stage 3**, once credit, failure and the markets exist and the offices
  of firms and banks are built. It replaces each decision's rule **one by one**, each rule retired in the same change
  as its move, with no second version kept.
- **Places** are built **with housing (S2.05)**: cells, buildings, land per cell and commuting.
- **Over budget**: the detail of minds and places gives way first (fewer character types, coarser traces, fewer options
  considered, larger cells); the persons the world holds are cut only last.
- **Data**: a one-time fetch of exactly the items listed in §1.9 and §2.6, into `data/sources/raw/`, after which the
  data are in hand again.

**The mind**
- **Concerns**: the eight proposed.
- **Concern weights**: simulated by a declared procedure from the GPS dimensions in hand (patience, risk taking,
  altruism, trust, positive and negative reciprocity); the World Values Survey is not fetched.
- **Character by country**: drawn around each country's own GPS profile, matched to its group. Migrants carry theirs,
  and their children draw from their new country's.
- **Children**: a newborn's character is drawn independently of its parents', from its country's distribution.
- **Age**: risk aversion, patience and concern weights drift with age along the profiles the literature measures, on
  top of experience.
- **Choice**: declared per decision. Random-utility best choice for most; satisficing where options arrive one at a
  time (job and house search).
- **Experience traces**: markets and prices; work (unemployment, layoffs, one's firm failing); home and debt
  (foreclosure, arrears, default, eviction); crises and disasters.
- **Goals**: home and family; career and business (founding a firm, reaching a board or the chief executive's chair);
  politics and public life (standing for parliament, mayor, head of state or government); education, retirement and
  moving.
- **Learning**: from the person's own record and from **peers it knows** (colleagues, kin, neighbours) whose outcomes
  it saw: social learning, only from what it could observe (Law 12).
- **Households**: adults' views combined by each one's share of the household's income; long divergence feeds
  separation (POP.17).

**Offices**
- **Mandate**: it enters only through the holder's contracts and the threat of removal (pay in shares, bonuses, a board
  or voters removing a holder whose results disappoint). No office declares a fixed weight for its mandate.
- **Self-interest**: office holders may act in their own interest against their mandate **within the law**
  (empire-building, perks, short-termism, political favours) where incentives and oversight allow; boards, voters,
  auditors and courts can catch and remove them. Crime is out of scope.
- **Selection**: by the candidates' records, their networks (past colleagues, kin, fellow members), their public
  reputation, and pay and competition between institutions for them.
- **Constitutions**: each country's (parliamentary or presidential) is a setup choice or drawn from its group's
  profile.
- **Founders**: persons found firms by their minds, and so do firms (spin-offs) and funds (ventures).

**Following lives**
- **Recorder**: every person's key life events are recorded compactly and **deleted at their death**, except those of
  office holders (presidents, governors, chief executives, ministers, mayors, founders), which are kept as the world's
  history.
- **Narrator**: a small language model **on the phone** writes a followed person's biography from its record, outside
  the world; nothing it writes enters the world.
- **The player's household**: its persons have minds like everyone's, which **advise**: they say what they would do
  and why, and the player decides or delegates.
- **Reasons shown**: the concern that tipped each decision.

**Places**
- **Cell size**: a resolution set by the budget, starting at 1 km and refined while the phone allows.
- **Buildings**: kind, floors, floor area, condition and age, **plus their footprint and exact position within the
  cell**, so the map is stable and streets can be drawn.
- **The opening's cities**: the largest are placed from data (urbanisation share, city sizes by the measured law, on
  favourable land); smaller towns form during settling from the mechanisms alone.
- **Transport**: roads whose capacity is shared and congests, and transit (buses, trams, rail) that municipalities and
  the state build, with fares and times in the choice.
- **Agglomeration**: productivity rises with the density of jobs within reach, more near firms of the same industry,
  and larger labour markets match better through the hiring round itself.
- **Municipalities**: a recognised settlement can incorporate by law, with an elected mayor and council, local taxes,
  zoning and local services.
- **Zoning**: by municipalities' elected offices, within national law.
- **Countryside**: farmland, forests and mines are parcels on cells, owned or leased, with farm buildings; villages form
  as towns do.
- **Display**: zoomed out, regions and recognised towns; zoomed in, cells with their buildings drawn from the save's
  holdings.

**What follows**: the MND system and the changes to GEO, CAP, HSG, REP, TEC, STA and the polity are written into the
specification, and the steps into the plan (a stage for the mind after Stage 3, places into S2.05), each in its own
change; this file is then removed.

## Part 4 — Risks

- **The budget.** The bench is above budget today (2.52 s against 1 s). The mind and the cells are cheap by estimate,
  but only the bench decides. If they break it, the representation changes first: fewer character types, traces
  quantised, attention shorter.
- **Too many primitives.** Concerns, types and traces multiply parameters. The guard is that each is a declared
  primitive with a source, and that the mind *replaces* the many decision forms rather than adding to them.
- **Tuning by the back door.** Concern weights are the easiest place to make results look right. They are drawn from a
  sourced distribution once, at birth, and never adjusted after seeing the world (N7, GEN.11).
- **Explainability.** A multi-concern choice is harder to audit than a threshold. Recording the tipping concern for
  followed persons, and the measures of §1.7, keep it inspectable.
- **Emergence may fail.** Towns may not form, or may form everywhere. That is a finding about a missing mechanism,
  fixed in the mechanism (Law 6), never by placing towns.
