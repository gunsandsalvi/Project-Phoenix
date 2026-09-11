# MISSING MECHANISMS

Eight mechanisms this world does not have. Seven the specification does not name at all; one it
names and nothing owns. Each entry says what the mechanism is, what its absence costs, what this
specification's own laws would require of it, and where it would sit.

Nothing here is scheduled. Law 10 makes an insertion the owner's call, and Law 11 says a
misbehaving number is not a work item — these are absences, not findings about numbers.

| | mechanism | what the model cannot do without it |
|---|---|---|
| **M1** | **Options, and a price for optionality** | price the optionality it already requires; hold a view on dispersion |
| **M2** | **An index-linked obligation** (and step-up, sinking fund, payment holiday, contingent coupon) | price inflation; give the sovereign a real cost of funds; shape a ladder |
| **M3** | **The central-bank swap line as a facility** | stop a cross-currency basis with a price instead of a balance sheet |
| **M4** | **The physical environment as standing state** | produce one real shock that reaches several systems at once |
| **M5** | **Productivity that improves with cumulative output** | let anything get cheaper to make; let an entrant out-learn an incumbent |
| **M6** | **Greenfield direct investment, and the parent–subsidiary group** | enter a foreign market by building; consolidate; repatriate; abandon |
| **M7** | **The covered bond** | fund a bank when unsecured funding closes |
| **M8** | **The deliverable bond future, the net basis, the basis trade** | generate real repo demand; trade cash against futures (specified, unowned) |

---

## A. NOT SPECIFIED AT ALL

### M1 — OPTIONS, AND A PRICE FOR OPTIONALITY

The largest of the eight, and the only one that is a whole instrument family. There is no option
anywhere: no contract, no market, no party that can be long or short volatility. Measured volatility
exists as a read — it widens a dealer's quote (`Dealer Desks C3`) and raises margin
(`Derivative Layer D5`) — but nothing **trades** it, so no price in this world carries anybody's view
of how far something will move.

**What the absence costs.** The specification requires the price of optionality in three places and
provides no market in which one is formed:

- `Bond N11` requires an early-termination regime — callable, make-whole, non-call, soft call — and
  `N11.a` requires it to have **a price the issuer pays to use it**. That price is an option premium.
- `Banks Lending` has prepayment **at the borrower's option**, which is the same premium seen from
  the lender's side.
- `Short-Term Debt B4` states the problem in its own words: a committed line with no fee on undrawn
  headroom is **"a free option the lender did not sell"**.

And it costs the expectations layer its second dimension. `§46` gives every deciding party its own
outlook and `§46 A3` makes the **disagreement** load-bearing — it is what gives a market two sides.
Parties can disagree about a level; nothing lets them disagree about **dispersion**, so a world of
identical variance opinions is assumed rather than cleared.

**What it has to be here.**

- **The premium is what clears.** Clearing a volatility and deriving the premium from it is the shape
  `Bond N7.b` forbids for a bond — clearing a yield and deriving the price — and `Law 3` states the
  general rule. The premium clears from real demand against real supply; **implied volatility is the
  derived measure an observer reads back off it**, like a spread off a bond price.
- **Two named counterparties** (`Derivative D1`), a holder and a writer, with the contract an asset to
  one and a liability to the other, and the marks across the two summing to zero (`D1.b`).
- **A premium paid once**, holder to writer, in the period the contract is struck — a periodic leg
  that fires exactly once — and then a **mark every period** whose change moves as variation margin
  between the two parties (`D8`, `D8.a`, `D9`).
- **Expiry is an event**: exercised at intrinsic value or expired worthless, truing up beyond what
  the marks already paid (`D11`, `D11.a`).
- **Initial margin from the reference's own measured move**, and the clearing house's member limits
  binding at the strike, so a member that cannot margin a position does not get it.
- **Demand with a reason**: a holder buying cover for a book it actually holds, sized by what its own
  surplus must absorb over the tenor at its management's own risk aversion, net of cover it already
  holds — never a hedge ratio.
