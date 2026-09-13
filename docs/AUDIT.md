# The audit

**This is the one place a finding lives.** `docs/BUGS.md`, `docs/SWEEP.md`, `docs/VERIFY.md` and
`docs/plan/14-polity.md` were merged into it and deleted; a finding leaves here only by being
PLACED — into the worklist item that fixes it, or as its own item at its dependency position — and
the entry says where it landed.

| part    | what it is                                                                                                                                                                   |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **I**   | A read of `packages/engine/src` — all 59,927 lines, file by file, in dependency order — against three questions. **A-1 … A-70.**                                             |
| **II**  | Every `done` row of `docs/WORKLIST.md` and every `MET` in `docs/COVERAGE.md`, against what is in the code — and the three project files that count the work. **B-1 … B-15.** |
| **III** | What was carried in from the three deleted holding pens, de-duplicated against Part I. **C-1 … C-7.**                                                                        |
| **IV**  | Item 14, the polity: the plan, and the two findings it carries. **D-1, D-2.**                                                                                                |

## Part I — the read

Three questions and nothing else:

1. **Is it BOTTOM UP?** Does the number come out of a mechanism, or was it written down?
   An outcome stated is the defect this whole project is organised against (Law 2).
2. **IS CURRENCY CONSERVED?** Every penny of somebody's revenue is a penny of somebody's
   expense. Every leg has an opposite leg, on a named party's book, in the same period and
   the same money (Law 5). A one-sided flow is a defect even when nothing fails.
3. **IS THE MECHANISM CORRECT AND REALISTIC** to the limit of this model's approximation
   (Law 1)? Not "does it run" — does it work the way the thing it names works?

Severity: **A** — breaks a law, and something in the world is wrong because of it.
**B** — breaks a law, and nothing is visibly wrong yet (a FORBID that holds by luck, an
unreached path). **C** — correct today, wrong under a stated near-future change; or a
declaration that lies about what the code does.

Every finding names the file, the line, what was measured or read, and what the fix is.

---

### The findings of Part I

**70 findings** — 30 A, 16 B, 24 C. Grouped by what is wrong rather than by file; several appear under more than one heading because they are one defect with two consequences.

### Value that is created or destroyed with no counterparty

- **A-39** (A) — the wage bill capitalised into inventory is bigger than the wage that was paid, every period, for every row
- **A-57** (A) — a securitisation vehicle keeps the whole interest stream of its pool, for ever, and nobody owns it
- **A-64** (A) — a firm pays rent for storage space and receives nothing for it
- **A-68** (A) — loading a cargo writes off what the cargo cost
- **A-1** (B) — `reseat` books an issuer's whole liability against a PER-MEMBER equity account

### A base, a price or a premium that is the wrong quantity

- **A-37** (A) — selling an asset counts as income, and the comment above the code says it must not
- **A-46** (A) — income tax is levied on the return of a household's own capital
- **A-38** (B) — a cell's outside option is every kind of money it received
- **A-58** (A) — the price a bank will pay for a note is a leverage ratio squared, called a cost of funds
- **A-65** (A) — every option premium is a money-squared number, and it does not depend on the strike
- **A-23** (C) — `wealthOf` and `atRisk` add currencies; the function beside them refuses to
- **A-51** (A) — the kernel converts currencies and five module reads of the same books do not
- **A-44** (B) — the household's liquidity premium is declared "over what a deposit returns" and is used as an absolute rate
- **A-67** (A) — nothing ever borrows a security: the whole securities-lending mechanism is unreachable
- **A-28** (A) — a seller's margin is on the buyer's side
- **A-45** (C) — a bank that owes nothing has a cost of funds of zero
- **A-34** (B) — a firm with no wage history bids for inputs as if labour were free
- **A-6** (C) — `labourParam` is declared in minutes and holds hours

### A mechanism that is built and never runs

- **A-60** (A) — no bank makes a market in anything, because no bank employs anybody
- **A-54** (A) — `gather` is called by exactly one module, so two other modules' venue schedules are never asked for
- **A-55** (A) — nobody in this world can buy a dwelling, so the whole of Housing B1–C4 never runs
- **A-56** (A) — three of this world's lines have a firm, a recipe and a market, and no buyer; they open at zero and never move
- **A-66** (A) — eight of the nine derivative books can never produce a first print
- **A-67** (A) — nothing ever borrows a security: the whole securities-lending mechanism is unreachable
- **A-9** (A) — the whole insurer sector cannot open a single policy
- **A-16** (C) — a reverse split throws halfway through and leaves the register restated for some holders and not others
- **A-17** (A) — three of XI-15's five weight events never fire, and nobody in this world is ever born
- **A-63** (A) — every household buys its groceries on thirty-day credit; the consumption-tax base is therefore always zero, and the households audit family fires on every purchase
- **A-69** (C) — nine exported entry points and reads that nothing calls
- **A-70** (A) — a takeover buys the shares and never absorbs the company

### An audit family that cannot fail, or has switched itself off

- **A-10** (A) — the currency audit family compares a number against itself
- **A-14** (A) — that same household check is an identity that cannot fail
- **A-42** (A) — two `units` families switch themselves off in any period with a weight event, which is every period
- **A-48** (C) — `equityIsZero` cannot fail for the reason it says it checks
- **A-13** (B) — the population identity is checked for households and for nothing else
- **A-12** (C) — a comparison in `names` that can never be true
- **A-52** (C) — an audit family recovers a fact by parsing the human-readable reason string
- **A-11** (C) — `flows` switches Money D3 off for a whole cell for a whole period

### A flow with one side, or a decision with no flow behind it

- **A-59** (B) — the resolution auction is decided on a price that is never paid
- **A-19** (B) — probate takes in every currency and pays out one
- **A-20** (A) — a recorded settlement failure in probate becomes an unhandled throw
- **A-41** (C) — `shed` says oldest first and does newest first; a trading employer that fails to pay severance records that nothing is owed
- **A-63** (A) — every household buys its groceries on thirty-day credit; the consumption-tax base is therefore always zero, and the households audit family fires on every purchase

### Arithmetic that loses people, money or capacity every period

- **A-18** (A) — the fraction of a person is discarded every period, and the world fragments into frozen micro-cells
- **A-40** (B) — the labour print publishes a volume nobody was hired for
- **A-26** (A) — a household redeems its whole shortfall from every fund it is in, and subscribes its whole cushion to every fund there is
- **A-27** (A) — the cell counts a commodity fund as money it can spend today
- **A-53** (B) — a household bids its entire income as rent, and its spending plan does not know rent exists
- **A-31** (A) — a depositor's stickiness decays to zero, because the test is a sunk cost that grows without bound

### A read that is stale, or reaches past what it may see

- **A-33** (B) — `lastOwn` has no period bound, and four firm reads treat a stale wage bill as this period's
- **A-2** (B) — the merge guard compares liens by COUNT, and its own docstring says it compares them
- **A-3** (B) — the merge writes the weight before the guard runs
- **A-15** (C) — the register has five per-party stores and the cell doors know about three
- **A-43** (C) — the labour module builds the household's schedule, through the door the architecture says not to
- **A-61** (A) — every central bank remits its income to the same treasury, in its own money
- **A-62** (B) — a central bank's loss is forgotten at the next remittance, and the comment says it is carried
- **A-50** (C) — `moneyOf` hand-builds an account id, which is the one thing `accountOf` exists to prevent

### A declaration, comment or docstring that says the opposite of the code

- **A-22** (C) — `lifecycle.ts`'s header says dying is not in this file, directly above the dying
- **A-4** (B) — a claimed assembly guard on `exposedTo` does not exist
- **A-5** (C) — `payable`, `cashFor` and `deliverable` take a unit and ignore it
- **A-7** (C) — the seed reads a capital kind's life off the declaration, not the register
- **A-8** (C) — a doc block describing a declaration that is gone
- **A-24** (C) — the household's own plan round-trips through `unknown` and drops what it cannot parse
- **A-25** (C) — an unpriced physical leg is valued at zero inside an audit family
- **A-29** (B) — a bid is capped at the issued amount to stop an overflow
- **A-30** (C) — three numeric defaults where the discipline is `Missing`
- **A-32** (C) — `levelsBelow` claims a refinement invariance its own arithmetic does not have
- **A-35** (C) — `buying` is computed twice, two ways
- **A-36** (C) — the treasury's immortality is unconditional where the kernel says it is conditional
- **A-21** (B) — one arbitrary cell inherits everything in a region
- **A-47** (A) — a fund's mandate has no currency in it, so it bids its own money at a foreign price and its NAV adds two moneys
- **A-49** (C) — `offeredYield` takes the last curve family, not the best, and answers zero when there is none

---

### Part I findings, in the order they were found

### A-1 — `reseat` books an issuer's whole liability against a PER-MEMBER equity account (B)

`ledger/settlement.ts:1228-1257`, the `reseat` op. `owed` is summed over holders as
`Σ (lots × weight)` — a TOTAL — and then `bump(op.from, owed)` / `bump(op.to, -owed)` write it
straight into `equity`, which every other path in this file keeps **per member of the party**
(`register/register.ts:EquityMove.delta`, "per member for a cell").

The same defect was found and fixed in two of its three places: `issue` (line ~1150) and `redeem`
(line ~1205) both wrap the amount in `perMemberOf(op.issuer, …)`, whose own comment says why —
_"the day a CELL issued something, it booked a million households' worth of liability against one
household's equity"_. `reseat` is the third place and it was not wrapped. So is the issuer re-mark
inside the `credit` case (line ~1120): `bump(issuerOf(inst), mul(op.totalQty, carrying − basis, …))`
uses `totalQty`, again unscaled.

**Not reachable today** — the three drafters of an `assume` leg (`estate`, `control`,
`money-market/resolution`) all move paper from a named party, and the `owes: 'value'` kinds are
issued by funds and insurers, which are named. It fires the first time a cell's issued paper is
assumed or its shares change hands, which is exactly the door 13d.1 opened.

**Fix:** the same one, in the two remaining places — `perMemberOf(op.from, owed)` /
`perMemberOf(op.to, owed)`, and `perMemberOf(issuerOf(inst), …)` on the credit re-mark. Better: make
`bump` take the total and divide, so there is one door and a new writer cannot forget.

### A-2 — the merge guard compares liens by COUNT, and its own docstring says it compares them (B)

`register/register.ts:724 sameState`. The doc above `forget` says a merge is legitimate because the
two cells' per-member state is _identical_, and `sameState`'s own doc lists _"the same lots in the
same order at the same basis and the same age, **the same liens**, the same equity"_. The code
compares `x.liens.length !== y.liens.length` and never looks inside a lien again: two cells with one
lien each, of different sizes, to different beneficiaries, for different reasons, are judged the
same. `forget` then deletes the absorbed cell's whole register position — its liens with it — with
no instruction and no counterparty, which is what `forget`'s own docstring forbids (Law 5,
Appendix B "no collateral counted as available by both").

A household cell does carry liens: `mechanisms/housing/index.ts` pledges a cell's dwellings to its
mortgage lender at a size that is `outstanding / price`, which differs between two cells whose
mortgages differ.

**Not reachable today** — nothing calls `ctx.cells.merge`; the live callers are `split`, `reKey`,
`die` and `weight`. It is a guard that does not guard, not a live corruption.

**Fix:** compare the liens as the lots are compared — qty, beneficiary, reason — and the equity
LEDGER as well as the equity balance (`forget` deletes both and only one is checked).

### A-3 — the merge writes the weight before the guard runs (B)

`world/cells.ts:85-95`. `d.parties.applyWeight({kind:'merge', …, after: ca.weight + cb.weight})`
runs, and _then_ `d.register.forget(b, a)` runs the state comparison that decides whether the merge
was legal at all. A merge the register refuses has already grown the absorbing cell's weight, and
the journal entry that would have recorded it comes after the throw, so the world is left with a
weight nobody can account for and no event saying where it came from.

The throw stops the run, so nothing persists — but the order is backwards and it costs nothing to
put right: ask the register first, apply the weight second, journal third.

### A-4 — a claimed assembly guard on `exposedTo` does not exist (B)

`registry/environment.ts:conditionsFor`, the comment on the `value !== undefined` branch: _"A line
naming a fact this world does not have stands in an ordinary period for it. The world that has the
fact is where the check belongs, **and assembly is where it fires**."_

It does not fire anywhere. `world/assemble.ts` checks module ids, dependency cycles, money issuance
and bank choices, and nothing else; `grep -rn exposedTo world/` returns nothing. A recipe naming a
fact no environment module declares silently multiplies by 1 — an ordinary period — for ever.

Today every `exposedTo` in `mechanisms/goods/data.ts` names `growing` or `warmth` and both are
declared, so the number is right. The guard is a lie, and it is the kind that is discovered by a
line that quietly never has a bad season.

**Fix:** check it at assembly, and delete the paragraph (Law 12: the fix removes the comment).

### A-5 — `payable`, `cashFor` and `deliverable` take a unit and ignore it (C)

`registry/registry.ts`. All three are declared `payable(_ccy, amount)`, `cashFor(_ccy, value)`,
`deliverable(_unit, qty)` and the body is `downTick(amount)` / `toTick(value)`. Every quantity in
the engine is already a count of pieces, so no conversion is possible or wanted — but the signature
says one happens, and it invites exactly one mistake: handing it a NAMED amount (188.49 USD) where
it wants pieces (18849). That is the same class of error as the two unit slips 11.5 found in the
seed, and here the parameter list actively suggests it.

**Fix:** drop the parameter, or make it do the conversion it claims. Not both.

### A-6 — `labourParam` is declared in minutes and holds hours (C)

`mechanisms/goods/index.ts:122-129`. The value is `(d.labourHoursPerUnit * TIME_PIECES) /
PIECES_PER_UNIT` and `TIME_PIECES` is 1 because a piece of time is AN HOUR
(`registry/grid.ts`: _"Time: THE HOUR"_). The declared `unit` string is
`` `minutes per piece of ${d.subUnit}` ``.

`dimension: 'ratio'` is what a machine checks, so nothing catches it; the prose is what a person
reads, and a person who believes it is out by sixty. Law 8 says the unit is part of the number.

**Extension (verified against `registry/grid.ts`).** `TIME_PIECES = 1` and grid.ts is explicit
about why: _"Time: THE HOUR. Labour is contracted, supplied and paid for by the hour in this world
and no wage in it is struck for part of one, so an hour is the smallest piece of somebody's time
there is."_ So the value is hours per piece and the unit string is wrong by a factor of sixty.

The same stale premise is in a second place. `mechanisms/labour/index.ts`:

```ts
// Labour A1, Law 8: time has a smallest piece too. A thousandth of an hour is about four
// seconds, which is finer than any contract in this world states and coarse enough to be real.
units: [{ id: HOURS, name: 'hours', perUnit: TIME_PIECES }],
```

`TIME_PIECES` is 1. The comment describes 1000. Two comments in two modules both describing a grid
this world does not have (Law 16).

### A-7 — the seed reads a capital kind's life off the declaration, not the register (C)

`seeds/foundation.ts:1102` — `div(inService, kind.usefulLifePeriods, 'what wears out in a period')`,
straight off `CapitalKindDecl`. Every other reader of that fact goes through the parameter register:
`mechanisms/capital-programme/index.ts:159,198` and `mechanisms/firms/decide.ts:525` all use
`params.periods(lifeParam(d.id))`, which is where the number is declared with its unit and owner
(XI-14) and where a resolution shift would move it.

Two readable homes for one fact (Law 4, Law 19). It agrees today because both read the same table.
`mechanisms/firms/produce.ts:349` is the pattern that is right: it touches the decl only to ask
whether the fact EXISTS and reads the value through `params`.

### A-8 — a doc block describing a declaration that is gone (C)

`registry/physical.ts`, immediately above `STORAGE_SESSION`: a full block beginning _"A4, A4.b, C3:
the two numbers a kind of capital states about itself, named."_ It sits directly on top of a SECOND
doc block, the one that documents `STORAGE_SESSION`. Whatever it described was deleted and the
comment stayed. A stale comment is a defect (Law 16).

### A-9 — the whole insurer sector cannot open a single policy (A)

`mechanisms/insurers/index.ts`. Four separate things, and each alone would be enough.

**1. The policy kind's unit does not exist.** Line 124:

```ts
unit: (ccy) => ccy as unknown as ReturnType<InstrumentKindProfile['unit']>,
```

It hands back the CURRENCY CODE (`"USD"`) where a `UnitId` is wanted. A currency's unit is
`currencyUnit(ccy)` = `"ccy:USD"`; the module declares a unit of its own, `COVER` (`"cover"`,
`perUnit: MONEY_PIECES`), and never uses it. `Instruments.add` calls `this.registry.unit(unit)`,
which throws `Missing [Appendix A] unit USD does not exist`. **No `policy` instrument can be
registered in any world.** The `as unknown as` is what got it past the compiler — the one cast in
the file, and it is casting away exactly the check that would have caught this.

**2. Nothing runs.** The module declares `phases: []` and `participants: []`. `quoteCover`,
`runCover`, `policyId`, `venueForCover` and `presentValueOf` are exported and called from nowhere in
the engine. §27 is a kind registry and an audit family over an empty set.

**3. `runCover` clears a book and settles nothing.** It opens the venue, posts the bids, calls
`clear`, and on `cleared` records an event and returns — `outcome.fills` is discarded. No policy is
issued, no premium is paid, no beneficiary is named. A session that strikes a price and moves no
money is the "cleared price nobody paid" the market runner deletes prints for (Clearing E1), here in
a venue that nobody checks.

**4. `derive` returns 0 where its own comment forbids it.** Lines 168-173:

```
// XI-6: a curve with nothing on it prices nothing. Missing is Missing — never a zero that would
// read as a liability the institution has discharged.
return priced.some ? priced.value : 0;
```

The comment says what must not happen and the next line does it. A policy whose curve has no points
values at nothing, so the insurer's largest liability vanishes and its equity jumps by the whole of
it — which is precisely "a liability the institution has discharged".

**5.** `claimsSeen` reads the insurer's OWN HOLDINGS for policies (`view.holdings()`, filtered by
`isPolicy`). A policy is the insurer's LIABILITY; the beneficiary holds it. So `written` is always
empty, `cover` is always 0, and `claimsSeen` always returns 0 — the experience half of A4.b's price
is structurally dead. It should read `view.instruments.issuedBy(self)`.

**Effect:** the model's largest holder of duration does not exist. B2.b is the clause §27 was built
for and the sector cannot write the promise the clause is about.

### A-10 — the currency audit family compares a number against itself (A)

`audit/families/currency.ts:revaluationAddsUp`. It walks `revaluation.fx` events and builds two
maps:

```ts
addTo(booked, ccy, delta);
addTo(implied, ccy, carried * sub(now, was, 'what the rate moved by'));
```

reading `delta`, `carried`, `was` and `now` off the same event. `world/revalue.ts:revalueForeign`
wrote that event, and it computed the field it wrote as:

```ts
const delta = mul(carried, sub(now, was, 'what the rate moved by'), 'what it did');
… { deltaPerMember: delta, ccy, home, was, now, carried }
```

So `implied` is `booked` recomputed from the operands `booked` was computed from — the same product,
to the bit. **The family cannot fail.** Audit A1.a in as many words: _"a read of two independent
things that must agree — never a read of one thing against itself, which always passes."_

The check it is trying to make is real and is not being made: what every revaluation BOOKED against
what the period's rate move on the positions comes to. The second half has to be reached from the
REGISTER — walk the holdings, take each one's carrying and the rate move, and compare the total
against the sum of what the equity and revaluation accounts actually moved by. That is two paths;
this is one path twice.

It is contributed to the `money` family, so `money` reports two contributions of which one is a
tautology, and `built` is true for both.

### A-11 — `flows` switches Money D3 off for a whole cell for a whole period (C)

`audit/families/flows.ts`, the `copied` set: a cell named as the `to` of a split or promotion, or
the `from` of a merge, has EVERY holding of it exempted from the identity for that period
(`if (copied.has(holder)) continue;`). The narrowing from "every subject of every weight event" was
right and this is still wider than the hole: what has no leg behind it is the copy of the parent's
book at the instant of the split, not everything that cell then does. A cell created by a split at
cycle 0 that trades at cycle 1 has that trade unchecked.

The exemption exists because `copyMemberState` moves a whole book with no instruction. The narrower
form is to compare against the parent's remembered per-member state rather than against nothing —
the split is exact, so the new cell's opening position IS the parent's, and any difference is a leg.

### A-12 — a comparison in `names` that can never be true (C)

`audit/families/names.ts`:

```ts
!view.markets.some(
  (m) => m.id === i.market.valueOf() || (i.market.some && m.id === i.market.value),
);
```

`i.market` is an `Option<MarketId>`; `.valueOf()` on it returns the option OBJECT, so
`m.id === i.market.valueOf()` compares a string to an object and is always false. The second
disjunct does the whole job. Dead code in the family whose subject is that references resolve.

### A-13 — the population identity is checked for households and for nothing else (B)

`audit/families/units.ts` checks that a weight is a positive count, that physical stock moves with
its create/destroy legs, and that every lot is on the grid. Part XII's units family is also
_"including a population: the sum of cell weights equals the population it stands for, and every
represented party sits in exactly one cell."_

Neither of those two is in the kernel family. One of them is contributed by a module:
`mechanisms/labour/index.ts:workforceIdentity` (contributor `labour`, family `units`) checks, per
region, that the employment rows' headcounts equal the cells' weights and that no cell holds two
jobs — a real F2 check on the register against the cells. The other contributors to `units` are
`capital-programme` (`index.ts:435`) and `goods` (`:266`, `:328`), neither of which counts people.

