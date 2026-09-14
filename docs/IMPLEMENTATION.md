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
| Equity | 25 | 3 | 9 | 0 | 37 |
| Money Market | 25 | 3 | 0 | 0 | 28 |
| Spot FX | 26 | 1 | 0 | 0 | 27 |
| Fund Shares | 23 | 3 | 0 | 0 | 26 |
| Securities Lending | 14 | 0 | 7 | **14** | 21 |
| Prime Brokerage | 16 | 3 | 5 | 0 | 24 |
| Derivative Layer | 32 | 0 | 0 | **17** | 32 |
| CDS | 17 | 0 | 8 | **17** | 25 |
| IRS | 14 | 1 | 5 | **13** | 20 |
| FX Forwards | 13 | 0 | 8 | 0 | 21 |
| Commodity Futures | 5 | 0 | 15 | **5** | 20 |
| **Commodities Spot** | **3** | 1 | **20** | 0 | 24 |
| Indices | 21 | 1 | 0 | **1** | 22 |
| Banks Lending | 21 | 5 | 6 | 0 | 32 |
| Banks Funding | 28 | 4 | 0 | 0 | 32 |
| Banks Capital | 20 | 2 | 1 | 0 | 23 |
| Dealer Desks | 26 | 1 | 0 | **2** | 27 |
| Insurers | 9 | 0 | 14 | **9** | 23 |
| Hedge Funds | 12 | 3 | 9 | 0 | 24 |
| Private Equity | 7 | 1 | 17 | 0 | 25 |
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
- **10e (the asset management sector) before 13**: 13 is the LEVERAGED, speculative slice and its
  investors are the institutions 10e makes into allocators — a hedge fund funded by nobody is a
  sector that cannot open. The bulk of the buy side is long-only, and that is 10e.
- **13 (asset managers) before 11 and 12**: three items have now built things to sell and none has
  built anybody to buy them. A sector of supply added in front of the demand item is a sector whose
  books say `noDemand` — which is an honest outcome and a wasted one, because nothing about the
  mechanism gets tested by a book nobody comes to.
- **10 (the corporate bond is issued, DONE) before 10b (short-term debt)**: a firm that cannot issue
  paper at five years cannot issue it at three months either; the issuance path is one mechanism, and
  10b now has one to shorten rather than one to build.

| # | item | closes | why here |
|---|---|---|---|
| **10f** | Public and private firms, and the market for control | — | **inserted** (owner, 2026-09-14, four sentences): listing is a funding choice and not a size; every firm has equity and public is a MARKET; M&A is general — acquisition, merger, disposal, take-private — run as a formal process by a bank's IBD; and institutions invest across strategies. It takes the next letter after 10e because it is the same shape of correction, and it comes before the rest of 13 because **13.5b is an application of it** |
| **13** | Asset managers: §28, §29, §15 | 1 | **MOVED AHEAD OF 11 AND 12** (owner): 10, 10b and 10c all built SUPPLY into a world whose only buyers are bank desks and bank liquidity books, and 11 and 12 add more issuers. This is the item that adds a BUYER. Unblocked since `Mandate` at 9.2a, and nothing in 11 or 12 needs it |
| **11** | Small-Business Pools (§42) | — | **inserted**: dependencies (trade credit 13e, bank lending 13d) are both closed and item 9 gave it the agreement; takeable now, and **12 needs it** |
| **12** | Firm birth, and the boundary firms cross | 3 | **needs 11**: a firm is born SMALL, which is §42's sector, and is promoted out of it when it outgrows one. Also **7** (`Lifecycle`, built) and **15** (`Objective`, built); worklist 13n |
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

**Stage B is finished**: every line the old file left open is closed. **Stage C (13, 11–18)** builds the
sectors that are not there; **10 is closed** and its section is gone. **10c and 10d are the owner's
two corrections of 2026-09-14 turned into work, and both are net DELETIONS** — a kind branch and a
duplicated issuance mechanism. **Stage D (19–24)** is the existing worklist tail. Items 1–10d and 7b are
closed and their sections are gone: this is the plan of what is left, and `docs/RECORD.md` is the
ledger of what was done.

**10e is closed and its section is gone.** It was the asset-management redesign the owner dictated
over five corrections — one object, a universal blueprint language over reads of the investment
universe, liquidity terms that decide who can be forced to sell, a manager that is a business with a
cost in people, and access as a policy asked at a door. Two of its steps left it by being PLACED and
not by being dropped: **17.9** (one funding-request channel, so a manager can borrow to seed a
launch — it lands with corporate credit because it is `corporate-bond` and `short-term-debt` that
also read the event it replaces) and **23.0a** (which blueprints this world grew and which it wound
down, which is a MEASUREMENT and belongs with the measurement pass). It also **shrinks item 13** to
two sentences and prime brokerage, which is the largest thing it did to the plan.

---

## Part 2 — The items

---

## 10f. Public and private firms, and the market for control — **inserted**

**Where this came from.** The owner, 2026-09-14, in four sentences:

> *"There should be named public and private firms. Going public is a matter of funding choice, not
> how large a firm is."*
>
> *"IPOs and take privates should exist, while also sponsor to sponsor and sales to corps should
> exist in formal exit processes and m&a processes lead by IBD departments."*
>
> *"Also, insurance and pension don't only go for duration. They invest in tons of different
> strategies."*
>
> *"And m&a, acquisitions, mergers and disposals should all exist, with the PE case being only an
> application."*

**Why it is numbered 10f and taken HERE** (Law 10: insert at the dependency position, and say where).
It is the same SHAPE of correction as 10e — a declared outcome becoming a decision — from the same
owner in the same conversation, so it takes the next letter. It is taken before the rest of item 13
because **13.5b is an application of it**: a private-equity fund buys unlisted equity, and there is
no unlisted equity in this world to buy. It also corrects 14.0, which is three commits old.