- **Supply with a reason**: a dealer's desk writing out of the same balance-sheet budget it runs every
  other class on, and a fund whose strategy is to be short dispersion — each quoting from the
  volatility it expects to realise plus the return its capital requires on what the position consumes.
- **An underlying that is priced elsewhere** (`Derivative D3`): a listed line's own cleared print, or
  an index that clears (`Indices C3`). `D3.a` forbids an underlying that exists only inside the
  derivative, which is why this cannot come before equity prints properly.

**Where it would sit.** After the derivative classes (`13b`) and after the equity book clears
(item 9 / `12c`). Its neighbours are `Indices C3` and `13h` — the funds that would write it.

### M2 — AN INDEX-LINKED OBLIGATION, AND THE OTHER SCHEDULE SHAPES

`Bond N5` admits **exactly three** coupon shapes — fixed, floating, zero — and says so deliberately;
`Corp Credit F3` admits bullet or amortising principal. So there is no indexed coupon and no indexed
principal anywhere, and no step-up, sinking fund, payment holiday, or coupon contingent on a
declared event.

**What the absence costs.** An index-linked bond is the only instrument in which **inflation itself
is priced**. This world has a PPI and a CPI (`Goods G1`, `Indices D4`) and it gives every party its
own inflation outlook (`§46`), but there is nowhere those outlooks **meet and clear**: no breakeven,
therefore no market-priced expectation of inflation anywhere, and no **real** cost of funds for the
sovereign whose paper the rest of the model treats so carefully. `XI-13` is the law that makes this
bite — the market must be able to disagree with the model — and on inflation it cannot, because there
is no instrument to disagree in.

The other shapes are smaller and each buys one thing. A **payment holiday** is the
restructuring-short-of-default that `Corp Credit B2.b` (waiver and cure) and `G7` (restructuring)
already imply and that nothing can express as a *term* of the paper. A **sinking fund** and a
**step-up** are the two commonest ways a real issuer shapes its own ladder, which `Sovereign A2`
asks to be a programme rather than a calendar.

**What it has to be here.** The indexation reference is **this world's own CPI print**, at a stated
publication lag, applied to the **principal** as well as the coupon, with the uplift a real claim
that settles. `Law 2` admits imported primitives and forbids imported equilibria: an imported
inflation series would be the second kind. `Law 8` applies at the writer — the lag, the base period
and the periodicity are part of the number.

**Where it would sit.** Against the bond contract, after `12a` (reporting and estimates) and the
indices are complete, so the reference is a real read; before `14` (the polity), because a sovereign
that issues linkers has a different funding constraint (`XI-9`).

### M3 — THE CENTRAL-BANK SWAP LINE AS A FACILITY

`§31 A2.b` names *"claims on other central banks, swap-line draws as rows"* — the **row** exists, as
an asset class on the central bank's sheet. Nothing names the **facility**: who may draw, at what
price, against what, to what limit, and what the drawing central bank does with the money.

**What the absence costs.** `FX Forwards B3.a` requires the cross-currency basis to widen when
funding in one currency is scarce, and `Law 6` forbids bounding it. With no facility the only thing
that stops a basis widening is private balance sheet — right as far as it goes, and missing the
mechanism that actually stops it in the world. It is also the joint `XI-3` needs most: a bank short of
a money its own central bank cannot print is the one liquidity failure the lender of last resort
cannot reach, and the swap line is why it sometimes can.

**What it has to be here.** A standing arrangement between two named central banks: one lends its own
money against the other's at the current rate for a term; the drawing central bank **on-lends it to
its own banks** at the overnight rate plus a stated spread, against collateral, limited by the line.
Both legs are instructions on both balance sheets, in two monies, with the revaluation account
`§31 A2.c` already requires. The consequence is that the basis meets a **ceiling that is itself a
price**, not a bound (`Law 6`, `XI-14`) — and a facility priced above the market does not bind, which
is the point.

