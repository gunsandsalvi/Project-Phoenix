# Item 12a — Reporting and estimates

**Read this part first — it is why the item exists and why it lands here.** The specification gained
§48 in the same change that inserted this item. Until then the world had the *substance* of corporate
reporting — accrual matching (§37 F5.a), inventory at the lower of cost and net realisable value
(§37 E2), a firm publishing its own expectation and being judged against it (§32 E7) — and none of
the *apparatus*: no fiscal calendar, no report, no bank publishing an estimate of anybody, no
surprise with a name on it. §48 is that apparatus, and this item builds it.

**Where it lands and why.** Inserted between 12 and 13a, and Part XIII step 12a says the same thing.
Not earlier: the surprise this system exists to produce is only observable against a share price that
is not walking, and not-walking is exactly what item 12's anchor delivers; and a bank publishing a
named opinion is §44's sibling, so it wants the surface item 12 builds for ratings. Not later: a
tender offer (13g) is priced off a target's reported earnings and the acquirer's own view of them,
and the income statement this item makes readable is a thing every system after it wants.

**Objective.** A public company publishes, on a fiscal calendar, a report that is a **read of its own
books** — the period's income decomposed, the balance sheet at its close, the cash that moved. Its
management guides to the coming period in the report's own lines, and can revise or withdraw.
Every bank that covers the name publishes **its own** estimate of the same lines, formed from what it
observed, revised on information, and wrong often enough to matter. The report settles both. What the
surprise causes is holders revising their own outlooks and therefore their schedules in the share
book — so the price moves because the book moved, and nowhere is there a rule that says by how much.
Consensus exists only as a read.

**Read first.** §48 (all); §32 E7 and E7.a; §46 (all — A2.a, A2.b, A4, B2, B3, C2.a and D4 in
particular); §44 A5, A2.a, E1, E3 (the sibling system, and the four defects it names that this one
can repeat); §45 A3, A4, A5, B2.a; §10 Equity B3, C1.b; §1 Money G3.a, G3.b, G3.c; §37 F5, F5.a,
F5.b, E2; §4 Audit B5, A1.a; §2 Register E2.a; Law 2, Law 3, Law 4, Law 11, Law 19; Appendix B 59–64.
Code: `register/register.ts` (`moveEquity`, `equityWalk`), `ledger/instruction.ts` (`EquityEffect`,
`Cause`), `world/revalue.ts`, `audit/families/accounts.ts`, `journal/journal.ts` (the `public` flag),
item 4's `expectations` module, item 9's `equity`, item 11's banks, item 12's `ratings`.

**Clauses this item meets.** Reporting A1, A2, A2.a, A3, A4, A5, B1, B2, B3, B4, C1–C6, D1, D2, D3,
D3.a, E1, E2, E3, F1, F2, F2.a, F3, F4, G1–G6; Firm E7 (fully, on a calendar); Observer A3 (the
report and the estimates as public state). Remain PARTIAL: Reporting H1–H4, which are Part XII
measurements and are marked so with worklist 16 — the reads exist here, the measuring does not
(Law 11).

**Guard — read before writing a line of it.**

- **No price reaction rule** (F2.a). The surprise changes outlooks; outlooks change reservations;
  the book clears. If anywhere in this item a price is written from a surprise — a coefficient, a
  drift, a nudge — the item has failed at the thing it exists to do. The tell is a parameter whose
  unit is "price move per unit of surprise".
- **No consensus a decision reads** (E2). It is computed at the moment of reading and stored
  nowhere. A module that takes the consensus and acts on it has re-created §46 A2.b's global
  expectation with an extra step.
- **No estimate off the price** (C6). §44 A2.a is the same defect and the record for 11.3 is what it
  looks like when a view follows the print: the thing walks away and the model cannot disagree with
  itself.
- **No number the books do not produce** (A2.a, G2). The report is a read. If a figure has to be
  assembled by a formula the ledger does not already carry, the equity ledger is missing a writer,
  not the report a calculation.
- **The equity ledger is not a second copy of the equity account.** It is the itemisation whose sum
  the account must equal, and the audit compares them as two independent records (Audit A1.a). If it
  is ever *summed to produce* the balance, Law 4 has been broken and the check has become a
  tautology.

---

## The finding this item carries

### Every sovereign is rated `c`, and every firm downgrades in one session (item 12's finding **12-17**; `docs/RECORD.md`)

**Measured** at item 12's close, in `mechanisms/ratings/assess.ts`: at period 1 all four treasuries
are graded `c` by all three assessors; at period 4 every firm goes `aaa` to `c` at once for two of
the three.

