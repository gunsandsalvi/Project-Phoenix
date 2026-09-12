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

---

## `clearing/solver.ts`

Clean, and the right things are stated rather than assumed: the tie rule is a named venue rule
(`sellersCompete` / `marginalBid`), rationing is a profile in a table, a market order is resolved to
a level somebody actually posted, and pro rata at the marginal level goes through `splitOnTick` with
the comment explaining why multiplying by a share was wrong. `validate` deliberately does NOT
re-check Law 8, with the reason (the type already did) — that is Law 12 applied to a check.

**MINOR — `Outcome.volume` is declared `number` while every quantity feeding it is `Qty`.**
`best.volume` is a `Qty`, `Fill.qty` is a `Qty`, and the field between them is not. Same for
`demandAtPrice` / `supplyAtPrice`.

## `clearing/market.ts`

**SEAM — `MarketDecl` is three market kinds in one optional-field bag, and two of the kernel's
hooks exist for exactly one module.** `MARKET_KINDS` is a genuine dispatch table and the right
shape. What is around it is not: `MarketDecl` carries `kind?`, `contract?` and `fx?` as independent
optionals, so `contractTrade` opens with `if (decl === undefined) throw new Missing(...)` and
`fxTrade` with the same, guarding at runtime against a state the type could forbid. `delivers(m)`
then has to know about both module shapes to answer one question. **What it would take:**
`MarketDecl` as a discriminated union on `kind` — `{kind:'asset'} | {kind:'fx', fx: ...} |
{kind:'contract', contract: ...}`. Every payload is then present by construction, both throws go,
`delivers` becomes a field, and a fourth kind really is a row.

And `MarketRunDeps` now carries `admits` and `marginLegs`, which exist solely for the derivative
layer: an asset market has no equivalent question ("how much of this can you actually take") even
though the question is just as real there — a buyer short of cash is refused at settlement instead.
So the kernel has two module-shaped holes in it, and the module that made them is the newest one.
This is the same growth as `PartyKindProfile`'s, in a second place. It is worth deciding, once,
whether the kernel's interfaces may grow per module or whether a module extends them from outside;
right now the answer is "yes, quietly", four times over.

**RISK — `admits` is a pure-looking callback with state behind it.** `runMarket` calls it once per
trade per side and the doc says the module draws the room down as it is consumed (E3). So the order
in which `pairFills` happened to pair buyers and sellers decides who gets the room, and the same book
re-run in a different pairing order gives different parties different fills. That may be exactly
right — it is a queue, and a real one has an order — but it is not stated anywhere as the rule, and
Clearing C5 says the session is a pure function of the orders. **What it would take:** say which it
is. If the room is consumed in fill order, that is a stated venue rule like rationing and belongs
beside it.

**MINOR — `pairFills` silently abandons the remainder below the common grain.** When `bLeft < step`
the buyer steps aside with `bLeft` unfilled: the solver said it bought that much and the market did
not deliver it. `settledVolume` records the difference in aggregate, but the party is never told
which of its fills evaporated, and the journal's `print` record carries `settledVolume` without the
grain reason. In a world with cells of different weights this is the normal case, not an edge one.

**GOOD — the cleared-house arithmetic is right.** `open(buyer, house)` + `open(house, seller)` with
the same terms means the house is `b` of one and `a` of the other, so its two marks negate exactly
and it is flat by construction rather than by a check. The premium legs net to zero on all three
books. This is the kind of thing that is easy to get subtly wrong and is not.

---

## `parties/party.ts`

**DEFECT — `cellKey` is declared as data, validated as data, and then never used.** The registry
takes `cellKey: readonly CellKeyDimension[]`, checks it has no repeats and includes `region`, and
its comment says "the cell key dimensions are registry data (XI-15): lifting a relationship into the
key is a data change". Nothing else in the engine reads it. `CellKey` in `party.ts` is a fixed
interface with all three dimensions required, and `Parties.add` validates all three unconditionally.
So lifting a dimension into or out of the key is a TYPE change in the kernel plus edits everywhere a
key is built — the opposite of what the registry declares. The declaration is decoration. **What it
would take:** either read it (`add` validates the dimensions the registry declares, `CellKey`
becomes `Readonly<Record<CellKeyDimension, string>>`) or delete it and say the key is fixed. Leaving
it is worse than either, because it tells the next person a change is cheap that is not.

**MINOR — `ofKind` rebuilds the whole party list on every call** (`all()` spreads the map, `alive()`
filters it, `ofKind` filters again). It is called by every participant of every module in every
phase. The journal and the ledger both built kind-indices for exactly this pattern and wrote a
paragraph about why; the party store, which is asked more often than either, did not.

## `journal/journal.ts`

Clean. The three indices are written where the event is written, so there is one writer and nothing
to go stale, and `lastBySubject` uses a NUL separator so a subject containing the delimiter cannot
collide.

**MINOR — `EventKind` ends in `` `${string}.${string}` ``, so the named kinds above it are
documentation rather than a type.** Any dotted string typechecks, `'print'` (no dot) is only valid
because it is listed, and a misspelled kind in a `lastOf` or `ofKind` call returns empty rather than
failing. Modules do need to add kinds — but they could register them the way they register
everything else, and then `ofKind` on an unregistered kind could throw instead of answering "never
happened".

---

## `world/module.ts` — and the answer to "why does adding one thing touch a thousand files"

This is where the question is settled, so it gets a section rather than a list of findings.

`SystemModule` has twenty-four fields. Eleven of them are optional single-answer hooks, and each one
was added when a module needed the kernel to ask a question it had not asked before:

| hook | added for |
| --- | --- |
| `curveFamilies` | sovereign curves |
| `indices?` | the index system |
| `outlooks?` | expectations (XI-16) |
| `marks?` | loans with no market (Banks Lending D1) |
| `creditDecisions?` | overdrafts (Money B3.a) |
| `bankChoices?` | depositors moving (Banks Funding E1) |
| `venueParticipants?` | the labour venue |
| `resolves?` | bank resolution instead of an estate |
| `derivativeKinds?` | 13a |
| `clearingCapacity?` | 13a |
| `seed?` | the seed |

Every one of them is argued correctly in place, and the argument is the same each time and it is
right: the answer belongs to the module that owns the party or the instrument, exactly one module
may answer, and a world where nobody answers must not seal. **The problem is not any one hook. It
is that inventing a hook is the only way a module can teach the kernel a new question, and
inventing one is a kernel change.**

