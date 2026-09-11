# Open findings — the holding pen

**What this file is.** A bug found while working an item does not derail the item and does not get
chased on the spot. It is written down here, with what was measured and where it was seen, and the
item carries on (Law 10: one ordered list, one item at a time; Law 11: a misbehaving number is not a
work item, the missing mechanism is).

**What happens to it.** When the item closes, every finding here is POSITIONED: moved into the
`docs/plan/<item>.md` of the item that should fix it, or inserted as an item of its own at its
dependency position, with the record saying where it landed and why. A finding is deleted from this
file only by being placed — never by being decided against without a line in `docs/RECORD.md`.

**This file is temporary.** When it is empty it goes.

---

## Found while working item 12

### 12-1 — A bank's opening funding cannot see the assets other modules endow it — **RESOLVED**

**Where.** `packages/engine/src/seeds/foundation.ts`, the opening-balance-sheet block.

**Measured.** At period zero `bank.c` opens at exactly 3.0% equity — its own leverage rule — but
`bank.a` opens at 29.7% and `bank.b` at 23.8%.

**Why.** Those two also hold firm shares and ETF shares, which the `equity` and `funds` modules
endow. Those modules seed **after** `seed.foundation` (assembly order: `… households →
sovereign-instruments → seed.foundation → equity → funds → …`), so the foundation cannot read what
they will hand out and funds only the assets it endows itself.

**Not a slip — an ordering constraint.** `equity` and `funds` need the parties the foundation
creates, so the foundation cannot simply run last. Either the funding derivation moves somewhere
that sees the whole opening state (the kernel's seal already computes assets and liabilities per
party in `stateEquityAsRead`), or each module funds what it hands out.

**Fixed by the first option, as a second seed module.** `seed.funding` requires only
`seed.foundation` and is declared last, so it runs when every module has handed out what it hands
out; it reads each bank's assets off the REGISTER rather than off a tally (Law 19 — a tally is a
second copy of the register that goes wrong the moment somebody endows something without adding to
it, which is exactly how this arrived). It does not name the modules it must follow: naming them
would mean a world assembled without `equity` or `funds` could not include it at all, and a world
that opens its banks and does not fund them is not a smaller world but one where nobody has a
deposit.

**And the number it derives against was wrong too.** It used the bare regulatory minimum, so a bank
opened in breach of its own buffer with no headroom to lend into. The line a bank runs to is the
requirement plus its own caution (Banks Capital B2): 5.0%, 3.5% and 7.0%, dispersed by numbers each
bank had already declared.

### 12-2 — A bill prints above par: somebody bids a guaranteed loss — **RESOLVED in 12's anchor step**

**Where.** The sovereign secondary market, seen after 12's opening-balance-sheet change.

**Measured.** Over a year `gov.north.bill.2026-06-15` printed to **1.1651** and
`gov.north.bill.2027-03-15` to 1.0946. A bill pays exactly 1.00 at maturity and nothing before it,
so any price above par is a certain loss to whoever paid it.

**Ruled out.** Not the households: their required yield is a positive PREFERENCE
(`HOUSEHOLD_PARAMS.liquidityPremium`) and `savingLines` prices with
`priceAt(flows, required, …)` over the instrument's own cash flows, which cannot exceed par at a
positive yield. The bidder is unidentified — the bank dealing quote and the money fund are the
candidates.

**It was worse than 1.1651 and it stopped the world.** On seed `review` the same bill printed
**3.3337** in period 22 — 3.33 paid for 1.00 due in seven days. No representable yield discounts
those flows to that price, so `yieldOf` drove `invertDecreasing`'s bracket to a discount base of
zero and the curve threw `NonFinite`. Two of seven seeds crashed mid-year.

**Cause, found and fixed.** `viewOf` in `banks/dealing-quote.ts` asked
`outlook('price.<instrument>')` FIRST — the desk's own adaptive expectation _of the price_, formed
from the prints — and fell back to the cash-flow reservation only when it had neither outlook nor
mark. That is XI-13's fixed point written out: print moves outlook moves view moves quote moves
print. For a dated claim the reservation valuation is now asked first, and the outlook is what a
share falls back to, because a share has no payments to discount. All seven seeds run; the audit is
green over a year; no line prints away.

