# Item 12b — The balance sheet that takes one step out of true

**Read this part first — it is why the item exists and what has already been ruled out.** Item 12
found it and did not chase it (Law 11), but it measured it far enough to be placed rather than
guessed at. The whole of what is known is in `docs/BUGS.md` **12-18** and is repeated here so this
file stands on its own.

**The finding.** Reproduced with `foundationSpec('year', drawBanks(4, 'year'), drawFirms(40, 'year'))`.
Every period to 12 the `accounts` family is green. At period 13 all four banks report at once, and
the size then never changes again for the forty periods after it:

```
bank.a: assets 15901695062647.4 - liabilities 13493808450681 != equity account 2407881467552.351
        a gap of 5,144,414.06 per member on a book of 1.6e13 — three parts in ten million
bank.b 42,621,128.06    bank.c 114,506,019.90    bank.d 76,298,831.49
```

**What is already ruled out.**

1. **The equity side is fully explained.** Over period 13 `bank.a`'s account moved
   −968,138,613,902.62, and the settled instructions plus the revaluation events account for exactly
   that: ledger +96,945,101,314.52 (corporateAction +104.4e9, coupon −3.4e9, trade −2.8e9, transfer
   −1.2e9) and revaluation −1,065,083,715,217.14. Nothing moved that account without an event.
2. **So the gap is on the READ side**: what the register and the price store say the book is worth
   changed by 5,144,414.06 more than every event that touched it.
3. **It is not the foreign half.** The first guess was the foreign coupon date. The holdings that
   moved most over the period are repo rows and equity lines in the bank's own money, and the
   foreign positions move by amounts the revaluation books. The guess does not survive the
   measurement.
4. **It is not accumulating dust.** It is a STEP: one period, then carried unchanged. Dust would go
   on growing, and the family's own derived tolerance (Law 7) is nowhere near it.

**Where it lands and why.** After 12a. 12a builds the published income statement — a read of a
party's own books, income decomposed, the balance sheet at its close, the cash that moved. That
decomposition is the one artefact that has to reconcile what an issuer recognised against what it
was paid, line by line, and it will name the event. Doing it before 12a means bisecting forty periods
of a four-country world by hand against an identity that only says the two sides differ.

**Objective.** `assets − liabilities == equity account` for every party, every period, with the
tolerance being what the arithmetic did and nothing else (Law 7). One cause, one fix, and the fix
removes code (Law 12).

**Read first.** §4 Audit B5, B5.a, B5.b; §6 Currency D2, D2.a, D2.b; §1 Money D2, D3; §2 Register
B3, C3.b, E1, E2; Law 5, Law 7, Law 12, Law 19. Code: `audit/families/accounts.ts`,
`world/revalue.ts`, `ledger/settlement.ts` (the equity effects an instruction books),
`prices/value.ts` (`valueOfLots`, `carryingPerUnit`).

## Steps

- [ ] Reproduce against 12a's income statement: the period the gap opens, decomposed by line, for
      one bank. The event whose read-side effect and equity-side effect differ is named by the
      decomposition or the decomposition is not yet a read of the books
- [ ] Fix the cause. A lot valued one way by `valueOfLots` and another by what settlement booked for
      it is one writer too many (Law 4): name the read that replaces the other
- [ ] The audit family's dust stays derived and is never widened (Law 7). A check that only passes
      with a band is reporting this defect a second time
- [ ] Test: the identity holds for every party over a year in four currencies, including the period
      it used to break in
- [ ] `docs/BUGS.md` 12-18 removed by being FIXED rather than by being re-positioned
- [ ] Coverage re-marked; record entry; delete this file; worklist row 12b → done
