# Project Phoenix — the implementation

> **This file replaces `docs/AUDIT.md`.** It carries every open finding that file held, adds the
> sectors that file could not see, and states each as work with files and steps rather than as
> evidence. `docs/AUDIT.md` is deleted in the change that lands this file.
>
> **One item at a time, in the order in Part 1 (Law 10).** An item is done when its exit holds, its
> steps are ticked, `docs/RECORD.md` has its entry and `docs/COVERAGE.md` is re-marked — one commit
> (Law 14). When an item closes, delete its section; the record says what it closed.
>
> **A finding under an item is evidence for the change, not a task of its own** — but a finding that
> is a MISSING SECTOR is an item, not evidence. That distinction is why this file exists: the file
> it replaces filed the absence of six sectors as a finding (`C-4`) under an item it then marked
> done, and the rule "do not chase a finding" kept anyone from looking at it again.
>
> **No test is run until an item's steps are complete.** Lint and typecheck are cheap and may run at
> any time; the suite is a measurement and measurements come last (Law 11).

---

## Part 0 — The measured state

### 0.1 What exists, by spec system

`docs/COVERAGE.md`, one row per REASON, VERIFY and FORBID, aggregated per spec system by
`npm run check:existence`. **1,361 clauses: 815 MET, 90 PARTIAL, 456 MISSING — and 95 of the MET
carry `NEVER REACHED`**, meaning a module cites the clause and has never produced an outcome. So
**641 of 1,361 clauses (47%) are missing, partial, or dead.**

Which system a clause belongs to is the SPEC's fact and is joined from the spec index, never parsed
out of COVERAGE's own headings (Law 4). A clause is counted where the spec puts it, and a spec
sub-clause carrying no REASON/VERIFY/FORBID word is a NOTE rather than a requirement — **8 `MET`
rows mark one of those** (`Sovereign I1.a`, `Banks Lending C1.d`, `XI-11`, `Labour A3.b`,
`Households A2.b`, `Households F1.a`, `Households F1.b`, `Households F2.a`) and are outside this
count. The tool names them on every run rather than absorbing them: two documents disagreeing about
what a requirement IS is the kind of thing this check exists to show. Every spec clause has a row —
checked, not assumed.

| system | MET | PARTIAL | MISSING | NEVER REACHED | total |
|---|---|---|---|---|---|
| Money | 28 | 0 | 8 | 0 | 36 |
| Register | 20 | 4 | 2 | 0 | 26 |
| Clearing | 21 | 5 | 1 | 0 | 27 |
| Audit | 20 | 3 | 0 | 0 | 23 |
| Seed | 16 | 0 | 6 | 0 | 22 |
| Currency | 24 | 0 | 1 | 0 | 25 |
| Bond | 14 | 1 | 1 | 0 | 16 |
| Derivative | 18 | 0 | 0 | **3** | 18 |
| **Corporate Credit** | **13** | 8 | **41** | 0 | 62 |
| Sovereign | 40 | 3 | 8 | **3** | 51 |
| Short-Term Debt | 12 | 3 | 4 | 0 | 19 |
| Equity | 25 | 2 | 10 | 0 | 37 |
| Money Market | 25 | 3 | 0 | 0 | 28 |
| Spot FX | 26 | 1 | 0 | 0 | 27 |
| Fund Shares | 23 | 3 | 0 | 0 | 26 |
| Securities Lending | 14 | 0 | 7 | **14** | 21 |
| **Prime Brokerage** | **0** | 0 | **24** | 0 | 24 |
| Derivative Layer | 32 | 0 | 0 | **17** | 32 |
| CDS | 17 | 0 | 8 | **17** | 25 |
| IRS | 14 | 1 | 5 | **13** | 20 |
| FX Forwards | 13 | 0 | 8 | 0 | 21 |
| Commodity Futures | 5 | 0 | 15 | **5** | 20 |
| **Commodities Spot** | **3** | 1 | **20** | 0 | 24 |
| Indices | 21 | 1 | 0 | **1** | 22 |
| Banks Lending | 21 | 5 | 6 | 0 | 32 |
| Banks Funding | 28 | 4 | 0 | 0 | 32 |
| Banks Capital | 19 | 3 | 1 | 0 | 23 |
| Dealer Desks | 26 | 1 | 0 | **2** | 27 |
| Insurers | 9 | 0 | 14 | **9** | 23 |
| **Hedge Funds** | **0** | 0 | **24** | 0 | 24 |
| **Private Equity** | **0** | 0 | **25** | 0 | 25 |
| Treasury | 20 | 1 | 4 | 0 | 25 |
| Central Bank | 22 | 3 | 4 | 0 | 29 |
| **Polity** | **0** | 0 | **32** | 0 | 32 |
| Firm | 20 | 7 | 3 | 0 | 30 |
| Capital Programme | 22 | 3 | 0 | 0 | 25 |
| Firm Birth | 7 | 6 | 12 | 0 | 25 |
| M&A | 10 | 0 | 12 | **10** | 22 |
| Trade Credit | 8 | 3 | 11 | 0 | 22 |
| Goods | 27 | 2 | 10 | 0 | 39 |
| Freight | 17 | 3 | 0 | 0 | 20 |
| Labour | 23 | 1 | 3 | 0 | 27 |
| **Housing** | **6** | 1 | **19** | **3** | 26 |
| Households | 20 | 4 | 9 | 0 | 33 |
| **Small-Business Pools** | **2** | 0 | **26** | 0 | 28 |
| **Cross-Border** | **6** | 2 | **18** | 0 | 26 |
| Ratings | 16 | 3 | 4 | 0 | 23 |
| Reporting | 32 | 5 | 1 | 0 | 38 |
| Observer | 16 | 0 | 10 | 0 | 26 |
| Expectations | 17 | 2 | 8 | 0 | 27 |

Regenerate with the check in item 1, never by hand.

### 0.2 The ten systems that produce nothing

**Five have no clause MET at all.** A sixth, Small-Business Pools, has two, and both are the generic
cell kernel (`parties/party.ts`, `world/cells.ts`) rather than anything that is a small firm.

| sector | spec | MET/total | who owns it today |
|---|---|---|---|
| **Polity** | §47 | 0 / 32 | worklist **14** → item **19** |
| **Private Equity** | §29 | 0 / 25 | worklist **13o** → item **13** |
| **Prime Brokerage** | §15 | 0 / 24 | worklist **13o** → item **13** |
| **Hedge Funds** | §28 | 0 / 24 | worklist **13o** → item **13** |
| **Short-Term Debt** | §9 | 0 / 19 | **nobody** → item **10b** |
| **Small-Business Pools** | §42 | 2 / 28 | **nobody** → item **11** |

**And five more are built on paper and dead in the world** — every clause they have MET carries
`NEVER REACHED`, so a module cites it and has never produced an outcome. The check names these
separately because a reader scanning for "0 MET" walks straight past `M&A 10 MET`, and all ten of
those are marks on a mechanism nothing has ever invoked.

| sector | spec | MET (all never reached) / total | the finding | item |
|---|---|---|---|---|
| **CDS** | §17 | 17 / 25 | `A-66`, `B-7` — the book cannot produce a first print | **6** |
| **M&A** | §35 | 10 / 22 | `B-4` — the market for control buys shares and never combines | **6**, **13** |
| **Insurers** | §27 | 9 / 23 | `A-9`, `B-2` — no policy can be registered in any world | **9**, **14** |
| **Securities Lending** | §14 | 9 / 21 | `A-67`, `B-3` — nothing ever borrows a security | **9** |
| **Commodity Futures** | §20 | 5 / 20 | `A-66`, `B-7` | **6**, **18** |

**Ten of fifty-one systems produce nothing.** Five were never written; five were written, marked
`MET`, and never reached. The second kind is the more dangerous, because it reads as done.

### 0.3 How six things were lost: the handoff chain

Traced from the worklist's own rows. Each arrow is an item closing and naming where its unbuilt half
goes.

```
13e done → PLACED small-business cells, the covered bond (M7), factoring,
           receivable pledges, senior notes as repo collateral  →  13f
13f done → its PLACED list names NONE of those five.                     ← dropped
13f done → prime brokerage, restructuring → 13h ;  short-term debt → 13i
13h done → "...came in from 13f and 13g and go on with them"             ← no destination
13i done → "hedge funds, private equity ... came in from 13h"            ← 13i closed; both 0
13n open → states it itself: "PLACED forward three times (13f to 13g to 13h) without landing"
```

Six things were placed into items that then closed without them: **small-business cells, factoring,
receivable pledges, senior notes as repo collateral, the covered bond (M7), short-term debt.** None of
the six had an open home when this file was written. Hedge funds, private equity and prime brokerage
survived only because 13o happens to name them.

This is finding **B-14** — *"a finding was positioned into item 13h, 13h closed, and the finding was
not done"* — repeated at least six more times, including once at the top level of `docs/AUDIT.md`
itself: its item **18** was titled *"The sectors that were waiting"*, owned four findings, and was
marked **DONE** having built one of them (`C-3`). `C-4`, the row naming the six absent sectors, was
inside it.

### 0.4 Why it was invisible, and the check that ends it

Three reinforcing reasons, and the third is the one to fix:

1. `docs/AUDIT.md` was produced by **reading `packages/engine/src`**. An absent sector leaves no
   trace in source, so the method could not find one. It found the six only by carrying them in from
   a deleted holding pen.
2. `docs/COVERAGE.md` is the file that *can* answer the question — one row per clause — and nothing
   ever **aggregated** it. The query is one `awk` line and had never been run.
3. `MET` is a claim about the source, not about the world, and the file says so in its own header —
   which means a reader who wants to know what EXISTS has to hold two facts in their head at once and
   nothing checks the pair.

**`npm run check:existence` is that check**, and Part 0's table above is what it verifies against.

---

## Part 1 — The order

Dependencies, not preference, and the open lines before the new ones. The two arrows that matter:

- **11 (Small-Business Pools) before 12 (firm birth)**: a firm is born SMALL and small firms are
  §42's cells, so there is nowhere for a birth to happen until that sector exists — and the
  promotion out of it (A6.c) is the other half of the same life cycle. Building birth first would
  mean firms appearing directly as named parties, which is the modelling line A6.b forbids.
- **10 (the corporate bond is issued, DONE) before 10b (short-term debt)**: a firm that cannot issue
  paper at five years cannot issue it at three months either; the issuance path is one mechanism, and
  10b now has one to shorten rather than one to build.