**Placed at 12's close**, in the record, as the anchor step's outcome.

### 12-3 — The long end has no holder — **INCOMPLETE MODEL, not a bug. Waits on 13h.**

**Twice mis-filed before it was read correctly.** First as "a bank expects to lose a third to a half
of what it lends the sovereign", then as "the treasury cannot place its paper". Both are Law 11:
_a misbehaving number is not a work item; the missing mechanism is._ Neither number is wrong.

**What is actually true.** Households will not tie money up past their own horizon — 52 periods,
`households.horizon.periods`, a PREFERENCE with a reason (Households D5). The treasury issues at two,
five and ten years. **Nothing in this world holds a long-dated claim**, because the parties that do
— insurers and pension funds — are worklist **13h**. So the only bidders at a long auction are the
primary dealers meeting their obligation, cover comes in at exactly `dealershipShare` (0.34), and the
issuer withdraws at its stated patience of 40bp over the curve (`treasury.concession`).

Everything downstream of that is the model working:

- the treasury funds nothing at the long end and misses payments — XI-9's constraint biting;
- the banks then require 0.0490 and 0.0352 of it, against 0.0833 and 0.125 on each other — a lender
  that has watched an issuer miss payment after payment asking more is the credit model doing its job;
- 748 `overdraftRefused` at the central bank — correct, and Appendix B forbids the alternative.

**Do not "fix" any of it.** An attempt was made and reverted: making the issuer read its own auction
history and stop bringing a tenor the market refused. It is a symptom patch (Law 12) and worse — it
would have deleted the one signal that says 13h is missing. The deliberately-failing auction is the
incomplete-model check working as designed.

**Placed at 13h**, whose insurers and pensions are the holders this end of the curve is waiting for.

### 12-5 — A bank fails in period 7 of the foundation world

**Where.** `foundationWorld`, seen after 12's opening-balance-sheet step (`cap-a`, and it is not
seed-specific).

**Measured.** `bank.b` ceases in period 7 with `bank.a` as its successor. Its equity walked up
28.7M → 31.2M over periods 1-6 and then went to zero in one step, which is the resolution taking the
book rather than a loss landing on zero. Ahead of it: `moneyMarket.refused` for `bank.b` at periods
5, 6 and 7 (short 1, then 45,554,802, then 45,194,801, reserves −23,288,853 at period 6), then
`centralBank.refused` with `collateral: 0, solvent: true` — refused at the window with nothing left
to pledge while still solvent. That is Banks Funding D6 firing: a funding failure, distinct from
insolvency, and the audit is clean through all of it.

**Not chased, on purpose.** It appeared when the opening balance sheet stopped stating three reserve
figures and started deriving them from the central bank's own balance sheet, so `bank.b` opens with
thinner reserves than the table used to hand it. Whether that is a bank that should fail — it runs
its account down for six periods and then cannot fund itself, which is the mechanism working — or a
seed that funds it wrongly, is a question about the opening sheet and about 12-1: the equity and
funds modules endow these banks with shares the foundation never funded, so what a bank opens with
is not what the foundation thinks it opened with.

**Do not read it as a regression.** The change that surfaced it is right on its own terms (Law 13):
four numbers chosen by reading the answer became two declared shapes, and the central bank no longer
opens at its own OMO target, which was an imported equilibrium.

**To be positioned at 12's close**, most likely into 12-1's resolution — a bank cannot be funded
against assets the funder cannot see.

**And the measurement the count-of-banks step asked for says it is not one seed's accident.** Nine
worlds — two banks, three and four, each at one, two and four cells per key — run 26 periods with
every audit family at zero, and the population is EXACTLY 6,000 in all nine, which is the invariance
that step wanted. What is not invariant is how many banks are left:

| banks opened with | grain 1 | grain 2 | grain 4 |
| --- | --- | --- | --- |
| 2 | 2 alive | 2 | 2 |
| 3 | 1 | 2 | 1 |
| 4 | 2 | 1 | 1 |

