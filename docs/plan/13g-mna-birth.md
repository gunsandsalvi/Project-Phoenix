# Item 13g — Corporate control and firm birth

**Objective.** The market for control: an acquirer with its own valuation bids for a target's shares
in a tender that the target's dispersed owners accept or refuse each from their own valuation, so
the premium clears; funding by cash (a loan or bond the credit market decides) or shares (dilution);
competing bidders; management resistance; on completion the two balance sheets combine in the
register, the target's debt is addressed, its shares cease to exist, its employees carry over or
are separated through the labour market's own path, and the payment equals what the acquirer and
its lenders put up. Firm birth: entry as a founder cell's decision from sector margins and what it
can fund, with a new identity, a balance sheet that balances, plant bought from a producer, and an
age that the things that price it read. Placed before 13h because a buyout bids through this market.

**Read first.** §35 M&A (all); §34 Firm Birth A, B2, E3, E4; Equity F3 (votes), C1.b (free float),
D1 (issuance for consideration); Indices B2 (constituent changes); Labour C4; XI-15 (a founder cell
splits); Part XII (a firm trading cheap attracts bids). Code: item 9's `equity` and votes read,
item 7's estate and `reseat`, item 10's investment decision, item 12's ratings and indices, 13d's
`labour.separate`, 13f's syndicated financing.

**Clauses this item meets.** M&A A1–A5, A3.a, A3.b, B1–B5, B2.a, C1–C4, D1–D5, D2.a, E1–E3; Firm
Birth A1, A2, A2.a, A3, A4, A4.a, A4.b, A5, B2, E3, E4; Equity E1, E2, E3 (from item 9's PARTIAL
list); Indices B2 (constituent exit on acquisition, fully).

---

## Design

### Sub-item 13g.1 Kernel: merging two parties

- `parties.merge(target, into, consideration)` (a `MechanismContext` door, journaled): every
  register row of the target (holdings, loans as creditor and debtor, invoices, contracts, employment
  rows) is **reseated** to the acquirer in one operation (A4: one party afterwards); the target's
  shares are redeemed against the consideration instructions already settled (A2); the target party
  is terminated with the acquirer as its **successor** (the names family resolves every reference to
  it: D4, E2); the target's own instruments (its bonds) keep their identity with the acquirer as
  issuer (A5: assumed) unless a change-of-control term makes them due (a bond term from 13f: the
  profile's `due` fires on the journaled event) or the acquirer's decision repays them.
- The door is used by acquisitions here and by 13h's buyouts (the holdco is the acquirer).

### Module `corporate-control`

- `requires: ['equity', 'firms', 'corporate-credit', 'bank-lending', 'labour', 'indices',
'ratings']`.
- **Intent** (B1): in `firms.decide`, a firm evaluates targets it can see (listed firms: public
  prints and published accounts): its own valuation is the target's expected earnings **to it**
  (from its outlook of the target's line plus what it expects to change: cost lines it would remove,
  a market position: its own estimate, journaled privately) discounted at its own hurdle (item 10),
  against the price; a positive gap above its own margin of confidence (its confidence read) is an
  intent. No screening threshold parameter exists.
- **Funding** (B3, A3): the acquirer decides cash, shares or a mix from its cost of capital: cash
  from its balance above its buffer, a loan quote (item 6; a syndicated loan through 13f when large)
  or a bond it can bring (13f: the commitment market runs in the session before the tender so the
  financing is known); shares by issuing new ones as consideration (A3.b: dilution is the register's
  arithmetic; Equity D1). The credit market decides which cash deals happen (B3: no financing, no
  bid).
- **The tender** (A1, B2, C1, C2): a **tender market** per target per bid, in `markets`, a variant
  of the primary block: the acquirer posts a bid (price per share, consideration form, an
  **acceptance condition**: the fraction of shares it needs, a term the acquirer chooses, typically a
  majority of votes: Equity F3), and every holder's participant (cells, funds, insurers, insiders)
  decides to tender its holding or not from its **own** valuation (C1: the price beats holding on
  its own outlook; C2: dispersed decisions summed); the solver clears the tender: tendered ≥ the
  condition → the bid completes at the bid price (B2: the premium is the cleared price over the
  last print); below the condition → the bid **fails** (B2.a: journaled `bid.failed`; the acquirer's
  and the target's prints move because participants acted on the information, not because the event
  did: Observer B2.a); the acquirer may return with a higher bid in a later period (its decision).
- **Competition** (B4): two bids on one target in one session are two tender markets whose holders
  tender to the one that leaves them best off (their participant sees both public bids); the higher
  bid fills first; the other fails.
- **Resistance** (C3): the target's management publishes a recommendation (a public journal event
  from its own valuation of the firm under its own hurdle: it is an interested party: its insiders'
  holdings do not tender (Equity C2.e) and its recommendation is one input other holders' outlooks
  read as public information); management cannot block a bid that clears (the owners decide).
- **After** (D): the merge door; D1: the combined cash flows are the sum plus whatever the acquirer
  actually changes (a cost line removed is a real separation through `labour.separate` (E3, 13d), a
  supplier contract ended is a real invoice stopping); D2: the acquirer's leverage read rises when it
  paid cash and every holder's PD model and the agency reassess (D2.a: its bonds may fall on the day
  its shares rise: two reads); D3: the target's shares are redeemed and the index rebalances at the
  next chained rebalance (Indices B2); D4: employees, suppliers and customers carry over by the
  reseat; D5: an audit contribution per deal: consideration paid to the target's holders equals what
  the acquirer's accounts and its lenders put up, from the ledger, exactly.
