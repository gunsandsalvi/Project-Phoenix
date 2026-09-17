# What this model does, against the best there is — eight proposals

*Written against `9bab466`. Every claim about Phoenix below is read from the source or measured from
a run; every claim about practice elsewhere carries its source. The proposals are ordered by what
they would change per unit of work, and each one names the recorded findings it would close.*

---

## The one-paragraph diagnosis

Phoenix's mechanisms are, individually, better than most of the literature: adaptive per-party
expectations with a switching rule, two-sided settlement with real refusals, a parameter register
that types every declared number, an audit that reports and never repairs. What it has is **one
market microstructure — a weekly uniform-price call auction — applied to bread, labour, loans,
shares and freight alike**, with **no resting orders**, **no inventory-holding intermediary in the
goods chain**, and **no queue in settlement**. Those four absences, not the decision rules, are what
produce a world in which *4,158 auctions clear 15 trades* and a third of the parties die in twelve
weeks. Six of the eight proposals below are about the plumbing between the decisions, because that is
where this model is furthest from both reality and the state of the art.

---

## P1. Give the world more than one market microstructure

**Today.** `clearing/solver.ts` is "one solver for every market": participants post limit schedules,
the solver takes the uniform price that executes the most volume. `VenueDecl` (`clearing/venue.ts`)
declares an id, a name, the module that clears it, a unit, a currency and a key — **there is no
protocol field**, and the only variation is a tie-break (`sellersCompete` | `marginalBid`). A bakery
selling bread to a household and a treasury auctioning ten-year paper run the identical mechanism.

**What it costs.** Measured over twelve periods of the rig: **4,158 sessions, 15 cleared (0.36%)**,
3,749 of them finding no demand at all; **8 books of 358 have ever cleared**. A call auction with no
resting orders clears only when two parties independently want opposite sides of the same book in
the same week.

**Against Law 1.** *Reflect the real mechanism: real named counterparties, intermediaries, lags,
fees, refusals.* A Walrasian auctioneer for bread is the one intermediary that does not exist.
Nobody has ever bought a loaf at a uniform clearing price struck against every other buyer in the
region that week.

