# The sweep — where this world is not how the world works

Every system read against one question: **is this the real mechanism in the limit of the current
approximation, or is it the plan being followed?** What follows is what the reading found, with the
measurement beside each. It is a holding pen with the same rule as `docs/BUGS.md`: a finding leaves
it only by being PLACED — into a worklist item at its dependency position — and the file goes when
it is empty.

Measured on `foundationWorld('real')`, the four-country world, at periods 1–5.

---

## A. Periodicity: things this world does every week that the world does not

Law 8 says the periodicity is part of the number. A weekly period is the resolution; it is not a
licence to do everything weekly. Four things are done weekly that are not weekly.

### A1. A dividend is declared and paid EVERY WEEK

**Measured.** Period 5 settles 249,288 instructions and **162,615 of them are dividend payouts** —
65% of everything the world does. `equity.decide` runs every period and `decideEquity` distributes
`spare / patience` each time.

**What is real.** A board declares with its results, on a fiscal calendar — quarterly, semi-annual
or annual, and which of those is a fact about the place. Between the declaration and the payment the
dividend is a LIABILITY of the firm and the share trades EX: the price drops by the dividend on the
ex-date, which is the single most-observed fact about dividends and the reason total return and
price return are different numbers. None of that exists here: there is no declaration date, no ex
date, no record date, no payable date, and no dividend liability.

**The machinery is already built and unused.** `reporting/fiscal.ts` has `quarterClosedBy`,
`anchorOf` (each company's own year end) and `publishableOn` (the lag). A dividend is declared with
the results it is declared out of.

**And it is most of the cost of a tick**: 65% of the instructions in a period, so a quarterly
dividend is a world that does what it does in about a third of the settlements.

### A2. A rating fee is charged every week

**Measured.** 16,527 `pays X for its rating` instructions in period 5. `ratings.assess` runs every
period and calls `collectFees` each time.

**What is real.** An issuer pays an issue fee once, when the paper is rated, and a surveillance fee
annually. Weekly billing makes the assessor's income a flow of the issuer's equity rather than a
price for a service, which is the conflict Ratings A5 exists to keep.

### A3. Tax is levied every week

**Measured.** 7,778 `tax due from X` instructions in period 5, every period.

**What is real.** Payroll withholding is the part that moves with the wage — weekly is right for
that. Corporation tax is assessed on a fiscal period and paid on dates; VAT quarterly. Levying
everything weekly removes the working-capital consequence of a tax bill, which is the thing a
treasury and a firm both plan around.

### A4. A buyback is decided weekly

`decideEquity` chooses between a dividend and a buyback each period from this week's spare cash. A
buyback is an announced PROGRAMME executed over time, and announcing it is the event.

---

## B. Four countries, and the things there is still only one of

### B1. Four central banks administer ONE policy rate

**Measured.** `centralBank.policyRate = 0.02`, one parameter, read by `corridor(ctx)` for every
currency. The Fed, the ECB, the Bank of England and the Bank of Japan all administer 2%.

**What this costs.** With no interest differential between two moneys there is no carry, so an FX
forward prices flat to spot, covered interest parity says nothing, the cross-currency basis has
nothing to be a basis against, and the carry trade — the single largest real FX flow — cannot exist.
Four countries with identical policy is not an approximation of the world; it is the one assumption
that switches the whole currency layer off.

A rate is a POLICY primitive (Law 2) and each central bank owns its own.

---

## C. Sectors that are not there

### C1. There is no commercial real estate, and no land cost to building

Residential exists: a dwelling is a good, `housing` lets it, prices it and lends against it. Nothing
else about the built environment does.

- **Warehouses** exist only as `STORAGE` plant a firm builds for its own stock. Nobody builds space
  to LET. There is no landlord, no commercial rent, no lease with a term, no CRE lending, and
  therefore none of what makes commercial property the asset class that breaks banks.
- **Commercial buildings** — the shop, the office, the works — do not exist as assets at all. A
  retail firm in this world sells from nowhere.
- **PORTS DO NOT EXIST.** `freight` sails a vessel from region to region with no berth, no quay, no
  congestion and no owner. The word "port" appears in this codebase once, in a comment I wrote about
  which bank a carrier uses.
- **Building consumes no land and never gets dearer.** `geography.ts` already does the right thing
  for resource yield — "the return from the next unit falls continuously and never reaches zero, so
  what stops an expansion is a firm's hurdle refusing the project, and never a refusal by the map" —
  which is exactly Law 6 done properly. Nothing does it for BUILT SPACE. A tile can carry any number
  of buildings at the same cost as the first.

### C2. No firm is ever born

`estate` kills firms; nothing creates one. The draw fixes the population of firms at the seed and it
only ever falls. Entry is what makes a market contestable — without it a surviving firm's margin is
never competed away, and every concentration measure in this world is one-way. Spec 34 (FIRM BIRTH
AND DEATH) is half built; the birth half has been PLACED forward three times (13f → 13g → 13h) and
never landed.

### C3. No asset manager runs a strategy

**Measured.** 13 funds. After four periods **4 of them hold anything at all**, 8 holdings between
them, against 1,274 subscriptions settled in period 4 alone. Every fund in this world is a money
fund, a single commodity fund, or an index tracker — and the trackers hold nothing.

- **Hedge funds (spec 28) and private equity (spec 29): no module.** Neither has been built.
- **Prime brokerage (spec 15): no module** — which is what a leveraged manager finances through.
- **The basis trade is measured and nobody trades it.** `bond-futures` computes the net basis
  against carry, correctly and with no parameter; repo exists in the money market. The two ends of
  the trade are built and there is no party whose reason is to put them together.
- **No fund holds credit.** `eligible` is `['sovereign.bill']` for money funds and `['good.grain']`
  for the commodity fund. Nothing holds a corporate bond, a sovereign BOND, or a credit index.
- **The credit index has no tracker.** `CREDIT_INDEX(ccy)` is declared per currency; the ETFs
  launched are the four equity indices, two US size segments and the global — so a credit index is
  measured every period and nobody can take a position in it.

### C4. Short-term debt: a firm cannot issue commercial paper

Spec 9 has no module. `money-market` has interbank rows and repo, which is the BANK's short-term
funding; a FIRM funding itself at three months, and the roll that can fail, is the mechanism the
clause is about and it does not exist. It has been carried since 13f.

### C5. Pensions are insurers wearing the same name

Spec 27 is "INSURERS AND PENSIONS" and there is one party kind, `insurance`. A pension has a
SPONSOR, contributions from an employer and its members, and a funding ratio that is the sponsor's
problem when it falls — none of which an insurer has, and none of which exists.

### C6. Small-business pools (spec 42) were carried from 13e and never built

---

## What goes where

| finding | item | why there |
|---|---|---|
| A1–A4 periodicity | **13k** | needs only `reporting`'s fiscal calendar, which is built. Cuts a tick by ~65% as a side effect. |
| B1 four policy rates | **13l** | needs 13j's four countries, which exist. |
| C1 the built environment, CRE and ports | **13m** | needs the map (13c.1), goods, the capital programme, freight and housing — all built. |
| C2 firm birth | **13n** | needs the capital programme and equity for a listing. |
| C3 asset managers, hedge funds, prime brokerage, the basis trade, credit funds | **13o** | needs securities lending and repo, both built, and 13n's entry for PE to have anything to buy. |
| C4 short-term debt, C5 pensions, C6 small-business pools | carried | stated here so they are not lost again; each names its own dependency. |
