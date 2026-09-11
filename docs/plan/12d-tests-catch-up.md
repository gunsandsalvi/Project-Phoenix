# Item 12d — The tests catch up with the world

**Read this part first — and read `docs/PLAN.md` §7 before anything else.** This item is the one
place where the answer to a red test is allowed to be "change the test", and it is dangerous for
exactly that reason. A test is not wrong because it is red. It is wrong when it asserts against a
world that no longer exists — an amount out of a seed that no longer states one, a party the draw
now names, a period found by running a world that now runs differently. **Every other red is a
finding about a mechanism** (Law 11), and this file names which are which and keeps them apart.

**Why it sits here.** Not earlier: 11.5 re-derives every opening balance sheet, 12a adds a published
report and a fiscal calendar, 12b names an event whose read and equity sides differ, and 12c anchors
the equity book — each of them moves the numbers these tests read, and migrating a test before them
is migrating it twice. Not later: from 13a the world gains whole systems, and a suite that cannot
guard is worth less with every one of them. So: after 12c, before 13a.

**Objective.** `npm run check` green, with every red either made to assert the property it is named
for, or converted into an inserted item that names the mechanism it is about. **No red is closed by
widening a tolerance, naming a party, or asserting the number the world happens to print** (Law 7,
PLAN §7).

**Read first.** `docs/PLAN.md` §7 (what a test may and may not do; the rig); Law 7 (dust is derived,
never a band, never widened), Law 11 (a misbehaving number is not a work item), Law 13 (a bad number
is a finding, not a regression); `packages/engine/test/rig.ts` (`foundationSpec`, `firmIn`,
`listedIn`, `dealerIn`, `rigFor`, `withDependencies`) — the door a test asks the draw through.

---

## What moved the world under them

Four things, all of them recorded in `docs/RECORD.md` under item 12, and every red below is one of
the four:

1. **The seed's scale is DERIVED** — from the hours this world's people offer against the hours its
   chain needs — where it used to be stated. Every opening balance sheet moved, so every test that
   asserted an absolute quantity (a firm's cash, a cell's holding, a wage bill) reads a different
   number. This is the largest class, and the record says why the stated number starved the world.
2. **Four countries where there was one**, renamed: the United States (USD, `fed`, `ust.*`), Europe
   (EUR, `ecb`, `bund.*`), the United Kingdom (GBP, `boe`, `gilt.*`) and Japan (JPY, `boj`,
   `jgb.*`). A test that took "the treasury" or "the first print" can now get Europe's, and any id
   written down with `north` or `PHX` in it names nothing.
3. **Three assessors charge every issuer a fee, every period** (Ratings A5). Any test that
   enumerated what reached a party's equity in a period is short by that.
4. **A busier world in the small worlds.** The seed needs `goods`, `households` and `labour` to size
   itself, so they are assembled wherever it is — quiet (no phases, no participants) unless the test
   asked for them, but their kinds, units and parameters are there.

## The state this item opens at

`npm run vitest run` at the close of item 12: **283 passed, 90 failed of 373.** Lint, typecheck and
`check:spec` are green, and the world is green in every built audit family at full scale for as long
as it runs. None of the 90 is a world that will not assemble.

---

### The 85 that are ASSERTIONS about a world that moved

By test file, worst first. Each is something a test expected of a world that has changed under
it. The work is to make each one assert the PROPERTY it is named for rather than the number it
happened to read — a test that reads an amount out of a seed that no longer states one, or names
a party the draw now decides, is asserting against a world that does not exist (PLAN §7).

#### `capital.test.ts` — 10

- is bought from a named producer, paid in cash, and built before it works (C1, C2, C3)
  - `AssertionError: expected [ Array(1) ] to deeply equal []`
- is never born from nothing: the machines are destroyed into it (Firm Birth A2.a, C4)
  - `AssertionError: expected 0 to be greater than 0`
- is the BUYER’s question, so a producer’s own output is stock and not plant (A4.c)
  - `AssertionError: expected 0 to be greater than 0`
- binds production, says so, and reports utilisation as a read of the outcome
  - `AssertionError: expected 0 to be greater than 0`
- reads what its EQUITY costs off its own share price when a market prices one (B1.b)
  - `AssertionError: expected null not to be null`