- **C4** (cheap firms attract bids) is a read at 16.

### Module `firm-birth`

- `requires: ['households', 'firms', 'capital-programme', 'goods', 'bank-lending', 'ratings']`.
- **Entry** (A4, A4.a): in `households.decide`, a cell with wealth above what a plant of the kind a
  sector's recipe needs costs, and an outlook of that sector's margin above its own hurdle
  (patience), **founds** a firm: a split of the founding members from the cell (XI-15: the founders'
  cell keeps its household identity; the firm is a new named `firm` party with a new identity: A1)
  funded by an instruction founders → firm for the equity (A2: from the founders' account; the
  founders receive the firm's shares as a strategic holding: item 9's flag), and optionally a loan
  the bank decides (item 6); A4.b: the opening size is what the founders fund; A2.a: the firm buys
  its plant from a capital-goods producer (item 10's spend, with the build lag) — no endowment.
- **Balance sheet** (A3): cash and shares only, balancing by construction; A5: it enters the sector's
  markets as a competitor from its first production (item 4's decisions run for it from period one:
  B1).
- **Age** (B2): `founded: Period` is a term; every PD model (item 5/6 shape) and the ratings module
  read age and history length: a measure over a window longer than the history is `Missing` and a
  rating with a Missing volatility component is the agency's stated coarse grade for "no history"
  (data), never the best; a lender's PD for a young firm reads its short history as such.
- **Population** (E3, E4): entry and death are decisions and events; a long-run test asserts the
  population, its age distribution and its sector mix move with conditions and are never equal by
  construction.

### Parameters

`tender.acceptance` is a **term** of each bid (the acquirer's choice); `rating.noHistoryGrade`
(data, assessor). No `mergerRate`, no `birthRate`, no `synergy.*`, no screening threshold exist.

### Audit contributions

- `flows`: M&A D5 per deal from the ledger.
- `names`: a merged target resolves to its successor everywhere; a new firm's identity is new.
- `units`: the firm population is a read of parties; every founder split is a weight event with
  a cause.

### Files

```
packages/engine/src/world/context.ts, register/instruments.ts (13g.1: parties.merge)
packages/engine/src/mechanisms/corporate-control/{index.ts,intent.ts,funding.ts,tender.ts,resistance.ts,after.ts}
packages/engine/src/mechanisms/firm-birth/{index.ts,entry.ts,age.ts}
packages/engine/src/mechanisms/firms/… (owner preferences, targets seen), ratings/… (age)
packages/engine/test/{merge-door,intent,tender,competing-bids,resistance,consideration,after-merger,firm-birth,age,population}.test.ts
```

---

## Steps

- [ ] **From item 11 (Banks Capital A3, C2.a, B3)**: a bank raises EQUITY, and breaching its buffer restricts what it distributes. Item 11 built the subordinated layer and the raise that can fail (C2.b), but a bank here has no share line and no owners: who owns a bank at the seed is what Seed E1/E2 refuse to invent, and a party comes to own one by funding its entry — which is this item. Test: an issue dilutes the holders there are, a failed one leaves the bank where it was, and a bank below its own line pays nothing out

- [ ] 13g.1 Kernel: `parties.merge` reseating every row, redeeming the target's shares against settled consideration, terminating the target with a successor; change-of-control terms fire; tests (A2, A4, A5, D4, E2)
- [ ] Intent from the acquirer's own valuation at its own hurdle against the price with no threshold parameter; the funding decision among cash, loan, bond and shares with the credit market deciding; tests (B1, B3, A3, A3.a, A3.b)
- [ ] The tender market: a bid with price, consideration and an acceptance condition; every holder tenders or not from its own valuation; the premium clears; a failed bid is an event and the acquirer may return; tests (A1, B2, B2.a, C1, C2, E1)
- [ ] Competing bids in one session; management's public recommendation and insiders not tendering; tests (B4, C3)
- [ ] Consideration paid into named accounts equals what the acquirer and its lenders put up: the D5 contribution; the target's debt repaid, assumed or triggered; tests (D5, A5)
- [ ] After: summed cash flows with real changes only (separations through labour, contracts ended), leverage and credit reassessed with bonds and shares moving separately, index rebalance on exit, employees and suppliers carried over; tests (D1–D4, E3)
- [ ] C4 as an observer read; scenario test: a firm trading below a buyer's valuation attracts a bid (direction only)
- [ ] Firm birth as a founder cell's decision from sector margins and its own funding: a split, a new identity, equity from the founders' account, a loan the bank decides, plant bought with a lag, a balance sheet that balances, a competitor from period one; tests (§34 A1–A5, A2.a, A4.a, A4.b, B1)
- [ ] Age read by every pricer: history-based measures Missing over short histories; the agency's stated no-history grade; a lender's PD reads a short history as such; tests (B2)
- [ ] Population, age distribution and sector mix as reads that move; a long-run test that births and deaths are not equal by construction; tests (E3, E4)
- [ ] Observer: bids, tenders, premiums, completed deals, births; year-long run green; determinism; coverage re-marked; Equity E1–E3 closed; record entry
- [ ] Delete this file; worklist row 13g → done; commit and push

## Exit criteria

A bid can fail because the owners refused; a completed deal leaves one party with every row of two
and a payment that balances to the ledger; a firm is born only from a founder's account and buys its
plant; a newborn is priced as one.

## Guard

M&A B5, E1–E3; Firm Birth A2.a, E1–E3; Equity C2.e; Observer B2.a (the event does not move the
price; the participants do).
