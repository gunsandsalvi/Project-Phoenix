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