**Why.** The one measure an assessor can make honestly today is what falls due against what the
issuer is WORTH — `owedIn` against `equity`, both reads of the issuer's own account through a view
with the prices closed. A treasury's book equity is deeply negative by construction: it owes its whole
debt and owns nothing. That is not a signal, it is what a state's balance sheet looks like, and a
state's capacity to pay is its TAX BASE, of which this world has no read. The firms move together
because the same common cost crosses them all in the same week.

**What this item gives it.** A published income statement per issuer — what it took in and what it
paid out, stated by the issuer itself (§48). With that read the measure becomes what falls due
against what it TAKES IN, which is a coverage ratio and is the measure an assessor actually uses. For
a treasury the same read is its receipts, which is its tax base stated by the party that collects it.

**Step this item owes it.** `ratings` reads the published report rather than the equity account, and
the grade distribution across issuers stops being one grade — tested as a spread, never as a target.

---

## Design

### Sub-item 12a.1 Kernel: the equity ledger (Law 4, Law 19, Reporting A2, G2)

**The problem.** `register.moveEquity({ party, delta, cause, through })` already receives a `cause`
for every move, and throws it away: only the `Running` balance survives. So comprehensive income is
recoverable exactly (Δequity, and it decomposes into revaluation plus the wire's `EquityEffect`s),
but nothing above the bottom line is. A report that wanted "revenue" would have to parse the
`reason` strings on money legs — recovering by inference a fact its writer knew and did not record,
which is the Law 19 defect this project is organised against.

**The change.** The register keeps an append-only `EquityEntry` per move: `{ party, period, cycle,
delta, cause, instruction? }`, where `cause` is the string the writer already passes and
`instruction` is the id when the move came from settlement. One writer (the register), appended in
`moveEquity`, never edited, no reversal (Register E2.a: a correction is a new entry).

- `RegisterReads` gains `equityEntries(party, from, to): readonly EquityEntry[]`.
- **It is not the balance.** `equityWalk` stays the accumulator and stays authoritative; the entries
  are the itemisation. A new contribution to the `accounts` family compares Σ entries against the
  walk over the same span, with the walk's own dust (Law 7) — two independent records of one thing,
  which is what Audit A1.a asks for and what makes the report checkable rather than merely produced.
- **Memory.** Entries are per party per period and the world runs 52 periods a year; the register
  already holds every lot of every holding, so this is the same order of growth. If a campaign is
  ever needed it is Law 18's and it changes no behaviour.

**What it is not.** It is not an accounting classification. Nothing here labels a move "revenue" or
"cost of sales" — the `cause` is the string the mechanism that moved the equity already wrote, and
what the report does with it is 12a.3's business. A `nature` enum on an equity move would be exactly
the "nature flag" ARCHITECTURE §4.9a refuses.

### Sub-item 12a.2 The fiscal calendar (Reporting A3, A4, G6; Money G3.a, G3.b)

**No kernel change.** `QUARTERLY` is already a `Periodicity` (`core/rate.ts`) and Money G3.a places a
periodicity by advancing a date, which `calendar/civil.ts` already does with an end-of-month
convention. A fiscal quarter is therefore a pair of dates, and "the periods whose dates fall in it"
is a read. G3.b's rule holds for free: nothing here is finer than a period.

- Each public company declares its **fiscal anchor** — the month its year ends — as data in the
  module's registry, so companies do not all report in the same week. Different anchors are what make
  reporting season a thing that happens repeatedly rather than once.
- **The lag** (A4): a POLICY parameter, `reporting.lag.days` — a rule somebody wrote about how long
  after the close a report must be out. Owner `parliament` (it is a disclosure rule), and it is the
  width of the only information asymmetry this world has.
- **The closed period** is what that lag *is*: between the quarter's last date and the publication
  date the firm's own module knows the figure and no view outside it may read it. Enforced by the
  door: the report is a journal event with `public: true`, and there is no other way out.

### Sub-item 12a.3 Module `reporting` — the report

- `id: 'reporting'`, `requires: ['firms', 'equity', 'expectations']`.
- **Who reports** (A1, A1.a, G4): a firm is public when its share line is live, has a market, and is
  held by parties other than its founders. That is a **read of the register** evaluated every period, not
  a flag and not a kind — `isPublic(view, firm)` is three register reads and no branch on anything.
  A firm that ceases to be public stops reporting in the period it stops.
- **Phase** `reporting.publish`, cycle 0, anchored `{ after: 'corporateActions' }`: for every public
  company whose fiscal quarter closed at least `reporting.lag.days` ago and which has not yet
  published for that quarter, record a public journal event `reporting.report` carrying:
  - **income**, from `equityEntries(firm, from, to)`: the entries grouped by the `cause` their writer
    wrote, plus the total, plus the revaluation subtotal (the entries whose cause the revaluation
    phase wrote). The groups are what the ledger says, in the ledger's own words — the report does
    not invent a chart of accounts.
  - **balance sheet** at the close, as `audit/families/accounts.ts` already computes it: assets from
    the register at marks, liabilities from what the firm issued read off its holders, equity from
    the walk. Named instruments, never buckets (Law 9).
  - **cash**, from the ledger's money legs for the firm over the span, with each leg's counterparty
    and reason — a direct-method statement, which is what the wire naturally produces.
  - **shares outstanding** at the close, so a reader can divide (G5) and nobody stores the quotient.
- **Restatement** (A5): a later `reporting.restate` event naming the original event and the corrected
  figure, with the original standing. It fires when a figure the report carried is later found to
  disagree with the books — which the accounts-family contribution in 12a.1 is what detects.

### Sub-item 12a.4 Guidance (Reporting B1–B4; Firm E7, E7.a)

The firm already forms and publishes an outlook of its own earnings (`firms/produce.ts`, item 4).
This item gives it the calendar and the lines and **does not give it a second number** (B4): the
guidance published is the same outlook object the firm's own production and investment decisions
read (§46 C2). Two numbers here would be Law 4's defect wearing a business suit.

- Published with the report, for the coming fiscal quarter, carrying horizon and unit (§46 A5).
- **Revision and withdrawal** between reports are their own events, fired when the firm's outlook
  moves by more than the dust of the arithmetic that produced it — never on a schedule, or the
  revision is a calendar and not information.
- A management persistently wrong (B3) is a **read** of its own past guidance against its own past
  reports. Nothing is stored: `view.guidanceRecord(firm)` computes it from the journal.

### Sub-item 12a.5 The estimate (Reporting C1–C6)

- Module `research`, `requires: ['reporting', 'banks', 'expectations']`. A bank participant, not a
  new party kind: the banks that exist do this.
- **The outlook is the bank's own** (C1, §46 A2, B1): formed adaptively from what *that bank*
  observed of *that company* — the reports it has seen, its own dealing flow in the share (§26), its
  own lending exposure to the issuer (§23), the prices in the company's own markets that the bank
  transacts in. It is the `expectations` module's machinery pointed at somebody else's earnings,
  which §46 C2.a is the clause that permits.
- **Never from the price** (C6). The estimate's inputs are stated in one function and the share price
  is not among them; a test asserts that moving the print with the company's state unchanged moves no
  estimate. That test is the guard, because the failure mode is silent.
- **Published** (C2) as a public journal event `research.estimate` with the bank, the company, the
  date, the lines, and the horizon.
- **Revision** (C4) on information — a report, a guidance change, an observation of the company's own
  markets — with its date and its size. A revision that no observation preceded is §46 B2.a's defect.
- **Disagreement is the point** (C3). Banks differ because their memories differ (§46 B1.a, drawn at
  entry) and because what each observed differs — a bank that lends to the name has seen its
  coverage, a bank that makes its market has seen its flow. Nothing disperses them by hand.

### Sub-item 12a.6 Coverage (Reporting D1–D3, D3.a)

- **Why**: the bank already forms the view for its own book (D1). Publishing is a decision, and it is
  taken for what publishing brings — the flow that comes to a desk that is known to have a view.
  Measurable, and measured at 16.
- **What it costs** (D2): analysts are **employed**, through the labour market that already exists
  (§39). Coverage of `n` names takes analyst hours, which is TECHNOLOGY, declared per name covered;
  the wage is whatever the labour venue clears at. There is no "research budget" parameter — the cost
  is a real payee, and a bank that cannot afford the desk drops names.
- **Initiate and drop**: a bank covers a name when the flow it expects justifies the hours, and drops
  one when it does not. Both are journaled events.
- **Uneven by construction** (D3, D3.a): nothing assigns coverage. A large widely-held name is in
  more banks' books, so more banks already hold the view, so more publish. The count per name is a
  read, and a test asserts the counts are not all equal — the cheapest possible guard against the
  defect D3.a names.

### Sub-item 12a.7 The surprise, and what it causes (Reporting F1–F4)

- **Settling** (F1): when a report publishes, every expectation standing against it — the management's
  guidance and each bank's estimate — is scored. Observed minus expected, per holder of a view, is
  §46 B2's surprise, recorded with the name of the party whose view it was.
- **What it causes** (F2): nothing in this item touches a price. The surprise is an observation; the
  `expectations` module already turns observations into revised outlooks; holders' revised outlooks
  already produce different reservations in the share book (§46 C3); the book clears. The whole
  causal chain is machinery that exists — this item adds the observation and *nothing else*.
- **F2.a is the test, not the comment.** A test asserts that the share market's participants read no
  `reporting.*` event directly: the only path from a surprise to an order is through the party's own
  outlook.
- **A bank's record** (F3) is a read of its own past estimates against the reports that settled them.
  It is what makes one bank's estimate weigh differently from another's in a holder's own outlook —
  §46 B3's confidence, applied to somebody else's forecast, and computed the same way.
- **F4**: guidance missed reaches the cost of capital through channels that exist — holders' outlooks,
  the lender's view of the name (§46 C4), the assessor's state-based judgement (§44 A2). No charge is
  applied to the firm anywhere in this item.

### Sub-item 12a.8 Consensus as a read (Reporting E1–E3)

- `view.consensus(company): ConsensusRead` — computed from the estimates that exist at the moment of
  reading: the count, the spread, and the aggregate, with the date of the oldest estimate in it so a
  reader can see how stale it is.
- **Stored nowhere** (E3), exactly as an index is a read of its constituents (§22 A2, and item 12
  builds that pattern).
- **Read by nothing that decides** (E2). It reaches the observer surface, and it reaches a party only
  the way any published statistic does — as one more thing observed, with its lag (§46 A2.a). A lint
  or a test naming the modules that may import it is the guard; the failure is silent otherwise.

### Sub-item 12a.9 Audit and the observer

- **`accounts` contribution** (12a.1): Σ equity entries over a span equals the walk's movement over
  the same span, per party, within the walk's own dust. This is what makes the income statement
  checkable rather than merely produced, and it is the check that catches a restatement (A5).
- **`names` contribution**: every estimate names a bank and a company that exist and are alive; every
  report names a company that was public in the quarter it covers.
- **`flows` contribution**: the analyst cost has a payee and settled (Law 5).
- **Observer**: a `StatementsView` per public company — the three statements, the guidance, the
  estimates with their banks and dates, the consensus read with its staleness, and the last
  surprise. Display only; §45 B2.a — looking changes nothing.

---

## Steps

- [x] 1. `EquityEntry` and the register's append-only list; `moveEquity` writes one; `RegisterReads`
      and the audit view gain `equityEntries(party, from, to)`. `EquityMove` now carries the period,
      the cycle and the instruction, which the type change caught at all five writers. Settlement's
      cause is the instruction's OWN reason (its writer's sentence), not `instruction <id>`, and
      `stateEquity` writes the opening as an entry so Σ entries IS the balance with nothing left
      over to argue about.