So the identity is measured for HOUSEHOLD cells through their employment, and is unmeasured for
every other cell in the world (`firms`' small-business pools, XI-15 cells generally): a split, merge
or promotion on one of those can lose or invent members and no family looks. Narrowed from the
original wording, which said the identity was checked nowhere.

### A-14 — that same household check is an identity that cannot fail (A)

`mechanisms/labour/index.ts:workforceIdentity`, the check A-13 credits, contains the half of Part
XII's identity that reads on population, and it is written as a tautology:

```ts
acc.people += p.weight;
acc.employed += isEmployed ? p.weight : 0;
// B3: exactly one of the three, counted once each way round.
acc.states +=
  (isEmployed ? p.weight : 0) +
  (!isEmployed && working ? p.weight : 0) +
  (!isEmployed && !working ? p.weight : 0);
```

The three terms are `E`, `¬E ∧ W`, `¬E ∧ ¬W`. They are mutually exclusive and exhaustive over the
two booleans, so for every cell exactly one term is `p.weight` and the other two are `0`. Therefore
`acc.states === acc.people` on every iteration, by construction, and

```ts
if (acc.states !== acc.people) { … 'employed plus unemployed plus inactive is …' }
```

can never fire. The comment says the opposite — _"counted once each way round"_ — but there is only
one round: both sides are the same sum of the same `p.weight`s over the same loop. This is A-10's
defect again (Audit A1.a: a read of one thing against itself), and it is severity A rather than B
because the docstring, the spec citation (`Labour B5`) and `built: true` all report that Part XII's
population identity is being measured when nothing is.

What would make it a measurement is a second, independent record of the three states — the counts
the labour mechanism itself acts on when it hires and separates, or the cohort register's own
population — compared against the cells. Two reads of the same `p.weight` is not two records.

The two neighbouring checks in the same function ARE real: `row.headcount !== weight` and
`inRows !== acc.employed` read the employment book against the parties store, which are genuinely
two writers. Only the B5 line is empty.

### A-15 — the register has five per-party stores and the cell doors know about three (C)

`register/register.ts` holds five things keyed by party: `byHolder` (126), `equityAccount` (128),
`equityLedger` (135), `revaluationAccount` (151) and `moneyAccount` (158, keyed `${holder}/${inst}`).

- `copyMemberState(from, to)` copies the first three. Its one-line doc says _"Copy per-member state
  to a new party"_ and the block inside says _"the ITEMISATION, which is per-member state like
  everything else here"_. The revaluation account and the money walks are per-member state and are
  not copied.
- `forget(party, into)` deletes the first three and leaves the other two behind, keyed to a party
  that no longer exists.
- `sameState(a, b)` — the guard that decides whether a merge is legitimate — compares holdings and
  the equity account, and never the revaluation account.

Reachability today: `world/revalue.ts:362` moves the revaluation account for exactly one party,
`registry.centralBankOf(home)`, which is always named and never a cell — so the revaluation half is
inert now and becomes wrong the day any cell holds foreign money. The money-walk half is live: after
a split the new cell's `moneyWalk` opens at `opened(0, …)` with **zero dust**, while its copied lots
carry a real balance. `accounts.ts:95,152` and `ownership.ts:29` add that dust into the tolerance,
so a split cell is checked against a tolerance that is too TIGHT for the balance it holds. That is
the safe direction under Law 7, and it is still a spurious violation waiting to be reported against
a cell that has done nothing.

### A-16 — a reverse split throws halfway through and leaves the register restated for some holders and not others (C)

```ts
restate(instrument, ratio) {
  for (const holder of this.holdersOf(instrument)) {
    const h = this.mutable(holder, instrument);
    h.lots = h.lots.map((l) => Object.freeze({ ...l,
      qty: this.onTheGrid(finite(l.qty * ratio, …), …),
```

`onTheGrid` **throws** (`impossible`, Law 8) when `l.qty * ratio` is not a whole number of pieces —
deliberately, and the docstring is proud of it: _"It throws rather than rounding"_. But the throw
happens inside a loop that has already assigned `h.lots` for every holder before this one. A
`PhoenixError` is not caught in the engine, so the world stops — with the line restated for holders
`0..k-1`, unrestated for `k..n`, and the issued amount not yet touched at all (`world.ts:1557-1559`
calls the register first, then `instruments`, then `prices`). The store is left in a state that
`Σ holdings == issued` no longer describes.

Two things are wrong and they are separable:

1. **The write is not atomic.** Every other multi-holder write in this file goes through settlement,
   which validates every leg before applying any. This one validates as it applies.
2. **There is no cash in lieu.** A real reverse split pays the fractional remainder out in money
   (Law 1: real mechanism, real refusals, real fees). Here a 1-for-10 on a holder of 7 pieces is not
   a payment of 0.7 pieces' worth of cash — it is an exception that stops the world. The mechanism
   that ought to exist (round the holder down, pay the stub) is missing, and `onTheGrid` is standing
   where it should be.

Reachable? Not today: `MechanismContext.split` (`world/context.ts:548`) is declared, wired
(`world.ts:1455`), guarded (`splits === true`), journalled — **and called by no module in the
repository**. Grep finds `splitInstrument` only at its definition and its one wiring site. So Equity
D4's split is built end to end and nothing in this world ever causes one. Under CLAUDE.md's own bug
rule ("a mechanism that never runs") that is the finding, and the reverse-split defect is what is
waiting behind it.

### A-17 — three of XI-15's five weight events never fire, and nobody in this world is ever born (A)

`world/cells.ts` implements the five: entry, death, promotion, split, merge. `CellEvents`
(`world/context.ts:384`) exposes `split`, `merge`, `weight` (entry/death/promotion), `reKey`, `die`.
Across `mechanisms/` and `seeds/` the callers are exactly:

| door           | callers                                                                           |
| -------------- | --------------------------------------------------------------------------------- |
| `cells.split`  | `labour/matching.ts:312`, `labour/matching.ts:405`, `households/lifecycle.ts:250` |
| `cells.reKey`  | `households/lifecycle.ts` (ageing)                                                |
| `cells.die`    | `households/lifecycle.ts` (after probate)                                         |
| `cells.merge`  | **none**                                                                          |
| `cells.weight` | **none**                                                                          |

So `entry` never happens: **no person is ever born in this world.** `age()` moves members from
cohort 0 into cohort 1 and nothing whatever moves into cohort 0. `die()` removes them at the top.
The population is monotonically non-increasing from the seed to the end of the run, by construction
and not as an outcome.

This is not "no birth rate" being respected (Appendix B forbids a _declared_ birth rate, which is
right). It is the mechanism that would produce births as an OUTCOME — a household's own decision,
with its own cause — being absent, and nothing naming it as absent. `lifecycle.ts`'s header names
what it leaves out and does not name this. Under Part II that makes it neither MISSING nor OUT OF
SCOPE but unstated, which is the one thing a clause may not be.

`merge` never firing is the other half of A-18 below: nothing in this world ever recombines two
cells that have become identical, so the cell count only ever rises.

### A-18 — the fraction of a person is discarded every period, and the world fragments into frozen micro-cells (A)

`households/lifecycle.ts`, twice:

```ts
// XI-15: a weight is a COUNT of people, so what crosses is whole people, and the fraction that
// is not somebody stays where it is until enough of it has accumulated to be somebody.
const crossing = Math.floor(mul(weightOf(cell), share, 'the people standing at the boundary'));
if (crossing <= 0 || crossing >= weightOf(cell)) continue;
```

```ts
// XI-15: whole people. The fraction that is not somebody waits until it is.
const dying = Math.floor(mul(weightOf(cell), rate, 'the people who die this period'));
if (dying <= 0 || dying >= weightOf(cell)) continue;
```

**The comment is false in both places.** Nothing accumulates. `crossing` and `dying` are recomputed
from `weightOf(cell)` every period, so the fraction below one person is thrown away every period and
never "waits until it is" anybody. There is no residual store, no carry, no accumulator anywhere in
the file.

The consequence is not a rounding nicety, it is a trap, and the arithmetic closes it:

- A cohort band spanning 10 years is ~521 periods, so `share ≈ 1/521 ≈ 0.00192`.
- The seed opens 15,000,000 members per cohort (`seed.households.membersPerCohort`) over
  `banks × 2` cells (`seed.households.cellsPerKey = 2`). With four banks a cell is ~1.9M members and
  ages ~3,600 of them a period into a **new** cell (`reKey` → `nextSplitId`, never into an existing
  cell of the target cohort — and `cells.merge` is called by nobody, A-17).
- That new cell of ~3,600 ages `floor(3600 × 0.00192) = 6` a period. Its children age
  `floor(6 × 0.00192) = 0` — **for ever**.
- Mortality is worse, because the rates are smaller: any cell below `1/rate` members has
  `floor(weight × rate) = 0` and **nobody in it ever dies**, at any age, for the life of the run.

So after a few hundred periods the world holds a linearly growing population of micro-cells that
can never age out of the cohort they are in, can never die, and can never merge back into anything.
They keep voting, working, consuming and holding money for ever. `crossing >= weightOf(cell)` adds
the other end of the same defect: a cell small enough that its whole weight would cross is skipped
entirely rather than moved as itself, so the last members of a band are pinned there too.

The honest mechanism is the one the comment already describes: a per-cell remainder that carries
forward, so that `floor` is a timing of a real event rather than a deletion of it. It exists
nowhere.

### A-19 — probate takes in every currency and pays out one (B)

`handToProbate` was deliberately widened to hand over every money a dead cell held:

```ts
// 13j: EVERY MONEY IT HELD, and not only its own. A world with four of them has households paid a
// coupon in one they do not bank in (Currency C4), and this used to move the cash of the cell's own
// region and leave the rest …
const monies = new Set<CurrencyCode>([ctx.registry.currencyOf(region)]);
for (const h of view.holdings()) { if (isMoney(…)) { monies.add(ctx.instruments.get(h.instrument).ccy); …
```

`settleEstates`, forty lines below, still does what the fixed side used to do:

```ts
const ccy = ctx.registry.currencyOf(office.region);
…
const cash = view.cash(ccy);
```

One currency: the office's own region's. Every other money a dead household held arrives at probate
and never leaves. Probate `never trades` (its own docstring), has `fails: []`, `borrows: false` and
no other outlet in the module, so a foreign balance there is permanent. It is a holder, so it is not
formally a residual with no holder — it is worse in one respect, because the accounts family will
happily confirm it every period: probate's equity grows without bound and the money is out of the
circuit for good.

The inbound fix names the exact reason the outbound one is needed (households are paid coupons in
money they do not bank in) and stops one function short.

### A-20 — a recorded settlement failure in probate becomes an unhandled throw (A)

```ts
const estate = ctx.cells.split(cell.id, dying, 'died');
handToProbate(ctx, estate, office, cell.region);
ctx.cells.die(estate, office, 'died');
```

`handToProbate` ends with `ctx.settle({...})` and **discards the record**. `settle` returns
`Settled | Failed` and a fail is an ordinary recorded state (`Money E1 / Register C3.b: a fail is a
real, recorded state; nothing half-settles`) with three reachable reasons —
`insufficientUnits`, `overdraftRefused`, `insufficientCollateral`.

`dieCell` (`world/cells.ts:238`) then does:

```ts
forbid(
  d.register.holdingsOf(cell).length === 0,
  'Appendix B',
  `${cell} still holds something and cannot die; …`,
);
```

`forbid` throws a `PhoenixError`, which the engine never catches. So any failed estate transfer
stops the world. The same happens with no failure at all whenever the dead cell holds anything
**encumbered**: the asset legs are built from `view.free(h.instrument)`, a lien is not free, the
units stay, and `dieCell` throws on the leftovers.

The error discipline in CLAUDE.md is explicit about which of the two this is: a contract violation
throws, an invariant violation is an audit finding — and "this cell still has units because a
payment did not go through" is neither. It is an outcome, and the module has to read it: if the
estate did not clear, the cell does not die this period and tries again next.

### A-21 — one arbitrary cell inherits everything in a region (B)

```ts
function heirOf(ctx, region, bank): PartyId | undefined {
  const first = String(ctx.registry.cohorts[0]?.id ?? '');
  return ctx.parties
    .ofKind(HOUSEHOLD)
    .find(
      (p) =>
        p.status.alive &&
        p.representation === 'cell' &&
        p.region === region &&
        p.bank === bank &&
        keyOf(p, 'cohort') === first,
    )?.id;
}
```

`.find` — the FIRST cell the parties store happens to return. Every estate in that (region, bank)
goes to that one cell and to no other, so one household cell in each region accumulates the wealth
of everybody who dies there, and the rest inherit nothing ever. Which cell it is depends on the
parties store's insertion order, which is a seed-draw artefact and not a fact about the world.

Two things are wrong. The distribution is an OUTCOME written as a lookup (Law 2), and it is a
decision taken at no party's own reason — nobody chose an heir, nobody has a claim, and there is no
mechanism (a will, a kinship key, a share of the cohort) behind it. `handToProbate`'s own header
says the survivors of the dead cell's _own key_ are the natural somebody; `heirOf` ignores the key
entirely except for the cohort, and then takes one of them.

Note also `ctx.registry.cohorts[0]?.id ?? ''` — an `?? ''` producing a cohort id that matches
nothing, where the file's own discipline is `Missing`.

### A-22 — `lifecycle.ts`'s header says dying is not in this file, directly above the dying (C)

```
 * WHAT IS NOT HERE IS DYING, and it is named rather than missing. What a dead cell held has to
 * reach somebody, … and that is 13d.1's remaining step rather than something to approximate here.
```

Lines 133-317 of the same file are `PROBATE`, `probateKind`, `heirOf`, `handToProbate`, `die` and
`settleEstates`. 13d.1 closed and the header did not move. Law 16: a stale comment is a defect, and
this one tells a reader the opposite of what the file does.

### A-23 — `wealthOf` and `atRisk` add currencies; the function beside them refuses to (C)

`households/consume.ts`:

```ts
function wealthOf(view: ParticipantView, cash: number): number {
  const terms = [cash]; // the HOME currency
  for (const h of view.holdings()) {
    // every holding, whatever it is denominated in
    const print = view.print(h.instrument);
    if (!print.some) continue;
    terms.push(mul(units.value, print.value.price, 'what it holds is worth'));
  }
  return sum(terms).value;
}
```

`view.print` returns the print in the INSTRUMENT's own money. `cash` is `view.cash(currencyOf(region))`,
the cell's own. Nothing converts and nothing checks: this is `Appendix B`'s _"two currencies never
added"_, written out. `atRisk`, ten lines up, does the same thing with `confidence`, which the
`Outlook` contract says is _"in the same unit as the variable"_ — the variable being
`price.<instrument>`, i.e. that instrument's money.

Both feed `spendPerMember`: `wealth` into the gap that sets what a household spends, `atRisk` into
the cushion it holds. A foreign line at a rate of 1 hides completely; at any other rate the cell's
whole spending decision is taken on a sum of two moneys.

`savingLines` in the same module, forty lines away, is careful about exactly this:

```ts
if (!i.status.live || !i.market.some || i.ccy !== ccy || !i.issuer.some) continue;
```

and the seed is careful too (_"Its own country's paper: a household saving in a money it is not paid
in would be a currency position nobody took (Currency D2)"_). So today a household's holdings are
all in its own money and the defect does not bite. It is C and not A for that reason only — the
guard is in the two places that CHOOSE what a household acquires, and absent from the place that
VALUES what it has. `handToProbate`'s own header asserts the opposite is already happening (_"A world
with four of them has households paid a coupon in one they do not bank in"_), and if that is true
this is A rather than C. `ctx.valuation.inMoney` exists and is the read that would settle it.

### A-24 — the household's own plan round-trips through `unknown` and drops what it cannot parse (C)

`decide` publishes its orders into a journal event as `Record<string, unknown>`; the participant
reads them back with `marketsIn`/`ordersFrom`, which re-validate from scratch:

```ts
if (price !== 'market' && typeof price !== 'number') continue;
if (typeof qty !== 'number' || qty <= 0) continue;
```

The Law 4 intent is right and worth keeping — one decision, one writer, read back rather than
recomputed. The implementation loses the type at the journal boundary and then handles the loss by
**silently dropping the order**. A cell whose plan wrote a row this pair of predicates rejects
simply does not trade that period, and nothing anywhere says so: no throw, no violation, no event.
That is a decision disappearing between the party that took it and the book it was for, which is
the one thing this round trip exists to prevent.

`ordersFrom` also drops any row with `qty <= 0` before `asQty` can complain, so the file's own
comment — _"through the one door that says a size is a count of pieces — and that throws if what it
published was not"_ — is only true for positive non-integers.

### A-25 — an unpriced physical leg is valued at zero inside an audit family (C)

`households/index.ts`, `consumptionIsBought`:

```ts
addTo(
  bought,
  leg.to,
  mul(leg.qty, leg.pricePerUnit.some ? leg.pricePerUnit.value : 0, 'what it took'),
);
```

A `? … : 0` on an `Option` inside a mechanism, which the error discipline forbids without
qualification (_"No `?? 0`, no `|| 0`, no numeric defaults … Missing is `Missing`"_). The family then
compares that zero against the money the cell paid and reports `took 0 of goods and paid X` — a
violation whose size and message are both about the missing price rather than about the flow. The
family's subject is that goods and money move together; an unpriced leg is a different defect and it
should be reported as one, or the leg should not be admissible.

### A-26 — a household redeems its whole shortfall from every fund it is in, and subscribes its whole cushion to every fund there is (A)

`households/portfolio.ts`, `fundOrders`:

```ts
for (const p of positions) {
  if (short > 0) {
    const want = downTick(div(short, p.perShare, 'shares it must give back'));
    out.push({ venue: p.venue, side: 'sell', sharesPerMember: atMost(want, p.sharesPerMember, …) });
    continue;
  }
  if (toFund <= 0 || p.offered < required) continue;
  const buying = downTick(div(toFund, p.perShare, 'shares it asks for'));
  if (buying <= 0) continue;
  out.push({ venue: p.venue, side: 'buy', sharesPerMember: buying });
}
```

Neither `short` nor `toFund` is decremented inside the loop. Both are the cell's WHOLE per-member
number, and both are used again, in full, at every position. A cell short of £100 with shares in
three funds asks all three for £100. A cell with £100 for its cushion subscribes £100 to every fund
whose `offered` clears its requirement.

This is reachable and not a corner: `drawFunds` (`mechanisms/funds/data.ts:262`) creates one money
fund **per bank** above `MONEY_FUND_SPONSOR_SIZE`, plus a commodity fund. With four qualifying banks
a cell that has saved for a while holds several positions, and every redemption is multiplied by
how many.

The consequence is the module's own subject, inverted. Its docstring says the redemption channel is
what makes _"a shock to incomes become a redemption wave"_ and that _"nothing here is an allocation:
the flow is a consequence of the cell's own budget"_. As written the flow is a multiple of the
cell's budget, and the multiplier is the count of funds in the world — a number no participant
decided and nothing in the spec names. The wave's size is set by the seed's bank draw.

On the way in it is worse than an over-commitment: the cell posts a buy for money it does not have.
`toFund` came from `cushionForFund(cash, spend, spare)` and is the whole of what its account holds
over what it is about to spend. Committing it N times is committing money that is not there — the
same money on N tickets, which is exactly what `paperBids`' sibling comment says the per-line budget
split exists to prevent (_"so the same money is never committed twice"_). Paper got the split; funds
did not.

### A-27 — the cell counts a commodity fund as money it can spend today (A)

`fundPositions` selects venues by `v.key['kind'] !== 'fund'` and nothing else. Every fund declares
that key (`funds/index.ts:1557`), the commodity fund included — `drawFunds` builds
`fund.physical.<bank>` with `eligible: ['good.grain']` and `maxTenorPeriods: 0`, on the stated
grounds that _"a thing has no maturity … what it is holding for is a price"_.

`decide` then does:

```ts
const positions = fundPositions(ctx.venues, ctx.journal.ofKind('fund.struck'), view);
const onDemand = sum(positions.map((f) => f.worthPerMember)).value;
const decided = spendPerMember(view, p, onDemand);
```

and `spendPerMember` puts `onDemand` straight into `budget` — _"THE WHOLE OF WHAT IT CAN PAY WITH …
because nobody lends to it and those are the two places its money is"_. So a household's grocery
budget this week includes the market value of its stake in a grain fund, on the same footing as its
current account.

`fundOrders` compounds it: the buy branch tests only `p.offered < required`, so the cell's CUSHION —
the money it is holding precisely because it may need it at no notice — is subscribed to the
commodity fund whenever that fund's published return clears its liquidity premium.

The distinction the module is built on is stated three times in its own prose (_"a MONEY FUND is a
claim on short paper that can be asked for back at any time"_, _"Anything it cannot get back on
demand is wealth (C1.b) but not budget (C1.d), which is the whole difference between a fund share
and a bond"_) and is nowhere in the code. The fund kind knows the answer — `maxTenorPeriods`, or
`eligible` naming a physical kind — and nothing asks it.

### A-28 — a seller's margin is on the buyer's side (A)

`savingLines` sets one price per line and both sides use it:

```ts
const price = sub(expected, outlook.some ? outlook.value.confidence : 0, 'what it will pay');
```

The comment is explicit that the subtraction is the buyer's margin: _"a saver buying a claim that
promises it nothing wants the margin on its side"_. `shareOrders` then offers at the same number:

```ts
const price = short > 0 ? ('market' as const) : line.price;
out.push({ market, instrument: id, side: 'sell', price, qty: units });
```

with a docstring that says something different from what the line does: _"it OFFERS its holding at
what it thinks the holding is worth"_. What it thinks it is worth is `expected`. What it offers at is
`expected − confidence`.

So a cell that has been surprised by a price does not widen — it moves **both** its bid and its ask
DOWN by its own uncertainty. Two consequences, and the second is the one that matters:

1. The cell will sell at a level it would simultaneously buy at. A single cell cannot cross itself
   (the module is careful about that), but two cells with the same outlook and opposite budgets will
   trade at a price neither thinks is fair.
2. Uncertainty transmits to the price level with a sign. When a shock widens every cell's confidence,
   every ask in the book falls by the widening as well as every bid, so the cleared price falls by
   the full amount instead of the book merely getting wider. §46 A3 says disagreement is what gives
   a market two sides; here it is a common downward shift applied to both sides at once, which is
   the opposite property.

The correct read is already in the file — `expected` is right there in the `SavingLine`'s scope and
is thrown away. An ask of `expected + confidence` against a bid of `expected − confidence` widens
the spread on a shock and leaves the mid where the outlook is, which is what the prose describes.

### A-29 — a bid is capped at the issued amount to stop an overflow (B)

`shareOrders`:

```ts
const exist = line.instrument.issued;
for (const rung of rungsOver(levelsBelow(line.price, steps), perLine)) {
  const wanted = mul(rung.qty, weight, 'what the cell puts in');
  const qty = downTick(atMost(wanted, exist, 'there are no more units of it than were issued'));
```

`atMost`'s own contract (`core/num.ts:229`) is _"They are NOT a place to put a cap. If the reason
cannot be written as 'there is no more of it', the number is a decision or a missing mechanism and
Law 6 says so."_ A bid for more units than exist is not arithmetically impossible — it is a bid that
cannot wholly fill, which is the ordinary state of a book. The clearing solver is what decides how
much of a bid fills, and it does not need the bidder to pre-truncate.

The comment says out loud why the line is there: _"`asQty` threw at 1.5e16 in a year-long run;
worklist 12c is why the price collapsed"_. That is a bound added because a number exploded, which is
the case Law 6 legislates for directly: _"If a number explodes, the compensating mechanism is
missing — build it, and delete the bound in the same change."_ The record says 12c built the
compensating mechanism (a saver's level now comes from its own outlook rather than from a
capitalisation of published earnings). The bound was not deleted with it.

It is not inert. `downTick(atMost(wanted, exist))` makes every deep rung of a cheap line post
exactly `issued`, so several cells post identical maximal sizes at descending prices and the book's
depth below the top rung stops being a function of anybody's budget.

### A-30 — three numeric defaults where the discipline is `Missing` (C)

- `portfolio.ts:ownUncertainty` — `if (!income.some || income.value.expected <= 0) return 0;`. A cell
  with no income outlook is reported as wanting **no** extra return for holding a claim that
  promises nothing, which reads as certainty and is ignorance.
- `portfolio.ts:fundPositions` — `offered: typeof offered === 'number' ? offered : 0`. A fund whose
  strike event did not publish `offered` reads as offering nothing, so `p.offered < required` is
  true and the cell silently never subscribes to it.
- `lifecycle.ts:heirOf` — `String(ctx.registry.cohorts[0]?.id ?? '')`, producing a cohort id that
  matches nothing, so a world with no cohorts silently has no heirs rather than saying so.

### A-31 — a depositor's stickiness decays to zero, because the test is a sunk cost that grows without bound (A)

`households/bank.ts`:

```ts
const gap = sub(best.rate, own.value, 'what it would gain');
if (gap <= 0) return none();
const cost = view.params.amount(HOUSEHOLD_SWITCHING_COST, currencyUnit(ccy));
const foregone = mul(balance, mul(gap, stayed(view), 'over the time it has stayed'), 'what staying cost it');
return foregone > cost ? some({ to: best.bank, … }) : none();
```

`stayed()` is years since the cell last moved, or since the epoch. So the test is

balance × gap × yearsStayed > oneOffCost

and `yearsStayed` increases every period a depositor does not move. **For any positive gap, however
small, the inequality is eventually satisfied.** A quarter of a basis point moves every household in
the world if you wait long enough. The stickiness the module is built to produce —
_"a small one may never go at all: the class drains instead of crossing in one instant"_ — holds for
a while and then stops holding, and after it stops the population churns on a fixed cycle of
`cost / (balance × gap)` years, because `stayed` resets on each move and starts climbing again.

The reason it is written this way is stated: _"Nothing here is a forecast (Law 17): it is what has
already happened to it."_ Law 17 forbids a forecast without a falsification test; it does not
require a decision to be taken on a sunk cost. What moving is worth is `balance × gap × (how long it
expects to stay)`, and this world already gives a party its own forward-looking number for exactly
this kind of question — `view.outlook(...)`, with its own confidence, formed from its own history
(§46). Using accumulated regret instead is not the conservative reading of Law 17, it is a different
mechanism with a different and unbounded limit.

Also in the same file: `uninsured()` returns the **whole balance** — i.e. treats the depositor as
entirely uninsured — on four separate parse failures of the published deposit classes. A missing or
malformed publication therefore reads as "nobody is insured", and every household at a shaky bank
runs. Defaulting to panic is a modelling choice and nothing states it.

### A-32 — `levelsBelow` claims a refinement invariance its own arithmetic does not have (C)

```ts
export function levelsBelow(opinion: number, steps: number): number[] {
  const out: number[] = [];
  for (let step = steps; step >= 1; step -= 1) {
    out.push(mul(opinion, div(step, steps, 'this level of the grid'), 'a level it would pay'));
  }
  return out;
}
```

with the docstring _"every level of a coarser grid is a level of a finer one, so refining adds
answers and moves none"_ and the parameter declared `kind: 'resolution'` with
_"change it and the answer must not move"_ (`households.demand.steps`).

Two things are false:

1. The levels are `opinion × k/steps` for `k = steps…1`. A grid of 5 and a grid of 7 share only the
   top level. The subset property holds only when one count divides the other.
2. The grid's **bottom** is `opinion / steps`, so the count of steps sets how far down the cell bids
   at all. A cell with `steps = 5` posts nothing below `0.2 × opinion`; with `steps = 10` it posts to
   `0.1 × opinion`. A session clearing anywhere below the coarse grid's floor sees a different
   quantity from this cell depending on the resolution, and a session clearing _between_ two rungs
   sees the rung above rather than the curve — `budget / rung` instead of `budget / cleared`.

The step function is a legitimate way to post a curve. The claim of invariance is what is wrong, and
Law 2 is specific that a RESOLUTION is _"tested by invariance"_ — so this is either a resolution that
has never been tested, or a SHAPE (a claim about the answer) wearing a resolution's name, whose count
must fall. `pricesOver` in `consume.ts` has the same property.

### A-33 — `lastOwn` has no period bound, and four firm reads treat a stale wage bill as this period's (B)

`world/world.ts:1204` — `lastOwn: (kind) => this.journal.lastOf(kind, party)`. It is the most recent
event of that kind for that party, from any period, ever. Several callers guard it
(`households/index.ts` and `firms/index.ts` both test `own.value.period !== view.period` before
reading a plan). Four do not, all of them reading `labour.wages`:

| reader                        | file                | what it becomes                                                                                                                                                                                                            |
| ----------------------------- | ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `wagesDue`                    | `firms/decide.ts`   | the payroll `sellSchedule` force-sells stock to cover                                                                                                                                                                      |
| `wagesPromised`               | `firms/index.ts`    | the payroll published to lenders in `firms.funding`                                                                                                                                                                        |
| `hoursUnderContract`          | `firms/decide.ts`   | the hours the production plan is built on                                                                                                                                                                                  |
| `wageFacing`                  | `firms/decide.ts`   | the wage a unit is costed at                                                                                                                                                                                               |
| `quotedRate`                  | `firms/invest.ts`   | the cost of debt in the cost of capital — _"AT THE MARGIN, NOW … not the average coupon on debt already outstanding"_, read from `lastOwn('credit.quoted')` with no period test, so it is the last quote the firm ever got |
| `lastWageBill`, `wageItFaces` | `treasury/index.ts` | the state's own payroll in its funding programme, and what it bids for an hour                                                                                                                                             |
| `linesCovered`                | `banks/staff.ts`    | how many lines a dealing desk can quote (A-60)                                                                                                                                                                             |

`payWages` writes the event **only for employers that have rows this period**
(`for (const [employer, bill] of bills)`, and `bills` is keyed off `allRows(book)`). So a firm that
sheds its last worker stops generating the event and every one of these four reads freezes at its
last employed period, permanently.

The consequences are not symmetric noise:

- It force-sells at `price: 'market'` every period to cover a payroll of nobody
  (`short = wagesDue − cash`), which is XI-2's forced seller firing on a phantom obligation.
- It publishes that phantom payroll in `firms.funding` as _"what it is about to have to pay"_, which
  is what a bank lends against (`Banks Lending C2`).
- It plans production on hours it no longer employs, and `produce.ts:start` then finds
  `productiveHours` is 0 (that one IS period-filtered) and records `firms.idle` — so the plan and
  the line disagree about the same firm in the same period.

Each docstring states the period it means (_"what it had under contract at the close of the period
before"_, _"the payroll it has already promised"_) and none of them asks for it. The fix is one
predicate, and the reason it matters is that `lastOwn`'s own contract — _"the most recent event of a
kind THIS party is a subject of"_ — makes no promise about when.

### A-34 — a firm with no wage history bids for inputs as if labour were free (B)

`firms/decide.ts`, the input bid:

```ts
price: div(
  sub(sub(
      sub(mul(price.value, tech.yieldRate, …), mul(tech.hoursPerUnit, wage.some ? wage.value : 0, 'its wages'), 'less wages'),
      capitalCharge, …),
    sum(…other inputs…).value, …),
  input.qtyPerUnit, 'what a unit of the input is worth'),
```

`wage.some ? wage.value : 0`. The same function handles the missing wage correctly twice — `unitCost`
is `none()` without a wage, and `worthMaking` switches from `perHour > wage.value` to `perHour > 0`
— and then treats it as zero here, in the one place that reaches a market.

What an input is worth to a firm is the output it makes possible less everything else that unit
needs, and labour is part of everything else. Pricing it at zero makes the bid too high by
`hoursPerUnit × wage`, which for most recipes is the largest term. Every firm is in this state until
it has employed somebody, so at world open every input market clears against a book of systematically
inflated bids, and the firms that have never hired outbid the ones that have.

`wageFacing` already has the fallback the bid should use — the published going rate
(`labour.goingRate`) — and returns `none()` only when neither exists. A firm that knows neither its
own wage nor the market's cannot price an input, and that is a `Missing`, not a zero.

### A-35 — `buying` is computed twice, two ways (C)

`firms/decide.ts:spendable`:

```ts
.map((o) => (typeof o.price === 'number' ? mul(o.price, o.qty, 'what it is about to buy') : 0))
```

`firms/index.ts:publishFunding`:

```ts
.map((o: PlannedOrder) => (typeof o.price === 'number' ? o.price * o.qty : 0))
```

Same filter, same sum, same meaning — what this firm's buy orders commit — computed in two places
from the same list. Law 4 asks for one writer of one fact; this is one fact with two implementations
that already differ in discipline (`mul`, which names the product and checks it, against a bare
`*`). Both ternaries are dead: the preceding `.filter` has already removed every `'market'` price.

### A-36 — the treasury's immortality is unconditional where the kernel says it is conditional (C)

`registry/profiles.ts:119`:

```ts
{ id: TREASURY, representation: 'named', moneyIssuer: null, fails: [], borrows: true, depositClass: null, sovereign: true },
```

`world/failure.ts`'s header says what this is meant to be: _"XI-3's two exceptions — the central
bank, and **a treasury in its own money** — are named consequences of what they ARE rather than
omissions."_ And Appendix B lists _"sovereign in foreign money"_ among the things that must not be
immortal.

`fails: []` carries no condition. `failedWhy` asks `partyKind(kind).fails` and nothing else — there
is no currency in the question — so a treasury cannot fail on cash or on solvency in **any** money.

Unreachable today: `mechanisms/treasury/index.ts` derives its currency from
`registry.currencyOf(region)` at every issuance site, so no treasury in this world has ever issued
paper in a money it does not print. The declaration is what is wrong, not the behaviour: it states
an unconditional exception where the spec grants a conditional one, and the condition is exactly the
prohibition. `failedWhy` already has `ccy` in scope (`const ccy = ctx.registry.currencyOf(...)`) and
already discards it on the solvency branch.

### A-37 — selling an asset counts as income, and the comment above the code says it must not (A)

`mechanisms/expectations/index.ts:observations`:

```ts
// Households B3, B3.a: what a party was PAID, which is not the same as what reached it. A claim
// handed back to WHOEVER PROMISED IT is capital returning — a bill that matured, a fund share
// redeemed (Fund Shares C2) — and a party that counted that as income would think itself richer
// every time it spent its own savings. A SALE TO SOMEBODY ELSE IS A TRADE AND IS NOT THIS.
const returned = new Set<PartyId>(
  r.instruction.legs.filter(isAssetLeg)
    .filter((leg) => issuedBy(ctx.instruments.get(leg.instrument), leg.to))
    .map((leg) => leg.from),
);
for (const leg of r.instruction.legs) {
  if (isMoneyLeg(leg)) {
    if (returned.has(leg.to.holder)) continue;
    …income.set(leg.to.holder, …)
```

`returned` catches only the leg whose asset went **to its own issuer** — a maturity, a fund
redemption. A sale into a market sends the asset to another holder, `returned` stays empty, and the
seller's money leg is booked as income. The last sentence of the comment is the case the code does
not handle.

`income` is not a display number. It is the outlook three separate decisions are taken on:

- `households/consume.ts:spendPerMember` — `wanted = income.expected + gap`, so a household that
  liquidated its portfolio last week spends as if the proceeds were a wage that will recur.
- `households/consume.ts` again — `buffer = bufferPeriods × (expected + confidence)`, so the same
  sale raises the cushion it wants, which partly hides the first effect and makes the net direction
  depend on `patience` and `bufferPeriods` rather than on anything economic.
- `labour/matching.ts:reservation` — `div(outlook.expected, hours)` is _"the least a cell will work
  for"_. **A household that sold its shares raises its reservation wage and stops offering hours.**

This compounds directly with A-28: `shareOrders` puts a cell's ENTIRE holding on offer in every
period it has no spare cash, so households liquidate routinely, and every liquidation reads as a pay
rise. Estate distributions from probate are money legs too, so an inheritance is income by the same
route.

The fact needed to fix it is already in the loop: an asset leg with `leg.from === <this party>` in
the same instruction says the money is proceeds of a sale, whoever the asset went to. `returned` asks
about the counterparty when the question is about the direction.

### A-38 — a cell's outside option is every kind of money it received (B)

Downstream of A-37 and separable from it. `labour/matching.ts`:

```ts
function reservation(ctx, cell, hours) {
  const outlook = ctx.participant(cell).outlook('income');
  if (!outlook.some) return undefined;
  return div(outlook.value.expected, hours, 'reservation wage');
}
```

with the docstring _"Its outside option is what it lives on without the job — read from its own
outlook of its own income, which for somebody not working is the benefit this world pays it."_

For somebody not working, `income` is the benefit **plus** every coupon on the paper it holds, plus
every distribution it received from probate, plus (per A-37) anything it sold. B1.a's outside option
is the income it has _instead of_ working; this is the income it has _including_ working, for an
employed cell, and _including its capital_ for any cell.

So the wage a cell demands rises with the paper it owns, one-for-one, with no reason behind it: a
saver is not less willing to work because a bill paid a coupon. And an employed cell's reservation
is its own current wage divided by its hours, so an employed cell can never be matched below what it
already earns — a wage can go up and never down, which is a downward rigidity nobody declared
(Appendix B forbids stated wage rules; this is one arriving through a read).

### A-39 — the wage bill capitalised into inventory is bigger than the wage that was paid, every period, for every row (A)

**This is the clearest conservation break found so far: value enters the world with no counterparty.**

`labour/matching.ts:payFrom` rounds a wage down to real money before it moves it, and says so:

```ts
// Law 8, E1: a wage is paid in whole pieces of the money, to each worker separately … What the
// fraction below one would have been is not paid, because there is no such coin.
const share = shareFor(ctx.registry, to, currencyUnit(ccy), perMember);
if (share.total <= 0) return true;
…
amount: share.total,          // = downTick(perMember) × weight
```

`payWages`, its only caller, then records the **unrounded** number as what was paid:

```ts
const settled = payFrom(ctx, row.employer, row.worker, perMember, `wages from ${row.employer}`);
const total = mul(perMember, row.headcount, 'wage bill');     // raw perMember, NOT share.perMember
bills.set(row.employer, { …
  paid: add(bill.paid, settled ? total : 0, 'wages paid'), … });
```

`produce.ts` capitalises that number into the batch:

```ts
const wages = wagesThisPeriod(ctx, firm);      // reads bill.paid
const costs: number[] = [wages];
…
legs.push({ kind: 'create', party: firm, instrument: wip, qty: batch,
  costPerUnit: div(cost.value, batch, 'what a unit on the line has cost') });
```

Trace the firm's equity over the two instructions:

| event             | firm equity                                                             |
| ----------------- | ----------------------------------------------------------------------- |
| the wage leg      | `− downTick(perMember) × headcount`                                     |
| the batch created | `− Σ inputCarrying + (wages + Σ inputCost)` = `+ perMember × headcount` |
| **net**           | **`+ (perMember − downTick(perMember)) × headcount`**                   |

and the household side rises by `downTick(perMember) × headcount`, so the world's equity rises by
`(perMember − downTick(perMember)) × headcount` and nothing anywhere falls. `wagePerMember` is
`wagePerHour × hoursPerMember` and `wagePerHour` is `marginalBid(outcome)` — an arbitrary real
number off the book, not a grid amount — so the fraction is non-zero on essentially every row.

It is not dust. The seed opens with `seed.households.membersPerCohort = 15,000,000`, so a headcount
is in the millions; half a piece times two million members is ten thousand currency units of equity,
per firm, per period, created out of nothing and compounding through the inventory it is capitalised
into.

Nothing catches it:

- `firms.productionCosts` compares the production instructions' equity effects against
  `firms.started`'s `wages` field. Both are `bill.paid`. The two records it thinks it is comparing
  are one number (this is the A-10 / A-14 shape again).
- the `accounts` family is satisfied — the firm's assets went up by exactly what its equity did,
  because the WIP carries the invented cost.
- `payFrom` returns `true` when `share.total <= 0`, so a per-member wage under one piece is recorded
  as fully paid with **no money leg at all**, and the whole of it is capitalised.

The fix is one word: `payFrom` already computes `share.total`, the amount that actually moved, and
has to return it instead of a boolean. Everything downstream — `bill.paid`, `firms.started.wages`,
the WIP cost — is then the money that changed hands.

### A-40 — the labour print publishes a volume nobody was hired for (B)

`runVenue` clears the book with the kernel solver, and then `match` **re-derives** who is hired
rather than reading the solver's sell-side fills:

```ts
const queue = offers
  .filter((o) => o.price !== 'market' && o.price <= struck)
  .sort((a, b) =>
    a.price === b.price ? (a.party < b.party ? -1 : 1) : Number(a.price) - Number(b.price),
  )
  .map((o) => o.party);
```

The solver's `proRata` allocation across sellers is computed and discarded; the module substitutes a
price-then-party-id priority order. The reason given is sound (_"people are whole (A4.b)"_), but the
two do not agree on the total:

```ts
let people = Math.floor(div(f.qty, p.hoursPerMember, 'people hired'));
```

Every buy fill loses its sub-person remainder to `Math.floor`, and the `while` stops when the queue
runs out. So the hours actually hired are strictly less than `outcome.volume` in general — and the
event published as the occupation's print says:

```ts
ctx.record('labour.print', [v.id], { …, wagePerHour: struck, hours: outcome.volume, … }, true);
```

`hours` is the solver's crossing, not the employment that resulted. It is public, it is the only
volume this market publishes, and no instruction and no register entry corresponds to it. Two
records of one fact where one of them is not what happened (Law 4, Law 19). `match` knows the real
number — it hires row by row — and does not return it.

Same `Math.floor`-with-no-accumulator shape as A-18: a firm whose fill is 39 hours against a 40-hour
week hires nobody, and the 39 hours are discarded rather than carried, every period.

### A-41 — `shed` says oldest first and does newest first; a trading employer that fails to pay severance records that nothing is owed (C)

```ts
/** C3: the employer sheds hours it no longer wants, oldest row first, and pays to do it. */
const rows = [...rowsAt(book, employer, occupation, region)].sort((a, b) => b.start - a.start);
```

`b.start - a.start` is descending by start period — **newest** row first, last-in-first-out. Which
of the two is intended is a real question (LIFO is the ordinary convention and is probably what is
wanted), and whichever it is, one of the comment and the code is wrong (Law 16).

In `separate`, the severance record:

```ts
const trading = ctx.parties.get(row.employer).status.alive;
const paid = trading && payFrom(ctx, row.employer, gone, perMember, `severance from ${row.employer}`);
…
severancePaid: paid,
// What is owed and has nowhere to rank, per member, said out loud rather than dropped.
severanceRanking: trading ? 0 : perMember,
```

A **trading** employer whose severance payment fails records `severancePaid: false` and
`severanceRanking: 0` — "nothing owed". It is owed, it was not paid, and it has nowhere to rank
either: there is no trade-payable instrument yet (the comment says so for the dead-employer case).
The field is right for the dead employer and wrong for the live one that could not pay, which is the
case the whole `payFrom`-returns-a-boolean design makes easy to miss.

More generally: a failed wage or severance leaves no obligation anywhere. `world/failure.ts:stillOwed`
only counts failures whose `instruction.period === view.period`, so last period's unpaid wage is
gone from the solvency test too. The worker worked, was not paid, and there is no liability on
anybody's book — which is a flow with one side that settlement never saw (Law 5, and Appendix B's
"no income without cash received" holds only because the claim vanished with it).

### A-42 — two `units` families switch themselves off in any period with a weight event, which is every period (A)

`mechanisms/goods/index.ts:unitsIdentity`:

```ts
// A weight event moves stock between books without an instruction (a cell splits, a member
// dies): the holders' totals are re-struck, and this period's identity is not about them.
const weights = view.journal.ofKindIn('weight', view.period).length;
const consecutive = seen.period !== undefined && view.period === seen.period + 1;
…
  if (!consecutive || before === undefined || weights > 0) continue;
```

`mechanisms/capital-programme/index.ts:461`, in the same words:

```ts
const weights = view.journal.ofKindIn('weight', view.period).length;
…
const comparable = consecutive && weights === 0;
for (const [k, before] of comparable ? seen.held : new Map<string, number>()) { … }
```

`weights` counts **every weight event in the world**, not the ones touching the instrument being
checked. `households/lifecycle.ts:age()` runs `ctx.cells.reKey(...)` — which journals a `'weight'`
event — for every household cell with enough members to move one person, every period. So from the
first period in which anybody ages, `weights > 0` holds permanently and **neither family reports
anything again for the rest of the run**. Both declare `built: true`, so the audit shows them green.

What is lost is the fine half of Part XII's stock identity. The kernel's own `units` family
(`audit/families/units.ts`) survives, but it compares each instrument's `issued` total against the
create/destroy legs. These two compare the **holdings** — `heldTotal` per instrument, and quantity
per (holder, instrument) — which is where stock that moved between books with no leg behind it would
show. That is the case the goods docstring says it exists for: _"so a stock that moved without a leg
has nowhere to hide."_

The exemption is also unnecessary, which is what makes it worth deleting rather than narrowing. Of
the three ways a weight can move today:

- `splitCell` / `reKeyCell` copy per-member state and split the weight, so
  `perMember × (w−m) + perMember × m = perMember × w`. The total does not move.
- `dieCell` refuses a cell that still holds anything, so nothing physical is on it to move.
- `weightEvent` (entry, death, promotion) would move a total — and per A-17 it is called by nobody.

So the guard protects against a case that cannot occur, and pays for it with the two families it
was attached to.

### A-43 — the labour module builds the household's schedule, through the door the architecture says not to (C)

`MechanismContext.gather`'s contract (`world/context.ts:508`):

> _"ask every party whose module declared a schedule for this venue for one, and post what comes
> back. The module that OPENED the venue calls it … each schedule is built by the module that owns
> that party, with that party's own view. … **building somebody else's schedule inside the clearing
> phase instead is that module deciding for a party it does not own.**"_

`labour/matching.ts:supply` does exactly that: it walks `ctx.parties.ofKind(HOUSEHOLD)`, reads each
cell's own `outlook('income')` through `ctx.participant(cell)`, decides what that cell will work
for, and posts the order itself. The households module declares no `venueParticipants` and labour
never calls `gather`.

The door is not theoretical — two modules use it (`housing/index.ts:711` and `banks/index.ts:1165`
declare `venueParticipants`; `money-market/index.ts:269` calls `gather`). Labour is the one venue
where the seller's decision is taken by the buyer's market.

It is C rather than B because the number `supply` computes is a household's own read and no private
state leaks. What it costs is where the decision lives: a household's reservation wage is
`households`' subject (it is the same `income` outlook `spendPerMember` uses), and it currently
cannot be changed without editing the labour module. A-38's defect — the outside option being every
kind of money received — is in `labour/matching.ts` for exactly this reason.

### A-44 — the household's liquidity premium is declared "over what a deposit returns" and is used as an absolute rate (B)

Declared in `households/index.ts`:

```ts
{ id: HOUSEHOLD_PARAMS.liquidityPremium, value: 0.005,
  unit: 'per annum OVER WHAT A DEPOSIT RETURNS',
  dimension: 'perAnnum', kind: 'preference', owner: 'model',
  why: 'Households D5, D5.a: what a saver wants for giving up instant access to its money. It is
        the whole of the substitution between a deposit and paper held directly …' }
```

Used in `households/index.ts:decide`:

```ts
const required = view.params.perAnnum(HOUSEHOLD_PARAMS.liquidityPremium);
```

and in `portfolio.ts:fundOrders`:

```ts
if (toFund <= 0 || p.offered < required) continue;
```

`required` is the bare 0.005. The deposit rate is never added to it, never read, and does not appear
anywhere in the households module's saving decision.

This was harmless when it was written and is not now. `portfolio.ts`'s own header still says so:

> _"A deposit is a holding of a bank's money — **it returns nothing at all here**, because paying
> for deposits is a decision a bank has not been given yet (Banks Funding B1, worklist 11)"_

Deposits now pay. `mechanisms/money-market/deposits.ts:payDepositInterest` settles a real money leg
every period for every holder of a bank's money whose kind has a deposit class, at the rate that
bank published on its board — and `banks/index.ts:1102` calls `setBoard` for every bank, every
currency, every period. So worklist 11 landed and the household's comparison did not move with it.

The consequence is a wrong substitution, not a cosmetic one: a household will subscribe to a money
fund offering 0.006 while its own bank's board pays 0.02, because `p.offered < required` tests
0.006 against 0.005. D5.a's substitution — _"the choice between a deposit and paper bought directly
… is how a rate reaches a saver"_ — currently runs against a constant instead of against the rate
the saver is actually being paid. The board is reachable: `households/bank.ts:board()` in the same
module already reads `bank.depositRate` for this cell's own bank and class.

Same shape, same file: `savingLines`' price and `fundOrders`' `required` are the only two things a
household weighs a claim against, and neither of them knows the deposit pays anything.

### A-45 — a bank that owes nothing has a cost of funds of zero (C)

`banks/index.ts:costOfFunds`:

```ts
const blend = (interest: number): FundingCost => ({
  perAnnum: funding <= 0 ? 0 : div(add(interest, onCapital, …), funding, 'per annum'), …});
if (funding <= 0 || ctx.period === 0) return blend(0);
…
if (year <= 0) return blend(0);
```

Three paths return a cost of funds of exactly **zero**: a bank funded by nothing, the first period,
and a zero-length year. Zero is not "unknown" here — it is "money is free", and it flows straight
into a price. `quote()` builds the lending rate on it and `dealing.ts` builds the desk's edge on it,
so in period 0 every bank in the world quotes as if its funding cost nothing.

The neighbouring `atLeast(capital, 0, 'a hole funds nothing: there is no less capital than none')`
is a different case and its argument holds (a negative residual is not a source anybody requires a
return on) — though it is worth noting the comment records exactly the Law 6 pattern: a number went
negative, a quote crossed itself, and a floor was added at the read rather than at the cause.

`FundingCost` could say `Missing` for a bank it cannot cost, and a bank that cannot cost its funding
has no business quoting a rate — which is what the surrounding code does everywhere else (`quote()`
returns `undefined`, `costOfCapital` returns an `Option`).

### A-46 — income tax is levied on the return of a household's own capital (A)

`treasury/index.ts:runReceipts`:

```ts
if (leg.to.holder === id) continue;
if (r.instruction.cause === 'coupon') {
  bases.interest = add(bases.interest, leg.amount, 'interest received');
  addTo(due, leg.to.holder, mul(leg.amount, onInterest, 'tax on interest'));
} else if (cells.has(leg.to.holder) && leg.from.holder !== id) {
  bases.income = add(bases.income, leg.amount, 'what households were paid');
  addTo(due, leg.to.holder, mul(leg.amount, onIncome, 'income tax'));
}
```

The `else if` is _every other money leg reaching a household cell_. It therefore taxes, at the
income rate, on the **gross** amount:

| what happens                         | the cause on the instruction             | what the household is billed              |
| ------------------------------------ | ---------------------------------------- | ----------------------------------------- |
| a sovereign bill it holds matures    | `'maturity'` (`world/actions.ts:264`)    | income tax on the whole **principal**     |
| it redeems a money-fund share        | `'maturity'` (`funds/index.ts:487`)      | income tax on the whole **redemption**    |
| it sells shares or paper in a market | `'trade'` (`clearing/market.ts:726`)     | income tax on the whole **sale proceeds** |
| it receives its share of an estate   | `'transfer'` (`households/lifecycle.ts`) | income tax on the whole **inheritance**   |
| an ETF pays it out                   | `'maturity'` (`funds/etf.ts:248`)        | income tax on the whole **payout**        |

Only the first row is a base anybody would call income, and only the interest on it. The seed
endows every household member with sovereign paper (`foundation.ts:1312`), so this fires from the
first maturity onward, in real money, out of real accounts — and a household that cannot pay it is
a cash failure it did not owe.

The engine already contains the correct discrimination, twenty lines of one file away.
`expectations/index.ts:observations` builds the same base and excludes exactly this:

```ts
// Households B3, B3.a: what a party was PAID, which is not the same as what reached it. A claim
// handed back to WHOEVER PROMISED IT is capital returning — a bill that matured, a fund share
// redeemed (Fund Shares C2) …
const returned = new Set<PartyId>(
  r.instruction.legs
    .filter(isAssetLeg)
    .filter((leg) => issuedBy(ctx.instruments.get(leg.instrument), leg.to))
    .map((leg) => leg.from),
);
```

So the world holds two definitions of "what a household was paid", one in the module that forms the
household's outlook and one in the module that taxes it, and they disagree about the whole of a
balance sheet's turnover. That is Law 4 with a cash consequence: `bases.income` is published in
`treasury.receipts` as the state's income-tax base, and it is mostly asset turnover.

(The comment directly above this loop is careful about the neighbouring cases — _"A base carries one
rate: the state does not tax back the transfer it just paid, and interest is taxed where it is
received rather than twice over as income as well"_ — which is what makes the omission look like an
oversight rather than a decision.)

Note the same file gets the currency question exactly right, with a worked example of what it cost
to get wrong (`if (leg.ccy !== ccy) continue;`, 13e). The base question has not had that pass.

### A-47 — a fund's mandate has no currency in it, so it bids its own money at a foreign price and its NAV adds two moneys (A)

Three places in `mechanisms/funds/`, one omission:

**1. The mandate.** `eligible(view, d, i)` (`index.ts:1120`) admits a line on three tests — live,
kind in `d.eligible`, and maturing inside `maxTenorPeriods`. There is no currency test.
`d.eligible` for every money fund is `['sovereign.bill']` (`data.ts:drawFunds`), which is **every
sovereign bill in the world**: `eligibleLines` counts the Japanese and European ones alongside the
American, and `ordersOf` will bid in any of their markets.

**2. The bid.** `ordersOf`:

```ts
const each = div(spare, lines, 'what it puts into each line it may hold');
const dirty = add(price, view.accrued(i.id), 'what a unit costs it');
const qty = downTick(div(each, dirty, 'units it bids for'));
```

`spare` comes off this fund's own `fund.struck` event and is in the fund's own money; `price` is
`priceAt(flows, required, …)` in the line's money. `each / dirty` divides one currency by another
and calls the answer a quantity. The fund also does not hold the foreign money it would have to pay
with — `strike` reads its cash as `ctx.register.quantity(fund, moneyOf(ctx, fund, ccy))` in its own
currency only.

**3. The NAV.** `nav.ts:navOf`:

```ts
for (const h of reads.holdingsOf(fund)) { … assets.push(worth.value.value); }
for (const other of reads.instruments()) { … owed.push(worth.value.value); }
const net = sub(sum(assets).value, sum(owed).value, 'what the fund is worth');
return { perShare: div(net, shares, 'net asset value per share'), … };
```

`worthOf` answers in the instrument's own money. Nothing converts. So a fund holding one foreign
bill publishes a NAV that is a sum of two currencies, and every subscription and redemption in the
world transacts at it (`strike` → `subscribe`/`redeem` both take `perShare`).

`holdingsWorth` (the base the forced pro-rata sale is struck on) does the same thing.

This also breaks the module's own audit family. `equityIsZero` works because the share liability is
`issued × navOf(...).perShare = assets − other liabilities` **by construction**, so equity is
identically zero — as long as both sides are computed the same way. `balanceSheet`, which the
`accounts` family uses, converts each holding into the party's own money (Currency D2 —
`world/assemble.ts:stateEquityAsRead` documents that conversion and the bug that came from omitting
it). `navOf` does not. A fund with any foreign holding therefore has a NAV and a balance sheet that
disagree by the FX difference, and `equityIsZero` reports a fund that has "mislaid somebody's money"
when what happened is that two readers of one book used two currencies.

The engine has the read: `ctx.valuation.inMoney` / `rateInForce`, which is what `revalue.ts` uses.

### A-48 — `equityIsZero` cannot fail for the reason it says it checks (C)

Separable from A-47, and worth stating because the family's docstring claims the opposite:

> _"A3: a fund's equity is ZERO. … **Nothing in the module enforces it: it falls out of the wire**,
> and this is the check that says whether the wire actually did it."_

It does not fall out of the wire. `fundShareKind.owes: 'value'` and
`derive: (i, at, reads) => navOf(i, at, reads).perShare`, and `navOf` returns
`(assets − other liabilities) / shares`. So the fund's liability is _defined_ as its assets net of
its other liabilities, and `assets − liabilities = 0` is an algebraic identity, not an outcome. No
subscription, redemption, fee, mark or trade can move it — which is exactly the property the module
elsewhere states out loud (`fundKind`: _"its equity is zero by construction (A3)"_).

What the family can still catch is a **disagreement between two valuation paths** — the FX one in
A-47, a liability `navOf` skipped because `worthOf` was `none` while `balanceSheet` counted it, or a
`div`/`mul` residue. That is worth having. It is not what the docstring says it is having, and a
reader who trusts the docstring believes the wire is being checked when the identity is being
restated (Audit A1.a; the same shape as A-10 and A-14).

### A-49 — `offeredYield` takes the last curve family, not the best, and answers zero when there is none (C)

```ts
let best = 0;
for (const family of ctx.registry.curveFamilies.values()) {
  if (family.ccy !== ctx.registry.currencyOf(region.id)) continue;
  const read = ctx.curve(family.id).at(yearFraction(family.dayCount, on, by));
  if (read.yield.some) best = read.yield.value;
}
return sub(best, fee, 'what a saver gets after the manager');
```

The variable is `best` and the loop is an assignment, not a maximum: with two issuers' curve
families in one currency it returns whichever the registry's `Map` iterates last. And `best = 0`
means a fund with no readable curve publishes `offered = −fee`, a negative return, as a fact rather
than as "no answer" — which is what the module's own `returned` field is careful about two
declarations away (_"A RETURN ON NOTHING IS NOT A NUMBER (Appendix A) … Zero would have been worse
than the throw"_).

It also sits behind A-27. The commodity fund has `maxTenorPeriods: 0`, so `by === on`,
`yearFraction === 0`, and `offeredYield` reads the very short end of the **sovereign bill** curve —
publishing a money-market yield as what a grain fund offers, which is the number a household then
compares against its liquidity premium before putting its cash cushion into grain.

### A-50 — `moneyOf` hand-builds an account id, which is the one thing `accountOf` exists to prevent (C)

```ts
function moneyOf(ctx: MechanismContext, party: PartyId, ccy: string): InstrumentId {
  const bank = ctx.parties.get(party).bank;
  return instrumentId(`money:${bank}:${ccy}`);
}
```

`MechanismContext.accountOf`'s contract says why this is not allowed:

> _"WHICH account a party holds a given money in. Its own money is at its own bank; a money its
> bank does not issue is held at that money's own central bank … One writer of that rule, the
> kernel's, so **a module never assembles an account out of a party's `bank` field**: a module that
> did would be right in one currency and wrong in every other."_

It also rebuilds the id string by hand rather than calling `moneyInstrumentId` (`core/ids.ts:124`),
so it is a second spelling of the same format too. It is the only such site in the engine
(`grep 'instrumentId(\`money:'` finds this one).

Used twice, both in `strike` and `redeem`, to read the fund's cash before deciding what it can pay —
while the leg that pays it goes through `ctx.accountOf(fund.id, ccy)`. Today both resolve to the
same account because a money fund's currency is its own region's. Under A-47 they diverge: the fund
reads zero at its own bank for a foreign balance held at that money's central bank, and refuses a
redemption it can afford.

### A-51 — the kernel converts currencies and five module reads of the same books do not (A)

`audit/families/accounts.ts:balanceSheet` is the one read that gets this right, and its comment says
why it has to:

```ts
// Currency C4, C5, D2: A POSITION IN ANOTHER MONEY IS AN ASSET LIKE ANY OTHER, converted at the
// rate in force — the same rate the same period settled at, so what a balance sheet says and
// what a payment does cannot disagree.
assetTerms.push(
  view.valuation.inMoney(
    view.valuation.valueOfLots(inst.id, h.lots, view.period),
    inst.ccy,
    home,
    view.period,
  ),
);
```

Every term on both sides goes through `inMoney`. `world/assemble.ts:stateEquityAsRead` carries the
same lesson written out as a defect that was found and fixed: _"it added `valueOfLots` across every
holding in whatever money the instrument was priced in, where the read it is checked against converts
each one into the party's own (Currency D2). At a rate of one the two agreed and nothing showed."_

`inMoney` is exported on `MechanismContext.valuation` and **is called by no module in the engine**
(`grep -rn 'inMoney' mechanisms/` finds one unrelated local variable in `banks/dealing-quote.ts`).
Every module that walks a party's book sums it in whatever money each line happens to be in:

| site                                       | what the sum is used for                                 | reachable today?                                       |
| ------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------------ |
| `funds/nav.ts:navOf`                       | the NAV every subscription and redemption transacts at   | **yes** — A-47: a fund's mandate has no currency in it |
| `funds/index.ts:holdingsWorth`             | the pro-rata base a forced sale is struck on             | **yes**, same reason                                   |
| `banks/capital.ts:capitalOf`               | the bank's capital position and its risk-weighted assets | only if a bank holds foreign paper                     |
| `money-market/resolution.ts:valueBook`     | the hole in a failing bank, which decides who bears it   | only if a failing bank held foreign paper              |
| `households/consume.ts:wealthOf`, `atRisk` | what a household spends (A-23)                           | guarded upstream today                                 |
| `equity/index.ts:531`                      | the opening share count of every listed firm             | seed-time, single-currency by construction             |

The two `funds` rows are live now. The rest are one holding away, and the failure mode is the one
`stateEquityAsRead` names: at a rate of one everything agrees and nothing shows, so the defect
arrives with the first non-unit rate rather than with the first foreign holding.

What makes it a single finding rather than six: the conversion is not hard, the door exists, the
kernel already uses it, and no module does. That is a habit rather than six oversights, and the
place to close it is at the reads that walk `holdingsOf`.

### A-52 — an audit family recovers a fact by parsing the human-readable reason string (C)

`equity/index.ts:dividendLegs`:

```ts
const reason = r.instruction.reason;
if (!reason.startsWith('payout on ')) continue;
// The reason names the line it is a payout ON, which is what the declaration named.
const line = reason.slice('payout on '.length, reason.indexOf(' ', 'payout on '.length));
```

`Instruction.reason` is declared as prose — _"C1.b: a human-readable reason, so a unit is traceable
to why it moved."_ This is the practice the register's own `EquityMove` docstring names as the
defect it was built to remove:

> _"only the running balance survived, so … a report that wanted 'revenue' had to parse the reason
> strings on money legs — **recovering by inference a fact its writer knew and did not record**."_

Two modules now depend on the exact wording, in two files, with no shared constant and nothing the
compiler can check:

- `equity/index.ts:payDividend` — `reason: \`payout on ${line.id} to ${holderId}\``
- `funds/index.ts:distribute` — `reason: \`payout on ${share} to ${holder}\``

Change either string and `declaredIsPaid` reports every payout in the world as declared-and-unpaid
(`moved` becomes 0 against a non-zero `paid`) — loudly, at least, but for a reason that has nothing
to do with the flow it is checking. Change the wording so the second token is no longer the line id
and it reports false violations instead, silently mis-keyed.

The fact is already in the instruction as data: the money legs carry `from.holder` and the payout is
identified by the line it is on, which `payout.declared` already publishes in its `line` field. What
is missing is a link from the instruction back to the declaration — the `Settled` record's own
`instruction.id`, recorded alongside the declaration, would make the family a read rather than a
parse (`EquityEntry.instruction` exists for precisely this).

Minor, same file: `if (!material(total, 2, total) || total <= 0) continue;` in both `payDividend`
and `distribute` — `material(x, 2, x)` compares a value against its own magnitude and is true for
every non-zero finite `x`, so the test reduces to `total > 0`.

### A-53 — a household bids its entire income as rent, and its spending plan does not know rent exists (B)

`housing/index.ts:reservation`:

```ts
const income = view.outlook('income');
if (!income.some || income.value.expected <= 0) return undefined;
const per = view.registry.pieces(
  goodUnitOf(view, self.region),
  view.params.ratio(HOUSING_PARAMS.perMember(keyOf(self, 'cohort'))),
);
// Law 8: money pieces a member expects, over the PIECES of occupancy a member lives under
return div(income.value.expected, per, 'what a member would pay for the roof it lives under');
```

`bid × per = income.expected`. The whole of it. `ordersOf` posts that as a **single point**, and
`letIn` clears on `marginalBid`, so in any region where dwellings are short the print rises to the
bids and rent takes a household's entire expected income, every period, indefinitely.

The docstring argues the bid: _"THE MOST A TENANT WILL PAY is what it has, because the alternative
is nowhere to live."_ That is a fair reading of B1.a read from the other side. Two things do not
follow from it:

1. **Nothing on the household's side knows.** `households/consume.ts:spendPerMember` computes
   `wanted = income.expected + gap` and `demandOf` spreads the whole of it over the basket. There is
   no rent term anywhere in the households module. So the same expected income is committed twice —
   once as the rent bid, once as the grocery budget — and the only thing that stops it being an
   arithmetic contradiction is that `collect` runs before the goods session and drains the account
   first. The household is then structurally surprised every period, which feeds back through
   `confidence` and widens its cushion, which it cannot fund either.
2. **It is a point where every other bid in this world is a schedule.** Goods, paper and shares all
   post a curve (`rungsOver`, `rungsUpTo`, `levelsBelow`) precisely because Clearing A2 asks for one.
   A tenant posting one level at its whole income cannot express "I would take a smaller place for
   less", which is the substitution the venue exists to find.

The wear floor on the other side (`wearOf` = spoilage × print) is deliberate and stated, and it is
the honest half: the range the rent clears in has a real bottom and a top that is a household's
entire livelihood.

Same shape as A-40 in the same file: `letIn` discards the solver's `proRata` sell-side allocation
and re-matches owners and tenants itself in ask/bid order. Here the two totals do agree (the loops
consume exactly `outcome.volume` on both sides), so it is a duplication rather than a divergence.

### A-54 — `gather` is called by exactly one module, so two other modules' venue schedules are never asked for (A)

`World.gather` (`world/world.ts:940`) is the **only** path that runs `venueParticipantDecls`:

```ts
gather(venue: VenueId, owner: string): void {
  const decl = this.venue(venue);
  forbid(decl.clearedBy === owner, 'Law 4', …);
  if (this.gathered.has(venue)) return;
  this.gathered.add(venue);
  for (const p of this.venueParticipantDecls) {
    for (const party of this.parties.ofKind(p.partyKind)) { …this.post(venue, o); }
  }
}
```

Across the whole engine, `ctx.gather(...)` appears once: `money-market/index.ts:269`. Three modules
declare `venueParticipants`:

| module                            | what the schedule is                      | gathered?                |
| --------------------------------- | ----------------------------------------- | ------------------------ |
| `banks` — `sessionOrders`         | a bank's money-market schedule            | **yes**, by money-market |
| `banks` — `staffOrders`           | a bank's bid for labour hours             | **never**                |
| `housing` — household rent orders | every bid and offer in the lettings venue | **never**                |

Consequences, both structural:

**1. The lettings venue is empty every period, for the life of the run.** `housing.lettings` runs
`letIn`, which reads `ctx.posted(rentVenue(region))`. Nothing has posted into it, `clear([])` is not
cleared, and the phase records `RENT_PRINT` with `outcome: 'noDemand'` and returns. So no tenancy is
ever signed, `collect` has no leases, no rent is ever paid, and `housing.rent` — a whole phase — does
nothing forever. Everything in the module's header about rent clearing between the owner's wear and
the tenant's income describes a session that never has an order in it.

**2. No bank ever bids for labour.** `staffOrders` is the bank's demand for hours, and the comment
beside it describes the mechanism it is meant to give: _"A bank whose book earns nothing bids nothing
and hires nobody, which is how a shrinking bank sheds staff without anybody writing a rule for it."_
It bids nothing because it is never asked. Banks in this world employ no one, so `operatingCostOf`
has no wage bill behind it and `labour.match` never sees a financial-sector employer.

`labour` reads `ctx.posted(v.id)` directly, so a FIRM's opening reaches the book (firms
`ctx.post(venue.id, …)` in `firms.decide`) — the bank's does not, because it went the other way.
That is the same split A-43 describes from the other side: labour builds one party's schedule itself
and ignores the door, and the door is what banks used.

### A-55 — nobody in this world can buy a dwelling, so the whole of Housing B1–C4 never runs (A)

`housing/index.ts:askForMortgages` begins:

```ts
for (const cell of ctx.parties.alive()) {
  /** 13d.1: A HOUSEHOLD CAN HOLD A ROOF AND CAN BORROW FOR ONE — … All four were built and
   *  measured here … WHAT STOPS IT IS ONE MORE THING, and it is not this item's: a borrower that
   *  MISSES A PAYMENT goes on accruing on the lender's book … the item that owns it is 13f … */
  if (cell.representation === 'cell') continue;
```

Every household is a cell, so no household ever publishes a `housing.funding` request, no bank ever
writes a mortgage, `mortgagesOf` is always empty, and `charge` and `foreclose` — two more whole
phases — are dead. The docstring is candid about the blocker; what it does not say is that the
consequence is the whole buying half of the module.

The other routes were checked and all are closed:

- `dwelling` is **not in the household consumption basket** — `households/data.ts` names 18 sub-units
  (bread, meat, clothing, appliances, vehicles, care, teaching, …) and no dwelling — so `demandOf`
  never bids for one.
- **No recipe names `dwelling` as an input**: the only occurrence in `goods/data.ts` is the good's
  own declaration, so no firm buys one to make something.
- **Merchants exclude it**: `marketsOf` filters `!i.terms.portable`, and a dwelling is declared
  non-portable (_"a house cannot be somewhere other than where it was built, at any price"_).
- **Fund mandates exclude it**: `drawFunds` gives `['sovereign.bill']` and `['good.grain']`.
- **Bank dealing desks exclude it**: `BankDecl.makes` is drawn from
  `['equity.share','fund.share','sovereign.bill','sovereign.bond']`.

So the market `mkt.good.dwelling.<region>` has a seller (the builder firms — `firms/data.ts` draws a
`dwelling` line with occupation `building`) and **no bidder, ever**. Three things follow:

1. The dwelling's price is the seed's placeholder for the life of the run. `foundation.ts:1571`
   writes an opening print for every good with `provenance: {kind:'opening'}` and the comment says
   _"that number is a placeholder and the market's own first session replaces it"_. For this one
   good no session ever does — so it is a PLACEHOLDER with no scheduled death (Law 2), and it is the
   number `wearOf` and `shortOfMoney` are both built on.
2. Builders accumulate unsold dwellings and lose them to spoilage (1% a year, `goods/data.ts:1735`),
   which is the only thing that ever removes one from the world.
3. Owner-occupation is unreachable, so the module header's _"OWNER-OCCUPATION IS AN OUTCOME AND NOT A
   TENURE FLAG: a household that owns as many dwellings as its members live in has nothing to rent"_
   describes a state no household can be in.

Combined with A-54, the housing system produces houses nobody can buy and lets none of them.

### A-56 — three of this world's lines have a firm, a recipe and a market, and no buyer; they open at zero and never move (A)

Cross-referencing every declared good against every source of demand. `GOODS` is
`[...MAKES, ...RETAIL.map(shelfLine)]` — 54 written lines plus 9 generated retail lines, 63 in all.
The demand sources are: the household basket (`households/data.ts`, 18 rows, all of which resolve to
real goods), every recipe's `inputs` (43 distinct goods are drawn by something), and the goods a
capital kind is `madeFrom` (`building`, `machine`, `vehicle`, `vessel`).

Three goods are in none of them:

| good         | recipe                                          | portable? |
| ------------ | ----------------------------------------------- | --------- |
| `dwelling`   | timber, concrete, steel, glass, building trades | no        |
| `facilities` | chemicals, power, paper                         | no        |
| `itServices` | power, electronics                              | no        |

Non-portable, so `merchants/index.ts:marketsOf` (`if (… !i.terms.portable) continue`) excludes all
three; not in a fund mandate (`['sovereign.bill']`, `['good.grain']`); not in a bank's `makes`
(`['equity.share','fund.share','sovereign.bill','sovereign.bond']`). Nothing anywhere bids for them.

They are also **zero at the seed**, by the same fact. `foundation.ts:985`:

```ts
const finalGoods = [...sizeOfLine.keys()]
  .filter((g) => (drawnBy.get(g) ?? []).length === 0 && !madeInto.has(g)).sort();
…
for (const g of finalGoods) {
  let want = 1;
  if (asked > 0) want = zeroIfNone(wantedInAPeriod.get(g));
  started.set(g, div(want, recipeOf(g).yieldRate, 'started for what is wanted of it'));
}
```

All three qualify as "final goods" (nothing draws them, they are not plant), `wantedInAPeriod` is
keyed by basket sub-unit so it has no entry for any of them, and `asked > 0` because the eighteen
basket lines do have entries. So `want = 0`, `started = 0`, `plantOf(...) = 0`, and
`finished`/`onTheLine` are 0 — their firms open holding nothing, with no plant.

From there nothing can start them. `firms/decide.ts:plan` requires an outlook of its own sales:

```ts
const sales = view.outlook(`sold.${output}`);
if (!price.some || !sales.some || inputPrices.some((p) => !p.some)) return … { planned: false, … };
```

and `expectations/index.ts` forms `sold.<good>` only from an asset leg the firm was actually a side
of. A firm that has never sold has no outlook; with no outlook it makes no plan; with no plan it
starts no batch; with no batch it never sells. The three lines are closed loops at zero from period
zero, with named firms, a market, a printed opening price, and no way in or out.

`dwelling` is the one with consequences beyond itself: it takes the whole housing module with it
(A-55), and its market's seed placeholder never gets replaced, which is the number the rent floor
and every mortgage size would have been built on.

**Correction to my own working note.** An earlier pass of this analysis put nine more goods on this
list (`bread`, `meat`, `clothing`, …) on the grounds that the basket names `retailBread` and no such
good is written in `goods/data.ts`. That was wrong: `shelfLine` **generates** the nine retail lines
from the `RETAIL` table, each taking one unit of the wholesale good as its input, so the basket
resolves and those chains are live. Only the three above survive the check.

### A-57 — a securitisation vehicle keeps the whole interest stream of its pool, for ever, and nobody owns it (A)

The vehicle collects everything the borrowers pay. `LOAN.due` (`banks/loan.ts:83`) emits a `coupon`
every period and a `maturity` at the end, and the kernel's corporate-action phase pays them to the
holder of record — the vehicle. So the cash in over the life of a deal is **principal + interest**.

What goes out is principal only. `trancheKind` is a pure pass-through with nothing to pay:

```ts
cashFlows: (): readonly CashFlow[] => [],
due: () => [],
accrued: () => 0,
```

and `payTranche` redeems face at par, one unit of money for one unit of face:

```ts
const toLayer = downTick(atMost(available, face, 'a layer takes no more than its face'));
…
{ kind: 'asset', from: holder, to: deal.vehicle, instrument: id, qty: share, pricePerUnit: some(1), … },
{ kind: 'money', from: ctx.accountOf(deal.vehicle, deal.ccy), to: ctx.accountOf(holder, deal.ccy), amount: share, … },
```

So `Σ out ≤ Σ face = the pool's opening principal`, and the interest — the entire economic return of
the deal — never leaves. Three things follow, and the third disables the module's own subject:

1. **A noteholder earns nothing but its discount.** It pays `priceFor(...)` per unit of face and
   receives exactly face. Whatever the borrowers paid in interest is not its.
2. **The residual has no holder.** When the notes are fully redeemed the vehicle still holds the
   un-run-off rows and every coupon it ever collected. Nothing ceases a vehicle whose pool has run
   off (`distribute` removes a deal only when the vehicle has already _ceased_), the party kind has
   no owner, no equity claim and no distribution, and `fails: ['cash','solvency']` will not fire on a
   party with positive equity. Appendix B: _"no residual with no holder"_.
3. **The attachment machinery goes inert.** `absorb` is the whole of C2/C6/D4:
   ```ts
   const pool = sum(
     deal.rows.filter(live).map((row) => ctx.register.quantity(deal.vehicle, row)),
   ).value;
   const notes = sum(deal.layers.map((id) => ctx.register.heldTotal(id).value)).value;
   let lost = sub(notes, pool, 'what the pool no longer covers');
   if (lost <= 0) return;
   ```
   Because interest is paid out as accelerated principal, `notes` falls faster than `pool`. Once
   `notes < pool` — which happens early and permanently — `lost ≤ 0` and **no loss is ever allocated
   to any tranche**, whatever the borrowers do. The junior/senior waterfall, the attachment points,
   the write-down leg and D4's senior losses are all downstream of a subtraction that has gone the
   wrong way round.

The module's header states the intended behaviour twice — _"it takes in what the borrowers pay and
passes it out by seniority"_, _"Σ tranche face equals the pool's face after every event"_ — and both
are true only if what is passed out is separated into interest and principal. Nothing separates them.

### A-58 — the price a bank will pay for a note is a leverage ratio squared, called a cost of funds (A)

```ts
/**
 * C3, Law 3, Law 19: WHAT IT WILL PAY, per unit of face. A note is worth a discount on its face to
 * a buyer that could have lent the money itself — the discount is what its own money costs it,
 * which it reads off what it actually pays for money and never off a table.
 */
function priceFor(view: ParticipantView, ccy: CurrencyCode): number {
  const owed = view.owedIn(ccy);
  const equity = view.equity();
  if (equity <= 0 || owed <= 0) return 1;
  const cost = div(owed, add(owed, equity, 'what funds it'), 'what its own money costs it');
  return sub(1, mul(cost, cost, 'the discount it wants'), 'what it will pay per unit of face');
}
```

`owed / (owed + equity)` is the **debt share of this bank's funding** — a dimensionless ratio
between 0 and 1 — not a rate and not a cost. The variable is named `cost` and the comment says it is
_"what it actually pays for money"_. Squaring it and subtracting from one produces a price per unit
of face with:

- **no periodicity** (Law 8: a rate is not a number until its periodicity is) — the note's tenor
  appears nowhere;
- **no relation to any rate this world produces** — a bank funded 90% by deposits bids
  `1 − 0.81 = 0.19` per unit of face, an 81% discount on a senior tranche; one funded 50/50 bids
  0.75;
- **no dependence on the pool** — two vehicles with completely different loan books get the same bid
  from the same bank.

`owedIn(ccy)` is also the wrong quantity even as a leverage measure: its contract is _"what falls
due in `ccy` less what it holds of it"_ — a short-term funding gap this period, not the bank's
liabilities.

The number the docstring describes exists and is published under the bank's own name every period:
`banks/index.ts:publishCostOfFunds` writes a real per-annum blended cost (`costOfFunds` →
`FundingCost.perAnnum`), built from the coupons the bank actually paid. Discounting the note's own
cash flows at that rate — the same `priceAt(flows, required, on, dayCount, …)` the money funds use
in `funds/index.ts:ordersOf` — is what C3 asks for and is one line away.

Related, same function: `noteBids` posts a single order for the bank's entire spare cash at that one
price. Every other buyer in this world posts a schedule (Clearing A2); this one posts a point.

### A-59 — the resolution auction is decided on a price that is never paid (B)

`money-market/resolution.ts:bidFor` computes what an acquirer will pay for a failed bank's book:

```ts
const wants = mul(v.assets, required, 'what it wants for taking the book on');
const pays = sub(sub(v.assets, v.deposits, 'assets over deposits'), v.borrowings, 'and its rows');
const bid = sub(pays, wants, 'what it will pay');
```

`resolve` then uses it, twice, to choose:

```ts
const taking = bids.filter((b) => !b.declined).sort((a, b) => b.pays - a.pays);
const winner = taking[0] ?? bids.sort((a, b) => b.pays - a.pays)[0];
…
const acquirer = winner.bank;
const borne = allocate(ctx, bank, acquirer, ccy, v);
moveBook(ctx, bank, acquirer, ccy);
ctx.cease(bank, acquirer);
```

and records it (`{ failed, bidder, pays, declined, why }`). `winner.pays` appears nowhere else in the
file: **no instruction ever moves it.** The acquirer takes the book, the guarantee pays it the hole
(`payFrom(insurer → acquirer)`, then `payFrom(treasury → acquirer)`), and the consideration it bid —
which is what `wants`, its own required return on the capital the book will consume, is inside — is
never settled in either direction.

Two consequences:

- **A stated price with no flow behind it** (Law 5). The record publishes `pays` as what the acquirer
  paid; nothing paid it. A reader of `bank.resolution.bid` and `bank.resolution.done` sees a purchase
  price and a completed sale.
- **The acquirer's required return is not compensated, and it selected the winner.** The guarantee
  pays exactly `v.hole`, so the acquirer ends whole in balance-sheet terms and earns nothing at all
  for taking on the book, while the auction ranked bidders precisely by how much they wanted for
  doing so. The mechanism that decides _who_ is faithful to XI-3; the mechanism that decides _what it
  costs_ is missing, and its absence is invisible because a number stands in the record where it
  would be.

Everything around it is careful about exactly this distinction — `allocate` moves every haircut as a
real leg, `payFrom` settles the insurer's and the treasury's money, and `moveBook` extinguishes the
acquirer's own claim on the failed bank with a real leg at zero. The consideration is the one number
in the resolution that stays a number.

### A-60 — no bank makes a market in anything, because no bank employs anybody (A)

This is A-54's consequence, and it is larger than A-54.

`banks/dealing.ts:dealingOrders` gates every order — bid, offer, primary bid **and forced sale** —
behind one test:

```ts
if (!covers(view, i.id)) return [];
const state = stateOf(view, d);
if (state === undefined) return [];
const quoted = quoteFor(view, i.id, state);
…
const urgent = urgentSale(view, d, i.id);
if (urgent > 0) return [{ party: view.self.id, side: 'sell', price: 'market', qty: urgent }];
```

`covers` asks `linesCovered(view)`, and `linesCovered` (`banks/staff.ts:152`) is:

```ts
const own = view.lastOwn('labour.wages');
if (!own.some) return 0;
const hours = own.value.data['hours'];
if (typeof hours !== 'number' || hours <= 0) return 0;
…
return Math.floor(div(hours, per, 'the lines its people can cover'));
```

A bank only gets a `labour.wages` event if it employs somebody. `payWages` writes one per employer
that has rows; rows come from `hire`; `hire` comes from bids in `ctx.posted(labourVenue)`. Firms and
the treasury post there with `ctx.post` directly. **A bank's labour bid is a `venueParticipant`
(`banks/index.ts:1171`, `staffOrders`) and nothing ever gathers the labour venue** (A-54). So:

no gather → no bank bid for hours → no employment row → no `labour.wages` event →
`linesCovered() === 0` → `covers()` false for every line → `dealingOrders()` returns `[]`, for every
bank, in every market, in every period.

The `banking` occupation exists in `labour/data.ts` and its venue is opened per region, so the
market for bank staff is there with nobody bidding into it.

What that switches off:

- **Every dealer quote in the world.** No bid-offer spread, no inventory, no inter-dealer market
  (Dealer Desks E3, which the module notes is "without a second venue for it" — because the desks
  face each other in the ordinary session, and none of them is there).
- **`market.noView` on every book with orders in it.** `world/runOne` journals that event when no
  `speculative: true` participant posted. The bank face is the `speculative` one for bill, bond,
  share and fund-share books; households and merchants cover some of them, so it fires wherever they
  do not.
- **A bank cannot sell to meet a shortfall.** `urgentSale` is _after_ the `covers` gate, so XI-2's
  forced-seller door for banks — the one Banks Funding D1 and Money Market A2.b describe, and the
  one a fire-sale print is supposed to come out of — cannot open.
- **Primary dealership.** `primaryBid` is behind the same gate, so a sovereign auction has no
  primary dealer bidding into it.
- **The staffing mechanism it was built to express.** _"a desk that sheds staff drops lines, whose
  books then journal `market.noView` because nobody is standing in them"_ — every desk is in the
  shed-everything state permanently, and for a reason that has nothing to do with its book.

Two smaller things in the same path, worth noting because they will bite once the gather is fixed:

- `covers` picks the covered lines as the **first `linesCovered` entries of
  `view.instruments.all()`** — the global instrument-store insertion order, which is identical for
  every bank. So it is not a per-desk specialisation; it is one global cutoff applied to all of them,
  and every line past position `linesCovered` is quoted by nobody however many banks make its kind.
- `linesCovered` reads `lastOwn('labour.wages')` with no period bound — A-33's pattern again.

### A-61 — every central bank remits its income to the same treasury, in its own money (A)

`mechanisms/central-bank-omo/index.ts:remit`:

```ts
const treasuries = ctx.parties.ofKind(TREASURY).filter((t) => t.status.alive);
const to = treasuries[0];
if (to === undefined) return;
…
const ccy = ctx.registry.currencyOf(ctx.parties.get(cb).region);
const leg: Leg = { kind: 'money', from: { holder: cb, issuer: cb }, to: ctx.accountOf(to.id, ccy), ccy, amount: paid, … };
```

`treasuries[0]` is whichever treasury the parties store returns first — an insertion-order artefact
of the seed's draw, not a fact about who owns this central bank. In a world with four countries
(which `13j` built and which the rest of the engine is careful about) **all four central banks remit
to one country's treasury**, each in its own currency, into accounts that treasury holds at three
foreign central banks.

The clause it cites says the opposite: _"REMITTANCE (E3) is its net INCOME … the treasury owns it."_
Each treasury owns its own central bank. As written, three governments never receive the seigniorage
on their own money and one receives all of it, as an unexplained foreign transfer.

The registry already answers the question: `registry.centralBankOf(ccy)` is used everywhere else to
pair a money with its issuer, and `treasuryOf(ctx, bank)` exists in `money-market/resolution.ts` for
exactly this lookup. Every neighbouring module got the multi-country pass — `declareVenues` opens a
money-market book per currency with the note _"a euro bank cannot settle a dollar loan on the Fed's
books"_; `treasury/runReceipts` skips foreign legs with a worked example of what it cost; the
resolution auction filters bidders by `currencyOf(other.region) !== ccy`. This one was missed.

### A-62 — a central bank's loss is forgotten at the next remittance, and the comment says it is carried (B)

Same file. `remit` decides the window it sums over from the last event it can find:

```ts
function lastRemittance(ctx: MechanismContext, cb: PartyId): Period {
  const events = [
    ...ctx.journal.ofKind('centralBank.remittance'),
    ...ctx.journal.ofKind('centralBank.loss'),
  ]
    .filter((e) => e.subjects.includes(cb))
    .sort((a, b) => a.period - b.period);
  const last = events[events.length - 1];
  return last === undefined ? period(0) : period(add(last.period, 1, 'after the last remittance'));
}
```

A LOSS advances the window exactly as a remittance does. So the sequence is:

- window `[a, b]` earns −100 → `centralBank.loss` recorded, nothing paid, _"E4: a loss is not
  remitted. It reduces its equity and stands there until income covers it."_
- next window starts at `b + 1`, earns +100 → the whole +100 is remitted.

The −100 is never covered. The central bank ends two windows down 100 in equity, having paid out
100 it did not earn, and the treasury has been handed money against a loss that is still on the
central bank's books. E4's _"until income covers it"_ is precisely the behaviour the `+1` removes.

The fix is one line and it is the honest one: a loss should NOT advance the window, so the next
window's income is netted against it before anything is remitted. Note that `income <= 0` is also
the branch taken when income is exactly zero, which then advances the window for no reason at all.

### A-63 — every household buys its groceries on thirty-day credit; the consumption-tax base is therefore always zero, and the households audit family fires on every purchase (A)

`clearing/market.ts:payment` asks the seller's module what the buyer pays with, on **every** asset
trade:

```ts
const promise = deps.onTerms?.({ seller: t.seller, buyer: t.buyer, ccy: m.ccy, cash, sold: m.instrument }) ?? none();
if (promise.some) {
  return { kind: 'asset', from: t.buyer, to: t.seller, instrument: promise.value, qty: cash, pricePerUnit: some(1), … };
}
return { kind: 'money', from: deps.accountOf(t.buyer, m.ccy), … };
```

`world.ts` routes that to `termsDeciders.get(parties.get(sale.seller).kind)`, and `trade-credit`
registers `termsOffered: [{ partyKind: FIRM, decide: shipsOnTerms }]`. Goods are sold by firms. So
the decision is taken for every goods sale in the world, and `shipsOnTerms` asks only four things:

```ts
if (cash <= 0 || seller === buyer) return none();
if (ctx.instruments.get(sold).issuer.some) return none();     // it is a good: no issuer
const both = …alive…; if (!both) return none();
if (letDown(ctx, seller, buyer)) return none();               // no OVERDUE invoice with THIS seller
```

Nothing asks what kind of party the buyer is. **A household cell gets thirty days on its bread**,
and issues an invoice for it. Three consequences, and each is independently a defect.

**1. The household's budget constraint is not enforced at the wire.**
`households/index.ts` states the opposite as a premise of the module:

> _"What it does not do is borrow — nobody lends to it yet (worklist 6) — so its budget is its own
> cash, and what it cannot pay for it does not buy."_

and its party kind says `borrows: false, fails: []`. As written, a household's goods purchase needs
no money at settlement; `spendPerMember`'s `atMost(wanted, budget)` is now a self-imposed plan rather
than a constraint anything checks. When the invoice matures the kernel collects it, the payment can
fail, and `fails: []` means the household can never be declared failed for it — so unpaid invoices
accumulate with no consequence beyond `letDown` cutting off that one seller, and there are many
sellers.

**2. The consumption tax collects nothing, ever.** `treasury/runReceipts` builds its base from money
legs:

```ts
for (const leg of r.instruction.legs) {
  if (!isMoneyLeg(leg)) continue;
  if (buyers.has(leg.from.holder)) {
    bases.consumption = add(bases.consumption, leg.amount, 'what households paid for goods');
    addTo(due, leg.from.holder, mul(leg.amount, onConsumption, 'consumption tax'));
  }
```

In a terms sale the household's side is an **asset** leg (the invoice), so the `continue` fires and
nothing is assessed. When the invoice matures a month later, that instruction has no physical leg in
it, so `buyers` is empty and nothing is assessed then either. `bases.consumption` is structurally
zero for the life of the run.

Meanwhile the household has already paid the tax in its own arithmetic: `demandOf` sets aside
`set = qty × perUnit` with `perUnit` including `consumptionTax`, and bids
`net = set / (1 + tax)`. So `treasury.tax.consumption` is a pure wedge that lowers every household
bid and raises no revenue for anybody — money that leaves demand and arrives nowhere.

**3. The households `flows` family reports a violation on every purchase.**
`consumptionIsBought` pairs physical legs in against money legs out:

```ts
if (isAssetLeg(leg) && cells.has(leg.to)) { …physical… addTo(bought, leg.to, mul(leg.qty, price, 'what it took')); }
else if (isMoneyLeg(leg) && cells.has(leg.from.holder)) { addTo(paid, leg.from.holder, leg.amount); }
```

The invoice leg is an asset leg whose `to` is the seller, so it matches neither branch. `paid` stays
empty, `took` is the whole basket, and the family emits
`${cell} took ${took} of goods and paid 0 for them` — for every cell, in every goods market, in
every period. Its spec citation is `Households C5` and its subject is _"Units of a physical thing
reaching a household with no money going the other way in the same instruction is a gift nobody
gave"_, which is exactly what a terms sale looks like to it and exactly what a terms sale is not.

The module that introduced terms is careful that both legs are in the one instruction (_"there is
never an instant where one side has parted with something and the other has given nothing"_) — the
three readers downstream were not told.

### A-64 — a firm pays rent for storage space and receives nothing for it (A)

`mechanisms/commodities/index.ts` clears a storage venue per region, pairs takers with letters, and
settles a real money leg for every match:

```ts
const due = ctx.registry.payable(ccy, mul(space, rate, 'what the space costs for the period'));
…
const r = ctx.settle({ legs: [{ kind: 'money', from: ctx.accountOf(taker.party, ccy),
  to: ctx.accountOf(letter.party, ccy), ccy, amount: due, … }], cause: 'transfer',
  reason: `${taker.party} rents ${space} of space from ${letter.party}` });
if (r.outcome === 'settled') {
  held.byParty.set(String(taker.party), add(leased(held, taker.party), space, 'space taken'));
  …
}
```

**Nothing ever reads `held.byParty`.** `leased()` is called in exactly one place — inside the
`.set` above, to accumulate. Grepping the engine for `commodities.leases`, `byParty` and `leased(`
finds only this file and only these lines.

What is supposed to read it is stated in the module's own header:

> _"Room BINDS what a line can have at the end of a period, exactly as its machinery binds what it
> can make (Capital Programme A2, D4): a good that takes space declares how much, and **the space a
> firm has — its own plus what it rented this period** — is one more plant need in the same
> arithmetic that already takes the scarcest."_

The arithmetic it means is `firms/decide.ts:technologyOf`, which appends a `STORAGE` plant need, and
`capacityFrom(tech.plant, vintages)` where `vintages = vintagesHeld(view, …)`. `vintagesHeld` walks
`view.holdings()` for instruments that `isPlant` — **owned plant only**. A lease is not a holding, is
not a vintage, and is nowhere in that read. So:

- a firm short of room bids for it, wins, and pays the letter;
- its capacity is unchanged, because the capacity read counts only the storage vintages it owns;
- next period it is short of exactly the same room and rents it again.

The money is conserved (the letter receives it), so no audit family will see anything. What is
broken is that one side of a real, settled, two-sided payment gets **no consideration at all** — the
buyer of a service that does not exist. Law 1's _"real mechanism … fees, refusals, failures"_ has the
fee without the thing.

Two smaller defects in the same loop:

- **A failed payment still consumes the space.** The `want` and `letters[at].qty` decrements sit
  outside the `if (r.outcome === 'settled')` block, so a taker whose rent does not settle absorbs
  the letter's room anyway and the next taker cannot have it. One insolvent taker can shut a region's
  storage market for the period.
- **The `Leases` store is never emptied.** Its own comment says _"Emptied at the top of every period:
  a lease is a week's"_ — `ctx.state` persists for the life of the world and nothing calls `clear()`,
  so `byParty` is a monotonically growing write-only accumulator.

### A-65 — every option premium is a money-squared number, and it does not depend on the strike (A)

`mechanisms/options/index.ts:optionOrders`:

```ts
const outlook = view.outlook(`${PRICE_OF}${String(t.underlying)}`);
const moves = outlook.some
  ? mul(outlook.value.expected, outlook.value.confidence, 'what it thinks it moves')
  : 0;
const mine = moves > 0 ? add(moves, mul(moves, aversion, 'what its capital wants'), 'its quote') : 0;
…
return [{ party: view.self.id, side: move > 0 ? 'buy' : 'sell', price: mine, qty: asQty(qty) }];
```

**Two things, and the first is dimensional.**

`confidence` is `width(surprises)` — the standard deviation of `observed − expected` in the
variable's own unit (`expectations/index.ts:183`). For `price.<instrument>` that unit is **money per
unit of the underlying**, which is how every other reader uses it:

- `savingLines`: `sub(expected, confidence, 'what it will pay')` — subtracted from a price;
- `consume.ts:pricesOver(expected, width, steps)` — added to and subtracted from a price;
- `consume.ts:atRisk`: `mul(units.value, confidence, …)` — units × confidence gives money.

`expected` is money per unit too. So `expected × confidence` is **money² per unit²**, and it is
posted as `price` into a book whose tick is `CENT_TICK` and whose unit is `optionContracts` — money
per contract. The comment immediately above says what the right term is and then multiplies it by
the wrong one:

> _"What it thinks the thing MOVES is its own outlook's CONFIDENCE **and not its level**"_

The consequence is not a scaling constant: the premium is proportional to the **square** of the
underlying's price level, so an option on a line at 100 with 1% surprises quotes 100 (the whole
value of the underlying) while the same 1% on a line at 1 quotes 0.01. Every option in this world is
mispriced by a factor of the underlying's price.

**Second: the premium does not depend on the strike, the moneyness or the time to expiry.** `mine`
is built from the outlook alone. `t.strike`, `t.right` and `t.expiry` appear nowhere in it — `t.right`
is used only to decide whether a HOLDER wants a put, and `t.multiplier` only to convert a size. So a
party quotes the same premium for a deep out-of-the-money call and an at-the-money put on the same
line in the same session, and the strike ladder `openBooks` builds (_"a ladder of books on lines this
world already clears, at strikes around what they print"_) is a set of books that every participant
prices identically.

D7's _"the premium is what clears"_ is respected — nothing here derives a price from a volatility,
and `measures` correctly takes the implied move back OFF the printed premium. What is wrong is the
reservation each party brings to the book, which is what decides where it clears.

### A-66 — eight of the nine derivative books can never produce a first print (A)

Every contract class builds a party's order the same way:

```
target  =  <its hedging need, from its own book>
        ±  <conviction, IF its own number differs from THIS BOOK'S LAST PRINT>
order   =  target − <what it already has>
```

Each class is careful, and says so, that the book's own print must not be the LEVEL it posts —
`bond-futures`: _"a party that posted where THIS book last was would be agreeing with it rather than
saying anything, and a book of those prints one number for ever"_; `cds`, `irs`, `commodity-futures`
and `fx-derivatives` all carry the same paragraph. What none of them noticed is that the print is
still load-bearing for the **direction**: with no print, the conviction term drops out and every
party in the book is left with its hedging need alone — and a hedging need has one sign.

| book                | with no print, `want` is                                                          | so the first session is | can it open? |
| ------------------- | --------------------------------------------------------------------------------- | ----------------------- | ------------ |
| fx forward          | hedgers one way, plus an arbitrageur quoting **bid and ask** around its own carry | two-sided               | **yes**      |
| option (put)        | `held × aversion / multiplier` for holders, 0 for everyone else                   | buy-only                | no           |
| option (call)       | 0 for everyone                                                                    | **no orders at all**    | no           |
| bond future         | `−held / contractSize`                                                            | sell-only               | no           |
| commodity future    | `−held / lotUnits`                                                                | sell-only               | no           |
| interest-rate swap  | `−fixedDebtOf(view, t)`                                                           | sell-only               | no           |
| CDS, single name    | `exposureTo(view, t)`                                                             | buy-only                | no           |
| index future        | `book / perContract`, and the only `side` in the file is `'sell'`                 | sell-only **always**    | no           |
| CDS series          | `if (!last.some) return []` on the book's OWN line                                | **no orders at all**    | no           |
| cross-currency swap | `if (!last.some) return []` on the book's OWN line, then buy-only                 | **no orders at all**    | no           |

Nothing seeds a print for a contract book: `openBooks` in each module calls `ctx.openMarket(...)`
with no price, and `foundation.ts` writes opening prints only for goods and sovereign lines. A
contract book's `instrument` is a synthetic line (`irsLineOf`, `seriesLineOf`, `t.book`) that only a
session can print. So `noDemand`/`noSupply` in period one, no print, and the same again for ever.

The one that works is the one whose author hit the problem and built the answer:

```ts
// B1, B2, B2.b: THE ARBITRAGE, and a party with nothing to hedge is the one that takes it. It
// quotes BOTH WAYS around its own carry — a bid a tick below and an ask a tick above …
// a book whose members were all hedgers printed one number for ever and the cash-and-carry
// relationship this class exists to express was live in period one and dead from period two.
return [
  { party: view.self.id, side: 'buy', price: bid, qty: asQty(size) },
  { party: view.self.id, side: 'sell', price: ask, qty: asQty(size) },
];
```

Two of the eight are worse than a bootstrap problem and would still be broken with a print in hand:

- **`index-futures/futureOrders` has no buy branch at all.** Its docstring is _"A DESK LONG A BOOK OF
  SHARES SELLS THE INDEX"_ — one true reason, and the only one implemented. §46 A3 and XI-13 are
  explicit that a market needs two, and this one is one-sided by construction rather than by
  circumstance.
- **`cdsIndexOrders` and `xccyOrders` read their own book's last print as a precondition**
  (`const last = view.print(seriesLineOf(...)); if (!last.some) return [];`) and then post at
  `last.value.price` or a multiple of it. That is exactly the fixed point the single-name CDS in the
  same directory refuses in a comment two hundred lines away: _"its OWN number — the cash market's
  charge for this credit … **Neither is this book's own last price**."_

Everything downstream of these books is downstream of this: `refusedThisPeriod`, the margin system,
the clearing house's waterfall, `marginIsHeld`, the option-implied move that §46 A3 says the world
needs so parties can disagree about dispersion, and the basis measures each class publishes. All of
it is built, wired and exercised by nothing.

### A-67 — nothing ever borrows a security: the whole securities-lending mechanism is unreachable (A)

`mechanisms/securities-lending/index.ts` builds the venue, the fee clearing, the title transfer, the
collateral pledge, the manufactured payment and the recall. Two exported functions are the only ways
in, and **both are called by nobody**:

```
$ grep -rn "runBorrows"    packages/engine/src   →  the definition, and nothing else
$ grep -rn "wantsToBorrow" packages/engine/src   →  the definition, and nothing else
```

The module's single phase is `borrow.economics`, which runs `manufacture` and `charge` — both of
which iterate `state(ctx).open`, the book of open borrows. Nothing ever pushes to it. So the phase
walks an empty list every period for the life of the world, and XI-11's _"title passes and the
economics do not"_ is built and never exercised.

The prohibition it exists to satisfy — _"no short without a borrow"_ — holds, but vacuously: there
is no short anywhere in this world either, so there is nothing for the borrow to be behind.

**Two further defects inside the unreachable code, which matter because they are what would run:**

1. **The fee is not cleared, it is the single bidder's reservation.** `runBorrows` loops
   `for (const w of wanted)` and clears a separate session per borrower, in which every lender posts
   `price: 'market'`:

   ```ts
   for (const s of supply)
     ctx.post(venue, { party: s.lender, side: 'sell', price: 'market', qty: s.units });
   ctx.post(venue, { party: w.borrower, side: 'buy', price: w.willPay, qty: w.units });
   const outcome = clear(ctx.posted(venue), 'proRata', 'sellersCompete');
   ```

   One bidder, and no seller with a level to compete on, so `outcome.price` is `w.willPay`
   regardless of how much paper is on offer. The docstring says _"Scarce paper is dear and abundant
   paper is cheap, and neither is a table"_ — as arranged, neither scarcity nor abundance can move
   the fee at all.

2. **Two `Want`s on one instrument double-post the lenders.** `world.post` appends and `postings` is
   cleared only at the top of a period, so the second iteration for the same instrument posts every
   lender's offer again into the same venue. The book then shows twice the supply that exists and a
   lender can be allotted twice what it holds.

**And the same non-rate appears here as in A-58.** `wantsToBorrow`:

```ts
willPay: div(owed, add(owed, equity, 'what funds it'), 'what a period of its own money costs'),
```

`owed / (owed + equity)` is the debt share of funding — a dimensionless ratio, not a per-period cost
— and it is the identical expression, with the identical comment, that `securitisation:priceFor`
uses to price a tranche. Two modules, one mistake, and in both of them the number the comment
describes is published every period by `banks/index.ts:publishCostOfFunds`.

### A-68 — loading a cargo writes off what the cargo cost (A)

`mechanisms/freight/index.ts`, the instruction that puts a cargo aboard:

```ts
{ kind: 'destroy', party: shipper, instrument: i.id, qty: asQty(take), why: 'consumed', fromCell: none() },
{ kind: 'create',  party: shipper, instrument: transit, qty: asQty(take),
  costPerUnit: div(add(share, 0, 'the freight'), take, 'what the voyage added to a unit'), toCell: none() },
{ kind: 'money', from: ctx.accountOf(shipper, ccy), to: ctx.accountOf(carrier, ccy), ccy, amount: share, … },
```

Settlement's effect on the shipper's equity, leg by leg (`ledger/settlement.ts:1089-1100, 1112-1116`):

| leg                                           | equity                                                |
| --------------------------------------------- | ----------------------------------------------------- |
| destroy the cargo                             | `− costOfDraw(lots, take)` — its whole carrying value |
| create the goods-in-transit at `share / take` | `+ share`                                             |
| pay the freight                               | `− share`                                             |
| **net**                                       | **`− costOfDraw(lots, take)`**                        |

The goods in transit are carried at the **freight alone**. Everything the cargo cost to buy or to
make is expensed at the moment it is loaded.

`add(share, 0, 'the freight')` is the tell: a sum of one term and a zero, where the second term is
what the cargo cost. `produce.ts` does the same operation correctly forty lines of another file
away — `const costs: number[] = [wages]; … costs.push(heldCost(ctx, firm, input.instrument, qty));`
— and `costOfDraw` is exported from `register/register.ts` for exactly this read.

The total over a completed voyage is right (`−C` at loading, `+P − F` at sale, so `P − C − F`), which
is why no conservation family catches it. What is wrong is the whole of what the module says it is
for:

> _"AND IT TAKES TIME (A3). A cargo leaves the origin now and is ON THE SHIPPER'S BOOK the whole
> way, at a place of its own — **A3.a's working capital: a shipper that has paid for a cargo and not
> yet got it is short of both**."_

It is not on the shipper's book at what it cost — it is on the book at the freight. So a shipper
mid-voyage shows an equity hole the size of its cargo (which `failedWhy`'s solvency trigger reads,
and which can kill a merchant on the water), and the arrival books a profit equal to the cargo's cost
that no trade produced. `arrive()` is correct — it carries the transit lot's own basis forward with
`div(cost, total, 'what a unit cost delivered')` — so the error is entirely at loading, and its own
docstring there says the opposite: _"the destination at what it cost INCLUDING the voyage (D2)"_.

### A-69 — nine exported entry points and reads that nothing calls (C)

Checked across the whole repository (`packages/`, tests and app included), these are defined,
exported, documented and referenced by nothing:

| function                                     | file                          | what it was for                                                                                                                                                                                                                             |
| -------------------------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `runBorrows`, `wantsToBorrow`, `returnLoans` | `securities-lending/index.ts` | the whole borrow mechanism (A-67)                                                                                                                                                                                                           |
| `quoteCover`, `policyTerms`                  | `insurers/index.ts`           | the whole insurance mechanism (A-9)                                                                                                                                                                                                         |
| `cdsBookOrders`                              | `cds/participants.ts`         | _"Law 15: ONE PARTICIPANT, TWO SHAPES OF BOOK, and the dispatch is on the shape of the terms rather than on an id"_ — the two classes call `cdsOrders` and `cdsIndexOrders` directly, so the dispatcher it argues for is not the one in use |
| `refusedThisPeriod`                          | `derivative-layer/index.ts`   | _"E4: what the markets struck BEYOND what their members could margin — a standing measurement"_                                                                                                                                             |
| `struckRate`, `findSession`                  | `money-market/session.ts`     | reads of what a session struck                                                                                                                                                                                                              |

The first two rows are missing mechanisms and are logged separately. The rest are dead code, and
they matter for one reason: each carries a docstring asserting that something is measured or
dispatched, and a reader checking whether the world has a given property will find the function and
believe it. `refusedThisPeriod` in particular is described as _"a standing measurement"_ of E4 and
measures nothing, standing or otherwise.

### A-70 — a takeover buys the shares and never absorbs the company (A)

`mechanisms/control/index.ts` builds both halves of corporate control. The first half is wired: the
`control.tender` phase calls `controlBidsFor` and `runTender`, so an acquirer bids for a target's
line, the fills settle, and `control.acquired` is published.

The second half — `combine` — is exported, documented at length, and **called by nobody** anywhere
in the repository:

```
$ grep -rn "\bcombine\b" packages/  →  the definition, and nothing else
```

```ts
/**
 * A4, A5, D4, XI-8: THE TWO BALANCE SHEETS COMBINE. The target's own paper is assumed by the
 * acquirer, its holdings are reseated, its shares cease to be a claim on anything because the
 * residual they were has been bought, and the party is terminated with the acquirer as successor.
 */
export function combine(ctx: MechanismContext, buyer: PartyId, target: PartyId): void { … }
```

So an acquirer accumulates a majority of a target's shares and nothing ever happens: the target
stays a separate party with its own balance sheet, its own line and its own board for ever, and
Control A4/A5/D4's "the two balance sheets combine" is a function nothing invokes. `control.acquired`
is published every time and is the end of the story.

**And the function does not do what its own docstring says it does.** It loops
`ctx.instruments.issuedBy(target)` and settles an `assume` leg for each — `assume` maps to the
`reseat` op, which changes an instrument's **issuer** (`register/instruments.ts:234`). That moves the
target's LIABILITIES. Nothing anywhere in it moves the target's HOLDINGS — its cash, its plant, its
inventory, its paper — and the docstring's _"its holdings are reseated"_ describes an operation that
is not in the function.

If it were called, the consequence is loud and permanent: `ctx.cease(target, buyer)` marks the
target dead while the register still has holdings under its id, and `audit/families/names.ts:96`
reports

```
${target} has ceased but still holds ${instrument}
```

for every line it held, every period, for the rest of the run — Register F2, the clause the
docstring cites.

It also inherits A-1: `combine` is the only caller of `assume` outside settlement's own paths, and
`reseat` books the whole reseated liability against the issuer's equity account without the
per-member wrapping that `issue` and `redeem` use. Targets are named parties today, so that half is
inert — but `combine` is where it would first bite if a cell ever issued anything.

---

## What Part I covered

`packages/engine/src`, 59,927 lines across 196 files, read in dependency order: `core/`, `calendar/`,
`rng/`, `registry/`, `parties/`, `register/`, `ledger/`, `prices/`, `clearing/`, `journal/`,
`audit/`, `world/`, then every module under `mechanisms/`, then `seeds/` and `observer/`.

The three questions were asked of every file. Where a file passed all three it is not in this
document — the absence of a finding is the finding, and the files that carry the most weight in this
engine are among them: `core/num.ts` and `core/tick.ts` (the whole grid and dust discipline),
`ledger/settlement.ts`'s money expansion (traced through every issuer/central-bank combination —
money is structurally conserved on the wire), `clearing/solver.ts` and `clearing/market.ts`,
`prices/curve.ts` and `prices/index-read.ts` (Law 3 held exactly), `registry/params.ts` (the
shape/placeholder guards, which are the strongest single piece of enforcement in the repository),
`banks/dealing-quote.ts` (a two-sided quote with no width parameter anywhere in it),
`banks/treasury.ts`, `mechanisms/spot-fx/` (four reasons and a two-way dealer — the pattern A-66
says the derivative books needed), `mechanisms/ratings/`, `mechanisms/reporting/`,
`mechanisms/research/`, `mechanisms/collateral`, `journal/`, `ledger/` and `observer/observer.ts`
(nothing in it writes).

### The pattern behind most of it

Three shapes account for more than half the findings, and none of them is a mistake in arithmetic:

1. **A door that exists and is not used.** `gather` (A-54) takes out the lettings market and every
   bank's employment, which takes out every dealer quote in the world (A-60). `combine` (A-70),
   `runBorrows` (A-67) and `askForMortgages`' cell guard (A-55) each take out a whole system. In
   every case the mechanism is built, documented and correct; nothing calls it.

2. **A number whose direction depends on a print that cannot exist yet.** Eight of nine derivative
   books (A-66) are one-sided in their first session for the same structural reason, and one book
   — FX forwards — is not, because its author hit the problem and wrote the two-way maker.

3. **A read that was right when it was written and is not now.** Deposits pay, so the household's
   liquidity premium is wrong (A-44). Trade credit exists, so the consumption-tax base is zero and
   the households family fires on every purchase (A-63). Loans are marked, so credit-events'
   "nothing books a provision" is false. Each is a live mechanism landing on a reader nobody
   revisited.

### What was NOT looked for

Performance, style, test coverage, and anything in `packages/app`. Nor was anything run: this is a
read of the source, and the two places where I state what a run WOULD do (`A-18`'s micro-cells,
`A-66`'s books) say so and show the arithmetic instead of a measurement.

---

# Part II — What the documents claim, and what is in the code

Part I read the source against the three questions and did not look at what anybody had said about
it. This part does the other half: it takes every row of `docs/WORKLIST.md` marked **done** and every
`MET` in `docs/COVERAGE.md`, and asks whether the thing claimed is in the code and produces anything.

The distinction that matters throughout is the one `docs/VERIFY.md` found first and put well:
**"built" was true of the source and false of the world.** Almost nothing here is missing code. It is
code that exists, compiles, is wired into the assembly, and has never produced an outcome — and a
worklist row, a coverage mark and a record entry all say it works.

### B-1 — `corporate.bond` is a kind, an id function and a covenant test, and nothing ever issues one (A)

Worklist 13f, **done**:

> _"**The corporate bond**: it can fail, it cross-defaults where a sovereign does not, it says where
> it ranks on the instrument the waterfall already reads, and it carries COVENANTS tested on the
> issuer's PUBLISHED accounts."_

`mechanisms/corporate-bond/index.ts` declares `CORPORATE_BOND`, `corporateBondId(issuer, n)`, a full
`InstrumentKindProfile` with `cashFlows`, `due`, `ranking` and cross-default, a covenant table and
the `covenant.test` phase. Searching the whole engine for a construction site:

```
$ grep -rn "kind: CORPORATE_BOND" packages/engine/src   →  nothing
$ grep -rn "CORPORATE_BOND"       packages/engine/src   →  only its own declaration and profile
```

**No corporate bond has ever been issued and none can be: there is no issuance path.** `testCovenants`
runs every period over an empty set. Three `MET` marks in COVERAGE (`Corporate Credit A1`, `B2`,
`B3`) cite this file.

The clause the module is for — a firm funding itself in a market rather than at a bank — has no
mechanism that puts a firm in it. `firms/index.ts:publishFunding` publishes what a firm is short of
and `banks` reads it; nothing reads it as a reason to issue paper.

### B-2 — the insurance sector has no seed, no phase and no participant (A)

Worklist 13h, **done**: _"Closed with the INSURER built."_ COVERAGE marks nine `Insurers` clauses MET
against this file.

```ts
export function insurers(): SystemModule {
  return {
    id: 'insurers',
    spec: 'Insurers',
    requires: ['sovereign-curve'],
    instrumentKinds: [policyKind],
    partyKinds: [insuranceKind],
    curveFamilies: [],
    units: [{ id: COVER, name: 'units of cover', perUnit: MONEY_PIECES }],
    params: [],
    phases: [],
    participants: [],
    families: [promises()],
  };
}
```

No `seed`, so **no `insurance` party is ever created**; `phases: []` and `participants: []`, so
nothing it exports is ever called; and per A-9 the kind's `unit` returns `"USD"` where the registry
wants `"ccy:USD"`, so a policy could not be registered even if something tried. `promises()` — the
`names` family contribution — walks an empty set and reports green.

The sector the module is a careful and correct piece of design for (B2.b's dated promise discounted
at a market rate, the institution wearing the rate move) does not exist in this world.

### B-3 — securities lending is claimed to clear a fee, and has no way in (A)

Worklist 13f, **done**: _"**Securities lending**: title passes and the economics do not … the fee
CLEARS in a book per line, the manufactured payment is read off what actually reached the borrower,
and the lendable pool is a read of who holds it free, which is what caps a short."_ Nine `MET` marks.

Per A-67: `runBorrows`, `wantsToBorrow` and `returnLoans` are exported and referenced nowhere. The
module's one phase walks `state(ctx).open`, which nothing ever pushes to. There is no short anywhere
in this world for a borrow to be behind.

### B-4 — the market for control buys shares and never combines (A)

Worklist 13g, **done**: _"On completion the two balance sheets combine through the ESTATE's own door,
because 'this party's obligations are now that one's' is one fact."_ Ten `M&A` clauses marked MET.

Per A-70: the tender half is wired and runs; `combine` is called by nobody, and does not move the
target's holdings even if it were.

### B-5 — the tenancy venue has never had an order in it (A)

Worklist 13d, **done**: _"a tenancy as a VENUE, clearing between what letting wears the owner and
what a household can pay rather than have nowhere."_ `Housing A2`, `B5` and `C4` marked MET.

Per A-54: housing declares its orders as `venueParticipants`, and `gather` — the only door that
runs them — is called by one module, which is not this one. The lettings session reads an empty
book every period and records `noDemand`. Per A-55 the buying half is unreachable too.

### B-6 — item 9's dealers quote nothing (A)

Worklist 9, **done**: _"Equity, and dealers that carry inventory … a desk with a limit, a funding
cost and an inventory."_ `banks/dealing-quote.ts` is among the best-built files in the engine.

Per A-60: the gate in front of it (`covers` → `linesCovered` → `lastOwn('labour.wages')`) is zero
for every bank for ever, because no bank employs anybody, because its labour bid is a
`venueParticipant` and nothing gathers the labour venue. Not one quote is ever posted.

### B-7 — the derivative layer and its nine classes have never produced a contract (A)

Worklist 13a and 13b, both **done**, 37 steps between them. **99 distinct requirements are marked MET
against modules that have never produced an outcome**, and 58 of those 99 are the derivative layer,
CDS, IRS, the two futures classes and options.

`docs/VERIFY.md` measured the runtime side: nine derivative kinds declared, **not one contract of any
class ever written** over five periods; 7,510 option sessions all `noDemand`, 3,680 commodity-future
sessions all `noDemand`, 60 CDS sessions all `noSupply`, the IRS books never run at all. A-66 gives
the cause at the source: every class makes a party's SIDE depend on comparing its own number against
this book's last print, so with no print the conviction term drops out and every party is left with
its hedging need, which has one sign.

**This corrects `docs/VERIFY.md`'s own conclusion.** That file diagnosed the single measured cause as
the margin gate — _"no party can post margin in a currency it does not hold"_ — and concluded
_"the never-crossing books are downstream of it, not a second cause."_ They are not downstream of it:
they never reach admission, because they never cross. The margin gate is the whole story for **FX
forwards only**, which is the one book that crosses, and it crosses because it is the one class with
a two-way maker that needs no prior print. Two causes, and the structural one is the larger.

### B-8 — the securitisation waterfall never allocates a loss (A)

Worklist 13e, **done**: _"a waterfall paying by seniority out of what was actually collected … with a
loss landing from the bottom and nothing stopping it reaching the senior."_

Per A-57: `absorb` computes `lost = notes − pool`, and because the pool's interest is paid out as
accelerated principal redemption, `notes` falls faster than `pool` and `lost` is negative from early
on. No loss is ever allocated to any tranche, whatever the borrowers do.

### B-9 — `docs/BUGS.md` contradicts its own header about the four countries (C)

Its header: _"Measured on the whole suite: 82 red of 662 **after item 13j gave this world four
economies**."_ Its section 2, in the same file: _"**The foreign countries are stubs.** Three of the
four countries are a central bank, a treasury and a bond line. There is no foreign economy … →
**13i**, which closed with the external accounts built and the foreign economies not."_

Both cannot be current. 13j is marked done and its row claims each of the four gets the same
construction from the same draw; section 2 says three of them are still stubs and points at the item
before it. One of the two is stale and the file does not say which. A-61 is a live piece of evidence
for the pessimistic reading: every central bank in the world remits its seigniorage to
`treasuries[0]`.

### B-10 — "checks green" has been satisfied by families that cannot fail (A)

The loop's definition of done is _"an item is done when its checks are green, `docs/RECORD.md` has its
entry, and `docs/COVERAGE.md` is re-marked."_ Four of the nine audit families are green for reasons
that are not the state of the world:

- `currency`'s money contribution recomputes the number it is checking (A-10);
- `labour`'s population identity is `Σ p.weight` against `Σ p.weight` (A-14);
- `goods`' and `capital-programme`'s `units` contributions switch off in any period with a weight
  event, which is every period after the first ageing (A-42);
- `funds`' `equityIsZero` restates the definition of the NAV (A-48);
- and `zeroSum` — the derivative layer's whole invariant — walks a set of size 0 (VERIFY's census),
  because of B-7.

A green audit has been part of the evidence for thirteen `done` rows.

### B-11 — `npm run check` has been red since the sweep, at the gate that counts the plan (A)

**First written as something smaller and it was wrong, so here is the correction and then the
finding.** The first version of B-11 said five open items (13k–13o) have no plan file where the
worklist's preamble says they must, and that the progress figure therefore counts five items fewer
than there are. The first half is true and harmless — those five were inserted from `docs/SWEEP.md`
with their reasoning in the worklist row and no plan written yet. The second half is false:
`docs/plan/manifest.json` **does** carry all five, as `"file": "<id>-no-item-file.md", "steps": 0`,
which is the convention 10.1, 10.2 and 10.4 established, and `plan:progress` counts them.

What is actually there is worse. The commit that inserted those five rows — `6d6cb6d`, "The sweep" —
wrote them **over** the manifest row for **13j** instead of after it. So the worklist has 51 items
and the manifest had 50, and `crossCheck()` — which exists for exactly this, and whose docstring
tells the story of the last time it happened — throws:

```
Error: docs/WORKLIST.md has items the manifest does not: 13j.
Add the row in the change that inserts the item (docs/PLAN.md §5)
```

`npm run check` runs `plan:check`, which calls it. So **`npm run check` has not been green since
`6d6cb6d`**, and two commits after it (`e543d8e`, `22fdc7f`) closed work in a tree where the last
gate CLAUDE.md names — _"All green or the module is not done"_ — could not have passed. A second
test in the same file was red with it: `tools/test/plan-progress.test.ts` asserts the items with no
plan file are exactly `['10.1', '10.2', '10.4']`, and the sweep added five more.

The progress block in `docs/PLAN.md` had gone stale in the same commit and stayed stale: it still
listed **13j as "in progress", 13 of 14 steps, linked to `plan/13j-four-countries.md`** — a file
deleted when 13j closed, in a row for an item the manifest no longer had. The one number this
project publishes about its own completion was describing a tree that had not existed for four
commits.

**And the same commit is where item 13j closed, without the three things that close an item.**
CLAUDE.md: _"An item is done when its checks are green, `docs/RECORD.md` has its entry, and
`docs/COVERAGE.md` is re-marked, all in one commit (Law 14)."_ `6d6cb6d` set row 13j from `open` to
`done` and deleted `docs/plan/13j-four-countries.md`, and it touched neither `docs/RECORD.md` nor
`docs/COVERAGE.md`:

```
docs/SWEEP.md                   | 151 +++++++++
docs/WORKLIST.md                |   7 +-
docs/plan/13j-four-countries.md | 103 -------
docs/plan/manifest.json         |  32 ++-
```

**`docs/RECORD.md` has no entry for 13j** — its last item entry is 13i's, followed by four Law 18
performance entries. The one item that gave this world four working economies, which is the biggest
change in it since the module contract, left no outcome in the ledger. Its measurements survive only
in the commit message (9,225 parties before, 9,231 after, 5,593 of them not American; 62,064
instructions and 3,620 markets in 19.7s), and a commit message is not the ledger. This is not fixed
here: an item's record entry is written by whoever did the item, out of what they measured, and
inventing one after the fact from a diff is the opposite of a ledger of outcomes.

**Fixed in this change**, because it is a build stopper and not a measurement: the `13j` row is back
in the manifest, both tests are green, and the block is recounted. What the recount says is in
**B-15**.

### B-12 — 99 `MET` marks stand on mechanisms that have never produced anything (A)

Counted over `docs/COVERAGE.md`'s 823 `MET` rows, by the module each cites:

| module                    | MET rows | why nothing comes out of it                                             |
| ------------------------- | -------- | ----------------------------------------------------------------------- |
| `derivative-layer`        | 22       | B-7                                                                     |
| `cds`                     | 17       | B-7 (single name never crosses; the series book requires its own print) |
| `irs`                     | 14       | B-7 (the books are never run)                                           |
| `control`                 | 10       | B-4                                                                     |
| `securities-lending`      | 9        | B-3                                                                     |
| `insurers`                | 9        | B-2                                                                     |
| `commodity-futures`       | 5        | B-7                                                                     |
| `bond-futures`            | 4        | B-7                                                                     |
| `corporate-bond`          | 3        | B-1                                                                     |
| `index-futures`           | 3        | B-7 (sell-only by construction)                                         |
| `housing`                 | 3        | B-5                                                                     |
| **distinct requirements** | **99**   |                                                                         |

COVERAGE's own header says `MET at <path>` means _"the cited module implements the clause"_ — which
is literally true of all 99 and is not what a reader takes from it. The file has no way to say
"implemented and never reached", and that is the state 99 of its 823 green marks are in.

### B-13 — the three things a firm sector does, and this one does none of them (A)

Carried from `docs/VERIFY.md`'s third sweep, and it is the finding that outranks the rest of that
file. Measured over the real world:

|                                                |                                                    |
| ---------------------------------------------- | -------------------------------------------------- |
| parties that have ever taken delivery of PLANT | **0**                                              |
| loans in the world, period 6                   | **96**, against 9,006 firms and thousands of banks |
| companies that have ever published accounts    | **0**                                              |
| research estimates                             | **0**                                              |
| parties that have ever ceased                  | **0**                                              |
| instructions settled over three periods        | 269,139                                            |
| FX revaluation events                          | 1,051,429                                          |
| goods perished in store                        | 51,521                                             |

Nothing is ever built, almost nobody borrows, nobody reports, nobody dies — and the two largest
event streams in the world are revaluing foreign balances and rotting food.

Part I found four contributing causes that VERIFY could not see from the outside, and they are not
the whole of it:

- **nobody reports** — `reporting/publish` requires `isPublic`, and the shares a firm issues are
  held by household cells, so this one should fire; it is unexplained and is the best single lead;
- **nothing is built** — `firms/decide.ts:plan` returns early without an `earnings`-based
  `costOfCapital`, and `requiredOnEquity` needs a share PRINT, which needs a share session that
  cleared, which needs a bank's dealing desk on the other side (A-60);
- **almost nobody borrows** — a firm's own `credit.quoted` read is period-unbounded (A-33), and its
  input bids are inflated by A-34, but neither explains two orders of magnitude;
- **nobody dies** — `fails: []` on households and the treasury (A-36), and `failedWhy`'s solvency
  branch reads an equity account that A-39 is inflating every period.

The diagnosis of the first and third is the next piece of work, and it is not this read's.

### B-14 — a finding was positioned into item 13h, 13h closed, and the finding was not done (B)

`seeds/foundation.ts`, on the derivative layer's list of who may hold a contract, said: _"13h is
where a fund holds derivatives on purpose — and where the one pass that re-marks a fund's claim on
itself is next opened (`docs/BUGS.md`, finding `13b-2`)."_ `docs/RECORD.md` (item 13b.1's entry)
confirms the placement: `13b-2` "to **13h**, folded into two steps there".

13h is **done** on the worklist. `TRADES_CONTRACTS` is still `[BANK, FIRM]`; no fund kind is on it,
and the pass that would re-mark a fund's claim on itself does not exist. The receiving item closed
without the step the positioning was for, and nothing anywhere says so: the record's entry for 13h
does not carry it forward, and the comment in the source went on naming a future that had already
passed and a file that had been deleted.

This is the failure mode of positioning as a protocol. A finding leaves the audit file by being
placed into an item, and from that moment nothing checks that the item ever did it — the finding is
out of the one place findings live and into a plan file that gets deleted when the item closes. Six
of Part II's thirteen findings (**B-1** through **B-8**) have the same shape read from the other end:
an item closed and the thing it was for was not there.

The comment is corrected in this change to say what is true. The finding itself is **unpositioned**:
whether a fund should hold contracts at all is `Fund Shares A3`'s question and it belongs with 13o
(asset managers with strategies), which is where a fund that takes a position on purpose first has a
reason to exist.

### B-15 — the recount, once the gate could run (C)

With the `13j` row restored, `plan:progress` recounts to **92.9% (678 of 730 steps across 51
items)**. The block it replaces published **92.7% (677 of 730 steps across 46 items)** — a figure
generated before the sweep, listing 13j as in progress at 13 of 14 and not listing 13k–13o at all,
because the commit that added those five broke the gate that would have recounted it.

The number barely moved, and that is not the finding. The finding is what it took to keep it from
moving the wrong way. `plan-progress.ts` counted **a deleted plan file as a fully done item**: item
14's plan is now Part IV of this document and its file is gone while the item is open, so the
untouched tool would have published **94.8% (692 of 730)** — fourteen worked steps for an item
nobody has started. The state of an item is written in the worklist's state column and nowhere else
(Law 4); the tool now reads it (Law 19) instead of inferring it from a stat call. For the same
reason 13k–13o rendered as **"closed (no item file)"** — five open items, five rows saying closed.
They say `open (no item file)` now, which is what they are.

The figure is still generous, and this is the honest reading of it: it counts **planned steps
ticked**, and Parts I and II are 85 findings against work those ticks call done. A step is ticked
when its code is written, not when its mechanism has ever run.

---

# Part III — carried in from `BUGS.md`, `SWEEP.md` and `VERIFY.md`

Those three files were holding pens with the same rule as each other: _a finding leaves only by being
PLACED_. They are now here, because three holding pens and an audit is four places a finding can be
and `Law 4` applies to documents too. Nothing was dropped in the merge; what was already in Part I is
named below rather than repeated.

## What was a duplicate

`docs/VERIFY.md`'s six lettered findings were all re-found independently in Part I's read. They are
the same defects, and Part I's entry is the one to work from — in three cases it is wider:

| VERIFY                                                               | Part I            | wider how                                                                                                             |
| -------------------------------------------------------------------- | ----------------- | --------------------------------------------------------------------------------------------------------------------- |
| F1 `conditionsFor`'s claimed assembly guard does not exist           | **A-4**           | —                                                                                                                     |
| F2 `payable` / `cashFor` / `deliverable` ignore their first argument | **A-5**           | —                                                                                                                     |
| F3 the orphan doc block above `STORAGE_SESSION`                      | **A-8**           | —                                                                                                                     |
| F4 `foundation.ts:1102` reads a life off the decl, not the register  | **A-7**           | —                                                                                                                     |
| F5 `sameState` compares liens by count                               | **A-2**           | A-15 adds the two per-party stores `copyMemberState`, `forget` and `sameState` do not touch at all                    |
| F6 `mergeCells` writes the weight before the guard                   | **A-3**           | —                                                                                                                     |
| the insurer exports nothing calls                                    | **A-9**, **B-2**  | A-9 finds the unit bug that makes a policy unregisterable, so the sector is not merely unwired                        |
| securities lending exports nothing calls                             | **A-67**, **B-3** | A-67 adds the two defects inside the unreachable code — the fee cannot clear, and two `Want`s double-post the lenders |
| nine derivative kinds, no contract ever                              | **B-7**           | A-66 gives the structural cause and corrects VERIFY's conclusion                                                      |
| 0 plant, 96 loans, 0 accounts, 0 estimates                           | **B-13**          | —                                                                                                                     |

VERIFY's own line-by-line pass covered 6,407 of 60,000 lines and stopped. Part I is that read
finished, so its "what was read this sweep, and found sound" table is superseded rather than merged:
every file in it was read again and the verdicts stand.

## C-1 — 82 red of 662, by cause (carried from `BUGS.md`)

Measured on the whole suite after 13j. The count is not the finding; the six causes are, and they are
carried verbatim because each names where it goes.

| cause                                            | ≈ red | what it is                                                                                                                                                                                                                                                                                                                                                                                                                                                            | placed                                                                                                     |
| ------------------------------------------------ | ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| **the rig has no firm in most lines**            | 20    | `RIG_PER_LINE` is 0, so twelve firms over sixty-two lines leave most empty and any test asking the draw for a mill is told "it drew 0". Measured: setting it to ONE takes the suite from 79 red to **139** — a rig with every line in it is a different small world, and a dozen files' assertions are written against the one it makes today. Those tests are not wrong, they are SPECIFIC.                                                                          | item **16**, as one bounded change: re-size the scale model and re-derive what every affected test asserts |
| **the foreign countries are stubs**              | 6     | see **B-9**: this contradicts the same file's header, and which of the two is current is not established                                                                                                                                                                                                                                                                                                                                                              | item **16**, after B-9 is settled                                                                          |
| **a bank that is insolvent is never resolved**   | 9     | `13b-12`: a bank published capital of −31,237,415,456 and went on making a market for twenty-three periods. Both triggers must exist and the resolution must name which fired (Banks Capital C1.a). Whether the solvency trigger is not reading the published position, or is reading it and the resolution is not being run, is **still unmeasured**                                                                                                                 | its own diagnosis, before **16**                                                                           |
| **an ETF cannot create**                         | 7     | `12d-8`: a creation delivers a slice of the book and a desk without the basket does not create. Nothing moves a share back to a desk, so `E3` runs one way and the premium — 0.28 of NAV, twenty-six times a period of carry — has nobody able to close it. The answer now EXISTS (securities lending), and wiring it is a BUILD not a fix: a module may not import another, so "whoever must deliver may borrow" needs a kernel door of the shape `termsOffered` has | with **A-67**: the borrow market has to be reachable first                                                 |
| **two banks, and everything that needs a third** | 4     | `research` wants coverage to VARY, `deposits` wants a class to split rather than cross, `dealing` wants a market to fail when the desks step back. With three banks and one listed line a count that should be an outcome has one value                                                                                                                                                                                                                               | item **16**, same scale-model question                                                                     |
| **singletons**                                   | 9     | `omo` remittance and run-off (see **A-61**, **A-62**); `raise`; `ratings` ageing; `equity-anchor`; `indices`; `tick`; `environment`'s crop; `treasury`'s receipts (see **A-46**); `world`'s year-long chain. Each needs reading on its own                                                                                                                                                                                                                            | item **16**                                                                                                |

**And the eight 13j cost.** Three measurements of the same suite: 74 red before 13j; **303** with the
rig opening all four countries; **82** with the rig opening one and the currency tests four. The 303
is a finding and not a bug — a dozen firms and three banks over four countries gives each a country
with no banking system its own depositors could fund, and `Seed D1` refuses exactly that ninety-four
times — so a world's count of countries is a RESOLUTION like its count of banks. The eight that
remain are all one shape: **a test that named the world it was written against**, in `currency`,
`spot-fx`, `omo`, `indices`, `opening-liquidity`, `bank-capital`, `deposits`, `money-market`, and a
long tail of sizes and totals. Positioned to **16**.

## C-2 — four things this world does every week that the world does not (carried from `SWEEP.md`; worklist **13k**)

Law 8 says the periodicity is part of the number. A weekly period is the resolution; it is not a
licence to do everything weekly. Re-verified at the source in this pass.

- **A dividend is declared and paid every week.** Period 5 settles 249,288 instructions and
  **162,615 of them are dividend payouts — 65% of everything the world does**. `equity.decide` is
  `cycle: 0, anchor: { after: 'firms.decide' }` and runs every period; `decideEquity` distributes
  `spare / patience` each time. A board declares with its results on a fiscal calendar; between the
  declaration and the payment the dividend is a LIABILITY and the share trades EX, which is why total
  return and price return are different numbers. There is no declaration date, no ex date, no record
  date, no payable date and no dividend liability. **The machinery is built and unused**:
  `reporting/fiscal.ts` has `quarterClosedBy`, `anchorOf` and `publishableOn`. A quarterly dividend
  is also a world that does what it does in about a third of the settlements.
- **A rating fee is charged every week.** 16,527 `pays X for its rating` instructions in period 5;
  `ratings.assess` is `cycle: 'anchor'`, every period, and calls `collectFees` each time. An issuer
  pays an issue fee once and a surveillance fee annually. Weekly billing makes the assessor's income
  a flow of the issuer's equity rather than a price for a service, which is the conflict Ratings A5
  exists to keep.
- **Tax is levied every week.** 7,778 `tax due from X` instructions in period 5. Payroll withholding
  is weekly and right; corporation tax is assessed on a fiscal period and VAT quarterly. Levying
  everything weekly removes the working-capital consequence of a tax bill, which is the thing a
  treasury and a firm both plan around. (See also **A-46**: the base is wrong as well as the
  cadence, and **A-63**: the consumption half of it collects nothing at all.)
- **A buyback is decided weekly.** `decideEquity` chooses between a dividend and a buyback each
  period out of this week's spare cash. A buyback is an announced PROGRAMME executed over time, and
  announcing it is the event.

## C-3 — four central banks administer one policy rate (carried from `SWEEP.md`; worklist **13l**)

Re-verified at the source in this pass:

```ts
function corridor(ctx: MechanismContext): Corridor {
  return corridorOf(
    ctx.params.perAnnum(MM_PARAMS.policyRate), // 'centralBank.policyRate', one row, 0.02
    ctx.params.perAnnum(MM_PARAMS.floorSpread),
    ctx.params.perAnnum(MM_PARAMS.ceilingSpread),
  );
}
```

`corridor` takes no currency. The Fed, the ECB, the Bank of England and the Bank of Japan all
administer 2%. With no interest differential between two moneys there is no carry, so an FX forward
prices flat to spot, covered interest parity says nothing, the cross-currency basis has nothing to be
a basis against, and the carry trade — the largest real FX flow there is — cannot exist. Four
countries with identical policy is not an approximation of the world; it is the one assumption that
switches the whole currency layer off. A rate is a POLICY primitive and each central bank owns its
own.

It compounds with **A-66**: FX forwards is the one derivative book that can open, and `carryOf` is
what its two-way maker quotes around.

## C-4 — the sectors that are not there (carried from `SWEEP.md`)

| what                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | state  | worklist |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | -------- |
| **The built environment.** Warehouses exist only as `STORAGE` plant a firm builds for its own stock — nobody builds space to LET, so there is no landlord, no commercial rent, no lease with a term and no CRE lending. Commercial buildings — the shop, the office, the works — do not exist as assets at all; a retail firm in this world sells from nowhere. **Ports do not exist**: `freight` sails a vessel region to region with no berth, no quay, no congestion and no owner. **Building consumes no land and never gets dearer**: `geography.ts` does the right thing for resource yield (the return from the next unit falls continuously, so what stops an expansion is a firm's hurdle and never a refusal by the map) and nothing does it for built space — a tile carries any number of buildings at the cost of the first. | ABSENT | **13m**  |
| **No firm is ever born.** `estate` kills firms and nothing creates one; the draw fixes the population at the seed and it only falls. Entry is what makes a market contestable, so a surviving firm's margin is never competed away and every concentration measure is one-way. Spec 34 is half built and the birth half has been PLACED forward three times (13f → 13g → 13h).                                                                                                                                                                                                                                                                                                                                                                                                                                                            | ABSENT | **13n**  |
| **No asset manager runs a strategy.** 13 funds; after four periods **4 hold anything at all**, 8 holdings between them, against 1,274 subscriptions settled in period 4 alone. Every fund is a money fund, one commodity fund, or an index tracker — and the trackers hold nothing. No hedge fund module (28), no private equity (29), no prime brokerage (15). The basis trade is measured by `bond-futures` and repo exists, and there is no party whose reason is to put the two ends together. **No fund holds credit**: `eligible` is `['sovereign.bill']` or `['good.grain']`, so nothing holds a corporate bond, a sovereign BOND or a credit index — and `CREDIT_INDEX(ccy)` is measured every period with no tracker able to take a position in it.                                                                              | ABSENT | **13o**  |
| **A firm cannot issue commercial paper.** Spec 9 has no module. `money-market` has interbank rows and repo, which is the BANK's short-term funding; a FIRM funding itself at three months and the roll that can fail is what the clause is about. Carried since 13f. See also **B-1**: it cannot issue a bond either, so a firm in this world has exactly one funding channel and it is a bank loan.                                                                                                                                                                                                                                                                                                                                                                                                                                      | ABSENT | carried  |
| **Pensions are insurers wearing the same name.** Spec 27 is "INSURERS AND PENSIONS" and there is one party kind, `insurance`. A pension has a SPONSOR, contributions from an employer and its members, and a funding ratio that is the sponsor's problem when it falls. See **B-2**: the insurer half does not exist either.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | ABSENT | carried  |
| **Small-business pools** (spec 42), carried from 13e and never built.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | ABSENT | carried  |

## C-5 — 12,519 declared parameters are never read (carried from `VERIFY.md`)

29,559 declared, 17,040 read in a run. Discounting the ones read at seed time before the instrument
was attached, the substantive ones:

| never read | what it is                          |
| ---------- | ----------------------------------- |
| 5,896      | `firm.hurdle.<firm>`                |
| 5,896      | `firm.horizon.<firm>`               |
| 493        | `equity.payoutPatience.<firm>`      |
| 30         | `bank.lending.capitalAtRisk.<bank>` |
| 9          | `fund.requiredYield.<fund>`         |
| 7          | `fund.fee.<fund>`                   |

VERIFY's own correction stands and is the point: the hurdle IS read, at `firms/decide.ts:461`, but
inside `cost.some ? project(...) : none()` — so a firm whose `costOfCapital` is `none` never
evaluates a project at all. **9,006 firms alive, 3,104 ever evaluate one.** Two thirds of the economy
never makes an investment decision, because no bank has quoted them and no market prices their
equity — which is B-13's first and second causes seen from the parameter register.

## C-6 — the margin gate: 31,640 admission decisions, 31,640 refusals (carried from `VERIFY.md`)

The one measured cause of a dead book, and it is a real missing mechanism rather than a bug. Over two
periods, diagnosed against the three gates in `capacity().admits`:

| gate                                                         | count      |
| ------------------------------------------------------------ | ---------- |
| the clearing house has ceased                                | 0          |
| **no room — the party holds no cash in the book's currency** | **31,640** |
| the kind cannot say what one unit's margin is                | 0          |
| admitted                                                     | **0**      |

`capacityOf(view, ccy, buffer)` is `cash − cash × buffer`, so room is zero exactly when the party
holds none of that money. Every one of those is an FX forward: a party trading EUR/GBP must post
margin in EUR or GBP and holds neither. In the real world it would buy the currency or post eligible
collateral in another one; here there is no such path.

Two things follow that VERIFY did not draw:

1. It is the whole story for **one** book, not for nine — see **B-7**.
2. The missing mechanism is **eligible collateral in another money**, which `money-market/collateral.ts`
   already implements for the repo market (`advances`, `valueToLender`, `haircut`). The derivative
   layer's `admits` asks only about cash.

## C-7 — the confidence question, answered and closed (carried from `VERIFY.md`)

Recorded because it is a hypothesis that was tested and **disproved**, and the next reader should not
test it again. Twelve periods, the same world:

| period | outlook rows | rows with confidence | contracts | loans |
| ------ | ------------ | -------------------- | --------- | ----- |
| 1      | 30,650       | 0                    | 0         | 6     |
| 2      | 64,803       | 0                    | 0         | 6     |
| 3      | 101,998      | 12,067               | 0         | 6     |
| 4      | 126,191      | 21,223               | 0         | 8     |

`width(surprises)` is zero for a party with one surprise or none, so confidence does not exist until
period 3 and then arrives in bulk. A fifth of all outlooks carry it by period four and the share is
rising, and still not one contract exists. **Confidence is not the gate and never was.**

(Part I found the one place where confidence IS load-bearing and dimensionally wrong: **A-65**, the
option premium, where it is multiplied by the price level instead of used as the width it is.)

---

# Part IV — item 14, the polity: folded in from `docs/plan/14-polity.md`

The plan file is deleted and its content is here, because the item's two carried findings are audit
findings and the rest of it is a design that has to survive contact with what Parts I–III found.
When the item is opened, this is its plan; `docs/WORKLIST.md` row 14 points here.

**Objective.** The fiscal and regulatory POLICY primitives get their subject: a parliament of a fixed
number of seats, a few parties each with a platform that is data, an electorate of household cells
each casting its weight from its own state and its own outlook for the platform that leaves it best
off, seats by one stated rule, a government by one stated coalition rule, and a mandate that is a
read of the parliament and the only writer of every parliament-owned policy in the register. Until
now those policies have a standing mandate declared at the seed; from here the register prints their
owner beside their value and nothing else can set them.

**Read first.** §47 Polity (all); XI-17; §30 Treasury B1, B3, C1, D4.b; §31 A3, A4; §46 (the cell's
outlook); §45 A5, B1, B2.a; XI-15; Law 2; Part XII. Code: `registry/params.ts`, item 3's `treasury`
programme, item 4's `households` and `expectations`, 13d's cohorts and employment states.

**Clauses.** Polity A1, A2, A2.a, A2.b, A3, A4, B1, B1.a, B2, B2.a, B2.b, B3, B4, C1, C2, C2.a, C3,
C3.a, C3.b, C4, D1–D5, D3.a, E1–E4, F1–F5; XI-17; Treasury B3, D4.b; Central Bank A3, A4.

## The two findings this item carries

### D-1 — a levy that fails is recorded and then forgotten: no arrears (`12d-5`) (A)

Measured in `estate.test.ts` at 14 periods of the rig world: **855 failed tax legs** from household
cells, and nothing else a cell posted ever failed. `treasury/index.ts` settles the levy and, when it
fails, adds it to `unpaid` on `treasury.receipts` — which is right (`Money E1`, `D3`: a payer that
cannot pay has not paid). But `unpaid` is a number in an event and **nothing carries it**: the cell
does not owe it next period, the treasury does not chase it, and the receipt is not short by it in
any account. A tax that failed is a hole between two balance sheets that only the journal knows
about.

The mechanism is **arrears** — a levy that is not paid becomes a claim the treasury holds on the
payer, ranking where the law says — and the law is this item: what a tax is, what happens when it
is not paid, and where the claim ranks in XI-8's waterfall are all fiscal policy with an owner
(Polity D1, D3). It is a claim like any other: an instrument with a named creditor and a named
debtor, carried until it is paid, written off, or ranked in an estate.

**It is the same shape as three findings from Part I and they should be built together**, because
one instrument and one door answers all four: **A-41** (an unpaid wage and an unpaid severance
leave no obligation anywhere and the record says nothing is owed), **A-20** (a failed estate
transfer becomes an unhandled throw because there is nowhere for the unpaid part to live), and the
observation under **A-39** that `world/failure.ts:stillOwed` only counts failures whose
`instruction.period === view.period`, so last period's unpaid amount is gone from the solvency test
too. This world records what did not settle and carries none of it.

### D-2 — the central bank is a marginal price-setter in the sovereign book (`13b.1-10`) (B)

Its open-market desk closes a gap towards a 25%-of-line target by posting a MARKET order — a
quantity with no level — so in every sovereign session where it has a gap it bids at the top of the
book for a quarter of the line. An order with no level is a price-taker of a price the mechanism has
not yet produced (Clearing A4), and a big enough one is the marginal order that sets it. The quantity
limit is real policy (Central Bank C1) and the module correctly refuses to stand in any other market,
so this is NOT Appendix B's buyer of last resort — but it is more aggressive than any real
open-market operation, and it belongs here because what a central bank may and may not do to a price
is this item's subject, alongside its administered rate.

What it needs is the level at which its own reason stops: a schedule (Clearing A2), not a quantity.
Test: the sovereign session's clearing price does not move when the desk's gap is doubled at an
unchanged book.

**Two of Part III's findings are in the same module and should be read with it**: **A-61** (every
central bank remits to `treasuries[0]`, in its own money) and **A-62** (a loss advances the
remittance window, so E4's "it stands there until income covers it" is not what happens).

## Design

### 14.1 Kernel: owners and the mandate door

- Every `policy` parameter already declares an `owner`
  (`parliament | centralBank | standardSetter | constitution`). This sub-item makes the owner
  **load-bearing**: a policy owned by `parliament` can be written only through
  `params.setByMandate(values, effective: Period)` on the `MechanismContext` of the module that
  declares itself the polity (assembly grants the door to exactly one module: D5, C3.a); every other
  write path throws `Forbidden`; the seed's standing mandate is the initial value with
  `setBy: 'seed.standingMandate'` recorded (XI-17); the observer prints the owner and the setter
  beside every policy value (D5).
- The central bank's **rate** stays the central bank's (D4, Central Bank A4); its **target** and
  mandate text are parliament-owned (D4, Central Bank A3): `centralBank.target.*` moves to owner
  `parliament` here.
- **C-3 lands here too.** `centralBank.policyRate` is one row for four central banks. Whether it is
  split into four before this item or as part of 14.1 is the open question — the worklist puts it
  at **13l**, which is earlier, and that is the right place: the polity needs a rate that belongs to
  a named central bank before it can hand one of them a target.

### Module `polity`

- `requires: ['households', 'treasury', 'expectations', 'labour']`.
- **The constitution** (A1, A4, C1): three policy primitives with owner `constitution` —
  `polity.seats` (a count), `polity.termPeriods` (placed on the calendar by date), and
  `polity.allotmentRule` (a named rule in a dispatch table: largest remainder, or highest averages).
- **Platforms** (A2): `registry/platforms.ts`, one row per party with a value for **every**
  parliament-owned policy (assembly throws if a platform misses one or names one the parliament does
  not own); platforms differ (A2.a: assembly refuses two identical rows); a party is not a ledger
  party (A2.b: no account, a name with a seat count).
- **The vote** (A3, B): phase `polity.election` on its date. For each household cell the module
  evaluates every platform **applied to that cell's own state at that cell's own outlook** (B1, B2):
  its expected income under that platform's tax and transfer rates, its expected prices paid at its
  own basket, what it owns and owes. It never reads a published unemployment or inflation rate
  (B1.a) — structurally, through a `withoutAggregates` view variant of the kind item 12 built for
  `withoutPrints`. It picks the platform with the highest expected position; a cell for which every
  platform gives the same position **abstains** (B2.b, the only abstention; B2.a: no turnout, swing
  or drawn share). The cell casts `weight` votes through `integrate` (A3, B3). Turnout is a read.
- **Seats and government** (C): seats by the allotment rule (C1); the government by the coalition
  rule (C2 — the largest party adds the party whose platform is nearest its own, distance over the
  policy vector in each policy's own unit, until it holds a majority); a **hung parliament** when no
  such coalition exists within a stated distance continues the standing mandate and is reported
  (C2.a).
- **The mandate** (C3, C4): the seat-weighted position of the coalition's platforms per policy; a
  journalled `polity.mandate` with date, parties, seats and what changed (C4, E4 — an event that
  causes nothing itself); written through the door with `effective = election + polity.mandateLag`.
  C3.b is an audit contribution: every parliament-owned policy's value equals the standing mandate's,
  every period, exactly.
- **What it controls** (D): tax rates on named bases, transfer rates, the treasury's buffer, the
  outlay programme's size and composition, regulatory ratios and floors, the central bank's target.
  D3.a: no price, quantity or outcome is a parliament-owned policy — assembly throws if a platform
  names a parameter whose kind is not `policy` or whose owner is not `parliament`.
- **Consequences** (E): E1 the treasury programme reads the register (it already does); E2 the chain
  runs through the mechanisms; E3 measured at 16; F4 the **approval rating** is the read "how would
  the cells vote today", computed by the observer with a lag and read by nothing.

### Parameters

`polity.seats`, `polity.termPeriods`, `polity.allotmentRule`, `polity.coalitionMaxDistance`,
`polity.mandateLag` (policy, owner `constitution`); `platforms` (data). No `turnout`, no `swing`,
no `loyalty`, no `bloc`, no `approval` input.

### Audit contributions

- `names`/`flows`: C3.b — register values equal the standing mandate (a check of one writer).
- `units`: votes cast + abstentions = Σ weights of the electorate, exactly (F5).

  **Write this one against `A-14`.** The labour module's population identity is
  `Σ p.weight` compared against `Σ p.weight` and cannot fail. A votes-plus-abstentions identity
  built the same way — one loop, one source, two accumulators — would be the third of its kind in
  this engine. The two records must be independent: the ballots the vote produced, against the
  electorate the parties store holds.

### Files

```
packages/engine/src/registry/params.ts (14.1), registry/platforms.ts
packages/engine/src/world/context.ts (the door granted to one module), world/assemble.ts
packages/engine/src/mechanisms/polity/{index.ts,vote.ts,seats.ts,coalition.ts,mandate.ts,approval.ts}
packages/engine/test/{param-owners,platforms,vote,abstention,seats,coalition,hung,mandate,
                      mandate-writer,spread-changes-seats,election-chain}.test.ts
```

## Steps

- [ ] 14.1 Kernel: policy owners load-bearing; parliament-owned policies writable only by the mandate door granted to one module; the seed's standing mandate recorded as the setter; owner and setter printed beside every value; the central bank's target moved to the parliament; tests (D4, D5, C3.a)
- [ ] The constitution's primitives: seats, term on the calendar, allotment rule from a dispatch table, coalition distance, mandate lag; tests (A1, A4, C1)
- [ ] Platforms as data rows covering every parliament-owned policy and nothing else, differing; a party holds no account; assembly validation; tests (A2, A2.a, A2.b, D3.a)
- [ ] The vote: each cell applies each platform to its own state at its own outlook through a view with no aggregate reads; votes weight for the best; abstains when indifferent; turnout as a read; tests (A3, B1, B1.a, B2, B2.a, B2.b, B3)
- [ ] Seats by the rule; the government by the coalition rule; a hung parliament continues the standing mandate and is reported; tests (C1, C2, C2.a)
- [ ] The mandate as the seat-weighted coalition platform, journalled with subjects, written through the door at the lag; C3.b contribution; tests (C3, C3.b, C4, E4)
- [ ] What it controls: fiscal rates, transfers, buffer, outlay programme, regulatory ratios, the target; never the rate, a price, a quantity or an outcome; tests (D1–D4, D3.a)
- [ ] **Arrears (`D-1`)**: a levy that fails becomes a claim the treasury holds on the payer — an instrument with two named sides, carried until paid, written off or ranked in an estate at the place the law states; the receipt is short by it in the accounts and not only in the journal. **Build it as one door for all four cases**: the unpaid tax, the unpaid wage and severance (`A-41`), the unpaid estate transfer (`A-20`), and the horizon `stillOwed` reads (`A-39`); tests (Money E1, D3; XI-8; Polity D1, D3)
- [ ] **`D-2`**: the open-market desk posts a SCHEDULE and not a quantity — the level at which its own reason stops. Read it with `A-61` and `A-62` in the same module. Test: the sovereign session's clearing price does not move when the desk's gap is doubled at an unchanged book
- [ ] Consequences through the mechanisms only: the treasury programme reads the new numbers; a multi-year scenario shows the deficit changing through named outlays and receipts after an election; tests (E1–E3)
- [ ] Approval rating as a lagged observer read that nothing reads; tests (F4)
- [ ] B4: two seeds with equal weighted-mean income and different dispersion give different seat counts; F5: seats and mandate computed from members, with the two records independent (see the note under Audit contributions); tests
- [ ] Observer: parliament, platforms, votes by cohort and region, the mandate with owner beside every policy; a multi-year run with two elections green; determinism; coverage re-marked; record entry
- [ ] Worklist row 14 → done; delete Part IV from this file; commit and push

## Exit criteria

No parliament-owned policy can be set by anything but a mandate; a cell votes from what it
experienced and can abstain; a hung parliament is a reported outcome; a change of government reaches
the deficit only through the treasury's programme.

## Guard

Polity B1.a, B2.a, C3.a, D3.a, F1–F4; Central Bank A4; XI-17 ("no policy set directly").