---

### 0. The defect underneath all four sentences

`drawListed` opens with `if (f.size < LISTING_SIZE) continue;` — **which firms are public is decided
by a size threshold.** Two things are wrong with that and the second is worse than the first.

1. **Listing is a declared outcome** (Law 2). Whether a firm is public is a funding decision it took,
   and here it is a fact about the draw.
2. **A firm below the threshold has no share line at all** — so of the nine thousand firms this world
   draws, the ones that are not listed **have no equity instrument and therefore no owner.** A firm
   is a residual claim on its own assets, and in this world almost every one of them is a residual
   claim nobody holds, which is the thing Appendix B names first: *no residual with no holder.*

That is not a gap to fill later. It is why there is nothing for a private-equity mandate to buy, why
a firm cannot be taken private, why nobody can sell a business, and why the only owners in this world
are bank dealing desks holding a float.

### 1. Every firm has equity. PUBLIC or PRIVATE is whether it has a MARKET

One change, and everything else follows from it:

```
today     a share line exists  IF  size >= 8          and it always has a market
10f       a share line exists  ALWAYS                 and it has a market IF the firm listed
```

The read already exists and is already load-bearing: **`instrument.market.some` is what item 10e's
classification calls `listed`**, so a blueprint that says `listed: false` starts admitting things the
day this lands, with no change to the language. §29's mandate over unlisted equity stops being a
mandate over nothing.

Who holds a private firm's shares at period zero is an opening condition (Seed B1) like every other:
they are owned by somebody named, and the honest somebody in this world is the household cells where
the firm is. A firm owned by nobody is what we have now.

### 2. An IPO is a funding decision, and the price CLEARS

A firm that wants money compares what it would cost from each source — which is exactly the shape
item 10 already built for the bond (*"a firm compares two prices for the same money"*). Equity is a
third column in that comparison, and a private firm that finds it the cheapest **lists**: it opens a
market in its line, offers new shares into it, and takes the proceeds.

**What the offering is worth has never been printed, so it is BOOK-BUILT** — the firm offers a size,
investors bid what it is worth to them (Equity B1, §46 A3: they disagree), and the price is what
clears (Law 3). That first print is the first price the line has ever had.

**And that is the same mechanism §29 D needs** — *"the exit produces the first real price the holding
has had"*. An IPO and a PE exit by flotation are one thing, which is the first place the owner's
*"the PE case being only an application"* bites.

A take-private is the reverse: somebody buys the line and **the market closes**. The firm does not
die — it is owned differently.

### 3. M&A is the general layer. Acquisition, merger, disposal — and PE is one caller

`mechanisms/control` already has the whole of a change of control: `worthToBuyer`, `tenders`,
`runTender`, an acceptance condition, and *"a bid is an ordinary book with an unusual buyer"*. What it
does not have is the distinction the owner is asking for:

| shape | what happens to the target | what exists today |
|---|---|---|
| **merger** | the two balance sheets combine and the target is terminated | `combine` — the only outcome there is |
| **acquisition** | the target **SURVIVES** under a new owner | **missing**, and it is what a buyout, a sponsor-to-sponsor and a trade sale all are |
| **disposal** | an owner SELLS a business it holds | **missing**, and it is a PE exit seen from the seller's side |
| **take-private** | an acquisition, plus the market closes | **missing** |
| **flotation** | the opposite: a market opens on a line that had none | **missing** (§2 above) |

`combine` is right for a merger and wrong for everything else. **A5 of §29 says acquired firms are
held in named vehicles** — the target has to keep its own balance sheet, because B2.a's debt is the
TARGET's liability and *"that is why a failed buyout kills the firm and not the fund"*.

### 4. The process is FORMAL, and a bank runs it

*"Formal exit processes and m&a processes lead by IBD departments."* So a sale is not a bilateral
tender that appears from nowhere:

- a seller **appoints a bank** to run the process — an agreement, the eleventh kind;
- the bank **invites bidders and runs an auction**, which is a book and it CLEARS (Law 3) — the
  solver already does this, and a controlled auction is `runTender` with more than one buyer;
- the winner buys; the bank earns a **fee out of the proceeds**, which is real income for real work;
- and **what a bank can run at once is the people it employs** — `advisory` is a trade in the labour
  market like `banking`, `dealing` and `analysis`, and an IBD's capacity is the same read
  `banks/staff.ts:linesCovered` already makes for a dealing desk. A bank that sheds its bankers runs
  fewer processes. Nothing states a capacity.

This is also what makes a process FAIL honestly: an auction with no bidders clears nothing, and the
seller still owns the company.

### 5. Institutions invest across strategies, not only duration

14.0 made an insurer match its liabilities' duration and nothing else, and the owner's correction is
that *"they invest in tons of different strategies"*. Both halves of that are right and the fix keeps
both:

- **What it requires is what its own promises are discounted at.** An insurer's liabilities have a
  present value at the curve, and the rate they are discounted at is the rate its assets must earn to
  cover them. That is §27 B2's actual economics and it is a READ — no new preference, no allocation
  percentage.
- **What it will not take is duration it did not promise.** A pool whose stated duration runs past
  its longest promise is refused, because holding it is taking a rate risk nobody asked it to take.
  A pool that states NO duration is not refused — equity has no duration to mismatch, which is how
  strategies, credit and equity all become eligible.
- **How it spreads is by feeding the smallest.** It tops up whichever of its positions is smallest.
  Diversification is then an OUTCOME and there is no allocation rule anywhere — no percentages, no
  target weights, no optimiser.

