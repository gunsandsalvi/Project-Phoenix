# PROJECT PHOENIX
## The model specification — what the world is, what must be true of it, and what must not exist

---

### What this document is

This is a **specification of a simulated economy**, written from the domain. It states what must
exist for each system to be that system, what must be true once it exists, and — the part no other
kind of document can carry — **what must be absent**.

It contains no implementation. There are no file names, no function names, no module boundaries, no
storage layouts, no language choices, no assessment of any existing system's state of completion.
Every one of those is a decision for whoever executes this, and stating them here would smuggle one
answer into a document whose whole value is that it does not contain answers.

It is written to be **executable by a developer who has never seen a line of the code that inspired
it**, and to leave that developer free to choose every technique.

### How to read it

The document is organised in the order a reader needs it:

- **Part I — First principles.** Nineteen standing laws. They govern every other part and they are
  not negotiable per-system. Read all of them before anything else.
- **Part II — The method.** How a requirement is written here, why the three node types exist, and
  why a specification is the only instrument that can find something missing.
- **Parts III–X — The forty-eight systems.** One section per system, each a tree of numbered
  requirements. The numbering is stable and is the citation grammar for the whole document: `Goods
  B1.b` is one requirement, referenceable from anywhere.
- **Part XI — Mechanisms in depth.** Seventeen cross-cutting mechanisms that no single system owns,
  written out at the length they need. These are where the model's causal chains live.
- **Part XII — The measurement programme.** Every VERIFY in the document, and the causal chains
  worth testing. This is what "is the model working" means.
- **Part XIII — Sequencing.** What must exist before what, and why.
- **Appendices.** The conventions, and the consolidated list of prohibitions.

### The single ambition

**A closed circuit.** Every dollar and every share has a named counterparty at every instant; every
asset that has a price has a *cleared* one and shows it; nothing is bounded, plugged or invented;
and the instrument that measures all this is itself true.

---

# PART I — FIRST PRINCIPLES

Nineteen laws. None restates another. They are the grammar every later requirement is written in,
and a proposal that violates one is wrong regardless of how well it works.

## The world

### 1. Reflect the real mechanism

When in doubt, the answer is **how it actually works**, with real named counterparties. Where a real
market has an intermediary, a lag, a fee, a refusal or a failure, the model has one. Ask before a
large scope decision rather than inventing a simplification that is then permanent.

### 2. The fewest primitives that generate the world

A number is a legitimate **primitive** only if no mechanism can produce it. There are exactly three
admissible kinds **of primitive**:

- **TECHNOLOGY** — what a process physically takes: a recipe's input quantities, a lead time, a
  useful life, a storage cost per tonne.
- **PREFERENCE** — time and risk: how patient a decider is, how risk-averse.
- **POLICY** — what an institution chooses: a tax rate, a replacement rate, a regulatory ratio, a
  haircut, a reserve requirement.

Everything else — ownership, prices, quantities, capacities, allocations, shares — is an **OUTCOME**,
and a stated value for one is a defect with a scheduled death.

The consequence that is most often got wrong: a real-world **primitive** may be imported; a
real-world **equilibrium** may not. A statutory tax rate is a primitive. An observed invoicing-
currency share, a foreign-ownership share, a central bank's balance sheet as a share of output, a
sector growth rate — these are *answers*, and importing one means the model can never tell you
anything about it.

Two further distinctions:

- A **target allocation** is a long-term policy guide, never what a participant buys this week.
- **A residual with no holder is a defect, not a boundary.** If a quantity is computed as
  "everything minus the parts we know about", the remainder must be given a named owner or the
  computation is wrong.

Two kinds of number are **not** primitives and are carried anyway, each with its own discipline.

**RESOLUTION parameters** (how many strata, how fine a grid, how long a period is, **how many cells
stand for a population**) are numerical choices, tested by invariance: change them and the answer must
not change materially. **SHAPE parameters** (a distribution's exponent, a tier share, a consumption
ladder) are claims about the answer, and their count is the honest measure of how much mechanism is
missing. That count must fall over time and must never rise. A **PLACEHOLDER** is a shape parameter
with a **scheduled death** — it names the mechanism whose absence it stands in for, and it is deleted
in the change that builds that mechanism (XI-14).

So a declared number is one of **five** things: technology, preference, policy, resolution, or a shape
— and a shape either has a scheduled death, in which case it is a placeholder, or it does not.

### 3. Every price is the result of real supply/demand clearing

Every asset is measured in **UNITS** — par, shares, tonnes, contracts, dwellings, hours — and carries
the cleared **PRICE** those units trade at.

Yield, spread, discount margin, OAS, price/earnings, cap rate: these are **statistics derived from a
cleared price**, never the mechanism that sets it. You *value* a loan on its discount margin and you
*trade* it on price. A mechanism that clears a spread and derives a price from it has the causation
backwards, and every number downstream inherits the inversion.

**One exception.** A central bank's **administered** rates — a deposit facility, a standing lending
facility, a reverse-repo window — are posted rather than traded. They qualify only when the posted
rate has a **real quantity response booked on both balance sheets**: money actually moves to or from
the facility, and both sides record it. A posted rate with no position behind it is not the
exception; it is an invented price.

### 4. "1$ is 1$" — one representation per real thing

Every dollar and every share traces to a named counterparty, and **every fact has exactly one
writer**.

The anti-pattern to hunt and kill is **two disconnected representations of one real thing**: a
cleared ledger and a parallel formula that overwrites or ignores it; a stated receivables ratio
beside a real invoice book; two default probabilities for one borrower; two index systems where one
is read and one is not.

### 5. Every flow has two sides, and both legs go in the same pass

A security movement has a cash leg. A derivative has a counterparty with finite capacity. A payment
leaving one book arrives on another, in the same period, in the same currency.

**A one-sided flow is a defect even when nothing fails and every test passes.**

### 6. No bounds of any kind

No cap, floor, ceiling, clamp, damper, rescale, guard-break, or "not less than zero" standing in for
a decision. And never a printed number resting on a bound — the subtlest form being a bound that
looks like an amount of money and is a percentage in money's clothing.

**The only admissible bound is arithmetic impossibility**: a price cannot be negative; a count cannot
be fractional; a share cannot exceed its whole.

If a number explodes, **the mechanism that should compensate is missing — build it.** A bound
covering a missing mechanism is deleted *in the same change that builds the mechanism*. That pairing
is the only sequencing allowed, and it is not an excuse for the bound to persist.

### 7. A tolerance is arithmetic dust, never a percentage

An identity holds or it does not. A check may forgive only the error the **floating-point arithmetic
itself** introduced — on the order of *(number of terms) × (machine epsilon) × (sum of absolute
magnitudes)*, derived from the size and the **count** of what was added, and orders of magnitude
below anything the model trades.

A percentage band is a business judgement in a numerical costume: it says a thousand units of
currency may go missing if the book is large enough, and law 4 says a unit is a unit whatever it sits
next to.

**A check that only passes with a percentage band is reporting a defect, and the band is hiding it.**
Set a new check's tolerance from the arithmetic, watch it fail, and fix what it names. Never widen
it.

### 8. Periodicity, price level and unit of meaning are part of the number

Every rate, flow and index carries a **period**. Confirm it at the source — where the number is
written, not where it is read — and name it in the identifier: weekly, annual, year-over-year. A
figure denominated in one money is not a share and not a figure in another money.

**A displayed change where no history exists is a lie — show the level.**

### 9. Instruments are named, keyed and shown as a market names them

- A bond is **issuer + coupon + maturity**.
- A loan is **issuer + margin + maturity**.
- A bill or commercial paper is **issuer + tenor**.
- A share is its issuer.
- A good is its own sub-unit, in its own market.

An internal identifier is fine as an identifier and is **never** the display name. And it is never an
invented **grouping** — a tenor bucket, a rating band, a maturity bucket — standing in for the
instrument: a book is keyed by the thing that was actually bought.

Every asset that has a price shows it. Fixed income shows **both the price and the derived spread**.
An asset with no displayed price is one nobody can judge.

## The work

### 10. One ordered list, worked strictly in order, one item at a time

- **In order.** Take the first open item. Not the interesting one, not the one that unblocks
  something else, not the one a measurement made look urgent. Later work depends on earlier work, and
  an item taken out of order is built against a world that has not arrived yet.
- **One at a time.** Finish what you started — checks green, record written — before opening the
  next. A half-done item is worse than an unstarted one: the next reader cannot tell which half is
  true.
- **A new idea is INSERTED, not appended.** When something new arrives mid-project it goes into the
  list at the position its dependencies put it, which is often *before* the item in hand. Say where
  it landed and why there.

### 11. Do not measure, evaluate or diagnose mid-build

**Build the mechanism, then measure.** Numbers taken halfway describe an economy that does not exist
yet. While the model is incomplete, its consistency checks are *deliberately* failing, and a failure
they report is one of hundreds whose cause is an unbuilt mechanism — so diagnosing one of them is
work on a world that does not exist.

Do not chase a moved baseline. Do not A/B. Do not open a run to explain a number. Do not add work
because a number looked wrong.

**A misbehaving number is not a work item; the missing mechanism is.** The single exception is a
number so far out that it *blocks* mechanism work — and then the work is naming which missing
mechanism it is the accumulated cost of.

### 12. Fix the cause, not the symptom

A finding is not understood until you can name the thing that **made** it, and the test of that is
whether the fix **deletes the symptom's plumbing** rather than fencing it.

The tell that you are patching a symptom: the fix is a **list** (add a bound, add a check, add a
flag), it leaves the original mechanism in place, and every item on the list is independently
arguable. **A cause has one fix and it removes code.**

### 13. Never roll back

When a change makes a number worse, the answer is never to restore the old number. A derivation that
replaced an invented constant does not become wrong because the world it now describes is uglier —
**the ugliness was there and the constant was covering it.**

**A bad number is a finding, not a regression.** Only a change wrong *on its own terms* may be
undone.

### 14. One bounded change per item

Never one large unreviewable change. The record of a change says what changed and **why**, for a
reader who was not there.

### 15. The targeted-change test

Adding a product line, a lead time, a revenue rule, a fund type or an instrument must be **one
change**: one registry entry, or one profile.

- All **DATA** lives in a registry.
- All **BEHAVIOUR that varies by kind** lives in a profile behind a dispatch table.
- **No mechanism of the world's evolution may branch on an industry, a sector, an entity type or a
  product identifier.**

A branch that names a kind is a bug report about the registry, not a case to be added.

### 16. Brevity, in the artefact and in the record

A comment earns its place by saying what the artefact cannot: **why** a constant has its value, what
a non-obvious mechanism is, what was tried and failed. A comment describing something that no longer
exists is a defect, and so is a stale reference. The project record is a ledger of **outcomes**, not
a diary.

### 17. No forecast without a falsification test

A record may state an expectation **only** alongside the measurement that would kill it.

### 18. Performance work: depth is untouchable, representation is free

Under a performance campaign, mechanisms, economics and named boundaries **never** change. Storage
layout, traversal order, decomposition and scaffolding removal are the campaign's to decide.

Gate it on **behaviour, not on bits**: the invariant families still hold exactly, and the run's
reported numbers do not move beyond arithmetic noise. A change that is inherently a relabelling is a
*declared* re-baseline, named as such in its record.

### 19. Read the source; do not re-derive it

Every fact in this model has **one place that holds it**. Where such a place exists, a reader
**reads** it. It does not recompute the fact, sum a second copy of it, infer it by subtraction, or
model a price the market already printed.

This is law 4 pointed at the read side, and it has four failure modes, all of which are easy to
reach:

- **The wrong quantity.** Reading a *mark* where the question is *face*, or the reverse. Two numbers
  that were equal at par and stopped being equal the week prices moved.
- **The stale mirror.** Reading a materialised week-end view in the middle of a week. A view is a
  view; the underlying record is the authority.
- **The residual.** Inferring a holding as *total minus the holders we know about*. A residual with
  no holder is law 2's defect.
- **The re-derived price.** Discounting to a price for an instrument whose own auction printed one. A
  bidder's **reservation** is legitimately its own opinion and is not this; a **mark** is not an
  opinion.

Before adding a derivation, name the store that already answers it. **Every deletion under this law
names the read that replaces it** — that is what makes it a deletion rather than a guess.

---

# PART II — THE METHOD

## Why a specification, and not a review

A review of an existing system reads what is there and asks whether it is correct. It finds
**defects**. It cannot find **absences**, because an absence has no location: nothing anywhere says
"and there is no price for credit here", or "and nothing in this world is ever forced to sell", or
"and this currency market does not exist". Everything present is plausible and well-explained;
nothing points at the hole.

**The only instrument that finds an absence is a specification written from the domain with the
implementation shut.** For each system: what must exist for it to be that system, what must be true,
and explicitly what must *not* exist. Then compare.

Two disciplines make it work, and without both it decays into decoration:

1. It is updated **in the same change as the thing it describes**.
2. **A clause is never deleted to make the comparison look better.** If the model deliberately does
   not have something, the clause stays and says so, with the reason. "MISSING" and "OUT OF SCOPE"
   are different answers and the document must distinguish them.

## Nothing is imposed: a requirement states a REASON, never an OUTCOME

This is the most important rule in the method and the easiest to violate.

*"Surplus banks lend and deficit banks borrow"* is **not** a requirement. It is what happens. The
requirement is that **each bank posts a schedule out of its own position, cost and constraints**, and
who ends up on which side is the **result**.

A requirement written as an outcome reads as complete, and then licenses a mechanism that assigns the
outcome directly — which is law 3's defect wearing a specification's clothes.

So every requirement in Parts III–X takes exactly one of three forms, and the form is marked:

- **REASON** — *"it has a cost of funds and a position it wants."* Checkable, and cannot be
  short-circuited by writing the answer down.
- **VERIFY** — *"worse credit trades wider."* A thing to **MEASURE**, never to enforce. A VERIFY that
  fails is a finding about the mechanism, not a licence to clamp the number.
- **FORBID** — *"there is no automatic central-bank overdraft for the treasury."* A requirement that
  something be **ABSENT**. These are the requirements no review of an existing system can ever
  produce, because an implementation cannot show you what it should not have.

**If a clause is none of the three, it is an outcome, and it does not belong here.**

## The two hardest things to get right

**A FORBID that holds is as valuable as a mechanism that works, and it is easy to break silently.**
No buyer of last resort in any auction. No netting across counterparties. No short without a borrow.
No unemployment rate that is anything but a read of real people's states. These are load-bearing, and
a future change must not quietly undo one.

**A VERIFY is not a target.** When a VERIFY fails, the answer is to find the mechanism that should
have produced it. Adjusting the number to satisfy the VERIFY converts a measurement into a
tautology, and the tautology cannot fail — which is the failure mode this entire method is organised
against.

## Granularity: what counts as a system

**A system is something with its own required tree — its own instrument, actor or mechanism that
could be wholly absent.** That is why banks are three systems and derivatives are five: a bank's
lending, its funding and its capital can each be absent independently, and so can each derivative
class.

There are forty-eight, plus two **instrument contracts** — the characteristics any bond must have, and
any derivative must have — which a system cites rather than restating. A sovereign bond and a
corporate bond are different **types**, not one type with fields unused, and the difference between
"this type answers that question differently" and "nobody ever answered that question" is the whole
reason the contracts are written separately.

---

# PART III — THE INFRASTRUCTURE

Six systems. Every other system's cash leg, ownership change and price lands here, so an error here
is an error everywhere.

---

## 1. MONEY AND SETTLEMENT

What money **is**, where it sits, and how it moves.

### A. What money is
- **A1** REASON — money is a **liability of somebody**. Every unit is owed by a named issuer.
  - **A1.a** a **deposit** is a named bank's liability to a named holder.
  - **A1.b** a **reserve** is the central bank's liability to a named bank.
  - **A1.c** **currency in circulation** is the central bank's liability to **whoever holds it**, and
    in this model currency is held in an account like any other money, so the holder is named. A
    bearer nobody names is law 2's residual with no holder, which A1.d forbids one line below.
  - **A1.d** FORBID — **no money without an issuer.** A balance that is nobody's liability is money
    created from nothing, and it is the defect every conservation check exists to catch.
- **A2** REASON — money is **denominated**: a unit is a unit *of* a currency.
  - **A2.a** a holder's own money is the one it keeps its books in.
  - **A2.b** FORBID — **two currencies are never added.** A sum across them is a conversion at a
    stated rate, or it is meaningless.
- **A3** REASON — money is **fungible within its issuer and currency**, and not across them.
- **A4** VERIFY — the **money stock** is a read of A1's liabilities, never a stored aggregate.

### B. Where it sits
- **B1** REASON — an **ACCOUNT** is (holder, issuer, currency). All three, or it is ambiguous.
  - **B1.a** a holder may have several, and holding a foreign currency is a real position.
  - **B1.b** VERIFY — the sum of accounts at an issuer equals that issuer's money liability.
- **B2** REASON — a balance is **carried** period to period, and changes only by a named movement.
- **B3** REASON — a balance can be **negative**, and what that means differs by holder.
  - **B3.a** a customer overdrawn is **borrowing**, and it is a **credit decision by its bank** — the
    bank lends only to the room its own capital supports, and **refuses past it**.
  - **B3.b** a bank overdrawn at the central bank is **borrowing from the central bank**, and the
    corridor prices it.
  - **B3.c** FORBID — an overdraft is **never a silent negative**. Somebody lent it, at a rate, or
    somebody refused it and the refusal is recorded.

### C. How it moves
- **C1** REASON — a **PAYMENT** is an instruction: payer, payee, amount, **currency**, reason.
  - **C1.a** it names both sides. A payment to nobody is not a payment.
  - **C1.b** it carries the **reason** it happened, so a unit is traceable to why it moved.
  - **C1.c** it may be **dated**: an obligation falling due later is an instruction now and cash
    then.
- **C2** REASON — settlement applies each instruction by **one rule**: payer minus, payee plus.
  - **C2.a** and **the interbank leg**: where the two sit at different banks, reserves move between
    those banks. That is what settlement *is*.
  - **C2.b** a same-bank payment moves **no reserves** — it is a relabelling of one bank's liability.
  - **C2.c** VERIFY — the sum of all legs is zero, per currency, every pass — **per leg kind**: a
    transfer's legs sum to zero, and an **issuance** leg is the one exception, because a creator of
    money has no debit anywhere (C4, §31 C2.a). C4.c counts exactly those legs, so the exception is
    the thing being measured rather than a hole in the check.
- **C3** REASON — a payment across currencies is **two amounts and a rate**, and somebody sells the
  currency. Never a restatement of one number.
- **C4** REASON — **the money creators are enumerable and few.**
  - **C4.a** a **bank writing a loan** creates a deposit.
  - **C4.b** the **central bank** buying an asset or lending creates reserves.
  - **C4.c** VERIFY — the money stock's change equals C4.a plus C4.b and nothing else. Any other
    source is A1.d.

### D. The wire — every move is an instruction
- **D1** REASON — **every asset move is a numbered instruction**, money included: from, to, asset,
  quantity, price, reason.
  - **D1.a** numbered, so a position can be **replayed** from its instructions.
  - **D1.b** a residual is therefore a **missing instruction**, never a mystery.
- **D2** REASON — money is an **asset like any other** in this ledger: a quantity of a currency at a
  price of one of itself. *That is the only place a hard-coded price of one is admissible.*
- **D3** VERIFY — for every asset kind, instructions in minus instructions out equals the change in
  holdings.
- **D4** FORBID — **no move without an instruction.** A book that changes with nothing behind it is
  A1.d one level up.

### E. Failure and finality
- **E1** REASON — a payer that **cannot pay** is a real state with a real consequence, and there is a
  named thing it then IS: in default of payment, observable, with a downstream effect on its
  counterparties, its lenders and its standing.
  - **E1.a** it does not silently not happen, and it does not silently overdraw.
  - **E1.b** the payee has a **receivable that did not arrive**.
- **E2** REASON — **settlement is final**: once applied, a payment is not reversed.
  - **E2.a** an error is corrected by a **new** payment in the other direction, itself traceable.
- **E3** REASON — **order matters within a pass**, and the order is defined. Two instructions drawing
  on one balance cannot both succeed by luck.
- **E4** REASON — a party that **ceases to exist** mid-pass still has its legs settled or refused by
  name — never dropped.

### F. What settlement reports
- **F1** REASON — the pass produces a **statement per book**, in that book's own money: a treasury's
  flows, a household sector's, a bank's reserves, a pool's income.
  - **F1.a** FORBID — a per-book statement is **never** a sum across currencies (A2.b).
- **F2** REASON — the **gross** and the **net** are different numbers and both are reported.
- **F3** VERIFY — the clearing house's residual is **zero**, per asset and per currency.
- **F4** VERIFY — money that landed on a holder with no account is **counted, never dropped**.

### G. The clock
- **G1** REASON — a period contains **more than one settlement cycle**, because a day has more than
  one and a period's money must settle inside the period. An instruction created by a mechanism late
  in the period still moves cash before the period closes.
- **G2** REASON — a **cycle** is the finest structure time has here. There is no clock inside a cycle,
  and an instruction belongs to the cycle it was issued in.
- **G3** REASON — **one calendar**: an epoch, a period length, one mapping from a period to a date,
  and one placement of **every periodicity** on that grid **by date**, read by everything.
  - **G3.a** a periodicity is placed by advancing a date, never by a fixed count of periods. A
    quarter is three months of calendar, which is a whole number of periods only by accident; a
    payment lands in the first period on or after its date.
  - **G3.b** FORBID — **no periodicity finer than a period.** It cannot be placed, and rounding it to
    the period is a payment moved to a date nobody chose. Finer structure is G1's cycles.
  - **G3.c** an accrual convention is a **day count**, read from the calendar's dates. A convention
    computed from a count of periods is a second calendar (law 4).
- **G4** REASON — **every instruction, movement, print and claim carries the period it belongs to.**
  - **G4.a** FORBID — **no default period.** A number that means "now" and "unset" and "the beginning"
    at once cannot age anything, and an arrear that cannot age is a flag.

---

## 2. THE REGISTER

Who holds what, in what units, and what happens to it.

### A. What a holding is
- **A1** REASON — a holding is a **triple**: named holder, named instrument, quantity.
  - **A1.a** the holder is a party that exists in the world and can be paid.
  - **A1.b** the instrument is one the issuer actually issued, in the size it issued.
  - **A1.c** the quantity is in the instrument's **own unit** — face, shares, units, contracts — and
    the unit travels with the number.
- **A2** REASON — a holding is a **claim on a named issuer**, not a token.
  - **A2.a** everything the instrument pays, it pays to whoever the register says holds it, then.
- **A3** FORBID — **no holding without a holder.** A residual position on nobody is a defect, not a
  rounding line.
- **A4** FORBID — **no holding without an issuer.** A claim on a party that never issued it is money
  invented in the ownership dimension.

### B. The issuer side
- **B1** REASON — an instrument has an **issued amount**, set when it was issued and changed only by
  an issuance, a re-opening, a buyback, an amortisation or a maturity.
- **B2** VERIFY — **holdings sum to the issued amount**, per instrument, always.
  - **B2.a** a shortfall means somebody's claim vanished; a surplus means somebody's was invented.
  - **B2.b** the tolerance is arithmetic dust, never a fraction of the issue.
- **B3** REASON — the issuer's **liability** is the same number read from the other side, never a
  second stored copy.
- **B4** REASON — an instrument **ceases** — matures, is redeemed, defaults into a recovery — and when
  it does, every holding in it resolves to something else, named.

### C. Transfer
- **C1** REASON — ownership changes only by a **transfer with two named sides**: the seller loses
  exactly what the buyer gains, both legs in one pass.
- **C2** REASON — a transfer has a **cause**: a trade, a maturity, a corporate action, a default.
  - **C2.a** and a **price**, if it is a trade, which is the print the market sees.
- **C3** REASON — the securities leg and the cash leg are **the same event**.
  - **C3.a** delivery versus payment: neither leg happens without the other.
  - **C3.b** a **fail** is a real state, not a silent half-settlement.
- **C4** FORBID — **no short by accident.** A party cannot deliver what it does not hold; a deliberate
  short is a **borrowed** position, which is a different thing with a lender on the other side.
- **C5** VERIFY — over any window, quantity bought equals quantity sold, instrument by instrument.

### D. What the register must answer
- **D1** REASON — **what does this party hold?** — the portfolio, for valuation and for risk.
- **D2** REASON — **who holds this instrument?** — the holder list, for paying a coupon and for
  finding who takes the loss on a default.
  - **D2.a** both directions must be answerable; a register that answers only one forces the other to
    be reconstructed, and a reconstruction drifts.
- **D3** REASON — **what is it worth?** — quantity times a price that came from a market, never a
  price stored on the holding.
- **D4** REASON — **what did it cost?** — the **basis**, because a realised gain is a real number and
  somebody is taxed on it. A position is a chain of **lots**, each carrying what those units cost;
  a sale draws lots by a stated and consistently applied flow assumption.
- **D5** REASON — **what of it is bound?** — a **lien** on units of a position, because pledged paper
  can be neither sold nor counted free, and a register that cannot say so lets one unit answer two
  claims.
  - **D5.a** free units are held units minus encumbered units, and **only free units can move**.
  - **D5.b** re-pledging is a **chain**, and the chain is traceable — it is how a single default
    reaches parties that never traded with the defaulter.

### E. Corporate actions and events
- **E1** REASON — a **coupon or dividend** pays to the holders of record, in the instrument's
  currency, and reaches their accounts.
- **E2** REASON — an **amortisation or maturity** reduces or extinguishes the holding and pays its
  face.
  - **E1.a** the **holders of record are read at the moment the action is applied** — there is no
    earlier announcement to remember, and A2.a already says everything the instrument pays, it pays to
    whoever the register says holds it, **then**. A buyer between two payment dates gets what it is
    owed through **the price**: N9.b's accrued travels with the trade, so the coupon is not a windfall
    to whoever happens to hold it on the date.
- **E3** REASON — a **default** converts the holding into a recovery claim; the loss lands on the
  holders, in proportion, and on nobody else.
- **E4** REASON — a **split, buyback or new issue** changes quantities on both sides at once.
- **E5** VERIFY — every event that moves a register also moves money, or explicitly says why not.

### F. Identity and survival
- **F1** REASON — an instrument has a **stable identity** for its whole life.
  - **F1.a** two instruments with the same terms from the same issuer are still two instruments.
- **F2** REASON — a party that **ceases to exist** leaves its holdings to a named successor — an
  estate, a buyer, the state — never to nobody.
- **F3** VERIFY — the register survives a period boundary unchanged unless an event in C or E moved
  it; unexplained drift is a defect.

---

## 3. THE CLEARING ENGINE

The one mechanism every market uses. One participant interface, one solver, one meaning of "price".

### A. What a market is
- **A1** REASON — a market is **two or more parties with different reasons** to want the same thing
  at different prices.
  - **A1.a** the differences **are** the market; identical participants have nothing to trade.
- **A2** REASON — each participant posts a **schedule**, not a point: how much at each price.
  - **A2.a** because a single quantity cannot answer *"and if it were cheaper?"*, which is the only
    question the mechanism asks. A market expressed as "here is the quantity I want" has no level,
    only a shape, and forces every venue to invent its own rule.
- **A3** REASON — the schedule comes from the participant's **own state**: its position, its cost of
  funds, its mandate, its view, its constraints.
- **A4** FORBID — **no participant is a price-taker of a price this mechanism has not yet produced.**
  A schedule written against the clearing price is the answer smuggled into the input.

### B. Who is in the room
- **B1** REASON — the participants are **named parties** with balance sheets that will actually
  change.
- **B2** REASON — a party is present **because it has a reason to be**: a maturity to roll, a mandate
  to fill, an inventory to shed, a view.
- **B3** REASON — a **dealer** may be there, and its reason is inventory and spread, which is a reason
  like any other.
  - **B3.a** it has a **limit**: capital, risk, inventory. A dealer without a limit is a synthetic
    counterparty wearing a dealer's name.
- **B4** FORBID — **no buyer of last resort by construction.** No participant exists whose schedule is
  *"whatever is left, at whatever price"*. A central bank is a participant with a policy reason and a
  stated facility, never a residual absorber.
- **B5** FORBID — **the mechanism does not add demand to make itself clear.** If it did, the price it
  produces is a fixed point of its own patch, and law 3 is satisfied in letter only.

### C. The clearing
- **C1** REASON — the mechanism finds the price where **posted supply meets posted demand**.
- **C2** REASON — the price is **discovered, not assigned** — it is a root of the schedules, and
  changing an input must be able to change it.
- **C3** REASON — quantity is **rationed** when the two sides are unequal at the clearing price, by a
  stated rule (pro rata, priority, time).
- **C4** REASON — **it can fail to clear.** No overlap is a real outcome.
  - **C4.a** a failed auction has consequences: the issuer does not get its money, the seller keeps
    its inventory, the maturity is not rolled — and those consequences propagate.
  - **C4.b** failure must be **representable and observable** in the mechanism's own output, not an
    exception path that quietly substitutes a price. There are at least three distinct outcomes —
    *cleared*, *nobody wants any at any level*, and *committed demand exceeds the float at every
    level* — and a book that did not clear carries **last period's** statistic, marked as stale.
  - **C4.c** FORBID — **a solver's search bracket can never be a print.** A bound of the search is
    not a price.
- **C5** VERIFY — the clearing price is a **function of the schedules alone**; feed the same schedules
  twice and get the same price.

### D. What comes out
- **D1** REASON — a **price**, in a stated unit, for a stated instrument, at a stated time.
  - **D1.a** price is the primitive; yield, spread, discount margin and OAS are **derived from it**,
    never the other way round.
- **D2** REASON — a set of **trades**, each with two named sides and a quantity.
- **D3** REASON — the trades hit the **register and the accounts in the same pass**.
- **D4** REASON — the price becomes the **mark** for everyone holding that instrument, and the
  revaluation is real money to somebody.
- **D5** VERIFY — bought equals sold, and cash paid equals cash received, per clearing.

### E. The print and what it means
- **E1** REASON — the price is **public**: other participants and other markets can see it.
- **E2** REASON — one market's print is another's **input** — a bond print moves a credit derivative,
  an equity print moves a merger, a funding rate moves everything.
- **E3** REASON — the **bid–offer is a consequence** of what dealers post, read off the schedules; it
  is never a prior applied to a mid.
- **E4** VERIFY — a market with no trades has **no new print**, and the stale mark must be **visibly
  stale** rather than silently refreshed.

### F. Order and time
- **F1** REASON — a market clears at a **stated point in the period**, and what it can see is what has
  already happened.
  - **F1.a** a market that needs a number produced later in the same period is in the wrong place —
    that is an ordering defect, and the fix is the order, not a forward reference.
- **F2** REASON — a **rate in force for the period is one rate**: a participant cannot value at one
  and settle at another.
- **F3** VERIFY — moving a market earlier or later changes results; if it does not, it is not reading
  anything the rest of the period produces.


---

## 4. THE AUDIT — WHAT MUST BE TRUE

The instrument that measures consistency. It is not the instrument that measures completeness; that
is this document.

### A. What an invariant is
- **A1** REASON — a statement that must be true of the **state**, checkable without knowing how the
  state was produced.
  - **A1.a** it is a read of **two independent things that must agree** — never a read of one thing
    against itself, which always passes.
- **A2** REASON — an invariant that fails must name **who**: a party, an instrument, an instruction.
  - **A2.a** a violation with no owner cannot be fixed, only tolerated, and tolerated violations
    become the baseline.
- **A3** REASON — and **how much**, in a unit, so the size is comparable period to period.
- **A4** REASON — the tolerance is **arithmetic dust** — the accumulated representation error of the
  arithmetic that produced the number — never a percentage of it.
  - **A4.a** a percentage tolerance hides a defect that scales, which is every defect that matters.

### B. The families — what must be true
- **B1** REASON — **money is conserved**: every payment has a payer and a payee, and the sum over all
  accounts changes only by an act of a money issuer.
- **B2** REASON — **ownership is conserved**: holdings sum to issued, per instrument.
- **B3** REASON — **prices exist and are cleared**: every instrument anyone marks has a price that
  came out of a mechanism.
- **B4** REASON — **cross-market consistency**: the same economic thing has one value however it is
  reached — a bond's price and its derived spread, a share and the index containing it, a future and
  its underlying.
- **B5** REASON — **accounts balance**: assets minus liabilities, **read** from the register and the
  ledger, equals the entity's **equity account** — a stated account moved only by capital paid in,
  income retained and losses booked.
  - **B5.a** the two are **independent records** and equality is the check. Equity defined as the
    residual would make this a read of one thing against itself, which A1.a forbids and which Equity
    B4.a names as a tautology that cannot fail.
  - **B5.b** neither side is stored **as a total**: the residual is computed at read, and the equity
    account is a balance moved by named events, exactly as B2 requires of any balance.
- **B6** REASON — **names resolve**: every party referenced exists; every issuer of a held instrument
  exists or has a successor.
- **B7** REASON — **flows are complete**: a flow leaving one place arrives somewhere named, in the
  same period, in the same currency.
- **B8** VERIFY — the families are **independent**: one defect should light one family, and a defect
  that lights five means the families overlap.

### C. When it runs
- **C1** REASON — at a point where the state is **supposed to be consistent** — after settlement, not
  in the middle of it.
- **C2** REASON — **every period**, so the period a violation first appears is known.
  - **C2.a** a violation's first period is the strongest evidence about its cause, and it is lost if
    the audit is sampled.
- **C3** REASON — with the **same invariants every period**: an invariant skipped in some periods
  measures the skipping, not the world.
- **C4** FORBID — **the audit never repairs.** It observes. A checker that fixes what it finds
  destroys the evidence and guarantees the cause survives.

### D. What it produces
- **D1** REASON — a **count by family**, comparable across runs, so a change can be attributed.
- **D2** REASON — the **worst instances**, with party and size, so a cause can be chased.
- **D3** REASON — a **run is reproducible**: same seed, same periods, same violations.
- **D4** VERIFY — runs of different lengths tell different stories, and the difference between them is
  itself a measurement: what accumulates versus what fires once.

### E. What the audit cannot do
- **E1** FORBID — **it cannot find an absence.** No invariant fires because credit has no price or
  because a currency market does not exist; there is nothing to be inconsistent with.
- **E2** REASON — so the audit measures **consistency**, and this document measures **completeness**,
  and neither substitutes for the other.
- **E3** VERIFY — a green audit alongside unmet requirements here is the **normal** state of an
  incomplete model, not a contradiction.


---

## 5. THE SEED — THE OPENING WORLD

### A. What the seed must be
- **A1** REASON — a **complete, consistent state**: every party, every account, every holding, every
  instrument, all present at once.
- **A2** REASON — it must **pass the audit at period zero**.
  - **A2.a** a violation present at period zero is the seed's, and attributing it to a mechanism costs
    weeks of the wrong search.
- **A3** REASON — it is a **stock**, and stocks are what the flows then act on.
- **A4** FORBID — **no free money and no free assets.** Every deposit is some bank's liability; every
  holding is some issuer's; nothing exists because something needed it to.
- **A5** REASON — it is **reproducible from a seed value**, so any run can be re-run.

### B. Who exists
- **B1** REASON — a **population of each type**: households, firms, banks, funds, insurers, the state,
  the central bank — enough of each that the type is a **distribution**, not a single instance.
  - **B1.a** a type whose members are named individually is seeded one party per member; a type
    represented as **cells** (XI-15) is seeded as an ensemble whose **weights sum to the population it
    stands for**, and that sum is a read, never a target the cells were fitted to.
- **B2** REASON — each has an **identity** that survives the whole run.
- **B3** REASON — each is placed in a **region**, and the region determines its money.
- **B4** REASON — each has a **size**, and the sizes are **dispersed**: a sector of equals never
  produces a market.
- **B5** FORBID — **no observed real-world ratio is copied in.** A share, a spread, a leverage ratio
  taken from data is an answer written down where a mechanism should be.

### C. What they hold
- **C1** REASON — every party's balance sheet **balances at period zero**, and its equity is the read.
- **C2** REASON — each asset is **somebody's liability**, party by party, not sector by sector.
- **C3** REASON — instruments outstanding at period zero have **terms and a remaining life** — a bond
  seeded at issue is a world with no maturity wall for its whole tenor.
  - **C3.a** and a **maturity profile that is spread**, or every roll arrives in the same period.
