# An independent review of the code, as it stands

Not a plan check and not a progress report: a reading of what is actually there, against the laws in
`CLAUDE.md`, against whether the modelling is real rather than asserted, and against whether a
module can be added without touching a hundred files.

Findings are written as they are found, file by file. Each carries **what** is wrong, **why** it is
wrong (the law or the mechanism), and **what it would take** to be right. A finding here is not a
work item: it is positioned the way everything else is, into the item that owns the mechanism.

Severity: **DEFECT** (a law is broken), **RISK** (it is right today and cannot stay right),
**SHAPE** (the modelling asserts rather than derives), **SEAM** (the module boundary leaks).

---

## `core/num.ts`

**RISK — `dustOf` has a floor, and the comment justifying it describes a failure it cannot fix.**
`dustOf` returns `SMALLEST_NORMAL` (≈2.25e-308) whenever the derived dust is smaller. That is a
floor on a tolerance, which is the one shape Law 6 and Law 7 both warn about, and the comment
defending it is long — which CLAUDE.md itself names as the tell. The defence may also be wrong: it
says the floor stops "a per-member share multiplied back by a weight" from failing an identity, but
at any magnitude a real quantity has, the derived dust is enormously larger than 2.25e-308 and the
floor never binds. It can only bind when the magnitudes themselves are denormal. So either the
comment is describing a bug that was actually fixed elsewhere, or the constant is the wrong one for
the bug it names. **What it would take:** find the case that motivated it and re-derive; if it was
a `magnitude === 0` comparison, say THAT (two numbers both stated as zero compare exactly, and the
floor is for the denormal case only) in one line, or delete the floor.

**MINOR — `moved(..., through = 0)` is a numeric default on a magnitude.** Appendix A says a missing
number is missing. Here the default is defensible — "no magnitude bigger than the move itself" is a
real statement, not an absent one — but it is written as the exact shape the rule forbids, so every
reader has to re-derive that it is safe. **What it would take:** make it explicit
(`through: Option<number>`), or rename it so the zero reads as a statement rather than a default.

**MINOR — `addTo` uses `?? 0`.** Sanctioned by the same reasoning as `zeroIfNone` (a key with no
terms has accumulated nothing) but not marked as such. `zeroIfNone` carries the argument in a
comment and this does not, so the next reader sees the forbidden pattern with no defence.

---

## `core/tick.ts`

**DEFECT — the three rounding doors cast to `Qty` without the check `asQty` exists to make.**
`toTick`, `downTick` and `upTick` end in `... as Qty` after a `Math.round/floor/ceil`. None of them
asks `Number.isSafeInteger`. `asQty` does, `onTick` does, and the file's own header says why it
matters — "Past 2^53 pieces integers stop being exact in a float, and a quantity there is not a
large amount, it is an amount the machine can no longer add up." So `upTick(2 ** 60)` returns a
`Qty` that is not a count of pieces, and every downstream `addQty`/`subQty` on it will throw
somewhere else, at a site that did not make the error. The type's guarantee is advertised as
compiler-checked and is in fact enforced at one of the four doors. **What it would take:** the
three doors return `asQty(rounded, what)` instead of casting. One line each, and the hole closes.