- has no project when it is running below its plant, and one when it is at it (B3)
  - `AssertionError: expected 0 to be greater than 0`
- does not invest when it cannot fund it, and says what it wanted to (B2, B2.a, Firm E4.a)
  - `AssertionError: expected 0 to be greater than 0`
- is bid for at what its REMAINING service is worth, and that is less (D3)
  - `AssertionError: expected 0 to be greater than 0`
- is built, and plant only moves when a leg says why
  - `AssertionError: expected 0 to be greater than 0`
- a dearer cost of capital means a dearer quote, less investment and less output
  - `AssertionError: expected 0 to be greater than 0`

#### `estate.test.ts` — 9

- opens an estate that takes what it held and assumes what it owed (D5, Register F2)
  - `AssertionError: expected [] to have a length of 1 but got +0`
- is what every reference to it resolves to, for as many hops as it takes (D5)
  - `AssertionError: expected true to be false // Object.is equality`
- pays senior first, pro rata within the rank, and the junior recovers nothing (G5.a)
  - `AssertionError: expected [] to deeply equal [ 'firm.2', 'firm.3' ]`
- writes off what it never paid, and the loss lands on the holders (D3, E5)
  - `AssertionError: expected 100000000 to be +0 // Object.is equality`
- releases the dead firm‘s workers through the labour market‘s own path (Labour C4, F1)
  - `AssertionError: expected 0 to be greater than 0`
- shows the estate, its programme and where the dead party went (Observer B5)
  - `AssertionError: expected undefined to be false // Object.is equality`
- dies of no cash while solvent, and pays only the people with a claim on it
  - `AssertionError: expected [] to have a length of 1 but got +0`
- leaves household cells alone, because nothing commits one past its cash (C1.d, XI-3)
  - `AssertionError: expected [ { outcome: 'failed', …(2) }, …(35) ] to deeply equal []`
- stays green and gives the same world twice from the same seed (Law 13, Observer E3)
  - `AssertionError: expected 0 to be greater than 0`

#### `bank-resolution.test.ts` — 7

- opens on the trigger that fired, and says which one it was (C1, C1.a)
  - `AssertionError: expected 'its liabilities are past its assets b…' to match /could not pay/`
- cuts every uninsured claim by the same proportion (D2, D2.a)
  - `AssertionError: expected +0 to be close to 0.1420784015002059, received difference is 0.1420784015002059, but expected 0.000049999999999999996`
- bids from its own view, and is paid to take a book with a hole in it (D3)
  - `AssertionError: expected true to be false // Object.is equality`
- takes the deposits and the book over the wire, and nothing is left behind (D6, C3.b)
  - `AssertionError: expected 'bank.c' to be 'bank.b' // Object.is equality`
- makes the insured whole and lets the uninsured take it (D4, A1.a, XI-15)
  - `AssertionError: expected [ …(3) ] to deeply equal []`
- pays out of the fund the banks paid into, and the purse only after it (D4, D5)
  - `AssertionError: expected 0 to be greater than 0`
- lands a failed bank losses on the banks that funded it, junior money first
  - `AssertionError: expected 0 to be greater than 0`

#### `credit-events.test.ts` — 7

- is a named state, publicly, with the payee that did not get paid (E1.b)
  - `AssertionError: expected undefined to be defined`
- shows a party its own failures and nobody else (Observer A4, Money E1.b)
  - `AssertionError: expected 0 to be greater than 0`
- makes the issuer other lines due when their own terms say so, and says why
  - `AssertionError: expected 0 to be greater than 0`
- is a private read with units and a carrying value, and it moves nothing
  - `AssertionError: expected 0 to be greater than 0`
- is named like anybody else, and the amount is the whole cell own
  - `AssertionError: expected undefined to be defined`
- shows a defaulted line as one, and a holder its own impairment (Observer A4, E2)
  - `AssertionError: expected true to be false // Object.is equality`
- runs a year on a state that spends past what it can fund, and it defaults (XI-9, XI-1)
  - `AssertionError: expected 0 to be greater than 0`

#### `equity.test.ts` — 6

- opens at a level that is a resolution and not a shape (Seed C4, Law 2)
  - `AssertionError: expected 18 to be 6 // Object.is equality`
