# WHAT TEST-APP MODELS AND PHOENIX DOES NOT

A one-way comparison, written 2026-09-11 by reading every document in `Project-Phoenix/docs`
(specification, architecture, plan, worklist, coverage, record, bugs) and then every document in
`Test-app/docs` (its master plan and log, its atlas of 45 required trees, its two instrument
contracts, its greenfield architecture, its unmapped list). It answers one question and no other:

> **what system, model or feature is present, in progress or planned in Test-app, and neither
> present nor planned here?**

Modelling content only. Nothing about storage layout, languages, workers, ids or file structure is
a finding in this file, because none of it is a claim about the world.

**This is not a worklist.** Nothing here is scheduled, and Law 10 says a new idea is INSERTED at its
dependency position rather than appended — so each entry ends with *where it would sit* as a
proposal for the owner, not as an edit to `docs/WORKLIST.md`. Law 11 also applies: the entries say
what is absent, not that any number is wrong.

---

## 1. THE RESULT, BEFORE THE DETAIL

**At system level the gap is empty.** Test-app's atlas has 45 systems and 2 instrument contracts;
this specification has 48 systems and the same 2 contracts. Every one of Test-app's 45 maps onto a
section here, and this specification adds three it has no counterpart for (§46 Expectations, §47 The
Polity, §48 Reporting and Estimates). The bond contract's fourteen characteristics and the
derivative contract's twelve plus three prohibitions are the same fourteen and the same twelve on
both sides.

So **every gap is at mechanism or instrument level**, and there are seven of them. Six are small and
one is large. Against that, the great majority of what a reader might expect to find here is
already covered — §4 lists thirty-three near-misses with the clause that covers each, so they are not
re-derived — and a further set is absent **by decision**, because a law here forbids what Test-app
does: §3 lists those, so the absence stays a decision on the record rather than an oversight.

Counted honestly, the one-way difference is:

| | Test-app | here |
|---|---|---|
| systems / trees | 45 | 48 |
| instrument contracts | 2 | 2 |
| derivative classes built or planned | 8 | 7 built or planned, 1 more specified |
| **modelling features absent here** | — | **7** (§2) |
| features absent here **by law** | — | 8 families (§3) |

---

## 2. THE GAPS

### G1 — OPTIONS, AND A PRICE FOR VOLATILITY

The largest gap by some distance, and the only one that is a whole instrument family.

**Test-app status: built.** Its log records the class (§9.17b-i) and then the market (§9.17b-iii);
its derivative-layer and derivative-contract documents map the class against all twelve
characteristics.

**What it models.**

- A contract with a **holder and a writer**, a **type** (call or put), a **strike** per unit, units
  in the underlying's own unit, and a notional that is the exposure at the strike. Its reference is
  either a **named issuer's shares** or a **region's equity index** — the index being a reference a
  derivative can name, which nothing here needs today.
- The **premium is paid once**, holder to writer, in the period the contract is struck, and it is
  the option's value at that period's print. After that the contract **marks every period** and the
  change in the mark moves as **variation margin** between the two parties, like every other class.
- **Expiry is an event**: exercised at intrinsic value or expired worthless, with the true-up beyond
  what the marks already paid. Initial margin is the reference's own measured move; the clearing
  house's member limits bind at the strike, so a member that cannot margin it does not get it.
- A market: **at-the-money index puts at a listed tenor, one book per region.** Demand is every
  institution holding an equity book, sized by the same hedging arithmetic every other book here
  uses — the exposure is the book, what must absorb a one-standard-deviation fall over the tenor is
  the holder's **own surplus**, at its management's **own risk aversion**, less the cover it already
  holds, no more than it can margin. Supply is the dealers' desks on the same balance-sheet budget
  they run every other class on, plus the funds whose **strategy is to sell volatility**.