### 6. The order

- [x] **10f.1 DONE.** Every firm has a share line and `EquityDecl.listed` says whether that line has
  a MARKET. `LISTING_SIZE` is gone and so is `drawListed`: what is drawn is `PUBLIC_AT_THE_OPENING`,
  one in twelve, **independently of size** — a firm is public because it once wanted money it
  preferred not to borrow (D1.b), which a small firm may choose and a large one decline. Three things
  came with it and none of them was foreseen in this step:
  - **A residual with no holder, at scale, and silent.** The size gate left more than eight thousand
    of nine thousand firms with no line at all — and among the ones it did list, the float was
    divided across every saver in the world and then DROPPED whenever the division came out under one
    piece a member: `if (perMember > 0)`, a bound (Law 6) behind which a line existed, a market opened
    and nothing was ever issued into it. Both are gone. The holders are now **as many cells as the
    count can fill**, taken in a rotated order so no one cell owns every small company: a big line
    reaches every saver, a small one reaches one cell of them, and *fewer people own a smaller
    company* is the true statement as well as the arithmetic one.
  - **`OPENING_SHARE` was too coarse to be a resolution.** A share is indivisible and a cell holds
    whole pieces per member (XI-15), so the opening price decides HOW MANY OWNERS A LINE CAN REACH:
    at a dollar a share a firm's book came to fewer shares than this world has savers. It is now ONE
    TICK — a cent a share, said as `CENT_TICK × MONEY_PIECES / SHARE_PIECES` rather than as a number
    — which cuts the same book into a hundred times as many pieces and, by D4, changes nothing else.
  - **The carrying rule had to learn to read the LINE** (§29 C5, C5.a). `pricing: 'cleared'` plus
    `carry: 'mark'` is a property of the KIND, so the first private share line in this world would
    have made the revaluation ask `printOrThrow` for a price that never existed and stopped the build.
    `Valuation.atCost(instrument)` is the one reader of it now — carried at cost, or a cleared kind
    whose line has no market — and `worthOf`, `valueOfLots` and the revaluation all ask it. **That is
    C5.a kept by there being no price to mistake for one**, and it closes §29 C5 and C5.a outright.
- **10f.2** The IPO: a funding comparison with a third column, and a book-built first price.
- **10f.3** M&A generalised: acquisition (the target survives), merger (`combine`, as today),
  disposal, take-private. One layer, four outcomes, no kind branch.
- **10f.4** The formal process: a mandate to a bank, an auction with bidders, a fee, and `advisory`
  as a trade with a capacity.
- **10f.5** Institutions across strategies (corrects 14.0).
- **10f.6** §29 B, C and D as CALLERS of 10f.3 — which is what 13.5b becomes, and it shrinks to
  naming the vehicle and the carry.

### What 10f.1 found and did not chase

- **F-1 (the resolution floor).** A line can only reach a cell whose whole weight it has a piece for,
  so a firm whose book is under one cell's worth of pieces — about twelve thousand dollars at a cent
  a share, against thirty million people in twenty-four cells — still opens with an **unissued line**
  and a residual nobody holds. It is two thousand four hundred times smaller a hole than the one
  10f.1 closed, and it is not a bound: nothing drops it, the arithmetic simply cannot cut a company
  into fewer people than a cell stands for. **The mechanism it wants is a finer population**, which is
  `seed.households.membersPerCohort`'s own placeholder (12.6) and XI-15's cell SPLIT, or a proprietor
  that is a party rather than a cell — Firm Birth A, where a founder is somebody who funded an entry
  (13n). It is visible without being looked for: `equity.reads` publishes `shares` every period, and
  for such a line it is zero. **Positioned at 13n.**