- **C4** REASON — prices at period zero are **the first clearing's inputs**, not permanent marks.
  - **C4.a** a seeded price that never clears is law 3's defect, seeded.
  - **C4.b** the distinction that must be written into the specification of every seeded instrument:
    an **opening condition** (a price, which the next period re-clears) is not a **term** (a coupon,
    which is fixed for the instrument's life). Seeded terms are permanent structure and must be
    justified individually, never drawn from a table by default. A seeded spread table that strikes
    every coupon in the world is a permanent cash flow, not an opening guess.
- **C5** VERIFY — the period-zero balance sheet of each **sector** is a read of its members, never a
  target the members were fitted to.

### D. Consistency with the flows
- **D1** REASON — the stocks must be **consistent with the flows that will run**: debt with a coupon
  somebody can pay, employment with a wage bill somebody can meet, a production line with the work in
  progress its lead time implies.
  - **D1.a** otherwise period one is a shock the model never recovers from, and everything measured
    afterwards measures the recovery.
- **D2** REASON — anything that **accrues** starts from a stated accrual position.
- **D3** VERIFY — with all shocks off, period one should be **quiet**: large first-period flows are the
  seed disagreeing with the mechanism, and they are a finding.
- **D4** VERIFY — the seed is a **fixed point of nothing**: running with no shocks must still evolve,
  because agents have reasons that differ.

### E. What the seed must not decide
- **E1** FORBID — **no outcome is seeded.** A seeded default rate, a seeded market share, a seeded
  spread curve is the result assigned in advance.
- **E2** REASON — the seed sets **reasons and endowments**; the mechanism produces outcomes.
- **E3** VERIFY — changing a seed parameter must change outcomes **through a chain that can be
  traced**, and if it changes an outcome directly, that outcome was seeded.

---

## 6. CURRENCY AND EXCHANGE RATES

### A. What a currency is
- **A1** REASON — a **unit of account** in which claims are denominated and settled.
- **A2** REASON — issued by a **named issuer** — a central bank — whose liability it is.
- **A3** REASON — it is a **property of every amount**: every balance, price, coupon, payment and
  contract carries one.
  - **A3.a** a function that takes an amount takes its currency with it, or it operates over a domain
    it cannot check.
- **A4** FORBID — **no implicit currency.** An amount whose currency is inferred from where it was
  found is inferred wrong exactly when it matters — a foreign holding, a cross-border payment.
- **A5** REASON — currencies are a **closed, named set**; a party can hold any of them.

### B. Who is in which
- **B1** REASON — a party has a **home currency**, the one its region uses and it reports in.
- **B2** REASON — a party can hold **any currency it has acquired**, and holding one is holding a
  claim on that currency's banking system, not a converted number.
  - **B2.a** so a party has an **account per currency it holds**.
- **B3** FORBID — **no conversion at the ledger boundary.** A payment in one money lands as that
  money; the decision to convert is a **separate, explicit trade with a counterparty and a rate**.
  Converting on arrival makes the currency market invisible and unmeasurable — the position never
  exists, so it can never be seen to be wrong.
- **B4** REASON — a party **short a currency it owes must acquire it**, from somebody, at a price.
- **B5** REASON — a **bank's foreign balance is its business**, not a client conversion: it is the
  other side of its clients' trades, and it is a position it chooses to run or square.

### C. What a rate is
- **C1** REASON — the **price of one currency in another**, and it is a price like any other: cleared,
  not assigned.
- **C2** REASON — it is **directional and consistent**: the rate one way is exactly the reciprocal of
  the rate the other.
- **C3** REASON — it is **transitive**: a route through a third currency and the direct route agree,
  or there is an arbitrage and somebody must be taking it.
  - **C3.a** which means the rates are **one object**, not a table of independent pairs.
  - **C3.b** and it means **no currency is the vehicle by construction**. Every pair clears on its own
    flow; whether one currency becomes the cheapest route is an **outcome**, and a conversion that
    triangulates through a chosen numéraire by design has assigned that outcome.
- **C4** REASON — a **numéraire** exists for reporting and aggregation only.
  - **C4.a** FORBID — the numéraire is **not where value lives.** Storing every balance in the
    numéraire and converting on read destroys the currency position, which is the thing that gains and
    loses money.
- **C5** VERIFY — the rate used to value and the rate used to settle are **the same rate**.

### D. Time and revaluation
- **D1** REASON — a rate is **in force for a stated period**, and every use in that period uses it.
  - **D1.a** a rate that changes mid-period lets a book be valued at one and paid at another, and the
    difference lands as an unexplained residual on somebody.
- **D2** REASON — when the rate moves, every foreign position **revalues**, and the revaluation is a
  real gain or loss to a named party.
  - **D2.a** it hits equity for a firm or a bank, and a revaluation account for a currency's issuer.
  - **D2.b** FORBID — **an unrevalued foreign position is money created or destroyed silently.**
- **D3** REASON — revaluation happens **before anything uses the new rate**, so nothing values at the
  new rate against a book still carried at the old.
- **D4** VERIFY — revaluation gains plus losses equal the rate move applied to the net open position,
  and the net open position across all parties in a currency is what its issuer and the rest of the
  world hold.

### E. What moves a rate
- **E1** REASON — the rate moves because **somebody trades at it**.
- **E2** REASON — the reasons participants have are real: **trade flows**, **rate differentials**,
  **portfolio shifts**, **hedging demand**.
  - **E2.a** a rate differential is a reason to hold one currency over another, and the cost of
    hedging it away is why the reason does not automatically become a free lunch.
- **E3** FORBID — **no exogenous rate path.** A rate that follows a written series is not a price, and
  everything derived from it inherits that.
- **E4** VERIFY — persistent one-way flow should move the rate; if it does not, the mechanism is not
  reading the flow.

---

# PART IV — THE INSTRUMENT CONTRACTS

A contract is not a system. It is the set of characteristics **any** instrument of that family must
have. A system whose subject is such an instrument **cites** the contract and states only where its
type answers a node **differently** — because the difference between *"this type answers that
question its own way"* and *"nobody ever answered that question"* is exactly what this document
exists to expose.

There are two, and there is room for more as the world grows.

---

## THE BOND — FOURTEEN CHARACTERISTICS

An instrument missing one of these is not a bond.

- **N1** REASON — an **ISSUER** who owes: a named party with a balance sheet that can be looked at.
- **N2** REASON — **PRINCIPAL**, an amount owed, counted in **units of par**.
- **N3** REASON — a **CURRENCY** it is denominated in, and every figure about it is in that money.
- **N4** REASON — a **MATURITY**: the date the principal is due.
- **N5** REASON — a **COUPON**, the compensation for time and risk, in exactly one of three shapes:
  - **N5.a** a **fixed** rate, locked at issuance;
  - **N5.b** a **floating** margin over a **named reference rate that is itself observable and
    transacted**;
  - **N5.c** **zero** — the return is the discount to par.
- **N6** REASON — a **PERIODICITY AND AN ACCRUAL CONVENTION**: how often it pays, and how interest
  accrues between payments. A rate without its periodicity is not a number.
- **N7** REASON — a **PRICE, per unit of par, that it changes hands at**.
  - **N7.a** cleared from real demand against real supply, once per period.
  - **N7.b** FORBID — **the price is never derived from the yield, the spread, the discount margin or
    the OAS.** Those are derived *from* it. A round trip through a curve cannot return the level it
    started at, and where one exists the print is arithmetic wearing a market's clothes.
- **N8** REASON — a **HOLDER OF RECORD**: who owns how many units.
  - **N8.a** VERIFY — units held sum to units issued, always. A unit with no holder, or with two, is a
    defect and not a rounding.
- **N9** REASON — **TRANSFERABILITY**: it can change hands.
  - **N9.a** two legs in the same pass — the paper one way, the cash the other.
  - **N9.b** **accrued interest travels with it**: the buyer pays the seller what has accrued since
    the last payment, or the coupon is a windfall to whoever happens to hold it on the date. A price
    is quoted **clean**; what settles is clean plus accrued.
- **N10** REASON — **REDEMPTION**: the principal is repaid and the instrument **ceases to exist**. The
  register empties.
- **N11** REASON — an **EARLY-TERMINATION REGIME**, stated even when it is *"none"*: callable,
  prepayable, make-whole, non-call period, soft call, or not terminable early.
  - **N11.a** whatever it is, it has a **price** the issuer pays to use it.
- **N12** REASON — a **DEFINITION OF DEFAULT**: what counts as failure to perform, **observable by a
  holder**.
- **N13** REASON — a **CLAIM ON FAILURE**: what the holder is entitled to, stated even when the answer
  is *"nothing seizable"*.
  - **N13.a** and a **RANKING** of that claim against the issuer's other claims — stated even when the
    answer is *"all equal"*.
- **N14** REASON — an **IDENTITY a market would use**: issuer + coupon + maturity, or issuer + tenor.
  An internal identifier is an identifier, never the name.

### How each type answers differently

| | corporate bond / loan | sovereign bond / bill |
|---|---|---|
| **N11** early termination | make-whole, non-call, soft call — stamped at issuance from what the issue **is** | **typically none**; the issuer manages its curve by buyback and switch instead |
| **N12** default | missed payment **or breached covenant** | **missed payment only** — there are no covenants to breach |
| **N13** claim | a claim on an **estate** that is realised and distributed | **nothing seizable**; a negotiated exchange, and the sanction is market exclusion |
| **N13.a** ranking | a real **waterfall**: senior paid in full before subordinated | **pari passu, always** — the ranking exists and never varies |
| **N5** coupon | fixed or floating; floating is the norm in the loan market | fixed for bonds, zero for bills; floating is rare |
| **N3** currency | usually the issuer's own, sometimes not | its own or another's — **and that single difference is the whole of its credit risk** |

**The consequence, stated because it is the trap.** *"A sovereign is the same construction as a
corporate bond"* is right about N1–N10 and N14 and wrong about N11–N13.a. A sovereign that inherits a
seniority attribute whose only correct value is a constant, and a covenant slot that must stay empty,
has two attributes that are second representations of nothing — which law 4 forbids and law 2 calls a
primitive that should not exist. The right shape is the contract above, with each **type** answering
N11–N13 its own way.

---

## THE DERIVATIVE — TWELVE CHARACTERISTICS

A derivative is **not a claim on an issuer**. It is a **bilateral obligation between two parties,
both of which can lose.** Every characteristic below follows from that one difference, and an
instrument missing any of them is not a derivative — it is a number moved between two accounts for a
reason nobody wrote down.

- **D1** REASON — **TWO NAMED COUNTERPARTIES**, and the contract is an asset to one and a liability to
  the other, at every instant.
  - **D1.a** FORBID — **no derivative with one side.** A payoff received from nobody is invented
    money.
  - **D1.b** VERIFY — marks across all parties to a contract sum to **zero, exactly**. This is the
    invariant that distinguishes a derivative from a security.
- **D2** REASON — a **NOTIONAL**, in a unit, which scales the payoff and is generally **not
  exchanged**.
  - **D2.a** so the notional is **not the exposure**, and the two must never be conflated.
- **D3** REASON — an **UNDERLYING** that is **observable and priced elsewhere**: a rate, a price, an
  index, a credit event.
  - **D3.a** FORBID — **no underlying that only exists inside the derivative.** Then the payoff is
    unfalsifiable and the contract prices itself.
- **D4** REASON — a **PAYOFF FUNCTION**: what one party owes the other as a function of D3.
- **D5** REASON — a **CURRENCY per leg**, and the legs need not share one.
- **D6** REASON — a **TERM**: a start, an end, and payment dates in between.
  - **D6.a** with a **periodicity and accrual convention** on any periodic leg.
- **D7** REASON — a **PRICE AT INCEPTION**: the rate, spread or strike at which the two sides agree to
  enter.
  - **D7.a** it is **cleared** from what the two sides were willing to do, never solved for.
  - **D7.b** many derivatives are struck **at par** — zero value at inception — and then the price
    **is** the fixed rate or spread that makes it so. That is still a cleared price, and it must come
    out of a mechanism rather than out of the valuation formula run backwards.
- **D8** REASON — a **MARK**: its value after inception, which moves and is not zero.
  - **D8.a** and the mark is a **real gain to one party and a real loss to the other**.
- **D9** REASON — **COLLATERAL AND MARGIN**, because D8 means one side is exposed to the other.
  - D8 and D9 together are why a derivative moves cash even when nothing has been paid on D4.
  - **D9.a** posted collateral **leaves the poster's free balance**. It is still owned and will come
    back; it is no longer available.
- **D10** REASON — **COUNTERPARTY CREDIT**: the other side can fail before the contract ends, and then
  the in-the-money party has a **claim on an estate**, not a payoff.
  - **D10.a** which is why D9 exists, and why **who you face is part of what the contract is worth** —
    a reservation for a trade carries a term for the counterparty, or a weak dealer never loses flow.
- **D11** REASON — **TERMINATION**: it expires, is closed out, or is torn up — and on termination it
  **ceases to exist on both books at once**.
  - **D11.a** early termination on default has a **stated close-out value**.
- **D12** REASON — an **IDENTITY**: counterparties + underlying + term + strike. Two contracts on the
  same underlying with different strikes are two contracts.

### What a derivative is not

- **X1** FORBID — **it is not a holding.** Nobody issued it, so it does not enter the issued-amount
  check; it enters the **zero-sum** check (D1.b).
- **X2** FORBID — **it is not a way to get an exposure for free.** The cash it moves — margin,
  premium, periodic payments — is real and comes out of a real account.
- **X3** FORBID — **it is not a substitute for the underlying market.** If the derivative's price is
  computed from a model and the cash market's price is computed from the derivative, neither has been
  cleared and law 3 is broken in a loop.

---


---

# PART V — THE MARKETS

Sixteen systems. Each is an instrument or a venue that could be wholly absent.

---

## 7. CORPORATE CREDIT

Satisfies **the bond contract** in full. This system covers the **market**, the **holder** and the
**life** around a bond, and states only where corporate answers a contract node its own way.

### A. The issuer and the promise
- **A1** REASON — a named legal entity with a balance sheet that can make a promise.
- **A2** REASON — it has a **capital structure and a reason for it**.
  - **A2.a** a mix of debt kinds and seniorities, each a real instrument.
  - **A2.b** a **target or a constraint** it is managing towards — leverage, coverage, a rating it
    wants — so that issuing is a **decision** and not an accounting consequence. That target is the
    management's own: a lender's covenant line moderated by the management's own risk aversion, and
    approached at the management's own pace.
  - **A2.c** VERIFY — the structure that results is an outcome of A2.b meeting the market's price,
    never assigned.
- **A3** REASON — a capacity to service: operating cash flow, and the coverage of the service by it.
  - **A3.a** the service is interest **plus scheduled principal**, both real payments.
  - **A3.b** coverage is a read, and it can fall below one.
- **A4** REASON — creditworthiness is **ASSESSED, and the assessment is an OPINION HELD BY SOMEBODY**
  — an agency, or each holder's own model. **It is not a property of the firm.**
  - **A4.a** the assessor is named, and can be wrong.
  - **A4.b** assessments **disagree**, and the disagreement is what makes two sides of a market. One
    universal rating means every participant agrees about credit by construction, which removes the
    demand dispersion the auction needs.
  - **A4.c** an assessment **changes**, and the change is an event other participants react to.

### B. How corporate answers the bond contract
- **B1** REASON — **N11**: a real early-termination regime, stamped at issuance from what the issue is
  — make-whole for investment grade, a non-call period for high yield, a soft call for floating paper.
- **B2** REASON — **N12**: default is a missed payment **or a breached covenant**.
  - **B2.a** **covenants exist**: promises about the issuer's conduct — leverage, coverage, restricted
    payments — whose breach is an **observable event**. Covenants are how credit risk is observed
    *before* a default; without them the only credit dynamic the model has is the binary one, and an
    assessment has nothing to update on between "paying" and "gone".
  - **B2.b** a breach can be **waived or cured**, at a price, and that negotiation is real.
- **B3** REASON — **N13**: a claim on an **estate**, and **N13.a**: a real seniority ranking that
  **varies by instrument and is honoured by the waterfall**. A seniority that changes the price and
  not the payout is not a seniority: if there is no state of the world in which subordination costs
  the holder anything, nothing makes it trade wider, and the ordering can never hold.
- **B4** REASON — **N5**: fixed or floating, and floating is the norm in the loan market.

### C. The primary market
- **C1** REASON — a new issue is **BROUGHT** by a named underwriter or arranger, appointed and paid.
- **C2** REASON — a **book is built**: real buyers indicate real demand at real levels.
  - **C2.a** an indication is a **schedule** — a size at a level — not a quantity.
  - **C2.b** the book is **information**: its size and shape decide where the deal prices.
- **C3** REASON — it **prices**: one level struck at which the book fills.
- **C4** REASON — the issuer has a **WALK-AWAY**, and a pulled deal never traded and never existed.
- **C5** REASON — **allocation**: who got how many units, decided out of the book.
- **C6** REASON — proceeds reach the issuer as cash, **net of a fee that reaches the underwriter**.
- **C7** REASON — the underwriter **bears risk between commitment and placement**.
  - **C7.a** it can be **left holding** paper it could not place, and that is its own position.
  - **C7.b** VERIFY — the fee it earns and the risk it takes are related; a fee with no risk behind it
    is a transfer.
- **C8** REASON — a deal may be a **TAP**: added face on an instrument that already exists and already
  prices, cleared in the same solve as its outstanding stock, at its own price, with the issuer's
  walk-away riding on it. A tap does not create a second instrument, and its buyers' holdings are
  holdings in the existing one. A debut, or an issuer with no printed paper near the tenor, brings a
  fresh instrument instead.
- **C9** REASON — a **committed facility** is one line per lender per borrower. A draw taps the
  existing line at the margin it was struck at; a new line opens only when none is live, at the margin
  the lender quotes now, for a stated term. A draw does not mint a facility per period.
- **C10** REASON — **a syndicate.** When an issue exceeds what one underwriter's own limit (Dealer
  Desks D1, D2) can carry between commitment and placement, the **lead** forms a syndicate of **named**
  banks, each taking a **stated share** of the underwriting risk and of the fee (C6, C7). Each member's
  share sits against **its own** limit and its own capital, and a syndicate that cannot be filled to
  the deal's size is a deal **downsized or pulled** (C4) — never one carried by a member past its limit.
  - **C10.a** the shares are struck **before the book opens** and each member is left holding **its
    share** of what the book did not take (C7.a); the lead's share is its own, not the remainder.
  - **C10.b** FORBID — **no syndicate share above a member's own limit.** The lead cannot lend a member
    capacity it does not have, and a member cannot be assigned what it did not agree to carry.
  - **C10.c** VERIFY — the largest deal the market can bring is bounded by the **sum of the willing
    members' limits**, and a deal larger than that fails to find a syndicate — an observable event
    with a named issuer, not a deal that silently shrinks to fit.
- **C11** REASON — **a placement has a stated basis**, chosen by the issuer as a decision, and the two
  bases are different products with different prices:
  - **C11.a** **best effort** — the bank is an **agent**: it builds the book and places what the book
    takes, commits **nothing** and carries **no balance-sheet risk**; what the book did not take is
    **not issued**, and the issuer bears the placement risk — a deal smaller than it wanted, or pulled
    (C4). The fee is lower because no risk sits behind it (C7.b). A selling group sharing that fee is
    named like a syndicate (C10) but commits nothing.
  - **C11.b** **backstopped** — the bank **underwrites**: it commits to take what the book does not
    (C7, C7.a) and is paid for the risk; a syndicate (C10) is the backstopped basis shared among named
    members. This is the basis an issuer that needs certainty of funds pays for.
  - **C11.c** the issuer chooses the basis from **its own** outlook of the book against the fee
    difference the banks quote — an issuer confident of demand goes best effort and saves the fee; one
    that must have the money buys the backstop. The basis is stamped on the deal and reported.
  - **C11.d** FORBID — **no best-effort deal that leaves the agent holding paper**, and no backstopped
    deal whose underwriter does not. On a best-effort basis the unplaced remainder is never issued; on a
    backstopped basis it is the underwriter's position. A fee with no stated basis is a transfer with
    no reason.
  - **C11.e** VERIFY — the backstop fee exceeds the best-effort fee for the same issuer and size, as a
    consequence of the risk behind it, and issuers with weaker credit or less confidence of demand buy
    the backstop more often. Measure, never enforce.

### D. The secondary market
- **D1** REASON — holders who want out and buyers who want in **post schedules**, and who trades is
  the outcome.
- **D2** REASON — **a PRICE clears**. Everything else about value is derived from it.
- **D3** REASON — a **dealer intermediates**, and it is a real party.
  - **D3.a** it quotes **both sides** out of its own inventory and its own cost.
  - **D3.b** it is bounded by its **balance sheet and capital**, and the bound bites.
  - **D3.c** its quote widens as its inventory fills — a reason, not a rule.
- **D4** REASON — the dealer earns the **bid-offer on the flow it facilitates**, and somebody pays it.
- **D5** VERIFY — **a seller that finds no buyer keeps its paper.** Illiquidity is an unsold position;
  there is no invisible bid. This is a VERIFY because it is what the mechanism must *produce*, and
  enforcing it as a rule is how a residual dealer gets invented.
- **D6** REASON — settlement moves **two legs in the same pass**.
- **D7** REASON — **accrued interest transfers with the paper**.
- **D8** FORBID — no derived measure may set the price.

### E. The holder
- **E1** REASON — a **register**: who owns how many units.
- **E2** VERIFY — held sums to issued.
- **E3** REASON — the holder **marks** at the cleared price; its value is **units times price**.
- **E4** REASON — the change in the mark is **P&L, and it reaches the holder's income**.
  - **E4.a** realised on sale, unrealised while held, and the two are distinguishable.
- **E5** REASON — the holder's willingness to hold has an **economic reservation** built from:
  - **E5.a** its **cost of funds**;
  - **E5.b** its **expected loss** — A4's assessment, times a loss given default;
  - **E5.c** the **capital** the position consumes;
  - **E5.d** VERIFY — the level a book clears at is where the marginal holder's reservation sits, and
    a spread below every reservation means demand is genuinely zero, not that a floor was applied.
- **E6** REASON — a **leveraged** holder funds the position, and **that funding can be withdrawn**.
  - **E6.a** the funding is a named liability to a named lender.
  - **E6.b** withdrawal **forces a sale** — the link from the money market to this one.
- **E7** REASON — the position **consumes regulatory capital** for a holder that has any, bounding
  size. Each holder class faces a different constraint and the model must say which: a bank faces a
  capital charge, a pension fund faces a mandate. *A holder facing neither is unconstrained, and that
  is a statement that must be made deliberately.*
- **E8** REASON — it can be **pledged as collateral**, at a haircut, and is then encumbered.
- **E9** REASON — the holder's **statement** shows the position, its price, its income and its P&L.

### F. The life of the promise
- **F1** REASON — interest **accrues** to the holder of record, continuously.
- **F2** REASON — on the date the issuer **PAYS**, to whoever holds it then, and the cash leaves.
- **F3** REASON — principal repaid: bullet at maturity, or on a schedule for an amortiser.
- **F4** REASON — the issuer may **prepay or call**, paying what B1's regime costs.
- **F5** REASON — **refinancing**: a new issue whose proceeds retire an old one, at the market's price
  on the day — which is how a rate rise reaches a firm that borrowed years ago.
- **F6** REASON — it **matures and ceases to exist**.

### G. When it goes wrong
- **G1** REASON — a missed payment or a breached covenant is an **EVENT, observable by holders** — not
  merely a state of the firm that a holder would have to infer.
- **G2** REASON — an event can **ACCELERATE**: the whole principal becomes due at once.
- **G3** REASON — **default**: the claim becomes a claim on an estate.
- **G4** REASON — the estate is **realised**: assets sold **for what they fetch**, into real markets
  with real bidders, not at a formula discount to book.
- **G5** REASON — proceeds distributed by **seniority** — a waterfall, senior in full first.
  - **G5.a** a junior claim can recover **nothing**, and that is the point of being junior.
- **G6** REASON — the holder **books the loss**: mark minus recovery, on a date.
- **G7** REASON — **RESTRUCTURING** is the alternative to liquidation: terms amended, or debt exchanged
  for equity, and **the holders decide**. Most real corporate defaults are negotiated, and the choice
  between workout and liquidation is a large part of what **sets** recovery. Until it exists, loss
  given default is a property of an asset sale rather than of the credit.
- **G8** REASON — a default is **INFORMATION**: it moves A4's assessments and E5's reservations for
  every other issuer, which is **how contagion travels without a correlation parameter**.

### H. The aggregate
- **H1** REASON — the market has a level: an index built from real prices and real weights.
- **H2** VERIFY — worse assessment trades wider. **Measure, never enforce.**
- **H3** VERIFY — junior trades wider than senior within one issuer. **Measure, never enforce.**
- **H4** REASON — the cash market and its synthetic are **separately cleared**, and the **basis between
  them is a real tradeable difference**.
  - **H4.a** FORBID — neither may be derived from the other. A basis computed from one price is not a
    basis; it is a restatement.

---

## 8. SOVEREIGN CREDIT

Satisfies **the bond contract** in full, answering N11–N13.a its own way.

### A. The borrower
- **A1** REASON — a fiscal authority with revenue and outlays.
  - **A1.a** revenue by base — income, consumption, corporate, payroll — each with a **payer who
    remits it**, never a rate applied to an aggregate.
  - **A1.b** outlays by kind — purchases, transfers, public wages, interest — each with a **payee**.
  - **A1.c** VERIFY — the deficit is the **residual** of A1.a and A1.b and is never itself a target.
- **A2** REASON — **ISSUANCE IS MANAGED TO COVER OUTLAYS.** The treasury has a funding plan: it knows
  its need, it chooses a size and a tenor mix against it, and it **issues ahead of the money leaving**.
  - **A2.a** the need is the deficit **plus redemptions falling due**.
  - **A2.b** the plan is made **before** the outlay, not after the account is empty.
  - **A2.c** the tenor mix is a real choice with a real cost: short is cheap and rolls, long is dear
    and does not.
- **A3** REASON — it has **one account** and every payment leaves it.
  - **A3.a** the account can be **empty**, and that is a real event with a real consequence.
  - **A3.b** FORBID — **there is no central-bank overdraft.** A treasury that has not funded itself
    has *failed to fund itself*; an automatic advance converts a fiscal failure into an accounting
    entry and deletes the reason A2 exists. Everything downstream inherits it: with no funding
    constraint a failed auction costs nothing, a sovereign cannot fail, its paper carries no credit
    risk, and its rating has no consumer.
- **A4** REASON — **it cannot be compelled to pay.** Default is a choice.
  - **A4.a** willingness to pay is a variable, not a constant.
  - **A4.b** in its own money it can always create more; in someone else's it cannot — **and that
    distinction is the whole of sovereign credit risk.**

### B. How sovereign answers the bond contract
- **B1** REASON — **N5**: a **bill** accretes to par and pays no coupon; a **bond** pays a fixed
  coupon. **Two instruments, not one with a flag.**
- **B2** REASON — **N3**: its own money or another's — and per A4.b that difference is its credit risk.
- **B3** REASON — **N2/N8**: fungible within a **benchmark line**.
  - **B3.a** a **re-opening** adds to an existing line rather than creating a new one, so the line and
    the individual issue are not the same object.
- **B4** FORBID — **N13.a**: the ranking exists and **never varies. All of it is pari passu.**
- **B5** FORBID — **N12**: default is a missed payment and **nothing else**. No covenants to breach and
  no acceleration to trigger.
- **B6** REASON — **N11**: **no early-termination regime.** The issuer manages its curve by buyback and
  switch instead.
- **B7** REASON — **N13**: the claim is on **nothing seizable**.

### C. The calendar and the auction
- **C1** REASON — issuance is **announced before it happens**, in a size.
  - **C1.a** the calendar is public ahead of the auction, which is what lets bidders prepare.
  - **C1.b** the size is the issuer's choice out of A2.
- **C2** REASON — **a uniform-price single-round sealed-bid auction.** Every winning bidder pays the
  stop-out. Chosen because it removes the winner's-curse adjustment a multiple-price auction forces
  every bidder to make, and because it needs exactly one cleared number — which is what law 3 wants
  from every book.
- **C3** REASON — **primary dealers with an obligation to bid**, in exchange for privileges.
  - **C3.a** this obligation — **not a central-bank backstop** — is what makes a sovereign auction
    **hard** to fail. It does not make failure impossible, and it must not: a dealer at its position
    limit bids nothing (Dealer Desks D4, D4.a), so an auction fails exactly when the dealers step back,
    which is a state with a cause. Treasury D5 and D5.a stand — **no forced buyer**, and nobody absorbs
    the remainder by construction.
  - **C3.b** the obligation has a cost: the dealer must bid, may bid badly, and wears it.
- **C4** VERIFY — the **tail** (stop-out against average) and the **cover ratio** are the information
  the market reads out of an auction.
- **C5** REASON — weak demand resolves as a **higher yield**, or as the issuer **cutting the size**.
- **C6** REASON — proceeds reach the treasury's account.
- **C7** REASON — paper nobody bid for is **withdrawn, and the withdrawal is an event**: the treasury's
  account is lower than the plan assumed, and that is observable.

### D. The secondary market and the curve
- **D1** REASON — it trades: a **price** per unit.
- **D2** REASON — the yield is **derived** from the price and never sets it.
- **D3** REASON — the curve is a **fit through observed points**.
  - **D3.a** **one owner** of the curve.
  - **D3.b** a tenor's point is a **trade**, or is interpolated **and labelled as interpolated**. Every
    point carries its provenance — traded, interpolated between trades, extrapolated beyond them, or
    never traded — and a consumer that needs a real price is told when it has not got one. **The
    fit's own previous output is not an observation.**
  - **D3.c** the curve's owner publishes it in **one compounding convention**, and every consumer
    compounds the same way. Two conventions for one question is law 4's defect in the discount factor.
- **D4** REASON — it is **the benchmark**: other credit is priced as a spread to it.
- **D5** REASON — it is repo collateral, at the smallest haircut of any asset.
- **D6** VERIFY — **the bid-offer is a consequence, not a prior.** Depth, competition between dealers
  and the size of the float produce it; that sovereign spreads come out tightest is something to
  **measure**.

### E. The holders
- **E1** REASON — a register: who holds how much of **which line**.
  - **E1.a** VERIFY — one walk answers it, over every store that keeps a position.
- **E2** REASON — holder classes hold for **different reasons**, which is what gives an auction two
  sides:
  - **E2.a** banks — the regulatory liquidity buffer (and E5 is why);
  - **E2.b** insurers and pensions — duration against their own liabilities;
  - **E2.c** the central bank — monetary policy;
  - **E2.d** foreign official holders — reserves;
  - **E2.e** funds — relative value;
  - **E2.f** **households and firms, holding it directly.** A saver's choice between a money fund and
    the bills the fund would have bought is a real substitution, and it is the channel by which a
    policy rate reaches a saver who is not a bank's customer.
- **E3** REASON — marked at the cleared price.
- **E4** REASON — pledgeable, at a haircut.
- **E5** REASON — a zero risk weight, which is *why* E2.a holds it at all.

### F. The life
- **F1** REASON — coupon accrues to the holder of record and is paid to whoever holds it on the date.
- **F2** REASON — a bill **accretes**, and the accretion is **observed against its own cleared price**,
  not computed from a curve.
- **F3** REASON — principal repaid at maturity out of A3.
- **F4** REASON — refinancing: the issue that funds the redemption, at whatever the market charges.
- **F5** REASON — **buybacks and switches.** The issuer manages its own curve: buying in an illiquid
  old line and switching holders into a benchmark is a real operation with a real cost, and it is part
  of A2's management.

### G. When it goes wrong
- **G1** VERIFY — in its own money the failure mode is **inflation**, not default.
- **G2** REASON — in a foreign money it can genuinely default.
- **G3** REASON — a default is **selective and negotiated**. **There is no estate** — nothing to seize.
- **G4** REASON — restructuring by **exchange offer, with holdouts**.
- **G5** REASON — the consequence is **exclusion from the market**, not liquidation. And the rating
  gains its first real consumer.

### H. The monetary boundary
- **H1** REASON — the central bank buys sovereign paper as policy, in a size **it** chooses.
- **H2** REASON — the purchase **creates reserves**; the base grows.
- **H3** REASON — the coupon on its holding returns to the treasury as **remittance**.
  - **H3.a** the coupon accrues on its book and is paid on the date like every holder's — **one
    calendar**; its income is what accrued, and the accrued is a receivable on its sheet.
- **H4** REASON — monetary financing against open-market operations is a **policy** boundary, not a
  mechanical one, and A3.b is where this model draws it.
- **H5** VERIFY — debt held by the central bank is economically consolidated away and accounting-wise
  is not, and **both statements must remain true of the books**.

### I. The future on the benchmark
- **I1** REASON — a **deliverable future on the benchmark bond**: a named line is the deliverable, the
  price is per unit of face, and at delivery the contract settles to that bond's own cleared cash
  price.
  - **I1.a** the **carry** ties it to the cash market: the bond financed in repo to delivery earns its
    coupon and pays the financing, and the print against that is the **net basis** — measured, never
    set.
- **I2** REASON — who is on the line: a duration mandate short of duration goes **long** the future
  below carry; a holder over its sovereign target **shorts** the excess above it; a dealer quotes
  **both ways** at carry.
- **I3** REASON — the **basis trade**: long the cash bond, financed in repo, short the future when the
  basis pays for it — **the largest single source of real repo demand in a real market**.
  - **I3.a** FORBID — no basis trader that cannot lose: it is funded, margined and cut on a drawdown.

---

## 9. SHORT-TERM DEBT

### A. The instrument
- **A1** REASON — it satisfies the bond contract, answering these nodes its own way:
  - **A1.a** **N5** — usually **no coupon**: issued at a **discount**, redeemed at par, and the
    discount is the whole return.
  - **A1.b** **N4** — under a year, typically weeks to months.
  - **A1.c** **N13.a** — senior unsecured, ranking with the issuer's other senior debt.
  - **A1.d** **N11** — none. It is too short to be worth an option.
- **A2** REASON — its **price** is what it clears at, and the yield is derived from price and days to
  maturity.
  - **A2.a** on a **stated day-count and quoting convention**, because at this tenor the convention is
    a material part of the number.
- **A3** REASON — there are **types by issuer**: the state, a bank, a firm — **and the type is the
  credit**.

### B. Why an issuer issues it
- **B1** REASON — to fund a **short, known need**: a tax date, a seasonal working-capital swing, a
  bridge to a term issue.
- **B2** REASON — because it is **cheap** when the curve is upward-sloping.
- **B3** REASON — **and it must be rolled.** That is the price of B2, and it is the whole risk.
  - **B3.a** a rollover is a **new issue into a market that must clear**: the issuer is asking the
    market to lend again, and it may not.
  - **B3.b** so a **run** is possible: buyers decline, the issuer must repay maturing paper out of
    cash it does not have, and it must find the money somewhere.
- **B4** REASON — the issuer therefore keeps a **backstop** — a committed bank line, a liquid buffer —
  **and the backstop costs money in every period it is not used**. A committed line with no commitment
  fee on undrawn headroom is a free option the lender did not sell.
- **B5** VERIFY — the **maturity profile** of outstanding paper is a read, and a concentrated profile
  is a foreseeable wall.

### C. Why a buyer buys it
- **C1** REASON — a **cash investor with a horizon**: a money fund, a corporate treasurer, a bank
  liquidity book.
- **C2** REASON — the reasons are **yield against the alternatives** — a deposit, a repo, a central
  bank facility — and **credit** and **liquidity**.
  - **C2.a** which makes short-term debt a real substitute for a deposit, and therefore one of the
    channels a policy rate travels down.
- **C3** REASON — a buyer has a **limit per issuer**, and the limit is why a deteriorating issuer
  loses funding **before** it loses solvency.
- **C4** VERIFY — when the policy rate moves, the bill yield should move with it **because the buyers'
  alternative moved** — not because a rule ties them.

### D. Trading and pricing
- **D1** REASON — it **trades** after issue, at a cleared price, so a holder can get out early.
- **D2** REASON — its price responds to **the level of short rates and the issuer's credit**, and at
  this tenor the first dominates until the second is in doubt, at which point it inverts.
- **D3** REASON — it is **collateral**, with a haircut, which is a large part of why anyone holds it.
- **D4** VERIFY — a spread over the equivalent-tenor bill is a **derived read of two cleared prices**,
  never a stored number.

### E. What must not happen
- **E1** FORBID — **no automatic roll.** Paper that always rolls at a written rate is not debt; it is a
  permanent liability with a coupon, and it removes the only risk the instrument has.
- **E2** FORBID — **no price without a market.** A discount computed from a curve nobody traded is law
  3's defect at the short end.
- **E3** FORBID — **no negative outstanding**, and no maturity that passes without cash moving.

---

## 10. EQUITY

### A. What a share is
- **A1** REASON — a **residual claim**: what is left after every other claim is paid.
  - **A1.a** it ranks **below all debt**.
  - **A1.b** its value can be **zero and not negative**: limited liability is a real property.
- **A2** REASON — counted in **SHARES**, a unit that is not money.
  - **A2.a** a **share count** that changes only by a named event.
- **A3** REASON — a **CURRENCY** it is quoted in — the issuer's own money.
- **A4** REASON — it is **PERPETUAL**: no maturity, no redemption. That is why equity is a different
  instrument rather than a long bond.
- **A5** REASON — **CONTROL** rides with it: a vote per share — and a cell holding shares casts
  `weight ×` its member's votes (XI-15), so a represented holder is not disenfranchised by its
  representation.
  - **A5.a** which makes a majority a thing that can be **bought**.
  - **A5.b** VERIFY — control has a value distinct from the cash flows, and a takeover pays for it.
- **A6** REASON — an **IDENTITY a market would use**.

### B. The price
- **B1** REASON — holders and buyers **post schedules**; who trades is the outcome.
- **B2** REASON — **a PRICE clears** per share, per period.
- **B3** FORBID — **the price is never derived from an earnings multiple, a book value, a discounted
  cash flow or a target.** Those are opinions *held by participants* that enter their schedules; a
  price computed from one is the opinion restated, not a market.
- **B4** REASON — **market capitalisation is a READ**: shares times price.
  - **B4.a** FORBID — nothing may compare market capitalisation against shares times price and call it
    a check. That is a tautology and cannot fail.
- **B5** REASON — a **dealer** intermediates out of inventory and capital, and earns the spread it
  quotes.
- **B6** VERIFY — a seller with no buyer keeps its shares; there is no invisible bid.

### C. The holder
- **C1** REASON — a **register**: who holds how many shares.
  - **C1.a** VERIFY — held equals shares outstanding, always.
  - **C1.b** the **free float** is what is genuinely tradeable — insiders and strategic holders are
    not.
- **C2** REASON — holder classes hold for **different reasons**:
  - **C2.a** **households**, directly;
  - **C2.b** **institutions** with mandates — and a mandate is a constraint, not a preference;
  - **C2.c** **index funds**, which do not price at all: they hold weight, whatever it costs;
  - **C2.d** **the issuer itself**, via treasury shares;
  - **C2.e** **insiders and founders**, whose holding is not for sale.
