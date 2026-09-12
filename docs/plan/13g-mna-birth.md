# Item 13g — Corporate development: where a firm builds, what it buys, and what it is in

**Objective.** WHAT A MANAGEMENT ACTUALLY DECIDES, which is one family and not three: where to put
new plant, whether to build it or buy somebody who already has it, which lines to be in at all, and
whether to take the margin or the share. They belong together because they are the same comparison —
what a unit of capacity is worth against what it costs to get — reaching three different markets:
the plant market, the market for control, and the decision to open or close a line.

Siting comes in from 13c.1, which drew the ground: `mechanisms/firms/invest.ts`'s `project()`
already takes every place-specific term as an argument and was called once only because there was
one place. Evaluating it at every place a firm can operate IS the siting decision; comparing its
answer against the price of a going concern IS build-versus-buy; and a line whose best project earns
less than the hurdle anywhere is a line to leave. Margin against share is the same number seen from
the other end: a firm that prices to fill its plant takes share and gives up margin, and which it
does is an outcome of its own cost and its own hurdle, never a strategy anybody declared.

The market for control: an acquirer with its own valuation bids for a target's shares
in a tender that the target's dispersed owners accept or refuse each from their own valuation, so
the premium clears; funding by cash (a loan or bond the credit market decides) or shares (dilution);
competing bidders; management resistance; on completion the two balance sheets combine in the
register, the target's debt is addressed, its shares cease to exist, its employees carry over or
are separated through the labour market's own path, and the payment equals what the acquirer and
its lenders put up. Firm birth: entry as a founder cell's decision from sector margins and what it
can fund, with a new identity, a balance sheet that balances, plant bought from a producer, and an
age that the things that price it read. Placed before 13h because a buyout bids through this market.

**Read first.** §29 Capital Programme B1, B1.a–B1.d, C1, C2 and `mechanisms/firms/invest.ts` (the decision siting widens); Firm A2, A3; §39 Cross-Border B1 and **M6** (the group and consolidation); §35 M&A (all); §34 Firm Birth A, B2, E3, E4; Equity F3 (votes), C1.b (free float),
D1 (issuance for consideration); Indices B2 (constituent changes); Labour C4; XI-15 (a founder cell
splits); Part XII (a firm trading cheap attracts bids). Code: item 9's `equity` and votes read,
item 7's estate and `reseat`, item 10's investment decision, item 12's ratings and indices, 13d's
`labour.separate`, 13f's syndicated financing.

**Clauses this item meets.** M&A A1–A5, A3.a, A3.b, B1–B5, B2.a, C1–C4, D1–D5, D2.a, E1–E3; Firm
Birth A1, A2, A2.a, A3, A4, A4.a, A4.b, A5, B2, E3, E4; Equity E1, E2, E3 (from item 9's PARTIAL
list); Indices B2 (constituent exit on acquisition, fully).

---

## Findings this item carries

Both are the same sentence — **a name that has gone is still referenced** — and this is the item whose
kernel door resolves every reference to a party that stopped existing.

### A ceased issuer's share line stays live and keeps printing (`12c-1`)

`firm.13` ceases; its share line `equity.firm.13` has `issued === 0` from then on, so its published
book per share reads `Infinity` — and the market still prints 11 every period, with `market.noView`
firing on it because the only parties left in the book are the desks. `XI-8`: no death without a
destination. A company that has ceased has no residual for a share to be a claim on, so its line
should stop trading and its market should close — the estate settles what is left and the claim is
extinguished. Instead there is a live market in the shares of a company that does not exist, priced
by desks alone. Seen at `mechanisms/equity/`, `mechanisms/estate/` with
`foundationSpec('probe', drawBanks(4,'probe'), drawFirms(40,'probe'))`.

### A research desk keeps covering a company that has ceased (`12b-6`)

