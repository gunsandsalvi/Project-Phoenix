# Line audit — every file of the engine, against what was claimed built

The method: read the source, file by file, in dependency order. For each file check
(1) does the mechanism its `@spec` names actually exist here, (2) is it REACHABLE — called from a
phase, a participant, a family or the seed, (3) is it INERT — present and never doing anything in a
real run, (4) does it break one of the 19 laws in a way the lint does not catch: a bound, a kind
branch, a second writer, a stale comment, a number that should be an outcome.

Verdicts: `OK` — does what it says. `THIN` — exists, runs, but is a narrower mechanism than the
claim. `INERT` — exists, never runs or never produces anything. `ABSENT` — claimed and not there.
`DEFECT` — present and wrong.

## The first pass: what exists, and what has ever happened

Two mechanical passes over all 59,927 lines, then a runtime census of `foundationWorld('real')` at
five periods.

**Pass 1 — the tells for unfinished work.** `TODO`, `FIXME`, "for now", "stub", "placeholder for",
"simplification", "approximate", "pretend", "fake", "dummy", "hack": **26 hits, every one of them
prose in a comment explaining why something is NOT an approximation.** No abandoned work markers
anywhere in the engine.

**Pass 2 — exported code nothing references.** 62 exported symbols are referenced nowhere in the
engine, the tests or the app. Two clusters matter:

- `mechanisms/insurers/index.ts` exports `policyId`, `policyTerms`, `quoteCover`, `venueForCover`
  and `runCover`, and the module declares `phases: []` and `participants: []`. **The insurer has no
  phase, no participant and no venue: nothing it exports is ever called.**
- `mechanisms/securities-lending/index.ts` exports `runBorrows`, `returnLoans`, `wantsToBorrow` and
  `loansOpen`; the module declares one phase (`borrow.economics`) and `participants: []`.

**Pass 3 — the census. What has ever EXISTED after five periods of the real world.**

| declared | ever exists |
|---|---|
| 14 party kinds | 13 — **`insurance` never** |
| 16 instrument kinds (excl. goods/wip/plant) | 11 — **`corporate.bond`, `policy`, `margin.claim`, `defaultFund.contribution`, `closeOut.claim` never** |
| **9 derivative kinds** | **NONE. Not one contract of any class has ever been written.** |
| 3,618 markets | 1,285 have ever printed; **2,333 never have** |

**Why.** Every contract session that has ever run, over five periods:

| book | sessions | outcome |
|---|---|---|
| option | 7,510 | `noDemand`, every one |
| commodity.future | 3,680 | `noDemand`, every one |
| cds | 60 | `noSupply`, every one |
| fx.forward | 60 | 34 `nothingSettled`, 14 `noOverlap`, 12 `noDemand` |
| bond.future | 30 | `noDemand` |
| index.future | 20 | `noDemand` |
| irs | 0 | the books are never run at all |

And the asset markets: 6,695 sessions, **772 cleared** — 11.5%.

### What this means against what was claimed

- **13a "the derivative layer" and 13b "the derivative classes" (37 steps):** the layer, the house,
  the margin, the default fund and nine classes are written, wired, and have never produced a single
  contract. `margin.claim`, `defaultFund.contribution` and `closeOut.claim` have never been
  instantiated because there is nothing to margin, mutualise or close out.
- **13f "the corporate bond ... it can fail, it cross-defaults, it carries COVENANTS":** no
  corporate bond has ever been issued.
- **13h "closed with the INSURER built":** no insurer exists and no policy has ever been written.
- **This session's Law 18 work:** I spent it making the world ask 15 million questions a period
  faster, about books in which nothing has ever traded. The speedups are real and the digest gates
  hold; what they optimised is a machine whose output is nothing.

The honest summary of the block-13 rows reading `done`: the CODE is there and reachable, and the
world does not use it. "Built" was true of the source and false of the world.

---

## The second sweep, from scratch: what verifies, what runs, what the numbers say