- **F-3 (a build-stopper, and it was fixed where it stood).** `Instruments.add` FORBADE a cleared
  kind with no market — *"priced by clearing but names no market"* (Clearing D1) — so the very first
  private share line this world declared would have thrown at the seed. It is a real state and not a
  defect: §29 C5 says a holding with no clearing is a MARK, so the forbid is deleted and what answers
  the question it was asking is `Valuation.atCost`, which reads the line and not only its kind. **Not
  a finding to position: the engine will not run past it (the exception the owner's three rules name),
  so it is fixed and written down.**
- **F-4 (a fund cannot bid for a share, and 10f.2 needs it to).** `funds/index.ts:ordersOf` prices
  its bid with `priceAt(flows, …)` and returns nothing when `flows.length === 0` — and a share
  promises no dated payment, so **the flows are always empty and no fund in this world has ever bid
  for a share**, even though `eligible` (three lines above, through `view.worth`) says its mandate
  admits one. Two valuations of one thing (Law 4) that disagree about whether a share can be valued
  at all, and the kernel already answers it: `view.worth` discounts a promise where there is one and
  capitalises what a company published where there is not. **Positioned at 10f.2**, which is where
  the bid side of an IPO comes from and cannot come from anywhere else.
- **F-5 (`control.tender` is now nine thousand by nine thousand).** It walks every FIRM against every
  equity line, and the line count went from seven hundred and forty to nine thousand. The guards are
  real — a bidder needs `credit.quoted` and cash, a target needs published accounts — but the walk
  itself is twelve times what it was. **Positioned at 16** with F-2, for the same reason.
- **F-6 (nobody can tender for a private company).** `control.tenders` asks a holder for its OUTLOOK
  on the line's PRICE, and an outlook is formed from prints. A private line never prints, so no
  holder ever has one, so every tender for a private company fails with `control.failed`/"nobody
  tendered" — the acquisition the owner asked for, refused by the one read that cannot answer for
  it. A holder of an untraded company answers from what the company published, which is the same
  `worthTo` door the buyer uses on the other side of the same book. **Positioned at 10f.3.**
- **F-2 (the cost of a line for every firm).** `equity.decide` now runs for nine thousand firms a
  period instead of seven hundred and forty, and nine thousand instruments exist where there were
  seven hundred and forty. Nothing about it is wrong — every firm decides about its own money and
  every firm's residual is a real claim — but it is the first item to multiply the instrument table by
  twelve, and Law 18 says that is a traversal question and not an economics one. **Positioned at 16
  (Measure)**, where what a period costs is measured rather than guessed.

### Exit

A firm this world drew as small is owned by somebody named; it borrows, and one day it lists because
equity is cheaper than its bank; a sponsor buys a listed company and the market closes behind it;
another sponsor buys it from the first through a process a bank ran and was paid for; and an insurer
holds six mandates because six of them cleared what its promises cost it.

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

**ITEM 10e SHRINKS THIS ITEM TO TWO SENTENCES AND A LENDER** (owner, 2026-09-14): *"everything is a
fund. An HF runs funds, same as PE."* So there is no hedge-fund party kind and no private-equity
party kind to build. §28 is **a manager whose blueprints permit leverage and shorting**; §29 is **a
manager whose funds are closed-end over unlisted equity**; and §15 prime brokerage is the LENDER
those permissions need, which is the one genuinely new party here (B1: *"leverage is a fact about a
loan, never a property of the fund"*).

**Its strategies are several and its investors are institutions**: long/short equity, long/short
credit, macro, futures, commodities — each a blueprint in 10e's language, not a mechanism — and the
money comes from pension funds and insurers, which item **14** turns into allocators. **An HF does
not go bankrupt because a fund does badly**: its exposure is the seed it chose to put in and the fee
income it stops earning, and the fund's holders bear the fund's losses (A3).

**AND ITS REASON MUST BE A VIEW, NOT AN ARBITRAGE IT CANNOT LOSE.** The owner's phrasing is that
hedge funds *"exist to exploit arbitrages"*, and the mechanism has to express that WITHOUT becoming
what Appendix B forbids — *no free arbitrage, no unlimited arbitrageur*. A party that closes every
gap by construction deletes the mechanisms it is meant to trade: the net basis never persists (M8),
the ETF premium stops meaning anything (§13 E3.a, which says explicitly that the gap closes because
somebody TRADES and *"can persist when they will not"*), and the liquidity premium XI-2's forced
seller pays has nobody to pay it to but also nobody who can decline. So §28 C1–C4's framing is the
one to build: relative value, direction, **a liquidity premium it is paid to hold** — a view it puts
its own and a named lender's money behind, that can widen against it before it comes right, and that
its lender can cut (B1: *"leverage is a fact about a loan, never a property of the fund"*).

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

- [x] 13.1 **DONE AT ITEM 10e**, and further than written: `Mandate` is not only the spine, it carries the whole PRODUCT — blueprint, liquidity, tracks, fee, buffer, required yield, access and whether notice has been given — and the pools that exist are the performing mandates rather than a declared roster. Original text: a hedge fund is **a mandate with leverage**, a separate account is a mandate whose pool is the client's own balance sheet, an ETF and an MMF are pools with different redemption rules. Build the three sectors on it and nothing else.
- [x] 13.2 Hedge funds: **the party, the manager, the two fees, the wide mandate** — and there is no hedge-fund party kind, because `mayWrite`, `leverage` and the performance fee are terms of a MANDATE and a strategy is a BLUEPRINT (10e's language). `mayWrite` and `leverage` came off `openMandate`, where they were hard-coded the same for every mandate in every world; `fundKind.borrows` no longer says `false` for the category; and the performance fee is charged over a high-water mark read off the last charge, never stored. **What is NOT here and why**: the SPECULATIVE side (C1) — a pool is spoken for in every contract book and its wide mandate says yes, but what it POSTS is the class's own `orders`, which is a hedger's reason; turning a party's own OUTLOOK into a position is a second reason on `DerivativeClassDecl` that every class implements, which is item 6's other half and **13.2b**. And leverage PERMITS without supplying: the lender is 13.3, and the bank's request channel is 17.9. Original text: — `mayWrite` and `leverage` are the two terms that make a mandate a hedge fund's, and both exist (item 9.2a, 9.7). `borrows` comes off the mandate, not off `fundKind` where it is hard-coded `false`. **Not before 13.6**: a mandate that writes contracts before the NAV pass reads them is a fund with equity.
- [ ] 13.2b **THE SPECULATIVE SIDE** (§28 C1, and the other half of item 6). A pool with a wide mandate is asked in every contract book and the NAV can see what it takes (13.6) — but the ONLY reason any class knows how to post is a HEDGER's: `futureOrders` sells against a book it holds, and a party with no such book posts nothing. So *"the natural home of the speculative side of every derivative book"* has a door and nothing to say at it. The fix is NOT a kind branch inside a class (forbidden, and it would be one): `DerivativeClassDecl` gains a second reason — what a party with a VIEW about this subject would take — answered by the class, which knows its own payoff, out of the party's OWN OUTLOOK, which §46 already builds and which every deciding party already has. A bank with a view speculates too, and that is right. It touches nine class modules, which is why it is its own step. **Read the item's own warning first**: the reason must be a view that can widen against it, never an arbitrage it cannot lose (Appendix B: no free arbitrage, no unlimited arbitrageur) — a party that closes every gap by construction deletes the net basis, the ETF premium and the liquidity premium XI-2's forced seller pays.

- [x] 13.3 Prime brokerage: the relationship as an `Agreement` (`banks.prime`, appointed and endable); portfolio margin as the broker's own decision, built out of ITS OWN OUTLOOK about each line (§46 B3's `confidence`) so two brokers want different amounts and **C4.a falls out rather than being written**; and **no floor on the line** — `available` is a subtraction allowed to come out negative, and that negative number IS the call. It lives in `banks/` because it is a bank's fifth business line and what it does is WRITE A LOAN (ARCHITECTURE 4.11b). Two kernel doors it needed: the FUND party-kind ids moved to `registry/profiles.ts` (a name a module that does not own the kind still has to say), and `leverageLimits`/`ParticipantView.mayBorrow` — the mirror of `mayTrade`, so a lender asks the borrower's own module whether it may be levered instead of reading its mandate. **What is left is 13.4's**: *"meet it or be liquidated"*, and the chain D1–D4.
- [x] 13.4 The loop D1→D4 falls out of the parts, and **no contagion step was written** (D4.a). Two changes, both of them REMOVALS of something that was preventing it:
  1. **An unmet call joins what the pool must find money for.** It is one line in `strike`: the shortfall a pool publishes is now its redemptions AND what its broker called and it could not pay. The pool already sells across its book pro rata at whatever the market gives when it has a shortfall (`ordersOf`, C2.b, XI-2), so the forced sale needed no mechanism of its own — and the loop then closes by itself: the sale is a print, the print is what every other levered pool is marked at, a lower mark is a smaller portfolio, a smaller portfolio against the same loan is a negative line, and a negative line is a call.
  2. **A SHARE CANNOT BE WORTH LESS THAN NOTHING**, which is what lets a levered pool die at all. While the claim could go negative, a fund's share liability absorbed every loss exactly — assets, less the loan, less (assets less the loan), is zero — so its EQUITY ACCOUNT stayed at zero however far underwater it went and `failedWhy`'s solvency test could never fire for one. §28 E3's *"no fund that cannot fail"* was true of no fund in this world. It is not a floor (Law 6): a negative share value is a claim that its HOLDERS OWE THE FUND MONEY, and a share is a limited liability — the `ranking` this kind already declared said so in words and `navOf` now says it as arithmetic. With the claim honest the account stops netting, the pool is insolvent, its estate opens and its broker eats the shortfall (§15 D2, XI-3's fund row).
- [ ] 13.4b **MEASURE the chain**: one fund's loss reaching another fund's margin call through prices and named counterparties, and the path traceable party by party (§15 D4: *"a loss that stops at the fund is a broker that was never really lending"*). The events to follow are `prime.line` → `prime.call` → `fund.struck`'s shortfall → a print → the next `prime.line` falling, and every one of them names both parties. **It is a MEASUREMENT of the assembled world and not a unit test** (Law 11, and the owner's rule that the suite runs at the end), so it goes with 23.0a — and it wants the same thing that one does: read `E-22` first, because the pools this path runs on gather nothing until a cell clears the accredited line or item 14's institutions arrive.
- [x] **14.0 INSERTED AHEAD OF 13.5 (Law 10: at its dependency position, and here is where).** 13.2 built a strategy house, 13.3 built its broker and 10e built a credit fund — **and none of them is offered to the public, so the only investors this world has cannot reach any of them.** A private-equity fund is the same shape and would have been the fourth. The money those sectors run on is INSTITUTIONAL, and nothing in this world allocated any: an insurer wrote cover, took premiums and sat on the cash for ever. So item 14's allocator half is taken here, and it is small because 10e made it small — *"insurance companies and pension funds don't invest themselves; their assets are always third party managed"*, so there is no portfolio mechanism to build. An insurer puts what it can spare at the door of the pool whose stated duration is nearest the furthest thing it has promised (B2.b), and everything after that is the manager's. **It also fixed the access ladder**: the rungs are not three kinds of investor but one structural fact — a CELL is people and answers with its wealth, a NAMED party is an institution and is qualified by being one, which is what the ladder said all along and what `E-22` was half about.
- [x] 13.5 Private equity, **§29 A: committed capital and the call**. There is no private-equity party kind — it is a manager whose pools are CLOSED-END over UNLISTED equity, which is four terms of a mandate. The commitment is an agreement between a named investor and the pool; the call is a real payment on a date it cannot refuse; the door is shut to ordinary subscription because the only way in is a call; and shares are issued against what a call actually brought in, at the NAV, through the door every subscriber uses. **A2.b — the FORBID this item singled out — is guarded rather than commented**: a call bounded by the investor's spare cash is not an obligation, so nothing in the call path may trim a demand to fit a balance, and `tools/check-forbids.ts` refuses `atMost`, `atLeast` and `Math.min` anywhere in it (the guard was proved to bite before it was trusted). The call path is one file for exactly that reason.
- [ ] 13.5b **FOLDED INTO 10f.6** (owner, 2026-09-14: *"m&a, acquisitions, mergers and disposals should all exist, with the PE case being only an application"*). What is left here once 10f.3 exists is naming the vehicle and the carry; the buyout, the sponsor-to-sponsor sale, the trade sale and the flotation are all CALLERS of one M&A layer rather than a private-equity mechanism. The analysis below stands and is what 10f.3 was written from. **The buyout, the hold and the exit** (§29 B, C, D — 19 clauses). What 13.5 built is how a PE fund is FUNDED; this is what it does with the money, and **this world has nothing for it to buy**: every share here trades, so a mandate over unlisted equity admits nothing and the fund holds the cash it called. Three pieces, and the first two are mostly reuse:
  - **B, the buyout.** `mechanisms/control` already has the whole tender — `worthToBuyer`, `tenders`, `runTender`, an acceptance condition, *"a bid is an ordinary book with an unusual buyer"*. Two differences make it a BUYOUT rather than a merger: **the target must SURVIVE** (`combine` terminates it and reseats its rows, which is right for an acquirer and wrong here — A5 says acquired firms are held in named vehicles), and **the debt is the TARGET's** (B2.a, *"which is why a failed buyout kills the firm and not the fund"*), which wants the target to raise it — and that is the funding channel 17.9 opens. B2.b then falls out: if the credit market will not lend to the target, the leverage does not happen and the fund owns an unlevered company.
  - **C5.a, the hold.** *"An unlisted mark is not a cleared price … the honest answer is 'marked, not cleared'."* `pricing: 'carriedAtCost'` is already the kernel's word for exactly this, and `E-18`'s neighbourhood (item 21) is where the display of it lands.
  - **D, the exit**, which *"produces the first real price the holding has had"* — the listing or sale that turns a carried mark into a print.
