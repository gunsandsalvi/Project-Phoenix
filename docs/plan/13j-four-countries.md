# Item 13j — The other three countries are economies

**Objective.** This world has four countries and one of them exists. The other three are a central
bank, a treasury and one line of paper each: no firms, no households, no labour market, no goods, no
banks, no shares. Every mechanism that crosses a border therefore has one real side and three
stubs — the external accounts 13i built measure a country whose only transaction is a reserve
manager's coupon, the currency layer's "two parties with opposite reasons" is one bank against one
sovereign, and a spread between two countries is a spread against a place where nothing is produced.
This item gives each of the four the same construction the first one has, from the same draw.

**Read first.** §Currency A1–A3, D2, E; §Cross-Border E; XI-12; §Seed B1, B1.a, B3, B4, C3, C4, E1,
E2; §Sovereign D3.a, D4; Law 2 (what may be declared), Law 4 (one draw), Law 15 (no kind branch),
Law 18 (this changes the world, so no digest holds). Code: `seeds/foundation.ts` (the whole of it),
`seeds/map.ts` (the ground is already drawn per country), `mechanisms/*/data.ts` (the draws).

**Clauses this item meets.** Currency A1, A2, A3, D2; Cross-Border E1, E2, E3 (with two real sides);
Seed B1, B1.a, B3, B4; Sovereign D3.a, D4.

---

## The finding this item carries

### Three of four countries are stubs, and the seed says so

`seeds/foundation.ts` on `ABROAD`: "They are SMALLER PLACES and they say so: a central bank, a
treasury and one line of paper each, with no firms, no households and no labour market, because a
real economy abroad is 13i's cross-border item and not this one." 13i closed with the external
accounts BUILT and the economies not: it built the walk, which is the right half to build first,
because a walk over a border is meaningless until there is something on the other side of it.

Measured: `foundationWorld('real')` opens with **9,225 parties, and 6 of them are abroad** — three
central banks, three treasuries. Its period-1 ledger is 50,825 instructions and the external accounts
of every foreign country are one coupon.

---

## Design

**Nothing is drawn per country.** There is one draw of banks, one of firms, one of listings, one of
funds — the world's — and what this item adds is WHERE each drawn party books. That keeps one writer
per fact (Law 4), keeps every id unique without a country in it, and means a world of four countries
and a world of one are the same draw read two ways. It is also what the map already does: the ground
is drawn for four countries and `placeFirms` then places each firm on it — it just filters to the
home country's places first.

**A country's size is an outcome, not a number.** Nothing here states how big a country is. The map
draws the ground; a firm is placed along it (`placeFirms`, already); people live where the ground
will feed them; a bank is where its depositors are. So the population splits over countries by what
the draw gave each of them, through the same `splitOnTick` that already splits depositors over banks
by size — and a world whose map came out with a bigger Europe has a bigger Europe.

**A party's money is its region's.** `registry.currencyOf(region)` is the one writer of that already.
Every place the seed says `USD` for a party that could be abroad reads it instead.

### Steps

- [x] 1. `Country` — one row per country carrying what `ABROAD` carries plus what the US has
      hard-coded (`HOME`, `REGION`, `CB`, `TREASURY_US`, `USD`, `PIP`, `ust`). Four rows, the US
      first. Every read of `ABROAD` becomes a read of the rows other than the home one, and every
      read of `REGION`/`CB`/`USD` in the seed becomes a read of a row.
- [x] 2. The map draws places for every country (`HOME_PLACES`, `ABROAD_PLACES` become one number:
      a country has as many places as the map gives it).
- [x] 3. `placeFirms` places along the whole ground, not the home country's: the weights are per
      region as now, over every region there is. `peopleIn` reads where the cells actually are.
- [x] 4. Banks book per country: the drawn banks split over countries in proportion to the people
      there, and a bank's own bank is its country's central bank.
- [x] 5. Money instruments: a central bank issues its country's money; a bank issues the money of
      the region it books in.
- [x] 6. Households: cells per (region, cohort, bank) for every region, weights splitting the
      world's population over the countries by ground and then over banks by size.
- [x] 7. Probate offices per (region, bank), which already reads a region.
- [x] 8. Goods: the lines open where firms settled, which `settled(placed)` already answers once
      step 3 spreads them.
- [x] 9. The firm endowments walk every settled region rather than `REGION`.
- [x] 10. Sovereign paper: every treasury issues the profile its own country's people and banks
      hold, in its own money, on its own curve — today the US issues the profile and the three
      others one line each.
- [x] 11. Equity: a listing is in its firm's own money and its line opens on a bank in its own
      country.
- [x] 12. Funds, ETFs, carriers, merchants, assessors: placed the same way.
- [x] 13. The indices, index futures and curves already take a list; they take four real ones.
- [ ] 14. Re-measure: parties, instructions, the external accounts of each country, and the period
      cost at the new scale.

## Tests

- Every country has firms, households, banks, a labour market and goods markets, and no country's
  parties are banked outside it.
- Every party's money is its region's, and no party holds an account in a money nobody in its
  country issues except through a market trade.
- The external accounts of the four countries sum to zero across the world, each with two real
  sides rather than one.
- A world of one country (the rig) is unchanged: the same draw read with one row.

## Exit criteria

`npm run check` green but for the reds already in `docs/BUGS.md`; every country's own audit families
report; the world's period cost and party count recorded in `docs/RECORD.md`.

## Guard

A country's size is never a declared number. If a step needs one, the mechanism that produces it is
missing and the step is a finding, not a parameter.