**What the best practice is.** The dominant protocol in macroeconomic ABMs is not an auction at all:
demand agents *"observe the prices or interest rates charged by a random subset of suppliers"* and
*"switch from the old partner to the best potential partner selected in this random subset with a
probability… as a non-linear function of the percentage difference in their prices"*, applied
*"in four markets: goods, labour, credit and deposit — according to a fully decentralized matching
mechanism"* ([Caiani et al.](https://www.sciencedirect.com/science/article/abs/pii/S0165188915301020),
[Delli Gatti et al., decentralized matching](https://link.springer.com/article/10.1007/s11403-014-0130-8)).
Financial markets went the other way — *"progressively adopted continuous double auctions… orders can
be submitted at any time and are immediately treated"* — while the periodic call is kept for openings
and closings ([call vs CDA](https://arxiv.org/pdf/1506.03758),
[market opening structures](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4550773/)).

**Proposal.** `VenueDecl.protocol`, as DATA with a dispatch table (Law 15), with three rows:

- `call` — today's solver, unchanged: sovereign auctions, the fixing, anything a real market strikes
  at one price at one moment.
- `posted` — the seller posts an ask it stands behind; a buyer sees a random subset of sellers (the
  subset size is a TECHNOLOGY: how much of the market a buyer can see) and buys from the best it saw
  at that seller's posted price. The price is still cleared from real supply meeting real demand
  (Law 3): the trade happens because a buyer accepted a price a seller was standing behind, which is
  what a price in a shop is.
- `book` — resting limit orders with continuous matching (P2), for exchanges.

Each protocol is a mechanism with its own module, chosen per venue by declaration. The engine keeps
one solver *for auctions*, and stops pretending a grocery is one.

**Closes.** 21.79 (0 bids in 66 contract books), most of 21.84 (nothing reaches a household),
12c.3's demand side, and — by removing 350 empty sessions a period — the largest single speed item
on the performance list.

---

## P2. Let an order rest

**Today.** `World.runOne` builds the book by asking every eligible participant for orders, clears,
and throws the orders away. Nothing survives the session. A buyer bidding 100 this week and a seller
asking 100 next week never meet, and neither ever learns the other existed.

**Why it matters more than it looks.** This is why `noDemand` is 90% of outcomes rather than
`noOverlap`: the two sides are not failing to agree on a price, they are **failing to be in the room
at the same time**. In every real market — a limit order book, a shop's shelf, a bank's standing
quote, a firm's price list — an offer *stands* until it is taken, withdrawn or expires.

**Proposal.** A standing-order register as a kernel noun, beside agreements and processes: an order
is `(party, venue, side, level, quantity, from, until, why)`, entered by a participant, cancelled by
its owner, expired by the calendar, and consumed by a match. Every session opens with the standing
book and adds this period's new orders. This is not a cache — it is the fact that an offer is a
commitment with a date, which is exactly this project's ontology everywhere else.

**Closes.** The structural half of 21.79 and 21.81 (10 securitisation deals failing "for want of a
bidder" — the bidders exist, they were asked in a different week).

---

## P3. Somebody has to hold the stock

**Today.** Producers sell what they made; `merchants` buy in one place and sell in another;
`banks` quote credit. **No party's business is to hold goods and stand on both sides of one book.**

**What the world does.** Between a mill and a household there is a wholesaler and a shop, and what
they are for is precisely the timing gap P2 describes: they buy when the producer wants to sell and
sell when the household wants to buy, and they carry the stock and the risk in between. In market
microstructure the same role is priced the same way: a dealer's spread is the cost of carrying
inventory and the risk of being adversely selected
([Glosten–Milgrom](https://ideas.repec.org/p/arx/papers/1902.10743.html)).

**Proposal.** A `stockist` participant in the goods chain: buys from producers at what it expects to
sell for less its own required return on the money tied up, posts an ask to households out of the
lots it holds, and wears the loss when the gap closes the wrong way. Its margin is an OUTCOME of its
own turnover, carrying cost and the prices it actually meets — no spread table, no fee schedule,
which is exactly how `merchants` is already written. It is the missing intermediary Law 1 asks for,
and it is the party that makes a consumer price index possible at all.

**Closes.** 21.84 directly (the consumer basket is empty because no physical good reaches a
household in a settled instruction), and the "producers produce once" half of 12c.3, because a mill
with a stockist has a buyer every week.

---

## P4. A firm that cannot sell should cut its price, not its existence

**Today** (`mechanisms/firms/decide.ts`): `wanted = (expectsToSell − stock) / yield` — a
stock-adjustment rule whose **desired buffer is exactly zero**. And the ask is "what it expects a
unit to fetch, less what perishes before it could": an expectation formed from its own past fills,
with **no feedback from unsold stock into the price**. A firm with more stock than it expects to sell
therefore starts nothing and waits, at a price it has no reason to lower. The plan file records the
consequence: *every plan after period 1 is `batch 0, bound demand`* (12c.3).

**What the best practice is.** In the K+S and AB-SFC families a firm carries a desired inventory as a
share of expected demand and adjusts its PRICE against two signals: whether it sold out, and whether
inventories are above or below that desire
([Caiani et al.](https://www.sciencedirect.com/science/article/abs/pii/S0165188915301020)). The
adjustment is bounded in size but not in direction, and it is what makes a glut clear.

**Proposal.** Two changes, both reasons rather than levels:

1. **A desired cover**, drawn per firm as a PREFERENCE (how many periods of expected sales it wants
   on the shelf) — dispersed like `payoutPatience`, so two firms with the same book plan differently.
   Nothing is stated globally and no ratio is imported.
2. **The ask answers the shelf.** What a unit is worth to the firm is what it expects to fetch *if it
   can sell it*, and a firm sitting on more cover than it wants is a firm whose alternative to
   selling today is selling later at a carrying cost it can read (storage, perishing, the money tied
   up). Subtracting that is not a markdown rule — it is the value of holding, which the docstring
   already claims to compute and currently computes as if the shelf were empty.

**Closes.** 12c.3, and with it the supply-side half of 21.79 and 21.84.

---

## P5. Nobody may post a schedule of zero width

**Today.** The expectation mechanism is genuinely strong — adaptive at a per-party memory, with a
switch to a public anchor when the anchor has surprised the party less (`expectations/index.ts`),
which is Brock–Hommes heuristic switching built honestly. But: *"a party that has never seen this
variable has no outlook… its first observation IS its outlook, and it is surprised by nothing"*, and
CONFIDENCE is the mean absolute surprise. So a party's first schedule has **width zero**
(`households/consume.ts`: `width: outlook.some ? outlook.value.confidence : 0`), and a point demand
meets a point supply.

**Why that is fatal in a thin market.** Gode and Sunder's result is that *"allocative efficiency of a
double auction derives largely from its structure"* and that a **budget constraint over dispersed
reservation prices** is enough to reach *"close to 100 percent"* efficiency with zero-intelligence
traders ([Gode & Sunder 1993](https://ideas.repec.org/a/ucp/jpolec/v101y1993i1p119-37.html)). The
dispersion is not noise — it is the thing that makes a market cross.

**Proposal.** A party's width is a read of the dispersion it has actually seen, and what it has seen
includes the public record it is entitled to read: a party with one observation of its own and a
visible history of prints has the dispersion of that history, not zero. No floor is introduced and
nothing is widened by decree — what changes is that "how uncertain am I?" is answered from a longer
source than "how often have I personally been surprised?". Combined with the chronicle
(`docs/OPENING.md`), no party ever posts a knife-edge again.

---

## P6. Settlement needs a queue, not only an outcome

**Today.** Every instruction settles atomically or fails; a failed money leg writes an arrear in the
same pass. There is no waiting, no retry within a period, and no resolution of circular shortfalls —
A cannot pay B because B has not yet paid A, and both fail.

**What real systems do.** Large-value payment systems queue, and the queue is the mechanism:
liquidity-saving features find cycles and settle them simultaneously, because a gridlock is not a
default — it is a timing failure, and resolving it needs no new money. (Phoenix already refuses
*netting across counterparties*, and it should keep refusing it: a cycle settled simultaneously is
not netting, it is every leg settling at full value in one pass, which is what a DvP cycle is.)

**What it costs today.** 135 living parties at the opening, **101 after twelve periods**. Some of
those deaths are real; the ones caused by a payment that could have been made an hour later are the
model's own construction.

**Proposal.** A settlement queue with a stated lifetime, in three states the project already has the
vocabulary for: due → queued (this period, retried after each later instruction that funds the
payer) → failed (the arrear, as today). Plus one pass at the end of the period that finds cycles in
the queue and settles them together, each leg at full value. The parameters are one TECHNOLOGY (how
long a payment may wait before it is late) and nothing else.

**Closes.** A large share of the estate churn recorded at 21.36, and the spurious deaths that make
every measurement of this world a measurement of its recovery.

---

## P7. Read the system as a network

**Today.** Bilateral exposures exist everywhere — loans, deposits, contracts, arrears — and nothing
ever reads them **as a graph**. The audit's nine families are per-party and per-instrument. When the
banks stopped quoting every name by period 9 (21.72), the reasons available were `appetite`, `it
cannot cost its own funding` and `nobody lends to a party of this kind`: three per-bank reasons for
what is almost certainly a system-level fact.

**What the best practice is.** DebtRank *"quantifies the extent of financial distress that a
particular node should face under external shocks and the corresponding risk contagion"*, recursively,
without waiting for a capital buffer to be exhausted — *"partial impact on solvency is quantified and
accumulated recursively"* ([DebtRank](https://arxiv.org/pdf/1504.01857),
[systemic risk in interbank networks](https://arxiv.org/pdf/2109.14360)) — and the empirical regularity
is that contagion *"decreases with capitalization but increases with concentration"*, with a
non-monotonic relation to connectivity.

**Proposal.** A network READ in Part XII's sense — published, causing nothing, never repairing: the
exposure graph each period, its concentration, and a DebtRank-style distress propagation from each
node. It is a measurement of the world the model already contains, it needs no new mechanism, and it
turns "the banks stopped lending" from a mood into a structure.

---

## P8. Trade credit is the missing half of firm finance

**Today.** `Trade Credit` is the weakest-covered system in the register that is not a derivative:
**9 clauses MET of 22**. Firms in this world buy with money or not at all.

**Why it is not a detail.** Trade credit is the largest source of short-term finance for firms in
most economies, and the channel through which a customer's failure becomes a supplier's failure.
Phoenix's firms die of a cash timing problem that real firms survive by paying in thirty days — and
its supply chain has no way to transmit distress other than a missed delivery.

**Proposal.** Promote it in the worklist: an invoice with terms, a discount for early payment that is
a price somebody quotes rather than a rate somebody states, and the failure chain that follows when
an invoice is not paid. It shares the agreement register and the arrears machinery that already
exist, so the cost is mostly in the decisions: who offers terms, to whom, and when they stop.

---

## What NOT to take from the literature

Stated because half of a review's value is the doors it closes.

- **Calibration to observed margins** (IPF, simulated minimum distance, stylized-fact fitting). Seed
  B5 and C5 forbid it, and the validation literature itself reports that *"almost any simulation
  output can be generated with an ABM, and thus replication of stylized facts only represents a weak
  test"* ([validation methodology](https://d-nb.info/1246195569/34)).
- **A representative agent anywhere**, including as an optimisation. XI-15 and §41 A2.f already say
  so, and the moment a cell's decision is taken at its average the dispersion that makes markets
  work is gone (P5).
- **GPU execution.** FLAME GPU's thousand-fold speedups are for local, homogeneous, parallel agent
  rules; a Phoenix period is a sequence of global serialisations — one solver, one settlement, one
  audit ([FLAME GPU 2](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3207)).
- **Nudging during a spin-up.** A bound with a nicer name.

---

## The order

P1 and P2 are one change in two parts and should be taken together: a protocol table is not worth
much while every order evaporates at the end of the session. P3 and P4 are the goods chain and are
what make the consumer basket exist. P5 is small and should ride with the chronicle. P6 is
independent and stops the world killing parties it has no reason to kill. P7 is a read and can be
taken any time; it is the cheapest of the eight. P8 is a system and belongs in the worklist rather
than in front of it.

Against the recorded findings: P1–P4 between them close or explain 12c.3, 21.79, 21.81, 21.84 and
the demand side of 21.72; P6 closes a share of 21.36; P7 makes 21.72 diagnosable; P5 and the
chronicle together remove the class of "nobody bid" that is an artifact of a world with no past.

---

## Sources

- [Caiani, Godin, Caverzasi, Gallegati, Kinsella, Stiglitz — Agent based-stock flow consistent macroeconomics (JEDC 2016)](https://www.sciencedirect.com/science/article/abs/pii/S0165188915301020)
- [Riccetti, Russo, Gallegati — An agent-based decentralized matching macroeconomic model (JEIC)](https://link.springer.com/article/10.1007/s11403-014-0130-8) ·
  [Gallegati et al., decentralized matching (PDF)](https://faculty.sites.iastate.edu/tesfatsi/archive/tesfatsi/AgentBasedDecentralizedMatchingMacro.Gallegati2012.pdf)
- [Gode & Sunder — Allocative efficiency of markets with zero-intelligence traders (JPE 1993)](https://ideas.repec.org/a/ucp/jpolec/v101y1993i1p119-37.html)
- [Bouchaud et al. — From Walras' auctioneer to continuous time double auctions](https://arxiv.org/pdf/1506.03758) ·
  [Market opening structures and market quality](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4550773/) ·
  [From Glosten-Milgrom to the whole limit order book](https://ideas.repec.org/p/arx/papers/1902.10743.html)
- [Battiston et al. — DebtRank: a microscopic foundation for shock propagation](https://arxiv.org/pdf/1504.01857) ·
  [Systemic risk in interbank networks: balance sheets and network effects](https://arxiv.org/pdf/2109.14360)
- [Towards a validation methodology for macroeconomic ABMs](https://d-nb.info/1246195569/34)
- [FLAME GPU 2 (SPE 2023)](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3207)