- **What clears is a volatility.** Each writer posts a reservation *volatility*: the volatility it
  expects to realise, plus the return its capital requires on the position, expressed in volatility
  points. The book clears one number per region — the **implied volatility** — and the class then
  marks at the implied volatility the market cleared, falling back to realised only before the first
  print. A closed-form pricer is used to *value* a contract at that volatility; it is not the
  mechanism that sets it.

**Here today: nothing.** No option in the specification, no option kind in `13b`'s class list (cds,
cds.index, irs, fx.forward, xccy, index.future), none in `13c`'s futures, no row in
`docs/COVERAGE.md`, no worklist item. Measured volatility exists as a read (it widens a dealer's
quote at `Dealer Desks C3` and raises margin at `Derivative Layer D5`); what does not exist is
**traded** volatility — no implied volatility, no market in it, and no party that can be long or
short it.

**What the absence costs.** This model already has optionality it cannot price:

- `Bond N11` requires an early-termination regime and `N11.a` requires it to have **a price the
  issuer pays to use it** — make-whole, non-call, soft call. That price is an option premium.
- `Banks Lending` has prepayment **at the borrower's option**.
- `Short-Term Debt B4` says in its own words that a committed line with no fee is **"a free option
  the lender did not sell"**.
- `§46` gives every deciding party its own outlook and makes the **disagreement** load-bearing
  (`§46 A3`). Parties can disagree about a level; nothing lets them disagree about **dispersion**,
  and no price anywhere carries the market's view of it.

So the model requires the price of an option in three places and has no market in which one is formed.

**The modelling question it raises here, and it is not Test-app's answer.** Test-app clears the
volatility and derives the premium from it. That is the same shape `Bond N7.b` forbids for a bond —
clearing a yield and deriving the price — and `Law 3` states the general rule: the price clears, and
the derived measure is read off it. On this specification's terms the **premium** is what a holder
and a writer agree, and implied volatility is what an observer reads back out of it. The one-book
design, the two roles, the once-only premium, the margin and the expiry event carry over unchanged.

**Where it would sit.** After the derivative classes (`13b`) and after equity has a cleared print
(item 9 / `12c`), because an index option needs an index that clears and a house that margins.
Its natural neighbours are `Indices C3` (something must trade the index) and `13h` (the funds that
would write it).

### G2 — AN INDEX-LINKED OBLIGATION, AND THE OTHER SCHEDULE SHAPES

**Test-app status: planned.** Its greenfield architecture makes it a stated capability requirement
(§3.1 F4: *obligations with arbitrary schedules — regular, irregular, amortising, **indexed**,
contingent on a declared event*) and its §7.3 names what a periodicity formula cannot express: *an
irregular schedule, a **step-up coupon**, a **sinking fund**, an amortiser, a make-whole or a
**payment holiday***.

**Here today.** `Bond N5` admits **exactly three** coupon shapes — fixed, floating, zero — and says
so deliberately. `Corp Credit F3` admits bullet or amortising principal. There is no indexed
coupon or indexed principal anywhere, no step-up, no sinking fund, no payment holiday, and no
coupon contingent on a declared event.

**What the absence costs — and this is the substantive half.** An index-linked bond is the only
instrument in which **inflation itself is priced**. This model has a CPI and a PPI (`Goods G1`,
`Indices D4`) and it gives every party its own inflation outlook (`§46`), but there is nowhere those
outlooks **meet and clear**: no breakeven, so no market-priced expectation of inflation anywhere in
the world, and no **real** cost of funds for the sovereign whose paper the model is otherwise very
careful about. `XI-13` is the law that makes this matter — the market must be able to disagree with
the model — and on inflation it cannot, because there is no instrument to disagree in.

The other shapes are smaller and each buys something specific: a **payment holiday** is the
restructuring-short-of-default that `Corp Credit B2.b` (waiver and cure) and `G7` (restructuring)
already imply and that nothing can express as a *term*; a **sinking fund** and a **step-up** are the
two commonest ways a real issuer shapes its own ladder, which `Sovereign A2` asks to be a programme
rather than a calendar.