- **C3** REASON — marked at the cleared price; value is shares times price.
- **C4** REASON — the change in the mark is **P&L reaching the holder's income**, for every holder
  class without exception.
- **C5** REASON — a leveraged holder **funds** the position and can be **forced to sell**.
  - **C5.a** margin, and a call on it.
- **C6** REASON — it can be **lent** and **pledged**, at a haircut.
- **C7** REASON — a **short** position is possible, is a borrow, and has a real cost and a real squeeze
  risk.

### D. What the firm does with it
- **D1** REASON — **ISSUANCE**: the firm sells new shares for cash.
  - **D1.a** it **dilutes** existing holders — the count rises and each claim shrinks.
  - **D1.b** it is a **decision with a reason**: a funding need it prefers to meet with equity.
  - **D1.c** it is **priced by the market**, at a discount the market demands, and **can fail**.
- **D2** REASON — **BUYBACK**: the firm buys its own shares for cash.
  - **D2.a** the count **falls**; each remaining claim grows.
  - **D2.b** the cash is **gone** — a buyback is a distribution, not an investment.
  - **D2.c** it competes with D3 and with real investment, and the choice has a reason.
- **D3** REASON — **DIVIDEND**: cash paid per share to whoever holds it on a date.
  - **D3.a** it leaves the firm and arrives at named holders.
  - **D3.b** it is a **decision**, and cutting it is an event others react to.
- **D4** REASON — a **split** changes the count and not the value, and must not change anything else.

### E. Corporate events
- **E1** REASON — **M&A**: shares bought for cash, for stock, or for both.
- **E2** REASON — a **spin-off**: a new share line, and a claim divided — including a pro-rata division
  of the firm's productive assets and of anything it has under construction.
- **E3** REASON — **DELISTING / TAKE-PRIVATE**: the line stops trading and the register is bought out —
  **through a tender that holders can refuse**, because A5's vote is what a take-private must obtain.
  A cell accepts or refuses as one holder, for its whole weight; a tender that would split the cell's
  answer splits the cell (XI-15).
- **E4** REASON — **INSOLVENCY**: equity is **wiped before any creditor takes a loss**, and the
  register goes to zero rather than to a recovery.

### F. What the holder is entitled to
- **F1** REASON — the **dividend** when declared.
- **F2** REASON — the **residual on wind-up**, after every other claim.
- **F3** REASON — a **vote**.
- **F4** FORBID — **no entitlement to earnings that were not distributed.** Retained earnings raise the
  claim's value through B2 and reach the holder only on sale or on F1/F2 — never as income credited to
  a holder who did not receive cash.

### G. The aggregate
- **G1** REASON — an **index** built from real prices and real free-float weights.
- **G2** VERIFY — the index is a read of its constituents and cannot move independently of them.
- **G3** VERIFY — a derived statistic is computed from the cleared price and never used to set it.

---

## 11. THE MONEY MARKET

### A. The need
- **A1** REASON — a bank's reserve position moves because **its customers paid other banks'
  customers**. Nobody decided it; it is the residue of everyone else's period.
  - **A1.a** in aggregate the system's reserves are unchanged — they are **redistributed**.
  - **A1.b** therefore one bank's deficit **is** another's surplus, by construction, and the market has
    two sides without anybody being assigned one.
- **A2** REASON — a bank holds a position for its own reasons.
  - **A2.a** **the buffer is a PREFERENCE derived from its own liabilities' liquidity, not a stated
    ratio.** A bank whose funding is overnight household money needs more than one funded by term
    wholesale. A regulatory floor may sit *under* the preference; it is not the preference.
  - **A2.b** VERIFY — missing the buffer has a cost the bank can feel, or the buffer is decoration.
- **A3** REASON — **the need is knowable only AFTER the period's flows.**
  - **A3.a** therefore the market must clear **after** them. A session held before the flows is a
    session that cannot see the thing it exists to fund. Sizing a shortfall against an opening balance
    and plugging the difference at the close is the same defect twice.

### B. The market
- **B1** REASON — **every bank posts a schedule out of its own position and its own cost of funds. Who
  ends up lending and who ends up borrowing is the OUTCOME.** Writing *"surplus banks lend, deficit
  banks borrow"* as a rule licenses moving cash from a computed surplus to a computed deficit without
  anybody quoting a rate.
- **B2** REASON — **unsecured lending prices the borrower's name.** The lender has a view on getting it
  back, and that view is in its schedule.
  - **B2.a** a name the market doubts pays more, **or finds no bid at all** — and refusal is a real
    outcome of a real schedule, not a special case.
  - **B2.b** VERIFY — the spread between the strongest and weakest name is a measure of stress.
- **B3** REASON — **secured lending prices the collateral, not only the name.**
  - **B3.a** eligibility is defined per asset, and something is ineligible.
  - **B3.b** haircuts by asset and tenor — and by the **issuer's own credit**, not one haircut per
    instrument type. A haircut that is identical for the best and worst credit of the same type is the
    one leg of the downgrade loop that is wholly absent.
  - **B3.c** pledged collateral is **encumbered** — it cannot be pledged twice, and running out of it
    is how a solvent bank stops being able to borrow.
- **B4** REASON — a rate **clears** from those schedules meeting each other.
- **B5** REASON — non-bank cash is in the same market: money funds, firms, institutions.
  - **B5.a** their alternative is the central bank's floor or bills directly.
- **B6** REASON — tenor: **overnight and term**, each with its own book.
  - **B6.a** VERIFY — the term-to-overnight spread is information about expected stress, not a
    parameter.
- **B7** REASON — **the market can fail to clear for a name.** That is what a funding squeeze *is*, and
  it must be representable.

### C. The corridor
- **C1** REASON — a **floor**: the central bank pays on reserves, or takes cash at a window.
  - **C1.a** cash parked there **leaves the banking system** — a real consequence, not a bookkeeping
    move.
- **C2** REASON — a **ceiling**: a standing facility that lends.
- **C3** VERIFY — the market rate sits inside the corridor; its width is a policy choice.
- **C4** REASON — the facility is **collateralised** and **priced above the market**.
  - **C4.a** so a bank prefers the market, and **drawing the facility is information**.
  - **C4.b** a bank out of eligible collateral **cannot draw** — the constraint has to bite.
- **C5** FORBID — **there is no uncollateralised, unpriced, unlimited central-bank credit.** A facility
  with none of the classical conditions is not a lender of last resort; it is a subsidy that makes B7
  and D unreachable.

### D. When a name cannot fund
- **D1** REASON — it **shrinks**: it sells assets, at whatever they fetch, and the sale is a real order
  in a real book. It also **stops originating**: a bank whose book loses money writes no new business
  and lets the existing book amortise until the margin is back.
- **D2** REASON — it **bids up for deposits**, and depositors respond to the rate.
- **D3** REASON — it draws the facility, at the penalty, against collateral.
- **D4** REASON — it **fails** — and **failure for liquidity is a distinct event from failure for
  solvency**, with a distinct trigger. A bank whose account at the central bank is below zero after
  the market and the window have both run cannot pay, whatever its capital ratio says.
- **D5** REASON — a **run**: depositors withdraw because they observe weakness.
  - **D5.a** what they observe must be **observable** — a published ratio, a facility draw, a rate
    paid, a run of short closes.
  - **D5.b** VERIFY — a run is **self-reinforcing**: the deposits leave with the reserves behind them,
    so the bank is shorter at the next close, and the model should be able to show the loop.
- **D6** REASON — the lender of last resort lends **freely, against good collateral, at a penalty, to
  the solvent.** All four. Drop one and C5 is violated. In particular the window does not lend to a
  bank that is insolvent — that bank goes to resolution.

### E. Transmission
- **E1** REASON — the policy rate reaches the economy **through this market** and not by assertion.
- **E2** VERIFY — a squeeze here raises funding costs elsewhere; it is a channel, not a scalar.
- **E3** REASON — interbank exposure is a **contagion path**: a failure lands on its lenders by name.

---

## 12. SPOT FOREIGN EXCHANGE

### A. What is traded
- **A1** REASON — an **exchange of two amounts in two currencies**, both legs settling.
- **A2** REASON — the price is the **rate**, quoted one way with its inverse implied.
- **A3** REASON — the pair set is **all pairs among the currencies that exist**, and the rates must be
  mutually consistent — so a cross is either traded or derived, and if both, they must agree or
  somebody is arbitraging.

### B. Who trades and why
- **B1** REASON — **a party that owes a currency it does not have** — an importer, a foreign-currency
  borrower, an investor settling a foreign purchase.
- **B2** REASON — **a party with a currency it does not want**: an exporter, a coupon received abroad.
- **B3** REASON — **an investor changing its portfolio's currency mix**, for yield or for risk.
- **B4** REASON — **a hedger** closing a currency exposure it took on for another reason.
- **B5** REASON — **a dealer**, whose reason is spread and inventory.
  - **B5.a** it quotes because it expects the flow to be two-way; it is **not** obliged to take
    whatever arrives.
- **B6** REASON — the **central bank may participate**, for a stated policy reason, as a participant
  with a size and a limit — never as the residual.

### C. The mechanism
- **C1** REASON — participants post **schedules in rate space**.
- **C2** REASON — the rate clears where the two sides meet, **per pair and consistently across pairs**.
  - **C2.a** the cross-consistency is a **constraint on the clearing**, not a correction applied after.
- **C3** REASON — **the bid–offer is a consequence** of what dealers posted — a client crosses it, a
  dealer earns it.
- **C4** REASON — **imbalance moves the rate**: persistent demand for a currency at the old rate means
  the old rate was wrong.
- **C5** REASON — one rate is **in force for the period** and both valuation and settlement use it.
- **C6** VERIFY — with flows netting to zero the rate should not drift; with a one-way flow it should
  move.

### D. The dealer's position
- **D1** REASON — a dealer that fills a client is **left with the other side**: a real open position in
  a real currency.
- **D2** REASON — it can **square** it — against another client, another dealer, or the market — and
  squaring is a trade with a counterparty, not a disappearance.
- **D3** REASON — what it does not square, it **carries**, and the carried position revalues; that is
  the risk it is paid the spread for.
- **D4** REASON — it has a **limit** on what it will carry, and when the limit binds it widens or stops
  quoting rather than absorbing more.
- **D5** VERIFY — dealer positions and client positions sum to zero in every currency, because every
  trade has two sides.

### E. What must not happen
- **E1** FORBID — **no conversion without a counterparty.** A party cannot turn one money into another
  by itself; somebody took the other side, and that somebody now holds the first.
- **E2** FORBID — **no rate from a formula.** Not purchasing-power parity, not a rate differential
  applied to a level, not a written path.
- **E3** FORBID — **no free arbitrage left standing.** If the cross and the direct route disagree,
  either a participant takes it and the rates converge, or the inconsistency is a defect. It is never
  a permanent feature — but the participants who close it are **bounded**, so a persistent gap is a
  finding about their capacity, and it must be **measurable**.
- **E4** VERIFY — a party's currency position after the market is exactly what it held plus what it
  traded, and **no leg landed converted**.

### F. One convention for what a payment settles in
- **F1** REASON — a purchase settles in **one** stated money, and the convention is owned in one place.
  The mechanism-consistent answer is the **seller's**: a factory-gate price is quoted in the seller's
  money, and a party short of that money **buys it**.
  - **F1.a** FORBID — a conversion inside a trade with **no counterparty on the other side of it**.
    Converting a price to the buyer's money inside the auction means the buyer is never short and no
    order is ever placed — the currency demand the trade should have created disappears.
  - **F1.b** FORBID — a convention that depends on **who the buyer is**. One purchase, one rule.

---

## 13. FUND SHARES

### A. What a fund is
- **A1** REASON — a **named party** with an account and a register of holdings.
- **A2** REASON — its **liability is its shares**, held by named holders, **counted in shares**.
- **A3** REASON — its **equity is zero by construction**: assets minus liabilities is zero, because the
  holders own the assets. **A fund with equity has mislaid somebody's money.**
- **A4** REASON — it has a **mandate**: what it may hold, and the mandate is a real constraint on what
  it buys, not a label.
  - A3 and A4 together are why a fund is a **transmission channel**: a flow into the fund becomes a
    purchase of what the mandate allows.

### B. Net asset value
- **B1** REASON — **NAV is (assets at market minus liabilities) divided by shares outstanding**, a
  read, every time, never a stored series.
- **B2** REASON — the assets are marked at **cleared prices**.
  - **B2.a** so a stale price makes a stale NAV, and somebody transacts on it — that is a real transfer
    between holders, not a rounding.
- **B3** REASON — **fees accrue** and are paid to the manager, and they reduce NAV.
- **B4** VERIFY — the sum of holders' share value equals the fund's assets minus its liabilities,
  exactly.

### C. Creation and redemption
- **C1** REASON — a **subscription** gives the fund cash and the holder new shares at NAV.
  - **C1.a** and the fund must then **buy something** with the cash, per its mandate.
- **C2** REASON — a **redemption** takes shares back and pays the holder cash at NAV.
  - **C2.a** and the fund must **find the cash**: from its buffer, **or by selling**.
  - **C2.b** selling is a trade into a market that must clear, at whatever price it clears — **this is
    the forced-seller channel, and it is the point.** A redemption rationed by the fund's cash, with
    the unfilled part dropped, deletes the entire system.
- **C3** REASON — the shares outstanding **change**, so a fund is not fixed-size.
- **C4** REASON — there is a **timing mismatch**: the holder is paid at today's NAV, the sales happen
  at tomorrow's prices, and the difference falls on the remaining holders.
  - **C4.a** which is why a redemption is a real cost to those who stay, and why runs are a thing.
- **C5** VERIFY — shares created minus redeemed equals shares outstanding, and cash in and out matches.

### D. The money fund specifically
- **D1** REASON — a mandate of **short, high-quality paper**.
- **D2** REASON — it is a **substitute for a deposit**, and that is its whole economic role: a saver
  chooses between a bank deposit, a money fund and bills directly.
  - **D2.a** so its yield competes with the deposit rate, and the competition is a real constraint on
    what banks pay.
- **D3** REASON — it is a **buyer in the short-term market**, and its size determines how much paper
  can be placed.
- **D4** FORBID — **no guaranteed constant NAV.** It is measured in **shares**, not in currency units;
  income is distributed as cash or as shares at NAV, not as free shares; losses fall on the NAV. If
  the assets fall, the NAV falls. **A fund that cannot break the buck is a fund with a hidden
  guarantor, and the guarantor is nobody** — and it makes the deposit substitution in D2 riskless in
  one direction.
- **D5** VERIFY — flows into money funds should rise when their yield beats deposits, **as a
  consequence of D2**, never as an imposed allocation.

### E. The exchange-traded fund specifically
- **E1** REASON — its shares **trade** on a market, at a price that clears.
- **E2** REASON — so it has **two values**: the traded price and the NAV, and they are different
  numbers.
- **E3** REASON — the gap is **arbitrageable**: somebody can create or redeem against the basket and
  pocket the difference.
  - **E3.a** which is a **reason for a participant**, not a rule tying the two — the gap closes because
    somebody trades, and it can persist when they will not.
- **E4** VERIFY — the premium or discount is a read of two prices; a persistently large one is a
  finding about liquidity, never a number to clamp.

### F. What a fund is not
- **F1** FORBID — **a fund does not create its assets.** Every holding is bought from a named seller at
  a cleared price.
- **F2** FORBID — **no leverage without a lender.** A fund that holds more than it raised has borrowed
  from somebody named.
- **F3** REASON — the **manager is a separate party** that earns the fee; the fee is its income and the
  fund's cost.

### G. The redeemable claim, generally
- **G1** REASON — every open-ended vehicle in this world has a **claim its investor can redeem**: a
  share count, a redemption request, a sale in the **same period's** books, and the cost of a late sale
  landing on the holders who stayed.
  - **G1.a** an in-kind redemption is correct for an exchange-traded fund and means that vehicle is
    **not** a forced seller; a world in which the largest fund complex redeems only in kind has no
    fund-driven forced selling at all, and some other vehicle must carry it.
  - **G1.b** an investor holding a scalar with no share count cannot ask for its money back, and a
    vehicle like that is outside this system whatever it is called.

---

## 14. SECURITIES LENDING

### A. The transaction
- **A1** REASON — the **lender delivers the security** and the borrower delivers **collateral** — cash
  or other securities — and both legs move in the register and the accounts.
- **A2** REASON — **legal title passes.** The borrower can sell what it borrowed; that is the entire
  point.
- **A3** REASON — **the economics stay with the lender**: it receives a **manufactured payment** equal
  to any coupon or dividend, so its cash flows are unchanged. The issuer pays the registered holder —
  the borrower — and the borrower **passes it on**. Without the manufactured payment the defining
  property of a stock loan (title moves, economics do not) is inverted, and the lender pays a fee to
  lose its income.
- **A4** REASON — it is **recallable and it terminates**: the security comes back, the collateral goes
  back.
- **A5** REASON — the borrower pays a **fee**, and the fee is a price.
  - **A5.a** it **clears**: scarce paper is expensive to borrow, abundant paper is cheap.
  - **A5.b** when the collateral is cash, the price is expressed as a **rebate** on that cash instead,
    and the two forms are the same number seen from two sides.

### B. Why each side is there
- **B1** REASON — the **borrower needs the security**: to deliver a short, to cover a fail, to meet a
  delivery obligation on a derivative.
- **B2** REASON — the **lender has it sitting there** and wants the fee: a fund, an insurer, a pension.
  - **B2.a** and it lends only within a mandate, against acceptable collateral, with a limit.
- **B3** REASON — an **agent** may sit in the middle and take part of the fee.
- **B4** VERIFY — the size of the lendable pool is a read of who actually holds the security and is
  willing; **it caps how large a short can get**, which is a real constraint.

### C. Collateral and margin
- **C1** REASON — the collateral is **worth more than the loan** — a **haircut** — because the lender
  must be able to sell it and be whole. Collateral exactly equal to the loan, re-marked to the same
  price, means the gap between two marks is covered by nothing.
- **C2** REASON — both sides are **marked every period**: when the borrowed security rises, the
  borrower posts more collateral.
  - **C2.a** the margin flow is real money moving between two named parties.
- **C3** REASON — **cash collateral is reinvested** by the lender, and that reinvestment is a position
  with its own risk — **this is where a lending programme actually loses money.** Posted cash
  collateral is spendable capacity to whoever holds it, and it is encumbered to whoever posted it.
- **C4** FORBID — **no collateral that is not held.** Posted collateral leaves the poster's free
  balance; it cannot be counted as available by both sides.
- **C5** REASON — **re-pledging**, where permitted, means the same security backs a **chain** of
  obligations — and the chain must be **traceable**, because it is how a single default reaches parties
  that never traded with the defaulter.

### D. Failure
- **D1** REASON — the borrower can **fail to return**, and then the lender keeps the collateral and
  buys the security back in the market, at whatever it costs. **A failed return terminates.**
- **D2** REASON — a **squeeze** is possible: shorts must buy, the lendable pool is small, the fee and
  the price both rise.
  - **D2.a** VERIFY — this is a consequence of B4 and C, to be measured, never a scripted event.
- **D3** REASON — a **recall** forces the borrower to find the security elsewhere or close its short.

### E. What must not happen
- **E1** FORBID — **no short without a borrow.** A negative position that nobody lent is an invented
  security.
- **E2** FORBID — **no double-counting the loaned security.** The lender's economic exposure and the
  borrower's legal title are two reads of **one** security, and holdings must still sum to issued.
- **E3** FORBID — **no free borrow.** A fee of zero is a cleared price only if somebody posted it.

---

## 15. PRIME BROKERAGE

### A. The relationship
- **A1** REASON — a **named bank and a named client**, with a contract that can be ended.
- **A2** REASON — the broker **holds the client's assets** and knows the whole position — that
  knowledge is what lets it lend against them.
- **A3** REASON — the client can have **more than one** broker, and then **no broker sees the whole
  position**, which is a real and material blind spot.
- **A4** REASON — the broker earns from **financing spread, stock-borrow fees and commissions**, and
  that income is a reason for it to take the risk.

### B. Financing
- **B1** REASON — the client buys more than its cash allows; the **broker lends the difference** against
  the assets as collateral.
  - **B1.a** so the client's leverage is a **loan from a named lender**, not a property of the client.
- **B2** REASON — the loan has a **rate**, above the broker's own cost of funds, and the client pays it
  in cash.
- **B3** REASON — the broker's balance sheet **grows** by the loan, and the loan consumes its capital
  and its liquidity.
- **B4** REASON — the **short side is financed too**: proceeds of a short are held, and stock is
  borrowed.
- **B5** VERIFY — the client's leverage is a read of borrowed against equity, and it must equal what
  the broker has lent.

### C. Margin — the core
- **C1** REASON — the broker sets a **margin requirement** on the whole portfolio, from its own view of
  the risk.
  - **C1.a** it accounts for **offsetting positions**, so a hedged book requires less than the sum of
    its legs.
  - **C1.b** it is a **decision by the broker**, not a formula the client can rely on.
- **C2** REASON — the requirement is **remeasured as prices move**, and a shortfall is a **margin
  call**: real money, from the client's account, now.
- **C3** REASON — the client must **meet it or be liquidated**.
  - **C3.a** and to meet it, it may have to **sell**, into a market that must clear.
  - **C3.b** FORBID — **the available line is never floored at zero.** A client drawn past its line is
    over the line, and the shortfall is what forces the sale. Flooring it makes the whole path
    unreachable — and lending the shortfall straight back at a penalty, from the same broker, makes it
    unreachable twice.
- **C4** REASON — the broker can **raise the requirement** when it likes what it sees less: worse
  markets, worse client, worse own position.
  - **C4.a** VERIFY — **raising margin into a falling market amplifies the fall.** That is a
    consequence to be measured, and it is the mechanism behind most of what looks like contagion. A
    margin rate that is a stated constant cannot rise when it matters, which deletes exactly that
    procyclicality.
- **C5** FORBID — **no margin that is only a number.** An unmet call must have a consequence, and a met
  call must move cash.

### D. Default
- **D1** REASON — a client can **fail to meet a call**, and then the broker **closes the positions**,
  selling collateral at market prices.
- **D2** REASON — the proceeds may be **less than the loan**, and the shortfall is the broker's loss,
  hitting its capital.
- **D3** REASON — the liquidation is a **real sale into a real market**, so it moves prices, which can
  margin-call other clients.
- **D4** VERIFY — the loss chain from one fund to one bank to other funds must be traceable party by
  party; a loss that stops at the fund is a broker that was never really lending.

### E. Concentration
- **E1** REASON — the broker has an **exposure per client**, and it should know it.
- **E2** REASON — the client's positions may be **concentrated**, so the collateral is worth less in
  liquidation than it is marked at.
- **E3** REASON — A3's multi-broker case means **each broker underestimates**: the true leverage is the
  sum, and no single lender sees it.
- **E4** FORBID — **no unlimited exposure.** A broker with no limit per client is a synthetic
  counterparty.

---

## 16. THE DERIVATIVE LAYER

The infrastructure every derivative class runs on. The instrument itself is **the derivative
contract** in Part IV.

### A. Why the layer exists
- **A1** REASON — a derivative is a **long-lived bilateral obligation**, so the two sides remain exposed
  to each other for years.
- **A2** REASON — that exposure has to be **managed**, and how it is managed is this system.
- **A3** REASON — the same obligation appears **twice**, as an asset and a liability, and the two must
  be the same number read from two sides.
- **A4** VERIFY — marks sum to zero across all parties, per contract and in aggregate.

### B. How a trade becomes a position
- **B1** REASON — two parties **agree terms at a cleared price**.
- **B2** REASON — the position is **recorded on both books**, and it is one contract, not two.
- **B3** REASON — it can be **closed** by an offsetting trade, by an early termination, or by running
  to expiry.
  - **B3.a** an offsetting trade with a **different counterparty does not remove the first**: the party
    now has two contracts and two counterparty exposures, and the market risk is flat while the credit
    risk has doubled. **Collapsing them hides the thing that actually breaks.**
- **B4** REASON — **novation** transfers a position to a new counterparty, with the old one's consent,
  and it is a real change of who faces whom.

### C. Bilateral against cleared
- **C1** REASON — **bilateral**: the two parties face each other, exchange collateral under an
  agreement, and net across the contracts they have **with each other**.
  - **C1.a** netting is **per counterparty pair**, and it is why gross notional and net exposure are
    orders of magnitude apart.
- **C2** REASON — **cleared**: a **central counterparty** steps in, becoming buyer to the seller and
  seller to the buyer, and then each side faces the house. **No member pays another** — every leg
  (periodic, mark, event, close-out) is written as two, member to house and house to member, and the
  house is flat on every leg by construction.
  - **C2.a** it does not remove the risk; it **concentrates** it in a named party whose own solvency
    now matters to everyone.
- **C3** REASON — the house is a **real entity with a balance sheet**: the margin it holds, a **default
  fund** its members paid into, and **its own capital**. Its own capital is the residual — what it has
  retained beyond margin and fund.
  - **C3.a** the margin a member posts is **its asset at the house**, on its own balance sheet, in its
    leverage denominator and in its resolution plan. Posting margin is an **asset swap, not an
    expense**.
  - **C3.b** the default fund is sized **cover-one**: enough to absorb the largest member's book,
    given that closing a defaulted book takes several sessions and the price move over that horizon
    scales with the square root of its length. Contributions are pro rata to each member's margin,
    trued up every period; a member that leaves is refunded.
- **C4** REASON — it has a **stated default waterfall, in order**: the defaulter's margin, the
  defaulter's fund contribution, the house's own capital, the surviving members' contributions written
  down pro rata.
  - **C4.a** which means a member's loss can come from **another member's default** — and that is the
    mutualisation channel. A surviving bank books the write-down against its equity: a real loss on a
    real sheet.
  - **C4.b** the survivors of a defaulted contract are **paid in full by the house** and get their
    margin back; the defaulter's leg is not written; what the defaulter owed the house **net across
    all its contracts at that house** is the loss the waterfall absorbs.
  - **C4.c** what the defaulter's own money did not cover is the house's **unsecured claim on the
    estate**, ranking with other unsecured claims. **A close-out is not paid ahead of every ranked
    claim.**
  - **C4.d** every round of a waterfall is recorded and reportable: who defaulted, the loss, what each
    line paid, what was left unfunded.
- **C5** FORBID — **the house is not a guarantor of last resort.** Its resources are finite and
  enumerable, and **running past the end of the waterfall is a real event with real consequences**, not
  an impossibility.

### D. Margin
- **D1** REASON — **initial margin**: posted up front against a potential future move, **sized from the
  risk of the position** — the underlying's own measured move, scaled by the notional and the remaining
  life. It is not a stated rate per class.
- **D2** REASON — **variation margin**: the change in the mark, paid in cash, every period.
  - **D2.a** it is **real money leaving one account and arriving in another**, in a stated currency,
    and it is the largest recurring flow this layer produces.
  - **D2.b** VERIFY — variation margin paid equals variation margin received, every period, exactly.
  - **D2.c** FORBID — **a variation-margin payment has a cash test.** A party that cannot pay it is in
    the state Money and Settlement E1 describes; it does not silently become a borrowing.
- **D3** REASON — margin is **held, not consumed**: the poster still owns it and gets it back, but it is
  no longer free.
- **D4** REASON — a **margin call must be met or the position is closed out**.
  - **D4.a** and meeting it may force a sale — the same liquidity channel as a redemption.
- **D5** REASON — margin **rises when volatility rises**, which is exactly when parties can least afford
  it. **That is procyclical by construction and it is a consequence to be measured.**

### E. Capacity — why a market can be refused
- **E1** REASON — a clearing member may carry **no more margin at the houses than its own liquid cash
  could re-margin over the close-out horizon.** That is a real limit, read once per period from the
  member's own liquid assets net of what it has already committed.
- **E2** REASON — a contract is **cut to the smaller of its two members' admitted shares**, size, units
  and margin together, or **refused** — and the cut happens **at the strike**, in the same pass as the
  contract and its margin leg.
- **E3** REASON — a party's second hedge in a period is sized against what its first will post. Capacity
  is drawn down as it is consumed.
- **E4** VERIFY — what the markets struck **beyond** what their members could margin is a measurable
  quantity. Non-zero means a market sized its demand to the wrong constraint; a member cut every period
  is one living at its limit. **Measure; do not raise the limit.**

### F. Default
- **F1** REASON — a party can **fail with open positions**.
- **F2** REASON — the positions are **closed out at a stated value** and the in-the-money side has a
  **claim on the estate**.
- **F3** REASON — the loss is **the mark minus the collateral held**, and it lands on named survivors:
  the counterparty bilaterally, the waterfall if cleared.
- **F4** VERIFY — the loss chain is traceable party by party; a default whose losses vanish is a layer
  that was never really bilateral.

### G. What must not happen
- **G1** FORBID — **no position without a counterparty.**
- **G2** FORBID — **no exposure without margin, or a stated reason there is none.**
- **G3** FORBID — **no netting across counterparties.** Exposure to one does not offset exposure to
  another, and treating it as if it does is how a book looks flat until one of them fails.
- **G4** FORBID — **no derivative that settles against a price this world does not clear.**

---

## 17. CREDIT DEFAULT SWAPS

### A. The contract
- **A1** REASON — satisfies the derivative contract, answering these its own way:
  - **A1.a** **underlying** — a **named reference entity** and **its default event**, not a price.
  - **A1.b** **payoff** — on the event, the protection seller pays **par minus recovery** on the
    notional; otherwise nothing.
  - **A1.c** **price** — the **running spread**, in basis points per annum on the notional, cleared.
  - **A1.d** **term** — a stated tenor, **and a curve of them**: several tenors, which is a **term
    structure of credit** and not one number. A single tenor means the model has no term structure of
    credit anywhere.
- **A2** REASON — the **premium leg is a real periodic payment**, in cash, in the contract's currency,
  and it **stops on the event**.
- **A3** REASON — the **protection leg is contingent**, and its value is the probability-weighted loss —
  which is a **read from the cleared spread**, never the input to it.
- **A4** REASON — the **reference entity must exist in this world** and be capable of defaulting.
  - **A4.a** FORBID — no protection on an entity nobody can observe failing.
- **A5** REASON — **the index**: a fixed basket of names traded as one line, which is how broad credit
  risk is actually bought and sold.
  - **A5.a** the basket is a **series**: names fixed at the roll, and a name's event settles its
    **weight** once for every contract on the line, the line running on with the survivors.
  - **A5.b** the index clears on its own book, and the **index-against-single-name basis** is a second
    measured relationship, never set.

### B. Why each side is there
- **B1** REASON — the **buyer of protection** has a reason: it holds the issuer's debt and wants the
  risk off, it lends to the issuer and cannot sell the loan, **or it thinks the credit will
  deteriorate**.
  - **B1.a** so a bank can hedge a loan it cannot sell, which is the contract's original economic
    purpose.
- **B2** REASON — the **seller of protection** has a reason: it wants credit exposure without funding a
  bond, **or it thinks the spread is too wide for the risk**.
  - **B2.a** it is **short a jump**: small regular income, large sudden loss — which is why its capital
    and margin matter more than its mark.
- **B3** REASON — **naked positions are possible on both sides**, and they are how the market gets
  liquid — but a naked seller is an unfunded credit exposure and must be capitalised as one.
- **B4** REASON — a **dealer** intermediates, and its book is rarely flat.
- **B5** FORBID — **not every participant is a hedger.** A book whose only buyer is a bank above an
  exposure limit and whose only seller is closing a regulatory gap produces a spread that is a function
  of regulatory gaps and never of a view; a period in which neither gap binds does not open the book at
  all, and the spread cannot move because somebody thinks the credit is mispriced. **A speculative
  participant with a view is required on both sides.**

### C. Price and what is derived from it
- **C1** REASON — the **spread clears** from the two sides' schedules.
- **C2** REASON — the **implied default probability and expected recovery are derived from the spread**
  and the term structure — never the other way round. **A default probability computed from the firm's
  accounts and fed to every seller's reservation means the market cannot disagree with the accounting
  model**, which is law 3 inverted in the one instrument whose entire purpose is to hold a second
  opinion about a credit.
- **C3** VERIFY — the swap spread and the cash bond's spread over the risk-free curve should be close,
  because both are compensation for the same credit.
  - **C3.a** the difference is the **basis**, and it is a **consequence** — of funding cost,
    deliverability, and who can trade which. Measured, never set, **at every tenor both books print**.
  - **C3.b** a persistently large basis is a finding about one of the two markets, and it must be
    visible.
- **C4** VERIFY — worse credit should trade wider, as a consequence of what participants post.

### D. The event
- **D1** REASON — a **stated definition of the credit event**, observable by both sides.
- **D2** REASON — a **recovery, determined by what the defaulted obligations are actually worth** — an
  auction or a realised workout, not an assumption.
  - **D2.a** FORBID — **no fixed recovery rate.** A constant recovery makes the payoff a constant and
    turns a credit derivative into an interest-rate instrument.
  - **D2.b** while the reference's workout is open, a triggered contract **pays no premium**, marks at
    its **expected** payoff, and **holds past its own maturity** until the workout closes; the
    settlement is then the true-up to what the obligations actually recovered.
- **D3** REASON — the payment on the event is **real money from the seller to the buyer**, and it can be
  large enough to fail the seller.
- **D4** REASON — the contract **terminates** on the event.
- **D5** VERIFY — protection paid equals protection received, and the net effect across the whole world
  of a default is a **transfer**, never a change in total loss.

### E. The systemic part
- **E1** REASON — it **moves credit risk to where it is not observed**: the bank looks hedged, and the
  risk sits with whoever sold it.
- **E2** REASON — that seller may be **correlated with the reference entity** — wrong-way risk — and
  then the protection is worth least exactly when it is needed. **A reservation that carries no term
  for the counterparty makes wrong-way risk cost nobody anything.**
- **E3** REASON — the **net notional per reference entity** is a real number and a real concentration,
  knowable only by adding up the contracts.
- **E4** FORBID — **no protection that pays without a payer.** The seller's ability to pay is part of the
  instrument.

---

## 18. INTEREST-RATE SWAPS

### A. The contract
- **A1** REASON — satisfies the derivative contract, answering these its own way:
  - **A1.a** **underlying** — a **named floating reference rate that is observable and transacted**.
  - **A1.b** **payoff** — periodic exchange of fixed against floating on the notional; only the **net**
    moves.
  - **A1.c** **price** — the **fixed rate that makes the swap worth zero at inception**, cleared.
  - **A1.d** **notional** — never exchanged, which is why a swap is not a loan.
- **A2** REASON — **two legs with their own periodicity and accrual convention**, and they need not
  match — that mismatch is real and it is part of the price.
- **A3** REASON — the floating leg **fixes** on a stated date against the observed reference, and the
  fixing is a real observation, not a forecast. Where the reference is an overnight rate, the leg is the
  **compounded** overnight print.
- **A4** REASON — both legs are in **one currency**; two currencies makes it a cross-currency swap.

### B. Why each side is there
- **B1** REASON — a **borrower who issued fixed and wants floating**, or the reverse — it has debt it
  cannot economically reissue.
- **B2** REASON — an **asset manager with a duration mismatch**: a pension whose liabilities are long
  and whose assets are not.
  - **B2.a** this is a **structural, one-way demand**, and it is why long swap rates behave the way they
    do.
- **B3** REASON — a **bank managing its own gap**: assets repricing at a different speed from
  liabilities.
- **B4** REASON — a **speculator with a view on rates**. Required, not optional: without one the cleared
  par rate is a function of two regulatory gaps and cannot move because somebody thinks rates are wrong.
- **B5** REASON — a **dealer** running a book and hedging its net position.

### C. The curve
- **C1** REASON — swaps exist at **many tenors**, and the set of cleared fixed rates **is** a curve.
  - **C1.a** the curve is a **read of cleared prices**, never a fitted object that then prices the
    swaps.
- **C2** REASON — a **forward rate is derived** from the curve, and it is what the market thinks, not
  what will happen.
- **C3** REASON — the **swap curve and the sovereign curve are different curves**, and the difference is
  the **swap spread**.
  - **C3.a** which is a **consequence** — of bank credit, collateral, balance-sheet cost and who is
    forced to be on which side — and it is measured, never set.
- **C4** VERIFY — a change in the policy rate should move the short end through the reference rate and
  the long end through expectations, and the two channels are different.

### D. Valuation and cash
- **D1** REASON — after inception the swap has a **mark**, positive to one side.
- **D2** REASON — the mark **moves with the curve**, and the move is a real gain and a real loss.
- **D3** REASON — **variation margin turns that mark into cash**, so a rate move is a **liquidity event
  long before it is a P&L event**.
  - **D3.a** a hedger whose hedge is winning is receiving cash while its hedged item shows an unrealised
    loss, and the mismatch is a real funding problem.
- **D4** VERIFY — marks across the two sides sum to zero, and net payments sum to zero, every period.

### E. What must not happen
- **E1** FORBID — **no notional exchange.** If the notional moves, it is a loan and it belongs on the
  balance sheet as one.
- **E2** FORBID — **no fixed rate solved from the discount curve alone.** The fixed rate is cleared; the
  curve is read from the cleared rates. Doing it the other way makes the market a restatement of a
  formula.
- **E3** FORBID — **no floating leg on a rate this world does not produce.**

---

## 19. FX FORWARDS AND CROSS-CURRENCY SWAPS