Measured on the three commits that built item 13:

| commit | kernel files | module files |
| --- | --- | --- |
| 13a — the derivative LAYER (a new category of thing) | **24** | 7 |
| 13b part 1 — seven CLASSES on that layer | 6 | **24** |
| 13b part 2 — the classes trade | 10 | 16 |

That is the whole answer, and it is better news than it feels like from inside the work. **Adding an
instance of a shape the kernel already knows is genuinely plug-and-play** — seven derivative classes
cost six kernel files, and most of those six were one field each. **Adding a new SHAPE costs
twenty-four kernel files**, because a contract needed: two new branded ids (`core/ids.ts`), a store
(`register/contracts.ts`), a profile type (`registry/derivatives.ts`), registration
(`registry/registry.ts`), a leg (`ledger/instruction.ts`), an op and its arithmetic
(`ledger/settlement.ts`), a value read (`prices/contract-value.ts`), a market kind
(`clearing/market.ts`), two `MarketRunDeps` callbacks, an audit family (`audit/families/zero-sum.ts`),
two `SystemModule` hooks, context reads (`world/context.ts`), assembly checks (`world/world.ts`),
revaluation (`world/revalue.ts`) and observer views.

**Was any of that avoidable?** Mostly not, and the parts that were are worth naming:

1. **The two ids in `core/ids.ts`.** A module cannot brand its own identifier type. One
   `Brand<string, 'ModuleKey'>` the owning module re-brands would have removed this and every
   future one.
2. **`MarketRunDeps.admits` / `marginLegs`.** Two callbacks threaded through the kernel's market
   runner for one module. A market kind that carries its own "what does a fill become, and what
   does the venue need answered" would have kept them in the layer.
3. **`MarketDecl.contract?` / `fx?` as optional fields** rather than a discriminated union (see
   `clearing/market.ts` above) — this forced two runtime throws and a `delivers()` helper that has
   to know both shapes.
4. **`DerivativeKindProfile.orders`** put a participant's behaviour in the registry (see
   `registry/derivatives.ts` above), which is why `registry/` now imports `world/context.ts`.

Points 1–3 together are most of the difference between twenty-four kernel files and about twelve.
The remaining twelve are real: a contract genuinely is a new kind of state, and new state means a
store, a leg, an op, a value and an audit family. **No architecture makes that free, and an
architecture that did would be hiding it.**

**The real cost is not the count, it is that nothing says where the line is.** There is no written
rule for when a module may add a field to a kernel type and when it must not, so each hook was
decided on its own and the tenth looked like the ninth. ARCHITECTURE.md 4.9b describes the module
contract; it does not describe how the contract itself is allowed to grow. **What it would take:**
one paragraph in ARCHITECTURE.md saying which kernel types are open to module extension
(`SystemModule` hooks: yes, with the single-answer rule that is already enforced), which are closed
(`core/ids.ts`, `registry/kinds.ts` profiles, `MarketRunDeps`), and what a module does instead when
it needs something a closed type does not have. Then the next 13a is a decision rather than a drift.

---

## `world/revalue.ts`

The FX decomposition is the best piece of arithmetic in the engine and is correct:
`v(t)r(t) − v(t−1)r(t−1) = r(t)(v(t)−v(t−1)) + v(t−1)(r(t)−r(t−1))`, with the rate step running
first on the carried value at the OLD rate and the mark step converting at the NEW one, so nothing
is left over and neither half is an approximation of the other. The central-bank exception is keyed
on `registry.centralBankOf(home) === h.holder` — the party that IS the issuer of that money, not a
party kind — which is Law 15 done right.

**RISK — an issuer of a liability in a money that is not its own never books the exchange-rate
movement on it.** `revalueForeign` walks `register.allHoldings()` and moves `h.holder`'s account.
The issuer side is handled only in the main loop, and only for the MARK move (`toTheMark`'s delta
converted at `toIssuer`) — never for the rate. So for a bond issued in SOU by a firm whose home is
USD: the holder books the FX gain and the issuer books no FX loss. That is a one-sided flow in the
equity accounts (Law 5), and the `accounts` family would report it against the issuer, because
`balanceSheet` re-reads that liability through `inMoney` at this period's rate while the equity
account was never moved by it.

**It does not fire today**, and the reason is worth stating: nothing in this world issues in a money
that is not its own. Central banks hold FOREIGN government paper as reserves, but that paper's
issuer is a treasury whose home money it is. The moment anything issues abroad — which is where
XI-12 and "no sovereign immortal in foreign money" go — this becomes live, and it will present as an
`accounts` violation against the issuer with no obvious cause. **What it would take:** the same walk
over `instruments.all()` that the main loop already does for `liabilityOfIssuer`, applied to the
rate step. Ten lines, and much cheaper now than as a mystery later.

**Cross-reference:** the own-credit sign problem recorded under `ledger/settlement.ts` appears here
too, in `issuerMoves`: a fall in a bond's mark is `delta < 0` to holders and `-delta > 0` to the
issuer. It is the same modelling choice made consistently in two places, which is good — it means
fixing it is one decision, not two.

**MINOR — `toWhatTheKindSays` returns `carried: units === 0 ? 0 : ...`** and `carryingOf` in
settlement does the same; two undefended zeros for "there was nothing to average".

## `audit/families/accounts.ts`

Exemplary, and the two-family split is the right idea: `accountsFamily` checks the read against the
walk, and `equityLedgerFamily` checks the itemisation against the walk BY COUNT as well as by sum —
"a sum can be made to agree by two errors, a count cannot". That is a real falsification test, and
the note explaining why it is never done the other way round (summing entries to produce the
balance would make the check a tautology) is exactly the discipline Law 17 asks for.

`balanceSheet` being one function shared by the audit and by the §48 report — rather than the report
having its own — is the single best structural decision in the tree.

**Cross-reference:** `equityLedgerFamily` compares `equityEntries(p.id, 0, view.period)` against the
whole walk, which is why the `stateEquity` period-0 defect recorded under `register/register.ts`
does not show here: this family always asks from zero. It will show the moment anything asks for a
SPAN, which is what Reporting G2 exists for.

---

## `audit/`