- changes only by a named event, and a split moves no value at all
  - `AssertionError: expected 74144590 to be close to 90722380, received difference is 16577790, but expected 5e-7`
- sells new shares when it is short and the market is dear, and the count rises (D1, D1.a)
  - `AssertionError: expected false to be true // Object.is equality`
- buys its own back only when the market is below its own book, and the cash is gone (D2, D2.b)
  - `AssertionError: expected 1964.668913037063 to be less than or equal to 13.813607319781648`
- is wiped by the waterfall when the firm fails, and not by a special case (E4, F2)
  - `AssertionError: expected undefined to be defined`
- holds exactly what is outstanding, every period of a year
  - `AssertionError: expected [ Array(1) ] to deeply equal []`

#### `loans.test.ts` — 6

- refuses to seal a world whose bank says it is a credit decision and nobody takes it
  - `AssertionError: expected error to be instance of InvalidRegistry`
- is four named terms, and two banks do not quote the same
  - `AssertionError: expected 'bank.a' to be 'bank.q' // Object.is equality`
- gets dearer for a borrower that has failed to pay (C1.b, C4, Corporate Credit G8)
  - `AssertionError: expected 0 to be greater than 0`
- carries the loan at what its lender expects to recover, and the charge is visible
  - `AssertionError: expected 1 to be less than 1`
- is refused when the bank has no room, and then the payment simply fails (B3.c)
  - `AssertionError: expected 0 to be greater than 0`
- accrues interest that is paid to the lender, period by period
  - `AssertionError: expected 15465074414 to be less than 15460665854`

#### `world.test.ts` — 6

- states no dispersion and produces one: the cells start equal and do not stay so (Seed B4)
  - `AssertionError: expected Set{ 10388972, 15846679, 10257682 } to deeply equal Set{ +0 }`
- runs a year of the whole chain, and every family it has built is green (Part XII)
  - `AssertionError: expected [ Array(8) ] to deeply equal [ Array(7) ]`
- clears, settles paper against cash in one instruction, and revalues everyone (XI-5, D4)
  - `AssertionError: expected 8503613400 to be 4000000 // Object.is equality`
- a buyer without the cash fails the whole trade, not half of it (Register C3.b)
  - `AssertionError: expected +0 to be 1 // Object.is equality`
- show a party its own state and the public state, and nothing of anyone else
  - `AssertionError: expected 4262447488 to be 6500000 // Object.is equality`
- reaches what is said once a period however much else was said (B1, D1)
  - `AssertionError: expected 10 to be 24 // Object.is equality`

#### `firms.test.ts` — 4

- plans and posts an opening once it knows what it sells (Firm E2, Labour C1, C5)
  - `AssertionError: expected 3221.666643189904 to be close to 9109.166104256543, received difference is 5887.499461066639, but expected 5e-13`
- consumes what the recipe says, carries the batch at what it cost, and yields late
  - `AssertionError: expected 0 to be greater than 0`
- is bound by the inputs on hand, and says which one bound it (Goods B1.b, B5.b)
  - `AssertionError: expected undefined to be defined`
- offers what it cannot keep at whatever the book gives, and holds the rest above it
  - `AssertionError: expected 0 to be greater than 25000000`

#### `labour.test.ts` — 4

- is one per region and occupation, in hours, in the money of the place
  - `AssertionError: expected [ …(20) ] to have a length of 5 but got 20`
- pays the wage out of the employer own account, every period (F1, E1)
  - `Error: expected 2964349843 to be 2965062261 to the nearest piece of money (within 3)`
- fills the offer above the going rate and leaves the one below it unfilled (D1.a)
  - `AssertionError: expected true to be false // Object.is equality`
- fills the higher offer first when there are not enough hours for both (D1.a)
  - `AssertionError: expected 60 to be greater than 60`

#### `omo.test.ts` — 4

- buys towards the share policy chose, paying with money it creates (C1, C1.a, C2)
  - `AssertionError: expected 1907374704218 to be greater than 3168853563004`
- is never in a primary market: the seller there is the issuer (C1.b, Treasury D3.a)
  - `AssertionError: expected 0 to be greater than 0`
- lets the book run off when reinvestment is off, and the base shrinks with it (C4)
  - `AssertionError: expected 1949116278319 to be less than 1942629451701`
