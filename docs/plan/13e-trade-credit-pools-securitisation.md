# Item 13e — Trade credit, small-business pools, securitisation

**Objective.** Trade credit as the receivable and the payable that are one obligation from two
sides, with terms that carry a price, lateness as a real state, a claim in the estate on failure,
and withdrawal of terms as a credit tightening no bank is involved in. Small businesses as cells with
weights, their own loan rows with named lenders, their own trade credit, promotion across the size
boundary, and defaults evaluated per cell. Securitisation as risk transfer with a transferee: a
vehicle that holds loan rows, tranches with stated attachment and cleared prices, a waterfall
allocating real losses to real tranches, named holders. Mortgage pools from 13d use the same vehicle.

**Read first.** §36 Trade Credit (all); §42 Small-Business Pools (all); XI-11; XI-15 (weights, five
events); Firm Birth D2.b (trade creditors rank); Housing C6; Banks Lending D4 (a loan can be sold);
Money Market B3.a (eligible collateral: senior tranches); Goods C (the invoice book from item 4:
receivables as the sum of invoices). Code: item 4's `firms.invoice` and `households.pay` (payments
on terms exist from item 4 with a single stated term), item 5's lateness event, item 6's loan rows,
item 7's estate waterfall, 13d's mortgage rows.

**Clauses this item meets.** Trade Credit A1–A4, B1–B5, C1, C1.a, C2, C3, C4, D1, D2, D2.a, D3,
D3.a, D4, D4.a, D5, E1–E3; Small-Business Pools A1, A2, A2.a, A3, A4, A5, A5.a, A6, A6.a, A6.b,
A6.c, B1–B4, B4.a, C1, C2, C2.a, C3, C4, C4.a, C5, C6, D1–D4, D4.a, E1–E6; XI-11; Housing C6;
Banks Lending D4 (sale); Firm Birth D2.b (fully).

---

## Design

### Module `trade-credit` (extends item 4's invoice book)

- `requires: ['firms', 'goods', 'credit-events', 'estate', 'bank-lending']`.
- **The obligation** (A1, A2): item 4's invoice is already the one row read from two sides (the
  seller's receivable, the buyer's payable: one writer, the invoice book in the kernel's ledger as an
  `Instruction` scheduled at its due date). This item adds **terms per (seller, buyer)** (A3): `{
days, earlyDiscount, earlyDays }` chosen by the seller's participant per buyer (B5) from the
  seller's own view of the buyer (its PD model of that buyer: the same shape as a bank's, item 5/6,
  instantiated per seller: one PD model per (assessor, borrower); the seller's assessment is an
  opinion held by it: Corporate Credit A4); the early-payment discount is an implicit rate and a
  price (A3): the buyer's `decide` pays early when its own cost of funds exceeds the implied rate,
  and that is what makes the terms a market.
- **Why** (B): the buyer needs to sell what it bought before paying (B1: its cash plan reads the
  terms); the seller competes on terms (B2: a seller whose sales are rationed offers longer terms);
  the seller enforces by **stopping shipment** (B3: its participant refuses the next order from a
  late payer); trade credit first, bank second (B4: the buyer's funding order in its cash plan).
- **Lateness** (D1): an invoice unpaid at its due date is `late` (item 5's missed-payment state
  applied to an invoice, journaled), stressing the seller's cash (its receivables ageing is a read);
  a late buyer's terms tighten (D4: the seller's participant reads its own ageing per buyer; D4.a: a
  solvent firm starved of terms can die of a rumour: emergent).
- **Failure** (D2, E3): the buyer's estate opens (item 7); the receivable becomes a claim ranking with
  unsecured creditors (Firm Birth D2.b: both ways: the estate collects the dead firm's receivables as
  an asset **and** its payables rank); the seller's loss is booked on the estate's close (D2.a) and
  can push the seller into distress (D3: emergent; D3.a: traceable firm to firm through the journal).
