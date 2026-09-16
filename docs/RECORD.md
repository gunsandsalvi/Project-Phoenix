# The record

> **Entries before this line cite `docs/AUDIT.md`.** That file was superseded by
> `docs/IMPLEMENTATION.md`, which carries every finding that was still open; the closed items are
> recorded here. The old file's item numbers are kept in these entries as written — a ledger is not
> rewritten — and `docs/IMPLEMENTATION.md` Part 3 carries the mapping from them to the new ones.

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
regenerated. Also added Corporate Credit C11, C11.a–C11.e: a placement has a stated basis — best
effort (the bank is an agent: no commitment, no balance-sheet risk, a lower fee, the unplaced
remainder never issued) or backstopped (the underwriter commits and is paid for the risk; the
syndicate is the shared form) — chosen by the issuer from its own outlook of the book against the
fee gap; FORBID: no best-effort deal that leaves the agent holding paper; VERIFY: the backstop fee
exceeds the best-effort fee and weaker or less confident issuers buy it more often. Built at 13f.

**Checked, already present.** Personal expectations: §46 A2–A3 and XI-16 state that every party
acts on its own outlook formed from its own history, that outlooks disagree, and that the
disagreement is load-bearing (two sides of a book, threshold crossings under a mean-preserving
spread, the transmission of a shock). Nothing to add to the spec; the digest in `CLAUDE.md` and
`docs/PLAN.md` 6.2 now say so explicitly. Coverage: 1323 clauses; the new FORBID/VERIFY rows are
MISSING until 13f.

## 3 — The sovereign's funding constraint

**What.** A treasury that must raise money before it spends it. Its **programme** is a read,
recomputed and published every period: what its own paper falls due for over its horizon, what its
standing mandate pays out, what it collected last period, and the buffer it wants. The difference is
the **need**, and it is raised by announcing an **offer** — a size and a walk-away — before the
session. The same solver clears it under a stated allotment rule, `marginalBid`, so every winner
pays the stop-out; the market reads the cover and the tail off the book and journals
`auction.result`; paper nobody bid for is withdrawn, and the withdrawal is that event.

Every bank is a primary dealer with an obligation to bid, out of the cash it has. Its demand is a
two-step schedule — what it needs at the yield it requires, and more only at a premium — so a
heavier auction clears lower rather than at a level somebody wrote down. Its secondary orders are
the same reservation applied to the buffer its deposits moved.

The **curve** is a read: `ctx.curve(family)` builds it when somebody asks, from prints already
produced and the instruments' own cash flows. Every point says whether it traded or was carried; a
tenor between points reads interpolated, beyond them extrapolated, and a family with no points says
none. Nothing stores it, so the fit's own output can never become an observation.

The **central bank** buys in the secondary market only, posting a quantity and no level; its
purchases pay with money it creates. Reinvestment is a separate policy, and with it off the book
runs off and the base shrinks with it. It remits what its own settled instructions earned — income,
never revaluation, because revaluation does not pass through an instruction — and a loss is not
remitted.

**Accrued interest travels with the paper** (Bond N9.b): the asset leg carries the accrued beside
the clean price and the money leg moves the dirty amount, so the next coupon is not a windfall to
whoever holds it on the day. The accrued is a read of the terms, never a stored receivable.

The seed opens with a **maturity profile** — bills at three, six and twelve months and bonds at two,
five and ten years on the issuer's quarterly grid, no two redeeming in one period — each priced at
par on one opening yield, so the seed states a level and no shape.

**Why.** Everything priced over the benchmark needs the benchmark to be issued by a borrower that
can fail to fund itself (XI-9). Without the constraint, a failed auction costs nothing, the
sovereign cannot fail, its paper carries no credit risk and its rating has no consumer.

**Found.**

- The central bank allowed the **treasury** an overdraft. It now refuses everyone that is not itself
  a money issuer: an advance whenever the account is empty converts a fiscal failure into an
  accounting entry (Treasury D3, Central Bank E2, Sovereign A3.b). An outlay it cannot meet fails
  and is journalled as a shortfall, and the next programme sees a need that did not shrink.
- A market kept clearing a **matured line**; markets now run only while the instrument is live.
- `Register.debit` compared its residual against zero rather than against the dust of its own walk,
  and left dust lots behind after taking everything. Both now use one derived dust.
- `issued` is a **running total**, so the tolerance of any comparison against it grows with its
  history. Instruments carry the dust they have accumulated and the ownership family adds it.
- Participants posted **dust-sized orders**, which booked dust lots that then outlived a maturity.
  `core/num.ts` gained `material`: a difference smaller than the rounding of the subtraction that
  produced it is not a decision to act on.
- A holder's reservation was **the curve plus a spread**, so the market could not disagree with the
  print it was about to set — XI-13's fixed point, one level up from the price. It is now the yield
  the holder requires, formed without reading the print, and the short end stopped running away.
- The prices family reported an **unheld line with no print** as a defect. Audit B3 is about what
  anyone marks: a line nobody holds is marked by nobody, and a line that has never traded honestly
  has no price. A held position with no print is still the defect.

**Deleted.** `seed.openingPrice.gov.north.2036` — the auction and the secondary market print the
line now. The single-line seed with it.

**Placeholders.** Three stand, each naming what it stands in for: `seed.openingYield` (an opening
condition the market replaces line by line; it stays while some lines have no second side, which is
Sovereign E2.f at worklist 4), `bank.liquidityBuffer.perDeposit` (the buffer a bank chooses, Money
Market A2.a, worklist 11) and `sovereign.holders.requiredYield` (a holder's cost of funds, expected
loss and capital, Corporate Credit E5, worklist 10). One died and two were added, and the count on
the surface reads three.

**Not met, and named.** Central Bank A3 and A4 are PARTIAL: the mandate's text and target are
parliament's (worklist 14) and there is no policy rate to be independent about until the corridor
(worklist 11). Sovereign D6 is PARTIAL until dealers exist (worklist 9). Treasury B3 and C2 are
PARTIAL until the real economy gives outlays a cycle and receipts a base (worklist 4).

**Departures from the plan file, and why.** The auction is a per-period **offer** posted through a
context door rather than a static block on the market declaration: a market declaration is
permanent and an auction is not, and one instrument still has one market and one print per period,
so a debut and a re-opening are the same act in the same book (Sovereign B3.a). The buyback is a
participant with a reason rather than a phase, because a phase cannot trade. The treasury holds no
module state: its programme is recomputed from reads and published, which is what made it possible
to keep every module a shared, stateless object.

**Forecast, with its killer.** The two-step demand schedule should make a heavier auction clear
lower (Treasury E4). Nothing here measures it; the measurement is Part XII's, and if issuance size
turns out not to move the clearing price, the schedule is not reading its own size.

## 4 — Firms, goods, labour, households, outlooks

**What.** The real economy, as five modules over four new kernel doors, in seven commits.

The **kernel doors** first (4.1). A module keeps its own register in a **state slot**
(`ctx.state(name, initial)`), owned by the module that created it and read by the observer as data.
A party's expectation is asked through **one door with two halves** — what it expects of a variable,
and which variables it has an outlook of at all — answered by exactly one module. Physical things
come into the world and leave it through **`create` and `destroy` legs**, one side each, because
nobody is on the other end of a harvest; a `create` is admitted only under production or the seed,
and only for a kind that says its units are physical. And an instrument kind may declare a
**`revalue`** so a thing carried at cost can be written down, never up unless it is marked both ways.

**Expectations** (4.2). Every deciding party carries its own outlook of every variable it has
observed, corrected towards what it observed at its own speed — a memory drawn once at entry, the
one preference here. Confidence is a read of how wide its own recent surprises have been, and a
surprise is a recorded event. There is no sector outlook and no consensus anywhere: the disagreement
between parties is what gives a market two sides, so it is the thing being built, not noise.

**Goods** (4.3). One instrument kind, a registry of sub-units, a market per (region, sub-unit), and
a **recipe** in physical quantities that lives on the good's own terms — so a firm reads what a
thing takes to make rather than being told. Inventory is lots at what it cost; spoilage destroys
units at their own basis; storage is a fee somebody is paid, and the two are different things. The
units family checks the identity every period: what the stock did against what said why.

**Labour** (4.4). An **employment register** of rows — employer, worker cell, occupation, contract
wage, start, headcount — and one venue per (region, occupation) in hours. Every posting is a bid;
the book fills the highest first and the level is where it cleared, so in a slack market the wage
falls to the seekers' own reservation and no further. A hire moves whole people, splitting the cell
first, and the wage is the contract's: it moves by renegotiation, and a separation costs severance.
Unemployment is a read of the cells with no row; there is no rate anywhere.

**Firms** (4.5). One decision function and no branch on what the firm makes. From its own view it
decides the batch to start, the hours it wants and the wage it will offer, what it will ask for its
output and what each input is worth to it. Production draws the recipe, capitalises the period's
wage bill into work in progress carried at cost, and takes off what the lead time says is due at the
yield — the whole batch's cost landing on the survivors. A period that starts nothing capitalises
nothing. It employs nobody directly and pays no wage: the labour module owns the relationship and is
the one writer of it.

**Households** (4.6). A cell decides for one household and carries how many of them it is. It spends
what it expects to earn, corrected towards the cushion it wants — so many periods of that
expectation, widened by how wrong it has recently been — closing the gap to what it owns at its own
patience, and never more than it holds, because nobody lends to it. What it takes to market is a
demand curve over the range its own surprises make plausible, not a point. What is left after the
cushion goes into sovereign paper when the paper clears what that cell requires of it, and what does
not stays in its account, which is what saving into a deposit is.

**The seed and the state** (4.7). The seed states technology and a stock: recipes, lead times,
yields, one lead time of work in progress on every line, and a level for each market that has never
traded. It states no wealth at all — every household cell opens with nothing — and no employment,
because a seeded row needs a seeded wage and a wage is a price. The **treasury** became an employer
and a buyer: it posts its own openings in the same venue as everybody else at what an hour has been
costing it, its wage bill is a read of its own rows, and it buys real things with a budget the goods
market rations it on like any other buyer. Its receipts now read three real bases off what named
payers actually did — interest received, what households were paid, what they paid for real things.

**Why.** Item 5 is "a loss is an event", and a loss needs somebody with something to lose. Before
this item nothing produced, nobody worked and no household earned; the only cash flows in the world
were the sovereign's. This is the item that gives the model a real cost base, a real income and a
real reason for a price to move.

**Found.**

- Settlement refused a **harvest**: a `create` leg had to sit beside the `destroy` legs of what it
  was made from, so a good made out of labour alone was unbuildable. Creation is admitted under
  production and the seed, and the recipe identity — not the leg rule — is what checks it.
- The central bank's open-market operation was buying **grain**. It bought anything with a market
  and a price; it now buys only paper a treasury issued, and a test says so.
- A firm paid **its own ceiling wage**. The labour venue cleared at the marginal bid, so the buyer's
  own number set the price even with the book slack. Sellers compete there now, and a slack market
  falls to the seekers' own reservation and no further (Labour D1).
- A firm planned on the hours that were **productive** last period, which are zero the period after
  a hire because finding somebody takes time — so a firm that hired could never plan on having
  hired. It plans on the hours under contract.
- Settlement's pre-check and the register's own draw each derived the dust of "can this party
  deliver" **for themselves**, disagreed at the fifteenth decimal, and turned a trade that had
  already been admitted into a throw. `Register.deliverable` is now the one reader of it.
- The **accounts** and **flows** families compared a fresh read against a balance that had been
  moved once per event since the party was born, using the dust of two additions. Over a year of a
  busy account that is a violation a week made of floating point. A balance now carries the walk that
  produced it (`Running`: value, moves, and Σ ε|balance after each move|), and the check adds it.
  This is the same defect the `issued` total had at item 3, in the two places it was left.
- The four numbers the world **opens at** — three goods levels and the sovereign opening yield —
  were declared placeholders with a scheduled death. No worklist item can keep it: a market that has
  never traded has no price, and a world that opens with stock and with paper outstanding opens with
  a level for them (Seed C3, C4). They are SHAPES, and what retires them is a measurement, not a
  mechanism: run the same world from different opening levels and see whether its path depends on
  where its markets opened. The placeholder count fell 6 → 2 and the shape count rose 1 → 5, which
  is the honest pair of numbers rather than the flattering one.
- **Households A2.g** holds exactly as written and is worth stating precisely, because it is easy to
  ask it for more than it says. A mean-preserving spread of income across cells raises the count of
  crossings — 25 cells below the cushion they want become 27 — while the weighted mean does not
  move. It was tempting to expect the sector's consumption to move too, and it does not: consumption
  is still linear in income at every cell, so a crossing here is a crossing with no consequence
  behind it yet. The first one that has a consequence is a default (worklist 5), and until then the
  crossings are the whole of what the representation buys.

**Measured, and what it means.** The **resolution** check (XI-15) runs the same world at one, two
and four cells per key for half a year. The population is exactly the same number of people at every
grain, the audit is at zero every period at every grain, and the stock of goods comes out the same
number to the dust of reading it. What moves is what whole people do: employment differs by at most
one person a venue, and the money follows those people — bounded by what a person earns over the
run, which is a quantity of money and not a percentage of anything. Turning the memory dispersion
off does not remove the move, so it is not only the finer draw of outlooks: the marginal match in a
venue lands on a different person when the seekers are grouped differently, and that is XI-15's
granularity, not error. **The size of that move is the honest error bar on every number the
represented sectors produce**, and judging whether it is material is Part XII's job (worklist 16).

**Deleted.** Three seed shapes: `seed.households.depositPerMember`, `seed.households.bondPerMember`
and `seed.households.dispersion`. What was unequal about households used to be drawn; it is now what
happened to them — who was hired, at what wage, and what each of them did with it. A sector of
equals never produces a market, and this one stops being equal in the first eight periods without
anybody stating that it should.

**Placeholders.** Two stand, and both are item 3's: `bank.liquidityBuffer.perDeposit` (Money Market
A2.a, worklist 11) and `sovereign.holders.requiredYield` (a holder's cost of funds, expected loss and
capital, worklist 10). This item added **none** — the four the plan expected (`firms.terms.periods`,
`seed.wage.<occupation>`, `seed.expectedIncome`, `labour.capacity.hoursPerFirm`) were each avoided by
building the thing instead of standing in for it: trade credit was left to 13e rather than faked, the
wage is cleared, the first outlook is the first observation, and there is no plant to have capacity.

**Not met, and named.** Goods D (freight, 13c), Goods E2.b (a dealer marking its book, 9), Goods G
(indices, 12); Labour A3.b, C4 and E4 (mobility, firm death, debt service — 13d and 7); Households
E1–E4 and F (borrowing and the life cycle, 6 and 13d); Firm A4 and E5 (owners of record and a
dividend, 9), C2 and C4 (payables and receivables, 13e), E3 and E4 (investment against a hurdle, 10);
Treasury B3 (outlays that rise with unemployment need the loss to be an event, 5, and a policy that
varies them, 14); Seed B4 (the household cells open identical and their dispersion is produced
rather than stated, which meets the reason and not the letter).

**Departures from the plan file, and why.** Six.

1. No `firms.invoice` and no `firms.pay`. A goods trade is delivery against payment in one
   instruction, so an invoice book of immediate terms would be a second representation of the trade;
   receivables arrive with trade credit at 13e, which is where the placeholder would have died.
2. Wages are paid by the **labour** module, which owns the employment register and is its one
   writer. The firm decides the employment it wants; it does not pay anybody.
3. No `households.pay`: a purchase is one instruction, and the tax on it belongs to the receipts
   phase that already exists and already has one writer.
4. No risk aversion is declared. Nothing in this world prices risk yet (worklist 9), and a number
   nobody could derive is what Law 2 forbids.
5. Employment is not seeded. The plan wanted rows at a seeded wage with `seed.wage.<occupation>` as
   a placeholder; a seeded wage is a price nobody cleared, and the seeded stock and work in progress
   carry the lines until the venue strikes the first rows in period two.
6. The resolution test does not claim the aggregates are **identical to dust**, because they are
   not and cannot be — see "Measured" above. It claims what XI-15 actually guarantees, and states
   the move as the error bar the spec says it is.

**Forecast, with its killer.** A wage rise should be more demand and more cost at once, and which
dominates should be a result (Labour E2.a). Nothing here measures it. The measurement is Part XII's:
raise the wage in one venue and follow both channels. If output moves only one way, one of the two
channels is not connected — most likely consumption reading something other than what was paid.

## 4a — A line is more than one firm, and firms differ in cost

**Where this landed and why (Law 10).** Inserted between 4 and 5, after item 4 closed. Item 4 ended
green with every audit family at zero, and the first long run after it showed the real economy
running down: households bought the finished good for twenty periods, less every period, and then
nothing for the rest of the year. Item 5 is "a loss is an event", and every loss its scenario
measured would have been measuring the starvation instead — so the fix goes in front of it.

**What.** Three things that are one thing, because fixing any of them alone makes the world worse.

**The wage prints where D1 says.** Item 4.5 set the labour venue's tie rule to `sellersCompete`,
reasoning that a slack market should fall to what the seekers will work for. D1 says the opposite in
as many words: *the bid that took the last match is the occupation's print*. It is now read off the
book — the lowest bid actually allotted — rather than taken from the solver's crossing, because the
crossing is a fact about both sides and in a slack market it sits on a **seller's** reservation,
which is a level no employer offered.

**A line is three firms.** There was one, and one is not a sector (Seed B1). Nine named firms, three
to each line, each with its own bank, cash and opening stock, all from one roster the seed builds
from. The line totals are exactly what they were: this changes the structure of the sector, not the
scale of the world.

**And no two of them alike.** Copies would not have helped — they share the good's recipe, so they
bid the same number and the marginal bid is the common bid. Each firm now has its own labour
productivity: the hours the recipe names, scaled by a number that is that firm's technology, read in
one place. It is the only number the firms module declares, and it is deliberately only labour
(Law 2): what makes one firm's wage bid differ from its neighbour's is what the venue needs two of.

**Why.** Firm A3 says firms are heterogeneous in size, cost and leverage **and that the dispersion
is the reason markets exist among them**. That is not decoration. A venue with one employer has one
bid, so whichever end of the book the rule takes the level from, one side gets the whole surplus: at
one end the employer pays what the last seeker will accept and the wage never rises towards what the
work is worth; at the other it pays what an hour is worth to it and earns exactly nothing. Measured,
both happened — 3.2e-2 bid against a 4.6e-4 print under the first rule, and unit cost 0.900 against
an expected price of 0.900 with no batch started in a year under the second.

**Found.**

- **`marginalBid` was not enough, and the new audit family said so on its first run.** The `prices`
  family gained one contribution — *the wage a venue printed is a level an employer actually bid* —
  and it immediately reported the bakery venue printing 3.4e-4 an hour that nobody had bid. The tie
  rule only breaks ties; the level itself still came from the volume-maximising crossing. D1 does not
  describe a crossing, it describes an allotment, and the print is now the lowest allotted bid. This
  is what a FORBID is for: it broke silently, because a wage looks like a number rather than a trade.
- **A bank now runs out of reserves, eleven periods in fifty-two, every fourth one.** It is the
  auction: a dealer bids out of the cash it has (C3.a) and its own customers move those reserves
  before the allotment settles, and there is no money market to lend it the difference overnight.
  B3.b says a bank overdrawn at the central bank is borrowing from the central bank and the corridor
  prices it — worklist 11, which is also where `bank.liquidityBuffer.perDeposit` stops being a
  placeholder. It appeared the moment wages became real, and the answer to it is the missing
  mechanism, not a smaller wage. It is named in `packages/engine/test/expected.ts`, counted by the
  year-long test, and every other violation any family reports still fails that test.
- **A year-long assertion was passing on nothing.** "Households bought something" counted any asset
  leg reaching a cell, sovereign paper included, so it stayed green while food purchases were zero
  for thirty periods. It counts a physical good now. Written in item 4, found here, which is what a
  long run is for.
- **Households A2.g cannot be measured yet, and is now PARTIAL rather than MET.** Not because the
  sector is an average — the spread is plainly visible cell by cell, and the test asserts it — but
  because the sector sits wholly on one side of every threshold it has: with a real wage every cell
  is below the cushion it wants, and none is up against what it holds. A threshold counts crossings
  only when the sector straddles it. The first one that will is a default (worklist 5).

**Measured, as it came out.** The three structural defects are fixed and the chain still runs down,
so the item states it rather than steering it (Law 11). Over fifty-two periods: all three bakeries
now plan, produce and hire, where one bakery produced nothing; the wage prints 53 times against 2;
household cash goes from 545 to 4,863. And the mills never start a batch, the farms never plan at
all, flour reaches zero, and households stop buying food around period thirty instead of period
twenty-one. Three things stand behind that, none of them what this item was about:

1. **A stage that needs less than a person.** Milling is three hours a tonne, and at this world's
   throughput the whole milling step wants about a quarter of one person's week. A hire moves whole
   people (Labour A4.b), so a mill that wants nine hours can never hire anybody, can never produce,
   and can never grow enough to want a whole one.
2. **A firm that has never sold can never sell.** A firm plans only once it has an outlook of its own
   fills, and an offer that filled NOTHING is recorded as no observation rather than as an
   observation of zero — so the farms, whose only buyer is a mill that never buys, stay silent for
   ever instead of correctly deciding to produce nothing.
3. **A price that cannot express excess demand.** The bread market printed 0.9 in every one of the
   fifty-two periods. The seller offers most of its stock at what it expects less what will perish,
   and the buyer posts a curve over the range its own surprises make plausible — so both sides are
   anchored to the last print, and a market where demand is many times supply clears at the same
   level for a year. A seller's expectation of demand is its own fills, its fills are capped by its
   own output, and nothing tells it there was more.

The third is the one that makes the other two matter, and all three want one item together.

**Deleted.** Nothing. `sellersCompete` remains a rule the solver offers — it is right for a book of
sellers undercutting each other, which is what a goods market is; it was wrong for a venue where
every posting is a bid.

**Not met, and named.** Firm A3 stays PARTIAL for leverage (worklist 6). Households A2.g is PARTIAL
as above (worklist 5). Money B3.c stays PARTIAL, and is now visibly so eleven periods a year
(worklist 11).

**Forecast, with its killer.** A firm that does more with an hour should be inframarginal, so it
should still be hiring when the one that takes the longest has stopped. Nothing here measures it over
a cycle — the test only checks that they rank by productivity today. The measurement is Part XII's:
raise the wage and watch which firms shed first. If they shed in any order but that one, the
dispersion is not reaching the hiring decision and one of the two numbers is not being read.

## 5 — A loss is an event

**What.** A claim that goes unpaid, and a party that could not pay, are now both dated events with
names on them. In two halves, because they are two different facts about two different things.

**The instrument's half, in the kernel.** An instrument states its own **definition of default**
(Bond N12), its **claim on failure** and its **ranking** (N13, N13.a) — the last two required of
every kind, because "stated even when the answer is nothing seizable" and "stated even when all
equal" is the whole of those clauses. Money is an unsecured claim on its issuer; a tonne of grain is
owned outright and nobody promised it; sovereign paper is pari passu, always, with nothing seizable
and exclusion from the market as the sanction. When a coupon or a maturity the kernel applied does
not settle, it asks the profile whether that meets the definition, and if it does it journals
`credit.default` publicly and writes `performing` false — a status one path writes and nothing
restores (Banks Lending E2). A kind may also declare that a default **accelerates** its issuer's
other lines (Corporate Credit G2); being made due is being redeemed now, through the same path a
maturity takes, so an issuer that cannot pay the accelerated face fails that too and each failure is
its own event. A sovereign declares it false (Sovereign G3).

**The party's half, in a module.** `credit-events` reads settlements that failed and statuses the
kernel wrote, and writes down what Money E1 says must exist: a payer that could not pay is **in
default of payment**, publicly, by name, with the payee that did not get paid and the amount that did
not arrive (E1.b). A holder of a claim that stopped performing carries a private **impairment** with
units and a carrying value, so the exposure has a holder and a size (Register E3). It decides
nothing, and it declares **not one number** — no probability of default, no loss given default, no
recovery rate. That absence is the clause.

**And the payee's side became readable.** `unpaid()` reads a failed instruction's money legs, and
`view.failedPayments` gives a party its own failures and nobody else's. It is a reader rather than a
stored field on `Failed`: the legs are the source, and a copy of them would be a second thing to
keep true (Law 19).

**Why.** XI-1 is first of the seventeen because at least four other mechanisms need "a claim goes
unpaid" to already be a thing that happens. Tranching, foreclosure, a credit market's second
opinion and a bank resolution all need an event with a date, a borrower and a recovery; a rate
subtracted from a book gives none of them.

**Found.**

- **The state services its debt before it services its mandate, and nothing said so.** It falls out
  of the canonical period rather than out of a rule: corporate actions are cycle 0 order 2 and the
  treasury's outlays order 5, so a stretched state pays its coupon and fails its transfers. It takes
  a mandate large enough to break both before its own paper defaults. That ordering is now asserted
  in the year-long scenario rather than left to be discovered again.
- **A maturity that fails is a missed payment too.** The first pass hooked only the coupon path, and
  the first thing the test caught was a default arriving on a different line from the one expected —
  a bill's face that fell due and was not paid. Both paths ask now.
- **A phase at cycle 0 cannot read today's mark.** The impairment wanted to say what a holder is
  carrying, and asking the valuation for the current period threw `NotYetProduced` — correctly
  (Clearing F1.a). It reads what the last mark made it, which is what the holder is exposed for at
  that moment, and the lag is stated where it happens.

**Changed from the plan file, and why.** The plan wanted **provisions as lot write-downs to the
holder's own recovery outlook**, unwinding as recoveries arrive. Neither was built, and the reason is
not that it was hard. A provision is a write-down of a claim to what its holder expects to recover.
Every claim in this world is carried at the **mark**, where a write-down would be a second
representation of a number the price store already owns (Law 4) — and the recovery to expect does not
exist, because a sovereign has nothing seizable (G3) and the negotiated exchange that would produce
one is worklist 13f. Inventing a recovery expectation to book against a marked bond is inventing
exactly the rate XI-1 exists to forbid. So the module states the exposure and stops (Law 11). The
door for a claim carried at cost is `profile.revalue`, which exists and is used by the goods module;
the first claim that will provision through it is a loan (worklist 6). Two steps became one, and the
manifest count fell from 14 to 13.

The write-off was kept, as the identity rather than the trigger: nothing in this world extinguishes a
claim yet, but E5.a is checkable now and is checked — a claim leaving the book at what it fetched
moves its holder's equity by exactly what it was carrying, and the issuer's by the same the other
way, out of settlement's own arithmetic rather than a second sum.

**Deleted.** Nothing. `subjectsOf` moved from settlement to `instruction.ts`, where it is a pure
function of an instruction and a party can use it to tell whether an instruction was its own.

**Not met, and named.** Register E3 is PARTIAL: the exposure has a holder and a size, and the loss
lands when there is a recovery to land against — an estate (7) or a negotiated exchange (13f). Banks
Lending E2 and E5, Corporate Credit G1 and G6, Firm D4, Firm Birth C1, C3 and C4 are each PARTIAL
with the half that is missing named: a covenant to breach, a claim carried at cost, a solvency test
with liabilities in it, an estate to realise. Sovereign G2 stays MISSING — a sovereign can only
genuinely default in a money it does not issue, and the currency layer is worklist 12.

**Forecast, with its killer.** A default should be **information** (Corporate Credit G8): it should
move what holders require of every other issuer, which is how contagion travels without a
correlation parameter. Nothing here does that — a holder's required yield is still a placeholder
that reads nothing. The measurement is Part XII's: default one issuer and watch what the others'
paper clears at. If nothing moves, the second opinion is not being formed from what a holder saw,
and the placeholder is standing in for more than it admits.

## 6 — Loans are rows

**What.** A bank lends by creating a deposit, and everything else follows from that being literally
what the instruction does.

**The loan.** An instrument like any other, held in the same register as anything else a bank owns,
with a lender of record, a borrower who issues it, a rate struck at origination and a maturity
placed by date. It is **not a security** (A1.a), so it names no market — the kernel refuses a
cleared kind that names none, which is what makes "carried at amortised cost, not marked to a market
that does not exist" (D1) a thing the type system holds rather than a convention. Interest accrues
by day count on what is outstanding and falls due every period the calendar places, through the
kernel's corporate actions, so a missed one is a default by item 5's machinery with nothing added.

**Writing it is one instruction.** The borrower issues the loan to the bank; the bank creates its
own money into the borrower's account. Both ends of the money leg have the same issuer, so
settlement generates **no interbank leg at all** — B1.a is not asserted anywhere, it is what the
wire does. There is nowhere in the module where a deposit or a reserve is consumed to fund a loan
(B1.c), and the test asserts the instruction's reserve legs are empty.

**The price is C1's four terms and nothing else.** What its funding costs it, what it expects to
lose on this borrower, the capital the loan consumes times what it needs on that capital, and what
it costs to run. Two banks differ in three of the four — how far back they look at a borrower, what
they need on capital, how far above the requirement they run, and how much they will have out to one
name — so they quote differently and the borrower takes the keenest, which is what makes C2 a
negotiation rather than a schedule.

**A customer overdrawn is borrowing** (Money B3.a). The kernel now asks the module the moment a
payment would take an account below zero, and the answer is the same credit decision: the room this
bank's capital supports, its own limit for that name. What it allows is a **drawing on that
borrower's line**, and the line is one row that the drawing moves (C9) — never a new loan every week.
What it refuses is a payment that fails, recorded.

**The provision** (D2). A loan is carried at what its lender expects to recover, from the **same
model** the price was struck with (C4) — two models would mean the price and the provision are
struck against different beliefs. The kernel writes the lot down and moves the equity account by the
same amount, so the charge is on the thing the book carries and there is no reserve anywhere for a
loss to be absorbed into (D2.b).

**Why.** XI-4 calls the bank's cost of funds joint one of the transmission chain, and says what
breaks it: a bank with no cost-of-funds term prices every loan as though it funded at the policy
rate whatever its own position. And Banks Lending B1.c says a model in which a bank lends out of its
deposits cannot produce a credit cycle. Both are structural, and both are now closed.

**Found.**

- **A rule with two writers, and the wrong one deciding.** The register refused to write a lot up,
  and the revaluation ALSO refused it unless the kind marks both ways. Which way a lot may move is
  the kind's business: inventory is written down and never up because nobody but a dealer marks up a
  thing it made (Goods E2.c), and a claim moves both ways because a provision unwinds when its
  holder stops expecting the loss (D2.a). The register's copy of the rule made the second one
  impossible. It is `remark` now, and the rule lives in the one place that knows the kind.
- **The money family had the same too-tight dust as the flows family**, in the third place that
  shape appears: a running `issued` total compared against a period's creation legs with the dust of
  two additions. It surfaced the moment lending made banks create and destroy money many times a
  period. There is one derivation now — `carriedDust` in `core/num.ts` — and every family that
  compares a book against the wire that moved it uses it.
- **Nobody in this world borrows**, and that is the honest state rather than a broken one. Its firms
  hold more cash than they spend; its households never spend past what they hold; and the one party
  that runs out — the treasury — banks at the central bank, which refuses everyone that is not a
  money issuer (Treasury D3). Stress the state's mandate until it cannot fund itself and the credit
  channel comes alive at once: 2 lines, 65 drawings and 22 written commitments over a year, one firm
  borrowing from both banks at 1.30% and 1.62% — the keener bank's line dearer, as its own numbers
  say it should be. What is built here is the supply side, and it answers the moment anybody asks.
  The demand side arrives with investment (worklist 10) and consumer credit (13d).

**Changed from the plan file, and why.** Three.

1. **No cost-of-funds placeholder and no `centralBank.policyRate`.** The plan would have priced the
   loan off the policy rate until worklist 11. The cost of funds is a READ instead — what a bank
   actually paid on what it actually owed. Nothing a bank issues pays interest in this world, so it
   is a true zero, and it becomes the term that differs between banks the moment deposits are priced
   without anything here changing. A placeholder standing where a read works stands in for nothing
   (Law 2), and using the policy rate as a bank's cost of funds is the precise mistake XI-4 names.
2. **No `bank.pd.curve` shape.** The plan would have mapped a borrower's coverage to a default
   probability by a stated curve. What a bank assesses instead is how often it has SEEN that
   borrower fail to pay, over the memory it keeps — a frequency of events it observed, not a hazard
   rate that produces them, and the channel by which a default becomes information about what the
   next loan costs (Corporate Credit G8). No shape was added and the count did not rise.
3. **The workout is deferred to worklist 7, and the undrawn commitment to 10 and 13f.** Restructure,
   extend and enforce are decisions between what each path is expected to bring, and what a defaulted
   claim brings is exactly the recovery that needs an estate to realise something. Building the three
   branches now would mean inventing the numbers that choose between them. Likewise a committed limit
   needs a borrower that asks for a limit rather than an amount.

**Deleted.** The kernel's placeholder refusal of every customer overdraft. `Register.writeDown` — it
is `remark`, and it no longer holds a rule that was not its to hold.

**Not met, and named.** Banks Lending A3.b, A5, D4, D5, E3, E4 and E6 are MISSING with the item that
brings each. B2 is PARTIAL for liquidity: the deposit a bank creates may be spent away and it must
fund that, and there is no market to fund in until the corridor (worklist 11) — a ratio invented in
its place would be a bound standing where a market belongs. E2 is PARTIAL for "impaired" as a state
of its own. Money B3.c is PARTIAL still, but for the other half now: a bank overdrawn at the central
bank has no lender row, which is B3.b's corridor.

**Forecast, with its killer.** Two banks with different appetites should stop lending at different
moments, so a downturn should show one still writing while the other has closed — which is what
makes a credit cycle a cycle rather than a level. Nothing here measures it, because nothing borrows
enough for either limit to bind. The measurement is Part XII's: stress the borrowers until capital
binds, and watch which constraint each bank hits first. If they bind together, the dispersion is not
reaching the decision and the two banks are one bank with two names.

---

## 7 — The forced seller, nothing is immortal, the estate

**What.** Nothing in this world is immortal any more. Every party kind states what it can FAIL on;
each period the two questions those answers name are asked of every party — did something fall due
out of its own balance that it could not pay and still cannot, and are its liabilities past its
assets at marks — and a party that answers yes to either dies. An `estate` module opens an estate
banking where it banked, moves every holding to it by instruction at carrying value, has it ASSUME
everything the dead party issued, and then ceases the party naming the estate, so every reference
still resolves to somebody who exists. The estate sells what it holds into the markets those things
always traded in, at a reservation that falls as its programme runs out and takes whatever the book
gives on the last period; distributes what it realises in rank order, pro rata within a rank, by the
instrument's own stated seniority; abandons what nobody bought; writes off what it never paid, so
the loss lands on the named holders; and then ends, succeeded by nobody, holding nothing in any
account. The labour module reads the party store and releases a dead employer's workers through its
own separation path. The `estate.programme.periods` policy is the whole of the difference between an
orderly wind-down and a fire sale.

**Why.** XI-3 is the reason: an immortal party is the TERMINATION CONDITION of every loss chain in
the model, and a cascade that reaches one stops there without saying so. Every institution in this
world could absorb an unlimited loss until now, which made every measurement of loss a measurement
of that absorption. XI-8 is what makes the death real rather than an accounting step: the assets are
SOLD rather than valued, every claim ranks by the instrument's own seniority, the real-economy
consequences are part of it, and it conserves and terminates.

**Found.** Six, and five of them were defects the deaths exposed rather than caused.

- **Re-seating an issuer was a one-sided flow.** The kernel had a `reseat` door that changed an
  instrument's issuer directly. The liability moved from one balance sheet to another with no
  equity effect on either — a change of state that never went over the wire, which is the one thing
  the wire rule forbids. It is a leg now: `assume`, from the party that owed it to the party that
  owes it from now on, valued at what the holders carry it at (Register B3), and the door is gone.
  The accounts family caught it on the first death.
- **A bank was lending to an estate, every week, to pay the interest on the loan it was winding
  up.** The overdraft decision asked only whether the bank had room, never whether the borrower was
  somebody to have a contract with (Banks Lending A1). A party kind now states whether anybody lends
  to it at all; an estate says no, a household says no (C1.d), and the refusal is recorded. The
  estate's interest simply goes unpaid after that, which is what happens in a liquidation.
- **Three audit families were comparing a walked balance as if it were one reading.** A money
  balance is one lot moved once per leg since the account opened, and what it is entitled to call
  nothing is that walk — the same number settlement already uses when it decides whether an account
  is short enough to ask its issuer for an overdraft. Two tolerances for one fact is Law 4's defect,
  and it showed up as a dust-sized overdraft on an estate that had divided a pot into shares, and as
  a 1.3e-9 gap between reserves held and reserves issued after fifty-two weeks. The register keeps
  the walk now (`moneyWalk`), and the money, names, ownership and estate families all read it.
- **An estate could not see money it was holding.** A loan creates its deposit at the LENDER
  (Banks Lending B1), so a firm that borrowed from two banks banks in two places — and `cash` reads
  the account at a party's own bank. The estate distributed one account and closed holding the
  other. Both the waterfall and the residual check work over every account it holds now, which is
  what D6.a asks for in as many words.
- **A firm that died had its rows read off it in the wrong direction.** Severance is a cost the
  employer pays — while there is an employer to pay it. One that has ceased owes it to the claimants
  on its estate, and D2.b says a claim like that ranks and is paid in the distribution, not in cash
  at the door: a liquidator does not borrow to settle a claim it is winding up. There is no
  instrument for a claim like that to rank AS until payables exist (worklist 13e), so it is recorded
  owed and unpaid on the separation event. MISSING named, not a payment invented.
- **The world now runs a year with firms dying in it and the audit stays green.** In the stressed
  world five firms die over fifty-two weeks — mills that bid the grain price up against each other
  until one of them is paying more for the grain than the flour fetches. Each one's estate opens,
  sells, pays its banks what it realised, releases its workers, writes off the rest and ends. The
  same seed gives the same world twice.

**Changed from the plan file, and why.** Three.

1. **`instruments.reseat` is not a door; it is a leg.** The plan had it as an estate-only kernel
   door with the estate module as its one caller. A door with one caller that moves value with no
   second side is a smaller version of the thing the wire exists to prevent, and the fix removed
   code rather than adding a rule to it (Law 12).
2. **A dead employer's severance is a recorded unranked claim, not a payment.** The plan said the
   estate pays it. It cannot: it has no instrument for the claim to rank as, and paying it in cash
   at the door would jump a waterfall the spec spends four clauses building.
3. **The household trigger has nothing to fire on, and that is the clause.** The plan wanted a cell
   whose members cannot pay to split and record the failed payment. Nothing a household does can
   commit it past its cash (Households C1.d: nobody lends to it), so there is no crossing to split.
   The absence is asserted rather than assumed, because a FORBID that holds breaks silently.

**Deleted.** `MechanismContext.reseat` and its journal event. The kernel's single-account residual
check on a closing estate.

**Deferred, with where each lands.** Two sub-items, and they were split in the plan file already.
Neither is left sitting unticked in a closed item: each is written into the file of the item that can
actually build it, and item 11 grew by six steps for them (Law 10 — inserted at its dependency
position, and this says where).

- **7.5, bank resolution** → **worklist 11**. Valuation of the book, the bail-in hierarchy, an
  acquirer's bid, the public path and deposit insurance all need a bank's capital to be RAISABLE and
  its funding failure to be reachable, and both arrive with the money market and the corridor. What
  is built here is the trigger: a bank states both failures, is asked them like anything else, and a
  failed bank resolves through the same estate as a firm. Banks Capital C1 is PARTIAL and says so.
- **7.6, the other forced-sale doors** → **each to the item that creates its door**, which is what
  the plan file said. The estate IS a forced seller and it is built: a party with no choice about
  selling, a reservation that falls to whatever the book gives, and a print that moves and reaches
  every other holder through their marks — XI-2's mechanism, working. The doors that push somebody
  else through it are a margin call (13a), a redemption (8), a funding line cut (11: a bank's limit
  is a ratio of its own capital, so it takes capital that can fall) and a covenant or downgrade
  (12/13f). The FORBID they must not break — no floor at zero on an available line — holds today by
  the lint rule and by the read itself: `limit − exposure` is published on every refusal, and no
  world yet reaches it negative, which is stated rather than tested green.

**Not met, and named.** Firm Birth D2 is PARTIAL for trade creditors (13e) and close-out claims
(13a); D4 is PARTIAL for suppliers' receivables (13e) and capital going to a buyer (10); E2 is
PARTIAL for contracts that are not instruments, because there are none. Banks Capital C1 is PARTIAL
per above and the rest of that section is MISSING with worklist 11.

**Forecast, with its killer.** A death should be CONTAGIOUS through prices rather than through a
parameter: an estate selling its stock into a thin market should move the print, and the mark
should reach every other holder of that good and reduce what each of them can borrow against it —
so the second death should come sooner after the first than the first came after nothing. Nothing
here measures it. The measurement is Part XII's: run the same seed with the programme long and
short, and count the periods between deaths. If the gap does not shorten, the forced sale is not
reaching the other balance sheets and the estate is selling into a market with no memory.

---

## 8 — Redeemable claims: the money fund, the gate, and the second forced seller

**What.** A fund is a named party whose liability is its shares, whose equity is therefore nothing,
and whose investors can ask for their money back. It has a mandate that constrains what it buys —
bills, and nothing else at any price — a net asset value that is a READ of its book over its share
count, and a manager, a separate party, that it pays a fee to. Households hold it instead of a
deposit and ask for the money back when they cannot cover what they mean to spend. A redemption its
buffer cannot meet is paid as far as the cash goes; the rest stays on the book under the holder's
own name at the NAV struck when it asked; the fund is GATED, publicly; and it sells into the bills'
own market at whatever that market gives, until the queue is paid.

The kernel gained a fourth pricing for it. `derived` is a value that is neither a print nor a cost,
because the thing IS a claim on a book: it is read at every ask, stored nowhere, and the same number
for every holder. It is not an exception to Law 3 — everything the book holds is marked at what a
market cleared — and it is a different question from the `marks` valuer, which says what a lot is
worth to the party HOLDING it and has a different answer per holder.

**Why.** XI-2. A price falls, somebody must sell into the fall, and the sale makes the fall worse.
Without a party that MUST sell, a shock is absorbed by nobody and dissipates, and every measurement
of contagion measures that dissipation. A fund is the cleanest such party: its investors can ask for
cash at any moment, it promised nothing about being able to pay, and what it must do when its buffer
runs out is sell. C2.b is the clause that makes it real — a redemption rationed by the fund's cash,
with the unfilled part dropped, deletes the entire system — so the guard against that is an audit
family pointed at it, and the test that fires it is a run in which every holder asks at once.

**Found.** Six, and five were defects the fund made visible rather than caused.

- **`material(x, 2, |x|)` is a no-op**: true for every non-zero number, because the dust it compares
  against is a fraction of the number itself. The estate's solvency test used it, and a fund's
  equity is zero by construction, so a fund was dying of dust every few periods. Two other sites
  were dropping nothing while looking like they dropped dust. There is one derivation now,
  `Valuation.equityDust`, and the audit and the failure test share it — one fact, one tolerance.
- **Solvency was asked before revaluation.** Firm D4 asks whether liabilities exceed assets AT
  MARKS, and the marks are not in anybody's book until revaluation has put them there. Asked
  before it, a party whose own liabilities are marked reads as insolvent by exactly whatever it paid
  out that week. The estate and the labour release now run after it.
- **Anchoring a phase AFTER another reversed assembly order.** Each insert landed immediately after
  the anchor, so a later module's phase ran in FRONT of an earlier module's — the opposite of what
  assembly promises. It is load-bearing wherever one phase must see what another wrote, and it
  silently stopped a dead firm's workers being released.
- **An equity walk's dust only saw the NET of an instruction.** A trade takes a book down by the
  price and up by the value; the move is the difference and the rounding is the price's. An account
  that is zero by construction is nothing BUT that residue, so a move now carries what the
  arithmetic passed THROUGH, in settlement and in revaluation both.
- **A party's income outlook counted capital coming back as income** — a matured bill, a redeemed
  share — so a household spending its own savings thought itself richer, and its expectation of its
  own income drifted up every time it drew on its savings. A claim handed back to whoever promised
  it is capital returning; a sale to somebody else is a trade and still counts.
- **Dust underflows at denormal magnitudes.** Below the smallest normal double there is no relative
  precision left, so a magnitude down there has a dust of its own that rounds to zero — and every
  identity in the wire becomes EXACT at exactly the scale where the representation is least exact.
  A cell with 1e-310 of spare cash posted an order for 1e-313 units, and settlement refused a leg
  with nothing wrong with it, because per-member × weight no longer gave the total back. `dustOf`
  is never smaller than the representation's own floor now. That is not a widened band and not a
  bound on anything the model decides: it says a quantity below the floor is not distinguishable
  from nothing, which is true.

**Changed from the plan file, and why.** Three.

1. **`pricing: 'derived'` carries a `derive` on the profile, not a kernel door.** The plan had a
   `profile.value(instrument, period, view)`. What it is given is the kernel's OWN reads and nothing
   else — no party's view, no module state — because a derived value is a fact about a book that
   anybody may compute and everybody gets the same answer from. A book whose value depends on its
   own claim throws with Fund Shares F2 rather than iterating to a number.
2. **A redemption the fund cannot fund does not FAIL.** The plan had the instruction fail and be
   recorded. A failed instruction is a claim that did not happen; C2.b wants a claim that did happen
   and has not been paid. So the module never drafts an instruction it cannot settle: it pays what
   the cash reaches and queues the rest under the holder's name, oldest first.
3. **A fund's investors are not seeded.** The plan implied an opening state; the foundation seed says
   households open with NOTHING, because everything anybody has in this world is something they were
   paid or something they decided to buy. So the fund opens with nobody in it, and the first
   subscription strikes at the unit its shares are counted in — a RESOLUTION, tested by invariance:
   double it and every share count halves and nothing else moves.

**Deferred, with where each lands.** Two, each written into the file of the item that can build it;
item 9 grew by three steps.

- **The exchange-traded fund (E1–E4, G1.a)** → **worklist 9**. E3.a asks for a participant with a
  reason and a LIMIT, not a rule tying the traded price to the NAV, and there is no party in this
  world whose business is carrying a position. Building it here would mean a market nobody trades in
  or an arbitrageur closing a gap for nothing — the free arbitrage Appendix B forbids. Item 9 brings
  desks that hold inventory, pay for the capital it uses, and run out of limit.
- **A fund failing into an estate (XI-3)** → **worklist 13h**. Its equity is zero by construction and
  nobody lends to it, so there is nothing for it to be insolvent WITH: XI-3's fund row is the levered
  case and needs a prime broker (13a) and a hedge fund (13h). The trigger is declared on the kind
  here and asked every period, so it fires the week leverage makes negative equity reachable.

**Three findings, restated in the tests they moved rather than tidied away.**

- **Savers stopped buying bills directly.** Every cell in this world is below the cushion it wants
  every week, so it has nothing it can tie up for a bill's own life — and a fund of short paper is
  exactly the thing it can hold instead and still ask back. The paper is still bought: by the fund,
  on their behalf, which is D3 in one sentence.
- **A saver's own account falls to what it is about to spend.** A deposit pays it nothing and the
  fund pays it something with same-period access, so there is nothing a deposit is for. That is
  D2.a's competition with the other side of it missing, and it becomes a real comparison the week a
  bank bids for a deposit (Banks Funding B1, worklist 11).
- **The sector's aggregate now feels a mean-preserving spread.** It could not before: a household's
  spending was linear in what it has while nothing bound it, so a linear rule summed over a spread
  gave back the same total. Having anything over what you are about to spend is a threshold, the
  spread moves cells across it, and the total moves — which is what "no decision at an average" is
  for (Households A2.g).

**Forecast, with its killer.** A redemption wave should move the price of what the fund holds, and
the move should reach every OTHER holder of that paper through their marks — so a run on a fund
should cost the banks that hold the same bills, without anything connecting them but the market. It
is not measured here. The measurement is Part XII's: run the same seed with the fund large and small
and compare what a bank's bill book is marked at through the wave. If the mark does not move, the
forced sale is meeting a book deep enough to absorb it and the channel is decorative — and the thing
to look at next is who is on the other side of the sale, not the fund.

## 9 — Equity, and dealers that carry inventory

> **CLOSED ON A WORLD THAT DID NOT ASSEMBLE, OR ON A MECHANISM FOUND UNREACHABLE.** Item 0 stepped
> the assembled world for the first time and it stopped in period 2; `docs/IMPLEMENTATION.md` Part 0
> names this module among those that compile, are cited, are marked MET, and have never produced an
> outcome. Every measurement in this entry was taken on a world that could not run to it. The
> mechanism may be right and the numbers are not evidence. Re-verified at item 0d; what the
> capabilities have actually produced is `npm run coverage:reached`.


**What.** A share is an instrument that is not a liability of anybody: a residual claim, counted in
shares, ranking below everything its issuer owes, perpetual, promising nothing dated, carrying a
vote — and a cell casts its whole weight of them. Three firms are listed, each with a market. What a
share is worth is nobody's formula: a saver reads what the issuer has actually been PAYING, which
every issuer that pays anything declares under one public name, and capitalises it at what it
requires of a claim that promises nothing — its own liquidity premium plus how wrong its own income
has recently been. Two cells with different histories therefore want different prices for the same
firm, and that disagreement is what gives the book two sides. The firm is in its own book too: it
issues when it is short and its shares are dear, buys them back when they are cheap to it, and pays
a dividend when they are dear and it has money spare — one of the three, never two, and the session
can refuse any of them. A split restates the count and moves nothing at all.

A **desk** is a named party inside a bank with its own account there. It opens holding the float of
every line it makes a market in, and it pays that bank RENT every period at that bank's own cost of
funds plus the charge on the capital its book consumes. Its two prices come out of its own state and
there is no width anywhere: what it wants to give a unit up for is its view plus what holding one
must earn it; what it will pay is its view minus what holding one will cost it; the difference is
the spread and nobody stated it. The further into its limit it already is, the more the next unit
costs it, so both sides come down together — long, it bids lower AND offers lower — which is how a
book mean-reverts with nobody telling it to and why order flow moves prices. Three limits bind its
size (this line's, the whole book's, and the money it actually has), the binding one is named on the
quote, and at the limit it stops quoting, which is what a failed market is made of.

Item 8's **exchange-traded fund** landed here, because E3.a asks for a party with a reason and a
limit rather than a rule tying two numbers together. Its shares trade, so a session prices them; its
book is read over the claims on it, so it has a net asset value at the same instant; the two are
different numbers and nothing anywhere reconciles them. Investors come and go IN KIND — a creation
takes a pro-rata slice of what the fund actually holds and gives shares against it, a redemption
gives the slice back — so nothing is sold, no market is touched, and this vehicle is not the forced
seller. A desk closes the gap when it is worth more than a period of carrying the position costs it,
and when neither direction is worth its while the gap simply stays open and the premium read says
how big it is.

**Why.** Two things could not be built without this item. Item 10 asks what equity COSTS a firm, and
a cost of capital with no traded claim behind it is a number somebody wrote down. And every market
in this world could fail only for want of orders until there was a party whose stepping back is the
failure — D4.a is what makes an auction's failure a consequence of somebody's limit rather than an
absence. XI-4's third joint runs through the rent: a desk never charged for its inventory carries a
position for free, has no reason to shed it, never skews, and order flow stops moving prices.

**Found.** Nine, and most were defects the desks and the fund made visible rather than caused.

- **A desk that guessed how long it would hold.** The first build charged the carry over the desk's
  own expected holding period, read off its own turnover. A desk that had sold a line once, in small
  size, expected to hold the next unit for fifty thousand periods, offered a bill worth one at
  nearly three, a market order took it, and the curve derived a yield that does not exist. What a
  desk KNOWS it will pay is this period's charge; the rest is the skew, which is what a position it
  has not shed does to what it will pay for the next one.
- **A desk that promised the same money in every book at once.** It sized each bid by the room left
  in that line, so a desk quoting four books committed its account four times. It ran out of cash
  and its bank followed it down. Three limits, and the binding one decides — which is also D4's
  "shrinks its size" made real rather than described.
- **A named holder with nothing to spend on.** The seed gave the listed firms a founder, to make the
  free float mean something. A named party that receives dividends and has no reason to buy anything
  is a hole in a closed circuit: the sector's money drained into it over a year and the treasury
  started missing coupons. There is no founder. The float is a read of what is ENCUMBERED, which is
  honestly zero until something pledges a share (13f) or somebody's stake is not for sale (13g).
- **A split moved holdings with no instruction, and the flows family said so.** It was right to:
  every other change of a holding is a numbered two-sided instruction. The family reads the
  published ratio now and carries the period's opening position forward restated, so the one event
  that legitimately moves no value is the one event it can account for.
- **The mark asked the market before it asked the book.** A claim on a book that also trades has two
  values, and the kernel was answering with the print — so the fund's own liability was carried at
  something other than what it owed, and its equity, which is zero by construction, was not. A
  `derived` kind is asked FIRST now, and the print is the other answer to a different question.
- **The fee was charged after the marks were taken**, so the book the shares were a claim on shrank
  after the claim had been valued. What an exchange-traded fund DOES belongs with the period's other
  payments; what it READS belongs after revaluation. Two phases.
- **Two names for one public fact.** A firm declared `dividend on <line>`, a fund declared
  `distribution on <line>`, and a saver had to know which system a claim came out of to find what it
  had been paying. One name, `payout.declared`, for every issuer that pays anything (Law 4).
- **A basket backing shares nobody took.** The launch credited the fund with the whole declared
  basket while endowing only the shares that parties who EXIST had taken, so a world without the
  desks had a fund whose net asset value was a multiple of what it owed. The launch is what somebody
  actually took.
- **A RESOLUTION that was deciding the answer.** `households.demand.steps` is declared a resolution
  — change it and the answer must not move — but the share ladder put its rungs at `k/(steps+1)` of
  the cell's own opinion. The top bid therefore sat BELOW what the cell said the claim was worth, and
  crept up as the count rose: a haircut on what a saver would pay that nobody had stated and no
  clause asked for. It was enough to keep the fund's market from ever crossing a desk's offer. Goods
  and shares build one curve now, the rungs reach the opinion, and a cell is on one side of a book —
  a bid and an ask from the same party in one session is a party trading with itself, and what
  printed out of it would be a trade that moved nothing between two balance sheets.

**Changed from the plan file, and why.** Three.

1. **A buyback is not a mechanism.** The plan had the firm cancel what it bought. It does not have
   to: a share arriving at its own issuer is extinguished by the kernel already, the same door that
   destroys a bank's money when it is paid back to that bank. So D2.a's falling count is what the
   wire does, and treasury shares (C2.d) are not representable here — which is stated rather than
   quietly true.
2. **The desk's arbitrage stands down on a carried mark.** The plan had it act on the gap between
   the print and the book. A price nobody traded at is not a level a desk could sell into, and a
   creation against one is a derivative on an uncleared price: the "gap" it closes is the carry's own
   artefact, and the first run of it emptied both desks' equity books into the fund against a level
   the market had not printed in twenty periods. `wasTraded` is the kernel's reader of that, beside
   `tradedIn`.
3. **The file layout is by DECISION, not by event.** The plan named `issuance.ts`, `buyback.ts`,
   `dividend.ts`. They are one decision with three answers — the firm looks at what it is short, what
   its shares are worth to it and what it has spare, and does one thing — so they are one function in
   `decide.ts`, and splitting them would have been three copies of the same read.

**Deferred, with where each lands.**

- **Insiders, founders, and a stake that is not for sale** (Equity C1.b, C2.e), **a majority that can
  be bought** (A5.a, A5.b), **M&A, spin-offs and take-privates** (E1–E3) → **13g**. All of them need
  a tender market, and a firm's birth is what puts a founder in a register with a reason to be there.
- **A leveraged holder, margin, shorts and stock lending** (Equity C5, C6's lending half, C7) →
  **13f**. Each is a borrow, and there is nothing that lends against a share yet.
- **The index** (Equity G1, G2) → **12**, with the other benchmarks: an index of real prices and real
  free-float weights needs a float that is not zero.
- **Hedging and the basis it leaves** (Dealer Desks E1, E2) → **13b**: a share against an index, a
  bond against a swap, and there are no derivatives yet.
- **The desk inside its bank's own ratio** (Dealer Desks F2) → **11**. It pays its bank for the
  funding and for the capital its book consumes, at that bank's own cost of funds; what the bank's
  ratio is and whether the trading book fits inside it is Banks Capital.

**A finding left standing, and where it is fixed.** A bank's estate cannot pay its depositors. Money
moves by a money leg and the waterfall builds asset legs, so a claim that IS the failed bank's own
money has no path through the distribution. It is not patched here: what a failed bank owes its
depositors is bail-in and deposit insurance, which is **item 11**, and the estate will take its
waterfall from there rather than growing a special case for one instrument.

**Forecast, with its killer.** D5 says that in a stress period inventory, spreads and capital usage
move together, and that a spread widening while inventory does not is a widening somebody imposed.
Nothing here measures it, and the numbers are all published together every period for the purpose.
The measurement is Part XII's: run the same seed with the desks' limits doubled and halved, and put
each desk's width beside its own inventory path through a period in which the float moves. If the
width does not move with the book, the skew is decorative and the thing to look at is the rent —
because the rent is the only reason a desk minds what it is holding.

## 10.1 — The kernel asks a kind what a LOT is carried at

**What.** `InstrumentKindProfile.revalue` became `carriedAt`. It was `(instrument, lot, mark) =>
delta` and every lot of a holding was then re-marked to the same number; it is now
`(instrument, lot, mark, period, calendar) => Option<perUnit>` — what THIS lot is carried at now —
and the kernel books the difference against the equity account and re-marks that lot to the answer.
Goods E2.c is unchanged and enforced in the same place: a rise is refused unless the kind says it
marks both ways.

**Why.** Inserted before item 10 because the capital programme cannot be written without it. Two
vintages of the same plant have different lives left, and a vintage bought second-hand carries what
its buyer paid rather than what the seller's book said (Capital Programme A6): the answer is per
lot, and the old door could only give one answer per holding. It is also a truer contract — a kind
now says what a lot is WORTH rather than how much to move it, which is what "one schedule, charged
against profit and against the stock" (A3) means when the two have to be the same number.

**Found.** The old shape had a rule nobody could see: a kind that wanted to be asked at all had to
have a market print, because the kernel skipped the whole holding when there was none. A thing that
wears out on a schedule of its own has no print and does not want one. `carriedAt` is handed the
mark as an `Option` and may ignore it, which is what plant does and what a good never does.

## 10.4 — What a lot is carried at, after the marks are taken

**What.** `Valuation.carryingPerUnit` asks one more question of a lot carried at the mark: has THIS
period's revaluation run? Before it, a lot held since last period carries last period's print, as
before; after it, this period's, because that is what the equity account now says. The kernel says
so once, where the marks are taken (`world.ts`, the `revaluation` phase, `valuation.remarked`).

**Why.** Carrying value is defined as *what the equity account has already recognised* and is
derived, never stored (Law 19) — but the derivation ignored the one event in the period that changes
the answer. Nothing settled after revaluation, so nothing saw it, until item 11's funding ladder made
a dealer fail: an estate opens in the resolution slot AFTER the marks are taken (XI-8, and it must,
because Firm D4 asks whether liabilities exceed assets AT MARKS), and it took the whole book over at
a carrying that was one revaluation stale. The estate's assets then exceeded its equity account for
ever by that difference — 173,162 on `estate.desk.b`, reported by the accounts family every period —
and the dead party walked off with the same amount as a residual nobody held, which Law 2 calls a
defect whether or not a check catches it.

**Found.** The gap was CONSTANT while both sides moved, which is what said it was a one-off booking
and not a revaluation that had stopped working. Transferring at the mark instead would have double
counted: the debit books the deceased out at its carrying, so with a stale carrying and an explicit
price the same re-marking is booked twice. One read, correct at both instants, fixes both ends.

## 10 — The cost of capital

**What.** XI-4's second joint, and the stock it acts on.

**Plant** is a dated VINTAGE: an instrument of its own kind, physical, owned outright, issued by
nobody, counted in its own unit, carrying the date it went into service and the date it is worn out.
It wears out on one straight line over the service it has left, and the kernel books that write-down
against the lot and against its holder's income in the same step (A3) — there is no second
accumulated total beside it and no charge struck as a share of revenue, so a firm that doubles its
plant takes twice the charge because it holds twice the units. Gross, net, accumulated and the
period's charge are reads over the vintages and the journal. A vintage that reaches its retirement
date leaves the register by a destroy leg, at a carrying value the schedule has already taken to
nothing.

**Capital is specific, and specific in kind** (A4). A registry of capital kinds states, per kind, the
unit it is counted in, the GOOD it is built from, how long it works and how long it takes to build.
A4.b is why that table has a life in it at all — the presence of a life is what makes a good a
capital good — and it means no flag on any good says so. Capacity is the SCARCEST of the kinds a
recipe names, in units the line can start per period; a line whose recipe names no plant is not
limited by plant, which is a different answer from being limited by a large number.

**A purchase becomes plant because of who bought it** (A4.c), and that is read off the wire: what a
firm BOUGHT of a capital good is commissioned into plant when the build lag is up, and what it MADE
of one is stock it sells. So a workshop holding its own output is not investing in itself and
nothing had to ask anybody's industry to know it. Commissioning is one instruction — the machines
are destroyed and the plant created at the same cost, in the same pass — so plant is never born from
nothing (Firm Birth A2.a) and the spend is irreversible (C4): the money went to the producer.

**The decision** (B) is the firm's, taken from its own view. Its cost of capital is what its debt
costs AT THE MARGIN, NOW — the quote a bank has given it this period, which XI-4 names the average
coupon as the way this joint is deleted — weighted with what its equity costs, read off its own
share price against its own outlook of its own earnings, by its own balance sheet. Its hurdle and
its horizon are its management's own, dispersed. What it commits to is what it would run at LESS the
width of its own recent surprises, which is B4's option to wait with no coefficient anywhere: the
margin IS the confidence read, applied to the quantity where that number's units live. The gap is
that rate against what its plant will still let it run at NEXT period, so a firm running full is
short of at least what is about to wear out and one running empty has no project at all (B3) — and
nothing computed a utilisation ratio to get there. It invests when a unit of capacity is worth more
to it than the plant that makes one is asking, which is exactly B1, and it posts what it could
actually pay for (B2); the rest is a PROGRAMME it publishes, which is what a bank lends into and
what a share issue is raised into (Firm E4.a).

**The world grew a capital-goods line**: a fourth good with a life, three workshops that build it,
and an occupation of their own. Investment is therefore somebody's revenue (C1), employs the people
who build it (E2), and is demand now and capacity later (C1.a). Every firm opens with plant in three
vintages of different ages, so replacement comes round a third at a time rather than all at once.

**Goods B5 gained its third term**: unit cost is inputs plus wages plus a capital charge, and the
charge is the same wear the stock is written down by. It reaches the decision, not just the report:
what an hour is worth to a firm, and what it will pay for a tonne of what it is made from, are both
net of what the plant that hour runs on wears out by.

**Corporate Credit E5.** A bank's reservation for a named issuer's paper is now its own blended cost
of funds and the capital the position consumes, computed where a bank's economics live and published
under its own name. Two banks with different capital and different required returns on it do not
require the same thing of the same bond, and a fund under a mandate requires something different
again — which is what makes a book have two sides. **`sovereign.holders.requiredYield` is deleted**:
one placeholder stands in this world now, and it names item 11.

**Joint one grew its missing half.** A bank's cost of funds was what it paid on what it owed, which
is nothing until deposits are priced. XI-4 says the blend is "deposits, wholesale borrowing AND
CAPITAL", so it is now what it actually paid plus what its own capital costs it, over the whole of
what funds its book. That number is not zero, it differs between the two banks, and it is what every
loan quote and every bond schedule in this world is now built from.

**Why.** Every cleared price in this model is inert until one of XI-4's three joints is closed. Item
9 closed the third. This closes the second and finishes the first: a bank's own funding cost reaches
a borrower, the borrower's cost of capital reaches a project, the project reaches a purchase from a
named producer, and the plant it buys reaches what the line can make — after the build lag, and never
directly.

**Found.** Seven, and four of them are findings about the world rather than about the code.

- **A firm that only sees its own fills can never see demand above its own ceiling.** The first build
  measured the gap as what it expects to sell against what it can make. A capacity-bound firm sells
  exactly what it makes, so its expectation converges on its ceiling, the gap is zero, and B3's
  "a firm running full has an obvious reason to expand" is unreachable — a fixed point, not a
  tuning problem. The gap is measured against what its plant will still let it run at NEXT period
  instead: a firm at its ceiling is short of what is about to wear out, and replacement is
  investment. Nothing had to be told what demand it could not see.
- **A bidder that sizes its order at its own reservation buys MORE when capital gets dearer.** With
  the funding constraint applied at the price it was bidding, a firm whose cost of capital rose bid
  lower and could therefore "afford" more units of a thing it valued less. How many it can pay for
  is a question about the price it expects to PAY; the most it would pay is a different question and
  a different number, and the two had been the same one.
- **A treasury can issue a coupon its holders would have to pay.** Once the sovereign secondary
  market actually cleared, a short line printing above par gave the curve a wildly negative
  annualised yield, and the debut stamped that as a coupon. A coupon is a payment the ISSUER
  promises (Bond N5, N6), so an issuer facing a curve below zero brings a line that promises
  principal and nothing else; the market still pays whatever it pays, and a zero-coupon line above
  par IS a negative yield. The profile also stopped emitting a coupon action for nothing at all.
- **The sovereign-bank doom loop, measured and then left out of scope.** E5.b's expected loss was
  wired to the loan model — how often this bank has seen this name fail, times a loss given default
  of all of it. One missed treasury payment then moved every bank's required yield by a
  twenty-sixth, repriced the whole curve, met the money fund's forced sale (XI-2) at the desks' bids
  and took a bank's capital with it. That is Corporate Credit G8 working exactly as written. It is
  not in the build, because both halves of it are wrong for an issuer whose paper is marked to
  market: a treasury that missed a payment has not stopped being able to create the money it
  promised (Sovereign G1), and a recovery stated at zero is the fixed recovery Appendix B forbids.
  The assessment E5.b needs is somebody's OPINION (Corporate Credit A4), which is the ratings system
  and the second opinion — **item 12**. The world cannot resolve a failed bank either, which is
  item 11; that is why the finding is recorded rather than shipped.
- **The real chain still runs down, and it did before this item.** The foundation world's output
  falls to nothing by about period thirty, unchanged by anything here — the same path, one period
  later. It is a finding about the world with the mechanisms it has, and Law 11 says the answer is
  the missing mechanism rather than a number. Investment is not visible in that world for the same
  reason nothing else is: nobody is at their ceiling. The item's tests turn one technology number up
  — a tonne of grain a week takes four machines instead of one — and the whole chain runs.
- **A dust shortfall in the flows family, reachable only in a contrived world.** In the tight world
  above, a household cell's share holding moves by about forty-five ulps more than its legs account
  for, around period sixteen. The tolerance is not widened: the derivation counts the lots at each
  END of the period and the legs between them, and a holding that was built up and sold down inside
  one period passed through more lots than either end has. It is named here because Law 7 says a
  check that only passes with a band is reporting a defect, and the defect is in the derivation.
  The foundation world runs two hundred periods green.
- **A surface that sifts a feed goes quiet as the world grows, and does not say so.** The inspector
  showed the sovereign's published programme by taking the last forty events and looking through
  them for it. A quote to every borrower from every bank made a period four hundred and seventy
  events long, the programme fell off the back of the feed, and the page simply stopped showing a
  section — no error, no missing marker, nothing. The depth was not the defect: reading a thing said
  once a period out of a feed of everything was. A viewer now names the KINDS it follows and gets
  the recent events of each (`Journal.recentOfKind`, same visibility rule), so what it reaches does
  not depend on how much the rest of the world had to say that week.

**Changed from the plan file, and why.** Four.

1. **The decision lives with the firm, not with the capital programme.** The plan put `decide.ts`
   in the module. What to invest is made of the same things every other firm decision is made of —
   its own outlook, its own cash, the prices it faces — and it shares its orders with them, so it is
   in `firms/invest.ts` and the module owns the STOCK: the kinds, the vintages, the schedule, the
   commissioning and the identity.
2. **A vintage is an instrument, not a lot.** A6 wants its own cost, its own service date, its own
   life and its own kind on it; the service date has to survive a sale, or a machine sold out of an
   estate would be brand new in its buyer's hands. Vintages are shared by everybody who commissioned
   the same week, so their number is bounded by the life of the kind rather than by the number of
   purchases.
3. **There is no opening-stock placeholder to delete.** F1.b's stand-in is an opening stock of work
   in progress handed to a firm TOUCHING A LINE FOR THE FIRST TIME. The work in progress this seed
   states is Seed D1's — a stock consistent with the flows that will act on it, for firms that have
   always been in their line — so deleting it would have removed a different thing for the wrong
   reason. F1 is PARTIAL and named to 13g.
4. **`labour.capacity.hoursPerFirm` was already gone**, deleted at item 4a; the plan's deletion list
   was written before it.

**Deleted.** One placeholder (`sovereign.holders.requiredYield`, item 3's), replaced by what a
holder actually requires. One shape added: `seed.openingPrice.machine`, the level the fourth good's
market opens at, which is the same kind of claim as the other three and dies the same way (it never
does, and the record at item 8 says why). Placeholders: two → **one**.

**Deferred, with where each lands.**

- **E5.b's expected loss for a holder** (Corporate Credit E5.b) → **12**, with ratings and the second
  opinion. A bank's model of a borrower it LENDS to is unchanged and still prices its loan book.
- **Entering a line** (Capital Programme F1) → **13g**. The project is the same arithmetic in the
  same market; what is missing is anything that can change a firm's line-up.
- **A completed second-hand sale out of an estate** (D3) → measured at **16**. The estate offers the
  plant into the vintage's own market and firms that can use it bid there, at what the service left
  in it is worth to them; whether a bidder has a gap at the moment an estate is selling is an
  outcome.
- **A second capital kind** → **13d** (dwellings) and **13c** (storage). The scarcest-of-several
  mechanism is built and tested; this world declares one kind so far and says so.
- **E4's size** (Capital Programme E4) → **16**. The direction is asserted as a mechanism test.

**Forecast, with its killer.** B1.c says a change in a market price changes real investment. The test
here shows the direction over sixteen periods with one number moved. The measurement that would kill
it is Part XII's: move the banks' required return on capital in small steps and plot total plant
commissioned against it. If investment is flat in the shock until it falls off a cliff, then the only
live channel is the gate — a firm invests or it does not — and the continuous margin B1 describes is
missing, which would mean the quantity a firm wants is not a function of the price it faces. The
thing to look at then is the gap: it is currently the capacity it is short of, and a firm that wanted
LESS capacity because capital got dearer would need a reason to run at a lower rate, which is a
different mechanism from this one.

## 10.2 — Every unit has a smallest piece

**What.** Money is discrete, and so is everything else that is counted. Each unit declares its own
grid as an exponent (`UnitDecl.tickExponent`, the tick is `2^-e`), the choices are made together in
`registry/grid.ts`, and one parameter (`resolution.tickShift`) moves them all so the choice can be
tested. Every quantity in the state is a whole number of its unit's tick: money and everything
denominated in it (par, loans, the money-market rows), goods, plant, hours and shares.

**The wire enforces it and never rounds for anybody.** A leg carrying a quantity that does not exist
throws at the site (`Impossible [Law 8]`), including the per-member side of a cell leg — every
member of a cell is a real holder with a real account (XI-15). The kernel rounding somebody's
payment would be the kernel deciding what they paid, so whoever builds the leg decides, with four
named questions: what somebody CAN pay or deliver (down), what a value COMES TO (nearest), what a
requirement NEEDS (up), and what a cell's own share is (per member, then times the weight).

**Splitting is the mechanism this makes real.** Ten pieces three ways is four, three and three:
`splitOnTick` gives the odd piece to the largest remainder, ties to the earlier claimant, and the
parts sum to exactly the whole. Law 2's "a residual with no holder is a defect" stops being
something the audit reports and becomes something the arithmetic cannot do. Where a population is on
one side of a trade, the least the two can exchange is the tick times the least common multiple of
their weights, because a cell of five hundred deals in five hundred pieces at a time.

**Why powers of two.** A decimal grid is not representable in binary floating point: sums of "exact"
hundredths drift off their own grid and the dust returns with an extra step. On a binary grid every
sum and difference of whole ticks is exact, so a balance moved a million times IS the balance. That
is what buys the change: comparisons that used to need a derived tolerance now need none.

**Why.** Law 1 asks for the real mechanism, and continuous money is not one — real currency has a
smallest unit, and when ten cents are shared three ways somebody gets four and somebody three. What
continuous money produced instead was a residue of floating-point dust that every check had to be
told to forgive, and a residual belonging to nobody. The record already carried one of those as an
unfixed defect (item 10's dust shortfall in the flows family); it is gone, and so is the class.

**Found.** Four.

- **The grid is set by the SMALLEST holder, not the largest.** The first choice put goods at a
  thousandth of a tonne — a kilo, which is what a lorry is loaded to. A household member buys a kilo
  or two of bread a week, so that grid rounded a person's entire weekly shopping up or down, and the
  sector's demand with it: total production over twenty-six periods came out a tenth lower on a
  coarse grid than a fine one. The grids are now set by what the smallest holder deals in, and the
  invariance test is what says so.
- **The world's path is invariant to the grid, and then it is not.** Every structural invariant —
  money conserved, holdings summing to what is issued, no residual — holds EXACTLY at grids four
  thousand times coarser and four thousand times finer. The aggregates converge as the grid refines:
  two grids eight halvings apart at the fine end agree to a hundredth over eight periods. Beyond
  about the twelfth period they separate, because a firm on the edge of starting a batch starts it in
  one run and not in the other. That is Law 2's own warning about deciding at thresholds, arriving
  from the measurement side, and it is a fact about the world rather than about the grid.
- **A payment of nothing is not a payment.** Quantisation makes a whole class of tiny flows vanish
  honestly: a fee below one piece is not charged, a dividend whose per-member share is under a piece
  is not paid, a trade whose cash comes to less than half a piece does not fill. Each of those was
  previously a leg for an amount that was mostly arithmetic noise. Three audit families had to learn
  the same thing — what a payment settles is the value ROUNDED to real money — and their tolerance
  is now the grid rather than the float's dust, which is a smaller, realer number.
- **An estate is not a depositor.** Deposit classes (worklist 11) had put an estate with the firms,
  which gave a party being wound up an interest income and the treasury a tax claim to rank among
  its creditors — neither of which this world has a mechanism for, and both of which the estate's own
  flows family caught immediately. An estate realises a business rather than running one; its balance
  is proceeds waiting to be paid out.

**Changed from the plan.** The item was inserted mid-flight, at the owner's ask, while item 11 was
being built — because the money market writes a payment every period for every bank, every depositor
and every row, and each of them would have had to be re-derived on the grid afterwards. Two decisions
were the owner's: the tick is a parameter and is tested by invariance rather than asserted, and every
unit gets one rather than currency alone.

**Deleted.** `UnitDecl.countable` — a countable unit is one whose tick is one, so the flag was a
special case of the grid and is gone with it.

**Forecast, with its killer.** The claim is that the grid is a resolution: change it and the world
does not change. The test measures convergence over eight periods, and the record above says where it
stops holding. What would kill the claim outright is a measurement at 16: run the same seed at every
shift and plot the distribution of an aggregate over many seeds. If the spread across grids is of the
same order as the spread across seeds, the grid is a resolution and this is settled; if a grid change
moves the aggregate further than a seed change does, then it is load-bearing and the world's
mechanisms are more sensitive to lumpiness than anything here has admitted.

## 10.3 — A quantity is a whole number of indivisible pieces

**What.** 10.2 gave every unit a smallest piece and made it a power of two — 2^-20 of a PHX, about a
ten-millionth. This item replaces it: **a quantity is a COUNT OF PIECES and the count is an
integer.** Not a fraction of a named unit — the piece IS the number. Money is counted in cents, a
cargo in grams, a workforce in hours, a register of members in whole shares. `UnitDecl.perUnit` says
how many pieces one NAMED unit is divided into (`registry/grid.ts`: money 100, a tonne 1,000,000, a
whole thing 1, an hour 1), and `resolution.pieceShift` multiplies them all so the choice can be
tested. The world is redenominated ×1000 so that a cent is a sensible piece of it: a weekly wage is
around 940 PHX where it was 0.65.

**Why.** A decimal cent and a floating-point fraction cannot both be exact — `0.07 + 0.01` is not
`0.08` in binary — so 10.2 bought exactness and paid for it with a granularity nothing real has.
Money exists up to the cent; nothing below one can be lent, paid or refused. The proof that the
difference matters is in item 11's own findings: the money market recorded a bank as refused funding
for a shortfall of 1.8e-7, that refusal is a public observable, and every uninsured depositor left
it. Integers add, subtract and compare exactly up to 2^53, so the exactness survives the realism: a
balance moved a million times is exactly the balance, and the decimal arithmetic everybody actually
does is exact because it is integer arithmetic on cents, which is what it always was.

**The boundary is the registry and it is the only one.** `pieces(unit, named)` turns a person's
number into the count the state holds and `named(unit, pieces)` turns it back for a reader;
`priceOf(ccy, unit, perNamedUnit)` does the same for a price, which is a ratio carrying both
subdivisions. Nothing between those two ever divides by a subdivision, because everything between
them is already a count.

**A parameter that is an AMOUNT says so.** `ParamDecl.denominated` with `params.amount(id, unit)`:
the declaration says what a person means — thirty thousand PHX a period, seven thousand hours, four
hundred thousand units of a line — and the register hands back the count of pieces. This was the
design question 10.2 left open and it is answered the expensive way rather than the cheap one,
because it is the ONLY way a declared amount moves with `pieceShift` instead of being restated
against it at every site — which is what makes the resolution an invariance rather than a rescaling
of half the world. WHICH unit is the reader's to name, because one policy is a number for whatever
money, time or paper the party reading it deals in; a `get` on such a parameter throws rather than
answering in the wrong number. The register is therefore built twice out of one list of declarations:
once with no units, which can answer only `resolution.pieceShift`, and once against the registry that
number built. It also deleted the three `no-magic-numbers` lint errors, which were the symptom that
pointed at it.

**Found.** Four, and every one of them was a number in the wrong unit hiding behind a plausible
answer.

- **The desks were a hundred times too small.** `mechanisms/dealers/data.ts` states named PHX
  (`cash: 400_000`, `limitAggregate: 1_200_000`) and 10.2 had converted them as though they were
  already pieces, so desk A opened with 4,000 PHX against its bank's 400,000 — a market maker with
  one per cent of the balance sheet its own data gives it. Fixed at the boundary, which restores
  exactly the proportion the world had before 10.2. Nothing measured about dealing, spreads or
  inventory since 10.2 was measured on the desks this world declares.
- **A desk was a footloose wholesale depositor.** At the right size, desk A WAS bank A's corporate
  funding — and the funding market moved it to bank B for two basis points, killing the bank. Dealer
  Desks A1 says a desk is its bank's trading arm and its account is there because that is what it
  is; an account at a rival would make it a different firm. So a party kind now declares whether it
  `choosesBank` (Law 15: the profile answers and nothing branches on a kind), and a bank, a treasury,
  an estate and a desk do not. It also removed a special case: the funding market no longer names
  banks to skip them.
- **The named red is gone.** Every foundation world now runs a full year with ZERO violations in
  every family. Until this change each of them reported one every auction cycle — a bank overdrawn at
  the central bank with nobody able to lend it the difference overnight (Money B3.b) — and
  `test/expected.ts` existed to name and hide exactly that. The corridor (item 11) is what a bank
  goes to instead, so the exemption is deleted rather than widened: `unexpected()` is now every
  violation the audit reported, and nothing anywhere is forgiven.
- **A fund can be gated for ever on a fraction of a share.** A household asks to redeem
  `short / perShare` shares per member, which is not a whole number; the fund pays the whole shares
  it can and the remainder never leaves its book, so a fund that owes nobody anything reports a gate
  every period. Named at the site and NOT fixed here: rounding it at the cause takes the money fund's
  forced sale past what this world can absorb — the bank funded against the bills it dumps fails —
  and that is item 11's change, not this one's.

**What is exact, and what is not.** The resolution test says both, and the second half is new. Every
STRUCTURAL invariant holds EXACTLY at the cent, a tenth of a cent and a hundredth of one — money
conserved, holdings summing to what is issued, no residual anywhere, nothing "within" anything. The
PATH is not, and honestly so: what a payment or a batch comes to is rounded to a whole piece and this
world's decisions are thresholds, so a firm on the edge of starting a batch starts it in one run and
not the other. 10.2's test claimed real output was invariant to within one coarse piece per batch;
that reasoning was wrong, because the rounding feeds back into the decisions that produce the next
one. What is asserted instead is that A FACTOR OF TEN FINER IS A FACTOR OF TEN CLOSER, in what the
world made and in the money it holds — which says the difference is the rounding and nothing else,
without anybody choosing a band, and which a number that stopped scaling with the grid fails rather
than passes quietly.

**The surface reads back in named units.** `Snapshot.subdivisions` hands the reader how many pieces
one named unit is and the app divides by it where it prints. Nothing is converted on the way out of
the engine: a surface that quietly rewrote the state's numbers would be a surface with arithmetic of
its own in it (Observer E3).

**Deleted.** `test/expected.ts`'s expected-red exemption and the `overdrafts` helper that measured
it; the stale `PIECE` and `GOODS_PIECE` constants, which were still `2^-20` and only survived because
the tests using them happened to pass; the three magic-number literals in the treasury and money
market; the special case that made the funding market skip banks by name.

**Not closed with it.** Four tests are red and all four end the same way: a bank fails and this world
has nowhere to put it. That is item 11's stated blocker (Banks Capital D), the tests are folded into
its list, and this item's own conversion is what made a bank failure reachable — the desks being the
size their data states is exactly what removed the padding. Two other findings are recorded there and
not acted on: a yield solve that throws past its own bracket at period 61 of a long run (it will stop
item 16 dead), and the treasury announcing an auction size that is not a whole number of pieces of
par — it never reaches the wire off-grid, but an issuer offering 53,308,760.87 units of par is
announcing a quantity that does not exist.

**Forecast, with its killer.** The claim is that how fine the pieces are is a RESOLUTION: declare the
same world in tenths of a cent and its path does not turn on it. The test measures convergence at
three subdivisions over eight periods and the record above says where exactness stops. What would
kill the claim outright is item 16's measurement: run many seeds at every subdivision and compare the
spread of an aggregate ACROSS SUBDIVISIONS with its spread ACROSS SEEDS. If they are the same order,
the piece is a resolution and this is settled; if changing the piece moves an aggregate further than
changing the seed does, the piece is load-bearing and this world is more sensitive to lumpiness than
anything here has admitted.

## 11.1 — A venue asks parties for schedules the way a market does

**What.** `VenueParticipantDecl` and `SystemModule.venueParticipants` beside the market's
`ParticipantDecl`, and `MechanismContext.gather(venue)`: the module that OPENED a venue asks for the
schedules its parties want to post there, and the kernel evaluates every declared one with that
party's own `ParticipantView` and posts it. Only the opener may gather — a module filling a book it
does not strike is a second writer of that book (Law 4) — and a venue is gathered once a period,
because the book is emptied at the top of one and a second ask would post every schedule twice.

**Why.** A market's orders have always come from the participants, each with its own view (Observer
A4). A venue's did not: the module that clears it built every party's schedule inside its own phase,
out of a context that can see every party's private state. So the money market decides what a bank
lends and borrows and at what rate, and the labour market decides what a firm hires — one module
deciding for parties it does not own, with a view no participant may have. Clearing is the venue's;
the schedule is the party's, and this is the door that says so.

**Nothing moved.** The money market and the labour market still build their schedules in their own
phases and neither calls `gather` yet; every existing world is the same world. The door is what
11.2 needs before a bank's treasury can post its own session schedule, and it is inserted rather
than folded in because the kernel changes only by an inserted item.

## 11 — The money market, the corridor, bank funding and bank capital

> **CLOSED ON A WORLD THAT DID NOT ASSEMBLE, OR ON A MECHANISM FOUND UNREACHABLE.** Item 0 stepped
> the assembled world for the first time and it stopped in period 2; `docs/IMPLEMENTATION.md` Part 0
> names this module among those that compile, are cited, are marked MET, and have never produced an
> outcome. Every measurement in this entry was taken on a world that could not run to it. The
> mechanism may be right and the numbers are not evidence. Re-verified at item 0d; what the
> capabilities have actually produced is `npm run coverage:reached`.


**What.** A bank's reserve position is now the residue of everybody else's payments, and there is a
market where that position meets somebody else's. The session runs after the flows because the need
is not knowable before them (A3.a); every bank posts a schedule out of its own position and its own
cost of funds, and who ends up lending is the fill (B1). Unsecured lending prices the NAME from what
that lender published about it; secured lending prices the paper, at that lender's own required
yield, with a real lien struck in the same instruction as the row. The central bank takes both sides
at two administered levels — a floor where parking cash destroys it, and a window that lends against
unencumbered eligible paper at the ceiling — so the policy rate reaches the economy through this
market and by no other route (E1, B3.a).

Around it: deposit classes with different stickiness, each bank setting its own board rate and
answering its rivals'; a blended cost of funds that is a read of what it actually paid; a capital
requirement with two rules and a read of which one binds; a subordinated layer it can raise into a
market that can refuse; a funding ladder with every rung a real action; and — when all of them run
out — a resolution: a valuation at marks, a hole, a hierarchy that absorbs it in rank order, an
acquirer that takes the book over the wire, a guarantee funded by the banks that have it, and a
public purse behind that.

**Why it is one item.** Every one of those is the other's precondition. A bank cannot fail for
liquidity until there is a market that can refuse it; a market cannot refuse it until the window is
priced and collateralised; a hole cannot be borne in order until there is a layer to bear it; and
none of it can be looked at until a failed bank has somewhere to go, because a bank that ceases with
its depositors still holding its money leaves a world that cannot settle a payment.

**What was built underneath it.** Two items were inserted and closed on the way, both because this
one reached them:

- **10.3**, a quantity is a whole number of indivisible pieces. A refusal made of 1.8e-7 started a
  real run, and that is what came out of it.
- **10.4**, what a lot is carried at after the marks are taken. An estate opened after revaluation
  took a dead dealer's whole book at a stale carrying and the difference sat on it for ever.

**Findings, in the order they arrived.**

1. **The deposit market was a metronome.** A bank valued money at the window whenever a session left
   it short by any amount, and repriced its ENTIRE deposit base at that — so one refusal moved every
   depositor in the world. There is one rate per liability (B2.b), so paying up for the next dollar
   means paying up on every dollar it already has: it does that when what it could not raise is
   bigger than what it already owes, and not otherwise. The other half was that a board never
   answered a board; now it holds a class at the rival's rate less what moving costs that class, up
   to what money is worth to it and no further (B1.a's own bound), and lets the class go past that.
2. **A limit that counts one kind of claim is not a limit.** A bank's cap on one name counted only
   LOANS, so the first capital raise took the whole of the other bank's spare cash. It counts
   everything that name owes it now (F3).
3. **A secured lender was being bailed in for collateral it was holding.** The pari passu pool took
   every claim on the failed bank; a repo lender holds the paper and the acquirer takes the book with
   the liens on it, so it is in the pool only for what its security does not reach.
4. **In this world a bank always fails for liquidity, never for solvency**, and when it does its hole
   is NEGATIVE — more assets than liabilities, nobody loses anything. Every seed tried reaches that
   and none reaches the other, which is why the insolvent path had to be reached by a penalty this
   item's own test module pays. It also means D4 and D5 are exercised for real but for small
   amounts: the uninsured pool is far bigger than the hole.
5. **Two banks both short of capital do not fund each other.** When the RULE is what puts them both
   under, neither has room to take the other's paper and every raise finds no bid. That is a real
   thing about a systemic squeeze and it is why the contagion test moves one bank's own buffer.
6. **The weighted requirement asks a bank here for nothing.** Its book is reserves and sovereign
   paper — claims on parties that cannot fail — so it weighs zero and the only rule with anything to
   say is the leverage backstop. B1.b's case, made by a world rather than argued.
7. **The world is knife-edge, and XI-15 is what caught it.** The last placeholder could not be
   deleted: both candidate replacements for the bank's paper target tripped the grain invariance,
   because any change to what banks want of sovereign paper re-rolls which whole cells cross between
   banks in which week, and a crossing carries a cell's whole account. The finding is recorded and
   the placeholder stands with it. What has to exist first is a deposit market where a class does not
   move as one block; findings 1's two rungs are the first half of that.
8. **XI-2's third door has no client here.** Wiring a bank's published per-name line to the desks
   made every desk dump its whole inventory every period. The finding is the party, not the number: a
   desk is its own bank's trading arm, so its funding is internal. The door is built for the party
   this world does have — a bank refused by the session sells its own paper — and the client version
   is placed at 13h.

**What it leaves.** A year runs green through a squeeze: a rival takes a bank's funding, the reserves
leave with the deposits, the session and then the window refuse it, it fails for liquidity with more
assets than liabilities, the other bank takes it over, and the world goes on — every family at zero
in every one of the fifty-two periods, and the same world twice from the same seed.

**Two things are placed rather than done**, in the items that give them a party: raising EQUITY and
restricting distributions (13g, which is where somebody comes to own a bank), and cutting a
leveraged client's line (13h, which is where the client exists). Neither is missing; both are in
those plans with the reason.


## 11.2 — one bank, in progress

**Not closed.** This is the fold, not the finish. What follows is what has moved, what it found, and
what is red because of it. The item's own plan file lists the steps that remain.

**There is no desk.** `mechanisms/dealers/` is deleted, both desk parties with it, and so is the rent
a desk paid the bank it lived inside — a transfer price between two parties that were economically
one. A bank quotes out of its own inventory, funded at its own blended cost of funds and against its
own capital, and what a line of business is, is data about the bank. Dealer Desks A1's "its own
balance sheet INSIDE a bank's" is a sub-ledger now, and F2's "no desk exempt from its own bank's
capital and funding" is true by construction rather than by a check.

**One face.** Exactly one module gives `BANK` a participant, in markets (`banks`) and in venues
(`banks`, through 11.1's door). `sovereign-curve` no longer decides anything for anybody: its bank
participant, its buffer target, its two-step demand schedule and its surplus premium are gone, and it
is a curve family and a check that its points are prints. `sovereign-auction` is deleted entirely —
the primary bid is the dealing line's, at its own price, for the obligation the ISSUER announces with
the line. `money-market/funding.ts` is deleted: a forced sale is the dealing line quoting with
urgency, not a second participant under the same bank's name. The money market keeps the venues, the
corridor, the window's own offer, `strike`, the rows, the insurer and the resolution, and reads no
bank's preference anywhere.

**The last placeholder is gone.** `bank.liquidityBuffer.perDeposit` stood in for a decision nobody
had built; the treasury now makes it — a coverage rule somebody wrote plus the bank's own cushion
over it, against the money its own books say could leave (Banks Funding C2). The engine declares 144
numbers and none of them is a placeholder.

**Findings, and they are one cause.** With the desks gone the banks are the only market makers in
sovereign paper, and that puts a number nobody had looked at on the front of every price in this
world: **a bank here is funded roughly three quarters by its own capital**, so its blended cost of
funds — a read across its whole mix including capital (§24 B2, XI-4 joint one) — is near a tenth. It
cannot hold the paper the liquidity rule makes it hold, it cannot win the auction it is obliged to
bid at, the auction and the secondary market print two prices for one line, and in the `tsy-g` world
the treasury misses eight of its own coupons. Under that: **every reserve the seed hands a bank is
central-bank money issued against no asset** (`endowMoney` issues the money of the party's own bank,
and a bank's bank is the central bank), so the central bank pays the floor rate for ever on a balance
it never bought anything with and books a loss instead of a remittance in every run; and both banks
are so long of cash that **the interbank session never opens** — no print, no refusal, no bank in
trouble, so the run, the contagion and the corridor's pass-through are unreachable. All of it is
**11.3**, inserted ahead of the depositors' item for that reason.

**Twelve checks are red and every one of them ends at that cause.** They are listed in 11.3's plan
with what each asks. None has been adjusted to pass and none has been deleted.

**A number that was mine and was wrong.** The fold briefly raised the banks' opening reserves from
70M to 135M "to absorb the desks' cash". A desk held a DEPOSIT at its own bank — that bank's
liability — so when the desk stops existing the deposit stops existing with it and the bank is left
owing less, not holding more. The extra 65M was central-bank money issued against nothing, and it
doubled the hole above. It is deleted; the seed states what it stated before.

**Numbers that were mine and were tuned.** The dealing line's limits were first stated as amounts of
money read off the book value a probe printed — which is a result wearing a preference's name (Law
2), and which the repository's own lint catches the moment such a literal sits outside a `data.ts`.
They are gone. What a bank will have standing behind its dealing book is a share of ITS OWN CAPITAL,
and how much of that book may be in one line is a share of the book, so neither has to be restated
when the world changes size. The liquidity coverage rule is 1 — the real-world primitive, imported,
which Law 2 allows where a real-world equilibrium would not be — and each bank's cushion over it is
stated in the same proportion as its capital buffer, from its own character rather than from an
outcome. `lines.ts` is deleted and every one of these lives in `banks/data.ts`, ONE ROW PER BANK:
what it is like as a lender, a funder, a treasury and a dealer are four sides of one disposition, and
holding them in four tables was what let one bank hold four opinions of itself.

**Deleted.** `mechanisms/dealers/`, `mechanisms/sovereign-auction/`, `money-market/funding.ts`,
`banks/lines.ts`; `sovereign-curve`'s participant, `bufferTarget`, `demandSteps`, `priceAtYield`,
`requiredYieldOf`, `sovereignValue`, `tenorOf`, `P_BUFFER` and `P_SURPLUS_PREMIUM`;
`money-market`'s `DepositBook`, `setRates`, `defended`, `rateFor`, `rememberReserves`, `bufferOf`,
`positionOf`, `lenderReservation`, `bankOrders`, `worthOfMoney`, `pledgeable`, `nameCost`, `FUNDERS`
and the `desk` entry in the wholesale deposit class. Every deletion names the read that replaced it.


## 11.2 — a third bank, and what it exposed

**Why a third.** With two banks every depositor that answers a rate is the whole of one side of the
deposit market, every interbank session is one name facing one name, a bank that must raise capital
has exactly one possible buyer, and a bank in trouble has exactly one possible acquirer. None of
those is a mechanism; they are all "there are two of them". `bank.c` is now in the foundation world:
the careful one, and the smallest — it remembers a borrower and its own bad weeks for a whole year,
asks the least on its capital, runs the widest cushion over both requirements, keeps the widest
margin on the money it takes in, and makes a market in the paper it holds for liquidity and in the
fund its own depositors buy but NOT in shares, which is what makes `makes` data about a bank rather
than a list they all share.

Nothing was created to make room for it: the same reserves over three banks instead of two, the same
sovereign debt outstanding split three ways, four firms banking at each, and the households' cells
per (cohort, bank) unchanged — so this world has half again as many households in it, because a key
is a cohort AT A BANK. The dealership share the issuer announces is a third and a little over rather
than a half, for the same reason it was a half: its dealers between them cover what it brings.

**A defect it found on the first run.** A loan was credited to the borrower's account AT THE LENDER.
With two banks the keenest quote was always the borrower's own bank and it never showed; with three
it does, and the borrower ends up holding money issued by a bank it never opened an account at,
invisible to every read that asks what it has. A drawing now lands in the borrower's own account
wherever that is: when lender and bank are the same the money leg has one issuer on both ends and no
reserve leaves it (Banks Lending B1.a, the whole of endogenous money); when they are not, the
lending bank has paid somebody who banks elsewhere and the reserves move at the drawing rather than
after it (B1.b).

**Twenty-six checks are red.** Sixteen of them are the one cause 11.3 names, and the third bank
sharpens it rather than changing it: **a bank funded three quarters by its own capital cannot be
driven insolvent, because its equity is larger than everything it can convert to cash.** The
resolution suite's ten reds are exactly that — a penalty of a third of `bank.a`'s equity, paid every
week, settles twice and then fails, and the bank is left with a hole of MINUS forty-two million: the
window refuses it for collateral, never for capital, so the bail-in, the acquirer, the guarantee and
the conservation all have nothing to work on. With a normal balance sheet the first instalment would
have buried it. The other six of the sixteen are the ones already recorded: the treasury cannot place
its debt, the central bank books a loss instead of a remittance, the curve describes neither of the
two prices this world has for one line, and the interbank session never opens, so the run and the
corridor's pass-through are unreachable.

**Ten are not that cause and are carried, each named here rather than adjusted:** `capital.test.ts`
(the XI-4 chain's middle link, and a firm whose cost of equity reads negative off its own share
price), `estate.test.ts` (a year with a death in it, and what the observer shows of the estate),
`credit-events.test.ts` (a state that spends past what it can fund), `tick.test.ts` (the piece-size
convergence, which is a RESOLUTION and now measured on a bigger world), `labour.test.ts` (a venue
that prints the higher of two bids and fills nothing — the employer cannot pay what the print asks,
and whether that is the venue's rationing or the employer's cash is not yet known), and one more of
`omo.test.ts` (the base grows when it buys). None has been adjusted to pass and none deleted.


## 11.2 — the rest of the item: a contract, capital by intent, and one surface

**A self-cross is a contract now, and it is a deletion.** `pairFills` used to step past a fill
between one party's own two orders, because a bank really could post both sides — a treasury buying
in one module and a forced seller selling in another. It was a patch in the KERNEL for a defect in
the bank (Law 12). With one face per bank the case is unreachable and the patch is gone: a party on
both sides of one book at crossing prices is refused at the site with a citation. It fires in a test
that posts exactly that and in nothing else — three banks, thirty-five suites, three hundred and
twenty-three checks, and no world in any of them trips it, which is the fold measured rather than
asserted.

**Capital is weighted by INTENT.** A holding up to where the bank's own treasury wants the line is
there because the treasury decided so, and it weighs what a claim on that issuer weighs — nothing,
for a party that cannot fail in the money it issues. What is held ABOVE that is a position somebody
took with a view, and a view can be wrong whoever it is about, so it weighs what a trading position
weighs (`regulation.riskWeight.tradingBook`, declared by item 9 and read here for the first time).
Only the part above: a line the bank is SHORT of its target is a position too, but it is not an
asset it holds, and capital stands against what a bank owns. That is Dealer Desks F2 with nothing
left to exempt, and Banks Capital B1.a's "risk weights differ by asset, and that is why a bank
prefers some assets to others" now has something to bite on — every bank in this world publishes
risk-weighted assets above zero and below its assets, and which of the two rules binds is an outcome.

**The `accounts` family measures it**, reading what the bank published about its own book — the
target and what it holds at marks, line by line — rather than rebuilding the treasury's arithmetic,
because a check that compared one derivation with another would pass whatever either of them did.

**One face is checked where it can be.** The audit sees the close of a period and a book's orders
are gone by then, so the `flows` family the plan asked for cannot be written: what it would measure
is checked instead as the ASSEMBLY fact it really is — exactly one module gives `BANK` a
participant, in markets and in venues — with the kernel's refusal as the run-time half.

**The observer shows a bank's dealing book on the bank's own card**, because that is whose it is.
There is no desk to show.

**Allocation is deferred to 11.3, with the reason.** The dealing limit is already a share of the
bank's OWN CAPITAL and moves with it; what is deferred is making that share an outcome of the lines
competing for headroom on their realised returns. Every realised return in this world is an artefact
of the opening balance sheets — that is what 11.3 is for — so an allocation driven by them would be
allocating on artefacts, which is Law 11 read forwards rather than backwards.

**The item is built and it is not closed.** Fourteen of its fifteen steps are done, one is deferred
above, and twenty-six checks are red for a cause that is not this item's and has an item of its own.
Closing it would mean either adjusting those checks or calling the tree green when it is not.


## 11.3 — an attempt, thrown away, and what it was worth

I tried to finish 11.3 and 11.4 in one pass. 11.3 got two of its four numbers — a bank opened at a
quarter to an eighth of its own capital instead of three quarters, and its blended cost of funds
fell from near a tenth to between one and three points — and it is reverted anyway, because of how
it got them.

**It was fitted.** The splits between the central bank, the three banks and the households, and the
size of the equity float, were chosen by running the world for a year, reading the capital share off
it, and going back to change them. Law 2: a result wearing a preference's name. Law 11: Part XII
work done in the middle of Part XIII. Whatever this item ends up stating has to be ONE declared
number with a reason, with every endowment derived from it.

**It papered over an explosion with a bound.** With the balance sheets rebuilt, sovereign lines
walked away — a bill worth one to nothing over a year, and when that was "fixed", to more than
everything it will ever pay. What I wrote to stop it was a floor under the dealer's offer and a cap
over its bid at what keeping the paper was worth to it, and I argued in the comment that this was a
choice between two named alternatives like a lender's floor. It is not. Parking at the central bank
is a REAL TRANSACTION at a rate with a counterparty; "keeping it" is a valuation with nobody on the
other side, and taking the better of a quote and a valuation is a cap. Law 6, and the comment I
wrote to justify it is the tell.

**What it was worth is three findings, now in 11.3's plan.**

1. **A market whose only participants are dealers has no anchor.** Both sides of a dealer's quote
   come from its own view, its own view follows the last print, so dealers sitting on the same side
   of their own targets cross each other in one direction every period and walk the line away. The
   missing party is a holder with a reason of its own — not another dealer's inventory position. In
   this world the households hold most of the sovereign's debt and never trade it, the money fund
   buys bills and nothing else, and the central bank buys to a share and stops. This is why every
   version of the item exploded, and it is NOT the opening balance sheet. It is now 11.3's first
   step, because nothing downstream can be measured until a book has two reasons in it.
2. **A bank never distributes**, so whatever share of its assets it opens with as capital, it
   retains everything it earns and the share ratchets. No opening state survives that alone (13g).
3. **The treasury's opening buffer cannot be stated independently of the central bank's assets.**
   It is central-bank money; what the central bank may owe is what it holds; so the buffer is what
   is left of that once the banks have their reserves. Derived, never stated.

**Nothing of the attempt is in the tree.** 297 of 323 checks pass, as before it.


## The 11s are folded into 12

**One item, not five.** 11.1 is closed and 11.2 is built; what was left of the eleven-family — 11.2's
deferred allocation, the whole of 11.3, the whole of 11.4 — is absorbed into item 12 and the sub-item
rows are gone from the worklist. Their plan files are deleted; item 12's carries their steps.

**They belong there and it is not tidiness.** XI-13 asks for a SECOND OPINION so that no price is one
party's own view read back to itself; §22 asks for an index that is not its own input; §44 asks for
an assessment made from state rather than from a price. What 11.2 exposed is that this world's
sovereign market has none of that: **its only participants are dealers, both sides of a dealer's
quote come from its own view, and its own view follows the last print.** Dealers on the same side of
their own targets cross each other in one direction every period and walk the line away. That is
XI-13's fixed point arriving in the one market the whole model funds itself through, and every other
piece of the eleven-family sits on top of it — an opening balance sheet cannot be measured in a
world whose prices walk, a depositor's decision cannot be measured against a bank whose funding cost
is an artefact, and a second currency cannot be laid over either.

So item 12 runs in that order: the anchor, then the balance sheets, then the depositors and the
count of banks, then a bank's own allocation across its lines, then the currency layer, the
benchmarks and the ratings. Its guard leads with Law 6, because the thrown-away attempt at 11.3
answered the walk-away with a floor and a cap, and the length of the comment justifying them was the
tell.

**What stands, and it is not small.** There is no desk; a bank is one party with one balance sheet
and one face to every market, in markets and in venues; the solver's self-cross patch is deleted and
the contract is refused at the site; capital weighs by intent and the trading book is capitalised
with a family measuring it; the money market decides nothing for a bank; the engine has no
placeholder; and there are three banks instead of two. 297 of 323 checks pass. The twenty-six that
do not are the anchor and the balance sheets, which is now the first thing item 12 does.


## pre12 — The guards that keep the documents true

**Where it landed and why.** Inserted between 11 and 12, ahead of the anchor, from a review of the
tree after the 11s were folded in. It touches no mechanism, no price, no balance sheet and no
participant, so nothing in it can move a number item 12 is about to measure — which is the only
reason a documents item may go before the item that needs the documents.

**What it did.** Eight things, each with the guard that catches the next one (Law 12).

1. **A shape with a scheduled death is a placeholder, and the register now knows it.** Both fund
   management fees said "worklist 13h" in their reason and were declared `shape`. Both existing
   guards check the FIELD `standsInFor`, so both passed, `report().placeholders` was empty and the
   honest measure (XI-14) read zero while two numbers with a named item sat in the shape count. The
   register refuses a `shape` whose reason names a worklist item; the two fees are placeholders
   standing in for `Fund Shares F3` at 13h. Placeholders 0 → 2, shapes 8 → 6. Neither number moved.
   The guard is narrowed to shapes deliberately: eleven policies and preferences cite a future item
   for something else — a rate parliament owns from 14 is still that rate at 14 — and a guard that
   read "worklist" and stopped would have called every one of them a defect.
2. **One fee, one formula, one convention.** Net assets × rate × year fraction was written twice —
   once in `payFee`, once inline in `runEtf` — and the copies disagreed about the case that matters:
   the ETF paid `min(owed, cash)` and wrote off the rest, the money fund settled the whole thing and
   let the wire refuse it. One `feeAccrued`, read by both. The convention is the wire's: what is owed
   goes over it whole and a fund without the money has a refused payment, because that is what
   happens to every other payment a payer cannot make (Money E1), and a part-payment with the
   remainder forgiven was income at one end with nothing at the other (Law 5) and a bound at the
   fund's cash balance (Law 6). `payManager` is deleted. The ETF now shows one refused fee in six
   periods where it used to show six discounted ones.
3. **`PLAN.md` says what the code is.** §3.2 gave `pricing` as three values; it has four. It listed
   six fields of `InstrumentKindProfile`; there are eighteen. §4's module table listed eleven fields
   of `SystemModule`; there are sixteen. All three re-synced, and `module.ts`'s `creditDecisions`
   doc comment, which sat above `marks`, moved to the field it describes.
4. **`ARCHITECTURE.md` §9 says what `no-bounds` matches.** It claimed `Math.abs` used as a floor;
   the rule never sees `Math.abs`, and never sees a minimum written by hand either. Both exclusions
   are kept and now stated, with what reads them instead — because the bound that got furthest into
   this tree was caught by reading the comment justifying it, not by a rule (11.3), and that is
   worth writing down as the mechanism it is.
5. **The progress figure counts every item.** The worklist had thirty-two rows, the manifest
   twenty-nine. `itemProgress()` now reads `WORKLIST.md` and refuses a mismatch in either direction.
6. **A PARTIAL row names the item that finishes it.** Seven named none; `coverage:spec` now reports
   any that do not and exits non-zero.
7. **CI runs the gate rather than a copy of it.** `ci.yml` listed five of `npm run check`'s six
   steps and omitted `plan:check`, so a stale progress block passed CI and failed locally. It runs
   `npm run check`, then `build` and `e2e`. A list in two places is a list that drifts (Law 4).
8. **`CLAUDE.md`'s spec size is right**: 5,285 lines, not 5,248. Both files were last written in
   the same commit.

**Found while working it.**

- **Items 10.1, 10.2 and 10.4 never had plan files at all.** The step said to give them the step
  counts their item files had; `git log` over `docs/plan/*` shows every file that ever existed and
  none of them is theirs. So the missing manifest row is the second half of a skipped first step
  ("open `docs/plan/<item>.md>`"), not a separate slip. They go in at `steps: 0` — an item with no
  plan file had no planned steps — and `render()` shows them as `closed (no item file)` with `—`
  rather than as items that were free. A fabricated count would have put a number I invented into
  the one figure that is supposed to be a recount (Appendix C). The percentage does not move.
- **The PARTIAL guard immediately found two rows the hand pass missed**, `Bond N11` and `Equity C2`
  — and its first version was WRONG in both directions: a bare `\b1[0-7]\b` matched the "11" inside
  the clause id `Bond N11`, and tightening it to require the word "worklist" then rejected
  `Equity C2`, which names 13g and 13h as bare ids. What the rule takes is a form that can only be
  an item: the word beside a number, an id carrying a letter or a point (`13h`, `4a`, `10.3`), a
  Part, or a Part XI mechanism. A bare number is a tenor as often as an item.
- **Two tests asserted the old shape count**, `world.test.ts` and `equity.test.ts`, and both carried
  a comment naming the two management fees and their worklist item while asserting the placeholder
  count was zero. The confusion was written down twice before anybody read it back.
- **`XI-14` has no coverage row, and neither has any Part XI mechanism.** This item's file said
  XI-14 would be re-marked, because `spec-coverage.ts` carries a PARTIAL text for it. `COVERAGE.md`
  has no such row: the spec index makes a row per REASON/VERIFY/FORBID *within a system*, and the
  seventeen mechanisms are cited by code and named in prose instead. Four keys in that file's
  initial-generation table — `XI-5`, `XI-6`, `XI-14`, `XI-15` — have therefore never matched
  anything, and five more name clauses that are no longer rows. Recorded, not fixed: the table feeds
  `--init` only, and it wants its own bounded change. Nothing was re-marked.
- **`npm run format:check` fails on 98 files** and nothing runs it — not `npm run check`, not CI.
  Left alone: formatting 98 files inside this item would bury its diff (Law 14). It is a real drift
  and it wants its own bounded change.

**The twenty-six red tests, named.** PLAN §5 asks for "the year-long run green **or its expected red
families named in the record**", and the fold's entry named none of them. They are the same
twenty-six before and after this item — verified by name, not by count. All of them arrived inside
11.2: 11.1 closed at 321 of 321 green, `392adc2` (fold the desks into the banks) took it to 11 red,
`40f596a` (a third bank) to 26, and it has been the same 26 since.

| file | test | waiting on |
| --- | --- | --- |
| `bank-resolution` | all ten, from "opens on the trigger that fired" to "lands a failed bank losses on the banks that funded it" | one number: the bank the test resolves has POSITIVE equity at marks (`hole` is −42,121,050), so there is no hole and nothing downstream fires. The opening balance sheet — a bank that is three quarters its own capital cannot be made insolvent by the trigger this test pulls |
| `run` | "takes the reserves behind it"; "leaves that bank shorter at the next close"; "stays green every period of a funding squeeze" | the depositors and the balance sheets they decide from |
| `omo` | "buys towards the share policy chose"; both remittance tests | the central bank's opening assets: no central-bank money exists that its issuer bought nothing with, and remittance is a read of what its own instructions earned |
| `estate` | "shows the estate, its programme and where the dead party went"; "a year with a death in it" | the same balance sheets, through who dies and with what |
| `capital` | "reads what its EQUITY costs off its own share price"; "a dearer cost of capital means a dearer quote" | the anchor: a cost of equity read off a share price is read off a price that walks |
| `treasury` | "funds itself over a year when the market is there" | the anchor: twelve payments missed over the year |
| `credit-events` | "runs a year on a state that spends past what it can fund" | the same |
| `money-market` | "moves the market rate when it moves, and only through the corridor" | the same |
| `curve` | "reads a level the holders of the paper put it at" | **the anchor itself**: the curve reads 0.0200 where the keenest holder posted 0.0585. This is XI-13's fixed point in the one market the model funds itself through, and it is item 12's first step |

**And two that are NOT obviously the anchor or the balance sheets.** Naming them separately is the
point of this half of the item: a reader who takes "the anchor and the balance sheets" at face value
will not look at them again, and item 12 must not close over either.

- **`tick` — "converges as the piece gets finer, in what it made and in the money it holds".** Law
  2's resolution invariance for the whole grid, failing by about three per cent (0.00928 against
  0.00898). If the balance sheets moved it, the record for 12 should say so; if they did not, it is
  a finding of its own about the grid.
- **`labour` — "fills the higher offer first when there are not enough hours for both (D1.a)".**
  Traced. The venue is correct: it prints 7000, allots all 2100 hours to the higher bidder, writes
  six rows of ten, and the wages settle in full (14,700,000 due, 14,700,000 paid, no failed
  instruction). Then `firm.1` **ceases inside the same period** — all six rows separate with
  `cause: "firm.1 ceased"` and unpaid severance — so both sides of the assertion read zero. The test
  builds an employer with no revenue whose only act is to hire sixty people, and it depended on that
  employer surviving the period it hired in. Whether what starves it is the opening balance sheet is
  exactly the question item 12 opens with; whether a test of a matching rule should depend on its
  employer's solvency is a question about the test. **Neither is answered here** — this item may not
  fix a red test, because a documents item that turned one green would be the anchor being built by
  accident.

**Deleted.** `payManager` and the second fee formula; the ETF's `owed > cash ? cash : owed`; the
`Math.abs` claim in §9; four wrong contract descriptions.

**Forecast, with its killer.** The two guards written here — manifest against worklist, PARTIAL
against a named item — will not fire again on this repository, because the drift they catch is
made one item at a time and `npm run check` now runs on the commit that makes it. The killer: if
either fires in the next three items on work that was done correctly, the guard is reading the wrong
thing and goes, rather than the items bending to it. The register's third guard has a different
shape and a different killer: it is a check on PROSE, and prose is where a person writes what they
mean. If it ever refuses a number that really is a shape — one whose reason mentions an item for
some other reason — then the reason is the wrong place to look and the death belongs in a field.

**State.** 313 of 339 checks pass, against 297 of 323 before. The sixteen new ones are this item's
guards; the twenty-six reds are the same twenty-six, by name.


## 12 — An anchored market: the second opinion, the balance sheets under it, and the currency layer

**What it was for.** XI-13 asks that no price be one party's own view read back to itself; XI-12 asks
for more than one money with no vehicle between them; XI-7 asks for a benchmark somebody paid; §44
asks for an assessment made from state rather than from a price. The item absorbed 11.3 and 11.4
because the anchor and the currency layer are one thing: no second currency can be laid on a market
whose only participants are dealers quoting off their own last print.

**FOUR COUNTRIES, and that was the first finding.** The plan said a second region and a second
currency. Two moneys cannot express XI-12 at all: `triangles()` is empty, there is no third leg and
no round trip, and "no vehicle currency" is a sentence about a world that cannot have one. So the
seed opens the United States (USD, Federal Reserve, US Treasury, `ust.*`) and three stub economies —
Europe (EUR, ECB, `bund.*`), the United Kingdom (GBP, Bank of England, `gilt.*`) and Japan (JPY,
Bank of Japan, `jgb.*`) — each a central bank, a treasury and one benchmark line, declared in one
table. The names are labels for clarity; a real economy abroad is 13i's.

**WHICH ACCOUNT A PARTY HOLDS A MONEY IN had twenty-odd writers.** Every one of them was
`{holder, issuer: party.bank}` — right in one currency and wrong in every other. It is
`ctx.accountOf(party, ccy)` now, one resolver: a party's own money at its own bank, a foreign money
at that money's own central bank. `world.cash` reads through the same rule, or a foreign balance is
not findable at all. That is Law 4 arriving in a place nobody had looked, because with one currency
the twenty copies all gave the same answer.

**What the layer needed that the plan had not named.**

- **A central bank lends to its own system** (Currency D4, Central Bank D1). A bank booked elsewhere
  has no reserve account here and no window, so a bank short of a foreign money has to BUY it —
  which is why a spot market exists. Without it the second currency arrived as an unlimited foreign
  overdraft and Money B3.c fired at −333,922,766 SOU on the first run.
- **A market says what it DELIVERS.** A pair delivers nothing anybody holds, so the asset desks and
  the liquidity walkers skip it, and a participant answers the sort of market it declared itself in
  (`ParticipantDecl.in`). Three separate crashes were one missing distinction.
- **`owedIn` is signed.** Short of a money and sitting on one are the same balance read from either
  end (Spot FX B1 and B2), so there is one read and not two rules.
- **A party deals in the pair between the money it needs and its own**, so one balance is never
  committed in three books at once and nothing is ROUTED through a third money (XI-12). **A bank
  speaks once in a book** — its unwanted balance IS its desk's position, and when the desk has
  decided on a round trip that leg is what it is doing there; both were Clearing A2 firing. **A desk
  past its own limit posts a size and no level**, which is the rung its paper book already had
  (XI-2) and is what moves a rate when every desk is on one side.
- **A tracker pays what a line is worth, never anything.** The first version posted market orders and
  walked the equity index from 99.88 to 4153 in two sessions: a forced SELLER is real (XI-2), a
  forced BUYER is what Appendix B forbids.

**And one fix that was not about currencies at all. A CLEARED SESSION THAT SETTLES NOTHING NO LONGER
PRINTS.** `mkt.share.etf.us` cleared at 63,905 against a NAV of 200 with `settledVolume: 0` — a level
nobody paid, written because the book had crossed. The authorised participants created 6.6 billion
shares against it and the next session's demand summed past 2^53 and threw. A price is what somebody
paid (Law 3, Clearing E1): the session now carries the last real price forward, visibly stale, with
a new `nothingSettled` reason, and the three no-trade paths are one function. The symptom was a
fund's share price; the cause was in the kernel's market; nothing between them was wrong.

**What else was built.** `indices`: one system, rules built from the registry's own regions and
currencies, levels that are reads and nothing that stores one, an index of an empty basket reporting
Missing rather than its base, and a benchmark that is TRANSACTED — secured and unsecured are two
benchmarks and this world's unsecured overnight book never clears, so it publishes no unsecured
fixing. `ratings`: three assessors with drawn methodologies, grades made from state through a view
with the prices CLOSED (so A2.a is structural, not a convention), coarse and sticky, and the conflict
kept and priced — the rated party pays the assessor every period, by instruction. The second opinion
is a fact on the party kind's profile (`speculative`) and a `market.noView` observation every period
a book runs with orders in it and nobody with a view: in this world that is the four goods markets
and nothing else. `crossMarket` built with both contributions; Currency D4 added to `money`.

**Performance, after the owner said twenty-five minutes was not sustainable.** The suite was
quadratic in the period number: `failedPayments` filtered the entire ledger on every call and an
assessor asks it of every issuer it rates; an index level chained from the base on every read, and a
basket weighted by what was BOUGHT reads a period of the ledger to answer; `owedIn` walked every line
a party had issued, once per pair market per money. All three are Law 18 — the same numbers, less
traversal — and the fourth was a correctness statement that happened to be most of the cost: an
assessor rates the paper anybody can buy, not every bilateral loan row in the world. A forty-period
run went from 962ms a period to 305ms, and the suite from over twenty-five minutes to four and a half.

**State.** 248 of 329 tests pass, against 195 of 338 when the item opened. The 81 reds are named
test by test in `docs/BUGS.md`, with the four things that changed under them — chiefly that the
seed's scale is DERIVED now, so every test that asserted an absolute quantity reads a different
number. None of them is a world that will not assemble.

**Six findings written down and positioned**, none chased (Law 11): 12-13 the banks sell the seed's
cross holdings in week one (13h's portfolio decision missing); 12-14 the dollar drifts one way
because only one side of the world trades (13i); 12-15 an equity line walks away as the sovereign
line did before 11.3 (its own item, before 13g); 12-16 the print above, fixed here; 12-17 every
sovereign grades `c` because a state's capacity is its tax base and no read of it exists (12a);
12-18 a bank's balance sheet takes one step out of true and never comes back — measured to the
point where the equity side is exactly its events, so the gap is on the read side (12a's income
statement is the decomposition that will name it).

**Forecast, with its killer.** The claim this item makes is that a price made by parties who all
follow the same rule is not a price, and that saying so (`market.noView`) is better than preventing
it. The killer: if the four goods markets it names stay the only ones for the next three items while
the equity book goes on walking away (12-15), then the observation is not reaching the thing it is
about and it wants a participant rather than a note.

## After 12 — the holding pen emptied

**What this is.** Not an item: the close of `docs/BUGS.md`. Item 12's own entry above says six
findings were positioned; the file held twenty-seven, in two numbering series that had collided —
`12-13` to `12-18` named two different findings each — plus a list of the reds the item closed with.
Every one is now either RESOLVED or POSITIONED into the plan file of the item that owns it, which is
the only way a finding may leave that file. **The file is empty, so it is deleted** — the same
treatment a closed item's own file gets. It comes back the next time a session parks a finding.

**Two new items were inserted**, both of them asked for by findings that said so themselves:

- **11.6 — the module contract** (after 11.5, before 12a). 12-7 (the kernel declares `FIRM` and
  `HOUSEHOLD` while `firms` and `households` own their behaviour, so the depositor guard cannot cover
  them) and 12-12 (a session asks every party of a kind whether it has an order — 827,000 questions
  a period at full scale, four fifths of what a period still costs, and the kernel cannot guess the
  answer, so a participant must declare which markets it is in). Taken together because both are what
  a `SystemModule` DECLARES and neither moves a number; item 2a is the precedent for settling the
  contract in one item. It sits between 11.5 and 12a because every neighbouring item moves numbers
  and this one cannot.
- **12d — the tests catch up with the world** (after 12c, before 13a). The reds item 12 closed with,
  refreshed against a full run: **283 passed, 90 failed of 373**. 85 are assertions about a world
  that moved; **3 are contract violations and are listed apart**, because a violation names a
  mechanism (ARCHITECTURE §5) and the danger of an item like this is that one gets migrated away.
  Not earlier, because 11.5, 12a, 12b and 12c each move the numbers these tests read; not later,
  because a suite that cannot guard is worth less with every system added to it.

**Where each finding went.**

| finding | what it was | where |
| --- | --- | --- |
| 12-1 | a bank's opening funding could not see the assets other modules endow it | RESOLVED in 12 |
| 12-2 | a bill printed above par | RESOLVED in 12's anchor step |
| 12-3 | the long end has no holder (not a bug — an incomplete model) | 13h |
| 12-4 | an audit violation in the bank-failure scenario, taking the whole file's collection with it | RESOLVED as written — `bank-resolution.test.ts` collects again (11 tests run, 4 pass). An unexpected violation is still asserted inside one of them, and that red is named in 12d with the other six |
| 12-5 | a bank fails in period 7, and a world that opens with more banks ends with fewer | 11.5 |
| 12-6 | tests that find their period by running the world | 12d |
| 12-7 | the kernel declares two kinds whose behaviour a module owns | 11.6 |
| 12-8 | a bank defends a deposit past what the guarantee on it costs | 11.5 |
| 12-9 | a lien could name a party that had ceased | RESOLVED in 12 (it blocked the work) |
| 12-10 | a ceased bank keeps its reserve overdraft and nobody is owed it | 11.5 |
| 12-11 | the journal and ledger walked end to end on every read | RESOLVED in 12; the event growth underneath it is named in 11.6 and measured at 16 |
| 12-12 | what a period costs at the real scale | 11.6 |
| 12-13 (first series) | a quantity was a `number`, so a fraction of a cent could be anything | RESOLVED in 12 (`Qty` became a branded type with five doors, and the clearing solver's runtime check was deleted) |
| 12-14 (first series) | every dealer opens above its own limit, because the float is the firm's whole book | 12c |
| 12-15 (first series) | the world produces a hundred and fiftieth of the scale its seed states | 16 |
| 12-16 (first series) | the ETF's price collapses and the demand curve walls past 2^53 | RESOLVED in 12 — the wall by `line.instrument.issued` (a bid is never for more of a line than exists), the collapse by the print fix below; what the ETF still fails is named in 12d |
| 12-17 (first series) | the suite after the scale change, 144 red of 338 | SUPERSEDED by 12d's refreshed list |
| 12-18 (first series) | the test rig is not the world (a decision, not a bug) | `docs/PLAN.md` §7 |
| 12-13 | the banks sell the seed's cross holdings in week one | 13h |
| 12-14 | the dollar drifts one way because only one side of the world trades | 13i |
| 12-15 | an equity line walks away | 12c (the item it created) |
| 12-16 | a cleared session that settles nothing used to print its level | RESOLVED in 12 |
| 12-17 | every sovereign is rated `c`, and every firm downgrades in one session | 12a |
| 12-18 | a bank's balance sheet takes one step out of true | 12b (the item it created) |
| 12-19 | the banking system opens below its own liquidity standard, and nobody lends | 11.5 (the item it created) |
| pre12-1 | the resolution invariance does not converge | 11.5 |
| pre12-2 | a matching-rule test depends on its employer's solvency | 12d |
| the reds | named test by test | 12d, refreshed against a full run |

**And pre12-1 is no longer the finding it was written as.** It was measured as a three per cent miss
(0.00928 against 0.00898). At item 12's close both tests in `tick.test.ts` stop earlier than that,
with `Impossible: [Law 8] what moves on treasury.us's money:fed:USD is 711624048078206100, which is
not a whole number of pieces` — at a finer grain some site divides and does not land on the grid, so
the convergence is not reached to be measured at all. 11.5 owns both halves, because it re-derives
the opening balance sheet they run against; if the opening is not the cause, the record there says so
and it becomes an item of its own with both measurements as its starting numbers.

**Two things were kept rather than tidied.** Every measurement moved into a plan file is repeated
VERBATIM, including strings printed before item 12 renamed the countries (`cb.north`, `PHX`), with a
line saying so — a measurement is what it said, and a re-typed one is a claim. And each finding's
number survives as its handle: the plan files name it, this table maps it, and no plan file links to
a path that is gone.

**Documents touched.** `docs/plan/11.5`, `12a`, `12b`, `12c`, `13h`, `13i`, `16` each gained the
findings positioned into them and the steps that close them; `11.6` and `12d` are new; the manifest
gained two rows and five step recounts; `docs/PLAN.md` gained the rig decision (§7) and says the
holding pen is deleted when empty; `docs/WORKLIST.md` gained two rows. Eight comments in the engine
and the tests cited the deleted file by finding number and now cite the worklist item that owns each
one — a stale comment is a defect (Law 16), and a path that does not exist is the stalest kind.

## 11.5 — The banking system opens meeting its own liquidity standard

**What it was for.** A bank lends what it holds beyond what could leave it (Banks Funding D4, D4.a),
so a bank that opens below that line lends nothing on the first morning and everything downstream of
credit is dark. Item 12 measured that and named it 12-19; this item was to make the opening satisfy
the rule the world is then measured against, with no number chosen to make it so.

**THE CAUSE WAS NOT THE ONE THE PLAN NAMED, and measuring it first is what found that.** The plan
said the mix: the banks hold too much of their book in government paper, which the window takes only
at a haircut, so derive `seed.centralBank.openingHoldingShare` from the standard. The algebra says
otherwise. `liquid ≥ couldLeave` rearranges to

```
A·line + insured  ≥  illiquid assets + paper·haircut
```

— **what cannot run (capital and insured deposits) must fund what cannot be turned into cash at
par** — and the haircut term is small: a paper share of 0.166 against a haircut of 0.05 is 0.83% of
a book, against capital lines of 3.8 to 6.8%. The mix was never the binding thing. What was on the
other side was `other`: in a six-bank world the two banks that made a market in a listed line opened
holding **6.84e10 of equity float against 1.76e10 of capital**, and opened at a liquidity metric of
0.84 and 0.89, while the four banks that made no market opened at 1.04 to 1.07. **A share raises
nothing at the central bank's window** (C1.a), so the float was four times the capital in
unfundable asset on the one balance sheet whose whole liquidity turns on what its assets raise there.

**THE FIX IS WHO HOLDS THE FLOAT, and it deletes a seeded outcome** (Seed E1). A share is a claim on
the residual (Equity A1) and the parties in this world with a reason to hold one are its savers
(B3); a dealer's inventory is a working position it takes by TRADING, so a seed that opens the desks
holding the whole float of every line they make has stated an outcome and given ownership to the one
party whose holding of it is supposed to be the result of a market. Item 9 opened them holding it
for a reason that was true then — "a bank that opens making a market with nothing to sell can only
ever bid", and the one alternative tried, a FOUNDER, drained the sector's money into a hole. A saver
is not that hole: it consumes, it banks, and since item 10 it has a portfolio decision of its own.
Every member holds the same slice of every line, because the seed has nothing to say about which
saver prefers which firm, and the line is what the members hold (Law 8: the division that does not
come out is never issued, so every piece has a named holder from the instant it exists).

**Measured after.** Every bank, in every seed, at 2, 3, 4, 6, 8, 10 and 20 banks, opens at a metric
of **1.03 to 1.08** — none below. The count of banks is no longer load-bearing (XI-15, Seed B1), and
what decides the number is each bank's own capital line against the haircut on its own paper. Credit
is quoted from period one, 40 quotes a period in a six-bank world, and `costOfCapital` is a NUMBER
(0.0320 for a firm at period 12) where it was Missing for most firms before. A twenty-bank world
stays above the line for six periods and then one bank and then five fall below, which is C4 working
— the position is the residue of everybody else's period and the bank did not choose it.

**`seed.centralBank.openingHoldingShare` IS A POLICY, and that is the item's answer to its own first
step.** It was a PLACEHOLDER standing for a derivation from the liquidity standard, and there is no
derivation to make: the standard is an inequality that is slack from a paper share of 0 to one of
about 0.77, not an equation with one root. Central Bank C1.a says it outright — "the size is set by
policy" — so the parameter is now a policy owned by the central bank, with the measurement in its
`why`. Nothing was derived into existence to satisfy a step; the step was wrong and the record says
what replaced it.

**AND TWO UNIT ERRORS IN THE SEED, both of the same class, which is what pre12-1 actually was.** The
resolution invariance (Law 2) is: declare the same world in finer pieces and its path must not move.
It was written up as a three per cent miss and had since become a throw. Neither description was the
defect. `params.amount` returns a count of PIECES, and:

- `hoursOffered` (pieces of an hour) was divided by `hoursForOne` (named hours from the recipes), so
  **this world's entire real economy was multiplied by the subdivision of an hour**;
- the central bank's foreign reserves were accumulated as `drawn × price` — a count of PIECES times
  a price per NAMED unit — so **eight per cent of the system's paper became eight times it**, and
  the money issued against it went with it.

Both move again when the pieces are made finer, which is why the invariance test caught them and
nothing else did. With one unit on both sides of every ratio and every value: the opening money
stock is 3.7046e9 / 3.7183e9 / 3.7197e9 USD at a cent, a tenth of a cent and a hundredth — a gap of
0.37% then 0.04%, converging by the factor the piece did — where it had been 3.13e10 / 2.83e11 and a
refusal to open at all. The audit is green in every family at all three subdivisions. The world is
**8.4× smaller** than it was, and that is the error leaving rather than the world shrinking.

**What is still not invariant, and it is 16's:** `produced` agrees to five figures at a cent and a
tenth and then moves 1.7% at a hundredth, with the batch count going 24, 23, 28. A firm on the edge
of starting a batch starts it in one run and not the other — this world decides at thresholds
because Law 2 forbids deciding at an average — so a path invariance over threshold decisions can
hold in distribution and not point by point. Worklist 16 already has the step ("resolution at
1×/2×/4× with the error bar recorded") and that is where the claim belongs; the numbers above are
its starting point.

**The other three findings this item carried.**

- **12-10, a ceased bank's reserve overdraft.** `moveBook` skipped everything that was not a
  positive quantity, so a bank whose reserve account was overdrawn when it failed took the overdraft
  to the grave: `money` reported a borrowing with no lender row and `names` a ceased party still
  holding something, every period for the rest of the run, and neither number moved again because
  nothing was still running. A negative balance is not a holding — it is a BORROWING from the
  issuer — and it moves the only way one can: the acquirer pays it in, the dead bank's account
  closes at zero, and the acquirer's falls by what it assumed. Appendix B's "no death without a
  destination", applied to a liability.
- **12-8, a bank defending a deposit past what the guarantee costs.** `setBoard` struck the board at
  `worth − margin − premium`, correctly, and then `defended` let it match a rival anywhere up to
  `worth` — comparing a RATE against a number the bank's all-in cost exceeds by the premium, so it
  paid the guarantee twice, once out of the board and once out of the match. The stopping point is
  now struck per class from the same numbers the board is: what the money is worth less what the
  guarantee on that class costs. One number, one place, per class (Law 12).
- **pre12-1** above.

**Still dark, and it is not credit's fault any more.** No loan is written and no firm invests, over
sixteen periods of a six-bank world — but `investmentGap` is 0 and 13 of 24 firms are bound by
DEMAND, with capacity 14× the run rate. That is 12-15's level, already positioned at worklist 16,
and this item removes the credit explanation from it: the chain from a bank's funding to a firm's
cost of capital is lit, and what is missing is demand rather than a lender.

**Found and not chased.** `docs/COVERAGE.md` has no row for `Central Bank C1.a` (and none for
`Central Bank C2.a`), so its 1,361 rows are not one per clause. Nothing enforces completeness — the
citation check only verifies that the citations used resolve. Worklist 16's coverage final pass is
where that belongs.

**Forecast, with its killer.** The claim is that a bank's opening liquidity is decided by what it
holds that the window will not take, and that the reserve-to-paper mix is second order. The killer:
if a later item puts any other unfundable asset on a bank's opening book — 13f's securitisation
vehicles, 13h's fund stakes — and the metric falls below one again without the float coming back,
then the mix was load-bearing after all and `seed.centralBank.openingHoldingShare` wants a derivation
rather than a policy.

## 11.6 — The module contract: a kind lives where its behaviour lives, and a participant says which markets it is in

**What it was for.** Two findings from item 12 about what a `SystemModule` DECLARES, neither of them
moving a number. Item 2a is the precedent for taking them together: the contract is one subject.

**A KIND'S PROFILE BELONGS TO ITS MODULE; A KIND'S ID BELONGS TO THE KERNEL, and that distinction is
the whole of the first half.** 12-7: `FIRM` and `HOUSEHOLD` were declared in `KERNEL_PARTY_KINDS`
while `firms` and `households` owned their behaviour, so the guard that says a module declaring a
depositor must say how it leaves (Banks Funding A1.d, E1) — which asks each module about its OWN
declaration, because a reduced world has the kernel's kinds in it and nothing to speak for them —
could say nothing about the two largest classes of deposit there are.

The plan's step said to move the kinds into their modules and have every module import them from
there. **That half is wrong and the lint rule says so**: `labour` posts openings for firms,
`ratings` charges them, `equity` opens a line on one, `treasury` taxes them — importing `FIRM` from
`mechanisms/firms/` is exactly the cross-module import `no-cross-module-import` forbids. What
separates cleanly is the profile from the id: a PROFILE is behaviour (representation, failure modes,
whether it borrows, what depositor it is, whether it issues money) and one module owns it; an ID is
a name and any module may need to say it. So the two profiles moved and the two ids stayed, the
`assemble.ts` comment recording the gap is deleted, and `requireBankChoices` now covers every
declared depositor with no exception list — the kernel's own kinds are the ones money needs, and not
one of them is anybody's deposit base. `seed.foundation` gained `firms` in its `requires`, because
it creates parties of that kind and a world that registered the kind elsewhere was relying on luck.

**A PARTICIPANT SAYS WHICH MARKETS IT IS IN** (12-12). A session asked every party of a kind whether
it had an order in it; at full scale that is 261 markets against 3,178 parties. The kernel cannot
guess which could answer yes — which books a party is in is its own business and changes period to
period — so the module that owns the party says, through `ParticipantDecl.markets(view)`, and the
kernel builds a per-cycle index from it. `firms` and `households` answer out of the SAME published
plan their `orders` are read out of, so the two lists cannot disagree (Law 4, Law 19); a participant
that declares nothing is asked about every market of its kind exactly as before.

**What could go wrong here is silent** — a `markets` narrower than the party's own `orders` loses
real schedules out of a real book and nothing throws — so `test/fan-out.test.ts` runs the expensive
path once against the cheap one: every participant asked for orders in every market there is,
asserting it never posts in a book it did not name.

**Measured, and the item's own preamble was wrong.** At `foundationWorld('full')` — 3,178 parties,
309 instruments, assembly 0.7s — with the door off and then on, and the same world both ways
(47,322 / 109,380 / 201,230 events):

```
        door off      door on
p1      8,419ms       6,888ms
p2     10,519ms       9,404ms
p3     21,270ms      19,277ms
```

**The market fan-out is 9 to 18 per cent of a period, not the four fifths item 12 attributed to it.**
That attribution came from reasoning about the question count rather than from timing it, and this
is the correction. The same identity was checked twice more at rig scale by hashing the ledger and
the journal with the door on and off: 8,581 instructions at 6 banks/40 firms and 7,635 at 20
banks/200 firms, identical hashes both times.

**And a period costs MORE than item 12 recorded, which is 11.5's doing and worth naming.** Item 12
measured p3 at 12.4s and about 69,000 events; it is now 19.3s and about 92,000. The float moved to
the savers in 11.5, so every household cell holds every listed line and every one of those holdings
is revalued every period. That is the price of putting ownership where the reason is, and it is a
real cost rather than a defect — the answer to it is 12c, which gives a saver a reason to hold one
line rather than all of them, after which what a cell holds is an outcome and not a slice of
everything.

**Forecast, with its killer.** The claim is that the fan-out was a second-order cost and that what
dominates a period is what the world has to SAY — revaluations, surprises, settlements — which grows
with holdings rather than with markets. The killer: if 13a's derivative books and 13b's classes push
the market count up an order of magnitude while holdings stay flat and a period gets dearer in step
with the markets rather than the holdings, then the fan-out was the thing after all and the index
wants to be the default rather than a door a module opts into.

## 12a — Reporting and estimates

**What it was for.** §48 arrived with this item inserted. The world already had the SUBSTANCE of
corporate reporting — accrual matching, inventory at the lower of cost and net realisable value, a
firm publishing its own expectation — and none of the APPARATUS: no fiscal calendar, no report, no
bank publishing an estimate of anybody, no surprise with a name on it.

**THE KERNEL CHANGE IS ONE LINE OF STATE AND IT UNLOCKS THE REST.** `moveEquity` always received the
cause its writer wrote and always threw it away, so comprehensive income was recoverable exactly —
it is the movement of the account — and NOTHING ABOVE THE BOTTOM LINE WAS. A report that wanted a
line of it would have had to parse the reason strings on money legs, recovering by inference a fact
its writer knew and did not record. The register now keeps an append-only `EquityEntry` per move;
`EquityMove` gained the date and the instruction, and making them required is what found all five
writers; settlement's cause is the instruction's own reason rather than `instruction <id>`, which
was useless to group by; and `stateEquity` writes the opening as an entry so Σ entries IS the
balance with no opening left over to argue about.

**It is the itemisation and never the balance** (Law 4). `equityWalk` is still the accumulator and
still authoritative, so the two are independent records and the `accounts` family compares them
(Audit A1.a). **The new contribution found a defect on its first run**: it checks the sum AND the
COUNT — a sum can be made to agree by two errors, a count cannot — and a cell split was copying the
equity walk but not the itemisation, so split cells' accounts said eighteen moves while their
ledgers carried nine, every period in every seed.

**WHAT A REPORT IS.** Every figure in it is the same number some other reader of this world already
has. Income is the equity ledger's entries grouped by the instruction's own cause with the
revaluation subtotal separable and the bottom line the SUM of that decomposition. The balance sheet
is `balanceSheet()`, EXTRACTED from the accounts family so the report and the audit are one
implementation — a report with its own would be a second set of accounts able to disagree with the
one the audit proves. The cash is the wire's own money legs grouped by NAMED counterparty, money and
cause: 321 lines naming the assessors a company pays, the bank that pays its coupon and the
household cells that take its dividend, where the raw legs were 2,141.

**THE CALENDAR IS DATES** (G6). A company's year ends in a month drawn from the world's seed and its
own identity, so reporting season happens continuously rather than in one week; a quarter is the
pair of dates that bound it and a whole number of periods only by accident. Two bugs in that walk,
both found by running it: `quarterClosedBy` walked back to a closed quarter and stopped, so a
company published its first quarter and then the same quarter for ever — three reports in forty
periods where there should have been nine; and the LABEL was wrong the same way, with July after an
April year end coming out FQ3 where it is FQ1 of the next fiscal year.

**GUIDANCE IS NOT A SECOND NUMBER** (B4). What is published is the firm's `income` outlook — the
same object `firms` reads when it plans — with the periodicity stated beside it. A revision is
looked for every period and published only when the management's own view has moved past the dust of
the arithmetic that produced it: a statement republished every period regardless would be a calendar
and not news. Restatement needs no audit coupling: the entries are append-only, so a published
figure can change exactly one way, and re-reading the span every period is what catches it.

**BANKS DISAGREE, AND NOBODY ARRANGED IT** (C3). An estimate is an adaptive outlook over what THAT
bank observed of THAT company — the reports it has seen and the guidance the management published,
weighed by what that management's record is worth — corrected at the bank's own DRAWN memory.
Measured over 52 periods: three banks covering one name with a spread of 2.6e7 between them.
Coverage is uneven because a bank covers a name its own book needs the view of, and it COSTS:
`research.hoursPerName` is technology, costed at what the labour venue cleared at and paid per
member to named household cells — 359 settled instructions over 40 periods, with no research budget
parameter anywhere, because a budget is the cost STATED where D2 asks for it to be PAID.

**Settling runs before covering**, so a surprise is measured against what the bank said BEFORE the
report. A view revised on the report and then scored against it would be surprised by nothing.

**THREE FORBIDS IN THIS SYSTEM BREAK SILENTLY, so they are CHECKS and not tests**
(`tools/check-forbids.ts`, wired into `npm run check`): no price read anywhere in `research` (C6),
no module outside `research` naming `research.surprise` (F2.a), and nothing outside `research`
calling `consensusOf` (E2, E3). The observer is the one exception to the last two and it is the
exception §45 B2.a names: a surface decides nothing. A world that broke any of these would run,
publish, print and balance, and look exactly like one that did not — which is precisely the case
for a guard rather than an assertion, and "a rule that can be a check should be one".

**AND THE RATINGS MEASURE CHANGED, which is 12-17's fix.** An assessor measures what falls due
against what the issuer TAKES IN — a coverage ratio, read off the equity ledger with the MARKS
EXCLUDED, because a revaluation is what the world now thinks a thing is worth and nobody handed it
over. A treasury is no longer graded by a book equity that is negative by construction. A second
defect was found on the way: `failedPayments` took a COUNT of failures rather than a horizon, so an
issuer that missed one payment in its first week was graded the worst there is for ever — a count of
events is not a horizon, and a rating is a judgement about a party's state now.

**The grades are still one grade, and that is the world.** Every issuer in this world misses payments
in every seven-period window — the treasury 1,129 of them, a firm 24 — and an issuer that cannot pay
what falls due is what the worst grade is FOR. Parked as 12a-2 and positioned with 12-15's level at
worklist 16: forcing a spread here would be tuning the assessor to make the world look solvent.

**Coverage.** 32 Reporting clauses MET, 5 PARTIAL (G3 and H1–H4, which are Part XII measurements and
say so). Requirement coverage 44.8% → 47.2%.

**Found and not chased.** `docs/BUGS.md` 12a-1: the equity index reads a level its own prints do not
make, from period 25 on, a step rather than a drift — not this item's, since nothing in it touches a
price, and most likely 11.5 moving the float, since a free-float weighting is a read of who holds
what. 12a-2 above. And `docs/COVERAGE.md` has no row for `Reporting A1.a` (nor `Central Bank C1.a`,
nor `C2.a`), so its 1,361 rows are not one per clause — worklist 16's coverage pass.

**Forecast, with its killer.** The claim is that a report built as a READ cannot disagree with the
books, because the two are the same function and the audit compares the itemisation against the
balance both ways. The killer: if 12b's balance-sheet step turns out to be an event whose read side
and equity side differ, then a report published in that period carried a balance sheet the equity
account did not agree with, and `reporting.restate` should have fired and did not — in which case
the restatement trigger is watching the wrong half.

---

## 12b — The balance sheet that takes one step out of true

**The step was named, and it was one conversion.** 12-18 reported that at period 13 every bank's
`assets − liabilities` parted from its equity account by a fixed amount — 5,144,414.06 on a book of
1.6e13 for `bank.a` — and never came back. Item 12 had ruled the foreign half out, on the evidence
that the holdings which MOVED most over that period were repo rows and equity lines in the bank's own
money. That rule-out was wrong, and it was wrong in an instructive way: the holding that moves most
is not the holding whose two sides disagree.

**It did not reproduce on today's tree**, so the first work was establishing why rather than
declaring it fixed (Law 13: a bad number is a finding). Reproduced exactly at `410bf16`, gone at
`a6b2922` — the commit that moved every cross-border holding from the commercial banks to the central
banks. The defect did not go with it; only its excitation did.

**The decomposition.** At `410bf16`, `bank.a`'s p13 mark event on `jgb.2036-03-15` booked
`-6,097,242,077.790232`, and the JPY/USD rate in force was `0.9991562719689123`:

```
6097242077.790232 x (1 - 0.9991562719689123) = 5144414.053   against the reported 5144414.049
```

**The cause, and it had two sites.** A VALUE IN ONE MONEY WRITTEN INTO AN ACCOUNT KEPT IN ANOTHER.
`world/revalue.ts` computed a mark move in the instrument's currency and wrote it to an equity
account in the holder's — while the read it is checked against converts (`Valuation.inMoney`). Its
own comment already claimed the marks "then convert at the new rate" and the code never did. The
issuer's mirror of that move had the same hole from the other end, and the issuer's money is not
always the holder's either. And `world/assemble.ts` `stateEquityAsRead` kept a SECOND COPY of the
balance sheet that summed `valueOfLots` across four moneys without converting any of them, so at any
rate but one the world opened with every central bank contradicting its own sheet.

**The fix removes code (Law 12).** `intoOwnMoney` is one read of one rate, and `revalueForeign` now
takes its `now` from it too — so the two halves of the decomposition
`v(t)r(t) − v(t−1)r(t−1) = r(t)(v(t) − v(t−1)) + v(t−1)(r(t) − r(t−1))` are exact by construction
rather than by coincidence. `stateEquityAsRead`'s copy of the balance sheet is deleted: the seed now
states the opening account as `balanceSheet`, which is the read the audit checks it against and the
read a public company publishes (Reporting A2.a). Three readers, one function. No tolerance anywhere
was widened, which 12b's own step forbids.

**Also corrected:** the Audit B5 violation reported its size against the equity account alone, when
the identity it had just failed is the read against equity AND the revaluation account — so for a
central bank, the one party whose second account is the whole point, it named a different number from
the one that failed.

**Measured.** A four-currency world at a stated opening rate of 0.8: before, every central bank
breaks at period 1 and stays broken; after, the identity holds for every party over 52 periods.
`packages/engine/test/balance-identity.test.ts` is that measurement, and it asserts the world it runs
in has foreign positions in it — a test of a conversion in a world where every rate is one is
`x * 1 === x`.

**Coverage.** `Audit B5`, `Seed C1`, `Currency D2`, `Money A2.b` re-marked with the one read.

**Found and not chased**, all in `docs/BUGS.md` and all to be positioned when this item's successors
open: **12b-3**, no pair has ever traded — every FX session is `noDemand` in all six pairs for ever,
because `a6b2922` moved the foreign holdings to the central banks and the demand side went with them,
so the entire currency layer's price discovery never runs; **12b-4**, the seed adds two currencies in
two places (harmless at parity) and `seed.openingRate` is one number for six pairs, so no consistent
non-parity world can be stated at all; **12b-5**, the trading-book check's dust counts its own terms
and not the published figure's; **12b-6**, a research desk keeps covering a company that has ceased.
**12b-1** — a price is on no grid — was not parked but DECIDED: the owner's standing decision is that
every quantity has a realistic minimum subunit, a price included, and it is worklist **12b.1**,
inserted after this item and before 12c and 12d.

**Forecast, with its killer.** The claim is that this identity now holds for structural reasons and
not arithmetic ones: there is one balance sheet, one rate read, and the two sides of the revaluation
decomposition are complements. The killer: if 12b.1's price grid or 12c's equity fix produces an
`Audit B5` violation whose size is not dust, then the identity was holding because every rate in the
delivered world is one, and `balance-identity.test.ts`'s second world was not the excitation it
claims to be.

---

## 12b.1 — A price has a smallest piece

**The owner's decision, and it was right for a reason the item only found by building it.** Every
quantity in this world has a realistic minimum subunit; a price is one of them. What the build then
measured twice contradicted what the plan predicted, and both corrections made the item better.

**What it is.** Each kind that can be POSTED at a level declares its smallest increment in the terms
a person states it in — a cent a share, a ten-thousandth of a bond's own face, a cent a tonne, a
whole money for a machine — and a rate's belongs to the money it is quoted in, which is a pip. They
sit in `registry/grid.ts` beside the piece constants, because they are the same decision about the
other grid. The registry converts them through `priceOf` (a tick IS a price: the smallest one that
is not nothing) and refuses at assembly both mistakes: a kind somebody can post that declares none,
and a kind nobody posts that declares one.

**Where it is applied, and where it deliberately is not.** ONE door, in the book: every order
reaching the solver goes through `runMarket`, and there every limit lands on the market's grid. The
PRINT is untouched — a cleared level is a posted level (Clearing C4.c, "posted and never a bracket"),
so every print is on the grid by construction and nothing rounds an outcome (Law 6). Measured: 29 of
29 markets that printed in a year, at three different grids, on the grid; none off it.

**Correction 1 — the direction is not a choice, and the item is far smaller for it.** The plan said
each poster must say which way its price rounds, as `downTick`/`upTick` make an author say for a
size. Wrong: a size means two things (what a party CAN do, what it MUST do) and a LIMIT means exactly
one per side, which `Order.price` already states — the most a buyer will pay, the least a seller will
accept. A buy that cannot be at 49.7938 can only be at 49.79 and a sell can only be at 49.80. So the
side decides, one place in the kernel honours what the poster promised, and the twenty-one module
files that post orders are untouched.

**Correction 2 — it is a TECHNOLOGY, not a RESOLUTION, and the measurement said so.** A finer PIECE
rounds an amount, so its effect shrinks with the piece and the path converges (the existing
invariance test asserts exactly that ratio). A finer TICK moves the LEVEL a decision is taken at, and
a coarser one pulls every bid down and every ask up until books that used to cross no longer do — so
it changes WHO TRADES. Measured: the money stock moves 3.2% between one tick grid and another and
does not converge in either direction. That is what a tick does in a real venue and why exchanges
argue about tick sizes. So `resolution.tickShift` became `markets.tickShift`, declared TECHNOLOGY,
and what the invariance test asserts is that every STRUCTURAL invariant holds EXACTLY at every grid —
never that the path is the same, which it is not and should not be.

**And the audit settled the derived case within two periods.** A fund's net asset value was ticked
first, on the argument that a fund publishes to the cent. `Fund Shares A3` reported it immediately —
`etf.us has equity of 3` — because rounding `assets / shares` leaves the difference with nobody
holding it and a fund's equity is zero BY CONSTRUCTION. A grid belongs to what is POSTED. So the
traded line keeps its cent tick (E2: a claim on a book and a line in a market are two numbers about
one thing) and the derivation is left exactly as the arithmetic gives it.

**One violation stopped the build and was fixed where it stood.** A fully-redeemed ETF has no shares
to divide by, and `navOf` threw at a dealer asking `view.mark()` for a line it holds none of.
`world.markOf` now answers `none` for a derived line with nothing outstanding: that is what an
OPTIONAL read owes a caller, and XI-6's "unpriced" is the right answer to "what is one of these
worth" when there are none. A reader that REQUIRES a price still throws at the site that requires it.

**What it exposed, which is the point of the exercise.** Without a grid, this world's shares had
drifted to 0.00026 USD each on floats of twenty-four billion shares a firm. With one they sit on the
smallest thing that exists. The grid did not break the share book — it made the break undeniable, and
it is 12c's finding 12-15 walking downwards. The tick is NOT loosened to accommodate it: a cent a
share is what a share market quotes in, and a grid widened to fit a broken price would be the price
choosing the resolution.

**What it does not fix.** A tick is not exactly representable in binary, so `n x tick` still carries
one rounding, and the cash a trade settles at still lands on its own money's grain — so the rate a
trade REALISES can differ from its print by less than one piece, exactly as `clearing/market.ts`
already said. What changed is that a price is now a whole number of a real market's increments
instead of a number with seventeen digits in it.

**Coverage.** `Money A2`, `Clearing C4.c`, `Seed C4`, `Spot FX C1`, `Fund Shares E2` re-marked.

**Found and not chased**, in `docs/BUGS.md`: **12b.1-1**, a fund whose whole float is redeemed lives
on for ever as an empty vehicle with a market nobody can be on either side of (XI-3, XI-8);
**12b.1-2**, a share in this world is worth a fraction of a cent, to be positioned with 12c.

**Forecast, with its killer.** The claim is that no price anywhere in this engine can now be off its
market's grid, because there is one way into the book and the print is a posted level. The killer:
if any market ever prints a level that is not a whole number of its ticks, then something reaches the
price store without passing the book — and the candidates are named, a seed's stated level and a
derived value, both of which this item decided deliberately.

---

## 12c — The equity book walks away

**The finding, and the cause.** `equity.firm.13` printed `100.00, 99.91, 99.76, 99.73, 127.13,
9613.36` and then stopped trading, every one of those a session with real settled volume. Nothing was
wrong with the solver. Both sides of a dealer's quote come from that dealer's own view, its view
follows the last print, and a book with nothing in it but desks is a fixed point (XI-13). What was
missing was a party whose reason to be there was not the last print.

**And the party was already in the register.** 11.5 put the float in the savers' hands; what they
lacked was a reason. The saver's share valuation was what the issuer had PAID it (`payout.declared`),
and TWO firms in forty declared a payout — so thirty-eight listed lines had nobody in them with a
view, and `market.noView` said so every period.

**The reason, and it is the one §48 made possible.** A share is a claim on the residual (Equity A1),
and since 12a every public company PUBLISHES what its residual is and what it earns on it, on its own
fiscal calendar, to everybody at once. So a saver prices the claim at

    the book it is a piece of  +  what it earns a year / what this cell requires

— two figures the company itself published, and one thing that is the cell's own: what it requires of
a claim that promises nothing, which is its liquidity preference plus how wrong its own income has
recently been (§46 B3). Two cells therefore want different prices for one firm, and that disagreement
is what gives the book two sides (§46 A3). Nothing is discounted, nothing is forecast, no multiple is
imposed, and there is NO BOUND anywhere: a company far enough under water simply has no bid, which is
the absence of a reason and not a floor. 11.3 was thrown away for writing one.

**Deliberately not consulted: the analysts.** A bank's estimate of what a company will earn is a
different object with a different owner (`research.estimate`), and a saver reading it would be one
more party with no reason of its own — which is exactly what Reporting E2 forbids and what
`check:forbids` enforces over the source.

**Measured.** Three worlds, a year each. `probe`: the two listed lines swing 1.18x and 1.23x over the
last twenty periods against 96x in five sessions before. `year`: 1.23x to 3.33x. `anchor`: flat. And
the print tracks the published residual — `firm.35` printed 71 against a published book per share of
71.71, a ratio of 0.99 — which is what `equity-anchor.test.ts` asserts, at a factor of two either way
that is "the same size of number" rather than a band anybody tuned.

**A second structural fix: whether a party has a view belongs to the PARTICIPANT, not the kind.**
`market.noView` counted `PartyKindProfile.speculative`, and a kind is the wrong owner — a bank's
dealing desk has a view and the same bank's treasury funding itself does not, and they are one party
of one kind. It moved to `ParticipantDecl`, where the posting is declared. `noView` on the equity
books fell from 84 events in a year to 36, and every one that remains is a line whose issuer has
never published anything — which is honest, because a company with no accounts cannot be valued by
anybody except off the last print.

**And 12a-1 is fixed here, as its positioning said it would be.** The equity index read a level its
own prints did not make — a step at one period, carried unchanged. The cause was WHEN each reader
took the step, not how: a level is a chain, and a basket is NOT a function of the period alone (a
constituent's shares outstanding is the count there is now, and a delisting takes a company out of
baskets it used to be in). The tracker fund asked during the markets phase, so the kernel chained
period t with the companies alive mid-period while the audit's independent reader chained it at the
close with the ones still alive — and the two parted by exactly that step. The step is now taken
ONCE, at the close of the period, for every index; a reader inside a period gets the last level that
is finished, which is the rule every other read here follows. Nothing is stored that anybody can read
(Appendix B stands); what is kept is each reader's own place in its own walk.

**Coverage.** `Equity B3`, `Indices A2`, `Indices E3` re-marked.

**Found and not chased.** `docs/BUGS.md` **12c-1**: a ceased issuer's share line stays live and keeps
printing — `equity.firm.13` has `issued === 0` and a market that still prints 11 every period with
only desks in it. XI-8 says no death without a destination, and a company that has ceased has no
residual for a share to be a claim on.

**Forecast, with its killer.** The claim is that a listed line is now anchored by a party whose level
comes from the issuer's own published accounts rather than from the book, so it cannot run away from
what the company is worth without the company's own reports running away first. The killer: if a
line's print parts from its published book per share by an order of magnitude while that company goes
on reporting normally, then the saver is not the marginal buyer and the desks are still pricing each
other — and the next place to look is how much of the float a cell can actually bid for.

---

## 12c.1 — The suite that got slower every period

**Inserted where the cost growth was observed** (Law 18), between 12c and 12d, because 12d is 168
test migrations and an eleven-minute loop is not a loop.

**643 seconds → 331, with 168 failed and 257 passed before and after — the same tests, name for
name.** That is Law 18's gate and it was checked as a diff of the failing names, not as a count.

**What the growth was.** Cost per period goes 146ms at period 5 to 1755ms at period 55 — twelve times
dearer, so a run is quadratic in the periods it takes. Three candidates were measured and ruled out
before anything was changed: the journal's reads are indexed (`ofKind` and `lastOf` are map lookups);
the equity ledger walk is 22ms at period 40; and removing `reporting` and `research` entirely leaves
the growth ratio unchanged (2.06x against 2.33x). The CPU profile is flat with 14% in the collector,
which is what a world that is simply BIGGER looks like — and it is:

```
p15  parties 318  holdings 3394  ledger rows 3188      household cells: 16 at the seed
p30  parties 604  holdings 6588  ledger rows 5761                       546 at period 30
p45  parties 902  holdings 10433 ledger rows 9829                       844 at period 45
```

**Nineteen new household cells a period, and `mergeCells` has never once been called.** But merging
would reclaim nothing: at period 45 the 844 cells make 843 distinct (key + state) groups. They are
genuinely different — a member who was hired has been paid and one who was not has not — so the
partition refines with every partial event and never coarsens. That is a MODEL finding about XI-15
(`docs/BUGS.md` 12c.1-1) and Law 18 forbids touching it in a performance item.

**What this item did instead** is stop the suite paying for it. A world is a pure function of (seed,
draw, periods run), so two tests asking for the same one are asking for ONE world. `test/rig.ts`
gained `ranWorld(seed, periods, banks, firms)`, which builds each once and shares it; the three
expensive files asked through it. `research.test.ts` 357s → 65s (it had built the same 60-period
world eight times); `reporting.test.ts` 170s → 75s.

**And the door's rule is the interesting part.** A shared world is for READING. A test that steps it
further, settles into it or assembles it with extra modules is CHANGING it and must build its own —
which the gate caught immediately: converting two tests in `equity.test.ts` that stepped their world
inside the test body turned six failures into seven, and the seventh was a test reading a world an
earlier one had already advanced. They were given their own back. `balance-identity.test.ts` was
converted and then REVERTED for the same reason, before running: every test in it steps the world it
is handed, so no two may share one.

`equity.test.ts` is unchanged at 258s and honestly so: its expensive tests step forty and fifty-two
periods asserting the audit at each one, which is the property they are named for.

**Found and not chased.** `docs/BUGS.md` **12c.1-1**: the cell partition refines every period and
never coarsens, so the representation degenerates towards one cell per person. Either a cell's state
is coarser than the register's or there is a rule for when two nearly-identical groups become one,
and both are modelling decisions with an owner. Positioned when this item closes.

**Forecast, with its killer.** The claim is that every world shared through `ranWorld` is read and
never written, so sharing cannot change an answer. The killer: any test whose result moves when the
file's test order changes is one that mutates a shared world — `vitest --sequence.shuffle` is the
measurement, and it belongs in 12d where the suite is being worked anyway.

## The missing mechanisms are positioned, 12d is closed, and the holding pen is emptied again

**What this is.** Not an item: three documents closed in one change, on the owner's instruction.
`docs/MISSING_MECHANISMS.md` held nine mechanisms this world does not have and said none of them was
scheduled; `docs/plan/12d-tests-catch-up.md` was one step into eight; `docs/BUGS.md` held twenty-nine
findings from 12a, 12b, 12b.1, 12c, 12c.1 and 12d. All three are now empty of anything that is not
placed, and all three are deleted — the treatment a closed item's own file gets, and the only way a
finding or an absence may leave a holding pen.

**Why the nine are scheduled now.** Law 10 makes an insertion the owner's call and the call is taken.
Each of them already argued its own position — "after 13b's classes", "with 13c, where the units
are", "13i, after 13g" — so none needed a new place in the order, and none becomes a new item: a
mechanism lands inside the item whose dependencies it named, written at the level of detail that
item's file is written at. What the fold-in is NOT: a promise that each is small. M1 is a whole
instrument family and M9 is four separate pieces in four places, and the step counts say so (13b 20 →
27, 13f 22 → 28).

| | mechanism | where it landed, and why there |
| --- | --- | --- |
| M1 | options, and a premium that clears | **13b** — after the classes and after the equity book clears (12c), because `D3.a` forbids an underlying that exists only inside the derivative. Its writer, a fund short dispersion, is a step in 13h |
| M2 | the index-linked obligation, step-up, sinking fund, payment holiday, contingent coupon | **13f** — the item that opens the bond's terms, after 12a and after 13c completes the CPI the linker references, before 14 |
| M3 | the central-bank swap line as a facility | **13i** — it cannot precede 13b's `xccy`: a facility that prices off a basis needs the basis to clear first |
| M4 | the physical environment as standing state | **13c**, where the units are; read by 13h when the cover market lands |
| M5 | productivity that improves with cumulative output | **13g** — with the firm's cost base and the entrant that `Firm Birth A5` says can beat an incumbent, upstream of the recipe work at 15 |
| M6 | greenfield investment and the parent–subsidiary group | **13i**, after 13g: it is a birth with a foreign funder, so it needs both |
| M7 | the covered bond | **13e**, as securitisation's on-balance-sheet sibling: the same parts, the other side of the sheet |
| M8 | the deliverable bond future, the net basis, the basis trade | **13b**. This one was written as needing a decision — an item at that position, or a note that it waits. It is neither: `Sovereign I1`, `I2`, `I3` and `I3.a` are SPECIFIED and carried MISSING with no item, and `I1.a` had no COVERAGE row at all. A specified clause no item names is a hole in the plan (Appendix C), so it is owned where the classes are and the row for `I1.a` is added here |
| M9 | the index and fund set | **13b** (equity size split, the global line, a vehicle per index, the default index in two grades) and **13f** (the IG/HY cash credit split; then the leveraged loan index, last, because `A2` says an index reads cleared prices and a loan has no market until 13f opens one) |

**Why 12d is closed at one step of eight, unread.** The owner's instruction, and the file was deleted
without being opened. What the record can say about it is what the findings say: of the ninety reds it
was written to migrate, the ones measured in detail are not migrations at all — ten in
`capital.test.ts` assert a mechanism that is correct and never fires (`12d-1`), six are a deposit
market with one side in it (`12d-11`), three are an FX book that has never had a bid (`12d-15`), four
are a fund whose desks cannot obtain the basket (`12d-8`), and one is a level (`12d-7`). A suite
migrated against those would be asserting agreement with a half-built world, which is the thing
Law 11 and the item's own guard both name. So the reds stay red, each is now named in the item that
builds the mechanism it waits on, and those items re-read them as they land.

**Where each finding went.** Every one is POSITIONED, verbatim where it was measured (a measurement
is what it said; a re-typed one is a claim).

| finding | what it was | where |
| --- | --- | --- |
| 12a-1 | the equity index read a level its own prints did not make | RESOLVED in 12c |
| 12a-2 | every issuer misses payments in every window, so every grade is the worst one | 16 |
| 12b-2 | a value in one money written into an account kept in another | RESOLVED in 12b |
| 12b-3 | no pair has ever traded: every FX session is `noDemand`, for ever | 13i |
| 12b-4 | one opening rate cannot state a consistent triangle, and the seed adds two currencies | 13i |
| 12b-5 | the trading-book check's dust counts its own terms and not the other side's | 13a |
| 12b-6 | a research desk keeps covering a company that has ceased | 13g |
| 12b.1-1 | a fund whose whole float is redeemed lives on as an empty vehicle | 13h |
| 12b.1-2 | a share in this world is worth a fraction of a cent | 16 |
| 12c-1 | a ceased issuer's share line stays live and keeps printing | 13g |
| 12c-2 | the two readers of an index walked the same step at different moments | RESOLVED in 12c |
| 12c.1-1 | the cell partition refines every period and never coarsens | 16 |
| 12d-1 | no firm ever wants plant, so nothing is ever built | 16 |
| 12d-2 | a treasury bill prints at five times what it redeems for | 13h |
| 12d-3 | a loan's rate is derived twice, and the two do not agree | 13f |
| 12d-4 | an exchange-traded fund's own market clears nothing | 13b (every index vehicle would inherit it) |
| 12d-5 | a levy that fails is recorded and then forgotten: no arrears | 14 |
| 12d-6 | twelve firms, eighteen periods, one loan | 16 |
| 12d-7 | a saver has no reason to hold a share until the first published quarter | 16 |
| 12d-8 | an exchange-traded fund whose desks cannot create | 13f (the borrow market is how a desk gets the basket) |
| 12d-9 | no listed firm is ever short, so Equity D1 never fires | 16 |
| 12d-10 | a line's makers are drawn and nobody reads them | 13b (before anything hedges off the book) |
| 12d-11 | every bank posts the same deposit board, so no depositor ever moves | 13f (a bank with cheaper paper does not match) |
| 12d-12 | a desk's limit is measured against its bank's liquidity portfolio | 13b |
| 12d-13 | the deposit guarantee can only pay for a loss no payment can cause | 13h (the loop is what makes a valuation loss reachable) |
| 12d-14 | a bank lends a money it has not funded | 13b (the FX swap that funds a foreign book) |
| 12d-15 | four countries, one population: no FX book ever has a bid | 13i |
| 12d-16 | the cell grain moves the money by more than whole people are worth | 16 |
| 12d-17 | the bank display names run past Z into punctuation | 13i |
| 12d-18 | the countries are the one population that was typed, not drawn | 13i |
| 12d-19 | `region` is a declared cell key dimension with one value | 13i |
| 12d-20 | `North Asset Management` manages a fund in a world with no north | 13i |
| 12d-21 | a stated balance is charged the dust of its answer, not of its terms | 13a |

**Six comments in the engine and the tests cited the deleted file by finding number** and now cite the
worklist item that owns each one — a stale comment is a defect (Law 16) and a path that does not exist
is the stalest kind. `docs/COVERAGE.md` gained the `Sovereign I1.a` row it never had. The manifest
gained ten step recounts (13a 16→17, 13b 20→27, 13c 14→16, 13e 18→20, 13f 22→28, 13g 13→15, 13h 19→22,
13i 11→17, 14 12→13, 16 13→14) and `docs/WORKLIST.md` says what each item now carries.

**Forecast, with its killer.** The claim in the fold-in is that none of the nine needed a new worklist
position — that every one of them is a mechanism inside an item that was already going to be worked,
rather than an item of its own. The killer is the work: if any of them cannot be built inside its
host item as one bounded change (Law 14), it is an item and the record of that item says so when it
is split out.

## 13a — The derivative layer

> **CLOSED ON A WORLD THAT DID NOT ASSEMBLE, OR ON A MECHANISM FOUND UNREACHABLE.** Item 0 stepped
> the assembled world for the first time and it stopped in period 2; `docs/IMPLEMENTATION.md` Part 0
> names this module among those that compile, are cited, are marked MET, and have never produced an
> outcome. Every measurement in this entry was taken on a world that could not run to it. The
> mechanism may be right and the numbers are not evidence. Re-verified at item 0d; what the
> capabilities have actually produced is `npm run coverage:reached`.


**What.** The kernel gained a SECOND REGISTER (`register/contracts.ts`) and the module that runs on
it (`mechanisms/derivative-layer/`). A contract is not a holding (Derivative X1): nobody issued it,
it has no issued amount, and it enters no ownership check — what it enters is the identity that the
marks across its two sides come to nothing, exactly (D1.b), which is the `zeroSum` family, built
here. Around that: a `DerivativeKindProfile` registered like an instrument kind's; a `contract`
market kind whose fills become rows; margin as a claim that is held rather than spent; a clearing
house that is a party with a balance sheet, a default fund sized cover-one and a waterfall it can
run past the end of; capacity that cuts a trade at the strike to what its two members can margin;
and a resolution slot where a term that ran out, a call that went unmet and a side that ceased are
three ways to the same close-out.

**Four decisions worth the record.**

1. **Open, close and novate are LEGS, not doors.** The item file said doors on `MechanismContext`.
   Every one of them is a change of two balance sheets — a row is an asset to one side and a
   liability to the other from the instant it exists (D1) — and Money D1 says a change of balance
   sheet goes over the wire. So `ContractLeg` joins `AssumeLeg` and `PledgeLeg` as a leg that moves
   an obligation rather than a holding, settlement is the contract store's one writer as it is the
   register's, and a premium and the row it buys are in the same numbered instruction (Law 5).

2. **The zero-sum family asks the PROFILE twice.** `b`'s value is `−a`'s, so comparing those two
   would be checking that a minus sign works. What is compared is the profile's own answer for the
   contract AS EACH SIDE STATES IT: `flip` gives the terms as the other side wrote them (a forward's
   direction, a swap's payer and receiver), the profile is asked again, and the two must negate
   EXACTLY — D1.b says exactly, and it is the one check in this tree with no tolerance in it at all.
   A kind whose mark is not antisymmetric in its own terms lights this family and no other, which is
   what Audit B8 asks of one.

3. **Margin is held, not consumed, and that is why the balance sheet still adds up** (D3, C3.a).
   Posting is an asset swap — money out, a claim on the holder in, one instruction, two legs — so a
   member that has posted margin has not spent anything and the variation margin is not a second
   flow bolted onto the mark: it is the same requirement per counterparty PAIR (C1.a), re-measured
   after the marks moved. There is no door anywhere that nets across counterparties (G3).

4. **A kind that cannot say what a position could do answers Missing** (G2, D1). Initial margin is
   the underlying's own measured move over its own prints, and a line that has printed once has not
   moved: there is no honest number, and G2 admits an exposure with no margin only with a STATED
   reason. "Nobody can say" is not one, so the trade is refused at the strike and the refusal is
   journaled (E2, E4). The alternative — admitting it at nothing — is E4's limit raised by omission.

**Also in this change, because 13a is the item that carries them:**

- **`12d-21`, a stated balance charged the dust of its answer.** `opened(0)` charged a fund's equity
  ε × 0 when that zero was a contribution of 4e11 minus a book of 4e11, and the seed audit reported
  the real residue of the subtraction as a violation. `openedFrom` takes the TERMS, and
  `stateEquity` now takes the dust its arithmetic earned; nothing is widened — the walk starts where
  its own arithmetic left it. It belongs here because D1.b is an exact identity and a family whose
  subject is exactness cannot sit on a door that mis-states what one costs.
- **`12b-5`, the trading-book check's dust counting its own terms and not the other side's.** The
  `bank.capital` event publishes the count and the magnitude its own sum had, and the check derives
  the tolerance from both walks. The 2^-12 gap on 5.0e10 was the missing half.

**Found on the way.** `carryingOfContract` first read `c.opened >= recognised`, which carried every
contract at its basis for ever and re-booked the whole mark each period; the accounts family put it
at 48 on each side of one pair, equal and opposite, within two periods of the first trade. It is the
same `>` a lot's `acquired < now` already had (`prices/value.ts`) and is now written the same way.

**And three more, found by the tests that close the item.**

- **A world whose clearing members are all banks has no member that can miss a call.** A bank pays
  what it owes by issuing its own deposit (Money A1), so there is no sum it cannot post and D2.c's
  cash test can never fire. The layer's tests bring two parties of their own — not banks, not money
  issuers, holding what they trade — and one of them duly fails a call, is closed out at its stated
  value and lands on the house. It is a fact about the model worth writing down: the cash test is
  real for everybody except the parties that make the cash.
- **A close-out claim smaller than one piece of its money threw.** `issueCloseOutClaim` sized the
  leg with `cashFor` after deciding to issue it, so a residue of half a cent became an asset leg of
  zero units and `Register C1` stopped the world. Law 8 says what it is: a claim is a count of
  pieces, and a residue below one piece is the dust of the close-out's own subtraction, not a claim
  anybody could be paid. Rounded before the instrument exists.
- **A contract market kept clearing into a house that had ceased.** The house ran past its waterfall
  and failed — C5 and XI-3 working, unprompted, in a scale model — and its book outlived it, so the
  next session wrote fills naming a party that no longer existed (Money E4 caught it at settlement).
  The capacity door now refuses the whole size when the named house is not alive, which is E4's
  "refused by name" and E4's measurement in one place.

**Carried forward, positioned in 13b.** A requirement below one piece of cash posts nothing and the
layer does not say so, where G2 allows an unmargined exposure only with a stated reason (`13a-1`);
and securities as collateral — the lien half of D9 — has no user until a class pledges a holding,
so the kernel's `pledge` door is built and unexercised (`13a-2`).

**Forecast, with its killer.** The claim is that the layer is complete enough that a class is a
MODULE and nothing in the kernel changes for it: 13b adds four kinds, a rate-quoted book and a
credit event, and if any of them needs a new leg, a new store or a new door, this forecast is wrong
and 13b's record says which.

## During 13b — an independent review of the code, and where its findings landed

**What.** A full reading of `packages/engine/src` — 160 files, 43,342 lines — against the laws in
`CLAUDE.md` rather than against the plan, asked for as an independent review: rule compliance,
whether the modelling is real rather than asserted, whether money is created and destroyed only in
the right places, and whether a module can be added without touching a hundred files. 83 findings
were written to `docs/REVIEW.md` as they were found, file by file. They are now positioned and the
file is deleted, because a finding leaves a holding pen only by being placed.

**Three answers, because they were the questions asked.**

- **Money is created and destroyed only in the right places.** Checked exhaustively. Two ops — `issue`
  and `redeem` in `ledger/settlement.ts` — reachable from `expandMoney` and `expandAsset` and nowhere
  else. Six module sites: a bank lending (its own deposit into the borrower's account, no reserve
  leaving, B1.a), a drawing on a line, the central bank remitting income in reserves it issues, a
  bail-in extinguishing a depositor's claim, and an acquirer taking over an account — the failed
  bank's deposit destroyed and the acquirer's created in **one instruction, two legs**. The hard case
  falls out of the wire rather than being coded anywhere: a loan repayment crosses issuers, so
  settlement redeems the borrower's deposit, credits the bank's reserves and debits them again, and
  the deposit is destroyed with no reserve moved. `adjustIssued` has three call sites, all inside
  settlement.
- **The modelling is honest, with three asserted numbers.** The estate's `print × left/(left+1)`
  reservation (a written price path in the module that forbids one); consumption as a fixed share of
  MONEY spending (unit-elastic demand, which `phoenix/no-value-recipe` refuses on the production
  side); and the own-credit gain — a firm's equity RISES as its own bonds fall, which fights XI-3
  because a firm cannot become insolvent if its own distress is a revenue line. All three positioned.
- **The architecture is plug-and-play for INSTANCES and costly for SHAPES**, measured on the three
  commits that built item 13: 13a, a new shape of state, cost **24 kernel files to 7 module files**;
  13b, seven classes of that shape, cost **6 to 24**. Most of the difference is irreducible — new
  state means a store, a leg, an op, a value and an audit family — and four causes were avoidable.

**What the review actually found, and it was not what was expected.** The laws are followed, largely
out of discipline. **The machinery built so that discipline would not be needed is the weakest part
of the tree.** `phoenix/no-cross-module-import` strips `../` and then tests for a leading
`mechanisms/`, so the only spelling it catches is the one nobody writes: it has never reported
anything, and six modules import each other — `firms`, `households`, `capital-programme` and
`treasury` all reach into `goods` for values, `banks` reads a parameter constant out of
`spot-fx/data.ts`, and `banks ↔ spot-fx` is a cycle, which is the shape that left a constant
undefined at module-init during this same item. ARCHITECTURE 4.9b's central claim rested on that
parenthesis. `phoenix/no-bounds` forbids `Math.min`/`Math.max` and the engine writes the same
operation as a ternary thirty-one times, most of them correctly. `Audit.run` marks a family built
when ANY contribution is — so `crossMarket` reads green while its kernel contribution is an unbuilt
stub that vanishes from the report. The flows family exempts a party's whole position for a period
because one weight event named it. And `register/register.ts` still carries the floating-point
tolerance Law 8 retired, including a branch that deletes a positive holding with no instruction and
no counterparty.

Two live defects were found by reading a file against its own header: `Instruments.restate` does not
invalidate the `all()` cache, so a share split leaves a stale `issued` in front of every reader for
the rest of the run; and `Contracts.open` validates neither its terms nor their kind, so a mis-kinded
row marks zero on both books and passes the zero-sum family forever.

**Two further readings, asked for after the first.** How a party names a level: the older half of the
engine is fair-value-first — a dealing desk prices a dated claim off its own required yield, a saver
off published accounts, a firm off its own book — and the seven derivative classes written in 13b are
print-first, so a party with no outlook posts AT the last print and the outlook that would move it
was formed from the prints. `banks/dealing-quote.ts` had already diagnosed that fixed point, cured it,
and written the incident down. And how clever a party is: the engine differentiates parties by what
they see, hold, owe and prefer, and in no way by what they can WORK OUT — the same household cell
prices bread off its own adaptive outlook and prices a bond by discounting its cash flows. The
consequence that matters is that XI-2 has no retail door: a household never sells because a price
fell, so a market shock produces no redemption wave.

**Where each landed.**

| finding | item |
| --- | --- |
| the guards, lint rules and audit families that do not fire; the Law 8 dust; the store guards; `Qty` on the wire; the declarations that are not true (`loan.operatingCost`, `ParamDecl.unit`, `cellKey`); the four avoidable kernel seams | **13b.1**, inserted |
| every class quotes the book's own print and the option book cannot open; `parLevel`; the contract store's missing validation; the layer's double-counted capacity | 13b |
| a second final good waits for 13d | 13c |
| the household that runs analyst's models; XI-2's retail door; consumption as a share of money; wealth as a key dimension; a bank that employs nobody, and `loan.operatingCost`'s death | 13d |
| the own-credit gain — a decision reserved to the owner | 13f |
| the estate's formula discount off book; an asset manager that decides nothing | 13h |
| what a module may extend and what is closed to it | ARCHITECTURE.md §4.9b |

**One item inserted: 13b.1**, after 13b because Law 10 finishes the open item first and two findings
are 13b's own, and before 13c because everything from there on adds modules and the rule meant to
keep them apart is the thing that does not work. Most of it removes code.

**Forecast, with its killer.** The claim is that 13b.1 changes no mechanism, no economic outcome and
no boundary (Law 18): the year-long run's audit totals and money stock should be identical before and
after. If they are not, the difference is a defect the item exposed — it goes in `docs/BUGS.md` and
13b.1's record says what it was, rather than being absorbed into the change.

**One thing worth writing down about the method.** Almost every finding was found by reading what a
file SAYS it does against what it does, and four of them are cases where a file states the rule in
its own header and the code two hundred lines down does not follow it — `core/tick.ts`'s "a leg's
amount", `register/register.ts`'s "guarding the STORE rather than each writer", `estate`'s "a formula
discount off book", `parties`'s "lifting a relationship into the key is a data change". In a codebase
with ordinary comments none of those would be findable at all. The prose is load-bearing.

---

## 13b — the derivative classes

> **CLOSED ON A WORLD THAT DID NOT ASSEMBLE, OR ON A MECHANISM FOUND UNREACHABLE.** Item 0 stepped
> the assembled world for the first time and it stopped in period 2; `docs/IMPLEMENTATION.md` Part 0
> names this module among those that compile, are cited, are marked MET, and have never produced an
> outcome. Every measurement in this entry was taken on a world that could not run to it. The
> mechanism may be right and the numbers are not evidence. Re-verified at item 0d; what the
> capabilities have actually produced is `npm run coverage:reached`.


**What the item was.** Five bilateral classes on 13a's layer (credit default swap, interest-rate
swap, FX forward, cross-currency swap, bond future), an option, an index future, the index set they
measure against, and the reads a reader is shown of all of it.

**What the classes are.** Each is a module: a `DerivativeKindProfile` with its own terms, legs,
mark, margin, close-out and expiry, and its own `orders` — the reasons a party has to be in a book
of that kind, asked of the kind by the layer that owns contract books. No class imports another, no
class touches the house, the margin or the waterfall, and adding one is a module and a worklist
item. That was 13a's forecast and this item is its first test; it held.

**What the pass changed about how a class names a price, and it is the item's own finding.** Every
class began by reading its own book's last print and falling back to something of its own. That is
XI-13's fixed point written five times: the print moves the outlook, the outlook moves the view, the
view moves the quote, the quote moves the print — so a book whose members had nothing of their own
to say printed one number for ever, which is what `banks/dealing-quote.ts` had already been fixed
for. Four of the five were turned round: the credit default swap reads the reference's own cash
bond, the swap the overnight fixing, the bond future the cash deliverable, the FX forward its own
carry — each a read of ANOTHER market, so a party has a level whether or not this book has ever
traded, and the book's own print is kept only as the comparator that decides which side it is on.
The cross-currency swap could not be turned round and is positioned (`13b-4`, to 13f): what it
quotes is a basis, a residual whose parity value is zero, and naming one needs a borrower's own
funding cost per money, which this world does not have yet.

**What a reader is shown.** Every contract book's own level with the word for what that level IS
(money per unit, or a rate per annum), those levels gathered by subject into curves in tenor order
with no point between two points, the hedged residual per party and per pair beside what the rate
actually did — and the bases. The bases are the part worth recording: **the surface does not know
what a basis is.** `DerivativeKindProfile.measures` is the `orders` pattern applied to measurement,
so the credit basis and the net notional, the swap spread, the net basis and the implied move each
live beside the class that knows them, and the observer walks the books and asks. A world with one
more class shows one more measurement without a line of the surface being touched.

**Two kernel reads were added, and each removes a seam rather than adding one.** `WorldReads` is the
read half of a context — a derived read is a function of state, so it asks for the doors it reads
through, and the surface that exists precisely because looking changes nothing no longer holds
`settle`, `post`, `issue` and `record`. `sovereignCurveIn(ccy)` answers which curve a spread in a
money is a spread over, off two things this world already declares (its curve families, and which
party kinds borrow on a state's credit), so a class asks instead of being handed `treasury.us` at
assembly — which is right in one world and wrong in the next. `bondFutures` still takes one and that
is now visible.

**Two defects the item closed, and the second was hidden by the first.**

`13b-5`: a desk's book is what it is holding AWAY from where its own treasury wants it, and
`bookValue` counted the distance EITHER WAY — so a bank holding less of a line than its treasury
asked for was read as a desk carrying the shortfall, and every desk in every seed opened past its
own aggregate limit before it had quoted. One holding, two owners inside one bank, and the
treasury's claim is senior: of what the bank holds of a line, the treasury has asked for `want` and
what is left over is the desk's.

`13b-7`: fixing that took the mask off the allocation underneath it. The treasury shares its
headroom between its lines of business in the order of what each earned — and the lending line's ask
was `left`, which is *everything there is*. An ask of everything is not an ask: whichever line was
served first took the whole headroom, the sort that was meant to choose between them chose nothing,
and the dealing line was allotted **zero in every bank in every period of the world**. Its desk's
limit is then exactly the book it already has, so a desk that starts empty can never open one, never
earns, and never outranks lending — the starvation sealed itself. It was invisible because
`bookValue` was enormous: `min(carried + 0, appetite)` returned the appetite, and the desk's limit
came out right for entirely the wrong reason.

The fix removes code and a Law 15 violation with it: the branch that did it was `r.line === DEALING`,
a branch on a line's id. Every line now asks the same way — the distance between its own appetite
and what it is already using — and a line's appetite is DATA, drawn per bank per line
(`BANK_SPREAD.appetite`), registered as a parameter per bank per line (`lineParam`). Adding a line of
business is a name and a spread, and no loop learns what the lines are called. Measured: the dealing
lines went from `room 0` in all three banks to 5.2bn, 497m and 2.9bn; the overnight book, which had
not cleared at all, cleared; the benchmark it publishes appeared; and the swap books that gate on it
opened. The banks are redrawn, because a world with one more drawn number is a different world.

**What is measured and NOT fixed, and it is the largest thing this item found.** `13b-10`: no
contract book in this world has two sides. Forty books open in the classes rig and not one contract
is written in sixteen periods. It is not the levels — the bond future's three banks name three
different prices, so their outlooks do disagree, which is what §46 A3 asks. It is that every party
the layer admits to a contract book is a bank or a firm, and in this world both want the same thing:
banks are long duration and short of fixed, firms are short of foreign money. Asking every party
rather than the eligible ones, the bond-future books have forty-two to fifty-two willing schedules —
the households and the money funds have reasons and are shut out by one line. Letting the households
in would be the wrong fix (Law 1): a household does not sell a bond future, it owns a fund that does.
The missing party is the one whose business is holding the market and taking the other side of
somebody's hedge, and it is 13h's. Positioned there with `13b-2` and `13b-6`, which are the same
absence measured in two other places.

So the two checks that assert a contract book prints are Law 11 checks against an incomplete model.
They now assert what a reader is SHOWN about each level there is, and never that there is one, and
they say in the test why.

**A third defect stopped a run and was fixed where it stood.** `13b-12`: at period 23 of thirty,
`[Clearing A2] bank.a is on both sides of mkt.ust.bill.2026-09-15 at crossing prices` — a desk
quoting a bid above its own offer. Behind it: the bank had published `capital −31,237,415,456`, and
`costOfFunds` was blending that hole in as a source of funds with a required return its owners
wanted on it. The blend came out at −0.0894 a year, a negative cost of funds is a negative carry, a
negative carry is a negative half-spread, and the bid crossed the offer. A hole is not a source of
funds: capital is the residual, a negative residual is a real state that stays real — still
published, still audited, still read by the resolution trigger — but it funds nothing and there is
nothing there to require a return on. What funds the book is what it owes plus the capital there IS.
The run completes. What is NOT fixed is why a bank thirty-one billion insolvent was still making a
market twenty-three periods in, and that goes to 13f with `bank-resolution.test.ts`'s seven reds.

**Findings positioned out of this item, and every one of them PLACED.** `docs/BUGS.md` is deleted,
which is what the protocol says happens when it empties. `13b-1` (a bank overdrawn at the central
bank with nothing lent to it) mostly closed — a treasury can now see what its own contracts will
take out of its account next period (`OwnContracts.cashDue`) — and the remainder to **13f**.
`13b-2` (a fund's equity is a millionth instead of nothing), `13b-6` (the trackers launch nothing,
because nobody can assemble a basket), `13b-9`'s remainder (a fund whose shares are marked at
nothing while shares are outstanding) and `13b-11` (the seeded tracker publishes no launch, so which
vehicle is on which rule has two answers and one of them is silent) to **13h**, folded into two
steps there with `13b-10`. `13b-3` (five equity tests moved when the world got richer and nobody has
measured which change moved them) to **16**, because it is a measurement and not a mechanism.
`13b-4` (the cross-currency basis, and an order book that treats a level at or below zero as the
absence of a price), `13b-8` (the treasury shares out one pool and its lines ask for it in a
different quantity — capital, risk-weighted assets and units of an asset in one arithmetic) and
`13b-12`'s remainder to **13f**. `13b-5`, `13b-7` and `13b-9`'s throw closed here.

**What the suite says, and it is not green.** At `01204e7`, before this item's engine work: **25
failed, 447 passed of 472**. After it: **37 failed, 453 passed of 490** — eighteen more tests run,
six more passing, five of the old reds gone (`credit-events`'s two year-long runs, `funds`'s load
failure, `opening-liquidity`'s desk limit, `resolution/cells`' three-grain check) and seventeen new.

Every one of the seventeen is accounted for and none is a mechanism silently broken:

- **Eleven are one world, redrawn.** A line of business now has a drawn appetite, so `drawBanks`
  takes one more number from the stream and every bank after it is a different bank (`13b-7`).
  `research`, `money-market`, `households`, `treasury`, `raise`, `bank-capital`, `tick`,
  `bank-resolution` and `equity-anchor` each assert something about a world that is no longer the
  world they were written against. A test that names what a draw made is the thing CLAUDE.md says a
  test must not do, and each of these is a candidate for that reading when the item that owns its
  mechanism reaches it.
- **Four are `13b-2`/`13b-6`**, the funds: an exchange-traded fund whose creation needs a party that
  can assemble a basket, and there is none. Positioned to 13h and placed in its file.
- **One is `13b-1`'s remainder**, `resolution/cells` reporting the overdrawn bank in its own trimmed
  world. Positioned to 13f.
- **One is `indices`' pre-existing red**, which was red at `01204e7` too and is not this item's.

Nothing was rolled back, no tolerance was widened, and no test was deleted or weakened to pass. Two
checks were re-stated rather than deleted, and both say in the test why: the swap book test waited a
fixed three periods for something that is gated on the overnight market having traded, and the two
"a contract book prints" checks now assert what a reader is SHOWN about each level there is, because
`13b-10` says there is not one.

## 13b.1 — the checks that do not check

**What.** The guards, the lint rules and the audit families written so the laws would not depend on
anybody remembering them — and which did not fire. Nothing here changes a mechanism, an economic
outcome, a price or a boundary (the item's own guard); every change makes a rule that was written
down into a rule that is enforced, and the diff is net negative in the places that matter.

**The boundary.** `phoenix/no-cross-module-import` stripped `../` from a specifier and tested for a
prefix, so it had never reported anything. It now resolves the import against the importing file's
directory, and it fired on all fifteen crossings in six modules. The crossings went with it: a
PHYSICAL LINE (`registry/physical.ts`) and a SOVEREIGN CLAIM (`registry/claims.ts`) are kernel data,
because four modules have to name a good and two have to name a bill — the same decision §4.9b took
for party kind ids. The FX desk draws its own numbers, so `BankDecl` loses three fields and the
`banks ↔ spot-fx` cycle goes in both directions. `tools/test/eslint-rules.test.ts` is the fixture
suite the rules never had.

**The bounds.** `atMost` and `atLeast` in `core/num.ts`, generic over the `Qty` brand, with the
REASON as the third argument — the thing that is not there. `no-bounds` now refuses the
comparison-ternary shape as well as `Math.min`/`Math.max`. The item expected thirty-one sites and
found **fifty-three**: the rule as written caught `a < b ? a : b`, and once it was running it also
caught `y > 0 ? y : 0`, which is "not less than zero" — the spelling Law 6 names in so many words —
in seventeen more places nobody had counted.

**The dust Law 8 retired.** `deliveryDust` and every use of it, including the lot-clearing branch
that destroyed units with no instruction behind them; `deliverable`'s second clause; `debit`'s two
`remaining <= dust` lines; `moneyDelta`'s Money B3.c guard **and its `allowNegative` parameter**,
which settlement passed `true` at both money sites unconditionally; `precheck`'s money tolerance;
`validateCellSide`'s comparison; the money family's B3.c walk. About forty lines, and with them the
one branch in the engine that could delete units.

**`Qty` is on the wire.** All eight fields `core/tick.ts` names, plus `Lien.qty`, `DrawnLot.qty`,
the settlement `Op` union, `Trade.qty`, `Pledged.qty`, a fund's `sharesPerMember`, a money-market
row's amount and an estate claim's units. About a hundred and ten sites followed the compiler, and
each made a decision the type forced into the open: a premium is money and lands on the money grid;
what the layer admits is money over a margin requirement and lands on the contract below; a
per-member share THROWS if the division did not come out whole. `-x` on a count is now a lint error.

**The audit that says what it does not check.** `built` means every contribution is built, and a
family nobody contributes to is not built either; `unbuilt` is reported beside `contributions` with
the names. `crossMarket`'s kernel stub was DELETED rather than built — Audit B4 is about two venues
for one economic thing and the kernel has no example of one. The flows family exempts THE ONE CELL
whose book really did appear or vanish, named on the split or merge event itself, and nothing else.

**A declared number says what it is in.** `ParamDecl.dimension` is a closed vocabulary of nine and
`ParamRegister.get` is gone, replaced by nine typed reads that each name what the caller expects; a
read naming a different dimension throws with both quoted. `unit` stays, because it is the sentence
a human reads and the dimension is the part a machine can check. 134 declarations, 176 read sites.

**A cell is keyed on what the registry declares.** XI-15 had already taken the decision — "at any
time the cell key is what the registry declares it to be, and lifting a relationship from a row into
the key is a data change ... never a change to a mechanism" — and the kernel was not reading the
list: `CellKey` named three dimensions and `Parties.add` checked all three unconditionally, so 13d's
wealth dimension was a kernel type change. `CellKey` is now the map of the declared dimensions,
`keyOf` its one reader, and `KEY_DIMENSIONS` the single table saying what a dimension MEANS — where
the same fact also lives on the party so the copies must agree, and what the value must name. A
dimension with neither is the key's own, and having nowhere else to live is precisely why it is a
dimension. `cellKeyFaults` is one writer of the rule with two readers at opposite postures: the door
throws, the names family reports.

**The four seams** (the measured causes of kernel churn: 13a cost 24 kernel files to 7 module files).
`ModuleKey<B>` lets a module brand its own identifier in its own file — five of those 24 were
`core/ids.ts` growing a brand per module. `MarketDecl` is a discriminated union, so both runtime
throws are deleted and `delivers` is a row in the kind table instead of testing two optional bags for
absence; 91 sites moved onto three kernel reads so that no module writes the discriminant test.
`admits`, `marginLegs` and `derivativeKind` moved into the contract kind's own deps row.
`DerivativeKindProfile.orders` and `.measures` became `DerivativeClassDecl`, collected at assembly —
`registry/` now imports no `world/` and no `clearing/` anywhere. And `SeedContext` hands out
facades, where it had held the write-capable stores themselves.

**The doors that let something through.** `quotedAs` is required, where "absent means money" was a
default wearing a type. `contractValueTo` throws for a party on neither side, where it returned 0 —
a real answer to a different question. `percent` trims trailing zeros only after the decimal point,
so `percent(0)` is "0%" and not "%", which had been naming a zero-coupon line "US Treasury %
2027-03-01". `moneyInstrumentId`, `currencyUnit` and `fxPairId` refuse a part carrying the separator
their id is built from. `couponsPaid` loses its `readonly` cast and the -1 sentinel it wrote through.
`PriceStore`'s three linear scans are one binary search, exact because `write` already refuses a
print that is not strictly after the last, and held to the walk's answer by a property test.

**The one question settled, and Clearing A2.a had settled it.** "A market expressed as 'here is the
quantity I want' has no level, only a shape, and forces every venue to invent its own rule."
`resolveMarketOrders` IS that invented rule and now says so where it lives. It stays as it is — the
worst level the other side actually posted — because the level taken is then one somebody really
asked for, so the price still comes out of posted supply meeting posted demand (Law 3); resolving to
the best level would turn an order with no limit into a price-sensitive one, which is a different
order. `Clearing A2` and `A4` are re-marked PARTIAL for what that costs.

**Two steps of this item were wrong on the arriving world's terms and both were changed** (PLAN
§5.2). `namesAnItem` on every kind refused the world at assembly: eleven policies and preferences
legitimately cite a future item, because what changes is who sets them. It asks a SHAPE and a
TECHNOLOGY instead — a technology is a fact about the world and a fact about the world has no
scheduled death. And `worthOf` and `markPerUnit` do not answer the same question: the first is "what
is this worth and HOW OLD is that", which is what makes a stale mark visibly stale; the second is
"what is a unit marked at NOW", asked inside a period whose phases are ordered, where a print that
is not there yet means a phase in the wrong place. Merging them turned the seed's own valuation into
a throw.

**Found, and where each went.** Twelve findings. Two were fixed where they were because they stop
the build (`13b.1-1`, a fund redeeming shares whose cash rounds to nothing, refused at the wire; and
`13b.1-3`, two tests naming an outcome of the draw). `13b.1-4` is the §5.2 note above. The rest are
positioned: **13h** takes `13b.1-5` (a resolution bid charging a year of required return for one
week — found by the dimension vocabulary on its first world), `13b.1-8` (the clearing house is not
flat, by nine hundredths, where C2 says it is nothing by construction), `13b.1-9` (not one party
posts into any option book, which is `13b-10` one class over and stronger) and `13b.1-1`'s open
half. **14** takes the central bank's market order. **13i** takes `13b.1-7` (no spot FX trade happens
at all: the venue is there and nobody has a reason). **16** takes `13b.1-2` (two publications of one
bank's book compared across two different moments — the gap is 2^-19, two thousand ulps, and the
derived dust is right to refuse it), `13b.1-6` (a mean-preserving spread moves what the household
sector decides by nineteen pieces a member), `13b.1-10`'s other eight posters, `13b.1-11` (an index
publishes a level it cannot then answer for) and `13b.1-12` (only some of the three deposit classes
ever moves bank). `docs/BUGS.md` is empty and gone.

**What the suite says, and it is not green.** Measured at the 13b close commit `9a22e76`: **37
failed, 453 passed of 490** — the figure that entry recorded, reproduced. At this item's close:
**47 failed, 458 passed of 505**. Fifteen more tests run, five more passing, twenty newly red and
eleven of the old reds gone. Every one of the twenty is accounted for.

- **Six are ONE FINDING, and it is the largest single cause of red in the suite.** A fund's share is
  worth nearly nothing, so every quantity computed by dividing money by it explodes — and the guards
  this item put on the rounding doors turned that from a wrong number into a refusal at the site. A
  household cell asks for `scaleQty(sharesPerMember, weight)` = **1.33e16 shares**, past the
  safe-integer range; a dealer's creation-unit arithmetic computes `money / nav` = **1.72e16**. The
  first takes both `equity-anchor` tests, the second takes `omo`'s remittance, `raise`'s
  subordinated raise and `resolution/cells`' three-grain check. Law 6 in so many words: a number
  that explodes means the compensating mechanism is missing. It is `13b-9`'s open half, positioned
  to **13h** and restated there with all three sites, because what a fund IS and what stands behind
  its shares is that item's.
- **Four were already found, written down and positioned** during the item: `derivative-layer`'s
  house that is not flat (`13b.1-8`), `derivative-classes`' option book nobody posts into
  (`13b.1-9`), `deposits`' three classes of which not all move (`13b.1-12`), and `bank-capital`,
  which is `13b.1-2` — the risk-weighted-assets comparison across two moments — now firing in the
  file's own setup rather than in one test.
- **Nine are the world, redrawn.** The FX desk draws its own numbers now, so `drawBanks` takes one
  number fewer and `drawFxDesks` one more, and every bank after that point in the stream is a
  different bank. `auction`, `credit-events`, `etf` (twice), `lines`, `money-market`, `reporting`
  (twice) and `run` each assert that something happens in a world that no longer happens in this
  one — "expected 0 to be greater than 0", "expected false to be true", `firm.8 reported with no
  listed line`, `[ 'corporate', 'wholesale' ] to include 'retail'`. A test that names what a draw
  made is the thing CLAUDE.md says a test must not do, and each is a candidate for that reading when
  the item that owns its mechanism reaches it.
- **One is `ratings`**, asking for a coverage measure above zero and getting -3.6e10 — the same
  magnitude family as the first bullet, one module over.
- **One was this item's own and is fixed here**: `world.test.ts`'s placeholder count moved from
  seven to eight, because `loan.operatingCost` is now honestly a placeholder. The test asserts
  eight, with the reason beside it.

Eleven of the old reds went green, and they are the item's own work rather than a redraw: the
window that refuses a bank with nothing to pledge, the corridor moving the market rate, the
acquirer bidding from its own view, the guarantee making the insured whole, `deposits` and
`resolution/cells` loading at all where the whole file had failed, the index reading the same level
as an independent walk, `raise`'s real refusal, `research`'s coverage costing real money, and
`tick`'s structural invariants at every subdivision.

Nothing was rolled back, no tolerance was widened, and no test was deleted or weakened to pass.

## 13c.1 — the map, and the things that move on it

**What.** A drawn world under the economy. Every tile of a rectangular grid carries its own
characteristics — the place it is in, a MIX of terrain shares summing to one, its elevation, and a
deposit of every declared resource — and all of them are drawn, none derived from another. Every
tile belongs to exactly one PLACE, a land region or a sea area, and places group into countries. A
vehicle is somewhere: a voyage is a kernel row with a tile path and a distance travelled, its hulls
held by a lien, advancing each period by what the weather AT THE PLACE IT IS IN allowed.

**Why.** 13c built freight against four legs declared identical (`transitPeriods: 4,
unitsPerVesselPerPeriod: 25000, sailsIn: 4` on every one). Freight A4 says capacity on one route is
not capacity on another; it was one route wearing four labels, so the location basis, "source
locally" and the price indices with freight in them would each have measured nothing. Inserted
before the rest of 13c because the dependency runs one way.

**The decisions.**

*A country has the money; a region is a place.* `RegionDecl` did two jobs and the map needs many
places per currency. `CountryDecl` takes the currency, the central bank and the treasury;
`RegionDecl.ccy` is deleted for `registry.currencyOf(region)`. Measured before taking: the economy
is already written per region — `goodId`, `plantVintageId`, the cell key, `PartyBase.region`, one
labour venue per (region, occupation) — so making regions smaller made all of it per-place with no
renaming. 176 `.region` sites did not move; 80 `.ccy` reads did. §39 Cross-Border and Indices D1 say
"region" and mean currency area; they are re-read as country and named to 13i.

*A tile's characteristics are primitives.* The first draft treated the physical world as something
to minimise — terrain as a thin index, water as a derivation, "declare only the ground a recipe
names". That is a rule applied where it does not bite: Law 2 admits real-world PRIMITIVES and the
physical world is what that clause exists to let in. A mix rather than a type because a
fifty-kilometre square is not one thing, and because the mix removes every threshold from what
follows. Every declared resource is drawn on every tile whether or not a recipe eats it, because
THE MAP MUST BE STABLE: a world that re-draws when a line is added makes no two runs comparable.

*Nothing about a leg is declared.* `RouteDecl` and its four numbers are gone, each naming its read:
transit → a voyage's own progress; capacity → what free hulls hold; sailsIn/sailsHardness → the
ground the voyage is crossing. A leg is every ordered pair of places a kind can get between, boarded
at a PORT — which is not declared either: it is where the ground a hull crosses meets the ground a
lorry does.

*Favourability is a read.* What a hectare yields, what building costs, how fast a voyage crosses and
what survives a gale are arithmetic over the primitives, computed where used. `groundFor` walks a
place's tiles best first and answers with the MARGINAL hectare; past the last tile the decline
continues by the ratio the last two set. So there is no cap on what a place holds, the return from
the next unit falls continuously and never reaches zero, and a rent emerges instead of being
assumed (Law 6).

**Law 8 gained two dimensions.** The map brings length into this world, so `km` and `kmPerDay` join
the closed vocabulary with reads of their own — kept apart for the reason the four durations are.

**Found on the way.** A tile made of 1.0000000000000002 of a tile (a ninth added nine times; the
sub-cells are counted as integers and divided once). Land the country seeds could not reach (a place
is ONE PIECE, so an archipelago has one per island). The first country taking the whole continent
(seeds now spread over the land WORTH a country). Picking a place by its best tile putting a farm on
twelve tiles of rock (`groundIn` sums; a sum, never a mean). A farm standing on a thousand times its
land (the register counts pieces and `landPerUnit` is per named unit — the same Law 8 defect storage
had at 13c). And no sea leg existing at all, because a hull cannot start from a land tile.

**The guards earned their keep.** `no-bounds` caught three ternaries and each is now arithmetic
impossibility saying so through `atMost`/`atLeast` with its reason. `no-numeric-default` caught every
`?? 0` in the draw's array reads; they throw now. The Law 8 dimension check refused a clumping
declared in `count` and read as `ratio` — exactly the defect 13b.1 built it for.

**Deleted.** `RouteDecl`, `ROUTES`, `routeParam` and the four route parameters; `RegionDecl.ccy`.
Each names the read that replaced it.

**Not built, and it says so.** Ballast. A hull cannot be repositioned: `vintagesHeld` reads a
party's plant in its OWN region and a vintage is an instrument per (kind, region, serviceDate), so
moving one needs a plant reseat the kernel does not have. A carrier serves the legs out of where it
is based, which keeps everything B2 and E2 ask for and loses one thing — a shortage on one leg
cannot pull hulls off another.

**Where each finding landed.** Good ground stopping plant binding → **13c.2**, which rewrites the
decision those tests are about and rebuilds the fixture in the same change. A hull that cannot be
repositioned → **13g**, because ballast and second-hand plant are one mechanism and it is one door.
A period costing twenty-five seconds — measured to PRE-DATE the map, 24.6s at `a702c37` against
25.8s now, with `markets` 21.5s of it either way — → **16**. `docs/BUGS.md` is gone, which is what
an empty holding pen means.

**Forecast, with the measurement that would kill it (Law 17).** Industry should gather where the
freight it saves is worth more than the ground it bids up, and hug the coast where it exports: the
pull is a freight print, the push is a rent and a wage, and neither is a coefficient. Three
scenarios would falsify it, and each is 16's to run: flatten the freight and the clustering should
go; flatten the ground and firms should stop piling into one place; landlock the exporter and the
coastal premium should fall to nothing. If the world does not cluster, that is a finding about a
missing mechanism and never a licence to add a term that makes it.

---

## 13c.2 — The rest of the economy: services, distribution and retail

**What was wrong.** This world MADE things and did nothing else. Thirty-six physical lines, nine
thousand firms, and every one of them a farm, a mine, a mill or a works — about a quarter of what an
economy is. Nobody treated a patient, taught a child, served a meal, carried a passenger, repaired a
machine or answered a telephone. Between the factory gate and the household there was nothing at
all: no wholesaler, no lorry, no shop. A household bought bread from a bakery at the bakery's own
cleared price, which is not how anybody has ever bought bread.

**Three structures, not a list of rows.**

**A thing that cannot be put in a box.** `GoodDecl.portable` — whether a unit of a line can be
somewhere other than where it was made. It is the one fact that divides a manufacture from a
service, and freight reads it, so a service price is LOCAL by the technology of the thing rather
than by a rule and no voyage can ever close a gap in one. Sixteen service lines stand on it: care,
teaching, hospitality, telecoms, software and support, professional practice, design, media, road
transport, repair, facilities, personal care, entertainment, security, waste and handling — each
made to order (`leadTime: 0`), keeping nothing (`spoilage: 1`, because an hour nobody bought was
still paid for, which is the whole of why a service business has operating leverage), and standing
on no ground of its own. A seventeenth, `wholesale`, arrived with the merchants.

**The distance between the gate and the shelf.** Nine retail lines, each one unit of the wholesale
line in and one on the shelf out, plus the hours, the premises, the power, the packaging, the
handling, the haulage and the merchant's margin that putting it there takes. Each is its own
instrument with its own market in each region, exactly as a tonne in transit is, and for the same
reason (Commodities Spot A1.a). **The distribution margin is therefore an outcome**: two prints,
less what the shop's staff and premises cost it. Nothing declares a mark-up, and a test walks the
declaration to say so.

**A basket whose preference is a quantity.** `ConsumptionDecl.share` is DELETED. A share of what a
cohort spends is the strongest substitution assumption there is — spending on a thing never responds
to that thing's price — and it is exactly what `phoenix/no-value-recipe` refuses on the production
side. What a cohort declares now is two physical quantities per good per member per period: what it
has before anything else, and what it takes on top when the money reaches. Eighteen lines, two
cohorts, and the differences are load-bearing: a retired household needs three times the care and a
tenth of the teaching, is at home more so wants more power, and makes a third of the journeys. The
share of income that goes on food is an outcome of two declared quantities meeting two prices and
falls as income rises with nothing stating that it does.

**And what a merchant is.** The firms in the wholesale line get a reason to buy something they will
never use: bid where you stand for what the thing fetches where it is DEAR, less what this
management wants for the wait; offer what you hold elsewhere at what those lots cost you plus the
same margin, read off the register where a basis already lives. It never estimates a freight cost to
subtract — what the voyage costs is cleared against carriers that have hulls, and a merchant that
bid too much finds it cannot pay for the passage. Two drawn preferences, margin and appetite, and no
schedule of margins by distance anywhere.

**What was deleted, and what replaced it.** `ConsumptionDecl.share` → two quantities and
`rungsUpTo`. The opening bundle's "one unit of each final good" → the basket, weighted over the
cohorts and never averaged: with fifteen final goods that 1 was setting this world's entire
industrial composition. `STORAGE_KIND.madeFrom: 'machine'` → `'building'` and
`VESSEL_KIND.madeFrom: 'machine'` → `'vessel'`: both were true when a machine was the only capital
good this world made and false since the real economy landed with a construction line and a
shipyard in it, and the seed now counts every kind of plant rather than the kernel's one.

**Two guards.** A line that cannot be moved cannot be warehoused either, so declaring both throws
where it is declared. And the chain runs ONE WAY through the services — a service consumes goods and
the goods that consume services are the made ones at the top — so the input-output graph is still a
graph; a test walks every service's inputs to their roots and refuses a world where the engineer's
week bought the machine that built it.

**Siting became a data read.** Which lines go where the people are is read off the BASKET and never
off what kind of line it is, so adding a line to the basket moves its firms and nothing learns the
name of an industry (Law 15). A shop is built next to its customers; a mine is built on the ore.

**The observer gained a sector view** — what each of the twenty-odd verticals made this period and
what is standing in it, both reads, neither stored.

**Forecast, with the measurement that would kill it (Law 17).** Service prices should diverge
between places further and for longer than goods prices do, because nothing can arbitrage them: a
region whose wages rise has no import of haircuts. Two falsifications, both 16's to run: make two
regions' tradeable prices converge and the service gap should stay open; raise one region's wage and
its service prices should move with it while its tradeable prices do not. If services track goods
prices place for place, something is closing a gap that has no mechanism to close it, and that is a
finding rather than a licence to add a term.

---

## 13c — Commodities and freight (closed)

> **CLOSED ON A WORLD THAT DID NOT ASSEMBLE, OR ON A MECHANISM FOUND UNREACHABLE.** Item 0 stepped
> the assembled world for the first time and it stopped in period 2; `docs/IMPLEMENTATION.md` Part 0
> names this module among those that compile, are cited, are marked MET, and have never produced an
> outcome. Every measurement in this entry was taken on a world that could not run to it. The
> mechanism may be right and the numbers are not evidence. Re-verified at item 0d; what the
> capabilities have actually produced is `npm run coverage:reached`.


Paused twice and finished: steps 1–8 before the map, step 9 after it (13c.1), and steps 10–16 after
the economy had somewhere to sell to (13c.2).

**Step 10 — shipper substitution, and the finding is that all three are STRUCTURAL.** A shipper can
hold, source locally, or not trade at all, and none of the three needed building: what a shipper will
pay for a voyage is the gap between the two places' prints and nothing else, so above the gap the
voyage is worse than selling at home; and a buyer standing where the thing is made is already in that
place's own market. The arithmetic leaves no room for a shipper to pay more than the alternative is
worth, so there is nothing to decide between them and no mechanism to write. The tests say it from
three sides (`test/freight.test.ts`), and 13c.1's dispersed placement is what gave the second one a
second place to source from.

**Steps 11–13 — the commodity future.** A lot is a silo's worth, READ off the good's own
`storagePerUnit`, so there is no contract-size parameter anywhere. Four delivery dates a quarter
apart on every grade that can actually be handed over — portable, storable, and with a spot print to
converge to. The carry is three reads and a subtraction: the room at the rate the storage session
printed in that place, the spoilage at the rate the good declares, and the money at the secured
benchmark, put into the same unit by the calendar's own day count. **There is no convenience yield**:
the gap between the future and spot-plus-carry is the measurement. Contango is bounded only through
somebody with room doing the trade; backwardation is unbounded, because you cannot borrow a tonne
that does not exist. Convergence is not enforced — at expiry the short hands over the units and the
long pays at the grade's own cleared spot price in one instruction, and a short with nothing in the
shed FAILS, recorded, with nothing paying a difference instead. Three reasons on the line and all
three in one target, so nothing branches: short by what it holds, long or short on its own view
against where the book stands, and short the contango when it has somewhere to put the thing.

**Step 14 — the two price indices can now move apart, and for a reason.** They were the same prints
weighed two ways. Since 13c.2 they are DIFFERENT BASKETS: a household buys the shelf line and a firm
sells the one at the gate, so what parts the two levels is real work by real firms — the shop's
staff, its premises, its round and its bin, plus the freight and the merchant's margin behind it.
D4.a's margin story is legible in exactly the way the clause wants, and nothing collapses the two.
`Indices D4` and `D4.a` → MET.

**What was NOT run, and it is said rather than implied.** The year-long run and the determinism
sweep in step 14 are a measurement over a world that now costs about a minute a period, and this
session did not run them. They belong where every measurement belongs (Law 11, worklist 16), and
nothing here claims a result it did not get.

**Not built, and it says so.** Ballast (13c.1's finding, positioned to 13g: a hull cannot be
repositioned without a plant reseat the kernel does not have). E2 — households buying energy
directly — is MET since 13c.2 put `power` in the basket, which is what its dependency note was
waiting for.

---

## 13d.1 — A cell's key carries its age, its tenure and its wealth (closed)

**What it was for.** Four of 13d's steps ran aground on one rock: a cell carries its holdings PER
MEMBER in whole pieces, so anything that moves people between two cells has to move VALUE between
two cells — and a leg's `qty` must equal `perMember × weight` on each side at once, which two
weights in the millions share no useful number for.

**The key needed none of the three dimensions it was written for, and that is the finding.**
AGE was already there: a cohort IS an age band with a declared entry age. TENURE was the wrong
diagnosis — a cell could not own a roof because the register's grid had one piece per whole thing,
so four tenths of a dwelling per member rounded to nothing; the grid is a RESOLUTION and it has to
be fine enough to say what one member holds. WEALTH would be a second writer of a fact this world
already has: since 13c.2 the basket is two physical quantities per member, so the share of income
going on food already falls as income rises, per cell rather than per band, and a band is three
answers where a budget gives as many as there are cells.

**What it built instead is the EVENT.** `ctx.cells.reKey` — a split with a different key on the part
that moved. Exact, per-member state and all, totals preserved by construction; nobody appears,
nobody disappears, and nothing crosses. The weight event is a PROMOTION, which is the word XI-15
already had for it. Ageing and retirement are that event and nothing else: the last band is the one
the registry puts past working age, labour reads the band, and no retirement mechanism exists.

**And PROBATE, because a cell cannot pay a cell.** What the dead held goes to a named party — whose
side of a leg is the total, so it can take a thing to the piece — and is divided from there in whole
pieces for every heir, with what will not divide staying on a named book until enough has
accumulated. `ctx.cells.die` refuses a cell that still holds something: no death without a
destination and no residual with no holder are one rule, enforced where a cell ends.

**Two kernel defects, found by being the first cell ever to issue anything.** `payToHolders`
denominated the HOLDER's side of a coupon per member and left the payer's at `none`, so the first
payment a cell owed was refused. And settlement booked an issuer's liability at the ISSUED TOTAL
against an equity account kept PER MEMBER — the same number for a named party, and a million
households' worth against one household's equity for a cell.

**Half the basket 13c.2 built was inert** and nobody had noticed: clothing, furniture, appliances,
electronics and vehicles all rounded to nothing per member before they reached a book. The grid fix
was worth more outside housing than in it.

**Three findings positioned.** A fund gains equity when a holder of its shares is not a cell
(measured: three reds to five once probate holds ETF shares) → **13h**. A borrower that misses goes
on accruing on the lender's book while nothing moves on its own, so its balance sheet drifts by
exactly the interest it did not pay — the reason a household mortgage is built but not switched on,
with Labour E4 and Households E3/E4 behind it → **13f**. The rig's twelve firms cannot cover
sixty-two lines, and making every line present means deriving the population from the firms the draw
actually made → **16**.

**Forecast, with what would kill it (Law 17).** An ageing population should tilt what this world
makes towards care and away from teaching, and should raise the dwellings it needs for the same
number of people, because the retired cohort's basket says both. Two falsifications, both 16's: hold
the cohort shares fixed and the tilt should vanish; move people across the boundary faster and it
should arrive sooner. If the composition does not move with the age structure, something is
averaging the cohorts that should not be.

---

## 13d — Labour mobility, housing, the household life cycle (closed)

> **CLOSED ON A WORLD THAT DID NOT ASSEMBLE, OR ON A MECHANISM FOUND UNREACHABLE.** Item 0 stepped
> the assembled world for the first time and it stopped in period 2; `docs/IMPLEMENTATION.md` Part 0
> names this module among those that compile, are cited, are marked MET, and have never produced an
> outcome. Every measurement in this entry was taken on a world that could not run to it. The
> mechanism may be right and the numbers are not evidence. Re-verified at item 0d; what the
> capabilities have actually produced is `npm run coverage:reached`.


Paused once, for 13d.1, and finished on the other side of it.

**Labour.** Mobility is TWO ROUNDS OF ONE MATCHING FUNCTION: every seeker offers in the trade it
has, then the ones nobody took offer in the trades they have not, into whatever the first round left
unfilled. Who moves is whoever was left over, and what moving costs is TIME — a mover is productive
after the hiring lag AND a quarter of retraining, so the employer pays weeks of wages for work it
does not yet get. Deliberately not a fee: a retraining payment with no payee is a one-sided flow,
and time is what a new trade actually costs somebody. It is also why an employer fills from its own
trade first and why unemployment and vacancies can be high at once. **Participation has the wage in
it**: the going rate is public, so a cell does not offer into a trade paying less than it lives on,
and employers bidding up bring the discouraged back.

**Housing.** A dwelling is a GOOD — built, half a year to make, standing where it was built, wearing
out, and `portable: false` for the oldest reason a price is local. A tenancy is a VENUE, because
what changes hands is the right to be in it for a period and not the thing itself. Rent clears
between what letting WEARS the owner (the dwelling's own spoilage against its own price) and what a
household can pay rather than have nowhere (its own outlook of its income over the occupancy a
member needs) — two reads, no coefficient. Owner-occupation is an outcome and not a tenure flag: a
household that owns what its people live in does not bid, and pays nobody.

**Credit.** A mortgage is a secured loan row with a REAL LIEN, placed every period up to what is
still owed, so the register itself refuses to let the roof be sold out from under the loan.
Foreclosure releases the lien, moves the dwellings, and the lender sells into the same session
everybody else does, so the recovery is what it FETCHED. The bank never learns what a dwelling is:
the request names an instrument it could take and realise, which is the whole of what security means
to a lender. **And the standard is a read**: what it would lose is the part the security does not
cover at the market's own price, so a bank lending against a falling thing requires more of every
borrower without anybody tightening anything.

**`loan.operatingCost` is deleted.** It was half a per cent a year on every principal, paid to
nobody. A bank employs people now, in three trades of its own, and what servicing costs is the hours
its book takes times what it is actually paying for an hour, over the principal it is servicing — so
a small loan is dearer than a large one out of the arithmetic rather than out of a table. A dealing
desk covers the hours it employs over the hours one line takes, so a desk that sheds staff drops
lines and their books journal `market.noView`.

**The household stopped running an analyst's models.** `savingLines` discounted a bond's cash flows,
capitalised a company's published earnings and marked a fund at its book, in one loop, for one cell.
That is one analytical technology handed to everybody — the representative agent one level up — and
it meant parties disagreed only because they had observed different things, which converges. Now it
is the same ladder a loaf is bought on: its own outlook, the last print, or nothing. What it will
PAY is the bottom of the range it thinks the price could be in, so two cells looking at one print
want different prices for it. `priceAt`, `curveFamilyOf` and the whole of `publishedBy` are gone.

**And the retail door into XI-2**: a cell's cushion is against what it holds as well as what it
earns, so a price shock makes it want more cash and bid lower in one read — the redemption is a
household wanting its cushion, not a coefficient. With a declared preference for owning the MARKET
rather than names, which is what makes retail flow undifferentiated and what a tracker is for.

**What is MISSING rather than out of scope, and says so.** Household formation and dissolution:
Appendix B forbids a birth rate by name, so a household coming into being has to be a decision
somebody takes, and nothing in this world yet has the reason to take it. Until it exists the
population falls, and that is a finding about a missing mechanism. Rent is not in the consumer index:
the index walks settled asset legs and a tenancy is a money leg with no instrument behind it, so it
needs a second index rule — positioned to 16. The pension claim is PARTIAL to 13h, as planned.

**Forecast, with what would kill it (Law 17).** A place that builds houses slowly should pay more
for them and more to rent them, and its firms should pay more to hire, because a household's
reservation wage is what it lives on and rent is most of that. Two falsifications, both 16's: cut
the build lag and the gap should close; raise the dwellings a cohort needs and the gap should widen
in the place that cannot build. If rent and wages move independently, the reservation wage is not
reading what it is supposed to be reading.


## 13e — trade credit and securitisation: the invoice, the vehicle, the layer

> **CLOSED ON A WORLD THAT DID NOT ASSEMBLE, OR ON A MECHANISM FOUND UNREACHABLE.** Item 0 stepped
> the assembled world for the first time and it stopped in period 2; `docs/IMPLEMENTATION.md` Part 0
> names this module among those that compile, are cited, are marked MET, and have never produced an
> outcome. Every measurement in this entry was taken on a world that could not run to it. The
> mechanism may be right and the numbers are not evidence. Re-verified at item 0d; what the
> capabilities have actually produced is `npm run coverage:reached`.


**Most of the credit in an economy is not a bank's.** A firm that buys from another firm pays in
thirty days, and until it does the seller has lent it the money without either of them calling it
that. This world had none of it: every trade settled money for goods in the same instant, so a supply
chain could not transmit anything except a price, and a firm could not fail because somebody else
failed to pay it.

**An invoice is ONE ROW.** The seller's receivable and the buyer's payable are the same instrument —
issued by the buyer, held by the seller — so there is one writer of what is owed and the two sides
cannot disagree (Law 4). It is written in the SAME numbered instruction the goods move in: the buyer
pays with a promise instead of money, both legs are there, and there is never an instant where one
side has parted with something and the other has given nothing. The kernel gained one door for it —
`onTerms`, answered by `SystemModule.termsOffered`, exactly one module per party kind — and the door
carries WHAT IS SOLD, because a good is a thing nobody issued and a share is somebody's promise,
bought and paid for on delivery. The register already held that distinction; nothing branches on a
kind.

**Lateness is a state, not a memory.** Whether a seller ships on terms is its own judgement of that
buyer and reads its own book: a buyer with an overdue row on it buys for cash, and when the row is
paid, terms resume. So the ageing IS the tightening, and a solvent firm starved of terms is what
happens when several suppliers are each looking at their own overdue row at once. There is no memory
parameter, because there is nothing to remember — the answer is on the books.

**Who is owed a loan is read off the register.** A loan named its lender in its terms and twelve
places read that name as "whose book this is". That was a mirrored fact and a stale one waiting to
happen: a sold row makes it a lie. The field is now `originator` — who WROTE it — and `creditorOf`
asks the register. The rename is what found the twelve sites; they did not all mean the same thing.
A mortgage sold into a pool is now foreclosed BY THE POOL, which is the whole point.

**XI-11 has all four of its objects and each is real.** A `vehicle` is a named party with a balance
sheet holding the rows. A `tranche` states its attachment and detachment and has a market. The
waterfall pays by seniority out of what the borrowers actually paid — LAST in the period, after
everything the vehicle itself owes, because a pass-through that pays its investors before its own
obligations is not a waterfall but a hole. And a loss lands from the bottom: when the pool falls
below the notes, the layer attaching at zero is written down until there is none of it left and then
the one above, which is what an attachment point IS. Nothing stops a senior loss.

**Nothing about the structure is declared.** The arranger sells no more than it must — its relief is
what LEFT its book — and keeps the rest, because every unit sold beyond its need is income given away
for nothing. The pool is whole rows and a row does not divide, so the overshoot lands in the junior
too. HOW MANY LAYERS A DEAL HAS is an outcome as well: a bank that had to sell everything keeps
nothing, and what the buyers hold is a pass-through over the whole pool with nobody underneath them.
The module declares no parameters at all.

**A vehicle CAN fail, and saying otherwise would have been the whole trick.** The temptation was to
call it bankruptcy-remote and leave `fails` empty, because it has no other business to be brought
down by. But `fails` is not about whether a party has other business — it is about whether a claim on
it can go wrong. Left empty, every capital rule here would have weighed a tranche at NOTHING, and a
bank could sell its book to a vehicle, buy the notes back, and watch its requirement vanish with the
risk still on its own balance sheet. That is regulatory arbitrage by construction.

**A defect the first single-currency income-earner found.** Every treasury in this world walked EVERY
settled money leg wherever it happened and billed the tax in its own money: a dollar coupon paid in
New York raised a euro assessment, a sterling one and a yen one, all at once, against a party that
had never held any of them ("two currencies never added", Law 8). Nobody had noticed, because every
party that receives interest here holds money in several currencies. A securitisation vehicle is the
first that holds one — it was billed four times, could not pay three, and died of a cash failure it
did not owe. One line fixes it. An estate test that REQUIRED a household to fail a tax bill was
measuring the defect and now states the absence instead.

**A finding, written as a test so it cannot be lost.** Securitisation relieves a risk-weighted
requirement and relieves a leverage one by exactly zero: a sale at a price is a swap, the rows go out
and money of the same value comes in, so total assets do not move. Every bank in this world that runs
out of room runs out of it on the LEVERAGE backstop — they carry reserves many times their capital,
and reserves weigh nothing under one rule and everything under the other. So the deal machinery
correctly does nothing here, and the mechanism is unreached rather than wrong. The day a bank here is
weighted-bound, that test fails, which is the point of writing it.

**What is placed rather than built**, and where: small-business cells with their own loan rows and
promotion across the size boundary → 13f, where cell-level borrowers and the corporate credit market
belong; the covered bond (M7) → 13f, with bank funding; financing a receivable — pledge and factoring
— → 13f; senior notes as repo collateral → 13f; mortgage pools through the same vehicle, and the
insurers that would buy the notes → 13h.


## 13f (step 1) — an issuer does not profit from its own decline

**The defect.** Every instrument that is a liability of its issuer was re-marked on BOTH books: the
holder booked the price move and the issuer booked the negative of it. So a firm walking towards
default booked a PROFIT on the way down — its equity rising as the market lost faith in it — and a
treasury whose paper was falling grew richer as its own credit went.

**Why it is structural and not a booking convention.** Debt is not carried at market value on an
issuer's balance sheet, because the borrower still owes the whole of it on the day whatever anybody
will pay for the paper today. And the fiction put XI-3 out of reach exactly when XI-3 is supposed to
fire: the closer a firm came to failing the more equity it made, so the solvency trigger receded as
the failure approached. For a sovereign it deleted the funding constraint XI-9 exists to impose.

**The fix is one declared fact on the instrument, and it is a fact rather than a flag.** `owes` says
which of its issuer's two situations a kind is in. `'face'` — the promise does not change, which is
every bond, bill, loan, deposit, repo row, invoice and note here. `'value'` — the claim IS the book,
which is the fund share and only the fund share: what a fund owes its holders is what its pool is
worth, and that move is what keeps a fund's own equity at zero where a fund's equity belongs (Fund
Shares A3). There is no default and no third answer: a kind that is somebody's liability has to say
which. A kind that is nobody's liability and claims to owe its value is refused at assembly.

**It had to be written in four places, which is how one knows it was one fact.** The revaluation
pass (the issuer's side now exists only where what it owes follows the price); settlement's `issue`
(what the issuer takes on is the FACE, so a bond brought at 98 leaves it owing 100 and holding 98,
and the two is a discount on issue it wears on the day — real, and the reason a poor name pays at
the moment it borrows rather than never); settlement's `redeem` and its holder-to-holder `reseat`
(two parties agreeing a price between themselves is not an event on the borrower's book); and the
balance sheet the accounts family reads, which was valuing own debt at the market on one side of
the identity while the equity account no longer moved with it.

**What found the last one.** The identity fired on ESTATES: an estate assumed a dead party's
liabilities at what the holders carried them at, while its balance sheet now read the face, and the
two disagreed by exactly the fiction that had been removed. `balance-identity.test.ts` is what
caught it, which is what that family is for.

**Also in this item so far.** Finding `12d-3`: a loan's rate was derived twice and the two did not
agree — quoted 0.013676115348016367 and written 0.013676161104839884 for the same borrower, same
period, same bank, because the inputs move between the two phases. What is written is now what was
QUOTED; `quote()` is called in one place. A bank that cannot lend after all records a refusal rather
than a different price, and a name nobody would quote is a refusal too, each bank naming which of
its own constraints stopped it (C3.a).


## 13f — the other credit channel, and an issuer that stopped profiting from its decline

> **CLOSED ON A WORLD THAT DID NOT ASSEMBLE, OR ON A MECHANISM FOUND UNREACHABLE.** Item 0 stepped
> the assembled world for the first time and it stopped in period 2; `docs/IMPLEMENTATION.md` Part 0
> names this module among those that compile, are cited, are marked MET, and have never produced an
> outcome. Every measurement in this entry was taken on a world that could not run to it. The
> mechanism may be right and the numbers are not evidence. Re-verified at item 0d; what the
> capabilities have actually produced is `npm run coverage:reached`.


**The own-credit defect** is written up above; it is the largest thing in this item and it was found
by being asked the right question. What follows is the rest.

**One rate per loan.** `12d-3`: the quote was derived once to publish and again to write, and the two
disagreed by three parts in a million because the inputs move between the phases. What is written is
what was QUOTED; `quote()` is called in one place. A bank whose room moved between the two records a
refusal rather than a different price, and a name nobody would quote is a refusal too, each bank
naming which of its own constraints stopped it (C3.a).

**Securities lending: the defining property is that two things come apart.** Legal title moves to
the borrower — it can sell what it borrowed, which is the entire point — while the economics stay
with the lender, which gets a manufactured payment equal to whatever the line actually paid. A world
that moved one without the other would have a lender paying a fee to lose its income.

It is built out of things that already existed. The security moves in the register like any units.
The collateral is a PLEDGE, so it leaves the poster's free balance and the register refuses to move
it — nothing here has to remember not to count it twice. The fee is a price and it clears in a book
per line, so scarce paper is dear and abundant paper is cheap; the rebate is the same number seen
from the cash side rather than a second table. The lendable pool is a read of who holds the line
FREE, and it is what caps how large a short can get, which is what makes a squeeze possible.

**The corporate bond, and what is NOT corporate about it.** When a coupon falls and how it accrues is
the same fact for any dated bond whoever issued it, so the schedule moved into the kernel and both
kinds read one statement of it — two copies would have been two writers of one fact. What differs is
the three things that make corporate credit a different subject: the issuer can FAIL, so a missed
payment is a default and the estate has a ranking to read; the line is senior or subordinated, and
the number is on the instrument where the waterfall already looks; and there are COVENANTS.

**A covenant is a term of the issue and not a bound.** It is what this issuer promised when it
borrowed, and breaching it is an EVENT with consequences — never a number pushed back inside a range
(Law 6). Two lines, because they are the two questions a lender actually asks and they fail in
different worlds: how much it owes against what it holds, which a fall in asset prices breaks, and
what it earns against what falls due, which a bad year breaks. A firm can pass either while failing
the other, and which one goes says what went wrong. Both are tested on the PUBLISHED accounts, with
the lag publishing already has, because a covenant a lender could test on private books is not a
covenant but surveillance — and reading the report rather than recomputing it is what stops a second
set of accounts existing (Reporting A2.a). The module declares no numbers at all: what a given firm
promised is an outcome of what it had to promise to be lent to.

**What is placed rather than built**, and where: prime brokerage and restructuring → 13h; short-term
debt and the roll that can fail → 13i. Syndication, bookbuilding, committed facilities and the
covered bond stay in the worklist's carried findings.


## 13g — the market for control

> **CLOSED ON A WORLD THAT DID NOT ASSEMBLE, OR ON A MECHANISM FOUND UNREACHABLE.** Item 0 stepped
> the assembled world for the first time and it stopped in period 2; `docs/IMPLEMENTATION.md` Part 0
> names this module among those that compile, are cited, are marked MET, and have never produced an
> outcome. Every measurement in this entry was taken on a world that could not run to it. The
> mechanism may be right and the numbers are not evidence. Re-verified at item 0d; what the
> capabilities have actually produced is `npm run coverage:reached`.


**A premium is a price and it has to clear.** Every share in this world already has a market and
every holder already has its own number for what a share is worth to it. A takeover is what happens
when somebody else's number for the WHOLE firm is higher than theirs for their piece of it — so the
tender is an ordinary book with an unusual buyer, and the premium is the distance between two
valuations rather than a percentage anybody chose.

**An acquirer values a target the way it values a machine.** What it would get, against what it
requires: the target's PUBLISHED earnings (Reporting A2, annualised by the span of its own report)
over the rate a bank quoted THIS acquirer. Both are reads and neither is a forecast. There is no
synergy term and no control premium — two acquirers want the same firm at different prices because
they have different costs of money and different views, which is what §46 A3 says disagreement IS,
and it is the whole reason a market for control exists. A world where everyone valued a firm
identically would never see a takeover.

**Nobody has to tender**, and the bid carries an acceptance condition. Below control nothing settles
at all: there is no half-acquisition in which the acquirer has paid for a minority it never wanted,
and a failed bid is an event with consequences rather than a silence.

**What a bidder looks for is the residual claim**, and it is read structurally rather than by name:
a claim ON somebody that its issuer does not owe. Buying every liability of a firm buys you nothing;
buying the residual buys you the firm. That is Equity A1's own definition, the register and the kind
profile already carry it, and reading it that way is what lets this module exist without importing
the one that owns shares.

**On completion the two balance sheets combine through the estate's door.** "This party's
obligations are now that one's" is one fact, and a world with two ways to write it would have two
ways for it to be wrong.

**A finding, written as a test.** Every listed firm in this world publishes a LOSS, so there is no
stream to capitalise and no bid to make. The mechanism is unreached rather than wrong, and the day a
listed firm here earns something the assertion fails and the tender machinery starts running.

**Placed rather than built**: firm birth, learning by doing (**M5**) and siting → 13h.


## 13h — the sector that has duration

> **CLOSED ON A WORLD THAT DID NOT ASSEMBLE, OR ON A MECHANISM FOUND UNREACHABLE.** Item 0 stepped
> the assembled world for the first time and it stopped in period 2; `docs/IMPLEMENTATION.md` Part 0
> names this module among those that compile, are cited, are marked MET, and have never produced an
> outcome. Every measurement in this entry was taken on a world that could not run to it. The
> mechanism may be right and the numbers are not evidence. Re-verified at item 0d; what the
> capabilities have actually produced is `npm run coverage:reached`.


**B2.b is the clause this exists for**, and it says what the easy version gets wrong: a liability
that accumulates contributions minus benefits plus investment income has no schedule, no discount
rate and no discounting — so it never moves when rates move, and THE SECTOR'S DEFINING RISK
DISAPPEARS. This is the model's largest holder of duration; it must have duration.

So a policy here is a promise to pay STATED AMOUNTS AT STATED FUTURE TIMES, and what it is worth is
those amounts discounted at a rate read from a market. That needed one new kernel read — `curve` on
`DerivedReads`, the world's own curve passed in rather than rebuilt, because a second derivation of
a discount factor is a second answer to one question. A derived value is allowed to be exactly this:
a fact about the world anybody may compute and everybody gets the same answer from.

**Falling rates raise the liability and the institution wears it.** The kind says its issuer owes
the VALUE of what it promised rather than a face, and the revaluation pass does the rest: the
liability rises, its equity falls, and a rate move is a solvency event for this sector and a P&L
event for everybody else. Nothing anywhere had to assert that — it is arithmetic on a schedule and a
discount.

It is the second kind in this world whose issuer owes a value, and it is the OPPOSITE case from the
first: a fund share moves because the assets moved, a policy because the discount rate did. Both
needed the same declared fact, and the fact was built two items ago for a different reason.

**A2.a is the whole difference between this sector and a fund**, and it is that one fact: the
beneficiary does not absorb the investment result, the promise is fixed, and the institution's
equity is what moves. A sector that passed the result through would be a fund wearing an insurer's
name.

**What it charges is its own experience and its own capital and nothing else** (A4.b): what a unit
of cover has actually cost IT, read off the claims it paid, plus the return required on the capital
held against the premium. Worse experience or dearer capital quotes higher. No loss ratio, no
industry table, no draw from a stated distribution, and no declared parameter in the module at all.

An insurer with no surplus writes nothing, which is not a threshold: it has nothing to stand behind
cover with, and that is arithmetic (A4.a, Law 6). `capacityOf` was written as a ternary that floored
a negative surplus at zero, which is a bound; it is gone, and the `<= 0` guard that was already
there says the same thing once.

**Placed rather than built**: hedge funds, private equity and the insurer's matching asset side →
13i.


## 13i — a country's accounts are a walk, and they sum to zero

**Every model that imports a balance of payments has imported an equilibrium.** Appendix B forbids
an exogenous trade or capital-flow series by name, and a table of flows is exactly that. So here the
external accounts are a WALK over the settled ledger: what actually crossed a border this period,
leg by leg, read off instructions that already happened. Nothing is stored and nothing is inferred
by subtraction; ask again next period and it is walked again, which is what makes it impossible for
this to drift away from what the wire says.

**They sum to zero because every transaction had two sides.** That is Law 5 seen from a country's
end, not an identity anybody enforces and not a residual anybody plugs. If the two halves did not
cancel, a leg went out with nothing coming back — the one-sided flow Law 5 forbids — which is the
only thing the audit family can find, and it reports it rather than repairing it.

**What decides which half is whether the thing that moved is somebody's PROMISE.** A tonne of grain
is nobody's liability and is trade; a bond is somebody's and is finance; money is a claim on the
bank that issued it, so it is finance too. The register already knows that — it is the same question
the balance sheet asks — so nothing here asks what kind of instrument anything is.

**The interesting case is the one a service economy is mostly made of.** A sale of grain abroad has
two crossing legs, the cargo out and the money in, and they cancel on their own. A WAGE has one: the
money goes, and the week of work that earned it is not an instrument anybody holds. Labour, rent and
a fee are all like that. So a payment with nothing delivered beside it books BOTH halves — the money
that moved and the thing it bought, valued at what was paid for it, which is a read of the payment
rather than a number invented for it. That is what a current account IS: the other side of every
payment whose subject never appeared in a register. Without it the accounts of this world's regions
were out by exactly their wage bill, which is how the case was found.

**Placed rather than built**: hedge funds, private equity and the insurer's matching asset side came
in from 13h; sourcing across regions, foreign-currency issuance and the central-bank swap line
(**M3**, **M6**) stay carried in the worklist.


## Law 18 — the schedule a line cannot change, computed once

**Measured first, on the compiled build.** At rig scale one period was 52 ms, and the profile put a
sixth of engine self-time in date arithmetic: `dayNumber` 4.3%, `compareCivil` 3.8%, `cashFlowsOf`
3.3%, `addMonths` 2.2%, `calendar.schedule` 1.6%, `civil` 1.2%, `yearFraction` 0.9%. Counting the
calls said why: `couponDatesOf` ran **3,003 times a period and built 48,162 dates**, in a world with
about thirty bonds in it, and `compareCivil` ran **99,580 times** with 93 parties on the books.

**Two things were wrong and they were the same thing.** `dueOf` and `cashFlowsOf` each walked a
line's schedule and each worked the coupon out with its own copy of one formula — two writers of
"what this coupon comes to" (Law 4), the kind of pair that drifts the day somebody fixes a day count
in one of them. And both regenerated the whole schedule from issue to maturity on every call, every
period, for every read: the curve, the revaluation, `due`, `accrued`.

A line's schedule CANNOT change: the terms are fixed at issuance and the calendar is the world's one
calendar. So it is computed once and read thereafter, keyed weakly on the terms and the calendar —
the two identities that decide the answer — which is the ledger's own pattern (`byPeriod`,
`failedBy`): the same facts under a second arrangement, written where they are first computed, one
writer, nothing that can go stale, and a line that ceases takes its schedule with it.

**`compareCivil` was converting twice to answer a question about order.** It called `dayNumber` on
both dates — ten `Math.floor`s and a couple of dozen operations — where a calendar answers by
reading three fields in order. Measured at **46.1 ns against 22.0 ns**, and the signs agree on all
143,000 pairs of a date grid, which is the behaviour gate for it. Every caller in the codebase uses
the sign — a comparison or a sort comparator — and none reads the magnitude as a day count, which
the comment now says out loud.

**The gate.** Not "the same tests pass" but the same WORLD: twelve periods of the rig, digesting
every market outcome and price, every audit total, every settled instruction and every issued
amount. Before and after are the same hash — 4,041 instructions, 347 instruments, identical. Law 18
asks for behaviour, not bits, and this is the stronger of the two.

**The result: 52.4 ms a period to 41.0 ms, 22% off**, with the least noisy number — the minimum of
five runs — going 48.8 to 37.6.


## Law 18 — a module's context, built once a cycle

**`mechanismContext` was 4.7% of the period and all of it was allocation.** A module's context is
forty closures over a world that is not going anywhere, and it was built fresh on every call — per
phase, per valuation, per outlook, per margin check, and once per TRADE for the terms door.

Everything in it reads `this` live, so one object serves a whole cycle. The two things that do not
are the period and the cycle, which are captured as values — so they are the key, and a context is
rebuilt exactly when one of them moves.

**The RNG is the exception, and it is the reason this needed care.** A derived stream is STATEFUL:
sharing one between two calls in a cycle would have the second caller continue the first one's draws
instead of starting where it started, which is a change to what the world does. So the cached object
is spread with a fresh stream over it — forty references copied instead of forty closures built.

**And the obvious next step was the slower one.** Handing the stream out through a lazy getter skips
the derive entirely for a module that never draws, which looked free: `prng.derive` was 3.1% on its
own. Measured, it was WORSE — 41.9 ms a period against 36.2 — because an object with an accessor on
it is not the shape V8 inlines property reads on, and the forty reads a module makes of everything
else in its context cost more than the stream saved. It is reverted, with the measurement written
where the next person will look. This is what Law 18 means by gating on behaviour rather than bits:
the cheap-looking change has to earn it on the clock.

**Gate**: the same digest as before, 4,041 instructions and 347 instruments over twelve periods.
**Result**: 41.0 ms a period to about 36, and 52.4 to 36 across the two Law 18 changes together —
roughly a third off, with the minimum of five runs going 48.8 to 32.6.


## Law 18 — the two reads the period loop makes most often

**A sum of whole pieces has no dust.** `register.quantity` was the most expensive read in the engine
at 3.7% of the period. It allocated an array of lot quantities and handed it to `sum`, which is
compensated summation with a finiteness check and a magnitude walk on every term — exactly right for
money, where a total carries the rounding of everything that made it, and unnecessary here: a lot's
quantity is a `Qty`, a whole number of the unit's indivisible pieces, and adding whole numbers is
exact. The compensation term Kahan accumulates is identically zero on integers, so the answer is the
same to the bit; `asQty` still refuses a total that has left the range integers are exact in, which
is the one case where the question arises at all. Measured in isolation at **92.3 ns against 23.5**,
identical on 20,000 random integer holdings. `encumbered` is the same read of the liens.

**`ofKind` walked the world to find its banks.** It spread the whole party map into an array and
filtered it twice, on every call — and modules ask it inside loops, so a world of three thousand
parties walked three thousand of them each time. There is now an index of party IDS by kind, in the
order they were added, written in `add` where a party's kind is settled and never changes again.

It holds IDS and never objects, and that is what keeps it honest: a party is frozen and REPLACED
when its weight moves or it ceases, so a bucket of objects would go stale the first time one did. A
bucket of ids is resolved through the map at read, so the answer is always current, and the
insertion order the old filter produced is the order the bucket is in.

**Where this leaves it.** Engine self-time over sixty periods fell from 5,159 ms to 4,246, and the
profile is now flat — nothing above 3%, where it opened with a sixth of the time in date arithmetic.
Per period, 52.4 ms to 33.4 on the median and 48.8 to 29.6 on the minimum of five runs, which is
**about 39% off** the number least disturbed by a noisy machine. Every step gated on the same
digest: 4,041 instructions, 347 instruments, every market outcome, price and audit total identical.


## Law 18 — the scans that grew with the world

**The rig is 179 parties and the world is 3,169**, so the last round went looking for traversals
that grow with size rather than costs that show on a small clock. Counting calls at two sizes said
nothing was superlinear in call COUNT — but three reads walked every instrument in the world from
inside a per-party loop, which is the product that does not show until the world is big.

**The accounts family was the worst of them.** `balanceSheet` found a party's liabilities by walking
every instrument and asking whether this party issued it — once per party, once a period. At real
scale that is three thousand parties against five thousand lines, fifteen million visits a period,
to find each party's handful of liabilities. `Instruments` has kept a `byIssuer` index since it was
written; this was the one read not using it.

**A loan is issued by its borrower**, because the borrower owes the money — so the rows that could be
a borrower's line at a bank are the rows that borrower issued, and `lineOf` can ask the same index
instead of scanning the world once per credit request.

**And the list of listed lines is the same list for every bidder.** `controlBidsFor` worked out which
instruments are residual claims — a claim on somebody its issuer does not owe — for each firm in
turn, arriving at the same handful of lines three thousand times. It is found once for the session
and handed in.

**Measured**: full scans of the instrument register fell from 530 to 439 a period at twelve firms
and from 660 to 525 at twenty-four, and the ones removed are precisely the per-party ones, so what
flattens is the growth rather than the level. **Gate**: the same digest again, which mattered more
here than elsewhere — `reseat` moves a line to the end of its new issuer's bucket, so an estate
assuming paper could have changed the order a sum was taken in, and the hash says it did not.


## The audit is one file, and the files that count the work now count it

**What this is.** Not an item: the close of a three-part read, and the repair of the two project
files that were describing a tree which no longer existed. `docs/AUDIT.md` is now the one place an
open finding lives, and `npm run check` can run again.

**One file, because three was the defect.** `docs/BUGS.md`, `docs/SWEEP.md` and `docs/VERIFY.md`
were three holding pens with the same rule as each other — _a finding leaves only by being placed_ —
and what three bought was the same defect written three times under three names, with the newest of
them (`VERIFY.md`) contradicting the other two about why the derivative books never cross. They are
merged into `docs/AUDIT.md` Part III, de-duplicated against Part I (`VERIFY.md`'s six lettered
findings were all re-found independently in the read, and the map from one to the other is stated
rather than the finding repeated). `docs/plan/14-polity.md` went in as Part IV, because item 14's
two carried findings are audit findings and its design has to survive contact with what the read
found. All four files are deleted. Law 4 applies to documents: one writer per fact.

**What the read found.** 94 findings. Part I is 70 from reading all 59,927 lines of
`packages/engine/src` against three questions — is it bottom up, is currency conserved, is the
mechanism real — of which 30 are severity A. Part II is 15 from the other direction: every `done`
row of the worklist and every `MET` in coverage, against what the code actually produces. Part III
is the 7 carried in. Part IV is item 14's two. The pattern behind most of it is one sentence, and
`VERIFY.md` had already found it: **"built" was true of the source and false of the world.** Eight
closed items are for mechanisms that are assembled, compile, and have never produced an outcome —
no corporate bond is ever issued, no insurance party is ever created, no dealer quotes in any
market, no order has ever reached the tenancy venue, eight of the nine derivative books can never
print a first price.

**The gate had been red for four commits and nothing said so.** `6d6cb6d` inserted five worklist
items by writing them **over** the manifest row for 13j instead of after it, so the worklist named
51 items and the manifest 50, and `crossCheck()` — which exists for exactly this — threw. `npm run
check` runs `plan:check`, which calls it, so the last gate CLAUDE.md names could not have passed
since. Two commits closed work in that tree. The `13j` row is restored; two tests in
`tools/test/plan-progress.test.ts` are green again, the second having asserted that the items with
no plan file were exactly three when the same commit had made them eight.

**An item's state has one writer, and it is the worklist.** `plan-progress.ts` read it from two
places: the state column, and whether `docs/plan/<item>.md` still existed. The second is a guess
that happened to be right while every deleted file belonged to a closed item. Item 14's plan is now
Part IV of the audit and its file is gone while the item is open, at which point the guess would
have published **94.8%** — fourteen worked steps for an item nobody has started. The tool now READS
the column (Law 19) and the inference is deleted. Five open items that rendered as "closed (no item
file)" say `open (no item file)`. Recounted: **92.9%, 678 of 730 steps across 51 items**, against
the 92.7% of 46 items the stale block published.

**`MET` says something about the source and the file now says so.** It is derived from `@spec`
citations: a module implementing the clause exists. It cannot say the module has ever run, because
nothing about a citation depends on the world — and 99 `MET` rows cite a module that has never
produced anything. The 96 that cite nothing else carry **NEVER REACHED** in their `where` cell; the
three that also cite a kernel path are left alone, because the kernel half of each does run. They
are **not** re-marked `PARTIAL`: a `PARTIAL` names the item that finishes it, none of these findings
has been positioned into one yet, and inventing a placement to satisfy a lint rule is the clause
deleted to look better, backwards.

**Found and not fixed.** Item **13j** closed with no entry in this file and no coverage re-mark —
`6d6cb6d` touched neither — so the item that gave this world four working economies left no outcome
in the ledger, and its measurements survive only in a commit message (`docs/AUDIT.md` **B-11**).
That is not repaired here: a record entry is written by whoever did the item, out of what they
measured, and one reconstructed from a diff afterwards is the opposite of a ledger. **B-14**: a
finding positioned into 13h was not done by the time 13h closed, and nothing checks that — the
comment in `seeds/foundation.ts` went on naming a future that had already passed and a file that had
been deleted. It is corrected to say what is true and the finding is left unpositioned.

**Nothing in the engine's behaviour changed.** One comment in `seeds/foundation.ts`, and the rest is
`tools/plan-progress.ts` and documents.


## The audit becomes the plan: sixteen missing primitives, and the register that would have found them

**What this is.** Not an item: `docs/AUDIT.md` converted from a catalogue of ninety-four findings
into ordered work. Every finding is POSITIONED under the change that closes it — nineteen items,
nothing dropped, and the index at the foot names where each one landed.

**The fault the ninety-four share.** This model has one rich type system and it is the
instrument/settlement layer — kinds with profiles, legs, causes, lots with basis, tick grids, one
writer per fact. It is why money is conserved on the wire. **Above it there is no type system at
all**: every time an economic category was needed it was expressed as a configuration of an existing
instrument-layer primitive rather than as a new one. Income is `Instruction.cause`, nine settlement
labels, so a household is taxed at the wage rate on a returned principal and on gross disposal
proceeds. Value is `outlook('price.<id>')`, so the equity market is a closed loop of price
extrapolators with no fundamental side. A service is `portable: false` with `spoilage: 1`, so a
school holds unsold teaching hours as inventory and writes them off. Who owns and who decides are
one `fund` party, so a separate account cannot exist and a hedge fund is built on a kind whose
`borrows` is hard-coded `false`. Alive-or-dead is a boolean, so a bank in resolution is `alive: true`.

**Sixteen missing primitives, found by enumerating the five axes of an ontology mechanically from
the code rather than by reading it again.** Entities from the nineteen `ctx.state` slots across
seventeen modules; relations from what the register can express (two, plus three fields on `Party`);
events from the nine wire causes and twenty-three free-text journal kinds; states from `PartyStatus`
and `InstrumentStatus`; properties from the ten fields on the instrument profile and the six on the
party profile. Four of the resulting nouns close no finding at all — `PublishedStatement`, `Space`,
`Guarantee`, `Objective` — because a primitive nobody attempted leaves nothing for a read to find.

**The primitive underneath the other fifteen** is the register of them. `ParamRegister` forces every
declared NUMBER to name its kind, its unit and its owner, and `npm run check` fails if one does not.
There is no equivalent for CATEGORIES: the spec's forty-eight systems enumerate behaviours,
`registry/kinds.ts` enumerates settlement objects, and nothing anywhere enumerates the things the
economy is made of. So "is there a noun for this?" had no lookup, only a grep, and a grep finds only
what somebody already tried to build. Item 0 is that register.

**Two dependencies that the enumeration turned up and that reorder the work.** `expectedStream()` —
the behaviour whose absence means only debt can be valued in this world — cannot be built until
published accounts are a kernel noun, because §48's whole output lives in one module's private bag
and a module may not import another module. And worklist **13k is not periodicity**: `Periodicity`
is a proper union in `core/rate.ts` and `reporting/fiscal.ts` is built and unused. A dividend is paid
every week because there is no DECLARATION to be separate from the payment, which is a missing noun.

**What is explicitly not to be touched** is now stated in the file: the grid and dust discipline,
settlement's money expansion, the solver, the curve, the parameter register's shape guards, the
two-sided FX quote. The absence of a finding is the finding.

**Nothing in the engine changed.** `docs/AUDIT.md` was rewritten by reassembling its own finding
blocks under the new spine, so no evidence was lost; `CLAUDE.md`, `docs/PLAN.md`, `docs/WORKLIST.md`
and `docs/ARCHITECTURE.md` now point at a plan rather than a catalogue.


## Item 0 — the register of nouns

**What.** `registry/nouns.ts`: every store a module keeps through `MechanismContext.state` is now
declared, and an undeclared one throws at the read. The first item of `docs/AUDIT.md`, and the reason
it was first: it is the thing that stops the rest recurring.

**Why.** `ParamRegister` forces every declared NUMBER to name its kind, its unit and its owner, and
the build fails if one does not. There was no equivalent for CATEGORIES. The specification's systems
enumerate behaviours and `registry/kinds.ts` enumerates settlement objects; nothing enumerated the
things the economy is made of. So "is there a noun for this?" had no lookup — it had a search, and a
search finds only what somebody already tried to build. Sixteen missing primitives were found over
six passes and every pass found more only because the method changed.

`ctx.state(name, initial)` took a name and an object and asked nothing, so it became where every
category the kernel had no home for ended up. Its own docstring lists three of them without noticing:
*"a register of employment rows, a book of invoices, a party's outlooks."*

**The construction is `ParamRegister`'s, deliberately.** A store is `noun` (a thing this economy has,
which the kernel should own), `working` (state one phase hands to a later phase inside a period) or
`physics` (the module's own subject matter, private by right). A `noun` is a PLACEHOLDER and must
name the plan item that gives it a kernel home — the same guard a placeholder number carries, for the
same reason (Law 2: a stand-in with no scheduled death is a permanent one). The module says what is
in the store; **assembly stamps the owner**, because a module writing its own owner could name
another's and the register would believe it (Law 4).

**Measured: 19 stores across 17 modules — 14 nouns, 4 working, 1 physics.** Where the fourteen are
going: seven to `Agreement` (employment, two kinds of lease, invoices, stock loans, covenants,
securitisation deals), four to `View` (outlooks, ratings, research — and `banks/reserves`, which the
register exposed as a **second private implementation of the outlook mechanism**, a bank's adaptive
memory of its own account kept outside the module that is supposed to be its one writer), two to
`PublishedStatement` (reporting, and the fund's previous NAV, which is a published figure no other
party can see), one to `Process` (estates). Only the weather is private by right.

**Found while classifying, and not chased.** `funds`' slot holds three things of two kinds — a queue
and a strike that are working state, and a previous NAV that is published. It is declared by the
strongest claim and the reason says so; item 3 splits it.

**Positioned out of this item.** **B-14** (a finding positioned into 13h, which closed without doing
it) was filed under item 0 and does not belong there: the register is about nouns, and B-14 is about
an item closing with its positioned work undone. That is what item 1's reach family detects, and it
is carried there.

**Forecast, with its killer.** After this, a module needing a category the kernel has no home for gets
a build failure naming what to declare, instead of reaching for the nearest bag. **What would falsify
it:** a seventeenth missing primitive found by reading rather than by the register — which would mean
the three kinds are the wrong vocabulary, not that the register is unnecessary.

**Green:** lint, typecheck, spec citations, forbids, plan progress, and `nouns.test.ts` — seven
checks including a rig world that opens and steps a period with every store on that path declared.
`world.test.ts` has five reds and they are **not this item's**: the same five fail at the commit
before it, and they are `docs/AUDIT.md` C-1's "a test that named the world it was written against"
(a small assembled world compared against a phase list from the foundation world). Measured both
ways rather than assumed.


## Item 1 — reach: what was declared, against what has ever come of it

**What.** `world/reach.ts`. Every capability this world declares is named before anything runs, and
what has ever come of it is counted. Published every period in the audit report's reads. **Measured
on the rig at three periods: 397 declared, 221 never reached.**

**The plan said "one audit family" and the spec forbids it.** Audit **E1**: the audit *"cannot find
an absence — no invariant fires because credit has no price or because a currency market does not
exist; there is nothing to be inconsistent with."* **E2** separates the two jobs: the audit measures
CONSISTENCY and the requirement document measures COMPLETENESS, and neither substitutes for the
other. A never-reached capability is an absence. So it is a READ — one of the standing measurements
the report already carries (Part XII) — and it feeds `docs/COVERAGE.md`, which is where completeness
lives. Found by reading the spec section before building, which is the point of reading it.

**Seven kinds, and only two of them are tallied.** What a participant posted and whether a module's
own store was opened have no store behind them, so the world records those as they happen. The other
five are DERIVED at read time from the stores that already hold the answer — an instrument kind has
reached the world when one of its instruments exists, a party kind when one of its parties does, a
derivative kind when one of its contracts has opened, a market when it has printed a price
`wasTraded` says is a trade. A carried mark is the market saying it did NOT clear, and counting one
would count the refusal as the outcome (Law 3, E4). No second copy of any fact (Law 4, Law 19).

**Every row has an owner**, which meant stamping one on participant and venue-participant
declarations the way `addPhase` already did. A finding with nobody to answer for it is not a finding
(Audit D2).

**What the first run found, in one period, that three reads of the source had found one at a time:**
`instrumentKind:corporate.bond`; `partyKind:insurance` and `instrumentKind:policy`;
`participant:banks/bank`; `venueParticipant:housing/household`; **all nine** derivative kinds;
`partyKind:vehicle`; `repo`, `tranche`, `margin.claim`, `defaultFund.contribution`,
`closeOut.claim`; two module stores never opened; and 190 markets that have never printed a cleared
price. Each of those cost a day of reading and is now a line in a report that runs every period.

**Closed:** **B-10** ("checks green" satisfied by families that cannot fail), **B-12** (99 `MET` on
never-producing mechanisms), **A-69** (dead entry points), **C-5** (params never read), **C-6**
(margin refusals). Their whole content was invisibility, and it is gone.

**Not closed, and the item stays open for it.** The read NAMES twenty dead capabilities; it does not
build them. Each is carried by name to the item that owns it. And deriving `docs/COVERAGE.md`'s `MET`
from the read rather than from the presence of an `@spec` tag needs `coverage:spec` to run a world —
a tooling change, carried to item 18.

**Forecast, with its killer.** From here a sector cannot be built, cited, marked `MET` and left dead
without the report saying so every period. **What would falsify it:** a capability that produces
something the seven kinds cannot see — most likely an outcome that is neither an order, a print, an
instrument, a party, a contract nor a store write.

**Green:** lint, typecheck, spec citations, forbids, plan progress, 51 tests. `world.test.ts`'s five
reds are the same five that fail two commits back (C-1) and are not this item's.


## Item 2 — missing is missing, in the file that owns the rule

**What.** `phoenix/no-numeric-default` turned back on in `packages/engine/src/core/num.ts`, three
numeric defaults deleted, and one pair of findings closed that turned out to be a single fact.

**The rule already existed.** Written, registered, enforced across the whole engine — and
`eslint.config.js:73` switched it off in `core/num.ts` alongside `no-bounds`. The exemption is right
for bounds: `atMost` and `atLeast` are defined in that file and ARE the admissible bound, so the rule
cannot apply to its own definition. It is wrong for the other one, and nothing said why it was there
— it was carried along in the same line. So **the one file that owns "missing is missing" was the one
file where a numeric default could not be seen**, and it held two: `addTo`'s `acc.get(key) ?? 0` —
the accumulator every tax base and half the engine's sums pass through — and `moved`'s `through = 0`.
A check switched off is the defect it was meant to catch, which is A-11 and A-42's shape appearing in
the lint rather than in a family. The tell was already in the tree: `register.ts` carried an
`// eslint-disable-next-line phoenix/no-numeric-default` for a rule that was not running there.

**Three sites now say what they mean rather than being excused.** An accumulator with no entry is
not a missing number defaulting to zero, it is the FIRST TERM, and it is written that way. A move
that names no `through` is not omitting a magnitude, it is saying the move IS what the arithmetic
passed through, and the absence is now carried as an absence. No disable comment survives.

**A-49 and A-30's second bullet are one fact and closed together.** `offeredYield` took the LAST
curve family the map happened to iterate rather than the best — so a world that registered its
curves in another order offered its savers a different number for the same paper — and answered `0`
when none replied, which made a fund with no curve publish a NEGATIVE offer of exactly its own fee.
It answers `Option<number>` now, and the strike event carries the key or omits it. That reaches
`fundPositions`, which read a missing offer as `0`: zero fails `offered < required` for every cell,
so the cell **silently never subscribed** and no reason appeared anywhere. It reads absent as absent
— a saver with nothing to compare does not compare, and the position is not in the list.

**Carried, and why.** **A-25** (an unpriced physical leg valued at zero), **A-34** (a firm with no
wage history bidding as if labour were free), **A-45** (a bank whose book is funded by nothing) and
A-30's other two bullets are not missing VALUES, they are missing MECHANISMS: what an unpriced leg is
worth, what a firm without history should think labour costs, what funds an unfunded bank. Law 11 —
a misbehaving number is not a work item, the missing mechanism is — so each goes to the item that
builds it rather than being forced into a throw here.

**Forecast, with its killer.** No numeric default can now enter the engine anywhere, including the
file that defines the arithmetic. **What would falsify it:** a default expressed as something the
rule cannot see — a helper whose own signature carries the zero, which is what `moved` was.

**AND A CLAIM OF MINE THAT WAS FALSE, CORRECTED IN THE SAME CHANGE.** `docs/AUDIT.md` item 3 said
*"nothing in this world can read a company's accounts"* and that it therefore blocked item 4. It
does not. `publish` writes the whole statement — income lines, earned, revaluation, assets,
liabilities — to the JOURNAL as `reporting.report`, which is a kernel store any module may read, and
**five already read it**: `research` at three sites, `control/index.ts:145`, `corporate-bond`,
`reporting/guidance.ts`, and the observer. What the module keeps privately is only bookkeeping ABOUT
publishing, which item 0 declared a `noun` on the strength of the same false belief and which is now
declared `working`, with the correction written into the declaration. Item 3 shrinks to what is
actually wrong — five modules each pulling one fact out of `unknown` and re-checking its type by
hand, which is five parses of one fact (Law 4) — and it is re-ordered to follow item 4 instead of
blocking it. **Item 4 is takeable now**: published earnings and a per-party required return both
exist, and `control/worthAt` already combines them correctly.

**Measured, both ways.** `funds.test.ts` and `households.test.ts` are the two files this change can
reach: **7 red before it and the same 7 red after it, by name.** Not this item's, and not made worse
by it. The full suite was not run to completion — a year of the whole chain at four countries is
long past the point where waiting on it teaches anything (C-1 owns that measurement).


## Item 4 — the valuation door: what a claim is worth to somebody who requires something

**What.** `InstrumentKindProfile.worthTo(i, required, reads)` and `ParticipantView.worth(instrument,
requiredPerAnnum)`. One question, asked of every kind of claim, answered by the kind's own profile.

**Why it was the item the equity question was really about.** The instrument profile had ten fields
— `pricing, carry, owes, liabilityOfIssuer, unit, priceTick, validateTerms, displayName, due,
cashFlows` — and every one answers *what is this thing legally and how does it settle*. **Not one
answered why anyone would hold it.** `cashFlows` is the issuer's CONTRACTUAL PROMISE, so this world
had exactly one theory of value: discount the promise. A share has no promise. Everything followed
from that single absence — `funds.eligible` returned false for anything with no cash flows, so no
fund could ever hold a share; a household therefore valued a share at
`outlook('price.<id>').expected − confidence`, an extrapolation of the price's own history; and the
one earnings-based valuation in the tree, `control/worthAt`, sat inside a module that has never
produced anything. The equity market was a closed loop of price-extrapolators with no fundamental
side, which is why the price collapsed in the year-long run and why 12c patched an anchor at the
seed. **The anchor is a buyer.**

**Absent is an answer and not a default.** A kind that says nothing is saying its promise IS the
expectation, which is true of every contractual instrument, and the kernel discounts `cashFlows` at
what the holder requires — one day count for the whole door, the one `control` was already using
(Law 4). The share kind says otherwise: it capitalises what its company published, annualised by the
span of its own report, over the shares in issue. No multiple, no forecast, no second set of
accounts (Reporting A2.a). It is not a price and cannot become one (Law 3) — the profile answers the
stream, the PARTY brings its required return, and two holders requiring different things want the
same claim at different levels, which is the disagreement §46 A3 says a book is made of.

**`funds.eligible` was asking one question for two.** It tested `cashFlows` for both halves of a
mandate — what it may hold AND how long its money may be tied up — so anything promising no dated
payment failed the tenor test by having no last flow at all. The tenor test now applies where a
tenor exists, and what the fund can put a number on is asked of the valuation door. A claim it
cannot value is one it does not buy: a real refusal, rather than a property of whether the claim
happens to promise anything.

**What is NOT done, and it is not hidden.** No seeded fund has an equity mandate — `eligible` is
bills or grain — so the capability exists and nothing exercises it yet; giving a manager a mandate
that holds shares is a seed decision and belongs with 13o. And `liquidity` — how fast a claim
becomes money, absent from the profile entirely and half of every funding decision (**A-27**) — is
not built here. Both stay on the item.

**Tested where the arithmetic lives.** A listed line is DRAWN and the scale model draws none, so the
share's answer is exercised against the profile directly rather than by waiting for a world to
produce a company with four published quarters. A bond's worth falling as the holder requires more
is tested on the world, because that path runs there.

**Forecast, with its killer.** A dealer, a fund and an acquirer can now form a view of a share from
what its company earns, and they will disagree with each other and with the tape. **What would
falsify it:** the first world where a fund holds shares showing that a capitalised published quarter
is too noisy to bid on — in which case what is missing is a smoothing that is itself a preference
somebody declares, not a change to this door.

**Measured.** `funds.test.ts` — 2 red before this change and the same 2 after, by name. Green: lint,
typecheck, spec citations, forbids, plan progress, `worth.test.ts` 5 of 5.


## Item 5 — a receipt says what money is to whoever gets it

**What.** `MoneyLeg.receipt` — a closed union the ledger owns, stated by the payer — and a treasury
that dispatches over it instead of inferring from the wire.

**What it replaced.** The income-tax base was every money leg into a household, from anyone but the
treasury, that was not a `coupon`. The only discriminator in the whole tax was
`r.instruction.cause === 'coupon'`, and `Cause` is nine SETTLEMENT labels for why bytes moved. So a
household was taxed at the wage rate on gross share-sale proceeds, on a maturing bill's principal, on
a fund redemption, on a probate distribution and on a loan drawdown. Borrowing was income.

**THE PAYER SAYS, because the payer knows.** A wage phase knows it is paying a wage; the taxman
working it out from the shape of the wire is the defect. Five sites now say what they pay: wages
(`labour/matching.ts`), coupons (`world/actions.ts`), deposit interest (`money-market/deposits.ts`),
a returned principal at maturity, the treasury's own standing mandate, and an estate being divided.
A receipt nobody classified is **not taxed** and is counted — `bases.unclassified` is published in
the receipts event, so the size of the remaining gap is a number rather than a silence.

**The measurement, and it is the whole argument.** In the scale model at six periods the old income
base was **11,519,429**, and **not one penny of it was a wage**: 132 state transfers under the
standing mandate and 18 estates divided by probate. The income tax was a tax on benefits and
inheritance. After classification `unclassified` is **0** — every penny reaching a household in this
world is named — and the income base is **0**, because no household here is paid a wage at all. That
zero is a finding about the world, not about the tax. Interest is unaffected and real: 14,313,339 of
base, 2,835,724 collected.

**One naming decision worth recording.** The union's tag is `of`, not `kind`. `kind` in this engine
means a REGISTRY kind and `phoenix/no-kind-branch` enforces Law 15 by that spelling — it fired on the
switch, correctly, because it cannot tell a closed union from a registry kind. Renaming the tag keeps
the rule absolute with no disable anywhere, and says at the point of use which of the two things this
is. An exhaustive switch with `assertNever` over a ledger-owned union is what the error discipline
asks for.

**Carried, and named.** A `disposal`'s gain is `proceeds − basis` and the arithmetic is in place, but
nothing emits a `disposal` receipt yet: the market settlement holds the basis at the debit and does
not pass it along. That is the next step on this item and it is what finally makes A-37 ("selling an
asset counts as income") false rather than merely untested. A capital-gains rate distinct from the
income rate, and a dividend rate, are POLICY primitives the polity owns (item 14).

**Forecast, with its killer.** From here the tax base is what payers declared, and a new payment
route cannot silently become taxable income. **What would falsify it:** `unclassified` climbing as
sectors come alive — which would be the read working, not failing.

**Measured.** `households`, `treasury` and `estate` — the three files this reaches: **10 red before,
the same 10 after, by name.** Green: lint, typecheck, spec citations, forbids, plan progress, 21
tests across the four files this work has added.


## Item 5, second half — the gain, and a distinction the union was missing

**What.** `Settled.realised`: settlement publishes, for each disposal, what came in beside what the
units had cost. The treasury reads the gain from there. And a ninth receipt kind, because the first
run proved eight were not enough.

**Why settlement and not the leg.** A buyer paying a seller knows the money is proceeds of a sale.
Only the register knows which lots the seller's debit drew and what they were carried at. Neither
half can state the other, and having the treasury pair them afterwards would be re-deriving what one
pass already knew (Law 19). Settlement does both in the same pass, so it puts them side by side and
nobody computes either twice.

**The first run returned nothing, and that was the finding.** Twenty-two money legs were labelled
`disposal`, all twenty-two settled, all twenty-two paired with an asset leg from the seller — and
`realised` was empty. Because `expandAsset` checks `issuedBy(inst, leg.from)`: **a seller that issues
the line is selling what it MADE**, which settlement expands as an ISSUANCE. No debit, no lots, no
cost — there is nothing it cost, because it was produced. Asking for a basis asked for something that
cannot exist.

So selling what you made and selling what you held are two different receipts, and calling both a
disposal was my error in the union, not a defect in the world. `sale` is revenue; `disposal` is
proceeds against a basis. The market picks between them on whether the seller issues the line.

**The census of what this world pays**, six periods, scale model: **264 interest, 150 transfers, 22
sales, 15 returns of capital — and nothing else.** No wage is paid, no dividend reaches anybody,
nothing held is ever sold, nobody borrows. Four of the nine things money can be to a party happen
here. `unclassified` is 0, which is what makes that a census rather than a sample — and it is a
sharper statement of what this world does not do than anything in the original read.

**Carried to 14.** A capital-gains rate distinct from the income rate, a dividend rate, and a tax on
what `sale` revenue nets to — a firm's profit — are policy primitives the polity owns. So is what a
realised LOSS does: carrying one against other gains is a fiscal rule, and inventing one to fill the
branch would be an outcome written as a rule.

**Measured.** `households`, `treasury`, `estate`: 10 red before, 10 after, same names. Green: lint,
typecheck, spec citations, forbids, plan progress, `receipt.test.ts` 6 of 6.


## Item 6 — a belief has a subject, and one of them is another party

**What.** `OutlookVariable` is branded and can only be made by `about(subject)`. `Subject` is a
closed union — `price | bought | sold` of an instrument, a party's own `income` or `earnings`, and
**`credit` of another party**.

**Why.** It was `export type OutlookVariable = string`. A party's whole belief system was a bare
string namespace, so the only belief this world could express was an extrapolation of an observable.
Twenty-five call sites, and **not one of them was about another PARTY** — which is what a probability
of default is, and a rating opinion, a dealer's adverse-selection charge, a depositor's confidence
and an acquirer's view of a target. Appendix B requires one PD model per borrower; there was nowhere
for one to live, so there is none, and this world has 96 loans across 9,006 firms.

**Two tells were already in the tree.** `control/index.ts:179` reached for a variable through an
`as never` cast — somebody fighting a type that could not say what they meant. And `options`
recovered which instrument a belief was about by `variable.startsWith('price.')` and
`variable.slice(...)`: a fact taken back out of a string, which is exactly A-52's shape. Both are
gone. The cast, because the constructor makes the key. The surgery, because the view hands back
`outlookSubjects()` — the things themselves — and `PRICE_OF` is deleted, which is the code this
change removes rather than adds (Law 12).

**One writer, one reader.** `about` makes the key and `subjectOf` reads it, in the same file, and
nothing else anywhere makes or parses one. The compiler generated the migration list: sixteen module
files and four test files, every site converted, no cast surviving.

**What this does NOT do, and it matters.** `credit` is expressible and **nothing forms one yet**. A
bank's credit decision reading its own view of a borrower is a mechanism, not a type, and it is what
would close A-31 and the lending finding. Folding `ratings`, `research` and `banks/reserves` into the
noun — three private stores that are this thing in three shapes — follows the same way. The test
asserts `credit` is absent from what the world currently holds, so the day one appears, the assertion
says so.

**A regression of my own, found and fixed here.** `doors.test.ts` had been red since item 0: its
fixture modules keep their own stores and the ontology register rightly refused them. I measured item
0 against `nouns.test.ts` and `world.test.ts` and not against the file whose subject is the module
doors. Two tests green again, and a sweep confirms it was the only file affected — nothing else in
the suite keeps a store of its own.

**Forecast, with its killer.** A belief under a name nobody declared can no longer be written, and
the test walks every belief a real world holds to prove it. **What would falsify it:** a subject the
closed union cannot express turning up as a genuine need — most likely a belief about a THING that
is not an instrument, such as a place or a line of business, both of which are nouns items 12 and 11
are about.

**Measured.** The five files this reaches: **7 red before, 5 after** — the two it fixed are the ones
item 0 broke. Green: lint, typecheck, spec citations, forbids, plan progress, `view.test.ts` 5 of 5,
`doors.test.ts` 19 of 19.


## Item 7 — the states between alive and dead

**What.** `Standing` on the alive branch of `PartyStatus` — `good | distressed | inResolution |
winding` — with a transition door on `Parties` and on `MechanismContext`, journalled as
`party.standing` carrying what it was, what it is and why.

**Why.** `PartyStatus` was `{alive:true} | {alive:false}`. Binary. **A bank in resolution was
`alive: true`**, the same value as one nobody had a claim against, so nothing reading a counterparty
could tell them apart. Everything an economy does between those two words had nowhere to be, and
each mechanism that needed one built half a lifecycle of its own: `estate` keeps a `Winding` record
in a private bag because there was no state to put it in, XI-1 publishes a default that changes no
status, and §25's resolution runs over a party the type says is fine.

**Where it went is why it was cheap.** 129 readers of `status.alive`, 13 constructions. Adding the
state to the ALIVE BRANCH means every reader still means exactly what it meant, and the compiler
made all 13 constructions say a standing. Nothing was rewritten and nothing was guessed.

**Wired at both places that needed it, because a type with no writer is what item 1 exists to
catch.** The estate moves a party to `winding` before ceasing it — the interval where it still holds
things and its book is being moved. `resolve()` moves a bank to `inResolution` the moment a trigger
fires.

**Not a ladder, and the test asserts it.** Banks Capital C1.a: a capital trigger takes a bank from
`good` straight to `inResolution` with no missed payment anywhere, and a resolution can end with the
bank still trading. Insisting on distress first would be an outcome written as a rule.

**Carried, with reasons rather than silence.** **A-17** — three of XI-15's five weight events never
fire and nobody is ever born — needs a BIRTH mechanism, not a state; worklist 13n. **A-36** — the
treasury's immortality is unconditional — is `fails: []` on the party kind, and what Appendix B says
should fail is a sovereign in a FOREIGN money: a currency-layer mechanism, not a standing. And
`distressed` is declared with no writer: XI-1 publishes a default that changes no status, and joining
those two is the next step on this item.

**Forecast, with its killer.** A counterparty's condition is now a fact the world can read, so a
depositor, a lender and a dealer can act on it. **What would falsify it:** four states turning out to
be too few — most likely a party that is solvent but cannot pay today, which is a liquidity state
distinct from `distressed`, and Banks Funding is where that would show.

**Measured.** `estate`, `bank-resolution` and `world` — **11 red before, 11 after.** Green: lint,
typecheck, spec citations, forbids, plan progress, `lifecycle.test.ts` 5 of 5.


## Item 8 — the bilateral commitment that is not an instrument

**What.** `register/agreements.ts`: two named parties, a currency, what is owed, what it is, why it
exists, and a state — `performing | breached | discharged | terminated`. Indexed both ways. Written
through `owes` / `paidOn` on `MechanismContext`, read through `world.agreements`, a real read-only
facade like the register's.

**Why.** An employment, a lease, an invoice, a repo, a stock loan, an insurance policy, a mandate and
an overdraft facility are ONE NOUN, and seven modules each invented their own book of it in
`ctx.state` — none visible to the kernel. So there was no answer to "what does this party owe that is
not an instrument", and that question is what an estate divides. What the absence actually cost was
not abstract: **four mechanisms had a payment fail, wrote a number in an event, and let the claim
evaporate between two balance sheets.**

**A write-off is not a discharge, and that is why this is a state machine.** `terminate` leaves what
is owed standing and says the commitment ended. Calling that a discharge would say somebody was paid
who was not — which is the entire difference between a write-off and a settlement, and a `boolean`
cannot hold it.

**The four, and a fifth found while measuring.**

- **D-1** — a failed levy incremented `unpaid` on `treasury.receipts` and nothing carried it. The
  treasury now holds a claim on the payer.
- **A-41** — an unpaid wage left no obligation anywhere, and `severanceRanking` recorded **`0`, "nothing
  owed"**, for exactly the case that needed it: a trading employer whose severance payment had just
  failed. Both halves are closed — the claim exists, and the field says what is unpaid rather than
  what is convenient. Its third half was a comment: `shed` said "oldest row first" over code that
  has always done newest-first. The code is the ordinary redundancy convention; the comment was
  wrong and is now what the code does.
- **E-1, new** — measuring the above, a world where no bank lends failed **18 rating fees in fourteen
  periods**, every one written as `rating.unpaid` and carried by nothing. The same hole, in a fourth
  module nobody had looked at. One call to the same door.
- **A-20** — a failed probate transfer became an **unhandled throw that stopped the world**.
  `handToProbate` discarded what `settle` returned; settlement is atomic, so on a fail nothing moved,
  the estate still held everything, and `dieCell`'s "no death without a destination" threw a
  `PhoenixError` the engine never catches. It reads the result now: what did not arrive is owed by
  the estate to the office, and an estate that still holds does not die — it is `winding` (item 7's
  own state, which is why item 7 came first).

**A-62 was mis-assigned by me and the finding is better for it.** A central bank's accumulated loss
is owed to NOBODY. No creditor, no agreement. The real cause was in `lastRemittance`: it read
`centralBank.loss` events and advanced its window past the loss it had just found. Removing that read
is the fix, and it removes code (Law 12) rather than adding a noun that would have been a lie.

**Measured.** Confiscatory-tax world (income tax at 20 — a POLICY set past what a payer holds, so the
levy failing is an OUTCOME): **11 failed levies, 11 arrears, and `Σ owed` equals `Σ unpaid` exactly**,
which is the Law 4 check that keeps the number in the event honest. No-lending world: **18 rating
fees failed, 18 claims, same equality.** The ordinary scale model exercises neither — **0 failed
instructions in 14 periods**, where D-1 was originally measured at 855 — so these branches are
reached by worlds built to reach them, and item 1's reach read is what publishes that about the
ordinary one. **The whole suite, before and after: 79 red, and the same 79 test-for-test** — 638
green becomes 647, the nine being this item's own. Green: lint, typecheck, spec citations (201),
forbids (198 files), plan progress.

**What is NOT done, and it is the larger half.** The seven private books are still seven private
books. They WORK. Migrating them is seven bounded changes, not one, and doing them here would have
been seven items in one commit (Law 14). `Mandate` — and with it the fund/pool/manager split, and
**B-2**, **A-9**, **A-67**, **B-3** — rides on that migration and stays open. What this item
establishes is the noun and its store, which is what those seven had nowhere to migrate to.

**Carried, and one of them is new and is mine.** `FailReason`'s discriminant is still `kind`, so
reading it needs the eslint-disable that `Receipt`'s `of` does not — item 17. `breached` has no
writer: nothing has yet decided when an arrear becomes a breach, which is fiscal policy with an owner
(Polity D1) and belongs with item 14. And **E-2**, which A-20's fix creates: an estate left `winding`
is still a household cell to the eleven places that iterate `ofKind(HOUSEHOLD)` asking `status.alive`,
so a cell of dead people would go on consuming and looking for work. It is the better of the two
states — the world does not stop, and the fact is written where a reader can find it rather than
being a throw — and it is unreached in the scale model (**0 failed probate transfers, 0 encumbered
household holdings over 12 periods**). An open probate is a multi-period procedure, so it is
positioned at item 14.

**Forecast, with its killer.** Every unpaid obligation in this world now has two named sides and a
size, so an estate can divide one and a solvency test can see it. **What would falsify it:** an
obligation with more than two sides — a guarantee, which is a third party standing behind a second —
turning up as something this store is asked to hold. That is item 13, and it is separate precisely
because this shape cannot express it.


## Item 3 — a typed read of what is public

**What.** `journal/published.ts`: `PublishedStatement` and `PublishedGuidance`, exposed as
`published` on **`KernelReads`** — so a participant holds it too, because published accounts are
public and a bank valuing a borrower is supposed to have seen them (Observer A3). Nothing
unpublished is reachable through it.

**Why.** §48's output was never unreadable — `publish` records the whole statement to the journal
and the journal is a kernel store. What was missing was a TYPE. So **eight sites each pulled
`unknown` out of `e.data`, checked it by hand, and recovered a different subset of the same fact**:
`world.ts`'s `worthReads`, `control`, `corporate-bond`, `reporting/guidance`, `research/index` twice,
`research/estimate` twice, and the observer. Four of them did `continue` on a record they could not
parse. The observer did worse: `nums(v) = typeof v === 'number' ? v : 0`.

**A dropped report is a `?? 0` in disguise**, because every one of those readers treats "not there"
as "never published". A statement whose `assets` went missing would have vanished from the covenant
test, stayed visible to the analyst, and shown a reader a company with no assets — and nothing
anywhere would have said so. `reporting` is the one writer of the event (Law 4), so a field it did
not write is its defect, and the read throws and names it at the site.

**It removes code, which is how you know it is the cause and not a symptom (Law 12).** 148 lines
deleted for 79 added across the eight sites; `control`'s `Published` and `corporate-bond`'s `Said`
gone as duplicate local types; the observer's `lastOf` helper deleted because the typed read answers
what it existed for. `guidanceRecord`'s parameter narrowed from "anything that can read the journal"
to the published read, which is a smaller surface than it had.

**`periods` is derived at the read** from the two dates the writer published, never stored beside
them (Law 19) — which is what four of the eight sites were separately recomputing as `to - from + 1`.

**Measured.** `reporting`, `research`, `control`, `corporate-bond`, `world`, `doors`: **7 red before,
7 after.** Six new tests. The last of them walks every statement a 30-period world publishes and
asks for every field — an assertion the hand-parses could not make, because a malformed statement
was invisible to readers that had all agreed to look away. Green: lint, typecheck, spec citations
(202), forbids (199 files).

**The item's own exit criterion was stale and is corrected in the plan.** It said "reporting keeps no
private state". Its bag is bookkeeping ABOUT publishing — which quarters are done, what each said so
a later disagreement is a restatement, the standing guidance — which is working state and was
re-declared `working` at item 0. What was missing was never the readability; it was the type.

**Forecast, with its killer.** Any module can now read a company's accounts without knowing how they
are written down, so a valuation, a covenant test and an estimate all read the same statement. **What
would falsify it:** a reader needing a line the statement does not carry — most likely a cash-flow
line, which `publish` writes as `cash` and this read deliberately does not expose, because no reader
has yet asked for one.


## Item 9 — who controls whom

**What.** `register/control.ts`: a controller, a subject, since when, and on what basis —
`shares | contract | appointment | resolution`. Written through `takeControl` / `releaseControl` on
`MechanismContext`, journalled publicly, read through `control` on `KernelReads` — so a participant
reads it too, because who owns a company is the one thing about it everybody knows (Observer A3).

**Why.** Grep `subsidiary|parentOf|controls|consolidat|group` across `register/`, `parties/` and
`registry/`: **zero hits.** Owning 51% of a company was a large holding and nothing more. So a
takeover bought the shares and nothing happened, no group consolidated, nothing could be
ring-fenced, a resolution could not transfer a subsidiary, and **private equity — which is
definitionally about control — was inexpressible.**

**It refuses the three things that are not control**: a party controlling itself (Law 5), a second
controller for one subject (Law 4), and a cycle — a group whose parent is its own subsidiary has
nobody at the top, so `ultimateOf` would not terminate and a consolidated sheet would count one
balance sheet twice.

**A group consolidates, and it is the SAME read.** `consolidated(view, group)` is `balanceSheet`
generalised with an elimination set, not a second implementation — a second one would be a second
set of accounts able to disagree with the one the audit proves (A2.a). A claim one member holds on
another comes out of the holder's assets AND the issuer's liabilities, and a contract between two
members comes out of both sides.

**And the two amounts are not the same, which is the economics and not an error.** The asset side
comes off at the holder's MARK and the liability side at what the issuer OWES. A parent holding its
subsidiary's paper at 80 against a face of 100 has, on consolidation, retired its own debt at a
discount, and the group's net worth is 20 above the two sheets added. Taking the same number off
both sides would be marking a liability to the market — the fiction the balance-sheet read removed.

**`combine` has a caller (A-70, B-4).** A completed tender reads what the buyer holds against what is
in issue. **More than half is a subsidiary** — control is taken and the target goes on trading, which
is what a majority actually buys. **Holding all of it is a combination**, because then nothing is
left outside and the residual is entirely the acquirer's, so combining describes what is true rather
than a decision somebody has to take. "More than half" is arithmetic over two reads; no threshold
was declared anywhere.

**And it no longer lies.** Its docstring said "its holdings are reseated" and nothing in it moved
the target's holdings: `assume` maps to `reseat`, which changes an instrument's ISSUER. It hands the
whole balance sheet over first, in one atomic instruction, then assumes the liabilities, then
ceases. **Measured**: with the hand-over removed the target keeps **9 holdings** and the `names`
family reports it **5 times in 8 periods**; with it, the target ends empty and dead and the family
is silent for 8.

**What it does when it cannot finish.** Units left after the hand-over are encumbered — a lien is not
free and the register refuses to move bound units — so the acquirer has bought a company whose
assets are pledged elsewhere. It does not die: it stays a named party under the acquirer's control,
recorded `combined: false`. Ceasing it would write a residual with no holder.

**Unreached in this world, and the reason was already a test.** Every listed firm here publishes a
loss, so there is no stream to capitalise and no bid is made — `control.test.ts` asserts that and
says the day one earns, the tender machinery starts. The doors and `combine` are therefore exercised
by a scale-model module that takes control of one drawn firm on behalf of another and combines it
two periods later.

**Measured.** `control`, `world`, `doors`, `equity`: **10 red before, 10 after.** Twelve new tests.
Green: lint, typecheck, spec citations (203), forbids (200 files).

**Forecast, with its killer.** Ownership and control are two facts now, and a group is a thing the
world can name — so consolidation, ring-fencing and a resolution that sells a subsidiary all become
writable. **What would falsify it:** control that is not a tree — a joint venture two parents control
between them — which this store refuses by design, because one controller per subject is what makes
`ultimateOf` and the consolidation well defined.


## Item 10 — what a company does to its own claims

**What.** `register/corporate.ts`: an issuer, a line, a kind
(`dividend | split | rights | buyback | spinOff | merger`) and **four dates** — announced, ex,
record, payable — with a state that walks `announced → recorded → paid` and will not skip a step.
Written through `announce` / `recordAction` / `payAction` / `cancelAction`, journalled publicly, read
through `actions` on `KernelReads` because a share trading EX is a fact every buyer has to know.

**Why.** `corporateActions` was a PHASE NAME. The only data anywhere was two booleans on the
instrument kind and a `cause` on the wire that `freight` also uses. **Period 5 settled 249,288
instructions of which 162,615 were dividend payouts — 65% of everything the world did** — not
because dividends matter that much, but because there was no declaration to be separate from the
payment.

**It refuses the dates out of order**, because every consequence depends on the order: a record date
before the ex date pays the seller of a share that had already gone ex; a payable date before the
record date pays before anybody knows who is owed.

**A plan is not a declaration.** `equity.decide` still runs every period — what a firm has spare is
its own funding read and that is weekly — but a board declares on **its own fiscal quarters**, the
same ones it reports on. So `fiscal.ts` and `anchorOf` moved to the kernel calendar: a fiscal quarter
is a fact about the CALENDAR and reporting was only the first thing that needed one. Leaving it in
`reporting` would have made `equity` derive its own year ends, and one company would have had two.

**Between the record date and the payment it is a LIABILITY, and that noun already existed.** On the
record date the holders stop being a date and become named parties, so the module opens an
`Agreement` per holder — item 8, used for what item 8 was for. A holder that dies before the payment
does not lose it; a payment that fails leaves the claim standing.

**And the payment reads the CLAIM, not the register.** A shareholder that sold the day after the
record date is still owed and the buyer is not; re-deriving from what people hold on the payable date
would pay exactly the wrong people (Law 19). What will not divide into whole pieces per member of a
cell whose weight has since changed stays owed rather than being rounded away.

**Measured.** Forty periods of the equity rig world: instructions **61,788 → 53,266**; dividend
payout legs **8,538 (13.8%) → 308 (0.58%)**; declare-and-pay events **38 → 2 declarations, 2 record
dates, 1 payment**. Plans carrying a positive dividend: 38 → 39, because the plan never stopped being
weekly. `equity` before and after: **5 red, the same 5 test for test.** Eight new tests.

**Two tests changed because their premise was the defect, and one of them found something.** The
cadence test ran six periods and found a payout; it now runs forty and asserts the three dates. The
waterfall test relied on a listed firm being bled dry weekly — with the cadence fixed, **no listed
firm dies in 60 periods** with a deep buyer in either of their own lines, because a listing is what a
LARGE firm gets and a deep buyer kills the marginal firm in a line, not the biggest. It now puts the
reason in the world (a claim bigger than everything the firm holds) rather than waiting for one.

**Found while measuring — E-3, positioned at item 17.** `estate.<firm> rents 1 of space from <firm>`
is reported by the estate's own `flows` family as "paid X to Y, who has no claim on it". The estate
is paying for the space its inventory sits in while it winds up. That is a cost of a winding-up, not
a distribution, and D6 is about distributions — the check forgives `corporateAction` and nothing
else. Named in the two tests that reach it rather than forgiven wholesale, so anything new still
fails them.

**13k is answered and was never periodicity.** `Periodicity` is a proper union in `core/rate.ts` and
the fiscal calendar was built and unused. What was missing was this noun. The rating fee and the tax
assessment remain genuinely periodicity and stay in 13k.

**Carried.** The buyback is still decided weekly out of this week's spare cash: a buyback is an
announced PROGRAMME executed over time, and `buyback` is a kind this store already has — it needs
item 14's `Process`. `split` and `rights` likewise have a kind here and no caller yet.

**Forecast, with its killer.** A dividend has an announcement, an ex date and a payment, so total
return and price return can differ and a share can trade ex. **What would falsify it:** the ex date
not actually moving the price — nothing yet reads `actions.exToday` when forming an opinion of a
line, so the price move D3.b describes is a consequence this makes possible and does not yet produce.


## Item 11 — stock or capacity

**What.** `GoodDecl.output: 'stock' | 'capacity'` — whether a line's output can be HELD. A stock line
makes a thing that waits and is sold later; a capacity line has nothing to wait, because an hour of
teaching, a night's lodging or a diagnosis is made where it is bought and at the moment it is bought.

**Why, and the case that proves it.** `portable` was declared to be "the one fact that divides a
manufacture from a service". It is a SHIPPING fact. The two coincide for the sixteen services and
**come apart at electricity**: power is the most movable thing in the file and nobody stores a
megawatt-hour. Reading one for the other is what had a school holding unsold teaching hours as
inventory and writing them off.

**Eighteen declared numbers are gone (Law 2, "count must fall").** `spoilagePerPeriod: 1` eighteen
times was a shape standing in for the mechanism `output` now names. A capacity line's spoilage is 1
by construction and `spoilageOf` derives it from a two-entry dispatch table; declaring one anyway
throws, and so does a stock line that declares none — a contradiction, not a hint.

**A-64 is closed, all three parts.** A lease now buys ROOM: `capacityFrom` takes what a party rented
this period beside what it owns, so a firm that bid, won and paid the letter is no longer short of
exactly the same room next period. A failed payment no longer takes room — the decrements sat
outside the settled branch, so one insolvent taker could shut a region's storage market for a period.
And the `commodities.leases` store is DELETED: a `ctx.state` map whose own comment said it was
emptied every period, which `ctx.state` never is and nothing did, and which nothing read. What each
taker rented is one public event per taker per period carrying the total; the per-match facts keep
their own kind, because who let it to whom is real and the letter's income has a payer.

**Measured**, forty periods of the equity rig world: **27 matches → 25 taker-period totals** (two
takers matched two letters each), the same 27 units of space and the same **$3,996,537,468** paid,
and the capacity read now counts it. **No firm's plan was bound by storage in either run**, so no
output changes: the fee buys room now, and in this world nobody was short of it. `goods`, `firms`,
`freight`, `storage`: **7 red before, 7 after.** Seven new tests. Green: lint, typecheck, spec
citations (204), forbids (201 files).

**What is NOT built, and it is named rather than implied — E-4, positioned after item 14.** A
capacity line still starts a batch into WIP and the unsold part is destroyed at the end of the
period. The number is right and the ACCOUNTS LINE is not: an inventory write-off where it should be
operating leverage on fixed cost. The fix is produce-to-order — a capacity line posting availability
into the session and making only what clears — and it cannot be done here: `firms.produce` anchors
after `labour.pay` and therefore before `markets`, so a line cannot know its demand when it produces,
and reordering for one kind of line would be a kind branch.

**Forecast, with its killer.** A service and a warehouse are now different things in the type system,
and a lease is consideration for something. **What would falsify it:** a line that is neither — a
thing that can be held but only briefly, where the honest answer is a fast spoilage on a stock line
rather than a third case.


## Item 12 — land with a supply and a price

**What.** `mechanisms/land/`: one line per place, its issue the place's effective buildable area,
held by named parties from the first period, traded in a market the kernel clears like any other.

**Why.** `RegionDecl` was `{id, name, country}` — three fields, no profile, no behaviours, no supply.
A place was a label. Land could not be owned, used up, run out or priced, so a tile carried its
thousandth building at the price of its first.

**Two declarations were already there and nothing read either.** `TerrainDecl.buildKm2` — what
putting up a unit of plant on ground like this takes, against ordinary flat ground at one — has been
declared for every terrain since the map was written and was read by nobody. And
`CapitalKindDecl.landPerUnit` was read only by the yield arithmetic, which asks how good the MARGINAL
hectare is and never whether there IS one.

**The supply is a read, not a number**: `Σ over tiles Σ over grounds (share × tileKm² ÷ buildKm2)`,
the same shape as `tileYield` beside it, so a km² of open water at twelve times the cost is a twelfth
of a km² of buildable place. Computed where used, never stored.

**It is not a capital kind and it is not a claim.** A capital kind is made from a good and has a
life; land is neither, and a fake life would put a wearing-out nobody pays into the capital charge.
It is `physical: true` with no issuer — the ground is nobody's promise.

**It is carried at cost, and that is Seed C4 read honestly.** A world that opened with every state
holding a continent would have had to SAY what a hectare was worth before anybody had paid for one —
a written price for the largest asset in the world, which Law 3 forbids and which `OPENING_SHARE`'s
invariance argument cannot cover, because an area is a physical fact and not a unit anybody chose.

**The price is cleared and there is no formula for it anywhere.** The state offers what it holds at
one piece of money — the least there is, not a posted price — and a firm bids what it has for the
ground its own plant is standing on and has not bought.

**Measured**, twelve periods: `us.1` **113,281,635 hectares** issued to the state, **16,502 hectares
traded**, **eight holders** where there was one, cleared at **$0.01** — the reservation, because
supply is four orders of magnitude above demand. Land here is nearly free, which is right for a
continent with a hundred firms on it, and it is an ANSWER now rather than an absence: when demand
grows the price rises with no code change. `world`, `firms`, `storage`, `doors`, `goods`: **10 red
before, 10 after.** Four new tests. Green: lint, typecheck, spec citations (205), forbids (202).

**Two things the build found and fixed at their cause.** A bid for less than one hectare asked what a
firm would pay for a ten-thousandth of one and got 9.6 × 10²⁰ — past what an integer holds, and the
run stopped where it should have shrugged (Law 8: a piece is the number). And the state selling
ground in a place it does not sit in reads as a CROSS-BORDER trade in the balance of payments, when a
hectare of us.2 sold to anybody is still in us.2.

**So the state sells only in its own place — E-5.** It holds the ground of every place in its
country, so nothing is a residual with no holder; what is missing is a party PRESENT in each place to
sell its ground. That is a local authority, the same noun a port and a planning consent need, and it
is where the rest of 13m starts: a port with an owner and a berth, commercial property, leases with a
term, CRE lending, a retail firm that sells from somewhere. Each is now expressible and none is built.

**Forecast, with its killer.** Ground is finite, owned and priced, so building can be made to consume
it and get dearer. **What would falsify it:** the price staying at the state's reservation for ever —
which it will while a continent has a hundred firms on it, and which the reach read publishes rather
than hides.


## Item 13 — a third party standing behind a second

**What.** `register/guarantees.ts`: a guarantor, an obligor, a beneficiary, what is guaranteed, a
currency, a limit and a basis — `insurance | parent | sovereign | clearingHouse | letterOfCredit`.
States `standing → called → exhausted`, with `released` beside them. Written through `guarantee` /
`callGuarantee` / `releaseGuarantee`, journalled publicly, read through `guarantees` on
`KernelReads` — a guarantee nobody can see guarantees nobody.

**Why item 8 could not absorb it.** An agreement is two named parties. What makes a guarantee a
guarantee is that **the party who pays is not the party who owes**. The only thing of this shape in
the codebase was `Contracts.novate`, for derivatives.

**A limit here is not a bound (Law 6).** "Insured up to a limit per member" is a TERM of the promise
— a policy somebody set, with an owner — not a clamp on a computed number. `null` is a guarantee
with no limit, which is what a parent gives a subsidiary, and it is a real answer rather than a large
number.

**`exhausted` is not `released`**, and a call past the limit is refused: a guarantor paying more than
it promised would be paying somebody else's obligation, and what the guarantee cannot meet is what
the next thing behind it is for (D5's purse) rather than something absorbed quietly here.

**Wired at the one real case, and it was working by accident.** Deposit insurance already collected
premiums and paid out — as an ordering of payments written into the resolution path rather than as a
thing anybody holds. So nothing could be asked who stood behind a bank, a guaranteed deposit ranked
in an estate exactly like an unguaranteed one, and LOLR's four classical conditions had nothing to
attach to. The insurer now gives the guarantee when a bank first pays a premium — which is when the
cover starts, so a bank that has never paid is not insured, the same fact read from the other end —
and the resolution calls it for what it paid.

**Measured**, eight periods: **one guarantee per bank, public, `standing`**, and no bank has two.
`money-market`, `bank-resolution`, `capital`: **6 red before, 6 after.** Six new tests. Green: lint,
typecheck, spec citations (206), forbids (203 files).

**Carried.** The other four bases have a key here and no caller: a parent behind a subsidiary needs
item 9's `Control` to say which parent — now sayable, and nothing says it; a clearing house needs one
to interpose; a letter of credit needs trade finance; and the sovereign backstop is the purse, which
pays in the resolution path and does not yet do it under a promise.

**Forecast, with its killer.** Who stands behind whom is a fact the world can read, so a lender can
look through to a guarantor and an estate can rank a guaranteed claim differently. **What would
falsify it:** a guarantee whose guarantor is itself guaranteed and where the chain matters — the
store answers `behind` one level at a time and nothing walks it, which is fine until a clearing house
stands behind a member that stands behind a client.


## Item 14 — a multi-period state machine

**What.** `register/processes.ts`: a named procedure about a named subject, with ordered steps and a
period it must be over by. Written through `beginProcess` / `advanceProcess` / `endProcess`,
journalled publicly, read through `processes` on `KernelReads`.

**Why.** `Winding = {dead, opened, closesAfter, closed}` — four fields in one module's bag, and the
only thing of its shape in the codebase. A construction project, an auction cycle, a tender period, a
rights issue, a resolution and a restructuring are the same shape, and each would have invented its
own version in its own bag, none visible to the kernel and none able to answer "what is this party in
the middle of".

**The steps are declared because a boolean could not say where it was.** `Winding` could say started
and finished, which are the two states an estate spends none of its time in. An estate is
`selling → paying → dividing`. Leaving the last step is ENDING rather than an off-by-one, and
`closed` and `abandoned` are different facts.

**The settle loop walks `ctx.processes.running('estate')`** instead of a private list, so the module
no longer keeps a second answer to a question the world already has one of — and it was the answer
nobody else could reach.

**A-21 is closed, and the fix is not another choice.** `heirOf` was `.find`: the first cell the
parties store happened to return, of the first cohort. Every estate in a (region, bank) went to that
one cell and to no other, so **one household cell in each region accumulated the wealth of everybody
who died there and the rest inherited nothing, ever** — and which cell it was depended on insertion
order, a seed-draw artefact rather than a fact about the world. It is `heirsOf` now: every surviving
cell where the dead lived and banked, in proportion to how many people each stands for. **Nobody is
picked.** The share is the population, and a region whose cells are all the same size divides equally
because that is what its cells are, not because a rule says so. What will not divide into whole
pieces stays with the office until enough of it has arrived.

**Measured**, fourteen periods: **14 divisions to 1 heir → 304 divisions to 35 heirs.** `estate`,
`households`, `world`: **14 red before, 14 after.** Six new tests. Green: lint, typecheck, spec
citations (207), forbids (204 files).

**Carried.** The `?? ''` in the old `heirOf` went with the function. `E-4` (a capacity line producing
to order) and `E-2` (a `winding` estate still a household cell to eleven readers) both wanted this
noun and now have it; neither is built on it yet. The estate is the only declared process — a
construction, a tender and a resolution are each one line of `beginProcess` away and none is written.

**Forecast, with its killer.** A procedure has a place it has got to and a date it must be over by,
which anybody can read. **What would falsify it:** a procedure whose steps are not a line — a
resolution that can go back to valuing after an acquirer walks away — which this walks forward only
and would have to be a set of transitions rather than a list.


## Item 15 — what a party is for

**What.** `PartyKindProfile.objective`, required, one of six:
`theResidual | itsMembers | itsFranchise | itsMandate | itsOffice | itsBook`. Read through
`objectiveOf(party)` on `KernelReads`. Fifteen kinds declared it.

**Why.** `PartyKindProfile` had six fields and every one was balance-sheet — how it is represented,
whether it issues money, what it can fail on, whether it borrows, what sort of depositor it is.
**Nothing said what a party is FOR.** A firm maximised nothing, a bank had no franchise to protect,
a manager had no career: every participant's reason was hard-coded inside its own module's
`orders()` — twenty-one private answers to "why does this party do anything", none declared, none
comparable, none checkable.

**The compiler is what asks.** A kind cannot be added without saying what it is for, which is the
whole of what a required field buys.

**It is a declared PREFERENCE and not a utility function (Law 2).** Nothing is maximised and nothing
takes an argmax over it. An objective the engine optimised would be the representative agent this
world does not have, wearing a different name — and a test asserts that no event anywhere publishes
a utility or a score against one.

**There is no `itsOwners` beside `theResidual`**, deliberately: the residual IS the owners' claim,
and two words for one thing is what Law 4 is about.

**Reading it removed a kind branch, which is the point.** The land market's seller was
`partyKind: TREASURY` and nothing else — a party kind named inside a mechanism, exactly the branch
Law 15 forbids. It asks the objective now: a party whose objective is `itsOffice` holds ground as a
duty and has nobody to enrich by keeping it, so when E-5's local authority arrives it will sell there
with no change to that file.

**Measured**: six objectives, fifteen kinds, **more than one answer**. `world`, `land`, `doors`,
`derivative-layer`: **5 red before, 5 after.** Five new tests. Green: lint, typecheck, spec citations
(207), forbids (204 files).

**Carried, and it is the larger half.** Twenty-one participant declarations still hard-code their own
reason inside `orders()`. What this establishes is that the reason is SAYABLE and asked of the kind;
moving each of the twenty-one to dispatch on it is twenty-one bounded changes, one per module, and
none is done. Until they are, "two kinds with different objectives behave differently under one
shock" is something the declaration makes possible and does not yet produce.

**Forecast, with its killer.** A mechanism can ask why a party would do a thing instead of assuming
it from the party's name. **What would falsify it:** two kinds that share an objective and must still
behave differently — a fund and an insurer are both `itsMandate` and their mandates are not alike,
which is item 8's `Mandate` and not this.


## Item 16, stage 0 — the dimension, in the type

**What.** `core/measure.ts`: `Measure<D>`, a phantom-typed number, with `Money<C>`, `Amount<U>`,
`Price<C,U>`, `Ratio`, `PerMember<D>` and `Total<D>` over it, and an algebra in which the eighteen
findings below it are programs that do not compile — `plus`/`minus` (same dimension only), `scale`
(a dimension times a ratio), `valueAt` (`Price × Amount → Money`), `pricedAt`, `ratioOf`, and
`acrossMembers`/`eachMember` as the ONLY door between per-member and total.

**Why.** `mul(a: number, b: number, what: string)` has **950 call sites** (1,091 counting `add`,
`sub` and `div`), and at every one of them the third argument is the only place the dimension lives.
Nothing validates it. `mul(money, money, 'the premium')` compiles — every option premium in this
world is a money-squared number that does not depend on the strike. `add(usd, eur, 'wealth')`
compiles. `mul(perMember, headcount, …)` on one side of a flow whose other side used
`perMember × weight` compiles, and that is the largest conservation break in the model. **594 fields
are typed as bare `number`.**

`core/num.ts` already said what the fix was, in its own words: *"A caller mixing a `Qty` with a plain
number is refused by the compiler, which is the question the brand exists to ask."* `Brand<T,B>` has
existed since `core/ids.ts` was written and is applied to identifiers and to `Qty` and to nothing
else.

**The phantom is a function type and that is not decoration.** A plain `readonly [d]: D` is
covariant, so `Measure<'money:USD'>` is assignable to `Measure<'money:USD' | 'money:EUR'>` — and
`plus(usd, eur, …)` then infers the union and compiles, which is the exact bug the file exists to
stop. `(d: D) => D` puts `D` in both a parameter and a return position, making it invariant. I built
it covariant first and the test caught it: five `@ts-expect-error` directives came back "unused",
which is the compiler saying the lines it was meant to refuse were fine.

**It erases completely.** `Measure<D>` IS a `number` at runtime — no wrapper, no allocation, no
arithmetic that was not there — so behaviour cannot change and Law 18's "gate on behaviour, not bits"
holds by construction. What changes is which programs compile.

**How it is tested**, and this is the part worth keeping: these findings are not runtime bugs to
catch, they are programs that should never have compiled. The test asserts them with
`@ts-expect-error` — it PASSES when the compiler refuses the line and FAILS when the compiler accepts
it. Six tests, five of them of that shape.

**The first door is adopted.** `params.ratio()` and `params.perAnnum()` return `Ratio`, so every
declared rate in this world enters the type system knowing it is dimensionless — and `Ratio` is the
one type that can never be an amount, which is A-44's shape (a spread used as a level) and A-58's (a
leverage ratio called a cost of funds).

**What is NOT done, and it is nine tenths of the item.** The sweep is staged in `docs/AUDIT.md` — ten
stages over 1,091 sites, kernel (157) first, then seeds (79), then module by module with `banks`
(124), `funds` (74), `firms` (73) and `households` (68) the largest. Each is independently
completable and the compiler generates its work list. **None of the ten is done**, and the eighteen
findings stay open until the stage that owns each closes. `world`, `param-owners`: **5 red before,
5 after.**

**Forecast, with its killer.** The dimension is expressible and enforced wherever it is declared.
**What would falsify it:** a dimension the six types cannot say — a periodicity, which Law 8 names in
the same breath as unit and price level and which `Measure` does not yet carry, so A-33's stale wage
bill is still a runtime question.


## Item 17 — the local repairs, and a regression of my own

**Eleven closed in one pass.** The item's own rule is "never a session's work; take each when its
file is already open" — items 8 to 16 opened most of these files.

- **A-2** — `sameState` compared liens by COUNT while its docstring said it compared them, and
  checked the equity BALANCE and not its walk. `forget` then deleted the absorbed cell's whole
  position, liens and ledger with it, with no instruction and no counterparty. Liens are compared by
  qty, beneficiary and reason now — never by id, because two identical pledges by two cells are two
  rows and that is what a merge renames — and the equity walk as well as the balance.
- **A-3** — the merge grew the absorbing cell's weight and THEN asked the register whether it was
  legal. Ask, apply, journal.
- **A-7** — the seed read a capital kind's life off the declaration while three other readers went
  through the parameter register. Through `params` now, like the rest.
- **A-8** — a doc block describing a deleted declaration, sitting on top of the block that documents
  `STORAGE_SESSION`. Gone.
- **A-15** — five per-party stores in the register and the cell doors knew about three. A split
  cell's `moneyWalk` opened at ZERO DUST against copied lots carrying a real balance, so the
  `accounts` and `ownership` families checked it against a tolerance too tight for what it held.
  `copyMemberState` copies the revaluation account and the money walks, `forget` deletes them, and
  `sameState` compares the revaluation account.
- **A-16** — `restate` validated as it applied, so a reverse split one holder's odd lot could not
  survive left the line at two counts. Every holder is checked before any holder is written, which
  is what settlement does everywhere else.
- **A-22** — `lifecycle.ts`'s header said dying is not in this file, directly above the dying.
- **A-28** — one `price` on a `SavingLine` and both sides used it, so a cell that had been surprised
  moved its bid AND its ask down and would sell at a level it would simultaneously buy at:
  uncertainty made it a seller. `bid` and `ask` now, and being surprised WIDENS.
- **A-29** — a bid truncated at `issued`: a bound added because a number exploded, whose
  compensating mechanism 12c built and which was not deleted with it. Deleted; the solver decides
  how much of a bid fills.
- **A-35** — `buying` computed twice from one list, once with `mul` and once with a bare `*`. One
  writer, `committedTo`.
- **A-59** — the resolution published `pays` as the price of a completed sale and **nothing paid
  it**. It is `worthToIt`, which is what it is and what ranks the bidders; what an acquirer should be
  PAID for taking the book on is a missing mechanism (**E-6**) rather than a number standing where
  one would have been.

**Repositioned rather than repaired, with reasons.** **A-43** — the labour module builds the
household's schedule through the door the architecture says not to — cannot move until the
EMPLOYMENT BOOK does: a households-side `venueParticipant` cannot see who is already employed or what
trade they have, and that book is one of item 8's seven. **A-24** — the plan round-trips through
`unknown` and silently drops what it cannot parse — is item 3's shape and wants item 3's answer, a
typed read that throws on a record its own writer malformed, applied to `firms.plan`. **B-9** and
**B-11** are already closed: the file B-9 is about was deleted, and `plan:check` is green.

**A REGRESSION OF MY OWN, and the way I found it is the point.** Item 12 declared `land()` beside
`commodities` — where it belongs by subject — and `land` requires `seed.foundation`. Assembly sorts
by `requires`, so a module needing the seed declared in the MIDDLE of the list drags the sort:
`freight` went after the foundation seed, the foundation's hull block ran before the carrier parties
existed, and **this world lost its entire merchant fleet**. Eleven tests across six files went red.

Every per-item measurement I took missed it, because I measured the files I thought each item
touched. Running the whole suite once found it in a minute. A module that requires the seed is
declared after the seed, and the comment there now says why.

**Also fixed here, and it was item 11's to fix.** `services.test.ts` defined a service as
`!portable && spoilagePerPeriod === 1`, which item 11 replaced with `output === 'capacity'`. Its
acyclicity test asserted a PROXY — no service's inputs reach anything that buys a service — which
held while "service" meant the sixteen unmovable lines and stopped the moment POWER joined the set.
It asserts the property itself now: the recipe graph is acyclic, over every line, with the cycle
named in the failure message if one ever appears.

**Measured.** The whole suite: **79 red before item 8, 79 red now, the same 79 test for test** — and
**707 passing where there were 638**, the 69 being this run's own tests. Green: lint, typecheck, spec
citations (208), forbids (205 files), `plan:check`.

## Item 18 — the worklist re-pointed, and four central banks get four rates

**This entry was missed at the commit it belongs to (`4018142`) and is written here.** The rule is
one item, one commit, carrying its record; item 18 carried its `docs/AUDIT.md` block and its
`docs/WORKLIST.md` re-pointing and not this. A record with a hole in it is a ledger nobody can read
back, so the hole is filled rather than left — and named, because that is what the ledger is for.

**The worklist was re-pointed at what items 0–17 changed.** 13k is reframed: the dividend half of it
was built at item 10 (`declareDividend` on the firm's own fiscal quarter, a claim per holder, a
payment that reads the claim), and what is genuinely still periodic is the rating fee and the tax
assessment. 13m stands on item 12's ground now that land has a supply and a cleared price. 13n is
unblocked by items 7 and 15: a firm can be born because there is a lifecycle to be born into and an
objective to be born with. **13o is still blocked and by a named thing** — item 8 built `Agreement`
and did not migrate the seven private books, so `Mandate` does not exist, and building asset
managers on today's `fund` party would bake the pool/decider conflation in permanently. 13l is done.

**C-3: `corridor(ctx)` took no currency, so four central banks administered one rate.** The Fed, the
ECB, the Bank of England and the Bank of Japan all sat at 2%. With no interest differential between
two moneys there is no carry — an FX forward prices flat to spot, covered interest parity says
nothing, and the cross-currency basis has nothing to be a basis of. **Four mechanisms this world has
built could show nothing**, because one row was shared by four institutions whose whole reason to
differ is that they set different rates. `POLICY_RATES` declares one policy per money and
`corridor(ctx, ccy)` is asked per currency at all four call sites, with one `centralBank.corridor`
announcement per central bank.

**And the yen wanted to be zero and could not be.** A policy rate of 0.0 puts the corridor's floor at
−0.001 and the solver refuses a negative price, so it sits at 0.002 and the absence is written down
as **E-7**: a negative policy rate is real, three of this world's four central banks have run one,
and nothing here can express it. That is a finding about the price grid, not a number to tune.

**Measured.** Whole suite 79 red before, 79 red after, the same 79 test for test.

## Item 16, stage 1 — the kernel carries its dimensions

**The stage's own count was wrong and the grep is why.** Stage 1 was written as 157 kernel sites; the
count came from `grep -E '(mul|add|sub|div)\('`, which counts every `Set.add` and `Map.set` sibling
in the kernel. There were **119**. They are **68** now, and what closed is every money arithmetic
among them.

**Fifty-two files, and every one of them either a door or a site the compiler found.**

**`Qty` IS `Amount<'piece'>` (Law 4), and that is the change the rest rests on.** The tick grid's
brand and the dimension system's amount were two representations of one real thing — a count of a
unit's smallest pieces — and Law 4 says that is a defect however well each is written. One line in
`core/tick.ts` made them one type, the whole engine typechecked unchanged, and `valueAt(price, qty)`
now takes the register's own quantity with nothing in between.

**The doors, and the ripple was small because a `Measure<D>` IS a number at runtime.** `Print.price`
is a `PerPiece`, and the six writers say so where they write (`params.price`, `registry.onQuoteGrid`,
`toTickOf`, the seed's `priced`, the solver's book, the funds' opening level). `Lot.basisPerUnit` and
`DrawnLot` are prices, so `costOfDraw` is a `valueAt` and returns `Cash`. `CashFlow.perUnit` and
`DueAction.amountPerUnit` are prices, so discounting a schedule gives a price back (`priceAt`) and a
yield comes out a `Ratio` (`yieldOf`) — Law 3 in the type: a yield is derived FROM a price and can
never be one. `Valuation` hands out `PerPiece` for what a unit is marked at and `Cash` for what a
position is worth, and `rateInForce` is a `Ratio`, which makes `inMoney` a `scale` and makes a rate
something that cannot be spent. The solver's outcome, every fill's level and every trade's cash go
the same way, and `revalue` — every re-marking and every FX move in this world — with them.

**`Sum<T>` carries its terms' dimension out of `sum`**, which is where most of this engine's totals
are made; a list of moneys totals to money and a list of prices to a price.

**Five operations were added to `core/measure.ts`** because the kernel needed them and each is real
algebra rather than an escape hatch: `over` (a dimension divided by a pure number — a split restating
a price, a total shared out), `absolute`, `negated`, and the two doors `asCash` and `asPerPiece`.

**The first thing the type found that was WRONG rather than merely unchecked.** `index.base` was
declared `dimension: 'price'` and read with `params.price`. An index level is a pure number — the
declaration's own `why` says so, at length, in the same breath as calling it a price — and a level
that could be added to a balance would be a claim rather than a measurement. It is a `Ratio` now and
the declaration was corrected with it.

**`eachMember` where `div` was, at the two per-member doors.** `perMemberOf` in settlement and the
coupon payer in `world/actions.ts` divided by a cell's weight with `div`, which answers `Infinity`
for a cell of nobody and puts it in an equity account or on a leg. `eachMember` refuses it. The
issuer's side of a revaluation crosses by `acrossMembers` and by nothing else.

**What this stage cannot check, said out loud rather than implied by an alias.** A currency is DRAWN
(Seed B1.a) and a unit is registry data, so neither is a literal the compiler ever sees: `C` and `U`
are `string` in every kernel signature and two currencies are one type to it. So **A-65** (money²),
**A-39**, **A-1**, **A-18** (per member against total) and **A-44**, **A-58** (a ratio as a level)
are unwriteable in the kernel now, and **A-23 and the four beside it are not** — cross-currency
addition stays a runtime refusal at the leg, which is where the currency is actually known, and
becomes a compile error only where a module names its money as a literal. Nor can the register say
whether a `Qty` it holds is per member or a total, because for a cell it is the first and for a named
party the second: **A-1's shape survives this type**, and closing it needs the ontology rather than
the arithmetic. Those limits are in `core/measure.ts` beside the aliases, where the next reader will
be standing when it matters.

**What is left in the kernel is not money.** Plant wear, storage and capacity
(`registry/physical.ts`), days over terrain (`registry/geography.ts`), a variance of price moves,
two audit families comparing two routes to one rate, and `core/num.ts`'s own definitions. Each
belongs with the stage that owns its module.

**Measured.** The whole suite was **79 red of 786** before this stage and is **79 red of 791** after
— the same 79, test for test — the five new ones being this stage's own, which assert that a bare
number can no longer be a print, a basis, a cash flow or a total. Green: lint, typecheck, spec
citations, forbids, `plan:check`.

## Item 16, stage 2 — the seed, and the second scale

**This world states its numbers in two scales and only one of them was in the type.** A price is
money for one NAMED unit — a dollar a tonne, a wage an hour, a level a share — and the state holds
PIECES of both: cents, and the smallest piece of a tonne. `Registry.priceOf` and `Registry.pieces`
are the two doors between them, and the seed's `cash`, `held`, `inNamedUnits` and `priced` are the
seed's own four names for the same two. Nothing said which side of them a number was on.

So `Stated` (money in its named unit), `Named` (an amount of a named unit) and `PerNamedUnit` join
`Cash` and `PerPiece` in `core/measure.ts`, and the two doors are the only crossings. The seed
speaks in named units throughout and converts at them.

**The count of the seed's `mul`/`add` sites is unchanged at 71, and that is not the measure of this
stage.** What closed is the SCALE, and the seed's own comment at the central bank's benchmark says
why that is the half worth closing first: *"This read multiplied a count of PIECES by a price per
NAMED unit, so the reserves the seed thought it had bought were the subdivision of a bond times too
big — eight per cent of the system's paper became eight times it."* That was found by hand, once,
after it had shipped. It is a type error now.

**Its money arithmetic went through the algebra with it**: the recipe's cost-of-one-unit table (a
`PerNamedUnit` all the way down — the wage scaled by the hours a unit takes, the inputs by how many
of each, the whole divided over what survives the line), the system's paper, the central bank's
assets, the treasury's buffer and the banks' reserves. Including **a bare `line.banks * price`** — a
count of units times a level, with no function around it at all, which is the one shape `valueAt`
exists to make unwriteable.

**A local `scale` shadowed the imported one** in the middle of the largest function in the seed and
the compiler caught it as "this expression is not callable". Renamed `madeByTheHours`, which is what
it is.

**THE TYPE FOUND TWO THINGS NOBODY HAD WRITTEN DOWN, which is what it was built for.**

**E-8 — a declared price does not say which scale it is in.** `dimension: 'price'` is one dimension
and this world states levels in two. `equity.openingShare` is declared in PIECES of money per share
(`value: MONEY_PIECES`) and written straight into a print; every goods opening level is money for a
NAMED unit and goes through `Registry.priceOf` first. Only the declaration's free-text `unit` says
which, and `params.price` cannot tell them apart. `SHARE_PIECES` is 1 and `MONEY_PIECES` is 100, so
the two scales are a hundred apart for exactly the line the known finding *"a share worth a
hundredth of a cent"* (worklist 16, step 214) is about. Positioned at stage 10, with the registry.

**E-9 — a dirty price adds two scales.** `readCurve` and `market.ts` both add a print — money pieces
per piece — to what a kind's `accrued` returns, which its writers state per named unit. It is right
today ONLY because `PAR` is declared `perUnit: MONEY_PIECES`, the same subdivision the money has, so
the two coincide for par-denominated paper and for nothing else. A coupon-bearing instrument whose
unit is not par, or a `pieceShift` that moves one and not the other, makes every dirty price wrong
by the subdivision, silently. Positioned at stage 3, with the banks and the money market.

**Measured.** **79 red of 791 before and after, the same 79 test for test.** Two new assertions in
`measure.test.ts` (13 now): dollars do not add to cents, and a count of pieces is not an amount of a
named unit. Green: lint, typecheck, spec citations, forbids, `plan:check`.

## Item 16, stage 3 — the banks

**130 sites became 44, and what closed is every number a bank decides with.**

`CapitalPosition` and `CapitalRules` carry their dimensions: a risk weight, a minimum, a buffer and
a limit-per-name are `Ratio`, what a book weighs and what stands in front of its creditors are
`Cash`, and the whole of `capitalOf` is `scale`, `plus`, `minus`, `over` and `ratioOf`.
`LiquidityPlan`, `DeskState` and `DeskQuote` follow: a desk's view, edge, skew, bid and offer are
LEVELS, its limit and its book are MONEY, its concentration and its carry rate are PURE NUMBERS.

**`amountOf` is the door this stage needed and `core/measure.ts` did not have.** `Money / Price =
Amount` is the third of the three ways money, a price and an amount meet, and every affordability
read in this world is one: what a party CAN do is its money over the level it would have to pay,
which is Law 6's one admissible case. A desk's three constraints — the room in its position, the
room in its whole book, and the money it has spread over the lines it quotes — are three of them.
It does not round: which way a fraction of a piece goes is the caller's decision and has a name.

**A `Quote` is five RATES and the type says so.** `costOfFunds`, `expectedLoss`, `capitalCharge`,
`operatingCost` and `rate` are per annum on a unit lent; `lossGivenDefault` is a share of what is
lent and `probabilityOfDefault` a share of the periods this bank remembers. A-44 and A-58 are the
mistake of reading one of those as a level, and neither is writeable in this module now.

**Eight doors where a published number re-enters a decision.** `roomFor`, `capitalOf`, `liquidOf`,
`liquidityPlan`, `numberIn`, `securityIn`, and the two reads of `moneyMarket.refused` all take a
number out of what a party itself published and hand it straight to a decision. Each now says what
it is at the read. `securityIn` does more than say: what crossed the 4.9b door claiming to be a
quantity goes through `asQty`, so a number that is not a whole count of pieces is refused where it
arrives rather than three phases later.

**A local `scale` was not the only shadow.** `FundingCost.interest` is money — what a bank actually
paid on what it owes, annualised — and `perAnnum` is that over what funds the book, which is a
ratio. Typing them separately is what made the distinction visible; both were `number`.

**Measured.** **79 red of 793, the same 79 test for test**, with 714 passing. (791 became 793
because stage 2's own two assertions were written after stage 2's run.) Green: lint, typecheck,
spec citations, forbids, `plan:check`.

## Item 16, stage 4 — the funds, and a name that meant two things

**68 sites became 37.** `NavRead` is the heart of it: assets and what is owed are `Cash`, shares
outstanding is a `Qty`, and **`perShare` is `pricedAt(net, shares)`** — money over the claims on it,
which is what a price IS. `claimOf` was `quantity * perShare`, a bare `*` of a count and a level; it
is `valueAt` now, the second such site this sweep has found (the seed had the first).

**AND `perShare` MEANT TWO DIFFERENT THINGS.** `nav.ts`'s is MONEY per share. `etf.ts`'s is UNITS OF
A LINE per share — a count over a count, which is a pure number. One identifier, one module, two
dimensions, and nothing said so: `basketValue` multiplied one by a mark and `create` multiplied the
other by a share count, and both compiled. They are `PerPiece` and `Ratio` now. Law 9 says an
internal id is never a display name; this is the same defect one level up — one NAME for two
dimensions — and the type is what tells them apart.

**`heldAsMoney` joins the doors, and it is not a cast.** Money is an INSTRUMENT in this world and an
account is a holding of it, so what the register counts is an `Amount<'piece'>` of a currency, and
what a price times a quantity comes to is `Money<'piece'>`. They are the same cents, and they are
the same cents ONLY because money's own price is one — the single hard-coded price this world has
(Money D2). Every read of a balance into a decision crosses that, and it is named now rather than
assumed.

`DerivedReads` — the kernel's own reads, which every derived value is given — carries its dimensions
too: `quantity` and `issued` are counts, `worthOf` is money. That one interface change is what let
the whole of `nav.ts` typecheck without a cast in it.

**Measured.** **79 red of 793, the same 79 test for test.** Green: lint, typecheck, spec citations,
forbids, `plan:check`.

## Item 16, stage 5 — the firms

**77 sites became 31.** The firm's plan is where the most dimensions meet in one expression, and it
shows: what it expects a unit to fetch is a LEVEL, what the recipe's inputs cost for one unit is a
level, what its plant wears out by per unit is a level, and the contribution is the three of them
subtracted — so it is a level too, and the value of an hour is that over the hours a unit takes.
Every one of those was a bare `number`, and every one of them met the others through `mul`, `sub`
and `div` with a string saying what it was.

`HeldVintage`, `Capacity`, `PlantOffer`, `CostOfCapital` and `PlannedOrder` carry their dimensions
now; so do the module's own readers — `expectedPrice`, `wagesDue`, `priceOf`, `quotedRate`, `owes`,
`requiredOnEquity`, `carryPerPiece`, `storageRateIn`, `wearPerPlantUnit`, `capitalChargePerUnit`.

**Three roundings that were implicit got names.** Capacity is `downTick` — what plant a firm HAS
over what one unit takes is what it CAN make, and the fraction above is a unit it cannot start. A
surprise width is `toTick` — a measurement takes the nearest piece, because it is not something a
party can or must do. A firm's purchase of an input is `subQty`. Each was a bare `div` or `sub`
landing between two pieces with nobody saying which way it went (core/tick.ts's whole subject).

**Measured.** **79 red of 793, the same 79 test for test.** Green: lint, typecheck, spec citations,
forbids, `plan:check`.

## Item 16, stage 6 — the households

**73 sites became 32.** A cell's decision is money all the way through, and it was `number` all the
way through: what it takes in, what a period could cost it, the cushion it wants against its income
AND against its savings, what it owns over that cushion, what it decides to spend, what it can pay
with, what it has spare, what it is short of, what belongs in a fund. They are `Cash` now — and the
two things in that sentence that are NOT money, the periods of cushion it wants and how fast it
closes a gap, are `Ratio`, so neither can meet the others.

**The demand curve is the clearest gain.** A `Rung` is a LEVEL and a COUNT. `rungsOver` is
`amountOf(budget, price)` — money over a level is how many it buys — `rungsUpTo` is the same under a
want, and `levelsBelow` is a level scaled down a grid. Every one of those was money over a price
with a string saying so, and every increment was a `sub` of two counts that is now `subQty`.

**A second local `scale`, and then a third.** `consume.ts` and `portfolio.ts` each had a
`const scale = sum([...])` — a magnitude for a dust check — shadowing the imported operation, in the
same shape the seed had in stage 2. Renamed `magnitudes`, which is what they are. Three files in six
stages: `scale` is the name this codebase reaches for when it means "how big the numbers here are",
and the algebra now owns it. That is a Law 4 observation about NAMES rather than numbers, and it is
the same one stage 4's two `perShare`s made.

**Measured.** **79 red of 793, the same 79 test for test.** Green: lint, typecheck, spec citations,
forbids, `plan:check`.

## Item 16, stage 7 — the money market, and `Total<D>` corrected

**60 sites became 33.** The resolution waterfall is where per-member and total meet hardest: a
bank's liability to a household CELL is what one member holds times the weight, and every one of
those crossings now goes through `acrossMembers`, which refuses a weight that is not a count of
people. `Valuation` — assets, deposits, borrowings, the hole, the insured part — is `Cash`, and
`insuredAt`, `uninsuredAt`, `coveredBy` and `payFrom` say what they return.

**AND THE TYPE WAS WRONG, in a way only this stage could show.** `Total<D>` was
`Measure<`total:${D}`>` — a dimension of its own — so a bank's total deposit liability could not be
added to the money beside it, and every sum in the waterfall wanted a door that did nothing.

The asymmetry is the point, and it took a module that sums over cells to see it. `PerMember<D>`
genuinely is NOT `D`: money per person is not money, and a whole liability booked against one
household's equity account is **A-1**. But a TOTAL of money is just money — `total:` was a
PROVENANCE dressed as a dimension. `Total<D>` is `Measure<D>` now, `acrossMembers` still guards the
only crossing there is, and the `@ts-expect-error` that proves **A-39** unwriteable still fails to
compile. That last part is the test which says the correction cost nothing: what the type was built
to stop, it still stops.

**Measured.** **79 red of 793, the same 79 test for test.** Green: lint, typecheck, spec citations,
forbids, `plan:check`.

## Item 16, stage 8 — the CDS, and the derivative layer with it

**46 sites became 26, and the change reached every derivative in the world.** A CDS is spread ×
notional, which is the shape **A-65** is made of, and the whole layer turned on three fields nobody
had typed: `Contract.notional` is a COUNT (`Qty`), `Contract.struckAt` is a LEVEL (`PerPiece`), and
`ContractReads.mark` and `measuredMove` return levels. A premium is `valueAt(spread, notional)` and
cannot be anything else now — and the same three fields carry futures, options, forwards, swaps, the
index and the layer's own margin, so typing them once typed all of them.

**A runtime check became the type, and the code came out.** `Contracts.open` called
`asQty(decl.notional, …)` and threw the answer away: a validation at ONE door for a rule that now
holds at every site that constructs a contract. The line is deleted and the comment says what
replaced it. That is Law 12's shape — a fix that removes code — and it is the first time in this
sweep that the type has been able to retire a guard rather than add one.

**Ten `-c.notional`s became `negQty`.** Short is the other side of a count, and `negQty` has said so
in `core/tick.ts` since it was written; what found them was the lint rule that refuses to negate a
branded number. One `sub(rate, -basis)` in the cross-currency swap became the `add` it always was.

`measuredMove` — the standard deviation of a line's prints, which every margin in the layer is sized
from — is a LEVEL, in the same money per piece the prints are in. It was a bare number, and what it
feeds is a spread.

**Measured.** **79 red of 793, the same 79 test for test.** Green: lint, typecheck, spec citations,
forbids, `plan:check`.

## Item 16, stage 9 — the treasury

**33 sites became 12.** The funding need is one subtraction with six money terms in it — what its
own paper will cost it over the horizon, the mandate, the buffer, what it collected last period and
what is in its account — and every one of them was a `number`. They are `Cash`; the horizon and the
buffer are `Ratio`s that `scale` them; the auction size is `over(need, auctions)` and the units it
brings are `amountOf(size, reservation)`.

**Every tax is a rate on a base, and every one was `mul(leg.amount, rate)`.** A leg's amount is a
count of money pieces, so it crosses `heldAsMoney` first and then `scale`s by the rate — which is
what says out loud that a tax is a SHARE of what somebody was paid and never an amount of its own.
`Settled.realised` carries `proceeds` and `basis` as money now, so a capital gain is a `minus` of
two moneys rather than a subtraction of two numbers that happen to be in the same currency.

`PROCUREMENT`'s `share` is a `Ratio` — the one declared number in the module that is not money and
was indistinguishable from one.

**Measured.** **79 red of 793, the same 79 test for test.** Green: lint, typecheck, spec citations,
forbids, `plan:check`.

## Item 16, stage 10a — the derivative tree, and the doors the kernel was missing

**111 sites went (697 → 506)**, and almost none of them by hand: typing
`DerivativeKindProfile` generated the list. A mark, a close-out, an initial margin and what a row
will cost in cash are `Cash`; a premium per unit of notional is a `PerPiece`; `ContractPayment.
amount` and `Contract.basis` are `Cash`. Eight kinds then said what each of their own numbers was,
and the layer that margins them said it too: `posted` and `inFund` are register reads (`Qty`),
`requirement` is a computed value (`Cash`), and the waterfall's rounds carry both.

**Four kernel doors were the blockers, one line each.** `finite` was erasing every dimension on its
way into the state — it is `finite<T extends number = number>(x: T): T` now, same runtime, no hole.
`Registry.cashFor` and `Registry.payable` take a `Cash`, because they are where a computed value
lands on the money's own grid. `OwnContracts`/`ContractsRead` answer in `Cash`. And
`EquityEffect.delta` is `PerMember<'money:piece'>`, which its own comment already said.

**The grid is not the type, and this stage is where that stopped being a detail.** `asQty`,
`addQty`, `subQty` and `negQty` assert a whole number of pieces; a DESIRED position is not one.
Typing `want`, `move`, `left to hedge`, `what falls due` and a dealer's book in a pair with those
doors made 330 tests throw `[Law 8] … is not a whole number of the unit's pieces`. `Qty` is two
claims in one name — a dimension and an invariant — and a target only has the first: those sites
use `asAmount<'piece'>` with `plus`/`minus`, and the grid assertion stays at
`registry.deliverable`, where a target becomes a quantity somebody can hold.

**Three crossings named at the site that knows which dimension it is**: a cross-currency swap's
`struckAt` is a BASIS where a forward's is a price (E-10 again); an index level times a multiplier
is money, which is the dimension of the book it hedges; and the test rig's `phx` is a VALUE in
cents, so an order sized in the money instrument says `asQty(phx(…))`.

**Measured.** **79 red of 793, the same 79 test for test.** Green: lint, typecheck, spec citations
(208), forbids (205 files), `plan:check`.

## Item 16, stage 10b — the kernel's own arithmetic, and the reads that carry it

**99 sites went (506 → 407).** 10a typed what modules ANSWER; this types what the kernel does with
the answers.

**Settlement's equity pass** is five functions — `bump`, `bumpIn`, `inOwn`, `perMemberOf`,
`carryingOf` — and all five were bare `number`. `Op.basis` and `Op.valuePerUnit` are
`PerPiece | 'carrying'`, `AssetLeg.pricePerUnit` is `Option<PerPiece>`, `CreateLeg.costPerUnit` is a
`PerPiece`, `RegisterDelta.qty` is a `Qty`, and `Valuation.inMoney` takes a `Cash` — which is what a
currency conversion is: the same value, in another money.

**The per-member claim is now made in exactly one place.** `bumpIn` ends in `asPerMember`, with the
sentence that justifies it; `perMemberOf` is its inverse for an ISSUER, whose issued amount is a
total. A-1's shape survives (the register still does not know which of the two a `Qty` in it is) and
the type says where the claim is made.

**The reads.** `PublishedStatement` carries `Cash` and `Qty` through the dimensions' own doors at
the one place a published number re-enters the type system; `IndexWorld.rate` is a `Ratio`,
`Constituent.weight` a `Qty`, `Benchmark.rate` a `PerPiece`, `AuditMemory` holds `Qty`,
`BalanceSheet` holds `Sum<Cash>`, `World.owedIn` answers in `Qty`.

**And the modules that read them**: securities-lending (a borrow FEE is a share of what the paper is
worth, not a price of it), control, merchants, corporate-bond (leverage and coverage are `Ratio`s),
insurers (a cover price was experience plus what its capital costs — two different dimensions added),
estate, capital-programme, equity's reads, indices, options' own quote, the observer, four audit
families.

**`mul(t.coupon.amount, 1, …)` was deleted rather than typed** — a multiplication by one is not an
arithmetic, and Law 12 says a fix removes code.

**Measured.** **79 red of 793, the same 79 test for test.** Green: lint, typecheck, spec citations
(208), forbids (205 files), `plan:check`.

## The audit file becomes the implementation file

**What was wrong with it.** `docs/AUDIT.md` was produced by reading all 59,927 lines of
`packages/engine/src` and every `done` row and `MET` mark made about it. That method finds defects.
**It cannot find a sector that was never written**, because an absence leaves no trace in source or
in a claim about source. Six of them were carried in from a deleted holding pen as a single row of
finding `C-4` — *"the sectors that are not there"* — filed under item **18**, which was then marked
**DONE** having built one of its four findings.

**What the aggregate says, and it had never been run.** `docs/COVERAGE.md` holds one row per spec
clause and nothing ever summed it. **1,369 clauses: 823 MET, 90 PARTIAL, 456 MISSING, and 97 of the
MET carry `NEVER REACHED`** — 47% of the specification is missing, partial or dead. **Five systems
have no clause MET at all**: Polity (0/32), Private Equity (0/25), Prime Brokerage (0/24), Hedge
Funds (0/24), Short-Term Debt (0/19). A sixth, Small-Business Pools, has 2 of 28 and both are the
generic cell kernel.

**How they were lost, traced through the worklist's own rows.** 13e closed placing small-business
cells, the covered bond, factoring, receivable pledges and senior notes as repo collateral into 13f;
**13f's PLACED list names none of the five.** 13f placed short-term debt into 13i; 13i closed without
it. 13h placed prime brokerage and firm birth into *"go on with them"* — no destination. 13n, still
open, says it itself: *"PLACED forward three times (13f to 13g to 13h) without landing."* That is
finding `B-14` — *a finding was positioned into an item, the item closed, and the finding was not
done* — **six more times**.

**What replaces it.** `docs/IMPLEMENTATION.md`, in four parts: the measured state per spec system;
one ordered list of **25 items** with dependency reasons; each item with files and line-by-line steps;
and every one of the **63 still-open findings carried verbatim** so nothing is lost. The open lines
come first (items 2–9 finish what the old file left half-built), then the sectors that are not there
(10b–19), then the worklist tail. Item **1** is a new gate, `check:existence`, which aggregates
`docs/COVERAGE.md` and fails when a system's counts move without the plan's table moving with them —
the check that makes a `done` row falsifiable. Six of the old file's item numbers are re-pointed at
it; the rest go to this ledger, which keeps the closed items and is not rewritten.

**`E-10` is indexed for the first time.** `Contract.struckAt` means a different dimension per kind —
a price for a bond future, a spread for a CDS, a rate for a swap, an index level for an index
future, a basis for a cross-currency swap. It was found by the dimension type at stage 10a, written
only into code comments, and never added to the findings index.

**Not measured.** No test was run for this change; it is documentation plus 38 re-pointed references
and 13 placeholder `planItem`s. Typecheck and lint are green. The four `cds` and `money-market` files
carried in from item 16 stage 10c are typed and unmeasured, and item **2** continues from there.

## Item 1 — the existence check: is a `done` row true?

**What it is.** `tools/coverage-existence.ts` and `npm run check:existence`, in `npm run check`.
Per spec system: clauses MET, PARTIAL, MISSING, OUT OF SCOPE, and how many of the MET carry
`NEVER REACHED`. Two reads and no world. Which system a clause belongs to is joined from the spec
index rather than parsed out of `docs/COVERAGE.md`'s own headings (Law 4), and the existing
`readCoverage` and `buildSpecIndex` are reused — no third parser.

**It found two things on its first run.**

**Ten of fifty-one systems produce nothing, not five.** Five have no clause MET at all: Polity 0/32,
Private Equity 0/25, Prime Brokerage 0/24, Hedge Funds 0/24, Short-Term Debt 0/19. **Five more have
every clause they MET marked `NEVER REACHED`** — CDS 17, M&A 10, Insurers 9, Securities Lending 9,
Commodity Futures 5. `builtAndDead()` names those separately, because a reader scanning for "0 MET"
walks straight past `M&A 10 MET` and all ten of those are marks on a mechanism nothing has ever
invoked. It is the more dangerous kind: it reads as done. The remedy differs too — an absent sector
needs writing; one of these needs a way IN to what is already written.

**The spec and COVERAGE disagree about what a requirement is.** 1,369 rows against 1,361
REASON/VERIFY/FORBID clauses. The eight extra are `MET` rows on spec sub-clauses that carry no form
word (`Sovereign I1.a`, `Banks Lending C1.d`, `XI-11`, `Labour A3.b`, `Households A2.b`,
`Households F1.a`, `Households F1.b`, `Households F2.a`), which the index correctly calls NOTEs.
Marking work you did is not wrong; a denominator that silently disagreed with the spec's own would
be, so they are reported on every run rather than absorbed. In the other direction every spec clause
has a row — asserted, not assumed, because a clause nobody answered and a clause somebody deleted
look identical from inside the file (Part II: never delete a clause to look better).

**One step was dropped and the reason is the interesting part.** As first written, step 1.3 had the
tool run the rig and cross-check `world/reach.ts`. It does not. The `NEVER REACHED` marks are
already WRITTEN into `docs/COVERAGE.md` by whoever re-marked it, so the tool READS them (Law 19)
rather than re-deriving them; the reach read is what keeps them true and this is what counts them.
That keeps the gate to two file reads — fast and deterministic, which is what lets it sit inside
`npm run check` instead of being a thing somebody remembers to run.

**The measured state, which Part 0 now carries and the gate now defends.** 1,361 clauses: 815 MET
(95 NEVER REACHED), 90 PARTIAL, 456 MISSING. **641 of 1,361 — 47% — are missing, partial or dead.**

**Verified in both directions.** Green as committed; perturbing one figure in Part 0 exits 1 and
prints the two rows that disagree. Eight tests: the join on a two-system fixture, MET-and-never-
reached as two facts about one row, a NOTE not counted, both absence reads, and three against the
real files — every absent sector is named in the plan, Part 0's table is the generated one, and no
spec clause lacks a row.

**Measured.** Lint, typecheck, `check:spec`, `check:forbids`, `check:existence`, `plan:check`: all
green. The tools suite is 30 green across 5 files. **The engine suite was NOT run** — the owner's
instruction stands until the plan's items are worked, and item 1 touches no engine code. No COVERAGE
re-mark: this item implements no spec clause.

## Item 2, stage 2a.1 — the banks, the funds, and the household demography

**Item 2 is in stages, and the reason is Law 14.** As written it bundled the TYPING — mechanical, no
behaviour change, gated by Law 18 — with eighteen FINDINGS, each a real change to what the world
does. Nineteen bounded changes in one item. The typing is 2a; the findings are 2b–2e, grouped by
what they are, each with its own entry. Said in `docs/IMPLEMENTATION.md` rather than done quietly.

**A rate is a `Ratio`, and that one change is the shape of the stage.** `LoanTerms.rate` and
`SubTerms.rate` were bare `number`s. What a span earns is the rate SCALED by the fraction of a year
it covers — dimensionless — and what that comes to per unit is par scaled by the share. So
`add(1, interest)` is gone: there is a named `PAR` in each file, *"one unit is one piece of its
money"*, which is the one place the two scales coincide (`E-9`) and now says so instead of being
assumed. The cascade off `LoanTerms.rate` was two sites, because stage 3 had already typed
everything around it.

**`E-11`, and the type found it.** `Outcome.price` is a `PerPiece` and **some books clear a RATE** —
the subordinated raise, the money market, the IRS, the CDS. The bidders post rates, the solver
strikes one, and nothing in the book can say which of the two its level is. It is `E-10`'s shape —
one field, a different dimension per use — at the CLEARING layer rather than the contract layer.
Named at the door in `subordinated.ts` rather than assumed, and positioned to 2a.2 and item 6.

**`A-18` is closed, both ends.** The comment said the fraction below one person *"stays where it is
until enough of it has accumulated to be somebody"* and nothing accumulated: `crossing` and `dying`
were recomputed from the weight every period, so the part of a person was DELETED every period.
`households.waiting` carries it per cell and per event, so `Math.floor` times a real event instead of
deleting it. The second end matters as much: a cell whose whole weight would cross was SKIPPED, which
pinned the tail of every band where it was — it now crosses as itself. Declared `physics` in the
ontology register: who is partway through the year in which they cross is this sector's own
demography, and the kernel wants no store for it.

What deleting it cost, from the arithmetic in the finding: `share` for a ten-year band is about
1/521, so a cell of 3,600 ages 6 a period and its children age `floor(6 × 0.00192) = 0` — for ever.
Mortality was worse because the rates are smaller: any cell below `1/rate` members had nobody die in
it, at any age, for the life of the run. Those cells could not age, could not die and (nothing calls
`merge`, `A-17`) could not recombine, and they went on consuming and looking for work.

**Other sites typed**: `hoursNeeded` is `scale` of a technology by a count of rows; `linesCovered`
and `probabilityOfDefault` are `ratioOf` two counts of the same thing; the desk's one-sided flow is
`plus`/`minus`/`absolute`/`ratioOf`; the fund's redemption shortfall is `minus`. **`mul`, `div`,
`add` and `sub` leave five files.** Arithmetic sites **402 → 390**.

**Measured.** Typecheck 0, lint 0, `check:existence` green, `nouns.test.ts` 7 green (the new store is
declared and the register accepts it). **The engine suite was not run** — 2a is a no-behaviour-change
typing stage except for `A-18`, which is a behaviour change and is named as one here rather than
hidden inside a refactor.

## Item 2, stage 2a.3 — the seed's build-out, and `E-8` closed

**The seed speaks in NAMED units and the type now says so.** `started`, `sizeOfLine`,
`wantedInAPeriod`, `plantOf`, `startsOf` and `cashOf` carry `Named`; every recipe coefficient is a
`Ratio` read at the site, so the build-out is `scale` and `over` throughout and a coefficient can
never become a quantity. `hoursOffered` is a `Qty` because `params.amount` answers in pieces — which
is the half of that ratio the seed's own comment says multiplied this world's real economy by the
subdivision of an hour the last time it was got wrong.

**`E-8` is closed, and it was worse than it was written up as.** The finding said *"a declared price
does not say which of the two scales it is in"*. Splitting `dimension: 'price'` (money per PIECE)
from `'pricePerUnit'` (money per NAMED unit) showed that **of six declared levels, four are stated
per named unit** — every goods opening level, the opening wage, and the FX opening rate — so the
mis-declaration was the majority rather than the exception. `equity.openingShare` and
`funds.openingShare` are genuinely per piece and stay as they were.

**A seventh was not a price at all.** `bondFuture.size` is 100,000 units of FACE per contract — no
money anywhere in it — declared `dimension: 'price'` and read as money-per-piece. It is a `count`
now. A contract size converts contracts to face; it can never be paid.

**And the split caught a live defect, which is why it was worth doing rather than annotating.** The
FX opening rate was written as a PRINT — money pieces per piece — directly from a declaration stated
in named units on both sides. `rateTickFor`, three lines away in the same registry, has always gone
through `priceOf` for exactly this reason. The level did not, so the tick and the level agreed only
while base and quote shared a subdivision. It crosses at `registry.priceOf` now, like the tick.

**A stale reader from stage 1**: `indices.test.ts` read `index.base` with `params.price` after stage
1 re-declared it a `ratio` — which throws at the read rather than compiling wrong. Corrected.

**Worth saying about the method.** A declaration change is a RUNTIME contract the typechecker cannot
see: `read(id, 'price')` on a `pricePerUnit` parameter throws, and nothing in `tsc` says so. Every
reader of a re-declared parameter has to be found by hand. That is the opposite of the type work
around it, and it is why the parameter register checks the dimension at the read at all.

Arithmetic sites **372 → 350**; the seed's own **75 → 53**. Typecheck 0, lint 0, `check:spec`,
`check:forbids`, `check:existence` green, tools suite 30 green. The engine suite was not run.

## Item 2, stage 2a.4 — the seed's stock, its banks, and its sovereign debt

**`downTick` has left the seed entirely, and that is the finding of the stage.** The seed speaks in
NAMED units — a whole machine, a whole hull, a whole tonne — and `downTick` floors to the smallest
PIECE the state holds one of. They coincide only where a unit's subdivision is one. Every place the
seed reached for "a machine is a whole machine" it was flooring on the wrong grid, invisibly, because
both were `number`. `core/tick.ts` gains `downToNamed`, which says which scale it is on, and the
seed imports no piece-grid door at all now.

**A third shadowed name.** `over` — the algebra's "a dimension divided by a pure number" — was
shadowed by a local meaning "what funds this bank", in the middle of the function that needed the
algebra's. Renamed `fundedBy`. Stage 6 found `scale` shadowed twice for the same reason: these are
the names this codebase reaches for when it means "how big" or "over what", and the algebra now owns
them. Three files, three stages — worth a lint rule if it happens again (a rule that can be a check
should be one).

**The bank funding block is money and says so**: `assets`, `already` and `funding` are `Cash`, and
what a household CELL has on deposit crosses `acrossMembers` — which refuses a weight that is not a
count of people — rather than a bare `held × weight`. The leverage line is `plus` of two `Ratio`s.

**The build-out**: a firm's opening stock, its work in flight, what a period of starting draws, its
share of its line's plant, and a carrier's fleet are all `scale` of a `Named` by a `Ratio`. The
sovereign's debt is `valueAt(openingLevel, whatItMakes)` scaled by the periods of it its sovereign
owes, over the people it is owed by. The central bank's opening holding — `others × share /
(1 − share)` — is `over(scale(others, share), minus(ONE, share))`, with `others` built from the two
things that make it rather than a bare `+`.

Arithmetic sites **350 → 331**; the seed's own **53 → 34**. Typecheck 0, lint 0, `check:spec`,
`check:forbids`, `check:existence` green, tools suite 30 green. The engine suite was not run: nothing
in this stage changes a number.

## Item 2, stage 2a — the dimension sweep's typing is finished

**Twelve stages, 402 grep sites to 55, and the 55 are named rather than left.** What remains is
arithmetic that is dimensionless on purpose: counts of periods, days, years and people; the discount
bases in `prices/curve.ts`; the two `Map.add`/`Set.add` the grep has always miscounted; and
`mechanisms/expectations`, which keeps `add`/`sub`/`mul`/`div` because an `Outlook` is generic over
its SUBJECT — no single `D` is true of that store, and the dimension is asserted at each READ by the
module that knows what it asked about. That is written into the file, not just here.

**What the type found, in the order it found it.** A rate declared as a number (`LoanTerms.rate`,
`SubTerms.rate`, `RowTerms.rate`, `Corridor`) — three loan books now say "a span earns a SHARE of
par" the same way, each with a named `PAR` where the two scales meet. A recipe coefficient declared
as a number while `params.ratio()` already returned one — `Technology` widened them back on the way
in. `E-8`: six declared levels, four of them stated per NAMED unit and all six read as per piece,
plus a seventh that was not a price at all. `downTick` flooring NAMED units on the piece grid
throughout the seed. `downTick` flooring a PRICE on the quantity grid in the land market. An equity
delta accumulated as a total when the register keeps it per member.

**`E-11` is this item's own finding and it appeared five times**: `Outcome.price` and `Print.price`
are `PerPiece` and some books clear a RATE — the subordinated raise, the interbank session, the IRS,
the CDS. Each site now names the crossing instead of assuming it. One of them changed an operation
rather than a name: `irs/sizeOf` divided equity by its book's level with `over` (money over a pure
number, which gives money) where the level is a level and the answer wanted is a notional —
`amountOf`. The book still cannot say which of the two it holds; that is what the finding is for.

**Three shadowed names** — `scale` twice in earlier stages, `over` once in the seed — all renamed.
When it happens a fourth time it should be a lint rule.

**And a method note that changed how the later stages were worked.** `Measure<D>` is
`number & {...}`, so it is assignable to `number`: the compiler drives the sweep in the direction of
CONSTRUCTION and not of USE. `div(Ratio, Ratio)` compiles happily. What works is typing the
STRUCTURE — a map, an interface field, a return type — which then forces every read behind it.

Typecheck 0, lint 0, `check:spec`, `check:forbids`, `check:existence` green, tools suite 30 green.
**The engine suite was not run at any point in 2a**: the typing erases, so behaviour cannot change,
and the one behaviour change made inside it (`A-18`) was named as one where it happened. Stages
2b–2e are the eighteen findings, which do change behaviour and will be measured.

---

## Item 2, stage 2b — the four conservation breaks

Stage 2a made the eighteen findings visible to the compiler; 2b fixes the four that create or
destroy value. Unlike 2a these change behaviour, and each is one place where a number was invented
with nothing falling on the other side.

**`A-39` — the wage bill.** `labour/matching.ts:payFrom` computed what actually left the employer —
`share.total`, whole pieces of the money to each worker separately — and returned a boolean. The
caller booked `perMember × headcount`, the unrounded figure, as what the firm had paid, and
`firms/produce.ts` capitalised that into the batch. A firm's equity rose by
`(perMember − downTick(perMember)) × headcount` every period, for every row, and nothing fell. With
headcounts in the millions that compounds through the inventory it was capitalised into. It returns
`Cash` now. Zero is a real answer and means what it says: a per-member wage below one piece of the
money pays NOTHING, because there is no such coin, and the employer owes it (`ctx.owes`) rather than
recording it as paid — that branch used to return `true`. `produce.ts` required no change, because
it already read the published `paid`; the fix is at the writer, which is where Law 4 puts it.

Nothing caught this: `firms.productionCosts` compares the production instruction's equity effect
against `firms.started.wages` and both were `bill.paid` — one number checked against itself, which
is A-10 and A-14's shape — and the `accounts` family was satisfied because the WIP carried the
invented cost.

**`A-68` — the cargo.** The loading instruction wrote `costPerUnit = share / take`, with an
`add(share, 0, 'the freight')` sitting where the missing term had been: what the cargo itself cost
was written off at the quay. It now reads `costOfDraw` off the lots the shipper is drawing from —
the read `register.ts` exports for exactly this — and a delivered unit carries both what it cost and
what the voyage cost. `arrive()` was already right and was the pattern.

**`A-19` — probate.** `handToProbate` was widened (13j) to hand over every money a dead cell held,
because a household can be paid a coupon in a money it does not bank in. `settleEstates`, forty
lines below, still paid out in one: `currencyOf(office.region)`. Every other money arrived at
probate and stayed there — the office never trades, has no fails and no other outlet, so the balance
was permanent and out of the circuit for good, with the accounts family confirming it every period
because probate is a named holder. The office now builds the same set of monies from its own
holdings and pays out in each. What arrives by every door leaves by every door.

**`A-1` — the equity door, and the fix is not the one the plan named.** The plan said "make `bump`
take the total and divide". It cannot: three of `bump`'s eight callers hold the register's own
PER-MEMBER numbers, and making them multiply would have put A-39's rounding back in a second place.
So there is no `bump`. There are two named doors — `bumpPerMember` and `bumpTotal` — and deleting
the old name is what did the work, because every existing caller then had to say which of the two it
was holding. That confirmed the two the finding named (`reseat`, both sides; the issuer's re-mark in
the `credit` case, which uses `op.totalQty`) and left `issue` and `redeem` shorter, since
`bumpTotal` now performs the division each of them was separately remembering to do. `perMemberOf`
is reachable only from `bumpTotal`. Before this, the day a CELL issued something its estate took on
a million households' worth of a liability against one household's equity, and the balance-sheet
family fired on the estate — which is how it was found.

One stale comment removed beside `payFrom` (Law 16): a header still saying it "returns whether it
settled".

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green. The engine suite is still not run — the owner's instruction stands until the plan is done.

---

## Item 2, stage 2c — three rates that were not rates

Each of the three is a number standing where a RATE belongs that was never a rate — a funding share,
a level times a width, a constant — and in each one the comment beside it already described the
number that should have been there. That is the shape of this stage: nothing here was unknown, it
was written down and then not built.

**`A-44` — the saver's comparison.** `households.liquidityPremium` is declared *"per annum OVER WHAT
A DEPOSIT RETURNS"* and was read as the bare 0.005, so `fundOrders`' test `p.offered < required`
compared a fund's offer against a constant. A cell subscribed to a fund offering 0.006 while its own
bank's board paid it 0.02. This was harmless when `portfolio.ts` was written and its header still
said why — *"a deposit … returns nothing at all here, because paying for deposits is a decision a
bank has not been given yet (worklist 11)"* — but worklist 11 landed: `money-market/deposits.ts:
payDepositInterest` settles a real money leg every period for every holder of a bank's money whose
kind has a deposit class, at the rate that bank published on its board. The saving decision had not
moved with it. `households/bank.ts:ownDepositRate` is the read — its own bank's board for its own
deposit class, the same `board()` the switching decision already used — and `required` is that plus
the premium. Zero where it has no deposit class or its bank has published no board is the rate
itself and not a default: what pays it nothing is what it is being paid.

Two stale comments went with it (Law 16): the header above, and a claim in both files that `required`
is *"what it posts DOWN from its own opinion"*, which it never was — the paper bid is the outlook
less the party's own error (§46 B3) and `required` never entered it.

That last is this stage's own finding, **`E-12`**, positioned at item 21: what a household requires
reaches the fund comparison and not the paper bid, so a rise in the deposit board makes a money fund
less attractive to a saver and does nothing at all to what it will pay for a bill. D5.a names
*"paper bought directly"* explicitly, so half the substitution is missing. It is written down and
placed, not chased.

**`A-58` — what a bank will pay for a note.** `securitisation:priceFor` was
`1 − (owed/(owed+equity))²`: the debt share of the BIDDER's own funding, squared, with the variable
named `cost` and the docstring describing a discount read off *"what it actually pays for money"*.
It had no periodicity (Law 8 — the note's tenor appeared nowhere), no relation to any rate this
world produces (a bank funded 90% by deposits bid 0.19 per unit of face on a SENIOR tranche; one
funded 50/50 bid 0.75), and no dependence on the pool at all, so two vehicles with completely
different loan books got the same bid from the same bank. `owedIn` was the wrong quantity even as a
leverage measure: it is what falls due in that money this period less what the bank holds of it — a
funding gap, not liabilities.

It is a bond price now. `poolSchedule` is what the pool promises: every row's own dated cash flows,
each scaled by how much of that row is in the pool, added up by date, divided by the pool's face — a
pass-through pays what the borrowers pay, and the schedule is read off the instruments rather than
forecast (Law 17, Law 19). `priceFor` discounts it with the same `priceAt` the money funds use, at
`costOfFundsIn` — a read of the bidder's own published `bank.costOfFunds` event, its home `perAnnum`
or the named `alsoIn` row for the deal's money. A bank that has published no funding cost, or a pool
that promises nothing, returns `Missing` and does not bid: a refusal is an answer (Law 1), not a zero.

The offer is passed to the bidder. That is not a hole in Observer A4: an offer is described to
whoever is asked to bid on it, and what a buyer sees is the rows it is being sold and nothing else
of the arranger's book.

**What is NOT done, and where it went.** `noteBids` still posts a single point for the bank's whole
spare cash where Clearing A2 wants a schedule. The ladder that builds one — `levelsBelow`,
`rungsOver`, `rungsUpTo` — lives in `households/demand.ts`, and a module never imports a module;
duplicating it would be Law 4. So it moves to the kernel, into `clearing/`, whose A2 that file
already cites. That is a kernel change, so it is INSERTED at its dependency position rather than
done here (Law 10): step **2.19**, where `A-32` opens the same function anyway. `ARCHITECTURE.md`
goes in that commit.

**`A-65` — the option premium, and it was the worst arithmetic in the model.**
`expected × confidence` is a LEVEL times a WIDTH. `confidence` is `width(surprises)`, money per unit
of the underlying — every other reader in the engine uses it that way, subtracting it from a price
or multiplying units by it — and `expected` is money per unit too, so the product is money² per
unit², posted as a price into a book whose tick is a cent. Every premium in this world was therefore
proportional to the SQUARE of the underlying's price level: an option on a line at 100 with 1%
surprises quoted 100 — the whole value of the underlying — and the same 1% on a line at 1 quoted
0.01. The comment directly above the line said the right term (*"its own outlook's CONFIDENCE and
not its level"*) and the code multiplied by the level anyway.

And the premium did not depend on the strike, the right or the expiry: `mine` was built from the
outlook alone, so one party quoted the same number for a deep out-of-the-money call and an
at-the-money put on the same line in the same session, and the strike ladder `openBooks` builds was
a set of books every participant priced identically.

`worthToIt` is two terms and the contract's own terms decide both. **Where exercise stands at the
price this party expects** — `expected − strike` for a call and the other way for a put, the same
distance `intrinsic` takes at a PRINT, taken here at the party's own outlook. **Plus what it thinks
the thing moves before this expires** — its own confidence over the root of the time the contract
runs, which is `impliedMove` read backwards: what that function takes OFF a printed premium is what
this one puts INTO a quoted one, so the quote and the read of the same book are inverses (Law 4),
and the expiry enters. Out of the money by more than it thinks the thing moves, it does not quote —
a decision, exactly like `intrinsic`'s when nobody exercises, and not a floor (Law 6). D7's *"the
premium is what clears"* is untouched: this is the reservation a party brings, not a model price.

**A third dimensional defect the fix exposed.** `price: mine` posted money per unit of the
UNDERLYING into a book whose unit is `optionContracts`, while `impliedMove` divides a printed premium
BY the multiplier to read it back — so the quote and the read of one book were out by the multiplier
against each other, and the `mine > book` comparison that decides which side a party takes compared
the two scales directly. It posts per contract now, and the comparison and the balance-sheet room
calculation use that same number.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green. The engine suite is still not run.

---

## Item 2, stage 2d — the currency reads, and the one door they were all missing

Five findings with one shape: a number in one money added to a number in another, in a place whose
answer has to be a single number in a single money. `Measure<D>` carries the DIMENSION and not the
currency, so nothing in a `Cash` says which money it is and the compiler could not see any of these.

**`A-61` — every central bank remitted its income to the same treasury.** `remit` sent to
`ctx.parties.ofKind(TREASURY).filter(alive)[0]`: whichever the parties store happened to return
first, an insertion-order artefact of the seed's draw and not a fact about who owns anything. In the
four-country world 13j built, all four central banks remitted to ONE country's treasury, each in its
own money, into accounts that treasury holds at three foreign central banks. Three governments never
received the seigniorage on their own money and one received all of it, as an unexplained foreign
transfer; E3 says *"the treasury owns it"*. Every neighbouring module had had the multi-country pass
— `declareVenues` opens a money-market book per currency, `runReceipts` skips foreign legs with a
worked example, the resolution auction filters bidders by currency — and this one was missed.

It is the treasury of the central bank's own region now. Not by importing `money-market`'s
`treasuryOf` (a module never imports a module) but by the same three-line read of the parties store
that `heirsOf` makes: the store is the one writer and both are reads of it. A central bank whose
sovereign has no live treasury records `centralBank.unremitted` and remits to nobody — a refusal is
an answer (Law 1), and defaulting to a stranger is what the old line did.

**`A-47` — a fund's mandate had no money in it, and `A-50` is what that cost.** `eligible` tested
three things — live, kind in `d.eligible`, maturing inside `maxTenorPeriods` — and never the
currency. `d.eligible` for every money fund is `['sovereign.bill']`, which is EVERY SOVEREIGN BILL IN
THE WORLD: `eligibleLines` counted the Japanese and the European ones beside the American, so a fund
divided its spare cash over three times the lines it could actually buy and each bid was a third of
what it meant to be. Then `ordersOf` divided money in one currency by a price in another to get a
size, and `navOf` summed two moneys into a per-share number.

The currency is the fund's own — where it banks, and what its shares are struck in. It is an OUTCOME
of where the fund is and not a field declared beside the mandate (Law 2: a declared number would be a
second representation of a fact the parties store already holds).

**`A-51` and `A-23` — and the reason `inMoney` had no callers.** `inMoney(value, from, to, at)` was
exported on `MechanismContext.valuation` and called by no module at all. The reason is its `to`:
every reader that needed it wanted the same one — the party's own book — and none of them had a
tidy way to say so, so each of them simply added. `Valuation.inOwnMoney(party, value, from, at)` is
that door. It is on `MechanismContext`, `DerivedReads` and `SeedContext`, and on `ParticipantView` as
`inOwnMoney(value, from)` where a participant summing its own holdings asks it. Nothing private is
reachable through it: an FX rate is a print and a print is public (Clearing E1).

Which money a party's book is in is a fact about its REGION, and the valuer is built before the
parties store, so it takes `bookMoneyOf` as an injected read the way it already takes the curve and
the calendar (Law 4: one answer to "whose money is this").

**`worthOf` carries the currency of its answer** beside the value and the period it was marked in.
That is what its readers were missing and it is Law 8: the money is part of the number.

Closed at eight sites, and each was a different consequence of the same omission: `navOf` (a NAV is a
price in one money; a foreign bill's face went into the per-share number), `holdingsWorth` (the
denominator a forced sale's pro-rata fraction is struck on, so a fund short of cash sold the wrong
number of units of everything it held), `capitalOf` (a published capital ratio built out of two
currencies added together), `valueBook` and its exposure ladder (a resolution hole decides who is
paid and who is not, and who ranks where), `wealthOf` and `atRisk` (**A-23** — and a household paid a
coupon in a money it does not bank in is precisely the case Currency C4 exists for), `equity`'s
opening book walk (a listed line's share count is its firm's residual, so the count came out of a
total of two currencies), and `subordinatedOf`.

**And it removed code**, which is what a fix to a cause does (Law 12). `ledger/settlement.ts:inOwn`
was this exact conversion written out by hand, and it was the ONLY place in the engine performing it
— the kernel converted currencies when it booked an equity delta and nothing else in the world did.
It is a call to the door now.

**`moneyOf` is deleted.** It built `money:${bank}:${ccy}` out of a string, beside `ctx.accountOf`,
which is the one writer of which account a party holds a money in. Four call sites read it instead.
Every deletion names the read that replaces it (Law 19).

`docs/ARCHITECTURE.md` carries the structural decision, in this commit.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green. The engine suite is still not run.

---

## Item 2, stage 2e — the local ones, and what they removed

Five findings that each live in one place, and between them they deleted more code than they added.

**`A-5` — three registry functions took a unit and ignored it.** `payable(_ccy, amount)`,
`cashFor(_ccy, value)`, `deliverable(_unit, qty)`. The step offered two answers — drop the parameter,
or make it do the conversion it claims, not both — and reading the code settles it: a PIECE is the
smallest thing there is in any money and any unit, which is what `core/tick.ts` means by the piece
grid, so the answer never depended on which money it was. Where a subdivision DOES matter the reader
is `named`/`downToNamed`, and it asks the registry for that unit's own subdivision by name.

Dropped at 73 call sites in the engine and 8 in the tests. The lint then found what the parameter had
been keeping alive: **nine locals and one function parameter that existed only to be passed to it**
(`const unit = view.registry.derivativeKind(decl.kind).unit` in six modules, `cds:sizeOf`'s `unit`
argument, `funds:feeAccrued`'s `party`), and six imports behind those. All deleted. A fix to a cause
removes code (Law 12) and this one removed a line for every place the claim had been repeated.

**`A-6` — a unit string that said minutes and held hours.** `goods/index.ts` declares
`labourHoursPerUnit × TIME_PIECES / PIECES_PER_UNIT` as `minutes per piece of <subUnit>`. A piece of
time is an HOUR (`registry/grid.ts`: `TIME_PIECES = 1`), so the number is hours per piece and the
declaration claimed it was sixty times itself. `labour/index.ts` still carried the premise that unit
string came from — a comment about a thousandth of an hour being "about four seconds" — which is what
the grid was before. Law 8 says the unit is part of the number; a wrong one is a wrong number waiting
for a reader.

**`A-33` — `lastOwn` has no period bound, and the fix put the bound in the ask.** `lastOwn(kind)` is
the most recent event of a kind for a party from any period ever. `ParticipantView.lastOwnSince(kind,
since)` is the same read with the period in the question, and it answers Missing when the last one is
older — a different answer from a stale one.

Eight callers had written `own.value.period !== view.period` by hand after calling `lastOwn`. They ask
for it now and the hand guards are deleted, which is the half of this that removes code. Five callers
did not guard at all, and every one of them read a number a writer only writes when it has something
to say: `payWages` writes for an employer WITH ROWS. So a firm that shed its last worker went on
force-selling stock at `price: 'market'` to cover the payroll of nobody — XI-2's forced seller firing
on a phantom obligation — publishing that payroll in `firms.funding` as what it is about to have to
pay, which is what a bank lends against, and planning production on hours it no longer employed, for
the rest of the run. `quotedRate` was the same shape on `credit.quoted`: XI-4's "what its debt costs
AT THE MARGIN, NOW, not the average coupon on debt already outstanding" was the last quote the firm
was ever given, carried into its cost of capital for ever.

The bound is `payrollSince(period)` — this period or the one before, because `labour.pay` settles in
cycle 2 and `firms.decide` runs in cycle 0, so a reader before it means the previous period's bill and
one after it means this period's. `treasury`'s two readers walk `journal.forSubject` directly and go
through one `currentPayroll` helper instead.

**`A-38` — a cell's outside option was every kind of money it received.** `reservation` read
`outlook('income')`: everything that reached it, INCLUDING ITS OWN WAGE. So an employed cell's
reservation was its current wage over its own hours and it could never be matched below what it
already earned — a wage that goes up and never down. Appendix B forbids a stated wage rule; this was
one arriving through a read. The other half was its capital: a cell's demanded wage rose one-for-one
with every coupon it was paid, as though a saver were less willing to work.

There is a new outlook subject, **`{ on: 'benefit' }`** — what reached a party that it did not WORK
for. It is observed in the same pass over the ledger as `income`, off the money leg's own `receipt`
(anything but `wage`), so there is no second walk and no second definition of what a party received.
`reservation` reads it, and B1.a's "what it lives on without the job" is now what the code does.

**`A-32`, and `A-58`'s schedule with it.** `levelsBelow(opinion, steps)` placed its levels at
`opinion × k/steps`, so its BOTTOM was `opinion / steps`: a grid of five bid down to 55% of the cell's
own opinion and a grid of 200 down to 0.5%. The number declared a RESOLUTION was deciding how far down
a saver bid at all, which is a SHAPE wearing a resolution's name. It is `levelsBelow(top, width,
steps)` now, spanning the cell's own width below its top price — the same width that put its bid below
its expectation, used once — so both ends are numbers the cell named and `steps` only samples between.

**`pricesOver` was not a defect.** The step said "same for `pricesOver` in `consume.ts`". It is not:
its span is `expected ± width`, both ends the cell's own, and `steps` already only sampled it. It is
the pattern `levelsBelow` now follows. Recorded rather than dropped — "MISSING and OUT OF SCOPE are
different answers" cuts both ways, and a finding that turns out to be wrong is answered, not deleted.

**The ladder moved into the kernel.** `packages/engine/src/clearing/schedule.ts` holds `Rung`,
`rungsOver`, `rungsUpTo`, `levelsBelow`, `levelsUpTo` and `pricesOver`; `households/demand.ts` is
deleted. It had to move: `securitisation:noteBids` needs it and a module never imports another module,
and copying it would be two representations of one thing (Law 4). Its own header already cited
Clearing A2 and A2.a, so it was a clearing concern living in a sector.

`levelsUpTo` is the second curve and it is `A-58`'s remaining half. A bidder stopped by a SIZE rather
than a price takes more of a thing the cheaper it is, all the way down, and what bounds it is the
`limitPerName` it already publishes — so its span is `(0, top]`, which is fixed, and the quantity
converges as the grid refines instead of diverging the way a budget over a price would. That is why
this one may run to zero and `levelsBelow` may not. `noteBids` posted ONE order for the whole of its
spare cash at its own reservation; it posts a curve now, with `securitisation.demand.steps` as its
resolution. `docs/ARCHITECTURE.md` 4.6 carries the decision, in this commit.

`test/resolution/demand.test.ts` was rewritten as the stage went: it asserts the SPAN is invariant at
every grain, which is the property `A-32` names and the one the old ladder failed, and it covers
`levelsUpTo` under a limit. It runs at the end of the plan with the rest.

**Stage 2 is closed apart from `E-9` and `E-10`** (steps 2.4 and 2.5), which are type changes rather
than fixes. All eighteen findings the item named are done.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence` green.
The engine suite is still not run.

---

## Item 2, stage 2f — the two type changes, and what the type found

`E-9` and `E-10` are the last two steps of item 2 that are not the suite. Neither changes a number
in the world as it stands today, and both remove a way for the next change to be silently wrong.

**`E-9` — a dirty price adds two scales.** `plus(clean, accrued)` in `prices/curve.ts:readCurve` and
in `clearing/market.ts` added a per-piece print to an accrual that had been written straight onto the
piece grid out of a number stated per NAMED unit of face. Three places made that assertion, all in
`registry/claims.ts`: the coupon schedule, `accruedOf`, and `cashFlowsOf`'s redemption — each writing
`asPerPiece(...)` on a named number, and the par at maturity as a bare `1`.

`InstrumentKindProfile.due`, `.accrued` and `.cashFlows` now take a fourth argument, a `PriceScale`
— one method, `priceOf`, which `Registry` satisfies structurally, so every caller passes the registry
it already had (42 sites). `ontoTheGrid(scale, i)` crosses through `Registry.priceOf`, which is the
door. TypeScript lets an implementation take fewer parameters than its type declares, so the forty
`() => 0` profiles were untouched: only the five that actually cross needed a line.

The memo in `scheduleOf` stays in NAMED terms. That is deliberate and it is why the memo is still
correct to key on the terms alone — a crossing inside it would have depended on an instrument the
key does not carry.

**Two corrections the read produced, and both matter more than the fix itself.** The step said the
addition is right today "only because par and money share a subdivision". For a BOND that is exactly
it — its unit is `PAR`, declared separately with `perUnit: MONEY_PIECES`, so the crossing factor is
one by coincidence, and it is crossed explicitly now. For a LOAN, a money-market row and a
subordinated note it is not a coincidence at all: those kinds declare
`unit: (ccy) => currencyUnit(ccy)`, so a piece of the claim IS a piece of its money and there is
nothing to cross. Their three `PAR` constants said "the two scales only coincide here (`E-9`)"; they
name the declaration that makes it true instead.

And `curveAt` carried a SECOND `asPerPiece` on the same profile call that `accruedPerUnit` already
wraps — two writers of one crossing. It reads `accruedPerUnit` now (Law 4).

**`E-10` — one field, five dimensions.** `Contract.struckAt` was a `PerPiece` whether the book had
struck a bond future's PRICE, a credit default swap's SPREAD, a swap's RATE, an index future's LEVEL
or a cross-currency swap's BASIS. `StruckAt` is now
`{ as: 'money'; level: PerPiece } | { as: 'rate'; level: Ratio }`.

The tag is the kind's own `quotedAs`, which **already existed** and which the observer was already
reading to label a display (`struckAs: 'money' | 'rate'`, with a docstring saying a reader shown 0.01
with no word for which has been shown a number and not a price). The fact had a writer. What was
missing is that the VALUE did not carry it, so every consumer got a `PerPiece` regardless.

`clearing/market.ts` tags the cleared level once, before admission, so `admits`, `marginLegs`,
`ContractAsk.struck`, `premiumPerUnit` and every kind's `mark` take what they mean. `moneyLevel` and
`rateLevel` are the reader doors and they THROW — a `Mismatch` citing Derivative D7, at the site,
never caught — where a book struck the other kind of level. That check cannot be written while the
field is one type, which is the point of the change.

**And the type found real arithmetic.** The three rate-quoted kinds computed
`valueAt(struckAt, notional)` — a PRICE times a QUANTITY — where a spread or a rate over a notional
is `scale(heldAsMoney(notional), rate)`. Five sites: the CDS mark and its premium leg, the credit
index mark and its premium leg, the swap's mark and both accrual legs, and the cross-currency basis.
They produced the same number only because a `PerPiece` and a `Ratio` are both `number` at runtime
and the notional happened already to be in money pieces — which is exactly the coincidence a
dimension is supposed to stop being load-bearing.

**`E-11` is named at every crossing and stays item 6's.** The PRINT on a rate-quoted book is still a
`PerPiece`, because `Outcome.price` is one type for every market in this world. Each of those marks
now writes `asRatio(print.value.price, 'the spread this book last printed')` and says so. Positioned,
not chased.

One helper deleted before it was ever used: `levelOf` had no caller, and something nobody reads is
not a door (Law 12).

**Item 2 is closed but for step 2.22 — the suite** — which the owner's instruction holds until the
plan is done. All eighteen findings the item named are closed, plus `E-8`, `E-9` and `E-10`. Three
findings came out of the work and are positioned: `E-11` (item 6), `E-12` (item 21), and `A-18`'s
`merge` half (item 12). One named finding — `pricesOver` under `A-32` — turned out not to be a
defect and is answered in stage 2e's entry rather than dropped.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green.

---

## Item 3 — the gather: one call, and a bank employs people

`World.gather` is the only path that runs `venueParticipantDecls`, and across the whole engine
`ctx.gather(...)` appeared **once** — `money-market/index.ts`. Three modules declare venue
participants and one of them was gathered. The chain behind the other two is the largest single
unlock in the model:

```
no gather → no bank bid for hours → no employment row → no labour.wages event
          → linesCovered() === 0 → covers() false for every line
          → dealingOrders() returns [] for every bank, every market, every period
```

So no bank made a market in anything (`A-60`): no bid-offer spread, no inventory, no interdealer
market, `market.noView` on every book households and merchants do not cover, and no primary dealer at
a sovereign auction. And the lettings venue had never had an order in it (`B-5`), so no tenancy was
ever signed, no rent was ever paid, and `housing.rent` — a whole phase — did nothing for ever.

The door was built, documented and correct. It is called now: in `labour.match` for every occupation
venue, and in `housing.lettings` for every rent venue.

**Where the call goes matters and the plan's wording did not settle it.** `labour.match` runs
`runVenue` TWICE per venue — the trade round, then the anywhere round — and a venue is gathered once
per period. The kernel's own `gathered` set would have made a second call a no-op, so putting it
inside `runVenue` would have worked by accident. It is in the phase, before either round reads the
book, and after `publishGoingRate` because a bidder may read what an hour last cleared at.

**And doing it exposed a second global list nobody filtered.** `venueParticipantDecls` is ONE list:
every declaration is asked about every venue that is gathered, and the only thing that stops a module
answering for another module's book is that module's own test. `banks` has one — `sessionOrders`
refuses a venue whose `market` key is not `money`, `staffOrders` one whose `occupation` is not
`BANKING`. `housing`'s participant tested the REGION and nothing else. A labour venue has a region.
The moment `labour` began gathering, the housing participant would have posted DWELLINGS into a book
quoted in HOURS. It tests `venue.clearedBy` now, which is the fact that actually distinguishes them.

That was not a known finding and it is not in the plan. It is what doing 3.1 exposed, and it is fixed
here rather than written down because it is the kind that stops the build: an impossible quantity in
somebody else's book.

**`covers` was one global cutoff (`A-54`).** It walked `view.instruments.all()` and took the first
`linesCovered` of every live line in the world — the same list for every bank. Two desks with
completely different businesses covered the same lines, and a line past that position in the store
was quoted by NOBODY however many banks made its kind. It walks the desk's own candidate set now: its
`BankDecl.makes`, and where a line names its makers, the ones that name it. The cutoff is the desk's,
which is what Dealer Desks A3 means by dealing in a NAME being a second decision from dealing in a
KIND.

**And the forced sale sat behind the coverage gate.** `urgentSale` was reached only after `covers`
returned true, so a bank that could not quote could not be a forced seller — and a bank in trouble is
exactly the one whose desk has stopped covering lines. XI-2's door for a bank could not open. It is
above the gate now: quoting a market and being made to sell are different acts and only the first of
them needs people.

**`3.5` (`A-43`) stays open and item 9 closes it**, as the plan says. `labour/matching.ts:supply`
walks `ctx.parties.ofKind(HOUSEHOLD)` and posts each cell's order itself — the buyer's market
deciding for the seller, which `gather`'s own contract forbids in as many words. With the venue
gathered, `households` can declare a venue participant and `supply` deletes; but a households-side
participant cannot see who is already employed or what trade they have, and that book is one of the
seven `Mandate` gives it. Doing it now would be inventing a read.

Closes `A-54`, `A-60`, `B-5`, `B-6`. Typecheck 0, lint 0, `check:spec`, `check:forbids`,
`check:existence` green. Not measured — the suite runs when the plan is done.

---

## Item 4 — the families that cannot fail

The loop's definition of done is "an item is done when its checks are green", and four of the nine
audit families were green for reasons that had nothing to do with the state of the world. A green
audit had been part of the evidence behind thirteen `done` rows. Audit A1.a, in the spec's own words:
*"a read of two independent things that must agree — never a read of one thing against itself, which
always passes."*

**`A-10` — the currency family read one event twice.** `revalueForeign` computes
`delta = carried × (now − was)` and journals `delta`, `carried`, `was` and `now` together;
`revaluationAddsUp` took `delta` for one side and recomputed `carried × (now − was)` from the other
three for the other. One path twice, passing in every possible world.

The positions come from the REGISTER now: the holder's own lots, valued at the carrying the
valuation derives from the period BEFORE — which is why re-reading it at audit time, after the marks
have run, gives the same number the rate step used. Only the two RATES still come from the event,
because the rate a period OPENED at is a fact nothing else in the engine stores (`rateInForce` is
answering with this period's by the time the audit runs). A carrying that has drifted from the copy
the event took, a revaluation booked twice, one booked to the wrong account, all now show up.

**And it gained coverage nothing had:** a foreign position the FX step never looked at. It is not a
disagreement of sizes — there is no second number at all — so it is reported as itself under
Currency D2.

**`A-14` — the workforce identity added a partition to itself.** `acc.states` was
`(E ? w : 0) + (¬E ∧ W ? w : 0) + (¬E ∧ ¬W ? w : 0)` over two booleans: mutually exclusive and
exhaustive, so it equalled the population on every iteration by construction.

The three states come from three places now. EMPLOYED from the module's own employment book,
INACTIVE from the REGISTRY's age bands read against the parties store, POPULATION from the cells —
so what is left over is the unemployed, and an inactive count that overlaps the employed one is a
real disagreement between the registry's bands and the book. **And a second reading of employment**:
what the module BILLED this period, summed off its own published `labour.wages` events, against what
its book says it employs. Those agree only where `labour.release` took out the rows of employers that
have ceased — which is exactly the failure the check should name.

**`A-42` — two families switched themselves off on the first ageing.** `goods:unitsIdentity` and the
capital-programme's own counted EVERY weight event in the world and stopped comparing when there was
one; `households/lifecycle.ts:age()` journals one every period. Neither had reported anything since.

The guard is deleted rather than narrowed, because there is no case for it to protect against: of
the three ways a weight moves, `splitCell` and `reKeyCell` preserve the total (the children's
per-member holdings are the parent's and their weights sum to its), `dieCell` refuses a cell that
still holds anything, and `weightEvent` has no caller at all (`A-17`). It bought nothing and cost two
families.

**`A-48` — a docstring that claimed more than the check does.** `equityIsZero` said a fund's zero
*"falls out of the wire"*. It does not: `fundShareKind` declares `owes: 'value'` and derives it from
`navOf().perShare`, so `assets − liabilities = 0` is an algebraic identity of the valuation. The
check is KEPT and the docstring says what it can actually catch — a disagreement between two
valuation PATHS, the equity walk of what was booked period by period against `navOf` re-derived now.
Stage 2d made that disagreement possible to have by fixing `navOf`'s currency.

**`A-11` — the flows family exempted a whole cell for a whole period.** A split, a promotion and a
merge copy a book without an instruction, so the new cell went into a `copied` set and every holding
of it was skipped for the period: Money D3 switched off across every line, and everything that cell
then did unexplained by construction.

What has no leg behind it is the copy of the parent's book AT THE INSTANT OF THE SPLIT. A split is
exact — the new cell's opening per-member position IS the parent's — so the parent's remembered book
is the `before` the child is measured from, and every difference from it is a leg like anybody
else's. Only a MERGE's absorbed cell stays exempt: its book really is forgotten with nothing behind
it.

**`A-12` — a comparison that could never be true.** `m.id === i.market.valueOf()` compared a string
to an `Option` OBJECT, at every instrument in every period; the second disjunct did the whole job.
Deleted, and the market name is taken out of the option before the closure so nothing is narrowed by
assumption.

**`A-13` — a population identity that was nowhere.** Part XII names it — *"the sum of cell weights
equals the population it stands for, and every represented party sits in exactly one cell"* — and
neither half was in the kernel family. The only contributor that counted people was `labour`, through
employment, so a cell nobody employs could lose or invent members and nothing looked: the firms'
pools and XI-15 cells generally were unwatched.

Both halves are in the kernel family now, for every cell. The population is a period-over-period
identity: what the cells stand for now, per kind, against what they stood for at the last audit plus
what the five WEIGHT EVENTS said happened in between. The audit's own memory is the second record,
which is the only honest way to have one — nothing stores a population (Appendix B forbids a stored
aggregate). And two live cells of one kind carrying the same key are one population wearing two
states, which is what A4.c forbids one cell over, a whole cell wide.

**`A-4` — and the check is NOT at assembly, deliberately.** `conditionsFor` said *"the world that has
the fact is where the check belongs, and assembly is where it fires"*, and `world/assemble.ts` checks
module ids, dependency cycles, money issuance and bank choices; `exposedTo` appears nowhere in it. A
recipe naming a fact nobody declares multiplied by one for ever, in silence — a crop with no weather
in it looking exactly like a crop having an ordinary year.

The plan said "check it at assembly and delete the paragraph". The read of the code says it cannot be
an assembly check and be this one: which facts a region has is a fact about the PERIOD's published
conditions, not about the module list, and a fact can be regional. So the throw is at the read that
needs it, where the world has already said what it publishes — `Missing`, citing Goods B4, with the
region, the fact and what was published. A world with NO environment module at all still answers 1,
because "this model has no weather" is an answer and "this line's weather went missing" is not. The
paragraph is deleted either way (Law 12).

Closes `A-4`, `A-10`, `A-11`, `A-12`, `A-13`, `A-14`, `A-42`, `A-48`, `B-10`. Typecheck 0, lint 0,
`check:spec`, `check:forbids`, `check:existence` green. Not measured — the suite runs when the plan
is done, and several of these families are now capable of red for the first time, which is the point.

---

## Item 5 — Missing is Missing, carried

The old file's item 2 turned the lint rule on in `core/num.ts` and carried four sites out of it,
correctly, because they are missing MECHANISMS rather than missing values (Law 11). Then nothing
built the mechanisms. These are the four.

**`A-25` — a zero standing where a defect belongs.** `consumptionIsBought` read
`leg.pricePerUnit.some ? value : 0` and carried the zero into the comparison, so a cell that bought
and paid was reported as `took 0 of goods and paid X` — a violation whose SIZE and MESSAGE are about
a missing price and whose citation is about the flow. A physical thing handed to a household with no
stated price is a real defect in whoever drafted the instruction (C2.a: a trade carries its print),
and it is reported as itself now, under Goods F1, with the line and the units.

**`A-34` — labour priced at zero in the one place that reaches a market.** `firms/decide.ts` handles
a missing wage correctly twice — `unitCost` answers `none()`, `worthMaking` switches its test — and
then the INPUT BID read `wage.some ? wage.value : asPerPiece(0, …)`. What an input is worth to a firm
is the output it makes possible LESS the wages that unit still needs, so a zero there makes the bid
too high by `hoursPerUnit × wage`, which for most recipes is the largest term in it. And EVERY firm
is in that state until it has employed somebody, so at world open every input market cleared against
systematically inflated bids and the firms that had never hired outbid the ones that had.

`wagesPerUnit` is an `Option` now: the wage where the firm has one, an honest zero where the recipe
takes no hours at all, Missing otherwise. A firm that cannot price an hour posts no input bid — it
still offers what it is holding, because selling stock needs no view of what labour costs.
`wageFacing` already falls back to the published going rate, so this is the firm that knows neither
its own wage bill nor what an hour last cleared at anywhere.

**`A-45` — "money is free" in every opening period.** `costOfFunds` returned exactly zero on three
paths: a bank funded by nothing, period 0, and a zero-length year. Zero is not "unknown" here, it is
a statement that funding costs nothing, and it flowed straight into `quote()` and into the desk's
edge — so in period 0 every bank in the world quoted as if its money were free, and the cheapest
lender in the world was whichever bank had never paid anybody.

`FundingCost.perAnnum` is an `Option<Ratio>` and each of the three says Missing with its own reason.
A bank that cannot cost its funding does not quote a loan, does not bid in another bank's
subordinated raise, and publishes no reservation. An OVERDRAFT is the one that throws: Money B3.a
says whoever ALLOWED the drawing writes the row that prices it, so a money issuer that allowed one
and cannot price it is a defect at the site, not a rate to be invented.

`publishCostOfFunds` publishes the rate as a bare number or omits the field entirely — a published
event is data, and a reader should never meet an option object inside one. That also keeps stage 2c's
`costOfFundsIn` (the securitisation note price) reading exactly what it read before.

**`A-30` — ignorance reported as certainty.** `ownUncertainty` returned 0 for a cell with no income
outlook, which is not "it has no view": it is "it wants NOTHING EXTRA for holding a claim that
promises it nothing", which is what a CERTAIN cell would want. A cell with no history is the opposite
of certain. It answers `Option<Ratio>` now, and so does the period-of-no-length case.

`heirOf`'s `?? ''` is confirmed gone with the function at old item 14, and a world that does not key
its cells on a cohort says so — `keyOf` throws `Missing` naming the dimension rather than answering
with an empty string.

**And `ownUncertainty` has no caller at all** (`E-13`, positioned at item 21). A saver's bid is built
from its PRICE outlook's confidence (§46 B3, `savingLines`), and the income-uncertainty channel this
function implements reaches nothing. Either it is wired or it is deleted; a mechanism nobody reads is
not a mechanism. It is written down rather than resolved here because which of the two is right is a
decision about §46 B3's shape, not a defect in this function.

Closes `A-25`, `A-30`, `A-34`, `A-45`. Typecheck 0, lint 0, `check:spec`, `check:forbids`,
`check:existence` green.

---

## Item 6 — the derivative books open

Nine derivative kinds were declared and **not one contract of any class had ever been written**:
7,510 option sessions all `noDemand`, 3,680 commodity-future sessions all `noDemand`, 60 CDS
sessions all `noSupply`, the IRS books never run. 58 of COVERAGE's 99 never-reached `MET` marks are
this layer.

**The cause is one shape, in eight of the nine (`A-66`).** Every class builds a party's order as

```
target = <its hedging need, from its own book>
       ± <conviction, IF its own number differs from THIS BOOK'S LAST PRINT>
order  = target − <what it already has>
```

Every one of them is careful that the book's own print must not be the LEVEL it posts — several say
so at length, and one of them names the fixed point explicitly. None noticed that the print is still
load-bearing for the **direction**. With no print the conviction term drops out, every party is left
with its hedging need, and a hedging need has ONE SIGN. So the first session is one-sided, never
crosses, never prints; the next session is the same; and the book is dead for the life of the run.

The one class that worked is the one whose author hit this and built the answer — `fx-derivatives`,
where a party with nothing to hedge quotes a bid a tick below and an ask a tick above its own carry,
under the comment *"a book whose members were all hedgers printed one number for ever"*.

That answer is now in all eight. A party with nothing to hedge posts a spread around ITS OWN number,
sized by what its own balance sheet carries, and the number is its own in every case: the
underlying's outlook (options), the deliverable's print (bond futures), the spot line (commodity
futures), the index's own level (index futures), the floating fixing (the IRS), the cash market's
charge for the credit (the single-name CDS), the weighted average of the constituents' charges (the
series), and its own funding basis (the cross-currency swap). Never this book's last print.

**`6.2` was worse than a missing side.** `index-futures` had `side: 'sell'` as the only side in the
file, its docstring naming one true reason — *"A DESK LONG A BOOK OF SHARES SELLS THE INDEX"* — and
two of the three cases unreachable: `book <= 0` sent a party holding none of the constituents away
with nothing to say, and `want <= 0` meant a desk short MORE index than its book takes could not buy
any of it back. All three exist now.

**`6.3` — two classes read their own book's print as a PRECONDITION** and then posted at a multiple
of it, which is the fixed point at its purest: no print, no orders; a print, and everybody agrees
with it. The series' own number is built from its constituents (each name's own cash-market charge
at the series' weights, over the names this party can price — a name whose paper has never printed
drops out of the numerator and the denominator both). The cross-currency swap's is `ownBasis`: what
this party pays for one money over what it pays for the other, each measured against that money's own
published benchmark, read off its own `bank.costOfFunds`. A cross-currency basis is a FUNDING fact —
a party expensive in dollars and cheap in euros will give up the difference to swap, and one the
other way round will take it — and that is two sides that are not a parity residual (Appendix B: no
parity-formula forward; B3.b: a level is a party's own reservation).

**`6.4` — and the mechanism is smaller and truer than the step said.** The plan named eligible
SECURITIES collateral, to be wired from the repo module. The read of the refusals says otherwise:
every one of the 31,640 is an FX forward whose two parties hold NEITHER LEG. They hold money — just
not the book's. `capacityOf` read `view.cash(ccy)` alone, so a member's room was zero exactly when it
banked in another currency: the gate was not measuring capacity, it was measuring whether the party
happened to hold the right money.

So the missing mechanism is Currency C4: **a member posts what it HAS.** `capacityOf` values every
money the member holds in the book's, at the rate in force; `postedOut` builds the legs out of those
moneys — its own region's first, then the rest in the register's own order, so the choice is stable —
because a margin line is ALREADY per (poster, holder, money). A house taking euros against a dollar
exposure holds a real euro claim and carries that FX position like any other holder, which stage 2d's
`inOwnMoney` books on its own account. Securities collateral is a further mechanism and it is not
what was standing between this gate and a single admission.

`ParticipantView` gained `inMoney(value, from, to)` beside `inOwnMoney`, for the member deciding what
it could post against a book in a money it does not hold. An FX rate is a print and prints are
public, so nothing private is reachable through it.

**`6.5` — `A-69`'s measurement now happens.** `refusedThisPeriod` was documented as *"E4: a standing
measurement"* and called by nobody, which is worse than its absence: a reader looking for the number
believes it exists. The layer publishes `derivatives.unmargined` in its own margin phase. Nothing
raises a limit in response to it (E4: measured, never relieved).

**`E-11` is re-positioned to item 21, not closed.** Its index row said item 6 would close it. Item 6
does not: `E-10` closed the CONTRACT layer — the level a contract carries is tagged by its kind's own
`quotedAs` — and every reader of a rate-quoted PRINT now says `asRatio` at its own door, so the
crossing is named everywhere it happens. What is left is that `Outcome.price` and `Print.price` still
cannot say it. The solver genuinely need not care (a schedule is size against a level either way, as
`derivatives.ts` says); the print STORE is where the claim would live. Saying that is better than
ticking a row the work did not do.

Closes `A-66`, `B-7`, `C-6`, and `A-69`'s `refusedThisPeriod` and `cdsBookOrders` rows. Typecheck 0,
lint 0, `check:spec`, `check:forbids`, `check:existence` green. **Not measured** — and this is the
item whose effects the suite will show most, because eight books that have never printed are about
to, and everything downstream of a first print (`zeroSum` walking a non-empty set, the option-implied
move §46 A3 needs, the margin gate admitting somebody) becomes measurable for the first time.

---

## Item 7 — the three closed lines: the bootstrap, and two mechanisms the plan called wiring

Three of this world's 63 goods have a firm, a recipe, a market and a printed opening price, and no
bidder ever: `dwelling`, `facilities`, `itServices`. They are zero at the seed and nothing can start
them.

**`7.4` is the bootstrap under all three, and it is done.** `firms/decide.ts:plan` returned
`{planned: false}` without an outlook of its own SALES, and `expectations` forms a `sold` outlook only
from an asset leg the firm was a side of. So: a firm that has never sold has no outlook; with no
outlook it makes no plan; with no plan it starts no batch; with no batch it never sells. Three closed
loops at zero from period zero, and every other line escaped only because the seed put stock on a
book for it.

The way out is not a number and not a fallback figure: **you cannot learn what you can sell without
making something.** A firm with a price for its output and a price for everything its recipe names
plans ONE UNIT — the smallest thing that exists, which is the grid (Law 8) and not a declared number
— and finds out. If it sells, its own outlook leads from the next period and this never runs again.
If it does not, it is holding one unit and offers it like anything else it made and did not sell,
which is what a firm that guessed wrong does. Everything downstream is unchanged: `worthMaking` still
has to hold, and the labour and capacity limits still bind, so a line whose contribution does not
cover a wage still starts nothing.

A firm with no sales history has also been surprised about nothing, and that is a real zero rather
than a default: it has no history to have been wrong about.

**`7.1` stays open and item 9 closes it**, as the step says.

**`7.2` and `7.3` are NOT done, and the reason is the work, not a shortfall.** Each turned out to be a
mechanism this world does not have rather than a call that was never made — and doing either as
wiring would have broken something.

`7.2`: a household bidding for a dwelling out of its savings **commits the same money twice**.
`households/index.ts:decide` already divides ONE budget over consumption, a buffer and every saving
line; a housing-side bid spends money that budget has allocated. Putting the bid inside the
household's own budget needs the household to know it is short of a dwelling — an occupancy ratio and
a lease book, both `housing`'s, and a module never imports a module. And a dwelling is a DURABLE:
`demandOf`'s basket is a per-period flow, so a basket row would buy one every period for ever.

`7.3`: nothing names `facilities` or `itServices` as an input, and neither is a recipe input — a firm
buys facilities management per SITE per period and IT support per MACHINE per period, not per unit of
output. `GoodDecl.inputs` only expresses per-unit-of-output. Deriving them from each line's own plant
is the right shape and would take two declared coefficients instead of 126 — **but it creates a
production CYCLE the seed's build-out cannot resolve.** `foundation.ts:652` builds a line only once
every input of it is built, and `power` would buy `facilities` while `facilities` buys `power`.
Neither would ever build, and a large part of the economy would go with them. The circularity is real
— utilities and services do buy each other — and it is the seed's topological order that cannot
express it.

**Both are inserted as item `7b`**, at item 7's dependency position (Law 10: a new idea is INSERTED
where it belongs and you say where), with four steps: a durable call inside the household's budget, a
public read of what a cell needs against what it owns, an overhead list on the recipe that is per
unit of PLANT rather than per unit of output, and a build-out that opens a line on what it can make
without its overheads and lets them arrive in the first period — which is also what happens, since a
new site is cleaned after it opens.

Typecheck 0, lint 0, `check:spec`, `check:forbids`, `check:existence` green.

---

## Item 7b, first half — a household buys the home its people live in

`dwelling` had a firm, a recipe, a market, a printed opening price and no bidder ever (`A-55`). Not
in the household basket, not an input to any recipe, not portable so no merchant carries it, not in a
fund's mandate, not in a bank's `makes`. Owner-occupation was a state the housing module's own header
described — *"a household that owns as many dwellings as its members live in has nothing to rent"* —
and no household in this world could be in.

**A home is the third thing a household can do with its money.** It is not consumption, because
`demandOf`'s basket is a per-period FLOW and a house bought every period for ever is not a house. It
is not a saving line, because a saving line is a claim that promises something and a home is a thing
its people live in. So it is its own call on the SAME `spare` — which is what keeps the money
committed once (Law 4), and was the whole reason 7.2 could not be done as a housing-side bid.

`households.toAHome` is the preference: how much of what a cell has left over goes towards a home it
needs and does not own. `homeBid` posts at the cell's OWN level — its outlook of the line where it
has one, the last print where it has not, the same ladder it buys a loaf on and never a valuation of
a house out of anybody's accounts (Law 3) — for what its share of its spare reaches, and never more
than it is short of: a cell does not buy a second home because it could afford one.

What the bid COMMITS is what the units it bids for come to, not the whole share. The rest is still
spare and goes to the saving lines with everything else.

**And the household has to be told it is short of one.** What a cohort's people need and what leases
a cell holds are `housing`'s facts, and a module never imports a module. `housing.asking` publishes
`housing.shortfall` once per cell per period — needs, owns, rents, and the difference — from the same
read `ordersOf` already makes to decide what that cell offers or takes in the lettings venue, so
there is one computation of it (Law 4). The household reads it under its own name through
`lastPublicAbout`, which is the route every cross-module read uses.

A negative shortfall is published as one: a cell with more housing than its people need is a real
state and is said as one rather than clipped to zero.

`7b.3` and `7b.4` — the overheads and the seed's build-out cycle — are the second half and are next.

Typecheck 0, lint 0, `check:spec`, `check:forbids`, `check:existence` green.

---

## Item 7b, second half — an overhead is not a recipe input

`facilities` and `itServices` are real lines with real firms and, before this, no buyer in any period
of any run. Nothing named either as an input — because neither IS one.

**A firm buys facilities management per SITE per period and IT support per MACHINE per period.** The
amount does not fall when the line stands idle, and `RecipeInput` is per unit of OUTPUT and cannot
say that. So `Recipe` has a second list: `overheads`, each `{subUnit, capitalKind,
qtyPerPlantUnitPerPeriod}`, derived from the plant each line already declares. **Two declared
coefficients instead of a hundred and twenty-six**, and each is a technology: what it takes to keep a
site open and a machine running, per period.

`decide.ts` bids for what its plant in service takes, less what it holds, at its own expected price
for the line — its outlook where it has one, the last print where it has not, the same ladder it buys
an input on. `produce.ts` draws what it HAS and capitalises it into what a unit cost, beside the
wage: a firm that could not buy enough ran a site that was not fully cleaned, which is a real state.
Nothing refuses to start over it — **an overhead is a cost and never a gate**, so it is not in the
production limits.

**And the separate list is what solves the build-out cycle.** `foundation.ts:652` opens a line only
once every INPUT of it is built. Had the services gone into `inputs`, `power` would require
`facilities` while `facilities` requires `power`, neither would ever be built, and a large part of
the economy would go with them — a real circularity that a topological order cannot express. In their
own list they never reach that order: a line opens on what it can make without them and buys them
from the first period, which is also what happens, since a new site is cleaned after it opens.
**`foundation.ts` needed no change at all**, and that is the point of the shape rather than an
accident of it.

A service line buys no service of its own kind, so the two do not clean and support each other into a
loop with nobody outside it.

Closes `A-55` and `A-56`. Typecheck 0, lint 0, `check:spec`, `check:forbids`, `check:existence`
green. Not measured.

---

## Item 8 — the securitisation waterfall: interest is not principal

Worklist 13e says *"a waterfall paying by seniority out of what was actually collected … with a loss
landing from the bottom and nothing stopping it reaching the senior"*, and it is marked **done**. **No
loss was ever allocated to any tranche, whatever the borrowers did.**

The vehicle collects PRINCIPAL AND INTEREST — `LOAN.due` emits a coupon every period and a maturity
at the end, both paid to the holder of record, which is the vehicle. What went out was PRINCIPAL
ONLY: `payTranche` redeems face at par, so `Σ out ≤ Σ face = the pool's opening principal` and the
interest, the whole economic return of the deal, never left.

Three things followed, and the third disabled the module's own subject.

1. A noteholder earned nothing but its discount: it paid `priceFor(...)` per unit of face and
   received exactly face.
2. The interest piled up in a vehicle nobody owns — a residual with no holder (Appendix B) wearing a
   party's name.
3. **`absorb` went inert.** `lost = notes − pool`, and because interest redeemed face, notes fell
   FASTER than the pool: `lost ≤ 0` early and permanently, so the junior/senior waterfall, the
   attachment points, the write-down leg and D4's senior losses were all downstream of a subtraction
   that could not be positive.

**`distribute` pays interest first, by seniority, and then principal.** What the vehicle collected as
interest is READ off the wire — this period's settled money legs into its own account whose `receipt`
says what the money IS to the party getting it (`world/actions.ts` sets `{of: 'interest'}` on a
coupon) — and never inferred by subtraction (Law 19). Each layer's entitlement is its share of what
is outstanding; senior first when there is not enough of it, which is what a waterfall is.

**8.2 is done and NOT as written, and building it as written would have been the defect.** The plan
asked for real `cashFlows` and `due` on `trancheKind`. A pass-through's payments depend on what
borrowers who have not paid yet do, so a schedule would be a FORECAST with no falsification test
(Law 17) — and a real `due` would have the KERNEL pay a coupon this module's own waterfall is already
paying, one fact with two writers (Law 4). What the step was after is that a noteholder should EARN
something, and that is 8.1. Its yield is what the payments came to against the price it paid: a
measurement of what happened (item 23), never an input to a price (C3, Law 3). The docstring where
the empty schedule sits now says all of that instead of just asserting the emptiness.

**C6 is asserted, both ways.** `absorb` journals `securitisation.absorbed` — what the pool lost,
before any of it is allocated — and the ownership family reads it against the `tranche.writtenDown`
events per vehicle per period. Two records of one fact reached from opposite ends (Audit A1.a), and
it fires in both directions: a pool that lost more than was written off the layers, and a layer
written down against a pool that lost nothing. Neither number existed at all before this, because
`notes − pool` could not be positive.

**The residual has a holder and the vehicle winds up.** The arranger already keeps the bottom (C4.a)
so the deal's equity is named; what had no holder was what was left INSIDE the vehicle once the notes
were redeemed. Nothing ceased it: `fails: ['cash','solvency']` will not fire on a party with positive
equity, the kind has no owner and no distribution, and `distribute` removed a deal only when the
vehicle had ALREADY ceased by some other route. `windUp` pays the residual to the arranger and ceases
the vehicle to it — and only when it holds nothing, which is XI-3's rule rather than a special case.
Item 14's `Process` was not needed: `ctx.cease` is the door and Register F2 is what makes every
reference resolve.

Closes `A-57` and `B-8`. Typecheck 0, lint 0, `check:spec`, `check:forbids`, `check:existence` green.

---

## Item 9, first stage — the insurer exists and can write a policy

`A-9`: four independent blockers, each alone fatal, and a fifth inside the pricing. `B-2`: the module
had no `seed`, so no party of the kind was ever created. Between them the sector was a set of
declarations that nothing in any world could reach.

**(1) It could not register a policy in any world.** `policyKind.unit` returned the CURRENCY CODE
where a `UnitId` is wanted, through the file's one `as unknown as` cast — so `Instruments.add` threw
`Missing [Appendix A] unit USD does not exist` at the first policy anybody tried to issue. The module
declares `COVER` in its own `units` and never used it. The cast is what let that compile; without it
the type said so.

**(2) Nothing ran.** `phases: []` and `participants: []`, so the price of cover, the capacity, the
venue and the audit family were all reachable only from a test. There is a phase now: every insurer
quotes what a unit of cover costs it out of its own claims experience and its own capital (A4.b),
once per money, before the markets — because what it writes this period is capacity it then has to
stand behind. The BUY side (a firm that stands in a physical fact and would rather not, B4) is item
14's; until it exists these sessions come back `noDemand`, which is a measured state and not an
absence.

**(3) A session that struck a price and moved no money.** `runCover` cleared the book and DISCARDED
`outcome.fills`, recording only how many there were. Every other market in this world turns a fill
into an instruction (Clearing D2, D3); this one turned it into a count. It writes the policies now:
the buyer pays the premium and the insurer ISSUES it the cover, both legs in one instruction in the
same period (Law 5). A unit of cover promises one unit of its money at the end of its term, discounted
at the sovereign curve of that money — which is what gives the sector its duration (B2.b). The TERM
is a convention of the contract and not a forecast of when a claim arrives: what a claim costs is
`claimsSeen`, read off what this insurer has actually paid, and it is the PRICE that carries it.

**(4) A zero that reads as a discharged liability.** `derive` returned `priced.some ? priced.value :
0` directly under a comment saying *"Missing is Missing — never a zero that would read as a liability
the institution has discharged"*. `derive` answers a number or it does not answer, so it throws
`Unpriced` at the site with a citation, which is what the kernel already does for a derived kind that
derives nothing.

**(5) It read its own holdings for policies it had written.** A policy is the insurer's LIABILITY —
the beneficiary holds it — so `written` was always empty and `experience` was always its claims over
nothing. It reads `instruments.issuedBy(self)` now, which the register indexes both ways.

**`B-2`: one insurer per region** that has a bank to hold its money, seeded and banked like any other
institution.

`9.1`, `9.2`, `9.4`, `9.6`–`9.9` are the rest of item 9 and are next.

Typecheck 0, lint 0, `check:spec`, `check:forbids`, `check:existence` green.

---

## Item 9, second stage — nothing was ever short, so nothing ever borrowed (`docs/IMPLEMENTATION.md` 9.4)

**What.** `A-67` and `B-3`: the securities-lending module builds the venue, the fee clearing, the
title transfer, the collateral pledge, the manufactured payment and the recall, and **every way in
was called by nobody**. `runBorrows`, `wantsToBorrow` and `returnLoans` were exported and referenced
nowhere; the one phase walked `state(ctx).open`, which nothing ever pushed to. Nine `MET` marks
stood against it.

**The cause was not a missing call site.** Being short is a POSITION a party takes, out of its own
view of a line it holds none of, and this module can see neither — so there was nowhere for the
first call to come from. `wantsToBorrow` is the tell: it was exported from a module, and a module
never imports another module, so the only party that could ever have called it was one
securities-lending owns, and it owns none. The function could have no caller by construction.

**The door.** `SystemModule.borrowNeeds`, the shape `termsOffered`, `bankChoices` and
`creditDecisions` are: the module that owns a party kind answers what a party of that kind must
deliver that it has not got, the most it will pay per period for the use of it, and what it will
pledge. Exactly one module may answer for a kind (Law 4); `ctx.borrowsWanted()` asks once a period
and stamps each answer with the party that gave it. It is declared in the reach tally, so a door
nobody ever answers is visible as never-reached rather than as an absence of evidence.
`docs/ARCHITECTURE.md` §4.9b carries the decision.

**The first answerer is the dealing desk** (`banks/dealing.ts:deskBorrows`), and it is the only
honest short this world has: a market maker whose own view of a line is BELOW what the book last
printed, which already holds less of it than it is prepared to have a position in. A desk that can
only offer what it holds is not making a market in a line it thinks is dear — it sells down to
nothing and then stands there with a bid nobody hits, which is `binds: 'position'` in every period
after the first. What it does instead is borrow the line and offer that, and the position it then
has is a short. Three numbers decide it and none of them is new: its own view against the print
(XI-13), the same three constraints its bid is cut to (its own limit, the room in its whole book,
the money it has — D1, D4, F1), and what a period of it costs it.

**Two phases.** `borrow.session` runs BEFORE the markets, so borrowed paper is in the desk's free
balance when it quotes — a borrow struck after the session it was for is paper nobody could sell.
The return is folded into `borrow.economics`, which was already there and walking an empty book.

**The fee clears, because there is one book per line and both sides carry levels.** It used to clear
a session PER BORROWER against every lender posting `price: 'market'`: one bidder and no seller with
a level to compete on, so `outcome.price` was that borrower's own reservation however much paper was
on offer, and the docstring's *"scarce paper is dear and abundant paper is cheap"* could not happen
at any supply. Now every borrower of a line bids in the same book and every holder of it offers at a
floor of its own — its own recent surprises about the line over what it thinks a unit is worth
(§46 B3), which is what a period of the loan puts at risk. Two holders of one line who have been
differently wrong about it want differently much to lend it, so the supply side is a CURVE and
scarcity moves the level: the cheap lenders run out before the bidders do. A holder with no outlook
on the line has no number and does not lend — a refusal and not a zero, which is E3 (*a fee of zero
is a cleared price only if somebody posted it*).

**The double-post goes with the grouping.** `post` appends and the book is emptied only at the top
of a period, so a second borrower of the same line re-posted every lender's offer into the same
venue and the book showed twice the supply that exists. One book per line posts each lender once.
`allot` then pairs them: the borrower that bid most is served out of the lender that asked least,
ties in the parties' own order, so a run is the same run twice from one seed.

**`wantsToBorrow` is deleted** (Law 12: a fix removes code). Its `willPay` was `owed/(owed+equity)`
— the debt share of funding, dimensionless, with no periodicity — called *"what a period of its own
money costs it"*: `A-58`'s non-rate with `A-58`'s comment, in a second module. The sentence the
comment was reaching for is right and is true where it now lives: a borrower will not pay more for a
period of somebody else's paper than a period of its own money and capital costs it, which is
`dealing-quote.ts:rateOf` — the rate the desk already prices its own quotes off.

**Three things the read of the code found that the step did not name.**

- **The haircut came in on the BORROWER's side**, which is C1 inverted: the party at risk asking the
  party that owes it how much cover it would like to give. It is the lender's now, and it is how far
  apart it thinks the two marks can move in a period — its own surprises on the borrowed line and on
  the collateral, which is the same read its floor is, used for the other question C1 asks.
- **A failed return built an instruction with no legs.** `returnLoans` cleared its leg list in the
  failure branch and pushed back only what had come back, so a borrower with nothing to return
  settled an empty instruction — which throws `Money D1` at the settlement door. That is the COMMON
  case for a short that sold the paper, so the mechanism would have stopped the world on the first
  failed return, which is why it is fixed here and not written down (the exception the three rules
  name). The dead `legs.shift()` before `legs.length = 0` went with it.
- **D1 says the lender KEEPS the collateral**, and keeping it is title, not a lien: a loan that has
  terminated cannot go on securing anything, and collateral encumbered to a row that no longer
  exists is units nobody can reach. The lien is released and the units are delivered to the lender,
  in that order and as two instructions — the register answers *can this move?* against what is
  bound BEFORE the instruction runs, so a release and a move of the same units in one instruction
  fails its own pre-check.

**And the term.** `securitiesLending.borrowTermPeriods` (4), declared TECHNOLOGY with an owner: how
long the paper stays out before it goes back. It is what makes D2 possible — a borrower that still
wants the position has to buy the line back before the term is up, which is what *"shorts must buy"*
means — and it is not a forecast of when a short is closed: a borrower that still needs the line
asks again in the same book, at whatever the fee has become by then.

`covers` in `banks/dealing.ts` is factored into `coveredLines`, one writer of which lines a desk is
in, because the borrow answer needs the list and the quote needs the membership test.

**COVERAGE**: `Securities Lending` A4, B1, B2, D1 and E3 go MISSING → MET, 9 of 21 to 14 of 21, and
Part 0's row with them. Every one of the fourteen keeps `B-12`'s **NEVER REACHED** marker: a way in
is not an outcome, and whether the module has now produced one is a MEASUREMENT that waits for 23.0
like every other. `E-14` is the finding this raised — `Securities Lending C2/C2.a` are not built,
nothing re-marks the collateral — and it is positioned at **13.8**, with prime brokerage, because
marking both sides of a position and calling the difference is §15 C1's mechanism with two callers.

**And `docs/IMPLEMENTATION.md` is pruned to what is left.** It was 3,900 lines and is 1,434: items
1–8 and 7b are closed, so their sections, their steps and their findings are deleted — this file is
the ledger of what was done and that one is the plan of what is not. Four things were carried out
before the sections went, not dropped: `2.22` (run `npm run check`) is **23.0**, where the owner put
it; `7.1` (housing's mortgage guard) is **15.0**; `3.5` is already `9.6`; `7.2` and `7.3` were
superseded by 7b, which closed. Part 4 keeps only findings that are still open.

`9.1`, `9.2`, `9.6`–`9.9` are the rest of item 9. **`9.1` is next, and it should have been first** —
the two steps that touch seven modules were left while the two that touch one each were taken, which
is Law 10's "appended rather than inserted" in miniature.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green.

---

## Item 9, third stage — an agreement has a kind and its own terms (`docs/IMPLEMENTATION.md` 9.1a)

**What.** The spine of item 9, inserted at its dependency position (Law 10) because nothing could
be migrated without it. `Agreement` held `{debtor, creditor, ccy, owed, what, why}` — no room for a
wage, a trade, a rent, a lien or a waterfall — so "migrate the seven private books" would have meant
throwing six sevenths of every row away. An agreement now carries a declared `kind` and its own
`terms`, the same construction `Terms` is for an instrument and `Contract['terms']` for a contract.

**The division.** Four things are the kernel's and are the same for every kind: two named parties, a
money, what is owed now, a state. Everything else belongs to the kind, and only the module that
declared it can read it. The kernel holds the row, indexes it by debtor, creditor and kind, and
ranks it in an estate; it narrows nothing and branches on nothing.

**`what` is deleted, and it was the defect.** It was free text, and six modules wrote six spellings
of "in arrears" into it. The one reader that had to tell two rows apart did it by comparing a string
it had built at the other end — `owed.what !== \`dividend ${action.id}\`` — which is A-52's shape
exactly: a fact recovered from a string, matching nothing the moment either end spelled it
differently. It is `terms.action !== action.id` now, behind a structural predicate.

**Six kinds, declared where they belong** (`SystemModule.agreementKinds`, the declaration
`instrumentKinds` and `partyKinds` are): `labour.wagesInArrears` (with the period whose pay did not
arrive — two periods of arrears are two rows, not one doubled), `labour.severanceInArrears` (with
what ended the employment; Firm Birth D2.b ranks it), `equity.dividendDeclared` (with the action and
the line), `households.estateUndelivered`, `ratings.feeInArrears` (with what was rated) and
`treasury.levyInArrears` (with the period assessed). A kind no module declared cannot open a row:
the store refuses it where it is opened, the way the registry refuses an unregistered instrument
kind. Each has one writer of its terms (a typed builder) and a STRUCTURAL type predicate to narrow a
row back — the idiom `isShare`, `isPolicy` and `isRow` already use, and the one the `no-kind-branch`
lint rule permits.

**Two consequences, both of them corrections.**

- **`owed` may be ZERO.** The store was built for payments that had FAILED, so `owed > 0` was true
  of everything in it by construction and the guard said "an agreement owing nothing" was not one.
  It holds commitments now, and a performing employment owes nothing this instant and is still an
  employment. What is refused instead is a NEGATIVE amount — arithmetic impossibility and not a
  floor (Law 6): a party owing minus five is owed five, which is the other row with the parties the
  other way round.
- **`MechanismContext.owedBy` and `owedTo` are deleted** and replaced by the read-only store,
  `ctx.agreements` (Law 12: the fix removes code). They were the only two questions a book of
  arrears could answer, and a module that declares a kind has to read its own book back — `ofKind`
  is how. Neither had a caller left in the engine after the equity change.

`ARCHITECTURE.md` §6b carries the decision. The seven books are seven kinds of this one noun and
`Mandate` is the eighth; `9.1` proper migrates them, one bounded change each.

Typecheck 0, lint 0 (src and tests), `check:spec` 208 tags, `check:forbids` 4 over 205 files,
`check:existence` green.

---

## Item 9, fourth stage — the employment book is the kernel's (`docs/IMPLEMENTATION.md` 9.1b)

**What.** The first of the seven. `EmploymentRow`/`EmploymentBook` lived in `ctx.state`, so an
employment ranked nowhere in an estate, no other module could see who was employed, and the audit
families that check the workforce identity CLOSED OVER the labour module's own book — a module
handing the audit the answer it is checking, which is the one thing Audit A1.a says a family may not
be. The rows are agreements of kind `labour.employment` now, with the trade, the wage, the hours,
the start, the productive-from date and the headcount in this module's own `EmploymentTerms`.

**What stays in the module is the INDEX, and the file already said why**: how a module finds a row
is not something it knows (Observer E3). `byWorker`, `byTrade` and `byEmployer` hold IDS and every
read resolves them against the kernel's store, so the arrangement is a traversal and never a
mirror — a stale copy of a fact is the defect Law 19 is about, and an index of snapshots would have
been one. It is rebuilt from `agreements.ofKind(EMPLOYMENT)` if it is asked for before anything
wrote to it, which is what makes the kernel the source rather than the destination.

**An employment owes ZERO** and that is the point of 9.1a's relaxed guard: the wage falls due at the
end of the period and is paid then, and a wage that does not arrive is a `labour.wagesInArrears` row
of its own. The commitment and the arrear are two rows because they are two facts.

**`book.next` and `employmentId` are deleted.** The row IS the commitment, so the kernel gives it
its identity; a module inventing a second id for the same thing is Law 4. `EmploymentId` is
`AgreementId`.

**A headcount change is an EVENT now.** `separate` did `row.headcount = sub(...)` on a mutable
object in a private book. The kernel's row is frozen, so part of a cell leaving is
`ctx.restate(id, terms)` — a new kernel door, added here with `endAgreement`: same two parties, same
id, different terms, journalled as `agreement.restated`, and a change of KIND refused (an employment
cannot become a lease). `leave` terminates rather than deleting, because a job that vanished from
the record would leave a severance nothing could be a severance FROM.

**`AuditView` gains `agreements`** — the third register, beside the holdings and the contracts — so
the workforce-identity and employer-exists families read the world instead of the module. That is
the change that makes them families rather than assertions.

`labour`'s noun declaration is re-written to say what is actually left in the slot: the SKILL, what
each cell can do, which is a fact about a person and not about any job — a `View`, carried to 9.9.

Typecheck 0, lint 0 (src and tests), `check:spec` 208 tags, `check:forbids` 4 over 205 files,
`check:existence` green. Six books left: lease, invoice, stock loan, loan, covenant, deal.

---

## Item 9, fifth stage — the lease is the kernel's, and housing keeps no book at all (9.1c)

**What.** The second of the seven. `Lease`/`LeaseBook` lived in `ctx.state`; they are agreements of
kind `housing.tenancy` now. The TENANT is the debtor and the landlord the creditor, which is what
makes an unpaid rent something a tenant's estate divides.

**Housing keeps no state slot afterwards at all.** `LeaseBook`, `emptyBook`, `bookOf`, the
by-landlord/by-tenant index and the module's whole `nouns` declaration delete — the entirety of what
this module was keeping turned out to be a thing the kernel should own, which is what a migration
looks like when it is the right one.

**The index deletes because it is not needed, and that is the interesting part.** It existed so that
`letOut(book, who)` and `taken(book, who)` would not walk every tenancy in the world for every
party. Both are reads of a party's own commitments now, through that party's own view — so the
question is asked of the party rather than of a private book this module kept about everybody and
answered out of on their behalf. Observer A4 is the rule, and the performance problem the index
solved was a symptom of breaking it.

**`ParticipantView.commitments()`** is the door that makes that possible: every agreement this party
is a side of, and none between any other two. A row where one party is both sides cannot exist, so
the debtor list and the creditor list never overlap. It is a party's own state — exactly as private
and exactly as knowable as what it holds — and it is what lets a party answer for itself about a
relationship another module owns. That is `A-43`'s fix (9.6) in advance: a household will be able to
say whether it is already employed without the labour module building its schedule for it.

`leasesOf` and `allLeases` were the same function under two names and are one.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green. Five books left: invoice, stock loan, loan, covenant, deal.

---

## Item 9, sixth stage — the invoice book was a mirror, not a noun (9.1d)

**What.** The third of the seven, and **the plan's text was wrong about it, which the read of the
code settled.** `trade-credit`'s `invoices` slot was declared a placeholder for `Agreement`, with
the same boilerplate `why` the other six carry. It is not one: **an invoice is an INSTRUMENT** in
this world — it has an issuer, a holder, an issued amount and terms, it is carried at amortised cost
and it can be discounted — and the register is its kernel home already.

What the book held was `{ id, seller, buyer, due }`, and every one of those four is on the
instrument: the id is the instrument's, and the other three are its `InvoiceTerms`. So it was a
second copy of facts the world already held, which is the mirror Law 19 is about, and the fifth row
of Appendix B's "stale mirror". Its docstring gave a reason for keeping it — *"so nothing asks an
instrument what kind it is (Law 15)"* — and that is a misreading: filtering a list by kind is not
branching a MECHANISM on one, `isInvoice` is this module's own structural predicate declared for
exactly that, and the lint rule that enforces Law 15 accepts it.

**The book is deleted and the ageing reads the register**: what the seller holds is the register's
answer, what each row promises is the instrument's, and it is overdue when the day on its own terms
has gone by. Nothing is stored and nothing can go stale.

**The counter was the one thing in it that was not a copy**, and it is replaced by a read rather
than kept: `freeRow` asks the register which `invoice:<seller>:<buyer>:<n>` is free. One pair can
trade twice in a period — two goods, two books — so the period does not name a row on its own, and
naming it by the period would have collided at `ctx.issue` and stopped the world. The register is
the one writer of what exists, so it is the one that can say which name is free.

`tradeCredit` declares no nouns at all now.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green. Four books left: stock loan, loan, covenant, deal.

---

## Item 9, seventh stage — the stock loan is the kernel's (9.1e)

**What.** The fourth of the seven, and the book 9.4 had just made reachable. `StockLoan[]` in
`ctx.state` becomes agreements of kind `securitiesLending.stockLoan`. The BORROWER is the debtor —
it has the paper, it has to bring it back, and it owes the fee every period — and the lender is the
creditor, which is what makes an unreturned line something a borrower's estate has to answer for.

The line, the units, the collateral, the lien, what was posted, the fee and the period it opened are
`StockLoanTerms`; `loansOpen` is a read of `agreements.ofKind(STOCK_LOAN)` and there is no state
slot left in the module. Like the employment and the tenancy, a loan owes ZERO the instant it is
struck: the fee falls due at the end of the period and `charge` moves it then.

`returnLoans` no longer splices a row out of an array — it ends the commitment, `terminated` and not
`discharged`, both when the paper comes back at term and when it does not (D1). That distinction is
the one 9.1a's four states exist for: the paper coming back is a loan running its course, not a debt
being settled, and a failed return ends it too.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green. Three books left: loan, covenant, deal.

---

## Item 9, eighth stage — the covenant memo was a mirror too (9.1f)

**What.** The fifth of the seven, and **the second one the plan's text was wrong about.**
`corporate-bond`'s `covenants` slot was declared a placeholder for `Agreement`, and its `why` argued
the case: *"a covenant is a TERM of an agreement, and a test of one is that agreement performing or
in breach. With no agreement to be a term of, the test is a private note and a breach reaches
nobody."*

The first half is right and the conclusion does not follow. **A corporate bond must not become an
agreement**: it is an INSTRUMENT — it has holders, an issued amount and a market — and
`register/agreements.ts` says in as many words what it is for, *"the register holds what is OWNED;
this holds what is OWED where the owing is not a security"*. The covenant is a term of that
instrument and it already lives on it, in `CorporateBondTerms.covenants`, where the test reads it.

**What the slot actually held was `tested: Record<line, quarter>`** — a memo of which line had
already been looked at against which set of accounts, so a breach is not announced twice. That is a
second copy of a fact the journal already holds, which is the mirror Law 19 names, and it is
replaced by the read: `journal.forSubject('covenant.breached', bond)` against the quarter.

The difference that falls out is the honest one. The old memo was written for every line it LOOKED
at, breach or not, so a passing line was marked as "tested" and never looked at again on those
accounts. The test is a read of one published report and the instrument's own terms — pure, and
free to repeat — so what must not happen twice is the EVENT and not the test. A line that passes is
re-read every period and produces nothing, which is what a VERIFY does.

`corporateBondModule` declares no nouns now.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green. Two books left: the loan and the deal.

---

## Item 9, ninth stage — the deal is a relation, and the rest of it was a read (9.1g)

**What.** The sixth of the seven. `securitisation`'s `deals` book held
`{vehicle, arranger, ccy, rows, pool, layers}` and its `why` said *"a deal is an agreement among an
arranger, a vehicle and the note holders"*. An agreement has exactly TWO named parties, so "among"
is the wrong word — and reading the six fields settles what the deal actually is:

- `vehicle` is a party; `ccy` is what its layers are denominated in; `layers` is what it issued;
  `pool` is on every tranche's own terms already. All four are READS, and keeping them beside the
  deal was a mirror.
- `arranger` is not derivable, and it is the whole of XI-11: **the arranger keeps the bottom, so the
  risk did not leave** (C4.a). That is a relation between two named parties — the vehicle owes the
  arranger what is left when the notes are paid — which is exactly an agreement.
- `rows` is not derivable either, once a row has amortised off the vehicle's book. Which loans left
  whose book IS XI-11's traceability, so it is the deal's one carried fact.

So a deal is an agreement of kind `securitisation.deal`, vehicle the debtor, arranger the creditor,
terms carrying `sold`. `layers` is now a read ordered by each tranche's own `seniority` rather than
by the insertion order of a stored list — which said the same thing only while nothing was ever
issued out of order.

**Two counters went the way the invoice's did.** `book.next` named the vehicle (`vehicleId(bank, n)`)
and the deal's display name (`"<arranger> pool <n>"`). The vehicle's name is asked of the parties
store — `freeVehicle` walks up until a name is free, because the store is the one writer of who
exists — and the deal is named for the vehicle that IS it (Law 9), which is the name the world
already carries.

**`windUp` and `distribute` end the agreement** rather than splicing an array, in both the run-off
case and the case where the vehicle has already ceased and its estate owes the notes.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green. One book left: the loan.

---

## Item 9, tenth stage — the loan was already the register's, and 9.1 closes (9.1h)

**What.** The seventh and last, and **the third the plan's text was wrong about** — for the same
reason the invoice was. A bank loan is an INSTRUMENT: `LOAN` is a registered kind with `LoanTerms`,
an originator, a rate, a schedule and a holder, and `banks` keeps no book of loans at all. There was
nothing to migrate.

What `banks`'s `book` slot actually held was two things, and only one of them belongs there: the
DRAWINGS the kernel has allowed this period and that have not become rows yet (Money B3.a — the slot
is correctly declared `working` for exactly that), and a COUNTER, `next`, used to name a loan and a
subordinated line.

**The counter is deleted and it was wrong in a way worth writing down.** It counted ATTEMPTS rather
than rows: `b.next += taken.length` moved on every raise whether or not anything was taken, and
`b.next += 1` sat after a `settle` that could fail, so a loan whose settlement failed still consumed
a name. The gaps in the sequence meant nothing at all. Both `loanId` and `subId` now ask the
register which name is free, which is the same fix the invoice and the vehicle got: the register is
the one writer of what exists, so it is the only thing that can answer, and nothing has to be kept
true beside it.

`runRaise` loses its `n` parameter and `writeSub` its counter argument with it.

**That closes 9.1: all seven.** Employment and lease and stock loan and deal became agreements —
bilateral commitments with two named parties, dated terms and a state, visible to the estate and to
every other module. Invoice and covenant and loan turned out to be MIRRORS: an invoice and a loan
are instruments and the register was already their home, a covenant is a term of an instrument and
the slot beside it was a memo the journal already held. **Four nouns and three mirrors**, and the
plan called all seven nouns — which is what reading the source rather than the plan is for.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green. No `standsInFor: { noun: 'Agreement' }` is left anywhere in the engine.

---

## Item 9, eleventh stage — `Mandate` exists (9.2a)

**What.** The eighth kind of agreement, and the one item 13 is blocked on. A mandate is a commitment
between a POOL and a MANAGER: the pool owes the manager its fee, the manager owes the pool its
judgement. That is what splits a fund into the three things it actually is — a pool that holds and
has no opinions, a mandate that rules, and a manager that decides.

It was a `FundDecl` row: a fact about this world's DATA rather than about these two parties, which
nothing outside `funds` could read and which no manager agreed to. `MandateTerms` carries `mayHold`
(A4's real constraint, which `eligible` now reads off the pool's own commitment) and `leverage`.

**`leverage` is the point of doing it now.** *A hedge fund is a mandate with leverage*, and it lived
on `fundKind.borrows` as a hard-coded `false` — a fact about the world stated as a fact about a
CATEGORY, which said no pool anywhere may ever be levered. That is what 13h claimed to have built
hedge funds on (`B-14`'s shape). Every mandate this world draws still says `false`, so the kind's
`borrows: false` is left saying the same thing while it is true, and its docstring now says what it
is: a SHAPE with a scheduled death, item 13.2, which opens the door for a credit decision to ask a
pool's own mandate. **It is not opened here because no mandate answers `true` yet, and a door nobody
answers is `A-67` again** — which is the lesson of this item, applied to itself.

**`SeedContext.owes`** is new and is the other half: a fund that exists at period zero was set up by
somebody, on terms, and a seed states an opening STOCK (Seed A3). A bilateral commitment is as much
an opening stock as a holding is, and pretending the mandate was struck in the first period would be
a flow nobody was a side of. It writes no journal event, because nothing happened.

**`fundManagerKind.objective` was wrong and is corrected.** `itsMandate` is what a party somebody
else set up and wrote the rules for is for — a pool, a central bank, an insurer. It was on the
MANAGER because the pool and the manager were one thing wearing two party ids. They are two now: the
pool is run under a mandate and the manager is a business that competes to be given one, so its
objective is `theResidual`.

The five remaining placeholder nouns (`View` ×4, `PublishedStatement` ×1) point at **9.9** rather
than at item 9 as a whole, which is where they are actually migrated.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:existence`
green. `9.2b` is the fee.

---

## Item 9, twelfth stage — two corrections and a new gate (9.2b)

**The fee does not clear, and saying it would have been the defect.** The step's claim is that with
`Mandate` built, *"two managers can now bid for one"* and the placeholder dies. The read of the code
says otherwise. A manager in this world has no cost base at all: it employs nobody, holds nothing it
must fund, and pays for nothing. Two of them in a book would bid each other down to the tick, and a
fee of one tick is not a cleared price — it is a competition between parties with no reason to
refuse. **The missing mechanism is a manager with a cost base** — people it employs over the assets
it runs, which is the shape `banks/staff.ts` already has for a dealing desk — and it is worklist
**13o**, asset managers with strategies. Law 11 exactly: the misbehaving number is not the work item,
the missing mechanism is.

**`fundManagerKind` is not deleted either.** The step calls it *"a party whose only behaviour is
answering where it banks"*, which is true and is a statement that **the manager has no behaviour
yet** — not that the kind is wrong. A manager is a real party with a real balance sheet that
receives a real fee; deleting the kind would mean the fee had no payee, and there is no other kind
it could be (a `FIRM` here produces goods from a recipe). What was actually wrong about it was its
OBJECTIVE, and 9.2a fixed that. Item 13 gives it behaviour.

**What this stage did do is close a hole that has now cost four defects.**

A placeholder names the item that kills it, and a noun in a module's bag names the item that gives
it a kernel home. Both are enforced at assembly — one without a death throws. **Neither guard asks
whether the item is still open.** So an item closes, the stand-in it was meant to kill is still
there, and nothing anywhere says so: it has become permanent while still carrying the words that
say it is temporary, which is the state Law 2 exists to prevent.

`B-14` is one finding of exactly this shape. Reading the fee placeholder found three more in the
same minute: two fee placeholders and a central-bank reserve share pointing at worklist **13h**, and
the population count pointing at **13f** — all closed, and the worklist's own header says 13f and
13h never produced an outcome. Each is re-pointed at the item that will actually do it: the fees and
the reserve share to **13o**, the population to **13n** (which is where `A-17` — nobody in this
world is ever born — is closed).

**`tools/check-deaths.ts` is the check, and `npm run check` runs it.** It reads the worklist's own
table and the plan's headings and ticked steps, and fails on any `worklistItem` or `planItem` naming
something done, absent or unparseable. A rule broken four times should be a check rather than a
reminder — which is the project's own instruction, applied to the project's own instruction.

Typecheck 0, lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files, `check:deaths` 11 of
11, `check:existence` green. `9.6`–`9.9` are the rest of item 9.

---

## Item 9, twelfth stage, second half — what was left open is queued where it will be done

**What.** `9.2b` left two things undone for stated reasons and pointed four placeholders at worklist
rows. Pointing a placeholder at an item is worth nothing if that item does not carry the work — **it
is the same hole one level up**, and the check as first written could not see it: a worklist row is
one line in a table, and "13o is open" says nothing about whether 13o will delete `fund.fee.*`.

**`PlaceholderDeath.worklistItem` is renamed `item`, and a death may name either list.** The name
had gone stale with the project: `docs/IMPLEMENTATION.md` is the ordered plan and is where the first
open item is taken from, and three of these four deaths belong to plan items with no worklist row at
all. `check:deaths` resolves an `item` against the worklist's open rows AND the plan's headings and
unticked steps, so a death can name the list the work is actually queued on.

**Three steps are INSERTED, each at its dependency position** (Law 10, said out loud):

- **12.6** — delete `seed.membersPerCohort`. Fifteen million a cohort is the plainest placeholder in
  the seed, and its own `why` already named what ends it: a population with births and deaths in it.
  12.2 and 12.3 ARE that mechanism, so item 12 is where it dies. It named worklist 13f, closed.
- **13.9** — the management fee, and what a manager COSTS. Two things in one order, because the
  second cannot work without the first: a manager HIRES in the labour venue and what it can run is
  the hours it pays for over the assets a mandate carries (`banks/staff.ts:linesCovered`'s shape);
  then the mandate is competed for, one book per pool, each manager's own cost base its floor. The
  placeholder dies in the same change and `fee` moves off `params` onto `MandateTerms`, because at
  that point it is an OUTCOME. It named worklist 13h, closed.
- **16.6** — delete `seed.crossHoldingShare`. What a central bank's reserves are MADE of is a
  cross-border question, so it belongs with the cross-border item rather than with asset managers: a
  reserve manager BUYS foreign paper, in the market 16.2 and 16.3 open, and what it ends up holding
  is then an outcome. It named worklist 13h, closed. It is the last cross-border placeholder.

The check now refuses a death that names something in neither list, so a step cannot be re-pointed
at a plan item that does not exist — which is what it did when these three were first written, and
is how the three got written.

Typecheck 0 (engine, app and tools), lint 0, `check:spec` 208, `check:forbids` 4 over 205,
`check:deaths` 11 of 11, `check:existence` green.

---

## Item 12 rewritten before it is taken: a firm is born in the pool and emerges from it

**What.** A correction from the owner, taken before item 12 is started, and it decides the item's
shape rather than adding to it. **Firms are born in the SME pool, and they emerge from it to named
status when they outgrow the sector.** The whole life cycle runs through §42.

**What item 12 said before.** Step 12.1: *"A firm is started by somebody, out of something. The
founder is a named party with a reason and a balance sheet the equity cheque comes out of."* That
describes a firm appearing directly as a NAMED party — which is the modelling line §42 A6.b forbids
in as many words: *"a weight of one is a named firm — so the boundary between this sector and
Corporate Credit's is not a modelling line but a SIZE."* Nobody founds a company with a bond line.

**What it says now.** A birth is `cells.weight(cell, 'entry', 1, cause)` into a small-business cell,
funded by somebody real, and the named corporate sector is where firms ARRIVE rather than where they
come from. Two things follow and both are the point:

- **Item 11's A6.c promotion cannot mean anything until births exist.** With no entry it can only
  move firms the SEED put in the pool — a fixed stock draining upward, which is §42 E4's *"entry is
  the accounting identity of exit"* wearing a different face. So 11.10 builds the door and the
  threshold; **12.4 is where it fires because a firm GREW**.
- **The boundary becomes a size a firm actually crossed**, which is what A6.b asks for, rather than
  two sectors that never exchange a member.

**The hard part is named rather than assumed.** A promotion takes one member out of a cell and makes
a named party of it, and the member's per-member state has to go with it — the same rock `13d.1` ran
aground on four times and solved with `reKeyCell`: nothing moves, the cell SPLITS exactly and the
part that left carries a different key. A split of one member leaves a cell of WEIGHT ONE, and A6.b
says a weight of one IS a named firm — so either that is the answer and `representation` is a fact
about the weight rather than a field, or crossing to `representation: 'named'` needs a kernel event
that does not exist. **Item 12 decides it in its record before writing the step**, and 11.10 is
written against whatever that says.

**The order changes with it.** Part 1 gains a third arrow — **11 before 12** — and item 12's row
says `needs 11`. It was ordered 11 → 12 already, for unrelated reasons; now the dependency is real
and stated. `A-17` is re-marked as closing across both: 11 builds the door, 12 makes it fire.

Item 12's other steps keep their places: household formation (12.3) is the same door at the other
end and is what makes `seed.membersPerCohort` deletable (12.6), `cells.merge` (12.5) is `A-18`'s
other half, and the entry/exit pair (12.7) is §42 E4 — which is why 11.12 is written red and stays
red until this item lands.

No code changed. Typecheck 0, lint 0, `check:deaths` 11 of 11, `check:existence` green.

---

## Item 9, thirteenth stage — the seller's schedule is the seller's (9.6, `A-43`)

**What.** `MechanismContext.gather`'s own contract says a venue's schedules are *"built by the module
that owns that party, with that party's own view … building somebody else's schedule inside the
clearing phase instead is that module deciding for a party it does not own"*. The labour market was
the one venue in the engine where the SELLER's schedule was built by the BUYER's market:
`labour/matching.ts:supply` walked every household cell, read each one's outlook through
`ctx.participant`, decided what that cell would work for and posted the order itself.

`households` declares a `venueParticipant` now and `supply` is deleted.

**The division is not "everything moves", and getting that right is the whole of the change.** What
a household decides is what it will work for — its own `benefit` outlook over its own hours, which
is A-43's complaint in one line (*"a household's reservation wage is `households`' subject"*, and
`A-38`'s outside-option defect lived in `labour` for exactly that reason) — and it reads the public
going rate to decide whether to offer at all, because D1.c publishes it every period.

What stays with the venue is the venue's own rules, applied to what it gathers:

- **B3**: a person is in exactly one state, so a cell that already holds a job is not also looking.
- **B1, F2**: only the workforce is in the book, measured against the same retirement age this
  module's own workforce identity is measured against.
- **A3, A3.b**: the two rounds — the trade you have, then the trades you do not.

None of those is the seller's to know, and a market deciding who is in its book is the market's
business. `eligible` is where they live and it is a filter over `ctx.posted`, not a walk over the
world's parties.

**Two facts about a PERSON moved to the registry** (`PEOPLE_PARAMS`): how many hours one person has
to sell in a week, and the age at which they stop. Neither is a fact about the labour MARKET — a
market is where hours are struck, not what a week holds — and neither is private to households
either, because the market has to size a match in the same hours the seller offered. They live
beside the cohorts, which is this registry's statement about people, and it is the construction
`registry/physical.ts` already uses for the `goods.*` ids that two modules both name. **The
declarations stay with `labour`**: what is shared is only the id both sides ask the one register for.

**COVERAGE**: `Labour B1`'s citation moves to `households/index.ts:willWork`, because that is where
the decision now is. `A-43` is closed and its text deleted; 21.2 is ticked, having confirmed
`supply` is gone and nothing calls it.

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files,
`check:deaths` 11 of 11, `check:existence` green. `9.7`–`9.9` are the rest of item 9.

---

## Item 9, fourteenth stage — being on the list is not permission (9.7, `B-14`)

**What.** `B-14` is the finding about positioning as a protocol: a finding was placed into worklist
13h, 13h closed, the work was not done, and the source went on naming a future that had already
passed. Its subject is `TRADES_CONTRACTS = [BANK, FIRM]` — whose reasons a contract book asks for —
with no fund on it and *"none scheduled to be"*.

**The fix is not adding `FUND` to a list, and finding that out is the work.** Whether a pool may take
a position is `Fund Shares A3`'s question and a MANDATE is what answers it — so the two facts that
were one while every kind on the list was under no mandate come apart:

- **Being on `TRADES_CONTRACTS` says the layer SPEAKS for parties of this kind** when a book asks. It
  has to, because a contract book is the layer's and one face per book is Law 4.
- **Whether a given party may take a position is its own module's answer.** `ParticipantView.mayTrade`
  is the door, `SystemModule.tradingLimits` is how a module answers, and a kind nobody answers for
  may trade anything — the absence of a rule is not a prohibition. That is the OPPOSITE default from
  `borrowNeeds` and deliberately so: there, silence means a party has no reason to be short; here it
  means nobody has said it may not.

`MandateTerms.mayWrite` is the term, and every mandate this world draws says `[]`. A money fund does
not write derivatives, and **saying so is what lets the layer speak for a pool at all**. So no fund
posts in a contract book today — by a term of its own contract rather than by its kind being left
off a list, which is the difference the finding is about. The layer asks in `markets` and again in
`orders`, because `everyone` can add a book `markets` never named and a mandate that forbids a kind
forbids it however the book arrived.

`FUND` goes onto the list in the SEED and not in the layer, because `FUND` is a module's kind and a
module never imports another module — which is what the constant's own docstring already said would
happen: *"a world with funds or insurers in it has more, and the world that assembles them says so."*

**The seed comment named a blocker I would otherwise have walked into, and it is now queued rather
than repeated.** A fund's equity is zero by construction because its own claim on itself absorbs
whatever its book comes to — and the pass that re-marks that claim reads the REGISTER, where a
contract is not (Derivative X1). A pool with a derivative position carries a mark its share value
was never told about, which is a fund WITH equity: **83,247,864 on `etf.us`**, measured the first
time funds were let in here. Nothing reaches it today because no mandate writes anything, but 13.2
is about to draw one that does. So **13.6 is rewritten to open that pass FIRST**, and 13.2 now says
"not before 13.6" in as many words. That is the whole lesson of `B-14` applied to `B-14`'s own fix.

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files,
`check:deaths` 11 of 11, `check:existence` green. `9.8` and `9.9` are the rest of item 9.

---

## Item 9, fifteenth stage — two of the five were mirrors (9.9a)

**What.** 9.9 names five stores standing in for a kernel noun. Reading them found that **two are not
homeless nouns at all** — they are second copies of numbers the same call already publishes, which
is the third and fourth time this item has found that shape (the invoice, the covenant memo, the
deal's five derived fields).

**`funds`' previous NAV.** `Book.previous` was written at the end of `strike`, out of the `perShare`
the same call had already recorded on `fund.struck` — a PUBLIC event. And the fact exists for the
public: D2 says a fund competes with a deposit, and *"the competition D2 names cannot happen against
a number nobody can see"* is the module's own comment, three lines from the private copy of it. It
is a read of the last strike now. One thing improves with it: a fund that missed a period returns
over the period it actually last struck in, which the private copy said nothing about either way.

**`banks/reserves`.** `ReserveMemory` kept `Record<bank, number[]>` — the last N weeks of its own
reserve account — and every entry was the `move` field of a `bank.buffer` event `publishBuffer` had
just recorded. **The neighbour in the same file was already doing it the other way**: `worthOfMoney`
reads `moneyMarket.print` over the same memory window and stores nothing. `bufferOf` reads
`bank.buffer` over the bank's own `bufferMemory` now, and the caller hands in this period's move
because the event carrying it has not been written yet — one read, one answer, no ordering to
remember. `banks` declares one fewer noun.

**The plan's claim about `View` is false and 9.9 is re-written to say so.** It reads *"four kernel
nouns were built and the module stores that stand in for them were never migrated"*. There is no
`View` store in `register/`, and `docs/RECORD.md`'s own entry for old item 6 says why — it built the
VOCABULARY (`Subject`, `about`, `subjectOf`) and states plainly: *"`credit` is expressible and
nothing forms one yet … folding `ratings`, `research` and `banks/reserves` into the noun — three
private stores that are this thing in three shapes — follows the same way."*

So the rest of 9.9 is a BUILD, and it is **9.9b** with the question it has to answer first stated in
it: `expectations.outlooks` IS the noun's implementation — adaptive, formed from the wire, §46 B1 —
while `ratings` and `research` are JUDGEMENTS one party holds about another, formed from `ctx.blind`
and not from an observable. Whether `View` is one store or two mechanisms sharing a subject is the
decision, and it goes in the record before the code.

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 208, `check:forbids` 4 over 205,
`check:deaths` 9 of 9 (two fewer, because two placeholders are gone), `check:existence` green.

---

## Item 9, sixteenth stage — whoever must deliver may borrow (9.8, `C-1`'s ETF row)

**What.** `C-1`'s largest single row, seven tests: **an ETF cannot create.** A creation is delivered
IN KIND — a pro-rata slice of the fund's own book (G1.a) — so a desk that does not hold every line
of the basket cannot create, whatever the premium. `deliverable` says it in one line: what a desk
can make is what the line it holds LEAST of backs, and a line it holds NONE of makes that zero.

Measured: **a premium of 0.28 of NAV**, twenty-six times a period of carry, with nobody able to
close it — and `E3` running one way for the life of the world, because the only desks that could
create were the ones that happened to hold the whole basket already.

**The missing mechanism was the borrow market**, which item 9.4 made reachable, and this is the
sentence the finding ends on: *whoever must deliver may borrow.* A desk short of a line it must
deliver borrows it, delivers the basket, and takes the shares — and it owes the line back, which the
shares it now holds can redeem into. That is the arbitrage as it actually works and every leg of it
is real: a lender who is paid, a fee that cleared, collateral that left the desk's free balance, and
a position that has to be closed.

`deskBorrows` is the borrow door's **second answerer** (the first, from 9.4, is the desk that thinks
a line is dear). It borrows only what it is SHORT of, and only for units it has room for and would
actually create.

**One read, two actors.** `arbitrage` and `toCreate` both need the same numbers — is this premium
worth closing, what is a share of the book worth, what room does the desk have, what is the basket —
and if they disagreed a desk would borrow for a trade it will not do. So the per-venue read is
factored into `etfGaps` and both act on its answer (Law 4: one writer of a fact). Nothing about what
`arbitrage` posts changed; the numbers on `bank.arbitrage` are the same numbers from the same read.

**And the new check earned itself.** Splitting 9.9 into 9.9a/9.9b left four `View` placeholders
naming a step id that no longer existed. `check:deaths` failed the build and named all four. That is
the second time in three commits it has caught a stale pointer, including one of my own making.

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files,
`check:deaths` 9 of 9, `check:existence` green. `9.9b` is the last of item 9.

---

## Item 9, seventeenth stage — `View` is not a store this kernel is missing (9.9b)

**The decision the step asked for, made before the code.** `View` was to be a kernel noun holding
"one party's assessment of another", with four module stores folded into it. Reading the four says
that is wrong in three different ways, and none of them is a migration.

**`expectations.outlooks` is §46's own subject matter and is `physics`.** The kernel already REACHES
it — `OutlookProvider` and `ParticipantView.outlook`, which is this architecture's answer to
"exactly one module answers for a kind". Moving the store into the kernel would move §46 B1's
adaptive formation with it, and Law 15 puts a mechanism in a module. A belief is also private by
right (Observer A4: no party sees another's state), and an outlook nobody else can see is the whole
of what A2 means by *personal*. What old item 6 built was the VOCABULARY — `Subject`, `about`,
`subjectOf` — and **its own record says the store was never what was missing**.

**`ratings.published`, `research.said` and `research.since` were mirrors.** Both are OPINIONS SOLD —
that is what a rating and a research estimate ARE — so both are announced, and the last announcement
is the current opinion. Two other readers already took it that way (`cds` and the observer read
`rating.action`; `consensus`, in `research`'s own file, reads `research.estimate` and
`research.dropped`). These two modules were each holding a private second copy of what they had just
published. That is the **fifth and sixth** instance of this shape in item 9.

What is left in each is real and stays private for a reason. `ratings` keeps the PATIENCE
bookkeeping — the grade its measure says today and how long it has said it — because an assessor
whose wavering was visible would be publishing the grade it is thinking about, which is the opposite
of A3's stickiness. `research` keeps `seenTo`, which is bookkeeping about a read: publishing it would
tell the market which reports a desk has got round to.

**The labour `skill` is not an opinion.** It was pointed at `View` by 9.1b, and a trade is not an
assessment of anything; no module but `labour` asks. Nor is it derivable from the employment history,
and the case that proves it is `separate`: when part of a cell leaves, the leaver is a NEW cell
holding no row, and *"an unemployed baker looks for baking"* (A3) is exactly the fact that would be
lost. XI-10 — who can do what, and what it costs to change — is this module's subject matter.

**So `registry/nouns.ts` has no `View` placeholder left, and not one of them moved.** The count of
homeless nouns falls because four declarations were WRONG about what they held, which is what an
ontology register is for: it made four modules say what was in their bags, and saying it is what
showed that three of the four bags held a copy and the fourth held the module's own subject.

**Item 9 is closed.** Five findings (`A-9`, `A-43`, `A-67`, `B-2`, `B-3`, `B-14`), the seven private
books, `Mandate`, `mayTrade`, the borrow market, `C-1`'s ETF row, and a new gate. Three placeholders
remain in the engine and every one names an open item.

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files,
`check:deaths` 5 of 5, `check:existence` green.

---

## Item 10 — the corporate bond is issued

**The thing this world could not do.** `mechanisms/corporate-bond/index.ts` has declared the kind,
the id, the profile with `cashFlows` and `due`, the seniority the waterfall reads, the cross-default
and the covenant test since 13f, and **nothing anywhere issued one**. `testCovenants` walked an
empty set every period for the life of the world; three `MET` marks stood on a line that could not
exist; 51 more Corporate Credit clauses stood behind it. The clause the module is FOR — a firm
funding itself in a market rather than at a bank — had no mechanism that put a firm in a market.
That is `B-1`, and it is closed.

**A firm compares two prices for the same money.** Both are prices somebody else made and neither is
invented here. What a BANK charges it is `credit.quoted` — the keenest of the quotes it was given,
which is already the outcome of banks competing for it (Banks Lending C3.a), published with the size
that bank will actually lend. What the MARKET would charge is `bank.reservation` — what holders have
published they require to hold this name (E5) — and the keenest of those is where a book for its
paper would start. It comes when the market is cheaper, or when its bank will not lend it enough at
any price, which is A1's *"a firm large enough to reach a market is not at the mercy of one lender"*.
A firm nobody has quoted at all is in the second case by construction, and a firm no holder has
published a requirement for has no book to come to and does not open one to find out (Appendix A).

**It does not price its own issue off either of them (Law 3).** Those two decide whether it COMES.
Its RESERVATION is the whole of the decision made again with money behind it: the price at which the
issue costs it exactly what its bank quoted. Below that the market is dearer than the loan and the
paper is withdrawn (C4) — which is not a bound but the alternative it already has — and a firm with
no quote walks away at the keenest requirement itself, the least anybody said they would take. **No
concession parameter was invented**, and the treasury's `concession` was deliberately not copied: a
sovereign has no bank to compare against and needs one; a firm has the comparison itself.

**What it promises is its own published accounts as this borrowing leaves them.** B2 says what a
given firm promised is an outcome of what it had to promise to be lent to, and there is no
negotiation in this world to produce one — so it promises not to get WORSE than this issue leaves
it: the leverage its published balance sheet reads with this face added to what it owes, and the
coverage its published earnings read against what this line costs it a year. Both are the arithmetic
`testCovenants` will do on the same two published numbers, so the promise is exactly *"no worse than
the day I made it"* and not one covenant number is invented. **A covenant struck before the face was
added would breach on the next publication by construction** — a false breach, and worse than no
covenant, because it is the mechanism reporting its own arithmetic as the issuer's failure. The
headroom a real negotiation would add is what a negotiation is (item 17), and its absence makes this
the tightest covenant a lender could ask for rather than a loose one: a breach here is never a false
negative.

**A firm that has published nothing does not borrow in a market**, and one that published a loss has
no cover to promise. Terms nobody can test are not terms (Reporting A2), and a negative promise is
not a promise. Reporting's lag therefore does real work: nothing issues until the first accounts are
out.

**One line per issuer per maturity, and that is C8 for free.** `corporateBondId` took a bare `n` that
named nothing; it now takes the maturity, which is the half of the name that distinguishes one of a
firm's lines from another (Law 9). A debut brings a fresh instrument and a return to the same date is
a TAP — added face on paper that already prices, cleared in the same solve as its outstanding stock,
at its own price and with the issuer's walk-away riding on it. Which of the two it is falls out of
the NAME and needs no flag. A tap pays the line's own coupon, because a coupon is locked at issuance
(N5.a): what moves is the price it gets, never what it promised.

**One shortfall, one channel (Law 4).** A firm publishes what it is short of once and both channels
read it, so a bank writing a loan against a number the firm has just raised in the market would fund
the same hole twice — money nobody needed, which is the residual with no holder Appendix B forbids.
`runRequests` now reads the issuer's own announcement off the market like anybody else and stands
down. A book that did not clear raised nothing, the firm is short again in its next accounts, and the
bank lends then: that is the cost of a failed auction (C4) and it is a lag, not a loss.

**The blocker that would have made this a `noDemand` every period.** No bank declared `corporate.bond`
in `makes`, so the first issue would have found an empty book and the mechanism would have looked
built and dead. A desk that takes a view on a company's SHARE has one on its CREDIT, so the
high-appetite desks now make both.

**The book is the kernel's, and it has to be.** Holders post schedules — a size at a level, which is
what C2.a says an indication is — one solver strikes the one level at which the book fills (C3), and
who got how many units comes out of that book and nowhere else (C5). A second book built in the
module would be a second answer to the question the clearing system exists to answer.

**What was NOT built, and was not faked.** Step 10.1 asked for *"the size is worth the fixed cost of
an issue"*. That cost is C6's underwriter fee, and there is no underwriter: C1's arranger, C6's fee
out of the proceeds, C7's risk between commitment and placement, C10's syndicate and C11's basis are
all item 17.1, and a `corporateBond.issuanceCost` invented here would be a number standing in for a
PARTY (Law 2). Item 10 issues directly into the kernel's book, which is C2–C5 in full and C6 without
its fee. A2.b's target — a leverage or a rating a management manages towards — is 17.1a, and A3.a's
scheduled principal in the service number is 17.1b; `Corporate Credit A2`, `A2.c` and `A3` are marked
PARTIAL for exactly those.

**The one declared number is a market convention.** `corporateBond.tenor`, 60 months, TECHNOLOGY,
stated in MONTHS because that is what the calendar takes (Law 8) — a tenor in years would be
converted somewhere, and the conversion is the place a duration stops being the number it was
declared as. It is not a forecast of how long the firm needs the money: what it needs is what it
published it is short of, and the term is the market's. `MONTHS_PER_YEAR` lives in the treasury
module and a module may not import another, which is the lint rule doing its job.

COVERAGE re-marked: `Corporate Credit A1`, `B2`, `B3` lose **NEVER REACHED**; `C2`, `C3`, `C4`, `C5`,
`C8` and `G1` become MET; `A2`, `A2.c`, `A3` and `C6` become PARTIAL with what is missing named.
§7 goes from 7 MET / 4 PARTIAL / 51 MISSING to **13 MET / 7 PARTIAL / 42 MISSING**, and the world's
NEVER REACHED count falls from 100 to 97. `B-13`'s *"almost nobody borrows"* loses one of its four
causes — there was ONE credit channel, so a firm whose bank would not lend it enough simply stayed
short — and what is left of `B-13` is a measurement of a finished world, which is item 23 (Law 11).

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 208 tags, `check:forbids` 4 over 205 files,
`check:deaths` 5 of 5, `check:existence` green.

---

## A correction from the owner: a bank loan is not distributed, and a leveraged loan is not a loan

Not an item — a correction to the mechanism (Law 1), folded into the plan at the positions its own
dependencies put it and recorded here because it **removes** work and a removal that is not written
down comes back.

**What was wrong.** Answering what of the primary market is built, I set out C10's syndicate and
C11's best-effort/backstop basis as things a future item would build *for corporate credit
generally*, which reads as though a bank loan could be syndicated out to investors. It cannot, and
not because this world has not got round to it.

**What is actually true.** IG loans stay inside the banking system — one bank, or a club of banks.
There is no distribution mechanism, so **C10's syndicate is a SECURITIES underwriting group and
nothing else**. A club of N banks lending to one borrower needs nothing built: C9's shape is one row
per lender per borrower, so a club is N rows, and the only thing a club has that this world lacks is
somebody to arrange it — which is 17.1's party, not a second kind of loan.

**A leveraged loan is a SECURITY** — the floating-rate counterparty of the fixed-rate bond — issued
through exactly item 10's path. That is 17.0, and it is the cheapest step in item 17: the same reason
to come, the same walk-away, the same covenants off the same published accounts, the same market and
the same tap, with one instrument kind's own profile reading a fixing where the other reads a locked
coupon. It is what answers `B4`, and it is a second KIND sharing one mechanism rather than a second
mechanism (Law 15, Law 4). The fixing is `index.benchmark` — what the overnight book PRINTED, the
same read the IRS floating leg uses — so it is a cleared rate and not a posted benchmark.

**And the reaction is the one item 10 already built.** When a borrower outgrows its bank the bank
stops increasing the line, and the REFUSAL is what pushes the issuer into the public market. That is
`issueBonds`'s second reason (*"its bank will not lend it enough at any price"*).

**It is an ADDITION and not a replacement, and the first draft of this entry got that wrong.** A firm
reaches the market for SEVERAL reasons and the price comparison is a real one (owner: *"it reaches
the market for multiple reasons, don't delete it"*). Two are built; at least three more are real and
unbuilt — TENOR (a bank lends a year; a bond runs five), SIZE beyond any single lender's appetite,
and diversifying the funding so that one lender's retreat is not the end of it. Each has its own read
and none is a special case of another, which is exactly why `issueBonds` must keep taking reasons
rather than collapsing to the sharpest one.

**The FORBID this makes explicit, and it already holds.** No bank loan reaches a party outside the
banking system. `LOAN` declares `market: none()` and `pricing: 'carriedAtCost'`, so a loan cannot be
posted into any book; the only transfer path is securitisation's sale into a `VEHICLE`, and what
investors buy there is the `TRANCHE`, a security. It holds by CONSTRUCTION, which is precisely the
kind that breaks silently — give the kind a market one day and nothing would complain — so 17.8
guards it as an audit family, with the design question named: "the banking system" is a set of party
kinds, and enumerating it inside the family is the kind branch Law 15 forbids, so it belongs on the
party kind as declared data and carries an `ARCHITECTURE.md` change with it.

**There IS a leveraged loan market, and the first draft of this entry denied it.** `docs/WORKLIST.md`'s
**M9** note says the leveraged loan index waits on the loan market, and it is right: the leveraged
loan market is a SECURITIES market, which is what 17.0's floating-rate note trades in, and item 10's
issuance path already opens one per line. What does not exist, and must not be built, is a market in
BANK loans.

---

## Three more owner corrections, and the third one names a cause the other two were symptoms of

**1. A firm reaches the market for several reasons, and the price comparison is one of them.** The
previous entry demoted it to a special case of the refusal. Wrong: `issueBonds` has two reasons
because they ARE two, and at least three more are real and unbuilt — tenor (a bank lends a year;
`banks/index.ts` sets `maturity: drawn.y + 1`, and a bond runs five), size beyond any single lender's
appetite, and diversifying the funding so one lender's retreat is not the end of it. None is a
special case of another, which is why that function must keep taking reasons rather than collapsing
to the sharpest one.

**2. There IS a leveraged loan market**, and the previous entry denied it. It is a SECURITIES market
— which is exactly the point of the correction that a leveraged loan is a security — and item 10's
path already opens one per line. What does not exist, and must not be built, is a market in BANK
loans.

**3. "Roll the bank bonds mechanism the same way corp bonds work. There shouldn't be 3 different
mechanisms."** This is the one that names a cause. The question that led to it was why a bank's
subordinated debt has no price, and the honest answer turned out not to be "it needs a secondary
market" — it is that `banks/subordinated.ts` **is a private copy of the issuance machinery the kernel
already has**, and the missing price is one of four things the copy drifted into:

```
:185  raiseVenue(bank)          a venue of its own
:234  ctx.openVenue({...})      opened by the module
:246  clear(ctx.posted(venue))  the module calls the solver ITSELF
:255  asRatio(outcome.price)    the book clears a RATE, not a price  (E-11)
:300  let id = subId(bank, 1)   ONE INSTRUMENT PER LENDER
:313  ctx.issue({... market: none() })   and so: no price
```

The second of those is the worst and I had not seen it: `subId(bank, n)` advances per FILL, so a bank
that raises from three investors ends up with **three instruments carrying the same promise** — the
exact opposite of Law 9 and of what item 10 established one commit ago. A second name for one promise
is one promise written twice (Law 4).

So **10d is a deletion**, not an addition: the venue, the `clear` call, the per-lender line and the
hand-rolled instruction all go, and the price, the single name and one of `E-11`'s five rate-quoted
books go with them (Law 12: a cause has one fix and it removes code). It needs nothing new — item
10's path opens a market per line, and `publishReservations` already makes every live bank an obligor
*"whether or not it has paper outstanding right now"*, so the holders' schedules are published
already.

**And the correction applies FORWARD, which is why 10b.0 now exists.** Commercial paper is the next
thing this world issues. A new module writing its own venue would put the third mechanism back the
week after 10d deleted it, so the step that says otherwise is written before the module is.

**4. Securitisation is for any non-tradable claim (10c).** `saleable` gates on `isLoan` — a kind
branch Law 15 forbids — and `arrange` starts a deal only on a capital shortfall, so investor demand
can never pull one. What "not tradable" IS needs no list and no branch: `pricing ===
'carriedAtCost'` (no market exists for it) AND `liabilityOfIssuer` (a named party owes it). The two
together are the definition of securitisable and that is not a coincidence — if it had a market the
bank would sell it, and if nobody owed it there would be no payments to tranche. Inventory drops out
on its own (*"A tonne is nobody's promise"*); the invoice comes in, which is most of what 17.5's
factoring wanted. A maturity-mismatch test was proposed and **withdrawn**: `poolSchedule` cuts the
notes against the pool's own aggregated flows, so a pool of one-week rows makes one-week notes and
the mismatch is not expressible.

**What the other two `carriedAtCost` families turned out to be, checked rather than assumed.**
`derivative-layer`'s three are margin posted, default-fund contributions and close-out claims — not
derivatives, which are contracts and ARE marked by their class; you cannot sell your margin balance
at a clearing house. `money-market`'s two are `INTERBANK` and `REPO`, bilateral rows you unwind or
let mature, not money market FUNDS, whose shares are correctly `derived`. Both tags stand.

---

## Item 10b — short-term debt (§9): the first liquidity failure this world can have

**What was missing was a KIND of failure, not a clause count.** Every failure in this world is a
solvency failure: a party fails because what it owes exceeds what it holds. Nobody has ever been
unable to find the money on a Tuesday. §9 B3.b is the other kind — *buyers decline, the issuer must
repay maturing paper out of cash it does not have, and it must find the money somewhere* — and it
was 0 of 19 clauses with no open item owning it. It is now **12 MET, 3 PARTIAL, 4 MISSING**, and the
world has one fewer absent sector (5 → 4).

**The roll is not a mechanism, and that is the whole of B3.a.** A rollover is a NEW ISSUE INTO A
MARKET THAT MUST CLEAR, so there is one issuing phase and what maturing paper does is ENLARGE the
need it brings paper against. E1 names the alternative and forbids it: paper that always rolls at a
written rate is not debt, it is a permanent liability with a coupon, and it removes the only risk the
instrument has. There is no renewal path in the module and the test asserts there is none.

**It issues through the path that exists.** A size and a walk-away into a kernel market, the one
solver striking it — the same doors the treasury and a corporate issuer use. The owner's correction
of this morning is what wrote `10b.0` before the module existed: commercial paper was the next thing
this world issued, and a new module writing its own venue would have put the third issuance mechanism
back the week after 10d deleted it.

**One promise, written once.** A bill and commercial paper promise the identical thing — one payment
of par on one day — and differ in the three things §7 already separates from §8: the issuer can FAIL
into an estate, the claim RANKS, and one miss makes the rest due. So `DiscountSchedule` went into the
kernel beside `CouponSchedule`, and `sovereignBill` now READS it where it had the same three lines
written out inline. Two implementations of one promise agree until the day somebody edits one.

**A2.a made the type system find its own gap.** The day count is a material part of the number at
this tenor, so it is a required field on the shared schedule — and adding it failed the build in
exactly three places, every one of them a bill constructed without saying what convention it was
quoted on. That is the clause doing its job before a single test ran.

**THE DEFECT THIS ITEM FOUND AND FIXED: one hole, two issuers.** `issueBonds` read
`firms.funding.short` — the whole funding gap — and so would paper. A firm would have brought a
five-year bond and three-month paper against the same published number and raised twice what it
needed: money nobody wanted, on a liability somebody owes (Law 5, Appendix B's residual with no
holder). `publishFunding` now splits the gap WHERE THE FIRM ALLOCATES ITS OWN MONEY, which is the one
writer of that decision: cash goes to the near need first, what is left goes to the programme, and
`shortNow + shortTerm === short` exactly. Paper funds the payroll, the bond funds the plant. The
`atMost` in it is not a floor on an outcome — it is what APPLYING money means, since a firm cannot
put more into a need than the need is, nor more than it holds.

**A BANK ISSUES PAPER TOO, and the first draft did not let it.** I had excluded banks because
nothing published a bank's dated shortfall the way `firms.funding` publishes a firm's, and wrote that
up as an absence. The owner's correction — *"Banks also issue commercial paper"* — is right, and the
read was there: `bank.buffer` publishes what a bank keeps back against a bad week, and its reserves
are in the register. The difference between those two reads is the point. **This phase runs after
the money market has sat**, so the published number is what it wanted BEFORE the session and the
register is what it holds AFTER it: a bank that funded itself overnight is short of nothing here and
brings no paper; one that could not is exactly the issuer B1 describes. Nothing has to stand down for
anything, because what this reads is the residue and not a second claim on one gap.

Which type is short of what is a TABLE (`NEEDS`) and not a branch — a firm says one thing, a bank
says another, and adding the state is a row. That is A3: the same instrument, and the type is the
credit.

**The buyer's reservation and the issuer's are the same arithmetic on different alternatives**, which
is why the book has two sides at all (§46 A3). The issuer computes the price at which this costs what
borrowing otherwise costs it; the buyer computes the price at which it returns what a deposit would
pay it. They overlap or they do not, and a book with no overlap says so — no demand is added to clear.
C2.a falls out for free: short paper substitutes for a deposit because the buyer's ALTERNATIVE moved,
with nothing tying a bill yield to a policy rate.

**C3 is why funding goes before solvency does.** A buyer will not add to a name past a share of its
own book, and will not lend at any price to one it has seen default or breach within the memory it
keeps. A buyer that waited for insolvency would be the forced buyer Appendix B forbids.

**The backstop is granted, not seeded.** B4's *"a committed line with no commitment fee on undrawn
headroom is a free option the lender did not sell"* is the load-bearing sentence: without the fee
every issuer would hold an unlimited backstop it never paid for and B3.b's run could never bite. It
is an `Agreement` between two named parties (nobody trades a commitment), the fee leaves the issuer's
account every period on what it has NOT drawn, and the draw happens when maturing paper exceeds what
it holds. Granting it as a standing decision rather than seeding it also means it COMES BACK: an
issuer whose bank failed has no line, and next period has one from wherever it banks now. A seeded
relation could not do that. Its LIMIT is the one number and it is a declared PLACEHOLDER naming item
17.2, because a facility is granted and re-sized by a lender and this world cannot yet take that
decision.

**B3.b's three paths, and only one of them is this module's.** It can DRAW (here). It can SELL —
XI-2's forced seller already exists and needed nothing. Or it can FAIL, which is the kernel's: the
maturity is a payment like any other, it does not happen, `defaultOn` says what that means and
`accelerates` carries it to every other line the issuer has. There is no fourth path.

Two lint rules earned their keep: `phoenix/no-kind-branch` caught two scans that filtered the record
by event kind where typed doors existed, and `phoenix/no-magic-numbers` caught a buyer's memory
window sitting as a bare `256` — which is a PREFERENCE (§46 B1) and is now declared as one.

Findings positioned: **E-17** (paper is not repo collateral; D3 says being collateral is much of why
anyone holds it — item 17.6 builds the same acceptance for senior notes, so once for both) and
**E-18** (`funds/index.ts` cites a `Clearing C1.b` that does not exist; it is PROSE, so `check:spec`
reads `@spec` tags only and cannot see it — item 21, with the question of whether the tool should
read prose too. The same wrong id went into a new `@spec` tag here and the tool DID catch it).

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 210 tags, `check:forbids` 4 over 207 files,
`check:deaths` 6 of 6, `check:existence` green. Tests written, not run (standing instruction).

---

## Item 10c — securitisation is for any non-tradable claim, and demand can pull a deal

Two defects, one cause between them: this module knew what it could sell as a LIST rather than as a
property, and it knew only one reason to sell.

**The kind branch, and where the same branch was hiding a level down.** `saleable` gated on
`isLoan` — which Law 15 forbids in a mechanism outright, and which is also a narrower world than the
one it models. What replaces it is not a longer list but TWO FACTS EVERY KIND ALREADY DECLARES:
`pricing === 'carriedAtCost'` (no market exists for it, so the bank cannot simply sell it) and
`liabilityOfIssuer` (a named party owes it, so there is a stream of payments to tranche). Together
those ARE the definition of securitisable and it is not a coincidence — it is what a securitisation
is FOR: if it had a market the bank would sell it and need no vehicle, and if nobody owed it there
would be nothing to cut into layers. Inventory drops out on its own, because *"a tonne is nobody's
promise"*.

Neither fact is one this module invents or maintains, which is the part that matters: a kind that
GAINS a market stops being securitisable the same day, with nothing here to edit. Item 10d does
exactly that to a bank's subordinated debt.

**The same branch was in the audit family** — `isTranche(i.terms) || isLoan(i.terms)` — where it
would have reported every invoice in a vehicle as an asset naming no borrower, the day a bank pooled
one. A false violation in the family that exists to catch anonymous exposure is worse than none. It
now reads the obligor off the instrument itself: its kind says somebody owes it, and it says who.
`isLoan` is no longer imported anywhere in the module.

**The reason that did not exist.** `arrange` began `if (gap <= 0) continue` — so no deal in this
world could ever happen because somebody WANTED the paper, only because somebody had to shed it.
The two reasons are now both there and they are deliberately NOT merged, because they behave
differently in the one way that matters:

- **NEED** — below its capital line, it must shrink, and it posts a size and NO LEVEL (Clearing C3),
  taking what the book gives it at a loss if a loss is what is there. A bank that refused a bad
  price would not be shrinking.
- **DEMAND** — not short, so a worse price is simply a deal it does not do. It posts what it is
  CARRYING the pool at and sells above it. That level is not a floor on an outcome (Law 6): keeping
  the rows and being paid on them is the alternative it already has, which is the same construction
  an issuer's walk-away is.

One mechanism, two entry conditions, and `securitisation.cut` now says which brought each deal —
because a reader who could not tell them apart would read a healthy market as a wave of distress.

**A test was narrowed rather than deleted, and that was the point of having written it.** The
existing finding-as-a-test asserted that no deal is ever cut here, because every bank that runs out
of room runs out on the LEVERAGE backstop and securitisation relieves the weighted rule. That
finding still holds — but the assertion was true only because NEED was the sole way a deal could
start. It now asserts no deal is cut OUT OF NEED, leaving a `demand` cut free to happen, and says in
its own comment why the distinction is the live one.

**A stale COVERAGE note, corrected by the owner's rule.** `Banks Lending D4` was MISSING, with a
note saying a buyer for a loan row and *"the syndicate of D4.a"* arrive with corporate credit. Both
halves were wrong. A loan row IS sold and has a buyer and a price — that is this module. And D4.a's
syndicated loan is *"one loan with several lenders of record, each a row per (lender, borrower)…
struck at one margin by a lead"*, which is the spec saying in its own words what the owner said
yesterday: a syndicated loan is a group of BANKS, not a distribution to investors, and it has
nothing to do with C10's underwriting syndicate. Its row shape already exists — C9 keeps one row per
lender per borrower, so a club IS N rows — and what it needs is the lead and the fee, which is item
**17.2** with the facility, not 17.1. D4 is now PARTIAL with exactly that named.

`E-16` is closed and its index row is gone.

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 210 tags, `check:forbids` 4 over 207 files,
`check:deaths` 6 of 6, `check:existence` green. Tests written, not run.

---

## The demand side: a defect of mine, and an item that moved

**The observation** (owner, 2026-09-14): *"Until we have actual asset managers with appetite nothing
will get demand."* It is right, and checking it turned up a defect I had introduced the same day.

**Every fund in this world may hold bills or grain.** `funds/data.ts` declares
`eligible: ['sovereign.bill']` for every money fund and `['good.grain']` for the one commodity fund;
the trackers hold an index. So items 10, 10b and 10c built a corporate bond, commercial paper and a
securitisation that can pool any claim — into a world whose only buyers are bank dealing desks and
bank liquidity books.

**And in 10b I had papered over exactly that.** I declared a `FUND` participant in
`short-term-debt` so that money funds would bid for commercial paper. That bids for a fund WITHOUT
ASKING ITS MANDATE — nor its currency, nor the tenor its investors agreed to, nor the yield it
requires, all four of which `funds/index.ts:eligible` asks and is the one gate for. A money fund
whose mandate says bills-only would have bought commercial paper, which is the mandate not binding
at all (Law 4: one writer of what a fund may hold). The participant is deleted.

**What replaces it is the mandate itself**, which is where a fund's appetite lives: a money fund's
`eligible` now names `commercial.paper` beside the bill. That is not a workaround for the missing
appetite — it is what a money fund IS. D1's saver holds it instead of a deposit, and §9 C1 names the
money fund first among cash investors precisely because its appetite is why commercial paper is a
market at all.

**It also connects B3.b's run to somebody.** A fund holding a firm's paper that meets a redemption it
cannot cover out of its buffer sells into that market at whatever it gives (XI-2 door 2) — so an
issuer that cannot roll and a saver who wants their money back are joined. Bills alone cannot carry
that transmission, because the issuer of a bill does not fail to roll.

**Item 13 moves ahead of 11 and 12**, keeping its number so nothing that references it breaks. The
dependency argument is the owner's observation stated as an ordering: three items have now built
things to sell and none has built anybody to buy them, and 11 and 12 add more issuers still. A
sector of supply built in front of the demand item is a sector whose books say `noDemand` — an
honest outcome and a wasted one, because nothing about a mechanism is tested by a book nobody comes
to. Item 13 is unblocked (its own blocker, `Mandate`, closed at 9.2a) and nothing in 11 or 12 needs
it.

Typecheck 0 (engine, app, tools), lint 0.

---

## Item 10d — a bank issues a bond the way everybody else does

**A deletion, and the thing it deletes is a second copy of the kernel.** The question that started
it was why a bank's subordinated debt has no price. The answer was not *"it needs a secondary
market"*: `banks/subordinated.ts` was a private copy of the issuance machinery, and the missing price
was one of four things the copy had drifted into.

```
DELETED                          REPLACED BY (Law 19: every deletion names the read)
raiseVenue / openVenue / post    ctx.issue + ctx.openMarket + ctx.offer
clear() / isCleared              the kernel's one solver, uniformPrice
asRatio(outcome.price)           a bond book clearing a price per unit of par  (E-11: four left)
subId(bank, n) per FILL          subId(bank, maturity) — one line per bank per date  (Law 9)
writeSub's hand-rolled legs      the primary market's own settlement
bidsFor, gathered by hand        `subscribes`, a participant the kernel asks for  (Clearing B2)
'bank.raise' / 'bank.raise.failed'   'auction.result', which the market writes for every issuer
```

**The worst of the four was the one I had not seen when I wrote the item up.** `subId(bank, n)`
advanced per FILL, so a bank that raised from three lenders ended up holding **three instruments
carrying one promise**. That is Law 9 exactly backwards — there is no market anywhere that would
call them different bonds — and Law 4 with it.

**`bank.raise` is deleted rather than reimplemented**, which is the Law 4 half of this. The kernel's
primary market already publishes `auction.result` for every issuer alike — size, allotted, withdrawn,
cover, stop-out, tail — so a module writing its own answer to *"what did the raise achieve"* was a
second writer of a fact the kernel owns. Its readers in two test files now read the auction.

**The walk-away is nothing, and that is Clearing C3 rather than a hole.** *"A size and no level. It
is short of capital, not shopping."* A bank raising capital has no alternative to hold out for —
raising equity and shrinking are what it does INSTEAD of this, not a price — so it accepts whatever
the book strikes. Nothing was invented to stand in for a reservation it does not have, and C2.b stays
reachable: a book with no bids allots nothing and the bank is exactly where it was. The size is face
at par, so a bank whose paper the market will only take below par raises less than it needed and is
still short — the honest outcome and the one C2.b is about.

**And then the price, which was the question.** `pricing: 'carriedAtCost'` with its comment *"nothing
trades these here"* is gone; it is `cleared`, marked, on the same face tick as every other piece of
paper. It is not cosmetic: subordinated debt is the instrument whose price moves FIRST when a bank's
solvency is doubted, before its equity and long before a depositor notices, which is what makes D2's
bail-in legible — a write-down landing on a layer whose value everybody could already watch falling.

**10c's rule did its own job with nothing to edit.** `saleable` admits a claim that is
`carriedAtCost` and `liabilityOfIssuer`; a bank's subordinated debt was in that set this morning and
is out of it now, because it has a market. You do not securitise a bond you can sell. That is the
whole reason 10c reads declared facts instead of keeping a list.

`E-15` is closed. `E-11` is down to four rate-quoted books.

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 210 tags, `check:forbids` 4 over 207 files,
`check:deaths` 6 of 6, `check:existence` green. Tests written and updated, not run.

---

## Item 10e, first stage — the investment universe and the blueprint language

**Two kernel reads, no table anywhere, and a correction I had got badly wrong.**

**THE RATING AGENCIES EXIST AND I SAID THEY DID NOT.** The proposal I wrote an hour earlier argued
that a quality band could not be built because §44's ratings were unbuilt, and cited
`banks/quote.ts`'s own comment about a doom loop to justify it. The owner asked *"There are no rating
agencies here?"* and the answer is that there are: `mechanisms/ratings/` has an `ASSESSOR` party
kind, several of them drawn and deliberately unalike — *"two assessors with the same thresholds and
the same patience are one assessor with two names"* — each publishing `rating.action` on the
kernel's ordered scale. A4.b's requirement that assessments DISAGREE was satisfied before I claimed
it could not be. What `quote.ts` records is that a BANK's loan model is the wrong input for a
marked-to-market issuer, which is a different question from whether this world has published grades.

**THE RULE IS THE LOWEST OF THE RATINGS AVAILABLE** (owner), and it is the conservative convention a
mandate is written to. It is a SELECTION over real opinions and never a blend, so Appendix B's "no
decision at an average" holds: every candidate is one named assessor's view it can be wrong about,
and what comes out is one of them. And it makes a downgrade TRANSMIT — one assessor moving is enough
to put a name below a mandate's line, so the name has to be sold, which is the channel §44 exists
for.

**⚠ It collides with a rule already in the kernel and I did not settle it.** `grades.ts:middleGrade`
takes the MIDDLE opinion and argues for it in its own words: *"a downgrade contestable rather than
arithmetic — one assessor moving changes nothing"*. Two readers use it (CDS A5.a's series division,
Indices A1.a). `lowestGrade` is built beside it for the MANDATE boundary and the collision is
recorded as **10e.7** for the owner, because the two buy opposite things deliberately and which
question each answers is worth deciding on purpose rather than by whichever I wrote last.

**`registry/universe.ts`: what an asset IS, asked of the asset.** Three structural reads give the
classes with no table: did somebody promise it (`liabilityOfIssuer`), does the promise end (does
`cashFlows` end), and whose promise is it (the issuer's party kind). Then currency, duration,
standing, secured, listed and grade. **Every one is a read and nothing is stored**, which is what
makes two of them right rather than merely tidy:

- a five-year bond BECOMES a three-year bond, because duration is measured from today;
- a company falls out of a size band BY FALLING, because size is what the market says it is worth.

A stored tenor or a stored cap does neither, and the mandate would stop biting the moment it mattered.

**`registry/blueprint.ts`: bands over those reads, and one `admits` for every vehicle in the world.**
A band not stated is silence, which is what lets one language describe a money fund (government and
corporate, under a year, own money, `worstGrade: 'a'`) and a macro strategy (nothing stated, holds
what it likes). And a band the asset CANNOT ANSWER is a refusal, not a pass: a fund that banded on
duration holds dated claims, a share has no duration, and admitting it by silence would be the `?? 0`
this codebase does not do.

**The owner's own test is the test file.** `universe.test.ts` takes the assets this world issues —
commercial paper, a securitisation note, a bill, a listed company, a private one — and asks whether
the schedule describes them. It does, and the one that proves the design is `structured`: a
vehicle's note is its own class not because anybody labelled it, but because the party that promised
it is a `vehicle`, whose assets are the pool and nothing else (XI-11).

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 212 tags, `check:forbids` 4 over 209 files,
`check:deaths` 6 of 6. Tests written, not run.

---

## Item 10e, second stage — the mandate speaks the language, and three hand-written constraints die

**`MandateTerms.mayHold: readonly string[]` is gone.** A mandate now carries a `blueprint` and
`liquidity` terms, and `funds/index.ts:eligible` — the one gate on what any pool may hold — is a
single call to `admits` over the kernel's classification. What that deleted is the point:

```
DELETED from `eligible`                    WHY IT WAS WRONG
the kind list                              could not say "credit, 3-7 years, senior" at all
the currency test (hand-written)           a multi-currency mandate was INEXPRESSIBLE (A-47, A-50)
the tenor test (through `cashFlows`)       failed anything promising no dated payment
```

**The tenor test was the interesting one.** It took the last cash flow and compared it to the fund's
maximum, so ANYTHING THAT PROMISES NO DATED PAYMENT failed by having no last flow at all. A share
promises none. That one line is why no fund in this world could ever hold a share, whatever its
mandate said (`docs/RECORD.md` item 4 found it and patched around it). A duration BAND asks the
question where a duration exists and says nothing where it does not, and a blueprint that wants
shares simply does not state one — so the defect is gone rather than guarded.

**`classify` is a KERNEL DOOR on all three views** (participant, mechanism, seed), because a mandate,
a manager and an audit family all ask what an asset is and must not get three answers (Law 4). The
reads it is built from are the kernel's stores anyway, so there was nowhere else it could honestly
live.

**A TRACKER'S MANDATE AND ITS BASKET WERE CONFLATED, and separating them is a real fix.** The ETF
took the kinds its basket happened to name and called that its mandate. But an index fund's mandate
is the ASSET CLASS its investors bought and the INDEX says which lines in what weights (E3) — so the
index can change its constituents without anybody rewriting the fund's mandate, which is what an
index doing its job looks like. Its blueprint is now the classes the basket is made of, read off the
classification; its liquidity is `listed`, which is G1.a's reason it is not a forced seller.

**The commodity fund got smaller.** `holdsPhysical` walked the mandate's kind ids asking the registry
whether each was `physical`. It is now `classes: ['thing']` — the class the classification gives
anything NOBODY PROMISED — so a good this world invents next year is inside the mandate without
`physical.ts` or its test knowing the name of it.

**And the money fund's mandate now says what the clause says.** `['sovereign.bill',
'commercial.paper']` meant "the two short things I know the name of" and would have gone stale at the
third. D1 says *short, high-quality paper*, and that is what the band says: a dated promise, under a
year, from a name the assessors are content with. A bill and a piece of commercial paper both answer
it without `data.ts` knowing either exists.

**Not yet done, and named so it is not lost: 10e.3b.** The liquidity terms are DECLARED but the
subscription and redemption path does not read them yet — every fund still redeems as though it were
liquid. A queue for semi-liquid, a refusal for closed and in-kind for listed is what makes those
terms decide who can be forced to sell, which is the whole of their point (XI-2).

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 212 tags, `check:forbids` 4 over 209 files,
`check:deaths` 6 of 6. Tests written and updated, not run.

---

## Item 10e, third stage — nothing explains what an ETF is

**The owner's instruction, after catching me adding a term to the mandate while leaving the type
system that made it necessary:** *"there should be nothing explaining what an ETF is aside from the
blueprint. Names should be derived from the fund characteristics. Remove each reference or system
that separates what different funds are."* And: *"fees should be in the same block. You should be
doing this analysis, not me."* Both are fair, and the second is the sharper of the two — I had been
making the owner find these one at a time instead of reading the sector and proposing the whole
shape.

**The analysis I should have done first.** Every place that separated what a fund is:

| where | what separated |
|---|---|
| `data.ts` | two declaration types, two draws, `MONEY_FUND_SPONSOR_SIZE`/`CREDIT_FUND_SPONSOR_SIZE`, `ETF_*` |
| `data.ts` | **the fee in two places**: every fund drew `between(rng, FUND_SPREAD.fee)`, the tracker had a literal `0.001` |
| `data.ts` | names TYPED — "Money Fund", "Credit Fund", "{index} tracker" |
| `index.ts` | `funds(decls, etfs)`, `paramsOf` + `etfParamsOf`, a separate `funds.etf` phase, ten ETF-only functions |
| `etf.ts`, `tracker.ts`, `physical.ts` | keyed off the DECLARATION TYPE, when each is really a TERM: `liquidity: 'listed'`, `tracks`, `classes: ['thing']` |

**`EtfDecl` is gone.** One declaration, in three blocks that say what a vehicle is: its identity, its
MANDATE (blueprint, currency, liquidity, tracks) and its ECONOMICS (buffer, fee, requiredYield). The
launch data an in-kind vehicle needs is `inKind`, present when its investors come and go that way —
a consequence of the liquidity term, never a kind of thing. `drawEtfs` is `drawTrackers` and returns
`FundDecl`.

**The fee was the sharpest of the owner's points and I had not seen it.** A tracker's fee was a
literal while every other vehicle's was drawn from a spread: two writers of one fact (Law 4), and
the consequence was that **a tracker's fee could not DISPERSE**. It could never be dearer than a
rival, never lose money to one, and never be wound up for costing more than it earned — so half of
what a manager's launch decision turns on was missing for one kind of vehicle, because its fee lived
somewhere else. One block, one spread, every vehicle.

**Names are DERIVED (`nameOf`).** They were typed, which meant a vehicle could be called one thing
and hold another, and that the name was a THIRD place a fund's type was written down after its
declaration and its behaviour. It reads the mandate and nothing else, so a fund whose blueprint
changes is renamed by the same change.

**Tracking is a mandate characteristic** (owner). `EtfDecl.tracks` already existed with exactly the
right reasoning — *"a tracker's mandate is not a list of lines somebody typed, it is a rule, and the
rule is the index's"* — but it sat on the row for ONE kind of vehicle, so an index mutual fund
(passive, not listed) or a passive segregated mandate could not say it. It is on `MandateTerms` now,
beside the blueprint: the blueprint says what it MAY hold, `tracks` says whether it chooses within
that or holds what an index says. Absent is active.

**And the phase is keyed off the term.** `const etfs = decls.filter(d => d.liquidity.how ===
'listed')` — the fact that decides it, not the launch data beside it.

**The liquidity terms now decide redemption** (10e.3b): a CLOSED vehicle refuses the request
outright and records the refusal, so it can never be a forced seller, which is the entire reason the
structure exists; a SEMI-LIQUID one pays only when its window is open, and what the queue costs the
holders who stayed is C4.a; a LISTED one has no cash redemption at all (G1.a).

**Still separated, and named so it is not lost:** `etf.ts` (319 lines), `tracker.ts` (152) and
`physical.ts` (93) still take a declaration and are still three files named after three kinds of
fund. Their CONTENT is right — create/redeem in kind, rebalance to an index, bid for a thing — and
each is one term's behaviour. Folding them into one orders path that reads the mandate is the rest
of this step.

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 212 tags, `check:forbids` 4, `check:deaths`
5 of 5, `check:existence` green. Tests written and updated, not run.

---

## Item 10e, fourth stage — three files named after three kinds of fund, folded into terms

**The dispatch was already conceptually right and implemented as three separate answers.** The
participant said it in its own comment — *"three mandates, three reasons, and a fund has exactly one
of them"* — and then called all three, letting each re-find the fund and decide it was not theirs.
Three functions each carrying a copy of "which kind of fund am I for" is the separation this item
exists to delete. It is one read of the mandate now, and the reason follows from its TERMS: it
tracks, or its blueprint is things, or it chooses.

**A bug the fold surfaced.** The dispatch I first wrote asked
`blueprint.classes.every((c) => c === 'thing')` — and `[].every(...)` is `true`, so a macro mandate
that states NO class band, which holds anything, would have been routed to the commodity path. The
fix is Law 4 rather than a guard: `holdsThings` is the one writer of that test and it is the one
that says the empty case out loud.

**Three files renamed for what they do, because a file named after a kind of fund IS a system that
separates them:**

```
tracker.ts  ->  passive.ts   what a mandate that DOES NOT CHOOSE does
physical.ts ->  things.ts    what a mandate over THINGS does
etf.ts      ->  inkind.ts    what `liquidity: 'listed'` means
```

Each header used to open *"A fund that tracks an index"*, *"A FUND THAT HOLDS THE THING ITSELF"*,
*"The exchange-traded fund"*. They now name the term, and say which term puts a pool there — so an
index mutual fund, an exchange-traded one and a passive segregated mandate all arrive at `passive.ts`
by the same door.

**And the machinery underneath: `runEtf` → `runInKind`, `seedEtf` → `seedInKind`, `readEtf` →
`readListed`, `etfVenue` → `inKindVenue`, `etfMarketOf` → `listedMarketOf`, `launchTracker` →
`launchInKind`, the `funds.etf` phase → `funds.inKind`, and the events `etf.launched`, `etf.struck`,
`etf.created`, `etf.redeemed` → `fund.*`.** The venue key went from `kind: 'etf'` to `kind: 'inKind'`,
which is the door stating a FACT ABOUT ITSELF rather than a label — the same discipline the
`minWealth` key follows.

**THE RENAME CAUGHT A BREAK I HAD MADE AND NOT NOTICED.** `banks/dealing.ts` matches the venue key to
find creation gaps to arbitrage, and it still read `v.key['kind'] !== 'etf'` after I renamed the key
— so **the dealer's ETF arbitrage had gone silently dead**. A sweep for the word found it; nothing
else would have, because a desk that finds no gaps posts no orders and that is indistinguishable
from a desk with nothing to do. It is the exact failure `world/reach.ts` exists to catch, and the
lesson is that renaming a key is a two-sided change like everything else here.

What is left of the word in the engine is three occurrences, and all three are the NAME OF A PARTY
this world draws (`etf.us`, `manager.etf.us`) — a name, not a type, and Law 9 says an internal id is
never a display name. The display name is derived from the mandate now.

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 212 tags, `check:forbids` 4 over 209 files,
`check:deaths` 5 of 5, `check:existence` green. Tests written and updated, not run.

---

## Item 10e, fifth stage — the manager is a business, and the roster of funds is an outcome

**The largest declared equilibrium left in this sector.** `drawFunds` returned a roster and every
phase in the module walked it, which said that the set of funds in this world is fixed for ever and
was decided before anybody in it had done anything (Law 2). A manager had ONE pool, no cost of any
kind, and a fee that was a PLACEHOLDER whose own reason named the missing mechanism: *"no manager
competes for the mandate, so the number stands where a competition should be… the missing mechanism
is a manager with a cost base"*. This is that mechanism, and the placeholder is dead —
`check:deaths` counts four scheduled deaths where it counted five.

**THE RUN-TIME OBJECT IS THE MANDATE, NOT THE DECLARATION.** Every phase asks `livingPools(ctx)` —
the performing mandates — so the pools that exist are the pools somebody is running, and the draw is
an OPENING CONDITION (Seed A3), the way the bank draw is the balance sheets this world opens with.
Nothing reads the declaration after the seed has written the mandates. That one change is what made
a launch possible at all: a pool that did not exist at assembly could never have been struck, placed
its cash, or paid anybody.

**And so the mandate carries the whole product.** `feePerAnnum`, `buffer` and
`requiredYieldPerAnnum` moved from the parameter register onto `MandateTerms`, which is where they
belong: they are what two named parties agreed, the way a loan's rate is on the loan. Three per-fund
parameters are gone, `fundParam` is gone, and `paramsOf` no longer takes the roster. A parameter
could not have been any of them anyway — parameters are declared at assembly, so a fund launched in
period forty could never have had one.

**What a manager now does, all three with real numbers on both sides:**

- **It employs.** Running a pool takes people, and `analysis` — *"research and credit analysis"*,
  declared at 13d for a bank's credit desk — was a trade with a venue in every region and **nobody
  on the bid side**. A manager posts openings there at what an hour is worth to it (its fee income
  over the hours its pools take) and is matched by the same rule as a bank or a baker. A bank's
  analysts and a manager's now compete for the same people, which links the two halves of this
  world's finance industry through one trade.
- **It winds a pool up** when the fee that pool pays stops covering what running it costs. Notice is
  a STATE of the mandate and not its end — a pool with no mandate has nobody deciding for it, and
  its holders and its book would both still be there. From there it is the machinery that was
  already here: every holder goes on the redemption queue at the struck NAV, the pool sells what it
  must at whatever the market gives (C2.a, XI-2), and when the last share is back and the book is
  empty the mandate ends and the pool ceases to its manager. **A wind-down is a forced sale of a
  whole book**, which is a channel this sector did not have.
- **It launches one** by copying a product it can SEE working: a blueprint somebody else runs, at the
  smallest book any of them has actually gathered, at a fee under the cheapest of them. If that
  covers what a pool costs it, it opens one. It cannot invent a product nobody runs — nothing would
  tell it what such a thing would gather, and a manager that guessed would be a forecast with no
  falsification (Law 17).

**Fees fall where several managers run the same blueprint, and NOTHING BOUNDS THE FALL.** Each
entrant comes in under the cheapest incumbent by its own drawn `undercut`; what stops entry is that
the next entrant's fee would no longer cover what a pool costs it in people, so it does not open
one. The refusal is the mechanism (Law 6). Two more preferences are drawn per house — `undercut` and
`patience`, how long it gives a product before judging it — because two managers with the same
preferences are one manager with two names (the argument the ratings module already makes about
assessors).

**One house per bank, running everything it sponsors.** Every pool used to get a manager of its own,
and the comment that did it argued *"two funds at one bank are not one business: a shared name would
be two funds' fees arriving in one account nobody could take apart (Law 4)"*. **That was wrong.** Two
funds' fees arriving in one account is what an asset manager IS; what takes them apart is the
`fund.fee` event, which names the pool and the manager on every payment. What the old shape cost was
everything this step is about: a manager with one pool has no book of business, cannot spread the
cost of its people, cannot lose one product and keep another, and cannot be bigger than a rival.

**Three defects, found by reading, all closed here:**

- **`E-20`, and it stops the build.** `offeredYield` read `fundParam(fund, 'maxTenorPeriods')` — the
  tenor parameter the blueprint language replaced at 10e.2 and which nothing has declared since. A
  read of an undeclared parameter throws, and this one is in the strike of every fund in the world.
  Fixed where it stands, as the rules require: the longest thing a mandate lets a pool hold is its
  duration BAND, in years, which is the number a curve is asked at anyway — so the tenor in periods,
  the date it came to, and the year fraction back out of that date all went with it.
- **`E-19`: the wage read was written twice and the copies disagreed about the key.** The firm's
  keyed the going rate by the venue id, which is what `publishGoingRate` writes; the bank's keyed it
  by `region|occupation`, which has never existed — so a bank with no payroll of its own could never
  fall back to the published rate and put NO staff cost in any quote it made. A manager costing a
  pool would have been the third copy. It is `registry/wages.ts` now, beside `storageRateIn`, and
  both copies are deleted (Law 12: the fix removes code).
- **`E-21`: a saver committed one budget to every fund it could reach.** `fundOrders` posted a buy
  for the whole of a cell's spare cash at every fund whose offer cleared what it required. Nothing
  was created — the wire refused the rest for want of money — but which fund got a saver's money was
  decided by the order the venue list happened to be in. It would also have made this whole step
  pointless: undercutting a rival wins nothing from a saver that subscribes to everything regardless,
  and a fee nobody can lose business over is not a price.

**Two more things this deleted.** `liquidityOf` walked the agreement store to find a term the caller
already had in its hand, and it is gone; and the fee arithmetic now has one writer (`feeOn`), asked
by the pool with what its own register says and by a manager with what a rival PUBLISHED — two
sources, one formula, which is the distinction Law 4 actually cares about.

**New: `funds.everyPoolIsRun`** in the `names` family. *"There is no fund without a manager"* used to
be true by construction and stopped being so the moment pools could be opened and closed mid-run: a
launch that entered the party and failed to write the mandate would leave a pool with an account,
holdings and nobody deciding for it, and nothing would throw. A FORBID that holds is as valuable as
a mechanism that works, and it breaks silently.

**Also extracted: `funds/mandate.ts`.** The mandate was inside `index.ts`, which was fine while the
module's only reader was its own phases. The manager reads it too, and a file both `index.ts` and
`manager.ts` import values from cannot be either of them.

Typecheck 0 (engine, app, tools), lint 0, `check:spec` 215 tags, `check:forbids` 4 over 212 files,
`check:deaths` 4 of 4, `check:existence` green. Tests written and updated, not run.

---

## Item 10e.5 — seed money, and the only way a pool reaches a balance sheet

*"Nothing sits on the balance sheet unless the entity decides to put in their own seed money"* (the
owner). A manager now puts its own money into a pool it opens, and it is the whole of what it can
lose from that pool: the holders own the assets and bear their losses (A3), so a house's exposure is
the seed it chose plus the fee income it stops earning — which is why *"an HF doesn't go bankrupt
because a fund does bad by itself"*.

**HOW MUCH is the launch decision read backwards.** The book at which this pool's own fee covers what
a pool costs it — below that the manager is running the product at a loss until savers arrive, and
above it there is no reason to tie up more of its own money. And **never its payroll**: what it can
put in is what it holds over what its people cost it for the pools it runs plus the one it is
opening. That is not a bound (Law 6) — a business does not spend money its staff are owed, and one
that did would meet its wage bill with a refusal at the wire next period.

**It goes in through the front door.** The manager POSTS into the pool's own venue and the strike
settles it at the NAV like any other subscription (C1) — same instruction, same convention, same
refusal as a household's. Nothing is endowed and nothing is written into the register (Appendix B:
no seeded outcome), which is why a pool a manager opened and a pool this world opened with are the
same object. It comes back the same way: a wind-down puts every holder on the redemption queue and
the manager is one of them, at whatever the sales realised.

**What is NOT built, and it is placed as 10e.5b rather than left implicit.** The step said *"raised
by issuing its own equity or debt"*, and a house here seeds out of the fee income it has. The reason
is worth recording because half of it already exists: `publishQuotes` walks every party whose KIND
borrows and `fundManagerKind.borrows` is `true`, so **a credit quote is already published under a
manager's own name every period**, priced off its own risk by whichever bank is keenest. What stops
it borrowing is one step further on — `runRequests` reads `firms.funding` and `housing.funding`, two
named event kinds where there should be one, which is a kind branch wearing a list's clothes and
which no third borrower can join without making it three. The fix (one published kind meaning *"a
named party said what it is short of"*) also has to be made by `corporate-bond` and
`short-term-debt`, which read `firms.funding` to decide whether a firm should come to market
instead. That is a change to the bank's request channel and two issuance modules, and the seed does
not need it.

Typecheck 0, lint 0, `check:spec` 215 tags, `check:forbids` 4 over 212 files, `check:deaths` 4 of 4,
`check:existence` green. Tests written and updated, not run.

---

## Item 10e.6 — access is a POLICY, asked at a door and answered by the entrant

The owner's ladder — *"retail able to access ETF and MMF, rich retail able to access funds,
institutional being also able to do mandates"* — is **not three kinds of vehicle and not three kinds
of investor**. It is one question a vehicle asks at its door and one answer an entrant gives, and
what separates the rungs is a number a regulator sets.

**The vehicle states whether it is offered to the public** (`MandateTerms.offeredPublicly`), which is
a real term of a real product — a public offering against a private placement — and a boolean rather
than a tier, because a tier would be a taxonomy of vehicles and this item exists to delete those.
A money fund and a listed tracker are offered publicly; a fund is not.

**The line is a POLICY owned by `parliament`** (`funds.accreditedWealthPerMember`), which is the
first real channel item 19 has into this sector. A round million, which is what the actual
accredited-investor and professional-client tests are built on — and deliberately NOT a quantile of
this world's own wealth, which would be an outcome wearing a rule's clothes, moving with the thing
it is meant to sort.

**The door carries the NAME of the policy and not the number.** A venue key is public data about
itself, so a saver reads what a vehicle asks without knowing how this module names anything (Law 15,
the same discipline `kind: 'inKind'` follows) — and a key with the figure baked into it would state
last year's rule for ever. A publicly offered vehicle carries no such key, and its absence is the
answer.

**The entrant ANSWERS, because the fund cannot look.** No party sees another's register (Observer
A4), so a cell that wants into a restricted vehicle CERTIFIES: it publishes what it is worth per
member, and the fund checks that against the line as the line stands that day. A cell with no
interest in such a vehicle publishes nothing — the disclosure is the price of access, which is what
certification is, rather than a surveillance of every saver in the world. A stale certification is
as good as none, because the line can move under somebody who qualified last year.

**A pool's own manager is not an entrant to it** — a house putting seed money into a product it is
opening is the sponsor, not somebody being sold it. That is a RELATIONSHIP (the party the mandate
names) and not a kind, so nothing here asks what sort of party anybody is.

**The SPLIT the plan expected is not there, and does not need to be.** The design said a cell
straddling the threshold splits per XI-15. Every member of a cell holds the same thing — per-member
state is what a cell IS — so a cell is never half over a line, and a threshold read is a clean
answer for the whole of it. Worth saying out loud rather than leaving as a mechanism nobody built.

**Two findings this made visible, both recorded rather than tuned away (Law 11):**

- **`E-22`**: whether any household cell in THIS world ever clears a million dollars per member is a
  question about this world's price level and its wealth distribution, and the answer is a
  measurement (item 23). If nothing clears it, the credit fund and the commodity fund gather nothing
  and the demand side arrives only with item 14's institutions. If it is the LINE that is wrong
  rather than the world, it is item 19's to move — which is exactly why the number is a policy with
  an owner and not a constant.
- **`E-23`**: retail's route into credit is a credit ETF, and this world has none. Every tracker it
  draws follows an equity index, because §26's indices are equity indices. The ladder made the
  absence visible for the first time; a credit index and a tracker on it is item 17's, and the same
  argument holds for commodities at 18.

Typecheck 0, lint 0, `check:spec` 215 tags, `check:forbids` 4 over 212 files, `check:deaths` 4 of 4,
`check:existence` green. Tests written and updated, not run.

---

## Item 10e CLOSED — asset management: one object, a language, and a business

Five stages and five owner corrections. What the sector was when it opened: a fixed roster of funds
drawn before period zero, each with a manager of its own that had one pool and no cost of any kind,
a mandate that was a LIST OF INSTRUMENT KIND IDS, three files named after three kinds of fund, and a
fee that was a placeholder naming its own missing mechanism. What it is now:

- **ONE OBJECT.** A fund is a pool with a manager. §28's hedge fund, §29's vehicle, an ETF, a money
  fund and a segregated mandate are one thing with its terms set differently — blueprint, liquidity,
  access, permissions — and there is no ETF declaration, no fund-type flag and no file named after a
  kind of vehicle anywhere in the module.
- **A LANGUAGE.** `registry/universe.ts` classifies any instrument from three structural reads (did
  somebody promise it, is it dated, whose promise) and answers every dimension as a READ: duration
  from today, so a five-year bond becomes a three-year bond; size from shares times the last print,
  so a company falls out of large-cap by falling; quality as the LOWEST grade any assessor
  published. `registry/blueprint.ts` bands over those reads, and one `admits` answers for every
  vehicle in the world.
- **TERMS THAT DECIDE WHO CAN BE FORCED TO SELL.** Liquid, semi-liquid with a window, closed, listed
  — one subscription and redemption path, and what differs is what the terms say. A shock reaches a
  closed fund and stops; it reaches a liquid one and becomes a sale into whatever the market gives.
- **A BUSINESS.** A manager runs several pools, employs people out of the labour market in a trade
  that had a venue in every region and nobody bidding in it, winds a pool up when its fee stops
  covering what running it costs, and launches one by copying a product it can see working at a fee
  under the cheapest incumbent. The roster of funds is an OUTCOME and the draw is an opening
  condition.
- **SEED MONEY,** which is the only way a pool's assets reach a manager's balance sheet, and
  therefore the whole of what a house can lose from a pool going wrong.
- **ACCESS AS A POLICY,** asked at a door and answered by the entrant, with `parliament` owning the
  line — the first real channel item 19 has into this sector.

**What it deleted.** `EtfDecl`; `MandateTerms.mayHold`; `fundParam` and three per-fund parameters
including the fee PLACEHOLDER; `liquidityOf`; the three fund-type-named files; two copies of the
wage read; a dead venue-key match in the dealer's arbitrage; the hand-written currency, tenor and
kind tests in `eligible`; and one manager per pool. `check:deaths` counts four scheduled deaths where
it counted five.

**What it found, by reading rather than running.** `E-19` (the wage read written twice with two
different keys, so a bank with no payroll put no staff cost in any quote), `E-20` (a read of a
parameter 10e.2 had deleted, in the strike of every fund — a build-stopper), `E-21` (a saver
committing one budget to every fund it could reach, which would have made the whole fee mechanism
pointless). All three closed here. `E-22` and `E-23` are recorded and placed rather than tuned away.

**Two steps left it by being PLACED, which is the only way a step leaves this file.** **17.9**: one
funding-request channel, so a manager can borrow to seed a launch — the lender side is already
general (`publishQuotes` walks every kind that borrows and a manager's does), and what stops it is
that `runRequests` reads two named event kinds where there should be one; it lands with corporate
credit because `corporate-bond` and `short-term-debt` read the same event. **23.0a**: which
blueprints this world grew and which it wound down, which is a measurement and belongs with the
measurement pass.

**And it SHRANK item 13**, which was the point of taking it first: §28 is a manager whose blueprints
permit leverage and shorting, §29 is a manager whose liquidity terms are closed-end over unlisted
equity, and what is left to build there is prime brokerage — the lender those permissions need.

§13 is 23 MET, 3 PARTIAL, 0 MISSING over 26 clauses. Plan completion 92.9%; requirement coverage
61.9% (847 MET of 1369).

Typecheck 0, lint 0, `check:spec` 215 tags, `check:forbids` 4 over 212 files, `check:deaths` 4 of 4,
`check:existence` green. **Tests written and updated, not run** — the suite is the owner's to call
for, and item 23.0 is where it is called.

---

## Item 13.6 — a fund's book does not stop at the register's edge

**A pass opened before the thing that needs it**, which is the order the plan set and the reason it
set it: item 13.2 is about to draw the first mandate in this world that writes contracts, and the
moment it does, a hole that nothing can reach today becomes a defect in every period.

**The hole.** A fund's equity is ZERO by construction (Fund Shares A3) — its share liability follows
whatever its book comes to, because `fundShareKind` declares `owes: 'value'` and derives that value
from `navOf`. That construction holds only while the NAV can see the WHOLE book. **A contract is not
in the register** (Derivative X1): nobody issued it, nobody holds units of it, and it is on both
sides' books at once, so it lives in its own store — and `navOf` walked holdings.

So the two halves of the identity read different books. The fund's **equity account** moved with
every contract revaluation (`revaluationOfContract` books a real gain to one side and a real loss to
the other); its **share liability** moved with the register only. They diverge by exactly the
contract book, and that divergence is a fund WITH equity, which is the one thing a fund may never
have. Measured at **83,247,864 on one vehicle** the first time funds were let into the contract
books; it is invisible today only because every mandate drawn says `mayWrite: []`.

**The fix is one read, and it is the kernel's.** `DerivedReads.contractsOf(party, at)` hands back the
party's OPEN contracts, signed, in the money each was written in. It is injected into `Valuation` the
way the curve, the calendar and the book-money read already are — because the contract store is built
after it, and because `prices/contract-value.ts` must stay the ONE writer of what a contract is worth
(Law 4). Nothing computes a mark twice.

**`navOf` splits it by SIGN and never nets it.** D1 says a contract is an asset to one side and a
liability to the other at every instant, so a positive mark is an asset and a negative one is
something the pool owes. A pool long one contract and short another HAS an asset and a liability —
adding them first would hide half of both, which is what §28 B5's three reads exist to prevent and
what Appendix B means by no netting across counterparties.

**What is NOT here, said rather than left implicit.** A contract mark carries no "period this came
from" the way a print does: it is computed from this period's public reads at the moment it is asked,
so there is nothing to age. What CAN be stale is a print the mark reads, and that is the underlying
line's own staleness, reported wherever that line is held (B2.a).

**The rest of the step is 13.2's.** *"Then wire hedge funds into every derivative book as the
speculative side"* needs a mandate that writes contracts, and there is none yet.

**Two more of item 13's steps are closed by what 10e already built**, and both are marked with what
was done differently:

- **13.1** (`Mandate` is the spine) — 10e went further: the mandate carries the whole PRODUCT, and
  the pools that exist are the performing mandates rather than a declared roster.
- **13.9** (the management fee and what a manager costs) — 10e.4 took it whole. The one deliberate
  difference is worth recording: this step wanted the mandate COMPETED FOR in a book per pool, and a
  mandate auctioned between managers with no reason to refuse clears at the tick — which is the
  defect the step was written to avoid, reproduced from the other side. What sets a fee is ENTRY: a
  manager opens a competing product under the cheapest incumbent and stops when the fee would no
  longer cover what a pool costs it.

Typecheck 0, lint 0, `check:spec` 215 tags, `check:forbids` 4 over 212 files, `check:deaths` 4 of 4,
`check:existence` green. Tests written and not run — `test/nav-contracts.test.ts` asserts the
identity on the read itself rather than through a world, because the property has to hold before
there is a party that can break it.

---

## Item 13.2 — a hedge fund is four terms of a mandate, and there is no hedge-fund party kind

The owner's correction is what made this small: *"everything is a fund. An HF runs funds, same as
PE."* So §28 is not a sector to build. It is a house running pools whose mandates say four things a
long-only pool's does not:

| term | what it means | what it cost to build |
|---|---|---|
| a WIDE blueprint | what the strategy is about, and a macro one states nothing at all | nothing — 10e's language already says it |
| `mayWrite: 'anything'` | §28 A4's wide mandate, and the door into every contract book | a word, not a list |
| `leverage: true` | a PERMISSION, and it never supplies (B1) | a term, and a shape off a kind profile |
| a PERFORMANCE FEE | A3's asymmetric second fee | the one genuinely new mechanism here |

**`mayWrite` and `leverage` came off `openMandate`**, where they were hard-coded `[]` and `false`
for every mandate in every world — a fact about this world's data stated as a fact about the shape
of an agreement.

**`'anything'` is a word and not a list of the nine classes this world happens to have.** A list is
the enumeration of the world item 10e deleted from `mayHold`: it goes stale the day somebody writes
a tenth class, and a mandate its investors agreed was unrestricted would silently stop being one.
The asymmetry with a blueprint band — where `[]` means "says nothing" and so admits anything — is
real and worth stating: **a band CONSTRAINS a universe that already exists, so silence is no
constraint; `mayWrite` GRANTS an ability, so silence is no grant.**

**`fundKind.borrows` no longer says `false`.** It was a SHAPE that said no pool anywhere may ever be
levered — a fact about this world's mandates wearing a category's clothes, and the thing 13h claimed
to have built hedge funds on. The kind now says only that the category is capable of it, and whether
a given pool may is its mandate's. **Leverage still PERMITS without supplying** (B1: a fact about a
loan, never a property of the fund): the lender is a prime broker (13.3) and a bank whose request
channel can hear a party that is not a firm (17.9), and until one exists this is a permission nobody
has acted on — a real state, said out loud rather than hidden behind a `false` that meant something
else.

**The performance fee, and the asymmetry is not a rule anywhere.** The manager takes a share of what
a share GAINED over the highest value it has been worth at a charge. A loss is not shared and not
refunded, and because the high-water mark does not fall the manager earns nothing until the pool is
back above where it last charged. That is what a high-water mark IS, and it is why a manager that
has just lost money has a reason to take more risk rather than less — A3's *"the asymmetry of that
second fee is a reason for risk-taking"*, falling out rather than being asserted.

**Law 19: the mark is not stored.** It is the NAV at the last charge, read off the event that
recorded that payment — a fact about a real payment between two named parties, not a running maximum
anybody keeps. A pool that has never charged one has never been above anything, so the mark is the
unit its shares were first counted in. Both fees go through `payFee`, so there is one payment
convention for what a pool owes its manager.

**Three strategies under one house** — equity, credit, macro — because a house with one product has
no book of business to spread its people over (10e.4), and because each is a BLUEPRINT rather than a
mechanism, which is the whole of what 10e's language bought here. The macro one is the test of that
language: it states no class band and no currency, which is how "unrestricted" stays true when this
world grows an asset class nobody has written yet.

**Their investors WAIT.** `semiLiquid` with a quarterly window is D5.a's notice period, *"a real
contractual term with real consequences for who gets out"* — and it is why a shock reaches this
vehicle later than it reaches a money fund.

**A pool is named for its HOUSE, not its bank.** `nameOf` read `d.bank` while every pool in this
world was bank-sponsored; a strategy house is sponsored by nobody, and the bank is where its account
is — a fact about its cash, not about the product (Law 9).

**WHAT IS NOT HERE, and it is 13.2b.** §28 C1 says a hedge fund is *"the natural home of the
speculative side of every derivative book"*. The door is open — a pool is spoken for in every
contract book, its wide mandate says yes, and 13.6 made sure the NAV can see what comes through it —
but **the only reason any class knows how to post is a HEDGER's**: `futureOrders` sells against a
book it holds, and a party with no such book posts nothing. The fix is not a kind branch inside a
class: `DerivativeClassDecl` gains a second reason, answered by the class out of the party's own
OUTLOOK (§46, built), and a bank with a view speculates too. Nine class modules, so it is its own
step — and the item's own warning is the thing to read first: the reason must be a view that can
widen against it, never an arbitrage it cannot lose.

§28 goes 0 MET to **5 MET, 3 PARTIAL, 16 MISSING**, and **Hedge Funds is no longer an absent
sector** — four down to three (Prime Brokerage, Private Equity, Polity).

Typecheck 0, lint 0, `check:spec` 215 tags, `check:forbids` 4 over 212 files, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 13.3 — prime brokerage: the lender `leverage: true` was permitting without

Item 13.2 left a mandate that permits borrowing and nobody to borrow from. §15 is that lender, and it
turned out to belong **in the banks module**: what a prime broker does is WRITE A LOAN, and the loan,
its rate, the capital it consumes and the room the bank has for it are all already the bank's own
economics (ARCHITECTURE 4.11b). What is new is only the decision about how much — which is the one
thing §15 calls the core.

**C1, and it is a view rather than a rule.** *"The broker sets a requirement on the whole portfolio
from its own view of the risk — a decision by the broker, not a formula the client can rely on."* So
the number is built out of the BROKER'S OWN OUTLOOKS: what it requires against a line is **how wide
its own recent surprises about that line have been** (§46 B3's `confidence`). Two brokers looking at
one portfolio want different amounts, because they have seen different things and been wrong by
different amounts. A line it has no view of it does not finance at all — a refusal, not a zero.

**C4.a falls out rather than being written, which is the point.** *"Raising margin into a falling
market amplifies the fall — the mechanism behind most of what looks like contagion."* A market that
moves violently makes every observer's recent surprises wider; wider surprises are a larger
requirement; a larger requirement on a book already worth less is a call; and meeting a call means
selling into the fall. **Nothing states a margin rate, so nothing had to remember to raise it** —
and a stated constant would have deleted exactly this.

**C3.b is asserted in a test, because a floor here is one character.** `available` is
`portfolio − required − financed` and it is allowed to come out negative. That negative number IS the
call. The clause says a floor *"makes the whole path unreachable — and lending the shortfall straight
back at a penalty, from the same broker, makes it unreachable twice"*, so neither happens: what a
client cannot pay is recorded as unmet and stays owed.

**C1.b's offsets, and what is deliberately NOT one.** The requirement is measured on the client's NET
position in each line, which is what a register holding is. Across different lines nothing offsets —
an offset between two lines is a claim that they move together, which is a correlation, and a
correlation nobody measured is a number invented to make a requirement smaller. The client's CONTRACT
positions are not margined here either: the derivative layer already margins them per pair with the
counterparty that holds them, and asking again here would be one exposure collateralised twice.

**Two kernel doors it needed, both of them the documented pattern:**

- **The FUND party-kind ids moved to `registry/profiles.ts`.** A prime broker has to be able to SAY
  what kind of party its clients are, and the module that owns a bank's economics may not import the
  one that owns a fund. An id is a name; what a pool and a manager ARE stays in `funds`.
- **`leverageLimits` / `ParticipantView.mayBorrow`** — the mirror of `tradingLimits` / `mayTrade`,
  and for the same reason. `PartyKindProfile.borrows` is the CATEGORY's answer; whether a given pool
  may be levered is its MANDATE's, which is what its investors agreed. A lender asks the borrower's
  own module rather than reading somebody else's agreement (Observer A4, Law 4). `publishQuotes` uses
  it too, which removes the noise 13.2's `borrows: true` had just introduced: a quote published for a
  money fund that may never borrow is a price for a trade that cannot happen.

**B1's division is now literal.** The POOL holds a permission and a target (`targetLeverage`, a
preference — what it MEANS to run at); the BROKER holds the loan and decides the amount. The pool
ASKS, out of what its broker published about its own account, and the broker answers with a number of
its own — *"the amount available is the lender's decision and it changes"* (B3). A house that wants
four and is offered two runs at two.

**No mirror was created.** The relationship carries the one number that is nobody else's to know —
what the broker required — and nothing else. What is financed is a fact about the register, and a
copy of it on the row would be a mirror the two could disagree about.

**§15 goes 0 MET to 12 MET, 3 PARTIAL, 9 MISSING, and Prime Brokerage is no longer an absent
sector** — three down to two (Private Equity, Polity).

**What is left, and both are placed.** *"Meet it or be liquidated"* and the chain D1–D4 are item
13.4, which the plan already says must FALL OUT of the parts rather than be written. And a finding
found while building this one: **`E-24`** — a margin loan matures in a year like every other loan,
because `write` gives every row a one-year bullet, so a broker's financing falls due rather than
rolling. It lands with 13.4, which already has to decide what happens to a client that cannot pay.

**And a caution worth stating rather than discovering later.** The strategies are not offered to the
public (`E-22`), so until a household cell clears the accredited line or item 14's institutions
arrive, the pools this whole path runs on may gather nothing — and a line against an empty book is
zero. The mechanism is built and correct; whether this world exercises it is a measurement (23.0a).

Typecheck 0, lint 0, `check:spec` 216 tags, `check:forbids` 4 over 213 files, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 13.4 — the loop, and both changes were REMOVALS of something preventing it

The step said the chain D1→D4 *"must fall out of the parts. Do not write a contagion step."* It does,
and nothing was written to transmit anything. What it took was two removals.

**1. An unmet call joins what the pool must find money for.** One line in `strike`: the shortfall a
pool publishes is its redemptions AND what its broker called and it could not pay. The pool already
sells across its book pro rata at whatever the market gives when it has a shortfall (C2.b, XI-2), so
the forced sale needed no mechanism of its own — and the loop then closes by itself:

> the sale is a print → the print is what every other levered pool is marked at → a lower mark is a
> smaller portfolio → a smaller portfolio against the same loan is a negative line → a negative line
> is a call

Nobody wrote any of that. **D4.a forbids a contagion parameter and there is none**, because there is
nothing for one to do.

§15 D1 says the BROKER closes the positions and here the POOL sells them, which is worth naming
rather than glossing: in this world the pool is the registered holder, so a sale by it is a real
transfer and a sale by its broker would be somebody moving units it does not hold (Register B2). It
is forced either way — nothing in the pool chooses.

**2. A SHARE CANNOT BE WORTH LESS THAN NOTHING, and this is the one that mattered.**

While the share claim could go negative, a fund's liability absorbed every loss EXACTLY: assets, less
the loan, less (assets less the loan), is zero. So a levered pool's equity account stayed at zero
however far underwater it went, `failedWhy`'s solvency test could never fire for one, and §28 E3's
*"no fund that cannot fail — a vehicle that absorbs losses indefinitely is the buyer of last resort
in a different costume"* was true of **every fund in this world**. The losses went somewhere nobody
was looking: into a share price below zero, which is a claim that its HOLDERS OWE THE FUND MONEY.

**It is not a floor** (Law 6), and the difference is the whole point. A share is a limited liability —
a fact about the instrument, and one the kind's own `ranking` already stated in words: *"a pro-rata
share of what is left of the fund once anything else it owes is paid"*. `navOf` now says that
sentence as arithmetic. What is left of a book that does not cover its senior claims is nothing, and
the difference is the CREDITORS' loss rather than the holders' debt.

With the claim honest, everything else arrived on its own: the equity account stops netting, the
solvency trigger fires, the estate opens, and the broker is a creditor of the pool like any other —
which is §15 D2's *"the shortfall hits the broker's capital"* and XI-3's fund row (*"its equity is
gone; its broker eats the shortfall"*). **Nothing was added to make that happen. It stopped being
prevented.**

**Two stale things the change exposed and fixed.** `failedWhy`'s comment said a fund *"sits at the
dust either side of zero every period"* — written when no pool could be levered, and a defect the
moment one could (Law 16). And `equityIsZero` reported every non-zero equity as *"a fund with equity
has mislaid somebody's money"*, which is the wrong sentence for half the cases now: **the sign says
which defect it is** — above zero it has mislaid money, below zero its creditors are impaired.

**Placed rather than done, both with the analysis:**

- **13.4b, the MEASUREMENT** the step asks for: one fund's loss reaching another's margin call, the
  path traceable party by party. Every link is already a named pair on an event — `prime.line` →
  `prime.call` → `fund.struck` → a print → the next `prime.line` — so the path is traceable by
  construction; whether it actually propagates in this world is a measurement of the assembled world
  (Law 11), and it goes with 23.0a.
- **`E-24` moved from 13.4 to 17.2.** This step turned out to need nothing of it: what it builds is
  what happens when a client CANNOT pay, and that answer is the same whatever demanded the money.
  What E-24 wants is a line that does not fall due while the commitment stands, which is a COMMITTED
  FACILITY — and it needs a kernel door that does not exist (`restate` amends an agreement's terms;
  nothing amends an instrument's), worth opening once for every revolving line rather than for this
  one.

§15 goes 12 MET to **16 MET, 3 PARTIAL, 5 MISSING**; §28 goes 5 MET to **12 MET, 3 PARTIAL, 9
MISSING**.

Typecheck 0, lint 0, `check:spec` 216 tags, `check:forbids` 4 over 213 files, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 14.0 — INSERTED ahead of 13.5: the money those sectors run on

**Law 10 says a new idea is inserted at its dependency position and that I say where. Here is
where, and why.** Item 13.2 built a strategy house, 13.3 built its prime broker and 10e built a
credit fund — and **none of them is offered to the public, so the only investors this world has
(household cells) cannot reach any of them.** A private-equity fund is the same shape and would have
been the fourth. I started 13.5, found it would be a vehicle nobody could invest in, and stopped:
that is the "door nobody answers" this codebase keeps catching.

The money those sectors run on is INSTITUTIONAL, and nothing in this world allocated any. An insurer
wrote cover, took premiums, and sat on the cash for ever.

**It is small because 10e made it small.** *"Insurance companies and pension funds don't invest
themselves. Their assets are always third party managed"* (the owner), and *"nothing in this world
invests except a fund, and every fund has a manager"*. So there is **no portfolio mechanism here, no
allocation rule, and no decision about which bond to buy** — there is a liability schedule and a
choice of manager, and B2.b's duration matching is the whole of the choice.

An insurer puts what it can spare at the door of the pool whose published mandate duration is
nearest the furthest thing it has promised, through the same venue a household uses, under the same
refusals. Everything after that is the manager's.

**Three things worth naming in how it was built:**

- **The promise is a SELECTION, not an average.** It matches on the FURTHEST date it has promised
  anything on — a fact about one real schedule — and not on a weighted average duration, which is a
  number no promise in the book actually has (Appendix B: no decision at an average).
- **A pool that states no duration is REFUSED, not used as a fallback.** An equity fund is not a
  place to put money you have promised somebody on a date, and the refusal is the answer.
- **What it keeps back is its OWN last claim**, not a ratio anybody stated (A4.c). An insurer
  sitting on cash is an insurer whose promises are unfunded, which is what B2 is about.

**And it fixed the access ladder, which was half of `E-22`.** The plan's rungs — *"retail (a
household cell) | rich retail (a cell over the line) | institutional (insurer, pension, treasury,
firm)"* — are not three kinds of investor. They are **one structural fact about who is asking**: a
CELL is people, and the accredited-investor test exists to protect individuals; a NAMED party is an
institution, and an institution is qualified by BEING one, here as in the world this models — nobody
asks a life company what it is worth before selling it a fund. It is a read of `representation`,
never a list of party kinds, which would be the kind branch Law 15 forbids and would go stale the day
somebody adds a pension. `E-22` is narrowed to the household half, which is the one the number was
always about.

**A pool now publishes its mandate's duration** on `fund.struck`, for exactly this reader. A fund's
mandate is public in the world this models — that is what a prospectus is — and an institution whose
whole decision is duration matching cannot make it from a yield alone.

**14.3 is closed by this and 10e changed what it meant.** It said *"the insurer BUYS duration"*;
after the owner's correction an insurer buys nothing. It is still the answer to *"nothing in this
world is a natural buyer of a long bond"* — the insurer's money reaches a long bond through a credit
fund's mandate rather than through a decision of its own, which is one fewer investor in this world
and one more real one.

**A finding found on the way, and it is a large one.** **`E-25`**: ninety-eight COVERAGE rows and the
COVERAGE header cite *"`docs/IMPLEMENTATION.md` B-1 to B-8 and B-12"* — **of which only B-2, B-9 and
B-13 still exist in that file.** The rest went when the plan was rewritten and nothing said so. It is
load-bearing rather than cosmetic: `check:existence`'s BUILT AND DEAD list is computed off those
marks, so five sectors are reported dead on the authority of a finding nobody can read. It lands at
**23.0b** rather than being fixed now, because what all ninety-eight assert is a MEASUREMENT — has
this module ever produced an outcome — and 23.0 is the run that re-takes it. Re-marking them from the
run is one pass; re-marking them from a guess is ninety-eight guesses. Found here, where two of the
rows described work done this year and said in the same breath that the module had never run.

§27 goes to 12 MET of 23.

Typecheck 0, lint 0, `check:spec` 217 tags, `check:forbids` 4 over 214 files, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 13.5 — committed capital, and the one payment in this world that is not a budget

§29 A is not about what a private-equity fund buys. It is about how it is FUNDED, and that is the one
thing here with no analogue anywhere else: **every other payment in this world is bounded by what the
payer has, and a capital call is not.**

A household spends what it holds. A fund redeems what its cash reaches. A bank lends what its room
allows. Each of those is a budget and each is correct. A2.b says a call is not one:

> *"A call bounded by the investor's spare cash is not an obligation."*

So the instruction goes to the wire **for the whole amount**. Nothing reads the investor's balance
first, nothing trims the demand to fit it, nothing pays it in part. An investor that cannot pay gets a
REFUSED instruction — a recorded failed payment, which is its own cash-failure trigger (Money E1) —
and that refusal IS the default the clause names.

**The property is an ABSENCE in code, so it is guarded as one.** There is no `atMost` in the call
path, and a test cannot assert that there isn't. `tools/check-forbids.ts` can: it now refuses
`atMost`, `atLeast` and `Math.min` anywhere in `commitment.ts`, and **the call path was made one file
for exactly that reason** — `callCapital` is handed the subscription it needs rather than importing
it, so the rule has one file to be true of. The guard was proved to bite before it was trusted: a
probe `atMost` was inserted, the check failed with the clause, and the probe was removed.

This is the fifth silent FORBID, and it is the clearest one yet. A trimmed call **settles, balances
and prints identically** to an untrimmed one. The only difference is that nobody ever defaults — the
entire clause evaporates and not a single number moves.

**The rest of §29 A, and none of it needed a new kind of thing:**

- **A1**: a commitment is an agreement between a named investor and the pool, with the INVESTOR as
  debtor — because what it holds is an obligation to pay money nobody has asked for yet. It is the
  tenth kind of commitment in this world.
- **A2**: the pool's door is now CLOSED to ordinary subscription. There is no subscribing to a fund
  that has been raised, which is what closed-end means and is the other half of why it can never be
  a forced seller: nobody can put money in either.
- **A3**: the manager is on committed capital plus CARRY — charged on what was PROMISED rather than
  on what is invested, which is why a fund that has not deployed still costs its investors something.
  The carry is the same asymmetric fee 13.2 built, over the same high-water mark.
- **A5**: the mandate is UNLISTED equity — `listed: false`, the read that separates a private company
  from a public one, which 10e's blueprint language already had.

**What is NOT here, and I chose to say it rather than widen the mandate to hide it.** This world has
no unlisted equity: every share in it trades. So a fund with this mandate can buy **nothing**, calls
its capital and holds it. The alternative was to give it a mandate over LISTED equity — and a
closed-end fund over listed equity is not private equity, it is a closed-end equity fund with the
wrong name on it. §29 B is 13.5b, and the analysis is written into that step: `control` already has
the entire tender, and what makes it a BUYOUT rather than a merger is two things (the target
survives; the debt is the target's).

**Why the investors committed is not modelled and is not pretended to be.** The commitments are an
opening condition (Seed A3) — a closed-end fund was raised before it existed, which is what a vintage
is, and this world opens with banks that have balance sheets on the same terms. The decision to lock
money up wants a pension with a very long liability (14.1) or a deal pipeline worth funding (13.5b),
and neither exists. What is built is everything that happens after.

§29 goes 0 MET to **5 MET, 1 PARTIAL, 19 MISSING**, and **Private Equity is no longer an absent
sector — there is one left in this world, the Polity.**

Typecheck 0, lint 0, `check:spec` 218 tags, `check:forbids` **5** over 215 files, `check:deaths` 4 of
4, `check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 10f.1 — every firm has a share line, and being public is a MARKET and not a size

**The owner's correction, in four sentences, and this is the first of them:** *"There should be named
public and private firms. Going public is a matter of funding choice, not how large a firm is."*

What stood in the way was one line of `drawListed`:

```ts
if (f.size < LISTING_SIZE) continue;   // LISTING_SIZE = 8
```

A firm was listed if it was eight times the smallest in its line, and a firm below that **had no
share line at all**. Two things are wrong with that and the second is much the worse.

**Going public is a funding CHOICE (D1.b).** A firm sells part of itself because it wants money it
would rather not borrow. That is a decision a small firm can take and a very large one can decline,
which is why a real economy's largest companies include private ones and its exchanges are full of
small ones. A rule that lists the large and only the large states the answer to the question 10f.2
exists to ask. So `PUBLIC_AT_THE_OPENING` is drawn per firm, **independently of size**, one in twelve,
and it is read exactly once: from period one a firm floats or is taken private and nothing reads the
draw again.

**And a firm with no share line has a residual NOBODY HOLDS.** Appendix B forbids that outright. At
nine thousand firms the threshold left more than eight thousand of them unowned — and it was silent,
because a firm whose residual has no holder still trades, still pays wages, still fails.

### Three things came out of it that the step did not foresee

**1. There was a second, deeper hole, and it had a bound in it.** Even for a listed line the float
was divided across every saver in the world and then dropped whenever the division came out under one
piece a member:

```ts
if (perMember > 0) for (const cell of cells) ctx.endowUnits(cell.id, id, perMember, price);
```

A cell is homogeneous and holds WHOLE pieces per member (XI-15), so a line can only be held by as
many people as it has pieces to give one each. At thirty million members a firm needed a
thirty-million-dollar book to give everybody one share — and below that the line existed, the market
opened, and **nothing was ever issued into it**. That `if` is a bound (Law 6) and it was hiding the
same defect the size gate was.

The holders are now **as many cells as the count can fill**, in a rotated order so that no one cell
ends up owning every small company in the world. A big line reaches every saver; a small one reaches
one cell of them; nothing is dropped. It is also simply true — *fewer people own a smaller company* —
and it is the shape C1.b's free float and C2.e's founders will take once a firm is born rather than
seeded.

**2. `OPENING_SHARE` was too coarse to be a resolution.** Its own comment already called it one and
tested it by D4's split invariance, and that was right — but a resolution has a job, and this one's
job is to decide **how many owners a line can reach**. At a dollar a share a firm's whole book came
to fewer shares than this world has savers. It is now ONE TICK, a cent a share, written as
`CENT_TICK × MONEY_PIECES / SHARE_PIECES` so it says *one tick* rather than a number. The same book is
cut into a hundred times as many pieces and reaches a hundred times as many owners, and by D4 that is
the only thing it changes.

**3. The carrying rule had to learn to read the LINE, not only the kind.** This is the structural
change and it is in `ARCHITECTURE.md`. `pricing: 'cleared'` and `carry: 'mark'` are properties of a
KIND, so the first private share line in this world would have sent the revaluation to
`printOrThrow` for a price that never existed, and the build would have stopped on the first period.

`Valuation.atCost(instrument)` is the one reader of it now — `carry === 'cost'`, or a cleared kind
whose line has no market — and every valuation asks it: `worthOf`, `valueOfLots`, and the revaluation,
which walks past such a line instead of marking it. **That is §29 C5.a — "an unlisted mark is not a
cleared price" — kept by there being no price to mistake for one**, rather than by a rule against
mistaking it. `markPerUnit` refuses to answer for a market-less line at all, citing the clause.

§29 C5 and C5.a go MISSING → MET, and `Private Equity A5`'s *"this world has no unlisted equity,
because every share in it trades"* is no longer true: eleven of every twelve firms in this world are
now private companies with named owners, a residual, a dividend and no price.

### What a private company can and cannot do

`decideEquity` takes the market as an `Option` now, and that is the whole of the difference. With no
market there is no print, so `dear` is false and it cannot issue; and it must not bid for its own
shares in a book that does not meet, so the buyback branch tests the MARKET and not the price. What
is left is what a private company actually does with spare cash: **it pays its owners**. The firm is
in no market's participant list at all unless its published plan has a buyback in it.

### What this found and did not chase (both positioned, Law 10)

- **F-1, the resolution floor → 13n.** A firm whose book is under one cell's worth of pieces — about
  twelve thousand dollars — still opens with an unissued line. It is a hole two thousand four hundred
  times smaller than the one this item closed and it is not a bound: the arithmetic cannot cut a
  company into fewer people than a cell stands for. What it wants is a finer population (12.6's own
  placeholder, or XI-15's cell split) or a founder who is a party rather than a cell — Firm Birth A.
- **F-2, the cost of a line for every firm → 16.** Nine thousand instruments and nine thousand
  decisions a period where there were seven hundred and forty of each. Nothing about it is wrong, and
  Law 18 says a traversal is measured rather than guessed.

Typecheck 0, lint 0, `check:spec` 218 tags, `check:forbids` 5 over 215 files, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 10f.2 — a firm goes public because equity is the cheaper money

**The owner's second sentence:** *"IPOs and take privates should exist."* This is the IPO. The
take-private is 10f.3, and the door it walks through (`Instruments.delist`) is built here, because
it is the same door the other way and writing one without the other would have been writing half a
fact.

### The decision, and neither side of it is invented

D1.b already said what an issuance is — *"a decision with a reason: a funding need it prefers to
meet with equity"* — and 10f.1 removed the size threshold that was standing in for the decision.
What a firm compares is two numbers somebody else published:

- **what debt costs it** is `credit.quoted`, the keenest quote it was given, which is already the
  outcome of banks competing for it (Banks Lending C3.a) and comes with the SIZE that bank will lend;
- **what its own money earns** is its published accounts (Reporting A2): what it made over the book
  it made it on, annualised by the span of its own report. No second set of accounts, no forecast.

And the comparison is the one a management actually makes. **Borrow at more than the business earns
and the owners are worse off; sell part of it and the new owners share what there is.** So a firm
whose book earns twenty per cent borrows at eight and keeps the difference — it stays private however
large it is — and one whose book earns four floats, however small. **Nothing in it reads a size.**

The other two reasons are refusals rather than preferences, and they are what the credit market
decided about the firm: nobody has quoted it at any price, or its bank will not lend it enough.
That is Corporate Credit A1's *"a firm large enough to reach a market is not at the mercy of one
lender"* seen from the borrower's side.

### It does not price its own issue

It brings a SIZE and the least it will take — its own book per share, which is a reservation and not
a price (B3) — and the session strikes the level out of what the bidders posted (B1, B2). **That
first print is the first price the line has ever had, and that sentence is §29 D2 verbatim**: an
exit produces a cleared price, which is the first real price the holding has had. A flotation and a
private-equity exit by flotation are ONE mechanism, which is where the owner's *"the PE case being
only an application"* first bites. D2 goes MISSING → MET on the strength of the same code.

It can fail (D1.c, Clearing C4.a), and what a failed IPO leaves behind is a listed company that
raised nothing. The listing happened; the offering did not. Its existing owners can now sell into
the book, which is a direct listing and a real thing rather than an artefact.

### Three things it needed, and two of them were defects

**`Instruments.list` and `delist`.** A line's market was fixed at issue, so nothing in this world
could ever start or stop trading. They are the only writers of it, and `ctx.list` opens the book in
the same call — one door, because `addMarket` refuses a market whose instrument does not name it and
a module that seated one and forgot the other would leave a line pointing at a book that is not
there.

**F-4: a fund could never bid for a share.** `ordersOf` priced its bid by discounting `cashFlows`
and returned nothing when there were none — and a share promises nothing dated, so the flows were
always empty. Meanwhile `eligible`, three lines above, used `view.worth` and said the mandate
admitted it. Two valuations of one thing that disagreed about whether a share can be valued at all,
with two different day counts (Law 4). It reads `view.worth` now, which is the kernel's one door: a
bond discounts its promise, a company capitalises what it published. **Without this an IPO would
have had no bid side at all** — households deliberately do not read accounts (item 13d), so the only
parties in this world that can value a line that has never printed are the ones with mandates.

**`atCost` asked the wrong question, and it would have stopped the build.** It asked whether a line
had a MARKET. A line that lists this morning and whose first book finds no bidder HAS a market and
has never printed, and the revaluation would have gone to `printOrThrow` for a price that does not
exist. It asks whether anything ever printed one now, which is simpler and truer, and
`carryingPerUnit` reads it too — that one asked the store for *last* period's print of a line that
did not trade last period, which is every line's first print. A lot nothing has ever priced has
recognised its basis, because there is nothing else it could have recognised.

### What is now true that was not

- Every firm in this world has a residual and named owners (10f.1), and a private one can become a
  public one by deciding to (10f.2).
- `EquityDecl.listed` is an OPENING CONDITION and nothing at runtime reads it: whether a line trades
  is a fact about the line, and `decide` and `float` both read the register (Law 19).
- A line's makers are an opening draw, so a firm that lists afterwards has none and the fallback this
  world already documented takes over — the bank's own `makes` decides. **What it wants is 10f.4**,
  where the bank that RAN the flotation is the one that quotes it, which is what an underwriter is.

§29 D1 goes MISSING → PARTIAL and D2 MISSING → MET; Equity E3 stays PARTIAL with the flotation built
and the take-private not.

Typecheck 0, lint 0, `check:spec` 219 tags, `check:forbids` 5 over 216 files, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 10f.3 — one M&A layer, four outcomes, and not one of them is a flag

**The owner's fourth sentence:** *"m&a, acquisitions, mergers and disposals should all exist, with
the PE case being only an application."* The plan's table said acquisition, disposal and take-private
were all missing and that `combine` was *"right for a merger and wrong for everything else"*. That
was right about the outcome and wrong about how much was missing: `control` already had the tender,
the acceptance condition, the majority and the combination. What it did not have was **a way to be
anything but a merger**, and **anybody to sell**.

### Which outcome a deal has is a READ

A buyer that ends up holding all of a company does one of two things, and they are different deals:

- a **MERGER** — the two balance sheets combine and the combined firm is one party (§35 A4);
- an **ACQUISITION** — the target goes on being a company with its own balance sheet, its own name
  and its own debts (§29 A5), and if it had a market that market closes, which is the TAKE-PRIVATE.

Which one is not a kind branch and not a flag on the bid. **It is whether the acquirer could run
what it bought, and running a business is having people do the work.** A wage bill, read off the
wire: a party that has never paid anybody has nobody to operate a factory, so it owns the company
rather than absorbing the business.

That one read is what makes §29 A5 — *"the acquired firms are held in named vehicles, each a party
with its own balance sheet"* — **fall out instead of being written for private equity.** A pool of
money employs nobody, permanently, and this module never has to know that a pool of money exists.
It is also B2.a's whole point arriving as a consequence: the target keeps its own debts, so a failed
buyout kills the firm and not the fund.

### F-6: nobody could sell a company nobody trades

A holder answered a tender from its OUTLOOK of the line's price. An outlook is formed from prints. A
private line makes none. So every bid for a private company failed with `control.failed` / *"nobody
tendered"* — **the acquisition the owner asked for, refused by the one read that cannot answer for
it**, and it would have refused every one of the eight thousand private companies 10f.1 created.

A holder with no outlook answers from what its own books carry it at: its LOTS, which is what it
paid. Its own number, read off its own books, and the one number a holder of something with no price
has. It is not a price and is not used as one — the book strikes the level where the holders' numbers
meet the buyer's — and a holder carrying it above the bid keeps its shares, which is C2 unchanged.

### The disposal is the seller's side of the same book

*"An owner SELLS a business it holds"* needed no new venue and no new instruction: it needed a
REASON, and the reason anybody sells anything is that it needs the money. A holder that published it
is short of what it wants to build takes what the book gives rather than naming a level (XI-2), which
is the difference between selling something and valuing it. It is the same `firms.funding` read the
bond channel and the flotation use — **a firm has one hole and three ways to fill it: borrow, issue,
or sell something** — and a second number here would have been a second hole.

### What this leaves

`M&A B2` goes MISSING → MET (the premium is what the tender book struck, and `premiumOver` is a read
of the distance between it and the last print, with no premium number anywhere). `Equity E3`
PARTIAL → MET and `Private Equity A5` PARTIAL → MET. M&A's ABSENT-SECTOR marks fall from ten to
seven.

`M&A B4` — a competing bidder — is still missing and is the honest gap: `runTender` opens one venue
per bid with one buyer in it, so a contested auction has nowhere to happen. **That is 10f.4's**,
where a seller appoints a bank and the bank invites bidders, and it is the same book with more than
one name in it.

Typecheck 0, lint 0, `check:spec` 219 tags, `check:forbids` 5, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 10f.4 — a bank runs the sale, and what it can run is the people it employs

**The owner's second sentence, second half:** *"formal exit processes and m&a processes lead by IBD
departments."* A sale was a bilateral tender that appeared from nowhere: one buyer, one venue, no
process, nobody paid for the work.

### `advisory` is a trade, and a capacity is a count of people

A corporate-finance banker is not a lending officer and not a market maker (Labour A3), so it is a
row in the occupations table and a bank posts openings in it like any other employer — for the hours
the sales it ran last period took, so a department nobody appointed shrinks without a rule.

**How many sales a bank can run at once is the hours it paid for over the hours one takes.** It is
the same read a dealing desk makes about how many lines it can quote and a manager makes about how
many pools it can run. Nothing states a capacity; a bank that has never met a wage runs none. Three
weeks of somebody is `bank.hoursPerProcess`, declared as the technology it is.

### The fee is what the work costs, and the seller picks the cheapest

Each bank publishes what it will run one for — hours at what an hour of that trade costs IT, which
differs by bank because what a bank pays differs — and the seller appoints the cheapest with people
free. **There is no percentage of a deal anywhere.** A percentage of an outcome is a fee with no work
in it (Law 2), and it would make a large sale dearer to run than a small one for no reason anybody
could name. What stops the fee falling is the same refusal 10e.4 set for a manager's fee: a bank that
cannot cover its people stops publishing, and nothing bounds the fall (Law 6).

It is paid out of the deal, in one instruction with two sides, and a client that cannot pay it leaves
the bank having done the work and not been paid — a recorded state, not an adjustment.

### And B4 bites arithmetically

Every bid for one company goes into ONE book against every holder's ask, and the solver strikes the
level. **A second bidder raises what the holders are met at whether or not it wins**, which is what
*"the price is contested"* means as arithmetic rather than as a sentence.

That needed the bids GATHERED BY TARGET before any of them ran. Two buyers taking turns at a target
in whatever order `parties.ofKind(FIRM)` happens to be in is not an auction: the first past the post
bought the company before the second was asked, and the second's willingness to pay more never
touched anything. `M&A B3` and `B4` go MISSING → MET.

**What is honest about the gap:** the winner is the highest bidder and it buys the whole cleared
volume. Pairing several buyers against several sellers inside one clear needs an allocation rule this
world does not have, and inventing one would be inventing who faced whom (Law 1). What the losing
bids do is move the price, which is the half of B4 that carries the economics.

Typecheck 0, lint 0, `check:spec` 219 tags, `check:forbids` 5 over 216 files, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 10f.5 — an institution buys more than duration, and diversification is an outcome

**The owner's third sentence:** *"Also, insurance and pension don't only go for duration. They invest
in tons of different strategies."* This corrects item 14.0, which is five commits old.

### What 14.0 did, and why it was worse than it looked

It matched the insurer's longest promise to the pool whose stated duration was NEAREST it, took that
one, and refused every pool that stated no duration at all. The refusal was written down as a
principle — *"an equity fund is not a place to put money you have promised somebody on a date"* —
and there was even a test asserting it.

**Every strategy pool, every equity pool and every private-equity pool in this world states no
duration**, because they have none to state. So 14.0 — which was INSERTED to give the sectors 13.2,
13.3 and 13.5 built some money to run on — reached exactly one of them: the credit fund. One
institution, one manager, one asset class, for ever.

### Three reads replace it, and not one of them is a preference

- **Duration is a refusal, not the decision.** A pool whose stated duration runs PAST the furthest
  thing this institution has promised is refused, because holding it is a rate risk nobody asked it
  to take. A pool that states NO duration is not refused: equity has nothing to mismatch. One test,
  and strategies, credit and equity become eligible through it rather than through three.
- **What it requires is what its own promises are discounted at** — the sovereign curve of the money
  they are promised in, at the tenor of the furthest of them. That is B2's actual economics and it is
  a READ: no preference declared, no spread over anything. A pool that has published that it earns
  less than that has told it it does not cover the promises and is refused; **a pool that has
  published nothing has not claimed anything to fail against** (App A), which is exactly why an
  institution reaches past bonds instead of sitting in cash when nothing yields enough.
- **How it spreads is by feeding the smallest.** This period's money goes to whichever acceptable
  pool it holds least of, valued at what that pool itself published. Diversification is then the
  OUTCOME of doing that every period, and there is no weight, no target, no maximum and no optimiser
  anywhere (Law 2, Law 6). A pool it has never bought is worth nothing to it and is therefore next.

An institution that has promised nothing applies neither test, which is right: it has capital and no
liabilities, and there is nothing for an asset to be mismatched against.

### The test that asserted the defect

`institutional-allocation.test.ts` had a case called *"refuses a pool that states no duration at
all"*, with a paragraph explaining why the refusal was the honest answer. It was the defect, written
down and guarded. It is rewritten around the two refusals that replaced it — and the case that
matters most now is the opposite one: **a pool that states no duration is NOT refused**, which is the
line the owner's correction turns on.

Typecheck 0, lint 0, `check:spec` 219 tags, `check:forbids` 5, `check:deaths` 4 of 4,
`check:existence` green. Tests written and not run.

---

## Item 10f.6 — §29 B, C and D turned out to be two reads

**The owner's fourth sentence, second half:** *"with the PE case being only an application."* 13.5b
was written as nineteen clauses of private-equity mechanism. After 10f.3 it is two reads, and
`control` gained no private-equity branch, no vehicle type and no buyout flag.

### A pool bids because it published a cost of money

M&A B1 values a target as *"what it would get out of it, against what its own money costs it"*, and
a company reads what a bank quoted it. A pool has never been quoted by anybody — but it has a cost
of money, and it is what its own investors require of it. That is a public fact in the world this
models (a prospectus states a target return), so `fund.struck` carries it beside the duration band
it already published for the insurer's allocator, and `costOfMoneyOf` reads whichever of the two
sources answered.

**One question, two sources, and nothing here knows what a pool is.** It is the same shape
`wageFacing` has for what an hour costs an employer: its own experience where it has one, what was
published where it has not.

And the buyers are a walk of the JOURNAL now rather than of the party list: whoever published a cost
of money this period. That is B3 — *"it must be able to fund it, so the credit market decides which
deals happen"* — and Law 18 at the same time, because the buyers are the handful who published
rather than every party in the world asked one at a time.

### A pool exits because it must find money

§29 D1's three exits are the flotation (10f.2), the sale to another fund and the sale to a corporate
(10f.3's tender). What was missing was the fund's REASON to sell, and it is the same reason a company
disposing of a business has: it needs the money. A company publishes what it is short of for the
thing it wants to build; a pool publishes what it must find — redeemers owed, a broker's call it
could not pay, or a wind-up that queued everybody. `mustSell` reads both.

The proceeds reach the investors through the redemption queue that was already there (D3), and **D4
falls out with nothing written**: *"in a bad market it does not happen, the hold extends"* is the
tender book striking what it strikes and a seller with no buyer keeping what it holds.

A5 was already done at 10f.3 — a buyer that employs nobody owns the company rather than absorbing it,
so an acquired firm is a named vehicle with its own balance sheet.

§29 goes 9 MET to **12 MET, 1 PARTIAL, 12 MISSING**.

### What it did NOT build, said plainly

**The LEVERAGE (B2, B2.a, B4, B5, and C3's recapitalisation).** *"Most of the price is debt raised
against the target itself"* needs the TARGET to borrow, conditional on a tender that has not settled
yet, and to pay the sellers in the same breath. That is a two-phase deal, and the period loop has no
shape for a funding request that depends on an outcome later in the same period. What exists instead
is honest and smaller: the pool pays out of the capital it called (13.5), so a buyout here is
unlevered and B2.b's *"the credit market decides which buyouts occur"* is not yet biting. It is
inserted as its own item at 17.9's dependency position, where the bank's request channel opens.

**Item 10f is CLOSED.** Four corrections, six steps, and the count it moves: Equity 26 MET of 37,
M&A 13 of 22, Private Equity 12 of 25 — from 5, 10 and 5 when the item was inserted.

Typecheck 0, lint 0, `check:spec` 219 tags, `check:forbids` 5, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 13.2b — the speculative side had a door, and a pool that could not be sized

**The step was written as**: the classes know only a hedger's reason, so `DerivativeClassDecl` needs a
second one — what a party with a VIEW would take — and it touches nine class modules.

**That is not what was wrong.** Every class already has a speculative term: its own number against
where the book stands, sized by conviction, both ways around its own level when it has nothing to
hedge. `A-66` added them after this step was written, and the step went stale without anybody
noticing — which is the ordinary hazard of a plan that outlives the code it was written against.

### What was actually wrong, and it was in all eleven of them

```ts
const own = view.equity();          // and ten more like it
const conviction = sizeOf(view, own, mine);
```

**A pool's equity account is ZERO by construction.** Fund Shares A3: the holders own the assets, so
assets minus liabilities is nothing, and a fund with equity has mislaid somebody's money. It is not
an accident to be worked around — it is the clause, and there is an audit family that enforces it.

So the one party §28 C1 calls *"the natural home of the speculative side of every derivative book"*
was asked in every book, was permitted by its wide mandate (13.2), had its position seen by the NAV
pass (13.6) — and computed a conviction of exactly nothing, in all nine classes. **The door was open
and there was nothing to say at it.**

And it left no trace anywhere. The pool is asked, it returns no order, the session prints exactly as
it would have. Nothing fails, nothing is unbalanced, no number is wrong. That is what a silent
prohibition looks like from the inside.

### One kernel door, eleven reads deleted

`ParticipantView.standsBehind()` is the third door of the shape `mayTrade` (9.7) and `mayBorrow`
(13.3) already have, with the same rule: exactly one module answers for a party kind, and a kind
nobody answers for gets the honest default. The default here is **the party's own equity account**,
which is what a loss falls on for a bank, a firm or a household and needed no saying — and a pool's
module answers differently, with its **own last published NAV** (Law 19: its own number, not a
second walk of its register).

**It is a magnitude and never a limit.** Nothing is bounded by it (Law 6): it is what a conviction is
scaled against. A pool that has been wrong loses money, its next NAV strike is lower, and it takes a
smaller position — which is the step's own warning honoured exactly: *"a view that can widen against
it, never an arbitrage it cannot lose"*.

### The sixth silent FORBID

Eleven sites, nine modules, no output. That is the case `check-forbids` exists for, and CLAUDE.md
says so outright: *a rule that can be a check should be one; when a rule is broken twice, write the
check*. `view.equity()` is now refused anywhere in the nine class modules, with the clause and the
reasoning in the failure message. **The guard was proved to bite before it was trusted**: one site
was reverted, the check failed naming it, and the site was restored.

§28 C1 goes PARTIAL → MET. Hedge Funds is 13 MET of 24.

### What the step asked for and did not need

A second `reasons` hook on the class. A fund forms outlooks the ordinary way — §46 builds one from
the instructions a party was actually a side of, and a fund trades — so the lines it has a view on
are already what `options.reasons` returns, and a book that has printed is already `openToAll` to
everybody. The narrowing was never the barrier; the sizing was.

**Also fixed here**: the `10g` row inserted at 10f.6's close sat at position two in the ordered plan
while its own reason said it belonged after 17.9. Law 10 says insert at the dependency position and
say where, and the two have to agree. It is **17b** now, after 17.

Typecheck 0, lint 0, `check:spec` 219 tags, `check:forbids` **6** over 216 files, `check:deaths` 4 of
4, `check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 13.5c — the second rung of the ladder A2.a describes

§29 A2.a: *"an investor must hold liquidity against calls it did not choose the timing of, and in a
stress the calls and its own troubles arrive together."* 13.5 built the obligation and the default —
the call goes to the wire for the whole amount, nothing is trimmed to fit, and an investor that
cannot pay gets a refused instruction. **What was missing is the rest of A2.b's sentence**: *"the
investor funds it from its own liquidity ladder — selling if it must — or it defaults."* An investor
that defaults and does nothing about it defaults again next time, and a ladder with one rung is not
a ladder.

### It holds liquidity, and not the way that would defeat the clause

What its last call TOOK comes off what it can put to work. It is its own experience and the same read
the claim buffer already is (A4.c), and no number is declared anywhere.

**It is never the undrawn commitment, and must not be.** Money set aside against a commitment in full
is money already paid, and the whole of A2 is that capital is committed and NOT paid — an investor
holding its commitment in cash has converted the structure into a subscription. So nothing here reads
a commitment's size. What it holds is what a CALL costs, which is the liquidity the clause names.

### And it sells if it must

A call it could not meet is money it must find, so it asks for its money back out of the pools it is
in. It names no price (XI-2): a forced seller that named one would not be one, and what it gets is
what the queue gives it at the NAV of the day it asked. It is the SAME channel a household short of
its own cushion uses — a holder that needs money asks the pools it is in — and there is one of it in
this world rather than one per holder (Law 4).

**The pool that called it is the one it cannot redeem from.** A closed-end fund has no redemption;
the money was committed for its life. So the trap A2.a describes arrives without being written: it
has to sell something else, and the refusal on the closed one is recorded rather than dropped.

**The lag is the clause.** A call arrives and settles in one pass, and the response is the next
period. *"In a stress the calls and its own troubles arrive together"* is what that looks like from
inside, and an investor selling into the market a week after it was called is the shape of it.

### One thing I checked and was wrong about

While marking coverage I found four §29 clauses with no COVERAGE row and measured what looked like
123 of them across the spec. `coverage-existence` already handles it and its own docstring says so:
*"a clause with no COVERAGE row at all is counted MISSING — treating an absent row as absent from the
count would let a system look complete by having fewer rows."* It iterates the SPEC, not COVERAGE.
A2.a itself is a spec NOTE (it carries no REASON/VERIFY/FORBID word), so its MET row is reported
beside the other eight rather than counted — which is the tool showing two files disagreeing about
what a requirement is, exactly as it was built to.

Typecheck 0, lint 0, `check:spec` 219 tags, `check:forbids` 6, `check:deaths` 4 of 4,
`check:existence` green. Tests written and not run.

---

## Item 13.7 — the verification step, and its own figures were the stale ones

13.7 asked for *"COVERAGE re-marked for all 73 clauses across the three"* and for `check:existence`
to show three fewer absent sectors. **The re-marking had already happened**, in the commits that built
each piece — which is the loop's own rule (one item, one commit, carrying its COVERAGE re-mark), so
there was nothing to batch. What was left is a CHECK of what those commits claimed, and the thing it
caught was the step's own text:

> *"§28 is done at 13.2: Hedge Funds is 5 MET, 3 PARTIAL, 16 MISSING … so the count is four down to
> three."*

That was 13.2's figure, written before 13.2b existed, and the count is not three.

### What the three sectors actually are

| sector | MET | PARTIAL | MISSING | of | built by |
|---|---|---|---|---|---|
| Hedge Funds §28 | 13 | 2 | 9 | 24 | 13.2, 13.2b |
| Prime Brokerage §15 | 16 | 3 | 5 | 24 | 13.3, 13.4 |
| Private Equity §29 | 12 | 1 | 12 | 25 | 13.5, 13.5c, 10f.6 |

**ABSENT SECTORS: ONE.** Only the Polity, 0 of 32, which is item 19. It was six when this plan was
written, and five of those six are closed.

Two other things the check looked for and found clean: no row in the three forward-references a step
that has since closed, and none of the three is in the BUILT AND DEAD list — the list of sectors
whose every MET clause carries `NEVER REACHED`, which is the more dangerous kind because it reads as
done.

**`Insurers` is on that list** (9 of 23, none reached), and it is an inherited measurement: it
predates the phases 9.5, 14.0, 10f.5 and 13.5c gave that module. It is already positioned at 23.0 as
`E-25` and was NOT re-taken here, because a measurement is item 23's and taking one mid-build is what
Law 11 forbids.

### Part 0's tables are annotated rather than rewritten

The two audit tables in Part 0 are the record of what the audit FOUND, and their "who owns it today"
column had gone stale in four rows. Rewriting the numbers would delete the finding; what is added is
what has since closed, beside each row, and one line saying the count is now one. **The shape of the
finding is what the table is kept for** — a sector with no clause MET leaves no trace in any output,
which is why nobody noticed six of them — and that does not stop being true because the list is
shorter.

Typecheck 0, lint 0, `check:spec` 219 tags, `check:forbids` 6, `check:deaths` 4 of 4,
`check:existence` green. Tests written and not run.

---

## Item 13.8 — the cover eroded in silence, and a margin call had two definitions

`E-14`. §14 C2: *"both sides are marked every period: when the borrowed security rises, the borrower
posts more collateral."* C2.a: *"the margin flow is real money moving between two named parties."*

`charge` moved the FEE every period and **nothing re-marked the collateral**. So between the strike
and the return the borrowed line could double, and the lender's cover did not move. C1's haircut is
one period's worth of the two marks moving apart — it is sized for a period, and it was standing
behind the whole four-period term. Nothing failed, nothing was unbalanced, and the module's own
header cited C2 and C2.a as MET, which is what made it a finding rather than a bug.

### One mechanism, two callers — which is what the step was written to force

A broker marking a client's portfolio against what it requires, and a stock lender marking lent paper
against the collateral it holds, are **the same sentence about two contracts**: what this exposure
requires, against what is actually there, at today's marks, and the difference is real money now.

`registry/margin.ts` is that sentence, once. `prime.ts:callOf` is a caller of it and is no longer its
own definition; `securities-lending:remark` is the second caller. Written twice they would be two
definitions of what a margin call is, and the day one of them gained a floor the other would not.

**It is never floored**, and §15 C3.b is the clause that says so outright: *"flooring it makes the
whole path unreachable."* So it is a subtraction and the sign is the answer — positive is a call to
meet, negative is cover to give back, **and it goes back**. A mechanism that took margin and never
returned it would be a one-sided flow that nothing ever failed on (Law 5). The single thing it cannot
do is hand back more than was posted, which is arithmetic impossibility and says so with `atMost`.

### Two terms the loan did not carry

- **`haircut`** — the lender's, struck when the loan opened, kept because the loan is re-marked
  against it every period. Re-asking the lender's view each period would be C4's *"the broker can
  raise the requirement when it likes what it sees less"*, which is a different clause and a
  different event, and conflating them would have made every re-mark look like a tightening.
- **`margined`** — the cash posted, net, over the life of the loan. It travels back with the
  collateral at term, in the same instruction, and the lender keeps it on a failed return for exactly
  the reason it keeps the collateral (D1): it is left to buy the line back at whatever it costs, and
  whether the two come to the same is its outcome and not a number anybody balances.

And the call can FAIL. The instruction goes to the wire for the whole amount; a borrower that cannot
pay gets a refused instruction (Money E1), and the lender is uncovered with both parties named on it.

### What stays MISSING, and why it is not hidden

**C3 — cash collateral reinvested**, *"this is where a lending programme actually loses money."* The
variation cash moves to the lender's account and stops there. It is the lender's money to put to
work and nothing does, so the position C3 names does not exist. That wants a lender with somewhere to
put it, which is item 14's allocator, and the row says so rather than reading as done.

§14 goes 14 MET to **15 MET, 6 MISSING** of 21.

Typecheck 0, lint 0, `check:spec` 220 tags, `check:forbids` 6 over 217 files, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 13 — CLOSED. Three sectors that produced nothing, and one absent sector left

Item 13 was the three asset-management systems: hedge funds (§28), private equity (§29) and prime
brokerage (§15). All three opened at **0 MET**, and they were three of the six absent sectors this
plan was written to find — a sector with no clause MET produces nothing and leaves no trace in any
output, which is why nobody had noticed them.

| | opened | closed | by |
|---|---|---|---|
| Hedge Funds §28 | 0 / 24 | **13** MET, 2 PARTIAL | 13.1, 13.2, 13.2b, 13.6 |
| Prime Brokerage §15 | 0 / 24 | **17** MET, 2 PARTIAL | 13.3, 13.4, 13.8 |
| Private Equity §29 | 0 / 25 | **12** MET, 1 PARTIAL | 13.5, 13.5c, and 10f.6 |
| Securities Lending §14 | 14 / 21 | **15** MET | 13.8 |

**ABSENT SECTORS: ONE.** Only the Polity, 0 of 32, which is item 19.

### What the item turned out to be, against what it was written as

Almost every step was smaller than its plan said, and each for the same reason: **the mechanism was
already there and something one line long was making it unreachable.**

- **13.1 was done at 10e**, and further than written.
- **13.2**: there is no hedge-fund party kind. What makes a pool a hedge fund is four terms of a
  MANDATE, and a strategy is a blueprint.
- **13.2b** was written as *"every class needs a second reason"*. Every class already had one. What
  none of them had was a party that could be SIZED: eleven speculative terms all read `view.equity()`,
  and a pool's equity is zero by construction. One kernel door, eleven reads deleted, and the rule is
  now the sixth silent FORBID.
- **13.3**: prime brokerage is a bank's fifth business line and what it does is WRITE A LOAN. Two
  kernel doors, and C4.a fell out of brokers disagreeing rather than being written.
- **13.4**: the loop D1→D4 needed no contagion step. **Both changes were removals.**
- **13.5**: no private-equity party kind either — a closed-end mandate over unlisted equity. Its
  FORBID (a call bounded by the investor's spare cash) became the fifth silent one.
- **13.5c**: the second rung of A2.a's ladder — hold liquidity against a call, and sell if you must.
- **13.6**: the NAV pass reads the contract store, so a pool with a position is not a fund with equity.
- **13.7**: a check, and the thing it caught was its own figures.
- **13.8** (`E-14`): both sides marked and the difference called. A margin call had two definitions
  in this world and now has one, with a broker and a stock lender as its two callers.
- **13.9 was done at 10e.4**, and differently on purpose: a fee set by ENTRY rather than by a book.

### The measurement is removed

**13.4b is gone on the owner's instruction.** It asked for a measurement of the loss chain — one
fund's loss reaching another fund's margin call — and measurement is not what this item is for
(Law 11). §15 D4, the clause it carried, asks that the chain be **traceable party by party**, and it
is, by construction: every link is a named pair on a public or own-name event, and a loss that
stopped at the fund would need a link with one party on it. There is no such link in the path. D4
goes PARTIAL → MET on the traceability the clause actually asks for, and the row says plainly that
whether it propagates in a long run is item 23's.

### What did NOT close, named rather than absorbed

- **§29 B2, B2.a, B4, B5 and C3 — the LEVERAGE.** *"Most of the price is debt raised against the
  target itself."* The target has to borrow conditional on a tender that has not settled yet and pay
  the sellers in the same breath: a two-phase deal the period loop has no shape for, and it needs
  17.9's request channel. It is **item 17b**, after 17. Until it lands a buyout here is unlevered and
  B2.b's *"the credit market decides which buyouts occur"* does not bite. **This is the third of item
  13's three exit sentences, and it is the one that does not hold.**
- **§14 C3 — cash collateral reinvested**, *"where a lending programme actually loses money"*. 13.8
  moves the variation cash and stops; putting it to work wants item 14's allocator.
- **§28 B1's other half**: leverage permits without supplying until a lender can hear a pool's
  request, which is 17.9 again.

### And the closed sections are gone

This file's own rule: *"this is the plan of what is left, and `docs/RECORD.md` is the ledger of what
was done."* 10f's section was left in when it closed, which was an omission; both are removed here.
Stage C is now 11, 12, 14–18.

Plan completion 92.9%; requirement coverage **65.4%** (896 MET of 1371), from 64.3% when 10f opened.

Typecheck 0, lint 0, `check:spec` 220 tags, `check:forbids` 6 over 217 files, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

---

## Item 11.1–11.4 — the sector exists, and it is a distribution

§42 was **2 of 28 MET and both were the generic cell kernel**: `A6` cited `parties/party.ts`
(cells exist) and `E5` cited `world/cells.ts` (a weight is a count). Nothing in sixty thousand lines
was a small firm, and the one mention of the sector was a comment in `firms/data.ts`.

What that cost is not its own 28 clauses. **A5.a says small firms are the sector where a credit
tightening bites first and hardest**, and without them a tightening has nowhere to bite: every
borrower in this world is large enough to reach a bond market. §36 A4 calls this *"the tier that
lives on"* trade credit, so eleven of Trade Credit's clauses are missing partly because the tier
below them does not exist.

### A small firm is a firm, and that decided the kind

A1 says it outright, and the plan's own words were *"not a new kind of thing; a firm with a weight."*
It is nevertheless a second KIND, and the reason is mechanical: a kind states how it is represented
(`PartyKindProfile.representation`) and cannot be both named and a cell.

What that costs is one name. What it buys is **A6.b**: *"a weight of one is a named firm, so the
boundary between this sector and Corporate Credit's is not a modelling line but a SIZE"* — and A6.c
promotes a cell across it. **Two kinds that can name each other is what a promotion needs**; one kind
with two representations is not a thing a registry can say.

So `SMALL_FIRM` is in `registry/profiles.ts`, for the reason `FUND` is: a name a module that does not
own the kind still has to be able to say. Beside it is **`PRODUCING_KINDS`** — the list every module
that asks *who are the firms in this world* walks — so a mechanism gains the tier by reading a
registry row rather than by branching on a kind id (Law 15). Nothing reads it yet; the modules move
over in 11.5–11.7, each with the behaviour it turns on.

### A6.a decided the data model, and it forced a third dimension

*"Every relationship that must be named is either a dimension of the cell's key or a register row,
never an attribute averaged inside it."* The key is **region, bank and LINE**.

The third is a data change A6.a explicitly permits (*"lifting a row into the key is a data change"*)
and it is **forced**: a cell is homogeneous, so a cell whose members were in different lines would
have a cost base, a customer and a labour venue that were averages over lines — A2.a one level down.

The **LENDER is not in it**, and a test says so. A lender is a loan row per (lender, cell), and
lifting it into the key is exactly the relationship the model would then be unable to name.

And the REGION is read off the BANK rather than drawn beside it: *a cell lives where its bank books*
is the same sentence the households seed makes about where this world's people are, and two
statements of one fact is how they come to disagree (Law 4).

### A2.a is the clause the item turns on, and it is what the test asserts

*"No representative small firm. Default is a threshold event; with one average firm a mean-preserving
spread causes no defaults, and the entire credit content of the sector is gone."*

Nothing about any one cell is written down. What is stated is the WIDTH — a tail, and a **fatter one
than the named sector's**, because the small tier is where the distance between the biggest member
and the median is greatest: the firm about to be promoted out of it and the one person with a van are
both in here. One population per key, cut into cells, each drawing its own size, **so the dispersion
is within a key and not only across keys**.

The test asserts the dispersion rather than the draw, deliberately: a flat draw would satisfy every
other clause in §42 and remove the sector's whole credit content without failing anything.

### What it does NOT do, said in the module rather than implied

The sector exists and **does nothing**. It declares no phases and no participants, because a module
with phases that ran and did nothing is the *"built on paper and dead in the world"* state Part 0
exists to find. It sells and buys on trade credit (11.5), employs (11.6), borrows from a named lender
on its own row (11.7), defaults from its own cash flow (11.8) and is promoted when it outgrows A5
(11.10) — five steps, each naming what it turns on, and the COVERAGE rows say which verb is missing
rather than reading as done.

§42 goes **2 MET to 4 MET, 3 PARTIAL** of 28, and it is no longer an absent sector.

Typecheck 0, lint 0, `check:spec` 222 tags, `check:forbids` 6 over 219 files, `check:deaths` 4 of 4,
`check:existence` green with Part 0 regenerated. Tests written and not run.

## Review 2026-09-15 — the world did not open, and the plan is rewritten

A full adversarial review of `docs/` against the engine, on the owner's instruction: every document
read in full, every claim checked at its file and line, one rig world assembled and stepped once.
The outcome is `docs/IMPLEMENTATION.md`, rewritten from the ground up as the one ordered list.

**What it found, in one line each** (Part 0 of the plan has the evidence):

- `rigWorld()` throws at assembly and has since item 10b (2026-09-14): nine successive blockers
  from four closed items (`shortTermDebt.line` declared `shape` with a death; `smallFirm` with no
  bank choice; `funds.manager` anchored to a phase declared after it; `paper.backstop` at the wrong
  cycle; the small-business seed running before the banks exist; the kernel's single cell-key
  schema refusing a second cell population; a manager party added once per pool; a subordinated
  raise throwing on a name nobody has priced; a fractional staff quantity at period 23). Items
  10b–11.4 were closed on a world that could not be built; their measurements are void.
- With the blockers patched (and the patches discarded), the rig clears one or two of ~340
  sessions a period for six periods and then none; parties grow 44 → 302 in twenty periods
  because cells split every period and `cells.merge` has no caller.
- The policy rate has no writer but its declaration; a negative rate is inexpressible; the OMO desk
  posts market orders to a written holding path; five party kinds are immortal; no household can
  borrow (`housing/index.ts:571`) or fail; no arrears exist; the opening states 22 prices, a yield
  and three shares; three numbering systems and two completion figures disagree; 98 coverage rows
  cite a finding that does not exist; 21 test files name drawn parties.
- Discipline that holds: no bounds outside `core`, no `?? 0`, no kind branches, no module imports,
  all `@spec` tags resolve, the silent FORBIDs hold, typecheck and lint green.

**What changed.** `docs/IMPLEMENTATION.md` is the plan: item **0** (the world opens, with
`check:opens` as a standing guard), **0a** (phase resolution after collection), **0b** (the cell key
on the party kind), **0c** (one truth in the documents), **0d** (the overdue suite run, triaged),
**0e** (Part 3, the optimisation campaign with a resolution ladder as its gate), then the carried
items with **12a** (households borrow, owe and fail; arrears), **18a** (a rate rule, negative rates,
the desk posts a schedule) and **22a** (the opening is not an equilibrium) inserted at their
dependency positions. No engine code was changed by the review.

**Tests run**: `npm run typecheck`, `npm run lint`, `check:spec`, `check:forbids`, `check:deaths`,
`check:existence` (all green); one smoke assembly of the rig (throws; see above); one profile of
the patched rig over 20 periods (Part 3 §3.1). The suite was not run: it cannot be until item 0.

## Review 2026-09-15 (second pass) — every engine file read; the plan rewritten from the source

The owner rejected the first pass as insufficient. This pass read every file under
`packages/engine/src` (187 files, line by line: kernel, registry, seeds, every mechanism, the
observer) and kept a working findings file as it read — 502 numbered findings, each at a file and
line — before rolling them into `docs/IMPLEMENTATION.md`. No engine code was changed.

**What the full read found beyond the first pass** (Part 0 of the plan carries the evidence):

- **Eight more stops the smoke run had not reached**, each a blocker the first time its path runs:
  no transit instrument exists so no cargo has ever loaded (`freight/index.ts:323`); a capital raise
  at reservation 0 is dropped silently by `onTheGrid`; a period-0 overdraft throws in `bookDraws`;
  a tender with a cell seller throws at `asQty`; the insurer delivers policy units it was never
  issued; a loan drawn on 29 February; the deposit-insurance limit is one hundred dollars per
  member, which closes every bank's funding room from period 1; and the cost of capital requires a
  quote that requires a project that requires a cost of capital. The last two together are why
  nothing is ever lent or built.
- **Four systems whose defects are one design decision**, recommended as remakes rather than
  repairs: the cell representation (per-member holdings with split-on-partial-event cannot merge,
  cannot trade cell to cell, cannot age into an existing cell, and contradicts Part XII's one cell
  per key); phase assembly; the bank's credit view (every required yield is one number for every
  name, so every desk's view of every bond is the same curve); the opening.
- **Twelve docstrings that describe mechanisms the code does not contain**, nine facts with two
  writers, six one-sided flows, a rate-quoted mark off by a factor of a hundred in every
  rate-priced derivative, a backstop drawn after the paper has defaulted and drawn as a gift, an
  invoice that is never presented again once missed, land that prints at one cent, rent at 100 %
  of income, and a small-business tier of cells that hold nothing.
- **A per-file inventory of every walk** that is O(parties), O(instruments), O(journal) or
  O(ledger) per period and of every one called per party, per market or per fill (Part 3 §3.1),
  with the index or cache that replaces each.

**What changed.** `docs/IMPLEMENTATION.md` rewritten: seventeen blockers in item 0; **0b′** (the
cell remake) and **12b** (employment as a standing relation) inserted; **17.0′** (the credit-view
remake) as the first step of 17; every item's steps name file, line, change and test; Part 3
carries fifteen layout steps gated on the ladder; Part 4 positions every one of the 502 findings.
The `check:existence` table is unchanged and verified.

**Tests run**: `check:existence`, `check:deaths`, `plan:check`, `check:spec` (green). The suite
was not run: item 0 is what makes it runnable.

## Review 2026-09-15 (third pass) — the architecture verdict, the macro gaps, the large optimisations, and the population lattice

The owner asked for four things the second pass had not delivered: a section-by-section verdict
on the architecture with big-bang changes where the design is the defect; the mechanisms the model
is missing as an ECONOMY, not by clause; the large (non-local) optimisation changes; and an
advanced alternative to the household and small-firm modelling grounded in current
heterogeneous-agent and agent-based literature. All four are in `docs/IMPLEMENTATION.md`:

- **Part 5** — every section of `docs/ARCHITECTURE.md` verified against the code (holds / holds
  wrongly described / holds wrong design / does not hold), and eight big-bang changes: cells hold
  totals (BB-1), phases ordered by declared reads and writes (BB-2), money that carries its
  currency (BB-3), a print that carries its dimension (BB-4), the period index written by the one
  writer of each fact (BB-5), numbers without objects (BB-6), markets that close and instructions
  that span books (BB-7), questions instead of fourteen kernel hooks and stores instead of the
  journal-as-database (BB-8). Items 0a′ and 0f inserted; 16.0, 18.0, 0e.0 named.
- **Part 6** — thirteen macro-level absences with the literature that names each and the item
  that builds it: no source of growth (the world decays by construction), no price level, no
  credit cycle, unemployment not a state, one propensity to consume, no information diffusion, a
  government that buys nothing, a production network that cannot substitute, a demographic stub,
  stub countries, no fixed costs at the firm, no household half of the financial accelerator,
  shocks with no readers. Items 12c (productivity), 12d (observation), 19.0 (the government buys)
  inserted; 22 widened.
- **Part 7** — four non-local optimisations after the local ones: columnar state with integer
  handles, an event-sourced period with incremental readers behind one memo door, parallel order
  generation with deterministic clearing, a tiered journal; the cost model they buy (a period in
  O(parties + instruments + legs)) and the order to take them in.
- **Part 8** — the population lattice: one kernel representation for households and small firms
  as a discretised heterogeneous-agent distribution with integer mass on declared dimensions
  (categorical and banded; bands are RESOLUTION), one live cell per key, movement only by the five
  weight events read against band edges, decisions as threshold rules on kind profiles
  (buffer-stock consumption with liquid/illiquid wealth, search-and-duration labour, collateral-
  read borrowing, life cycle, experience-weighted expectations that observe public prints,
  predictor switching with its killer; for small firms: drawn productivity, retained-earnings
  growth to promotion, owner draws to a household cell, relationship lending, trade-credit chains
  cell to cell, liquidity-constrained entry). Six non-linear multi-order chains stated as
  measurements; two preferences in total (memory, patience); no coefficient, no hazard, no
  stochastic process. Item 10.5 inserted after 0b′, which it subsumes.

No engine code changed. `check:existence`, `check:deaths`, `plan:check`, `check:spec` green.

## Review 2026-09-15 (fourth pass) — the plan reorganised as one tick list

`docs/IMPLEMENTATION.md` rewritten as an ordered, non-repeating list of 198 steps across 28 items
(639 lines, from 1,533): Part 0 the measured state in six compact tables; Part 1 the order; Part 2
the items, each with binding design decisions where a remake is involved and steps that name file,
change and test; Part 3 the finding index. The narrative parts of the third pass (architecture
verdict, macro gaps, large optimisations, the population lattice) are folded into 0.4, 0.5, 0g and
0f respectively. Order fixed: 0 → 0a (phases by reads/writes) → 0b → 0c → 0d → 0e (questions) →
0f (the lattice, subsuming the cell remake) → 0g (fast, after the register's cell half is final) →
sectors. Doc checks green; no engine code changed.

## Item 0 — The world opens

**What.** The assembled world now steps. `check:opens` runs the rig for 30 periods and the
four-country world for 12, asserting only that neither throws, and is the first thing `npm run
check` does. Twenty-three separate stops were removed, each at its cause: nine found by assembling,
eight by reading, six by stepping — every one of the last six hidden behind the one before it.

**Why.** Every mechanism test passed while the assembled world died in period 2. The suite tested
each module against a rig built for it, so nothing ever asked the only question that matters first:
does the thing run. Item 0 asks it, and the answer is now a check rather than a session.

**The stops, and what each fix deleted.** Declarations: `shortTermDebt.line` became a placeholder
naming Corporate Credit C9; `small-business` declared the bank choice its depositor kind owes
(Banks Funding E1) and dropped `seed.foundation` from its requires. Phases: `funds.strike` moved
above the two phases anchored to it, `paper.backstop` before the maturity it funds, `reporting.
publish` after the revaluation it reports (Clearing F1.a), both research phases onto their anchor's
cycle (Money E3). Reads: `WorldReads.requiredOf` reads the keenest yield published for a name and
deleted three private scans of the same journal; `firms/invest.ts` reads its own last quote, then
the sovereign curve, and the recency gate is gone. Arithmetic: staff hours, tender fills and primary
reservations are all on their own grid, and `PrimaryOffer.reservation` is an `Option` so a seller
with no reservation is a market sell rather than an order silently dropped. Writers: a listed fund's
share line has one writer, not two (`share.etf.us already exists`).

**Succession, which was missing.** An agreement row named a party after it had ceased, so the first
fee charged on one was an instruction addressed to nobody (Money E4). Register F2 says every
reference resolves to the estate or the successor, and the register kept that promise for what a
party HELD and nothing for what it OWED. `world/succession.ts` moves every live row at all three
cease sites. WHICH rows move is the kind's own answer: `AgreementKindDecl.binds` is
`'whoeverSucceeds'` for a debt — what an estate divides — or `'aGoingConcern'` for a relationship an
estate cannot perform, so a mandate ends with its pool while the wage it owed passes on. The kernel
branches on nothing (Law 15) and fourteen modules keep no liveness check of their own (Law 4).

**Two prices that were not prices.** A cash investor bid for paper it had issued itself and the
solver refused the crossing fill (Clearing A2); a bank quoted and wrote a loan in a money it issues
none of (Money A1) — a loan is the bank's own money lent into existence, so that instruction had no
paying account. A bank now quotes where it issues, and lending across a currency waits for XI-12.

**Cargo, which had never loaded.** No `good.<x>.transit.<a>.<b>` instrument existed anywhere, so
every voyage stopped at the `has` check and `freight.loaded` had never fired in any run. The freight
seed opens one per portable good per leg, carrying the origin line's terms with `portable: false`.
It still does not load, and now says why: on `us.1 → us.2` the keenest shipper bids 0.029 and the
only carrier asks 11.39. The session record carries the best bid and ask so that is read, not
guessed. The gap is 13i's — only one region has production.

**A silent regression, the second time.** Giving `small-business` `requires: seed.foundation` dragged
the assembly sort and put `freight` behind the foundation seed; the hull block then ran before the
carriers existed and the world opened with no merchant fleet. It threw nothing and was found only by
asking why no leg had a carrier. `land()`'s comment records the same failure once before. The rule
is now stated where it can be broken: a module that needs the seed is DECLARED after it.

**A rule made a check.** `ParamDecl.denominated` was `true` — an amount of something, with no way to
say of what — so a money constant and an hours constant were one declaration. `insuranceLimit` sat
at 100 through every run of this world, a deposit guarantee two orders under the balance it
guaranteed, which left every retail deposit uninsured and inverted Banks Funding E4.a. It is now
`'money' | 'time'`, and `test/params.test.ts` checks every money amount is a positive whole number
of this world's pieces.

**The census.** Rig, 30 periods: 61 parties and 219 markets at the seal, 439 parties by period 30;
180–450 ms a period; 1–7 sessions clear a period against ~500 that report `noDemand`; 7 loans in
period 1, 2 in period 2, 12 in period 3, then almost none. Abroad, 20 periods: 175 parties and 589
markets at the seal, 1,147 by period 20; 0.9–3.0 s a period. Cargo 0 in both.

**What it found and did not fix**, each positioned in `docs/IMPLEMENTATION.md` item 0: the freight
gap and the three legs with no bid (13i); the loan that stops after period 3 (17, 12a); an insurer
with no claims history that cannot quote its first policy (14); households that open with $1.13 to
$22 a member because a bank's assets and its depositor count are drawn independently (22a);
`shareFor`'s unread `unit` parameter (0g); a backstop granted to every solvent name every period
with no decision behind it (17.3).

**The suite.** `npm run check:opens` is green on both worlds, which is item 0's exit. The full suite
is 200 red of 857 and was redder before: on the thirteen files nearest these edits it went from 59
failed / 16 passed to 32 failed / 45 passed, and nothing went from green to red — tests that used to
abort at the first throw now reach their assertions. Triaging the 200 is item 0d, where the
measurement is written; none of them is chased here (Law 11).

**Deviations from the step list, and why.** 0.3 is two mechanisms and not one — collapsing the rate
and the trouble would delete Banks Funding B1.a. 0.5's `cycle: 2` contradicts `before:
corporateActions`. 0.8b's journal line is impossible from a venue participant. 0.11 belongs to
`freight`, not `goods`: the legs are freight's knowledge and a module never imports another. 0.12's
stop does not exist — an `asset` leg from the issuer already expands to an issuance — and the test
the step asked for proves it. 0.15's "within two orders of the opening deposit" is not a true rule
of this world, and why is the finding above. 0.16's third fallback would read another party's
private state (Observer A4).

## Item 0a — A phase says what it needs of the period it is in

**What.** `PhaseDecl` gains `reads` and `writes`, and loses `cycle`. Every phase in this world — 84
module phases and the 3 kernel ones — declares what it needs of the period it runs in and what it
puts into one. The seal refuses a phase in front of a `thisPeriod` read's writer, and refuses a
`thisPeriod` read nothing in this world writes (`world/order.ts`). At run time a read the phase did
not declare throws `Forbidden 'Clearing F1.a'` at the site. The settlement cycle is the anchor's and
a module no longer states one.

**Measured before it was designed.** Both worlds were stepped with every journal, price and store
read attributed to the running phase — 30 periods of the rig, 12 of `abroad`, 68 phases, 33 event
kinds. Four things the measurement settled, three of them against this item's own premise:

- **A print has one writer.** `runOne` is called from the kernel `markets` phase and nowhere else,
  so a price read is one edge and there is no family to name. The plan's `{ kind: 'print', family }`
  was a guess the source answers.
- **A read carries its period.** Eight phases read the very kind they write — a bank's last deposit
  rate, a fund's last strike, a desk's last estimate — and every one is a read of history. Blind to
  the period, each reads as a cycle. `anyPeriod` orders nothing; `thisPeriod` is the only edge.
- **A store carries no ordering.** `banks/book`, `banks/banks.couponsPaid` and `money-market/market`
  are each touched from nine or ten phases belonging to other modules, because a kernel hook — the
  overdraft credit decision, the money market's resolution — re-enters the owning module from
  whatever phase triggered it. Store edges would order `treasury.receipts` against `lending.write`
  because both make a payment. `{ kind: 'store', noun }` is dropped; the re-entrancy is 0e's.
- **Almost nothing reads the period it is in.** Of 84 phases, NINE. And the order this world already
  had satisfied all nine.

**So the anchor stays, and this item's premise is overturned.** "Order is a function of declared
reads and writes only" cannot hold: `goods.spoilage` runs after the period's trades and before the
marking — E4's own words — and no read or write says so, because it reads holdings and writes
holdings exactly as `markets`, `firms.produce` and forty others do. A holdings dependency makes
every pair of them mutually dependent and the graph is one cycle. The three kernel acts are
world-wide moments and where a module sits against them is a fact only that module has. What this
item delivers is therefore a GUARD, not a re-ordering — and a guard is what two of item 0's stops
needed and neither had: a phase anchored to one declared below it, and a phase running before the
maturity it was meant to fund. Both are a phase in front of something it needs; in both cases what
said so was a run that failed three phases later.

**A false paragraph deleted.** ARCHITECTURE 4.8 said a phase reading a not-yet-produced print gets
`NotYetProduced` rather than a stale value. It never did: `lastOf` answers with last period's event
and `latest` with last period's price. That falseness IS stop 18 — `reporting.publish` published a
company's worth from the week before and nothing complained. 4.8 now says what is true and says
where the check is.

**Declarations.** 65 phases from the measurement; 10 that have never run in any period of either
world declared from their module's source and marked as such, so they narrow the first time the
check can say which reads they actually wanted; 9 that read nothing. The kernel's three declare the
one journal kind that reaches them through a hook (`credit.default`, read as history).

**Deleted.** `environment`'s `written === period` guard and the store field behind it: a phase runs
once a period by construction (`addPhase` refuses a name twice, `step` walks the list once), so the
guard never fired — a check on something arithmetic is the symptom patch Law 12 names. The "order
matters at one anchor" comment in `seeds/foundation.ts` STAYS: the three phases it names have no
dataflow between them and declaration order is what puts them in order, so the comment is the only
thing that says why.

**Found and not fixed.** `plan:progress` counts only items with a row in `docs/WORKLIST.md`, and
items 0a to 24 have none — so "plan completion" measures item 0 and the closed rows, and ticking
every step of this item moved it by nothing. Positioned at **0c**.

**Checks.** `check:opens` green on both worlds with the runtime check live; lint, typecheck, spec
citations, forbids, deaths, existence green. `test/phases.test.ts` is seven new tests: the cycle is
the anchor's, a late reader is named with both positions, a need nothing writes is refused, a
history read stands anywhere, a price read before the session is refused, an undeclared read throws
at the site, a declared one does not.

## Item 0b — The cell key belongs to the party kind

**What.** `PartyKindProfile.cellKey` says what stratifies a population of that kind.
`RegistryData.cellKey` and `Registry.cellKey` are deleted. `cellKeyFaults`, `rekey` and
`Parties.sameKey` read the kind; `sameKey` across two kinds is false. The small-business seed is on,
and this world now holds two cell populations: 12 household cells standing for 40,000 people, and
144 small-firm cells.

**Why.** One list of key dimensions for the whole world made two populations mutually exclusive. A
household has no line of business and a small firm has no cohort, so `cellKeyFaults` refused a
household for carrying no `line` and a small firm for carrying one — both ways at once, whichever
kind was declared second. Item 0's sixth stop was every cell the small-business seed tried to add,
and the seed was turned off to get the world open. It is on again and nothing about a household
changed.

**What each kind says.** A household is `region, cohort, bank`: people age, so a cell whose members
were in two cohorts could not be aged as one; and a deposit is a claim on a NAMED issuer, so two
cells at two banks hold two different instruments and merging them would net a claim on one bank
against a claim on another. A small firm is `region, bank, line` and has no cohort, because a firm
has no age at which it retires. `CellKeyDimension` gains `line`, whose terms are empty — a line is a
good's sub-unit and the registry has nothing to check it against, which is the honest statement of a
key that carries a fact about the members rather than a reference.

**And two kinds never merge.** `sameKey` answers false across kinds before it looks at a dimension.
A household and a small firm in one region banking at one bank are not the same people, and a merge
would be a weight that counts two things.

**Declared after the seed, not requiring it.** The sector's cells are keyed on a bank by name, and
the banks are parties the foundation seed makes. Requiring `seed.foundation` is what dragged the
assembly sort in item 0 and lost this world its merchant fleet, so `smallBusiness` is declared after
`foundationSeedFor` exactly as `land()` is — which is the rule that comment has carried since the
first time it happened.

**Found and not fixed.** The 144 small-firm cells each have weight 1 and share keys three at a time:
`sb.bank.a.power.0`, `.1` and `.2` are one key. A cell of one firm is a named party with a count
stapled to it, and three cells on one key are a population cut by nothing — `drawSmallBusiness` cuts
the sector before it knows what a key is. Positioned at **0f** (one live cell per key).

**Checks.** `check:opens` green on both worlds; `test/small-business.test.ts` green, all six. On the
three suite files nearest this change: 13 failed / 23 passed before, 12 failed / 26 passed after —
one fixed, two added, none broken. Lint, typecheck, spec citations, forbids, deaths, existence and
plan green.

## Item 0c — One truth in the documents; the guards that bite

**What.** The plan counts itself; the worklist says it is history; the coverage file stops claiming
a measurement nobody took; two ratchets now refuse a new rounding call and a new floor at zero; and
five docstrings that described something not there say what is.

**The counter read the wrong file.** `plan:progress` walked the rows of `docs/WORKLIST.md`, and
items 0a to 24 have no row there — they live in the plan alone — so every step of every one of them
was invisible, which is how the defect was found: ticking all five steps of item 0a moved the figure
by nothing. It counts the plan's sections now, in the order they are written, and reads the worklist
for the one thing it is the writer of: which items it worked and closed, so a closed item's steps
count after its section was deleted.

**And the two files used one id for two items.** The worklist's `14` was the polity; the plan's `14`
is the insurers. `15`, `16` and `17` differed the same way. The plan's ids win — it is the ordered
list — and each superseded row points at the item that carries it (`13k` → 20, `13m` → 15, `13n` →
12, `13o` → 17b, `14` → 19, `15` → 22, `16` → 23, `17` → 24). Three tests hold it: the plan's items
are counted, a closed worklist item keeps its steps, and no plan item ever takes its state from a
moved row of the same id.

**`NEVER REACHED` became `UNMEASURED`**, in all 96 rows and in the tool that counts them. The old
mark asserted a fact nobody had measured: what was known is that the source is there. The
measurement now exists — `npm run coverage:reached` steps both scale models and asks the kernel's
own `Reach` register, which declares every capability at assembly so "never" is a state rather than
a silence — and item 0d is where it is taken and the marks are settled. It lives beside the rig
rather than in `tools/`, because `tools/` compiles as its own project and cannot import the engine.

**Ten record entries carry a warning.** Item 0 stepped the assembled world for the first time and it
stopped in period 2; entries 9, 11, 13a, 13b, 13c, 13d, 13e, 13f, 13g and 13h closed on that world
or on a mechanism Part 0 found unreachable. Their measurements are not evidence. The entries stand,
because a ledger is not rewritten backwards (Law 13), and each says so at the top.

**Two ratchets.** `Math.floor`, `ceil`, `round`, `abs`, `exp`, `pow`, `sqrt`, `min` and `max`
outside `core/` — 125 calls in 53 files — and `atLeast(x, 0)` / `atMost(x, NO_QTY)` outside
`core/num.ts` — 7 in 5 files. `atLeast(x, 0)` is "not less than zero", the exact phrase Law 6
forbids, and it is harder to see than a `Math.max` because the honest uses of both live in `core/`.
Neither is banned, because deleting them is item 21 one file at a time; the check fails when a file
has MORE than it had, or when a clean file acquires one. A single added `Math.floor` fails it by
name.

**A prose regex, tuned twice, was the tell.** Step 0c.7 said to widen `params.ts`'s scheduled-death
match from `worklist [0-9]` to `item [0-9]`, because the plan is `docs/IMPLEMENTATION.md` now.
Widened, it fired on two numbers that STAY: an overhead per machine ("...an overhead rather than an
input (item 7b)") and `goods.power.spoilage` ("Goods A3, E4, item 11: NOT DECLARED"). This codebase
cites an item wherever a number came from. A second round of tuning — exempting a citation in
parentheses — caught the first and not the second, and that was the signal (Law 12): the rule was
never about the word. It matches the CLAIM now — an item BUILDS the mechanism, the number is
DELETED when it lands — and six cases are asserted, three deaths and three citations.

**Five stale docstrings.** The freight header said a gale "takes a share of the hulls caught in it
WITH THE CARGO ABOARD on two named books": nothing in this world emits `act: 'lose'`, so a storm
delays a voyage and has never sunk one. `ledger/instruction.ts` said a `create` may only appear
beside the `destroy` legs of what it was made from, which is not the rule and could not be — a thing
drawn from labour and land alone destroys nothing. `kinds.ts` and `module.ts` each named the other's
moment for its refusal. `households` was waiting for worklist 9 to price risk, and 9 closed. The
other seven the review named are accurate, or were made accurate by items 0 to 0b: `goods/data.ts`'s
"exactly as a tonne in transit is" became true when 0.11 opened the transit lines.

**Found and not fixed.** A `create` leg is refused unless the instruction's cause is `production` or
`seed`, and BOTH of freight's are not: `cargo()` loads under `cause: 'trade'` and `arrive()` lands
under `cause: 'corporateAction'`. Neither has ever run, because no cargo has loaded in any period of
either world — so the first voyage this world manages will throw. Positioned at **13i**, with the
rest of the freight gap. The 125 rounding calls and 7 zero-floors are positioned at **21**.

**Three PARTIAL rows named nothing**, which is a MISSING with a softer word, and
`tools/test/spec-coverage.test.ts` had been red on them since before item 0. `Short-Term Debt C3`
(every buyer sharing one concentration and one memory) is item 17; `Prime Brokerage A2` (custody
distinct from banking) and `A4` (stock-borrow fees, commissions) are 17b.

**Checks.** `check:opens`, `check:existence --verify`, `plan:check`, `check:deaths`,
`check:forbids`, `check:spec`, lint and typecheck green. The plan reads 10 of 28 items closed.
`tools/test/` is green, all 24 — it had one red before this item.

## Item 0d — The overdue suite run, triaged

**What.** The suite was run once and every red file placed. 106 files, 869 tests, **199 failed, 670
passed**. Four assertions wrong on their own terms are fixed here; a build-stopper found by the
measurement is fixed here because the measurement could not be taken without it; everything else is
positioned.

**The attribution is the AUDIT's, not the assertion's.** Most of the 199 are a test asserting the
audit is clean and finding it is not, with the diff truncated — so the reds were attributed by
running the two worlds and reading the audit by family. The rig at period 30 reports 333: 299
`units` (two cells standing for one household), 23 `flows` (a cell moved N per member and the
instructions sum to something else), 8 `prices`, 3 `names`. `abroad` at period 12 reports 7,877, and
**7,185 of them are one sentence**: the Fed carries a euro bond and no revaluation has ever looked
at it (Currency D2). That single defect is more than nine tenths of everything wrong in the
four-country world.

**Where they went.** Duplicate cells → **0f** (19 files, and the largest count in the rig). The
prices family and `Valuation.atCost` giving two answers to one question → **21**. The Fed's
unrevalued euro bond → **16**. A rig that no longer draws what three tests ask it for, and a
15-phase assertion against a 33-phase world → **23.1**. Four build-stoppers with their sites → three
to **21**, one to **22a**.

**The recursion is worth naming.** `worthToItsLender` asks what a loan is worth; it reads the
borrower's exposure; that reads a MARK; the mark asks the kind's valuer; the valuer is
`worthToItsLender`. XI-6 says exactly one module answers what a lot with no market is worth, and
this one answers with itself. The stack overflows in five tests.

**The twenty-fourth stop, found by the measurement and fixed here.** `coverage:reached` over 52
periods stopped at period 47: `payDividend` paid FROM `action.issuer` and TO the row's creditor,
neither resolved through successors, and `firm.2` had ceased that period. A declaration names its
holders on the record date and is paid on the payable date, and either end can cease in between. The
kernel moves an agreement row at the cease (item 0's `world/succession.ts`), so a row that existed
then is already on the estate; the ISSUER, read from the corporate action rather than from the row,
was not. Both ends resolve now (Register F2), the loop matches the RESOLVED issuer so an estate pays
what the firm declared — it matched the dead name before, so a ceased company simply stopped paying
— and a claim whose two ends resolve to one party is not paid and stays, which is the estate's to
divide.

**What has never produced anything, measured.** 52 periods of both worlds: the rig declares 600
capabilities and reaches 193; `abroad` declares 1,826 and reaches 229. Almost all of the difference
is the seed's markets, 1,661 of 1,702, which is honest — a market is opened for every good in every
region and one region has firms in it. What is left is the list that matters: **nine derivative
classes of nine have never produced a contract** (Part 0 said eight), no corporate bond has ever
been issued, no insurance policy ever written, and two of the four currency-pair participants have
never posted. The `UNMEASURED` marks in `docs/COVERAGE.md` **stand**, and they are evidence now
rather than a reading of the source.

**Twelve periods understate it.** Four capabilities produce between period 12 and period 52 —
a bank as a borrower, a fund and its manager, a household in a currency pair — so the long run is
the honest measure and the short one is not.

**Also fixed.** 0c's rounding baseline counted a generated `.d.ts` beside the source it came from;
the ratchet skips generated output and the baseline is 52 files.

**Checks.** `check:opens` green on both worlds, and the rig now runs 52 periods without throwing.
Lint, typecheck, spec citations, forbids, deaths, existence and plan green. On the five files
nearest this change: 31 failed / 48 passed before, 28 failed / 51 passed after.

## Item 0e — Questions, not hooks

**What.** The eleven questions this kernel asks and cannot answer are DATA
(`registry/questions.ts`), held in one register with one door and one refusal. Eleven maps, eleven
`provide*` methods and eleven hand-written refusals are gone from `world/world.ts`. A cross-module
ratchet in `check:forbids` stops the event coupling growing, and the first of it is removed: every
borrower now publishes what it is short of through one door.

**Fourteen, measured, are eleven.** `venueParticipants`, `indices` and `derivativeClasses` are
REGISTRIES and not questions: a question has one answer per kind and its absence is a state the
seal can refuse; a registry has as many entries as modules put in it, and an empty one is emptiness.
Saying so is the difference between a check that means something and a check that fires on nothing.

**What the duplication cost was not the lines.** `requireCreditDeciders` checked that ONE of the
eleven was answered where a registered kind needed it — the overdraft — and the other ten could be
silently unanswered until something asked mid-period. `requireBankChoices` checked a second, and
only against the module that DECLARED the kind. `refuseUnanswered` checks all eleven against every
kind the registry holds, and it found something at once.

**A bank has a deposit class and does not shop.** `depositClass` says what a party's balance is to
the bank holding it, and a bank's is wholesale money (A1.c). It does not follow that a bank chooses
where to bank: its own account is at its central bank because that is what settling in central-bank
money IS (Money C2.a). The predicate is a depositor that is not itself an issuer. **The per-module
check could not have found this**, because nothing declares a bank and a bank's banking together.

**The coupling is 52 pairs over 31 event kinds.** "A module never imports another module" is checked
by lint, and the journal is the hole it leaves: `banks` read `firms.funding` and `housing.funding`
BY NAME, so a third borrower had to be added to that list by hand and none was — the small-business
sector published nothing a bank would look at and got no credit at all (BK4). `check:forbids` now
baselines every pair and fails on a new one.

**One kind for what a borrower is short of.** `ctx.request(borrower, { ccy, short, security })` is
the one door, `ctx.requests(at)` the one read, and the kernel stamps the party and the period so
nothing else can say either. A firm and a landlord publish the same shape; a bank reads one thing.
`firms.funding` stays and carries what only a firm's own readers want — the split between what falls
due soon and what it wants to build — which is a different fact with a different audience.

**Item 0e′ inserted after this one and before 0f**, carrying what is left: the 51 remaining pairs,
the eight nouns the journal is standing in for, and the observer's module imports. It is before 0f
because the lattice declares stores and what a store IS has to be settled first. `registry/wages.ts`
and `registry/physical.ts rentedRoom` already exist and are the right shape — each is a pure
function over an event the CALLER fetched, so the module still names the kind, and moving the fetch
inside on the `registry/switching.ts` pattern is 0e′.1.

**Checks.** `check:opens` green on both worlds; `test/questions.test.ts` is five new tests; on the
seven files nearest this change nothing went from green to red. Lint, typecheck, spec citations,
forbids, deaths, existence and plan green.

## Item 0e′.1 — The registry does its own fetching

**The shape that was wrong.** `registry/wages.ts` and `registry/physical.ts rentedRoom` were built
to end a formula written twice, and they did — but each took the EVENT as an argument, so every
caller still had to write `lastOwnSince('labour.wages', …)` or `lastOwn('commodities.leased')` to
produce it. The formula moved and the NAME did not, which is half a fix: eleven mechanism sites in
six modules still reached for another module's event kind, and each one still decided for itself
what `hours`, `due`, `paid` or `space` meant when it came back. The registry now names the kind and
does the fetch; the caller passes a party, or a party's door.

**What one read replaced.** `ownPayroll(reads, at)` is the extraction five sites were doing by hand
(`banks/staff.ts` twice, `firms/index.ts`, `firms/decide.ts` twice). `ownPayrollOf`,
`wageFacingParty`, `wagePrintedIn`, `payrollSettledIn` and `hasEverMetAPayroll` are the same reads
through the door a PHASE has, which is a journal and not a view; `firms/produce.ts`,
`research/index.ts`, `control/index.ts` and `treasury/index.ts` take them. `rentedRoom` takes the
party. `hoursPaidFor` had no caller and is deleted.

**A third writer of the wage formula, found by moving the fetch.** `treasury/index.ts` held
`currentPayroll` (which was `payrollSince` inlined), its own copy of the own-bill-over-own-hours
calculation, and a fallback of its own. That fallback took the LAST `labour.print` ANYWHERE, so a
state read another country's wage whenever that country printed later — a wage in the wrong money as
often as not. The registry's fallback is the party's own REGION, which is where an hour is hired.
That is a behaviour change and it is the point of the read: the file header says this module's rules
have caught a second copy three times, and this is the fourth.

**Checks.** `check:opens` green on both worlds. `check:forbids` cross-module baseline 51 → 44, the
seven closed rows deleted (`banks|control|firms|treasury:labour.wages`,
`research|treasury:labour.print`, `firms:commodities.leased`). Lint and typecheck green;
`output-kind.test.ts` moved to the new `rentedRoom` door and its seven tests pass.

## Item 0e′.2 — What a bank publishes about itself, read in one place

**`registry/banking.ts`**, on the shape 0e′.1 settled: the kind names live in the registry and the
registry does the fetch. A bank computes its own economics once and publishes them under its own
name — what a money costs it, what capital it has spare and what it will have out to one name, what
it keeps back, the board it is offering, what it requires of a name, the lines its desk quotes, what
it can pay with. Eleven other modules price against those facts and none of them may import the
module that writes them, so each reached for the EVENT and unpacked the fields itself.

**Two more second copies, found the same way.** `bank.costOfFunds`'s `alsoIn` walk was written in
`securitisation` and again in `fx-derivatives`. `bank.reservation`'s `required` map was unpacked in
`banks/treasury.ts` and again in `money-market/collateral.ts` — one bank's own credit view read by
two formulas. And `quotedTo` (the keenest quote a name was given, and how much that bank will lend)
was the SAME FUNCTION UNDER THE SAME NAME in `equity/float.ts` and `corporate-bond/index.ts`. Three
finds in one pass, and none of them was being looked for: they surface because moving a fetch makes
you put the two callers side by side.

**Twenty-two pairs closed.** `bank.buffer` ×2, `bank.capital`, `bank.costOfFunds` ×2, `bank.dealing`,
`bank.depositRate` ×2, `bank.liquidity`, `bank.reservation`, `deposit.classes`,
`centralBank.corridor` ×2, `moneyMarket.print` ×2, `moneyMarket.refused`, `credit.quoted` ×5,
`credit.default`. With `households:labour.goingRate` (0e′.1's family, closed here by extracting
`goingRateIn` — a third copy of the venue-keyed lookup) that is 43 → 21.

**Checks.** `check:opens` green on both worlds; lint, typecheck and `check:spec` green.
`money-market`, `short-term-debt` and `securitisation` were run against a worktree at the parent
commit and carry the same 18 reds and 22 greens before and after, so nothing there went from green
to red. The suite itself runs at the end of the module (Law 11), not here.

## Item 0e′.3 — The cross-module baseline is empty

**The step's premise was wrong and the measurement said so.** It read "the rest are questions, not
reads", meaning `registry/questions.ts` — a module SUPPLIES behaviour the kernel asks for. Not one
of the twenty-one remaining pairs was that. Every one was a READ of a fact somebody published: what
a firm is short of, what a pool struck at, what a benchmark fixed at, what an assessor graded, what
a broker called. A question would have been a mechanism where a read belongs, which is the shape
Law 2 calls an invented primitive. Two more registry files, on 0e′.1's pattern.

**`registry/funding.ts`** — what a party published it NEEDS and what a pool published it is WORTH.
A firm publishes ONE gap and four channels fund it (equity float, bond, paper, and a control bidder
reading whether it is forced), which is the arrangement that once had a bond and three-month paper
both brought against one published `short` and the firm raising twice what it needed. The split is
now typed where it is read: `shortNow` is weeks, `shortTerm` is the plant. Nine pairs.

**`registry/notices.ts`** — public announcements about a named party: an overnight fixing, an
auction's dealership share, an offer of paper, a rating action, an estate closing, an advisory
quote, a margin call. Small, and they do not resemble each other; what they have in common is that
they are public and about a name, and that is all a reader needs. Twelve pairs.

**52 → 0.** The ratchet in `tools/check-forbids.ts` is now an empty set, so it is a plain FORBID:
a mechanism that names another module's event kind fails the build. Its header says so; nothing is
added back, because the registry read is the fix. Four passes closed it — `wages`, `banking`,
`funding`, `notices` — and each pass found a second copy of a formula nobody was looking for.

**Checks.** `check:opens` green on both worlds; lint, typecheck, `check:spec` and `check:forbids`
green. The suite runs at the end of the module (Law 11).

## Item 0e′.4a — A plan is a store, and the kernel has a door for one

**The door, built first.** A participant is handed a `ParticipantView` and nothing else — no
`MechanismContext`, so no `ctx.state` — so the only place a decide phase could leave something for
its own `markets` and `orders` to pick up was the journal. `firms.plan` and `households.plan` are
both recorded PRIVATE and were read back by their own writer in the same period: a store wearing a
log's clothes, undeclared, invisible to `registry/nouns.ts` and to the phase-order check.

`ParticipantView.working(name, initial)` and `MechanismContext.workingOf(party, name, initial)` are
the two ends of one store, `Map<PartyId, T>` under (owner, name). The owner is resolved the way
`declared` resolves a phase's: from the participant the kernel is currently evaluating, or failing
that from the phase that is running — a module's own phase handing its own party's view to its own
store is the same module either way. Outside both there is no owner and it throws. A participant
gets its OWN party's entry and never the map, so a firm cannot read another firm's plan: private by
construction, which is the property `blindView` already has for prices.

**Four parsers deleted.** `marketsIn` and `ordersFrom` existed twice — once in `firms/decide.ts`,
once in `households/index.ts` — and each took `orders` back out of the event as `unknown[]`,
re-checked every field, and DROPPED any order that did not survive the round trip. The orders never
leave the type system now. The households pair moved the `asQty` door from the read to the WRITE,
where the module knows what it decided; at the read it was checking a number that had already been
through `unknown`, which is where a size that was not a count went missing quietly.

**The events stay.** Both are the record of a decision and the tests read them; what changed is
that nothing reads them back to act on. That is the step's rule: written from the store, once, and
never read back by its writer.

**Checks.** `check:opens` green on both worlds; lint, typecheck, `check:spec`, `check:forbids`
green. Same-period read-backs 25 → 20; 0e′.4b–4d carry the rest.

## Item 0e′.4b–4d — Nine stores, and a count that was wrong twice

**The step said eight. The first measurement said 25. It was 11.** The step's list came from five
findings; a classifier over the whole tree found 25 modules reading back an event they wrote in the
same period, and working through them showed the classifier counting two things that are not
defects. A module reading back an event of a period STRICTLY BEFORE this one is remembering a
public fact — `bank.capitalPlan`, the buffer's own memory window, `credit.quoted`,
`cds.index.settled` — which is Law 19 working. An AUDIT FAMILY reading the public record is what an
audit IS: `tradingBookIsCapitalised` and `securitisation C6` cross-check one module's publications
against another's on purpose, and handing them the module's own store would be checking a derivation
against itself and passing whatever it did.

**Converted (with 4a's two, nine stores):** `firms.plan`, `households.plan`, `fx.arbitrage`,
`equity.plan`, `fund.struck`, `bank.lines`, `bank.buffer`, `treasury.programme`, `margin.call`.
Each is now a declared `working` noun, each event is written FROM the store and never read back by
its writer, and four `unknown[]` order parsers are deleted. `bank.lines`'s `roomFor` also stops
walking every row of its bank's allotment to find its own line.

**The door grew where the measurement pushed it.** `view.working` first resolved its owner from the
participant being evaluated. `runLine` broke that — a module's own phase handing its own party's
view to its own store — so the owner also resolves from the running phase. Then `whatItMustBorrow`
broke it again: a QUESTION is answered by a named module through a view, and the kernel knew the
owner and was not saying. All eight answer sites now run as their answering module. That is the
same fact three times: the owner of a read is whoever the kernel asked, and the kernel always knows.

**What is left and is not a defect:** `securitisation` ×2 (audit) and `banks:credit.written` —
`publishStandard` aggregates the period's own writes into a public count and volume, a read of
events that already happened, causing nothing (Observer A5). A log read as a log.

**Counts.** `working` nouns 9 → 18; homeless unchanged at 14, because none of these is a kernel
noun — a plan nobody has acted on is not a thing the world has. `ARCHITECTURE.md` and `CLAUDE.md`
both carried "14 of 19" and both are corrected.

**Checks.** `check:opens` green on both worlds; lint, typecheck, `check:spec`, `check:forbids`
green.

## Item 0e′.5–.6 — The observer imports no module; the item closes

**Four imports, three doors.** `observer.ts` reached into four modules: two data tables and two
functions. The tables (`OCCUPATIONS`, `OCCUPATION_OF`) are DATA and live in `registry/occupations.ts`
now (Law 15); the labour and firms modules read the same rows from there. The functions are answers
to two world-scoped questions — `whatTheConsensusIs`, `whatIsHedged` — declared by the research and
fx-derivatives modules through `SystemModule.measures` and registered by the one assembly walk that
registers every other answer. Their result types moved beside the questions, so the observer names
the shape of what it is shown without naming who computes it. A world assembled without either
module shows no consensus and no hedges, which is the truthful surface rather than a zero.

**The rule is a check.** `phoenix/no-cross-module-import` now covers `src/observer/`; it was shown
to fire on a module import there and to be silent on the clean file. The first widening caught
`seeds/foundation.ts`, which the rule had always exempted because a seed assembles modules and sits
directly under `seeds/`; the exemption is restated in the rule with its reason.

**The item closes.** `check:forbids` reports no module reading another's event by name (52 → 0) and
no module reading back a same-period event of its own that is not an audit or an aggregate of what
already happened. ARCHITECTURE 4.9b carries the three-door rule and the observer's place under it;
4.10a carries the journal-is-a-log rule and the working-store door. Of the nine findings the step
named, five closed here and four are WITHDRAWN on measurement — CD5, RP10, IN2, MM13 — each with
its reason in the Part 3 index; a finding leaves the file only by being placed, and "not a defect,
and here is the measurement" is a placement.

**Checks.** `check:opens` green on both worlds; lint, typecheck, `check:spec`, `check:forbids`
green; `plan:progress` recounted. The full suite was not run for this item — the owner's rule is
that it runs at the end of a module, and the four sector items 0f–0g and 11 onward are what this
block was clearing the ground for.

## Item 0f.1 — A cell holds totals

**One representation change, not three steps.** 0f.1 as written was the register alone. It could
not open the world alone: the moment a lot is a total, settlement writing a leg's per-member side
into it is wrong, and every read that multiplied a holding by `weightOf` double-counts. So this is
the register on totals, settlement handing the register the leg's total (the cell side stays on the
leg as a statement until 0f.2 deletes it), the read-side half of 0f.5, and the balance sheet as a
total on both sides — and nothing else. The decision × weight sites (a per-member demand ladder
becoming a total order, a bequest per heir × heirs) are correct and untouched: a decision is per
member and an order is a total.

**The register.** `quantity` is the party's total; `perMember` is a read of it through the door that
refuses a cell of nobody; `totalQuantity` is deleted (ten source sites, eight test sites).
`copyMemberState` is `moveShare`: a split or promotion of `members` of `weight` people moves
`floor(qty × members / weight)` pieces of every lot and lien and the remainder stays with the people
who stayed — arithmetic, not a bound. `forget` is `merge`: lots and liens concatenate, money walks
add, and the per-member equity and revaluation accounts take the weighted mean of the two, the one
division a merge costs, its dust on the walk. `sameState` is deleted with the representation.
`validateCellSide` and the per-member divisibility check are deleted here, not at 0f.2: 15,169,244
pieces among 3 people is not a whole number each, and a total that does not divide is arithmetic.

**Where the double-counting hid, found by opening the world.** Each stop was a site that read a
holding and multiplied by a weight, and each was found by the world refusing to run: a bank's
resolution moving every cell's deposits × weight (22.7 trillion out of a 5-billion account, period
1); the audit's `money` family scaling every ledger delta by the weight it was struck at; the
balance sheet scaling holder-side liabilities by the holder's weight and the consolidation by each
member's; deposit interest on a total balance handed to `shareFor`, which multiplies by the weight —
interest on the whole cell, once per member, compounding to 10¹⁶ by period 12; a fund's pass-through
and a firm's dividend read as per member; plant scrapping, storm loss, plant building, perishing and
a mortgage pledge each moving `free × weight`. Nine `cellSideOf` sites derive a leg's side from the
total; 0f.2 deletes the side and the helper together.

**Households decide per member and read per member.** `ParticipantView.perMember(instrument)` and
`cashPerMember(ccy)` are the two doors; the spending decision, the spare, and the fund position
read through them, because everything they are compared with is per member (A2.e, A2.f).

**Findings.** `funds/index.ts` writes the register directly at two sites, which the module contract
forbids — positioned under 0f.2. `funds.strike` read `investor.wealth` undeclared and never reached
it until cells held their real totals; declared. The block's full suite, run once at its close,
measured **200 failed / 674 passed of 874** against 0d's 199/670 of 869: five tests added, four
green, one red not attributed — the failing list is saved with this item and the difference is
inside the same forty-seven files 0d placed.

**Checks.** `check:opens` green on both worlds — the rig thirty periods, the four-country world
twelve — on totals. Lint, typecheck, `check:spec`, `check:forbids` green.

## Item 0f.2 — A leg moves a total

**The cell side is gone from the leg.** `CellSide`, `fromCell`, `toCell`, `pledgorCell`, the delta's
`weight`, `validateCellSide`, `cellSide`, `cellSideOf`, `optionalCell`, `cellPays`, `commonGrain`
and the three grain checks settlement made on a leg's per-member side are deleted — 52 files. A leg
on a cell is denominated like a leg on anybody: the register holds the cell's total, the leg moves a
total, and what one member's share is is a read. `pairFills` steps on the unit's own piece: a fill
between two populations was struck on the least common multiple of their two weights so that every
member held a whole piece, and with totals in the register there is no such member to protect.

**What survives, and why it is not the same thing.** A per-member RULE over a cell — a transfer per
person, a tax per person, a wage per person — is a number stated per member and paid to every
member, and the total a cell moves is that on the grid for one, times the count. That is arithmetic
on a count and it is one door, `gridPerMember`/`totalOverMembers` in `parties/party.ts`, beside the
weight it multiplies by. `splitOnTick` stays for the same reason from the other side: it splits a
TOTAL across named claims — an estate's creditors, a seed's banks — and the parts sum to what there
was.

**Reads of a leg's side became reads of the leg.** `capital-programme` counted what a cell bought
per member; `expectations` recorded what a MEMBER received off the leg's side — it is the leg's
amount over the people now (A2.f), so that a decision is still never taken on the whole cell's
receipt. A coupon is owed on the holding and struck once on the money's grid; a redemption moves
the total; the fx and asset trade builders put cash on the money's own grain.

**Checks.** `check:opens` green on both worlds; lint, typecheck, `check:spec`, `check:forbids`
green. Exit measured: `commonGrain`, `shareFor`, `sameState` gone from `src` (three comments name
them as what was deleted).

## Item 0f.3 — The lattice

**Stratification is data.** `registry/lattice.ts` declares what makes one cell a different cell:
categorical dimensions each owned by the one event that moves them, and banded dimensions read per
member against RESOLUTION edges, one parameter per edge with its `why`. `PartyKindProfile.cellKey`
(0b) — four names in a closed union — is `lattice`, the same idea with the quantities and the events
that make a dimension a dimension. The kernel checks the three dimensions it can check against
something it declares and takes every other as the kind's own fact; a key's dimension names are
strings, not a union the kernel has to grow.

**Households** live on region, bank, cohort, employment (moved by a hire), credit record (moved by
a default), and bands on liquid wealth in weeks of expected income (Deaton, Carroll), illiquid wealth
(Kaplan–Violante), tenure in dwellings per member (Mian–Sufi), and spell length since the last
separation (Kroft–Lange–Notowidigdo). **Small firms** live on region, bank, line, age, and bands on
size and leverage (Hopenhayn, Axtell, Stiglitz–Weiss). Every edge is a resolution and 0f.10's test
is what makes that claim true or false.

**Placed at the seal, read, not assumed.** The seed supplies what it knows; `placeCellsOnLattice`
reads the rest: employment off whether a hire was ever recorded, credit off whether a default was,
each band off the register and the edges. A band whose quantity cannot be read — weeks of expected
income before anybody has an outlook — is `unread`, which is a real state. `Parties.place` is the one
write of a key outside the five weight events and it runs before the world has moved. A cell
`add`ed later — a split, a promotion — carries the whole key and is checked on all of it.

**Checks.** `check:opens` green on both worlds; `test/lattice.test.ts` (three tests) green; lint,
typecheck, `check:spec`, `check:forbids` green.

## Item 0f.4 — Movement is the five events, and a key has one cell

**There is no split.** A cell holds totals and its people are alike on every dimension of its key,
so the only reason part of a cell ever left was that part of it moved to another key — a hire, a
separation, a death — and that is `reKey`. `CellEvents.split` and `splitCell` are deleted; the
three callers re-key: the hired onto `employment: employed`, the separated onto `unemployed`, the
dying onto a probate cell of their key. A probate cell is a cell of the dead and not an age, so
`estate: living | probate` is a categorical dimension of the household lattice, opened as `living`
and moved by death — the "standing probate cell of the key" the design named.

**At most one live cell per key.** `Parties.liveOnKey` finds the standing cell; `reKeyOntoStanding`
moves a whole cell in place or promotes part of one off it, and if a cell already stands on the new
key the mover merges into it. A merge adds a money holding into ONE lot — a balance is one lot
(Money D2) — and keeps every other lot with its own basis and date.

**Crossings.** At the close of revaluation the kernel re-reads every cell's banded dimensions
against its kind's edges and moves a cell whose people crossed an edge, as a whole, to the key on
the other side, recording `lattice.crossed`. Categorical dimensions are moved only by their owning
events. A band is read at the MARK, never the holder's own valuer.

**Two stops, both written down.** `exposureTo` read a loan at the lender's mark; a loan's mark asks
the lender's valuer; the valuer reads the exposure — the recursion 0d placed under item 21, reached
here through a credit decision during a probate distribution. Exposure is the face the name owes
(F3, Law 19), which is the cause; the valuer question for loans stays 21's. And acceleration was
mutually recursive — a called line that fails calls the line that called it — reached at rig
period 24 once cells held totals; a line is called once per pass, which is what acceleration is.

**Checks.** `check:opens` green on both worlds; lint, typecheck, `check:spec`, `check:forbids`
green.

## Item 0f.5–0f.6 — What survives a weight, and how the audit reads it

**0f.5 was half wrong, and the measurement says which half.** Every site that scaled a HOLDING
READ by the weight was deleted at 0f.1, because the world could not open otherwise. What survives
is a count of people, a per-member RULE times the count, or a total read per member — the
arithmetic of a population, not a second representation. The step closes with that classification
and one named default gone: a good with no charge journals no charge, not a charge of nothing.

**0f.6.** The `units` family's seat check already keys on the whole lattice key, so the invariant
is one live cell per key as declared. The household `consumptionIsBought` check loses its
half-a-piece-per-member band: goods are bought and paid for in whole pieces of the total now, and
the tolerance is derived dust only (Law 7). The `flows` family assumed a split COPIED the parent's
book and measured the child from it; under `moveShare` and `merge` holdings move between cells with
no leg, so the weight event now carries what moved per instrument and the family reads it as the
explanation on both sides (Law 19: the record, never an inferred copy). The mover lost it, the
destination gained it, a cell that merged away is skipped, and everything else is measured like any
holding.

**Checks.** `check:opens` green on both worlds; lint, typecheck, `check:spec`, `check:forbids`
green.

## Item 0f.7 — The household profile, sized at the step

**The step as written was a rewrite, and a rewrite is not a bounded change.** `consume.ts`,
`portfolio.ts` and `lifecycle.ts` already meet the clauses 0f.7 named; what the 0f design needed
and the module did not do was four things, and the plan now says so. Lifecycle re-keys by date
since 0f.4 and the portfolio is already the price ladder over every public print; both stay.

**0f.7a — what it will work for is a threshold on its own key.** The `benefit`-outlook gate is
gone. A cell in the first spell band asks the trade's going rate; one in a longer spell asks what
its members' NEEDS cost over the hours it offers, and does not offer into a trade paying less than
feeds its people (Labour B1, B3, D1.c). The basket is costed once (`basketOf`) and both decisions
read it (Law 4).

**0f.7b — a roof before paper, and the bank asked through the one door.** `homeBid` bids what it
is short of at its own level, up to what the whole of its spare reaches; the half-of-spare rule
(`households.toAHome`) was a shape and is deleted. What its spare does not reach it asks its bank
for through `ctx.request`, secured on the dwellings (Corporate Credit A1, A4; Households E2).
Housing's `askForMortgages` keeps skipping cells: the borrower's module owns the reason.

**0f.7c — one preference, drawn.** `spend = basket cost`, paid out of what is liquid after what
falls due — debt service on what it issued (`owedIn`) and the rent on its tenancy, a new read
`rentOwedBy` in `registry/funding.ts` off the kernel's own agreements, with the tenancy's public
shape moved there so housing and households read one shape. What stands above
`target = patience × (expected income + its own surprise) + what its holdings could move by` is
spare. `patience` is weeks of buffer drawn once per cell from `households.patience` and
`households.patience.dispersion`, the way `memory` is (XI-16 A3); the gap-closing rate that id
used to name and `households.buffer.periods` are deleted. Two tests that zeroed the buffer now zero
the patience mean.

**0f.7d — found reading for (c), fixed here because it is this item's code.** `wealthOf` and
`atRisk` valued the cell's TOTAL units against a PER-MEMBER cash since 0f.1; nothing throws on a
wrong wealth, so the opens run could not see it. Both read `perMember`.

**Positioned.** The `benefit` outlook has no reader now → 0f.11. A cell can now carry a mortgage
and 13d.1's missed-payment drift is measurable → 13f (not measured, Law 11).

**Checks.** `check:opens` green on both worlds; lint, typecheck, `check:spec`, `check:forbids`
green.

## Item 0f.8 (repositioned) and 0f.9 — One cell per key, from the draws

**0f.8 was an item wearing a step's number.** The small-firm profile as written was every verb of
§42 — produce, sell, employ, buy on terms, invest, draw, borrow, default, promote — for a module
that has `phases: []` and `participants: []`; items 11 and 12.4 both sat "on 0f.8", which is the
sector's item hiding inside the lattice's. Law 10: it is INSERTED at its dependency position as
11.0, in sub-steps sized on the day each is taken, and the record says where. 0f owns the
representation of a population, and for small firms that is 0f.3, 0f.4 and this step.

**0f.9 — households.** One cell per (cohort, bank). `seed.households.cellsPerKey` and
`splitPopulation` are gone: how finely a population is cut is the lattice's resolution — its band
edges — and a count of cells per key was a second resolution that put two cells on one key, which
is what 0d measured as 299 `units` findings.

**0f.9 — small firms.** `drawSmallBusiness` draws one size per FIRM, so the dispersion A2.a asks
for is in the population and not in a count of cells. The module's own seed opens each firm with
`smallBusiness.opening.cash` (a PLACEHOLDER that dies at 12.1, when a founder puts the money in)
times its size, cuts the firms of one (bank, line) by the size band that puts them in, and adds one
cell per occupied key — the count as its weight, what they hold between them endowed per member.
The band the kernel reads at the seal is the band the members were cut by, because a mean of
values in an interval is in the interval. `CELLS_PER_KEY`, `split` and the per-cell
`smallBusiness.size.*` parameters are deleted: the register prints what the register holds.
`SMALL_PER_NAMED` stays beside `BANK_COUNT` and `FIRM_COUNT` — three draw-time counts of one class,
each an outcome at item 12.

**A build stop, fixed at the cause.** The small-firm kind banked as `operational`, a deposit class
nothing declares; nothing noticed for as long as the sector held nothing. The first board paid
threw at `classOf`, and the class is `corporate` (Banks Funding A1.b). Two tests followed the seed:
`equity.test.ts` named two cells of one key and now names the same cohort at two banks;
`resolution/cells.test.ts` measured invariance to a resolution that no longer exists and is
deleted — its purpose is 0f.10's.

**Checks.** `check:opens` green on both worlds; the lattice, small-business and household-profile
tests green; lint, typecheck, `check:spec`, `check:forbids`, `check:deaths` green.

## Item 0f.10–0f.11 — The lattice over a year, and 0f closed

**Six tests, and what they found.** `test/lattice.test.ts` asks whether a cell stays what the
lattice makes of a population once people move. Two of the answers were kernel defects the
representation exists to make impossible, and both are fixed at the cause: a bank move rewrote a
cell's key in place and never merged, so after a year every household of a cohort had moved to one
bank and stood there as three cells on one key (the standing cell is now found before the move and
the mover merges into it); and a merge left what the mover ISSUED naming a ceased issuer — no cell
had issued anything before 0f.7b — so the next maturity was refused at settlement (`mergeCells`
carries issued instruments through `Instruments.reseat`, as it already carried agreements).

**Two of 0f.7's own rules were shapes, and the invariance test is what says so.** The reservation
wage read the cell's spell BAND; a decision keyed to a band index makes the band's edge a
preference, and the rule now reads the basket only (the spell stays a dimension: the lattice
stratifies on it, nothing decides on it). Patience was drawn by cell id, so a member that crossed
an edge drew a new one; it is drawn under the population's name — the seeded dimensions of the key
— and every cell of one population holds the one draw. With those two gone, refining every band
edge by two leaves the population the same population and moves a year's household spending by
less than the grain's own dust (one piece per cell-decision, derived), which is the exit.

**0f.11.** ARCHITECTURE 4.4 is rewritten for totals, the lattice, the five events and one cell per
key; the CLAUDE.md digest line follows it; COVERAGE is re-marked for §41, §42 and §46 A3 (Part XI
has no rows of its own — it is reached through the clauses that cite it). The `benefit` outlook is
deleted from `expectations` and the `Subject` type: nothing has read it since 0f.7a.

**Positioned.** A cell buying from a cell, the size-draw spread and clustered defaults → 11.0 and
11.3; entry and exit both non-zero → 12.7; the outlook-spread raising crossings → Part XII; a
`leverage` band on the household lattice → 12a.4; the switching cascade that moved every household
to one bank within a year → 17.0.

**Checks.** `check:opens` green on both worlds; the lattice suite green; lint, typecheck,
`check:spec`, `check:forbids`, `check:deaths` green.

## Item 0g.1 (in progress) — The ladder, and what its first rung found

**The ladder is a script, one rung per process** (`npm run ladder -- <banks> <firms> <grain>`,
`test/ladder.ts`), because a measurement runs at the end of a step and not in the suite (Law 11);
what a rung must hold by construction — people and firms equal at both grains, no fewer keys at
the finer one — is `test/ladder.test.ts` on the first rung, where the suite can afford it.

**Two build stops at the first rung, fixed at the cause.** `equity/share.ts votesOf` multiplied a
cell's TOTAL shares by its weight — a 0f.1 site no opens run reached, because votes are counted
only where a listed line has a record date — and the first rung cast more votes than there are
safe integers; votes are the units held times the votes a share carries. Then the second rung ran
out of six gigabytes of heap: `Register.moveShare` COPIED the whole equity ledger of the cell the
members left — every entry since the seed — into every cell a death or a crossing split off, and
`merge` concatenated it again, so the history of one population was held once per event that ever
touched it. The movers now arrive with ONE entry, their per-member equity under the cause that
moved them, on a fresh walk; a merge adds the one entry its mean adjustment is. The `accounts`
family's identity (entries = moves + 1, Σ entries = the walk) holds by construction on both.

**First rung, 3 banks / 12 firms, a year, before and after** (behaviour identical: people, small
firms, events, sessions, money per member and the going rate did not move — Law 18's gate):

| | ms/period at 52 | heap MB at 52 | live sampled heap | audit findings |
|---|---|---|---|---|
| before | 121 | 370 | 235 MB (162 MB under `households.lifecycle die` → `moveShare`) | 5,418 |
| after | 113 | 124 | 69 MB (journal 13, ledger 9, lots 3) | 3,812 |

The 1,606 findings that went were the `accounts` family counting a copied ledger's entries against
a fresh walk's moves. The second rung is running as this is written; its line and the third rung
follow in the next entry. Still to place in 0g: lots never coalesce (36,197 lots over 168
holdings after a year at this rung — 0g.6), and the journal is the largest thing left (0g.14).

## Item 0g.1 (continued) — The second rung, and what it found

**A third build stop, at period 32 of the second rung: acceleration walked every ordering of an
issuer's lines.** 0f.4 made a line called once per pass, but the set held only the lines DOING the
calling, so a line was still redeemed once for every line that had defaulted before it in the
pass — factorial in the count of an issuer's lines — and one firm's paper ran 1.2 million failed
maturities inside one period before six gigabytes of heap gave out. The set now holds every line
CALLED in the pass, whoever calls it, and is cleared when the outermost call returns
(`world/actions.ts accelerate`). First-rung behaviour did not move but for the events that were
the repeated calls (56,543 → 54,431).

**The ladder so far** (a year; ×1 the declared grain, ×2 every band cut in two):

| rung | ms/period at 1 / 13 / 26 / 52 | heap MB at 52 | parties / cells / people | events | sessions | audit | money/member | wage/h |
|---|---|---|---|---|---|---|---|---|
| 3b/12f ×1 | 243 / 130 / 118 / 115 | 120 | 76 / 37 / 39,171 | 54,431 | 57 | 3,812 | 1,117,471 | 1,351.52 |
| 3b/12f ×2 | 255 / 134 / 127 / 128 | 318 | 84 / 45 / 39,171 | 57,539 | 57 | 5,360 | 1,117,471 | 1,351.52 |
| 6b/60f ×1 | 521 / 331 / 315 / 375 | 207 | 118 / 38 / 195,844 | 148,173 | 107 | 7,250 | 2,939,371 | 1,473.73 |
| 6b/60f ×2 | 491 / 320 / 301 / 367 | 197 | 133 / 53 / 195,844 | 150,195 | 107 | 7,857 | 2,939,371 | 1,473.73 |

People, small firms, sessions cleared, money per member and the going rate are the same at both
grains on both rungs; what differs is what should — keys occupied, and the events and findings
that come with more cells. Time is reported, never asserted. Lots grow without bound (302,645 at
the second rung after a year, ~5,800 a period): 0g.6's coalescing is where that goes.

## Item 0g.1 (closed) — The third rung, and the ladder as it stands

**A fourth build stop: lots.** The third rung held 4.4 million lots by period 4 and spent nine
seconds a period on them. Three things made them, all layout (Law 18): every fill of a session at
one cleared price was its own lot; a re-key copied lot by lot and a merge concatenated, so a cell
that hired every period held a hundred copies of one lot by spring; and the bank every household
moved to interleaved every arriving book's `[p0, p1, p2]` after the last, so nothing ever joined —
sixteen thousand lots of one line by period 3. A lot is a basis and a date, and two credits with
the same of both are one lot: `credit` joins onto a matching last lot, `moveShare` and `merge`
join runs of equal lots, and a merge orders the two books by date first (stable), which is what
first-in-first-out means once a holding has two histories. No value, no draw and no carrying
answer moves; the first rung's events moved by two (54,431 → 54,429), which is the two draws whose
lot changed hands in a different order, and every ratio stood. 0g.6's "lots coalesce" is done here.

**The ladder** (a year; ×1 the declared grain, ×2 every band cut in two):

| rung | ms/period at 1 / 13 / 26 / 52 | heap MB at 52 | parties / cells / people | events | sessions | audit | money/member | wage/h |
|---|---|---|---|---|---|---|---|---|
| 3b/12f ×1 | 245 / 125 / 110 / 98 | 192 | 76 / 37 / 39,171 | 54,429 | 57 | 3,812 | 1,117,471 | 1,351.52 |
| 6b/60f ×1 | 521 / 308 / 261 / 262 | 333 | 118 / 38 / 195,844 | 148,176 | 107 | 7,250 | 2,939,371 | 1,473.73 |
| 6b/60f ×2 | 543 / 328 / 272 / 269 | 333 | 133 / 53 / 195,844 | 150,198 | 107 | 7,857 | 2,939,371 | 1,473.73 |
| 12b/200f ×1 | 923 / 950 / 842 / 704 | 975 | 212 / 38 / 652,806 | 323,106 | 605 | 16,215 | 822,500 | 16,052.20 |
| 12b/200f ×2 | 938 / 965 / 831 / 692 | 374 | 229 / 55 / 652,806 | 325,955 | 605 | 17,082 | 822,500 | 16,052.20 |

Lots after a year at the third rung: 5,952 (from millions). Against 0g's exit — (12, 200) a year
under 60 s, (3, 12) under 3 s — the third rung is at 37 s and the first at 5.1 s; the first is
where the per-period fixed cost lives and 0g.2–0g.5 are what take it down. Heap at the third
rung peaks near a gigabyte inside a period and settles to 975 MB at a year (the journal at
323,000 events; 0g.14). The wage per hour at the third rung (16,052 against 1,474 at the second)
is a measurement of the labour venue at scale and is positioned at 12b.3, not chased.

**Checks.** `check:opens` green on both worlds; the lattice, ladder, register tests green; lint,
typecheck, `check:spec`, `check:forbids`, `check:deaths` green.

## Item 0g.5 (first site) — The basket costed once

The first rung's profile put an eighth of a period in the labour venue's gather, and most of that
was `willWork` costing a cell's basket again for every venue it was asked about — twenty venues in
the four-country world. The basket is costed once per cell at its decision, its needs are kept in
the cell's own `DECIDED` working store, and the venue reads them. A cell with no income outlook
still costs its basket and still offers its hours for what feeds its people, so the ladder's first
rung is byte-identical (events, sessions, money per member, the going rate) at 94 ms a period
from 98. The rest of 0g.5's sites stay listed; this was the one the profile named.

## Item 0g.2a — The period's deltas, indexed where the record is appended

Three kernel families rebuilt the same maps from a period's settled records every period — holding
deltas by holder and line, issued deltas by line, units made and used up — each with its own walk.
The ledger writes those maps as it appends each record, the one writer of the record, and the
families read them (`Ledger.deltasIn`). The money family's per-instruction netting (C2.c) still
walks the records, because it is a check per instruction and there is nothing to index. First rung
94 → 88 ms a period, everything else byte-identical; third rung at eight periods 931 ms from 992.
The remaining readers on 0g.2's list are module walks the third-rung profile does not name (no
family or module walk above three per cent), so they stay listed and are taken when a profile
names them rather than as churn (Law 18: gate on behaviour, and on a number).

## Item 11.0a — A small firm makes and sells

**The sector does something.** A cell of small firms makes its line out of its members' hours — one
person and a van, so no wage leaves it for them — at the recipe the good declares, from the inputs
it holds; bids for the next batch's inputs at what each is worth to it (the output at the price it
expects, less the other inputs, the arithmetic a named firm runs with no wage term) and with the
money it has; and offers what it made at whatever the book gives, because a service cannot be held.
The decision is taken before the labour venue meets, the batch is started after wages are paid, and
the sell is read off the register when the market asks. The seed opens each cell with one period
of its inputs at the opening print, the foundation's own statement for a named firm.

**Two things the step named that it does not do.** A drawn productivity per cell would be a second
dispersion beside size (Law 2) over a party that is many firms; the recipe is the technology. And
`ordersFrom` is not shared with the firms module, because a module never imports a module; what
is shared is the one public read both needed — `registry/expectation.ts expectedPriceOf`, own
outlook else the tape — and the firms' and households' own copies of it are a Law 4 finding.

**What the test found about the rig.** A rig of twelve firms has drawn no mine as often as not, so
`rigFor` gained `makes: ['coalRaw']`; and in the world that has one, the mine never starts —
`bound: labour`, no unit cost, no stock at the seed — so no input line trades and the small firms
live off their opening stock. That is the named firms' condition and is positioned at 12b.3.
`PRODUCING_KINDS` claimed every module walked it; none does, and the comment says so.

**Three build stops in freight, reached for the first time.** The small firms of one country bid
for flour across the water, and a merchant shipped it — the first cargo ever loaded in an opens
run. Loading and landing create the good in transit under causes settlement refuses for a
`create` (Commodities Spot F1: units are produced); both are the transformation a batch is and
carry `production` now. A carrier loading two cargoes in one session pledged the same hull twice,
because the free count was taken when the session opened; the pledge reads the register. And the
sail phase read the weather and wrote its own event without declaring either (Clearing F1.a).
All three are fixed at the site and none is this item's mechanism.

**Measured, not chased.** With the sector live the first rung ends the year with 63 small firms of
144 and 26 cells of 37: cells that cannot buy inputs run out of cash and fail, which is XI-3 doing
what it says (`fails: ['cash', 'solvency']`) in a rig whose upstream lines never trade. Where they
go — an estate for a cell — is 11.0e's.

**Checks.** `check:opens` green on both worlds; the freight, small-business, lattice, ladder and
household tests green; lint, typecheck, `check:spec`, `check:forbids`, `check:deaths` green.

## Item 11.0b — Inputs on terms

**Who takes terms is a fact about the kind.** `PartyKindProfile.buysOnTerms` is declared on every
kind: firms and small firms say yes (§36 A4: the tier that lives on trade credit), and so does the
treasury, because the measurement said it was the one buyer that ever took terms in the scale
model; households say no (C1.d: a loaf is paid for with money it has), and so do estates, pools,
houses, insurers and the rest, which buy nothing that ships. The seller's judgement of the buyer
(`shipsOnTerms`) is now taken among those kinds only, where before it judged every buyer and the
registry said nothing. And the trade-credit module answers for small-firm sellers as it does for
named ones — one decision, one record, because a corner shop and its supplier are both firms.

**One test read too literally.** The invoice test named the seller as the holder of its own
receivable; with the sector live the seller had ceased by period three and its estate held the
row, which is what Register F2 says happens. The test resolves the seller through its successor.

**Measured, not chased.** No small firm takes an invoice in the rig, because no input line trades
there (the named mines never start: 12b.3). The mechanism is the same one a named firm runs, and
it is measured where a named firm's is.

**Checks.** `check:opens` green on both worlds; the trade-credit and small-business tests green;
lint, typecheck, `check:spec`, `check:forbids` green.

## Item 11.0c — Hours

A cell of small firms posts into the trade its line employs, in its region, at what an hour is
worth to it — the output an hour makes possible at the price it expects, less the rest of the
recipe — for the hours its stock of inputs can use beyond its members' own; when that is nothing
it posts an empty opening, and the venue sheds what it no longer wants at the cell's cost. What it
employs counts in what it plans, and the payroll that settled counts in what it starts and in what
a unit cost, read off the labour module's own event as a named firm reads it. `PRODUCING_KINDS`,
the list every module was said to walk, had no reader and is deleted.

**Measured, not chased.** In the rig every small firm's members already out-run its stock of
inputs, so every posting is empty and no small firm hires; the ladder's first rung is unchanged.
The hire is measured where a named firm's is, and that is 12b.3's world.

**Checks.** `check:opens` green on both worlds; the small-business and trade-credit tests green;
the five labour reds are the five that predate this session; lint, typecheck, `check:spec`,
`check:forbids` green.

## Item 11.0d — The owner

**Who owns a cell of small firms is a row** (`smallBusiness.ownership`), opened at the seed from the
cell to the household cell of its region and bank in the first working cohort, and binding a going
concern: an estate winding the cell up does not run it. Each period after the session, what stands
above what the cell keeps goes to its owner as a dividend in a two-sided instruction, which is how
a household comes to have income that is not a wage (Households B3).

**What it keeps was the whole of the step.** The first cut drew everything above what the cell had
bid for; every cell was emptied to the same nothing, and the lattice — correctly — merged the
sector into one cell a line, 98 to 27 in a period. What a firm keeps is a period of trading at its
own scale: the inputs for the batch its hours can make, in full, at the prices it expects, plus the
wages it owes and what falls due on what it has issued. It is its own working capital, read off its
own plan and its own book; there is no payout ratio and no target return (Law 2).

**Found with it.** A cell ran its members' hours flat out into a book that takes a fraction of it,
and the unsold perished at cost: five cells in six were insolvent by period one. What it makes is
what it expects to sell — its own outlook of its own fills, one piece to find out — which is the
named firm's rule (Firm B1) and was never the small firm's until now. And the make-and-sell test
took its cell ids after the year, when the cells that had merged onto another's key were gone from
the parties store; it takes them at the seal.

**Measured, not chased.** With the owner drawing, the first rung ends the year with 65 small firms
of 144 in 27 cells, as before the draw; in the world with a mine, the sector settles at twenty cells
by period five, most of them merged rather than dead.

**Checks.** `check:opens` green on both worlds; the small-business tests green; lint, typecheck,
`check:spec`, `check:forbids` green.

## Item 11.0e — Borrow and default

What a period of trading at its own scale needs beyond what it holds, a cell of small firms asks
its bank for through the one door every borrower uses, unsecured: a service line has no plant to
pledge. The bank reads the ask next period and decides, and the banks module already lends to a
cell (13d built the drawing per member; 0f made it totals). The first rung ends the year with 71
small firms of 144 against 65 without credit, which is A5's dependence measured rather than
asserted. Default is the kernel's: a coupon a cell cannot pay fails at settlement, the cell fails
on cash and goes to its estate (XI-8).

**Not built, and said so.** The step named the failed MEMBERS moving to the estate while the rest
trade on. With holdings as totals a cell fails as one; a loan and a miss that are per member are
12a's arrears design, and that is where it sits (12a.1, 12a.5).

**Found with it, fixed in the kernel.** `owedIn` summed a coupon due as a rate times a face, never
put on the money's grid, and the first cell to owe a coupon read a position that was not a count
of anything (Law 8). A coupon due is `payable` now, which is what will be paid.

**Checks.** `check:opens` green on both worlds; the eight small-business tests green; lint,
typecheck, `check:spec`, `check:forbids`, `check:deaths` green.

## Item 11.0f — Plant and promotion: positioned, not built

The step was two mechanisms and neither is the profile's to build today, and the plan now says
where each of them goes rather than carrying a claim.

**Plant.** Every line a small firm runs declares plant (the room the table is in, the ward, the
classroom — Capital Programme A2), the named firm's start is limited by its vintages, and the
profile's `canStart` reads hours and inputs and no plant: a cell serves a meal with no room, which
is a finding and is written down. The limit cannot go in alone — a cell holds no plant, so it would
be a sector that makes nothing — and the purchase needs what a cell's money costs, which is its
bank's quote (11.2), and the arithmetic of what a project must earn, which the firms module owns
and a module never imports. It is 11.2a: the cost-of-capital read becomes the registry's, the way
the expected-price read did at 11.0a, the seed opens a cell with the plant its opening batch takes,
and the profile bids in the plant market with what it retains. One arithmetic, no copy (Law 4).

**Promotion.** The event exists and fires — a cell whose band moves is re-keyed by a `promotion`
event with the cause `crossed an edge` — and what it makes is a cell, never a named firm, because
a named firm is a declaration the firms module draws at the seed and nothing births one after it.
That is 12.1's instruction and 12.4's promotion. With holdings as totals *a member* whose size
crosses the last edge is not visible to the model at all; the per-member position is 12a.1/12a.5's,
as 11.0e already said.

**COVERAGE.** A6.b MET (a weight of one is a named party; the boundary is a size). A6.c PARTIAL,
naming 12.4.

## Item 11.0 — closed

The profile is built in five sub-steps (11.0a–e) and the sixth is positioned. What a cell of small
firms can do in the scale model and the rig: decide a batch from its members' hours and its own
outlook of its fills; bid for its inputs at what they are worth to it; be shipped on terms and ship
on terms; post for hours; keep a period of trading at its own scale and pay the rest to its owner;
ask its bank for what it lacks and fail on a coupon it cannot pay. What it cannot yet do is under
the item that gives it the mechanism: plant (11.2a), promotion (12.4), a per-member failure
(12a.1/12a.5), an input line that trades in the rig (12b.3). Nothing in the code changed in this
close; `check:opens` stands as at 11.0e.

## Item 11.1 — §42 re-marked from a run

A 52-period year of both scale models through `coverage:reached`, the kernel's own reach register
asked what was ever produced. Every capability the small-business module declares (3), and every
one the modules a small firm trades through declare — banks (8), labour (1), goods (126),
trade-credit (1) — was produced. A1 and A3 go from PARTIAL to MET: a small firm sells, borrows
and fails in the run, and its leverage is a loan row banded by the lattice, not a mean. A1 is
UNMEASURED on employment, because in the scale model no input line trades and every posting is
empty (12b.3); the mark says so rather than claiming it. A2, A5, A6 stand as marked at 0f.9 and
11.0e.

**Found by the run, fixed at cause, committed on its own (5f0b85c):** the four-country world
stopped in the year at a drawing on a line a cell had inherited by merging — the loan's terms name
who signed it and the row had been reseated. Two sibling reads of the same stale term are written
under 11.2 and 12a.4.

**Checks.** `check:opens`, `check:existence`, `plan:check` green. Nothing in the engine changed
in this close.

## Item 11.2 — Loans: the bank of the key, and the amortiser

**The bank of the key.** A cell keyed on a bank is bank-dependent in the spec's sense (A5, A6.a):
the lattice put its bank in its key, and `publishQuotes` now reads that dimension — a cell keyed on
a bank is quoted by that bank and by nobody else; a population keyed on no bank, and every named
borrower, is quoted by every bank that issues its money as before. A structural read of the
lattice, not a kind. Test: every quote a never-moved cell received in the scale model names the
bank of its key.

**The amortiser.** `DueAction` gains `amortisation` — the slice of every holding the issuer
redeems at par on the date, a fraction of what is outstanding and never an amount, so the row's
units stay the one writer of what there is to repay. The loan kind's `due` puts one on every
period an amortising row has left: one over the periods to and including the maturity, read off the
calendar each time, so a further drawing on the row is repaid over the same remaining term and
nothing stores a schedule. The kernel's `redeem` takes the slice in whole pieces of the
instrument; the maturity takes what is left. `owedIn` counts the slice in what a borrower has to
find, and the row's own `cashFlows` carry the same schedule, so a yield derived from them is a
yield on what will actually be paid. Which rows amortise is a term struck at origination like the
rate: a row written against a pledge is a term loan and amortises (the mortgage pays interest and
principal, Housing C2), an unsecured ask is a line drawn and repaid at the borrower's option (C9)
and falls due once. Test: a secured ask's row falls by exactly the money the slice moved, the
unsecured one does not move, and the row's flows sum to more than par.

**Said plainly.** A small firm's working-capital ask is unsecured, so today a cell's row is a
line: the amortiser reaches a cell with its first secured row (11.2a's plant loan, 12a.4's
mortgage). §42 B1 is marked MET on the mechanism and UNMEASURED on that.

**Found, positioned.** A cell quoted by its bank and moved the same period has next period's row
written by the bank it left (17.0). `test/loans.test.ts` carries seven reds that predate this
item and are identical without it; each is written under 17.0 with what it says, and the
`units` family's "smallFirm cells stand for −51 people more than the weight events account for" is
E5's own family and sits under 11.5.

**Checks.** `check:opens` green on both worlds; `loans.test.ts` 8 green and 7 pre-existing reds
unchanged; `small-business.test.ts` 8 green; lint, typecheck, `check:spec`, `check:forbids`,
`check:deaths` green.

## Item 11.3 — Correlation, with no parameter

B4 and B4.a are measured rather than declared. One four-country world at one seed — identical
draws — stepped twelve periods: the small-firm cells that failed on their cash are counted by
region and by period. Regions differ (over twenty periods the count was 34, 5, 29 and 5) and
within a region the failures land together (26 of the 34 in one period), because the same demand,
the same rates and the same region hit every cell in it. Nothing declares a correlation, and the
test says so over `params.all()`. B3 is PARTIAL: the unit that defaults is the cell, and the
individual firm's miss is 12a's per-member position.

**Checks.** `small-business.test.ts` 9 green; `check:opens` green; lint, typecheck, `plan:check`,
`check:existence` green. No engine code changed.

## Item 11.4 — The pool into the vehicle

`saleable` never needed a change: 10c replaced its `isLoan` gate with the two facts every kind
declares — no market, a named obligor — and a cell's loan row has both, so a bank's own view of
what it would sell shows a cell's row beside a named firm's. A probe in the scale model, asking
each bank the way the arranger does, sees one. What did change is the money the arranger sells
in: `moneyOf` read the registry's first currency for every bank in the world, so in a world with
four currencies a bank abroad would have sold rows in a money it does not issue and found no
bidder. It is deleted; `arrange` reads `registry.currencyOf(bank.region)` per bank.

**Said plainly.** No deal carrying a cell's row has been cut in the scale model yet; §42 C1 is
MET on the mechanism and UNMEASURED on that. The §42 C2–C6 and D rows are XI-11's own clauses
under a second name and are re-marked at 11.5 with the rest.

**Found, positioned.** Two pre-existing reds in `test/securitisation.test.ts`, identical without
this item: a stale assertion that the module declares no number (it declares one RESOLUTION), and
a stale reading of which capital rule binds. Under 17.0.

**Checks.** `small-business.test.ts` 10 green; `securitisation.test.ts` 13 green and 2
pre-existing reds; `check:opens` green; lint, typecheck green.

## Item 11.5 — E1–E6 as tests; §42 re-marked; module 11 closed

Six FORBIDs, each a test over one twenty-period scale model: every row a vehicle holds names its
obligor (E1); every live tranche has a holder (E2); every deal names a vehicle that holds the rows
and the arranger holds none of them (E3); the population is not what it opened at and the change
is exactly the sum of the dated weight events (E4); a weight is an integer count moved by five
named events with a cause and a period, and the units family is silent on the kind in every period
(E5); every impairment names the row's obligor and every write-down the holder it landed on (E6).

**E5's test found a kernel defect and it is fixed at cause.** The kernel's `cease` recorded a
party ceasing and nothing about its weight, so a cell that failed on its cash went to its estate
and its members vanished from the population with no event behind them — the units family said
`smallFirm cells stand for −49 people more than the weight events account for` in the first period
of every run, and the 52-period loans run carried the same sentence at −51. `ceaseCell` is the one
writer of that death now: a cell that ceases for any reason records the death of all its members
with a cause and a successor; `dieCell` is the empty-handed case of it. The family is silent.

**COVERAGE.** All 28 rows of §42 re-marked: 23 MET, 4 PARTIAL (A6.c promotion to a named firm,
12.4; B2 a cell's first pledge, 11.2a/12a.4; B3 the unit of default is the cell, 12a), 1 MISSING
(D3, a tranche as collateral, 14/17b).

**The suite, at the end of the module (Law 11).** `npm run test`: 109 files, 892 tests —
**191 failed, 701 passed**. At 0d it was 106 files, 869 tests, 199 failed, 670 passed. The reds
are of 0d's character — a test asserting the audit is clean and finding it is not, a rig that no
longer draws what a test asks for — and the per-file comparison against a run at the 0d commit is
recorded below when it lands. Every red file stands under its 0d cause or under the item that owns
it (17.0 for `loans` and `securitisation`, 12b.3 for `labour`, 23.1 for the rig draws).

**The section's exit, said plainly.** A credit tightening reaching a small firm before a large one
is not measured — there has been no tightening in the scale model to measure it by; an invoice
naming a small firm has not been seen because no input line trades there (12b.3); small-firm
sessions clear in the census. The mechanisms are built and the measurements that are missing are
named.

**Checks.** `small-business.test.ts` 16 green; `check:opens` green; lint, typecheck,
`check:spec`, `check:existence`, `plan:check` green.

## Item 11.2a.1 — Plant, the stock

A small firm's line now carries the plant its recipe names, and what a cell can start is limited
by the plant it holds through the same read the named firm's start is — `capacityFrom` over its
vintages and what it rents. Before this a cell served a meal with no room to serve it in (11.0f's
finding). The seed opens each cell with the plant a member's opening batch takes: the whole
machine the fraction reaches, because a firm whose batch takes three hundredths of a room still
needs the room; in the three vintages every seeded party's plant is in, carried at the straight
line since service. Measured before the rounding was right: ten of twelve lines in the scale
model opened with no plant at all, because a member's share of a machine is a fraction and a
fraction rounds down to nothing.

`seedVintage` and the seed's plant ages moved out of the capital-programme module and the
foundation seed into `registry/physical.ts`: a module's seed may not import a module, and a
vintage is one construction wherever a seed opens one.

**Not yet:** the purchase (11.2a.2) — the cost-of-capital read to the registry and a cell's bid in
the plant market — which is the half that needs the bank's quote.

**Checks.** `small-business.test.ts` 17 green; `check:opens` green; lint, typecheck green.

## Item 11.2a.2 — Plant, the purchase

**One arithmetic.** What a project must earn and the project itself moved out of the firms module
into `registry/capital.ts`: `costOfCapital` (the debt cost off the quote a bank gave it in the
caller's own window, else the state's curve at the caller's own horizon; the equity cost off what
its shareholders are paid against what the market says a share is worth), `project` (what it is
sure enough of to build for, less its own surprises, against what its plant lets it run at, at
the price where the contribution of the capacity it adds clears what its money costs it plus its
hurdle), and `plantOffers`. The firms module's copy is deleted, and with it its own `expectedPrice`,
which was the registry's `expectedPriceOf` written twice (11.0b's finding, closed for firms).

**A cell decides like a management.** Its hurdle and its horizon are two preferences drawn once
per population — region, bank, line — and kept; what it retains above a period of trading after
this period's bids is what it can put to a project; the orders it decides go out with its input
bids and its ask names its plant as security. Two populations do not take the same project.

**Found by the measurement, fixed at cause.** With its plant pledged on every week's
working-capital ask, a cell was written a new term loan every week — 414 rows in twenty periods —
because 11.2 inferred "term loan" from "secured". The borrower now SAYS how it repays on the ask:
at its option is a line, one row per (lender, borrower) drawn on again; on a schedule is a term
loan that amortises. A mortgage says schedule; working capital says option; a test says each.

**Measured, not chased.** No cell bids for plant in the scale model: every cell opened holding the
whole machine its batch reaches, so its plant lets it run at more than it expects to sell and a
management with no gap builds nothing (B3). The test asserts the rule from both sides — a bid
where there is a gap, none where there is not — so the day a cell's sales outgrow its room, the
assertion is the one that turns. §42 B2 is MET: a cell's row is secured on what it has.

**Checks.** `small-business.test.ts` 18 green; `loans.test.ts` 8 green and 7 pre-existing reds;
`check:opens` green; lint, typecheck, `check:spec` green.

## Item 12.1 — Firm birth into the small-firm tier

**The mechanism.** A household cell with money spare above its buffer — its own plan's number —
founds firms in the line whose return on what it costs to start is highest of those that clear
what it requires of a claim, its plan's number too. The starting cost is a member's period of
trading: the inputs at the prices the founder sees and the whole machine that batch takes. One
instruction moves the founders' money out of their own account and into the firms'; the new
members enter the live cell of their key by an `entry` event, or a cell that did not exist enters
the world with them — the kernel's `enter` takes a cell now and places it on its lattice the
moment it exists; and the founders hold the ownership row, which counts the members who run one.
A refusal is on the record with its reason, every period a household had money spare.

**Found by the measurement, built.** The first cut founded thirty-eight million security firms
in one period: the return test had no labour in it, so a line that paid anything paid infinitely
on a starting cost of a dollar; and every member of a cell decides alike, so a cell founded with
the whole of itself, and again the next period. What was missing was mechanism, not a bound:
the founder's own hours at the region's printed wage come off what a period brings (Firm A3: a
line nobody leaves a job for is a line nobody founds); a firm has a person in it, so a household
founds one per member not already running one, and the count lives on the ownership row; and an
entrant enters where demand is not being met (A4) — no more firms than the demand the book left
unmet at the last print, which a traded print now carries (`demandAtPrice`, `supplyAtPrice`) and
the price store reads (`unmetAt`), a kernel read and not a number anybody keeps.

**Measured, not chased.** No service line has ever traded in the scale model. Every small-firm
line's print is stale from period 0 — `noDemand` for ten of them (nobody buys a security guard,
a lesson or a haircut: the basket and the overheads name no service) and `noSupply` for six (the
small firms in them never offer, because their inputs never trade there). So every household is
refused with that reason and none founds; the mechanism is complete and its measurement waits on
a book with two sides. The demand side is positioned at 22 (the recipe) and the supply side at
12b.3; a founder's hours still offered by its household at 12b; a cell founding all-or-none at
12a.1/12a.5.

**Coverage.** Firm Birth A1, A2, A2.a, A3, A4 (UNMEASURED), A4.a, A4.b MET; A5 PARTIAL. The
opening-cash placeholder that died here is a SHAPE now: an opening condition, read once.

**Checks.** `small-business.test.ts` 20 green (founded-or-refused; a cell enters and the
population accounts for it); `core.test.ts` green; `check:opens` green; lint, typecheck green.

## Item 12.2 — Households form

**Sized honestly first.** The plan wrote formation as members of a cohort k moving to cohort 0
when their own income clears the rent; this world's cohorts are `working` and `retired`, and a
cohort of dependants is a population that consumes and does not work, which is a household
composition noun the model does not have. Formation is therefore ENTRY at the first cohort's
lower boundary (F1.b), by the geometry that already ages the band out at its upper one: the share
of a band standing at a boundary in a period, carried as whole people. No rate anywhere.

**The condition.** A household forms when its people can pay for a roof of their own — the
cell's own expected income per member, off its own plan, clears the rent its region's lettings
venue last struck. The rent is a registry read (`rentPrintedIn`), never the housing module's event
name. The ones who cannot form wait at the boundary, whole, and the record says why each period.
A formed member enters with nothing, which is what an adult with nothing is.

**Measured, not chased.** The scale model's lettings venue has never cleared: `noSupply` in
eleven periods of twelve, because no owner offers a dwelling to let. So no rent stands, nobody
forms, and the people at the boundary accumulate on the record with that reason — 89 of them
after twelve periods against 8 a period ageing out. Positioned at 15.

**Coverage.** Households F1 and F1.b carry formation (F1.b UNMEASURED on the entry); F4 PARTIAL.

**Checks.** `formation.test.ts` green; `household-profile.test.ts` green; `check:opens` green;
lint, typecheck green.

## Item 12.3 — Mortality is technology, per five-year band

The table was two rows, one per cohort, and a cohort is a band of this world's lattice — a number
per cohort was a number shaped to the model's own cut. It is sixteen five-year bands now, from 18
to 100, each the chance of dying within a year with its source on the row: the SSA 2020 period
life table, sexes averaged, rounded to two significant figures — a real-world primitive imported
as one. What a cohort dies at is derived: the mean of the bands it spans, weighted by the years of
each inside it, by the same uniform-age geometry that ages a band out; the last cohort spans to the
end of the table; and the period's share is the year's put on the calendar's own year fraction. A
cohort the table does not reach dies at nothing it can state, which is a refusal. The test asks
for the source on every band and for the retired to die faster than the working, derived.

**Checks.** `mobility.test.ts` 14 green, `formation.test.ts`, `household-profile.test.ts` green;
`check:opens` green; lint, typecheck green.

## Item 12.5 — A payer walks its own rows; a dividend leg says what it is on

The equity module paid a declaration by scanning every `DIVIDEND_DECLARED` row in the world and
resolving each debtor to see whether it was this issuer. The store kept a debtor index all along;
`byDebtorAndKind` reads it, on the facade, and the payer walks the rows it owes of the kind — the
store moved them to its successor at the cease (Register F2), so the resolved issuer is the debtor
of record. The audit's `dividendLegs` sliced the line's name out of an instruction's reason string;
the dividend receipt says what it was paid ON now — the share line for a declaration, the
ownership row for a small firm's draw to its owners — and the family reads the leg. Nothing parses
a reason. The 16 reds of `equity.test.ts` and the one of `equity-ledger.test.ts` predate this and
are unchanged.

**Checks.** `check:opens` green; lint, typecheck green.

## Item 12.4a.1 — The kernel's doors for a party born after the seed

Two doors the promotion of a small firm into a named one needs, and neither existed. A number
whose owner is born after the seal is declared the period it is born: `ParamRegister.declare`
asks the constructor's own guards of one declaration, and `ctx.declare` journals it
(`param.declared`); XI-14 is about declaration, not about timing, and twice is still twice.
`ctx.cells.promote` moves members out of a cell into a named party that entered this period and
holds nothing, with their share of every lot; the weight event carries before and after, because
unlike a re-key the population of cells falls by what left — which is exactly what the units
family reads — and what moved per instrument, which the flows family reads on both sides. When the
whole cell goes it ceases with the party it became as its successor. `moveShare` may take the last
member now; a split still leaves somebody behind.

**Not yet, and sized:** a party that entered as a `FIRM` decides nothing, because the firms and
equity modules index their rows once at assembly. 12.4a.2 says the three changes that make a born
firm a firm to them, and the trigger — a cell whose equity per member reaches the smallest named
firm's, a read of the world and never the lattice's edge.

**Checks.** `promotion.test.ts` green; `check:opens` green; lint, typecheck green.

## Items 12.4a.2 and 12.4 — A born firm is a firm to every module; promotion fires

**The boundary is a size and it moves (A6.b, A6.c).** A cell has outgrown the tier when one of
its members is worth what the smallest named firm with paper outstanding in a market is worth —
the size at which the bond market took somebody, read off the world; never the lattice's top
edge, which is a resolution and would move the answer with the grain. The first read was "the
smallest named firm" and it promoted a power cell in period five against a firm that was failing:
a failing firm's equity is not the boundary. Every member of such a cell becomes a named firm, one
per member, because a member is a firm.

**What being born means, module by module.** The firms module gained a register that grows
(`firms.register`) and `bearFirm`: a decl drawn from the seed's own spreads under the born name,
its three numbers declared the period it is born through the kernel's new door, the party entered,
the member promoted into it with its share of every lot. The equity module reads the birth through
`registry/births.ts` and gives the firm its residual: a private share line, issued to whoever
owned it as a small firm at the one opening level every line is cut at, the consideration being
the business the promotion brought; its payout patience drawn and declared. Both modules read the
seed's rows and the born ones as one from then on, so a born firm decides, produces, is quoted
and can float like a seeded one. The small-business module records the intent before the firms
module's own phase and its ownership row counts the members who left.

**Measured.** Twelve periods of the four-country world: 16 cells, 63 members, in power, telecoms,
waste and logistics; 63 firms born, 63 share lines issued and settled, 161 decisions by born firms.
Thirty periods of the one-country scale model: none, because no firm there has brought paper to a
market, which is 0d's finding read from the other side.

**Found and fixed at cause.** The last member's promotion left a cell of nobody, and the kernel
refused a weight of zero: the cell ceases into the party it became, what it issued is reseated
onto it (a coupon addressed to the ceased cell stopped the four-country world), and the event
carries before and after. And the labour module re-keyed an employment row's worker by the name
that signed it after that cell had merged away — a seed no test had drawn stopped the world in
period five — resolved through its successor now; the row's worker is a term and should be its
creditor (12b).

**Positioned.** A member promoted out of a cell that stays takes its assets and none of its
liabilities (12a.1/12a.5); the firms audit family skips born firms (21).

**Checks.** `promotion.test.ts` 2 green; `small-business.test.ts` 20 green; `check:opens` green on
both worlds; the equity, firms, labour and ledger reds unchanged; lint, typecheck, `check:spec`,
`check:forbids` green.

## Item 12.6 — The population is a fact, not a resolution

The plan said the cohort count is a resolution tested by invariance. The ladder had already
measured otherwise: per-member money and the wage per hour move with the count across its rungs,
so doubling it is not a thing the answer ignores. Thirty million people in a region is the size
of a country — a real-world primitive imported as one — and the number is re-kinded `technology`
with that reason. From period one the population is what formation, death and promotion make of
it, read off the cells, and the seed's count is read once and never again. The ratios that move
with scale are the ladder's findings about the venues at scale, and they are where they were.

## Item 12.7 — Entries, exits and promotions, measured; item 12 closed

Twenty periods of the four-country world: households and small firms lose members by dated
events with a cause; 87 members of small-firm cells outgrew the tier and were promoted, from some
cells and not all; and entry is decided every period. Said plainly: nobody entered. Founding is
refused every period because no service line has traded in the scale models, formation because
no rent has cleared — both on the record with their reasons, both positioned (22, 15). The test
asserts what is true: the decision is taken every period, never by a rate, and exit is not its
accounting identity; the day a book has two sides the refusals turn into entries and the test
does not change.

**Item 12 closes** with 12.1–12.7 and 12.4a done and its sections removed from the plan; the
findings it raised sit under 12a, 12b, 15, 21 and 22.

**Checks.** `population.test.ts` green; `check:opens` green; lint, typecheck, `check:deaths`,
`plan:check`, `check:existence` green.

## Items 12a.1 and 12a.2 — A missed payment is a row

A payer that cannot pay is a real state with a real consequence (Money E1), and until now the
consequence was a record in the ledger that one read walked and nobody held. It is a row now:
`ARREAR`, issued by the payer to the payee for what did not arrive, in the money it was due in,
carried at what it is because no market exists in a missed payment, named by the instruction that
failed and the class of payment the leg carried — a wage, a rent, a coupon — because the class
is what an estate ranks it by (XI-8; an order of classes, senior first, a fact about the law and
data). Settlement writes it in the same pass as the fail, as an issuance with cause `default`, so
the register, the ledger and every audit family carry it like any claim; a fail that was not the
payer's money — a delivery that was short — leaves nobody in arrears. Registered as a kernel kind
beside money.

What a party still owes on payments it missed is the rows it issued, in that money, off the
register (12a.2). It was a walk over THIS period's failed instructions, so a payer that missed a
wage last week and had the money this week was never short and one that missed today was.
Measured: the payer fails on what it owes, its estate pays what it has against the row, and the
rest stands, held by the payee or the payee's own estate.

**Checks.** `arrears.test.ts` green; `check:opens` green; `check:forbids` green; lint, typecheck
green.

## Item 12a.3 — What was missed is presented before what is new

An arrear falls due every period until it is paid — the whole of it, at par — and the kernel
walks a payer's arrears before the rest of the record, so money it has today goes to the wage it
did not pay last week before this week's coupon. What is paid is redeemed by the same door every
maturity uses; what is not stands, and settlement writes no row on a row: a missed payment on an
arrear is the same arrear. An invoice is presented from its due date every period until it is
paid, where it used to be asked for once and forgotten. The unpaid wage, the unpaid rent, the
treasury's transfer and the estate's hand-over need nothing of their own: each is a money leg a
refused payer could not fund, and settlement writes the row for every one. The households module's
agreement for an unfunded hand-over to probate was a second representation of that row and is
written no more; it stands only for a hand-over that failed for another reason.

**Checks.** `arrears.test.ts`, `mobility.test.ts`, `trade-credit.test.ts` green; `check:opens`
green; lint, typecheck green.

## Item 12a.4 — A cell asks for a mortgage every period it is short of roofs

The guard the step named had moved: it was `homeBid`'s "one mortgage at a time", which at a cell
of twenty thousand households read as one roof for all of them, ever, once one had a row — so
every cell was short of a roof every period for the life of a run. It is gone. A cell short of
roofs asks its bank every period for what its people's spare does not reach, on a schedule,
secured on what the loan would buy; a mortgage is a row per (lender, cell) in totals. A lender
forecloses on a DEFAULT recorded on the row — the kernel's own event, read through the banking
registry — and never on the borrower being gone: a cell that moved bank is succeeded, not failed,
and its dwelling stays where it lives (the 11.1 finding closed); the legs name the borrower as it
is now.

**Measured.** Twelve periods of the scale model: household cells ask every period and are
declined on appetite every time — no bank quotes a household name — so no mortgage is written
and three cells stay short of a roof, from every cell every period. The ask is the households';
the answer is the credit view's (17.0), where the haircut on the print also lives.

**Checks.** `mortgage.test.ts` green; the household, housing, formation, mobility and arrears
tests unchanged; `check:opens` green; lint, typecheck green.

## Item 12a.5 — A household that cannot pay fails

Nothing is immortal (XI-3), and a household was: it could miss a wage, a rent, an invoice, and
carry the arrears forever with no consequence but the rows. Now the household kind declares what
it fails on — cash — and the households module, which owns the population, says what happens: a
living cell the kernel says cannot pay what it still owes goes whole to the standing probate cell
of its key, as the dead do, with `credit = defaulted` on the key, its holdings handed to the
probate office and the kernel's reason on the record. A lender reads the record off the key
(Corporate Credit E5). Consumer credit is the same loan row unsecured: what its basket is short
beyond what it holds it asks its bank for, at its option, through the door every borrower uses.

**Two stops on the way, both fixed where they were.** A cell that ceased into its estate still
had instruments issued in its name, and the next coupon addressed to it was Money E4's refusal —
the four-country world stopped in period 7; `ceaseCell` now reseats what the ceasing cell issued
onto its successor, as a merge and a promotion already did. And a cell that had absorbed both the
payer and the payee of an arrear held a claim on itself; handing it to probate was an issuance
with no price. A self-claim stays where it is: it is the estate's to extinguish.

**Finding (positioned under 12a.5, waits on 12b).** With holdings as totals a cell fails as ONE:
every member of a twenty-thousand-household cell goes to probate for a miss that was some of
theirs. The per-member miss needs a member with an income of its own, which is what 12b's
employment register gives it.

**Checks.** `household-failure.test.ts` green (a cell that cannot cover an arrear fails on cash,
its people are in probate marked defaulted, the reason names the miss); the mortgage, arrears,
formation and mobility tests green (16); the households suite's nine reds are the nine of 12a.4;
`check:opens` green; lint, typecheck green.

## Item 12a.6 — The state fails only in a money it does not issue

A treasury could not fail at all: its kind said so, and XI-3's exception for it was the whole of
the kind. Now it fails on cash like anybody, and WHICH money is a read, not a kind: the kernel's
failure test walks every money the party has live arrears in and skips the one it is sovereign in
— the kind borrows on the state's credit and the money is the one of where it sits. In its own
money a miss is a shortfall (Treasury D3), the row stands (Money E1) and no credit event is
recorded for it (G1); in a money it does not issue it is a default (G2), the treasury module says
so with the kernel's reason, no estate opens (G3: it resolves its own kind), and it is out of the
market while the row stands — no auction is announced, and the rows are named every period (G5).

**A kind that fails on nothing now says why.** `cannotFail` carries the clause, and the registry
refuses a kind with an empty `fails` and nothing beside it: the central bank, probate, the deposit
insurer and the estate name theirs. A securitisation vehicle's senior coupon it cannot fund has
been an arrear since 12a.1, so `cash` already fires on it; the comment that said otherwise is
fixed.

**Found and fixed on the way (Law 4).** A bank's risk weight read "cannot fail" off the kind's
`fails` list, so the moment the treasury could fail anywhere its own paper weighed as an ordinary
exposure — every bank in the scale model raised capital against its bills from period two and the
treasury ran dry by period thirty. The weight now reads `sovereignIn` for the paper's own money
(Sovereign E5); the same treasury's foreign-money paper weighs what any exposure does (G2). A
first cut excluded the treasury on a DOMESTIC arrear too, and twelve missed coupons in the year
measured that G1 means exclusion follows a default, not a shortfall — corrected before commit.

**Checks.** `treasury-default.test.ts` green (two: a domestic miss is no default, no estate, and it
keeps bringing paper; a foreign-money miss is a default, no estate, and it is excluded while the
row stands); the treasury, estate and credit-events suites at their 12a.5 reds (17, none new);
`check:opens` green; lint, typecheck, spec, forbids, deaths green.

## Item 12a.7 — A backstop drawn is a loan row

A draw on a committed line was a transfer and a number: the bank paid money away with nothing on
its book to show for it, and what the issuer had drawn lived in the agreement's terms where no
register could read it. Now a drawing is a loan row on both books — the credit registry's kind,
issued by the issuer, held by the committing bank for what it lent, at the rate the bank quoted
the name when it committed the line, maturing a year out (`shortTermDebt.lineMonths`, a
convention of the facility), repaid at the issuer's option and drawn on again; the kernel
presents its interest and its maturity like any loan's. What is drawn is the row's outstanding,
read off the register; the fee falls on the headroom above it. A line is arranged WITH the paper:
an issuer gets one the period it first offers paper, from its bank, at that bank's quote of the
name that period — a name nobody quoted has no lender behind its paper — and nobody else gets one.

**Measured.** In the scale model every paper issuer is in an estate before its paper matures, so
the draw is never reached there; `backstop.test.ts` builds the line, the paper and the run out of
the doors any module has and lets the module draw — one row, the bank holding it for what it lent,
the par paid out of it, no default. The equity the period-nine issuers read at `paper.issue` is
exactly nothing, and they are the ones that die by period twenty-one: a finding for the credit
view that sizes the line (17.0), written under 12a.7.

**Checks.** `backstop.test.ts` green (two); `short-term-debt.test.ts` green (nine); the estate,
securitisation and bank-capital suites at their reds of 12a.6 (none new); `check:opens` green;
lint, typecheck, spec, forbids, deaths green.

## Item 12a.8 — One place to buy a machine, and an estate that asks a price or takes the market

A firm's project bid in every market its plant could come from — the capital-goods line and every
dead firm's vintage of the kind — and spread its money across them in proportion to what each was
asking; when two cleared it had bought the gap twice. Now it buys each kind of plant from ONE
place: the offer asking least per year of the service it would count out to its own horizon, a
second-hand vintage or a new build, with a tie going to the new build that has its whole life ahead
of it. The bid itself was already what the service left is worth at its hurdle (D3), so a used
machine wins when a year of it is cheaper and not otherwise.

The estate's ask had three states and a path between two of them: the last print while it had
time, sliding to nothing as its programme ran out, and the market in its last period. The slide was
a written price path (Appendix B) — a reservation nobody had cleared, moved by a count of periods.
It asks the last print while it has time and takes the market in its last period; nothing between.

**Measured, and written down under 12b.3.** The `sb-found` scale model now throws in period six
where it used to run: bank.c's wage bill is 5.1e15 and its line publisher cannot tick a negative
room. The cause is on the record at the baseline too — the banking labour venue prints sixty-seven
billion an hour at period three on one bank's bid at any price, and bank.c pays 2.7e12 of wages a
period from period four; this change moved the path enough to cross the throw. A bank's bid for
hours is the labour venue's finding (12b.3), and `small-business.test.ts:316` is red for it.

**Checks.** `second-hand.test.ts` green (three: the vintage wins when a year of it asks less, the
new build when not, never both; the estate's ask is the print or the market); `check:opens` green;
lint, typecheck, spec, forbids, deaths green; the estate suite at its reds; small-business one
more red, above.

## Item 12a.9 — The tests; item 12a closed

Six things the item promised, measured. A failed levy is a row: the payer's arrear to the state,
class `tax`, written by settlement in the pass of the fail and named by it — and the agreement the
treasury wrote beside it since D-1 was the same debt twice (Law 4), so it is deleted and the tax
leg says what it is. A mortgage built in the scale model (no bank there quotes a household name,
12a.4) is charged on the roofs, serviced — interest and a slice of principal every period — and
when the cell's money goes elsewhere the default is recorded on the row, the lender forecloses
through the lien and holds the roofs, and the cell is in probate marked defaulted. A treasury
short in a foreign money fails and a backstop drawn is a row on both books (12a.6, 12a.7's
tests). `housing.shortfall` fires for fewer cells than there are, every period.

**Finding (positioned at 21.2).** Twelve levies at period three of the confiscatory scale model
failed because the payer's BANK was refused at the central bank, and no row was written on
anybody: settlement writes the payer's arrear only when the payer was refused. A bank refused at
the window has failed to deliver its customer's money; the row is the bank's.

**Item 12a closed.** The section is removed; its findings are positioned at 12b.1 (a cell fails as
one), 12b.3 (the banking venue's print), 17.0 (nobody quotes a household; a paper issuer's equity
reads nothing) and 21.2 (the bank refused at the window). Coverage 68.3%.

**Checks.** `agreement.test.ts` green (six), `mortgage.test.ts` green (four), `arrears.test.ts`
green (two); `check:opens` green; lint, typecheck, spec, forbids, deaths green.

## Item 12b.1 — The employment register has one home

The rows were the kernel's and the reads were not: who works where, for how many hours, at what
wage, since when, was indexed and summed inside the labour module, and every other employer read
its own payroll off a tally that module published once a period. The tally was a stored aggregate
of the register (Appendix B), re-derived every period and left standing when the rows changed —
which is how a firm that shed its last worker went on covering the payroll of nobody (0e′) — and
eight phases across four modules declared it as a read.

`register/employment.ts` is the noun's home now: the kind, declared by the kernel at assembly
beside the arrear; the terms — occupation, region, wage per hour, hours per member, since,
productive from, notice, headcount; and every read over the rows, on `ctx.employment` and
`view.employs()`. Wages are instructions that read it, one leg per row; what is due is the row's
and what was paid is the ledger's. The labour module keeps the trade each cell can work in (noun
`skill`) and nothing that is a copy of a row; `labour/register.ts` and the `labour.wages` tally are
deleted, and so is the `labour.wagesInArrears` agreement written beside settlement's arrear on a
failed wage — the same debt twice (Law 4). `banks/staff.ts` had no staff map left to delete: it
read the tally, and reads the register.

**Measured.** Three tests that were red are green — a buyout buyer that never met a payroll, a
small-firm cell's wage bill, the state's own payroll — and nothing new is red across the world,
firms, small-business, control, households, treasury and banks suites (31 red of 76, from 34).

**Checks.** `labour.test.ts` at its five reds (12b.3's); `check:opens` green; lint, typecheck,
spec, forbids, deaths green; `docs/ARCHITECTURE.md` carries the structural decision.

## Item 12b.2 — The venue matches net changes, and a cut is notice given

An employer posted the employment it wanted and the labour venue read its rows against the posting
to work out whether that was a hire or a cut — the venue deciding what the employer meant. Now the
employer posts the CHANGE against what it will have, through one read every employer makes
(`netChange` in the register): more hours is a bid at the wage it offers, fewer is a sell the venue
reads as a cut, nothing is a withdrawn vacancy (C5). Firms, small firms, a bank's two desks, a fund
manager and the treasury all post the same way.

A cut gives notice (C3). The row is restated to say how many are leaving and the period the notice
ends; nobody moves; the wage runs to the end because `payWages` reads the row; and when it runs
out `labour.release` separates the leavers with nothing more owed — paid to the end is the cost of
the firing. The lump severance is gone. An employer that has CEASED releases its people at once
(C4) and owes them the notice it could not run, as the unsecured claim on its estate.

**Found and fixed on the way (Law 5, A4.b).** `payFrom` paid a wage to every member of the worker
CELL rather than to the people on the ROW. The standing cell of the employed key holds every
employer's staff of that region, cohort and bank at once, so a firm with a row of ten paid two
hundred and ten — the treasury's hires in the same cell — twenty-one times its bill in the labour
scale model, every period. The wage is the row's headcount now. The `sb-found` world that threw in
period six at 12a.8 runs: that bill was this.

**Findings.** The merged cell itself is 12b.2a, inserted: the employed key must name the employer
so a cell is one row, which is what A4.c's findings in the labour tests are. With wages paid to the
row the treasury misses twelve coupons in period 52 of the `tsy-g` scale model (XI-9), re-measured
at 12b.6.

**Checks.** `labour.test.ts` at its five reds (A4.c's, 12b.2a); the notice test green with the audit
gate lifted (ten paid through four periods of notice, five separated with nothing owed, five paid
after); `check:opens` green; lint, typecheck, spec, forbids, deaths green; 41 red of 88 across the
world, firms, small-business, control, households, treasury, banks and funds suites, from 42.

## Item 12b.2a — The employed key names the job

A hire re-keyed the hired onto the standing cell of `employment = employed`, and the standing cell
of a key holds everybody on it: every employer's staff of a region, cohort and bank sat in one
cell, a row of ten on a cell of two hundred and ten, and the units family said so every period
(`Labour A4.c`) — the finding the labour scale model's tests were red on, and the cell 12b.2 found
paying twenty-one wage bills. The value of the dimension names the JOB now: who, in what trade,
hired when and in which round — the four things that make one row's terms — written once in the
register (`employedKey`) and spelled by the hire and by the lattice's opening rule. A second hire
into the same job in the same period lands on the same standing cell and moves the row's headcount
rather than opening a second row: a cell holds one job (B3).

**Measured.** The labour scale model's audit carries no Labour finding. What is left in its five
red tests is the world's own — prints held without a print (`Clearing F2`), accounts that do not
close per member (`Audit B5`), plant lines naming markets that do not exist (`Clearing D1`), flows
the weight events do not sum to (`Money D3`) — which is 12b.6's to read and Part XII's to measure.
Across the world, firms, small-business, control, households, treasury, banks and funds suites: 38
red of 88, from 41; the treasury's twelve missed coupons of 12b.2 are gone.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green; `lattice.test.ts`
at its one red (a basket, not a key).

## Item 12b.3 — A wage bid is from an outlook, and an ask is a cost

A bank's desk and a fund's manager bid an hour against `earned(1)` — last period's equity moves
by instructions — and one good week at a bank became a wage of sixty-seven billion an hour. They
bid against their own outlook on their EARNINGS now (§46: formed adaptively from what the book
made, at their own memory), which is what an employer weighs an hour against. A price a party
watches and did not trade at this period reaches it as the venue's print (A2.a: one more thing
observed, never the outlook itself), so a seller that sat out a week is not a seller that saw
nothing. And a firm's sell reservation is its COST — what each lot cost it, off the register —
never what it expects to fetch: the ask that came from an outlook was a price level made out of a
belief, asking the market to agree with it (F15). What it expects to fetch bids for hours and
sizes batches, and nothing else.

**Measured.** The banking venue prints 43 million an hour at period 4 of the `sb-found` scale
model and falls to 13 million by period 8. Six `small-business` scale models throw in period 4:
with asks at cost, a machinery maker asks 0.616 a piece where the print had been 630,879, a fuel
line's gap takes thirty-nine million pieces of it, and `capacityFrom` counts a capacity of 9.7e15
pieces a period, which `downTick` refuses past 2^53. The price level that belief was holding up is
12d.1's; a count of pieces past 2^53 is a grid and is 21.1's. Both are written there.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green; `labour.test.ts` at
its five reds, `expectations.test.ts` at its two (the baseline's); 44 red of 88 across the world,
firms, small-business, control, households, treasury, banks and funds suites, from 38 — the six are
the throw above.

## Item 12b.4 — Unsold hours are on the record

The labour venue matched whole people and said nothing about the rest: a session with seekers and
no employer returned before it wrote anything, and the hours a cleared book took that did not make
one more person vanished between the solver and the hire. Both are journaled now, `labour.unsold`
with the venue's own outcome — `noDemand`, `noSupply`, `noOverlap`, or a whole-person `remainder` —
and the hours offered and wanted, so a reader can tell a trade nobody hires in from one nobody
offers in. The print a reader faces was already its own region's since 12b.1.

**Measured.** Six periods of the scale model: 304 venue-sessions with seekers and no employer,
eight with employers and no seeker, seven whole-person remainders, eight prints. Most of this
world's labour venues are seekers waiting for a bid — the price level of 12b.3 and the buyers of
22.3, read against each other here.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green; `labour.test.ts`
at its five reds.

## Item 12b.5 — Analysts are staff

A research desk was paid by a transfer to every household that banked with the bank, split per
member, at the printed wage, for hours nobody had contracted — a wage with no job behind it. A
desk is people now: the bank posts for analysts in the `analysis` venue as its other desks post —
the change against what it has, for the hours the names it wanted last period take, at what an
hour is worth to it — pays them off the employment register with everybody else's wages, and covers
no more names than their hours reach; a name it cannot staff is dropped with that reason. The
desk payment and the family that watched it settle are deleted.

**Found and fixed on the way (Law 8).** A desk that needs thirty-two hours of a thirty-five-hour
week and posted thirty-two hired nobody: the fill did not make a person. Every employer's posting
now rounds up to whole people, through one helper in the registry that names the labour module's
hours per member for the purpose.

**Measured.** In the research scale model no bank wants a view of anybody — it holds nothing any
company issued — so nothing is initiated in sixty periods with or without this change (eight of
nine research tests red at the baseline, seven now). The fund managers hire analysts in the same
venue, and one that is the only bidder pays its whole bid — four analysts at 43.7 an hour, 6,116
out of 5,245, gone the period after: a desk is worth what it lets the manager keep, not everything
the pools pay it, and that is written under the asset managers' item (14); `funds.test.ts:227` is
red for it.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green; `labour.test.ts`
at its five reds.

## Item 12b.6 — The tests; item 12b closed

One register answers who works where: the agreement store, the employment reads, an employer's
rows and a worker's row are one answer, a cell's weight is its row's headcount (12b.2a), an
employer's payroll is its rows and nothing else, and every hire is on a row now or was separated
since. A bank that lost money this week posts the same thing for its desk as one that did not,
because the bid is its outlook and not the week (12b.3). A separation runs its notice with wages
due and ends with nothing more owed (12b.2) — the world-audit gate is lifted from that one test,
whose subject it is not.

**Item 12b closed.** The section is removed. Its findings are positioned: the price level and the
banking venue's print at 12d.1; the fund manager that bids its whole earnings at 14; the capacity
past 2^53 at 21.1; the service lines nobody buys from at 22.3. Two are closed by the item itself:
an employment row's worker is its creditor and never a term (12b.1), and the labour test that
divided a wage by a cell's weight divides by the row's people now that the cell is the row's
(12b.2a).

**Measured.** 56 red of 108 across the world, firms, small-business, control, households, treasury,
banks, funds, labour and research suites: the world's own audit findings, the price level, the
throw of 12b.3, the research rig where nobody wants a view, and the manager that bid everything.

**Checks.** `employment.test.ts` green (three); `check:opens` green; lint, typecheck, spec,
forbids, deaths green. Coverage 68.3%.

## Item 12c.1 — Hours per unit are a read of what the line has made

A recipe's hours per unit were a constant, scaled once per firm (Firm A3). Every recipe now
declares, as TECHNOLOGY, the rate at which its hours fall with cumulative output — Wright's curve,
an exponent on the pieces the line has made, its `why` naming the class of work it comes from:
grown or dug barely learns, a process learns a tenth a doubling, assembly a fifth, a service about
a fourteenth. A firm's hours per unit is a READ against that: the recipe's hours times what it
has made to the minus rate, the count off the ledger's own index over the create legs (the record,
never a level), through `view.made(instrument)`; a small firm's against what a member of the cell
has made. Nothing stores productivity, and an entrant's edge (12c.3) will be a different base, not
a different rate. The power is core's (`raised`): the forbids gate counts arithmetic outside it,
and the first commit of this item went in with that gate red — this record and the move are the
follow-up.

**Measured.** In eight periods of the scale model every firm that has produced takes fewer hours
a unit than its recipe says, exactly by the curve; a line with no learning takes the same hours
for ever. The four-country world and the rig run; nothing new is red across the ten suites (56 of
108).

**Checks.** `learning.test.ts` green (two); `check:opens` green; lint, typecheck, spec, forbids,
deaths green.

## Item 12c.2 — Learning travels with the people who did it

What a line had learned was the line's alone: a firm's hours per unit read its own ledger count,
and a baker who left took nothing with them. The employment row now carries what its people
BROUGHT — the pieces of the trade's output they had made at the employer they left, read at the
separation off that employer's own count over the heads in the trade, kept with the people while
they look, and moved onto the new row at the hire. A firm's hours per unit read over its rows: what
the line has made plus what its people brought, on the recipe's curve. The row carries a total, so
a second hire onto the same row adds and leavers take their share.

**Found and fixed on the way (XI-15).** The separated re-keyed onto the one standing `unemployed`
cell of their region, bank and cohort, so every separation's people shared a cell and the last to
arrive wrote its trade and its learning over everybody's — the same defect 12b.2a found on the
employed side. The unemployed key names the trade, the employer and the period they left; a cell
is one group with one history, and a fresh cell that never worked is `unemployed`.

**Measured.** Ten bakers who made three hundred loaves at one firm are cut, run their notice, and
are hired by another firm carrying three hundred loaves' worth of learning onto its row; the
people who came from nowhere in the trade bring nothing. Nothing new is red across the ten suites
(56 of 109).

**Checks.** `labour.test.ts` mobility test green; `check:opens` green; lint, typecheck, spec,
forbids, deaths green; `lattice.test.ts` at its one red.

## Item 12c.3 — Entrants enter with the line's practice

A firm born of a small firm that outgrew its tier drew the hours a unit takes it from the seed's
width, as if the day it was born were the day the world was seeded. It reads the line now: every
live named firm making its good in its region, each at what a unit takes it TODAY — its own scale
and what it has learned, the one read a firm's technology has — and takes one of those at or
above the median, whole, because the practice an entrant copies is a practice somebody has. The
draw's `why` in the parameter register says which. A line with nobody to see is entered from the
width, like the seed's own. Exit is the kernel's default and nothing was added for it.

**Found and fixed on the way (Law 8, 12b.5's claim).** 12b.5 said every employer's posting rounds
up to whole people. Firms, small firms and the treasury did not: a baker that needed fourteen hours
of a thirty-five-hour week posted fourteen, no whole person fits in fourteen, and it hired nobody —
every period, for a year, which is why the scale model's firms had no payroll. All three post
through the one registry helper now.

**Found and fixed on the way (Money E4, Register D5, Firm Birth D2).** A firm that died with
margin pledged to the clearing house left the pledged lots on its own dead book: the estate moved
what was free and skipped the rest, and the spoilage phase of the next period addressed a party
that had ceased and threw (period 42 of the `growth` rig). The estate takes pledged units with
their lien — release, move and a new lien on its own book to the same beneficiary for the same
reason — and the wire's precheck credits what an instruction frees to what it may deliver, so the
collateral is never briefly nobody's and the secured creditor ranks on the estate as it ranked on
the dead.

**Measured, and not measured.** The five-year run the item asks for cannot yet be taken: with the
two fixes above the `growth` rig runs to period 55 and stops there — the securities-lending desk
marks a treasury at this period before its market has run (`NotYetProduced`; positioned at 21.19)
— and in the fifty-four periods it does run its twelve firms produce once, in period 1, off the
seed's work in progress: every later plan is `batch 0, bound demand` while the seed's stock
exceeds what they expect to sell, and six of twelve are dead by period 52 (positioned at 22.2).
Output per hour has nothing to be measured on, and the size distribution's tail exponent is not
recorded: `growth.test.ts` carries both measurements and is red with those two reasons. The
entrant draw itself cannot be measured in the scale model either: the period after a promotion
throws at `capacityFrom`'s grid (21.1, the same throw that takes `promotion.test.ts`). Nothing new
is red across the ten suites (56 of 109, the same set).

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green.

## Item 12c closes

Productivity is an outcome: a recipe's hours fall with what the line has made (12c.1), the
learning travels with the people who did it (12c.2), and an entrant enters at the line's current
practice (12c.3). No stored productivity anywhere. The item's findings are positioned at 21.1
(the capacity grid), 21.19 (the mark before the market) and 22.2 (the firms that never start);
its section is removed and its row marked done.

## Item 12d.1 — What is public about what a party is exposed to reaches it

An outlook was formed from a party's own fills and receipts, and from the print of a line it had
once traded. What is public about the rest of its exposure reached nobody: a household holding a
share it had never traded saw no print of it, a worker saw no going rate, a depositor no board,
and a holder nothing of what the company it held published. The exposure set is a read of the
party's holdings and its rows — nothing here is a list anybody wrote — and every public fact about
it enters the outlook as one more observation, corrected towards at the party's own memory, never
as the outlook itself (§46 A2.a): the print of every line it holds; what a company whose paper it
holds published it earned a period, the period after it published, which is the lag a statement
has; what an hour cleared at in every venue its employment rows put it in, on either side, off the
going rate the labour module publishes about the period that closed; the board of the bank it
banks at, on its class of deposit. Three subjects joined the kernel's vocabulary for it
(`wage.<venue>`, `deposit.<bank>`, `reported.<company>`), and the reads go through the registry.

**Not built here.** The retail prints a cell buys at before it has ever bought, and the dwelling
print for a cell that rents: a basket is the households module's costing and a tenure row names a
venue, neither is a holding or a row naming the line. Both go with 12d.2's anchored predictors.

**Found on the way (XI-15).** The book of outlooks is keyed by party id and follows no cell event:
a cell split off its parent after a statement came out has a fresh book, although its people saw
the statement as members of the parent. Positioned at 21.20; the test skips those cells and says
so.

**Found and fixed on the way (the plan's own tool).** `plan:progress` reported plan item 12d as
closed with four open steps: the worklist's `12d` (the test migration, closed by the owner) shares
its id, and the tool let a worklist `done` speak for an item that has a section. An item with a
section is judged by its section alone now.

**Measured.** Twenty-two periods of the scale model: every party holds an outlook on every line it
held that printed this period; every party on an employment row holds one on the going rate where
it works or hires and a cell that never worked holds none; depositors hold their bank's board in
`per annum`; the holders of the one company that reported by period 20 hold what it published,
the period after, and nobody else does (`observation.test.ts`, four green). Nothing new is red:
the ten suites at 56 of 109 and `expectations.test.ts` at its two, the same sets.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green.

## Item 12d.2 — Two predictors, and the party follows the one that has surprised it less

A party had one way of expecting: its own history corrected at its own memory. It has two now.
Beside the adaptive predictor it keeps an anchored one — the last PUBLIC level of the variable,
where the variable has one: the venue's print for a price; a going rate, a board and a statement,
which are public and anchor themselves; nothing for its income, its earnings or its own sales.
Each predictor keeps its own track of surprises over the same memory, and at the top of the period
the party follows whichever track is narrower. It is a switch and never a blend: a weight between
the two would be a second primitive (§46 B1.b), and there is none. The switch is decided from
history only (B4) and left alone on a tie or while either track has nothing to compare. What the
outlook read returns, what its confidence is a read of, and what the published dispersion is a
dispersion of, are the followed predictor's. The surprise event records the surprise the party
took, the predictor it followed and both tracks' entries, so the choice replays off the journal.

**Measured — the killer held (Law 17).** Fifty-two periods of the scale model: every change in a
variable's published dispersion follows a surprise on that variable in the period the statistic
is about, or a party arriving in or leaving the aggregate; the aggregate never moved first (§46
E2). Some parties follow the public level and some their own history — the histories differ, so
the choices do (A3) — and the recorded choice replays exactly from the recorded tracks over every
party and every variable (`predictors.test.ts`, two green). Nothing new is red: the ten suites at
56 of 109 and `expectations.test.ts` at its two, the same sets.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green.

## Item 12d.3 — A cold week burns more

The household basket was costed at a normal week's quantities whatever the weather; the world
publishes every region's warmth every period and nobody in a house read it. A basket row now
declares what a member takes more of to stand against the weather — the two fuel rows name the
region's warmth, every other row nothing — and the period's need and want are the normal week's
over how the condition stands: a week at four fifths of its warmth burns a quarter more. The
condition is read from the cell's own view of the public event about its region, through the
registry (`conditionsSeen`, `conditionsStanding`) — the same product a crop's yield reads through
`conditionsFor` from a phase — and nothing in the households module parses the event.

**Positioned, not built here.** The item names three readers. The central bank's read belongs to
its outlook on the price level and the rate step (18a.1) and the insurers' to the buyer that bids
at its own outlook of the loss (14.2); neither decision exists yet, and a read into a decision
that does not exist is a read into nothing. Both lines carry the note and the door to use.

**Measured.** Six periods of the scale model, every living cell: the fuel line's need and want are
the row's litres over the week's warmth to twelve places; every other line is its normal week's;
the environment moved off normal in some week, so the read bit (`heating.test.ts`, one green).
Nothing new is red: the ten suites at 56 of 109, the same set.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green.

## Item 12d.4 — The no-view census of the goods books

The test is written and it is red, and what it measured is two things. Over thirty periods of the
scale model twenty-four of its hundred and twenty-six goods books ran without a counted view at
least once, ten in period thirty. The wholesale books — bread, flour, power, IT services,
facilities, chemicals, meat, cloth, steel — have firms bidding in them for inputs at what they
think the output will fetch less the wages a unit still needs, which is a view by the kernel's own
definition, and the census cannot count it: the mark is on the PARTICIPANT, one firm posts both its
input bids and its asks at cost, and the firms' participant is rightly not flagged. That is a
kernel repair — the mark belongs to the order — positioned at 21.22. The retail books of a place
with no household in it run with sellers and nobody to buy, thirty periods of thirty; that is
honest, and a fact about where the seed puts people (22.3).

**Checks.** `no-view.test.ts` red as measured; lint and typecheck green; no engine change, so the
opens check and the suites stand at 12d.3's.

## Item 12d closes

Observation: what is public about what a party is exposed to reaches its outlook (12d.1); a party
keeps two predictors and follows the one that has surprised it less, and the aggregate never moved
before the surprises (12d.2); a cold week burns more (12d.3); and the goods books' no-view census
is measured and red for two named reasons (12d.4). The item's findings are positioned at 21.1 (the
capacity grid), 21.20 (the book of outlooks and cell events), 21.22 (the view mark belongs to the
order) and 22.3 (the price level's asks, the banking venue, the shops where nobody lives); its
section is removed and its row marked done.

## Item 14.1 — An insurer opens with a surplus its savers subscribed

An insurer was created by its own module's seed with nothing — no cash, no line, no owner — and a
buyout fund called it for a commitment of forty to a hundred and forty million in period 1, so
every insurer in every world defaulted on the first call and went to its estate before it had
done anything. The sector was assembled and dead. The foundation seed now creates one insurer per
place that has a bank and endows its opening surplus in cash, a SHAPE per head of the region's
households with its death named (14.8, 22a), before the equity seed runs; the equity seed floats
its line like any company's — shares worth its book at the opening price, held by the savers at
cost, every piece with a holder — so its equity is a positive read from the first period and the
surplus is somebody's, subscribed, not money from nowhere (Seed A2). The manager of a pool bids
for analysts its expected fee income over the hours it needs, not its earnings.

**Found and fixed on the way.** Five defects, each in code no party had lived to reach: a
commitment is a share of what the investor holds when the promise is opened, never an amount drawn
blind; `funds.strike` and `insurers.allocate` read events they had not declared; a pool passed on
its whole account as a payout, so a subscriber's eighty million came straight back as a dividend
and went round the trackers at a NAV falling a decade a period until a household divided by
nothing (`seed-B`, period 19) — it passes on the dividends and interest that reached it, read off
the legs, and no more than it has; a named party's cease reseated its agreements but not the paper
it had issued, so the arrear a wound-up fund's last fee made was redeemed against a party that had
ceased (Register F2: live issued paper reseats to the successor, as a cell's does); a bank booked a
drawing to the party that drew it and not to the party it is now.

**Measured.** The insurer holds cash and owes nobody at the opening; its line is held by household
cells at cost, Σ held is what is issued, what they subscribed is its cash to within a piece a
member; it lives, allocates into the trackers and holds their shares through twelve periods
(`insurer-float.test.ts`). Three seeds that stopped at periods 19, 21 and 23 run a year. Eighteen
suites at 90 red of 196, as before: two greens gained (the treasury collects the tax on interest,
`expectations.test.ts` B3) and two lost — `equity-anchor.test.ts` has nothing to measure because
the one listed firm of the (4, 40) rig, which lived to report in period 29, now fails in period
21 unable to pay 44.8m that fell due; the phantom income removed from every saver's account was
holding it up (positioned at 22.3).

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green.

## Item 14.2 — The buyer of cover

Cover had a seller and no buyer: the insurers' book cleared only in a test that posted both sides
itself. A firm standing in the weather now bids for cover on what its plant is on its books at, at
the share of it the wind it expects would take over the term — read through the one survival
relation the weather itself scraps plant by, moved to the registry so there is one writer — as a
ladder down from that price out of the cash it holds; a cell bids for life cover on what a member
owes, at the chance a member dies within the term off its own outlook of its cohort's mortality.
Both expectations are §46 outlooks like any other: every party observes its region's published
weather every period and every cell its cohort's published deaths, adaptive at their own memory
and anchored on the public level. The names of cover are the registry's, the book is opened at
the seed so the buyers can post before the quote, and every session is on the record with its
outcome and who was in it.

**Found and fixed on the way (Law 8).** `rungsUpTo` made a count of what the money would take at a
rung before comparing it with what was wanted, and at a premium of a few millionths a unit the
count passed the grid: both worlds threw the first period a firm bid. It compares first now.

**Not built here.** The insurer's quote: with no claims experience and no debt-funded capital it
prices cover at nothing and writes none, so every session says `noSupply` with the buyers in it.
That is 14.4's return on the capital it holds against a premium.

**Measured.** Eight periods of the scale model: every living party holds an outlook on its region's
wind and every cell one on its cohort's mortality; every firm with plant records what it has at
risk, what it expects to lose — a share, never all — and its ladder; the bids are in the book
every period and nobody writes them (`cover.test.ts`, three green). Nothing new is red: eighteen
suites at 90 of 196, as at 14.1.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green.

## Item 14.3 — The claim

A policy promised a unit of money at the end of its term and nothing happened in between: the
weather took plant from parties holding cover and no claim was ever made, because nothing read
the loss and nothing paid on it. The capital programme's own event now says what the lost units
were on the holder's books at, the registry reads the period's losses, and the insurers' claims
phase — after the weather, at the same anchor — pays each covered loser what it lost, up to the
cover it holds, oldest cover first, and takes the cover used back in the same numbered instruction
(an asset leg to the issuer is a redemption, so the cover is gone). `insurer.claim` is that
instruction's event, paid or failed. A claim the insurer cannot pay fails on the wire like any
payment and its arrear ranks as a policyholder's; that is how an insurer dies of a storm (A3, B4).
One storm is one period's list of losses, and every line of it reaches the phase. `claim` is a
receipt class of its own and nobody's income.

**Measured.** Cover written by hand to a carrier for a million and a storm written by hand that
takes 137 million of its hull: the insurer pays the million, the policy's outstanding is nil, the
money out and the cover back are two legs of one instruction, and the claim is on the record
(`claim.test.ts`, one green). The rig's own storms are too gentle in eight periods to take a whole
piece of anything, and the rig's insurer quotes nothing yet (14.4), so the world's own claims wait
on both. Nothing new is red: eighteen suites at 90 of 196.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green.

## Item 14.4 — Experience, and the price of cover

The insurer's experience was the last claim it paid over the cover it had written, and its capital
cost it the share of its funding that was debt — nothing, for an insurer with no debt — so a fresh
insurer quoted nothing and no cover ever cleared in the world's own book. Every party with cover
outstanding now observes each period what a unit of its cover cost it in claims, read off its own
legs over the cover it has out, and forms its outlook of it like any other (§46; a period with no
claim is observed as one, so experience decays as well as rises). The quote is that outlook over
the term plus the return its capital requires — XI-4's read of what its capital costs per annum,
over the term's fraction of a year, on the surplus behind a unit of cover — and its capacity is its
surplus. A refusal to quote is a public event with its reason.

**Measured.** In `quote-37`, the one seed of forty whose insurer's line the draw listed, cover
clears in the world's own book from period 2 at a few hundredths a unit, the rig's own weather pays
four real claims in period 3, and after a hand-written storm the insurer's experience forms and it
quotes higher (`experience.test.ts`, two green). The hand-written cover test now sizes its buyer to
the insurer's standing quote. Nothing new is red: eighteen suites at 90 of 196, the same set.

**Finding (XI-4).** The listed insurer refused to quote in two periods of six for a price of
nothing: `requiredOnEquity` reads a company's cost of equity as its earnings yield, so one that has
not yet earned is told its equity costs nothing. Positioned at 21.23.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green.

## Item 14.5 — A policy is a row, and the kernel marks it

A policy was an instrument: issued in units, held by the buyer, marked at the present value of a
promise to pay a unit of money at the end of the term whether or not anything was lost — a
zero-coupon bond wearing an insurer's name, and one that paid twice once claims existed. It is a
row now: what a named insurer promises a named holder — so much cover, until a day, at a curve —
owing nothing until a loss makes a claim, and worth the insurer's own expected claims on it, its
`claims` outlook on the cover for every period left of the term, discounted at the curve the row
names. For that the kernel learned to mark rows: a kind may say what a live row of its is worth
now, and the revaluation moves the creditor's account up and the debtor's down by the change since
the last mark, in the same pass, each in its own money, unwinding the mark when the row ends;
nothing is stored but the last mark. Claims restate the cover down and end the row when it is
paid out; a term whose day has passed ends it. Fills are paired by the solver's allocation, every
buyer's fill spread over the insurers that filled the other side in proportion to what each wrote,
one premium and one row per pair in the same pass. The instrument kind and everything that read it
are deleted.

**Measured.** The four insurer suites are green (fourteen tests): the row opens with the premium,
a claim restates it and ends it when paid out, the insurer's experience forms off the claims it
paid, and the hand-written cover of the older test is a row the insurer owes and the buyer is owed.
Nothing new is red: eighteen suites at 90 of 196, the same set.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green.

## Item 14.6 — Pensions: the second profile behind the dispatch table

A pension fund is the sector's second party kind, and it differs from an insurer in exactly what
its profile says: it writes no cover, it fails on cash and not on solvency, and what stands behind
it when it is short is not its capital but its sponsors. One fund per place, made empty. Membership
is a fact about the person — a lattice dimension of the household, opened off the record like a
hire — and the worker cell of every employment row is enrolled whole the period its payroll first
runs, so a working member ages into a retired member and nobody the seed placed is promised what
nobody paid in for. Every employer with a payroll owes the fund of its place a sponsorship row, and
the wage instruction reads it: the wage net of the member's share, the member's share and the
employer's share, three legs in one instruction. A member cell that reaches the last cohort is
owed a pension row — a replacement share of what a week of the trade most of the place works in
earns now, per member per period, read at every payment — which the kernel marks at the members
alive times what one is expected to draw in each period to the end of the mortality table, decayed
at the fund's own outlook of the cohort's mortality, at the sovereign curve. The pension is paid
every period as income. The funding ratio is a read of the marks and the equity account, published
every period, and a fund whose equity is negative calls a recovery period's share of the shortfall
from the sponsors whose payrolls fed it, each in proportion; a call not paid is the kernel's arrear
on the sponsor. Three new names: the `pension` and `contribution` receipts, and the `pension`
lattice dimension. `RowValuationReads` grew the weight, the going rate and the params a schedule
reads; `AgreementReads` grew the mark.

**Measured.** `pensions.test.ts`, three green: the contribution legs ride the wage instruction and
nothing else reaches the fund; a hand-written crossing opens one row, the pension is paid at whole
pieces a member, the mark is positive and is what the funding event reports, equity is below nothing
and the ratio below one, the sponsors are called for at most a recovery period's share and every
call is money moved or an arrear owed; the schedule decays with mortality and a lower rate makes
the same promise worth more. In the rig's own draw the first member retires in period 15. Two
assertions were brought to the payroll's new shape — `insurers.test.ts` allowed the scheme's rules
as POLICY beside the module's one convention, and `labour.test.ts` counts the employer's share in
what a firing costs through the notice. Nothing new is red: eighteen suites at 90 of 196, the same
set; the nineteenth is green.

**Findings.** The sponsor's covenant is not on the sponsor's books (21.24); a promise indexed to a
trade nobody works in any more is worth nothing until somebody does (21.25). (The note here that the
fund "puts almost nothing to work" was wrong — it invests a period's contributions the period after
they arrive, the allocation sitting before the strike and the payroll after the markets — and 14.7
corrected it.)

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.

## Item 14.7 — What an institution keeps back is what it expects

An institution kept back its last claim and its last call: one storm and it sat on that much for
ever, no storm and it kept nothing. It keeps back three named reads of its own book now — the
claims it expects a period on the cover it has out (its `claims` outlook), the pensions its rows say
it pays next period (a schedule), and the calls it expects (a new own-observation subject, `called`:
an investor observes what the pools called of it each period, and once called, a quiet period is
observed as one, so the buffer decays as it rises) — and publishes them beside what it put to work.
A missed call is read for the period it was made in, every call of that period, through the
registry; a door is live only while its fund is; and a fund's longest promise reads its pension rows
to the end of the mortality table, so its duration test is against the long promise C2.a names.

**Measured.** `institutional-allocation.test.ts`, twelve green: every allocation puts to work exactly
the account less the three buffers through a living fund's door; a hand-written call the insurer
cannot meet forms its `called` outlook, it asks the pools it is in for what it missed the period
after at no price, and what it keeps back for calls thereafter is the outlook and never the call.
The 14.6 note that the pension fund put almost nothing to work was wrong and is corrected.
**Found and fixed on the way:** with the buffers changed, the money fund of a bank in `exp-f` winds
up at a NAV of nothing in period 16, and a redemption at that price divided by it and stopped the
world; a share struck at nothing is owed nothing now, said on the record, and why the pool's book
came to nothing is positioned at 21.26. Nothing new is red: eighteen suites at 90 of 196, the same
set.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.

## Item 14.8 — COVERAGE re-marked; item 14 closes

The twenty-three clauses of §27 read against the code: fourteen MET, five PARTIAL (B4 the insurer
pricing a correlation, C2 the one-way demand for long paper, C3 the illiquidity premium unmeasured,
C4 an institution lending its own paper when it holds through pools, D5 the hedge half of the
asymmetry), two OUT OF SCOPE with the item that owns them (C5 the mandate against a published
grade, 17.0 and 17.10; D4 the hedge, 18.3's hedger with the promise book as its target), and E2
built: the sector's `names` family now checks every unit an insurer or a fund holds is a live
instrument with an issuer or a thing by its profile, and E4 is the register's own ownership family.

**Item 14 closed.** The section is removed. Its findings are positioned: the listed company told its
equity costs nothing at 21.23; the sponsor's covenant off the sponsor's books at 21.24; a promise
indexed to a trade nobody works at 21.25; the money fund wound up at nothing at 21.26; the listed
firm of the (4, 40) rig dying at period 21 at 22.3. The opening surplus per head is a SHAPE with
its death at 22a, stated on the declaration.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green; eighteen
suites at 90 of 196, the same set.

## Item 15.1 — The ground has a seller in each place, a planning policy and a residual bid

A country's one treasury held every place's ground and sold it at one cent, and a firm with no
opinion bid one cent back. A LOCAL AUTHORITY present in each place holds what nobody has built on
there now, and offers a period what the planning policy says — parliament's number, a square
kilometre, never everything it holds — at its own outlook of what ground here has fetched, the last
print where it has none, and at whatever the book gives where nothing has ever fetched anything.
The firm's bid is its project's: the whole hectares the plant it wants would stand on, less what it
holds beyond its standing plant, at the residual — what a unit of capacity is worth to it over the
plant it takes, spread over the hectares a unit of capacity takes — and ground before plant. The
programme stands plant only on ground the firm holds, refuses and says the rest, pledges the ground
under a new vintage to the authority in the vintage's name and releases it when the vintage retires.
The seed states the plant and the ground under it together. Both one-cent asks are gone, and the
names of the ground are the registry's.

**Measured.** `land.test.ts`, six green: the authority holds the unbuilt ground and every firm what
its plant stands on; the authority sells at most the release a period and only its own ground; a
firm with ground and a machine commissions it with a lien on the hectares it takes, a firm with the
machine alone is refused and keeps the machine; nobody stands plant on ground it does not hold. In
the rig's own draw the first hectare clears in period 2 at a firm's residual bid. `capital.test.ts`
and `housing.test.ts` green before and after. Nothing new is red, and thirteen are newly green —
six of `small-business.test.ts`, six of `equity.test.ts`, one of `funds.test.ts` — with the one-cent
ground bids gone and the ground under the opening's plant its holder's: eighteen suites at 77 of
196 (Law 11: measured, not diagnosed).

**Findings.** The opening plant's ground is not pledged (21.27); refused machines are bought twice
(21.28); a pool's rooms stand on nobody's ground (21.29). The lettings finding moves to 15.5.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.

## Item 15.2 — A port has an owner and a berth

A vessel sailed region to region with no quay, no berth, no congestion and no owner. A place's quay
is its local authority's now, and it works so many berths a period — a call being a vessel loading
to sail from it or landing at it. The calls it has worked this period are read off the ledger's
settled voyage legs and never kept; a vessel arriving at a quay with every berth worked waits at
anchor, first sailed first alongside, and a cargo that cleared a session at a quay with no berth
left does not load. Every call turned away is public with the owner named. The berth count is a
placeholder with its death at 22a: a berth is capital the authority builds and wears out.

**Measured.** `freight.test.ts`, three new green: the berths are a placeholder naming their
mechanism; the call count reads sails and landings off a hand-made ledger; over eight periods no
quay works more calls than it has berths and every congestion names the owner. Three of the file's
older tests are red before and after this step (21.31). Nothing new is red in the eighteen suites.

**Findings.** Port dues have no cleared price (21.30); the three older freight reds (21.31).

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.

## Item 15.3 — Space built to let, a lease with a term, a landlord sector, a loan on the building

Nobody built space to let: a warehouse was plant a firm built for its own stock, and a retail firm
sold from nowhere. A `property` module now: landlords are a mass sector — cells with a lattice,
the third such profile — each opening with premises, the ground under them and a building's worth
of cash a member. A lease is a row with a term, signed in a lettings book per place where landlords
offer what they have not let at their reservation (the rent the book last struck, never below the
wear a unit costs them) and a firm whose plan binds on premises bids at what a piece earns it a
period. Leases are signed in whole units a landlord that pay each a whole piece; rents are collected
every period as rent; the tenant runs on the leased room as on its own; a lease ends when its day
passes or a side ceases. A landlord fully let prices a building at the rent over its horizon at its
own cost of capital, buys a building a member a period for what its money reaches, ground first,
and asks its bank for the rest secured on its premises through the one credit door.

**Measured.** `property.test.ts`, three green: the cells open holding premises, ground and cash; a
hand-posted tenant bid clears, leases sign in whole units a landlord, rent moves every period as a
`rent` receipt, the tenant's rented room is the leased units, and a two-period lease has ended by
the third period; a landlord fully let and short of its pace asks its bank secured on its premises.
Nothing new is red in the eighteen suites.

**Findings.** No firm's plan binds on premises in the rig, so no tenant bids of its own (21.32); a
bank resolution's merge leaves a lease below a piece a landlord (21.33); no builder sells buildings
in the rig, so no building is built to let after the opening (21.34).

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.

## Item 15.4 — A retail firm sells from a lease

A shop owned the room it sold from. Its recipe's premises are a LEASED need now — a project never
builds them and its bid for room is the lettings book's — and the foundation seeds the landlords of
each bank before the firms' plant, so a shop opens holding a lease of its room from the landlords
of its bank instead of the vintages: whole rooms a landlord, rounded up, at the landlords' cost a
room a period until the lease term's day. Two world stops were found and fixed on the way: the
kernel booked a cell's marks per member at the total over its lots (a landlord cell of two hundred
wore its buildings two hundred times a period and was insolvent in five; every household cell's
equity walk left its balance sheet by the weight on every mark), and the securities-lending module
marked paper at this period from a phase before its market ran (21.19, closed: every mark there is
the last print now).

**Measured.** `property.test.ts`, four green: the rig's three shops open leasing 400, 200 and 200
rooms at 159 a room a period from the landlords of their banks, own no premises, run on the leased
room and pay every rent while the landlords live. The eighteen suites, run in two halves: 73 red of
200, from 76 — four funds tests green now that a cell's marks are booked per member (the NAV falls
with the book, is a read of the book over the shares, pays the manager out of the holders, and
shows on the surface), and one red that was green: `control.test.ts` *bids for nothing* times out
at 180 s (21.36). A third stop was found on the changed path and fixed where it was: `banks.prime`
priced a broker's line off every published default, as every line the banks module writes does, and
had not declared the read — the research suite threw `Clearing F1.a` at the first default that ran
before it, and declares it now.

**Findings.** Household cells' equity walks still leave their balance sheets by the weight at the
settlement side (21.35). A dead firm's estate re-presents every claim on it each period of its
programme, each failed maturity writes an arrear, and each arrear matures, fails and writes another:
the control rig goes from 1,200 ledger records a period to 27,000 by period 26 (21.36).

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.

## Item 15.5 — Tenancy: a term, a landlord's view of its tenant, and dwellings behind the rental stock

The lettings book printed `noSupply` in every period because no dwelling existed: forty thousand
households, a dwelling line, and nobody holding a unit of it. The landlord sector (15.3) opens
holding dwellings of its place now and offers them in the same book, by the same reads, a household
does. A tenancy has a term, and the roof is re-let when its day passes. A landlord holds a view of
whether its tenant pays — the expectations module observes, for every money leg that pays what was
owed under a standing commitment, the share of what fell due that arrived, as `credit.<payer>` of the
party owed it — and ends the tenancy when the rent at that view is worth less to it than the wear
the tenant puts on the roof. The session signs at the solver's own price; everything in the module
is in the book's own unit, occupancy; the mortgage reads compare names, never prefixes; the register
indexes lines by kind.

**Measured.** `housing.test.ts`, eleven green, four new: every landlord opens holding dwellings and
every dwelling has a live holder; tenancies are signed at the level the book printed, run to a day,
and the rent moves tenant to landlord every period after; a tenancy of two periods ends on its day
and the tenant is re-let; a tenant drained of its money fails three rents, is in probate that period,
the landlord's view of it falls from 1 to 0.62 in two periods and it ends the three tenancies at
0.62 × 0.0032 < 0.0022 a piece, saying both numbers. In the rig's own draw the book clears in period
2 at 3,240 a dwelling a period for 7,996 dwellings, the landlords' whole stock is let by period 5,
and the book prints `noSupply` with bids standing after — a shortage. `property.test.ts` four green
(the CRE drain now runs just before the landlords ask, since dwelling rents refill them);
`mortgage.test.ts` buys the roof its row stands on by hand, and its *when it cannot pay* is red:
the bank now draws the roof-holding cell on its line when its account is emptied (21.40). Two stops found on the way and fixed where they were: the occupancy conversion published a short
of 2,112,800,000.0000002 pieces (Law 8: the counts are multiplied before they are divided now), and
an estate CEASED with a claim still standing on it when the holder could not hand the claim back —
its units were bound to somebody else, the write-off leg failed and `close` went on — so the next
maturity addressed a party that no longer existed (Money E4); an estate whose write-off does not
settle says so and stays open now, as it does with paper nobody bought. The eighteen suites, in two
halves: 73 red of 200, from 73 — *bids for nothing* (`control.test.ts`) green again at eleven
seconds, and one red that was green: the treasury *funds itself over a year when the market is
there* has eleven failed instructions in a year of the rig (21.41).

**Findings.** A renting household never buys (21.39); a singleton's rent is its whole income
(21.38); the dwelling good is on the tonne's grid (21.37); a roof-holding cell's default cannot be
staged from outside (21.40); the treasury fails eleven instructions in a year of the rig (21.41).

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.

## Item 15.6 — A region with people and no dwelling line is a seed finding at the seal

Goods are placed where firms were placed and people where the banks are, so a region with a bank
and no firm opened with households and no dwelling line, and nothing said so. The housing module
contributes a `names` audit family now: a peopled region with no dwelling line is a violation owned
by the region and sized in its people, reported at the seal (period 0) and never repaired; a
dwelling whose holder has ceased is a house without an owner (E1).

**Measured.** `housing.test.ts`, twelve green: the family is built at the seal, every peopled
region either has its line or is named, and asked about a world with no dwelling line anywhere the
check names every peopled region with its people.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.

## Item 15.7 — Housing COVERAGE re-marked; item 15 closes

Fifteen Housing clauses were MISSING at 15.6. Each was read against the code rather than against
the item's wish: five are MET by mechanisms that exist and are measured (C3 the loan-to-value read
at the lender, D1 the mark on a held dwelling reaching the cell's equity and its spending, E2 no
written price path, E3 no mortgage without the lender's account on the other side, E4 debt owed
equals assets held by the ownership family); seven are PARTIAL, built but unmeasured because no
builder sells and no owner buys in the rig (A5, B1, B3, B4, C6, D2, D4); three stay MISSING and say
where they go — B4.a and D5 are measurements for Part XII, D3 is the rent absent from the consumer
basket (21.42). The item said "built or OUT OF SCOPE"; none is out of scope, and the three that are
not built are named rather than deleted (Part II). Two findings surfaced by the read: rent is not in
the basket (21.42) and a dwelling cannot be maintained (21.43).

Item 15 closes: 15.1 the ground and its seller, 15.2 the port and its berth, 15.3 the lease and the
landlord, 15.4 the shop on a lease and the cells' marks, 15.5 the tenancy and the landlord's view,
15.6 the roofs family, 15.7 the re-mark. Its section is removed; its findings are positioned at
21.27–21.29 and 21.31–21.43; 12.2 closed at 15.5.

**Checks.** lint, typecheck, spec, forbids, deaths, existence, plan green.

## Item 16.0 — Money carries its currency

`Cash` was a count of pieces that did not know which money's pieces they were, so two currencies
added by arithmetic that typechecked (K2, A-23 and its siblings). It is a value now — `{ pieces,
ccy }` — and `plus`, `minus`, `sumCash`, `atMostCash`, `atLeastCash` and `ratioOf` refuse two
currencies where they meet with `Impossible('Money A2.b')`; `valueAt` takes the instrument's money,
`heldAsMoney` and `asCash` take theirs, and there is no constructor without one (Currency A3, A4).
The journal refuses a `Cash` value in event data: money on the record is its pieces beside a named
`ccy` (Law 8), and every reader of a money event requires the `ccy` it names — `credit.quoted`,
`fund.called`, `reporting.guidance`, `research.estimate`, `bank.costOfFunds`, `firms.funding`,
`households.plan` and eighty-four other fields now publish it.

**The owner's correction, taken as the design rule.** A party's balance sheet is NOT in one money.
Each party holds an account per currency at its bank (Currency B2.a); nothing converts at the ledger
boundary (B3) — a party that wants its own money sells the other in the spot book, or keeps it, or
hedges it; a numéraire is for a REPORT only (C4). So `inMoney`/`inOwnMoney` are translations at the
rate in force, used where a report, a mark or a size in one money is what is wanted and nowhere
else: a balance sheet and its dust (`accounts`), an index level (`readIndex` translates each line
into the money the rule is stated in — `IndexDecl.ccy`, new — so a size-weighted basket no longer
scales a COUNT by a rate), a sector total (`observer`, in the first region's money), a country's
external accounts (`external`), a dealer's book against its treasury's want, a bank's capital behind
a book in a foreign money (`costOfFunds`), a paper buyer's concentration room, a bond-futures desk's
book. A central bank's window lends its own money against paper in its money and nothing else
(Currency B4: `windowAdvances`); a firm's promotion boundary is kept per money (21.44). A test's
`phx(x)` is USD.

**Measured.** Not yet: the owner's rule from 16.0 on is that the eighteen suites run at the end of a
major item (16, 17, …), not after each step; `check:opens` is the per-step measurement. What the
partial run before the rule showed is on the record: `world.test.ts` fourteen red of twenty-one, all
in the baseline but one — *is reproducible from the seed value* threw on a guidance record with no
`ccy`, fixed in the same change; `small-business.test.ts` two red of twenty, one new — *fails
together, by region* threw at the central bank's window summing collateral in two moneys, fixed in
the same change (a window lends its own money against paper in it, Currency B4). The rig's period
zero has twenty-one violations, all in the baseline: ten held equity lines with no print (Clearing
F2) and eleven plant instruments naming a market that does not exist (Clearing D1).

**Findings.** The promotion boundary is per money (21.44); the observer's and the global index's
reporting money are declared in code rather than as a RESOLUTION with an invariance test (21.45); a
bank's room behind a foreign book moves with the rate and nothing hedges it (21.46).

**Checks.** `check:opens` green (the rig thirty periods, the four-country world twelve); lint,
typecheck, spec, forbids, deaths, existence, plan green. `check:forbids` also caught a cross-module
read `prettier` had hidden on two lines (`cds/index.ts` reading `credit.default` by name); it reads
`creditDefaults` from the registry now.

## Item 16.1 — Every currency literal gone

The engine named a dollar in five places that were not a registry declaration: the seed's `priced()`
put every opening level on the USD grid whichever region's line it was (a euro good's opening
price was on the dollar's grid); the indices module defaulted the global line's money to `'USD'`;
the CDS roll picked the FIRST currency in the registry as the money of every series and every
name in it, so a euro name sat in a dollar series clearing at the dollar house; an M&A violation
was sized in dollars when it counts parties; a fund parameter's unit said "one USD". The grep the
item names returns registry declarations only now: the four currency rows of the seed, the
money-market's per-currency data, and the doors where a `ccy` a writer published re-enters the
type. A series is per grade AND per money, named `grade.ccy.period`, at that money's house (18.5
carries the rest of the series design); the global index is stated in the seed's first country's
money, declared once in the seed and passed to the module rather than assumed by it (Currency C4).

**Checks.** `check:opens` green; lint, typecheck, spec, forbids green. Suites at the end of item 16.

## Item 16.2 — The four-country world, thirty periods, tabled

`abroadWorld('abroad-16-2')` steps thirty periods in 84 s; the table per money is in
`docs/IMPLEMENTATION.md` 16.2. Two stops on the way, fixed where they were: a fund seeded in kind
with lines in two moneys summed them and a bank drawn to hold a foreign line added it to its dollar
book (both translated into the party's own money for the report, Currency C4, Money A2.b); and a
second stock loan between the same lender and borrower on the same line pledged under the same
reason as the first, so `lienFor` gave every loan the FIRST lien's id (three rows named lien 101; the
register held 102 and 104) and the return of the second threw `Register D5` — a loan's lien is the
one no open loan already holds (Law 4). Finding 21.47: one session in a hundred clears and a
hundred-odd parties per money are dead by period thirty, with the US treasury refused an overdraft
of 1.6 × 10¹¹ pieces every period from 29.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids green. Suites at the end of item 16.

## Item 16.3 — Sourcing across regions

A cargo goes to ONE place: `freight.session` sends what a place has to ship down the leg with the
widest gap between the far print and the home print, instead of bidding the same unsold stock for
room on every leg out of the port at once (Cross-Border B1, Freight C1.a); a leg out of a place that
has something to ship still runs and says `noDemand` when every cargo chose another (Clearing C4.b).
A merchant bids the far print LESS ITS OWN OUTLOOK OF THE FREIGHT on that leg — a new outlook
subject, `freight.from.to`, observed by every party from the leg's session print each period (§46
A2.a) and read through `registry/ports.ts freightRateOn` where it has none yet — and its appetite is
one budget spread across the portable lines it buys in, not that budget again in every book (Freight
C3, Law 2). A merchant that cannot say what the passage costs does not bid for that destination (App
A). The freight session's name moved to the registry so the expectations module reads the rate by a
registry read and not by another module's event name (0e′.3). Finding 21.48: three freight tests
were red before this change and are red after it, and say so.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids green; `freight.test.ts` 13 green,
3 red as at 15.7. Suites at the end of item 16.

## Item 16.4 — Foreign-currency issuance: the treasury half

A treasury borrows where it reads the cost lowest (Sovereign A4.b, B2, Cross-Border C2). At home
that is its own curve at the tenor its mix wants; in another money it has no curve of its own, so it
opens at that money's sovereign benchmark (Sovereign D4) and what the borrowing costs it is the
benchmark yield plus what it expects its own money to do against that one over a year, off its own
outlook of the pair's print — every party now observes the pairs between its money and every other,
each period (§46 A2.a, Currency B1.a). A treasury with no view of a pair has no number to compare
and stays at home (App A). What it must raise abroad is its need at the rate in force, the line
carries the borrowing money's own convention, and the proceeds land in its account in that money;
it owes a money it cannot create and fails in it as 12a.6 made it (G2). The first four-country run
picked EUR at once (21.50). The firm half — a firm choosing its money by the lenders and the cost it
reads in each — is 17.4's and is positioned there (21.49).

**Checks.** `check:opens` green; lint, typecheck, spec, forbids green; `treasury-default.test.ts`
green; `treasury.test.ts` five red, all in the baseline. Suites at the end of item 16.

## Item 16.5 — The swap line, and one instruction from three books

**`transact`.** A participant declares a group of books for the period (`ctx.transact(party,
markets)`); its fills in each of them are drafted by the session and handed to the kernel instead of
settling one by one, and after the last book has run the kernel settles every leg from every book as
ONE numbered instruction — or, when a book did not fill it or the joint instruction fails, nothing
in any of them, and `transact.unfilled` / `transact.failed` say so (Spot FX A1, C2.a, XI-5). The FX
desk's round trip through three pairs is the first user: it is atomic because it is one instruction,
and its size is now the least of what it will risk and what it HOLDS of the money it hands over on
each leg, read off its accounts. A book whose only trades were a trip's legs prints when the trip
settles, at the level it cleared with the volume that moved, or carries the last print saying
`nothingSettled` — a print is what somebody paid (Law 3); a book that also settled ordinary trades
printed those, and the trip's volume there is on the record (21.51).

**The swap line.** A bank short of a money it does not book in has no window in it (a central bank
lends its own system); it has its OWN central bank, which draws that money on its swap line with the
issuer, hands its own money across at the rate in force, and lends the foreign money on at the
foreign window's ceiling plus its penalty — two dated rows on two central banks' books, and the
line's size is the term of an agreement two institutions set (`centralBank.swapLine.size`, POLICY;
Central Bank A2.b, Currency B4, E4). A draw past the line is refused and the refusal says where the
line stood.

Two stops on the way, fixed where they were: a fund manager compared rivals' books across two
moneys (read in its own now, Currency C4); and `world.ts` was truncated by an editing slip and
restored from the commit before the edits were re-applied.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids green; `spot-fx.test.ts`,
`currency.test.ts`, `money-market.test.ts` at their reds of 15.7 (none new; *carries the last real
price, visibly stale* went red mid-item when a pending book wrote a print with nothing settled, and
is green with the print written when the trip settles). Suites at the end of item 16.

## Item 16.6 — FX participation is a question of every kind

Owing a money it has not got, or holding one it does not want, is a reason ANY party can have, so
the spot-FX module asks it of every party kind the registry knows rather than of three named kinds:
a participant declared for `EVERY_PARTY_KIND` is expanded at assembly into one declaration per kind
(Law 15), and a party that runs a desk is not asked twice because its rows say it runs one, not
because of what kind it is. A party with no balance in the money and no live line it owes in it is
not walked for its dues (Law 18). Funds, estates, insurers and clearing houses square their foreign
balances now, and the first consequence is on the record: the USD clearing house ceases by period 2
to 4 of the four-country worlds (21.53), and a member defaulting into it tore up against the dead
name — margin now returns to a side's living successor (Money E4). A firm whose short came to
10²² units is refused at the site and said, rather than stopping the world at the tick (21.52).

**Checks.** `check:opens` green; lint, typecheck, spec, forbids green; `spot-fx`, `currency`,
`money-market` suites at their 19 reds of 15.7, none new. Suites at the end of item 16.

## Item 16.7 — A central bank holds abroad what it bought

The seed stated eight per cent of every central bank's reserves as another country's paper
(`seed.crossHoldingShare`, a placeholder whose named mechanism was Central Bank F4) and planted the
holdings and their cross-currency coupons at the opening. The share is deleted with the block that
spent it, and no central bank opens holding another money's paper: what it holds abroad is what it
BOUGHT, and nothing in this world yet gives it a reason to buy (Central Bank F1, F2 are a reserve
target and an intervention rule, both a mandate's decisions, positioned at 18a.1 as 21.54). The
`world.test.ts` placeholder census falls from nine to eight. The other half of this item — the
opening share of its own sovereign's paper a central bank holds (`seed.centralBank.openingHoldingShare`,
a POLICY under Central Bank C1.a) — is kept: it is the size of the central bank's balance sheet the
policy opens with, and 18a.3's open-market operations are what move it thereafter.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, existence green. Suites at the end
of item 16.

## Item 16.8 — Cross-Border re-marked; item 16 closes

Every Cross-Border clause is re-read against the tree at 16.7 and re-marked: 20 MET, 5 PARTIAL
(A2.a the corporate's hedge, A4, B3.a, C2 the firm half at 17.4, C3, D6, F1), 1 MISSING (C4 — the
leveraged buyout's foreign tranche, 17b); four rows the coverage lacked are added (A2.a, B3.a, D3.a,
D4.a). Central Bank F1 is annotated to 21.54. `check:opens` now prints how many FX pairs cleared
alongside the census, and the four-country `opens` world at twelve periods reads:

| period | pairs cleared | swap-line draws | cargoes | transact trips |
|---|---|---|---|---|
| 1 | 3 of 6 | 0 | 0 | 0 |
| 2 | 4 of 6 | 5 | 0 | 1 |
| 3 | 6 of 6 | 2 | 0 | 1 |
| 4 | 6 of 6 | 4 | 0 | 2 |
| 5–7 | 3, 3, 2 of 6 | 2, 3, 3 | 0 | 0 |
| 8–12 | 0 of 6 | 6, 5, 5, 5, 5 | 0 | 0 |

That is a measurement, not a diagnosis (Law 11): the pairs stop clearing from period 8 while five
banks a period draw on the swap line to settle what they owe abroad, and no cargo sails in twelve
periods — written as 21.55 and positioned at 18a.1 with 21.54, since a world where no central bank
buys abroad has one fewer side in every pair. Item 16 closes: its section is removed from
`docs/IMPLEMENTATION.md`; its findings are 21.44–21.55; 16.7's other half sits at 18a.3, the firm
half of 16.4 at 17.4, Cross-Border C4 at 17b.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence, plan green.
**Measured.** The eighteen suites run at the close of this major item; their result is the next
entry.

## Item 17.0a — Every company prepares full financials; publication and disclosure are separate acts

The owner's rule: *"each and every company prepares full quarterly financials and then they are
published or shared only when necessary"*, after two corrections — a lender reads a borrower's
financials and the price of its traded paper, not only its own history of it; and *"you need a
better way to screen who has access to what information"*.

**What was there.** Two tiers: PUBLIC events and a party's OWN view. The kernel already carried the
primitive for a third — `journal.visibleTo` returns public events plus the private ones a party is a
subject of — and nothing wrote one. So modules peeked: `ctx.participant(<another party>)` at 96 sites
in 41 files. The assessor graded every issuer from its private `earned`, `owedIn` and failed
payments. `Reporting A1.a` made a company public by LISTED SHARES only, so a firm whose bonds the
world holds published nothing, and the report was a trial balance — the equity movement grouped by
instruction cause, a balance sheet, cash by counterparty, shares.

**The three tiers now.** PUBLIC is a read of the register, widened to what A1.a actually says: any
market-priced claim the company issued that somebody outside holds, so a bond-only issuer publishes.
SHOWN is `ctx.disclose(from, to, event)` — the kernel records that one party showed another one of
its own events, private to the two of them, read back with `view.disclosedToMe(kind, from)`; it
refuses to show an event its shower is not a subject of. The reporting module shows a statement to
every lender of record and to the banks that keep the company's accounts; the ratings module to the
assessor the issuer pays (Ratings A5: that is what the fee buys, and an issuer that has prepared no
statement is UNRATED); the kernel to whoever a borrower asks, because `CreditRequest` now names the
statement opened with the ask. OWN is unchanged. A `PEEK` ratchet in `check:forbids` holds the count
of cross-party view reads per file and it may only fall.

**What a full statement is.** `docs/IMPLEMENTATION.md` 17.0a tables every section against what in
this world produces it. Written: the income statement by what the money WAS — the receipt the wire
itself wrote on each leg, so revenue, wages, rent, interest, tax, claims, dividends, production,
wear, spoilage, write-offs and the marks are named lines that sum to the equity account's movement;
comprehensive income, the same marks cut by what was marked; the balance sheet with cash, securities
at their marks, inventories at cost, plant by vintage with its accumulated wear, debt at face, lease
rent and the kernel's mark on every standing commitment; the changes in equity, whose identity is
opening plus earned equals closing; the cash statement both classified (operating, capex,
securities, interest, principal, financing, opening and closing cash) and by named counterparty;
segments by product line and by geography; a debt schedule with each line's next payment, last print
and holders; the leases; the commitments as debtor and the guarantees as creditor; every open
derivative with its counterparty, notional, value and margin; the deals; the employees by trade and
place; the share note; the related parties; the subsequent events; what the party published about
its own regulation; the management's own outlooks; and EBITDA, free cash flow, debt service and net
debt, each a sum of named lines so a covenant and a reader read one sum.

Every position carries its FAIR-VALUE LEVEL, read off the print's provenance — printed this period,
a print carried forward, or carried at cost. NO PER-SHARE FIGURE is stored (G5): earnings per share
is income over shares, both published, and a reader divides. ABSENT and stated so, with the item
that builds each: receivables and payables (17.5), undrawn commitments (17.3), goodwill and
intangibles and a corporate profit tax with its deferred tax (19, the polity).

**One parse.** `journal/published.ts` already held the kernel's typed read of the report and threw
on a field its writer had not filled. It keeps that job; what a line MEANS now lives once in
`registry/statements.ts`, which the writer writes from and every reader reads through.

**Two phases ran for the first time.** `covenant.test` and `control.tender` had never been reached
in any period of any world, because both need the target's published accounts and only listed
companies had any. Both declared their module's whole read set as a placeholder for their own, with
a comment saying it would narrow the first time the phase ran. It ran; they are narrowed to what
they read.

**The credit view reads them.** `banks/credit-view.ts` replaces `banks/quote.ts`: one view per bank
per money per period, read by the quote, the overdraft's row, the provision, the refusal and the
published reservation alike (C4). Its default frequency is annualised off the calendar; its loss
given default is the bank's OWN recoveries on estates it was a creditor of, walked once over settled
periods (a bank that has met no estate treats all of a claim as at risk, which is ignorance and not
a stated recovery rate); the capital charge is at the GRADE the assessors publish on the name, asked
through one `weightOfName` that the capital position uses too — so a downgrade consumes a bank's
capital without the bank doing anything (Ratings C2). Three new reasons to decline, each said by
name so declined volume is visible (C3.a): a name that prepared books and did not open them; a name
whose earnings did not cover its debt service; and a name whose own paper the market already prices
above the rate this bank would lend at — *nobody lends at six to a name whose bonds yield fifteen,
it buys the bonds*.

**And the floor under a bank's capital is gone.** `atLeastCash(capital, 0)` in `costOfFunds` hid
insolvency as free capital (Law 6). An insolvent bank now says so — `bank.insolvent`, public, under
its own name — and quotes nothing, which is what a bank in the hands of its resolver does.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green;
`reporting` and `published` suites green (25 tests). The eighteen suites run at the close of item 17.

## Item 17.0 — The credit view

`banks/quote.ts` is deleted and `banks/credit-view.ts` stands where it did. What changed is not the
four terms — a bank's cost of funds, what it expects to lose, the capital the claim consumes and
what running a loan costs it (C1) — but that there is now ONE of them per bank per money per
period, memoised at the module and read by the loan quote, the overdraft's row, the provision, the
refusal and the published reservation alike. C4 says one default-probability model per borrower;
what it is really about is that a bank cannot price a name one way and provision it another, and
the only way to be sure of that is for there to be one derivation with one reader's door.

Four things in it are new. The **default frequency** is annualised off the calendar rather than
counted in periods, so a memory of six weeks and a memory of a year mean what they say (Law 8). The
**loss given default** is the bank's OWN recoveries — every claim it handed to an estate, and what
that estate paid for it against what it wrote off, walked once over settled periods and held as a
memo of that walk. A bank that has met no estate treats the whole of a claim as at risk, which is
what ignorance honestly says rather than a fixed recovery rate (Appendix B forbids one), and the
first estate that pays it moves it. The **capital charge** is at the grade the assessors publish on
the name (Ratings C2), through one `weightOfName` that the bank's own capital position asks too — so
a downgrade consumes a bank's capital without the bank doing anything, which is what C2 is for, and
the charge a borrower pays and the capital the bank holds cannot disagree. And **E5.b is no longer
empty**: what a bank requires to hold a name's paper now carries its expected loss on that name,
because the assessment it needed — somebody's opinion, named and wrong-able (A4) — exists.

The reasons to decline arrived with 17.0a and are the item's other half: a name that prepared books
and did not open them, a name whose earnings did not cover its debt service, and a name whose own
paper the market already prices above the rate this bank would lend at. Each is said by name in
`credit.declined`, so declined volume is visible (C3.a).

`atLeastCash(capital, 0)` at `banks/index.ts` is deleted with the item, and the state it hid is
said: an insolvent bank publishes `bank.insolvent` under its own name and does not quote, because
there is nothing for its owners to require a return on and no book its liabilities fund (Law 6: the
state is the answer, not a floor under it).

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. The
`loans` suite: four new tests for the view, all green — two names one bank has watched differently
price apart; two banks that have recovered differently quote one name apart; a name that kept its
books shut is declined where one too young to have any is quoted; and coverage below one and a
market yield above the rate each decline with their own reason. Its seven standing reds are the
same seven, in the same order, as at the close of item 16.

**Left for the close of item 17**, where the suites run: the two stale assertions in
`securitisation.test.ts` and the two findings at 12a.4 and 12a.7, all four parked at this item to be
re-read against the view rather than against the world that had none.

## Item 17.1 — The leveraged loan: a coupon that is a margin over a transacted rate

B4 says a corporate line is fixed or floating and that *"floating is the norm in the loan market"*;
N5.b says what floating IS — a margin over a named reference that is itself observable and
transacted. This world already produced such a reference: the overnight book's own cleared fixing,
published as `index.benchmark` by the index system, with a money whose book never traded having no
benchmark at all. So the second kind is `corporate.leveragedLoan`, and what it promises is a margin
over that.

**Fixing is the kernel's write and the module's decision.** A kind declares `floats`; a module reads
what the benchmark PUBLISHED and calls `ctx.fixCoupon`, which refuses a kind that did not declare it,
writes the rate in force onto the terms and announces `coupon.fixed` publicly — so what falls due,
what has accrued and what a holder thinks the line is worth are read off ONE number (Law 4) and
nobody re-derives a fixing. What it paid before is settled and in the ledger. A line whose benchmark
has not fixed since it was drawn keeps the rate it was drawn at: the money market did not trade, and
inventing a rate for it would be the posted benchmark Appendix B forbids.

**What a holder projects is stated rather than hidden.** Beyond the current accrual period the
schedule is every remaining coupon AT THE RATE IN FORCE — what the line pays if the rate never moves
again. That is not a forecast, and nothing in the module has one.

**Fixed or floating is a decision (A2.c).** The issuer compares the coupon it would have to lock
today against the rate in force plus the margin it would promise, carried at ITS OWN outlook of that
rate — a new subject, `rate.<benchmark>`, which every party forms from the fixings it observes
(§46 A2). Two firms with different views choose differently on one day, which is what makes the mix
of debt kinds an outcome rather than an assignment. A firm with no view has what the rate IS.

**One placeholder, with its death named.** What a firm promises over the reference is
`corporateBond.margin` (three per cent), and it dies at 17.2 where an arranger builds a book and the
margin is what the book strikes. The module's other number is the tenor, a technology.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green. `corporate-bond`:
four new tests green — the coupon is the fixing plus the margin and nothing else; a firm floats when
floating is cheaper on its own view and fixes when it is not; a brought line is named by its margin
and every fixing on the record is a published rate plus that line's own margin; and the two kinds
differ in exactly one declaration, which is what lets the kernel refuse to fix a locked coupon. Two
tests were updated because the module changed under them: it now declares a second number (the
placeholder) and runs a third phase (`loan.fix`). The suite's two standing reds are unchanged and
are written up as 21.58 — no firm in that world brings paper at all, which was true at the close of
item 16 and is where 17.2 lands.

## Item 17.2 — The arranger: who brings a deal, what it is paid, and what it is left holding

C1 says a new issue is BROUGHT by a named underwriter, appointed and paid. Until now a firm walked
into a market on its own, and the eleven clauses behind C1 — the fee, the risk between commitment
and placement, the syndicate, the two bases — had nothing to attach to.

**Two bases, two prices, and the price is the risk.** On a BEST-EFFORT basis the bank is an agent:
it commits nothing, what the book does not take is never issued, and the issuer keeps the placement
risk (C11.a, C11.d). On a BACKSTOPPED basis the members commit, and after the session they take up
what the book did not, at the price the book struck (C7, C7.a). What the backstop costs MORE is
exactly what the risk costs the bank: what it requires per annum to hold that issuer's paper — its
own published credit view (E5, and 17.0's one derivation) — over the length of the placement. So
C11.e's "the backstop fee exceeds the best-effort fee" is a consequence of the arithmetic, and
C7.b's relation between fee and risk is the identity itself, not a rule anybody wrote.

**Which basis is the issuer's decision (C11.c),** and it is a distinction the issue path already
made: a firm coming to market because the market is CHEAPER than its bank is choosing between two
channels and can take a smaller deal, so it goes best effort and saves the fee; a firm whose bank
will not lend it ENOUGH has no alternative at any price and buys the backstop. Nothing is drawn.

**The syndicate is limits meeting a size (C10).** The lead takes what its own dealing line was
allotted and names members, keenest first, each within ITS OWN room (C10.b) — and what the willing
members can carry between them is what the deal is brought at, so a bigger one is DOWNSIZED and the
record carries what the issuer wanted beside what it brought (C10.c). The shares are struck before
the book opens and each member's share of what the book left is split on the kernel's own pro-rata,
in whole units summing to exactly what was unplaced, so no unit of an issue is left with nobody.

**Everything an issuer reads about a bank is public.** What it charges is published under its own
name (`bank.underwriting`, a PREFERENCE drawn per bank — its own price for its own people's work);
what it will commit is its dealing line's allotted room (`bank.lines`), read at the last close, which
is what an underwriter actually knows when it agrees to a deal; what the risk costs it is its
published reservation on the name. Three public facts, composed by whoever is asking, and no view of
a bank is taken anywhere.

**And the margin placeholder is dead, as this item promised.** What a floating line pays over its
reference is the keenest holder's requirement LESS what the reference is fixing at — both published
(E5.d and Indices A1). `corporateBond.margin` is gone, the module declares one number again (the
tenor, a technology), and the count of shapes falls by one.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.
`corporate-bond`: six new tests. The syndicate's limits are tested directly — three banks with 40,
30 and 20 units of room against a deal of 100 take exactly their rooms and bring 90 — because the
tests that read a RUN are vacant while no firm in that world brings paper at all (21.58), and a test
that iterates an empty list asserts nothing. The suite's two standing reds are unchanged.

## Item 17.3 — The committed facility: the lender sets the line

`shortTermDebt.line` was a tenth of the BORROWER's own book — a placeholder that named this item and
made the size of a committed facility a fact about the borrower's equity rather than a decision its
lender took. A bank out of capital went on committing lines; a borrower with a big balance sheet got
a big one from a bank that would not have lent it a penny.

It is deleted. C9's line is granted by the bank that QUOTED the name, at the margin that bank
quoted, for what that bank published it will have out to it — the least of what its capital leaves
it, what its own limit for one name allows and what its funding can carry, which is 17.0's one
credit view read through the quote it already published. One line per lender per borrower, and the
lender is a lender that actually wanted the name.

**An undrawn commitment consumes capital (A3.a).** A committed line cannot be refused when it is
drawn, so the capital stands behind it before it is — at the standard-setter's conversion factor,
`regulation.creditConversion.undrawn`, a POLICY about a promise (a half: less than the loan it would
become, because not every line is drawn). What is undrawn is asked of the commitment KIND itself,
because only the kind knows what its own limit is and what drawing on it looks like (Law 15), and
the kind reads the drawing where every claim lives — the register — rather than mirroring it onto
the row (Law 19). The kernel supplies the world the kind is asked with, one statement of it shared
with revaluation, so a kind valuing a row and a reader asking what is undrawn cannot be asking two
different worlds.

**And it is visible (A3.b).** `bank.capital` carries what the bank has committed and what those
commitments weigh, so a reader can see that a facility costs it something before it is drawn —
which is the whole clause: *a facility that costs nothing until drawn is a free option the bank did
not sell.*

**Not done, and named:** `ctx.restateInstrument(id, terms)` for rolling a maturity while the
relationship performs. The item listed it; it is a kernel door for a different mechanism (a roll
rather than a commitment) and it is left open — written up as 21.59 and positioned at 17.7, where
restructuring is the subject and an agreement transition is already the work.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.
`short-term-debt`: four new tests, all thirteen green — the line is the quoting lender's at what it
published; the module declares no line size and no placeholder at all; the commitment consumes less
capital than a drawn loan would and says how much; and the headroom is the limit less what was
drawn, answered by the kind.

## Item 17.4 — Issuing reasons: a target, a service, and a tap that taps

**A2.b: the target, and not one number was declared for it.** *"A target or a constraint it is
managing towards... that target is the management's own: a lender's covenant line moderated by the
management's own risk aversion, and approached at the management's own pace."* All three are reads
this world already has. The LINE is the tightest leverage covenant its own lenders actually imposed
on it, which is a fact about what it had to promise to be lent to (B2). The MODERATION is the risk
aversion drawn per firm at the seed — the margin a management insists on before committing money it
cannot get back — because a management cautious about a project is cautious about a balance sheet
and runs inside its promise rather than against it. The PACE is the horizon it counts, its patience:
it closes the gap over that many periods, so a management that will not look past two years moves
faster than one that looks past ten. A firm nobody has lent to on terms has no covenant, no target,
and brings what it is short of — it has made no promise to run inside. A firm already past its
target issues nothing and stays short, which is what a target being real means.

**A3.a: the service is interest PLUS scheduled principal.** The covenant test read the coupon alone,
so a five-year bullet cost its issuer the same in its last year as in its first — and its last year
is exactly the year a coverage covenant is for. `serviceOverAYear` reads the line's own schedule for
what falls due within a year, whatever shape it is.

**C8: a tap taps.** A line is named by its issuer and its maturity, so a firm coming back a week
later at "today plus five years" was opening a SECOND line, with its own book and its own covenants,
every week it was short — a market with one bond per issuer per week and no line deep enough to
trade. Paper matures on stated dates: the tenor now lands on the end of the month it falls in, every
issue inside that month is the same line, and the band is the calendar rather than a number.

**Three of the item's reasons are NOT built, and are written up as 21.60:** tenor, size and
diversification as separate reasons; covenants as a term of the holder's bid (B2's negotiation still
has one side, so an issuer promises the tightest covenant a lender could ask for); and 21.49's
Cross-Border C2 — a firm issues in the money of its published need, which is its home money, so the
lender base and the cost in another money never enter. All three are mechanisms rather than numbers,
and all three land at 17b, where a buyout is the first deal that needs a tranche in another money
and a lender that bargains.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.
`corporate-bond`: three new tests — every maturity is a month end, the covenant's service is a real
figure, and nothing anywhere declares what a management wants. The suite's two standing reds are
unchanged (21.58).

## Item 17.5 — Trade credit: a seller short of cash ships for cash

**B2, B5: the other half of the decision.** A seller tightened terms on a buyer that had let it
down, and on nothing else. But offering terms IS lending — the goods go now and the money comes in a
month, funded out of the seller's own account — so a seller that has published a gap of its OWN has
nothing to fund anybody with, and what it needs from the sale is the cash. It reads what it
published (Law 19), so a firm whose gap closes says so the next period and its terms come back with
it. This is the channel D3 names: a cash squeeze that travels along the supply chain rather than
through a bank.

**Law 9, Law 18: a row says when.** An invoice was named `(seller, buyer, n)` with `n` found by
scanning from one, so a pair that had traded every week for a year scanned a year of rows to write
this week's, and the name said nothing about when it was written. The period is in the name now.

**Law 18: the ageing is walked once a seller a period.** Every sale asks whether this seller has
been let down, and the answer walked the seller's whole book each time. It is a memo of that walk,
declared as a `working` store, cleared when the period turns — the rows are the source and the next
period walks them again.

**Not built, and named:** *the days are the seller's preference*. They are still one number for the
whole world. What stopped it is written up as 21.61: a seller here is a firm, a small-business cell
or a merchant, and only firms are drawn numbers — so the preference needs a home every seller has,
and "no preference" must not come to mean "no terms", which would delete the tier that lives on them.
Positioned at 17.7.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green. `trade-credit`: three
new tests, all ten green.

## Item 17.6 — Short-term debt: whose alternative is whose

Three reads, each of them somebody else's published number.

**An issuer's alternative is what money last cost IT.** A2 says an issuer will not sell paper below
what borrowing otherwise costs it, and that was read as the loan rate a bank quoted it. Nobody
quotes a BANK a loan — so a bank had no alternative to compare against and could not issue paper at
all, which left the one issuer whose whole business is borrowing short with no price in this market.
Its own blended cost of funds is that number and it publishes it every period (Banks Funding C1).

**A buyer's alternative is its KIND's.** A bank does not put spare money on deposit at another bank:
it has an account at the central bank, and what money earns there overnight, risk-free with no name
to doubt, is the deposit facility — the floor of the corridor (Central Bank B2). A corporate
treasurer is a depositor and compares against the keenest board anybody is showing. Which of the two
a buyer is comes off its kind's own profile: a bank is a kind that ISSUES money, which is the same
fact as having an account at the central bank, so nothing branches on a name (Law 15).

**And an unpaid maturity is not an unpaid maturity when the issuer is dead.** E3's family reported
paper still outstanding past its date, every period, for the life of a liquidation — but a name
whose estate is open owes what it owes into a waterfall (XI-8): the paper is a claim ranking with the
rest, and the cash moves when the estate distributes or never. The family skips a ceased issuer, so
what it reports is cash that did not move rather than a mechanism doing its job.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths green. `short-term-debt`:
three new tests, all sixteen green.

## Item 17.7 — The re-agreement

Terms are fixed at issuance, and until now there was no way out of that at all: a claim that stopped
performing stayed in default for the rest of its life, and a claim that was being paid fell due on
the day it was written for whatever either party wanted. One door out, and it is narrow in three
ways rather than general.

**The KIND says what a re-agreement of it may not change.** `InstrumentKindProfile.reagree` returns
the reason to refuse, and a kind that declares nothing cannot be re-agreed at all — so a share, a
good and a bond are not reachable from here, which is right: a share is not renegotiated and a
bond's restructuring is an exchange offer to its holders, not a private word with one of them
(Sovereign G4, still missing and now at 17b where the tender is). A loan says what a loan means: the
two parties, the day the money moved, the convention every accrual was struck under and what stands
behind it are not negotiable after the fact, and a date brought forward is an acceleration. Time and
price are what the two of them can agree.

**The STATUS says which event this is.** A `rolled` line is performing and a `restructured` one is
not, and the kernel refuses the other pairing — because a reader counting workouts must not count
rolls, and a lender's record of how a name has behaved must not confuse them.

**And the line performs on the terms that stand.** That is one sentence and not a branch: performing
means no promise of this claim has been broken, and the promise that was broken is not a promise of
this claim any more. It is the only path that restores what `markDefaulted` took away, and the
journal still has the miss and the re-agreement both.

**The workout is a decision with a cost on both sides** (E3). Enforcing brings what an estate would
return — this creditor's own recoveries, netted against the market's own price of whatever is
pledged — and brings it now. Agreeing brings par less what it still expects to lose on a name that
has just failed it, at the new maturity, so the row goes on consuming the bank's capital until then
at the charge the bank itself requires. A creditor that has met no estate has recovered nothing, so
enforcing brings it nothing and it agrees; one that has been paid in full by every estate it met
takes the money; and between them it is the length of the new terms that decides. Neither number is
a price and neither is discounted: a loan has no market and is carried at what its holder expects to
recover, and these are two readings of that one expectation over two futures.

**The roll is 21.59, and it was a missing mechanism rather than a preference.** A borrower in this
world publishes what its wages and its orders cost it and never what falls due, so nothing refinances
a maturity: a performing borrower with a loan maturing had no channel at all and failed on the date.
A lender now agrees another term with a name its own standard would still write today, at what its
view of that name now requires, the period before the money is due. A name it declines is not rolled
and has to find the money.

**What the census said.** The mechanism fires in a scale model whose loans reach a maturity (the
test shortens the term rather than stepping a year of weeks), and it fires nowhere in the thirty
periods of the rig — for a reason that is nothing to do with it and is now written down: every bank
in that world is insolvent from period six and none is resolved, an insolvent bank publishes no cost
of funds, and with no cost of funds there is no credit view, no quote, no reservation and no
workout. `credit.written` is zero in every period after the fifth. That is 21.64, positioned at 23.2.
Two more found in the same census: a row repaid to the last unit stays live (21.63, at 17.9), and an
agreement has no cure — `breached` reaches only `discharged` or `terminated` (21.62, at 17.9).

**Split, and where the rest went.** 17.7 carried five mechanisms. The covered bond, factoring on §42
receivables and repo-eligible senior notes and CP are inserted as 17.7a, immediately after this item
and before 17.8: each is an instrument shape that needs this door and nothing later, and factoring is
where 21.61's seller-chosen days belong, because Trade Credit A3 makes the early-payment discount the
implicit rate a receivable is sold at and says there is no factoring market without one. The
index-linked schedule is inserted at 18a.1, where the first published price level in this world is.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. `loans`:
eight new tests, all eight green; the file's seven reds are the seven that were red at `8f4d70e`
before this change and are unchanged by it (verified in a worktree at that commit — same seven, same
names), and they are 21.64's world rather than this item's.

## Item 17.7a — The seller's own terms, and the price of paying early

Trade Credit A3 has two halves and this world had neither.

**How long is the seller's, and it is drawn.** `tradeCredit.days` was thirty days for everybody — a
mill and a corner shop wrote the same invoice — and B5 says the seller decides its terms. What had
stopped this being fixed was a home: a seller here is a firm, a small-business cell or a merchant,
and only firms have a module that draws numbers for them. THE PARAMETER REGISTER IS THE HOME THEY
SHARE. It is keyed by the party's own name, it holds a number for the life of the world, and it is
already where a firm born after the seed declares its hurdle and its horizon. So each seller draws
its days, its window and its discount from the module's stated widths the first time it ships on
terms, declares them under its own name, and writes them on every invoice afterwards. The module
declares nothing at assembly at all. `params.has` is the one kernel addition: a caller that declares
on first use has to be able to ask whether it already did, and `decl` throws on a number that is not
there — right for a read, wrong for a question.

**And the discount is an interest rate, which is why A3 is a clause.** Paying ninety-eight on the
tenth instead of a hundred on the thirtieth buys the buyer twenty days and costs it two of the
ninety-eight it still owed: thirty-seven per cent a year, derived once from the terms the two of them
struck and declared nowhere. A buyer that reads it chooses against what its money earns where it is
— the board its own bank published for the class its kind belongs to (Law 15: the registry says which
class, not this module) — and pays early when the seller is offering more for its money than its bank
is. A buyer that does not know what its money earns has nothing to compare and does not pay early,
which is a stated answer rather than a zero.

Paying early is a redemption below par: the holder gives up the row at what it agreed to take and
realises the difference against what it was carrying, the buyer keeps what it did not pay, and it is
one instruction with two legs like every other. It settles whoever HOLDS the row rather than whoever
wrote it, which is what a receivable that has been sold needs (17.7b).

**What the census said.** The mechanism is exercised by a test and by nothing in the assembled world,
and the reason is written down as 21.65: every invoice the scale model writes is one firm on the
TREASURY — one seller, one buyer, seventeen rows in twelve periods — and no firm ships another firm
on terms at all, which is the connection §42 A4 calls the tier that lives on it. The one buyer this
world has banks at the central bank and so keeps no deposit board to compare against. One of the four
seeds tried writes no invoice at all in twelve periods. Positioned at 23.1, where the scale model is
resized.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.
`trade-credit`: four new tests and one rewritten (it asserted the world-wide number this item
deleted; what it also asserted — that no `tradeCredit.` number is a share, a rate, a loss or a
default — is kept and now covers the derived rate too), all fourteen green.

## Item 17.7b — Factoring

Trade Credit A3 ends *"without it there is no rate, and no factoring market can exist."* 17.7a made
the rate; this is the market.

**What the factor pays** is the receivable discounted at what that factor requires of the name that
OWES it, which is the BUYER. A factor that priced the seller would be pricing the wrong credit: the
money comes from the buyer on the due date and the seller is out of it the moment the row changes
hands. Every bank publishes what it requires of every issuer of live paper, and an invoice is live
paper whose issuer is its buyer, so every receivable in this world already had a price waiting for
it. The seller shops all of them and takes the keenest; a name no bank has priced has no factor,
which is a refusal and not a price.

**What the seller compares it with** is two alternatives it already has, and neither is a gate
anybody chose. The first is its own discount — it has published what it will pay to be paid early,
so a factor has to beat its own customer. The second is what money costs the seller itself, the rate
it was last quoted to borrow. That second comparison is why factoring exists and why §42 A5's tier
uses it most: selling a receivable is borrowing against your CUSTOMER's credit instead of your own,
worth doing exactly when the customer is the better name. Nothing states which is which. In the run,
the firm borrows at 1.9 per cent and its customer is taken at 0.28, so it sells.

**What stops a factor buying everything** is the line its own treasury allotted it and published,
spent as it buys. It is the LENDING line and not the dealing one: a factor holds the bill to its day
rather than turning it over. A bank that published no room takes nothing.

**What the census said, and it is a large one.** EVERY BANK IN THE SCALE MODEL OPENS IN CAPITAL
BREACH, IN PERIOD ONE, with negative headroom — so both lines are allotted zero room in every bank in
every period and nothing can be taken onto any bank's book at all. That is what stops an arranger
backstopping an issue at 17.2, and it is upstream of every bank being insolvent by period six.
Relaxing the standard-setter's two ratios to a thousandth gives one bank room and the factoring
market opens immediately, seven receivables in six periods — so what is missing is the response to a
breach, not the mechanisms behind it. Written up as 21.66 and positioned at 17.9, whose subject is
exactly that: a negative want publishes `mustShed`, and the floor that turns it into a zero goes.

**Law 4, in passing.** Two modules now ask what a bank has spare on a named line and neither may
import the module that names it, so both were spelling the line out. The names are in
`registry/banking.ts` and the arranger reads the same constant.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.
`trade-credit`: three new tests, all seventeen green.

## Item 17.7c — Security reaches the price

A pledge was worth something to a lender provisioning a loan and nothing to a desk pricing a claim.
The lender's arithmetic was there — what the security covers at the market's own price of it, taken
off what it expects to lose — and the desk asked a different question and got the name's answer: what
this bank requires of the ISSUER, whatever the claim it is being offered. Two claims on one name
priced identically, one of them with a building behind it.

**What a security covers is now one derivation** (`uncoveredShare`), and both readers take it. The
loss a lender provisions is that share times what it expects to lose on the name; the yield a desk
requires of a claim is what it requires of the name less the covered part of the expected loss it has
already published. Nothing new is believed in either: the expected loss is the bank's own published
number and the share is arithmetic over prints.

**What is pledged is the KIND's to say.** It is read off `ranking(i).secured` — the same answer an
estate takes when it decides who gets what — so a loan, a covered bond and anything else with a
pledge behind it are priced by one rule and none of them is named anywhere. The loan provision and
the workout both stopped reading a loan's own terms for it, which was the read that would have had to
be rewritten for every claim that can be secured.

**Why it is an item and why it is here.** A covered bond is a bond whose pool makes it cheaper than
the same bank's unsecured paper. Built on a desk that prices every claim at the issuer's unsecured
yield, it would be a shape: the pool would be decoration and the saving would have to be asserted.
So it is inserted before 17.7d rather than folded into it.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. `loans`:
four new tests, all four green; the file's seven reds are the seven that were red at `8f4d70e` and
are unchanged.

## Item 17.7d — What the secured market will take

**The covered bond left this list first.** It was 17.7's third mechanism and the owner's decision is
that it is out of scope: §42's securitisation of mortgages does the same work — a pool of named loans
standing behind claims held by named holders — and one mechanism for that is what Law 4 asks for.
Nothing had been built for it. What the item had already produced is 17.7c, which stands on its own
and is what this step is built on.

**Eligibility stopped asking for a schedule.** What a market will take as security is three facts
about what the paper IS: somebody owes it, a market clears it, it is alive. A schedule was a fourth,
and it refused exactly the paper §42 D3 is about — the senior tranche of a securitisation, which
*"is used as collateral, so its liquidity matters to the funding system"*. A pass-through pays what
the pool pays when the pool pays it, so it promises no schedule and never will; the tranche kind
refuses to invent one because a schedule for it would be a forecast with no falsification test
(Law 17). What makes it good collateral is not a promise but a market.

**So there are two advances, and both are reads.** A claim that promises a schedule is worth what
that schedule is worth at what this lender requires OF THAT CLAIM — its reservation for the name less
the part of it whatever is pledged behind the paper takes away, which is 17.7c arriving in the repo
market. A claim that promises nothing is worth what it could REALISE: the market's own last print,
less what this lender expects to lose on the name over the term of the loan it is making. The print
is the market's number and the expected loss is the lender's, published under its own name; the
haircut against the market is still the read it was, never a table.

**Law 4, in passing.** What a pledge covers is now one derivation in `registry/secured.ts`, where the
three modules that need it can reach it and none has to import another: the bank provisioning a loan,
the desk pricing a claim, the lender deciding what to advance.

**What the census said.** No tranche exists in the scale model at all — fourteen periods of the
securitisation rig produce none — because this world's banks open in capital breach and shed nothing
(21.66), so the pool is never cut. And fifteen of the sixteen tests in `money-market.test.ts` are red
and none of them is about the money market: each asserts that the WHOLE WORLD's audit is empty, and
the assembled world reports 79 to 325 violations a period across four families. Written up as 21.67,
with the test's shape positioned at 23.1 and each family's cause at 23.3.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.
`money-market`: two new tests, both green; the file's fifteen reds are the fifteen that were red at
`af70a23` and are unchanged by this (verified in a worktree at that commit).

## Item 17.8 — A bank loan is not distributed outside the banking system

The rule was the owner's, written into COVERAGE in September and nowhere else: a loan is not a
security (A1.a), so a syndicate is a group of BANKS and never a distribution to the public (D4.a),
and the way credit risk reaches an investor is a NOTE issued against a pool held by a named vehicle
(XI-11) rather than the row itself landing in a pension fund. Nothing in the engine knew it.

**Who is inside the system is declared on the kind.** `PartyKindProfile.banking`, beside `borrows`
and `buysOnTerms` and for the same reason: it is a fact about what a party IS, and a list kept inside
the check would be a list somebody has to remember to add to (Law 15). Three kinds say yes — a bank,
the central bank at the centre of it, and a securitisation vehicle, which exists to hold exactly
these rows for the noteholders that funded it. It is required rather than optional, so a new kind
answers the question instead of inheriting an answer; every one of the eighteen declarations in the
engine now says which it is, and says why in a line.

**An estate is not one of them and does not need to be.** It holds what a dead bank held, and only
until it has sold it (XI-8, Register F2). Winding a loan book up is not running one, and a check that
fired on a succession would be reporting the wind-down as a defect.

**It is a family and not a refusal at a door, because that is what a FORBID is.** Part II: a FORBID
that holds is as valuable as a mechanism that works, and it breaks silently — so what is wanted is
something that says so the day a row goes somewhere it should not, not a throw at a door nobody
walks through. It reports nothing in either scale model today. A test sells part of a bank's book to
a firm and it names the holder, the row and how much, and moves nothing back: the audit never
repairs.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. `loans`:
two new tests, both green; the file's seven reds are the seven that were red at `8f4d70e` and are
unchanged.

## Item 17.9 — What the treasury allots can be negative

A line asks its treasury for the distance between its own appetite and what it is already using.
That distance can be negative, and what a negative distance means is not "this line is asking for
nothing" — it means IT MUST COME DOWN. Two floors said the first thing (`atLeastCash` on the ask,
`atLeast` on the allotment), and with them the world had no way to tell a bank in breach the one
thing it has to be told: sell something. Both are deleted and `banks/lines.ts` leaves the zero-floor
ratchet.

**Three passes, each a sentence.** Every line past its own appetite comes back to it, and doing so
RELEASES the capital it was using — so what the bank has to share out grows by exactly what its
lines are giving back. The lines that still want room share what there is, best earner first, and
what is left can be nothing. And if the bank is still over the rules after all that, the rest of the
hole is shed too, worst earner first, because that is what a treasury actually cuts; no line sheds
more than it is using, which is arithmetic and not a floor, and what no line can cover is left
standing, because a bank that cannot shed its way back is its resolver's and not its treasury's.

**Nothing new reads it.** A line's limit was already "what it carries plus the room it was given", so
a negative room lowers the limit below the book and the lending line writes nothing. What was
missing was the other half: a desk over its limit had no reason to sell. `mustRaise` is now the two
reasons in one place — the overnight session refused it, or its book is above what its treasury
allotted — and the bigger of the two is what it raises, because a sale answers both: the money comes
in and the capital the position was using is released with it.

**Law 4, in passing.** The ask was derived twice inside the same function and is now derived once and
PUBLISHED beside what the line got. They are two facts — what it wanted and what it got — and a
reader that had to rebuild the first out of the other three would be deriving it a second time.

**What the census said.** The credit side came back to life: 38 loans written in period 2 of the
scale model, where the world had written none after period 5. The hole did not close — two of the
three banks are insolvent by period 3 and stop publishing, and the third's headroom runs from −34bn
to −102bn over twelve periods while it sheds. 21.66 is rewritten to say exactly that and re-aimed at
23.2, whose sentence it now is: a bank that is insolvent is never resolved. And one more of 21.67's
shape, worse than the first: `test/bank-capital.test.ts` does not collect at all — it asserts an
empty audit at module scope — so every case in it has been silently unrun.

**Split.** 17.9 carried the treasury's allocation and the loan's own lifecycle. The second is
inserted as 17.9a, immediately after: prepayment and the write-off are one bounded change that needs
nothing of this one, and two findings already wait there.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. `loans`:
three new tests, all three green; the file's seven reds are the seven that were red at `8f4d70e` and
are unchanged.

## Item 17.9a — A line is repaid at the borrower's option

Only half of C9 was built. A borrower could draw on its line, and nothing it ever did brought the row
back down again — so a firm that had a good quarter carried the debt of its worst one for ever, went
on paying interest on it, and went on consuming its lender's capital and its own large-exposure limit
for money it was not using. F2's *"new lending, amortisation, prepayment and write-off account for
the change in the book"* named it, and the comment standing over that family had been claiming three
of the four for months.

**What it repays is its own published number.** A borrower publishes what it is short of through the
one door every borrower uses, and that number is SIGNED: short of money it asks, over it publishes
the surplus as a negative. Nothing here peeks at an account to decide a borrower has too much — it
said so, last period, in public, to its lenders — and the repayment carries the same lag the ask
does, because a lender acts on what it has already been told.

**Dearest first**, which is what paying down debt means: of two lines it owes, the one that costs it
more is the one it retires. In the scale model a borrower with two lines pays down the one at 158
per cent before the one at 1.3. And it repays only what it holds; a borrower that promised more than
it has is a failed instruction rather than a repayment.

**Repayments run before new lending in the same phase.** It is the same money: a bank that lent out
what a repayment was about to bring back would be sizing its book against a number that had already
moved.

**21.63 is answered rather than fixed.** A live loan row with nothing outstanding is an UNDRAWN LINE,
which is exactly what C9 says a line is. The five such rows in the thirty-period rig are
prime-brokerage and overdraft lines their borrowers had paid back, and every one of them is
performing; a term loan that amortises to nothing does cease, at the redemption that empties it. The
part of the original note that said two of them were still not performing was a mis-pairing of two
reads, and no such row exists.

**Split.** 17.9a carried prepayment and the write-off. The write-off is inserted as 17.9b: a
repayment is a borrower acting and a write-off is a lender giving up, and the second needs the end of
an estate — what is left to recover once the estate that succeeded the borrower has closed.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. `loans`:
two new tests, both green; the file's seven reds are the seven that were red at `8f4d70e` and are
unchanged.

## Item 17.9b — The end of a claim

E5 says the loan leaves the book, on a date. Nothing in this world ever took one off. Twenty-three
loan rows in twenty-four periods of the estate rig were live claims promising nothing, owed to
nobody, on borrowers that had finished existing — carried, marked and counted by no one.

**It is the kernel's and not a lender's, because there is no lender left to decide anything.** The
claim has no holder: the estate settled it for whatever it fetched and the loss reached its holder's
capital in that redemption. What is left is an empty line on a name that is gone. A terminal party
succeeds ITSELF — an estate that has paid everything away has nobody left to succeed it — so when
the successor is the party, the chain of references ends there and nothing it promised can be
presented to anybody again. Every row it promised with nothing outstanding ceases with it, on that
date, in the journal, with the reason.

**A row with something still outstanding stays exactly as it is.** That is a debt nobody can pay, and
it is a fact the audit should go on reporting rather than one this tidies away: a write-off is an
outcome with a size and a date, never a line quietly disappearing.

**And an undrawn line is untouched.** It is the same shape — nothing outstanding — with a living
borrower, and that is a FACILITY, which is what C9 says a line is. The difference is whether there is
anyone left to draw on it, and that is what the rule asks.

**E5.a needed nothing.** The cease books no value at all, because by then the loss has already
reached capital in the redemption that emptied the row. A cease that booked the principal again
would be the double-count that clause names.

**What was tried first and was wrong.** A lender-side write-off: the bank walks its book and gives up
on claims on names that have ended. It fired zero times, and the reason was the answer — those rows
have no creditor at all, so there is no bank whose book they are on and nothing for a lender to
decide. The mechanism is a fact about the register, not a decision, and it belongs where facts about
the register live.

**21.62 moves.** An agreement still has no cure, and nothing in this world asks for one; the
write-off's supposed sibling turned out not to need it. Building the door now would be building a
door nobody opens, so it is repositioned to 19, where a payer given time on an arrear with the
treasury is the first thing that will ask.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. `loans`:
two new tests, both green; the file's seven reds are the seven that were red at `8f4d70e` and are
unchanged.

## Item 17.10 — A credit index over the rated universe

A credit market is not bought as one thing. It is bought on ONE SIDE OF A LINE: investment grade and
high yield are a single scale with a boundary across it, and everything that matters about a credit
market happens at that boundary — a mandate says which side it may hold, an index is built on one
side, and a name that CROSSES is sold by everybody who may not hold the other side. This world had
one credit basket per money, so membership changed only when a line was born or died, and a rating
had nothing to do.

**The boundary is data beside the scale**, in `registry/grades.ts`, where the scale itself is. It is
not a number: it is a grade on the published scale, and where a market draws it is that market's
convention. `isInvestmentGrade` partitions the scale and nothing derives it.

**Who is on which side is the assessors' to say.** A basket rule can now read what they published,
through the kernel, the way it reads a print — a grade is a public fact somebody else produced, and
a rule that formed its own opinion would be an index with a credit view. A name nobody has graded is
in NEITHER basket: unrated is the absence of an opinion, not a side of the line, and an index that
guessed which side such a name belonged on would be inventing the thing it exists to measure.

**Law 4, in passing.** `gradeOn` — the middle of what a name's assessors have published — lived in
the banks module, where the index and the audit could not reach it. It moved to `registry/notices.ts`
beside `gradesOn`, and the credit view re-exports it for its own readers.

**What the census said, and it is the item that follows.** THE ASSESSORS SAY NOTHING. Three assessor
parties exist in every scale model and not one `rating.action` is ever published — zero in twenty
periods, in four different seeds. So every name is unrated, every claim carries the regulation's
ungraded weight, and both new lines report Missing because their baskets are empty. Written up as
21.68 and inserted as 17.10a, before the tracker: a tracker on an empty index is nothing, and a
rating that never happens is what makes it empty.

**Split.** The tracker is 17.10b. It needs the seed to draw a CREDIT tracker — a debt blueprint and a
basket of bond lines, where `drawTrackers` draws an equity one — which is the seed's work and not
the index's.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. `indices`:
two new tests, both green; the file's three reds were red at `b540d35` before this change.

## Item 17.10a — The assessors say nothing: answered, not a defect

Two censuses found the same silence and I wrote it up as a defect twice: three assessors in every
scale model, zero ratings, every name unrated, both rated indices empty. It was the CALENDAR.

A quarter is thirteen weeks, and nothing is reported on a quarter that began before this world did —
so the first fiscal close is between one and four quarters into a run, plus the reporting lag. In the
rig it lands at period 20. The first rating follows at 21 and the first rated index level at 21. The
censuses had stepped twelve and twenty periods: one week short in the second case.

**What it cost to find out** is the reason this is written down rather than quietly fixed. A
mechanism that has not happened YET looks exactly like a mechanism that does not work, and the only
thing that tells them apart is stepping far enough. Every census in this file that reports an absence
over twelve or twenty periods is reporting on a world that has not closed a quarter.

**A test pins the chain and its order.** A company closes a quarter; it shows the statement to the
assessor it pays; the assessor grades what it was shown; the index reads the grade. Four mechanisms,
each somebody's act, and each one can only happen after the one before — which is what the test
asserts, rather than the dates, because the dates are a draw.

**What the census said instead.** Every grade this world has ever published is the WORST one: all 240
rating actions carry `c`. So the investment-grade index is permanently empty, the high-yield one
carries every rated line, and three assessors looking at twelve issuers never disagree about
anything. It may be a true reading of a world whose banks open in breach and whose firms fail, or the
measure may be saturated. Written up as 21.69 and positioned at 23.3; not chased.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. `indices`:
one new test, green; the file's three reds were red before this change.

## Item 17.10b — The tracker over the rated universe

An index with no vehicle following it is a measurement nobody trades. 17.10 built two credit lines
and neither had one, and the reason it could not have one was structural.

**A tracker's launch basket is its index's, not a list.** The declared basket was written down at the
seed, which works for a vehicle on a market that exists at period zero — a broad equity index is its
listed constituents, and they are listed — and cannot work for one on a market that does not. A
credit index holds paper issued years into the run, with ids nobody could name in advance, so a
tracker on it would have launched out of a basket naming nothing this world has. Now the launch reads
the index, which is what the code's own comment had been claiming all along: the mandate is the asset
class its investors bought and the INDEX is what says which lines and in what weights. Nothing after
the launch reads either — a creation unit is already a pro-rata slice of what the fund actually
holds.

**And a tracker draw carries three things it did not.** What its vehicles may HOLD — the claims a
company issued rather than the residual, because without that a credit tracker is an equity mandate
holding bonds — the house they belong to, and whether the seed may open them. A vehicle the seed may
not open is one whose index has not answered yet, and it launches in kind when it does.

**Two of them, because the line has two sides.** A name downgraded across it leaves one vehicle's
index and joins the other's, and both have to trade; that is C2's simultaneity with something to be
simultaneous about. A world where every name sits on one side launches one and leaves the other
waiting, which is a true statement about that world rather than a gap.

**Law 9.** Each vehicle is named for the line it follows, so a reader sees which market it is in
without looking anything up; only the one the seed opens is named for its house.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.
`fund-manager`: two new tests, all fourteen green.

## Item 17 — Corporate credit, the rest: closed

Twenty-one steps, and what they changed is easier to say by what a lender can now do that it could
not. It can form ONE view of a name out of its own record and what the name showed it (17.0). It can
price the paper and the loan from the same belief, one term apart (17.0). It can read a company's
full quarterly accounts if it is owed money by it, and not otherwise (17.0a). It can float a line
over a benchmark that actually traded (17.1), bring somebody else's issue and be paid for it (17.2),
commit a facility and hold capital against what it has promised and not lent (17.3). It can be told
by a borrower which reason it is issuing for (17.4). It can watch a supplier decide whether to ship
on terms at all (17.5), and read what a bank's own paper costs it rather than what somebody quoted a
firm (17.6).

And then the second half, which is what happens to a claim after it exists. It can be RE-AGREED —
rolled while it performs, restructured when it does not — through one narrow door the kind itself
holds the key to (17.7). A seller can set its own terms and put a price on being paid early (17.7a),
and sell the receivable to whoever waits cheaper (17.7b). A pledge reaches the price of the claim it
stands behind, everywhere at once (17.7c), which is what lets the secured market take a claim that
promises no schedule at all (17.7d). A bank loan cannot leave the banking system without the audit
saying so (17.8). A treasury can tell a line to come DOWN, and the desk sells (17.9). A borrower can
pay its line down out of money it said it does not need (17.9a). A spent claim on a name that has
finished existing ceases (17.9b). And credit has an index with two sides and a vehicle following
each of them (17.10, 17.10b).

**What it deleted.** Three placeholders (`seed.crossHoldingShare`, `corporateBond.margin`,
`shortTermDebt.line`), the world-wide `tradeCredit.days`, two zero floors in `banks/lines.ts`, the
second statement parser in `journal/published.ts`, a duplicate `gradeOn`, a duplicate ask inside
`publishLines`, and a duplicated dealing-line name.

**What the owner decided.** The covered bond is out of scope: §42's securitisation of mortgages does
the same work, and one mechanism for it is what Law 4 asks for.

**§7 is answered 62 of 62** — 49 MET, 7 PARTIAL, 6 MISSING, and every one of the six says where it
lands. Twenty-eight of those rows had been blank, and a blank cell is not an answer; most of them
were mechanisms this world has had for months that nobody had gone back to mark. Three of the six
sit behind B1's early-termination regime, which nothing stamps on a line, and that is now a named
absence rather than an empty cell.

**Where the findings went.** 21.59 closed at 17.7 and 21.61 at 17.7a. 21.63 and 21.68 were ANSWERED
rather than fixed: an undrawn line is not a spent one, and the assessors were silent because a
quarter is thirteen weeks. The rest are positioned: 21.65 and 21.67 at 23.1, 21.64 and 21.66 at 23.2,
21.67 and 21.69 at 23.3, 21.62 at 19.

**The suites run at the end of step 19**, by the owner's instruction (2026-09-16), not at this close.

## Item 17b.1 — A lender that agreed to lend

§29 E1 is *"no buyout without a lender who agreed to lend"*, and this world had no way for a lender
to agree to anything. A borrower published what it was short of and a bank wrote it a row the next
period; there was nothing in between, and the thing a deal needs is exactly the thing in between.

**Why a row will not do.** The money must not exist unless the deal closes. A loan written the
period before a tender is money a failed tender has to hand back — an interest bill, a repayment and
a borrower briefly holding money it had no use for — where a COMMITMENT drawn inside the instruction
that completes the tender is money that was never made. That is the whole reason this is an
agreement (XI-8) and not an instrument: nobody trades a promise to lend, it has no issued quantity
and no holder, and it is one of the things an estate has to divide.

**One field on the one door every borrower uses.** `CreditAsk.wants: 'money' | 'commitment'`, stated
either way and never inferred, exactly as `repays` is. It is not a second door because the DECISION
is the same decision — `shop` prices the name, finds which bank quoted it and how much room that
bank has — and only what the decision produces differs. Six existing asks say `'money'` in a line
each; a seventh kind of borrower is one word away rather than a new channel.

**The lender's capital stands behind it from the moment it says yes.** `FACILITY` is declared with a
`headroom` contribution, so what is promised and not drawn consumes the same room a drawing would
(Banks Lending A3.a) — which is the difference between a commitment and an intention, and the reason
a bank cannot commit to every deal in the world at once. B2.b's *"the credit market decides which
buyouts occur"* is that sentence, and it is arithmetic rather than a rule.

**And it lapses.** A commitment with no end is a free option the lender did not sell — §18 B4's
sentence about backstops, as true here — so a facility carries the period the deal has to close in
and is TERMINATED when it passes: ended by its own terms, not discharged by a payment, because
nothing was ever owed on it. The room comes back before the bank decides on the next period's asks.

**What it did not do.** `short-term-debt`'s `BACKSTOP` is this object with a commitment fee and no
end date, and the two are not merged: a backstop's fee is §18's economics and changing it does not
belong in §29's item. The duplication is written down as finding 21.71 with the item that closes it.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. `loans`:
two new tests, both green; the file's seven reds are unchanged.

## Item 17b.2 — A buyer that must borrow says so

§29 B2 is *"most of the price is debt raised against the target itself"*, and B2.a says whose it is:
*"the debt is the TARGET's liability, not the fund's — which is why a failed buyout kills the firm
and not the fund."* So the borrowing a deal needs is published **in the target's name**, through the
kernel's one borrowing door. A request says who will owe the money and the answer is the company; a
fund that borrowed for its deals would be a fund a failed deal can kill, which is the clause read
backwards — and it is why `funds/data.ts` has said `leverage: false` about the buyout pool since
13.5.

**One ask per target, not one per bidder.** A company cannot owe two deals' debt at once. Where two
buyers want the same firm the ask is the biggest of the holes they would have to fill — what the
company would carry if the dearest of them wins — and the two of them meet in the book afterwards
(B4), not here. And it does not ask twice while its last ask is unanswered: a borrower publishes in
one period and hears in the next, so a repeated ask is one deal holding three lenders' capital.

**B2.b needed no mechanism at all.** *"The deal only happens if lenders will lend, at a price — the
credit market decides which buyouts occur."* That is the ordinary credit decision with nothing added:
`shop` finds which bank quoted the TARGET and how much room that bank has for the name, and what
comes back is a commitment at that size and rate, or nothing. There is no buyout branch anywhere in
the banks module, and a company nobody will commit to simply never has the money.

**What did not change, deliberately.** A bid is still only what the buyer holds. A commitment is not
money until it is drawn and drawing it is 17b.3, so a deal a lender has already promised to fund is
neither bid for nor asked about again — it waits one step. Letting the commitment size the bid today
would have let a tender clear at a price the buyer could not pay, and half of it would have settled:
the acceptance condition is checked on the book, not on the money.

**Two findings, neither chased** (Law 11). **21.73:** `control` has never run in an assembled world —
`control.tender` 0 over twenty-four periods of the rig — because the only parties that publish a cost
of money are index trackers, and the rig draws no buyout fund at all. Positioned at 23.1, which is
where the scale model is resized. **21.72:** the banks stop quoting every name by about period 9
(121 quotes in period 1, 0 from period 9 on, 106 declines in period 24), giving `appetite`, `it
cannot cost its own funding` and `nobody lends to a party of this kind` as their reasons. A world
with no credit market cannot exercise B2.b; what a bank's room is made of is a Part XII measurement
and it is positioned at 23.3.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. `loans`:
three new tests, all green.

## Item 17b.3 — The tender has two payers

§29 B2.a is the sentence the design turns on: *"the debt is the TARGET's liability, not the fund's —
which is why a failed buyout kills the firm and not the fund."* The only two-sided way for a
company's own borrowing to reach its own shareholders is for it to get its shares back for the
money. So the tender has two payers.

**One book, one cleared price, two sources.** Every holder is met at the same level. What each fill
is paid WITH is either the facility the company draws — and the shares that buys go back to their
ISSUER and cease, which settlement already does for any asset leg whose payee is the issuer
(Register B3) — or the buyer's own account, and those shares move to it. Three legs in the drawn
case, all in one numbered instruction: the shares back to the target, the row issued by the target
to the bank that committed it, and the bank's money created against it landing in the seller's
account. Nothing left the bank to make that loan (Banks Lending B1) and the proceeds never sat
anywhere: they paid the shareholder they were borrowed to pay.

**The buyer ends up with a majority of what is LEFT, because the denominator shrank.** That is what
leverage is, arithmetically, and it is why B4 needed no code at all: *"leverage up, interest cost up,
ownership changed in the register"* is what the instruction did.

**Whole fills, not a share of each.** Every seller is met at the same price for the same shares and a
share is fungible, so which of them was redeemed and which was bought is immaterial to all of them —
where splitting each fill between two payers would put a cell's grain through the mill twice
(XI-15). The facility is offered the biggest fills first so the least of it goes unused, and ties
break on the name so two worlds from one seed agree.

**B5 balances by construction rather than by a rule.** What the sellers were paid IS what was drawn
plus what the buyer put up, leg by leg; `control.acquired` carries all three. It is still marked
PARTIAL, because a VERIFY that cannot fail still has to be MEASURED — that is 17b.5.

**No escrow, and none was needed.** The item's own note asked for conditional issuance escrowed by
the arranger with a failed tender returning it. Settlement is atomic (Law 5, XI-5), so a tender that
does not meet its acceptance condition draws nothing and moves nothing, and there is nothing to
return. An escrow is what a world whose closings are not atomic needs.

**A test never names a party.** `buyout.test.ts` asks the world the draw made for the equity line
with the most holders, reads the dearest basis anybody carries a unit at, and bids four times it so
that every holder sells. What is asserted is the arithmetic of the deal: the company borrowed, the
buyer paid the rest, the two add to what the sellers got, the row is the target's and the buyer owes
nothing, and the buyer holds a majority of what is left.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green. `buyout`:
three new tests, all green. §29 B2, B4 and E1 re-marked MET; B3 and B5 PARTIAL with their step named.

## Item 17b.4 — The call is paced by the deal

§29 A2: *"capital is committed, not paid: it is called when A DEAL NEEDS IT, and the call is a real
payment from the investor's account on a date it cannot refuse."* The clause has two halves and only
one of them was built. `callCapital` called the whole of what every investor had promised the moment
the pool's cash reached zero — which for a fund with nothing to buy was once, at the start — and the
file said so, naming this item. The investors' money then sat in the pool from period 1 for the rest
of the run, which is the opposite of what A2 describes.

**The deal is the pace.** A buyer that cannot pay for a company out of what it holds now publishes
both halves of what it has to find: the debt a lender is asked to commit, and the CHEQUE it must
bring itself. The pool reads its own cheque and calls that — and a pool with no deal in front of it
calls nothing at all, which is the clause rather than a placeholder for it.

**Neither module names the other's event.** The module that runs a tender and the module that owns a
pool may not import each other, so `control.financing`'s name lives in `registry/funding.ts` beside
`firms.funding` and `fund.struck`, and both of them ask about a PARTY (0e′.3). It is LAST period's:
the capital phase sits at 40 and the tender at 70, so a call takes a week like every other payment
anybody has to arrange — the ordering check said so and the lag is the answer, not a reordering.

**What each investor is called for is its share of what is still promised** — `undrawn_i / undrawn`
of what the deal needs — and a deal bigger than everything left to call takes all of it. Two real
quantities and a branch between them, never a bound on either (Law 6), and the investor's balance
appears nowhere in it: **a call sized by the DEAL is not a call sized by the investor's cash**, which
is what A2.b forbids. `check:forbids` still refuses `atMost`, `atLeast` and `Math.min` anywhere in
that file, and the arithmetic is a pure read (`calledFrom`) so it can be tested without one.

**And a pool can now weigh a deal before it has the money.** `fund.struck` publishes what the pool
could still call, and a buyer counts it beside its balance when deciding whether to pursue a company
— but never when bidding, because what it could call is not money until it has called it (A2). The
sequence is: it sees the deal and says what it needs (p), it calls and its lender commits (p+1), it
bids with cash in hand (p+2).

**Not exercised end to end, and why.** No world this repository builds draws a buyout pool at all —
the rig's only pools are two credit trackers, and `fund.called` has never fired in it. That is
finding 21.73 and it is positioned at 23.1, where the scale model is resized. What is tested here is
the arithmetic of the call as a read, which is where the clause actually lives.

**Checks.** `check:opens` green; lint, typecheck, spec, forbids, deaths, existence green.
`committed-capital`: four new tests, all green; the file's one red (`:45` asserts a `Cash` against a
number) is the one that was red before, written down as 21.74. §29 A2 re-marked with the pacing and
B3 MET.