- remits income and not revaluation (E3.a): what it hands over is what its ledger produced
  - `AssertionError: expected 19134450450.19645 to be less than or equal to 0.22142578228197812`

#### `run.test.ts` — 3

- takes the reserves behind it, in the same instruction (E3.a)
  - `AssertionError: expected 0 to be greater than 0`
- leaves that bank shorter at the next close, and that is the loop (E3, D5.b)
  - `AssertionError: expected -405207702 to be less than -439930195`
- stays green every period of it, and is the same world twice from the same seed
  - `AssertionError: expected [ Array(1) ] to deeply equal []`

#### `auction.test.ts` — 2

- brings dealers to every auction, so the paper is placed and the cash reaches the issuer
  - `AssertionError: expected 0 to be greater than 0`
- fails: nobody absorbs the remainder, and the treasury is left lower than the plan assumed
  - `AssertionError: expected false to be true // Object.is equality`

#### `bank-capital.test.ts` — 2

- weighs a claim on somebody who cannot fail at nothing, and everything else at one (B1.a, E5, XI-3)
  - `AssertionError: expected 0 to be greater than 0`
- is an outcome of what the bank holds, and moves when the rules move (B1.c)
  - `AssertionError: expected [ Array(1) ] to deeply equal []`

#### `derived.test.ts` — 2

- is the book divided by the claims, read at the ask and stored nowhere
  - `AssertionError: expected 32516605.679680005 to be close to 23858631.64, received difference is 8657974.039680004, but expected 5e-10`
- moves when the book moves, and the holders carry the move (Clearing D4, Audit B5)
  - `AssertionError: expected 1625830233.9840012 to be close to 1625830233.9840002, received difference is 9.5367431640625e-7, but expected 5e-7`

#### `money-market.test.ts` — 2

- differs between two banks with different mixes, and the difference reaches the quote
  - `AssertionError: expected 0.08872050184756518 to be 0.004381465526944053 // Object.is equality`
- moves the market rate when it moves, and only through the corridor (B4, B3.a, E1)
  - `AssertionError: expected 0 to be greater than 0`

#### `treasury.test.ts` — 2

- collects on what households were paid and on what they bought (C1, C1.a)
  - `AssertionError: expected 0 to be greater than 0`
- funds itself over a year when the market is there: the debt is serviced and the buffer survives
  - `AssertionError: expected 432183593164.3969 to be less than 0`

#### `dealing.test.ts` — 1

- shrinks the bid to whichever of its limits binds, and says which one did (D4)
  - `AssertionError: expected 'book' to be 'position' // Object.is equality`

#### `deposits.test.ts` — 1

- gives each kind a reason of its own, and all three of them fire
  - `AssertionError: expected false to be true // Object.is equality`

#### `etf.test.ts` — 1

- is closed by a bank when it is worth more than carrying it costs, and only then
  - `AssertionError: expected 0 to be greater than 0`

#### `expectations.test.ts` — 1

- is corrected towards what happened, at the party own speed, and never faster
  - `AssertionError: expected false to be true // Object.is equality`

#### `households.test.ts` — 1

- holds a claim instead of a deposit when the claim pays it enough, and not otherwise
  - `AssertionError: expected 0 to be greater than 0`

#### `params.test.ts` — 1

- counts the two management fees as the placeholders they are, and names their item
  - `AssertionError: expected 4 to be 2 // Object.is equality`

#### `raise.test.ts` — 1

- is bounded by what a lender will have out to one name (F3)
  - `AssertionError: expected 2 to be greater than 2`

#### `resolution/cells.test.ts` — 1

- differs only by what whole people do, and by no more than those people are worth
  - `AssertionError: expected 178 to be less than or equal to 20`

#### `spot-fx.test.ts` — 1

- moves one money against another, both on their own grids, or neither moves
  - `AssertionError: expected 0 to be greater than 0`

### The 3 whose message is a CONTRACT VIOLATION

These are NOT migration work. A contract violation is something impossible by construction,
thrown at the site (ARCHITECTURE §5), so each names a mechanism. They are listed apart so that
none of them is quietly "migrated" away, and each is root-caused here and then either fixed (if
the cause is the test) or INSERTED as its own item at its dependency position (if it is not).