| # | item | closes | why here |
|---|---|---|---|
| **10d** | A bank issues a bond the way everybody else does | — | **inserted** (owner): deletes the THIRD issuance mechanism. Needs nothing — item **10**'s path and `publishReservations` are both there |
| **11** | Small-Business Pools (§42) | — | **inserted**: dependencies (trade credit 13e, bank lending 13d) are both closed and item 9 gave it the agreement; takeable now, and **12 needs it** |
| **12** | Firm birth, and the boundary firms cross | 3 | **needs 11**: a firm is born SMALL, which is §42's sector, and is promoted out of it when it outgrows one. Also **7** (`Lifecycle`, built) and **15** (`Objective`, built); worklist 13n |
| **13** | Asset managers: §28, §29, §15 | 1 | `Mandate` exists (item 9.2a), so it is unblocked; worklist 13o |
| **14** | Insurers and pensions (§27) | 2 | the insurer exists and can write a policy (items 9.3, 9.5); what is left is the sector and pensions |
| **15** | Housing, the rest | — | needs **3** and **7**; worklist 13m, and `E-5` |
| **16** | Cross-border, the rest | — | needs **10b** (foreign-currency issuance) and **13** |
| **17** | Corporate credit, the rest | — | needs **10**; 51 clauses, the largest single gap |
| **18** | Commodities spot and futures | — | 35 clauses across two systems; needs **6** for the futures half |
| **19** | The polity (§47) | 3 | worklist 14; carries `D-1`, `D-2`, `E-7` |
| **20** | Periodicity: the fee and the assessment | 1 | worklist 13k; needs **19** for who sets a fiscal period |
| **21** | The local repairs | 3 | each belongs in the next change that opens its file |
| **22** | The recipe | — | worklist 15; last before measurement, deliberately |
| **23** | Measure (Part XII) | 1 | worklist 16; carries `C-1`'s 82 red |
| **24** | The app and the APK | — | worklist 17 |

**Stage B is finished**: every line the old file left open is closed. **Stage C (10d–18)** builds the
sectors that are not there; **10 is closed** and its section is gone. **10c and 10d are the owner's
two corrections of 2026-09-14 turned into work, and both are net DELETIONS** — a kind branch and a
duplicated issuance mechanism. **Stage D (19–24)** is the existing worklist tail. Items 1–10c and 7b are
closed and their sections are gone: this is the plan of what is left, and `docs/RECORD.md` is the
ledger of what was done.

---

## Part 2 — The items

---

## 10d. A bank issues a bond the way everybody else does — **inserted**

**Where this came from.** Owner, 2026-09-14: *"Why does bank sub debt has no price? They are just
bonds as much as corporate bonds"*, then *"You should roll the bank bonds mechanism in the same way
corp bonds work. There shouldn't be 3 different mechanisms."* Both are right, and the second is the
bigger of the two: the missing price is a SYMPTOM and the third mechanism is the cause (Law 12).

**The measurement.** `banks/subordinated.ts`'s own header says *"C2, C2.a, C2.b: RAISING IT IS A REAL
ISSUE INTO A REAL MARKET… the bank posts a size and no level (Clearing C3) and takes what the book
gives it."* It is a real issue into a real book. It is also **a private copy of the issuance
machinery the kernel already has**, and the copy has drifted from the original in four ways:

```
banks/subordinated.ts:185   raiseVenue(bank)          a venue of its own
                     :234   ctx.openVenue({...})      opened by the module
                     :246   clear(ctx.posted(venue))  the module calls the solver ITSELF
                     :255   asRatio(outcome.price)    the book clears a RATE, not a price
                     :300   let id = subId(bank, 1)   ONE INSTRUMENT PER LENDER
                     :313   ctx.issue({... market: none() })
```

1. **It clears its own book.** The treasury and item 10's corporate issuer both bring a size and a
   walk-away to a KERNEL market and let the one solver strike it. This one posts into a venue it
   opened and calls `clear` by hand. Three issuers, two mechanisms, and the spec has one primary
   market (Law 4).
2. **It writes one line per LENDER.** `subId(bank, n)` advances per fill, so a bank that raises from
   three investors ends up with three instruments carrying the same promise. That is the exact
   opposite of Law 9 and of what item 10 established one commit ago: one line per issuer per
   maturity, named as a market names it, and a second name for one promise is one promise written
   twice.
3. **It clears a RATE**, which is `E-11`: a book struck a level and the print store cannot say which
   of the two kinds of level it is, so nothing printed. A BOND book clears a price per unit of par.
4. **It hand-rolls its own two-sided instruction** in `writeSub`, where the kernel's primary market
   already settles an issuer's sale against the book in one instruction.

**And the missing price falls out of all four.** `market: none()` per line, so `pricing:
'carriedAtCost'`, so a bank's capital layer has no price — and that is not cosmetic. Subordinated
debt is the instrument whose price moves FIRST when a bank's solvency is doubted, before its equity
and long before a depositor notices. It is what makes Banks Capital **D2**'s bail-in legible: a
write-down lands on a layer whose value everybody could already watch falling. A world where the
capital layer has no price is one where a bank deteriorates invisibly in the one instrument built to
show it, and the resolution arrives as a surprise to holders who had no number to watch.

**This item is a DELETION.** It needs nothing new: item 10's path already opens a market per line,
and `banks/index.ts:publishReservations` already makes **every live bank an obligor** — it says so
explicitly, *"a bank is a name anybody may lend to whether or not it has paper outstanding right
now"* — so the holders' schedules a book needs are already published every period.

### Steps

