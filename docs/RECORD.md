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
