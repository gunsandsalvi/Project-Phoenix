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