- [ ] 13.5c **A2.a: an investor holds liquidity against calls it did not choose the timing of.** 13.5 built the OBLIGATION and the DEFAULT; what an investor does to avoid defaulting — selling down its own liquidity ladder when a call arrives — is its own module's, and for an insurer that means redeeming the fund shares 14.0 gave it. It is the same forced-seller channel every other party here uses (XI-2), pointed the other way.
- [x] 13.6 **The pass that re-marks a fund's claim on itself reads the CONTRACT store**, and it landed before 13.2 draws a mandate that writes anything. `DerivedReads.contractsOf` is the door — signed, per contract, in the money each was written in — injected into `Valuation` the way the curve and the calendar are, so `contract-value.ts` stays the one writer of a mark (Law 4). `navOf` splits it by sign: an asset to one side and a liability to the other (D1), never netted, because a pool long one and short another HAS both. **What remains of this step is 13.2's**: wiring hedge funds into every derivative book as the speculative side (§28 C1) needs a mandate that writes, and there is none. Original text: A fund's equity is zero by construction (Fund Shares A3) because its own claim on itself absorbs whatever its book comes to — and that pass reads the REGISTER, where a contract is not (Derivative X1). A pool with a derivative position would carry a mark its share value had never been told about, which is a fund WITH equity: **measured at 83,247,864 on `etf.us`** the first time funds were let into the contract books. Item 9.7 put `FUND` on `TRADES_CONTRACTS` and made every drawn mandate say `mayWrite: []`, so nothing reaches this today — **by a term of a contract, which 13.2 is about to change.** Open the pass first. Then wire hedge funds into every derivative book as the speculative side (§28 C1), which is the other half of item 6.
- [ ] 13.7 COVERAGE re-marked for all 73 clauses across the three; `check:existence` shows three fewer absent sectors. **§28 is done at 13.2**: Hedge Funds is 5 MET, 3 PARTIAL, 16 MISSING and is no longer an absent sector, so the count is four down to three. Prime Brokerage (13.3) and Private Equity (13.5) are the two left.
- [ ] 13.8 **`E-14`** — `Securities Lending C2, C2.a` are not built, and they are §15 C1's mechanism seen from the other end: both sides of a position marked every period and the difference CALLED in real money between two named parties. `securities-lending:charge` moves the fee and nothing re-marks the collateral, so between the strike and the return the lender's cover erodes silently and C1's haircut is all that stands behind it. Build it once, here, for the portfolio and the stock loan together — two callers of one mechanism, not two mechanisms (Law 4).
- [x] 13.9 **DONE AT ITEM 10e.4**, which took it whole and went further than this step asked. (a) a manager HIRES in the labour venue, in the `analysis` trade, at what an hour is worth to it. (b) The fee moved off `params` onto `MandateTerms` and the placeholder died — `check:deaths` counts four where it counted five. What 10e.4 did DIFFERENTLY, and deliberately: the fee is not won in a book per pool. A mandate auctioned between managers with no reason to refuse clears at the tick, which is the defect this step was written to avoid and would have reproduced from the other side. What sets a fee is ENTRY: a manager opens a competing product at a fee under the cheapest incumbent, and stops when that fee would not cover what a pool costs it — so the fee falls where several run one blueprint and nothing bounds the fall (Law 6: the refusal is the mechanism). Original text: `fund.fee.<fund>` is a `placeholder` per pool whose own `why` says *"no manager competes for the mandate, so the number stands where a competition should be"*. Item 9.2a built the `Mandate` and 9.2b measured why that is not enough: **a manager in this world employs nobody**, funds nothing and pays for nothing, so two of them in a book bid each other to the tick — which is a competition between parties with no reason to refuse, not a cleared price (Law 11: the missing mechanism, not the number). So this step is two things in one order: (a) a manager HIRES, in the labour venue, like anything else that needs people, and what it can run is the hours it pays for over the assets a mandate carries — the shape `banks/staff.ts:linesCovered` already has for a dealing desk; (b) then the mandate is COMPETED FOR, one book per pool, each manager bidding a fee with its own cost base as its floor, and the winner's bid is the mandate's `fee`. The placeholder dies in the same change (Law 2) and `fee` moves off `params` onto `MandateTerms`, because at that point it is an OUTCOME. A separate account is a mandate whose pool is the client's own balance sheet, so the same book prices that too (§15).

