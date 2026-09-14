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
`issueBonds`'s second reason (*"its bank will not lend it enough at any price"*), and this correction
is why it is a reason at all rather than a special case of the first: a firm does not always reach a
market because the market is cheaper. Sometimes it reaches it because it was told no.

**The FORBID this makes explicit, and it already holds.** No bank loan reaches a party outside the
banking system. `LOAN` declares `market: none()` and `pricing: 'carriedAtCost'`, so a loan cannot be
posted into any book; the only transfer path is securitisation's sale into a `VEHICLE`, and what
investors buy there is the `TRANCHE`, a security. It holds by CONSTRUCTION, which is precisely the
kind that breaks silently — give the kind a market one day and nothing would complain — so 17.8
guards it as an audit family, with the design question named: "the banking system" is a set of party
kinds, and enumerating it inside the family is the kind branch Law 15 forbids, so it belongs on the
party kind as declared data and carries an `ARCHITECTURE.md` change with it.

`docs/WORKLIST.md`'s **M9** note said the leveraged loan index waits on "the loan market". There is
no loan market to wait for; it waits on 17.0's note, and the note is corrected to say so.