**Where it would sit.** With or just after `13i` (cross-border). It cannot precede the private
cross-currency funding market (`13b`'s `xccy`): a facility that prices off a basis needs the basis to
clear first.

### M4 — THE PHYSICAL ENVIRONMENT AS STANDING STATE

Four sections require the consequences of a physical shock and nothing produces one as part of the
running world:

- `Commodities B3` — production can be disrupted, and a disruption is a **real loss of units** at the
  point they would have been made.
- `Goods B4` — **yield**: not everything started is finished.
- `Freight B4` — capacity can be **lost or blocked**.
- `Insurers B4` — **a catastrophe is one event hitting many policies at once**, which is different
  from the average being higher.

What produces any of them today is a **scenario seed's event** (`13c` for a disruption, `13h` for a
catastrophe) plus per-line claim frequency and severity declared as technology primitives (`13h`).
There is no state of the physical world that varies period to period, and therefore no single cause
that reaches several systems at once.

**What the absence costs.** A hazard declared only inside insurance means the insurer's loss and the
producer's loss are **two unrelated draws for one event** — `Law 4`'s "one representation per real
thing", one level above a number. And the chain `Commodities E4` names — commodity shock to margins
to inflation to policy — can only ever be exercised by injecting a scenario, never by the world
producing one.

**What it has to be here.** **TECHNOLOGY** under `Law 2`: a physical fact with a unit, an owner, a
region and a period, carried as state, read by commodities, goods, freight and insurance and written
by none of them. One event, one cause, several consequences, each a real destruction of units or
capacity where it happens. Not a probability parameter living in the insurance module — `13h` already
refuses `catastrophe.probability` as a primitive, and this is that refusal seen from the other side.

**Where it would sit.** With `13c` (commodities and freight), where the units are; read by `13h` when
the cover market lands.

### M5 — PRODUCTIVITY THAT IMPROVES WITH CUMULATIVE OUTPUT

A firm's productivity is its **technology**: a declared primitive, the recipe's hours scaled by that
firm's own number. Nothing makes a unit cheaper with experience, and nothing makes a firm better at
making a thing by having made more of it. There is no R&D and no innovation decision anywhere —
`research` in this specification is **sell-side research** (`§48`), an assessor's estimate, not a
firm's spending.

**What the absence costs.** `Firm A3` requires firms to differ in cost, and they do — by a dispersion
set at the seed that then never changes. Relative cost is therefore fixed at birth up to scale: an
entrant can never out-learn an incumbent (`Firm Birth A4`, `A5`), the capital programme's expansion
can only ever buy capacity and never a lower unit cost, and nothing in the world gets cheaper to
make. A model whose costs only ever move with input prices has no supply side of its own.

**What it has to be here.** A **drift is a written path** and `Appendix B` forbids that class of
number, so the admissible form is TECHNOLOGY keyed to **cumulative units actually produced** — a
state the register can answer, carried by the firm, with the rate of decline declared and owned — so
that a firm's unit cost is a consequence of what it has made rather than of how long it has existed.
It lands in unit cost (`Goods B5`) and therefore in the offer, the wage bid and the margin, which is
why it cannot be a display number. R&D, if it is ever wanted, is a different mechanism — a spend, a
lag, an uncertain outcome — and not this one.

**Where it would sit.** With the firm's cost base (item 4's descendants), upstream of the goods
recipe work, because it changes what a unit costs.

### M6 — GREENFIELD DIRECT INVESTMENT, AND THE PARENT–SUBSIDIARY GROUP

Two thirds of this exists. `Cross-Border C4` names direct investment as **buying a firm outright** —
an acquisition, which `13g` covers. `Firm Birth A2` lets **any named investor** fund a birth, and
`A2.a` requires a greenfield build to **buy** its plant from a producer. What is nowhere is the
**relation**: a firm whose owner is another firm, in another region, and what follows from it —

- solo versus **consolidated** accounts (`§48` reports a company's own statements; a group is not a
  reporting entity here, and "consolidated" appears in this specification only for the central bank's
  holding of sovereign debt);