### Findings this closes

`E-14`. (`B-14` closed at item 9.7: a pool's mandate is what answers whether it may hold contracts, and 13.6 is the pass that must open before 13.2 draws one that does.)

### Exit

A hedge fund takes the speculative side of a derivative book; a margin call forces a sale that moves a
price that calls margin on somebody else; a buyout puts debt on a target and the credit market decides
whether it happens.

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
- [x] 14.3 **DONE AT 14.0, and 10e changed what it means.** This said *"the insurer BUYS duration"*, and after the owner's correction an insurer buys nothing: it hands its assets to a manager whose mandate is shaped like its promises (B2.b), and the manager buys. It is still the answer to *"nothing in this world is a natural buyer of a long bond"* — the insurer's money reaches a long bond through a credit fund's mandate rather than through a decision of its own, which is one fewer investor in this world and one more real one. What is NOT done is the other half of 14.3's sentence: whether a long issue actually finds that money is a measurement (23.0a).
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
- [ ] 17.2b **A PRIME LOAN DOES NOT FALL DUE WHILE THE RELATIONSHIP STANDS** (`E-24`, re-positioned here at 13.4). A margin loan gets `maturity: drawn.y + 1` like every other row, so a broker's financing falls due rather than rolling and a client that cannot find the whole principal that week has a failed payment for a reason that has nothing to do with its margin. It is 17.2's because a line that does not fall due while the commitment stands IS a committed facility. **It needs a kernel door that does not exist**: `ctx.restate` amends an AGREEMENT's terms and nothing amends an INSTRUMENT's, so rolling a maturity is not expressible today — and that door is worth opening once, here, for every revolving line in this world rather than for this one.

- [ ] 17.9 **ONE FUNDING-REQUEST CHANNEL, so any borrower can use it** (from 10e.5b). `banks/index.ts:runRequests` reads `firms.funding` and `housing.funding` — two named event kinds where there should be one, which is a kind branch wearing a list's clothes (Law 15) and which no third borrower can join without making it three. **The lender side is already general**: `publishQuotes` walks every party whose KIND borrows, so a credit quote is already published under a fund manager's own name every period, priced off its own risk — what stops a manager borrowing to seed a launch (`Fund Shares F3`) is only the shape of the request. The fix is one published kind meaning *"a named party said what it is short of"*, which also lets an insurer, a vehicle or a small pool borrow with nothing added. **It lands HERE because it is not free**: `corporate-bond` and `short-term-debt` both read `firms.funding` to decide whether a FIRM should come to market instead, so a generic kind means each of them must say it means firms — which is this item's files, opened for this item's reasons.

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

- [ ] 23.0b **Re-mark the ninety-eight NEVER REACHED rows from the run** (`E-25`). They cite `B-1 to B-8 and B-12`, of which only B-2, B-9 and B-13 still exist in this file, and `check:existence`'s BUILT AND DEAD list is computed off them — so five sectors are reported dead on the authority of a finding nobody can read. Several of the modules have been given seeds, phases and participants since the measurement was taken (insurers at 9.5 and 14.0, securities lending at 9.4, the derivative layer throughout). One pass over the journal of a long run answers it for all ninety-eight: a module that produced an event produced an outcome. Fix the COVERAGE header in the same change.

- [ ] 23.0a **Which blueprints this world grew, and which it wound down** (from 10e.8b). Item 10e made the roster of funds an OUTCOME — a manager launches a product it can see working at a fee under the cheapest incumbent, and closes one whose fee stops covering what running it costs — and nobody has yet run the world to see what that produces. Read `fund.launched`, `fund.notice` and `fund.woundUp` over a long run, against `fund.fee` and the wage bill on the other side. **Read `E-22` first**: if no household cell clears the accredited line then two of the three products this world opens with have no investors at all, and what the run is measuring is the LINE rather than the industry.

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
| **E-11** (B) | 21 | `Outcome.price` and `Print.price` are `PerPiece` and some books clear a RATE — the subordinated raise, the money market, the IRS, the CDS, the cross-currency basis, and now the BORROW book, whose level is a fee per period on value and not money per piece (item 9.4). `E-10` closed the CONTRACT layer (the level a contract carries is tagged by its kind's `quotedAs`) and every reader of a rate-quoted print now says `asRatio` at its own door, so the crossing is named everywhere it happens — what is left is that the STORE still cannot say it. **Item 10d TOOK one of the five books away rather than teaching the store a trick**: the subordinated raise cleared a rate only because it was a private venue, and it is now a bond book clearing a price per unit of par. Four left. The solver genuinely need not care (a schedule is size against a level either way); the print store is where the claim would live. **Re-positioned from item 6, which did not close it** |
| **E-13** (C) | 21 | `households/portfolio.ts:ownUncertainty` implements §46 B3's income channel and has NO CALLER — a saver's bid is built from its PRICE outlook's confidence and its income uncertainty reaches nothing. Either wire it or delete it; a mechanism nobody reads is not one. **Found closing `A-30` at item 5** |
| **E-17** (B) | 17.6 | **commercial paper is not repo collateral** (`Short-Term Debt D3`), and D3 says being collateral is *"a large part of why anyone holds it"*. `money-market/collateral.ts` already has the haircut machinery and what is missing is that it accepts this kind — 17.6 is the same shape for senior notes, so it is built once for both. **Found closing item 10b** |
| **E-18** (C) | 21 | `funds/index.ts:1296` cites `Clearing C1.b`, which does not exist in the spec. It is a PROSE citation so `check:spec` cannot see it — the tool reads `@spec` tags only — which makes it the kind of stale comment Law 16 calls a defect and nothing guards. Two questions for 21: the right clause for that sentence, and whether the citation check should read prose citations too. **Found closing item 10b**, where the same wrong id was written into a new `@spec` tag and the tool DID catch it |
| **E-24** (B) | 17.2 | **a margin loan matures in a year like every other loan.** `write` gives every row `maturity: drawn.y + 1` and the kernel books the principal as due on that day, so a prime broker's financing falls in one week rather than being rolled — and a client that cannot find the whole principal that week has a failed payment for a reason that has nothing to do with its margin. A real prime loan is callable and rolls; the row should be rewritten or the maturity reset by the broker while the relationship performs. **RE-POSITIONED at 13.4 from 13.4 to 17.2.** 13.4 turned out to need nothing of it: what it builds is what happens when a client CANNOT pay, and that answer is the same whatever demanded the money. What E-24 actually wants is a line that does not fall due while the commitment stands — which is a COMMITTED FACILITY, and 17.2 is the step that builds one (*"a stated limit the bank is obliged to honour, undrawn headroom, the fee on it... so a borrower stops being re-underwritten at every draw"*). It also needs a kernel door that does not exist: `ctx.restate` amends an AGREEMENT's terms and there is nothing that amends an INSTRUMENT's, so rolling a loan's maturity is not expressible today. Both halves are 17.2's. **Found building 13.3** |
| **E-25** (B) | 23.0 | **ninety-eight COVERAGE rows and the COVERAGE header cite findings that no longer exist.** They carry *"NEVER REACHED: the module is assembled and has never produced an outcome (`docs/IMPLEMENTATION.md` B-12)"*, and the header names *"B-1 to B-8 and B-12"* — **of which only B-2, B-9 and B-13 survive in this file**; the rest went when the plan was rewritten and nothing said so. A citation that points at nothing is the stale doc Law 16 calls a defect, and this one is load-bearing: `check:existence`'s BUILT AND DEAD list is computed off these marks, so five sectors are reported dead on the authority of a finding nobody can read. **It lands at 23.0 rather than being fixed now** because what all ninety-eight assert is a MEASUREMENT — has this module ever produced an outcome — and 23.0 is the run that re-takes it. Re-marking them from the run is one pass; re-marking them from a guess is ninety-eight guesses. **Found building 14.0**, where two of the rows described work done this year and said in the same breath that the module had never run |
| **E-22** (B) | 19, 23 | **the accredited line may lock this world's HOUSEHOLDS out of everything but a money fund and a tracker.** **NARROWED at 14.0**: it was written as though it locked out every investor, and it does not — the line is a test for PEOPLE, and an institution is qualified by being one (a named party, never a cell), so an insurer reaches a strategy or a credit fund whatever it is worth. What is left of the finding is the household half, which is the one the number was always about. The threshold built at 10e.6 is a POLICY and its value is the real one — a million dollars per member, which is what the actual accredited-investor and professional-client tests are built on. Whether any household cell in THIS world ever clears it is a question about this world's price level and its wealth distribution, and Law 11 says the answer is a measurement and not a thing to tune now. What it would mean if nothing clears it: the credit fund and the commodity fund gather nothing, and the demand side item 10e was built for arrives only with item 14's institutions. **Look at it at 23; if it is the LINE that is wrong rather than the world, it is 19's to move**, which is the whole reason the number is a policy with an owner rather than a constant |
| **E-23** (B) | 17, 18 | **retail's route into credit and commodities is an ETF, and this world has none.** The owner's ladder is explicit — *"retail able to access ETF and MMF, rich retail able to access funds"* — so a household reaches a company's paper through a CREDIT tracker, not through the credit fund. Every tracker this world draws follows an equity index (`drawTrackers`), because §26's indices are equity indices. A credit index and a tracker on it is what closes the gap, and it wants a rated universe to build the index over (§44 is built) and the corporate lines item 17 finishes. The same argument holds for a commodity tracker at 18. **Found building 10e.6**, where the access rule made the absence visible for the first time |
| **E-19** (B) | ~~10e.4~~ | **the wage read was written twice and the two copies did not agree about the key.** `firms/decide.ts` looked the going rate up under the VENUE's id, which is what `publishGoingRate` writes; `banks/staff.ts` looked it up under `region|occupation`, which is a key that has never existed. So a bank that had never met a payroll of its own could never fall back to the published rate, concluded it could not price an hour, and put NO staff cost into any quote it made — for exactly as long as it had no staff, which is when the published rate is the only thing there is. **Found and CLOSED at 10e.4**: the read is `registry/wages.ts` now and every employer in this world asks the same one (Law 4, Law 12 — the fix deleted both copies) |
| **E-20** (A) | ~~10e.4~~ | **a build-stopper I put there at 10e.2.** `offeredYield` read `fundParam(fund, 'maxTenorPeriods')` — the tenor parameter the blueprint language replaced — and nothing has declared it since. A read of an undeclared parameter THROWS, and this one is in the strike of every fund in the world, so the first period would not have finished. **Found and FIXED where it stands at 10e.4**, which is what the rules require of a violation that stops the build: the longest thing a mandate lets a pool hold is its duration BAND, in years, which is the number a curve is asked at anyway — so the tenor in periods, the date it came to, and the year fraction back out of that date all went with it. It is also the clearest argument this file has for the owner's rule about running the suite at the END of a module and not during one: nothing short of running the world finds this, and it was found by READING, which is what the item was for |
| **E-21** (B) | ~~10e.4~~ | **a saver committed one budget to every fund it could reach.** `households/portfolio.ts:fundOrders` posted a buy for the WHOLE of what a cell had spare at every fund whose offer cleared what it required, inside the loop over its positions. Nothing was created — the first strike took the money and the rest were refused by the wire — but which fund got a saver's money was decided by the order the venue list happened to be in. **Found and CLOSED at 10e.4**, because it is what would have made the whole item pointless: a manager undercutting a rival wins nothing from a saver that subscribes to everything regardless, and a fee nobody can lose business over is not a price (Law 3). A cell puts its money in the fund that offers it most, and ties break on the fund's own name |
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