**Constraint if it is ever built.** The index must be this world's own CPI print, at a stated lag,
with the indexation on the **principal** as well as the coupon — `Law 2` admits imported
primitives and forbids imported equilibria, and an imported inflation series would be the second
kind.

**Where it would sit.** Against the bond contract, after `12a` reporting and the indices are
complete (an index-linked bond needs a CPI that is a real read), and before `14` the polity — a
sovereign that issues linkers has a different funding constraint (`XI-9`).

### G3 — THE CENTRAL-BANK SWAP LINE AS A FACILITY

**Test-app status: built.** Its log records the draws as rows (§9.20-LLR-b) and, before that, the
facility itself; its FX-forwards tree cites it at the node about a widening basis: *the basis has a
ceiling that is a price — the central banks' swap lines lend the unfilled funding at overnight plus
a stated spread once the basis clears past it, so the widening runs to the line's price and stops.*

**What it models.** A standing arrangement between two central banks: one lends its own money to
the other against the other's money at the current rate, for a term, and the drawing central bank
**on-lends it to its own banks** at the overnight rate plus a stated spread, against collateral,
limited by the line. The consequence is that the cross-currency basis has a **price ceiling that is
itself a price** — not a bound — and the ceiling appears on both central banks' balance sheets as
a real claim and a real liability in two monies.

**Here today.** `§31 A2.b` names *"claims on other central banks, swap-line draws as rows"* — the
**row** exists, as an asset class on the central bank's sheet. Nothing names the **facility**: who
may draw, at what price, against what, to what limit, and what the drawing central bank does with
the money. No plan item mentions it; `13i` (cross-border) does not.

**What the absence costs.** `FX Forwards B3.a` requires the basis to widen when funding in one
currency is scarce, and `Law 6` forbids bounding it. With no facility, the only thing that stops a
basis widening is private balance sheet — which is right as far as it goes, and leaves out the
mechanism that actually stops it in the world. It is also the joint `XI-3` (nothing immortal) needs
most: a bank short of a money its own central bank cannot print is the one liquidity failure the
lender of last resort cannot reach, and the swap line is why it sometimes can.

**Where it would sit.** With or immediately after `13i` (cross-border). It cannot come before the
private cross-currency funding market (`13b`'s `xccy`) exists — a facility that prices off a basis
needs the basis to clear first.

### G4 — THE PHYSICAL ENVIRONMENT AS STANDING STATE

**Test-app status: built.** A per-region weather/anomaly state that runs every period: a stated
share of a commodity's units is lost **where they would have been made**, a plant's finished output
falls short of what it started, and a freight lane can be affected. Its commodity tree marks the
disruption node satisfied by it; its goods tree marks its yield node by it; its insurers tree names
the same anomalies as the exposure a catastrophe would run through.

**Here today, the consequences are all required and the cause is not.**

- `Commodities B3` — production can be disrupted, and a disruption is a **real loss of units** at
  the point they would have been made.
- `Goods B4` — **yield**: not everything started is finished; scrap is a loss of units.
- `Freight B4` — capacity can be **lost or blocked**.
- `Insurers B4` — **a catastrophe is one event hitting many policies at once**, which is different
  from the average being higher.

What produces any of them is, in the plan, a **scenario seed's event** (`13c` for a disruption,
`13h` for a catastrophe) plus per-line claim **frequency and severity** declared as technology
primitives (`13h`). There is no state of the physical world in the *running* model, and therefore no
single cause that reaches several systems at once.

**What the absence costs.** A hazard declared only inside insurance means the insurer's loss and the
producer's loss are **two unrelated draws for one event** — the same "one real thing, two
representations" that `Law 4` exists for, one level above a number. And the chain
`Commodities E4` names (commodity shock → margins → inflation → policy) can only ever be exercised
by injecting a scenario, never by the world producing one.