**DEFECT — a partly-built family reports as built, and its unbuilt contributions disappear.**
`Audit.run` sets `built = contributions.some((c) => c.built)` and then lists
`contributions.filter((c) => c.built)`. So a family with one real check and three stubs reports
`built: true` with three contributors invisible — no count, no name, nothing. Audit C2 and this
file's own header say an unbuilt family "says so and is never green by omission"; at the
CONTRIBUTION level it is green by omission, and that is the level modules contribute at. **What it
would take:** report `contributions` and `unbuilt` separately, and let `built` mean every
contribution is built. The honest number is the one the whole audit exists to produce.

**MINOR — money's C4.c check is skipped in the first period** (`memory.period !== undefined &&
memory.period !== view.period`), so the money created by the seed is the one issuance nothing
verifies. It is also the largest.

**MINOR — the `Audit` constructor throws bare `Error`** for an unknown family and a duplicate
contributor, in the one file whose subject is that every finding names its clause. Both are
`InvalidRegistry` conditions with citations available (Audit B8).

**MINOR — the B3.c negative-balance check tolerates `moneyWalk(...).dust`,** which is the same dead
integer tolerance recorded under `register/register.ts`. Money is a count of cents; a negative
balance is at least one cent; the dust is ~1e-13 of one.

`zero-sum` is right and is the model for how a family should be built: it compares the profile's own
answer for the contract as EACH side states it (`flip`, then ask again) rather than comparing a
number against its own negation, and the per-contract check is EXACT with the tolerance appearing
only in the per-currency aggregate, where a real sum happens. The note explaining why the tautology
was avoided is the kind of thing that should be in more of these files.

**DEFECT (confirmed instance) — `crossMarket` reports built while its kernel contribution is an
unbuilt stub that vanishes from the report.** `standardFamilies` registers `crossMarketFamily()`
from `unbuilt.ts` with `built: false`; the derivative layer registers a second contribution from
`cross-market.ts` with `built: true`. `Audit.run`'s `some()` makes the family green, and
`contributions.filter(c => c.built)` drops the unbuilt one, so the report shows one contributor and
says nothing about the kernel check that was never written. Audit B4's original subject — the same
economic thing in two venues — is now silently unmeasured behind a family that reads as built.

**DEFECT — the flows family exempts a party's ENTIRE position for a period because one weight event
named it.** `weightSubjects` is built from every `'weight'` event's subjects, and then
`if (weightSubjects.has(holder)) continue` skips every (holder, instrument) pair that party has. The
untracked change is narrow — `copyMemberState` copies per-member state to a NEW cell without an
instruction — and the exemption is total: an entry, a death or a promotion, which move a weight and
copy nothing, exempt the cell from Money D3 for the whole period on every line it holds. Households
are cells and their weights move constantly, so in a populated world this is not an edge case.
Audit C3 says every family runs the same checks every period; this one runs fewer checks in exactly
the periods when something happened. **What it would take:** exempt the pair, not the party — the
split event already names `from`, `to` and `members`, so the exemption can be the two cells' shared
instruments for that one event, and an entry or a death (which do not copy state) can be checked
normally by scaling `before` by the weight ratio the event records, the same way the split
restatement already scales by its ratio.

`ownership`, `prices` and `units` are clean; `flows`'s split-restatement handling (compose two
ratios in a period, read the ratio off the public event rather than exempting) is the right shape
and is what the weight exemption should look like.

---

## `world/context.ts`

The three contexts are the best-defended boundary in the codebase and the comments on each door
(why `gather` exists, why `chooseBanks` is a door and not a loop inside the deposit market, why
`accountOf` is the kernel's and not a module's read of `party.bank`) are doing real architectural
work. `MechanismContext` genuinely has no register write, no price write and no weight write.

**SEAM — `SeedContext` hands modules the raw stores.** `parties: Parties`, `instruments:
Instruments`, `register: Register` — the write-capable classes, not the read facades that exist
beside them (`PartiesReads`, `InstrumentsReads`, `RegisterReads`, and `registerReads()` which builds
a real runtime-frozen facade). So a seed module can call `register.debit`, `register.moveEquity`,
`register.pledge`, `parties.applyWeight` or `instruments.adjustIssued` directly, and the module
contract's claim — "never holds a reference to a kernel store" — is false for this one context.
`endowMoney` and `endowUnits` are the intended doors and are right there; `stateEquity` is
deliberately NOT exposed (assembly does it, as the read of what the party turned out to hold),
which shows the line was drawn on purpose in one place and not in the others.
**Today's seed does not abuse it** — it uses `parties.add`, `instruments.add` and the two endow
doors, and reads through `register.quantity` / `holdingsOf` — so this is a capability finding, not a
behaviour one. **What it would take:** `SeedContext` exposes `parties: Pick<Parties, 'add' | ...>`
and the reads facade for the register, which is a five-line change and makes the contract true.

**MINOR — `blind(party)` is a kernel door built for one module,** and a negative one: a
`ParticipantView` with the prices removed so an assessor cannot look at them. The argument is right
(Ratings A2.a: an assessment is made from state BECAUSE the prices are unreachable, not because the
code chose not to look) and it is still the kernel growing a shape for one caller — the same
pattern as `clearingCapacity` and `admits`, in a fourth place.

**MINOR — `ContractsRead.marginFor` takes an inline structural copy of most of `Contract`.** The
thing it describes has a name — a contract that has not been written yet — and giving it one would
let the same shape be reused by `validateContract`'s `asIfOpen` in settlement, which builds it too.

## `rng/prng.ts`

Clean, and used once in the whole engine: `expectations/index.ts` draws each party's MEMORY
preference. That is exactly the right use — a drawn PREFERENCE, in the spirit of Seed B1.a's drawn
world — and not a decision taken by a coin flip. Deriving streams by label so that a new mechanism's
draws do not reshuffle an existing one's is the detail that makes `resolution.pieceShift` and the
other invariance tests meaningful.

---

## `world/world.ts`

**DEFECT — "one party shows one face to one book" is solved twice, differently, and only one of the
two is structural.**

For a CONTRACT book, item 13a's answer is: the layer declares ONE participant per party kind and
dispatches to `DerivativeKindProfile.orders`, so a party's schedule in a book has exactly one
author. For an ASSET market, the answer is: any number of modules may declare a `ParticipantDecl`
for the same `partyKind` (`addParticipant` checks nothing), `runOne` collects orders from all of
them, and a party that ends up on both sides at crossing prices is caught by a `forbid` throw inside
`pairFills` — at settlement time, as a crash, naming the party rather than the two modules that
spoke for it. Today `FIRM` has two participants (`equity`, `firms`), `BANK` has two (`banks`,
`spot-fx`), `FUND` and `FUND_MANAGER` two each. They do not collide because each self-selects its
markets by convention, and `ParticipantDecl.markets` is explicitly "a filter and not a claim".

So the invariant holds by discipline in the older half of the engine and by construction in the
newer half. Law 4 is about facts, but the same instinct applies: one problem should have one
solution, and having two means the next module has to know which world it is in. **What it would
take:** the asset side adopts the contract side's shape — one participant per party kind, owned by
the module that owns the kind, dispatching to reasons the other modules register. That is a real
piece of work and it is the single change that would most reduce the "everything touches everything"
feeling, because it makes "who speaks for this party" a question with one answer.

**GOOD — `addPhase`'s anchoring rule.** The asymmetry between `before` (insert AT the anchor, so
later modules land in front of the anchor behind earlier ones) and `after` (skip past every phase
already anchored to the same point) is subtle, correct, and explained. The monotonic-cycle
re-validation after every insert is the right place for it.

**GOOD — the `provide*` doors all `forbid(!this.sealed, 'Law 10', ...)`** and `requireCreditDeciders`
runs at the seal, so a world that cannot answer a question it must answer never starts. That is the
right shape for every single-answer hook and it is applied consistently.

**MINOR — `reads()` recomputes `stalePrints`, `reserveOverdrafts` and `failedInstructions` by
filtering the whole period's journal and ledger three times,** once per period. Free by Law 18 and
worth noting only because the journal's own `byKind` index would answer two of the three directly.

---

# The mechanisms

## THE HEADLINE FINDING — `phoenix/no-cross-module-import` does not fire, and six modules import each other

**DEFECT — the lint rule that enforces the module boundary has a regex bug and has never caught
anything.**

`tools/eslint-rules/index.js`, `noCrossModuleImport`:

```js
const resolved = source.replace(/^(\.\.\/)+/, '').replace(/^\.\//, '');
const sib = /^(mechanisms|seeds)\/([^/]+)/.exec(resolved);
```

A module importing a sibling writes `from '../goods/index.js'`. Stripping the leading `../` leaves
`goods/index.js`, which does not begin with `mechanisms/`, so `sib` is `null` and nothing is
reported. The only spelling the rule catches is `'../../mechanisms/goods/index.js'` — which nobody
writes, because it is two directories up and back down again. Verified: `npx eslint
packages/engine/src/mechanisms/firms/decide.ts` passes clean, and that file imports nine symbols
from `capital-programme` and four from `goods`.

**And the boundary has in fact been crossed, six times, including one cycle:**

| module | imports from | what |
| --- | --- | --- |
| `firms` | `goods`, `capital-programme` | **values** — `goodId`, `goodMarketId`, `goodTerms`, `capacityFrom`, `capitalChargePerUnit`, `capitalKindOf`, `isPlant`, `lifeParam`, `plantTerms`, `serviceLeft`, `vintagesHeld`, `costOfDraw`, `dueFromLine`, `wipId`, `plantHeld`, `utilisation` |
| `capital-programme` | `goods` | **values** — `costOfDraw`, `goodId`, `isGoodTerms` |
| `households` | `goods` | **values** — `goodId`, `goodMarketId` |
| `treasury` | `goods`, `sovereign-curve`, `sovereign-instruments` | **values** — `isGoodTerms`, `CURVE_DAY_COUNT`, terms constructors |
| `banks` | `spot-fx` | **a value, and it is a parameter** — `FX_SPREAD` from `spot-fx/data.ts` |
| `spot-fx` | `banks` | types — `BankDecl`, in four files |

`banks → spot-fx → banks` is a **module cycle**. That is not a style point: an import cycle between
two modules that both declare `const`s at module scope is precisely how a constant comes back
`undefined` at initialisation, which cost an afternoon in this session's 13b work (`cds/contract.ts
→ participants.ts → series.ts → contract.ts`, leaving `PROTECTED` undefined and surfacing as
`unit undefined does not exist`). The same trap is armed between `banks` and `spot-fx` today.

`banks/data.ts` importing `FX_SPREAD` from `spot-fx/data.ts` is the worst of the six on its own
terms: a behaviour-shaping number declared by one module and read by another, which is Law 4's "one
fact, one writer" broken at the parameter register, not at the code level. If a bank's dealing
spread and the FX module's spread are one number, one module owns it and the other reads it through
`params`; if they are two numbers, they are two declarations.

**Why this is the answer to "why does everything touch everything".** ARCHITECTURE 4.9b's claim —
"a module reaches the kernel only through `ParticipantView`, `MechanismContext`, `SeedContext`; it
never imports another module (lint)" — is the load-bearing statement of the whole design, and the
parenthesis is what makes it true. It has been false since the rule was written. So six modules are
coupled directly, and the coupling is invisible: it does not show up in `requires`, it is not in the
contexts, and nothing in a review would surface it because the lint says it is fine.

**What it would take:** one character class. `const sib = /(?:^|\/)(mechanisms|seeds)\/([^/]+)/`
applied to the FILE-RELATIVE resolution rather than the stripped string — or, more simply, resolve
the import against the importing file's directory and compare real paths. Then fix the six. Some of
them are genuinely shared vocabulary (`goodId`, `goodMarketId` are naming functions, exactly what
`registry/naming.ts` is for) and belong in the kernel; `FX_SPREAD` belongs to one module and is read
through `params` by the other; `BankDecl` in `spot-fx` is a type that should be a kernel read
(`registry`/`ParticipantView`) rather than another module's declaration table.

**Expect this to be a real piece of work, and expect it to shrink the tree.** Four of the six are
`firms`/`households`/`capital-programme`/`treasury` reaching into `goods` for the same two naming
functions — which means the goods naming grammar wants to be kernel data, and once it is, four
modules stop knowing about a fifth.

## The lint rules generally — they enforce spellings, not laws

**DEFECT — `phoenix/no-bounds` forbids `Math.min`/`Math.max` and the engine writes the same
operation as a ternary thirty-one times.** A sample:

```
labour/matching.ts:221      const taken = people < available ? people : available;
funds/index.ts:340          const budget = wanted > cash ? cash : wanted;
banks/dealing.ts:306        return want < free ? want : free;
derivative-layer/index.ts:175  return affordable < wanted ? affordable : wanted;
firms/invest.ts:323         const qty = canPay < x.order.qty ? canPay : x.order.qty;
households/consume.ts:121   const affordable = wanted > budget ? budget : wanted;
```

**Most of these are correct** — "you cannot deliver more than you hold", "you cannot pay more than
you have", "you cannot hire more people than there are" — which is Law 6's one admissible case,
arithmetic impossibility. Two I checked closely (`banks/treasury.ts`'s `c.floor` and `defended`'s
`stopAt`) are real alternatives a bank has, argued at length and correctly. **That is exactly the
problem:** the rule cannot tell them apart, so it bans a spelling, everyone writes the other
spelling, and the law goes unchecked either way. A reviewer grepping for bounds finds nothing; a
reviewer grepping for ternaries finds thirty-one and no way to sort them.

**What it would take:** `core/num.ts` gets `atMost(value, limit, because)` and `atLeast(...)` whose
third argument is the REASON — "what it holds", "what the corridor pays" — and `no-bounds` forbids
the bare ternary shape as well as `Math.min`/`Math.max`. Then every bound in the engine is a call
with a stated reason, the thirty-one become greppable and readable, and a bound with a weak reason
is visible as one. The law asks for the compensating mechanism; naming it at the site is the
cheapest possible enforcement of that.

**MINOR — `phoenix/no-kind-branch` sees only `===`, `!==` and `switch`.** `d.eligible.includes(i.kind)`
(`funds/index.ts:898`) passes, and is fine — a mandate naming the kinds it may hold is data about
the fund. But the rule would not catch `['a','b'].includes(x.kind)` written inside a mechanism
either, which is not.

## `households/`

The design claim in the header — "no representative household, no propensity applied to an
aggregate, no average anybody could have crossed a threshold at" — holds. Every number in
`consume.ts` is per member of the cell that decided it; the cushion is the cell's own outlook
widened by the cell's own surprises; the demand curve is posted as a step function over the cell's
own uncertainty, so a cell that has seen prices move bids over a wider range than one that has not.
`constrained` is published as a flag precisely because it is a THRESHOLD a mean-preserving spread
moves cells across, which is A2.g taken seriously.

**SHAPE — consumption is a fixed share of MONEY spending, which is the assumption `no-value-recipe`
exists to forbid on the production side.** `ConsumptionDecl.share` is "the share of what it spends
that goes on this good", and `data.ts` states the consequence plainly: "a household facing a dearer
loaf buys fewer of them and spends the same on bread". That is unit-elastic demand — spending on a
good never responds to its price, and a price change never moves spending BETWEEN goods. The
`phoenix/no-value-recipe` rule refuses exactly this on the input side, with the argument written out
in `tools/eslint-rules/index.js`: "A price that doubles would halve the physical draw, which is the
strongest substitution assumption there is, sitting where the model chose none."

**It is harmless today** — there is one final good and both cohorts' share is 1, so the share does
nothing — and it becomes the strongest substitution assumption in the model on the day a second
final good exists. **What it would take:** the same answer the production side got. A cohort's
preference over goods is a real PREFERENCE primitive and may be declared; what it may not be is a
share of money. Declare it as a quantity a household wants per period (a basket), and what it
actually buys is then the outcome of that want meeting a price, which is what the demand schedule
is already shaped to express.

## `mechanisms/derivative-layer/`

**DEFECT — a member's clearing capacity is reduced twice for every unit of margin it posts.**

```ts
const room = sub(sub(cash, keep, 'net of its buffer'), committed(view, m.ccy), 'left to commit');
```

`committed` sums the margin CLAIMS the party holds (`margin.ts`), and a party holds a margin claim
because it posted cash for it: `moveMargin` moves the claim to the poster and the money to the
holder, in one instruction, settled before the next trade in the session is admitted. So by the
time `admits` runs again, `view.cash(ccy)` is ALREADY net of everything posted — and subtracting
`committed` takes it off a second time. A member that has posted 100 has its room cut by 200.

The intent is stated and is right (E3: "the second hedge of a period is sized against what the first
will post, so capacity is drawn down as it is consumed rather than measured fresh each time") — and
`cash` alone already does exactly that, because the margin settles inside the trade's own
instruction. The effect is conservative (the layer refuses more than it should, never less), which
is why nothing has caught it; it makes every derivative book thinner than the mechanism says, and it
makes E4's refusal measurement wrong in the direction that looks like prudence. **What it would
take:** delete the `committed` term. It is Law 19 in miniature — the cash balance is the source, and
`committed` re-derives from a second read something the source already carries.

**MINOR — `admits` returns `0` when the market names no contract; `contractTrade` throws `Missing`
for the same condition.** One impossible state, two answers, and the quiet one is in the module.

**MINOR — the waterfall records an extinguishment as a trade at a price of zero.**
`pricePerUnit: some(0)` on the asset leg, while the equity effect actually uses the estate's
carrying value (settlement's `redeem` takes `carryingOf(fromDebit)` whenever the from-side was
debited). So the arithmetic is right and the LEDGER RECORD is false: a reader of that instruction
sees a claim that changed hands at nothing. `none()` — "a transfer at carrying value", which is what
`AssetLeg.pricePerUnit`'s own doc says none means — is both correct and true.

**MINOR — `Round.paid` means two different things.** For `defaulterMargin`, `defaulterFund` and
`survivors` it is an amount that moved in this call; for `houseCapital` it is an amount that moved
EARLIER (the house's equity fell when it paid the survivors, which is argued correctly). A reader
summing `paid` across the rounds counts the house's capital twice. Naming the line's field
`absorbed` and noting which lines settle would fix it.

**GOOD — the capacity refusal is genuinely a refusal.** `admits` returning 0 on a ceased house, on
`initialMargin` being `none` ("the underlying has not moved yet, so nobody can say" is refused
rather than admitted at zero), and on a requirement below one piece of money — with every one of
them journaled and measured (E4) and nothing anywhere raising the limit — is Law 6 done properly:
the constraint is a quantity somebody HAS, not a number somebody chose.

## Money creation and destruction — checked exhaustively, and it is right

The user's question deserves a direct answer, so here is the whole of it.

Money can enter or leave this world at exactly two places in `ledger/settlement.ts`: the `issue` op
and the `redeem` op, each reachable only from `expandMoney` and `expandAsset`. Every module-level
site that triggers one is:

| site | what it is | correct? |
| --- | --- | --- |
| `banks/index.ts:436` | a bank lends: its own deposit created into the borrower's account, in the same instruction as the loan the borrower issues | **yes** — this is endogenous money, and the comment correctly notes no reserve leaves the bank when both sides bank there (B1.a) |
| `banks/index.ts:504` | a drawing on an existing line | yes |
| `central-bank-omo/index.ts:158` | the central bank remits its income to the treasury, in reserves it issues | yes |
| `money-market/resolution.ts:394` | a bail-in: a depositor's claim on a failed bank extinguished | yes |
| `money-market/resolution.ts:757+765` | an acquirer takes over an account: the failed bank's deposit destroyed and the acquirer's created, **two legs, one instruction** | yes, and this is the hard one to get right |

And the implicit path, which is the one most likely to be wrong and is not: a loan REPAYMENT. The
borrower pays `accountOf(bank)`, which is the bank's account at the CENTRAL bank, so the leg crosses
issuers; settlement then redeems the borrower's deposit (destroying it), credits the bank's reserves
and debits them again in the cross-issuer branch — net zero reserves, one deposit destroyed. That is
exactly right and it falls out of the wire rather than being coded anywhere.

**There is no other way for money to be created or destroyed in this engine.** No module writes
`issued` directly, no module holds the register, and `adjustIssued` has three call sites, all inside
settlement. On this criterion the answer is unqualified: yes.

## `mechanisms/estate/`

**SHAPE — the estate's reservation is a formula discount off the last print, and the module's own
header says that is the thing it must not be.**

```ts
const total = sub(closesAfter, view.period, 'left') + 1;
price: mul(print.value.price, div(left, total, 'how much of its patience is left'), 'reservation')
```

That is `print × left/(left+1)` — 1/2, 2/3, 3/4, 4/5 as the programme runs down. The inline comment
says "There is no discount curve here — the number is how long is left", and the module header says
"ASSETS ARE SOLD, not valued... **a formula discount off book is a stated price with no buyer**".
`left/(left+1)` is a discount curve off book. It is not derived from anything the estate knows about
the asset, the bidders, or what it has to raise — only from the calendar and a parameter.

Two consequences. First, it is a written price path (Appendix B: no written price path), in the one
module whose whole purpose is that a forced sale realises what a market pays. Second, the schedule
is PUBLIC — `estate.opened` journals `closesAfter` and the programme length is a parameter — so any
bidder can compute the exact period the estate will capitulate and simply wait. That is predatory
trading, which is real, but here it is an artefact of the formula rather than an outcome of anyone's
decision.

**And the codebase already contains the right answer, one module over.** `funds/index.ts:861`, the
other forced seller: `price: 'market'`, with the comment "XI-2: at whatever the market gives. A
forced seller that named a price would not be one." Two modules, one problem — a seller under time
pressure — and two answers, one of which is a reason and one of which is a curve. **What it would
take:** the estate's reservation is what it expects to get by waiting (its own outlook of that
line's price, which XI-16 already gives every party) against the chance its programme ends first —
or, simpler and just as honest, it is the fund's answer: an estate that must sell names no price.

## The rest of the mechanisms — swept, with what the sweep found

Mechanically clean across all twenty-eight: no `Math.min`/`Math.max`, no `?? 0` or `|| 0`, no
`=== 'someKind'` branches, four `eslint-disable` lines in the whole tree and each defended in a
sentence. `Math.floor`/`Math.round` appear ten times and all ten are on counts of people, cells,
months or auctions rather than on a unit's pieces.

Every module's header states an architecture and, where I checked, the code holds to it:
`households` really has no representative agent; `funds` really lets the forced sale name no price;
`ratings` really keeps the issuer-pays conflict rather than assuming it away; `research` really
makes coverage an outcome of a bank's own book (D3.a) rather than universal; `indices` really stores
no level; `reporting` really reads the same `balanceSheet` the audit checks. That consistency is the
most valuable property this codebase has and it is not an accident.

**MINOR — `banks/index.ts:309` writes through a `readonly` with a cast:**
`(held as { period: number }).period = ctx.period;`. The field is declared `readonly` on
`CouponsPaid` and then written anyway. The cache is correct and the argument for it (Law 18: one
walk of a period that has stopped moving) is right; the cast is a type-system escape in a codebase
whose whole method is that the compiler holds the rules.

**MINOR — `central-bank-omo` posts `price: 'market'` for its open-market purchases.** In this
solver a market buy resolves to the HIGHEST ask anybody posted, so a central bank closing a gap
towards its 25%-of-line target is a price-insensitive bidder at the top of the book in every
sovereign session where it has one. The quantity limit is real policy (Central Bank C1, and the
module correctly refuses to stand in any other market), so this is not the buyer of last resort
Appendix B forbids — but "market" here means "pay the worst level posted", which is more aggressive
than any real open-market operation, and it makes the central bank the marginal price-setter in the
sovereign book rather than a large participant in it.

---

# The verdict, and what to do first

83 findings: 25 DEFECT, 6 RISK, 9 SHAPE, 5 SEAM, 38 MINOR.

## Answering the four questions directly

**Does it follow its own rules?** Mostly, and where it does not the cause is almost always the same:
**a rule is enforced by a check that has drifted from the rule.** `no-cross-module-import` never
fires; `no-bounds` bans a spelling; `no-kind-branch` sees three syntaxes; a shape's scheduled death
is policed by a regex over prose; `Audit.run`'s `built` flag is `some()` when it should be `every()`.
The laws in `CLAUDE.md` are good and the code mostly obeys them out of discipline. The machinery
that was supposed to make discipline unnecessary is the weakest part of the tree.

**Is the modelling realistic, and is anything top-down?** The modelling is unusually honest. Every
decision I traced is a party's own, taken from its own state and its own outlook, with no aggregate
anywhere a decision could read. Three things are asserted rather than derived, and all three are
identified above: the estate's `left/(left+1)` reservation (a written price path, in the module that
forbids one), consumption as a fixed share of money spending (unit-elastic demand, the assumption
`no-value-recipe` refuses on the production side), and the issuer's own-credit gain (a firm's equity
RISES as its own bonds fall). The third is the one that will bite a mechanism: it fights XI-3.

**Is money created and destroyed only in the right places?** Yes, unqualified. Two ops, six module
sites, all correct, and the hard case (a loan repayment destroying a deposit without moving a
reserve) falls out of the wire rather than being coded. See the table above.

**Is it really modular?** Yes for INSTANCES, no for SHAPES, and the numbers are in the table under
`world/module.ts`: seven derivative classes cost six kernel files; the derivative LAYER cost
twenty-four. About half of those twenty-four were avoidable and the four avoidable causes are named.
But the deeper answer is the headline finding: **the module boundary is not enforced at all**, six
modules import each other including one cycle, and the architecture's central claim rests on a lint
rule with a regex bug.

## What to do, in order

The order is by what unblocks the most, not by severity.

1. **Fix `no-cross-module-import`, then fix the six crossings.** One regex; then the six. It is the
   only finding that makes every other structural claim in `ARCHITECTURE.md` checkable again, and
   `banks ↔ spot-fx` is an initialisation bug waiting for the next `const`.
2. **`Instruments.restate` does not invalidate `all()`.** One line. A share split currently leaves a
   stale `issued` in front of every reader for the rest of the run.
3. **`Contracts.open` validates neither its terms nor their kind.** Two lines, copied from
   `Instruments.add`. Without them a mis-kinded contract marks zero on both books forever.
4. **Delete the dead dust from `register/register.ts` and `ledger/settlement.ts`.** Law 8 retired it;
   about forty lines go, including the one branch in the engine that can destroy units.
5. **Delete the `committed` term from the derivative layer's `admits`.** One line. Every contract
   book is currently half the size the mechanism says.
6. **Decide the own-credit question** (settlement's issuer re-mark and revaluation's `issuerMoves`).
   This is a modelling decision, not a bug fix, and it should be recorded as one.
7. **Fix the flows family's weight exemption and the audit's `built` flag.** Both are holes in the
   instrument that is supposed to find everything else.
8. **Give the estate a reason instead of a curve**, and take the fund's answer if nothing better
   presents itself.
9. **Write the paragraph in `ARCHITECTURE.md`** saying which kernel types a module may extend and
   which are closed. Every one of the eleven hooks was a good decision; nothing says where the line
   is, so the twelfth will be taken the same way the eleventh was.

Everything else is real and can wait. Nothing in this review is a reason to stop building forward:
items 1–5 are each one to three lines, 6 is a decision to record, and 7–9 are the kind of work that
is cheapest done between items rather than inside one.

## One thing worth saying plainly

The prose in this codebase is doing real work, and it is what made this review possible: almost
every finding above was found by reading what a file SAYS it does against what it does. That only
works because the files say what they do, at length, with the clause cited. Four of the findings —
`core/tick.ts`'s "a leg's amount", `register/register.ts`'s "guarding the STORE rather than each
writer", `estate`'s "a formula discount off book", `parties`'s "lifting a relationship into the key
is a data change" — are cases where a file states the rule in its own header and the code two
hundred lines down does not follow it. In a codebase with ordinary comments none of those would be
findable at all.

---

# Addendum: how parties name a level, and what happens when nothing printed

Asked after the review: things lose their price when there was no print the period before, and every
party should be running its own fair value rather than quoting off the last print. Both halves are
right, and the codebase has already diagnosed and cured this once — in one file, which the seven
newest modules then did not follow.

## What each party actually posts

| party / book | the level it names | reads this book's own print? |
| --- | --- | --- |
| bank dealing desk — **dated claim** | `priceAtYield(own flows, requiredYieldOf(issuer))` | **no, deliberately** |
| bank dealing desk — share | own outlook of the price, then what it carries one at | yes |
| household — sovereign paper | `priceAt(flows, its own required yield)` | **no** |
| household — share | `bookPerShare + earnedPerShare ÷ year ÷ required` | **no** (published accounts) |
| household — fund share | the NAV (`view.mark`) | **no** (arithmetic on a book) |
| household — goods | own outlook of the price, spread over own confidence | via its outlook |
| firm — its output | own outlook × (1 − spoilage) | via its outlook |
| firm — plant | `worth − rest of the bundle`, from expected contribution | **no** |
| firm — its own shares | `equity ÷ issued` | **no** (print only decides cheap/dear) |
| fund — buying | `priceAt(flows, what its investors require)` | **no** |
| fund — forced sale | `market` | n/a, and correct |
| index future | the index level | **no** (the index is a read of constituents) |
| money market | the corridor's floor / ceiling | n/a, administered |
| labour | the firm's own wage bid; the cell's own outside option | **no** |
| estate | `print × left/(left+1)` | yes — already a finding above |
| **cds** | this book's print, else the reference's cash bond below par | **yes** |
| **irs** | this book's print, else the overnight fixing | **yes** |
| **fx.forward / xccy** | this book's print, else spot | **yes** |
| **bond.future** | this book's print, else the cash bond | **yes** |
| **option** | this book's print, else **one tick** | **yes** |

The older half of the engine is fair-value-first. The seven derivative classes built in 13b are
print-first with a bootstrap behind them. That split is the whole of the problem.

## The cure is already written down, in `banks/dealing-quote.ts`

`viewOf` asks `requiredYieldOf` → `priceAtYield` **first**, and falls back to the print-derived
outlook only for a claim that promises nothing. The comment says why, and it is an incident report:

> It used to be the last resort... and what stood in front of it was `outlook('price.<instrument>')`:
> the desk's own adaptive expectation OF THE PRICE, formed from the prints. That is XI-13's fixed
> point written out — **the print moves the outlook, the outlook moves the view, the view moves the
> quote, the quote moves the print** — and in the one market this model funds itself through it
> walked a bill that pays 1.00 in seven days to a print of 3.3337, at which no representable yield
> discounts its own payments to what somebody paid, so the curve threw and the world stopped.

That is exactly the failure mode, named, with the fix. **The seven classes written afterwards do the
thing it describes.**

## DEFECT — the derivative classes post AT the last print when the party has no view

Every one of the five bilateral classes has this shape (`irs/participants.ts:81`,
`bond-futures/index.ts:260`, `fx-derivatives/participants.ts:91`, `options/index.ts:247`,
`cds/participants.ts:44`):

```ts
const last = view.print(t.book);
const level = last.some ? last.value.price : /* bootstrap */;
...
let price = level;                       // <- the party's order, before any view
const outlook = view.outlook(`price.${book}`);
if (outlook.some && ...) {
  if (expects > level + tick)  { want += conviction; price = expects; }
  else if (expects < level - tick) { want -= conviction; price = expects; }
}
```

Three consequences, in order of how much they matter:

**1. A party with no outlook posts exactly the last print.** `outlook('price.<book>')` exists only
for a party that has traded in this book (Expectations A2: a party that never observed a variable has
no outlook of it). So a borrower hedging its own coupon, a holder buying a put, a bank taking the
other side for the first time — each posts a limit at the last print, on whichever side its own
position puts it. A book whose participants are all in that state is a crowd of orders at one level:
it clears at that level, prints that level, and prints it again next period. **The book is frozen at
whatever it first printed**, and nothing in it is a price in Law 3's sense — real supply met real
demand, but neither side named a number of its own.

**2. When somebody does have an outlook, the outlook was formed from the prints.** `observations()`
in the expectations module writes `price.<book>` from that party's own fills in that book. So the
outlook converges on the print it came from, `expects` lands inside `level ± tick`, the nudge never
fires, and `price = level` again. That is the fixed point the dealer's comment describes, closed.

**3. The print is doing two different jobs and only one of them is legitimate.** Deciding whether a
party is a buyer or a seller by comparing its own value to where the market is, is what a trader
does and is right. Using the market's last number AS the party's own limit is not a view — it is the
absence of one. The option participant gets this half-right and is worth reading as the pattern: it
computes `asks = moves + moves × aversion` from its own outlook's CONFIDENCE (a real, independent
view of dispersion, which is what an option is a price of) and posts `price = asks` whenever that
differs from `level` — so `level` is the comparator and the party's own number is the order. The
other four never do that.

## DEFECT — the option book cannot start, and the reason is a placeholder standing in for a value

`options/index.ts:247`:

```ts
const level = last.some ? last.value.price : view.registry.tickForDerivative(decl.kind, m.ccy);
```

With no print, the level is **one tick**. Every party's own `asks` is then above it, so
`if (asks > level)` fires for everybody, every party subtracts `room` from `want`, and **every party
in the first session of an option book is a writer.** All sells, no buys, `noDemand`, no print — and
next period the bootstrap is one tick again. The book can never open.

It is also a declared number standing in for a mechanism, written in code rather than in the
parameter register: Law 2 would call it a placeholder and ask it to name the item that kills it. The
value it is standing in for is computable from what the participant already reads three lines below:
the underlying's own measured move (`ContractReads.measuredMove`, which `initialMargin` already uses
for exactly this kind) and the strike. **What it would take:** the bootstrap is the same arithmetic
as `asks` — what the party's own view of the move says optionality is worth — which means the
fallback and the view are one expression and the whole branch disappears. That is Law 12's shape: the
fix removes code.

## RISK — the four other bootstraps are real values that are only ever used once

`cds` (the reference's own bond below par, per year of term), `irs` (the overnight fixing),
`fx.forward` (spot), `bond.future` (the cash deliverable) are each a genuine, print-independent fair
value for that contract, and each is correctly argued in its own comment. They are reachable only
when the book has NEVER printed. From the second session on, the book's own print displaces them
permanently — including when the print is stale, days old, and the underlying has moved.

That is backwards, and the asymmetry shows it: `fx.forward` bootstraps from spot, then stops reading
spot; but a forward whose spot has moved five per cent and whose own book did not trade is worth a
different number, and every party posts the old one. The cash-and-carry relationship the class exists
to express is live in period one and dead from period two.

**What it would take, for all five:** the swap `dealing-quote.ts` already made. The bootstrap becomes
the party's own value, computed every period; the book's print becomes the comparator that decides
which side the party is on and how hard. Concretely:

```ts
const mine  = ownValue(view, t);          // what this class's bootstrap already computes, always
const level = view.print(t.book);         // where the market is, or none
// side and conviction from mine vs level; the ORDER is posted at `mine`
```

Nothing new has to be invented — every class already has `ownValue`; it is the `else` branch. And
`DerivativeKindProfile` already has the machinery in the right shape: `mark(c, at, reads)` says what
an open row is worth from public state, and `ContractsRead.marginFor` already asks the "what if this
existed" question about a row that does not. A `parLevel(terms, at, reads)` on the profile — the
level at which a new contract on these terms is worth nothing to either side (D7.b, which is the
definition already in the file) — is the same shape as both, would be one function per class, and
would give every participant a number of its own without any of them reading the book.

## On the symptom: "no price because there was no price the tick before"

Tracing it precisely, because the mechanism is not quite what it looks like.

`clearing/market.ts` `carryLast` writes a **stale print** whenever `prices.latest` finds any earlier
one, so after a line's first print it has one in every subsequent period. A line loses its price
entirely only if it has **never** printed, or if its market did not run (`world.ts` filters out
markets whose instrument has ceased). So the live failure is not "did not print last tick" — it is
**"has never printed"**, and it is self-sustaining exactly where the level is read off the book:

- an **option book** — never, per the finding above;
- a **new line with no seeded opening print**, where the only parties who could price it read the
  print. A mid-run corporate bond is the case to watch: the household buys sovereign paper only
  (`portfolio.ts:124` `if (!sovereigns.has(i.issuer.value)) continue;`), and the dealer needs
  `requiredYieldOf`, which is the bank's own published reservation for that issuer — so if no bank
  has published a reservation for a new name, nobody in the world can name a level for its paper.

Everything else bootstraps: the seed writes opening prints (Seed C4), sovereign lines come up through
a primary auction where the treasury names a reservation and bidders price off the curve, and the
curve survives on `prices.latest` rather than this period's print.

**Two things follow.** First, `Valuation.markPerUnit` calling `printOrThrow` (already a finding above,
under `prices/`) is what turns "never printed" into a thrown `Unpriced`/`NotYetProduced` at a reader
far from the cause — which is likely what was actually observed. Second, the honest answer for a line
nobody has priced is the one `cds/participants.ts` already gives: *"A reference whose debt has never
printed either has no anchor and no book — which is the honest answer for a name nobody has ever put
a price on."* That is right, and it is only right when the parties who COULD price it have genuinely
tried. Today, for four of the five bilateral classes, they have not tried since period one.

## Where this belongs

Not a new worklist item. The five class fixes are one bounded change each and belong in **13b**'s
remaining steps, which are still open; the option bootstrap is the same change and is the one that is
currently load-bearing. The `parLevel` profile function, if it is taken, is a change to the derivative
kind contract and so belongs with them, in 13b, not after.