Reported by the `names` family: `p52 names Reporting C2: an estimate names firm.17, which has
ceased`, and the same for `firm.11` at p53. Coverage is initiated and dropped for reasons the desk
has (`§48 D1`, `D3`), and death is not one of them: nothing in `cover` asks whether the name is still
alive, so a desk goes on publishing a view of a company that no longer exists. The estate gives death
a destination; being dropped by the analysts who covered it is part of what happens to a name. Seen
at `mechanisms/research/`.

---

## Design

### Sub-item 13g.1 Kernel: merging two parties

- `parties.merge(target, into, consideration)` (a `MechanismContext` door, journaled): every
  register row of the target (holdings, loans as creditor and debtor, invoices, contracts, employment
  rows) is **reseated** to the acquirer in one operation (A4: one party afterwards); the target's
  shares are redeemed against the consideration instructions already settled (A2); the target party
  is terminated with the acquirer as its **successor** (the names family resolves every reference to
  it: D4, E2); the target's own instruments (its bonds) keep their identity with the acquirer as
  issuer (A5: assumed) unless a change-of-control term makes them due (a bond term from 13f: the
  profile's `due` fires on the journaled event) or the acquirer's decision repays them.
- The door is used by acquisitions here and by 13h's buyouts (the holdco is the acquirer).

### Module `corporate-control`

- `requires: ['equity', 'firms', 'corporate-credit', 'bank-lending', 'labour', 'indices',
'ratings']`.
- **Intent** (B1): in `firms.decide`, a firm evaluates targets it can see (listed firms: public
  prints and published accounts): its own valuation is the target's expected earnings **to it**
  (from its outlook of the target's line plus what it expects to change: cost lines it would remove,
  a market position: its own estimate, journaled privately) discounted at its own hurdle (item 10),
  against the price; a positive gap above its own margin of confidence (its confidence read) is an
  intent. No screening threshold parameter exists.
- **Funding** (B3, A3): the acquirer decides cash, shares or a mix from its cost of capital: cash
  from its balance above its buffer, a loan quote (item 6; a syndicated loan through 13f when large)
  or a bond it can bring (13f: the commitment market runs in the session before the tender so the
  financing is known); shares by issuing new ones as consideration (A3.b: dilution is the register's
  arithmetic; Equity D1). The credit market decides which cash deals happen (B3: no financing, no
  bid).
- **The tender** (A1, B2, C1, C2): a **tender market** per target per bid, in `markets`, a variant
  of the primary block: the acquirer posts a bid (price per share, consideration form, an
  **acceptance condition**: the fraction of shares it needs, a term the acquirer chooses, typically a
  majority of votes: Equity F3), and every holder's participant (cells, funds, insurers, insiders)
  decides to tender its holding or not from its **own** valuation (C1: the price beats holding on
  its own outlook; C2: dispersed decisions summed); the solver clears the tender: tendered ≥ the
  condition → the bid completes at the bid price (B2: the premium is the cleared price over the
  last print); below the condition → the bid **fails** (B2.a: journaled `bid.failed`; the acquirer's
  and the target's prints move because participants acted on the information, not because the event
  did: Observer B2.a); the acquirer may return with a higher bid in a later period (its decision).
- **Competition** (B4): two bids on one target in one session are two tender markets whose holders
  tender to the one that leaves them best off (their participant sees both public bids); the higher
  bid fills first; the other fails.
- **Resistance** (C3): the target's management publishes a recommendation (a public journal event
  from its own valuation of the firm under its own hurdle: it is an interested party: its insiders'
  holdings do not tender (Equity C2.e) and its recommendation is one input other holders' outlooks
  read as public information); management cannot block a bid that clears (the owners decide).