**Constraint if it is ever built.** It is **TECHNOLOGY** under `Law 2` — a physical fact with a
unit, an owner, a region and a period — read by commodities, goods, freight and insurance, and
written by none of them. Not a probability parameter living in the insurance module: `13h` already
refuses `catastrophe.probability` as a primitive, and this is the same refusal seen from the other
side.

**Where it would sit.** With `13c` (commodities and freight), because that is where the units are;
read by `13h` when the cover market lands.

### G5 — PRODUCTIVITY THAT IMPROVES WITH CUMULATIVE OUTPUT

**Test-app status: built, and unmapped by its own atlas.** A firm carries a learning position; the
plant-utilisation read feeds a Wright's-law learning curve; there is also a trend productivity
drift, which Test-app's own audit calls an undeclared shape. No required tree in its atlas describes
any of it — the file is on its unmapped list — so this is a mechanism that exists in its engine and
that its reference model never asked for.

**Here today.** A firm's productivity is its **technology**, a declared primitive: the recipe's
hours scaled by that firm's own technology number. Nothing makes a unit cheaper with experience and
nothing makes a firm better at making a thing by having made more of it. There is no R&D and no
innovation decision anywhere — `research` in this specification is **sell-side research** (`§48`),
an assessor's estimate, not a firm's spending.

**What the absence costs.** `Firm A3` requires firms to differ in cost, and they do — by seed
dispersion that then never changes. So relative cost is fixed at birth up to scale: an entrant can
never out-learn an incumbent (`Firm Birth A4`, `A5`), and the only return the capital programme's
expansion can earn is capacity, never a lower unit cost. Nothing in the world gets cheaper to make.

**Constraint if it is ever built.** A **drift is a written path** and `Appendix B` forbids that
class of number. The admissible form is TECHNOLOGY keyed to **cumulative units actually produced** —
a state the register can answer, carried by the firm, falling with experience at a declared rate —
so that a firm's unit cost is a consequence of what it has made, not of how long it has existed.
R&D, if it is ever wanted, is a separate mechanism (a spend, a lag, an uncertain outcome) and not
this one.

**Where it would sit.** With the firm's cost base (item 4's descendants) and upstream of the goods
recipe work, because it changes what a unit costs.

### G6 — GREENFIELD CROSS-BORDER DIRECT INVESTMENT, AND THE PARENT–SUBSIDIARY GROUP

**Test-app status: built.** A third birth path beside the spin-off and the domestic founding: a
parent capitalises a **new firm in another region**, and the subsidiary's opening plant comes from
the parent. Its log lists the three births together wherever equity, plant or the register is
discussed; the group relation survives a merger.

**Here today, two thirds of it and not the third.** `Cross-Border C4` names direct investment as
*buying a firm outright* — an acquisition, which `13g` covers. `Firm Birth A2` lets **any named
investor** fund a birth, and `A2.a` requires a greenfield build to **buy** its plant from a
producer — so the capability is admitted. What is nowhere here is the **relation**: a firm whose
owner is another firm in another region, and what follows from it —

