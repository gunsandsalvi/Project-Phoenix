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

**Answered.** Twelve periods, the same world: confidence does not exist until period 3, and then it
arrives in bulk.

| period | outlook rows | rows with confidence | contracts | loans |
|---|---|---|---|---|
| 1 | 30,650 | 0 | 0 | 6 |
| 2 | 64,803 | 0 | 0 | 6 |
| 3 | 101,998 | 12,067 | 0 | 6 |
| 4 | 126,191 | 21,223 | 0 | 8 |

So confidence is not the gate and never was: a fifth of all outlooks carry it by period four and the
share is rising, and still not one contract exists. The single measured cause stands — no party can
post margin in a currency it does not hold — and the never-crossing books are downstream of it, not
a second cause. `width(surprises)` is doing exactly what a world two periods old should do.

---

## Third sweep: reading the codebase line by line

Method unchanged from the second sweep, but applied to the SOURCE rather than to the claims: read
each file in dependency order, and ask of every line (1) does the mechanism its `@spec` names exist,
(2) is it REACHABLE, (3) is it INERT, (4) does it break a law the lint cannot see. Stopped partway
by a finding that outranks the rest of the read.

### The finding that stops the sweep: 9,006 firms, 96 loans

The twelve-period run was meant to settle whether confidence was the gate on the derivative books.
It settled that, and it produced a worse number on the way:

| period | loans in the world |
|---|---|
| 1 | 6 |
| 2 | 6 |
| 3 | 6 |
| 4 | 8 |
| 5 | 46 |
| 6 | 96 |

I reported this growth as reassurance — "the world is young, lending is slow" — and that reading is
wrong. **This world has 9,006 firms and thousands of banks.** In the limit of any approximation of
the real world, a firm sector that size carries thousands of credit rows from the first period: a
revolving facility, a term loan against plant, trade finance against a shipment. Ninety-six rows
across six periods is not a young loan book. It is a lending mechanism that essentially never fires,
and the growth curve is the handful of borrowers who eventually stumble into the one path that works.

It sits beside the other two of its kind, and the three are now one shape rather than three
findings: **nothing is ever built (0 plant delivered), nobody ever reports (0 accounts published),
and almost nobody ever borrows.** Those are the three things a firm sector does. The measured cause
of the dead derivative sector — no party can post margin in a currency it does not hold — does not
explain any of them, so there is at least one more missing mechanism of the same size, on the
borrower's side of the credit decision. It was not diagnosed: the diagnosis is the next piece of
work, not this sweep's.

### What was read this sweep, and found sound

| area | lines | verdict |
|---|---|---|
| `rng/` (prng, spread) | 157 | OK — sfc32 keyed by (seed, label); `drawSize` cannot divide by zero because `next()` is [0,1) |
| `registry/grades.ts` | 49 | OK — one ordinal scale, `middleGrade` returns undefined rather than the worst for an unrated name |
| `registry/naming.ts` | 50 | OK |
| `registry/grid.ts` | 78 | OK — the tick is declared a TECHNOLOGY, with the measurement that says why it is not a resolution |
| `registry/credit.ts` | 85 | OK — `creditorOf` reads the register; two holders of one loan throws |
| `registry/environment.ts` | 89 | **DEFECT (F1)** |
| `registry/profiles.ts` | 120 | OK |
| `registry/claims.ts` | 191 | OK — one writer of "what this coupon comes to", memoised per (terms, calendar) |
| `registry/derivatives.ts` | 238 | OK — `cashDue` is implemented by commodity-futures and bond-futures and read by world and bank treasury |
| `registry/params.ts` | 335 | OK — every read is dimension-checked; there is no unchecked `get` |
| `registry/kinds.ts` | 369 | OK |
| `registry/registry.ts` | 529 | OK but **THIN (F2)** — fourteen assembly guards, three lying signatures |
| `registry/physical.ts` | 647 | **DEFECT (F3)** |
| `registry/geography.ts` | 723 | OK — `geographyFaults` is a serious validator: mix sums to one to its own dust, no region on the frame, no place in two pieces |
| `register/contracts.ts` | 234 | OK — indexed by pair because netting across counterparties is forbidden, and never collapses |
| `register/instruments.ts` | 311 | OK — every mutator bumps `version` and drops the memo, including `restate` |
| `register/register.ts` | 876 | OK but **THIN (F5)** — `deliverable` is exact integer comparison with no tolerance at all |

Also checked and cleared: every caller of `instruments.issuedBy` that does not filter `status.live`
(banks, estate, firms/invest, the accounts family). A ceased line has `issued === 0` and no holders,
so each of those sums zero. Not a defect.