### A. The forward
- **A1** REASON — satisfies the derivative contract, answering these its own way:
  - **A1.a** **underlying** — the **spot rate** at the future date.
  - **A1.b** **payoff** — exchange of two fixed amounts at maturity; **both notionals do move**, unlike
    a rate swap.
  - **A1.c** **price** — the **forward rate**, cleared.
  - **A1.d** **currency** — two of them, one per leg, by definition. **A leg states its own money.**
- **A2** REASON — it **settles**: real amounts in real currencies on the date, into accounts.
- **A3** REASON — before then it carries a **mark and margin** like any derivative, so a forward is a
  funding item long before it is a settlement. Its mark is against the forward for the tenor **left**,
  so a parity-struck forward is worth nothing at strike and the carry is **earned over its life, not
  booked at inception**.
- **A4** REASON — an **FX swap** — spot one way, forward back — is the standard form, and it is a
  **secured loan of one currency against another**, which is what it must be modelled as. **The banks
  are the largest users of it**, and a world without it has no market in which they can fund a foreign
  book.

### B. The forward rate
- **B1** REASON — the forward rate is **cleared** from what participants will do.
- **B2** VERIFY — it should sit near spot adjusted for the **two currencies' funding costs**, because
  otherwise somebody can borrow one, buy the other, lend it and lock a profit.
  - **B2.a** covered interest parity is therefore a **consequence of an arbitrage somebody takes**,
    never an identity applied to produce the rate. A forward struck as spot moved by a basis, carrying
    **no interest differential at all**, is neither cleared nor at parity, and carry is absent from the
    instrument.
  - **B2.b** and the arbitrage is **not free**: it uses balance sheet, capital and credit lines, so a
    persistent basis is possible and is a finding about those constraints.
- **B3** REASON — the **cross-currency basis** is the deviation, and it is a real price paid by whoever
  needs the currency more.
  - **B3.a** it widens when funding in one currency is scarce, which is exactly when hedgers need it.
  - **B3.b** FORBID — **there is one basis.** A cleared funding basis and a second basis on a random
    walk, with the second being the one participants see, trade and book P&L on, is law 4's defect in
    the most consequential possible place.
- **B4** VERIFY — a region running a funding deficit in a foreign currency should pay the basis, as a
  consequence of B3 and not as a rule.

### C. The cross-currency swap
- **C1** REASON — **two legs in two currencies**, notionals exchanged at start and end, periodic interest
  on both.
  - **C1.a** it is an interest-rate swap with an FX leg attached, and it inherits both curves.
- **C2** REASON — its economic use is **funding**: a party that raised money in one currency and needs it
  in another, for years, without an open FX position.
- **C3** REASON — the **notional exchange at the end is at the original rate**, which is what removes the
  currency risk and what creates the counterparty risk.
- **C4** REASON — its price includes the **basis**, and that is where a foreign-currency funding shortage
  shows up as a number.

### D. Why each side is there
- **D1** REASON — an **importer or exporter** with a known future foreign payment.
- **D2** REASON — an **investor holding a foreign asset** who wants the asset and not the currency.
  - **D2.a** and it must **roll** the hedge as the asset persists, which is a recurring demand and a
    recurring cost.
- **D3** REASON — a **bank funding a foreign-currency book**: deposits in one money, loans in another.
- **D4** REASON — a **dealer**, whose reason is spread, and whose own currency and funding positions
  constrain what it will quote. Its width is what carrying the position costs it — the return it needs
  on the capital the position consumes — not a stated number.

### E. What must not happen
- **E1** FORBID — **no forward rate from a parity formula.** It is cleared, and parity is checked against
  it.
- **E2** FORBID — **no hedge that removes the position without a counterparty holding it.**
- **E3** FORBID — **no maturity that passes without both legs settling** in full, in both currencies.
- **E4** VERIFY — a party's hedged foreign asset shows the asset revaluing one way and the forward
  revaluing the other, and **the residual is the basis and the imperfection — not zero by
  construction**.

---

## 20. COMMODITY FUTURES

### A. The contract
- **A1** REASON — satisfies the derivative contract, answering these its own way:
  - **A1.a** **underlying** — a **stated grade at a stated delivery location**.
  - **A1.b** **payoff** — delivery of the commodity, or cash settlement against the spot price at
    expiry.
  - **A1.c** **price** — the **futures price**, cleared.
  - **A1.d** **notional** — a fixed quantity per contract, so size is in **contracts**, not money.
- **A2** REASON — it is **standardised**, which is what makes it fungible, and standardisation means the
  delivery terms are part of the instrument.
- **A3** REASON — it has an **expiry**, and a **series of them** — so there is a **curve**.
- **A4** REASON — it is **margined every period**, so a price move is cash today, not at expiry.

### B. Why each side is there
- **B1** REASON — a **producer hedging** output it will have: it locks a price it can plan against.
- **B2** REASON — a **consumer hedging** input it will need.
- **B3** REASON — an **investor** taking a view, or wanting commodity exposure without storage.
  - **B3.a** it must **roll** as contracts expire, and the roll has a cost or a gain determined by the
    curve — **which is most of an investor's return and is not a fee**.
- **B4** REASON — an **arbitrageur** between the future and the physical, **who can only act if it can
  actually store and finance**. Without a storable stock there is no such participant, and the curve has
  nothing tying it to the physical world.
- **B5** REASON — a **dealer**, quoting both ways.

### C. The curve
- **C1** REASON — the relationship between futures prices and spot is a **consequence** of storage cost,
  financing cost and scarcity.
  - **C1.a** **contango**: forward above spot, **bounded above by what it costs to buy, store and
    finance** — because past that, B4 arbitrages it.
  - **C1.b** **backwardation**: forward below spot, **unbounded below, because you cannot store a
    shortage.** That asymmetry is real and it is why the two states are not symmetric.
- **C2** REASON — the curve therefore **carries information about physical tightness**, and inventory is
  the state variable it reads.
- **C3** VERIFY — inventories low implies backwardation, as a consequence of C1.b and never as a rule.
- **C4** REASON — the **futures price converges to spot at expiry, because delivery is possible**.
  - **C4.a** convergence is a **consequence of deliverability, not an enforced boundary condition.**

### D. Expiry and delivery
- **D1** REASON — at expiry the contract **delivers or cash-settles**, and both are real.
- **D2** REASON — **physical delivery must be possible for at least some participants**, or the
  convergence in C4.a has no mechanism behind it.
- **D3** REASON — a party that cannot take delivery must **close or roll before expiry**, which is a real
  forced trade at a known time.
- **D4** REASON — cash settlement is against an **observed, cleared spot price**.

### E. What must not happen
- **E1** FORBID — **no futures price without a physical market underneath it.** A futures curve on a
  commodity that is never actually traded prices itself.
- **E2** FORBID — **no unlimited open interest against finite deliverable supply** without the squeeze
  that implies.
- **E3** FORBID — **no roll that is free.** The roll's cost is the curve, and it must land in the
  roller's P&L.

---

## 21. COMMODITIES SPOT

### A. What a commodity is
- **A1** REASON — a **standardised, fungible unit** — a grade, at a location, in a quantity unit.
  - **A1.a** **location is part of the identity**: the same grade in two places is two prices, and the
    difference is transport.
- **A2** REASON — it is **produced by named producers** and **consumed by named consumers**.
- **A3** REASON — it is **storable, at a cost**, which is what makes it an asset and not just a flow.
- **A4** REASON — its **stock is finite and observable**: inventory is a real number **held by real
  parties, at real locations.**

### B. Supply
- **B1** REASON — a producer produces at a **cost**, and it produces because the price covers it.
  - **B1.a** costs differ across producers, so the supply schedule is a **consequence of the cost
    distribution**, never a curve written down.
- **B2** REASON — **capacity is fixed in the short run** and changes only through investment, which takes
  time.
  - **B2.a** which is why supply is inelastic on the horizon that matters, and why price does the
    adjusting.
- **B3** REASON — production can be **disrupted**, and a disruption is a **real loss of units** at the
  point they would have been made — not a multiplier on a price.
- **B4** REASON — a producer can **hold inventory** rather than sell, if it expects a better price.

### C. Demand
- **C1** REASON — a consumer buys because it **needs the input** or consumes it.
- **C2** REASON — demand is **inelastic in the short run** for the same reason as B2: the buyer cannot
  change its process this period.
- **C3** REASON — an **investor** may also buy, to hold, because it expects the price to rise — a reason
  like any other, competing for the same physical units.
- **C4** VERIFY — with both sides inelastic, **small imbalances should produce large price moves**. A
  consequence to be measured, not a volatility parameter.

### D. The market
- **D1** REASON — the price **clears**, per grade and location.
- **D2** REASON — **inventory is the buffer**: when demand exceeds production, stocks fall, and when
  stocks approach zero the price has nothing left to ration with.
  - **D2.a** so the inventory level is a **state variable that carries across periods**, moved by
    production and consumption, and **the price depends on it**. A percentage on a random walk, untouched
    by production or consumption and not an input to the price, is not inventory.
- **D3** REASON — **storage costs money** and the cost is paid to somebody who owns the storage.
- **D4** REASON — the relationship between spot and forward is a **consequence** of storage cost,
  financing cost and scarcity, never an imposed basis.
- **D5** VERIFY — **produced plus opening inventory equals consumed plus closing inventory**, per
  commodity and location, exactly.

### E. What it feeds
- **E1** REASON — commodity prices are **input costs to firms**, and they show up in producer prices
  before consumer prices.
- **E2** REASON — energy reaches **households directly** as consumption.
- **E3** REASON — a **producing region's terms of trade** move with the price, which moves its currency's
  fundamentals.
- **E4** VERIFY — a commodity shock should propagate **to margins, then to inflation, then to policy** —
  through the chain, and if it arrives anywhere directly, a link has been short-circuited.

### F. What must not happen
- **F1** FORBID — **no consumption without production or inventory.** Units cannot be conjured.
- **F2** FORBID — **no negative inventory**, ever, anywhere.
- **F3** FORBID — **no price from a written path.** A commodity price series applied to the world removes
  every mechanism above.

---

## 22. INDICES

### A. What an index is
- **A1** REASON — a **stated rule** over a **stated set of constituents** at **stated weights**.
  - **A1.a** all three are public and stable; an index nobody can reproduce is not a benchmark.
- **A2** REASON — it reads **cleared prices** and nothing else.
- **A3** FORBID — **an index is never an input to its own constituents.** If a constituent's price is
  derived from the index, the index measures itself and the circularity is invisible in every output.
- **A4** REASON — it has a **unit and a base**: a level is meaningless without them.

### B. Construction
- **B1** REASON — **weights come from something real**: market capitalisation, amount outstanding, equal
  weight — and the choice is stated.
- **B2** REASON — the constituent set **changes**: firms enter and leave, bonds mature.
  - **B2.a** and a change must not create a jump in the level: the index is **chained** across the
    rebalance, because the level's continuity is the whole basis of a return series.
- **B3** REASON — **corporate actions** are handled explicitly — a split changes shares and price together
  and must not change the level.
- **B4** VERIFY — the index return over a period equals the weighted return of its constituents, to
  arithmetic dust.

### C. What it is used for
- **C1** REASON — a **benchmark**: a manager's performance is measured against it, and that measurement
  drives flows.
- **C2** REASON — a **mandate**: a fund tracks it, so a change in the index is a **real forced trade** by
  every tracker, at the same time.
  - **C2.a** VERIFY — inclusion and exclusion should therefore be visible in the constituent's price, as
    a consequence of C2, never as an applied bump.
- **C3** REASON — an **underlying**: futures, options and swaps settle against it, which makes it a
  settlement price and therefore money.
- **C4** REASON — a **signal**: participants read it as the state of a market.

### D. The index families this world needs
- **D1** REASON — an **equity index** per region.
- **D2** REASON — a **credit index**: an average spread or price over a defined bond set — a derived read
  of derived reads, which must be built from prices first.
- **D3** REASON — a **rate benchmark**: the reference short rate that floating instruments fix on.
  - **D3.a** it must be a read of **actual transactions**, because everything that references it pays
    real money against it. A cleared overnight rate exists in this world; that is what floating coupons
    fix on.
  - **D3.b** FORBID — **no benchmark that is posted rather than transacted.** A rate nobody traded at is
    an assigned price with a huge notional attached to it. **A policy rate is not a benchmark.**
- **D4** REASON — a **price level for the real economy** — and producer prices and consumer prices are
  **different indices**: different baskets, different stage of production, different weights, and they can
  move apart.
  - **D4.a** the difference between them **is a margin story**: input prices rising faster than output
    prices is a squeeze on firms, and collapsing the two hides it.
- **D5** REASON — **one index system.** Two index systems, of which the read one is invented and the
  computed one is unread, is law 4's defect at the level of the whole market's benchmark.
  - **D5.a** and an index's **opening history is not a random walk.** Every covariance measured against
    that history — every beta, and therefore every discount rate that uses one — is a covariance against
    noise for as long as the invented history dominates the window.

### E. What must not happen
- **E1** FORBID — **no index without constituents.** A level that moves without a constituent moving is
  an invented price.
- **E2** FORBID — **no stored level.** It is recomputed from the register and the prices, always.
- **E3** VERIFY — an index and its constituents move together by construction, and a divergence is a
  defect in the read, not a market event.

---

# PART VI — FINANCIAL INSTITUTIONS

Seven systems. A bank is three of them, because its lending, its funding and its capital can each be
absent independently.

---

## 23. BANKS — LENDING

### A. What a loan is
- **A1** REASON — a **bilateral contract** between a named bank and a named borrower.
  - **A1.a** it is **not** a security: it is not transferable by default and has no market price.
  - **A1.b** it is therefore carried differently.
- **A2** REASON — terms fixed at origination: principal, maturity, rate or margin, currency.
- **A3** REASON — a **drawdown structure**: a term loan drawn at once, or a **facility** drawn and repaid
  at the borrower's option.
  - **A3.a** an undrawn commitment is a **real obligation of the bank** and consumes something.
  - **A3.b** VERIFY — undrawn commitments are visible; a facility that costs nothing until drawn is a
    free option the bank did not sell.
- **A4** REASON — **security**: secured on named collateral, or unsecured.
- **A5** REASON — **covenants**, and a breach is an observable event.

### B. Writing it — where the money comes from
- **B1** REASON — **a bank lends by creating a deposit.** The loan appears on one side and the borrower's
  balance on the other, at the same instant.
  - **B1.a** **no reserve leaves the bank at origination.** This is the whole of endogenous money.
  - **B1.b** reserves move only when the borrower **spends** it to a customer of another bank, and then
    as an ordinary payment.
  - **B1.c** FORBID — a bank does not lend "out of" its deposits or its reserves. **A model in which it
    does cannot produce a credit cycle.**
- **B2** REASON — but the bank is **constrained**, and the constraints are real and separate:
  - **B2.a** **capital**: the loan consumes it;
  - **B2.b** **liquidity**: the deposit it created may be spent away, and it must fund that. **A bank
    out of cash and collateral does not write the same book as one flush with reserves**;
  - **B2.c** **its own risk appetite**, which is a decision;
  - **B2.d** VERIFY — which constraint binds is an outcome and differs by bank and by period.

### C. The price
- **C1** REASON — the bank **quotes a rate**, built from its own economics:
  - **C1.a** its **cost of funds** — its own, from its own funding mix, not the policy rate;
  - **C1.b** the borrower's **expected loss** — a probability of default and a loss given default it
    assesses itself;
  - **C1.c** the **capital** the loan consumes, times its required return on it;
  - **C1.d** an **operating cost** of making the loan.
- **C2** REASON — the borrower **accepts or refuses**, and can go elsewhere. **The borrower shops**: each
  lender quotes, the borrower takes the keenest quote that its own hurdle accepts, and a wide quote loses
  volume to a tight one.
  - **C2.a** so the rate is the outcome of a negotiation, not a schedule the bank imposes.
- **C3** REASON — the bank can **decline**, and declining is the credit decision.
  - **C3.a** VERIFY — declined volume is visible. **A bank that never says no has no credit standard.**
- **C4** FORBID — **one default-probability model per borrower.** Two models that disagree mean the price
  and the provision are struck against different beliefs.

### D. Carrying it
- **D1** REASON — held at **amortised cost**, not marked to a market that does not exist.
- **D2** REASON — a **provision** against expected loss, taken as a charge to income.
  - **D2.a** it moves when the assessment moves, and the movement is an income event.
  - **D2.b** FORBID — a provision is never a reserve that quietly absorbs losses. It is booked, and the
    booking is visible. **A realised loss rate is not a provision stock.**
- **D3** REASON — interest **accrues** and is **received**, and non-payment is observable.
- **D4** REASON — a loan can be **sold or syndicated**, and then it has a price and a buyer.
  - **D4.a** a **syndicated loan** is one loan with **several lenders of record**, each a row per
    (lender, borrower) in its own share, struck together at **one margin** by a **lead** that arranges
    it and takes a fee. Each lender's share sits against its own capital (B2.a) and its own
    large-exposure limit (F3); a loan too large for one bank's limit is written by several, or not at
    all — never by one bank past its limit.
- **D5** REASON — it can be **pledged**, at a haircut.

### E. When it goes bad
- **E1** REASON — a missed payment or a covenant breach is an **EVENT**.
- **E2** REASON — the loan is **reclassified**: performing to impaired, with a bigger provision. **A loan
  status that no path ever writes is not a status.**
- **E3** REASON — **workout**: restructure, extend, or enforce — and each is a decision with a cost.
- **E4** REASON — **enforcement**: the collateral is realised for what it fetches.
- **E5** REASON — the **write-off**: the loan leaves the book, on a date, and the loss hits capital.
  - **E5.a** VERIFY — the loss that reaches capital equals principal minus recovery minus provisions
    already taken. Double-counting a provision flatters capital.
- **E6** REASON — losses are **correlated across borrowers**, because they share a cause.

### F. The book in aggregate
- **F1** REASON — the book is the **sum of named loans**, never a scalar that grows by a rate.
  - **F1.a** FORBID — **no "loan book" number that is not the sum of loans.** A book with no loans in it
    cannot default, cannot be provisioned and cannot be sold. Every loan a bank makes — corporate,
    pooled, mortgage, consumer, and the central bank's loan to it — is a **row with a lender of record, a
    borrower, and its own terms**, held in the same register as anything else it owns.
- **F2** VERIFY — new lending, amortisation, prepayment and write-off account for the change.
- **F3** REASON — **concentration**: exposure to one name, one sector, one region is measurable and is a
  risk the bank manages, **with a limit that binds** — a large-exposure limit that changes what the bank
  will write.

---

## 24. BANKS — FUNDING AND LIQUIDITY

### A. Who funds it
- **A1** REASON — **deposits**, and they are not one thing:
  - **A1.a** **retail**: many, small, sticky, and insured up to a limit — and where the depositor is a
    cell (XI-15), the insured amount is `weight × min(member balance, limit)`, which is exact because
    the cell is homogeneous, and is what makes E4's break in the loop real rather than notional;
  - **A1.b** **corporate**: fewer, larger, operational — a firm banks where it transacts;
  - **A1.c** **institutional/wholesale**: few, very large, and **rate-sensitive**;
  - **A1.d** VERIFY — stickiness differs by class, and it is the whole of liquidity risk. **A model with
    one deposit type cannot have a run.**
- **A2** REASON — **wholesale borrowing**: interbank, repo, paper it issues.
  - **A2.a** short, and it **rolls** — which is where a funding squeeze bites.
- **A3** REASON — **capital**: equity and subordinated debt, which do not run.
- **A4** REASON — **the central bank**, on the corridor's terms.
- **A5** REASON — each source has a **price**, the prices differ, and the mix is a decision.

### B. The cost of funds
- **B1** REASON — the bank **pays a rate on each source**, and it is a real payment to a real holder.
  - **B1.a** a deposit rate the bank **sets**, and depositors respond to — bounded above by the cheaper
    of the bank's own wholesale cost and the money fund's yield, on the contested share of its base.
  - **B1.b** a wholesale rate the **market** sets.
- **B2** REASON — its **blended cost of funds** is a read of B1 across the mix.
  - **B2.a** which feeds the loan price directly. **A bank that has no cost of funds prices every loan as
    if it funded at the policy rate whatever its own position** — and then no funding condition anywhere
    can reach a borrower.
  - **B2.b** FORBID — **one rate per liability.** A loan whose interest cost is computed one way for a
    margin statistic and another way for the cash that leaves is two representations of one price.
- **B3** REASON — **net interest margin** is what it earns minus B2, **and it can be negative.**

### C. The liquidity position
- **C1** REASON — the bank holds **liquid assets**: reserves, and securities it can sell or pledge.
  - **C1.a** they differ in how fast and how surely they convert — a haircut and a market depth.
- **C2** REASON — it holds them against **what could leave**, and that is A1.d's stickiness.
  - **C2.a** a **buffer preference derived from its own liabilities**, not a stated ratio.
- **C3** REASON — **maturity transformation is the business**: it funds long assets with short
  liabilities, and that gap is why it earns anything.
  - **C3.a** VERIFY — the gap is measurable, and a bank with none is not a bank.
- **C4** VERIFY — its position is the **residue of everybody else's period**. It did not choose it.

### D. When the position is short
- **D1** REASON — it **borrows in the market**, secured or unsecured.
- **D2** REASON — it **sells or pledges** liquid assets — a real order in a real book.
- **D3** REASON — it **bids up for deposits**, and pays for them.
- **D4** REASON — it **shrinks its assets**: it stops lending, and lets the book run off.
  - **D4.a** which transmits a funding problem into the credit decision — this is the credit crunch.
- **D5** REASON — it draws the **central bank facility**, collateralised and at a penalty.
- **D6** REASON — **it can fail to fund itself**, and that is a distinct failure from insolvency.
  - **D6.a** FORBID — **there is no unbounded, uncollateralised, unpriced credit line that makes D6
    unreachable.** A facility with none of the classical conditions does not bound anything; it deletes
    the entire branch above it, and with it the reason C2 exists.

### E. The run
- **E1** REASON — depositors **can leave**, and the ones in A1.c leave fastest.
- **E2** REASON — they leave **because they observe something**.
  - **E2.a** and what they observe must be **observable**: a capital ratio, a facility draw, a rate paid
    up, a rating action, a run of periods ending short.
- **E3** REASON — leaving **forces D2 and D4**, which produce more of E2.a.
  - **E3.a** VERIFY — the loop is self-reinforcing, and the model should be able to show one. **The
    deposit leaves with the reserves behind it**, so the bank is shorter at the next close.
- **E4** REASON — **deposit insurance breaks the loop** for A1.a and not for A1.c.
  - **E4.a** which is why a run is a wholesale phenomenon first.
- **E5** REASON — a run at one bank is **information about others**, through E2.a — and a bank whose only
  capital is retained earnings has no answer to it.

### F. What it reports
- **F1** REASON — its **deposit lines by class**, as reads of who actually banks there.
- **F2** REASON — its **reserve balance**, as a read of its account — one row at the central bank, moved
  only by settlement legs. Not a mirrored copy.
- **F3** VERIFY — assets equal liabilities plus equity, in the bank's own money, every period.
- **F4** REASON — a **liquidity metric somebody outside can see**.

---

## 25. BANKS — CAPITAL AND RESOLUTION

### A. What capital is
- **A1** REASON — capital is the **residual**: assets minus liabilities. **It is not a fund.**
  - **A1.a** FORBID — capital is never a pot that is spent. It is what is left, and it falls when a loss
    is booked because the asset fell, not because something was withdrawn.
- **A2** REASON — it is **layered**, and the layers absorb in order:
  - **A2.a** **equity** absorbs first and fully;
  - **A2.b** **subordinated debt** absorbs next, and its holders are creditors who took that risk. **A
    ladder with no subordinated layer is one layer short at the top and one over-punished in the
    middle**: senior paper gets bailed in and depositors are never touched;
  - **A2.c** **senior creditors and depositors** last, and only in resolution.
- **A3** REASON — it **grows** by retained earnings **and by issuance**, and both are decisions. **A bank
  is a firm and its financing decision is a firm's**: an equity issue priced by the equity market, which
  **can fail**, and a subordinated issue priced by the credit market.
- **A4** REASON — it **falls** by losses and by distributions, and both are events with dates.

### B. How much there must be
- **B1** REASON — a **requirement**, expressed against risk-weighted assets.
  - **B1.a** **risk weights differ by asset**, and that is why a bank prefers some assets to others.
  - **B1.b** a **leverage** constraint that does not use weights, as a backstop.
  - **B1.c** VERIFY — which binds is an outcome and differs by bank.
- **B2** REASON — a **buffer above the requirement the bank chooses**, because hitting the requirement has
  consequences. The buffer is the bank's own choice, not a stated ratio.
- **B3** REASON — breaching it triggers **consequences before failure**: distributions restricted, a plan
  demanded, supervision intensified.
  - **B3.a** VERIFY — a bank near the line behaves differently. If it does not, the requirement is
    decorative.

### C. When it runs out
- **C1** REASON — **insolvency** is assets below liabilities, and it is **distinct from illiquidity**.
  - **C1.a** a bank can be **solvent and illiquid**, or **insolvent and liquid**, and the two failures
    have different triggers and different remedies. Both triggers must exist, and the resolution must say
    which one fired.
- **C2** REASON — **recapitalisation first**, if somebody will provide it.
  - **C2.a** existing holders diluted, new money priced by whoever provides it.
  - **C2.b** it **can fail** — nobody has to buy.
- **C3** REASON — **RESOLUTION**: the bank stops being a going concern.
  - **C3.a** a **trigger somebody applies**, on an observable.
  - **C3.b** it is not the same as bankruptcy: deposits keep working.

### D. The resolution itself
- **D1** REASON — a **valuation**: what the assets are **actually worth**, not their book.
  - **D1.a** and the hole is the difference. **A resolution cannot value the book it takes until a loan
    can be worth less than its face.**
- **D2** REASON — **the hierarchy is respected**: equity wiped, then subordinated bailed in, then the
  rest.
  - **D2.a** VERIFY — **no creditor is worse off than in a liquidation.** That is the constraint the whole
    design serves.
- **D3** REASON — an **acquirer** takes the book, or there is none.
  - **D3.a** it takes assets **and** liabilities, and pays or is paid the difference.
  - **D3.b** the acquirer is **choosing, and can decline** — it makes a **bid**, and a resolution with no
    bid falls through to the public path. An assigned acquirer whose bid is the estate's own value by
    construction is not choosing.
- **D4** REASON — **deposit insurance pays** what the estate cannot, up to the limit, and the insurer
  becomes a creditor of the estate. Where the depositor is a cell the limit applies **per member**
  (Banks Funding A1.a), so a large cell of small depositors is covered and a cell of large ones is not,
  which is the distinction the insurance exists to draw.
- **D5** REASON — **the public purse is the last resort**, and it is a **fiscal cost with a payer**.
- **D6** REASON — the failed bank's **positions do not vanish**: every book it was on has a counterparty
  problem, and that is contagion. Its loans, its lines, its swap draws and its margin all re-seat on a
  named successor.

### E. After
- **E1** REASON — the **estate** is realised over time, and creditors are paid from it.
- **E2** REASON — the **surviving system is more concentrated**, and that is a measurable consequence.
- **E3** VERIFY — the resolution **conserves**: what the acquirer took, what the insurer paid, what the
  estate realised and what holders lost sum to the hole in D1.

---

## 26. DEALER DESKS

### A. What a dealer is
- **A1** REASON — a **named party**, usually a bank's trading arm, with its own balance sheet inside a
  bank's.
- **A2** REASON — it **quotes a price at which it will buy and a price at which it will sell**, and it is
  willing to do either.
- **A3** REASON — it holds **inventory**: what it has bought and not yet sold, and the reverse.
- **A4** REASON — it makes money from the **spread** and loses money from the **inventory**, and the two
  are the whole business.

### B. Why it quotes
- **B1** REASON — it expects **two-way flow**: buyers and sellers arriving at different times, so it earns
  the spread for bridging the time between them.
- **B2** REASON — it has **information** from seeing the flow, **and it knows whom it faced** — the
  information is worth something only if the counterparty is named.
- **B3** REASON — the client **pays for immediacy**: the alternative is waiting for a natural counterparty,
  which may not come.
- **B4** FORBID — **it does not quote because the mechanism needs somebody to.** If a desk's schedule is
  derived from the residual imbalance, it is the buyer of last resort with a different name, and every
  price the mechanism produces is a fixed point of that patch.

### C. How it prices — the spread is a consequence
- **C1** REASON — the quote comes from the desk's **own state**: its inventory, its cost of funds, its
  risk limit, its view.
- **C2** REASON — **inventory skews the quote.** Long already means it bids lower and offers lower, because
  it wants to sell.
  - **C2.a** this is how a desk mean-reverts its book without anyone telling it to, **and it is why order
    flow moves prices.**
- **C3** REASON — **risk widens the quote**: volatility, illiquidity, a position it cannot hedge.
- **C4** REASON — **adverse selection widens it**: a client who knows more is expensive to face.
- **C5** REASON — the **bid–offer is therefore the output** of C1–C4, and the same posted quote belongs in
  every book the desk makes a market in, cash books included.
  - **C5.a** FORBID — **no spread applied to a mid.** A mid with a spread bolted on is a single price
    pretending to be two, and it cannot skew, widen, or refuse.
  - **C5.b** FORBID — **no stated spread table.** A width per book, per instrument or per client type is a
    price assigned, not a consequence: what a desk charges is what carrying that position costs it.
  - **C5.c** a client's conversion or assembly fee is the same thing seen from the client's side: **the
    desk's own width on that flow, on its own share of it**, not a flat charge to the market.

### D. Limits — why an auction can still fail
- **D1** REASON — it has a **position limit** per instrument and in aggregate, set by its own risk
  function.
- **D2** REASON — it has a **capital charge** on what it holds, and the charge is real.
- **D3** REASON — it has a **funding cost on the inventory, paid every period it holds it.** **A desk
  never charged rent for its inventory carries a position for free and has no reason to shed it.**
- **D4** REASON — when a limit binds it **widens, shrinks its size, or stops quoting** — and stopping is a
  legitimate, representable state.
  - **D4.a** which is precisely what makes a **failed auction** possible. **A market fails when the dealers
    step back, and the dealers step back for the reasons in D1–D3.**
- **D5** VERIFY — in a stress period, desk inventory, spreads and capital usage should all move together;
  if spreads widen without inventory moving, the widening is imposed.

### E. Hedging and the rest of the book
- **E1** REASON — it **hedges what it can**: a bond against a swap, a share against an index, an FX
  position against another client's.
  - **E1.a** a hedge is a **trade with a counterparty**, not a reduction in a number.
- **E2** REASON — the hedge is **imperfect**, and the residual is basis risk it carries.
- **E3** REASON — **desks face each other**: an **interdealer market** exists, for cash instruments as well
  as derivatives, and it is where inventory gets redistributed.
- **E4** VERIFY — dealer inventory across desks equals the position the rest of the world does not hold; it
  is a real number and it should move with client flow.

### F. What must not happen
- **F1** FORBID — **no infinite balance sheet.** Every desk's capacity is finite and enumerable.
- **F2** FORBID — **no desk exempt from its own bank's capital and funding.**
- **F3** FORBID — **no desk whose P&L is the spread times volume.** Its P&L is the spread earned **minus
  what the inventory did**, and a desk that cannot lose money is not taking the other side.

---

## 27. INSURERS AND PENSIONS

### A. What they are
- **A1** REASON — **named parties with accounts and registers**, holding assets against liabilities they
  owe to named beneficiaries.
- **A2** REASON — **a real liability**: a promise to pay stated amounts at stated future times.
  - **A2.a** it is a **liability of the institution**, not a fund share — **the beneficiary does not
    absorb the investment result.** That is the whole difference between this sector and a fund, and a
    sector that passes the investment result straight through is a fund wearing an insurer's name.
  - **A2.b** except where the contract says otherwise, in which case it **is** a fund share and must be
    modelled as one.
- **A3** REASON — **equity is assets minus liabilities**, a read, and **it can go negative** — which is a
  solvency event with consequences. **These institutions can fail.**
- **A4** REASON — they receive **premiums or contributions** and pay **claims or pensions**, both real
  flows to and from named parties.
  - **A4.a** an insurer carries a **book of cover** and **quotes a price** for it that answers its own
    losses and its own capital; **a policy goes to the insurer that prices lower**, subject to the cover
    that insurer's surplus can stand behind. An insurer with no surplus writes nothing and loses its
    renewals — **it loses book before it loses its licence** — and cover nobody can write is unplaced and
    pays no premium.
  - **A4.b** the price is the claims a unit of cover is expected to bring, plus the return required on the
    capital held against the premium. Worse experience or dearer capital quotes higher.
  - **A4.c** an insurer draws **its own** claims off **its own** cover and **its own** experience, and its
    experience moves toward what its periods actually cost it.

### B. The liability side
- **B1** REASON — the liability has a **schedule**: how much is owed in each future period.
- **B2** REASON — its **present value depends on a discount rate read from a market**.
  - **B2.a** so **falling rates raise the liability**, which is why a rate move is a solvency event for
    this sector and a P&L event for everybody else.
  - **B2.b** FORBID — **no fixed discount rate, and no liability that is a cash balance.** A liability
    that accumulates contributions minus benefits plus investment income has no schedule, no discount rate
    and no discounting — so it never moves when rates move, and **the sector's defining risk disappears.**
    This is the model's largest holder of duration; it must have duration.
- **B3** REASON — the schedule is **uncertain**: mortality, longevity, claim frequency.
- **B4** REASON — an insurer's claims can be **correlated and lumpy** — **a catastrophe is one event
  hitting many policies at once**, which is different from the average being higher. Claims computed as a
  ratio of premium for every policy have no representation for one.

### C. The asset side
- **C1** REASON — it invests the premiums, and the **portfolio is a decision** with reasons.
- **C2** REASON — the **dominant reason is matching B1**: long assets against long liabilities.
  - **C2.a** so it is a **structural buyer of long bonds and long swaps** — a one-way demand that exists
    whatever the price, which is a real force in that market and not a preference.
- **C3** REASON — it can hold **illiquid assets**, because it does not face redemption the way a fund does
  — **that is what it is paid for**, and it should therefore be a limited partner in illiquid vehicles and
  earn the premium.
- **C4** REASON — it **lends securities** for extra return.
- **C5** REASON — it is a **buyer of credit**, and its mandate limits which credits — so a downgrade can
  **force a sale**.

### D. The gap and what it forces
- **D1** REASON — assets and liabilities **do not match**, and the mismatch is measurable in duration and
  in cash flow.
- **D2** REASON — the mismatch **moves equity when rates move**, in the opposite direction to a bank's.
- **D3** REASON — a **funding shortfall** has consequences: the sponsor contributes, the fund de-risks, or
  benefits are cut — each a real action by a named party. **An underfunded institution reaching for more
  risk with no solvency consequence is not a constraint.**
- **D4** REASON — it can **hedge** the gap, and hedging it costs money and creates margin calls.
  - **D4.a** a leveraged hedge turns a solvency improvement into a **liquidity requirement**, which is the
    failure mode of the whole sector.
- **D5** VERIFY — a large rate move should show as: the liability revaluing, the hedge revaluing the other
  way, **and cash moving on the hedge but not on the liability.** That asymmetry is the finding.

### E. What must not happen
- **E1** FORBID — **no liability without beneficiaries.** Somebody named is owed the money.
- **E2** FORBID — **no asset that is not somebody's liability or a real thing.**
- **E3** FORBID — **no solvency measured against a stored liability value.** It is a read from B1 and B2,
  every time.
- **E4** VERIFY — the sector's holdings, added to every other holder's, equal what was issued.

---

## 28. HEDGE FUNDS

### A. What they are
- **A1** REASON — a **named party** with investors, a register of holdings, and accounts.
- **A2** REASON — **investor capital is equity**: the investors bear the result, **and they hold a share
  count they can redeem**.
- **A3** REASON — a **manager** is a separate party earning a fee — a management fee on assets and a
  **performance fee on gains**, and the asymmetry of that second fee is a reason for risk-taking.
- **A4** REASON — a **mandate that is wide**: it can be long, short, levered, and in many markets.
- **A5** REASON — everything is **marked to market at cleared prices**, so its equity moves continuously.

### B. Leverage
- **B1** REASON — it **borrows to hold more than its equity**, from a named lender.
  - **B1.a** leverage is therefore **a fact about a loan**, never a property of the fund.
- **B2** REASON — it also levers **through derivatives**, where the notional exceeds the margin.
- **B3** REASON — and through **repo** against the securities it holds.
- **B4** REASON — the amount available is the **lender's decision**, and it changes.
- **B5** VERIFY — gross exposure, net exposure and equity are three different reads and all three are
  needed; a single "leverage" number hides which one moved.

### C. What it does in a market
- **C1** REASON — it takes **positions for reasons**: a relative-value view, a directional view, a liquidity
  premium it is paid to hold. **It is the natural home of the speculative side of every derivative book.**
- **C2** REASON — it will be the **buyer when others are forced sellers**, if it has capacity — which makes
  it a genuine participant, with a limit like everyone else.
- **C3** REASON — it **shorts**, which requires a borrow.
- **C4** REASON — its trades are **real trades with real counterparties at cleared prices**.

### D. The failure mode
- **D1** REASON — a **loss reduces equity**, and with fixed borrowing, leverage rises.
- **D2** REASON — the lender **calls margin**.
- **D3** REASON — meeting the call requires **selling**, at market prices, **which moves prices**.
- **D4** REASON — the price move hits **other holders of the same positions**, who may be levered too — and
  D1 starts again for them.
  - **D4.a** this loop is the mechanism, and it must be **emergent from D1–D3**, never a contagion
    parameter.
- **D5** REASON — **investor redemptions** arrive at the same time, for the same reason, and they are a
  second forced-seller channel.
  - **D5.a** a **redemption gate or notice period** delays it, which is a real contractual term with real
    consequences for who gets out.