- [x] 2. The `accounts` family contribution (`equityLedgerFamily`): Σ entries against the walk's
      value within the walk's own dust, AND the COUNT of entries against the walk's count of moves —
      a sum can be made to agree by two errors, a count cannot. It found one immediately: a cell
      split copied the walk and not the itemisation, so a split cell's account said eighteen moves
      and its ledger carried nine. The itemisation is per-member state and now travels with it.
- [x] 3. `test/equity-ledger.test.ts`: the count says none is missing, any span sums to what the
      account did over that span (measured against the balance read at both ends, which is the read
      a report is built on), the split carries its history, and `equity()` is still exactly
      `equityWalk().value` — so the balance is never produced by summing the entries (Law 4).
- [x] 4. The fiscal calendar as a read (`reporting/fiscal.ts`): a company's anchor month DRAWN from
      the world's seed and its own identity (so years end in different months and reporting season
      is continuous rather than one week), the quarter's dates from `civil.ts`, the periods in it
      from the calendar. Nothing counts periods anywhere (G6). `MONTHS_IN_YEAR`, `MONTHS_IN_QUARTER`
      and `QUARTERS_IN_YEAR` moved into `calendar/civil.ts`, where the Gregorian year is arithmetic
      rather than a parameter.
- [x] 5. `reporting.lag.days` declared POLICY, owner `parliament`, with its reason: it is the width
      of the only information asymmetry this world has.