- solo versus **consolidated** accounts (`§48` reports a company's own statements; a group is not a
  reporting entity here, and "consolidated" appears in this specification only for the central
  bank's holding of sovereign debt);
- **profit repatriation** as an intra-group dividend across a border (`Cross-Border C5` carries the
  flow, not the intra-group case);
- a parent that can **support or abandon** a subsidiary, which is `XI-3`'s question asked of a
  group rather than of a firm.

**What the absence costs.** Building abroad is the main route by which a real firm acquires a cost
base in another currency, and the only one that creates a **new competitor** in the foreign market
(`Firm Birth A5`) rather than changing the owner of an existing one. Without it every cross-border
corporate position in the model is either portfolio or acquisition.

**Where it would sit.** `13i` (cross-border), after `13g` (M&A and birth) — it is a birth with a
foreign funder, so it needs both.

### G7 — THE COVERED BOND

**Test-app status: named in its plan as a worked example, not a committed item.** Its greenfield
architecture uses the covered bond as the case that exercises every joint at once (§17.1): a bank's
bond **secured on a ring-fenced pool of its own loans**, where the pool must be replenished to hold
a cover ratio (a substitution is a release then a pledge, and a release below the ratio is refused),
and where holders have recourse **both** to the pool and to the bank.

**Here today.** Securitisation (`13e`, `XI-11`) moves loans **off** the balance sheet into a vehicle
with tranches and attachments. A covered bond is the on-balance-sheet alternative and a different
instrument: the loans stay with the bank, and the holder has a lien on a named pool **plus** an
unsecured claim on the bank. `Banks Funding A2` names "paper it issues" with no secured variety, and
`XI-8`'s waterfall has no dual-recourse claim to rank.

**What the absence costs.** It is the instrument a bank reaches for when unsecured funding closes —
the one funding channel that survives a downgrade — which is exactly the state `Banks Funding` and
`Money Market` are built to reach. Listed last because Test-app carries it as an illustration
rather than as work.

**Where it would sit.** With `13e` (pools and securitisation), as its on-balance-sheet sibling.

---

## 3. PRESENT IN TEST-APP, ABSENT HERE BY DECISION

Not findings. Each is something Test-app's engine has and this specification **forbids**, so the
absence is a decision with a citation. Recorded because "MISSING" and "OUT OF SCOPE" are different
answers and neither should be discovered twice. In several of these Test-app's own rules agree with
ours and its atlas marks the thing as its own defect.

| Test-app has | forbidden here by |
|---|---|
| A region-level macro block: GDP, an unemployment rate, a NAIRU, an output gap, a consumer-confidence index, trend productivity growth, seeded household debt ratios, a neutral rate stated as productivity plus target | `App B` no stored aggregate, no unemployment rate, no sentiment coefficient, no decision at an average; `Cross-Border D1` every aggregate is a read |
| An inertial **Taylor rule** setting the policy rate, and a posted policy rate used as the floating-rate benchmark | `App B` no policy set directly or by path; `XI-7` a posted policy rate is not a benchmark; `§47`, `XI-17` the polity sets a mandate, never a rate |
| A **constant-NAV money fund** that cannot break the buck | `App B` no constant NAV; `Fund Shares` NAV is a read |
| **Loss rates** where an event belongs: a bank loan that erodes, a mortgage default that is a rate, a pool whose losses are a share | `XI-1` a loss is an event, not a rate; `Firm Birth C2.a` no exogenous default event |
| **Comparable-multiple** valuation of unlisted firms, and a sponsor's own balance sheet as the unlisted mark | `App B` price never from a multiple or a DCF; `13h` a private mark is not a price |
| A **player** with a portfolio, cash and P&L | `§45` no privileged actor, no surface that changes the model. `§45 A4` — inspector or participant — is the one question this specification reserves for the owner, so a player, if ever wanted, enters there and nowhere else |
| **Seeded equilibria**: an opening spread table that strikes every coupon, a fitted opening yield curve, a seeded FX rate | `Law 2` real-world primitives may be imported, real-world equilibria may not; `Seed` |
| A **synthetic index level** with a fabricated pre-history, and index levels stored | `Indices` no stored index level, no index that inputs to its constituents |

---

## 4. CHECKED, AND NOT A GAP

Thirty-three things a reader might expect in §2 and which are already required here. The clause is
given so none of them is re-derived.

| looked for | covered by |
|---|---|
| A bank's short-term paper / certificate of deposit | `Short-Term Debt A3` — types by issuer: the state, **a bank**, a firm, and the type is the credit |
| A commitment fee on an undrawn committed line | `Short-Term Debt B4` |
| Primary dealers with an obligation to bid, and the privilege paid for it | `Sovereign C3` |
| Auction **tail** and **cover ratio** | `Sovereign C4` |
| An announced issuance programme rather than a calendar | `Sovereign A2`, `Treasury` |
| Buyback and switch as the sovereign's liability management | `Sovereign B6`, `F5`; `Bond N11` |
| A re-opening / tap of an existing line | `Register B1` (a re-opening changes the issued amount), `Sovereign B3.a` |
| Covenants, breach, **waiver and cure**, acceleration | `Corp Credit B2`, `B2.a`, `B2.b`; plan `13f` |
| **Restructuring** and an exchange offer | `Corp Credit G7`; `Sovereign G` |
| Sovereign default in a foreign money, and market exclusion | `Sovereign A4.b`, `G`; `Bond N12`/`N13` per type |
| A subordinated rung that actually ranks | `Bond N13.a`; `XI-8` |
| **Wrong-way risk** priced in the buyer's reservation | `CDS E2`; plan `13b` |
| **Net notional** per reference entity as a read | `CDS E3` |
| A deliverable bond future, the net basis, and the basis trade | `Sovereign I1`–`I3.a` (see §5) |
| An equity index future | plan `13b` `index.future`, `Indices C3` |
| A relative-value book and a registry of comparables | `Hedge Funds C1`; `XI-12`, `App B 21` — no free arbitrage, and no arbitrageur without a limit |
| A forward rate derived from the curve | `IRS C2` |
| A **fixing** that is an observation, with a lag | `IRS A3`; `Indices D3` |
| **Re-pledging / rehypothecation** as a traceable chain | `Sec Lending C5` |
| **Custody**: the broker holds the client's assets | `Prime Brokerage A2` |
| Manufactured payments on a stock loan | `Sec Lending A3` |
| ETF **creation and redemption in kind**, authorised participants | `Fund Shares C`, `G1.a` |
| Index trackers that hold weight whatever it costs | `Equity C2.c`; `Indices C1` |
| **Treasury shares** as a holder of the issuer's own stock | `Equity C2.d` |
| **Currency in circulation** | `Money A1.c` |
| Synergies that must show up as real revenue or cost | `M&A E3` |
| PE **management fee and carried interest** | `Private Equity`; plan `13h` |
| **Inheritance** on a household's dissolution | `Households F2`; `XI-15` (an inheritance splits the cell) |
| Inventory at the **lower of cost and net realisable value**, the write-down a charge, never written up | `Goods E2`, `E2.a` |
| **Cost of goods sold** on delivery and absorption of an idle line's cost | `Goods F5`, `B5.a`, `B5.b` |
| The **option to wait** when the spend is irreversible | `Capital Programme B4`, `C4` |
| A catastrophe as one event on many policies | `Insurers B4` (its **cause** is G4) |
| Terms of trade moving with a commodity price | `Commodities E3` |

---

## 5. ONE THING FOUND ON THE WAY, AND IT IS OURS

Not a Test-app gap — the opposite. `Sovereign I1`, `I1.a`, `I2`, `I3` and `I3.a` specify the
**deliverable future on the benchmark bond**, the **net basis** measured against carry, the two
sides of that line, and the **basis trade** that `I3` calls *"the largest single source of real repo
demand in a real market"*, with `I3.a` forbidding a basis trader that cannot lose.

`docs/COVERAGE.md` carries four of the five — `I1`, `I2`, `I3`, `I3.a` — all **MISSING with no
evidence**, and has no row at all for `I1.a`, the net basis. No worklist item and no plan file names
any of them: the words "bond future" do not occur anywhere in `docs/`, the specification included,
which is why looking for the item finds nothing. Test-app has the whole branch built and marked ✅,
including the forced cut on a drawdown that `I3.a` demands.

So this is the one place where a specified system here has no owner in the plan, and it is worth a
decision: either an item at its dependency position (after `13b`'s classes and the repo market it
would create demand in), or an explicit note that it waits. It is recorded here rather than acted
on, because Law 10 makes the insertion the owner's call.