Different instruments from the first. The first asked what EXISTS; this asks what VERIFIES and what
HAPPENS.

### The audit is green over empty sets

Nine families, all `built: true`. Six report zero violations. But a family over an empty set reads
green, which is the same lie item 13b.1 was named after. The sets, measured at three periods:

| family | walks | size |
|---|---|---|
| **zeroSum** | `contracts.open_()` | **0** |
| **accounts** (contract half) | `contracts.openOf(party)` | **0** |
| currency | `journal 'revaluation.fx'` | 1,051,429 |
| crossMarket | `indexList` | 29 |
| ownership, prices | `instruments.all()` | 15,752 |
| names | `parties.all()` | 11,378 |
| flows, money, units | `ledger.inPeriod` | 158,429 |

**`zeroSum` is the invariant that a derivative nets to zero between its two sides — the single most
important thing the derivative layer must be true of — and it has never inspected one row.**

### Numbers declared that nothing reads

29,559 parameters declared; 17,040 read in a run; **12,519 never read.** Discounting the ones read at
seed time before the instrument was attached (geography, terrain, `seed.*`), the substantive ones:

| never read | what it is |
|---|---|
| 5,896 | `firm.hurdle.<firm>` |
| 5,896 | `firm.horizon.<firm>` |
| 493 | `equity.payoutPatience.<firm>` |
| 30 | `bank.lending.capitalAtRisk.<bank>` |
| 9 | `fund.requiredYield.<fund>` |
| 7 | `fund.fee.<fund>` |

**Correction to my own first reading of this**: the hurdle IS read, at `firms/decide.ts:461` — but
inside `cost.some ? project(...) : none()`, so a firm whose `costOfCapital` is none never evaluates a
project at all. Measured: **9,006 firms alive, 3,104 ever evaluate one.** Two thirds of the economy
never makes an investment decision, because no bank has quoted them and no market prices their
equity.

### What the world actually does, over three periods

| | |
|---|---|
| instructions settled | 269,139 |
| **parties that have ever taken delivery of PLANT** | **0** |
| **loans ever written** | **6** |
| credit quotes published | 27,168 |
| **companies that have ever published accounts** | **0** |
| **research estimates** | **0** |
| **parties that have ever ceased** | **0** |
| labour hires | 1,607 |
| goods perished in store | 51,521 |
| rating actions | 29,445 |
| **FX revaluation events** | **1,051,429** |

So: nothing is ever built, six loans exist, nobody reports, nobody dies, and the two largest event
streams in the world are revaluing foreign balances and rotting food.

### The root cause of the dead derivative sector — measured, not guessed

The layer records a refusal for every trade it will not admit. Over two periods: **31,640 admission
decisions, 31,640 refusals.** Diagnosed against the three gates in `capacity().admits`:

| gate | count |
|---|---|
| the clearing house has ceased | 0 |
| **no room — the party holds no cash in the book's currency** | **31,640** |
| the kind cannot say what one unit's margin is | 0 |
| admitted | **0** |

`capacityOf(view, ccy, buffer)` is `cash − cash × buffer`, so room is zero exactly when the party
holds none of that money. Every one of those is an FX forward: a party trading EUR/GBP must post
margin in EUR or GBP and holds neither. **In the real world it would buy the currency or post
eligible collateral in another one; here there is no such path, so the trade is refused and the
session records `nothingSettled`.** That is one missing mechanism, not a bug.

It is not the only cause. Seven of the nine books never CROSS at all — option and commodity future
sessions return `noDemand` every time, CDS `noSupply`, and the IRS books are never run. For options
the chain is: `mine = expected × confidence`, `confidence = width(surprises)`, and `width` is zero
for a party with one surprise or none. 1,207 parties do hold price outlooks on share lines and
10,411 such outlooks exist — so the narrowing is not the gate — but of 101,998 outlook rows only
12,067 have more than one surprise. **Whether that is a defect or simply a world three periods old is
being measured over twelve periods now; the answer belongs in this file before anything is concluded.**