- **D6** REASON — the fund can **fail**, and then its broker eats the shortfall and its investors lose their
  equity.
- **D7** VERIFY — the chain from one fund's loss to another fund's margin call must be traceable through
  prices and named counterparties.

### E. What must not happen
- **E1** FORBID — **no leverage without a lender.**
- **E2** FORBID — **no position that does not mark.** A fund carrying an unmarked position has hidden its
  own equity from itself.
- **E3** FORBID — **no fund that cannot fail.** A vehicle that absorbs losses indefinitely is the buyer of
  last resort in a different costume.

---

## 29. PRIVATE EQUITY

### A. The structure
- **A1** REASON — a **fund with committed capital from named investors**.
- **A2** REASON — capital is **committed, not paid**: it is **called** when a deal needs it, and the call is
  a real payment from the investor's account **on a date it cannot refuse**.
  - **A2.a** so an investor must hold liquidity against calls it did not choose the timing of, and in a
    stress the calls and its own troubles arrive together.
  - **A2.b** FORBID — **a call bounded by the investor's spare cash is not an obligation.** The investor
    funds it from its own liquidity ladder — selling if it must — **or it defaults on the call**, which is
    itself an event with consequences.
- **A3** REASON — a **manager** earning a fee on committed capital and a share of the gains.
- **A4** REASON — the fund has a **life**: it invests, it holds, it exits, **and it winds up** — and on
  winding up the investors' claims resolve into cash rather than freezing.
- **A5** REASON — the acquired firms are **held in named vehicles**, each a party with its own balance
  sheet.

### B. The buyout
- **B1** REASON — it buys a firm at a **price agreed with the sellers**.
- **B2** REASON — most of the price is **debt raised against the target itself**.
  - **B2.a** the debt is the **target's** liability, not the fund's — which is why a failed buyout kills
    the firm and not the fund.
  - **B2.b** so the deal only happens if lenders will lend, at a price: **the credit market decides which
    buyouts occur**, and that is a real constraint, not a rate applied to a plan.
- **B3** REASON — the **equity cheque is the rest**, funded by A2.
- **B4** REASON — the target's balance sheet is **transformed at the moment of purchase**: leverage up,
  interest cost up, ownership changed in the register.
- **B5** VERIFY — **the sources and uses of a deal must balance exactly**, and the money must come out of
  named accounts. Sellers paid the equity cheque while the debt proceeds stop at the target is a deal that
  did not balance.

### C. The hold
- **C1** REASON — the firm **operates and services its debt** out of cash flow, and the higher leverage
  means less room.
- **C2** REASON — the owner **influences the firm**: investment, costs, distributions.
- **C3** REASON — it can **recapitalise**: raise more debt to pay itself a distribution, which is a real
  transfer from the firm's future to the owner's present.
- **C4** REASON — it can **fail**: the leverage in B2 makes default a real outcome, the loss falls on the
  lenders and wipes the equity.
- **C5** REASON — the holding has a **value that is not a market price**: no clearing, so it is a **mark**.
  - **C5.a** FORBID — **an unlisted mark is not a cleared price**, and it must never be treated as one by
    the holder's own accounts. The honest answer is *"marked, not cleared"*.

### D. The exit
- **D1** REASON — it **sells**: to another fund, to a corporate buyer, or to the public market.
- **D2** REASON — the exit produces a **cleared price**, which is the first real price the holding has had.
- **D3** REASON — proceeds are **distributed to the investors**, in cash, into their accounts.
- **D4** REASON — the exit **depends on the market being open**: in a bad market it does not happen, the
  hold extends, and the distributions do not arrive.
  - **D4.a** which feeds back to A2.a — investors owe calls and are not receiving distributions at the same
    time.
- **D5** VERIFY — the fund's returns are a read of D3 against A2, and both are actual cash.

### E. What must not happen
- **E1** FORBID — **no buyout without a lender who agreed to lend.**
- **E2** FORBID — **no capital call that is not paid from a real balance.**
- **E3** FORBID — **no exit at a price nobody paid.**

---

# PART VII — THE PUBLIC SECTOR

---

## 30. THE TREASURY

### A. What the treasury is
- **A1** REASON — a **named party with an account** like any other.
  - **A1.a** it pays out of a balance, **and the balance can run low**.
- **A2** REASON — its money is its region's currency.
- **A3** REASON — it has a **balance sheet**: cash, debt outstanding, and whatever it owns.
  - **A3.a** its equity is negative and that is normal; the number is still a read.

### B. Outlays
- **B1** REASON — it **spends on named things** — transfers to households, purchases of goods, wages,
  interest — and each reaches a named recipient's account.
- **B2** REASON — **interest is an outlay**, and it is the sum of what its own bonds pay, **read from the
  register**, never a rate applied to a total.
- **B3** REASON — outlays have **causes that vary**: the cycle, unemployment, policy (§47 D2).
  - **B3.a** so they are not a constant, and **a downturn raises them while lowering receipts**, which is
    the whole reason the constraint in D bites when it does.
- **B4** REASON — **maturing debt must be repaid** in full, in cash, on its date, and it is the largest
  single outlay in most periods.

### C. Receipts
- **C1** REASON — **taxes**, levied on real bases: income, consumption, profit.
  - **C1.a** paid by named payers out of their accounts, so the tax is a real flow both ways. **The base
    is the payer's own statement, not a stated proxy for it.**
- **C2** REASON — receipts **follow the economy**: they fall when income and spending fall.
- **C3** VERIFY — receipts are the sum of what was actually collected from named payers, never a rate
  applied to an aggregate the payers were never charged.

### D. The funding constraint — the core of this system
- **D1** REASON — **outlays minus receipts is the amount that must be raised, and it must be raised before
  it is spent.**
- **D2** REASON — it is raised by **issuing debt into a market that must clear**.
  - **D2.a** at whatever price the buyers are willing to pay — **the treasury chooses the size and the
    maturity, the market chooses the price.**
- **D3** FORBID — **there is no central-bank overdraft.** The treasury cannot draw on the central bank to
  cover a shortfall, directly **or by any facility that amounts to it** — including one that appears
  automatically whenever the account is negative and is cleared by the next issue.
  - **D3.a** the central bank may hold sovereign debt **bought in the market** for a policy reason — that
    is a different act, with a price and a seller.
- **D4** REASON — **issuance is managed to cover outlays**: the treasury runs a **forward-looking
  programme**, sized against what it knows it must pay.
  - **D4.a** it knows its maturity profile, so a wall is foreseeable and pre-funded.
  - **D4.b** it holds a **cash buffer**, because the alternative to a buffer is dependence on every single
    auction clearing.
- **D5** REASON — **an auction can fail**, and the failure has consequences the treasury must then handle:
  pay from the buffer, cut or defer an outlay, come back at a different size or maturity.
  - **D5.a** FORBID — **no forced buyer.** Nobody is obliged to bid, and no participant absorbs the unsold
    remainder by construction.
- **D6** VERIFY — the debt outstanding is the accumulated deficit plus rollovers, read from the register,
  and it reconciles.

### E. Debt management
- **E1** REASON — the treasury **chooses the maturity mix**, and the choice has a trade-off: short is
  cheaper when the curve is upward-sloping and rolls more often.
- **E2** REASON — it chooses **size and timing per auction**, against its cash position.
- **E3** REASON — the **cost of its debt is a consequence** of what it has issued and at what prices,
  accumulated — never a rate it sets.
- **E4** VERIFY — heavier issuance into the same demand should show up in the clearing price, and then in
  E3 with a lag; if it does not, the auction is not reading the size.

### F. The fiscal feedback
- **F1** REASON — spending is **somebody's income**.
- **F2** REASON — taxes are **somebody's outflow**, and they reduce what that party can spend.
- **F3** REASON — interest paid is **income to holders**, most of whom are domestic.
- **F4** VERIFY — the fiscal balance and the private sector's net saving move together, **as an accounting
  consequence and not as an enforced identity**.

---

## 31. THE CENTRAL BANK

### A. What it is
- **A1** REASON — the **monopoly issuer of reserves** in its currency.
  - **A1.a** which is why it can always meet an obligation in that currency, and why it can never run out.
- **A2** REASON — it has a **balance sheet, and it is a real one**.
  - **A2.a** **liabilities**: reserves, currency, the treasury's account, the deposit window.
  - **A2.b** **assets**: sovereign paper, **loans to banks as a book of dated rows**, foreign reserves,
    claims on other central banks, **swap-line draws as rows**.
  - **A2.c** VERIFY — assets equal liabilities plus its own equity, every period, and its equity includes a
    **revaluation account** for positions held in another money.
- **A3** REASON — it has a **mandate**: an objective it is trying to achieve, stated — and set by the
  parliament (§47 D4), which owns the objective and not the rate.
- **A4** REASON — it is **operationally independent of the treasury and financially owned by it**, and both
  halves have consequences.

### B. The policy rate
- **B1** REASON — it **sets** a rate, as a decision, on a rule or a judgement.
  - **B1.a** against its mandate: inflation against target, activity against capacity.
- **B2** REASON — the rate is **administered, not traded**: it is a price it declares.
- **B3** REASON — it makes the rate **effective through the corridor**, not by assertion.
  - **B3.a** FORBID — **the policy rate never appears directly as a market's cleared rate.** If the money
    market's rate equals the policy rate by construction, the corridor is decoration.
- **B4** VERIFY — the market rate tracks the policy rate **because** of B3, and the gap is information.

### C. Open-market operations
- **C1** REASON — it **buys and sells** sovereign paper, in a size **it** chooses.
  - **C1.a** the size is set by **policy**, never by an auction's weakness.
  - **C1.b** FORBID — it is **not a buyer of last resort in the primary market.**
- **C2** REASON — a purchase **creates reserves**; a sale destroys them.
  - **C2.a** it pays with money it creates, so there is no debit anywhere. That is what a central-bank
    purchase **is**.
- **C3** REASON — it is a **price-taker in the auction**: it posts a quantity, not a level.
- **C4** REASON — **reinvestment of maturities is a separate decision** from new purchases, and the
  difference is quantitative tightening.

### D. Lending to banks
- **D1** REASON — the **standing facility**: it lends against collateral at a stated rate, as a **seat in
  the money market's own session** — priced at the top of the corridor and bounded by the borrower's
  unencumbered eligible paper.
- **D2** REASON — **collateral eligibility and haircuts are its choice**, and they are a policy instrument
  in themselves.
- **D3** REASON — **the lender of last resort: freely, against good collateral, at a penalty, to the
  solvent.**
  - **D3.a** FORBID — drop any of the four and it is a subsidy. In particular **it does not lend to a bank
    that is insolvent** — that bank goes to resolution.
  - **D3.b** an overdrawn reserve account is a real overdraft, **priced** at the window rate plus a
    penalty, and it stands as the negative it is until repaid.
- **D4** REASON — it can **refuse**, and refusal must be reachable.

### E. The treasury relationship
- **E1** REASON — the treasury **banks with it**, and its account is a liability.
- **E2** FORBID — **no automatic overdraft.** An advance that appears whenever the account is empty
  converts a fiscal failure into an accounting entry.
- **E3** REASON — **remittance**: its net income goes to the treasury, because the treasury owns it.
  - **E3.a** **income, not revaluation.** An unrealised currency gain is not remitted.
- **E4** REASON — it can make a **loss**, and a loss is not remitted — it reduces its equity, and the
  treasury may have to make it good. **A loss made good the same period it occurs deletes the case the rule
  exists for**: the deferred asset must be reachable.
- **E5** VERIFY — its holding of sovereign debt is economically consolidated away and accounting-wise is
  not, and both statements must remain true of the books.

### F. Foreign exchange
- **F1** REASON — it holds **reserves in other currencies**, and they are real assets.
- **F2** REASON — it can **intervene, bounded by F1** — and a bank at zero cannot defend anything.
  - **F2.a** which is why a peg breaks: the constraint is real, not a rule.
- **F3** REASON — its foreign claims **revalue**, into A2.c's revaluation account.
- **F4** REASON — **claims on other central banks** from cross-border settlement are bilateral and sum to
  zero across the world.

---

## 47. THE POLITY

*Numbered 47 and placed here: numbers are the citation grammar and never move, so a system added
after the document was written takes the next number; it lives in Part VII because it is the public
sector's third institution — the one that owns the other two's POLICY primitives.*

### A. What it is
- **A1** REASON — a **parliament of a fixed number of seats**. The seat count is a POLICY primitive of the
  constitution — the one number here that no mechanism produces — and it is declared once.
- **A2** REASON — a **few parties**, each with a **platform**: a stated position on every POLICY primitive
  the parliament controls — a tax rate on each base, a transfer rate, the size and composition of the
  outlay programme, a regulatory ratio. A platform is **data**, not code.
  - **A2.a** the platforms **differ**, and the difference is load-bearing: two parties with one platform
    are one party, and an electorate offered one platform has no decision to make.
  - **A2.b** a party is not a party in the ledger's sense. It holds no account and no register; it is a
    named platform with a seat count. Campaign money, membership and donation are absent, and said so.
- **A3** REASON — the **electorate is the household cells**. A cell casts `weight` votes, all the same
  way, because a cell is one possible household with a multiplicity (XI-15) and one possible household
  has one vote.
- **A4** REASON — an **election** every stated number of periods — the **term**, a POLICY primitive of the
  constitution — placed on the one calendar (Money G3) by date.

### B. The vote
- **B1** REASON — a cell **votes from its own state and its own outlook** (§46): its expected income, its
  confidence, its employment state, the prices it has paid, what it owns and what it owes.
  - **B1.a** FORBID — **no vote from an aggregate.** A cell that voted on the published unemployment rate,
    the published inflation rate or a sentiment index would be voting on a number it did not experience.
    What it experienced is its own wage, its own prices, its own job — and the published statistic reaches
    it only as an observer's aggregate, with the lag and revision a statistic has (§45 A5).
- **B2** REASON — it votes for the platform that **leaves it best off by its own expectation**: each
  platform applied to the cell's own state, at the cell's own outlook, and the one with the highest
  expected position takes the votes.
  - **B2.a** FORBID — **no turnout parameter, no swing parameter, no vote share drawn.** A vote is an
    outcome of a decision, and a share stated for one is law 2's outcome seeded.
  - **B2.b** REASON — **abstention is a decision**, not a rate: a cell whose expected position is the same
    under every platform has nothing to vote about, and stays home. That is the only abstention there is.
- **B3** REASON — the vote is evaluated **per cell and summed weighted** (XI-15): `Σ vote(xᵢ)·wᵢ`, never
  a sector's mean voter.
- **B4** VERIFY — a **mean-preserving spread of household incomes changes the seat count** while the
  weighted mean does not move. A polity whose result depends only on the mean is one voter wearing a
  parliament.

### C. Seats and the mandate
- **C1** REASON — seats are **allotted in proportion to votes by one stated rule** — a POLICY primitive of
  the constitution, and the whole of the electoral system.
- **C2** REASON — a **government** is a coalition holding a majority of seats, formed by one stated rule:
  the largest party adds the party whose platform is nearest its own until it holds a majority.
  - **C2.a** a parliament in which no such coalition exists — every platform too far from every other —
    is a **hung parliament**, and the standing mandate continues. That is a real outcome and it is
    reported, not repaired.
- **C3** REASON — the **mandate is a read of the parliament**: for every POLICY primitive the parliament
  controls, the seat-weighted position of the governing coalition's platforms.
  - **C3.a** FORBID — **no policy set directly.** A POLICY primitive the parliament controls changes by a
    mandate and by nothing else — not by a scenario, not by a schedule, not by a hand on a dial.
  - **C3.b** VERIFY — every such primitive's value in the register **equals the standing mandate's**,
    every period, exactly.
- **C4** REASON — a mandate takes effect from a stated period after the election and is a **journaled
  event** with a date and named subjects (§45 B1, B3): which parties, how many seats, what changed.

### D. What the parliament controls, and what it does not
- **D1** REASON — **fiscal**: the rates on each tax base (§30 C1), the transfer rates (§30 B1), the cash
  buffer the treasury holds (§30 D4.b).
- **D2** REASON — **spending**: the size and composition of the outlay programme — purchases, transfers,
  public wages (§30 B1). *Policy* is the third of B3's *"causes that vary"*, and this is where it varies.
- **D3** REASON — **regulatory**: the ratios and floors that are POLICY primitives in the register — a
  capital ratio, a reserve requirement, a haircut floor, a replacement rate — and nothing else.
  - **D3.a** FORBID — **the parliament never sets a price, a quantity or an outcome.** Law 2 admits three
    kinds of primitive, and the parliament owns the third. A mandate that named an interest rate, a wage,
    an exchange rate or a growth target would be a written path with a majority behind it.
- **D4** REASON — the **central bank's mandate text and its target are the parliament's** (§31 A3); its
  **rate is not** (§31 A4: operationally independent). A parliament that set the policy rate would
  delete the corridor's reason to exist.
- **D5** REASON — a **primitive has an owner**. Every POLICY primitive in the register names who sets it —
  the parliament, the central bank, or a standard-setter — and the register prints the owner beside the
  value.

### E. Consequences
- **E1** REASON — the treasury's programme **reads the mandate**: its outlays are the mandate's outlays and
  its receipts the mandate's rates applied to real bases with named payers (§30 B–C). The programme does
  not read the election; it reads the numbers the election produced.
- **E2** REASON — a change of mandate reaches everything else **through the mechanisms** — the need, the
  auction, the curve, the cost of capital — and never directly.
- **E3** VERIFY — a change of government shows in the deficit within a stated horizon, **through named
  outlays and named receipts**, party by party.
- **E4** REASON — an election is an **event** on the observer surface (§45 B1), generated from the state,
  and news of it causes nothing (§45 B2.a): the mandate is what changes behaviour, when it takes effect.

### F. What must not happen
- **F1** FORBID — **no exogenous election result.** A parliament written into a scenario is §45 E2's
  scripted narrative at the level of the state.
- **F2** FORBID — **no policy path.** A tax rate that follows a series has had every vote cast for it.
- **F3** FORBID — **no party that votes for a household.** Every vote is a cell's own, from its own state;
  a bloc, a demographic weight or a loyalty coefficient is a decision evaluated at an average (§41 A2.f).
- **F4** FORBID — **no approval rating as an input.** What the electorate thinks of the government is a
  read of how the cells would vote today, published with a lag like any statistic, and it causes nothing.
- **F5** VERIFY — the polity's aggregate — seats, mandate, the deficit it produces — is the sum of its
  cells' votes and its named flows, computed from members, never a target they were scaled to.

---

# PART VIII — FIRMS

---

## 32. FIRM FUNDAMENTALS

### A. What a firm is
- **A1** REASON — a **named party** with an account, and a register of what it owns and what it owes.
- **A2** REASON — it exists in a **region** and a **sector**, and both are load-bearing: the region fixes
  its money and the sector fixes what it buys and sells.
- **A3** REASON — firms are **heterogeneous in size, cost and leverage**, and the dispersion is the reason
  markets exist among them.
- **A4** REASON — it has an **owner or owners** whose claim is the residual.

### B. The operating flow
- **B1** REASON — **revenue**: quantity sold times price achieved, from named buyers.
  - **B1.a** it is a **consequence of a market**, never a growth rate applied to last period.
  - **B1.b** and its **baseline for comparison is its own measured history**, never a compounded path.
- **B2** REASON — **input costs**: what it bought, at prices it paid.
- **B3** REASON — **labour costs**: headcount times wage, paid to named households.
- **B4** REASON — **operating profit is the residual** of B1 minus B2 minus B3, and it can be negative.
  - **B4.a** and the **margin is a read**, never a target the revenue was fitted to. **A cost line struck
    at the seed as the gap to a chosen margin, and then applied as a fixed share of revenue for ever, makes
    that margin an attractor and makes the cost base perfectly variable by construction.**
  - **B4.b** every cost line is a **named line with a real payee**.
- **B5** REASON — **fixed and variable costs differ**, which is why margin moves more than revenue —
  operating leverage is a consequence of the cost structure, not a coefficient.
- **B6** VERIFY — every cost is somebody's income and every revenue is somebody's outlay, party by party.

### C. The balance sheet
- **C1** REASON — **assets**: cash, receivables, inventory, fixed capital.
- **C2** REASON — **liabilities**: payables, bank debt, bonds.
- **C3** REASON — **equity is the read**, C1 minus C2, and it can be negative.
- **C4** REASON — **working capital is a real use of cash**: inventory bought and not yet sold, invoices
  sent and not yet paid.
  - **C4.a** so **profit and cash are different numbers, and the difference is where firms die.**
  - **C4.b** reported receivables and payables are the **sum of the actual invoice book**, not a ratio of
    revenue. Two representations of one thing, with the decision reading the stated one, is law 4's defect
    at the point it matters most.

### D. Cash and solvency
- **D1** REASON — it pays out of a **balance**, and the balance can hit zero.
- **D2** REASON — **debt service is a fixed claim ahead of the owners**: interest and principal.
- **D3** REASON — **coverage** — operating cash against debt service — is a read, and it is what lenders
  look at.
- **D4** REASON — it can **fail two ways**: no cash to pay something due, or liabilities exceeding assets.
  They are different, and a firm can be either without the other.
- **D5** REASON — when it cannot pay, it **defaults**.

### E. What it decides
- **E1** REASON — **price and quantity** it offers, and **which lines it is in** — a line can be exited when
  it neither produces nor sells, and its share redistributes to the firm's other lines.
- **E2** REASON — **how many people to employ**.
- **E3** REASON — **how much to invest**.
- **E4** REASON — **how to fund itself**: retained cash, debt, or new equity — and the choice depends on
  what each costs.
  - **E4.a** its **leverage target is the management's own**: the lender's covenant line moderated by the
    management's risk aversion, approached at the management's own horizon, and **the money raised is
    raised into an actual investment programme** — a firm with no programme raises nothing. A management
    above its target pays down toward it whatever debt costs.
- **E5** REASON — **what to pay out**: dividends and buybacks, real cash to owners.
- **E6** REASON — every one of E1–E5 is made from the firm's **own state and the prices it faces**, which
  is what makes the aggregate a consequence.
- **E7** REASON — the management **publishes an expectation** — what it expects to deliver — and is then
  judged against it. The expectation is its own adaptive read of its own earnings (§46 C2), and the surprise
  against it is a real event. It is never a choice among written phrases.
  - **E7.a** for a firm whose shares trade, §48 is where this happens: the calendar it publishes on, the
    report that settles it, and the banks that publish estimates of the same lines. A firm that is not
    public still forms the expectation and still takes the surprise; what it does not do is publish.

### F. What must not happen
- **F1** FORBID — **no revenue without a buyer** and no cost without a payee.
- **F2** FORBID — **no exogenous earnings path.** A firm whose profit follows a series has had every
  decision in E made for it.
- **F3** FORBID — **no firm that cannot run out of cash.**
- **F4** VERIFY — the sector aggregate is the sum of its firms, computed from members, never a target they
  were scaled to.

---

## 33. THE CAPITAL PROGRAMME

### A. What capital is
- **A1** REASON — a **stock of productive assets** held by a named firm.
- **A2** REASON — it **produces**: capacity is a function of the stock, and output is limited by it.
- **A3** REASON — it **depreciates**: it wears out, and the depreciation is **a real cost against profit and
  a real reduction in the stock — one schedule, charged in both places.** A charge struck as a share of
  revenue means a firm that doubles its plant takes no extra charge, and capacity is free on the income
  statement and on the tax base built from it.
- **A4** REASON — it is **specific**, and specific **in kind**: capital of one kind is not capital of
  another, and a use that needs several kinds is limited by the scarcest of them. Which is why
  misallocation is possible and costly, and why capital acquired for one purpose is worth less to another.
  - **A4.a** what a firm's plant is **made of** is a property of its industry, not one basket shared by
    every buyer in the world.
  - **A4.b** a capital good has a **useful life of its own** — and the presence of a life is what makes a
    good a capital good.
  - **A4.c** whether a purchase is plant, an input or an expense is the **buyer's** question, not the
    good's: what the buyer's own recipe consumes is an input, what has a life is plant, the rest is
    operating cost.
- **A5** REASON — its **value** is what it can produce, and it can be written down when that falls.
- **A6** REASON — the stock is a set of **dated vintages**, each with its own cost, its own service date,
  its own life and its own kind. A vintage **leaves the register when fully worn**, so the charge stops when
  the plant is gone. Gross, net, accumulated depreciation and the period's charge are **reads** over the
  vintages.
  - **A6.a** every movement of plant between two parties is an ordinary transfer with two named sides — a
    sale from an estate, a spin-off's carve-out, a merger, a resolution — and every non-movement
    (commissioning, wearing out, scrapping, abandonment) is a recorded transformation on the holder's own
    book.
  - **A6.b** VERIFY — the change in gross plant equals commissioned minus retired minus scrapped minus
    abandoned plus transfers in minus transfers out, per firm, every period; and the same for capital that
    has arrived and is not yet in service.

### B. The decision
- **B1** REASON — a firm invests when it **expects the return to exceed its cost of capital**.
  - **B1.a** the **return** comes from expected demand and price.
  - **B1.b** the **cost of capital** comes from the markets: what its debt costs **at the margin, now** —
    not the average coupon on debt already outstanding — and what its **equity** costs.
  - **B1.c** so a change in a market price **changes real investment, and that is the transmission channel
    this whole document exists to make real.**
  - **B1.d** the **hurdle and the horizon are the management's own**, read off its risk aversion and its
    patience — which is what a growth-against-margin orientation actually is, rather than a stated posture.
- **B2** REASON — it must also be able to **fund it**: cash on hand, borrowing capacity, or new equity.
  - **B2.a** **a firm with a good project and no funding does not invest**, and that is a credit constraint
    doing real work.
- **B3** REASON — **capacity utilisation is a reason**: a firm running full has an obvious reason to expand,
  one running empty does not.
- **B4** REASON — **uncertainty delays it**: the option to wait has value when the spend is irreversible.
- **B5** FORBID — **no investment rate.** A fixed fraction of profit or output reinvested, however many
  multipliers are applied to it, is B1 deleted — and with it every link from finance to the real economy.

### C. The spend
- **C1** REASON — investment is a **purchase from a named seller** — a capital-goods producer — and it is
  that seller's revenue.
  - **C1.a** so investment is **demand now and capacity later**, and the two effects arrive at different
    times.
- **C2** REASON — it is **paid for in cash**, out of an account, in a currency.
- **C3** REASON — there is a **lag** between spend and capacity: the asset is built, then it works.
- **C4** REASON — it is **irreversible**: the money cannot be recovered by cancelling.

### D. The stock over time
- **D1** REASON — capital next period equals capital now plus investment minus depreciation, per firm.
- **D2** REASON — the **aggregate stock is the sum over firms**, and it changes slowly relative to prices.
- **D3** REASON — a firm that fails leaves its capital to **somebody named** — sold into a market with real
  bidders, at what they will pay, not at a formula discount to book.
- **D4** VERIFY — capacity, output and utilisation must reconcile: a world producing more than its capital
  allows has capacity from nowhere.

### E. What it feeds back into
- **E1** REASON — investment is a **large, volatile component of demand**, so its swings drive the cycle.
- **E2** REASON — it **employs people** to build the capital.
- **E3** REASON — it is usually **debt-funded at the margin**, so it drives credit growth.
- **E4** VERIFY — a tightening in credit conditions should reduce investment through B1.b and B2.a, **and
  then output — through the chain, with the lag in C3, and never directly.**

### F. Entering a line
- **F1** REASON — a firm that does not yet make a good **can enter that line**, and entering it is an
  investment project like any other: plant for a good it does not yet make, commissioned through the same
  build lag, posting nothing until the plant is in service, and its revenue share is what it then sells.
  - **F1.a** entry and exit together are what make a sector's line-up an **outcome**.
  - **F1.b** until entry exists, a firm touching a line for the first time must be handed an opening stock
    of work in progress at its own running cost, **and that opening stock is a stated stand-in for exactly
    this mechanism** — not a fact about the world.

---

## 34. FIRM BIRTH AND DEATH

### A. Birth
- **A1** REASON — a **new named party** appears, with an identity that is new and not reused.
- **A2** REASON — it is **funded by somebody**: founders' money, an investor, a lender — and the money comes
  out of a named account.
  - **A2.a** FORBID — **no firm born with an endowment from nowhere.** That includes its plant: a greenfield
    build **buys** its plant from a producer.
- **A3** REASON — it starts with a **balance sheet that balances** and usually very little.
- **A4** REASON — entry happens **for a reason**: profitability in a sector, available funding, demand not
  being met.
  - **A4.a** so entry is a **consequence** of conditions, never a birth rate.
  - **A4.b** and **its opening size is what its founders can fund**, against the opportunity — never a
    constant share of anything.
- **A5** REASON — it enters a **market as a competitor**, which changes what incumbents face.

### B. Life and distress
- **B1** REASON — it is subject to firm fundamentals from the first period.
- **B2** REASON — a young firm is **more fragile**: less cash, no track record, worse credit terms — **and
  its age is read by the things that price it.** An age written and never read is not a property; an
  assessment whose volatility measure returns its best score for a firm with no history scores a newborn
  best, which is backwards.
- **B3** REASON — distress is **observable before default**: coverage falling, cash falling, spreads
  widening.
- **B4** REASON — a distressed firm **acts**: cuts costs, sells assets, raises expensive money, approaches
  its lenders, exits a line — and each act is a real transaction.

### C. Default
- **C1** REASON — a **stated, observable definition** — a payment missed, a covenant breached.
- **C2** REASON — it is a **consequence of the firm's state**, never a draw from a default probability.
  - **C2.a** FORBID — **no exogenous default event.** A default assigned by a hazard rate cannot be
    prevented by a firm's actions or caused by a market's, which deletes B4 and every credit channel here.
- **C3** REASON — it **triggers things**: a credit event, the lenders' loss, a rating action.
- **C4** VERIFY — every default is traceable to the cash or solvency failure that caused it.

### D. Resolution — where everything goes
- **D1** REASON — the firm's **assets are realised**: **sold to named buyers at cleared prices**, or taken
  over as a going concern. Its inventory is offered into the same market its goods always sold in; its plant
  is offered to real bidders who value it for what it can produce **for them**; what nobody buys by the
  programme's last period is abandoned or perishes, and that is an outcome too.
- **D2** REASON — the proceeds are **distributed by seniority**: secured lenders, senior, subordinated,
  equity last.
  - **D2.a** and the **recovery is what the assets actually fetched**, divided as D2 says — never a
    percentage.
  - **D2.b** **trade creditors rank**, with the other unsecured claims. An estate that **collects** the
    dead firm's receivables as an asset while its own trade creditors rank nowhere biases every recovery
    upward by exactly that asymmetry.
  - **D2.c** a **counterparty's close-out claim** ranks as an unsecured claim, not ahead of the waterfall.
- **D3** REASON — **losses land on named holders** in the register, in proportion.
- **D4** REASON — its **employees lose their jobs**, its **suppliers lose their receivables**, and its
  **capital goes to a buyer**.
  - **D4.a** these are the real-economy consequences, and they are what makes a default cost more than the
    credit loss. **A death drops a headcount only through the labour market's own separation path.**
- **D5** REASON — the party then **ceases to exist**, and every reference to it must resolve to the estate
  or the successor. **A dead firm with no claims still opens an estate**; it does not keep its cash for
  ever.
- **D6** VERIFY — recoveries plus losses equal the firm's assets at realisation, exactly. **Money is not
  destroyed by a default; it is transferred and revalued.**
  - **D6.a** a residual left on a dead party is a defect, and it must be found and paid away **in every
    currency the party held**.

### E. What must not happen
- **E1** FORBID — **no firm that cannot die.**
- **E2** FORBID — **no death without a destination** for every asset, liability, employee and contract.
- **E3** FORBID — **a constant population.** If births exactly offset deaths by construction, A4.a and C2
  have both been overridden.
- **E4** VERIFY — the population, its age distribution and its sector mix are all reads, and they should
  move with conditions.

---

## 35. M&A AND CORPORATE CONTROL

### A. What a deal is
- **A1** REASON — an **acquirer**, a **target**, and a **price per share that the target's owners accept**.
- **A2** REASON — the ownership of the target **transfers in the register**, and the target's shareholders
  are **paid**.
- **A3** REASON — the consideration is **cash, shares, or both**, and the choice matters:
  - **A3.a** cash needs funding and increases the acquirer's leverage;
  - **A3.b** shares dilute the acquirer's existing owners, which is a real cost to them.
- **A4** REASON — after it, the **two balance sheets combine**, and the combined firm is one party.
- **A5** REASON — the target's **debt does not disappear**: it is repaid, assumed, or triggered by a
  change-of-control term.

### B. Why an acquirer bids
- **B1** REASON — it thinks the target is **worth more to it** than the market price: cost savings, market
  position, an asset it wants. **The intent is the acquirer's own valuation** — the target's expected
  earnings discounted at the acquirer's own hurdle, against the price — formed by its management, never a
  screening threshold.
- **B2** REASON — the **premium is what it must pay** to get the owners to sell, and it is therefore a
  cleared price like any other.
  - **B2.a** a target's owners can **refuse**, and a bid can fail — a real outcome with consequences for
    both prices.
- **B3** REASON — it must be able to **fund it**, so the credit market decides which deals happen.
- **B4** REASON — a **competing bidder** can appear, and then the price is contested, which is the auction
  working.
- **B5** FORBID — **no deal by assignment.** A merger that happens because a rule fired has no bidder, no
  premium, no acceptance, no rival and no funding constraint — it is a coin flip with a price attached.

### C. Why a target's owners sell
- **C1** REASON — the **price beats holding**, on their own valuation.
- **C2** REASON — they are **dispersed**, so each decides individually and the outcome is the aggregate of
  those decisions.
- **C3** REASON — **management may resist**, and its interests differ from the owners' — which is the
  corporate-control problem and the reason takeovers discipline firms at all.
- **C4** VERIFY — a firm trading cheap relative to what a buyer would pay should attract bids, as a
  consequence; if it never does, the discipline channel is absent.

### D. After
- **D1** REASON — the combined firm's **cash flows are the sum**, plus whatever the acquirer claimed it
  could change — **and whether that materialises is measurable.**
- **D2** REASON — the acquirer's **leverage is higher** if it paid cash, and its credit is reassessed.
  - **D2.a** which is why an acquirer's bonds can fall on the day its shares rise, and both are correct.
- **D3** REASON — the target's **shares cease to exist** as a separate instrument, and any index containing
  them changes.
- **D4** REASON — **employees, suppliers and customers carry over to a different owner.** An acquired firm
  is **not** a dead firm: its obligations are assumed, not written off.
- **D5** VERIFY — the money paid to target shareholders equals what the acquirer and its lenders put up,
  exactly, and it lands in named accounts.

### E. What must not happen
- **E1** FORBID — **no acquisition without payment.**
- **E2** FORBID — **no target that disappears without its liabilities being addressed.**
- **E3** FORBID — **no synergy that is assumed into the cash flows** without showing up as real revenue or
  real cost — and no headcount saving that deletes workers without a separation event.

---

## 36. TRADE CREDIT

### A. What it is
- **A1** REASON — a **sale delivered now and paid later**: the seller has a **receivable**, the buyer a
  **payable**, and they are the same obligation from two sides.
- **A2** REASON — both sit on real balance sheets.
- **A3** REASON — it has **terms**: how long, and often **a discount for paying early** — which makes the
  discount an **implicit interest rate and therefore a price**. Without it there is no rate, and no
  factoring market can exist.
- **A4** REASON — it is **unsecured credit extended by a supplier**, and the supplier decided to extend it.

### B. Why it exists
- **B1** REASON — the **buyer needs to sell what it bought before it can pay**, and the terms bridge that
  gap.
- **B2** REASON — the **seller wants the sale**, and offering terms is a way to compete.
- **B3** REASON — the seller often **knows the buyer better than a bank does**, and can enforce better by
  threatening to stop shipping.
- **B4** REASON — it is **cheap or free at the point of use**, which is why firms use it first and bank
  credit second.
- **B5** REASON — the seller **decides** whether to offer it, **per buyer, on that buyer's condition** — and
  **it tightens terms when it is worried**, which is a real credit tightening with no bank involved. Terms
  that are a formula cannot tighten, and the mechanism is absent.

### C. The flow
- **C1** REASON — the goods move at one time, the money at another.
  - **C1.a** so revenue and cash receipt are **different periods**.
- **C2** REASON — the receivable is an **asset that can be financed**: pledged, factored, or sold to a bank
  at a discount — which turns it into bank credit.
- **C3** REASON — payment, when it comes, is a **real settlement between two named parties**.
- **C4** VERIFY — receivables sum to payables, across the whole world, exactly. **This is the cheapest
  possible check on the system existing at all.**

### D. Failure — why it matters
- **D1** REASON — a buyer can **pay late**, and **lateness is a real state** that stresses the seller's
  cash. A world in which nothing is ever late has no such state.
- **D2** REASON — a buyer can **fail**, and then the receivable is a **claim in the estate**, ranking with
  other unsecured creditors.
  - **D2.a** so the seller takes a real loss it did not choose, from a party it is not a lender to on paper.
- **D3** REASON — the loss can **push the seller into distress**, and its own suppliers then take losses — a
  chain that runs **along the supply network and not through the banking system**.
  - **D3.a** VERIFY — this contagion path must be **emergent from D2**, traceable firm to firm.
- **D4** REASON — the anticipation of D2 makes suppliers **withdraw terms** from a firm they doubt, which
  starves it of working capital faster than any lender could.
  - **D4.a** which is **how a solvent firm dies of a rumour**, and it is a real mechanism.
