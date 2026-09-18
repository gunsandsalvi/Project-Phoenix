# Requirement verification — findings

A read of `docs/spec/PROJECT_PHOENIX.md` against the source, clause by clause, against
`docs/COVERAGE.md`'s claim for each. One entry per finding: what the claim is, and what is
actually there. No corrections are made here; each entry is a statement of the problem.

Method: for each row in `docs/COVERAGE.md`, the cited code is opened and read. A row is a finding
when (a) the citation does not resolve, (b) the code it resolves to does not implement the clause,
or (c) the code implements it in a way a law forbids.

---

## What the pass found, in one page

`docs/COVERAGE.md` before: **1,090 MET, 130 PARTIAL, 176 MISSING, 2 OUT OF SCOPE.**
After reading every row against the source: **223 MET, 126 PARTIAL, 1,049 MISSING.**

The rows did not change because the standard moved. They changed because the evidence for 1,121 of
them was a path that is not in the repository, and because reading the code that IS there shows
most of the mechanism is written, tested, and never reached.

| #   | finding                                                                                                                                                 | where                    |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| 1   | `packages/engine` does not exist; 1,121 of 1,398 rows cited only paths inside it                                                                        | 0.1                      |
| 2   | 539 of 625 public items in `src/mechanisms/` are reached only by their own tests; **22 modules have not one item reached**                              | 11.1                     |
| 3   | 33 of 41 mechanisms propose no instruction. The world has **ten** places outside a book where anything happens                                          | 9.1                      |
| 4   | 7 of 10 audit families print `not-built` every period                                                                                                   | 1.4, 3.1                 |
| 5   | The one Ownership check that reports a number compares a row's lots with a quantity computed from those lots — it cannot fail                           | 1.1                      |
| 6   | Nothing sums holdings against an issued amount, and **there is no issued amount**                                                                       | 1.3, 7.1                 |
| 7   | There is no seed (§5), no central bank (§31), no policy rate, no money market mechanism (§11), no revaluation, no successor, no estate claim ever filed | 2.1, 14.1, 6.3, 7.2, 4.1 |
| 8   | The kernel's balance-sheet read values every holding **at cost**, and it is what decides who dies                                                       | 6.1                      |
| 9   | Every participant's reservation is `last print × a stated constant` — XI-13's fixed point                                                               | 13.2                     |
| 10  | Outlooks are formed correctly and no participant reads one                                                                                              | 13.3                     |
| 11  | `dealer.width` is a stated spread applied to a mid — §26 C5.a and C5.b, verbatim                                                                        | 13.1                     |
| 12  | `Servicing` pays a whole coupon to the **first** holder it finds                                                                                        | 15.1                     |
| 13  | A rating's PD and LGD are written as literal `0.0`; an ungraded name is given a grade                                                                   | 15.2, 15.3               |
| 14  | Settlement converts between currencies at par with no counterparty (latent: the world has one currency)                                                 | 8.1, 8.4                 |
| 15  | Two `settles_in`, two `basis`, two `parity`, each documented as the only one                                                                            | 9.3, 10.2                |
| 16  | `Leg::Mint` is unchecked: any party can create any money, including a negative amount                                                                   | 5.1                      |
| 17  | `irs` runs the same mechanism as `lending`; 21 systems run code from outside their own module                                                           | 12.1, 12.2               |
| 18  | A price is declared a POLICY primitive owned by the Parliament (§47 D3.a)                                                                               | 13.4                     |
| 19  | The guard for eight FORBIDs, `tools/check-forbids.ts`, is gone and the rows still cite it                                                               | 4.3                      |
| 20  | One row is malformed and has never been counted by any tool                                                                                             | 0.5                      |

**What is genuinely well built**, and reads as such against the spec: the wire
(`ledger.rs` — atomic instructions, declared delivery-versus-payment, a fail as a named state, the
payment queue and the gridlock ring); the register's lots, liens and both-direction indices; the
clearing solver and the three protocols (`clearing.rs`, `protocols.rs`, `session.rs`); the
calendar; `Making` and `recipe`; `Wages` and the cell split; `PlantMoves`, which is the one audit
family that reads two independent things; and the parameter register's shape.

---

## Part 0 — the systemic finding, which decides most of the rest

### 0.1 `packages/engine` does not exist. 1,121 of 1,398 rows cite only paths inside it.

The engine is Rust (`packages/kernel-rs`); `package.json` says so in its own description, and the
TypeScript workspace it names — `packages/engine`, `packages/app` — is gone from the tree. Every
`MET at packages/engine/src/...` row is a claim about a file that is not in the repository.

Counted from `docs/COVERAGE.md` as it stands: 1,398 rows; 1,151 cite at least one path; **1,121
cite paths of which NONE exists**; 11 more cite a mix of live and dead paths; 218 distinct dead
paths against 20 live ones.

This is not a cosmetic staleness. `MET` is defined at the top of the file as "the cited module
implements the clause" — a claim whose only evidence is the citation. Where the citation does not
resolve, the row asserts nothing that can be checked, and the `where / why` prose attached to it
(often several sentences of mechanism description) describes code that is not there.

`npm run check:existence` does not catch this: `tools/coverage-existence.ts` aggregates statuses
per spec system to name ABSENT SECTORS and never opens a cited path.

### 0.2 Two rows mark clauses the specification does not contain

`Money D5` and `XI-11` have rows in `docs/COVERAGE.md`. Neither is a clause. Money's section D
(`The wire`) runs D1, D1.a, D1.b, D2, D3, D4 and stops; there is no D5. `XI-11` is a Part XI
mechanism heading, not a numbered requirement, and it is the only mechanism given a row — the other
sixteen are not, so the file is inconsistent with itself about what a row is for.

`Money D5` carries the longest `where` cell in the file (roughly 1,400 words on the payment queue
and the gridlock ring) against a clause number that does not exist, so none of it can be checked
against a requirement.

### 0.3 The file's own header is stale, and so is the tool that guards it

The header says "**Ninety-nine** of these rows cite a module that has never produced an outcome …
and **ninety-six** of them carry **UNMEASURED**". The file contains **97** `UNMEASURED` rows, not 96. Law 16: a comment describing something that no longer exists is a defect.

`tools/coverage-existence.ts` `marksOnNotes` documents "**Eight** of them carry a `MET` row anyway."
The real count is **35** rows marking a spec NOTE (a continuation sub-clause carrying no
REASON/VERIFY/FORBID word), all but two of them `MET`. The check reports them rather than failing,
so the number in the prose drifted by 27 without anything going red.

The header also describes the settling measurement as `npm run coverage:reached` and points at item
0d. There is no `coverage:reached` script in `package.json`; the scripts there are `check:laws`,
`check:tests`, `check:types`, `check:tools`, `check:existence`, `plan:*`, `spec:index`, `bench`,
`world:runs` and `format`. The measurement the header names as the thing that settles 97 rows
cannot be run.

### 0.4 `npm run check` cannot pass as written, so no commit gate stands behind these marks

`package.json` defines `check` as `check:laws && check:tests && check:types && check:tools &&
check:existence && plan:check`. `CLAUDE.md` instead documents `npm run check` as "opens + lint +
typecheck + tests + spec citations + plan progress" and tells the reader to run
`npm run check:opens` before any commit and `npm run check:spec` for citations. Neither
`check:opens` nor `check:spec` exists in `package.json`. The two documents disagree about what the
gate is, and the gate `CLAUDE.md` names is not there.

### 0.5 One row is malformed and has never been counted by anything

The row for `Capital Programme A3`, as this pass found it, had no closing table cell:

```
| `Capital Programme A3` | PARTIAL | … it is charged in one place of the two (item 22f).
```

Every other row ends `… |`. This one ends at the newline, so it is not a table row: it renders as
plain text and every reader that parses the file by row — `tools/spec-coverage.ts`, and therefore
`check:existence` and the aggregate in `docs/IMPLEMENTATION.md` Part 0 — skips it silently.
`npm run check:existence` counts a clause with no row as `MISSING`, so the system's tally has been
one row short and one `PARTIAL` light for as long as the row has been broken. It was also invisible
to this pass's own tooling until a cross-check against the spec index reported the clause as
unanswered, which is how it was found.

It is also the row whose own text says the clause is "**re-marked PARTIAL at 22**" — a re-mark that
no count has ever seen.

---

## Part 1 — the audit

### 1.1 `LotsAgainstQuantity` is a tautology: it compares the lots with a number computed from those same lots

`packages/kernel-rs/src/audit.rs:211` sums a row's lots into `summed`, then reads
`register.quantity(row)` into `held`, and reports a violation when they differ.

`packages/kernel-rs/src/register.rs:126` — `quantity()` for any row that is not `total_only` **is**
`self.lots[at..at + len].iter().map(|l| l.qty).sum()`. The same lots, the same slice, the same
order, the same f64 addition. `summed - held` is exactly `0.0` and the branch is unreachable.

