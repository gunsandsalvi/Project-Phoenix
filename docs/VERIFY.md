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