- `capital.test.ts` — is a bank’s own cost of funds and the capital it consumes, and it differs from a fund’s
  - `Missing: [Law 2] parameter fund.requiredYield.fund.money.north is not declared`
- `etf.test.ts` — is a smaller fund and not a broken one: the basket backs the shares that were taken
  - `Missing: [Register A4] instrument share.etf.us does not exist`
- `households.test.ts` — moves cells across a threshold under a mean-preserving spread while the mean stands (A2.g)
  - `Forbidden: [Money E4] instruction 6104 addresses payer.1, which ceased in period 10`

(`tick.test.ts`'s two `Impossible: [Law 8]` throws are not counted here: the resolution invariance
has one owner and it is worklist 11.5, which re-derives the opening balance sheet those two run
against.)

---

## Two findings about the SHAPE of a test, which this item also owns

### Tests that hunt for a period by running the world (item 12's finding **12-6**; `docs/RECORD.md`)

**Where.** `bank-resolution` ("makes the insured whole"), `estate` ("shows the estate, its programme
and where the dead party went"), `households` ("spends what it expects to earn"), `omo` ("lets the
book run off"; "remits income and not revaluation") — and, by the same shape, most of the list above.

**What they do.** They run the foundation world to a period found by running it — `RUN = 22`, "the
period this seed's death happens in" — and then assert about that period. Item 12 moved the path
three times (the depositor's three reasons, the opening balance sheets, the count of banks as a
resolution), and each time these had to find their period again. The things they assert still happen
— a bank still fails, an estate still opens, the central bank still remits — at different periods.

**What this item owes them.** A test asks the world for the EVENT it is about ("the period this
world's first bank failure lands in", read from the journal) and asserts about that, or it builds a
world in which the event is certain. A constant found by running the world is a number copied out of
an answer (Law 19).

### A matching-rule test that depends on its employer's solvency (item 12's finding **pre12-2**; `docs/RECORD.md`)

**Where.** `labour.test.ts`, "fills the higher offer first when there are not enough hours for both
(D1.a)".

**Measured and traced.** The venue is correct: it prints 7000, allots all 2100 hours to the higher
bidder, writes six rows of ten, and the wages settle in full (14,700,000 due, 14,700,000 paid, no
failed instruction). Then `firm.1` **ceases inside the same period** — all six rows separate with
`cause: "firm.1 ceased"` and unpaid severance — so both sides of the assertion read zero.

**The finding is about the test.** It builds an employer with no revenue whose only act is to hire
sixty people, and then depends on that employer surviving the period it hired in. A test of a
matching rule asserts about the match, and the world it builds has to let the match stand.

---

## Steps

- [ ] Every red above is triaged in writing: **moved world** (migrate) or **mechanism** (insert an
      item). The triage is written down before a single test is edited, so the second class cannot
      be absorbed into the first while the file is being worked
- [ ] The three contract violations are root-caused. Each becomes either a one-line fix in the test
      that built an impossible world, or an inserted worklist item naming the mechanism — with the
      reason recorded either way
- [ ] The migration itself, file by file, worst first: an assertion reads the property the test is
      named for, through the rig's own door, with no party id and no amount written down
- [ ] No test finds its period by running the world: each asks the journal for the event it is about
      or builds a world in which that event is certain (the finding above)
- [ ] `labour.test.ts`'s matching-rule test builds an employer that survives the period it hires in
      (the finding above)
- [ ] No tolerance is widened and no band is introduced anywhere in the item; every dust tolerance
      touched is re-derived as `terms × ε × Σ|magnitudes|` (Law 7)
- [ ] `npm run check` green — lint, typecheck, the whole suite, spec citations and plan progress
- [ ] Coverage re-marked where a red was standing in for an unmeasured clause; record entry with the
      triage counts and every inserted item named; delete this file; worklist row 12d → done

## Exit criteria

`npm run check` is green, and every red that was closed is closed for a stated reason: the world
moved and the test now asserts the property, or the mechanism was wrong and an item says so. No
party id, no seed amount and no widened tolerance is left in a test that did not have one.

## Guard

PLAN §7 (a test never names a party, never widens a tolerance, never needs a bound); Law 7; Law 11
(the temptation this whole item is exposed to: a red that is a mechanism's fault made green by
editing the assertion); Law 13 (a bad number is a finding, not a regression).