- **Financing** (C2): a receivable can be pledged (a lien on the invoice row as collateral for a loan
  from item 6), factored (sold to a bank at a discount: an instruction transferring the invoice row's
  creditor with a money leg: the kernel's `reseat` door from item 7 generalised to invoice rows), or
  securitised (below).
- **C4** is the flows family's existing invoice check (receivables = payables exactly by
  construction: one row); reported per period.

### Module `small-business` (a cell profile for firms)

- `requires: ['firms', 'labour', 'trade-credit', 'bank-lending']`.
- **Cells** (A6): a `smallFirm` party kind with `representation: 'cell'` (item 2's cells; XI-15):
  key dimensions declared in the registry: `(region, sector, bank)` (A6.a: its region and its bank
  are dimensions; its lender is a loan row per (lender, cell); its customers and suppliers are invoice
  rows per counterparty); per-member state: cash, inventory, headcount, leverage, coverage (A3: the
  distribution is the members); weight is a count (E5); the five events only (entry with a cause:
  Firm Birth A4.a's conditions read per sector; death by item 7; promotion; split; merge).
- **Firm profile reuse** (A1): the `firms` module's decisions run per member through `integrate(f)`
  (no representative firm: A2.a: a mean-preserving spread of member cash causes defaults among the
  members below the threshold, and the test says so); they sell to and buy from named firms and other
  cells on trade credit (A4, D5: the tier that lives on it).
- **Loans** (B): each cell borrows from its bank on a loan row per (lender, cell) with per-member
  split events when only some members are refused or default (B3: default per member; the cell
  splits: the defaulted members become a cell in item 7's estate path); secured on the firm's assets
  or the owner's dwelling (B2: a lien on the household cell's dwelling; the cross-sector link); B4's
  correlation is emergent from shared rates, demand and region; E6: no loss struck against the pool.
- **Promotion** (A6.c): a member whose size crosses the bond-market threshold (a read: the smallest
  issue a desk will underwrite, from item 9/13f's desks' limits: not a parameter) is promoted: a
  split of that member into a named `firm` party with its rows reseated (a weight event with cause).
- **A5.a** (tightening bites here first) is emergent: banks' credit decisions per cell read the same
  cost of funds and capital headroom as for named firms.

### Module `securitisation`

- `requires: ['bank-lending', 'small-business', 'housing', 'estate', 'money-market']`.
- **Vehicle** (C1, XI-11): party kind `vehicle`, named; holds loan rows transferred from the
  originator by instructions (Banks Lending D4: the sale: the row's creditor is reseated to the
  vehicle against cash; the originator's capital is freed because the row left its book: D2 as an
  outcome of item 11's risk-weight read); it has no employees and no decisions beyond its waterfall.
- **Tranches** (C2): instrument kinds `tranche` with terms `{ vehicle, attachment, detachment,
rank }` (C2.a: stated boundaries), `pricing: 'cleared'` (C3: each tranche has a market; the yield
  is derived), `liabilityOfIssuer: true` (the vehicle's liability; its equity is zero by
  construction like a fund); holders are named (C4: funds, insurers, banks; C4.a: the originator
  often keeps the bottom tranche: its participant's decision).
- **Cash flows** (C5): the loans' interest and principal arrive at the vehicle (the kernel's
  corporate actions pay the row's creditor); the vehicle's phase `vehicle.distribute` (cycle 0 after
  `corporateActions`) pays the tranches by seniority by instructions.
- **Losses** (C2, C6): a loan row's write-off (item 5) inside the vehicle reduces the pool's face; the
  waterfall allocates the loss to the tranches from the bottom by their attachments: a write-down of
  the tranche's `issued` face and of every holder's lot pro rata (a `revalue`-phase write-down through
  the tranche profile: XI-1: a loss is an event on a date); C6: Σ tranche face = pool face after every
  event, exactly (audit contribution); D4: senior losses when correlation exceeds what the attachment
  assumed are emergent (D4.a).
- **Collateral** (D3): the senior tranche is eligible collateral in 11's repo market (data:
  `repo.eligible.tranche.senior`).
- **Mortgage pools** (Housing C6): 13d's mortgage rows use the same vehicle; the foreclosure path
  runs with the vehicle as lender of record (its waterfall's servicer decision is the originator's,
  under a servicing agreement: a fee row).

### The covered bond (securitisation's on-balance-sheet sibling)

Securitisation moves loans **off** the balance sheet into a vehicle with tranches and attachments.
The on-balance-sheet alternative does not exist anywhere in this world: a bank's bond **secured on a
ring-fenced pool of its own loans**, where the pool is replenished to hold a cover ratio and the
holder has recourse **both** to the pool and to the bank. `Banks Funding A2` names "paper it issues"
with no secured variety, and `XI-8`'s waterfall has no dual-recourse claim to rank. It is the
instrument a bank reaches for when unsecured funding closes — the one channel that survives a
downgrade, which is exactly the state `Banks Funding` and `Money Market` are built to reach — and
without it a bank's only answers to a funding squeeze are repo against sovereign collateral and the
central bank. It is built in this module because it is made of the same parts: named loan rows, a
pool, a ranking, and holders.

- **Kind** `coveredBond` (owned by `securitisation`): terms `{ issuer (a bank), coupon, maturity,
pool: PoolId, coverRatio }`; `liabilityOfIssuer: true`, `pricing: 'cleared'`, ranking declared so
  that the estate and the resolution read it off the instrument like every other claim (Law 15).
- **The pool is liens, not a transfer** (`Register D`): the bank still owns the loan rows and still
  collects on them; each is pledged, pledgor the bank and beneficiary the bond, so the units leave
  the free balance and cannot be pledged twice or sold (`D5.a`). A **substitution** is a release then
  a pledge in one instruction; a release that would take the pool below the cover ratio is **refused
  at the call** (the instruction fails, the row is never written), not corrected afterwards — the
  cover ratio is a **term of the instrument**, not a bound on a number (Law 6).
- **Dual recourse in one waterfall** (`XI-8`): the holder's claim ranks on the pool first, and what
  the pool does not cover ranks **unsecured against the bank** in the same ranking as everything
  else. The estate reads both from the instrument; there is no special case anywhere.
- **Why a bank issues one**: its funding participant compares the covered quote against unsecured
  paper and repo at its own cost of funds (item 11's reads), and the gap is what its own name is
  worth unsecured — so a downgraded bank's issuance shifts into the secured channel as a consequence
  rather than as a rule.

### Parameters

`coveredBond.coverRatio` is a **term of each issue** (the bank's own commitment at issuance), not a
parameter and not a bound; `tradeCredit.terms.*` are **decisions**, not parameters; `smallFirm.key` (data: dimensions),
`smallFirm.promotionThreshold` does not exist (a read of the desks' smallest underwriting);
`tranche.attachments.<deal>` (data per deal, stated by the arranger's decision); `repo.eligible.
tranche.senior` (policy). No `lossRate`, no `poolPopulation`, no `defaultCorrelation` exist.

### Audit contributions

- `flows`: Trade Credit C4 (receivables = payables, per period, exactly); vehicle cash in = cash out
  - retained, per period.
- `ownership`: Securitisation C6: tranche faces sum to the pool's face after every loss; every tranche
  has holders (E2); a covered pool's pledged face over its bond's face is at or above the cover ratio
  every period, reported as a READ of the liens (never enforced by adjusting anything).
- `units`: Σ smallFirm weights per key is the sector population; changes only by the five events.
- `names`: every receivable has a live payer or a claim in an estate (E2, E3); every vehicle's rows
  name borrowers (E1).

### Files

```
packages/engine/src/mechanisms/trade-credit/{index.ts,terms.ts,lateness.ts,factoring.ts}
packages/engine/src/mechanisms/small-business/{index.ts,profile.ts,promotion.ts}
packages/engine/src/mechanisms/securitisation/{index.ts,vehicle.ts,tranche.ts,waterfall.ts,transfer.ts,covered.ts}
packages/engine/src/register/instruments.ts (reseat for invoice rows)
packages/engine/src/mechanisms/estate/waterfall.ts (trade creditors both ways)
packages/engine/test/{trade-terms,lateness,shipment-stop,factoring,supply-chain-contagion,small-firms,cell-default,promotion,vehicle,tranches,tranche-loss,mortgage-pool,covered-bond}.test.ts
```

---

## Steps

- [ ] Terms per (seller, buyer) as the seller's decision from its own PD of the buyer; the early discount as an implied rate the buyer's cash plan compares to its cost of funds; tests (A3, B1, B2, B5)
- [ ] Enforcement by stopping shipment; trade credit before bank credit in the buyer's funding order; tests (B3, B4)
- [ ] Lateness as a recorded state on the invoice; ageing as a read; terms tighten on the seller's own ageing; tests (D1, D4, D4.a as a scenario)
- [ ] Failure: the receivable ranks in the estate; the estate collects the dead firm's receivables and its payables rank; the seller's loss on close; tests (D2, D2.a, E3, Firm Birth D2.b)
- [ ] Supply-chain contagion traceable firm to firm in a scenario test (D3, D3.a)
- [ ] Financing a receivable: pledge (lien), factoring (reseat with a money leg); tests (C2)
- [ ] Trade Credit C4 and names contributions; observer: receivables ageing, terms per pair, late payers
- [ ] `smallFirm` cell kind with declared key dimensions and per-member state; the firms profile runs per member through `integrate`; test: a mean-preserving spread of member cash causes defaults (A2.a, A6, A6.a)
- [ ] Small-firm loans per (lender, cell) with member-level default and split; security on assets or the owner's dwelling; correlation emergent; tests (B1–B4, E6)
- [ ] Promotion across the size boundary as a split into a named firm with rows reseated; entry with a cause per sector; population as a read; tests (A6.b, A6.c, E4, E5)
- [ ] `vehicle` party kind; loan rows transferred by sale; the originator's risk weight falls because the row left; tests (C1, D1, D2, XI-11)
- [ ] `tranche` kind with stated attachments, cleared markets, named holders; the originator keeping the bottom as its decision; tests (C2–C4, E2)
- [ ] Vehicle distribution by seniority from the loans' actual cash; tests (C5)
- [ ] Loss allocation: a write-off in the pool writes down tranches from the bottom by attachment on a date; C6 contribution; senior losses emergent from correlation in a scenario; tests (C2.a, C6, D4, D4.a, E1)
- [ ] Senior tranches as repo collateral; mortgage pools through the same vehicle with the foreclosure path; tests (D3, Housing C6)
- [ ] `coveredBond`: a pool of liens on loan rows the bank still owns and still collects on; a substitution is a release and a pledge in one instruction and a release that breaks the cover ratio is refused at the call; the ratio is a term of the issue and not a bound; tests (Register D, D5.a, Banks Funding A2, Law 6)
- [ ] Dual recourse in the one waterfall: the pool first, then unsecured against the bank in the same ranking as every other claim, read off the instrument with no special case; a bank whose unsecured quote has widened issues secured as a consequence; tests (XI-8, Banks Funding, Law 15)
- [ ] Observer: pools, tranches, attachments, losses, holders; year-long run green with a regional shock scenario; determinism
- [ ] Coverage re-marked; record entry
- [ ] Delete this file; worklist row 13e → done; commit and push

## Exit criteria

A receivable is one row read from two sides and can be late, factored or lost; a supplier can starve
a firm of working capital without a bank; a small-firm cell defaults member by member and can be
promoted; a tranche takes a real loss on a date and its holders are named; a bank's capital is freed
only because a row left its book.

## Guard

Trade Credit E1–E3; Small-Business Pools A2.a, E1–E6; XI-11 (no risk transfer without a
transferee); Housing E3 (no mortgage without a balance sheet behind it, the vehicle's included);
Law 6 (a cover ratio is a promise the issuer made, enforced by refusing the release — never a clamp
on a number afterwards); Register D5.a (no unit pledged twice).
