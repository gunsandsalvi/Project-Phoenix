# Findings, waiting to be positioned

Everything measured during an item that is not the item's own work (CLAUDE.md: *every bug is written
down — all of them*). A finding leaves this file only by being **positioned**: moved into the
`docs/plan/<item>.md` of the item that should fix it, or inserted as its own item at its dependency
position. The file is temporary and goes when it is empty.

Open at: **13b**, the derivative classes.

---

## 13b-1 — A bank closes overdrawn at the central bank with nothing lent to it — MOSTLY CLOSED

**What was built.** A treasury funds what it knows it owes, and what it owed was not knowable: a
contract's term is a date its own terms carry, so what falls due next period IS knowable now, by the
party that owes it. `OwnContracts.cashDue(ccy, at)` is that read — the sum of what each kind says a
row will take (`DerivativeKindProfile.cashDue`, defaulting to its legs), plus what the LAYER will
ask it to post (`ClearingCapacity.dueNext`, because what a margin claim is, is the layer's
instrument and not the kernel's). A deliverable future answers with the whole face at the
deliverable's own price, which is nowhere in its legs because a payment cannot carry a bond.

The bank's own funding schedule adds it to its gap, and `world.test.ts`'s year-long run went green.

**What is left.** `deposits.test.ts` still reports it in its own trimmed world. Whether that is the
same shortfall reaching a world with fewer mechanisms to fund it in, or a second cause, is not
measured.

---

## 13b-1 (original statement, kept because the measurement is the evidence)

**Measured.** `loans.test.ts`, `estate.test.ts`, `credit-events.test.ts`, `deposits.test.ts`, each
in its own world, every period after the classes start clearing:

```
money: bank.a closes -246390697 per member on money:fed:USD with no lender row behind it
money: bank.b closes -589342714 per member on money:fed:USD with no lender row behind it
money: bank.c closes -1271863046 per member on money:fed:USD with no lender row behind it
```

**What it is.** The classes move real cash — a protection premium (CDS A2), the net of two swap legs
(IRS A1.b), variation margin (Derivative Layer D2), a delivery against payment (Sovereign I1) — and
a bank that has posted all of it ends the period short of reserves. The money family is right: a
negative balance at the central bank is a LOAN, and there is no row saying anybody made one.

**What is missing is the ANSWER TO A CALL, not a bound.** `margin.calls` runs at the top of the
period (13a put it there deliberately: a call measured against last close's marks can be met out of
this period's session). What no module does is MEET it: `D4.a` says a called party posts cash,
pledges, or sells — this world now has the second (13b built the pledge path) and the third
(XI-2 door one), and nothing goes to the money market for the first. A bank short of reserves
after a margin call borrows them; that is Money Market C and Central Bank D1, and it is a
mechanism, not a limit.

**Where it goes.** The bank's own response to a margin call, which is the banks module's and the
money market's. It is 13f's (corporate credit, short-term debt, lending and financing) unless a
closer item claims it: the response is a funding decision by a bank that has just been called.

---

## 13b-2 — A fund's equity is a millionth instead of nothing

**Measured.** `resolution/cells.test.ts`, at every grain:

```
accounts: fund.money.bank.c has equity of 8000.000001030508: a fund with equity has mislaid
somebody's money
```

**What it is.** Fund Shares A3 makes a fund's equity zero by construction: what it holds belongs to
its shareholders and its own claim on itself absorbs the difference. It is now off by 1.03e-6 on a
book of 8e3 — about 1.3e-10 relative, which is far bigger than the dust of stating 8000 and far
smaller than anything anybody did. A fund that holds a contract mark has a number in its balance
sheet that the fund's own share value is not re-derived from at the same moment.

**What it is not.** It is not a tolerance to widen (Law 7) and it is not the accounts family being
wrong. It is either the contract mark reaching the fund's assets before its claim-on-itself is
re-marked (Fund Shares B1: the two are re-marked in one pass), or a walk whose dust is charged at
the wrong magnitude (the `12d-21` shape, which 13a fixed for a stated balance and not for a
revaluation).

**Where it goes.** 13h (insurers, hedge funds, private equity), which is where a fund holds
derivative positions on purpose and where the pass that re-marks a fund's own claim is next opened.

**Held off until then.** Funds are no longer among the kinds the layer asks for reasons in a
contract book (`derivativeLayer([...TRADES_CONTRACTS])`), so no fund in this world carries a mark
its own share value has not been told about. Measured at 83,247,864 on `etf.us` the one time they
were let in. 13h lets them back in, with the read that keeps A3 true.

---

## 13b-3 — Three equity tests moved when the world got richer

**Measured.** `equity.test.ts`, after 13b's classes and the treasury's new funding read:

- `sells new shares when it is short and the market is dear, and the count rises (D1, D1.a)`
- `buys its own back only when the market is below its own book, and the cash is gone (D2, D2.b)`
- `prints near the residual the company itself published, a year in`
- `has a party with a view in it once the company has published anything`
- `gives the room to the higher earner first, and the other can get nothing (no floor)`

**What it is.** Not one thing, and not measured to a cause. What changed underneath them: banks now
borrow for what their own contracts will take (so the money market clears at different rates, and a
bank's cost of funds moves), a desk is charged the cost of the money a line is IN rather than its
own, and a desk quotes only the lines the listing drew it as a maker of — so which banks quote which
share lines is a smaller and different set than it was.

Every one of those is a change that is right on its own terms (Law 13), and a print that moves
because the world it is made in moved is a finding about the world, not a regression in the print.
What is NOT established is that these five are that, rather than one of them being a defect.

**Where it goes.** It needs measuring before it can be positioned: which of the three changes each
test turns on, one at a time. That is a measurement (Part XII) and not a mechanism, so it waits for
an item that is measuring rather than building — unless one of them turns out to be a defect, in
which case it goes to whichever item owns the mechanism it is in.