- **After** (D): the merge door; D1: the combined cash flows are the sum plus whatever the acquirer
  actually changes (a cost line removed is a real separation through `labour.separate` (E3, 13d), a
  supplier contract ended is a real invoice stopping); D2: the acquirer's leverage read rises when it
  paid cash and every holder's PD model and the agency reassess (D2.a: its bonds may fall on the day
  its shares rise: two reads); D3: the target's shares are redeemed and the index rebalances at the
  next chained rebalance (Indices B2); D4: employees, suppliers and customers carry over by the
  reseat; D5: an audit contribution per deal: consideration paid to the target's holders equals what
  the acquirer's accounts and its lenders put up, from the ledger, exactly.
- **C4** (cheap firms attract bids) is a read at 16.

### Module `firm-birth`

- `requires: ['households', 'firms', 'capital-programme', 'goods', 'bank-lending', 'ratings']`.
- **Entry** (A4, A4.a): in `households.decide`, a cell with wealth above what a plant of the kind a
  sector's recipe needs costs, and an outlook of that sector's margin above its own hurdle
  (patience), **founds** a firm: a split of the founding members from the cell (XI-15: the founders'
  cell keeps its household identity; the firm is a new named `firm` party with a new identity: A1)
  funded by an instruction founders → firm for the equity (A2: from the founders' account; the
  founders receive the firm's shares as a strategic holding: item 9's flag), and optionally a loan
  the bank decides (item 6); A4.b: the opening size is what the founders fund; A2.a: the firm buys
  its plant from a capital-goods producer (item 10's spend, with the build lag) — no endowment.
- **Balance sheet** (A3): cash and shares only, balancing by construction; A5: it enters the sector's
  markets as a competitor from its first production (item 4's decisions run for it from period one:
  B1).
- **Age** (B2): `founded: Period` is a term; every PD model (item 5/6 shape) and the ratings module
  read age and history length: a measure over a window longer than the history is `Missing` and a
  rating with a Missing volatility component is the agency's stated coarse grade for "no history"
  (data), never the best; a lender's PD for a young firm reads its short history as such.
- **Population** (E3, E4): entry and death are decisions and events; a long-run test asserts the
  population, its age distribution and its sector mix move with conditions and are never equal by
  construction.

### Productivity that improves with cumulative output

A firm's productivity is its **technology**: a declared primitive, the recipe's hours scaled by that
firm's own number. Nothing makes a unit cheaper with experience, and nothing makes a firm better at
making a thing by having made more of it. `Firm A3` requires firms to differ in cost, and they do —
by a dispersion set at the seed that then never changes. Relative cost is therefore fixed at birth up
to scale: **an entrant can never out-learn an incumbent** (`Firm Birth A4`, `A5`), the capital
programme's expansion can only ever buy capacity and never a lower unit cost, and nothing in the
world gets cheaper to make. A model whose costs only ever move with input prices has no supply side
of its own. It lands in this item because the entrant is what it is about, and it is upstream of the
recipe work (item 15) because it changes what a unit costs.

- **A drift is a written path** and `Appendix B` forbids that class of number, so the admissible form
  is TECHNOLOGY keyed to **cumulative units actually produced** — a state the register can answer
  (the journal's own `production` legs by maker and good), carried by the firm, with the rate of
  decline declared and owned. A firm's unit cost is then a consequence of what it has MADE rather
  than of how long it has existed.
- It lands in unit cost (`Goods B5`) and therefore in the offer, the wage bid and the margin, which
  is why it cannot be a display number: the same read every other cost line goes through.
- An entrant with a better rate of decline overtakes an incumbent that has made more, or it does not,
  and either way the crossing is an OUTCOME nobody wrote (`Firm Birth A5`).
- R&D, if it is ever wanted, is a different mechanism — a spend, a lag, an uncertain outcome — and
  not this one. (`research` in this specification is sell-side research: `§48`, an assessor's
  estimate, not a firm's spending.)

### Parameters

`tender.acceptance` is a **term** of each bid (the acquirer's choice); `rating.noHistoryGrade`
(data, assessor); `firm.learningRate.<good>` (technology, owner stated: the decline in hours per unit
per doubling of cumulative units made) and the cumulative count itself is a READ of the journal, not a
parameter and not a stored aggregate. No `mergerRate`, no `birthRate`, no `synergy.*`, no
`productivity.growth` path and no screening threshold exist.

### Audit contributions

- `flows`: M&A D5 per deal from the ledger.
- `names`: a merged target resolves to its successor everywhere; a new firm's identity is new.
- `units`: the firm population is a read of parties; every founder split is a weight event with
  a cause.

### Files

```
packages/engine/src/world/context.ts, register/instruments.ts (13g.1: parties.merge)
packages/engine/src/mechanisms/corporate-control/{index.ts,intent.ts,funding.ts,tender.ts,resistance.ts,after.ts}
packages/engine/src/mechanisms/firm-birth/{index.ts,entry.ts,age.ts,learning.ts}
packages/engine/src/mechanisms/equity/… (a ceased issuer's line stops trading), research/cover.ts (coverage dropped on death)
packages/engine/src/mechanisms/firms/… (owner preferences, targets seen), ratings/… (age)
packages/engine/test/{merge-door,intent,tender,competing-bids,resistance,consideration,after-merger,firm-birth,age,population}.test.ts
```

---

---

## Carried in from 13c.1 — a hull cannot be repositioned, so ballast is not built

Freight's capacity is the hulls a carrier has FREE, read through the lien a voyage binds them with.
But `vintagesHeld` reads a party's plant in its OWN region (`registry/physical.ts`:
`if (terms.region !== view.self.region) continue`) and a plant vintage is an instrument per
`(kind, region, serviceDate)` — so a hull that lands elsewhere is still an instrument of the region
it was built in. Moving one needs a plant reseat the kernel does not have: `Instruments.reseat`
changes an issuer, not a place.

What 13c.1 built instead is a carrier serving the legs out of where it is based. That keeps
everything Freight B2 and E2 ask for — capacity fixed in the short run, none of it without an owner,
none counted twice — and loses one thing: a shortage on one leg cannot pull hulls off another, so
freight capacity does not reallocate and a busy leg stays dear longer than it should.

It lands HERE because it is the same door a firm needs to sell a working vintage to somebody in
another place, which this item already owns: ballast and second-hand plant are one mechanism.

---

# Where a firm builds, and whether to build at all (folded in from 13c.2)

## The decision

### What is place-specific, and what is not

| | place-specific | why |
| --- | --- | --- |
| `offers` (what a unit of plant costs) | **yes** | the plant market is per region, and plant bought elsewhere must be shipped in |
| the wage inside `contributionPerUnit` | **yes** | one labour venue per (region, occupation), each clearing its own |
| the ground inside the yield | **yes** | 13c.1's `groundFor` |
| input costs inside `contributionPerUnit` | **yes** | the input's price where it is made, plus freight to here |
| what the output fetches | **yes** | the price where it is sold, less freight from here to there |
| `hurdle`, `horizonPeriods`, `cost`, `spendable` | no | a management's patience and a firm's cost of capital are the firm's, not the place's |
| `gap`, `capacityNext`, `surpriseWidth` | no | they are about this firm's own demand and its own plant, wherever they stand |

So the loop is: for each candidate place, build the place-specific arguments and call `project()`.
Take the project with the **highest return over what it requires** — a comparison of two numbers the
function already returns, never a weighting of factors.

### The delivered margin

A project at place P for a firm whose buyers are at H earns the price at **H**, less the freight from
P to H, less what it costs to make at P (ground, wage, inputs delivered to P). Every term is a print
or a read that 13c.1 supplies. **Nothing is estimated and nothing is discounted by a distance
factor**: the freight is what the session charged on that route, and if the route has never cleared
there is no freight print, so the place is not a candidate — which is a real answer, not a gap to
fill (Missing is Missing).

### Where a firm may look

A place is a candidate when the firm can actually operate there, and every condition is a read that
answers `Missing` when it cannot:

- a labour venue for the occupations the recipe needs, with a wage that has cleared;
- a plant market with an offer for every kind the recipe needs (`project()` already returns `none`
  otherwise);
- a route from P to where it sells, with a freight print;
- for a foreign place, an account in that country's money and a rate to convert the comparison at.

**A place with no wage is not rejected — it is not a candidate**, because the firm has nothing to
compute with. As the world fills in, the choice set widens by itself. No stub, no default, no
special case.

### Across a border: a branch now, the group at 13i

A firm building in another country holds foreign plant, hires at a foreign venue, sells in foreign
money and funds itself at home. Every mechanism that needs exists after 12's currency layer and
13c.1's places — so what it builds is a **branch**: foreign plant owned directly, on the firm's own
balance sheet, with a real FX trade against a named counterparty for every foreign payment and no
netting across them.

**What it is not, and this is named rather than skipped**: a **subsidiary** — a separate legal party
with its own balance sheet, its own creditors and its own failure — and **consolidation as a read**.
That is M6, already positioned at 13i, and the COVERAGE row says so. Comparing a foreign project
against a domestic one converts at a **cleared FX print** (never a parity formula), and the record
states that a comparison at a rate is a read while a commitment at a rate is a trade with a
counterparty.

### The literal dies

`TechnologyDecl.terms.region` was a bare literal — a SHAPE with no scheduled death. 13c.1 gave it a
reason (the seed draws a line onto ground that suits it). **This item is its death**: where a line
is becomes an outcome of a decision, and the seed's placement becomes an opening position like every
other endowment rather than permanent structure.

### What this item does NOT do

- **It does not close a plant.** Exit is a firm deciding a line is not worth running, which is
  XI-3/Firm D's territory and reachable only once a place can be unprofitable for long enough to
  matter. Named to 13g with firm birth, where entry and exit belong together.
- **It does not move existing plant.** Plant is built where it is built; a firm that wants capacity
  elsewhere builds it elsewhere. Relocating a working vintage is a sale to somebody who wants it
  there, which needs a second-hand market across places — named to 13g.

---

---


## Carried in from 13c.1 — good ground stopped the capital programme binding

`test/capital.test.ts` went 12 green to 11 red when the ground reached the yield, and every failure
is the same shape: `capital.commissioned` is empty where the test expects one. Measured on
`ranWorld('cap-c', 4)`: plant standing on 4,894 km2 of `us.1`, whose arable walk gives
`groundFor(..., 4894) = 1.3986`, so survival is `pow(0.92, 1 / (season x 1.3986))` — about 0.942
against 0.920 — and the same plant and the same hours bring in more tonnes.

Not a defect but a consequence: the world got more productive, so plant stopped binding in a fixture
built to make it bind. It lands HERE because this item rewrites the decision those tests are about —
`project()` is evaluated at every place a firm could operate — so `tightWorld` is rebuilt in the same
change and re-tightened then. Ruled out already: the piece-to-named-unit conversion (fixed at the
site) and the ground read itself (1.649 on the best tile of `us.1`, 0.109 on the single tile of
`jp.2`, which is the dispersion the draw made).



## Steps

- [ ] **From item 11 (Banks Capital A3, C2.a, B3)**: a bank raises EQUITY, and breaching its buffer restricts what it distributes. Item 11 built the subordinated layer and the raise that can fail (C2.b), but a bank here has no share line and no owners: who owns a bank at the seed is what Seed E1/E2 refuse to invent, and a party comes to own one by funding its entry — which is this item. Test: an issue dilutes the holders there are, a failed one leaves the bank where it was, and a bank below its own line pays nothing out

- [ ] 13g.1 Kernel: `parties.merge` reseating every row, redeeming the target's shares against settled consideration, terminating the target with a successor; change-of-control terms fire; tests (A2, A4, A5, D4, E2)
- [ ] Intent from the acquirer's own valuation at its own hurdle against the price with no threshold parameter; the funding decision among cash, loan, bond and shares with the credit market deciding; tests (B1, B3, A3, A3.a, A3.b)
- [ ] The tender market: a bid with price, consideration and an acceptance condition; every holder tenders or not from its own valuation; the premium clears; a failed bid is an event and the acquirer may return; tests (A1, B2, B2.a, C1, C2, E1)
- [ ] Competing bids in one session; management's public recommendation and insiders not tendering; tests (B4, C3)
- [ ] Consideration paid into named accounts equals what the acquirer and its lenders put up: the D5 contribution; the target's debt repaid, assumed or triggered; tests (D5, A5)
- [ ] After: summed cash flows with real changes only (separations through labour, contracts ended), leverage and credit reassessed with bonds and shares moving separately, index rebalance on exit, employees and suppliers carried over; tests (D1–D4, E3)
- [ ] C4 as an observer read; scenario test: a firm trading below a buyer's valuation attracts a bid (direction only)
- [ ] Firm birth as a founder cell's decision from sector margins and its own funding: a split, a new identity, equity from the founders' account, a loan the bank decides, plant bought with a lag, a balance sheet that balances, a competitor from period one; tests (§34 A1–A5, A2.a, A4.a, A4.b, B1)
- [ ] Age read by every pricer: history-based measures Missing over short histories; the agency's stated no-history grade; a lender's PD reads a short history as such; tests (B2)
- [ ] Population, age distribution and sector mix as reads that move; a long-run test that births and deaths are not equal by construction; tests (E3, E4)
- [ ] A name that has gone is gone everywhere (`12c-1`, `12b-6`): a ceased issuer's line stops trading and its market closes with the claim extinguished at the estate, and the desks that covered it drop coverage; the names family holds over both; tests (XI-8, §48 D3, Reporting C2)
- [ ] Productivity that improves with cumulative output: TECHNOLOGY keyed to the units a firm has actually made, read off the journal's own production legs, landing in unit cost and therefore in the offer, the wage bid and the margin; an entrant can out-learn an incumbent and the crossing is an outcome; tests: no written path, no stored cumulative aggregate (Firm A3, Firm Birth A4, A5, Goods B5, Law 2)
- [ ] Observer: bids, tenders, premiums, completed deals, births, cumulative output and unit cost per firm; year-long run green; determinism; coverage re-marked; Equity E1–E3 closed; record entry
- [ ] **A project has a place.** `Project` gains the place it would stand in; `project()` takes its
- [ ] **The candidate set is what the firm can read.** A place is a candidate when it has a cleared
- [ ] **The delivered margin.** `contributionPerUnit` at P is the price where it sells less the
- [ ] **The choice.** The best project by return over what it requires, and it is a comparison of two
- [ ] **The plant is bought where it is made and shipped to where it will stand.** The order lands on
- [ ] **Across a border, as a branch.** A foreign project pays in that country's money through real
- [ ] **The literal dies.** `TechnologyDecl.terms.region` becomes an opening position rather than
- [ ] **Measure.** A year-long run; everything it reports goes in `docs/BUGS.md` before anything is
- [ ] **The documents.** `ARCHITECTURE.md` on the project's place; `COVERAGE.md` — Capital Programme
- [ ] Delete this file; worklist row 13g → done; commit and push

## Exit criteria

A bid can fail because the owners refused; a completed deal leaves one party with every row of two
and a payment that balances to the ledger; a firm is born only from a founder's account and buys its
plant; a newborn is priced as one.

## Guard

M&A B5, E1–E3; Firm Birth A2.a, E1–E3; Equity C2.e; Observer B2.a (the event does not move the
price; the participants do); Appendix B (no written productivity path); Law 19 (cumulative output is
read off what was produced, never accumulated in a second place).