### Findings

**F1 `registry/environment.ts:conditionsFor` — a guard that was claimed and does not exist.**
The comment says: *"A line naming a fact this world does not have stands in an ordinary period for
it. The world that has the fact is where the check belongs, and assembly is where it fires."*
`world/assemble.ts` checks module ids, dependency cycles and money issuance, and nothing else; `grep
exposedTo world/` returns nothing. So a recipe naming a fact no environment module declares silently
produces standing 1 — an ordinary period — forever. Today no recipe does: every `exposedTo` names
`growing` or `warmth`, and both are declared. The number is right; the guard is a lie. One fix, and
it deletes the paragraph: check it at assembly.

**F2 `registry/registry.ts:payable / cashFor / deliverable` — a parameter that is a lie.**
All three take a currency or a unit as their first argument and ignore it (`_ccy`, `_unit`); the body
is `downTick(amount)` or `toTick(value)`. Every quantity in the engine is already a count of pieces,
so no conversion is possible or wanted — but the signature says otherwise, and it invites exactly one
mistake: handing it a NAMED amount (188.49 USD) where it wants pieces (18849). I made that mistake
in item 13j and every opening balance in the world came out a hundred times too small. No number is
wrong today. The signature is.

**F3 `registry/physical.ts`, above `STORAGE_SESSION` — a comment attached to nothing.**
A full doc block — *"A4, A4.b, C3: the two numbers a kind of capital states about itself, named..."*
— sits immediately above a SECOND doc block, the one that documents `STORAGE_SESSION`. The
declaration it described is gone; the comment stayed. A stale comment is a defect.

**F4 `seeds/foundation.ts:1102` — a number read off the declaration, not the register.**
`kind.usefulLifePeriods` is read straight off `CapitalKindDecl`, while
`mechanisms/capital-programme/index.ts:90` declares that same number as `lifeParam(kind)` in the
parameter register. Two readable homes for one fact. `mechanisms/firms/produce.ts:349` is the
pattern that is right: it touches the decl only to ask whether the fact EXISTS (null or not) and
reads the value through `params.ratio(...)`.

**F5 `register/register.ts:sameState` — the merge guard does not compare what it guards.**
`forget(party, into)` is the door that deletes a cell's whole register position, and its own comment
says what makes that legitimate: the two cells' per-member state must be IDENTICAL. `sameState`
compares the lots exactly (qty, basis, acquisition period) and the equity — and compares the liens
only by COUNT. Two cells with the same lots and one lien each, of different sizes or to different
beneficiaries, are judged identical. A household cell does carry liens:
`mechanisms/housing/index.ts:555` pledges a cell's dwellings to its mortgage lender, and the size is
`outstanding / price`, which differs between two cells whose mortgages differ. What follows is the
thing `forget`'s own docstring forbids — units and a beneficiary's security deleted with no
instruction and no counterparty (Law 5, Appendix B).
Reachability is not established: `mergeCells` requires the same cell key, `bank` is in this world's
key (`['region', 'cohort', 'bank']`), so two mergeable cells share a lender, and their differing
mortgage balances would usually make their EQUITY differ, which `sameState` does compare. No module
calls `cells.merge` today — the callers are `split`, `reKey`, `die` and `weight` — so the door is
currently unreachable from the world. It is a guard that does not guard, not a live corruption.

**F6 `world/cells.ts:mergeCells` — the weight moves before the guard fires.**
`d.parties.applyWeight(...)` runs first; `d.register.forget(b, a)` — which is where the state
comparison happens — runs second. A merge that the register refuses has already written the
absorbing cell's new weight. The throw stops the world, so nothing persists, but the order is
backwards: the guard belongs before the write.

### Where the read stands

| verified line by line | remaining |
|---|---|
| `core` 992, `calendar` 334, `rng` 157, `registry` 3,503, `register` 1,421 of 1,604 | `register/voyages.ts` 183, `ledger` 1,977, `parties` 346, `prices` 1,046, `clearing` 1,250, `journal` 184, `audit` 1,504, `world` 4,245, `seeds` 3,383, `observer` 1,183, **`mechanisms` 38,129 across 125 files** |

6,407 of roughly 60,000 lines. The kernel's data and ownership layers are sound; nothing read so far
invents a number, bounds one, or writes a fact twice. The three defects found are a guard that was
claimed and never written, a comment describing a declaration that no longer exists, and a
comparison that omits the field it exists to compare.