- **D5** REASON — the smallest firms live on trade credit more than anyone, so a tier that pays cash is the
  tier where this system is most conspicuously absent.

### E. What must not happen
- **E1** FORBID — **no sale that settles instantly by construction.** If every transaction pays on delivery,
  this entire system is absent and its failure channel with it.
- **E2** FORBID — **no receivable without a named payer.**
- **E3** FORBID — **no receivable that survives its debtor's death.** It resolves into a recovery or a loss.

---

# PART IX — THE REAL ECONOMY

---

## 37. GOODS

### A. What a good is
- **A1** REASON — a good is a **sub-unit of an industry with a physical unit of measure**.
- **A2** REASON — it is produced from inputs by a **recipe**.
  - **A2.a** **fixed input quantities per unit of output — a Leontief recipe, no substitution.** Chosen
    deliberately, for one reason and with one cost stated: **fixed coefficients make an input shortage bite
    as a real production constraint rather than being smoothed away by a substitution elasticity nobody can
    observe** — which is what makes a supply shock transmit at all. The cost is that a firm facing an
    expensive input cannot economise on it, so **substitution is a MISSING mechanism here, not an
    assumption away**: if a relative-price response is wanted later it is a new mechanism, not a parameter.
  - **A2.b** FORBID — **a recipe is not a value share.** A recipe expressed as cost per unit of revenue,
    with the physical draw computed as money needed divided by the input's price, means **a price doubling
    halves the physical draw** — the strongest substitution assumption there is, sitting exactly where the
    model chose no substitution at all, and invisible because it reads as an ordinary units calculation.
  - **A2.c** plus labour, plus capital services.
- **A3** REASON — storable or perishable, as a property of the good.
- **A4** REASON — homogeneous within its sub-unit; otherwise it is a different sub-unit.

### B. Production
- **B1** REASON — **a firm has a production DECISION with reasons: expected demand, its margin, its
  capacity, its inputs, its labour. The quantity is the OUTCOME.**
  - **B1.a** capacity is one of the reasons, and binding capacity is a real state.
  - **B1.b** inputs on hand are another, **and a shortage is a real state that reaches the decision** — a
    constraint computed and read by nobody is not a constraint.
  - **B1.c** labour available is another.
  - **B1.d** VERIFY — utilisation is a **read of the outcome against capacity**, never an input to it.
- **B2** REASON — **production consumes the inputs it consumes.** The consumption is the physical
  consequence of B1's decision, and the recipe says how much — never a separately chosen number.
- **B3** REASON — **work in progress** exists between input and output, owned by somebody, and it carries
  **what it cost**.
- **B4** REASON — **yield**: not everything started is finished. Scrap is real and it is a loss — of
  **units**, at the point they would have been made. What survives is dearer per unit than what was started,
  because normal waste is absorbed into the cost of the survivors.
- **B5** REASON — unit **cost** equals inputs consumed plus wages plus a capital charge.
  - **B5.a** **no units, no capitalised cost.** A period in which the line started nothing capitalises
    nothing; the firm still incurs the cost, and it is a period expense.
  - **B5.b** a **throttled** period is different: a line's whole cost over a smaller batch **is** a higher
    unit cost, and that is what running a plant below its rate does.

### C. The market
- **C1** REASON — sellers offer quantities; buyers post the most they will pay.
- **C2** REASON — **a price clears per (good, market, period)**, and the price is **stored as the market's
  print** so that next period can re-mark against it. A price computed and discarded cannot value anything
  the period after.
- **C3** REASON — buyers are heterogeneous and bid for their own reasons: firms buying inputs, households
  consuming, government procuring, foreign buyers, **and the estates of dead firms selling**.
- **C4** REASON — **rationing** when demand exceeds supply, by a rule stated once.
- **C5** REASON — **unsold output stays with the seller.** Illiquidity in goods is unsold stock.
- **C6** REASON — the price is in the **seller's** currency; a foreign buyer converts, **by buying the
  seller's money from somebody**.

### D. Delivery and logistics
- **D1** REASON — goods move physically from seller to buyer.
- **D2** REASON — that takes **time** and costs **money**.
- **D3** REASON — a **carrier** is a named party that earns the freight.
- **D4** REASON — **landed cost** equals ex-works price plus freight plus duty.
- **D5** REASON — goods **in transit** are owned by somebody and sit on somebody's book.

### E. Inventory
- **E1** REASON — a holder's stock is a **quantity of units**, and it is held as **lots, each with what it
  cost**.
- **E2** REASON — **carried at the LOWER OF COST AND NET REALISABLE VALUE.** That is the actual accounting
  rule: cost until the market falls below it, then written down to market, **and the write-down is a charge
  to income in the period it happens.**
  - **E2.a** a write-down is **not reversed** beyond the original cost — an unrealised holding **gain** on
    ordinary inventory is not recognised.
  - **E2.b** the exception is real and narrow: **a commodity broker-dealer carries at fair value through
    income**, because for it the inventory **is** the position.
  - **E2.c** FORBID — **inventory is never carried at market when market is above cost** for a
    non-broker-dealer. Marking it up invents profit the firm has not earned; a warehouse revalued upward
    when the market rises and downward when it falls, with neither move booked as an event, is this defect
    in both directions at once.
- **E3** REASON — the **holding loss is an event with a date, a size and an income line** — the asymmetry in
  E2/E2.a is the mechanism, not an approximation of one.
- **E4** REASON — **spoilage, obsolescence and shrinkage remove units without a sale**, at the lot's own
  cost per unit, recorded so the units identity can see them.
  - **E4.a** a **storage fee** and a **spoilage rate** are two different things and must not be summed into
    one number: one is cash paid to whoever stores the goods, the other is units that perish. A distributor's
    margin covers both; a charge to the holder is the fee alone.
- **E5** REASON — **cost flows out first-in-first-out or by weighted average; last-in-first-out is not
  permitted.** The choice is disclosed and applied consistently. It changes reported profit and the
  balance-sheet carrying value in opposite directions when prices move, so it is a real decision with a real
  consequence.

### F. The cash legs
- **F1** REASON — the buyer pays the seller, by name.
- **F2** REASON — **payment terms**: immediate, or trade credit with a due date.
- **F3** REASON — trade credit is a **loan between two named firms**, and it can go bad.
- **F4** REASON — the freight is paid **to the carrier**.
- **F5** REASON — **revenue is recognised on delivery; cost of goods sold is the units that left**, valued
  per E5.
  - **F5.a** so the income statement charges **what it sold**, not what it drew. Costs that no batch
    absorbed — an idle line's payroll — are **period costs**, and that is what makes idle capacity
    expensive. A firm that produces and does not sell **carries the cost in its stock instead of charging
    it**, which is what absorption means.
  - **F5.b** FORBID — **one cost in two places.** A production cost capitalised into a batch **and**
    expensed in the period it was incurred is counted twice.

### G. The aggregate
- **G1** REASON — **producer prices and consumer prices are different indices over different baskets.**
  - **G1.a** **producer prices** — prices received by domestic producers at the factory gate, weighted by
    production. Excludes freight to the buyer, excludes distribution margin, excludes consumption tax, and
    **includes intermediate goods a household never buys.**
  - **G1.b** **consumer prices** — prices paid by households for final consumption, weighted by household
    expenditure. Includes freight, the channel's margin and consumption tax, includes imports at the price a
    household actually pays, and **excludes intermediate goods entirely.**
  - **G1.c** VERIFY — the two therefore **diverge**, and the gap is economically meaningful: it is the
    distribution wedge plus tax plus the import/export mix. **A model with one index and two names cannot
    show margin compression, which is most of what a cost shock does to a firm.**
- **G2** REASON — inflation is the change in the **relevant** index, and which one is always stated.
- **G3** REASON — real and nominal output are distinguishable, deflated by the **right** index.
- **G4** VERIFY — capacity utilisation is a read.

---

## 38. FREIGHT AND LOGISTICS

### A. What is bought
- **A1** REASON — a **service: moving a quantity from one place to another, over a time.**
- **A2** REASON — it is bought by a **named shipper** from a **named carrier**, at a price, in a currency.
- **A3** REASON — it has a **duration**, so goods are **in transit** — owned by somebody, not yet where they
  are needed.
  - **A3.a** in-transit inventory is a real asset on a real balance sheet and a real use of working capital.
- **A4** REASON — the price is **per unit per route**, and **routes are distinct**: capacity on one is not
  capacity on another.

### B. Capacity and supply
- **B1** REASON — a carrier owns **capital**: ships, trucks, planes, warehouses — with their own lives.
- **B2** REASON — capacity is **fixed in the short run** and expensive and slow to add.
  - **B2.a** so the freight price is **extremely inelastic in the short run**, which is why it moves
    violently and why it is a good early indicator of a real imbalance.
- **B3** REASON — it has an **operating cost**: fuel, labour, and the capital charge.
- **B4** REASON — capacity can be **lost or blocked**: a disruption is a **real reduction in units moved**,
  not a price shock. **A route can be blocked.**

### C. Demand
- **C1** REASON — demand is **derived**: it exists because somebody is trading goods.
  - **C1.a** so it is not an independent market — it moves with trade volumes, and that dependence is the
    point.
- **C2** REASON — a shipper can **not ship**: hold the goods, source locally, or not trade at all, and that
  substitution is what caps the freight price.
- **C3** VERIFY — freight demand should equal the volume actually moving between locations, **read from the
  trades**, never a separate series.

### D. The price and what it does
- **D1** REASON — it **clears** per route.
- **D2** REASON — the freight cost is **part of the delivered price** of the good, so it flows into the
  buyer's cost and the seller's margin.
- **D3** REASON — it is therefore the **mechanism behind the location basis**: the same commodity priced
  differently in two places, with the gap bounded by what it costs to move it.
  - **D3.a** and the arbitrage that enforces the bound is **somebody actually shipping**, with capacity and
    money — so when capacity binds or is blocked, the basis widens and stays wide.
- **D4** REASON — the **transit time** is a real lag between a purchase and a delivery.
- **D5** VERIFY — the price gap between two locations should track the freight price on the route, and a
  persistent divergence is a finding about capacity, not a number to close.
- **D6** REASON — **capacity rations quantity, not only price.** A route's fill must be able to turn
  shippers away; a fill ratio computed and read by nobody means capacity sets the price of distance and
  never the quantity of it.

### E. What must not happen
- **E1** FORBID — **no instantaneous, costless transport.** That collapses every location into one and
  deletes D3 entirely.
- **E2** FORBID — **no shipment without capacity**, and no capacity without a carrier that owns it.
- **E3** FORBID — **no goods in transit owned by nobody.**

---

## 39. LABOUR

### A. What is traded
- **A1** REASON — **hours of a person's time**, supplied by a named household to a named firm.
- **A2** REASON — the price is the **wage**, per unit of time, in a currency.
- **A3** REASON — labour is **heterogeneous**: skill, sector, region — and a job in one is not a job in
  another.
  - **A3.a** which is why **unemployment and vacancies can be high at the same time**, and a single
    homogeneous labour market cannot produce that.
  - **A3.b** and why **supply moves to where the vacancies are**: what one occupation's own search left
    unmatched can be spread over what the others left unfilled, matched through the same matching function,
    capped on both sides. Movers **enter the new occupation at the bottom** — retraining has a cost, and
    that cost is the entry wage. This is slower than own-occupation search by construction, and it is a
    **flow of people**, never a coefficient reading a wage gap.
- **A4** REASON — the relationship **persists**: employment is a **state**, not a per-period trade, which is
  what makes hiring and firing **decisions** rather than continuous adjustment.
  - **A4.a** so there is an **employment relationship**, not a headcount: a register of employment — a firm,
    a worker or cohort, a wage, a start date — from which the wage bill, the unemployment rate and the
    separation flow are **reads**. Without it a hire and a separation are additions to a count: there is
    nothing for stickiness to be a consequence **of**, nothing a severance payment could sever, and no
    household that can be told its earner lost a job.
  - **A4.b** where the worker is a **cohort** (XI-15), the relationship carries a **headcount**, and a
    hire or a separation moves an integer number of people between a firm and a named household cell —
    never a fraction of one, and never a share of a sector. The cohort is a party that can be told.
  - **A4.c** a hire or a separation that applies to **part** of a cell **splits the cell** (XI-15): the
    hired members become a cell of their own, with their own employment row, and the row's headcount is
    that whole cell's weight. A cell with some members employed and some not is two populations wearing
    one state, which A2.d forbids one system over — and it is why B5's *"employed plus unemployed plus
    inactive equals the population, exactly"* is checkable rather than approximate.

### B. The supply side
- **B1** REASON — a household **decides** whether to work and how much, given the wage and its alternatives.
  **Participation is a decision with the wage in it**, not a constant keyed off a regime label.
  - **B1.a** and a matched seeker **accepts or refuses**: it takes nothing below its own outside option —
    the benefit this world already pays it, as a share of the going rate. In a slack market the print falls
    to that and no further; in a tight one the bids set it.
- **B2** REASON — the **workforce is finite**: a stock of people, which caps total employment.
- **B3** REASON — a person is in exactly one state: **employed, unemployed, or out of the workforce**, and
  moving between them is an **event**.
- **B4** REASON — an unemployed person **searches**, and search takes time — which is why unemployment is
  never zero even when every vacancy could be filled.
- **B5** VERIFY — employed plus unemployed plus inactive equals the population, exactly, every period.

### C. The demand side
- **C1** REASON — a firm **hires when the worker adds more than the wage costs**.
  - **C1.a** which depends on its **output price** and its **capital**.
- **C2** REASON — hiring has a **cost and a lag**: finding, and the time before the person is productive.
- **C3** REASON — **firing has a cost too**, which is why firms hold labour through a soft patch and shed it
  when they are sure — **and that asymmetry is where the cycle in employment comes from.** A pair of
  adjustment speeds is not a cost.
- **C4** REASON — a firm that **fails releases its workers at once**, through the same separation path as
  any other separation — and so does a merger that removes jobs.
- **C5** REASON — a **vacancy is a real posted intention to hire**, owned by the employer, and it can go
  unfilled or be **withdrawn**.

### D. The clearing
- **D1** REASON — the wage is a **price that clears** between posted supply and posted demand, per skill and
  region. **Every posting is a bid** — the employer's openings at the wage it offers — and the period's
  matches go to the **highest bids first**, pro rata within an equal bid. The bid that took the last match
  is the occupation's print.
  - **D1.a** so an offer above the going rate fills more than one below it. A single fill ratio applied
    identically to every employer means an employer's wage does not affect what it gets, which deletes the
    price from the market.
  - **D1.b** a firm the market rationed bids higher next period; a firm that filled bids its bargain's
    level; and either closes the gap **at its own management's horizon**.
  - **D1.c** the going rate is the **employment-weighted average of what is actually paid** — a read.
- **D2** REASON — it does **not clear instantly**: wages are sticky because the relationship in A4 is
  contractual and renegotiating is costly.
  - **D2.a** so the adjustment falls on **quantity** — employment — which is the central fact about this
    market and the reason recessions have unemployment in them.
  - **D2.b** VERIFY — stickiness must be a **consequence of the contract and the renegotiation cost**, never
    a coefficient damping a wage series.
- **D3** REASON — the **matching is imperfect**: not every unemployed person meets every vacancy.
- **D4** VERIFY — unemployment and vacancies should move against each other over the cycle, as a consequence
  of B4, C5 and D3.
- **D5** FORBID — **only one channel from prices to wages.** A firm's bid already carries a price rise as a
  nominal surplus per head; a second, separate cost-of-living adjustment applied to the going rate is law
  4's defect in the wage.

### E. What it feeds
- **E1** REASON — wages are **household income**, which drives consumption.
- **E2** REASON — wages are **firm cost**, which drives margin and price.
  - **E2.a** so a wage rise is simultaneously more demand and more cost, **and which dominates is a result,
    not an assumption.**
- **E3** REASON — wage income is **taxed**.
- **E4** REASON — job loss changes a household's **ability to service debt**, which is where labour reaches
  the credit system.

### F. What must not happen
- **F1** FORBID — **no employment without an employer.** Every job is at a named firm, and the wage leaves
  that firm's account.
- **F2** FORBID — **no wage bill without headcount**, and no headcount above the workforce.
- **F3** FORBID — **no exogenous unemployment rate.** It is a read of B3, and a written path deletes C1, C3
  and D3 at once.

---

## 40. HOUSING

### A. What a house is
- **A1** REASON — a **durable, immovable, indivisible asset owned by a named party**, in a named location,
  **counted in dwellings**.
  - **A1.a** **location is part of the identity**, and it is why there is no single housing market.
- **A2** REASON — it **yields a service**: shelter, consumed by whoever lives in it — so it is a consumption
  good and an asset at once, and both must be present.
- **A3** REASON — the owner and the occupier **can be different parties**, and then there is **rent**, which
  is a real payment between them. **A rental stock must have dwellings behind it**: a landlord is the owner
  of a dwelling somebody lives in, not a firm producing an abstract service.
- **A4** REASON — the **stock is finite and changes slowly**: new building takes years, **and a house built
  this period exists next period.** The stock is a register of units with owners, moved only by what changes
  hands and by what is built.
- **A5** REASON — it **depreciates and needs maintenance**, which is a real cost to the owner.

### B. The price
- **B1** REASON — it **clears between buyers and sellers**, per location.
  - **B1.a** the **offers** are owners whose tenure ends, each with a **reservation** — what it must fetch
    to discharge its own mortgage, and never below what it costs to build — plus the builders' completions
    at that cost.
  - **B1.b** the **bids** are what buyers can borrow at the keenest quote available to them.
  - **B1.c** it is a cross: units clear where a bid meets a reservation, and **an offer no bid reaches does
    not clear.**
- **B2** REASON — the buyer's demand is governed by **what it can borrow**, not only what it wants.
  - **B2.a** so the **mortgage rate and the lending standard are the dominant inputs to the price** — the
    strongest single transmission channel from policy to a household balance sheet.
- **B3** REASON — supply is **inelastic in the short run**, so a demand shift moves price, not quantity.
- **B4** REASON — a seller can **refuse to sell**, and **in a falling market transaction volumes collapse
  before prices do.**
  - **B4.a** VERIFY — so a price index built only from transactions is measuring a changing sample, and that
    is a real property of housing data, not an error to correct away.
- **B5** REASON — the **rent and the price are linked but not equal**: the yield is a read of the two, and it
  competes with other yields.

### C. The mortgage
- **C1** REASON — a **loan from a named lender secured on the house**, held as a row like any other loan.
- **C2** REASON — it has a **term, a rate — fixed or floating — and an amortisation schedule**, and the
  borrower pays interest **and** principal.
- **C3** REASON — the **loan-to-value is a read** of the loan against the house's current price, and it moves
  when the price moves without anyone doing anything.
- **C4** REASON — the borrower can **default, and then the lender takes the house and sells it.**
  - **C4.a** **a foreclosure moves a dwelling**: from the household sector to the lender or the estate, and
    what the estate's sale fetches is the recovery. **The foreclosed supply returns to the market**, and the
    extra supply is what makes a falling price fall further. A loss rate that reduces a loan's principal and
    the bank's profit, with no house seized and nothing sold, removes the loop that makes a housing bust a
    housing bust.
  - **C4.b** so losses are correlated exactly when they are largest.
- **C5** REASON — the lender's **standard is a decision**: how much loan-to-value, what income multiple —
  **and it tightens when the lender is worried.**
  - **C5.a** the lender already measures everything a standard should respond to: the loan-to-value
    cross-section of its own book, its hurdle, its headroom. **The standard is a read of those
    measurements, not a constant** — and a constant means only the rate channel loops.
  - **C5.b** which feeds straight back into B2.a, **and that loop is the housing cycle.**
- **C6** REASON — mortgages can be **pooled and sold**, which moves the risk to a named holder.

### D. What it feeds
- **D1** REASON — house price changes are **household wealth changes**, and wealth affects consumption.
- **D2** REASON — housing construction is **investment and employment**.
- **D3** REASON — rent is a **large component of the consumer price level**.
- **D4** REASON — mortgage debt is the **largest household liability** and its service is a fixed claim on
  income.
- **D5** VERIFY — a rate rise should reach consumption through D4 on floating mortgages **and** through B2.a
  and D1 on prices, with different lags, and the two channels are distinguishable.

### E. What must not happen
- **E1** FORBID — **no house without an owner and no owner without a house they hold** in the register.
- **E2** FORBID — **no exogenous house price path.** It is cleared, and a written path deletes C3, C4.a and
  C5.a — **the entire collateral channel.**
- **E3** FORBID — **no mortgage without a lender's balance sheet on the other side.**
- **E4** VERIFY — mortgage debt owed by households equals mortgage assets held by lenders and pools, exactly.

---

## 41. HOUSEHOLDS

### A. What a household is
- **A1** REASON — a unit that **earns, consumes, saves and owns**.
- **A2** REASON — households are **heterogeneous**, and the heterogeneity is load-bearing:
  - **A2.a** by **income and wealth** — the propensity to consume differs, so the same aggregate income
    produces different demand depending on who has it;
  - **A2.b** by **life stage** — earning, accumulating, drawing down;
  - **A2.c** by **employment state**;
  - **A2.d** FORBID — **the sector is never a single representative agent.** Every decision that matters
    here is a threshold, and **a mean-preserving spread must be able to cause defaults** — with one agent it
    cannot. A default threshold applied to a band's mean is the same defect one level down.
  - **A2.e** REASON — the sector is therefore a set of **CELLS** (XI-15): each a named party with an
    account, a register of holdings and a **WEIGHT** — an integer count of how many real households it
    is. A cell is one *possible* household with a multiplicity, **never the average of a group**.
  - **A2.f** FORBID — **no decision evaluated at an average.** Every consumption choice, every
    portfolio choice, every default test is evaluated **per cell** and then summed weighted. The
    sector's numbers are `Σ f(xᵢ)·wᵢ`, and never `f(Σ xᵢ·wᵢ)`.
  - **A2.g** VERIFY — a **mean-preserving spread raises the count of crossings** while the weighted
    mean does not move. This is the measurement that proves the representation is a distribution and
    not an average wearing a distribution's clothes.
- **A3** REASON — it is a **named party in the ledger** with an account.

### B. Income
- **B1** REASON — **wages**, from named employers, for labour supplied.
- **B2** REASON — **transfers** from the government, to named recipients.
- **B3** REASON — **investment income**: dividends, interest, coupons — from named payers.
  - **B3.a** FORBID — **income the household did not RECEIVE is not income.** Retained earnings raise the
    value of what it owns and reach it on sale or distribution.
- **B4** REASON — **income is taxed**, and the tax is remitted by somebody.
- **B5** VERIFY — sector income is the sum of what households were actually paid, never an accounting
  identity solved for.

### C. Consumption
- **C1** REASON — it **decides** how much to spend, and the decision has reasons:
  - **C1.a** current income;
  - **C1.b** **wealth**, which is why an asset price matters to demand;
  - **C1.c** expectations and confidence — its **own** (§46 C1), never a published aggregate;
  - **C1.d** **liquidity**: a household that cannot borrow spends what it has, whatever it wants.
- **C2** REASON — the residual is **saving**, and saving is a flow into B3's stock.
- **C3** REASON — spending is **allocated across goods** by preference and relative price.
- **C4** REASON — it **buys at a price it pays** — including tax and the distribution margin.
- **C5** VERIFY — consumption is the sum of what households actually bought, and it reaches named sellers.

### D. The balance sheet
- **D1** REASON — **assets**: deposits, securities held directly, fund shares, pensions, housing.
  - **D1.a** each is a real claim on a named issuer, **held in a register** — the household sector is the
    largest holder class in the model and it holds a real book, not a residual.
- **D2** REASON — **liabilities**: mortgages, consumer credit, and they are somebody's asset.
- **D3** REASON — **net worth is D1 minus D2**, a read and never a stored number.
- **D4** REASON — it **revalues** when prices move, and the revaluation is not income.
- **D5** REASON — the **portfolio allocation is a decision** with reasons: yield, risk, liquidity.
  - **D5.a** the choice between a **deposit, a money fund and bills directly** is a real substitution and it
    is **how a policy rate reaches a saver**. Fund shares issued pro rata and never chosen means the
    substitution never happens.
- **D6** REASON — households **bid** in the markets they participate in — they are participants, not a
  residual holder of what nobody else took.

### E. Borrowing
- **E1** REASON — it **borrows for reasons**: a house, consumption, a shortfall.
- **E2** REASON — a lender **decides** to lend to it, on affordability and collateral.
- **E3** REASON — it **services** the debt out of income, and the service is a real payment.
  - **E3.a** interest **plus principal**, and the distinction matters.
- **E4** REASON — it can **default**, and the default depends on the **distribution**, not the mean.
  - **E4.a** with a consequence: the collateral, the credit record, the loss to the lender. The credit
    record is the **cell's**, and a default that applies to part of a cell splits it first (XI-15), so
    the record still belongs to exactly the households it describes.
- **E5** VERIFY — the debt-service burden is a read of E3 against B, and it can become unpayable.

### F. The life cycle
- **F1** REASON — households **form, age and dissolve**.
  - **F1.a** cohort is a **key dimension** of the cell (XI-15), so **ageing is a split at the cohort
    boundary**, by date: when the calendar carries some of a cell's members across it, those members
    become a cell in the next cohort and the split is exact. A cell whose members straddle a boundary
    is an average of two cohorts, which A2.d forbids.
  - **F1.b** formation is **entry** and dissolution is **death**, both weight events under XI-15, both
    dated, and both with a cause.
- **F2** REASON — **wealth transfers on dissolution**, and it goes somewhere named — somebody inherits it.
  - **F2.a** at cell granularity the heir is a **named heir cell**, and the transfer is an ordinary
    movement of `weight × the member's wealth`. An estate that dissolves into nobody is law 2's
    residual with no holder.
- **F3** REASON — **retirement**: income switches from wages to drawdown, and the pension claim funds it.
  - **F3.a** which is a cohort crossing under F1.a, so it is a split and not a reclassification of an
    average.
- **F4** VERIFY — the sector's composition changes over time, and the aggregate follows from it.

---

## 42. SMALL-BUSINESS POOLS

### A. The sector
- **A1** REASON — small firms are **firms**: they sell, employ, borrow and can fail.
- **A2** REASON — they are represented as a **pool with a distribution**, not as an average.
  - **A2.a** FORBID — **no representative small firm.** Default is a threshold event; with one average firm
    a mean-preserving spread causes no defaults, and the entire credit content of the sector is gone.
- **A3** REASON — the pool has **observable characteristics**: size, sector, region, leverage, coverage — and
  losses depend on the **distribution** of those, not the mean.
- **A4** REASON — they **employ people** and **buy from and sell to** larger firms, so the sector is
  connected in both directions — **including on trade credit, which is the tier that lives on it.**
- **A5** REASON — they are **bank-dependent**: too small for the bond market.
  - **A5.a** which makes them the sector where a credit tightening bites first and hardest.
- **A6** REASON — the pool is a set of **CELLS** (XI-15): each a named party with a **WEIGHT** — an
  integer count of the firms it is — carrying its own state, borrowing from a **named** lender on its
  own loan row, and selling to and buying from named counterparties on trade credit.
  - **A6.a** every relationship that must be named is either a **dimension of the cell's key** or a
    **register row**, never an attribute averaged inside it. Which of the two it is, is what the
    registry declares (XI-15): its region and its bank are dimensions; its lender is a loan row per
    (lender, cell); and lifting a row into the key is a data change. A relationship averaged inside a
    cell is a relationship the model cannot name, and law 4 is broken quietly.
  - **A6.b** a weight of **one** is a named firm, so the boundary between this sector and Corporate
    Credit's is not a modelling line but a **size**.
  - **A6.c** a cell that **outgrows A5** — large enough to reach the bond market — is
    **promoted to a named firm**, and the promotion is an event with a cause. Without it the boundary
    is arbitrary and no firm can ever grow across it.

### B. The loans
- **B1** REASON — each is a **loan from a named lender** with a rate, a term and an amortisation.
- **B2** REASON — they are often **secured** on the firm's assets or the owner's house.
- **B3** REASON — they **default**, and the default depends on the individual firm's cash flow, aggregated
  over the pool.
- **B4** REASON — **defaults are correlated**: the same rates, the same demand, the same region hit all of
  them.
  - **B4.a** so the pool's loss is **not the sum of independent draws**, and the correlation is what makes
    the tranching in C meaningful or dangerous.

### C. The pool as an instrument
- **C1** REASON — the loans are **transferred into a vehicle** — a named party holding them, funded by
  issuing claims.
- **C2** REASON — the claims are **tranched by seniority**: losses hit the bottom first, and the top is
  protected until the bottom is gone.
  - **C2.a** the tranche boundaries are **stated**, and the loss allocation is a real rule applied to real
    losses.
- **C3** REASON — each tranche has a **price that clears**, and its yield is derived from that price.
- **C4** REASON — the tranches are **held by named holders**, and that is where the loss actually lands.
  - **C4.a** often the originating bank keeps the bottom, which means the risk did not leave.
- **C5** REASON — the vehicle's **cash flows are the loans' cash flows**: interest and principal in,
  distributed by seniority out.
- **C6** VERIFY — tranche values sum to the pool's value; losses allocated sum to losses incurred, exactly.
  **No tranching creates or destroys loss.**

### D. Why it matters
- **D1** REASON — it **moves credit risk from banks to investors**, and the investors are named.
- **D2** REASON — it **frees bank capital**, which lets the bank lend again — so securitisation is a
  **lending-capacity mechanism**, not just an instrument. Without it, credit risk sits on the bank that
  wrote it for ever and origination can never be expanded by selling risk.
- **D3** REASON — the senior tranche is **used as collateral**, so its liquidity matters to the funding
  system.
- **D4** REASON — when B4's correlation is worse than the tranching assumed, the **senior tranche takes
  losses it was not supposed to**, and every holder in D1 and D3 is affected at once.
  - **D4.a** VERIFY — this must be emergent from B4 and C2, never a scripted event.

### E. What must not happen
- **E1** FORBID — **no pool without underlying loans to named borrowers.** A pool whose losses come from a
  loss rate has no A3, no B3 and no D4 — and **tranching a loss rate yields senior notes that can never be
  touched.**
- **E2** FORBID — **no tranche without a holder.**
- **E3** FORBID — **no risk transfer without a transferee.** If the bank's exposure fell, somebody named
  picked it up.
- **E4** FORBID — **a constant pool population.** If entry is the accounting identity of exit, the population
  is constant by construction and nothing the sector experiences can change it.
- **E5** FORBID — **no weight that is not a count.** A weight is how many firms a cell **is** — never a
  share, never a scale factor, never a probability. It changes only by **entry**, by **death**, by
  A6.c's **promotion**, by a **split** and by a **merge** (XI-15) — five events, each with a cause and
  a date — and the sum of weights is a read.
- **E6** FORBID — **no loss allocated to a pool rather than to its cells.** A loss struck against the
  pool and spread back over its members has been evaluated at an average, which is A2.a one level up.

---

# PART X — CROSS-CUTTING

---

## 43. CROSS-BORDER

### A. What makes a transaction cross-border
- **A1** REASON — the two named parties are in **different regions**.
- **A2** REASON — so the transaction is in **one of two currencies, or a third**, and somebody has to decide
  which.
  - **A2.a** and whoever is not in the invoice currency has an **FX exposure**, which it can hedge or carry.
- **A3** REASON — the settlement crosses banking systems: a payment in a currency reaches an account **in
  that currency**, wherever the account holder is.
- **A4** REASON — the counterparty is **foreign**, which is a real credit and legal difference.

### B. Trade in goods
- **B1** REASON — a firm **buys from or sells to** a firm in another region, because of price, availability
  or cost.
- **B2** REASON — the goods **move**, which costs money and takes time.
- **B3** REASON — the price the buyer pays in its own money depends on the **exchange rate**, so a rate move
  changes real trade decisions.
  - **B3.a** which is the expenditure-switching channel, and it must be a **consequence** of B1's decision
    facing a changed relative price, never an elasticity applied to a balance.
- **B4** VERIFY — one region's exports are another's imports, **unit for unit and party to party**.

### C. Cross-border finance
- **C1** REASON — an investor **holds a foreign asset** because of its return, and it is a claim on a foreign
  issuer.
- **C2** REASON — a borrower **issues in a foreign currency** because the funding is cheaper or the buyer
  base is deeper — **and it then owes a money it does not earn.**
  - **C2.a** which is a real solvency risk that a rate move triggers, not a translation adjustment.
- **C3** REASON — a **bank funds in one currency and lends in another**, and it must square that
  continuously.
- **C4** REASON — a **direct investment** buys a firm outright, which is a lasting claim.
- **C5** REASON — every one of these produces **income flows across the border**: coupons, dividends,
  interest — paid to foreign holders in a currency.

### D. The balance
- **D1** REASON — a region's **current account is a READ**: goods and services plus income flows, computed
  from actual transactions. **Never a stored field.**
- **D2** REASON — its **financial account is the other side**: net acquisition of foreign claims.
- **D3** VERIFY — the two sum to zero for each region **as a consequence of every transaction having two
  sides**, never as an identity imposed after the fact.
  - **D3.a** a residual that has to be plugged is a transaction that lost a leg.
- **D4** REASON — **a deficit region must be financed by somebody who chooses to finance it, at a price** —
  so the financing is a market outcome, **and it can stop.**
  - **D4.a** and a persistent one-way trade flow financed by the banking system is a **real phenomenon**,
    not necessarily a defect — but it must be visible as a position on somebody's book, not as an
    unexplained drift.
- **D5** REASON — the accumulated position is a **stock of claims held by named parties** that revalues when
  rates and prices move.
- **D6** VERIFY — summing all regions gives zero in every category, **because the world is closed.**

### E. What it forces on every other system
- **E1** REASON — a **market must be able to have foreign participants with foreign money**, or it is a
  domestic market wearing a region label.
- **E2** REASON — a **register must hold foreign issuers' instruments** for domestic holders.
- **E3** REASON — a **default must reach foreign holders** in proportion, like any other.
- **E4** REASON — a **central bank's actions reach other regions** through the rate and the currency.

### F. What must not happen
- **F1** FORBID — **no region that is a closed box.** If every party trades only domestically, every node
  above is dead and exchange rates exist only as a number.
- **F2** FORBID — **no netting of cross-border flows into a regional aggregate.** The parties are named on
  both sides, or the loss chain in E3 cannot be traced.
- **F3** FORBID — **no exogenous trade or capital-flow series.** Both are consequences of B1 and C1's
  decisions.

---

## 44. RATINGS AND ASSESSMENT

### A. What a rating is
- **A1** REASON — an **ordinal judgement about a named issuer or instrument**, published, and visible to
  everyone.
- **A2** REASON — it is derived from **observable state**: leverage, coverage, cash, size, sector, **age**,
  and the trend in them.
  - **A2.a** FORBID — **a rating is never derived from the price.** If it reads the spread, it is a
    restatement of the market and cannot disagree with it — which deletes both its information content and
    the feedback in D.
- **A3** REASON — it is **coarse and sticky**: a small change in state does not move it, which is what makes
  a move meaningful and what makes it late.
- **A4** REASON — it is **an opinion, not a fact**, and it can be wrong — a rated-safe issuer can fail.
- **A5** REASON — it is published by a **named assessor**, which is a party with its own incentives.
  - **A5.a** and it is **not the only assessment**: holders run their own, and the disagreement is a large
    part of why a book has two sides. One universal rating held by nobody means every participant agrees
    about credit by construction.

### B. What it measures
- **B1** REASON — the **probability of failing to perform**.
- **B2** REASON — and, separately, the **loss given that failure**, which depends on seniority and security.
  - **B2.a** so an **instrument's rating differs from its issuer's**, and both must exist.
- **B3** REASON — it is **relative**: an ordering across issuers, which is what makes it usable in a rule.

### C. Why it matters — the rules that refer to it
- **C1** REASON — **mandates** restrict what a fund, insurer or pension may hold.
  - **C1.a** so a downgrade past a boundary is a **forced sale by every holder bound by it, at the same
    time** — a real, dated, mechanical flow.
- **C2** REASON — **capital charges** depend on it, so a downgrade consumes a bank's capital without the
  bank doing anything.
- **C3** REASON — **collateral haircuts** depend on it, so a downgrade reduces how much can be borrowed
  against the asset. **A haircut that is one number per instrument type — the same for the best and worst
  credit — is the one leg of the downgrade loop that is wholly absent.** A per-issuer risk measure closes
  it, with or without a rating table.
- **C4** REASON — **contract terms refer to it**: covenants, triggers, the right to demand more collateral.
- **C5** REASON — participants use it as **information** when they have no better.

### D. The feedback — the point of this system
- **D1** REASON — C1–C4 mean a downgrade **causes selling, capital pressure and funding loss**.
- **D2** REASON — those raise the issuer's **cost of funds**.
- **D3** REASON — which **worsens the state in A2**, which can cause a further downgrade.
- **D4** VERIFY — this loop must be **emergent from A2, C and D2 and traceable step by step.** It is the
  mechanism behind a cliff edge, and it is precisely what a rating read off the spread can never produce,
  because there the loop is a tautology.
- **D5** REASON — it works the other way too: improvement widens the buyer base and cheapens funding.

### E. What must not happen
- **E1** FORBID — **no rating with no consequence.** A published letter that no rule refers to is
  decoration; the whole system is C. A sovereign rating whose only consumers are display strings has no
  system behind it.
- **E2** FORBID — **no rating that changes for no reason.** Every move traces to a change in A2.
- **E3** FORBID — **no assessment that is always right.** If a rating never misprices, C1's forced sales
  never surprise anyone and A4 is deleted.
- **E4** VERIFY — the distribution of ratings across issuers is a **read** of their states, never a target
  distribution the issuers were fitted to.

---