Audit A1.a: "it is a read of **two independent things that must agree** — never a read of one
thing against itself, which always passes." Equity B4.a and Audit B5.a name the same shape.

The register's own comment records how this happened: the quantity used to be a maintained total
beside the lots, the family compared the two, and it found a real drift
(`27.143341836734685` against `27.143341836734628`). The fix deleted the total — correctly, under
Law 4 — but left the family that was reading it, pointed at a `quantity()` that now re-derives from
the lots. The deletion did not name the read that replaced it (Law 19), and what was a check is now
a green line.

Its own docstring is now false: "It reads the SOURCE — the lots — and never the total it is
checking, which is what makes it a check and not a restatement." There is no total; the comparison
is the restatement.

This is one of only two contributions to the `Ownership` family, and it is the one `docs/COVERAGE.md`
cites for `Register B2`.

### 1.2 The same tautology exists a second time, as a free function, with a stale comment

`packages/kernel-rs/src/register.rs:388` `lots_against_quantity` is a byte-for-byte reimplementation
of the loop in 1.1 — same sum, same `quantity()` read, same dust formula. Law 4: one fact, one
writer; this is the check written twice.

Its comment says it is "the one place the maintained total is checked against what it is a total
of". The maintained total was deleted at the same commit; the comment describes something that no
longer exists (Law 16).

It is never called by the audit. Its only callers are one of its own unit tests and
`src/bin/register_at_scale.rs`, so the benchmark asserts against a check the world does not run.

### 1.3 `Register B2` — "holdings sum to the issued amount, per instrument" — is not checked anywhere

The clause is the ownership invariant: per instrument, the sum of every holder's units equals the
issued amount, with B2.a naming both directions of failure and B2.b insisting the tolerance is
dust. `docs/COVERAGE.md` marks it `MET` and cites `register.rs quantity` and `audit.rs
LotsAgainstQuantity`.

Neither is that check. `quantity()` answers for one row. `LotsAgainstQuantity` compares one row
with itself (1.1). Nothing in `packages/kernel-rs/src` walks `of_instrument(i)`, sums the holders
and compares the result with the instrument's issued amount. `Instruments::owed_by` exists but no
audit family reads it.

The same gap swallows Bond `N8.a` ("units held sum to units issued, always"), Corporate Credit
`E2`, Equity `C1.a`, Insurers `E4` and Audit `B2`, all of which are the same identity and all of
which are marked `MET`.

### 1.4 Seven of the ten declared audit families are `NotBuilt` in the assembled world

`Family::ALL` declares ten. The whole repository contains four `Contribution` implementations:

| family    | contribution               | what it actually checks                                |
| --------- | -------------------------- | ------------------------------------------------------ |
| Money     | `ATotalCarriesNoLots`      | a money account carries no lots                        |
| Ownership | `LotsAgainstQuantity`      | nothing (1.1)                                          |
| Ownership | `NoCollateralCountedTwice` | `free(row) >= 0`                                       |
| Units     | `PlantMoves`               | legs against register, per capital line — a real check |

`assembly.rs:338` adds the first three; `systems.rs:1174` adds the fourth and calls it "the one
module family this world has". `Audit::over` then fills **Prices, Cross-Market, Accounts, Names,
Flows, Zero-Sum and Liveness** with `NotBuilt`.

The machinery for saying so is right — an unbuilt family reports unbuilt and never green, which is
Audit E2 honoured. The finding is what `docs/COVERAGE.md` does with it: `Audit B1` (money is
conserved), `B3` (prices exist and are cleared), `B4` (cross-market), `B5` (accounts balance),
`B6` (names resolve) and `B7` (flows are complete) are each marked `MET`, citing
`packages/engine/src/audit/families/*.ts` — files that are not in the repository, for families the
Rust audit declares as not built.

`Audit B1` in particular — "money is conserved: every payment has a payer and a payee, and the sum
over all accounts changes only by an act of a money issuer" — has no implementation at all. The one
Money-family contribution checks that money accounts carry no lots.

### 1.5 There is no seeded generator anywhere in the engine, so `Audit D3` and `Seed A5` cannot hold

Audit D3: "a **run is reproducible**: same seed, same periods, same violations." Seed A5: "it is
**reproducible from a seed value**, so any run can be re-run."

`packages/kernel-rs/src` contains no random source, no `Prng`, and nothing that takes a seed value.
The only generator in the repository is a four-line xorshift called `Draw`, and it is **redeclared
independently in seven binaries** — `audit_at_scale.rs:24`, `module_at_scale.rs:30`,
`period_at_scale.rs:38`, `register_at_scale.rs:23`, `session_at_scale.rs`, `wire_at_scale.rs`,
`world_at_scale.rs`, `world_runs.rs` — each with its own hard-coded constant (`0x9E37…`,
`0xCAFE_F00D…`, `0xB5AD_4ECE…`, `0x0F1E_2D3C…`). Law 4: one fact, eight writers.

`world_runs.rs:48` calls it "the same counter-based draw **the engine uses**". The engine uses none.

Runs are deterministic, but nothing accepts a seed, so "same seed, different run" is not a thing
this world can be asked to do.

---

## Part 2 — the seed (§5)

### 2.1 §5 is not built. The binary that stands in for it says so in its own header.

`packages/kernel-rs/src/bin/world_runs.rs:5`:

> **THE WORLD THIS BUILDS IS ARBITRARY AND IS DECLARED ARBITRARY.** Every number in it is drawn
> from a counter. It is NOT a seed and must never be read as one: nothing here is cleared, nothing
> is decided, and no quantity in it is an outcome of anything (5 E1). … the seeding replaces it
> when the seeding is built.

There is no `src/seeds/` in `packages/kernel-rs`, no `SeedContext`, and `module.rs`'s claim that a
module declares "its seed contribution" has no corresponding trait method — nothing in the module
contract carries one.

`docs/COVERAGE.md` marks **fifteen of the twenty Seed rows `MET`** — A1, A2, A3, A4, A5, B1, B2,
B4, C1, C2, C3, C4, D1, D4, E2 — every one of them citing `packages/engine/src/seeds/foundation.ts`
or `packages/engine/src/world/assemble.ts`. Both are gone, and the mechanism they described was not
ported.

The consequences run past §5, because the seed is what several other clauses are checked at:

- `Seed A2` — "it must **pass the audit at period zero**" — there is no period zero to audit.
- `Seed C1` — "every party's balance sheet **balances at period zero**" — no such state exists,
  and the Accounts family that would check it is NOT BUILT (1.4).
- `Seed D1`/`D3` — "the stocks must be consistent with the flows that will run", "period one should
  be **quiet**" — nothing states an opening stock, so there is nothing for period one to be quiet
  against.
- `Seed B1.a` and Part XII's Units family — "the sum of cell weights equals the population it
  stands for" — no population is stated anywhere for a weight sum to be checked against.

---

## Part 3 — what the assembled world actually does

`npm run world:runs` (4 periods, 10,318 parties, 16,750 instruments, 1,546 books). This is not a
mid-build measurement of the economy (Law 11) — it is the evidence for or against the `MET` claims,
which is what this pass is for.

```
period 1  50 phases ran · 5 books cleared · 2255 trades · 0 made
          queue: 15564 still waiting ·   85 went through ·     0 ran out of days
period 2  50 phases ran · 4 books cleared ·  498 trades · 5 made
          queue: 25602 still waiting ·  353 went through ·  1134 ran out of days
period 3  50 phases ran · 1 books cleared ·    1 trades · 5 made
          queue: 33229 still waiting ·  658 went through · 22211 ran out of days
period 4  50 phases ran · 1 books cleared ·    1 trades · 5 made
          queue: 38708 still waiting · 2378 went through · 29238 ran out of days
audit (every period): accounts not-built · cross-market not-built · flows not-built ·
          liveness not-built · money 0 · names not-built · ownership 0 · prices not-built ·
          units 0 · zero-sum not-built
```

### 3.1 The audit line confirms 1.4 on the live world

Seven of ten families print `not-built` in every period of the assembled world. The three that
report a number are `money`, `ownership` and `units`, and two of those three are the contributions
in 1.1 and 1.2. `docs/COVERAGE.md` marks the clauses behind five of the seven `MET`.

### 3.2 One book of 1,546 clears, and one trade happens, in each of periods 3 and 4

`5 books cleared · 2255 trades` in period 1 falls to `1 books cleared · 1 trades` by period 3 and
stays there. 1,546 books are declared and open. Every `VERIFY` in the document that reads a cleared
price — worse credit trades wider, junior wider than senior, the bid-offer is a consequence,
inclusion shows in the price, a seller with no buyer keeps its paper — has one print a period to
read, in one book, and therefore cannot be taken at all.