- [x] 6. `isPublic(ctx, firm, line)` as register reads, with `listedLineOf` asking the kind's
      PROFILE (the one instrument a party issues that is not a liability of its issuer and that a
      market prices) rather than any kind id — so nothing branches on what sort of thing a share is
      (Law 15). Tested: only listed companies report, and the firms that are not listed say nothing.
- [x] 7. The `reporting.publish` phase and the `reporting.report` event: income from the equity
      entries grouped by the instruction's own cause, plus the revaluation subtotal, plus the bottom
      line which IS the sum of the decomposition. A quarter that opened before the epoch is not
      reported — it has no books in it, which is an absent period rather than a short one.
- [x] 8. `balanceSheet(view, party)` extracted from `audit/families/accounts.ts` and used by BOTH
      the family and the report — one implementation, structurally typed so an `AuditView` and a
      `MechanismContext` can each pass what they have (Law 4).
- [x] 9. The cash statement from the ledger's own money legs, grouped by NAMED counterparty, money
      and cause — every key a fact the wire wrote, so nothing is bucketed and nothing invented
      (Law 9, A2.a). A quarter is two thousand payments and two thousand rows is the ledger printed
      out; 321 lines naming assessors, banks, the fund and the household cells is a statement.
- [x] 10. Shares outstanding in the report, read off the line; a test that no per-share figure is
      stored anywhere (G5: earnings per share is income over shares, both of them reads).