and what the survivors hold liquid against what could leave them swings from 181M/109M to 459M/400M
across the nine. A world that opens with more banks ends with fewer. That is 12-1 and 12-5 read at
nine points instead of one: the foundation funds a bank against the assets it can see, the equity
and funds modules hand it more afterwards, and the more banks there are the thinner each one's
reserves are when it opens. **Nothing here is tuned and nothing is judged** — whether the swing is
material is Part XII's question at worklist 16 (Law 11), and the mechanism it names is 12-1's.

### 12-6 — Five tests that hunt for a period the world no longer reaches there

**Where.** `bank-resolution` ("makes the insured whole"), `estate` ("shows the estate, its
programme and where the dead party went"), `households` ("spends what it expects to earn"), `omo`
("lets the book run off"; "remits income and not revaluation").

**Measured.** Reds stayed at 42 across the depositor change (four new tests were added and four
others went green: `auction`'s stepped-back dealers, `bank-capital`'s published position, `equity`'s
two cells wanting different prices, `treasury`'s wage bill). What changed in each of the five is
WHEN: they run the foundation world to a period found by running it — `RUN = 22`, "the period this
seed's death happens in" — and then assert about that period.

**Why.** Depositors now move for three different reasons at three different signals, so who banks
where in week 22 is not what it was. Every one of these is Law 13's moved number rather than a
mechanism that stopped working: the audit is green over 52 periods on all seven seeds, and the
things they assert still happen — a bank still fails, an estate still opens, the central bank still
remits — at different periods.

**Not chased, on purpose.** A test that finds its period by running the world has to find it again
whenever the world moves, and doing that inside this step would be tuning the tests to the run
(Law 11). It is one bounded change of its own once the item's remaining steps have stopped moving
the path: the count of banks (a resolution) and the currency layer both move it again.

**To be positioned at 12's close.**

**Added when the opening sheet was funded against what a bank actually holds** (12-1), which moved
every bank's capital from 3.0/23.8/29.7 per cent to 5.0/3.5/7.0: `auction` (both), `bank-capital`
("the buffer above it"), `capital` (four), `equity` ("never sells below its own reservation", "the
count", "buys its own back"), `funds` ("breaks the buck", "sells into a market it does not price"),
`labour` ("fills the offer above the going rate"), `money-market` ("what somebody outside can see"),
`omo` (both), `raise` ("asks for what it published"), `treasury` ("employs people on rows"). Nineteen
in, ten out. Every one of them runs the world to a state and asserts about it; the audit is green
over 52 periods on all seven seeds, and no family, contract or door is among them.

**Added when the bank count became a table rather than three names** (same cause, same treatment):
`auction` ("nobody absorbs the remainder"), `bank-resolution` ("contagion by name"), `capital`
("binds production"), `credit-events` ("runs a year on a state that spends past what it can fund"),
`funds` ("sells into a market it does not price"), `labour` ("fills the offer above the going rate"),
`omo` ("hands over what its own instructions earned"), `raise` ("asks for what it published"),
`tick` ("converges as the piece gets finer"). Each moved because the seed now spreads the firms
across whatever banks exist and splits the paper by each bank's stated size, so who banks where and
who holds what is different — and every one of them reaches the state it asserts about by running
the world to a period it found by running the world.

### 12-7 — The kernel declares two kinds whose behaviour a module owns

**Where.** `packages/engine/src/registry/profiles.ts`, `KERNEL_PARTY_KINDS`: `FIRM` and `HOUSEHOLD`.

**How it was found.** Building the `bankChoices` door. The guard that belongs with it — a module
that declares a depositor must say how it leaves (Banks Funding A1.d) — can only ask a module about
its OWN declaration, because a world assembled from four modules to exercise a kernel door has the
kernel's party kinds in it and no `firms` to speak for them. So the guard covers `FUND` and
`FUND_MANAGER`, which the `funds` module declares, and says nothing about `FIRM` and `HOUSEHOLD`,
whose reasons live in `firms` and `households` while their kinds live in the kernel.

**Why it is a defect and not a preference.** ARCHITECTURE 4.9b: the kernel "owns the money
instrument kind and the party kinds money needs", and a kind is owned by exactly one module. Money
needs the central bank, the treasury and the bank — it does not need a firm or a household. The
first attempt at the guard checked every registered kind and refused six legitimate reduced worlds,
which is the same defect read from the other end.

**One thing was fixed on the way and is worth naming**: `SMALL_BUSINESS` was a third such kind, and
nothing anywhere ever instantiated it — 13e declares its own `smallFirm`. It is deleted.

**Not chased, on purpose.** Moving two kinds out of the kernel touches every module that imports
them and is a kernel change of its own (PLAN §4.4: an inserted item with a record entry). It is not
this item's, and it moves no number.

**To be positioned at 12's close**, most likely as an inserted kernel item before 12a.

### 12-8 — A bank defends a deposit past what the guarantee on it costs

**Where.** `packages/engine/src/mechanisms/banks/treasury.ts`, `defended()`, read with `setBoard()`.

**Measured.** `money-market.test.ts` ("pays a deposit rate per class, and the classes are not one
rate"): one bank shows retail 0.0180 and wholesale 0.0130.

**What is wrong.** `setBoard` prices a class at `worth - margin - premium`, so an insured class is
offered less by exactly what the guarantee costs the bank — correct. `defended` then lets it MATCH a
rival at anything up to `worth`, and it compares the RATE against `worth` while the bank's cost of
that money is `rate + premium`. So a bank defending retail against a keen rival pays up to
`worth + premium` all-in for money worth `worth` — the thing `defended`'s own comment says it will
not do: "it stops at what the money is worth to it — past that it funds itself in the market
instead". The stopping point for a class is `worth` less what the guarantee on that class costs.

**It is a cause with one fix** (Law 12): one number, in one place, per class. It is NOT a bound — it
is the alternative D1 names, measured correctly for the class.

**Not chased, on purpose** (Law 10): it was reachable before the step that surfaced it and it is not
that step's. **To be positioned at 12's close**, into the step that settles the boards.

### 12-9 — A lien could name a party that had ceased, and then nobody could end it

**Where.** `packages/engine/src/mechanisms/money-market/resolution.ts` (`moveBook`), read with
`index.ts` (`freeRepaidCollateral`) and the `collateralHolds` family.

**Measured.** After the opening sheet was funded properly, five of seven seeds went red in the
`ownership` family every period from period 9 on: `bank.b still has 613,744 of
gov.north.2031-03-15 bound to repo:bank.c:bank.b:25, which is not a live row`. 44 a run.

**Cause, and it is two defects that only bite together.**

1. **Two definitions of "a live row".** The family counted a row finished when it had ceased OR
   nothing was outstanding on it; the releaser only when the instrument had ceased. A repaid row
   keeps its instrument — the kernel redeems the claim, it does not delete the line — so the
   releaser thought a repaid row still held its collateral. One writer now: `securesAnything`.
2. **A resolution re-seated only some of the liens the failed bank held.** It released liens over
   the ACQUIRER's paper and left liens over anybody else's naming a party that no longer existed.
   Survivable until a SECOND bank fails and the chain of successors brings the beneficiary round to
   the pledgor itself: both ends then resolve to one party, Register D5 refuses a leg whose two
   sides are the same party, and no release anybody could write would settle. The paper stayed
   bound for the rest of the run. Now no lien ever names a ceased party.

**Fixed rather than parked, under Law 11's one exception**: not a misbehaving number but an audit
family red every period in five of seven seeds, and four test files that could not collect — it
blocked the work. Both fixes remove code rather than add a check.

### 12-4 — An audit violation in the bank-failure scenario

**Where.** `packages/engine/test/bank-resolution.test.ts`, at module scope — the file's own setup
builds a world in which a bank fails and asserts the audit is clean before any test runs.

**Measured.** After 12's balance-sheet and anchor changes the whole file fails to collect:
`AssertionError: expected [ Array(1) ] to deeply equal []` — one unexpected audit violation. Eleven
tests do not run at all, which is why the suite total moved from 339 to 328.

**Not reached by the ordinary run.** `foundationWorld` is green in every built family over 52
periods on seven seeds; this is a scenario the foundation does not enter. The violation's family,
owner and size are not yet read — the assertion reports the array, not its contents.

---

### 12-10 — A ceased bank keeps its reserve overdraft, and nobody is owed it

**Where.** `audit.money` and `audit.names`, in a world of thirty banks.

**Measured.** Probe of `foundationSpec` at 30 banks / 300 firms, seed `probe`, periods 3-5:

```
p3 money(4): bank.ab closes -4,075,747,840 per member on money:cb.north:PHX with no lender row behind it
   names(2): bank.w has ceased and still holds -37,303,027 of money:cb.north:PHX
p4 money(3), names(6): bank.ab -2,982,351,011 and bank.z -817,328,529, both ceased
p5 the same two, unchanged
```

Three banks fail, and each leaves a NEGATIVE balance on the central bank's money behind it. Two
families see it and say different halves of the same thing: `money` says an overdraft with no lender
row behind it (a borrowing with no lender is money with no issuer), and `names` says a party that
has ceased still holds something.

**Why it was never seen.** It needs a bank to fail. With three banks in the world the foundation ran
fifty-two periods without one; with thirty, three fail inside five. The count of banks was carrying
this, exactly the way it was carrying the opening balance sheet (12-1).

**What it is not.** It is not the resolution missing the overdraft: the number does not move after
the bank ceases, so nothing is still running. It is that resolution ends a bank whose reserve
account is overdrawn and the overdraft goes nowhere — Appendix B's "no death without a destination",
applied to a liability rather than a holding.

**Where it belongs.** Estate/resolution, and `money-market/resolution.ts` is where a failed bank's
book is moved. Not positioned yet.

### 12-11 — The journal and the ledger were walked end to end on every read — **RESOLVED**

**Where.** `journal/journal.ts`, `ledger/ledger.ts`.

**Measured.** 10 banks / 60 firms: p1 226ms, p3 280ms, p5 567ms, p6 764ms, with the journal at
17,000 events — the cost of a period was the cost of every period before it. `lastOf(kind, subject)`
scanned backwards to the beginning of time whenever a party had never said that kind of thing, and
every party asks what it last said, every period.

**Fixed in this item under Law 18** (layout and traversal are free; the reads return exactly what
they returned). The same events under three arrangements, written where the event is written: a list
per kind, a list per period, and the latest per (kind, subject). 30 banks / 300 firms went from
5.7s to 1.5s at p3 and 10.5s to 2.2s at p4.

**Still open underneath it.** Events per period keep RISING (3,929 at p1 to 21,000 at p5 in the same
run) because rows accumulate and each says something every period. That is a real growth in what the
world has to say, not a defect, but at thirty million people and three thousand firms it decides
whether a run is possible at all. Not positioned.

### 12-12 — What a period costs at the real scale

**Measured.** `foundationWorld('full')` — 30 banks, 3,000 named firms, 30,000,000 people, 3,169
parties, 303 instruments, 261 markets. Assembly 0.4s. Audit green in every built family.

```
        before this item's layout work     after
p1      27.1s                              7.0s
p2      30.5s                              8.6s
p3      (not reached in 115s)             12.4s
```

What one period of it SAYS, at p3: 69,344 events over 36,887 settled instructions — 18,407
revaluations, 16,804 settlements, 7,382 surprises, 5,971 perishings, 3,038 credit quotes, 3,000
plans. None of that is a defect: three thousand firms deciding and being revalued is what three
thousand firms cost. It rises period on period as holdings accumulate.

**What it means for the loop.** A 26-period run is about seven minutes and roughly 2,000,000 events
in memory. That is a deliberate run, not a unit test. The tests therefore build SMALLER worlds
explicitly, through the same `foundationSpec(seed, banks, firms)` door the real one uses — a test rig
that says it is one. The WORLD is the real one; the rig is visibly a rig.

**Not positioned, and it is the next thing this will need.** Four fifths of what is left is the
market fan-out: a session asks every party of a kind whether it has an order in it, so 261 markets
and 3,169 parties is 827,000 questions a period whose answer is almost always no. The kernel cannot
guess which parties could have an order — that is a mechanism's own business — so the fix is a
module contract change (a participant saying which markets it is ever in) and it needs an item.


### 12-13 — A quantity was a `number`, so a fraction of a cent could be anything — **RESOLVED**

**The defect.** Every number in the engine was a `number`. Nothing distinguished A COUNT OF PIECES
from a price, a rate or a value, so every author had to remember Law 8 by hand — and the ones who
divided money by a price to get units did not. What was caught was caught by runtime guards at the
far end (settlement's `onTheGrid`, the register, the order book), long after the site that made it;
and a published DECISION that never reached any of them was never caught at all.

**Measured.** A scan of every quantity-named field in 78,607 journal events of a 24-firm world:

```
    288  firms.plan.batch          e.g. 4,649,446.196200187 grams started
     85  bank.lines.lines[].room   e.g. 1,545,512,753.9337654 cents allotted to a line
     56  fund.struck.spare         e.g. -182,283,969.53417602 cents
      5  bank.arbitrage.gap        (a PRICE, misread by the scan's own field names)
      1  bank.arbitrage.shares     e.g. 246,798.02755023047 shares
```

Plus, at the order book: a desk's `bidSize`, a firm's sell schedule, a household cell's demand
curve, a treasury's procurement, an equity issue, a plant bid — all sizes computed by division and
posted as they came out.

**Fixed at the cause.** `Qty` is now a TYPE (`core/tick.ts`): a branded count of pieces, produced by
exactly five doors — `toTick`, `downTick`, `upTick`, `splitOnTick`, and `asQty` for a number that is
already a count (it throws if it is not). Everything that CARRIES a quantity asks for it: an order's
size, a fill, a lot, a lien, what is outstanding, a primary offer, what the register reads back, what
a declared amount comes to, and the decisions modules publish (a plan's batch and hours, a quote's
two sizes, an equity buyback and issue, a cell's shares per member, a venue's hours). A module that
divides money by a price now gets a `number` and cannot put it anywhere a quantity goes without
saying WHICH WAY IT ROUNDS — which is the decision it was skipping, and it is its own to make.

**And one duplicate deleted.** `core/money.ts` held a second `Qty` — a boxed `{amount, unit}` value
object with its own add, subtract and scale — that NOTHING in the engine ever used and that enforced
no grid. Two representations of one real thing (Law 4). It is gone; the currency-mismatch read it
carried is the wire's (`ledger/settlement.ts` refuses a leg in a currency the party does not book
in), and `test/core.test.ts` says so where the old test stood.

**The result.** The same scan reports ZERO off-grid quantities in 97,867 events, the compiler names
every new site at once instead of a probe finding them one at a time, and the runtime check in the
clearing solver is deleted — the type is the check, and a check written twice is the one that fires
last.


## Carried in from item pre12

These two were named in `docs/RECORD.md`'s `pre12` entry and handed to item 12 rather than fixed,
because a documents item may not fix a red test. They are findings, not expected reds.

### pre12-1 — The resolution invariance does not converge

**Where.** `packages/engine/test/tick.test.ts`, "converges as the piece gets finer, in what it made
and in the money it holds".

**Measured.** 0.00928 against 0.00898 — a miss of about three per cent. This is Law 2's resolution
invariance for the whole grid: declare the same world in finer pieces and its path must not move.

**Open question.** Whether the opening balance sheets moved it. If they did, 12's record says so; if
they did not, it is a finding of its own about the grid and wants an item.

### pre12-2 — A matching-rule test depends on its employer's solvency

**Where.** `packages/engine/test/labour.test.ts`, "fills the higher offer first when there are not
enough hours for both (D1.a)".

**Measured and traced.** The venue is correct: it prints 7000, allots all 2100 hours to the higher
bidder, writes six rows of ten, and the wages settle in full (14,700,000 due, 14,700,000 paid, no
failed instruction). Then `firm.1` **ceases inside the same period** — all six rows separate with
`cause: "firm.1 ceased"` and unpaid severance — so both sides of the assertion read zero.

**The finding is about the test.** It builds an employer with no revenue whose only act is to hire
sixty people, and depends on that employer surviving the period it hired in. Whether what starves it
is the opening balance sheet is 12's question; whether a test of a matching rule should depend on
its employer's solvency is a question about the test.