**DEFECT — `splitOnTick` has a silent exit that leaves a residual with no holder.** The
distribution loop ends `if (next === undefined || has === undefined) break;`. If that branch ever
runs, the function returns parts summing to LESS than the total and says nothing — which is exactly
the defect Law 2 names and exactly what this file's header promises the arithmetic "cannot violate
instead of something the audit reports afterwards". It is unreachable today (`remainders.length ===
weights.length >= 1`, and `parts[at]` is always written before the loop), so it is not a live bug —
it is a `noUncheckedIndexedAccess` appeasement written as a silent failure. **What it would take:**
`throw new Impossible('Clearing C3', ...)` in place of the `break`, or restructure so the indices
cannot be optional. A defensive branch that swallows the one outcome the file forbids is worse than
no branch.

**SHAPE — `downToGrain` and `toGrain` return `number`, not `Qty`.** They take counts of pieces and
return counts of pieces — the doc says so in the same sentence ("Both are counts of pieces, so this
is exact") — but the type drops the brand, so every caller re-launders through `asQty`. That is the
habit the `Qty` type was introduced to end, reintroduced two functions after the argument for it.
**What it would take:** return `Qty`, take `Qty`.

**MINOR — `scaleQty(a, times)` does not say `times` is a whole number.** The comment says "a count
by a whole number of them", the signature says `number`. `asQty` catches the case where the product
is fractional, but `scaleQty(2, 1.5)` is 3 and passes, so the stated rule is not the enforced one.

---

## `core/format.ts`

**MINOR — `percent(0)` renders as `"%"`.** `(0).toFixed(3)` is `"0.000"`; stripping `/0+$/` takes
the leading zero with the trailing ones and leaves `"."`, which the next replace removes. A zero
rate displays as a bare percent sign. Nothing reads it back, so it is a display defect only — but
Law 9 says a priced thing shows its price, and "%" is not one. **What it would take:** strip only
zeros that follow a decimal point (`/(\.\d*?)0+$/`).

## `core/rate.ts`, `core/option.ts`, `core/assert.ts`, `core/errors.ts`

Clean. `Periodicity` is a closed union carrying its own label, `Option` is the single absent-value
channel, the error classes each name the discipline they enforce. The one loose thread:
`months(n)` does not check `n` is a positive whole number, so `months(1.5)` is constructible and
would reach the day-count arithmetic as a real periodicity.

## `core/ids.ts`

**SEAM — the kernel's identifier vocabulary grows every time a module is added.** `CohortId` is
the households module's word; `CurveFamilyId` is the benchmarks module's; `VenueId` exists because
four mechanisms clear something themselves. They sit in `core/`, which the whole engine imports, so
every new family of names is a change to the most-depended-on file in the tree. It is a small cost
per id and it is exactly the "modify a thousand files" complaint in miniature: the kernel is
learning module nouns. **What it would take:** one opaque `Brand<string, 'ModuleKey'>` that a
module brands for itself, or accept the growth and say so — but today it is neither decided nor
written down.

**MINOR — composite ids are built by string concatenation with no escaping.**
`moneyInstrumentId(issuer, ccy)` is `money:${issuer}:${ccy}` and `fxPairId` is `fx:${base}/${quote}`.
A party id containing `:` (nothing forbids one; `partyId` only rejects empty) makes two different
parties' money collide into one instrument id — one holding, two issuers, which is the Law 4
violation the whole register is built to prevent. Not reachable from today's seeds, and silent if
it ever is.

---

## `calendar/`

`civil.ts` is clean: no clock, no time zone, proleptic Gregorian arithmetic, and the one rounding
it does (the 31st into a 30-day month) is argued as a calendar convention rather than a bound,
correctly. `daycount.ts` reads dates and never a count of periods, as G3.c asks.

**DEFECT (stale comment, Law 16) — `nextCycle` does not do what its doc says.** The doc: "the first
period at or after `at` that is a whole number of `every` from the epoch". The code:
`Math.ceil((at + 1) / every) * every`, which is the first such period STRICTLY AFTER `at`. With
`every = 4`, a contract written in period 4 lands in period 8, not 4. The code is probably right
(a contract cannot expire in the period it is struck) and the comment is wrong — but a reader
sizing a ladder from the comment gets the wrong book. **What it would take:** say "strictly after,
because nothing is struck into the cycle it settles in", which is the real reason and is worth
having written down.

**RISK — `Calendar.schedule` loops forever on a zero periodicity.** `months(0)` is constructible
(`core/rate.ts` does not check `n`), and `advanceTimes` then returns `start` every time, so the
`for (let k = 1; ; k += 1)` never reaches `end`. A hang is the worst failure this engine can have,
because it is the one that reports nothing. **What it would take:** the check belongs in `months`,
which is where the number is declared — one `positiveCount`.

**MINOR — `place` is `periodOf` under a second name.** Law 4 is about facts, not functions, but
a second name for one mapping is the same habit: two call sites now differ in vocabulary for no
difference in meaning, and the doc on `place` describes a rule ("the first period on or after its
date") that is not what `periodOf` computes for a date inside a period.

---

## `registry/grid.ts`

Clean, and the long note on why a TICK is a technology and a PIECE is a resolution is the best
piece of prose in the tree: it is a measurement that contradicted the item's expectation, written
down as what was found rather than what was hoped. `PIP_YEN` is a per-currency fact read once by
the seed, not a branch in a mechanism, so it is data and belongs here.

## `registry/params.ts`

**DEFECT — Law 8 says the unit is part of the number, and every read drops it.** `ParamDecl.unit`
is a free string ('periods', 'days', 'per annum', 'ratio of the position'). The constructor checks
it is non-empty and nothing ever reads it again: `get(id)` returns a bare `number`. So a caller
that reads a parameter declared in 'periods' and multiplies it by a day count is not wrong at any
site — and the one guard that exists (`amount()` vs `get()`) separates only AMOUNTS from
everything else, not periods from days from ratios. The register is the one place in the engine
where every behaviour-shaping number is declared with its unit, and it is also the one place where
the unit cannot be checked. **What it would take:** a small closed vocabulary
(`'periods' | 'days' | 'perAnnum' | 'ratio' | 'count' | 'amount'`) and a read that names what it
expects — `params.periods(id)`, `params.ratio(id)`. That is a mechanical change across every read
site, which is why it has not happened; it is also the difference between a declared unit and a
commented one.

**SHAPE — `namesAnItem` polices prose with a regex.** `/\bworklist\s+[0-9]/i` catches "worklist
13h" and misses "item 13h", "13h builds it", "when the credit model lands". The intent is right
(a shape with a scheduled death IS a placeholder) but the test is on wording, so it enforces a
house style rather than the rule. Law 12: a check standing in for a structural fact is a symptom
patch. It is worth keeping — it caught real cases — but it should be named for what it is (a lint
on reasons) rather than presented as the guard for Law 2's distinction.

---

## `registry/registry.ts`

**DEFECT — derivative kinds are registered and never validated; instrument kinds are validated six
ways.** The constructor checks, for every instrument kind: derived-implies-derives, priced-implies-
no-derive, quoted-implies-a-tick, unquoted-implies-no-tick, and tick > 0. For every derivative kind
it checks only that the id is unique. Nothing checks `priceTick > 0`, and nothing checks that
`DerivativeKindProfile.unit` is a unit this registry declares — so a module that registers a kind
whose unit was never declared fails later, inside `subdivision` during a price conversion, with
`unit undefined does not exist` and no indication of which kind asked. (That failure happened in
this build, from an import cycle leaving a unit constant undefined at module-init time; it cost an
afternoon precisely because the error names the symptom and not the registrant.) **What it would
take:** the same loop the instrument kinds get — `units.has(k.unit)` and `k.priceTick > 0` — six
lines, at assembly, naming the kind.

**RISK — `tickShift` is never validated.** `pieceShift` is checked per unit inside `piecesPerUnit`
(integer, >= 1). `tickShift` is stored raw and then used as a divisor in `tickFor`,
`tickForDerivative` and `rateTickFor`. A zero makes every tick `Infinity`; a negative one inverts
every grid. Both are assembly-time mistakes that should be refused at assembly.

**SHAPE — `payable`, `cashFor` and `deliverable` take an argument they do not use.** All three
signatures open with `_ccy`/`_unit` and all three are `downTick`/`toTick` of the second argument.
Every module in the tree calls them, so every module is passing a currency into a function that
cannot see it, and reads as though the rounding were currency-aware. It is not: by the time a
number reaches here it is already a count of that money's pieces, which is the whole achievement of
the Law 8 grid. **What it would take:** either drop the parameter and let the call sites read
honestly, or say in one line why it is kept (it is a real argument — it keeps the call site naming
the money, and leaves room for a money whose rounding is not symmetric) and stop the `_` prefix
implying it is vestigial.

**MINOR — `cohort(id)` is a linear scan** over an array in a class whose every other lookup is a
map. Cohorts are few, so this is not a cost; it is the one place the file's own pattern is broken.

---

## `registry/kinds.ts`

This file is where the "why does adding a module touch a thousand files" question gets its clearest
answer, and the answer is: **because a module's needs are declared as fields on the kernel's
profile types.**

**SEAM — `PartyKindProfile` has grown a field per module that needed to ask a question about a
kind.** `terminal` (estate, XI-8), `fails` (XI-3), `borrows` (bank lending), `depositClass` (bank
funding), `sovereign` (indices/ratings), `moneyIssuer.overdraft` (settlement). Each is correctly
argued — a fact about the kind belongs with the kind, and the alternative really was a table inside
the asking module — but the consequence is that the kernel's central type is a union of every
module's questions, and a new module that needs a new fact about a party edits `core`-adjacent code
that everything imports. That is the mechanism behind the complaint: modules are plug-and-play for
BEHAVIOUR (a profile, a phase, a participant) and are not plug-and-play for VOCABULARY.
**What it would take:** one `facts: ReadonlyMap<FactKey, unknown>` on the profile, declared by the
owning module and read through a typed accessor the module exports — the module owns both the
question and the answer, and the kernel owns neither. That is a real design decision with a real
cost (the compiler stops checking the shape at the declaration site), which is why it should be
DECIDED rather than drifted into. Today it is drifting.

**DEFECT — `OverdraftDecision` is a boolean, and the obligation that makes it legal lives in a
comment.** Money B3 is: somebody lends it at a rate, or somebody refuses and the refusal is
recorded. The type says `{ allow: true } | { allow: false }`. Who lends, and at what rate, is not
in it — the doc says "Whoever ALLOWED it writes the row that prices it before the period closes",
which makes permitting and booking two acts in two places with nothing tying them together. Both
implementations do write the row (`banks/index.ts` `bookOverdrafts`, `money-market/index.ts`
`bookOverdrafts`), at the END of the period, so between the allow and the close there is money
issued to a holder with no lender row behind it — and if a phase in between ends the period, or a
party dies, or the module's close phase is trimmed out of a small world, the row is never written
and the audit reports a negative balance with no lender. That is bug `13b-1` in `docs/BUGS.md`,
still open, and this is its cause rather than its symptom. **What it would take:** the decision
RETURNS the lending — `{ allow: true, lender, rate, instrument }` — so that permitting an overdraft
and booking the loan are one act the settlement applies, and "an account below zero with no lender"
becomes unrepresentable rather than audited.

**SHAPE — `depositClass: string | null` moved the branch, it did not remove it.** The comment
argues, rightly, that a table inside the funding market mapping party kinds to deposit classes was
a kind branch written out. What replaced it is a free-form string on the profile and a table inside
the funding market mapping STRINGS to what they mean. The branch is the same size and now it is
untyped: a kind declaring `'retail '` with a trailing space is a silently different class.
**What it would take:** the taxonomy is the funding market's, so let the funding market declare the
closed set and the kind reference it — or accept the string and validate it at assembly against the
set the owning module registered.

---

## `registry/profiles.ts`

Clean, and the note explaining why the kernel holds the IDS of kinds whose PROFILES live in modules
is the right call written down properly. `moneyKind` states `accrued: () => 0` and `cashFlows: () =>
[]` with reasons rather than as stubs.

## `registry/grades.ts`

**SHAPE — `middleGrade` breaks a tie in favour of the better grade, and does not say so.** With an
even number of published opinions `(ranks.length - 1) >> 1` takes the LOWER index, and the scale is
best-first, so two assessors split between `bbb` and `bb` produce `bbb`. The doc argues the middle
opinion is what makes a downgrade contestable — "a name moves across a boundary when a majority say
it has" — but with two assessors there is no majority and the code silently picks the optimistic
side. In a world where a downgrade triggers a margin call, a mandate sale and an index exclusion,
a systematic bias towards the better grade delays every one of those. **What it would take:** state
the rule (it is a real convention — the second-best of three, the lower of two, is what the actual
market does, and it is the PESSIMISTIC one) and pick it deliberately.

## `registry/derivatives.ts`

**SEAM — the registry now imports the world and the clearing layer.** `DerivativeKindProfile.orders`
takes a `ParticipantView` (from `world/context.ts`) and a `MarketDecl` (from `clearing/market.ts`)
and returns `Order[]` (from `clearing/solver.ts`). So the DATA layer, which everything imports,
depends on participation and on the order book. `InstrumentKindProfile` has no such field: an
instrument's participants are declared by the module that owns them, through `SystemModule`.
The `orders` hook exists for a real reason (Clearing A2 — one party shows one face to one book, so
a contract book's schedule cannot be assembled from six modules) and it works, but it was solved by
moving behaviour INTO the registry rather than by giving the layer a way to collect reasons.
**Worse, it contradicts this same file's own argument two interfaces up:** `ContractReads` is
documented at length as the kernel's public reads and "no view of anybody", because a mark must
answer the same from both sides. `orders` hands the profile a party's private view. Two fields of
one interface now have opposite rules about what a profile may see, and only a comment separates
them. **What it would take:** `orders` belongs on a `DerivativeClassModule` the layer collects at
assembly, keyed by derivative kind — same dispatch, same "one face", and the registry goes back to
being data.

**MINOR — `quotedAs?: 'money' | 'rate'` with "absent means money".** An optional whose absence is a
default value is the `?? 0` habit expressed in the type system. Appendix A's rule is that missing is
missing; here missing means one of the two answers.

**MINOR — an index is identified by a bare `string`.** `Underlying` `{ kind: 'index'; index: string }`
and `ContractReads.index(id: string)`, in a codebase where every other identifier is branded and
`core/ids.ts` exists precisely so a `MarketId` cannot be passed where an `InstrumentId` goes.

**MINOR — `Contract.notional` is a `number`, not a `Qty`,** while its own doc says it is "a count of
contracts, each for one unit of the underlying" — the exact thing `core/tick.ts` introduced the
brand for.

**MINOR — `premiumPerUnit(struckAt, terms)` is the one profile function that does not take
`(c, at, reads)`.** It cannot need a period today; the inconsistency costs a reader a lookup every
time and will cost a signature change the first time a premium depends on anything dated.

---

## `register/instruments.ts`

**DEFECT — `restate` does not invalidate the `all()` cache.** Every other mutator on `Instruments`
(`add`, `adjustIssued`, `markDefaulted`, `reseat`, `cease`) ends with `this.everything = undefined`.
`restate` — the share-split door — does not. After a split, `all()` keeps returning frozen records
carrying the PRE-split `issued` count, for the rest of the run, to everyone who asks: the ownership
audit family, the observer, every derived value that walks the instrument list. Holdings are
restated in the register and the issued count in the cached copy is not, so "holdings sum to issued"
is being checked against a stale number. This is a Law 4 violation produced by a Law 18 optimisation:
the memo IS a second copy of the register, and the file's own comment two fields up says why an
index must hold ids and not records — "a record is replaced when something is issued or redeemed,
and an index of stale copies is a second register". The memo is exactly that. **What it would take:**
one line in `restate`; and then the harder question of whether `everything` should exist at all,
because the argument against it is written in this file and was not applied to itself.

The rest of the file is exemplary: one writer per fact, `markDefaulted` idempotent with the reason
stated, `reseat` moving the by-issuer index with the record, `cease` refusing while anything is
outstanding.

## `register/contracts.ts`

**DEFECT — `Contracts.open` never validates the terms, and never checks that they belong to the
kind.** `Instruments.add` does both: `terms.kind === decl.kind` (cited Law 4) and
`profile.validateTerms(decl.terms)`. `Contracts.open` checks only that the two sides differ and that
the notional is positive. `world.ts` validates a contract MARKET's template terms at assembly, which
is a different object from the row settlement later opens. So a row can be written with
`kind: 'cds'` and IRS terms, and the failure is silent rather than loud: every class's mark begins
`if (!isCds(c.terms)) return 0`, so the contract sits open on two balance sheets, marked zero on
both sides, passing the zero-sum family forever. **What it would take:** the two lines
`Instruments.add` already has, in `open`.

**DEFECT — a contract's notional is not on the grid.** `Instruments` routes every change to `issued`
through `onTheGrid` and explains why: "a fractional issued count is a fractional holding somewhere
or an identity that cannot close". D2 says a notional is a count in the kind's own unit, and
`open` checks only `> 0`. A fractional notional multiplies into every mark, every margin call and
every close-out, and the zero-sum family will still pass because both sides are equally fractional
— so this is the failure mode Law 8 was built to end, alive in the newest store.

**MINOR — `open_()`.** The trailing underscore is there because `open` is taken. Two doors of one
store whose names differ by punctuation is the kind of thing that gets misread at a call site; `live`
or `openRows` costs nothing.

**MINOR — `all()` rebuilds the array on every call and `open_()` filters it,** in a store the
zero-sum family walks every period. `Instruments` memoizes the same read. Not a law — but the two
stores in one directory do the same thing two ways, which is the seam Law 4 is about even when the
subject is code rather than facts.

---

## `register/register.ts`

This is the best-argued file in the engine and it carries the largest single piece of dead
machinery in it.

**DEFECT — `debit` can delete units from the register with no instruction and no counterparty.**

```ts
if (remaining === 0 && h.lots.length > 0) {
  const left = sum(h.lots.map((l) => l.qty));
  if (left.value <= dust) h.lots.length = 0;
}
```

`h.lots.length = 0` destroys a positive holding. Not transfers it, not redeems it — deletes it.
Appendix B: no residual with no holder; Law 5: every flow has two sides. The defence in the comment
("dropping it is arithmetic, not a transfer") was true when quantities were continuous. **It is no
longer true**, because Law 8 landed: every lot quantity passes `onTheGrid` at every door in this
same file, so a residue is a whole number of pieces and the smallest one is 1. `dust` here is
`dustOf(lots + 2, |qty| + |free|)`, which is about `3 x 2.2e-16 x magnitude` — below 1 for any
magnitude under roughly 1e15 pieces. So the branch is unreachable in this world, and where it
becomes reachable (a holding of ~4.5e15 pieces) it silently destroys units.

**And the same argument kills the rest of the dust apparatus in this file.** `deliverable` is
`qty <= free || qty - free <= dust`: with integers `qty - free` is at least 1 when it is positive,
so the second clause is dead. The `take` selection in `debit`, the two `if (remaining <= dust)
remaining = 0` lines, `deliveryDust` itself, and the `encumbered - after <= dustOf(2, ...)` clause
in `moneyDelta` are all dead for the same reason. `core/tick.ts` states this outright: integers
"add, subtract and compare EXACTLY... the checks that compare it need no tolerance at all rather
than a derived one". The register is the place that sentence was written FOR, and it still carries
the tolerance the sentence retired. **What it would take:** delete `deliveryDust` and every use of
it, delete the lot-clearing branch, and let `deliverable` be `qty <= free`. Law 12: the fix removes
code. This is about forty lines of it, and it is also the one place a unit can leave the world
unaccounted.

**DEFECT — `stateEquity` dates every opening entry to period 0, including for parties born later.**

```ts
this.equityLedger.set(party, [{ party, period: 0 as Period, cycle: 0 as Cycle, ... }]);
```

`stateEquity` takes no period. A firm incorporated in period 40, a bank's successor estate, a cell
promoted out of a split — each gets an opening equity entry dated to the epoch. Two consequences:
`equityEntries(party, from, to)` over any span after its birth omits the opening, so a report of
what a young party earned is missing its starting point; and Reporting G2's "Σ every entry is the
balance, exactly", which this file's own comment claims as the reason the opening is itemised at
all, is only true when the whole history is asked for. **What it would take:** `stateEquity` takes
the period and cycle it is called in — every caller has them.

**RISK — a share split is one fact written by two stores through two calls.** `Register.restate`
multiplies every lot and lien; `Instruments.restate` multiplies `issued`. Nothing binds them: a
caller that makes one call and not the other leaves holdings and issued permanently inconsistent,
and the ownership family reports it in a period with no other cause. The split door on the world is
correct today. Law 4 asks for one writer of one fact, and the fact here ("the count of this line is
restated") has two. **What it would take:** one door that takes both stores, so the pair cannot be
half-made.

**MINOR — `forget(party)` deletes a party's holdings, equity account and ledger without checking
any of it is empty,** on the strength of a comment saying the caller has verified. It is the one
door in the file that trusts its caller instead of guarding the store, and the file's own argument
for `onTheGrid` ("guarding the STORE rather than each writer is what makes this a rule instead of a
habit") applies exactly.

---

## `ledger/instruction.ts`

**DEFECT — the wire does not use `Qty`, and `core/tick.ts` says it does.** That file's argument for
the branded type names its call sites explicitly: "Everything that CARRIES a quantity asks for this
type: an order's size, **a leg's amount**, **a lot**, **what is issued**, what a seed endows."
Three of those five are plain `number`: `MoneyLeg.amount`, `AssetLeg.qty`, `CreateLeg.qty`,
`DestroyLeg.qty`, `PledgeLeg.qty` and `CellSide.perMember` here; `Lot.qty` in `register.ts`;
`Instrument.issued` in `instruments.ts`. So the compiler-checked guarantee the type was introduced
for — "a module that divides money by a price now gets a `number` and cannot put it anywhere a
quantity goes" — does not hold at the wire, which is the one place every quantity in the world
passes through. What catches a fractional leg today is the runtime `onTheGrid` in the register: the
far-end guard the `Qty` doc says was the problem. **What it would take:** change the six field types
and follow the compiler. It is a large mechanical diff and it is the difference between the rule
being enforced and being described.

**DEFECT — a failed contract close or novation is not filed under the parties it belongs to.**
`subjectsOf` on an `act: 'open'` leg adds `a` and `b`; on `'close'` it adds only the CONTRACT ID,
and on `'novate'` only the contract id plus `from` and `to` (never the untouched counterparty).
`Ledger.failedBy` is built from `subjectsOf`, and `failedFor(party, since)` is what a party reads to
know what it failed to pay and what an assessor reads to grade an issuer (§44 A2, Ratings A2). So a
close-out that failed — the single most informative failure a derivative counterparty can have — is
invisible to both sides. **What it would take:** resolve the contract's sides when the leg names a
row, or carry them on the leg.

**MINOR — `subjectsOf` returns `string[]` and mixes three brands in one set.** Party ids, instrument
ids and now contract ids share a key space that `core/ids.ts` exists to keep separate. It works
because every brand is a string underneath; a party and a contract that ever shared a spelling would
silently share an index entry.

---

## `ledger/settlement.ts`

The expansion is right where it matters most: money is created only by an `issue` op and destroyed
only by a `redeem` op, both reachable from exactly three places — a money leg whose payer is its own
issuer, a money leg whose payee is its own issuer, and the cross-issuer pair (the payer's bank
destroys its deposit and gives up reserves; the payee's bank creates its own and takes them). A
central bank's own money never round-trips through creation on a transfer between two of its
account holders, which is correct and is easy to get wrong. Physical units enter only through
`exist`, only from a `create` leg, only on a kind that declares itself physical, and only under
cause `production` or `seed`. **On the user's question — is money created and destroyed only at the
right places — the answer from this file is yes.** The findings below are elsewhere.

**SHAPE (and the most consequential finding in this review) — an issuer books a PROFIT when its own
debt falls in price.**

```ts
if (this.d.registry.instrumentKind(inst.kind).liabilityOfIssuer) {
  bump(issuerOf(inst), mul(op.totalQty, carryingOf(op.fromDebit) - basis, 'issuer re-mark'), inst.id);
}
```

On every holder-to-holder trade of a claim, the issuer's equity moves by
`qty x (what the seller carried it at − what the buyer paid)`. So when a firm's bonds trade DOWN —
which is what happens as it approaches distress — `carrying − basis` is positive and the issuer's
equity RISES. This is the own-credit gain, and it is famous precisely because it is perverse: a firm
on its way to insolvency books profits from the market's growing doubt that it will pay. It fights
XI-3 directly (nothing immortal — a firm cannot become insolvent if its own distress is a revenue
line), and it fights §31/Banks Capital the same way for a bank whose subordinated paper is falling.
The citation given is Register B3, "a liability is the same number read from the other side", which
is true about the BALANCE SHEET and does not settle where the change goes. Real accounting sends
own-credit movements to other comprehensive income exactly so they do not flow through profit, and
this world has a reporting system (§48, item 12a) that could hold that distinction. **What it would
take:** the own-credit component of the re-mark lands in a named account of its own (the
revaluation account already demonstrates the pattern), not in the equity line that says what the
issuer earned. This is a real modelling decision and should be recorded as one rather than arrived
at through a one-line bump.

**DEFECT — there are two overdraft guards and the one in the register cannot fire.** `apply` calls
`register.moneyDelta(..., allowNegative: true)` at both money sites, unconditionally. The register's
`forbid(allowNegative || after - encumbered >= 0 || ...)` therefore never runs for any instruction
in a running world (only `assemble.ts` passes `false`). Its own comment says it is "not a second
tolerance beside the one settlement uses when it decides whether to ask the issuer for an overdraft
at all (Law 4)" — but that is exactly what it is: a second computation of the same question, in a
second place, with a second `dustOf`, that the only caller disables. **What it would take:** delete
the guard from `moneyDelta` (the netting question is genuinely settlement's — leg order inside one
instruction must not decide an overdraft) and let the `allowNegative` parameter go with it.

**DEFECT — `dustOf` in `precheck` is dead arithmetic on integers.** `Math.abs(after) <=
dustOf(ops.length, Math.abs(free) + Math.abs(n.delta))` — money is a count of cents, so `after` is a
whole number and the dust is below 1 at every magnitude this world reaches. Same for
`validateCellSide`'s `Math.abs(expected - total) <= dustOf(2, ...)`: `perMember x weight` is an
exact integer product. Both are pre-Law-8 residue.

**DEFECT — a contract close does not check that its two sides are alive; an open does.**
`validateContract` calls `this.alive()` on `a`, `b` and the house for an `open`, and on nothing at
all for a `close`. `tearUpContract` then moves BOTH sides' equity. Money E4 says a ceased party's
legs are settled or refused by name — and the guards that actually stop this today were added
inside the derivative layer during 13b (a ceased house, an estate that equals its own survivor),
which is the wrong place: the wire is what knows a party is dead. **What it would take:** the same
two `alive` calls, on the row's `a` and `b`, for `close` and `novate`.

**MINOR — `carryingOf` returns 0 when the drawn total is 0**, an undefended zero in a file that is
otherwise strict about missing-is-missing. A debit that drew nothing should not be reachable
(`debit` requires `qty > 0`), so the branch is either unreachable or hiding something.

---

## `prices/`

**DEFECT — two readers of "what is this holding worth" disagree about staleness.**
`Valuation.worthOf` reads a cleared line with `prices.latest(instrument, at)` (takes the last print,
carries its age with it). `Valuation.markPerUnit` — which `valueOfLots`, `valueAtMark`,
`carryingPerUnit` and therefore `equityDust`, settlement's `reseat` and every balance-sheet read go
through — uses `prices.printOrThrow(instrument, at)`, which demands a print for EXACTLY that period
and throws `NotYetProduced` otherwise. So a fund's net asset value (which goes through
`DerivedReads.worthOf`) and the audit's read of the same holding use different rules for the same
fact. Law 4: one fact, one writer — and one reader. **What it would take:** decide which rule is
right (`printOrThrow` is, for anything inside a period whose phases are ordered — a stale read is
the thing F1.a exists to prevent) and make `worthOf` use it, or make the staleness explicit in both.

**MINOR — `valueOfLots` accumulates with `+=` and drops its dust,** in the one class whose
`equityDust` then claims to derive the tolerance from "every event that ever moved it". Every other
accumulation in the engine goes through `sum` (Neumaier) so the terms and the dust travel with the
answer; this one does not, and `equityDust` counts `sides.terms` over the OUTER sum only. The
tolerance it produces is therefore slightly smaller than the arithmetic that produced the number —
which is the safe direction, and still means the derivation is not the one described.

**MINOR — `recognisedThrough = -1` is a sentinel, not an `Option<Period>`.** A `Period` is a
branded non-negative integer and this field holds a number outside that set to mean "none yet",
in a codebase whose rule is that missing is `Missing`.

**MINOR — `contractValueTo` returns `0` for a party on neither side of the contract.** A caller
that asks the wrong question gets an answer that sums silently into a balance sheet. The two named
sides are the whole of Derivative D1; a third party's stake in a contract is not zero, it is
nonsense, and `Option` or a throw says so.

**MINOR — `PriceStore.read` and `printOrThrow` scan the instrument's whole print history
forwards** (`list.find(x => x.period === period)`) on every call, in a store that is read thousands
of times a period. The ledger and the instruments store both built an index for exactly this
(citing Law 18); the price store did not.

**MINOR — `CurveRead.priceOf` has no callers.** A door on the kernel's curve read that nothing uses;
`priceAt` (the free function) is what the modules call. Dead surface in the one place where "a price
is never derived from a yield" is being carefully argued.

`index-read.ts` is clean and the chaining argument (B2.a: a rebalance must not print a jump) is
correct and correctly implemented — each step compares this period's basket against itself a period
ago, so a line entering has no "then" and contributes nothing. The reader-owned cache is the right
shape: it holds only periods that are over, and two readers never read each other.