- [ ] 11. `reporting.restate` and its trigger: a figure the report carried that the books later
      disagree with, caught by 12a.2's family contribution.
- [ ] 12. Guidance published with the report, from the firm's existing outlook object; a test that the
      published number and the number the firm's own decisions read are the same object (B4).
- [ ] 13. Guidance revision and withdrawal as events, fired on a move bigger than the arithmetic's
      dust, never on a schedule.
- [ ] 14. `view.guidanceRecord(firm)` as a read of the journal; nothing stored.
- [ ] 15. Module `research`: the estimate outlook, formed from the bank's own observations of the
      company, published as `research.estimate`.
- [ ] 16. **The C6 test**: move the share print with the company's state unchanged; no estimate moves.
- [ ] 17. Estimate revision on information, with its date and size; a test that a revision with no
      observation behind it fails §46 B2.a.
- [ ] 18. Coverage: analyst hours as TECHNOLOGY per name covered, hired through the labour venue, paid
      to a named payee; initiate and drop as decisions.
- [ ] 19. **The D3.a test**: the counts of estimates per name are not all equal, and no name is covered
      by every bank.
- [ ] 20. Settling: on a report, score the guidance and every estimate; record the surprise per party.
- [ ] 21. **The F2.a test**: no participant in the share market reads a `reporting.*` or `research.*`
      event; the only path from a surprise to an order is the party's own outlook.
- [ ] 22. `view.consensus(company)` computed at read time with its staleness; a test that it is stored
      nowhere and that no deciding module imports it (E2, E3).
- [ ] 23. The `names` and `flows` contributions.
- [ ] 24. The observer's `StatementsView`; the browser smoke test shows one company's three statements,
      its guidance, its estimates and the consensus.
- [ ] 25. A year-long run: reports land on their own calendars, estimates disagree, surprises are
      recorded, and the audit is green every period.
- [ ] 26. `ratings` reads the published report rather than the equity account (the finding above):
      what falls due against what the issuer TAKES IN, which for a treasury is its receipts. Test:
      the grades across issuers are a spread and not one grade — measured, never targeted.

---

## Tests

- `equity-ledger.test.ts`: entries sum to the walk over any span; a busy party in one period; a
  revaluation-only period; a party born mid-run.
- `reporting.test.ts`: who is public and who is not; the quarter's dates; the lag; the report's three
  statements against the same reads the audit uses; a restatement.
- `guidance.test.ts`: one outlook, published and acted on; revision; withdrawal; the record read.
- `research.test.ts`: two banks with different histories of one name estimate differently; the price
  moves and no estimate does (C6); a revision follows an observation; coverage is uneven (D3.a); a
  bank that cannot afford the desk drops a name.
- `surprise.test.ts`: a report settles guidance and every estimate; the surprises are recorded with
  their parties; the share book's schedules differ in the period after; no module wrote a price.
- `consensus.test.ts`: computed from the estimates present; changes when one is revised; stored
  nowhere; no deciding module imports it.
- Property: for any sequence of equity moves, Σ entries = walk movement, exactly (integers, no
  tolerance — item 10.3's grid).

## Exit criteria

- `npm run check` green.
- `npm run coverage:spec` recounted: Reporting A–G MET, H1–H4 PARTIAL naming worklist 16.
- `reads.placeholders` unchanged — this item introduces none. Any number it adds is a POLICY, a
  PREFERENCE or a TECHNOLOGY with its reason, or it does not go in.
- The year-long run green, with reports on at least two different fiscal anchors and at least one
  name covered by more than one bank and one covered by none.
- `docs/ARCHITECTURE.md` updated in the same change: §4.9a gains the equity ledger and why it is an
  itemisation rather than a second account.
- `docs/RECORD.md` entry; `docs/COVERAGE.md` re-marked; this file deleted; the manifest row stays.