## 48. REPORTING AND ESTIMATES

*Numbered 48 and placed here: it is cross-cutting and it is §44's sibling — an opinion published by a
named party with its own incentives, which rules then refer to. §32 E7 owns management's expectation and
§46 owns how any outlook is formed; this system owns the calendar, the report that settles them, and what
the settling causes.*

### A. What a report is
- **A1** REASON — a **public company** — a firm whose shares are held by parties other than its founders
  and trade in a book (§10) — publishes, on a stated calendar, **what its own books produced**: the fiscal
  period's income, the balance sheet at its close, and the cash that moved.
  - **A1.a** being public is a **state read from the register**, never a label: a firm becomes public when
    its shares are listed and held by outsiders, and stops when they cease. A firm whose shares do not
    trade publishes nothing, and there is no kind of firm that reports.
- **A2** REASON — the report is a **read of the ledger and the register**, not a statement management
  composes. Every figure in it is reachable from instructions that settled and marks that were taken.
  - **A2.a** FORBID — **no reported number the books do not produce.** A figure that cannot be traced to
    settled instructions is a second set of accounts (law 4), and the accounts family (§4 B5) is what
    proves the two agree.
- **A3** REASON — it covers a **fiscal period**: a quarter placed by date on the one calendar (§1 G3.a),
  which is a whole number of periods only by accident.
- **A4** REASON — it is published **after a lag** — the books close, then the report comes out — and in
  between the firm knows its result and nobody else does.
  - **A4.a** that gap is **real information asymmetry**, and it is the only kind this world has: everything
    else is public when it happens. What management may do while holding it is §35's business.
- **A5** REASON — a figure can be **restated**: republished with a correction, dated, with the original
  standing (§2 E2.a: a correction is a new entry, never an erasure). A restatement is information about the
  management.

### B. Guidance
- **B1** REASON — management publishes an **expectation of the coming fiscal period** — §32 E7's
  expectation, on this system's calendar and in the report's own lines.
- **B2** REASON — it carries a **horizon and a unit** (§46 A5), and it can be **revised between reports**
  or **withdrawn**; both are events with a date.
- **B3** REASON — it is management's **own outlook** (§46 A2), so it can be wrong, and a management that is
  persistently wrong is one whose guidance others weigh less.
- **B4** FORBID — **no guidance that is a second number.** The published figure is the one the firm's own
  decisions read (§46 C2), never one composed for the audience. A management that guides to a number it is
  not itself acting on has had its decisions made somewhere else.

### C. The estimate
- **C1** REASON — a **bank publishes its own estimate** of a covered company's coming report, in the lines
  that report will carry, formed as any outlook is (§46 A2, B1) from what it has observed of that company.
- **C2** REASON — it is **named and dated**: the estimate belongs to a bank and is visible to everyone.
- **C3** REASON — estimates **disagree**, and the disagreement is load-bearing (§46 A3): banks with
  different histories of a name estimate differently, and that is one of the reasons a share book has two
  sides.
- **C4** REASON — an estimate is **revised on information** — the company's report, its guidance, what the
  bank observes of the company's own markets — and a revision is an event with a date and a size.
- **C5** FORBID — **no estimate that is the model's own forecast** (§46 A4). A bank handed the answer
  deletes both C3's disagreement and F's surprise.
- **C6** FORBID — **no estimate derived from the share price.** §44 A2.a's defect in this system: an
  estimate that reads the price is a restatement of the market, cannot disagree with it, and makes F2 a
  tautology.

### D. Coverage — why a bank does it
- **D1** REASON — a bank covers a name because **its own book needs the view**: it makes a market in the
  share (§26), lends to the issuer (§23), or holds it — so it already forms the outlook (§46 C4).
  Publishing is a decision to disclose one it has, taken for what disclosure brings it.
- **D2** REASON — coverage **costs**: the analysts are employed (§39) and the cost has a named payee. A
  bank initiates on a name it wants flow in and drops one it cannot justify, and both are decisions.
- **D3** REASON — **coverage is uneven**: a large, widely held name carries many estimates and a small one
  none or one. How many cover a name is an OUTCOME of D1 and D2.
  - **D3.a** FORBID — **no universal coverage.** Every company covered by every bank makes the count of
    estimates a constant rather than a read, and deletes D3.

### E. Consensus
- **E1** REASON — the **consensus is a read**: the aggregate of the estimates that exist, computed when
  somebody looks, published with the lag and revision any statistic has (§45 A5).
- **E2** FORBID — **no consensus a decision consults.** §46 A2.b: there is no variable in this world called
  the market's expectation. A party may observe the consensus as one more published statistic (§46 A2.a)
  and weigh it in forming its own outlook; nothing may read it *as* its outlook.
- **E3** FORBID — **no stored consensus.** It is computed from the estimates at the moment of reading, like
  an index from its constituents (§22 A2), or it is a second number that can disagree with the estimates it
  is made of.

### F. The surprise, and what it causes
- **F1** REASON — the report **settles** every expectation standing against it — management's guidance and
  each bank's estimate. Observed minus expected, per holder of a view, is §46 B2's surprise with a name on
  it, and it is recorded.
- **F2** REASON — what a surprise causes is **participants revising their own outlooks**, and therefore
  their reservations in the share book (§46 C3): the price moves because the schedules moved.
  - **F2.a** FORBID — **no price reaction rule.** A stated move per unit of surprise is a written price
    path (law 3) and it deletes F2 — the move must be what the changed schedules cleared at, or nothing.
- **F3** REASON — a bank's **record is a read**: how wide its own past errors on a name have been, visible
  to everyone. It is what makes one bank's estimate weigh differently from another's in a holder's own
  outlook — §46 B3's confidence, applied to somebody else's forecast.
- **F4** REASON — guidance missed is **information about the management**, and it reaches the cost of
  capital through the ordinary channels: holders' outlooks, the lender's view of the name (§46 C4), the
  assessor's state-based judgement (§44 A2). It is never a charge applied to the firm.

### G. What must not happen
- **G1** FORBID — **no report with no consequence** (§44 E1 in this system). If nothing reads the report,
  the calendar is decoration and the surprise is a number nobody acts on.
- **G2** FORBID — **no earnings that were not earned.** Reported income is the equity account's movement
  over the fiscal period, decomposed into what the instructions and the marks did — never a figure
  management chose, and never smoothed.
- **G3** FORBID — **no analyst always right, and none always wrong by a fixed amount.** Either is the
  answer with an offset, which is the answer.
- **G4** FORBID — **no estimate of a company that does not report**, and no report from a company whose
  shares nobody outside holds.
- **G5** FORBID — **no per-share figure that is a primitive.** Earnings per share is income divided by
  shares outstanding, both of them reads; a stated one is an outcome written down (law 2).
- **G6** FORBID — **no reporting calendar finer than a period** (§1 G3.b), and none placed by a count of
  periods rather than by a date.

### H. Measurement
- **H1** VERIFY — the **dispersion of estimates** on a name widens after its results have been volatile and
  narrows when they have not — a read of the estimates, never a target.
- **H2** VERIFY — the **share price moves more** on a large surprise than a small one, and the relationship
  is emergent from F2 and never stated.
- **H3** VERIFY — a company that has missed guidance repeatedly is **covered differently**: the count of
  estimates and their dispersion respond to its record.
- **H4** VERIFY — the consensus **lags** the information that produced it (§46 E2), and never moves in the
  period a surprise lands.

---

## 45. THE OBSERVER SURFACE AND THE NEWS

### A. What an observer can see
- **A1** REASON — **prints**: prices that cleared, with their instrument, time and unit.
  - **A1.a** and a **stale mark must be visibly stale**: a screen that shows a price without saying when it
    traded is misinformation.
- **A2** REASON — **its own positions and balances**, exactly as the register and the accounts hold them.
- **A3** REASON — **public state**: what an issuer has published — including a public company's report,
  its guidance and every bank's estimate of it (§48) — what a central bank has decided, what an
  assessor has said.
- **A4** FORBID — **no observer sees another party's private state.** Positions, intentions and limits are
  private, and a surface that exposes them makes the market a solved game. *Whether the surface is an
  inspector's full view or a participant's partial view is a decision that must be taken deliberately, and
  it is a different product in each case.*
- **A5** REASON — **aggregates that are genuinely published** — indices, official statistics — with the lag
  and the revision that real statistics have.
  - **A5.a** a statistic available instantly and exactly is not a statistic; it is the model's internals.

### B. What an event is
- **B1** REASON — a **change of state that somebody would notice**: a default, a downgrade, a policy move, a
  large print, a failed auction, an under-subscribed offering, a run of periods a party could not pay, a
  waterfall drawing on its members, an estate's weekly distribution.
- **B2** REASON — it **describes something that actually happened in the state**, and it is generated FROM
  the state.
  - **B2.a** FORBID — **news never causes anything.** An event that moves a price directly is an exogenous
    shock with a headline attached; the price moves because participants acted, and the event is the report
    of it.
- **B3** REASON — it has a **time and named subjects**, so it can be checked against the state.
- **B4** REASON — it can be **wrong or incomplete** in the same way real reporting is, but it may **never be
  invented**. A story that asserts a recovery rate while an estate computes a real one is invented.
- **B5** REASON — a **story develops**: a workout runs for periods, paying classes and selling assets, and
  each period of it is reportable — not only its opening and its close.

### C. What an actor can do
- **C1** REASON — the actions available are the ones **any participant has**: post a schedule, trade, lend,
  borrow, hedge, hold.
  - **C1.a** acting means **entering a market that must clear** — the price is not the actor's to set, and
    the fill does not arrive from the actor.
- **C2** REASON — an action **requires the means**: cash in the right currency, the holding to sell, the
  borrowing capacity, the collateral.
  - **C2.a** FORBID — **no privileged actor.** Nobody transacts without the balance, outside the mechanism,
    or at a price that did not clear. **The notional leaves the account, not just the fee.** A surface that
    lets its user do otherwise is measuring a different world from the one it is displaying.
- **C3** REASON — an action has **consequences that propagate exactly like anyone else's**.
- **C4** REASON — the actor is a **named party in the register and the accounts**, and it appears in every
  consistency check like the rest.

### D. The record
- **D1** REASON — a **history that is a read of what happened**, not a separate log that can drift.
- **D2** REASON — **performance is computed from real positions and real prices, so it can be bad.**
- **D3** VERIFY — anything shown must be **reproducible from the state**; a number on the surface with no
  derivation behind it is a number invented for display.

### E. What must not happen
- **E1** FORBID — **no display-only number.** If it is worth showing it is worth deriving, and if it cannot
  be derived it must not be shown.
- **E2** FORBID — **no scripted narrative.** A sequence of events written in advance is law 2's defect at
  the level of the whole world.
- **E3** FORBID — **no surface that changes the model.** Observing must not move anything — not a balance,
  not a price, and not an internal table. **If looking at a market changes it, every measurement here is
  contaminated.**

### F. Naming and presentation
- **F1** REASON — every instrument is displayed by the **name a market would use**, from one grammar.
- **F2** REASON — every priced asset **shows its price**, and fixed income shows **both the price and the
  derived spread or yield**; unprinted paper shows that it has no price, never par.
- **F3** REASON — one **calendar** — Money G3's, and not a second one. A surface that dates anything
  its own way is law 4's two writers pointed at time.
- **F4** REASON — a **missing number is shown as missing**, never as zero or as a formatted default.

---

## 46. EXPECTATIONS

*Numbered 46 and placed here: it is cross-cutting — every deciding party in Parts V–IX carries one —
so it sits with the other cross-cutting systems, and XI-16 carries the mechanism at length.*

### A. What an expectation is
- **A1** REASON — a party's **own forecast of a variable it will act on**: its income, the prices it pays,
  the rate it borrows at, the demand for what it sells, the value of what it holds, whether it keeps its
  job. Every decision in this world is taken looking forward, and this is what the decider sees.
- **A2** REASON — it is **personal**. It is formed from **what that party observed** — its own wage
  received, its own prices paid, its own coupons, its own fills — and from nothing it did not experience.
  - **A2.a** a **published statistic** reaches a party only as an observer's aggregate (§45 A5), with the
    lag and revision a statistic has, and it enters the party's outlook as one more thing observed — never
    as the outlook itself.
  - **A2.b** FORBID — **no global expectation.** There is no variable in this world called *the* expected
    inflation rate, *the* expected growth rate or *the* market's expectation. An aggregate of expectations
    is a read (§45 A5); it exists nowhere a decision can consult it.
- **A3** REASON — expectations are **heterogeneous, and the heterogeneity is load-bearing**. Different
  histories make different outlooks, and different outlooks are the different reasons a market needs to
  have two sides (Clearing A1.a, XI-13). A world in which every party expected the same thing would trade
  once and stop.
- **A4** FORBID — **no expectation that is the model's own forecast.** An outlook computed by running the
  world forward and handing the answer to the participants is XI-13's fixed point: a price that is a
  function of the model's prediction of the price carries no information. The expectation is formed from
  the **past**, by the party, and it can be wrong.
- **A5** REASON — an expectation **carries its unit and its periodicity** (law 8): an expected income per
  period, an expected price level in a named currency, an expected rate over a stated horizon.

### B. How it is formed
- **B1** REASON — **adaptively, from the party's own history**: the last outlook, corrected towards what
  actually happened, at the party's own speed.
  - **B1.a** the speed is the one **PREFERENCE** this system admits — a **memory**: how many of its own
    periods a party weighs. It is dispersed across parties (§41 A2), because a sector whose members all
    remembered the same way would move as one, and drawn once at entry.
  - **B1.b** FORBID — **no second primitive.** Confidence, sentiment, optimism, an anchor, a target: each is
    either a read of B2's surprises or a number nobody could derive, and the second kind is forbidden by
    law 2.
- **B2** REASON — the **surprise is a real event**: observed minus expected, per party, per variable, per
  period, and it is recorded. It is the only thing that changes an outlook.
  - **B2.a** VERIFY — an outlook that moved with no surprise behind it moved for a reason nobody stated.
- **B3** REASON — **confidence is a read**: how wide a party's own recent surprises have been. A party that
  has been surprised often and widely is one that does not trust its own outlook, and it acts on that —
  it holds more liquidity, spends less of an expected windfall, bids more cautiously. Confidence is never
  stated, never drawn, never an input.
- **B4** REASON — an outlook is formed **before the period it is used in**, from history only. A party
  that could read the period's own result before acting would be a party with no expectation at all.
- **B5** VERIFY — expectations **lag turning points**, and the lag differs by party with its memory. An
  economy whose parties all saw the turn in the period it happened has been handed the answer.

### C. What it does
- **C1** REASON — **consumption** reads the cell's own expected income and its own expected prices — §41
  C1.c, made concrete. A cell expecting a fall spends less of what it has; one that has been surprised
  widely holds more liquidity (B3, §41 C1.d).
- **C2** REASON — a **firm's** output, hiring and investment read its own expectation of demand for what
  it sells — §32 E7's published expectation is this outlook, made public, and the surprise against it is
  the event E7 names.
  - **C2.a** an outlook may be **about another party**: a bank's estimate of a company's coming report
    (§48 C1) is formed the same way, from what that bank observed of that company, and is one more
    published thing others may weigh (A2.a) rather than an outlook anybody inherits.
- **C3** REASON — a **participant's view** in any book is its expectation of the price, and the drift of a
  view between periods is the surprise it took. This is the mechanism that XI-13 asks for: a reservation
  formed from something other than the quantity the clearing reveals — and it is the party's own past.
- **C4** REASON — a **lender's view of a name** (§11 B2, §23) is its expectation of getting it back,
  formed from what it has seen of that name; a **desk's** view of a line (§26 C1) is its expectation of
  the flow.
- **C5** REASON — the **treasury** and the **central bank** are parties, and their programme and their
  rule read **their own** expectations — of receipts, of prices, of the position they will be in — formed
  the same way from what they observed. The public sector does not get the model's forecast either.
- **C6** REASON — the **vote** (§47 B1) reads the cell's outlook and its confidence: a household votes on
  where it expects to be, not on where a statistic says the economy is.

### D. What must not happen
- **D1** FORBID — **no peeking.** No expectation reads the period's own result, another party's private
  state (§45 A4), or the model's internals. B4 is a rule about time and A2 is a rule about scope, and
  together they are the whole of what makes an expectation *personal*.
- **D2** FORBID — **no expectation evaluated at an average** (§41 A2.f). A cell's outlook is one possible
  household's outlook, carried with its weight; the sector's is `Σ f(eᵢ)·wᵢ` and never `f(Σ eᵢ·wᵢ)`.
- **D3** FORBID — **no expectation without its falsification.** Law 17 at the level of the world: every
  outlook is later scored against what happened, the score is B2's surprise, and an outlook that is never
  scored is a forecast nothing can kill.
- **D4** FORBID — **no sentiment as a cause.** An aggregate of outlooks — a confidence index, a survey —
  is a read that is published with a lag (§45 A5), and it causes nothing (§45 B2.a). What causes things is
  each party acting on its own outlook; the index is the report of it.

### E. Measurement
- **E1** VERIFY — the **dispersion of outlooks widens in a downturn**, and a mean-preserving spread of
  outlooks raises the count of threshold crossings (§41 A2.g) while the weighted mean does not move.
- **E2** VERIFY — the published confidence aggregate **moves after** the surprises that produced it, never
  before, and never in the same period a shock lands.
- **E3** VERIFY — a shock felt by some parties changes **their** behaviour before it changes anyone
  else's: the parties who were surprised act first, and the rest act when the consequences reach them.
- **E4** VERIFY — the aggregate forecast error of a sector is the weighted sum of its cells' errors and is
  never itself a stated number.

---

# PART XI — MECHANISMS IN DEPTH

Seventeen mechanisms that no single system owns. Each is a **causal chain**: it runs through several
systems, and it is the reason those systems are connected at all. A model can satisfy every system
tree in Parts III–X node by node and still have none of these, because each of them lives in the
joints.

They are written here at the length they need, because each one is the difference between a world
that transmits and a world that merely computes.

---

## XI-1. A LOSS IS AN EVENT, NOT A RATE

**The mechanism.** A borrower crosses a threshold. That crossing is an **event with a date**. A claim
becomes non-performing, then impaired, then written off. Something is seized or realised. **The
recovery is what that something fetched.** The loss lands on named holders in proportion.

**Why a rate is not a substitute, in four separate ways.**

- **There is no borrower.** *Principal times a probability of default times a loss given default,
  divided by the number of periods in a year*, subtracted from a book, extinguishes debt by
  arithmetic: no event, no borrower, no cash, no recovery, nothing to observe and nothing to react
  to.
- **There is nothing to distribute.** A tranched claim over a loss rate produces senior notes that
  **can never be touched**, because a rate applied smoothly never concentrates. The entire purpose of
  tranching — that correlation worse than assumed hits the top — is unreachable.
- **There is nothing to seize.** A mortgage loss rate that reduces a principal and a lender's profit
  leaves the house where it was. **The foreclosed supply that makes a falling price fall further does
  not exist**, and the housing cycle has only its rate channel.
- **There is nothing to disagree with.** A credit market's whole function is to hold a **second
  opinion** about a borrower. If the loss is an arithmetic function of that borrower's accounts, and
  every participant's reservation is built from the same function, the market **cannot** disagree with
  the accounting model, and its price carries no information the accounts did not already have.

**The threshold matters more than the mean.** Apply a default test to a band's *average* borrower and a
mean-preserving spread causes no defaults at all — which is exactly backwards, because widening
dispersion at constant mean is what a downturn does. **Population-level default must be a read of
cell-level crossings.** What a cell is, and what may never be done to one, is **XI-15**.

**Where it reaches.** Bank lending (a status that is written, a provision that is booked, a write-off
with a date); small-business pools (a population that changes because firms fail, not because entry is
defined as the identity of exit); housing (a repossession, a sale, a supply); credit default swaps (an
event to trigger on); bank resolution (a book that can be worth less than its face, which is the only
thing that makes a valuation meaningful).

**Sequencing.** This is the **first** of the fifteen, because at least four other mechanisms need
"a claim goes unpaid" to already be a thing that happens.

---

## XI-2. THE FORCED SELLER

**The mechanism.** A party is made to sell something it did not want to sell, at whatever price the
market gives it, **in the same period**, and the sale moves the price, and the price move reaches
somebody else.

There are exactly four doors into this, and the model needs all of them because they arrive from
different directions:

1. **A margin call the client cannot meet from cash.** The broker's requirement rises — because prices
   moved, or because the broker likes what it sees less — and the client must sell. **The requirement
   must be able to rise**: a margin expressed as a stated rate cannot, which deletes precisely the
   procyclicality that *is* the contagion mechanism.
2. **A redemption the fund cannot meet from its buffer.** The fund sells, into a market that must
   clear. A redemption rationed by the fund's cash, with the unfilled part dropped, is not a
   redemption.
3. **A funding line withdrawn.** A leveraged holder's lender stops lending, and the position must go.
4. **A mandate breach.** A downgrade past a boundary forces every holder bound by it to sell, **at the
   same time** — the most mechanical and most synchronised of the four.

**The three ways the channel is silently closed**, each of which looks locally reasonable:

- **A floor at zero on available headroom.** A client drawn past its line is *over* the line, and the
  shortfall is what forces the sale. Flooring the available amount at zero makes the shortfall
  unrepresentable.
- **Lending the shortfall straight back.** Cutting the line and then meeting the resulting negative
  balance with an automatic loan from the same lender, at a penalty, restores the position and deletes
  the sale.
- **A cash test that is not applied.** A variation-margin payment with no test of whether the payer has
  the cash becomes a borrowing, silently, and the call never bites.

**The consequence of its absence, stated plainly.** *Nothing being forced to sell anything* is why no
shock ever propagates through a price. Every other transmission channel in this document ultimately
terminates in somebody having to transact when they would rather not.

**And the chain must terminate somewhere.** A forced sale that hits a party that cannot fail is a chain
that runs into a wall — see XI-3.

---

## XI-3. NOTHING IS IMMORTAL

**The mechanism.** Every kind of party in this world can cease to exist, and each has its own trigger
and its own consequence:

| Party | Fails when | And then |
|---|---|---|
| Firm | it cannot pay, or liabilities exceed assets | estate, waterfall, employees released, suppliers take losses |
| Bank | it cannot fund itself, **or** its capital is gone | resolution: valuation, bail-in hierarchy, acquirer or public path |
| Fund | its equity is gone | its broker eats the shortfall; its investors lose their equity |
| Insurer / pension | assets fall below the present value of its liabilities | sponsor contribution, de-risking, benefit reduction — or failure |
| Sponsor / vehicle | it cannot meet its commitments | it winds up; its investors' claims resolve into cash |
| Clearing house | it runs past the end of its waterfall | a real event with real consequences, not an impossibility |
| Sovereign | it will not or cannot pay, in a money it cannot create | selective default, exchange, market exclusion |
| Household cell | it dissolves (Households F1) | its wealth transfers to a **named heir cell**, never to nobody (F2) |

**The one exception, stated rather than left as an omission.** A **central bank** cannot cease in its
own money: §31 A1.a says it can never run out of what it alone issues, which is the whole reason a
corridor works. It can still make a loss, and the loss is real — it reduces its equity, it is not
remitted, and the deferred asset is a row the treasury may have to make good (§31 E4). So the mechanism
is *"every kind of party can cease"* with one named exception whose immortality is a **consequence of
its balance sheet**, not an oversight.

**Why it is load-bearing rather than tidy.** A default state that is read in many places and **written
only by the seed** means every institution outside the firm and bank sectors is immortal. That is not a
missing feature; it is the **termination condition of every loss chain in the model**. A forced-seller
cascade that reaches an immortal fund stops there, and stops without saying so.

**The specific sharp cases.** An underfunded pension whose funding gap makes it reach for *more* risk,
with no solvency consequence, is a party rewarded for being insolvent. A vehicle that never winds up
leaves its investors' claims frozen for ever, which converts an illiquidity premium into a free lunch.

---

## XI-4. THE COST OF CAPITAL — THE TRANSMISSION JOINT

**The mechanism.** A financial price changes. Somebody's cost of capital changes. A real decision
changes. Output changes, with a lag.

**This is the chain the whole model exists to have**, and it has three joints, each of which can break
it on its own.

**Joint one — a bank's cost of funds.** A bank prices a loan from *its own* economics: its blended cost
of funds across its own mix of deposits, wholesale borrowing and capital; the borrower's expected loss;
the capital the loan consumes times the return it needs on that capital; and an operating cost. **A bank
with no cost-of-funds term prices every loan as though it funded at the policy rate whatever its own
position** — and then nothing about that bank's funding condition can ever reach a borrower.

Corollary: **one rate per liability**. A liability whose interest cost is computed one way for a
reported margin and another way for the cash that actually leaves is two prices for one thing, and the
decision reads whichever one it happens to reach.

**Joint two — investment as a project with a return and a hurdle.** A firm invests when it expects the
return to exceed its cost of capital. The **return** comes from expected demand and price. The **cost**
comes from the markets: what its debt costs **at the margin, now** — not the average coupon on debt
already outstanding — and what its **equity** costs. The **hurdle and the horizon are the management's
own**, read off its risk aversion and its patience.

*Investment as a rate on revenue, however many multipliers are attached, is this joint deleted.* Nothing
anywhere compares an expected return to a cost of capital, and no financial price can reach a real
decision.

**Joint three — a dealer's inventory pays rent.** A position consumes cash and capital, and it is
charged for both, every period it is held. A desk that carries inventory for free has **no reason to
shed it**, so its quotes never skew, and the mechanism by which order flow moves prices does not exist.

**What downstream depends on it.** Every cleared price in this model is inert until at least one of these
three joints is closed. A cost of capital is also what prices a bank's own equity raise, what sets an
acquirer's valuation of a target, and what makes entering a product line a decision rather than a rule.

---

## XI-5. DELIVERY AND PAYMENT ARE ONE EVENT

**The mechanism.** The security leg and the cash leg of a trade are **the same event**. Neither happens
without the other. A failure to complete is a **fail** — a real, named, observable state.

**Why separating them is not a simplification.** If the paper moves at one point in the period and the
cash at another, then:

- **A fail has nowhere to live.** There is no single event to fail, so settlement risk is
  unrepresentable.
- **The two legs can be different sizes.** The classic form: the cash the issuer receives is limited to
  what the buyers could afford, while the **whole** amount of paper moves — and because the cash leg was
  capped, the leftover is zero and any guard on the difference cannot fire.
- **Nothing records what a position cost.** Without a single event there is no natural place to write the
  basis, and without a basis there is no realised gain and no tax base.

**Two riders that belong to the same mechanism.** A **basis** on every position, recorded at the moment of
acquisition, as lots. And a market with **no trades** producing **no print**, flagged as such rather than
silently repeating its previous statistic.

---

## XI-6. VALUE IS A FUNCTION, NOT A FIELD

**The mechanism.** A position is `(asset, units)` — nothing else. An asset has **one cleared price per
period**, in a price store, written only by its market. **Value is `units × price(asset)`, computed at
read.** Money is the single degenerate case: its price is one by definition, and that is the only place a
hard-coded one is allowed.

An asset genuinely not traded is **carried at cost**, and *carried at cost* is a **declared property of
the asset**, not an accident of nobody having written it a market.

**Why a stored value cannot be repaired in place.** A stored value **cannot be re-marked**, because the
number that produced it no longer exists. So *"what is this worth"* is answered by whatever happened to
be true when it was written: a cost, a par, a stale mark. Every identity that compares two subsystems is
then comparing two different **vintages** of that answer. And wherever units and value are stored side by
side, they drift — they are equal only at the instant they are written, and every reader between two
writes gets a stale product with no indication that it is stale.

**What this forces, and why it is a build rather than a refactor.**

1. **Assets with no units at all** — plant, dwellings — cannot be priced until it is decided what a unit
   of them **is**. That decision is load-bearing and unavoidable.
2. **Inventory carried at cost, against inventory bought at cost, in a market that has moved,** is a real
   holding gain or loss. That is a **new mechanism**, not a refactor: it needs a basis, a lower-of-cost-
   and-market rule, and a write-down that is a charge to income on a date.
3. **A price a market computes and discards** must be stored per (market, instrument, period), so that
   next period can re-mark against it.
4. **A cache re-derived once per period is one step short of the requirement.** If a stored value is
   refreshed from `units × price` at each close, it is a cache — better than an independent number, and
   still not a function.
5. **The registry must be what the mechanism reads.** How an asset is quoted is declared once; every
   venue reads that declaration. Declaring it in one place and hard-coding it beside every venue is two
   representations, and nothing can catch the moment they disagree.

**It must not be output-identical.** The moment value becomes units times a cleared price, **every balance
sheet moves**, because the values it replaces are costs, pars and stale marks. Capital moves, ratios move,
net asset values move, and identities that have been quietly comparing a cost to a mark start failing.
**That failure is the finding.** Seeding at par to keep the first period unchanged preserves the defect and
proves nothing.

**The order this is done in** is from hardest to easiest, because the hardest class is the one with the
most parallel representations to collapse: sovereign, then corporate credit, then equity, then inventory
(the new mechanism), then goods, then plant and dwellings (which need their units defined first).

---

## XI-7. THE BENCHMARK PROBLEM

**The mechanism.** The things everything else prices off must themselves be **prices**.

There are three, and each contaminates everything downstream of it:

**One index system, built from constituents.** Two index systems — one computed from real constituents
and read by nobody, one a stored level moved by a delta and read by everything — is law 4's defect at the
level of the market's own benchmark. And an index's **opening history must not be a random walk**: every
covariance measured against that history is a covariance against noise, and a covariance against noise is
a **discount rate** wherever a beta is used — in equity valuation, in loan pricing, in a wage decision, in
a freight decision.

**The floating benchmark is a transacted rate.** A cleared overnight rate exists in this world; that is
what floating coupons fix on. **A posted policy rate is not a benchmark.** Fixing coupons on an
administered rate means the corridor is decoration, the money market's own price is unused, and a named
reference on an instrument is a label nothing prices off.

**Producer prices and consumer prices are two indices.** One index wearing both names cannot show a margin
squeeze — input prices rising faster than output prices — which is most of what a cost shock does to a
firm. Real growth deflated by the wrong index is wrong in the same direction every time.

---

## XI-8. THE ESTATE AND THE WATERFALL

**The mechanism.** A party dies. An estate opens. Its assets are **sold into real markets to real
bidders**. Its claims are **ranked**. The proceeds are distributed in rank order. What is left over is
loss, and it lands on named holders. The estate closes.

**The four things that make it real rather than an accounting step:**

**Assets are sold, not valued.** Inventory is offered into the market its goods always sold in. Plant is
offered to bidders who value it for what it can produce **for them** — capital of the wrong kind is worth
less to a buyer that cannot use it, and a slice nobody can use draws no bid. What no bidder takes by the
programme's last period is abandoned or perishes. **A formula discount off book is a stated price with no
buyer.**

**Every claim ranks, and the ranking is honoured by the payout.** Secured, senior, subordinated, trade
creditors, counterparty close-outs, equity last. Two failures are common and both bias recoveries:
ranking by instrument **type** rather than by the instrument's own stated seniority (which makes
subordination decorative and means a subordinated bond can never trade wider than a senior one); and
letting the estate **collect** the dead firm's receivables as an asset while its own trade creditors rank
nowhere (which biases every recovery upward by exactly that asymmetry).

**The real-economy consequences are part of it.** Employees are released — through the labour market's own
separation path, not by decrementing a count. Suppliers lose their receivables, and that loss can push
*them* into distress, which is a contagion path running along the supply network rather than through the
banking system. Capital goes to a buyer.

**It conserves and it terminates.** Recoveries plus losses equal the assets at realisation, exactly. Every
currency the party held is paid away. A dead party with no claims still opens an estate rather than keeping
its cash for ever. Every reference to the party resolves to the estate or a successor.

**The clearing house's version.** A member defaults; the house closes its contracts out at the mark; the
survivors are paid in full and get their margin back; what the defaulter owed **net across all its
contracts at that house** is absorbed in a stated order — its margin, its fund contribution, the house's
own capital, the survivors' contributions written down pro rata — and **past the end of that order there
is nothing**, which is a real event. What the defaulter's own money did not cover is the house's
**unsecured** claim on the estate, ranking with the others.

---

## XI-9. THE SOVEREIGN'S FUNDING CONSTRAINT

**The mechanism.** The treasury raises money **before** it spends it, from a market that must clear.

**Why the automatic overdraft is the single most consequential thing to get wrong.** If the treasury's
account can go negative against the central bank, and the next issue is sized to clear that negative,
then:

- **Causation reverses.** The treasury spends into the overdraft and issues to clear it, rather than
  planning a programme and issuing against it. The forward-looking funding plan has nothing to do.
- **The cash buffer is unnecessary**, so the reason to hold one disappears.
- **A failed auction costs nothing**, so the auction carries no information and its participants face no
  consequence.
- **A sovereign cannot fail**, so its paper is risk-free by construction, so nothing prices its credit,
  so its rating has no consumer, so the whole assessment system above it is decoration.
- **The interest round-trips**: the cost of the overdraft returns to the treasury as remittance in the
  same period, so even the price of it is zero.

**What replaces it, in two parts.**

*First, the account is a balance like any other.* The programme is sized **forward** against redemptions
and outlays. The buffer exists and is a real holding. A shortfall is a real event with real handling: pay
from the buffer, cut or defer an outlay, come back at a different size or maturity.

*Second, a sovereign can default.* A missed-payment definition. An exchange offer with holdouts. Market
exclusion as the sanction. And then the rating gains its first real consumer.

**Sequencing.** Everything priced over the sovereign curve assumes a borrower with a funding constraint. A
benchmark issued by a borrower that cannot fail is not a benchmark for credit.

---

## XI-10. THE EMPLOYMENT RELATIONSHIP

**The mechanism.** Employment is a **relationship**, recorded: a firm, a worker or cohort, a wage, a start
date. The wage bill, the unemployment rate and the separation flow are **reads** over that record.

**What a headcount cannot do.** If a firm's employment is an integer and a worker is a fraction spread
across occupations by a fixed sectoral mix, then a hire and a separation are additions to a count. And
then:

- there is **no contract** for stickiness to be a consequence **of** — so stickiness becomes a coefficient
  damping a series, which is exactly the shape the method forbids;
- there is **nothing a severance payment could sever**, so firing has no cost, and the asymmetry between
  hiring and firing — where the employment cycle comes from — is a pair of adjustment speeds;
- there is **no household that can be told its earner lost a job**, so the channel from labour to household
  credit is missing at its source;
- a **quit** and a **vacancy withdrawal** have no owner: a posting is something an employer holds, and a
  quit is something a worker does to a specific employer.

**And the market must clear on the wage.** Every posting is a bid at the wage the employer offers; matches
go to the highest bids first, pro rata within a tie; the bid that took the last match is the print. A
single fill ratio applied identically to every employer means an offer well above the going rate fills the
same share as one well below it — which removes the price from the labour market entirely.

**Supply moves.** What one occupation's search leaves unmatched can flow to what another leaves unfilled,
through the same matching function, capped on both sides, with movers entering at the bottom because
retraining costs something. That is **people moving**, and it is slower than own-occupation search by
construction. A coefficient that drifts occupational shares toward a wage gap is a price being read where
a person should be moving.

---

## XI-11. RISK TRANSFER NEEDS A TRANSFEREE

**The mechanism.** Credit risk moves from the bank that originated it to somebody else, **named**, who
then takes the loss.

It needs four things and each is a real object: a **vehicle** that is a party holding the loans; a
**tranche** instrument with a stated loss attachment and a **cleared price**; a **waterfall** that
allocates real losses to real tranches; and **named holders** of each tranche.

**Why its absence is structural rather than cosmetic.** Without it, small-business and mortgage credit risk
sits on the originating bank **for ever**. A bank's origination capacity can never be expanded by selling
risk, so the lending-capacity channel does not exist. And the event this system exists to produce —
correlation worse than the tranching assumed, so the senior tranche takes losses it was not supposed to and
**every holder is hit at once** — has no holders to hit.

**Its two prerequisites, in order.** It needs XI-1 first, because tranching a loss *rate* yields senior
notes that can never be touched. And it needs loans to be **rows in a register** before they can be
transferred at all: a loan that exists as a field on a lender's balance sheet cannot be sold, pooled,
pledged or matured, however true the field is.

---

## XI-12. EVERY PAIR CLEARS ON ITS OWN FLOW

**The mechanism.** Each currency pair clears on the flow that actually crosses it. Triangular consistency
is an **outcome** that bounded arbitrageurs enforce — and may, at their limits, fail to enforce.

**The half that is easy to leave undone.** Clearing every pair in the market and then, at the ledger,
promoting only the legs against one currency and triangulating every conversion through it, restores the
vehicle currency **by construction**. The market half is then decorative: the arbitrage has no consequence
and cannot be measured.

**The three companion pieces.**

- **The forward rate carries the interest differential.** A forward struck as spot moved by a basis, with
  no differential in it, is neither cleared nor at parity — so nothing can be checked against parity, and
  carry is absent from the instrument.
- **There is one basis.** A cleared funding basis and a second basis on a random walk, with the second
  being the one participants see, trade and book against, is the same defect as two index systems.
- **The swap and the cross-currency swap exist.** Without them the forward never delivers, and the banks —
  the participants who need this market most — are not in it.

**One convention for what a payment settles in.** A purchase settles in the seller's money; a party short
of that money buys it. A conversion of a price into the buyer's money **inside** a trade has no
counterparty on the other side of it: the buyer is never short, no order is placed, and the currency demand
the trade should have created disappears. And a convention that depends on **who** the buyer is means the
same purchase lands the flow in two different places.

---

## XI-13. THE MARKET MUST BE ABLE TO DISAGREE WITH THE MODEL