- [ ] 10d.1 Delete `raiseVenue`, the `openVenue`, the `ctx.post` calls, the `clear`/`isCleared` import and the hand-rolled legs in `writeSub`. What replaces each is named in the same change (Law 19): the kernel's `ctx.issue` + `ctx.openMarket` + `ctx.offer`, and the primary market's own settlement.
- [ ] 10d.2 **One line per bank per maturity**, found by its own name exactly as `corporateBondId` does it — so a bank coming back taps the line it has instead of minting a fourth. `subId(bank, n)` goes.
- [ ] 10d.3 **The one real design question, and it is not settled here.** Clearing C3 says the bank brings *a size and no level*, and `PrimaryOffer.reservation` is a number. A firm's walk-away is the price at which the issue costs it what its bank quoted (item 10); a BANK's alternative to subordinated debt is raising equity (C2) or shrinking (10c's securitisation), so its walk-away is the price at which this costs the same as those. That is the same construction, not a new one — **but check it against C3 before building it**, because "no level" may mean the reservation belongs elsewhere entirely. Do not invent a floor to stand in for an alternative.
- [ ] 10d.4 `subordinatedKind.pricing` becomes `'cleared'`, `carry` follows, and the stale comment (*"nothing trades these here, so they are carried at what they cost"*) goes in the same change — a stale comment is a defect (Law 16).
- [ ] 10d.5 Check every other reader of that profile before flipping it: the estate, the resolution write-down (D2), and **10c's securitisable read**, which should stop admitting it — you do not securitise a bond you can sell.
- [ ] 10d.6 `E-11` loses one of its five books. Say so in its index row; a rate-quoted primary raise that became a price-quoted bond book is one fewer place the print store has to learn a new trick, and the finding shrinks rather than closes.
- [ ] 10d.7 COVERAGE re-marked for Banks Capital A2.b, C2, C2.a, C2.b and D2, and the record entry saying what was deleted and what read replaced it.

### Exit

A bank's subordinated paper is one named line with a cleared price that falls when the bank does, a
bail-in writes down a layer somebody was already watching, and **there is one issuance mechanism in
this world, used by a treasury, a firm and a bank alike.**

---

## 11. Small-Business Pools (§42) — **inserted**

**Where this came from.** Worklist 13e closed with *"PLACED: **small-business cells**, the covered
bond (M7), factoring and receivable pledges, and senior notes as repo collateral → 13f"*. **13f
closed and its PLACED list does not mention any of the five.** `docs/AUDIT.md` carried the fact in a
single row of finding `C-4` — *"Small-business pools (spec 42), carried from 13e and never built"* —
under item **18**, which was then marked DONE.

**§42 is 2 of 28 MET, and both are the generic kernel**: `A6` cites `parties/party.ts` and
`registry/profiles.ts` (cells exist), `E5` cites `audit/families/units.ts` and `world/cells.ts` (a
weight is a count). Nothing in the engine is a small firm. The one mention in 59,927 lines is a
comment at `firms/data.ts:124`.

**Inserted here** because both its dependencies are closed — trade credit (13e) for A4, bank lending
(13d) for A5 — and item 9 gave it the loan agreement B1 needs. **It is takeable now**, and item 12 needs it: a firm is born SMALL, so there is nowhere for a birth to happen until this sector exists.

**Why it matters beyond its own 28 clauses.** A5.a: small firms are bank-dependent, *"which makes
them the sector where a credit tightening bites first and hardest"*. Without them a credit tightening
has nowhere to bite: the model's only borrowers are named firms large enough to reach a bond market.
And §36 A4 — trade credit *"is the tier that lives on it"* — so 11 of Trade Credit's 22 clauses are
missing partly because the tier below them does not exist.

### What it is

`src/mechanisms/small-business/`, one `SystemModule`. **A pool with a distribution, never an
average** (A2, A2.a: *"Default is a threshold event; with one average firm a mean-preserving spread
causes no defaults, and the entire credit content of the sector is gone"*).

- **The sector** (A1): small firms are **firms** — they sell, employ, borrow and can fail. Not a new
  kind of thing; a firm with a weight.
- **Cells** (A6): each a named party with a **WEIGHT** — an integer count of the firms it is —
  carrying its own state, borrowing from a **named** lender on its own loan row, and selling to and
  buying from named counterparties on trade credit.
- **A6.a is the design rule and it decides the whole data model.** *"Every relationship that must be
  named is either a dimension of the cell's KEY or a REGISTER ROW, never an attribute averaged inside
  it."* Its **region** and its **bank** are dimensions of the key; its **lender** is a loan row per
  (lender, cell). *"A relationship averaged inside a cell is a relationship the model cannot name,
  and law 4 is broken quietly."*
- **A6.b**: a weight of **one** is a named firm — so the boundary between this sector and Corporate
  Credit's is **not a modelling line but a SIZE**.
- **A6.c**: a cell that **outgrows A5** — large enough to reach the bond market — is **promoted to a
  named firm**, and the promotion is an event with a cause. This is XI-15's `promotion`, one of the
  three weight events that **never fire today** (`A-17`). *"Without it the boundary is arbitrary and
  no firm can ever grow across it."*
- **Observable characteristics** (A3): size, sector, region, leverage, coverage — and **losses depend
  on the DISTRIBUTION of those, not the mean**.
- **Connected both ways** (A4): they employ people and buy from and sell to larger firms, **including
  on trade credit**.
- **The loans** (B1–B4): each a loan from a **named lender** with a rate, a term and an amortisation;
  often **secured** on the firm's assets or the owner's house; they **default**, and the default
  depends on the individual firm's cash flow aggregated over the pool; **defaults are correlated**
  (B4) — same rates, same demand, same region — so the pool's loss is **not the sum of independent
  draws**, which is what makes C's tranching meaningful or dangerous.
- **The pool as an instrument** (C1–C6): transferred into a **vehicle** — a named party holding them,
  funded by issuing claims; **tranched by seniority** with **stated** boundaries; each tranche has a
  **price that clears**; held by **named holders**, *"and that is where the loss actually lands"*;
  C4.a — *"often the originating bank keeps the bottom, which means the risk did not leave"*. This is
  the vehicle item 8 fixes, reused, not a second one.
- **Why it matters** (D1–D4): it moves credit risk from banks to investors; it **frees bank capital**,
  so securitisation is a **lending-capacity mechanism**; the senior tranche is **collateral**; and
  when B4's correlation is worse than the tranching assumed the **senior takes losses it was not
  supposed to** — D4.a, **emergent from B4 and C2, never a scripted event**.

### What must not happen (E1–E6 — each is a test)

- **E1** no pool without underlying loans to named borrowers. *"Tranching a loss rate yields senior
  notes that can never be touched."*
- **E2** no tranche without a holder. **E3** no risk transfer without a transferee.
- **E4** **no constant pool population.** *"If entry is the accounting identity of exit, the
  population is constant by construction and nothing the sector experiences can change it."*
- **E5** no weight that is not a count — five events only, each with a cause and a date (XI-15).
- **E6** **no loss allocated to a pool rather than to its cells.** *"A loss struck against the pool
  and spread back over its members has been evaluated at an average, which is A2.a one level up."*

### Steps

- [ ] 11.1 The module skeleton: `requires: ['firms', 'banks', 'trade-credit']`. Kinds, profiles, units, params, phases anchored at `corporateActions | markets | revaluation`, participants per party kind, audit contributions, a seed contribution. Every clause cited with `@spec`.
- [ ] 11.2 The party kind: a firm cell. `representation: 'cell'`, `fails: ['cash','solvency']`, `borrows: true`, an `objective` (item 15 of the old file requires one), and a key whose dimensions are **region** and **bank** (A6.a) — never its lender, which is a register row.
- [ ] 11.3 Per-member state: what it holds, what it owes, its cash flow, its leverage and its coverage. **`integrate(f)` and never a mean** (XI-15). A partial event splits the cell.
- [ ] 11.4 The seed draw: a distribution of sizes, sectors and regions — **A3's distribution is the point**, so the draw must produce dispersion and the test must show a mean-preserving spread changing the count of defaults (A2.a). No representative small firm.
- [ ] 11.5 They sell and buy (A4): wire them into the goods markets as both sides, and into `trade-credit`'s `termsOffered` as buyers — which is where **A-63**'s "a household gets thirty days on its bread" was fixed by asking what kind of party the buyer is; a small firm is the party that SHOULD get terms.
- [ ] 11.6 They employ (A4): a `venueParticipant` into the labour venue — which item 3 made reachable.
- [ ] 11.7 The loans (B1, B2): a row per (lender, cell) through item 9's agreement, with a rate the bank **quoted** (Banks Lending C3.a — one rate per loan, what was quoted), a term, an amortisation, and security on the firm's assets.
- [ ] 11.8 Default (B3): from the individual cell's cash flow, aggregated over the pool — a **threshold event per cell**, never a hazard rate (Appendix B).
- [ ] 11.9 Correlation (B4, B4.a): the same rates, the same demand and the same region reach every cell in a region, so the correlation is **emergent from shared causes** and is not a parameter.
- [ ] 11.10 **A6.c, the promotion — the DOOR and the threshold.** A cell whose size clears A5's bond-market threshold leaves the sector as a named firm. This fires `cells.weight` with kind `promotion` — one of the XI-15 events with no caller — and is half of `A-17`. Build the door, the threshold and the state that travels with the member; **item 12.4 is where it actually fires because a firm GREW**, which needs 12.1's births, so the pool is not a fixed stock draining upward. The representation question — a split of one member leaves a cell of WEIGHT ONE, and A6.b says a weight of one is a named firm — is settled in 12 and recorded there; write this step against whatever that says.
- [ ] 11.11 The pool into item 8's vehicle: C1–C5 reuse the securitisation module. Do not write a second waterfall (Law 4).
- [ ] 11.12 The six FORBIDs as six tests, E4 and E6 especially — E4 needs births (item 12) to be non-vacuous, so write it here and expect it red until 12 lands, and **say so in the record** rather than deleting it.
- [ ] 11.13 COVERAGE re-marked for all 28 clauses; record entry; `check:existence` shows §42 is no longer an absent sector.

### Exit

A credit tightening reaches a small firm before it reaches a large one; a mean-preserving spread over
the pool changes the count of defaults; a cell can be promoted to a named firm, so the boundary
between the two sectors is a size rather than a modelling line — and item 12 is what makes a firm
arrive at that boundary from below rather than start there.

---

## 12. Firm birth, and the boundary firms cross (worklist 13n)

**Why.** `world/cells.ts` implements XI-15's five weight events. The callers are:

| door | callers |
|---|---|
| `cells.split` | `labour/matching.ts`, `households/lifecycle.ts` |
| `cells.reKey` | `households/lifecycle.ts` (ageing) — the only `promotion` in the world |
| `cells.die` | `households/lifecycle.ts` (after probate) |
| `cells.merge` | **none** |
| `cells.weight` | **none** |

So **entry never happens: no person is ever born in this world**, and no firm either. `age()` moves
members from cohort 0 into cohort 1 and nothing moves into cohort 0; `die()` removes them at the top.
The population is monotonically non-increasing from the seed, by construction and not as an outcome.
`estate` kills firms and nothing creates one.

Appendix B forbids a *declared birth rate*, which is right. What is absent is **the mechanism that
would produce births as an outcome** — a decision somebody takes, with its own cause — and nothing
naming it as absent. Under Part II that makes it neither MISSING nor OUT OF SCOPE but **unstated**,
which is the one thing a clause may not be.

**Entry is what makes a market contestable.** Without it a survivor's margin is never competed away
and every concentration measure is one-way.

### A firm is born SMALL, and it is born in the pool

**This is the correction that decides the item's shape** and it is why 12 now sits behind **11**.
Nobody founds a company with a bond line and a dealing desk. A firm starts as one of many small ones
— §42's sector — and §42 A6 already says what a small firm IS here: a **member of a cell**, an
integer count of firms carrying its own per-member state, borrowing from a named lender on its own
row. So a birth is **`cells.weight(cell, 'entry', 1, cause)`** into a small-business cell, funded by
somebody real, and NOT a named party appearing with an equity cheque.

Two things follow, and both are the point:

- **The named corporate sector is not where firms come FROM.** It is where they ARRIVE. Item 11's
  A6.c promotion is the arrival, and until entry exists it can only ever move firms that the SEED
  put in the pool — a fixed stock draining upward, which is E4's "entry is the accounting identity
  of exit" wearing a different face.
- **The whole life cycle runs through one sector.** Born in the pool, grows or does not, is promoted
  out of it if it clears §42 A5's bond-market threshold, and dies in either place (`estate`). The
  boundary between §42 and Corporate Credit is then **a SIZE a firm actually crossed** rather than
  the modelling line A6.b says it must not be.

**The hard part is the representation, and this item must answer it rather than assume it.** A
promotion takes ONE member out of a cell and makes a named party of it, and the member's per-member
state has to go with it — which is the same rock `13d.1` ran aground on four times and solved with
`reKeyCell`: nothing moves, the cell SPLITS (exact, per-member state and all) and the part that left
carries a different key. A split of one member leaves a **cell of weight one**, and §42 A6.b says a
weight of one IS a named firm — so either that is the answer and `representation` is a fact about
the WEIGHT rather than a field, or crossing to `representation: 'named'` needs a kernel event that
does not exist. **Decide it here, in the record, before writing the step.**

**Unblocked by** item 7 of the old file (`Lifecycle` — the states), item 15 (`Objective` — a party
declares what it is FOR, which is what somebody starting a firm needs a reason from), and now
**item 11**, which builds the sector a firm is born into and the promotion door it leaves by. Spec 34
is half built and **the birth half was PLACED forward three times (13f → 13g → 13h) without
landing** — the worklist row says so itself.

### Steps

- [ ] 12.1 **A firm is born into a small-business cell** (§42 A6, A1): `cells.weight(cell, 'entry', …)` with a cause and a date. It is founded by somebody — a named party with a reason (`Objective`) and a balance sheet the opening equity comes out of — because **no firm appears from nowhere**, which would be a residual with no holder in reverse. What the founder buys is a claim on the new firm, so the entry and the funding are one instruction (Law 5).
- [ ] 12.2 **The reason to found one.** A birth is an OUTCOME and Appendix B forbids a rate, so what produces it is a party's own decision: what the sector earns against what starting costs, read from what this founder can see. It is the same shape as any other decision here — its own outlook, its own money, and a refusal is an answer.
- [ ] 12.3 Household formation is the same door at the other end: nothing moves into cohort 0. Give it a cause. A household is not founded by anybody, so its cause is its own (Households A5), and this is the step that makes `seed.membersPerCohort` deletable.
- [ ] 12.4 **The promotion FIRES** (§42 A6.c). Item 11.10 builds the door and the threshold; this is where a cell actually crosses it in an ordinary run, because it grew — which needs 12.1's entries so the pool is not a draining stock. Settle the representation question above and write the answer in the record.
- [ ] 12.5 `cells.merge` has no caller and is the other half of `A-18`: nothing recombines two cells that have become identical, so the cell count only ever rises. With 2.14's remainder fixed the micro-cells stop being created; merge is what removes the ones already there. **A-2 and A-3 guard `merge` and have never run** — closed as repairs, and this is where they first execute.
- [ ] 12.6 **Delete `seed.membersPerCohort`.** Fifteen million a cohort is a PLACEHOLDER and the plainest one in the seed: how many people there are, stated. Its own `why` says what ends it — *"a population with births and deaths in it rather than a count anybody states"* — and 12.3 is that mechanism. It named worklist 13f until item 9.2b, which is CLOSED and which the worklist's own header says never produced an outcome; this is where it actually dies. The seed states the opening cohorts and the count moves from there (Seed A3: a stock the flows then act on).
- [ ] 12.7 Firm death already works (`estate`). Assert the pair, which is **§42 E4** and is why 11.12 is written red: over a long run, entries and exits are both non-zero and **neither is the accounting identity of the other** — and the same for promotions, which must be neither zero nor every cell.

### Findings this closes

`A-17`, `A-18`'s merge half, and §42 `E4` (with item 11).

### Exit

A firm is born in the small-business pool in an ordinary run, with a founder and a reason; one that
grows is promoted out of it and the boundary between the two sectors is a size a firm crossed; the
population is an outcome in both directions; `cells.merge` and `cells.weight` have callers.

---

## 13. Asset managers: hedge funds (§28), private equity (§29), prime brokerage (§15)

**Why.** Three systems, **0 / 24, 0 / 25 and 0 / 24 MET**. Worklist 13o, open, and correctly blocked:
*"item 8 built `Agreement` and did NOT migrate the seven private books, so `Mandate` does not exist
yet. Building this on today's `fund` party would bake the pool/decider conflation in permanently."*
Item 9 unblocks it.

**Measured**: 13 funds, of which **4 hold anything at all** after four periods — 8 holdings between
them against 1,274 subscriptions settled in one period. Every fund here is a money fund, one
commodity fund, or an index tracker, and the trackers hold nothing.

**§28 C1 is why this is not an optional sector**: a hedge fund *"is the natural home of the
speculative side of every derivative book"*. Item 6 gives each book a maker with its own number; this
gives it a party whose whole reason is to hold the other side.

### Hedge funds (§28)

- **A1–A5**: a named party with investors, a register and accounts; **investor capital is equity** and
  investors hold a redeemable share count; a **manager** is a separate party earning a management fee
  on assets and a **performance fee on gains**, *"and the asymmetry of that second fee is a reason for
  risk-taking"*; a **wide mandate** — long, short, levered, many markets; everything **marked at
  cleared prices**.
- **B1–B5**: it borrows from a **named lender** — *"leverage is a fact about a loan, never a property
  of the fund"*; it levers through **derivatives** (notional over margin) and through **repo**; the
  amount available is the **lender's decision** and it changes. B5: gross, net and equity are **three
  reads** and a single "leverage" number hides which one moved.
- **C1–C4**: positions for reasons — relative value, direction, a liquidity premium it is paid to
  hold; it is the **buyer when others are forced sellers, if it has capacity**; it **shorts**, which
  requires a borrow (item 9.4); real trades at cleared prices.
- **D1–D7, the failure mode, and it must be emergent**: a loss reduces equity → with fixed borrowing
  leverage rises → the lender calls margin → meeting the call requires selling → **which moves
  prices** → the move hits other levered holders and D1 starts again for them. **D4.a: never a
  contagion parameter.** D5: redemptions arrive at the same time for the same reason and are a second
  forced-seller channel; D5.a a **gate or notice period** is a real contractual term with real
  consequences for who gets out.
- **E1–E3**: no leverage without a lender; no position that does not mark; **no fund that cannot
  fail** — *"a vehicle that absorbs losses indefinitely is the buyer of last resort in a different
  costume"*.

### Private equity (§29)

- **A1–A5**: committed capital from named investors; **capital is committed, not paid** — it is
  **called** when a deal needs it, *"and the call is a real payment from the investor's account on a
  date it cannot refuse"*. A2.a: an investor must hold liquidity against calls it did not choose the
  timing of. **A2.b is a FORBID and it is the one to get right**: *"a call bounded by the investor's
  spare cash is not an obligation"* — the investor funds it from its own liquidity ladder, selling if
  it must, **or it defaults on the call**, which is itself an event with consequences. A manager on
  committed capital plus carry; the fund has a **life** and **winds up** (item 14's `Process`);
  acquired firms are held in **named vehicles**.
- **B1–B5, the buyout**: a price agreed with the sellers; **most of it debt raised against the target
  itself** — B2.a the debt is the **target's** liability, *"which is why a failed buyout kills the
  firm and not the fund"*; B2.b **the credit market decides which buyouts occur**, a real constraint
  and not a rate applied to a plan. B5: **sources and uses must balance exactly** and the money must
  come out of named accounts.
- **C1–C5, the hold**: the firm services its debt out of cash flow with less room; the owner
  influences investment, costs and distributions; it can **recapitalise** — *"a real transfer from the
  firm's future to the owner's present"*; it can **fail**; and the holding has **a value that is not a
  market price** — C5.a, *"an unlisted mark is not a cleared price … the honest answer is 'marked, not
  cleared'"*.
- **D1–D5, the exit**: it sells, and **the exit produces the first real price the holding has had**;
  proceeds distributed in cash; **the exit depends on the market being open** — in a bad market the
  hold extends and the distributions do not arrive, which feeds back to A2.a.
- **E1–E3**: no buyout without a lender who agreed; no capital call not paid from a real balance; no
  exit at a price nobody paid.

### Prime brokerage (§15)

- **A1–A4**: a named bank and a named client with a contract that can be ended; the broker **holds the
  client's assets and knows the whole position** — that knowledge is what lets it lend; **A3: the
  client can have more than one broker, and then no broker sees the whole position**, a real and
  material blind spot; the broker earns financing spread, stock-borrow fees and commissions.
- **B1–B5**: the broker lends the difference against the assets; **the client's leverage is a loan
  from a named lender, not a property of the client**; the loan has a rate above the broker's own cost
  of funds; **the broker's balance sheet grows by the loan** and it consumes capital and liquidity;
  the short side is financed too.
- **C1–C5, margin, and the spec calls it the core**: the broker sets a requirement **on the whole
  portfolio from its own view of the risk**, accounting for **offsetting positions** — *"a decision by
  the broker, not a formula the client can rely on"*; remeasured as prices move; a shortfall is a
  **call: real money, now**; meet it or be liquidated, and to meet it the client may have to **sell
  into a market that must clear**. **C3.b: the available line is never floored at zero** — *"a client
  drawn past its line is over the line, and the shortfall is what forces the sale. Flooring it makes
  the whole path unreachable — and lending the shortfall straight back at a penalty, from the same
  broker, makes it unreachable twice."* C4.a: **raising margin into a falling market amplifies the
  fall**, *"the mechanism behind most of what looks like contagion"* — and a stated constant margin
  rate deletes exactly that. C5: no margin that is only a number.
- **D1–D4**: the client fails a call → the broker closes the positions, **selling collateral at market
  prices** → proceeds may be less than the loan and the shortfall hits the broker's capital → the
  liquidation **moves prices, which can margin-call other clients**. D4: the chain must be traceable
  party by party; *"a loss that stops at the fund is a broker that was never really lending"*.
- **E1–E4**: exposure per client, known; concentration means collateral is worth less in liquidation
  than marked; A3's multi-broker case means **each broker underestimates**; **no unlimited exposure**.

### Steps

- [ ] 13.1 `Mandate` (item 9.2) is the spine: a hedge fund is **a mandate with leverage**, a separate account is a mandate whose pool is the client's own balance sheet, an ETF and an MMF are pools with different redemption rules. Build the three sectors on it and nothing else.
- [ ] 13.2 Hedge funds: the party, the manager, the two fees, the wide mandate — `mayWrite` and `leverage` are the two terms that make a mandate a hedge fund's, and both exist (item 9.2a, 9.7). `borrows` comes off the mandate, not off `fundKind` where it is hard-coded `false`. **Not before 13.6**: a mandate that writes contracts before the NAV pass reads them is a fund with equity.
- [ ] 13.3 Prime brokerage: the relationship as an `Agreement`; portfolio margin as the broker's own decision (C1.b) held as a `View`; **no floor on the line** (C3.b).
- [ ] 13.4 The loop D1→D4 must fall out of the parts. Do not write a contagion step. Test: one fund's loss reaches another fund's margin call through prices and named counterparties, and the path is traceable.
- [ ] 13.5 Private equity: committed capital, the call as an obligation the investor cannot bound by its spare cash (A2.b), the buyout with debt on the target, the mark that is not a price, the exit that produces the first cleared price. Needs item 9 of the old file (`Control`), which is built.
- [ ] 13.6 **The pass that re-marks a fund's claim on itself must read the CONTRACT store, and it must land before 13.2 draws a mandate that writes anything.** A fund's equity is zero by construction (Fund Shares A3) because its own claim on itself absorbs whatever its book comes to — and that pass reads the REGISTER, where a contract is not (Derivative X1). A pool with a derivative position would carry a mark its share value had never been told about, which is a fund WITH equity: **measured at 83,247,864 on `etf.us`** the first time funds were let into the contract books. Item 9.7 put `FUND` on `TRADES_CONTRACTS` and made every drawn mandate say `mayWrite: []`, so nothing reaches this today — **by a term of a contract, which 13.2 is about to change.** Open the pass first. Then wire hedge funds into every derivative book as the speculative side (§28 C1), which is the other half of item 6.
- [ ] 13.7 COVERAGE re-marked for all 73 clauses across the three; `check:existence` shows three fewer absent sectors.
- [ ] 13.8 **`E-14`** — `Securities Lending C2, C2.a` are not built, and they are §15 C1's mechanism seen from the other end: both sides of a position marked every period and the difference CALLED in real money between two named parties. `securities-lending:charge` moves the fee and nothing re-marks the collateral, so between the strike and the return the lender's cover erodes silently and C1's haircut is all that stands behind it. Build it once, here, for the portfolio and the stock loan together — two callers of one mechanism, not two mechanisms (Law 4).
- [ ] 13.9 **The management fee, and what a manager COSTS.** `fund.fee.<fund>` is a `placeholder` per pool whose own `why` says *"no manager competes for the mandate, so the number stands where a competition should be"*. Item 9.2a built the `Mandate` and 9.2b measured why that is not enough: **a manager in this world employs nobody**, funds nothing and pays for nothing, so two of them in a book bid each other to the tick — which is a competition between parties with no reason to refuse, not a cleared price (Law 11: the missing mechanism, not the number). So this step is two things in one order: (a) a manager HIRES, in the labour venue, like anything else that needs people, and what it can run is the hours it pays for over the assets a mandate carries — the shape `banks/staff.ts:linesCovered` already has for a dealing desk; (b) then the mandate is COMPETED FOR, one book per pool, each manager bidding a fee with its own cost base as its floor, and the winner's bid is the mandate's `fee`. The placeholder dies in the same change (Law 2) and `fee` moves off `params` onto `MandateTerms`, because at that point it is an OUTCOME. A separate account is a mandate whose pool is the client's own balance sheet, so the same book prices that too (§15).

### Findings this closes

`E-14`. (`B-14` closed at item 9.7: a pool's mandate is what answers whether it may hold contracts, and 13.6 is the pass that must open before 13.2 draws one that does.)

### Exit

A hedge fund takes the speculative side of a derivative book; a margin call forces a sale that moves a
price that calls margin on somebody else; a buyout puts debt on a target and the credit market decides
whether it happens.

---

## 14. Insurers and pensions (§27)

**Why.** **9 of 23 MET, all nine never reached.** Item 9 fixes the five blockers (`A-9`) and gives the
sector a seed (`B-2`). What is left is the half `docs/AUDIT.md` named and never placed:

> *"**Pensions are insurers wearing the same name.** Spec 27 is 'INSURERS AND PENSIONS' and there is
> one party kind, `insurance`. A pension has a SPONSOR, contributions from an employer and its
> members, and a funding ratio that is the sponsor's problem when it falls."*

**The model's largest holder of duration does not exist**, which is why nothing in this world is a
natural buyer of a long bond.

### Steps

- [ ] 14.1 Split the kind: an insurer writes policies against premiums; a pension has a **sponsor**, contributions from an employer **and** its members, and a **funding ratio that is the sponsor's problem when it falls**. Two profiles behind one dispatch table, never a kind branch.
- [ ] 14.2 A policy is an **agreement** (item 9), not an instrument kind. That is `A-9`'s root: *"an insurance policy is modelled as an instrument kind — a tradeable security — which is why the insurers module has no seed, no phase and no participant: nobody knows how to WRITE one, because writing an agreement is not something the kernel does."*
- [ ] 14.3 The insurer buys duration: it is the buyer item 10 and 10b need on the other side of a long issue.
- [ ] 14.4 COVERAGE re-marked; the 14 MISSING clauses built or stated `OUT OF SCOPE` with a reason.

### Findings this closes

The remainder of `A-9` and `B-2` after item 9.

### Exit

A policy is written, a premium is paid, a claim is met; a pension's funding ratio falls and its
sponsor has a problem; somebody in this world wants a thirty-year bond.

---

## 15. Housing, the rest (worklist 13m)

**Why.** **6 of 26 MET.** Item 3 opens the lettings venue, item 7 gives the dwelling a buyer, item 12
of the old file built `Space` (the ground of a place, finite, held and cleared — 113,281,635 hectares
in `us.1`, 16,502 traded, eight holders). What stands on it is not built.

`E-5`: **the state holds the ground of every place in its country and can only SELL in the one it sits
in**, because a seller in another place reads as a cross-border trade in the balance of payments. What
is missing is a party **PRESENT in each place** to sell its ground — a local authority, which is the
same noun a port and a planning consent need.

### Steps

- [ ] 15.0 **`housing/index.ts:askForMortgages` opens `if (cell.representation === 'cell') continue;`** — every household IS a cell, so no household ever publishes a `housing.funding` request, no bank writes a mortgage, `mortgagesOf` is always empty, and `charge` and `foreclose` are dead phases. It was item 7.1. The blocker its own docstring names — a borrower that misses a payment goes on accruing with nowhere for the arrears to sit — is item 19's arrears and item 9's agreement. Remove the guard once both are there.

- [ ] 15.1 The local authority: a party present in each place, objective `itsOffice` (which the land market already dispatches on, so this lands with no change to that file).
- [ ] 15.2 A port with an owner and a berth, and congestion as an outcome of the berth.
- [ ] 15.3 Commercial property: buildings as assets, leases with a term, CRE lending. *"Warehouses exist only as `STORAGE` plant a firm builds for its own stock — nobody builds space to LET, so there is no landlord, no commercial rent, no lease with a term and no CRE lending."*
- [ ] 15.4 A retail firm that sells from somewhere.
- [ ] 15.5 **A-53**: `housing/index.ts:reservation` bids a household's **entire expected income** as rent, as a **single point**, and `letIn` clears on `marginalBid` — so in any region where dwellings are short the rent takes a household's whole livelihood every period. Two things do not follow from B1.a: nothing on the household's side knows rent exists (`spendPerMember` spreads the same expected income over the basket, so it is committed twice), and every other bid in this world is a **schedule** (Clearing A2). Post a curve, and give `households` a rent term.

### Findings this closes

`A-53`, `E-5`.

### Exit

A tenancy is signed and rent is paid; a landlord exists; a household's budget knows what it pays for
its roof.

---

## 16. Cross-border, the rest

**Why.** **6 of 26 MET.** 13i closed with the EXTERNAL ACCOUNTS built and the rest PLACED — *"sourcing
across regions, foreign-currency issuance and the swap line (**M3**, **M6**) stay carried"*, and
nothing has carried them since.

**B-9 is unsettled and must be settled first.** `docs/BUGS.md` claimed *"13j gave this world four
economies"* in its header and *"the foreign countries are stubs — three of the four are a central
bank, a treasury and a bond line"* in its own section 2. Both cannot be current. `A-61` (every central
bank remitting to `treasuries[0]`) was live evidence for the pessimistic reading and is fixed in item
2.13; measure which is true before building on either.

### Steps

- [ ] 16.1 Settle B-9: run the four-country rig and count what each country actually has. Write the answer here.
- [ ] 16.2 Sourcing across regions: a buyer in one place buying from a seller in another, with freight and the balance of payments both seeing it.
- [ ] 16.3 Foreign-currency issuance (**M6**): an issuer raising money it does not print. This is where **A-36** becomes live — `fails: []` on the treasury is an unconditional exception where the kernel grants a conditional one, and Appendix B names *"sovereign in foreign money"* among the things that must not be immortal. `failedWhy` already has `ccy` in scope and discards it on the solvency branch.
- [ ] 16.4 The central-bank swap line (**M3**).
- [ ] 16.6 **Delete `seed.crossHoldingShare`.** Eight per cent of every central bank's reserves is another country's paper, split evenly — a `placeholder` for the portfolio decision (Central Bank F4). It named worklist 13h until item 9.2b, which is CLOSED; what a reserve manager holds abroad is a cross-border question and this is where it is answered. A central bank holds foreign paper because that is what reserves ARE, so it BUYS it, in the market 16.2 and 16.3 open, for a reason of its own — and what it ends up with is then an outcome of what it bought and sold rather than a share the seed stated. It is the last cross-border placeholder.
- [ ] 16.5 The 18 MISSING clauses built or stated `OUT OF SCOPE` with a reason.

### Findings this closes

`A-36`, `B-9`.

### Exit

A firm sources from another country; an issuer raises in a money it does not print and can fail in it;
the swap line exists.

---

## 17. Corporate credit, the rest

**Why.** **7 of 62 MET — 51 MISSING, the largest single gap in the model.** Item 10 gives it the one
thing it has never had: an issued corporate bond. What is behind that is the whole of §7 —
syndication, bookbuilding, facilities, restructuring, the covered bond (**M7**, one of the six lost
things), the index-linked obligation and other schedule shapes (**M2**, placed to 13f and never
built), and the **floating-rate note** — the leveraged loan, which is a security and not a loan.

**Take this item's 51 clauses in the spec's own order** and mark each `MET`, `PARTIAL` with what is
missing, or `OUT OF SCOPE` with a reason. Do not delete a clause to look better.

**A CORRECTION FROM THE OWNER (2026-09-14) that removes work rather than adding it.** A bank loan is
not distributed to investors, and the thing that is:

- **IG loans stay inside the banking system** — one bank, or a club of banks. There is no loan
  distribution mechanism to build, and **C10's syndicate is a SECURITIES underwriting group and
  nothing else**. A club of N banks lending to one borrower is already expressible today with no
  new mechanism: C9's shape is one row per lender per borrower, so a club is N rows.
- **A LEVERAGED LOAN IS NOT A BANK LOAN. It is a SECURITY** — the floating-rate counterparty of the
  fixed-rate bond — and it is issued through **exactly** item 10's path. This is what answers `B4`
  (*"fixed or floating, and floating is the norm in the loan market"*), and it is a second
  instrument kind sharing one issuance mechanism, never a second mechanism (Law 15, Law 4).
- **A bank loan never reaches a party outside the banking system**, and one of the reasons a borrower
  ends up in the public market is that it outgrew its bank: the bank stops increasing the line, and
  the refusal pushes it out. That is `issueBonds`'s second reason and it is an ADDITION, not a
  replacement — **a firm reaches the market for several reasons and the price comparison is a real
  one** (owner, 2026-09-14: *"it reaches the market for multiple reasons, don't delete it"*). Two are
  built — the market is cheaper, and the bank will not lend it enough — and more are real and
  unbuilt: TENOR (a bank lends a year, `banks/index.ts` sets `maturity: drawn.y + 1`, and a bond runs
  five), SIZE beyond any single lender's appetite, and diversifying the funding itself so that one
  lender's retreat is not the end of it. Each is a separate reason with its own read, and none of
  them is a special case of another.

What that rules out is as load-bearing as what it builds: no loan syndication to investors, no
secondary loan market, no "loan fund" holding rows. **It is already true of the code** — `LOAN`
declares `market: none()` and `pricing: 'carriedAtCost'`, so a loan cannot be posted into any book —
and it is true by CONSTRUCTION, which is exactly the kind of FORBID that breaks silently the day
somebody gives the kind a market. 17.8 guards it.

### Steps

- [ ] 17.0 **The leveraged loan, which is a floating-rate note** (`B4`, `A2.a`). A second instrument kind in `corporate-bond`, reusing `issueBonds`/`place`/`openLine` unchanged: same reason to come, same walk-away, same covenants off the same published accounts, same market, same tap. All that differs is the KIND'S OWN PROFILE, which is where Law 15 puts it — `cashFlows` reads the fixing instead of a locked coupon. The fixing is `index.benchmark`, which is what the overnight book PRINTED (the same read `irs/contract.ts:floatingRate` uses), so it is a cleared rate and not a posted benchmark (Appendix B). **The one thing to get right is the spread at issue**: the fixed line strikes its coupon at the keenest published requirement, so the floater strikes its SPREAD at that requirement less the current fixing — a term the issuer promises, struck once at issuance, never a spread table and never a spread on a mid. The price still clears in a book (Law 3).

- [ ] 17.1 Syndication and bookbuilding **of a SECURITIES issue** — a bond or 17.0's note, never a loan. A bank arranging an issue it does not hold all of. **Item 10 brought the paper and left the ARRANGER out**: C1's named underwriter, C6's fee out of the proceeds, C7's risk between commitment and placement, C10's syndicate and C11's basis. Item 10 issues DIRECTLY into the kernel's book, which is C2–C5 in full and C6 without its fee — so this step adds a party to a path that exists rather than building the path. **The closest working precedent is in the sovereign book**: `banks/dealing.ts:primaryBid` reads `dealershipShare` off `auction.announced` and bids for that share whether it wants to or not, wearing what it gets — which is structurally what a C11.b backstop is, and C10's "each member's share sits against its own limit" already has its limit read (`lines.ts:roomFor`). It is also the home of **step 10.1's "worth the fixed cost of an issue"**: that cost IS the underwriter's fee, and item 10 deliberately did not invent a `corporateBond.issuanceCost` to stand in for a party this item creates (Law 2).
- [ ] 17.1a A2.b: **the target a management is managing towards** — a leverage, a coverage or a rating it wants, approached at its own pace. Item 10 made issuing a DECISION (a firm compares what its bank quoted against what holders require and takes the cheaper), which is A2.c's "never assigned"; what it compares is price against price and not price against a plan. Marked `Corporate Credit A2`/`A2.c` PARTIAL for exactly this.
- [ ] 17.1c **The other reasons a firm comes to market**, named in this item's preamble and built nowhere (owner, 2026-09-14: *"it reaches the market for multiple reasons"*). `issueBonds` takes two — the market is cheaper, the bank will not lend enough — and each of these is a separate read with its own answer, not a special case of either: **TENOR** (a bank writes a year, `banks/index.ts` sets `maturity: drawn.y + 1`; a bond runs five, and a firm funding a five-year asset at one year is rolling risk it need not take); **SIZE** beyond any single lender's appetite, which is `lines.ts:roomFor` read across the banks rather than the keenest one's `most`; and **DIVERSIFYING THE FUNDING**, so one lender's retreat is not the end of it — a reason about the NUMBER of lenders rather than the price any of them quotes. Add them as reasons, not as a score: `issueBonds` must keep taking reasons rather than collapsing them into one number nobody can read back.
- [ ] 17.1b A3.a: **the service is interest PLUS SCHEDULED PRINCIPAL.** `testCovenants` covers a line's coupon against published earnings and nothing else, so an amortising line looks as serviceable as a bullet. One read of the kernel's own `due` schedule, at the one place coverage is computed (Law 4).
- [ ] 17.2 Facilities: a committed line, with a commitment fee on undrawn headroom (the same noun item 10b.6 needs — build it once). **The DRAW half is already built and C9 is marked PARTIAL for it**: `banks/index.ts:lineOf`/`draw` keeps one row per lender per borrower and taps it at the margin it was struck at. What this step adds is the word COMMITTED — a stated limit the bank is obliged to honour, undrawn headroom, the fee on it, and the capital an undrawn line consumes — so a borrower stops being re-underwritten at every draw. Two things found while checking: a SECURED request opens a new row instead of drawing (`write` is called with `onTheLine = security.length === 0`), and `runRequests` computes a fresh quote that `draw` then correctly ignores. **A CLUB OF BANKS NEEDS NOTHING BUILT**: C9's one row per lender per borrower already means a borrower with three lenders has three rows, which is what "a group of banks" IS. What is missing from a club is only that nobody arranges it — and an arranger is 17.1's party, not a second kind of loan.
- [ ] 17.3 Restructuring: placed 13f → 13h, never built. A borrower and its lenders agreeing new terms is an `Agreement` transition, not a new instrument.
- [ ] 17.4 The covered bond (**M7**): placed 13e → 13f, never built. One of the six.
- [ ] 17.5 Factoring and receivable pledges: placed 13e → 13f, never built. Two more of the six, and they sit on item 11's small firms, which is why this is after 11.
- [ ] 17.6 Senior notes as repo collateral: the last of the six. `money-market/collateral.ts` already has the haircut machinery.
- [ ] 17.7 The index-linked obligation and the other schedule shapes (**M2**).
- [ ] 17.8 **Guard the FORBID: no bank loan held outside the banking system.** It holds today by CONSTRUCTION — `LOAN` names no market and is carried at cost, so the only transfer path is securitisation's sale into a `VEHICLE`, and what investors buy there is the `TRANCHE`, a security. A FORBID that holds is as valuable as a mechanism that works and this one breaks silently: give the kind a market one day and nothing anywhere would complain. An audit family, reported with owner and size like every other invariant, never thrown. **The design question to settle first**: "the banking system" is a set of party kinds (`bank`, `vehicle`, and `estate` while a failed lender winds up), and enumerating it in the family is the kind branch Law 15 forbids in a mechanism — so it belongs on the PARTY KIND as declared data, which is a structural decision and carries an `ARCHITECTURE.md` change in the same commit.

### Exit

§7 is 62 of 62 answered — MET, PARTIAL with what is missing named, or OUT OF SCOPE with a reason —
and all six of the lost things from 0.3 have a home and a state.

---

## 18. Commodities spot and futures

**Why.** **Commodities Spot 3 / 24, Commodity Futures 5 / 20.** 35 missing clauses across two systems
that 13c closed. The futures half needs item 6 (a book that can open); the spot half is independent.

### Steps

- [ ] 18.1 Read §20 and §21 clause by clause against `mechanisms/commodities/` and `mechanisms/commodity-futures/` and write down which of the 35 are genuinely absent and which are MET-and-unmarked. **This is a measurement of the claim, not of the world** — do it with the item 1 tool, not by reading.
- [ ] 18.2 Build what is absent, in the spec's order.
- [ ] 18.3 `A-64`'s storage rent is closed (item 11 of the old file) and the leases store is gone; confirm nothing reintroduced it.

### Exit

§20 and §21 are answered clause by clause.

---

## 19. The polity (§47) — worklist 14

**Why.** **0 of 32 MET.** The plan below is item 14's own, carried unchanged from `docs/AUDIT.md`
Part IV, because it was written against the spec and nothing since has changed it. It carries three
findings: **D-1** (arrears), **D-2** (the central bank as a marginal price-setter), and **E-7** (a
negative policy rate is real and this world cannot express one).

### 19.1 Kernel: owners and the mandate door

Every `policy` parameter already declares an `owner` (`parliament | centralBank | standardSetter |
constitution`). This makes the owner **load-bearing**: a policy owned by `parliament` can be written
only through `params.setByMandate(values, effective: Period)` on the `MechanismContext` of the module
that declares itself the polity (assembly grants the door to exactly one module: D5, C3.a); every
other write path throws `Forbidden`; the seed's standing mandate is the initial value with
`setBy: 'seed.standingMandate'` recorded (XI-17); the observer prints the owner and the setter beside
every policy value (D5).

The central bank's **rate** stays the central bank's (D4, Central Bank A4); its **target** and mandate
text are parliament-owned (D4, Central Bank A3): `centralBank.target.*` moves to owner `parliament`.

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
  such coalition exists within a stated distance continues the standing mandate and is reported (C2.a).
- **The mandate** (C3, C4): the seat-weighted position of the coalition's platforms per policy; a
  journalled `polity.mandate` with date, parties, seats and what changed (C4, E4 — an event that
  causes nothing itself); written through the door with `effective = election + polity.mandateLag`.
  C3.b is an audit contribution: every parliament-owned policy's value equals the standing mandate's,
  every period, exactly.
- **What it controls** (D): tax rates on named bases, transfer rates, the treasury's buffer, the
  outlay programme's size and composition, regulatory ratios and floors, the central bank's target.
  D3.a: no price, quantity or outcome is a parliament-owned policy — assembly throws if a platform
  names a parameter whose kind is not `policy` or whose owner is not `parliament`.
- **Consequences** (E): E1 the treasury programme reads the register; E2 the chain runs through the
  mechanisms; E3 measured at item 23; F4 the **approval rating** is the read "how would the cells vote
  today", computed by the observer with a lag and read by nothing.

**Parameters.** `polity.seats`, `polity.termPeriods`, `polity.allotmentRule`,
`polity.coalitionMaxDistance`, `polity.mandateLag` (policy, owner `constitution`); `platforms` (data).
No `turnout`, no `swing`, no `loyalty`, no `bloc`, no `approval` input.

**Audit contributions.** `names`/`flows`: C3.b — register values equal the standing mandate (a check
of one writer). `units`: votes cast + abstentions = Σ weights of the electorate, exactly (F5).
**Write this one against `A-14`**: a votes-plus-abstentions identity built from one loop with two
accumulators would be the third of its kind in this engine. The two records must be independent — the
ballots the vote produced, against the electorate the parties store holds.

### Files

```
packages/engine/src/registry/params.ts (19.1), registry/platforms.ts
packages/engine/src/world/context.ts (the door granted to one module), world/assemble.ts
packages/engine/src/mechanisms/polity/{index.ts,vote.ts,seats.ts,coalition.ts,mandate.ts,approval.ts}
packages/engine/test/{param-owners,platforms,vote,abstention,seats,coalition,hung,mandate,
                      mandate-writer,spread-changes-seats,election-chain}.test.ts
```

### Steps

- [ ] 19.1 Kernel: policy owners load-bearing; parliament-owned policies writable only by the mandate door granted to one module; the seed's standing mandate recorded as the setter; owner and setter printed beside every value; the central bank's target moved to the parliament; tests (D4, D5, C3.a)
- [ ] 19.2 The constitution's primitives: seats, term on the calendar, allotment rule from a dispatch table, coalition distance, mandate lag; tests (A1, A4, C1)
- [ ] 19.3 Platforms as data rows covering every parliament-owned policy and nothing else, differing; a party holds no account; assembly validation; tests (A2, A2.a, A2.b, D3.a)
- [ ] 19.4 The vote: each cell applies each platform to its own state at its own outlook through a view with no aggregate reads; votes weight for the best; abstains when indifferent; turnout as a read; tests (A3, B1, B1.a, B2, B2.a, B2.b, B3)
- [ ] 19.5 Seats by the rule; the government by the coalition rule; a hung parliament continues the standing mandate and is reported; tests (C1, C2, C2.a)
- [ ] 19.6 The mandate as the seat-weighted coalition platform, journalled with subjects, written through the door at the lag; C3.b contribution; tests (C3, C3.b, C4, E4)
- [ ] 19.7 What it controls: fiscal rates, transfers, buffer, outlay programme, regulatory ratios, the target; never the rate, a price, a quantity or an outcome; tests (D1–D4, D3.a)
- [ ] 19.8 **Arrears (`D-1`)**: a levy that fails becomes a claim the treasury holds on the payer — an instrument with two named sides, carried until paid, written off or ranked in an estate at the place the law states; the receipt is short by it in the accounts and not only in the journal. **Build it as one door for all four cases**: the unpaid tax, the unpaid wage and severance (`A-41`), the unpaid estate transfer (`A-20`), and the horizon `stillOwed` reads (`A-39`); tests (Money E1, D3; XI-8; Polity D1, D3). Note: item 8 of the old file built the store and wired all four — what remains here is `breached`, which has **no writer**: a levy in arrears stays `performing` because nothing has decided when an arrear becomes a breach, and that is fiscal policy with an owner.
- [ ] 19.9 **`D-2`**: the open-market desk closes a gap towards a 25%-of-line target by posting a **MARKET order** — a quantity with no level — so in every sovereign session where it has a gap it bids at the top of the book for a quarter of the line. An order with no level is a price-taker of a price the mechanism has not yet produced (Clearing A4), and a big enough one is the marginal order that sets it. The quantity limit is real policy (Central Bank C1) so this is not Appendix B's buyer of last resort — but it is more aggressive than any real open-market operation. **It posts a SCHEDULE**: the level at which its own reason stops. Test: the sovereign session's clearing price does not move when the desk's gap is doubled at an unchanged book.
- [ ] 19.10 **`E-7`**: a policy rate of zero puts the corridor floor at minus a tenth of a point and the solver refuses a negative price (`Impossible [Law 6] a price cannot be negative: -0.001`). **That is right for the price of a THING and wrong for the price of TIME**, which the Bank of Japan and the ECB both ran below zero for years. A negative policy rate is real and this world cannot express one. Fix it here, where the rate has an owner, rather than rounding the yen up.
- [ ] 19.11 Consequences through the mechanisms only: the treasury programme reads the new numbers; a multi-year scenario shows the deficit changing through named outlays and receipts after an election; tests (E1–E3)
- [ ] 19.12 Approval rating as a lagged observer read that nothing reads; tests (F4)
- [ ] 19.13 B4: two seeds with equal weighted-mean income and different dispersion give different seat counts; F5: seats and mandate computed from members, with the two records independent; tests
- [ ] 19.14 Observer: parliament, platforms, votes by cohort and region, the mandate with owner beside every policy; a multi-year run with two elections green; determinism; coverage re-marked; record entry
- [ ] 19.15 Worklist row 14 → done

### Findings this closes

`D-1` (the `breached` writer), `D-2`, `E-7`.

### Exit

No parliament-owned policy can be set by anything but a mandate; a cell votes from what it experienced
and can abstain; a hung parliament is a reported outcome; a change of government reaches the deficit
only through the treasury's programme.

### Guard

Polity B1.a, B2.a, C3.a, D3.a, F1–F4; Central Bank A4; XI-17 (*"no policy set directly"*).

---

## 20. Periodicity: the fee and the assessment (worklist 13k)

**Why.** Item 10 of the old file fixed the dividend — a board declares on its own fiscal quarters with
an ex date, a record date and a payable date, and **dividend payout legs fell from 8,538 of 61,788
instructions to 308 of 53,266**. Three of `C-2`'s four are still weekly.

- **A rating fee is charged every week.** 16,527 `pays X for its rating` instructions in period 5;
  `ratings.assess` is `cycle: 'anchor'` and calls `collectFees` every period. An issuer pays an
  **issue fee once** and a **surveillance fee annually**. Weekly billing makes the assessor's income a
  flow of the issuer's equity rather than a price for a service, which is the conflict Ratings A5
  exists to keep.
- **Tax is levied every week.** 7,778 `tax due from X` instructions in period 5. Payroll withholding
  is weekly and right; **corporation tax is assessed on a fiscal period** and VAT quarterly. Levying
  everything weekly removes the working-capital consequence of a tax bill, which is the thing a
  treasury and a firm both plan around.
- **A buyback is decided weekly** out of this week's spare cash. A buyback is an announced
  **PROGRAMME** executed over time, and announcing it is the event. Its kind is already in
  `register/corporate.ts` and item 14 of the old file built `Process`; this is one `beginProcess`
  away.

`reporting/fiscal.ts` and `core/rate.ts`'s `Periodicity` union are built. **This item is after 19**
because who sets a fiscal period is fiscal policy with an owner.

### Steps

- [ ] 20.1 The rating fee: an issue fee once, a surveillance fee annually.
- [ ] 20.2 The tax assessment: withheld weekly on the wage, **ASSESSED** on a fiscal period.
- [ ] 20.3 The buyback as a `Process`.

### Findings this closes

`C-2`.

### Exit

Nothing happens weekly that does not happen weekly in the world.

---

## 21. The local repairs

**Why.** Three findings with no type-level cause: plain bugs and a stale read. The item's own rule is
**never a session's work; take each when its file is already open**. Fifteen of the original eighteen
are closed.

### Steps

- [ ] 21.1 **A-24**. `households/decide` publishes its orders into a journal event as `Record<string, unknown>`; `marketsIn`/`ordersFrom` re-validate from scratch (`if (price !== 'market' && typeof price !== 'number') continue;`) and **silently drop** what they cannot parse. A cell whose plan wrote a row this pair of predicates rejects simply does not trade that period, with no throw, no violation and no event — a decision disappearing between the party that took it and the book it was for, which is the one thing the round trip exists to prevent. Apply item 3 of the old file's answer — a typed read that **throws** on a record its own writer malformed. `ordersFrom` also drops `qty <= 0` before `asQty` can complain, so the file's own comment about throwing is only true for positive non-integers.
- [x] 21.2 **A-43** — closed in item 9.6, and confirmed there: `labour/matching.ts:supply` is gone and nothing calls it.
- [ ] 21.3 **A-53** — closed in item 15.5; confirm the rent bid is a schedule and `households` has a rent term.
- [ ] 21.4 **E-6**, found while building item 17 of the old file: **an acquirer's consideration in a bank resolution is a missing mechanism.** `money-market/resolution.ts` ranks bidders by what the book is worth to each — faithfully, and `A-59` is closed by publishing `worthToIt`, which is what ranks them. What an acquirer should be **PAID** for taking the book on does not exist: the guarantee pays exactly `v.hole`, so the acquirer ends whole in balance-sheet terms and earns nothing at all for taking on the book, **while the auction ranked bidders precisely by how much they wanted for doing so**. Build the consideration.
- [ ] 21.5 **E-3**, found while building item 10 of the old file: an estate pays RENT for the space its inventory sits in while it winds up, and its own `flows` family reports every non-`corporateAction` payment. Decide which is wrong and fix that one.
- [ ] 21.6 **E-2**, found while building item 8 of the old file: an estate left `winding` because it could not hand over **is still a household cell to the eleven `ofKind(HOUSEHOLD)` readers**, which all ask `status.alive` — so a cell of dead people goes on consuming and looking for work. Item 14 of the old file built `Process` and this is not built on it yet.
- [ ] 21.7 **E-4**, found while building item 11 of the old file: a capacity line still produces into WIP and destroys the unsold part. The number is right and the **accounts line** is not — an inventory write-off where it should be operating leverage on fixed cost. The fix is **produce-to-order**: a capacity line posting availability into the session and making only what clears. It could not be done in item 11 because `firms.produce` anchors after `labour.pay` and therefore **before** `markets`, so a line cannot know its demand when it produces — and reordering for one kind of line would be a kind branch. It needs `Process`, which exists.
- [ ] 21.8 **A stale placeholder of the old file's own making.** `estate/index.ts:598` still declares `standsInFor: { noun: 'Process' }` — but old item 14 BUILT `Process`, deleted `Winding`, and pointed the settle loop at `ctx.processes.running('estate')`. The store is migrated and the declaration was not removed with it. Law 16: a stale declaration is a defect, and this one makes `registry/nouns.ts`'s count of homeless nouns wrong — which is *"the honest measure of how much ontology is missing"*. Delete it and recount.
- [ ] 21.9 **A-69**'s dead reads: `struckRate` and `findSession` (`money-market/session.ts`), `cdsBookOrders` (`cds/participants.ts`). Each carries a docstring asserting something is measured or dispatched and is called by nobody. Call it or delete it.

### Findings this closes

`A-24`, `E-2`, `E-3`, `E-4`, `E-6`, and `A-69`'s remainder.

---

## 22. The recipe (worklist 15)

Unchanged, and last before measurement, deliberately. Goods A2.

---

## 23. Measure — Part XII in full (worklist 16)

**Why.** Part XII is the measurement pass, and it comes **only after** everything above (Law 11: do
not measure mid-build). It carries `C-1`'s 82 red tests, which are not bugs — they are **tests that
named the world they were written against.**

| cause | ≈ red | what it is |
|---|---|---|
| **the rig has no firm in most lines** | 20 | `RIG_PER_LINE` is 0, so twelve firms over sixty-two lines leave most empty and any test asking the draw for a mill is told "it drew 0". Measured: setting it to ONE takes the suite from 79 red to **139** — a rig with every line in it is a different small world, and a dozen files' assertions are written against the one it makes today. Those tests are not wrong, they are **SPECIFIC**. |
| **the foreign countries are stubs** | 6 | see `B-9`; settled at item 16.1 |
| **a bank that is insolvent is never resolved** | 9 | `13b-12`: a bank published capital of −31,237,415,456 and went on making a market for twenty-three periods. Both triggers must exist and the resolution must name which fired (Banks Capital C1.a). Whether the solvency trigger is not reading the published position, or is reading it and the resolution is not being run, is **still unmeasured** — and it has its own diagnosis, **before** this item |
| ~~**an ETF cannot create**~~ | 7 | `12d-8`; **answered at item 9.8** — the desk borrows what it must deliver |
| **two banks, and everything that needs a third** | 4 | `research` wants coverage to VARY, `deposits` wants a class to split rather than cross, `dealing` wants a market to fail when the desks step back. With three banks and one listed line a count that should be an outcome has one value |
| **singletons** | 9 | `omo` remittance and run-off (`A-61`, `A-62`); `raise`; `ratings` ageing; `equity-anchor`; `indices`; `tick`; `environment`'s crop; `treasury`'s receipts (`A-46`); `world`'s year-long chain. Each needs reading on its own |

**And the eight 13j cost.** Three measurements of one suite: 74 red before 13j; **303** with the rig
opening all four countries; **82** with the rig opening one and the currency tests four. The 303 is a
finding and not a bug — a dozen firms and three banks over four countries gives each a country with no
banking system its own depositors could fund, and `Seed D1` refuses exactly that ninety-four times —
so **a world's count of countries is a RESOLUTION like its count of banks.**

### Steps

- [ ] 23.0 **Run `npm run check`** — the first suite run since the plan opened. It was item 2's last step (`2.22`) and it is here because the owner moved it here: a suite run mid-build measures a half-built world and reports the half that is missing (Law 11). Read what it says and write every finding into this file under the item that should fix it before changing anything. Nothing is chased.

- [ ] 23.1 Resize the scale model as **one bounded change** and re-derive what every affected test asserts. A test never names a party (`packages/engine/test/rig.ts`): it asks the draw for a mill, a dealer, a listed line.
- [ ] 23.2 Diagnose "a bank that is insolvent is never resolved" **before** this item, on its own. It is the one row of `C-1` that is a live defect rather than a specific test.
- [ ] 23.3 Part XII's measurements, in full, with the level carried from eight measurements of it (`12-15`, `12d-1`, `12d-6`, `12d-7`, `12d-9`, `12b.1-2`, `12a-2`) and the representation (`12d-16`, `12c.1-1`).

### Findings this closes

`C-1`.

---

## 24. The app and the APK (worklist 17)

Unchanged. The reserved decision (§45 A4, inspector vs participant surface) taken with the owner; the
product shipped to the target device. GitHub Pages on `main`; APK via `android.yml` (Capacitor);
target Pixel 11 Pro XL.

---

## Part 3 — The findings, carried

Every open finding from `docs/AUDIT.md`, verbatim, under the item that closes it. A finding
here is **evidence for the change, not a task of its own** — do not chase one. The severity
letter in each heading is the original read's: **A** structural, **B** live defect, **C** local.

> **Item numbers INSIDE these verbatim bodies are `docs/AUDIT.md`'s**, not this file's. The old
> numbering maps: old 1 Reach → items **1**, **3**, **4**, **6**, **7**, **10** here; old 2 → **5**;
> old 8 `Agreement` → **9**; old 10 `CorporateAction` → **20**; old 16 `Measure<D>` → **2**;
> old 17 the local repairs → **21**; old 18 the sectors that were waiting → **10b**, **11**, **13**,
> **14**, **16**, **17**, **19**. Worklist rows keep their own ids (13k → **20**, 13m → **15**,
> 13n → **12**, 13o → **13**, 14 → **19**, 15 → **22**, 16 → **23**, 17 → **24**).

---

#### B-13 — the three things a firm sector does, and this one does none of them (A)

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
  input bids are inflated by A-34, but neither explains two orders of magnitude. **Item 10 removed
  one cause and it was not a small one**: there was ONE credit channel, so a firm whose bank would
  not lend it enough had nowhere else to go and simply stayed short. It can now bring paper;
- **nobody dies** — `fails: []` on households and the treasury (A-36), and `failedWhy`'s solvency
  branch reads an equity account that A-39 is inflating every period.

The diagnosis of the first and third is the next piece of work, and it is not this read's.

---

### For item 12. Firm birth

#### A-17 — three of XI-15's five weight events never fire, and nobody in this world is ever born (A)

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

---

### For item 15. Housing, the rest

#### A-53 — a household bids its entire income as rent, and its spending plan does not know rent exists (B)

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

---

### For item 16. Cross-border, the rest

#### A-36 — the treasury's immortality is unconditional where the kernel says it is conditional (C)

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

#### B-9 — `docs/BUGS.md` contradicts its own header about the four countries (C)

Its header: _"Measured on the whole suite: 82 red of 662 **after item 13j gave this world four
economies**."_ Its section 2, in the same file: _"**The foreign countries are stubs.** Three of the
four countries are a central bank, a treasury and a bond line. There is no foreign economy … →
**13i**, which closed with the external accounts built and the foreign economies not."_

Both cannot be current. 13j is marked done and its row claims each of the four gets the same
construction from the same draw; section 2 says three of them are still stubs and points at the item
before it. One of the two is stale and the file does not say which. A-61 is a live piece of evidence
for the pessimistic reading: every central bank in the world remits its seigniorage to
`treasuries[0]`.

---

### For item 19. The polity

#### D-1 — a levy that fails is recorded and then forgotten: no arrears (`12d-5`) (A)

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

#### D-2 — the central bank is a marginal price-setter in the sovereign book (`13b.1-10`) (B)

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


### Item 14 — the polity: the plan, carried unchanged

### Design

---

### For item 20. Periodicity

#### C-2 — four things this world does every week that the world does not (carried from `SWEEP.md`; worklist **13k**)

> **The dividend is fixed (item 10); the other three are not, and they stay in 13k.** A board now
> declares on its own fiscal quarters with a record date and a payable date after it, and dividend
> payout legs fall from **8,538 of 61,788 instructions to 308 of 53,266** over forty periods. The
> **rating fee**, the **tax assessment** and the **buyback** are still weekly. The first two are
> genuinely periodicity and are 13k's remaining subject; the buyback is a PROGRAMME and needs item
> 14's `Process` — its kind is already in `register/corporate.ts` waiting for one.


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

---

### For item 21. The local repairs

#### A-24 — the household's own plan round-trips through `unknown` and drops what it cannot parse (C)

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

---

### For item 23. Measure

#### C-1 — 82 red of 662, by cause (carried from `BUGS.md`)

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

---

#### C-7 — the confidence question, answered and closed (carried from `VERIFY.md`)

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

## Part 4 — The index

### Every open finding, and the item that closes it

| finding | item | |
|---|---|---|
| **A-9** (A) | ~~9~~, 14 | the whole insurer sector cannot open a single policy — **the five blockers closed at 9.3**; what item 14 owes it is the sector itself |
| **A-17** (A) | 11, 12 | three of XI-15's five weight events never fire, and nobody is ever born. **11** builds the promotion door (§42 A6.c); **12** is the birth, and is what makes the promotion fire because a firm GREW rather than because the seed put it there |
| **A-18** (A) | ~~2~~, 12 | the fraction of a person is discarded every period — **the fraction and the whole-cell crossing closed at 2a.1**; `cells.merge`, which removes the micro-cells already there, is item 12 |
| **A-24** (C) | 21 | the household's plan round-trips through `unknown` and drops what it cannot parse |
| **A-36** (C) | 16 | the treasury's immortality is unconditional where the kernel says conditional |

| **A-53** (B) | 15 | a household bids its entire income as rent |
| **A-69** (C) | ~~1~~, ~~6~~, 21 | nine exported entry points that nothing calls — **closed at item 1** (the reach tally names them) and **two of them wired at 6.5**; `wantsToBorrow` was another and 9.4 DELETED it. What is left is whichever of the nine item 21 finds still dead |
| **B-2** (A) | ~~9~~, 14 | the insurance sector has no seed, no phase and no participant — **the seed, the phase and the way in closed at 9.5 and 9.3**; what item 14 owes it is pensions |
| **B-9** (C) | 16 | the four countries: header and section 2 contradict each other |
| **B-13** (A) | ~~10~~, 23 | the three things a firm sector does, and this one does none — **"almost nobody borrows" lost one of its causes at item 10**: a firm now has a second credit channel. The other three legs are each placed, and what is left of B-13 is a MEASUREMENT of the finished world, which is item 23 (Law 11) |

| **C-1** (—) | 23 | 82 red, by cause |
| **C-2** (—) | 20 | four things this world does every week that the world does not |
| **C-4** (—) | **0.2, 0.3, and items 10b–19** | the sectors that are not there — **promoted from a finding to the work itself**; its six rows are the absent sectors in Part 0 |
| **D-1** (A) | 19 | a levy that fails is recorded and then forgotten |
| **D-2** (B) | 19 | the central bank is a marginal price-setter in the sovereign book |
| **E-2** (B) | 21 | a `winding` estate is still a household cell to eleven readers |
| **E-3** (C) | 21 | an estate pays rent while it winds up and its own family reports it |
| **E-4** (B) | 21 | a capacity line produces into WIP and destroys the unsold part |
| **E-5** (B) | 15 | the state can only sell ground in the place it sits in |
| **E-6** (B) | 21 | an acquirer's consideration in a bank resolution is a missing mechanism |
| **E-7** (B) | 19 | a negative policy rate is real and this world cannot express one |
| **E-12** (B) | 21 | what a household requires of a claim reaches the fund comparison and not the paper bid, so a change in the deposit board does not move what it will pay for a bill — half of D5.a's substitution. **Found closing `A-44` at stage 2c** |
| **E-11** (B) | 21 | `Outcome.price` and `Print.price` are `PerPiece` and some books clear a RATE — the subordinated raise, the money market, the IRS, the CDS, the cross-currency basis, and now the BORROW book, whose level is a fee per period on value and not money per piece (item 9.4). `E-10` closed the CONTRACT layer (the level a contract carries is tagged by its kind's `quotedAs`) and every reader of a rate-quoted print now says `asRatio` at its own door, so the crossing is named everywhere it happens — what is left is that the STORE still cannot say it. **Item 10d takes one of the five books away rather than teaching the store a trick**: the subordinated raise clears a rate only because it is a private venue, and a bond book clears a price per unit of par. The solver genuinely need not care (a schedule is size against a level either way); the print store is where the claim would live. **Re-positioned from item 6, which did not close it** |
| **E-13** (C) | 21 | `households/portfolio.ts:ownUncertainty` implements §46 B3's income channel and has NO CALLER — a saver's bid is built from its PRICE outlook's confidence and its income uncertainty reaches nothing. Either wire it or delete it; a mechanism nobody reads is not one. **Found closing `A-30` at item 5** |
| **E-17** (B) | 17.6 | **commercial paper is not repo collateral** (`Short-Term Debt D3`), and D3 says being collateral is *"a large part of why anyone holds it"*. `money-market/collateral.ts` already has the haircut machinery and what is missing is that it accepts this kind — 17.6 is the same shape for senior notes, so it is built once for both. **Found closing item 10b** |
| **E-18** (C) | 21 | `funds/index.ts:1296` cites `Clearing C1.b`, which does not exist in the spec. It is a PROSE citation so `check:spec` cannot see it — the tool reads `@spec` tags only — which makes it the kind of stale comment Law 16 calls a defect and nothing guards. Two questions for 21: the right clause for that sentence, and whether the citation check should read prose citations too. **Found closing item 10b**, where the same wrong id was written into a new `@spec` tag and the tool DID catch it |
| **E-15** (A) | 10d | **the THIRD issuance mechanism.** `banks/subordinated.ts` is a private copy of the kernel's issuance machinery: its own venue (`raiseVenue`), its own `clear()` call, a book that clears a RATE, and `subId(bank, n)` advancing per FILL — so a bank raising from three investors holds **three instruments carrying one promise** (Law 4, Law 9), and `market: none()` on each, which is why a bank's capital layer has no price. **Found answering the owner's question of 2026-09-14**; the item is a deletion |
| **E-14** (B) | 13 | **Securities Lending C2, C2.a are not built.** `charge` moves the FEE every period and nothing re-marks the collateral: when the borrowed line rises the borrower owes more collateral and no leg posts it, so the lender's cover erodes silently between the strike and the return and C1's haircut is the only thing standing behind it. The same mechanism is §15 C1's — a broker marking a whole portfolio and calling the difference — which is why it lands with prime brokerage rather than here. **Found closing `A-67` at item 9.4** |

## Appendix — the lessons this file exists to keep

1. **A `done` row and a `MET` mark are claims, and a claim nobody can falsify goes stale silently.**
   Six things were placed into items that closed without them, and nothing anywhere said so. Item 1
   is the check; Part 0 is the number it checks against.
2. **"Do not chase a finding" is right for a defect and wrong for an absence.** `C-4` was six missing
   sectors filed as evidence. A missing sector is an item.
3. **A finding leaves this file only by its item closing** — and the item that receives a placement
   must carry it in its own steps, not in prose. Every placement in Part 2 is a checkbox for that
   reason.
4. **Measure at the end of a module, never mid-item** (Law 11), and **run the whole suite, never the
   files you think are affected**. Item 17 of the old file cost this world its entire merchant fleet
   for four commits because a module declared beside the wrong neighbour dragged the assembly sort,
   and every per-item measurement missed it.
5. **A refactor that cannot change behaviour cannot reduce a failure count**, and saying "the same 79"
   is not a result. The 79 red are Law-11 incomplete-model checks waiting on mechanisms in items
   3, 6, 7, 9, 10b, 11, 13 and 14 — not on types.