This is also what makes `Clearing E4` ("a market with no trades has no new print, and the stale mark
must be visibly stale") the load-bearing clause for almost every priced row in the file rather than
an edge case: it is the ordinary state of 1,545 of the 1,546 books.

### 3.3 The payment queue does not drain; by period 4 it is the world's main outcome

Waiting payments rise monotonically — 15,564 → 25,602 → 33,229 → 38,708 — and payments running out
of days rise from 0 to 29,238. `Queue::gave_up` records each of those as an arrear on the wire
(`Outcome::ShortOfMoney`).

The queue's own doc is explicit that this number "is NOT insolvency and must not be read as one",
which is right. What it is, is that the great majority of what the world proposes never settles. A
clause marked `MET` because a module proposes an instruction is met only if the instruction settles;
by period 4 the base rate for that is against it.

`0 of them in a ring` in all four periods: the gridlock-cycle pass (`Settlement::unwind`, 22d.2)
has found nothing to do in any period of any run. It is built, reached, and has never fired.

### 3.4 `CLAUDE.md`'s statement of the homeless nouns names three that are not the three

`CLAUDE.md` (the standing rules, always in context) says:

> **Three homeless today**, and they are not the three of a week ago … The three now declared were
> found by the re-read of item 21 and each names its item — `registry.indices` (21.116),
> `settlement.realised` (21.112), `reporting.accounts` (21.76). … `npm run world:runs` prints them.

`npm run world:runs` prints:

```
3 nouns with no kernel home:
  agreements.states (item 21.62)
  control.resistance (item 23.1)
  derivatives.collateral (item 23.1)
```

None of the three named in `CLAUDE.md` is among them. The count matches by coincidence; the list
does not overlap at all. The rule file names the very read that falsifies it.

---

## Part 4 — doors that exist and nobody opens

### 4.1 `ctx.is_owed` has no caller, so no claim is ever filed against any estate

`module.rs:654` is the only way a module can write a claim against a dead party. The kernel plumbs
it through on both sides — `assembly.rs:713` and `running.rs:4611` both do
`claims.against(on, holder, owed, ranks)` from `asked.claimed`. Nothing in
`packages/kernel-rs/src/mechanisms/` or `running.rs` calls it.

`Ranked` (`running.rs:904`), the estate waterfall, opens with:

```rust
let rows = ctx.claims().on_estate(estate);
if rows.is_empty() { continue; }
```

so it returns immediately for every dead party, every period. The waterfall in `estate.rs` is
tested in isolation and has never run on the world.

Its own docstring points at the wrong door: "a claimant gets in by being written one through
`ctx.claims()`". `ctx.claims()` returns `&Claims` — a read. The write is `ctx.is_owed`.

What this empties out, all of it marked `MET` or `PARTIAL` in `docs/COVERAGE.md`:
XI-8 end to end; Corporate Credit `G3`, `G5`, `G5.a`, `G6`; Firm Birth `D2`, `D2.a`, `D2.b`,
`D2.c`, `D3`, `D6`, `D6.a`; Bond `N13`, `N13.a`; Derivative `D10`, `D10.a`; Derivative Layer
`C4.c`, `F2`, `F3`; Trade Credit `D2`, `D2.a`; Banks Capital `E1`, `E3`; Money `E1` (the state's
preferential claim, `estate.rs:152 owed_to_the_state`, whose only caller is its own test).

### 4.2 Three more `ParticipantView` doors have no caller

`cash()`, `public_event(row)` and `versions()` (`module.rs`) are never called from anywhere outside
`module.rs` itself.

`public_event` is the one that matters: it is the filter that decides whether a participant may see
a journal row —

```rust
self.journal.is_public(row) || self.journal.subjects_of(row).contains(&self.who.0)
```

— which is Observer `A4` ("no observer sees another party's private state") expressed as a check.
No participant calls it, so nothing in the world is actually gated by it. `docs/COVERAGE.md` marks
`Observer A4` `MET`.

### 4.3 The guard that held eight FORBIDs, `tools/check-forbids.ts`, is gone — and those rows still cite it as the reason

Part II: "A **FORBID** that holds is as valuable as a mechanism that works, and it is easy to break
silently." Eight rows in `docs/COVERAGE.md` say, in their own words, that the reason the absence
holds is a static check rather than a test, because the absence "breaks in perfect silence"
otherwise:

| row                    | what the guard was holding                                                         |
| ---------------------- | ---------------------------------------------------------------------------------- |
| `Corporate Credit D8`  | no yield, spread or model value ever written into the price store                  |
| `CDS B5`               | a speculative participant on both sides of every derivative book                   |
| `Prime Brokerage C3.b` | `atMost`, `atLeast`, `Math.min` refused in the margin call path                    |
| `Private Equity E3`    | no exit at a price nobody paid                                                     |
| `Reporting C6`         | no read of a print, a mark or the price store in the research module               |
| `Reporting E2`         | nothing outside research and the observer calls `consensusOf`                      |
| `Reporting F2.a`       | no parameter anywhere whose unit is a price move                                   |
| `Observer A4`          | the PEEK ratchet — `ctx.participant(<another party>)` reads per file may only fall |

`tools/` contains `calibrate/`, `coverage-existence.ts`, `phoenix-check/`, `plan-gaps.ts`,
`plan-progress.ts`, `plan-progress.test.ts`, `spec-coverage.ts`, `spec-index.ts` and config. There
is no `check-forbids.ts`.

`tools/phoenix-check` does check Law 6 bounds, Law 15 kind-branches and module imports, Law 7 bands
and undeclared numbers, across the Rust tree — so some of this ground is covered. None of the eight
specific absences above is among what it checks: there is no rule about the price store, about
`consensusOf`, about a price-move unit, or about cross-party participant reads.

Each of these eight rows is `MET` on the strength of a file that is not in the repository, guarding
source that is also not in the repository.

---

## Part 5 — money and settlement (§1)

### 5.1 `Leg::Mint` is not checked at all. Any party can create any money, in any amount, including a negative one.

`Leg::Mint { issuer, ccy, money, amount }` is the leg whose own doc says it exists because
"**Money A1: an issuer creating its own money**".

The pre-check skips it outright (`ledger.rs:960`):

```rust
Leg::Create { .. } | Leg::Mint { .. } | Leg::Pledge { .. } => {}
```

and the application is one line (`ledger.rs:1015`):

```rust
Leg::Mint { issuer, money, amount, .. } => {
    reg.money_delta(issuer, money, amount);
}
```

Nothing compares `issuer` with `instruments.issuer_of(money)`. A module may propose a mint of the
central bank's reserves onto a household's account, or of a rival bank's deposits onto its own, and
the wire will apply it. Nothing checks the sign either: a negative `amount` destroys somebody else's
money, and `money_delta` has no sign check of its own (`register.rs:313` is `self.total[row] +=
delta`).

This is Appendix B #1 ("no money without an issuer") and Money `A1.d`, `C4` and `C4.c` at the single
site in the engine that creates money. `Money C2.c` names issuance as the _one_ exception to the
legs-sum-to-zero check "because a creator of money has no debit anywhere" — and then `C4.c` says
"counts exactly those legs, so the exception is the thing being measured rather than a hole in the
check." Nothing counts them; the Money family is `ATotalCarriesNoLots` (1.4). So the exception is
open and unmeasured at both ends.

`Leg::Create` is unchecked in the same line, so units of any instrument can be created on any
party's book by any module.

### 5.2 A leg's `ccy` is a second copy of the instrument's currency, never checked against it, and one mechanism reads it

`Leg::Money` carries both `ccy: CurrencyCode` and `instrument: InstrumentId`. The instrument already
answers the question — `instruments.ccy_of(instrument)`. Settlement destructures the leg as
`Leg::Money { from, to, instrument, amount, .. }` in both the pre-check and the application, so the
`ccy` field is never read there and never compared with the instrument's own.

It is not dead, though. `running.rs:4370` (`CrossBorder`) reads it:

```rust
let Leg::Money { from, to, ccy, amount, receipt, .. } = *leg else { continue };
...
flows.push(Flow { ..., invoiced_in: ccy, entry });
```

So a region's current and financial accounts (§43 D1–D3) are built from the copy, and the copy is
the one nobody validates. Law 4's named anti-pattern exactly: "two disconnected representations of
one real thing … one is read and one is not", with the read one being the unchecked side.

`Leg::Mint` carries the same unread `ccy`.

### 5.3 `Money A2.b` — "two currencies are never added" — has no enforcement in the Rust engine

The row's `where` describes `packages/engine/src/core/measure.ts`: a `Cash` type that "carries its
currency and `plus`/`minus`/`sumCash` throw where two currencies meet — refused at the site,
everywhere, not at a boundary".

`packages/kernel-rs` has no such type. Money amounts are bare `f64` everywhere — in `Leg::Money`,
in `Register::money_delta`, in every mechanism. `num.rs` contains free functions over `f64` and
nothing that carries a unit or a currency. Adding two currencies is an ordinary `+` and nothing
anywhere can refuse it.

The same goes for `Money F1.a` (a per-book statement is never a sum across currencies) and
Appendix A's "money **in each currency separately**".

### 5.4 `Money A2`'s row answers a different clause

`Money A2` is "money is **denominated**: a unit is a unit _of_ a currency." The row's `where` is
about indivisibility and price ticks — `core/tick.ts`, "money is a COUNT of indivisible pieces …
and a PRICE has a smallest increment", `downToTick`, `upToTick`, `registry/grid.ts`. That is a
different requirement (Law 8's unit, and arithmetic impossibility under Law 6); it is not
denomination.

What does answer A2 is `Instruments.ccy` and `ccy_of`, which the row does not cite. And the tick
machinery it does cite has no counterpart in `packages/kernel-rs` — there is no `tick.rs`, no
grid, and no rounding-to-a-grain anywhere; quantities and prices are unconstrained `f64`.

---

## Part 6 — value (XI-6, Register D3, Clearing D4)

### 6.1 The kernel's own balance-sheet read values everything at cost, so no price move ever reaches anyone's solvency

`instruments.rs:232` `equity(party, register, instruments, claims)` is the read the world uses for
"what is this party worth". Its asset side is:

```rust
.map(|row| at_cost(register, HoldingId(*row))).sum::<f64>() + claims.owed_to(party)
```

and `at_cost` (`instruments.rs:261`) is

```rust
register.lots(row).iter().map(|l| l.qty * l.basis_per_unit).sum()
```

— the lot basis. Its liability side reads `held_total(i)` at par. **`Prints` is never consulted.**

XI-6: "Value is `units × price(asset)`, computed at read." Register D3: "quantity times a price that
came from a market, **never a price stored on the holding**." The basis is a price stored on the
holding, and this is the read that uses it.

`equity()`'s callers are what makes it load-bearing rather than cosmetic:

| caller            | what it decides                                    |
| ----------------- | -------------------------------------------------- |
| `running.rs:359`  | whether a party is insolvent and **ceases** (XI-3) |
| `running.rs:1096` | a firm's published result                          |
| `running.rs:1297` | a lender's view of a name                          |
| `running.rs:1420` | the size of a holder                               |
| `running.rs:4291` | a participant's capacity                           |

So a bond that halves in price leaves its holder's equity untouched, and the holder cannot be made
insolvent by a market. XI-6 says it plainly: "**It must not be output-identical.** The moment value
becomes units times a cleared price, **every balance sheet moves** … Capital moves, ratios move, net
asset values move, and identities that have been quietly comparing a cost to a mark start failing.
**That failure is the finding.**" That change has not happened in `equity()`.

`docs/COVERAGE.md` marks `Register D3`, `Clearing D4`, `Corporate Credit E3`, `Corporate Credit E4`,
`Equity C3`, `Equity C4`, `Hedge Funds A5` and `Audit B5` `MET`.

### 6.2 Where a mark _is_ taken, it is written out inline three times, each with the same silent fallback to cost

`running.rs:3062`, `running.rs:3221` and `running.rs:3484` each contain, character for character:

```rust
at_market += match ctx.prints().latest(line, ctx.period()) {
    Some(print) => units * print.price,
    None => ctx.register().lots(row).iter().map(|l| l.qty * l.basis_per_unit).sum(),
};
```

for the fund's NAV, the prime broker's client assets, and a third book. Law 4: one fact, three
writers. XI-6's whole point is that value is **a function**, and there is no such function in the
kernel — `prices.rs` exposes `Prints::latest` and nothing that turns a holding into a value.

The `None` arm is the second half of the defect. XI-6: "An asset genuinely not traded is **carried
at cost**, and _carried at cost_ is a **declared property of the asset**, not an accident of nobody
having written it a market." `Instruments` has no such declaration, so every unpriced line silently
takes the cost arm. Per Part 3, in periods 3 and 4 exactly one book of 1,546 printed — so in the
assembled world this arm is not the exception, it is what the valuation is.

### 6.3 There is no revaluation anywhere in the kernel

`REVALUATION` exists as a phase anchor (`world.rs:42`) and twelve systems anchor work to it, but
nothing in `packages/kernel-rs` re-marks a holding, books a revaluation gain or loss, or moves
equity because a price moved. `spot_fx.rs:165 revalued()` and `insurers.rs` compute a revalued
number as pure functions; neither has a caller that writes the result to a book.

That leaves unmet, all of them marked `MET` and citing the deleted
`packages/engine/src/world/revalue.ts` (24 rows cite that one file): Clearing `D4`; Currency `D2`,
`D2.a`, `D2.b`, `D3`, `D4`; Corporate Credit `E4`, `E4.a`; Equity `C4`; Households `D4`; Central
Bank `A2.c`, `F3`; Firm `C3`.

Currency `D2.b` is a FORBID — "**an unrevalued foreign position is money created or destroyed
silently**" — and it is exactly the state the engine is in.

---

## Part 7 — the register (§2)

### 7.1 There is no issued amount, so `Register B1` is unbuilt and `B2` is unwritable as stated

`Instruments` (`instruments.rs:52`) has columns for `issuer`, `ccy`, `class`, `unit`, `coupon`,
`matures`, `money_of` and `by_issuer`. There is **no `issued` column**.

`Register B1`: "an instrument has an **issued amount**, set when it was issued and changed only by
an issuance, a re-opening, a buyback, an amortisation or a maturity." Nothing stores it. Units come
into existence by `Brings.units` crediting the issuer's own row, or by `Leg::Create`, and after that
the only fact in the world is who holds what.

That makes `Register B2` ("holdings sum to the issued amount") not merely unchecked (1.3) but
unwritable: there is no second, independent number for the holdings to be summed against. Written
against `held_total(i)` it would be Audit A1.a's tautology again, since `held_total` **is** the sum
of the holdings (`register.rs:198`).

`Register B1` is marked `MET`, citing `settlement.ts` and `instruments.ts`.

### 7.2 A party that ceases keeps everything it held, for ever

`Parties::cease` (`parties.rs:213`) is:

```rust
pub fn cease(&mut self, p: PartyId) {
    self.alive[p.row()] = false;
}
```

Its own comment says "What it held is the estate's; this only records that the party has ceased."
Nothing else does the rest. There is no successor field, no `resolve`, no estate party created, and
no transfer of holdings — `parties.rs` has no `successor` anywhere in it. The estate waterfall never
runs because no claim is ever filed (4.1), so the dead party's cash and holdings stay on its own
rows.

That is XI-8 named exactly: "**A dead party with no claims still opens an estate rather than keeping
its cash for ever.**" And Appendix B #10: "No residual with no holder, in any currency, anywhere —
**including on a dead party**."

`docs/COVERAGE.md` marks `Register F2` `MET` and cites `packages/engine/src/world/succession.ts`
with four sentences about live agreement rows re-seating on a successor. There is no succession in
`packages/kernel-rs`.

### 7.3 Settlement never checks whether a party is alive, so `Money E4` does not hold

`Money E4`: "a party that **ceases to exist** mid-pass still has its legs settled or refused by
name — never dropped." The row says "a leg naming a ceased party throws at the site".

`grep alive packages/kernel-rs/src/ledger.rs` returns nothing. Settlement's pre-check tests
balances, free units and liens; it never asks `parties.alive`. A dead party pays and is paid like
any other, and given 7.2 it still has the money to do it with.

### 7.4 What the register does get right

Read against the source rather than the citation, these hold in `packages/kernel-rs`:

- `A1`, `A1.a`, `A1.c` — a row is (holder, instrument) with a quantity in the instrument's own unit.
- `A3`, `A4` — `Register::open` asserts both (`register.rs:196`), at the site.
- `C1`, `C2`, `C3`, `C3.a` — `Leg::Asset` has two named sides; `Instruction` carries a `Cause`;
  `Settlement::attempt` pre-checks every leg before applying any, so delivery-versus-payment is
  structural rather than hoped for. This is the strongest part of the kernel.
- `C3.b` — `Outcome` has six named states and a fail is recorded on the wire.
- `C4` — `ShortOfUnits` refuses a delivery the holder cannot make.
- `D1`, `D2`, `D2.a` — `of_holder` and `of_instrument` are both indexed.
- `D4` — lots carry `basis_per_unit`, drawn in order by `debit`.
- `D5`, `D5.a` — liens, and `free(row) < qty` refuses the move.
- `E5` — `Delivery::Free` is a real implementation of "moves money, or explicitly says why not":
  the writer has to declare it and `Instruction::shape()` holds the declaration to the legs.
- `F1` — `InstrumentId` is the row and never changes.

---

## Part 8 — currency at the wire (§6, §12, XI-12)

### 8.1 Settlement converts between currencies at par, silently, with no counterparty and no rate

`ledger.rs across()` decides where a payment lands when the payee banks elsewhere. It compares
banks and **never compares currencies** — `grep ccy` over the whole function returns nothing:

```rust
let payers_bank = instruments.issuer_of(money);
let payees_bank = parties.bank_of(to);
if !payees_bank.some() || payees_bank == payers_bank || to == payers_bank { return None; }
let payees_money = account_of(parties, instruments, to)…;
let reserves     = account_of(parties, instruments, payees_bank)…;
```

and the application (`ledger.rs:990`) is:

```rust
reg.money_delta(from, instrument, -amount);          // the payer's own money
reg.money_delta(to, payees_money, amount);           // the payee's bank's money
reg.money_delta(payers_bank, reserves, -amount);
reg.money_delta(payees_bank, reserves,  amount);
```

`amount` is carried across unchanged. If `instrument` is euro deposits and `payees_money` is dollar
deposits, `amount` euros leave and `amount` dollars arrive. The exchange rate is 1, always, and it
is never read from anywhere — `Prints` is not consulted and no FX counterparty exists.

This is:

- Currency `B3` FORBID — "**no conversion at the ledger boundary.** A payment in one money lands as
  that money; the decision to convert is a **separate, explicit trade with a counterparty and a
  rate**. Converting on arrival makes the currency market invisible and unmeasurable."
- Spot FX `E1` FORBID — "no conversion without a counterparty."
- Spot FX `E4` VERIFY — "**no leg landed converted**."
- Money `A2.b`, `C3` — a cross-currency payment is "two amounts and a rate", never one number.
- XI-12's "one convention for what a payment settles in".

All of them are marked `MET` or cite modules that are gone.

**What this world actually hits.** `world_runs.rs` issues every money as `CurrencyCode::at(0)`,
so the assembled world has one currency and no payment ever crosses two. The conversion above is
therefore a latent defect rather than one the run exercises — but it is unguarded at the wire, and
Currency `B3` is a requirement about the mechanism, not about whether a second currency happens to
have been drawn yet. Nothing in `across()` would refuse it.

The reserve legs have the same shape and a second problem: `reserves` is the money the **payee's**
bank banks in, and the payer's bank is debited in that same line. Across two regions the payer's
bank is made to hold reserves at the payee's central bank, which nothing established, and the two
reserve lines may themselves be different currencies.

### 8.2 The trade path reaches it on every cross-currency purchase

`session.rs:268` builds each trade as:

```rust
Leg::Money {
    from: buyer, to: seller,
    ccy: book.ccy,
    instrument: account_of(parties, instruments, buyer)…,   // the BUYER's own money
    amount: qty as f64 * at,                                // priced in book.ccy
    receipt: Receipt::Sale,
}
```

The leg says `book.ccy`, the money that moves is the buyer's own, and the amount is the book's
price times the quantity. Nothing reconciles the three. A foreign buyer would therefore pay the
seller's-currency amount out of its own-currency account, at one, and the seller be credited in its
own money by 8.1 — latent in the same way, for the same reason.

Goods `C6` says the price is in the seller's currency and "a foreign buyer converts, **by buying the
seller's money from somebody**". Nobody is bought from. §12 F1.a is explicit about what this costs:
"the buyer is never short, no order is ever placed — the currency demand the trade should have
created disappears."

### 8.3 A book prints even when every trade it produced failed to settle

`session.rs:256` writes the `Print` with `Provenance::Cleared` **before** any trade is attempted,
and the settle loop that follows only counts outcomes:

```rust
Outcome::Settled => settled += 1,
_ => failed += 1,
```

The comment defends it — "what cleared, cleared. Nothing is unwound and nothing is invented" —
and for a partial failure that is right. What it also means is that a book in which _no_ trade
settled still prints a `Cleared` price, and `Session.settled` can be 0 with a fresh print standing.

Clearing `E4`: "a market with **no trades** has **no new print**." Clearing `D5`: "bought equals
sold, and cash paid equals cash received, per clearing." With 38,708 payments queued and 29,238
out of days by period 4 (Part 3), the case is not hypothetical.

### 8.4 The assembled world has one currency, so §6, §12, §19 and §43 are untested by every run

`world_runs.rs` issues reserves and every bank's deposits as `CurrencyCode::at(0)`. There is one
money. Everything in Parts III §6, V §12, V §19 and X §43 — the currency position, the pair, the
rate, the forward, the basis, the cross-border payment — is unreachable in the world that is stepped
four periods to show "the machine runs in full".

It is also why `SpotFx` does nothing even on its own terms: `InstrumentId::at(ccy)` with `ccy = 0`
asks for the print of instrument row 0, which is the central bank's reserve line, which no book
prints. `SpotFx` returns before posting anything, every period.

---

## Part 9 — the shape of the whole implementation

### 9.1 Thirty-three of the forty-one mechanisms move nothing. The world has ten places outside the trading session where anything happens.

Every mechanism reaches the world through `MechanismContext::propose(legs, cause, delivery, why)`.
There are **ten** `ctx.propose(` call sites in `packages/kernel-rs/src`, and that is the whole of
what the world can do outside a book clearing:

| site              | what it moves                                            |
| ----------------- | -------------------------------------------------------- |
| `running.rs:177`  | what fell due on the schedule this period                |
| `running.rs:457`  | the week's wages on a standing engagement                |
| `running.rs:733`  | the batches that came off the line                       |
| `running.rs:755`  | the inputs the line drew                                 |
| `running.rs:867`  | a winding pool paying its holders pro rata               |
| `running.rs:962`  | an estate paying a ranked claimant (never reached — 4.1) |
| `running.rs:3140` | a subscription: cash to the pool, shares to the holder   |
| `running.rs:3695` | storage paid to whoever owns the storage                 |
| `running.rs:4492` | a private-equity capital call                            |
| `goods.rs:347`    | the share of stock that did not survive the period       |

(`assembly.rs:1356`/`1362` are a test fixture.)

Counting by mechanism: of the 41 `impl Mechanism for …` in `running.rs`, **33 propose nothing at
all.** They read the stores and write a line to the journal. The full list of the 33:

`Funding`, `Failing`, `Fixes`, `Forming`, `Reporting`, `Reads`, `Owed`, `Publishes`, `Grading`,
`ForcedSelling`, `Elections`, `Losses`, `TradeCredit`, `Floating`, `BankCapital`, `CostOfCapital`,
`BankFunding`, `Building`, `SecondOpinion`, `Observing`, `Protection`, `SpotFx`, `FxForwards`,
`Broking`, `StockLending`, `Levered`, `Sovereign`, `Housing`, `Securitising`, `Control`,
`Derivatives`, `SmallBusiness`, `CrossBorder`.

The eight that do propose are `Servicing`, `Wages`, `Making`, `Winding`, `Ranked`, `Subscribing`,
`Storing` and `Calling`.

This is the substance behind Part 3's numbers. A mechanism that says a rating changed, a margin was
called, a protection payment was owed, a bid was made or a rate cleared, **and proposes no
instruction**, has produced a journal entry and not an outcome. Law 5: "a one-sided flow is a defect
even when nothing fails" — here there is no flow at all.

Two examples, because the pattern is uniform:

**`SpotFx` (`running.rs:2819`)** reads who owes a foreign money and who holds one, builds the two
sides, calls `clearing(posted)`, gets a rate — and then:

```rust
ctx.say(self.kind, &[], &[(0, Value::Num(rate)), …], true);
let _ = ccy;
```

`let _ = ccy` is the tell: the currency the rate is about is discarded because nothing uses it. No
trade, no leg, no position. §12 `A1` ("an exchange of two amounts in two currencies, **both legs
settling**"), `D1`–`D5`, `E1`, `E4` are all marked `MET`.

**`Control` (§35, M&A)** calls `ctx.opens(…)` and `ctx.say(…)` and nothing else. No share changes
hands, nobody is paid. §35 `A2` ("the ownership of the target **transfers in the register**, and the
target's shareholders are **paid**"), `D5` ("the money paid to target shareholders equals what the
acquirer and its lenders put up, exactly, and it lands in named accounts") are marked `MET`.

### 9.2 `SpotFx` reads a price print keyed by a currency code cast to an instrument id

`running.rs:2875` and `:2881`:

```rust
let Some(rate) = ctx.prints().latest(InstrumentId::at(ccy), ctx.period()).map(|p| p.price) else { … };
```

`ccy` here is `crate::ids::CurrencyCode`'s inner `u32`, taken with `.0` a few lines earlier
(`ctx.instruments().ccy_of(line).0`). `InstrumentId::at(ccy)` then re-brands it as an instrument
row. The rate every participant's reservation is built from is therefore the last price print of
**instrument number N, where N is a currency code** — an unrelated row.

`ids.rs:8` states the invariant this defeats: "The brands are separate types for the reason the
TypeScript ones were: **passing a party where an instrument is wanted is a type error and not a
wrong answer**." Unwrapping to `u32` and re-wrapping is how that protection is lost, and it is done
in the one place the currency layer reads a price.

### 9.3 `settles_in` is written twice, in two modules, each claiming to be the only one — and the wire does neither

```rust
// mechanisms/currency.rs:146  "…and never on who the buyer is"
pub fn settles_in(sellers_money: CurrencyCode) -> CurrencyCode { sellers_money }

// mechanisms/spot_fx.rs:224   "One rule, owned in one place"
pub fn settles_in(sellers_money: CurrencyCode) -> CurrencyCode { sellers_money }
```

Appendix B #47: "one payment convention, owned in one place". There are two, each documented as the
one. Neither has a caller outside its own tests. And 8.2 shows the actual trade path settles out of
the **buyer's** account, which is the convention §12 F1.b forbids.

---

## Part 10 — the currency layer (§6, §12, §19, XI-12)

### 10.1 `mechanisms/currency.rs` is entirely dead code

Nothing outside the module's own tests refers to `Rates`, `Crossed`, `Rates::of`, `gap`,
`Arbitrageur`, `arbitrage`, `covered`, `basis`, `settles_in`, `short_of` or `MustBuy`. The module
that XI-12 is named for has no caller.

### 10.2 There are two cross-currency bases and two parity points, each documented as the only one

```rust
// mechanisms/currency.rs:130  "The parity point"
pub fn covered(spot, base_rate, quote_rate, year_fraction) -> f64 {
    spot * (1.0 + quote_rate * year_fraction) / (1.0 + base_rate * year_fraction)
}

// mechanisms/fx_forwards.rs:70  "where the forward would sit if the arbitrage were free"
pub fn parity(spot, base_funding, quote_funding, year_fraction) -> f64 {
    spot * (1.0 + quote_funding * year_fraction) / (1.0 + base_funding * year_fraction)
}
```

The same expression, twice, under two names. And on top of each:

```rust
// currency.rs:137   "**THE basis** — … One number, derived from a print (Law 19)."
(forward_cleared / parity - 1.0) / year_fraction

// fx_forwards.rs:77 "One basis, derived from the CLEARED forward against parity (Law 19, B3.b)."
(f.rate / at_parity - 1.0) / f.year_fraction
```

§19 `B3.b` is a FORBID and says it in three words: "**there is one basis.**" XI-12 spells out the
cost: "A cleared funding basis and a second basis … is the same defect as two index systems." There
are two, and each carries a comment asserting it is the one. There are also two `Arbitrageur`
structs, one per module.

`docs/COVERAGE.md` marks `FX Forwards B3.b` `MET`.

---

## Part 11 — how much of the modelled mechanism is unreachable

### 11.1 Five sixths of it. 539 of the 625 public items in `src/mechanisms/` are never reached from outside their own module.

Counted over `packages/kernel-rs/src/mechanisms/*.rs` with comments and `#[cfg(test)]` blocks
stripped: **625** `pub fn` / `pub struct` / `pub enum` / `pub trait` / `pub const` items. Counting
only references from another file through a qualified path (`mechanisms::<mod>::<Item>` or a
`use crate::mechanisms::<mod>::{…}` list), **86** are reached and **539** are not. Their only
callers are their own unit tests — which is why the 737-test suite is green (Part 0) while the
world does nothing (Part 3).

**Twenty-two modules have not one item reached:**

`cds`, `corporate_credit`, `currency`, `dealing`, `employment`, `equity`, `expectations`, `firms`,
`freight`, `hedge_funds`, `households`, `insurers`, `irs`, `lending`, `money`, `money_market`,
`mortality`, `observer`, `polity`, `private_equity`, `second_opinion`, `securitisation`.

The best-connected modules are `recipe` (6 of 12), `housing` (6 of 16), `cross_border` (6 of 14),
`control` (6 of 13) and `capital_programme` (6 of 9). Nothing reaches more than six.

Some examples of what "reached" amounts to where it is not zero:

- `goods`: 4 of 19 — `CostFlow`, `Lot`, `take`, `Perishing`. Not `carry` (lower of cost and net
  realisable value, §37 `E2`), not `charge` (absorption, `F5.a`), not `Consignment`/`landed_cost`
  (`D4`), not `clearing`, not `storage_fee`.
- `treasury`: 1 of 18 — `must_raise`.
- `sovereign`: 3 of 5 — `handle`, `Programme`, `Shortfall`; not `Auction`, not `Missed`.
- `benchmarks`: 1 of 13 — `fix`; not `Index`, not `Constituent`, not `Weighing`.

### 11.2 `mechanisms/money.rs` is dead, and with it the overdraft decision and `NoOverdraftForTheTreasury`

The module is a correct Law 15 dispatch table: an `Issuer` trait keyed by party kind, an `Issuers`
registry, `Overdraft::Lend { owes, per_annum } | Refuse`, and
`NoOverdraftForTheTreasury { treasury_kind }` answering `Refuse` for any shortfall however small.
Every one of its five public items — `Issuer`, `Issuers`, `Overdraft`, `NoOverdraftForTheTreasury`,
`as_legs` — is referenced nowhere else in the engine. `Issuers::ask` has no caller.

Settlement does not consult it. When a payer is short, `ledger.rs` queues the payment or records
`ShortOfMoney`, and no issuer is ever asked whether it would lend.

So:

- Money `B3.a` — "a **credit decision by its bank** — the bank lends only to the room its own
  capital supports, and **refuses past it**" — is never asked.
- Money `B3.b` — "a bank overdrawn at the central bank is **borrowing from the central bank**, and
  the corridor prices it" — cannot happen.
- §30 `D3` / §31 `E2` / XI-9 — "**there is no central-bank overdraft**" — holds, but by the
  treasury having no overdraft path at all rather than by `NoOverdraftForTheTreasury` refusing one.
  The guard written for the single most consequential FORBID in the document is not installed.

---

## Part 12 — which systems are actually wired to their own mechanism

`systems.rs` wires a system in one of two ways: `works("<name>", <anchor>, Box::new(<Mechanism>))`
gives it a phase, and `posts("<name>", <anchor>, Box::new(<Participant>))` gives it somebody who
posts orders into books. 51 names are wired — 42 by `works`, 9 by `posts`, 1 (`benchmarks`) by
another door — which is the "51 wired systems" `npm run world:runs` reports.

Crossing that against which `mechanisms::<module>` are imported by the two files that run the world
(`systems.rs`, `running.rs`): of the 51 wired names, **21 have a module that is never imported**.
The system is wired; the module written for it is not what runs.

### 12.1 Twenty-one systems run code that does not come from their own module

`cds`, `corporate_credit`, `currency`, `dealing`, `employment`, `equity`, `expectations`, `firms`,
`freight`, `hedge_funds`, `households`, `insurers`, `irs`, `lending`, `money`, `money_market`,
`mortality`, `observer`, `private_equity`, `second_opinion`, `securitisation`.

For the nine wired by `posts`, what runs is a small participant struct declared in `systems.rs`:

| system         | participant that runs | its own module                            |
| -------------- | --------------------- | ----------------------------------------- |
| `money_market` | `MoneyMarketBanks`    | `mechanisms::money_market` — not imported |
| `insurers`     | `InsurerMatching`     | `mechanisms::insurers` — not imported     |
| `households`   | `HouseholdBuyers`     | `mechanisms::households` — not imported   |
| `freight`      | `LetsItsPlant`        | `mechanisms::freight` — not imported      |
| `dealing`      | `Dealers`             | `mechanisms::dealing` — not imported      |
| `goods`        | `GoodsSellers`        | imported                                  |
| `treasury`     | `TreasuryIssues`      | imported (`must_raise` only)              |
| `funds`        | `FundMandates`        | imported                                  |
| `stockists`    | `Stockist`            | —                                         |

Each of those five participants posts one order from a stated parameter (13.2): `MoneyMarketBanks`
quotes `money_market.lends_at` and `money_market.borrows_at`, both declared 1.0 per annum;
`HouseholdBuyers` bids `last print × 1.2`; `InsurerMatching` bids `last print × 1.05`. None of them
reads the module written for its system, so §11's reserve position and corridor, §27's liability
schedule and discount rate, §41's cell-level consumption decision, §38's routes and capacity and
§26's inventory-driven quote are all present as modules and absent from the world.

### 12.2 Three mechanisms serve two systems each

- `Servicing` runs both **`lending`** and **`irs`**. §18 Interest-Rate Swaps is therefore the same
  generic "pay what fell due on the schedule" mechanism as a bank loan. There is no swap: no fixed
  leg, no floating leg, no fixing, no netting, no mark. §18 `A1.b` is "periodic exchange of fixed
  against floating on the notional; only the **net** moves", and `E1` is a FORBID — "**no notional
  exchange.** If the notional moves, it is a loan and it belongs on the balance sheet as one."
  What `irs` runs is a loan.
- `Funding` runs both **`short_term_debt`** and **`corporate_credit`**. §9 and §7 are one mechanism.
- `Owed` runs both **`money`** and **`currency`**, and proposes nothing.

`npm run world:runs` prints "0 of 51 wired systems only count", which reads as reassurance. What it
measures is whether a system's mechanism did _something_ in a period; it does not ask whether the
something was that system's.

### 12.3 What `check:existence` was built to catch, and why it does not catch this

`tools/coverage-existence.ts`'s own header says it exists because "a sector that was never written
leaves no trace in the source a read looks at, nor in the claims made about that source — only in
the aggregate nobody took". It then takes that aggregate **from `docs/COVERAGE.md`'s own status
column**. A system with 30 `MET` rows pointing at deleted files is not an ABSENT SECTOR by that
measure, however absent it is. The five systems in 12.1 all pass it.

---

## Part 13 — the parameter register, and what the participants actually post

`npm run world:runs` prints "**0 of 61 declared numbers are shapes**". Every one of the 61 is
declared `Technology` (22), `Preference` (30), `Policy` (6) or `Resolution` (3). None is a `Shape`
and none is a `Placeholder`.

CLAUDE.md makes the argument against reading that as good news, about the homeless-noun count:
"**A count of zero would be the measure switched off**: zero means _nothing anybody DECLARED is
homeless_, not _nothing is missing_." The same applies here, and several of the 61 are outcomes
wearing a primitive's label.

### 13.1 `dealer.width` is a stated spread applied to a mid — the exact shape §26 C5.a and C5.b forbid

`systems.rs:974`: `say("dealer.width", 0.02, "share of the mid", Dimension::Ratio, Kind::Preference, …)`

`systems.rs:402` (`Dealers`), whose `orders` reads:

```rust
let width  = view.params().ratio(self.width);   // dealer.width  = 0.02
let around = view.params().ratio(self.around);  // dealer.around = 1.0
let skew = width * (held / limit);
let bid = around - width - skew;
let ask = around + width - skew;
```

- §26 `C5.a` FORBID — "**no spread applied to a mid.** A mid with a spread bolted on is a single
  price pretending to be two, and it cannot skew, widen, or refuse."
- §26 `C5.b` FORBID — "**no stated spread table.** A width per book, per instrument or per client
  type is a price assigned, not a consequence: what a desk charges is what carrying that position
  costs it."
- Appendix B #18 — "No spread applied to a mid; no stated spread table."

The skew term is real and is the one part of §26 C2 that is present. The width itself is a declared
constant, which is what both FORBIDs are about. `docs/COVERAGE.md` marks `Dealer Desks C5.a` and
`C5.b` `MET`.

There is a second defect in the same five lines. `dealer.around` is declared with the unit
"**multiple of the last print**" and is used as a bare number: `around - width - skew`, never
`print.price * around`. With `around = 1.0` the desk quotes a bid of about 0.98 in absolute money,
whatever the line last traded at. Law 8: the unit is part of the number, and the declaration and
the use disagree. The comment at `systems.rs:193` records the identical bug being fixed for
`household.will_pay` — "it was being posted as though it were the price itself … invisible while
the declared value happened to be one" — and the same mistake is still standing here, invisible for
the same reason.

A third, in the line above: `if held.abs() >= limit` compares `held` (units of the instrument)
with `limit` (`dealer.limit`, declared `Denomination::Money`), and `whole_pieces(limit - held)`
subtracts units from money. Appendix A: "Every quantity carries its unit … The only route from a
quantity to a value is `quantity × price`."

### 13.2 Every participant's reservation is a stated multiple of the price the book last printed

| param                    | value | unit                       | who posts it         |
| ------------------------ | ----- | -------------------------- | -------------------- |
| `household.will_pay`     | 1.2   | multiple of the last print | every household cell |
| `fund.will_pay`          | 1.1   | multiple of the last print | every fund           |
| `insurer.will_pay`       | 1.05  | multiple of the last print | every insurer        |
| `dealer.around`          | 1.0   | multiple of the last print | every dealer         |
| `goods.seller.will_take` | 1.0   | multiple of what it cost   | every seller         |

`systems.rs:199`: `let limit = print.price * view.params().ratio(self.will_pay);`

XI-13, in full: "Any mechanism in which the _input_ to the participants' schedules is derived from
the same quantity the clearing is supposed to _discover_ produces a price that is a fixed point of
its own formula. **It will look like a market and it will carry no information.**"

Every bid in the world is `last print × k`, with `k` a constant per party kind. Clearing `A1.a`
("the differences **are** the market; identical participants have nothing to trade") is gone: within
a kind there are no differences at all. §46 `A3` ("expectations are heterogeneous, and the
heterogeneity is load-bearing") likewise.

This is the mechanical reason for Part 3's numbers — one book of 1,546 clearing, one trade a period.

### 13.3 Outlooks are formed correctly and nothing reads them

`Forming` (`running.rs:1010`) does what §46 B1 asks: for each party, the mean of the prints on the
lines it holds, corrected toward the last outlook at a memory rate, recorded as
`about::WHAT_IT_SELLS_FOR`; plus a second outlook of what it delivered, read off the wire. 1,151
outlooks in period 1 rising to 2,515 by period 3.

**No participant reads one.** `grep -n 'outlooks()' packages/kernel-rs/src/systems.rs` returns
nothing, so every order posted into every book comes from the stated multiples in 13.2 instead —
which is §46 `C3` exactly ("a participant's view in any book **is** its expectation of the price"),
and it is the clause XI-13 turns on.

Three _mechanisms_ do read one, and those are the part of §46 C that works:

| site                                    | reads                                    | for                                |
| --------------------------------------- | ---------------------------------------- | ---------------------------------- |
| `running.rs:571` (`Making`)             | `HOW_MUCH_IT_SELLS`                      | the production decision — §46 `C2` |
| `running.rs:2464`, `:2467` (`Building`) | `HOW_MUCH_IT_SELLS`, `WHAT_IT_SELLS_FOR` | the investment decision — §46 `C2` |
| `running.rs:3824` (`Housing`)           | `WHAT_IT_KEEPS_EARNING`                  | what a household can bid           |

So §46 `C2` holds. `C1` (consumption), `C3` (a view in a book), `C4` (a lender's view of a name, a
desk's view of a line), `C5` (the treasury and the central bank) and `C6` (the vote) do not.

**And one of the three reads an outlook nothing forms.** `about::` declares six kinds. The only
`ctx.form` call in the engine is in `Forming` (`running.rs:1074`), and it forms two:
`WHAT_IT_SELLS_FOR` and `HOW_MUCH_IT_SELLS`. `WHAT_IT_KEEPS_EARNING` is read by `Housing` and
written by nobody, so that read takes its `None` branch in every period of every run and a
household's bid is never governed by what it expects to earn. `WHAT_CREDIT_COSTS`,
`WHAT_A_HOUSE_IS_WORTH` and `WHETHER_IT_IS_PAID_BACK` are declared, never formed and never read.

Two further problems inside `Forming` itself:

- **One memory for the whole world.** `outlook.memory = 0.3`, one row in the register, read once at
  the top of `run` and applied to every party. §46 `B1.a`: "It is **dispersed across parties** …
  because a sector whose members all remembered the same way would move as one, and **drawn once at
  entry**." Nothing is drawn and nothing is dispersed.
- **The observation is a mean over unlike prices.** `seen += print.price; lines += 1.0; let now =
seen / lines;` adds the price of every line the party holds — a good, a share, a bond — and
  divides by the count. Law 8: the unit is part of the number.

### 13.4 A price and a share are declared as primitives, one of them owned by the parliament

- `treasury.will_accept = 0.98`, `Dimension::Ratio`, **`Kind::Policy`, `Owner::Parliament`**, unit
  "price per unit of par". §47 `D3.a` FORBID: "**the parliament never sets a price, a quantity or an
  outcome.** … A mandate that named an interest rate, a wage, an exchange rate or a growth target
  would be a written path with a majority behind it." This is a reserve price on a sovereign auction,
  owned by the parliament in the register.
- `funding.coupon = 0.04` and `paper.coupon = 0.03`, both `Kind::Technology`,
  `Owner::StandardSetter`, described as "the coupon the paper carries as a TERM … never what it is
  worth". Every bond `Funding` brings in the world carries the same coupon whoever issues it. Law 2:
  a coupon is the price of credit for one issuer at one time, which is an OUTCOME. Seed `C4.b`: "A
  seeded spread table that strikes every coupon in the world is a permanent cash flow, not an
  opening guess" — here it is not even a table, it is one number.
- `equity.shares = 1000.0`, `Kind::Technology`. §10 `A2.a`: a share count changes only by a named
  event; it is an outcome of issuance, not a technology.
- `capital.debt_share = 0.6`, `Kind::Preference`, "debt per unit raised". §7 `A2.c`: "the structure
  that results is an outcome of A2.b meeting the market's price, **never assigned**."
- `lending.will_lend = 0.3`, "per unit held". §23 `B1.c` FORBID: "a bank does not lend '**out of**'
  its deposits or its reserves. **A model in which it does cannot produce a credit cycle.**"
- `money_market.lends_at = 1.0` and `money_market.borrows_at = 1.0`, both `Preference`, both "per
  annum" — a 100%-per-annum lending rate and an identical borrowing rate. §11 B4 wants a rate that
  clears; these are posted, equal, and (per 12.1) belong to a system that does not run.

---

## Part 14 — the monetary layer

### 14.1 There is no central bank (§31) and no money market (§11). The policy rate does not exist anywhere in the engine.

`systems.rs` has no `works("central_bank", …)`. `kinds::CENTRAL_BANK` appears only in test fixtures.
Grepping the whole of `packages/kernel-rs/src` for `policy_rate`, `corridor`, `deposit_facility` or
`standing_facility` returns only comments — in `mortality.rs`, `benchmarks.rs`, `money.rs` and one
test name in `cost_of_capital.rs`. There is no administered rate, no deposit facility, no standing
lending facility, no reserve requirement, no open-market operation, no remittance and no revaluation
account.

§11 has a participant and no mechanism (12.1): `posts("money_market", …, MoneyMarketBanks)` quotes
two stated rates into an overnight book if one exists, and `mechanisms/money_market.rs` — the
module with the position, the corridor, the facility, the haircuts and the run — is imported by
nothing.

So the whole of Part III's monetary machinery is absent, and with it:

- §31 `B1`–`B4` (the policy rate and the corridor), `C1`–`C4` (open-market operations), `D1`–`D4`
  (lending to banks, the lender of last resort), `E1`–`E5` (the treasury relationship, remittance,
  a loss it cannot remit), `F1`–`F4` (reserves and intervention).
- §11 `A1`–`A3` (the reserve position as the residue of everyone else's period), `B1`–`B7` (the
  market and its two sides), `C1`–`C5` (the corridor), `D1`–`D6` (a name that cannot fund), `E1`–`E3`
  (transmission).
- XI-4's first joint — a bank's own blended cost of funds — which the mechanism says is what makes
  every financial price able to reach a real decision.
- Money `C4.b` (the central bank creating reserves), `A1.b` (a reserve as the central bank's
  liability to a named bank) in any acting sense.

Several FORBIDs in §31 and §11 now hold, and they hold by the system being absent rather than by a
mechanism refusing: `B3.a` (the policy rate is never a market's cleared rate), `C1.b` (not a buyer
of last resort in the primary market), `C5` (no uncollateralised unlimited central-bank credit),
`D3.a` (it does not lend to the insolvent), `E2` (no automatic overdraft). Part II is explicit that
this is not the same thing: "**A FORBID that holds is as valuable as a mechanism that works**" —
but a FORBID that holds because nothing exists to break it says nothing about the model.

`docs/COVERAGE.md` marked all 29 Central Bank rows and all 28 Money Market rows `MET` or
`PARTIAL`, none `MISSING`.

---

## Part 15 — defects inside the eight mechanisms that do move money

These are the ones that matter most, because they are the only places the world acts.

### 15.1 `Servicing` pays the whole coupon to one holder — the first row that is not the issuer

`running.rs:148` (`Servicing`) is the mechanism the entire credit side rests on, by its own
docstring. For each scheduled payment it does:

```rust
let holders = ctx.register().of_instrument(line);
let owed_to = holders
    .iter()
    .map(|r| ctx.register().holder_of(crate::ids::HoldingId(*r)))
    .find(|h| *h != owes);
```

`running.rs:162`'s `find` returns the **first** holder that is not the issuer, and the full amount is
proposed to that one party. There is no loop over holders and no division by units held. A line held by twenty
parties pays all of it to whichever row `of_instrument` returns first; the other nineteen get
nothing and are owed nothing afterwards.

- Register `E1` — "a coupon or dividend pays to the **holders** of record" — plural, and E3 makes
  the proportionality explicit for a loss.
- Register `A2.a` — "everything the instrument pays, it pays to whoever the register says holds it,
  then."
- Corporate Credit `F2`, Sovereign `F1` — "to whoever holds it on the date".
- Appendix B #10 — what the other holders were owed and did not get is a residual with no holder.

The comment directly above it states the principle it breaks: "Appendix B: no liability without a
beneficiary. **Whoever HOLDS the line is owed**, which the register says — never a second list of
who is owed what."

`Servicing` runs both `lending` and `irs` (12.2), so this is how every loan, bond, premium and rent
in the world is paid.

### 15.2 A rating's probability of default and loss given default are written as literal zeros

`running.rs` (`Grading`) writes each house's view of each name as:

```rust
// B1, B2: the probability of failing and, SEPARATELY, the loss given it. Both are the
// house's own view and both are stood behind with the grade.
ctx.now_stands(standing::GRADE, by, of, vec![grade.rank(), 0.0, 0.0]);
```

The comment names §44 `B1` and `B2` and says both are the house's own view. Both are `0.0`.
Appendix A: "A quantity that is absent is **absent**, never zero. … **Zero multiplies.**" A
probability of default of zero and a loss given default of zero are a stated view that this name
cannot fail and that nothing would be lost if it did, standing behind every grade in the world.

§44 `B2.a` ("an **instrument's** rating differs from its **issuer's**, and both must exist") has no
representation at all: a `standing::GRADE` row is about a party.

### 15.3 An ungraded name is given a grade

`running.rs:2141`:

```rust
let grade = Grade::at_rank(*worst.get(&issuer.0).unwrap_or(&Grade::Substantial.rank()))
    .unwrap_or(Grade::Substantial);
```

A name no house has graded is treated as graded `Substantial`, and that grade then sets the risk
weight on everyone's holding of its paper. The comment argues the bottom of the scale is what an
ungraded name weighs. §44 `A1` makes a rating "an ordinal judgement about a named issuer …
**published**" by "a **named assessor**" (`A5`); `E2` forbids a rating that changes for no reason,
and a default is a rating that appeared for no reason. Appendix A: absent is absent.

### 15.4 `recipe::picks` is a substitution mechanism, which §37 A2.a says must not exist

`mechanisms/recipe.rs:232` `picks(line, priced, wage, capital_service)` walks `line.ways`, costs
each at the prices this firm faces, and returns the cheapest. Its own comment states the behaviour
plainly: "a firm picks differently **when a price moves**, which is the whole reason a line has more
than one way of being made."

§37 `A2.a`:

> **fixed input quantities per unit of output — a Leontief recipe, no substitution.** Chosen
> deliberately … The cost is that a firm facing an expensive input cannot economise on it, so
> **substitution is a MISSING mechanism here, not an assumption away**: if a relative-price response
> is wanted later it is a **new mechanism**, not a parameter.

A discrete choice among recipes on relative input prices is a relative-price response. It may be
the right thing to build, but the specification requires it to arrive as a declared new mechanism
with its clause re-marked, and `docs/COVERAGE.md` records no such thing — `Goods A2.a` was marked
`MET` on the strength of there being no substitution.

It is unreached (11.1): `running.rs` imports `Line`, `Recipe` and `decide` from the module and not
`picks`. So what runs is the Leontief draw the clause asks for, and the substitution sits beside it
in the same file.

---

## Part 16 — what this pass leaves red

### 16.1 `npm run check:existence --verify` now fails, and that is the check doing its job

`tools/coverage-existence.ts --verify` compares the table it generates from `docs/COVERAGE.md`
against the one in `docs/IMPLEMENTATION.md` Part 0, "so the plan's own statement of what exists
cannot go stale in silence". Re-marking the rows changed the table, so the two now disagree and the
check is red. Its own message says the remedy: **re-generate Part 0 with `npm run check:existence`
in the same change.**

That has not been done here, because `docs/IMPLEMENTATION.md` was out of scope for this pass and
rewriting a section of a file one has not read is how a document goes stale in the first place. It
is one command, and it is the next thing to run.

What the comparison also shows is that Part 0 carried the same inflated picture COVERAGE did —
`Freight 17 MET, 3 PARTIAL, 0 MISSING`, `Capital Programme 20 MET, 0 MISSING`,
`Small-Business Pools 26 MET, 0 MISSING`, `Cross-Border 20 MET, 0 MISSING` — for systems that are
0, 5, 2 and 3 `MET` when read against the source.

### 16.2 `check:existence` now names two ABSENT SECTORS, and named none before

```
ABSENT SECTORS — no clause MET at all: 2
  Insurers                   0 of 23
  Freight                    0 of 20
```

This is the read the tool was built to produce — "a sector that was never written leaves no trace
in the source a read looks at, nor in the claims made about that source — only in the aggregate
nobody took". It reported zero absent sectors for as long as the claims about the source were
citations that did not resolve.

### 16.3 Two rows in this file mark a clause the specification does not contain, and 37 mark a NOTE

`check:existence` reports both, and reports rather than fails. The counts are now:

```
1361 clauses: 222 MET, 125 PARTIAL, 1014 MISSING
37 MET row(s) mark a spec NOTE rather than a requirement, so they are outside the count above
```

`Money D5` and `XI-11` remain in `docs/COVERAGE.md` as rows against non-clauses (0.2). They are
left standing rather than deleted, because Part II is explicit that a clause is never removed to
make a comparison look better — and the same courtesy is owed to a row that says something true
about the source even where the clause number is wrong.
