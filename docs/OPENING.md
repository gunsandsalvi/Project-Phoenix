# The opening world — what it is, why it is where the bugs come from, and what to do instead

A proposal, written against the state at `2ec6b4c`. It answers one question the owner put: *this
simulation should not start from scratch; it should open in an already existing, fully active world —
like peeking inside one.* What follows is what the opening is today, the causal chain from it to the
findings, four ways it could be built instead, a recommendation, and the items it becomes.

---

## 1. What the opening is today

`seeds/foundation.ts` (3,168 lines) and `seeds/funding.ts` run once, before period 0, through
`SeedContext`. What they may do is narrow and deliberate: add parties and instruments, `endowMoney`
and `endowUnits`, write opening prints, open markets. What they do NOT do is settle anything —
**`ctx.settle` is called zero times in the seed**. Balances are written straight into the register
through `moneyDelta` and `adjustIssued`.

So the opening is a **stated stock**: a complete balance sheet, party by party, asserted.

Three facts about it matter more than the rest.

**Every holding arrives without a flow behind it.** The register has the units; the ledger has no
instruction that put them there. The seed is the one writer besides settlement, and this is the
exemption that lets it be.

**Every line is issued at the epoch.** `issueDate: ctx.calendar.epoch` for every seeded sovereign
line. Maturities are spread across a grid (3, 6, 9, 12 months and out), but issue dates are not: at
period 0 nothing has ever accrued, nothing is seasoned, every bond is on-the-run, and Seed D2's
"anything that accrues starts from a stated accrual position" is satisfied only by the position
being zero everywhere.

**Stocks are walked from ratios, not stated one by one** — which was the right fix to an earlier
problem (seventy-two hand-written quantities) and is the source of the biggest one now. The seed
states the SCALE (what a member takes off the end of the chain in a period) and walks it up each
recipe, so "a world of thirty million people opens with thirty million people's worth of stock in
it." The measured consequence, recorded at 12c.3: **one baker opened holding 314 million loaves
against an expected 27 million a period** — eleven periods of the entire world's demand, in one
firm's warehouse, on day one.

---

## 2. The chain from the opening to the findings

The recorded findings are not twelve independent bugs. They are one shape seen from twelve sides.

- **12c.3** — the twelve named firms produce ONCE, in period 1, and never again. Every plan after it
  is `batch 0, bound demand`: the opening stock exceeds what they expect to sell for longer than
  they live. By period 52 six of twelve are dead and thirty-seven estates are open.
- **21.79** — sixty-six supply books a period, sixteen periods: **0 bids, 1 ask, 0 contracts**. What
  a party posts is what its own outlook of `bought`/`sold` says, and almost no firm has either,
  because almost no firm trades.
- **21.84** — `consumer.us.1` and `producer.us.1` read NONE over a basket of ZERO: no physical good
  reaches a household in a settled instruction. So the central bank meets, reads the basket, finds
  nothing, and **no rate has ever moved in this world for a reason**.
- **21.77** — two overnight fixings in twelve periods, so 2 loans of 52 float and there is no channel
  from a policy rate to a borrower's payment.
- **21.72** — the banks stop quoting every name by period 9.
- **21.81** — 0 securitisation cuts in twenty-four periods, every deal failing for want of a bidder.
- **21.76** — the first NAMED firm publishes accounts at period 25, so nothing that reads published
  accounts has anything to read for half a year; **21.73** — `control` never runs at all.

Two mechanisms carry the whole chain.

**(a) A world that opens full does not produce.** Production is a decision against expected sales;
a firm holding eleven periods of world demand decides nothing. No production means no inputs bought,
so no supply contracts, no deliveries, no consumer basket, no price level, no policy response — and
no revenue, so no wages, so no household income, so no demand. It is a dead loop entered on day one,
and the audit cannot see it because nothing is violated: every number is consistent, and the world
is simply asleep.