**The mechanism.** A traded price is a **second opinion**. For it to be one, the participants' reservations
must be formed from something other than the quantity the price is supposed to reveal.

**Three shapes that quietly delete the disagreement:**

- **A probability computed from the accounts and fed to every seller.** Then the credit derivative's spread
  is a restatement of the accounting model, in the one instrument whose entire purpose is to hold a
  different view. **The implied probability is a read from the cleared spread**, never an input to it.
- **A book with two participants and both of them hedgers.** If every buyer of protection is a bank above
  an exposure limit and every seller is closing a regulatory gap, then the cleared spread is a function of
  regulatory gaps and never of a view; a period in which neither gap binds does not open the book at all;
  and the price cannot move because somebody thinks the credit is mispriced. **Every derivative book needs
  a participant whose reason is a view**, and a two-sided dealer posting into it.
- **One rating held by nobody.** An assessment that is a property of the firm rather than an opinion held
  by a named assessor means every participant agrees about credit by construction, which removes the
  dispersion the auction needs to have two sides at all.

**The general form.** Any mechanism in which the *input* to the participants' schedules is derived from the
same quantity the clearing is supposed to *discover* produces a price that is a fixed point of its own
formula. It will look like a market and it will carry no information.

---

## XI-14. WHAT A BOUND COSTS

**The mechanism this replaces.** Wherever a bound stands, ask: **what compensating mechanism is missing?**
Build that, and delete the bound in the same change.

**A catalogue of what specific bounds have deleted**, so the pattern is recognisable:

| The bound | What it deleted |
|---|---|
| Available credit floored at zero | The forced-sale path, entirely |
| A margin rate stated per class | Procyclicality — the contagion mechanism |
| A recovery rate fixed at a constant | The credit content of a credit derivative |
| A loss rate in place of a default | An event, a borrower, a recovery, and a distribution |
| A liability discounted at a constant | The defining risk of the pension sector |
| A constant NAV that cannot break | The one-way riskiness of a deposit substitute |
| Futures convergence enforced at expiry | Delivery, and the arbitrageur who makes it converge |
| A price bracketed by a solver's search bounds | The distinction between a price and a failure to find one |
| A cap on how far a rate may move in a period | The information in a large move |
| A fund's redemption rationed by its own cash | The forced seller, from the other direction |
| A stated spread applied to a mid | The dealer's ability to skew, widen or refuse |
| An unemployment rate written directly | Hiring, firing and matching, at once |

**Two rules about the deletion itself.**

*A bound covering a missing mechanism is deleted **with** that mechanism built, in one change.* That
pairing is the only sequencing allowed, and it is not a licence to defer.

*Deleting a shape parameter before its mechanism exists makes the model wrong, not more bottom-up.* The
order is forced: build, then delete. But the parameter must be **registered** in the meantime — with its
value, its unit, its owner, whether it is measured, estimated, assumed or a placeholder, and, for a
placeholder, **the mechanism whose absence it stands in for.** The count of placeholders is the honest
measure of how much model is missing, and it should be visible.

---

## XI-15. THE UNIT OF REPRESENTATION

**The mechanism.** A party is either **named** or it is a **cell**, and which one it is turns on a single
question: **is its idiosyncrasy load-bearing on its own?**

A bank's is. One bank failing is an event the rest of the world feels by name, through its lenders, its
depositors and its counterparties. A single household's is not. What matters about a hundred million
households is the **shape** of them and the **crossings** that shape produces — and a model that names
each one has paid for a hundred million identities to learn what a distribution would have told it.

So institutions, issuers, and every party whose own name appears inside an instrument are named
individually. The household and small-firm sectors are **cells**: each a named party in the ledger, with
an account, a register of holdings, and a **WEIGHT** — an integer count of how many real parties it is.

**A cell is not an average.** It is one *possible* household, carried with a multiplicity. That
distinction is the whole of this mechanism. An average of a group has no state a threshold can cut; a
possible member with a multiplicity has exactly the state its members have, and the threshold cuts the
ensemble where it really cuts.

**A cell is therefore HOMOGENEOUS, and that has arithmetic consequences.** Every member has exactly
the cell's state, so:

- its account holds **weight × the member's balance**, and its holdings are **weight × the member's
  holding**;
- every movement to or from it is **weight × a per-member amount** — a payment, a coupon, a wage, a
  purchase, a loss;
- both are therefore **always divisible by the weight**, and that divisibility is an invariant of Part
  XII's Units family, checked every period. A cell holding an amount no member could hold has stopped
  being one possible household and has become the average this mechanism exists to forbid — and it
  drifts there silently, one indivisible payment at a time, unless something checks.

**Part of a cell: a SPLIT, never a fraction and never an average.** A hire, a separation, a default, a
cohort boundary, an inheritance — many events apply to *some* members. They **split the cell**: the
affected members become a new cell with the same state and the new relationship, and the split is
**exact**, because identical members divide without remainder. Two cells that have arrived at an
identical key and an identical state **merge**. Both are register events with a cause and a date, like
any other.

So **a weight changes by five events**: **entry**, **death**, **promotion**, **split** and **merge** —
and by nothing else. It is still never a share, never a scale factor, never a probability. Without
split and merge only two answers are available for a partial event, and both are wrong: move the whole
cell, which quantises the world to the weight, or carry a headcount inside the cell and let its members
differ, which is an average one level down.

**The rule that makes it legal: aggregate AFTER the nonlinearity, never before.**

> `Σ f(xᵢ)·wᵢ` — never — `f( Σ xᵢ·wᵢ )`

Every decision, every threshold, every default test is evaluated **per cell** and then summed weighted.
Taken the other way round, a mean-preserving spread causes nothing at all — which is exactly backwards,
because widening dispersion at constant mean is what a downturn does (XI-1), and the sector's entire
credit content is gone.

The strongest way to hold this is to make the average **unreachable**. A cell that answers *"integrate
this function over me"* and does not answer *"what is your mean"* cannot be asked the wrong question. A
rule that can be broken silently will be; a question that cannot be phrased will not.

**What must still be named, and how it survives.** A cell does not weaken *"1$ is 1$"*, because **every
relationship that must be named is either a dimension of the cell's key or a register row, and never
an attribute averaged inside it.** A relationship averaged inside a cell is a relationship the model
cannot name, and law 4 is broken quietly rather than loudly.

**Which of the two a relationship is, is DECLARED.** The key's dimensions are **registry data**: at any
time the cell key is what the registry declares it to be, and lifting a relationship from a row into
the key is a data change and a re-stratification event, **never a change to a mechanism**. That is law
15's targeted-change test applied to the representation itself, and it is why the choice below is a
starting point rather than a commitment.

- **Declared as key dimensions:** region, cohort, and the party's **bank**.
- **Carried as rows:** employment (a firm, a cell, a wage, a start date, a headcount — Labour A4.a), a
  loan per (lender, cell), a tenancy or a mortgage per (lender, cell, dwelling), and trade credit
  between cells and named firms, so receivables still sum to payables exactly (Trade Credit C4).
- **A row's headcount is always a whole cell's weight**, because a partial event split the cell first.

The reason not to lift everything is arithmetic: a key that is the cross product of every relationship
a household can have is a cell count that grows multiplicatively and is bounded by nothing. The reason
to be able to lift anything is that an unlifted relationship the model needs to *stratify* on — rather
than merely to name — is a finding, and the fix must not be a rewrite.

**The boundary, and crossing it.** A weight of **one** is a named party — so the line between the named
and the represented is not a modelling decision but a **size**, and it moves. A cell that outgrows bank
dependence is **promoted** to a named firm (Small-Business Pools A6.c), and that promotion is an event
with a cause. Without promotion the boundary is arbitrary, nothing can grow across it, and the sector's
composition is constant by construction — which Small-Business Pools E4 forbids.

**Resolution is measured, not asserted.** How many cells stand for a population is a **RESOLUTION
parameter** under law 2, and law 2 says what to do with one: **change it and the answer must not change
materially.** Run the same seed at one, two and four times the cell count. If the aggregates move, the
resolution is too coarse and the finding is the resolution. This is what turns *"is the representation
good enough?"* from an argument into a measurement — and it is the bound on everything below.

**What it costs, and the cost is declared.** Per Appendix C, an approximation stays in the document with
its reason rather than being quietly dropped. Three things are genuinely lost, and each is registered
with the mechanism it stands in for:

- **Within-cell network structure.** Two small firms in one cell cannot trade differentially with each
  other, so the supply-network contagion of Trade Credit D3 runs cell to cell rather than firm to
  firm. Coarser, not absent — the chain still runs and is still traceable.
- **Discreteness of crossings.** A cell's default moves its whole weight at once, so losses arrive in
  multiples rather than one at a time. Bounded by the resolution measurement above.
- **Unlifted relationships.** Any relationship that is neither a key dimension nor a row is not named.
  That is a defect wherever it happens, and the fix is to lift it — into a row, or into the key by the
  data change above — never to average it.
- **Per-person facts are held per member, and that is exact rather than lost.** A limit, a vote, a
  claim, a record: because the cell is homogeneous, *balance ÷ weight* is the member's balance
  **exactly** and no mean is taken. Deposit insurance up to a limit is `weight × min(member balance,
  limit)`; a vote is `weight` votes; an heir is a named heir cell; a credit record is the cell's.
  What **is** lost is dispersion *within* what a cell stands for — every member of one cell is insured
  to the same extent and votes the same way — and the answer to that is resolution, which is measured
  below, or a split, which is exact.

**Where it reaches.** Households A2.e–g; Small-Business Pools A6 and E5–E6; labour, through the cohort
in the employment register (Labour A4.a–b); housing and the mortgage book, where the loan-to-value
cross-section **is** the distribution the lender's standard reads (Housing C5.a); consumer credit; and
the small tier of trade credit, which is the tier that lives on it (Small-Business Pools A4).

**Sequencing.** Early — with the register, not after it. A cell **is** a party, and a weight retrofitted
into every mechanism that touches a household or a small firm is a rewrite rather than a refactor.

---

## XI-16. PERSONAL EXPECTATIONS

**The mechanism.** Every decision in this world is taken by a party looking forward, and what it sees when
it looks forward is **its own past**. A household deciding what to spend, a firm deciding what to make, a
desk deciding where to quote, a treasury sizing an auction, a bank deciding whether to lend to a name — each
holds an **outlook** for the variables it will act on, formed from what it observed, corrected by how wrong
it was last time, at a speed that is its own.

**Why it is personal and not global.** The real economy runs on a hundred million forecasts that disagree,
not on one that is right. The disagreement is not noise around a true expectation; it is **the** thing
that makes the mechanisms of this document work. A market has two sides because two parties expect
different prices (Clearing A1.a). A mean-preserving spread of outlooks produces crossings where a single
outlook produces none (XI-15, §41 A2.g). A downturn is felt first by the parties whose surprises were
largest and reaches the rest through their actions — which is a **transmission**, and a model with one
expectation has nothing to transmit through.

**The trap this mechanism exists to avoid** is the one XI-13 names for prices, arriving one level up.
*Rational expectations* — an outlook computed by running the model forward and handing every party the
answer — makes each decision a function of the model's own prediction of the consequence of that decision.
It looks like foresight and it is a fixed point: nothing can be surprised, so nothing adjusts, so the model
converges to whatever it was told to expect. The specification's answer is the same as XI-13's: the input
to a decision must be formed from something other than the quantity the decision is supposed to produce.
Here that something is the party's own history.

**The fewest primitives.** One. A party's **memory** — how many of its own periods it weighs — is a
PREFERENCE, dispersed across parties and drawn once at entry. Everything else is a read:

- the **outlook** is last period's outlook corrected towards what was observed, at the memory's speed;
- the **surprise** is observed minus expected, and it is a recorded event;
- **confidence** is the width of a party's recent surprises — a party surprised often does not trust its
  outlook, holds more liquidity and acts more cautiously, and nobody wrote a confidence coefficient;
- the **aggregate** — a sentiment index, a survey of expectations — is `Σ eᵢ·wᵢ` published with a lag
  (§45 A5), and it causes nothing (§45 B2.a).

There is no optimism parameter, no anchor, no target, no animal spirits. Each of those is either a read
of the surprises or a number nobody could derive, and law 2 forbids the second kind.

**Why it comes early.** The consumption decision (§41 C1) is the first decision in the build order that
reads an outlook, and it arrives with the firm's real cost base (Part XIII, step 4). Everything decided
after that — output, hiring, investment, the desk's view, the lender's view, the programme, the vote — reads
one too. An outlook retrofitted after those decisions exist is a rewrite of every one of them; an outlook
built first is a field they read.

**What it repays.** Two shapes the register carried from M2: a participant's *view* of a line and the
*drift* of that view between periods. Under this mechanism the view **is** the participant's expectation
of the price and the drift **is** the surprise it took — the numbers that were declared as SHAPE become
reads, and the debt is paid in full rather than narrowed.

---

## XI-17. THE MANDATE

**The mechanism.** A POLICY primitive is a number an institution chooses (law 2), and *chooses* is a verb
with a subject. This mechanism gives the fiscal and regulatory primitives their subject: a **parliament**,
elected by the household cells from their own outlooks, whose seat-weighted platform **is** the value in
the register.

**Why a polity at all.** Treasury B3 says outlays have *"causes that vary: the cycle, unemployment,
policy"*, and B3.a says that variation is *"the whole reason the constraint in D bites when it does."* Two
of the three causes arrive with labour and the real economy; the third is this. A model in which policy is
a schedule has removed the one cause that responds to the other two: a downturn that raises transfers and
lowers receipts, and then changes who governs and what they spend, is a loop with a period of four years —
and a world with the loop cut has a fiscal stance that nothing inside the world can move.

**The fewest primitives.** Three constitutional POLICY primitives and one data file:

- the **seat count**, one number;
- the **term**, one number, placed on the calendar by date;
- the **allotment rule** — votes to seats — one rule, stated;
- the **platforms**, one row per party: a position on every primitive the parliament controls. Data, not
  code; adding a party is a row.

Everything else is a read. The **vote** is each cell's own decision — which platform, applied to the cell's
own state at the cell's own outlook, leaves it best off — summed weighted. The **government** is the
coalition one stated rule assembles. The **mandate** is the seat-weighted platform of that coalition, and
the register's fiscal and regulatory primitives are set to it and to nothing else. Turnout is what it is
because abstention is a decision (a cell indifferent between every platform has nothing to vote about);
there is no turnout parameter, no swing, no loyalty, no bloc.

**What the parliament owns, exactly.** The POLICY primitives, and only those: tax rates on named bases,
transfer rates, the outlay programme's size and composition, the treasury's buffer, the regulatory ratios
and floors, and the central bank's target. It does **not** own the central bank's rate (§31 A4), and it
does **not** own any price, quantity or outcome (§47 D3.a): a mandate that named an interest rate or a
growth target would be a written path with a majority behind it. Every POLICY primitive in the register
names its owner — the parliament, the central bank, or a standard-setter — and the register prints the
owner beside the value, so *"who set this number"* is a read and never a question.

**How it reaches the world.** Through the mechanisms and never directly. A new mandate changes the numbers
the treasury's programme reads; the programme changes the need; the need changes the auction; the auction
changes the curve; the curve changes the cost of capital. Each step is one this document already specifies,
and the election adds no new channel — it moves the primitives at the top of the chain and lets the chain
run. An election is an event on the observer surface (§45 B1), and like every event it is the report of a
change of state, not its cause.

**Why it comes late.** A vote reads the cell's employment state (XI-10), its outlook (XI-16), the prices it
paid (Goods C) and what it owns. It can only be built once those exist, and it is placed after employment's
other half in Part XIII for that reason. Until then the fiscal primitives have a stated owner — a standing
mandate declared at the seed — and the register says so.

---

# PART XII — THE MEASUREMENT PROGRAMME

## What measurement is for

**Build the whole model, then measure.** Numbers taken while the model is incomplete describe an economy
that does not exist. Until then, only the cheap deterministic checks run, and they are **gates, not
experiments**.

When the model is complete, measurement has exactly two jobs:

1. **Confirm the invariants hold.** These are pass/fail and their tolerance is arithmetic dust.
2. **Read the VERIFY nodes.** These are not pass/fail. **A VERIFY that fails is a finding about a
   mechanism, never a licence to adjust a number.**

**A number that is still wrong at the end is a missing mechanism named at last, not a tuning target.**

## The invariant families

Each is pass/fail, every period, at a point where the state is meant to be consistent, and each names the
party and the size when it fails:

- **Money** — every payment has both sides; the sum over all accounts changes only by an act of a money
  issuer; the money stock's change equals bank lending plus central-bank action and nothing else.
- **Ownership** — holdings sum to issued, per instrument; a claim beyond its instrument's face is somebody's
  claim invented, and a shortfall is somebody's claim vanished.
- **Prices** — everything anyone marks has a price that came out of a mechanism; a fitted curve is measured
  against the points that actually cleared.
- **Cross-market** — the same economic thing has one value however it is reached.
- **Accounts** — assets minus liabilities equals equity, for every entity, read and not stored.
- **Names** — every party referenced exists; every issuer of a held instrument exists or has a successor.
- **Flows** — every flow leaving one place arrives somewhere named, in the same period, in the same
  currency; for every asset kind, instructions in minus out equals the change in holdings.
- **Zero-sum** — derivative marks sum to zero per contract and in aggregate; variation margin paid equals
  received.
- **Units** — produced plus opening equals consumed plus closing, per good and per location; the same
  identity for dwellings, for plant, and for anything else counted in physical units — **including a
  population: the sum of cell weights equals the population it stands for, and every represented party
  sits in exactly one cell.**

**Independence is itself a measurement.** One defect should light **one** family. A defect that lights five
means the families overlap and none of them is telling you where to look.

## The VERIFY nodes — measurements, not targets

Every VERIFY in Parts III–X is a standing read. They fall into four groups.

**Conservation and reconciliation** — things that must hold, and whose failure names a lost leg:
receivables against payables; mortgage debt owed against mortgage assets held; sector holdings against
issued amounts; exports against imports party by party; a region's current and financial accounts;
recoveries plus losses against assets at realisation; sources against uses in a buyout; the money paid to a
target's shareholders against what the acquirer and its lenders put up; a resolution's conservation across
acquirer, insurer, estate and holders; every population's states summing to its population.

**Reads that must not be stored** — each of which is a place a second representation likes to appear: the
money stock; market capitalisation; net asset value; net worth; a sector aggregate; an index level; a
current account; a loan book; capacity utilisation; an unemployment rate; a debt-service burden; a
rating distribution; a party's confidence; an aggregate of expectations; an approval rating; the mandate.

**Behavioural consequences** — the ones that say a mechanism is doing its job:
worse credit trades wider; junior trades wider than senior within one issuer; stronger names fund cheaper;
declined volume is visible; the constraint that binds differs by bank and by period; a bank near its
capital line behaves differently; a bank with no maturity gap is not a bank; flows into money funds rise
when their yield beats deposits; a persistent premium or discount on a traded fund is a liquidity finding;
inclusion in an index shows in the constituent's price; small imbalances in an inelastic market produce
large price moves; a seller with no buyer keeps what it has; unemployment and vacancies move against each
other; volumes collapse before prices in a falling housing market; a firm trading cheap attracts bids;
expectations lag turning points and the lag differs by party; the dispersion of outlooks widens in a
downturn; a change of government shows in the deficit through named outlays and named receipts; a
mean-preserving spread of incomes changes the seat count while the mean does not move.

**Causal chains** — the ones that test transmission rather than a level, and therefore the ones worth
naming individually:

| The chain | What its failure means |
|---|---|
| A credit tightening reduces investment through the cost of capital, then output, **with the build lag** | XI-4's joints are not closed, or the lag is not real |
| A rate move shows as **liability, hedge and cash in three different places** for a pension | The liability has no duration, or the hedge moves no cash |
| Low inventories imply backwardation | The curve is not reading the physical state |
| Heavier issuance moves the sovereign clearing price, then the cost of debt with a lag | The auction is not reading its own size |
| The fiscal balance and private net saving move together | A flow lost a leg |
| A commodity shock reaches **margins, then inflation, then policy, in that order** | A link has been short-circuited; something reads the shock directly |
| One defect lights one invariant family | The families overlap |
| A shock a firm hedged is felt less by that firm | The hedge is a number, not a position |
| A swap spread and a credit-derivative basis behave differently calm and stressed | Neither is cleared, or both read the same input |
| Futures converge at expiry **because delivery is possible** | Convergence is enforced, not produced |
| A rate rise reaches consumption through floating mortgages **and** through prices, with different lags | One of the two channels is missing |
| A run at one bank is information about the others | Observability is not wired |
| The loss chain from one fund to one bank to other funds is traceable party by party | Somewhere a loss stopped without a holder |
| A downgrade causes selling, capital pressure and funding loss, and worsens the state that caused it | The rating reads the price, so the loop is a tautology |
| A shock reaches the parties it surprised first, and everyone else through their actions, with a lag | There is one expectation, so there is nothing to transmit through |
| A downturn raises transfers and lowers receipts, then changes who governs, then changes what is spent — over a term | Policy is a schedule, and the loop is cut |

## Standing observations worth keeping

Some quantities are worth watching continuously without ever being fixed. They are **measurements about the
model**, not defects with owners:

- **Unowned money.** Any amount that is nobody's is a defect at its site, not a tolerance to widen. A new
  line here is a regression.
- **Population resolution.** The same seed run at one, two and four times the cell count (XI-15). The
  aggregates must not move materially, and the size of the move **is** the honest error bar on every
  number the represented sectors produce. A move that grows with resolution is a finding about the
  ensemble, never a reason to prefer the coarser run.
- **Cost growth over a long run.** If the per-period cost of stepping the world rises as the run goes on,
  something is accumulating that should not be — most likely rows that are never retired.
- **Refusals.** What the markets struck beyond what their participants could actually margin, cut at the
  point of admission. Non-zero means a market sized its demand to the wrong constraint; a participant cut
  every period is one living at its limit. **Measure; do not raise the limit.**
- **Banks paying depositors above the policy rate.** Not necessarily a defect: a stressed bank buying
  funding is a real thing. Count the banks and the periods; never band it.
- **Concentration and dispersion.** A sector whose members are converging is a sector losing the
  heterogeneity that makes its market a market.
- **Where a market is thin.** Depth is an outcome of who is in the room. Measure it; do not tune it.

## The run ladder

Three lengths, each answering a different question:

- **A short profile run** — for performance work only. It measures cost, not correctness.
- **A working run of about a season** — long enough for the flows to have circulated and for the first
  maturities to arrive, short enough to iterate on. **A short probe samples one season; price behaviour is
  judged on whole years.**
- **A long run with shocks** — the close. Long enough for accumulation, for a full set of rolls, and for the
  slow loops (housing, capital, ratings) to complete a cycle.

Alongside them: **a fixed-point run.** Step the model with nothing exogenous changing and see where it
settles. The world at its own fixed point exposes rules that are wrong on their own terms in a way no
ordinary run does, because there is nothing else moving to hide them.


---

# PART XIII — SEQUENCING

## The dependency order

Nothing here is a schedule. It is a statement about **which mechanism must exist before which**, and the
reason in each case.

1. **Money and settlement.** Everything's cash leg lands here. And **one calendar** (Money G3), because
   every instrument's dates and every rate's periodicity are placed on it, and a second clock arriving
   later is law 4's two writers pointed at time.
2. **The register and the clearing mechanism.** Everything's ownership and price. Three things settle
   **here, with them**, and not afterwards:
   - **the unit of representation** (XI-15), because a cell is a party in the register and a weight
     retrofitted afterwards is a rewrite of every mechanism that touches a household or a small firm;
   - **delivery versus payment** (XI-5) and **value as a function** (XI-6), which are properties of what
     a register and a price store *are*, not additions to them — a book that can half-settle, or that
     stores a value, does not become correct by being fixed later;
   - **the parameter register** (XI-14), because the seed is the first thing that must state a number it
     cannot derive, and a debt undeclared at the moment it is incurred is a debt nobody pays.
3. **The sovereign's funding constraint** (XI-9), because the benchmark curve must be issued by a borrower
   that can fail before anything priced over it means anything.
4. **A firm's real cost base** — named cost lines with real payees, receivables as the sum of the invoice
   book, a baseline that is measured history — **and the households and labour it pays and sells to**,
   **and the outlooks they all act on** (XI-16). The consumption decision is the first that reads an
   expectation, and an outlook retrofitted after the decisions exist is a rewrite of every one of them.
   This step was written after (5) and (6) below and is now before them, for a reason its own systems
   state: a firm *"is subject to firm fundamentals from the first period"* (§34 B1); a borrower is
   *"a named legal entity with a balance sheet that can make a promise"* whose service is covered by
   *"operating cash flow"* (§7 A1, A3); an assessment is read from *"leverage, coverage, cash, size,
   sector, age"* (§44 A2); and XI-1's borrower *"crosses a threshold"* that does not exist until there is
   a cash flow to cross it. A loss needs somebody to be losing something.
5. **A loss is an event** (XI-1), because at least four systems need "a claim goes unpaid" to be a thing
   that happens — and now there is a party for whom it is one.
6. **Loans are rows**, because a loan must be a row before it can be transferred, pooled or sold.
7. **The forced seller** (XI-2), **nothing is immortal** (XI-3) and **the estate** (XI-8) together,
   because a chain needs a push, a place to terminate, and something at the end that distributes what is
   left. XI-8 was written at (12) and is here: a cascade that reaches a party with no estate stops
   without saying so, which is the defect XI-3 exists to prevent.
8. **Redeemable claims**, which is the other door into the forced seller.
9. **Equity, and dealers that carry inventory.** A share must exist and have a price before (10) can ask
   what equity costs, and a dealer must have a limit before a market can fail because it stepped back.
10. **The cost of capital** (XI-4), which is what makes every cleared price above actually do something.
11. **Bank capital that can be raised**, which needs a price for the raise, which is (9) and (10).
12. **The currency layer** (XI-12), the **benchmarks** (XI-7), and the **second opinion** (XI-13).
12a. **Reporting and estimates** (§48). It goes here and not earlier because the surprise it exists to
    produce is only observable against a share price that is not walking — which is (12)'s anchor — and
    because a bank publishes an estimate as §44's sibling, on the surface (12) builds for a named
    assessor's opinion. It goes here and not later because a tender offer (13) is priced off a target's
    reported earnings and the acquirer's own view of them, and because the income statement it reads is a
    thing every system after it wants.
13. **Employment's other half** (XI-10), housing's other half, **securitisation** (XI-11), and the
    corporate-control market.
14. **The polity** (XI-17). A vote reads a cell's employment state (13), its outlook (4), the prices it
    paid and what it owns; it can only be cast once those exist. Until then the fiscal primitives have a
    stated owner — a standing mandate declared at the seed — and the register says so.
15. **The recipe.** Last, deliberately: changing input-output relationships moves **every** quantity in the
    model, so it must land against a stable measurement, and the comparison across that change is the whole
    point of it.

And then, and only then, measure: Part XII, in full.

## Two sequencing rules that are easy to break

**Later work depends on earlier work.** A step taken out of order is built against a world that has not
arrived yet, and the reader afterwards cannot tell which half of it is true.

**A prerequisite is inserted, not appended.** When something turns out to be needed first, it goes **before**
the thing in hand, and where it landed is stated.



# APPENDIX A — CONVENTIONS

**Units.** Every quantity carries its unit: face, shares, physical units, contracts, dwellings, hours, floor
area, and money **in each currency separately**. The only route from a quantity to a value is
`quantity × price`.

**Money naming.** A figure in its owner's own money, a figure whose currency is named beside it, a figure in
the reporting numéraire, and a figure in some *other named party's* money are four different things, and the
identifier says which. A figure whose currency must be inferred from where it was found will be inferred
wrong exactly when it matters.

**Periodicity.** Every rate, flow and index names its period in its identifier, confirmed **at the point it is
written**, not where it is read.

**History and lag.** A quantity whose newest entry is through an earlier period than the reader expects must
say so. A reader that wants "this period's" and is handed last period's, silently, has a stale read that
looks like a fresh one. A legitimate lag and an accidental staleness must be distinguishable by the
reader, without reasoning about what ran when.

**Keys.**
- A **firm** is its own identifier; its display name is a display name and an address, never a key.
- An **institution** is its own identifier.
- A **piece of paper** is the instrument it **is** — the individual issue for credit and for a sovereign,
  the issuer for equity, the fund for a fund share. **There is no bucket.**
- A **good** is its sub-unit; a **market in a good** is (region, sub-unit).
- A **contract** is its own identifier, and what a contract is *on* is keyed the way that thing is keyed
  above.
- A **cell** is what the registry **declares** its key to be — at present its region, its cohort and
  its bank — and every other named relationship it carries is a **register row** (XI-15). A
  relationship that is neither is a relationship the model cannot name.

**Populations and weights.** A **weight** is a **count** — how many real parties a cell *is*. It is
never a share, never a scale factor, never a probability, and it changes only by **entry**, by
**death**, by **promotion**, by a **split** and by a **merge** (XI-15). The population a sector stands
for is the **sum of its weights**, and that sum is a read. A cell is **homogeneous**: what it holds is
`weight × what one member holds`, so everything it holds is divisible by its weight, and every movement
to or from it is `weight × a per-member amount`. Every number a represented sector produces is
`Σ f(xᵢ)·wᵢ`; a number of the form `f(Σ xᵢ·wᵢ)` is a decision taken at an average and is a defect
wherever it appears.

**Missing values.** A quantity that is absent is **absent**, never zero. An unpriced instrument is *not
priced*, and whoever asked must handle that; a price of zero is a price, and it propagates. **Zero
multiplies.** A displayed number that does not exist is shown as missing, never as a formatted default.

**Parameter provenance.** Every number that shapes behaviour is declared with its value, its unit, its
owner, and which of law 2's five kinds it is: one of the three **primitives** — technology, preference
or policy — or a **resolution** choice tested by invariance, or a **shape**, which is a claim about the
answer. A shape with a **scheduled death** is a **placeholder**, and it names **the mechanism whose
absence it stands in for**. The count of shapes and placeholders is the honest measure of how much
model is missing, and it must fall.

---

# APPENDIX B — THE PROHIBITIONS, CONSOLIDATED

Every FORBID in this document, in one place. These are the requirements no review of an existing
implementation can produce, because an implementation cannot show you what it should not have. **A FORBID
that holds is as valuable as a mechanism that works, and it is the easiest thing to break silently.**

**Money and ownership**
1. No money without an issuer.
2. Two currencies are never added.
3. No holding without a holder; no holding without an issuer.
4. No move of value without a two-sided, numbered instruction behind it.
5. No overdraft that is a silent negative — somebody lent it at a rate, or somebody refused it and the
   refusal is recorded.
6. No conversion at the ledger boundary; no conversion without a counterparty.
7. The reporting numéraire is not where value lives.
8. No short by accident; no short without a borrow; no double-counting a lent security.
9. No collateral counted as available by both the poster and the holder.
10. No residual with no holder, in any currency, anywhere — including on a dead party.

**Prices and markets**
11. No participant is a price-taker of a price the mechanism has not yet produced.
12. No buyer of last resort by construction; the mechanism does not add demand to make itself clear.
13. Price is never derived from yield, spread, discount margin or OAS; nor from an earnings multiple, a book
    value, a discounted cash flow, or a target.
14. No price from a written path — not for a currency, not for a commodity, not for a house, not for an
    index.
15. No benchmark that is posted rather than transacted; a policy rate is never a market's cleared rate.
16. No index that is an input to its own constituents; no stored index level; no index without constituents.
17. A solver's search bracket is never a print; a market with no trades has no new print.
18. No spread applied to a mid; no stated spread table; a dealer that cannot lose money is not a dealer.
19. No underlying that exists only inside a derivative; no derivative settling against a price this world
    does not clear.
20. No forward rate from a parity formula; no fixed swap rate solved from a discount curve; no floating leg
    on a rate this world does not produce.
21. No free arbitrage left standing, and no arbitrageur without a limit.

**Bounds and tolerances**
22. No cap, floor, ceiling, clamp, damper or rescale standing in for a decision. The only admissible bound
    is arithmetic impossibility.
23. No percentage tolerance on an identity. A check that only passes with one is reporting a defect.
24. No fixed recovery rate; no fixed discount rate on a liability; no guaranteed constant net asset value.
25. No enforced convergence — a future converges because delivery is possible.
26. No investment rate; no exogenous earnings path; no exogenous unemployment rate; no birth rate; no
    default drawn from a hazard rate; no exogenous trade or capital-flow series.

**Institutions and failure**
27. No firm that cannot die; no fund that cannot fail; no bank that cannot fail for liquidity *and*
    separately for solvency; no clearing house that cannot run past the end of its resources; no sovereign
    that cannot default in a money it cannot create.
28. No death without a destination for every asset, liability, employee and contract.
29. A constant population — of firms, or of a pool — by construction.
30. No central-bank overdraft for the treasury, direct or by any facility amounting to one; no forced buyer
    in any auction.
31. No uncollateralised, unpriced, unlimited central-bank credit; a lender of last resort has all four
    classical conditions or it is a subsidy.
32. No unlimited counterparty exposure; no infinite dealer balance sheet; no desk exempt from its own bank's
    capital and funding.
33. No netting across counterparties.
34. No margin that is only a number; no exposure without margin or a stated reason there is none.
35. No leverage without a lender; no capital call that is not paid from a real balance; no buyout without a
    lender who agreed to lend; no exit at a price nobody paid.
36. Capital is never a pot that is spent.

**Structure and representation**
37. No representative agent for a sector whose decisions are thresholds.
38. No "loan book" number that is not the sum of loans; no stored aggregate anywhere.
39. No stored value beside units; no value that is not `units × price`.
40. No pool without underlying loans to named borrowers; no tranche without a holder; no risk transfer
    without a transferee.
41. No liability without beneficiaries; no solvency measured against a stored liability value.
42. No income to a holder who did not receive cash.
43. No sale that settles instantly by construction; no receivable without a named payer; no receivable that
    survives its debtor's death.
44. No employment without an employer; no wage bill without headcount.
45. No instantaneous, costless transport; no shipment without capacity; no goods in transit owned by nobody.
46. No consumption without production or inventory; no negative inventory; inventory never carried above
    cost for a non-dealer.
47. One cost is never in two places; one fact has one writer; one borrower has one default-probability
    model; one payment convention, owned in one place.
48. No branch on industry, sector, entity type or product identifier in the mechanics of the world.
49. No observed real-world equilibrium imported as a value; no outcome seeded.
50. No decision evaluated at an average; no threshold applied to a cell's own mean. A represented sector's
    numbers are the weighted sum of its cells' answers, never one answer to the sector's mean question.
51. No weight that is not a count; no represented party outside a cell; no cell without a weight; no loss
    allocated to a pool rather than to the cells that crossed. A weight changes by entry, death,
    promotion, split or merge, and by nothing else.
52. No cell holding what no member could hold. What a cell holds is its weight times what one member
    holds, every movement to or from it is its weight times a per-member amount, and an event that
    applies to part of a cell splits it. A relationship that is neither a declared key dimension nor a
    register row is not named.

**The observer**
53. No observer sees another party's private state; no privileged actor; no display-only number; no scripted
    narrative; **no surface that changes the model.**

**Expectations and the polity**
54. No global expectation; no expectation that is the model's own forecast; no expectation that reads the
    period's own result, another party's private state, or the model's internals.
55. No second primitive behind an outlook — no optimism, anchor, target or sentiment coefficient;
    confidence is a read of surprises. No sentiment as a cause.
56. No vote from an aggregate; no turnout, swing, loyalty or bloc parameter; no party that votes for a
    household; no approval rating as an input.
57. No policy set directly, and no policy path: a POLICY primitive the parliament controls changes by a
    mandate and by nothing else. No exogenous election result.
58. The parliament never sets a price, a quantity or an outcome, and never the central bank's rate.

**Reporting and estimates**
59. No reported number the books do not produce; no earnings that were not earned; no smoothing. Reported
    income is the equity account's movement over the fiscal period, decomposed into what the instructions
    and the marks did.
60. No consensus a decision consults; no stored consensus. An aggregate of estimates is a read, published
    with a lag, and it causes nothing.
61. No estimate derived from the share price; no estimate that is the model's own forecast; no analyst
    always right or always wrong by a fixed amount.
62. No price reaction rule — a stated move per unit of surprise is a written price path.
63. No universal coverage; no report with no consequence; no per-share figure that is a primitive.
64. No reporting calendar finer than a period, and none placed by a count of periods rather than a date.

**The method itself**
65. The audit never repairs.
66. No forecast without the measurement that would kill it.
67. No clause deleted from this document to make a comparison look better.

---

# APPENDIX C — HOW TO KEEP THIS DOCUMENT ALIVE

It stops working the moment either of two things happens, and both are gradual.

**It drifts from what exists.** Update it in the **same change** as the thing it describes. A stale
specification is worse than none, because it is still trusted. Where a requirement is met, say where; where
it is not, leave the requirement standing and say that.

**A requirement is softened to make the comparison look better.** If the model deliberately does not have
something, **the requirement stays and says so, with the reason**. Distinguish *missing* from *out of scope*
— they are different answers and only one of them is work.

Two further habits that cost little and pay a great deal:

**Re-mark in the same change that closes something.** A structural check can prove a citation resolves; it
can say nothing about whether an assessment is still *true*. So a requirement met by a change that did not
re-mark it stays marked unmet for ever, and nothing fails. Re-mark as you go, and **recount rather than
adjust** any tally.

**A requirement's history is not this document.** This is a specification, not a diary. What a change found
and did belongs in the project record; what is *true* belongs here.

---

*Project Phoenix — the model, and nothing about how to build it.*