- **profit repatriation** as an intra-group dividend across a border (`Cross-Border C5` carries the
  flow, not the intra-group case, and `XI-12` decides the money it settles in);
- a parent that can **support or abandon** a subsidiary — `XI-3` asked of a group rather than of a
  firm, with `Firm Birth E2`'s destination for everything the subsidiary held.

**What the absence costs.** Building abroad is the main route by which a real firm acquires a cost
base in another currency, and the only one that creates a **new competitor** in the foreign market
(`Firm Birth A5`) rather than changing the owner of an existing one. Without it every cross-border
corporate position is either portfolio or acquisition, and a multinational's exposure to a currency
is a translation question rather than a real one.

**What it has to be here.** The parent funds the subsidiary out of a named account, across the border,
in a money somebody sold it (`Spot FX F1`); the subsidiary buys its plant from a producer (`Firm
Birth A2.a`, no minted endowment); the parent's holding is a register row in the subsidiary's own
equity; dividends travel the same way, priced and settled; consolidation is a **read** over the
group, never a stored aggregate.

**Where it would sit.** `13i` (cross-border), after `13g` (M&A and birth) — it is a birth with a
foreign funder, so it needs both.

### M7 — THE COVERED BOND

Securitisation (`13e`, `XI-11`) moves loans **off** the balance sheet into a vehicle with tranches and
attachments. The on-balance-sheet alternative does not exist: a bank's bond **secured on a
ring-fenced pool of its own loans**, where the pool is replenished to hold a cover ratio and the
holder has recourse **both** to the pool and to the bank. `Banks Funding A2` names "paper it issues"
with no secured variety, and `XI-8`'s waterfall has no dual-recourse claim to rank.

**What the absence costs.** It is the instrument a bank reaches for when unsecured funding closes —
the one channel that survives a downgrade — which is exactly the state `Banks Funding` and
`Money Market` are built to reach. Without it a bank's only answers to a funding squeeze are repo
against sovereign collateral and the central bank.

**What it has to be here.** The pool is **liens on named loans** the bank still owns (`Register`
liens), pledgor the bank and beneficiary the bond; a substitution is a release then a pledge, and a
release that would break the cover ratio is refused at the call, not corrected afterwards; the
holder's claim ranks on the pool first and then **unsecured against the bank** in the same waterfall
as everything else (`XI-8`). The cover ratio is a term of the instrument, not a bound on a number.

**Where it would sit.** With `13e` (pools and securitisation), as its on-balance-sheet sibling.

---

## B. SPECIFIED HERE, AND NOTHING OWNS IT

### M8 — THE DELIVERABLE BOND FUTURE, THE NET BASIS, AND THE BASIS TRADE

`Sovereign I1` specifies a **deliverable future on the benchmark bond** — a named line is the
deliverable, the price is per unit of face, and at delivery it settles to that bond's own cleared cash
price. `I1.a` makes the **carry** the tie to the cash market and the **net basis** a measurement
against it, never a setting. `I2` names who is on the line: a duration mandate short of duration goes
long below carry, a holder over its sovereign target shorts the excess above it, a dealer quotes both
ways at carry. `I3` is the **basis trade** — long the cash bond, financed in repo, short the future
when the basis pays for it — which the clause itself calls *"the largest single source of real repo
demand in a real market"*. `I3.a` forbids a basis trader that cannot lose: it is funded, margined and
cut on a drawdown.

**Its state.** `docs/COVERAGE.md` carries four of the five — `I1`, `I2`, `I3`, `I3.a` — all **MISSING
with no evidence**, and has no row at all for `I1.a`, the net basis. No worklist item and no plan file
names any of them.

**What the absence costs.** `I3` is the sentence: without the basis trade the repo market has no
large, price-sensitive source of demand, so repo clears against liquidity needs alone and the secured
curve is thinner than the model claims. It is also the one place a cash market and a derivative market
are tied by delivery rather than by a formula (`Derivative X3`).

**Where it would sit.** After `13b`'s classes, with the repo demand it creates. It needs a decision
rather than a finding: an item at that position, or an explicit note that it waits.