**(b) A world with no past has no expectations.** §46 is emphatic that every deciding party's outlook
is formed from **what that party itself observed**, and `observations()` reads exactly one source:
`ctx.ledger.inPeriod(...)` — the instructions the party was a side of. The seed settles nothing.
Therefore **at period 0 every party in this world has observed nothing and has no outlook of
anything**. Every mechanism that decides against an outlook — post a supply order, bid in a book,
plan a batch, form a position, vote — is silent until a history accumulates, and where the silence
stops the flow that would have created the history, it never accumulates. The polity showed the same
shape from its own side: a cell that has never been paid has no expectation of pay and cannot vote.

That is the answer to the owner's question. The problem is not that the world starts small. It is
that it starts **full of stock and empty of history** — the exact inverse of an existing world.

---

## 3. Four ways to build an opening

### A. Stated stocks, fixed (the status quo, plus item 22a)

Keep the seed as a declaration; correct the numbers (stock basis to something a firm would actually
hold), delete the seeded prices, spread the issue dates, state accrual positions.

*For:* cheapest; 22a is already written; no new machinery.
*Against:* it does not touch (b) at all — a stated stock cannot produce a ledger history, so parties
still open blind. And every number stated is a number that has to be right: Seed D1 ("stocks
consistent with the flows") stays unfalsifiable, which is how a baker ended up with eleven periods
of demand without anything saying so. Each new mechanism adds new opening state by hand.

### B. Burn-in: run the world before period 0 and keep what it becomes

Open thin, run N periods, discard the transient, call period N the opening.

*For:* D1 becomes automatic — the state is consistent with the flows because the flows made it.
Histories, ages, accruals, maturities and lots all come out real. It is the standard answer.
*Against:* the transient is the pathological path this world is currently in. A burn-in from the
current thin start converges on a dead world, so it cannot be adopted before the mechanisms it
depends on work. It is also slow (the warm-up is real computation) and the length is a number
somebody chooses.

### C. A seeded PAST: dated instructions, replayed through settlement

The seed stops writing balances and starts emitting **instructions with past dates**, which the
kernel replays through the ordinary settlement path before period 0. A holding exists because a
two-sided instruction put it there, on a date, from a counterparty, at a price.

*For:* Seed A4 ("no free money, no free assets") becomes structural rather than a rule — there is no
door left that creates a holding without a giver. Lots get real acquisition dates and bases; bonds
get real issue dates and therefore real accruals and a seasoned maturity profile; **and the ledger
has a history, so every party has observed something and has an outlook on the day the world opens**.
The audit families work on it unchanged, because it is ordinary settlement.
*Against:* somebody writes the past. A scripted history is a seeded outcome (E1) if it decides
anything the mechanisms should decide, and it needs care to stay a stock-creating device rather than
a narrative (§45 E2).

### D. Solve for a consistent state (calibration)

Choose the opening as a fixed point of the mechanisms.

*Rejected, and the spec rejects it twice:* Law 2 forbids importing a real-world equilibrium, Seed D4
says the seed must be a fixed point of NOTHING, and E1 forbids seeded outcomes. A solved opening is
the answer written down where the mechanism should be.

---

## 4. The proposal: C now, B after — one machine, two sources

**One mechanism, two phases.** Build the replay machinery once; change where the instructions come
from as the world becomes able to produce them.

### Phase 1 — the opening is a replay (`world.prehistory`)

1. **A dated pre-history.** The kernel gains a pre-period phase that walks a list of
   `{ at: Civil, draft: InstructionDraft }` in date order and settles each through the ordinary path,
   with the calendar standing at that date. Periods before the epoch are real period indices on the
   one calendar (the epoch moves back by the warm-up length; `period(0)` remains the opening).
2. **`endowMoney` and `endowUnits` are deleted.** Money is created the way money is created — a bank
   or a central bank issues it to somebody, both legs — and units the way units are created (a
   `create` leg whose cause is `seed`, which the register already understands). What cannot be
   created by an instruction is the physical world: land, people, plant, the state, the central bank.
   Those stay declarations, and the list of them becomes short enough to read.
3. **Every seeded instrument gets its real issue date** and is sold to its first holder on that date
   at a price that instruction carries. Accruals, seasoning and the maturity profile stop being
   stated and start being read.
4. **The census replaces the exemption.** `check:opening` (new, cheap, runs with `check:opens`)
   asserts the properties of an *existing* world rather than of a consistent one:
   every market has traded at least once; every party has at least one outlook; every firm has
   produced and sold and been paid; every bank has lent and been repaid; the maturity profile spans
   more than one period; ages are dispersed; no holding is younger than the world. Each is a
   property of an opening this world could not previously have had, and each is a rule that can be a
   check, so it is one.

What Phase 1 buys immediately: (b) disappears — parties open with histories, so outlooks exist on
day one and the silent mechanisms have something to decide with. (a) becomes visible — with stock
arriving through purchases instead of assertion, a firm's opening inventory is whatever it actually
bought, and 314 million loaves cannot be written down by accident.

### Phase 2 — the past is produced, not written

Replace scripted pre-history events, module by module, with **warm-up periods the engine runs
itself**: the same period loop, before period 0, with the same modules deciding. The machinery is
identical; only the source of the instructions changes. A module is migrated when the world can
produce that behaviour on its own, which makes the migration a measurement of whether a mechanism
works — the thing this project keeps discovering late.

The honest ordering: Phase 2 for the goods chain cannot come before the production loop is fixed
(22.2, 22a), and that is fine — Phase 1 does not depend on it.

### What this deletes

`seed.openingPrice.*`, `seed.openingWage`, `seed.openingYield`, `seed.openingRate`,
`seed.households.openingHoldingShare`, `PUBLIC_AT_THE_OPENING`, `prices.write` from `SeedContext`,
`endowMoney`, `endowUnits`, `SEED_STOCK_BASIS` — every one of them replaced by a read of something
that happened. That is most of item 22a, arrived at from the other side: 22a asks the seed to claim
less, and this asks it to claim differently, and what is left to claim is the physical world and the
primitives.

---

## 5. The risks, stated

- **A scripted past is still a script.** The guard is that pre-history instructions may only create
  STOCKS — issue, buy, hire, lend — and may never write a price, a rate or a decision. What they
  cost is what the instruction says; what anything is worth from period 0 is cleared.
- **The replay is a new period-0 surface**, and a bug in it is a bug in every run. It is mitigated by
  it being the ordinary settlement path: the audit families run over the pre-history unchanged, and
  Seed A2 ("pass the audit at period zero") becomes a genuine test rather than a test of a hand-made
  balance sheet.
- **Cost.** Phase 1 is a kernel phase, a data format, and a rewrite of `seeds/foundation.ts`'s
  endowment half — the largest single change since the calendar. Phase 2 is per-module and cheap.
- **It will fail loudly at first**, because a world that cannot open through its own settlement path
  is a world with a missing mechanism. That is the point: it converts today's silent, consistent,
  asleep opening into a stop with a name.

---

## 6. The items

Inserted at their dependency position, before 22a (which they largely absorb) and after 21's stops:

- **22b.1** `world/prehistory.ts`: the dated replay, the epoch shift, `cause: 'seed'` on the legs,
  and the refusal that a pre-history instruction may not write a print.
- **22b.2** `check:opening`: the census of an existing world, as above, red until it passes.
- **22b.3** The sovereign and money layer through the replay: reserves issued, bills sold at auction
  dates, the central bank's holding bought rather than endowed. Deletes `endowMoney`.
- **22b.4** Firms, plant and inventory through the replay: plant bought from its maker on its vintage
  date, inventory bought from the chain. Deletes `endowUnits` and `SEED_STOCK_BASIS`.
- **22b.5** Households, employment and the savings stock through the replay: wages paid, deposits
  accumulated, fund shares bought.
- **22b.6** Delete the opening prints (22a.1) — by then every market has a traded price from the
  pre-history, which is what made them necessary.
- **22b.7** Phase 2's first module: warm-up periods replacing the scripted events wherever the
  mechanism can now produce them; the record says which.

**Exit.** `grep -rn "endowMoney\|endowUnits\|openingPrice\|prices\.write" src/` returns nothing;
`check:opening` is green; and the census of period 0 reads like a census of period 100.
