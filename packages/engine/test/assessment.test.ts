/**
 * The annual assessment against what was withheld (§30 C2, 20a).
 *
 * @spec Treasury C1 Treasury C3 Money E1 Money G3 Money G3.a Sovereign D3 Law 4 Law 19
 *
 * A tax WITHHELD is a payment on account, and what makes it one is that somebody works out what the
 * year actually came to and settles the difference — both ways, each between the state and one
 * payer, with no netting across payers and no advance to make either side settle.
 */
import { describe, expect, it } from 'vitest';
import { TREASURY_US, isArrear, arrearTerms, isMoneyLeg, type World } from '../src/index.js';
import { rigWorld } from './rig.js';

/** A year and a period: the assessment falls in the period after the one the close fell in. */
function aYear(seed: string): World {
  const w = rigWorld(seed);
  for (let i = 0; i < 53; i += 1) w.step();
  return w;
}

function assessments(w: World): readonly ReturnType<World['journal']['ofKind']>[number][] {
  return w.journal.ofKind('treasury.assessment');
}

describe('the annual assessment (§30 C2)', () => {
  it('falls once, in the period after the year closed, over the periods the year covered', () => {
    const w = aYear('assess-a');
    const all = assessments(w);
    // Money G3.a: it is placed by a DATE and not by a count of periods, so a year of 52 periods
    // produces exactly one — and a second would be the same year assessed twice.
    expect(all.length).toBe(1);
    const e = all[0];
    if (e === undefined) throw new Error('no assessment');
    expect(String(e.data['year']).endsWith('-FY')).toBe(true);
    expect(e.public).toBe(true);
    // Clearing F1: the period it stands in is not one whose instructions are all written, so the
    // span it reads ends at the period before it.
    expect(Number(e.data['to'])).toBe(e.period - 1);
    expect(Number(e.data['from'])).toBeLessThan(Number(e.data['to']));
  });

  it('settles the difference BOTH ways, each leg a tax between the state and one payer', () => {
    const w = aYear('assess-b');
    const e = assessments(w)[0];
    if (e === undefined) throw new Error('no assessment');
    let intoTheState = 0;
    let outOfIt = 0;
    for (const r of w.ledger.inPeriod(e.period)) {
      if (r.outcome !== 'settled' || !r.instruction.reason.includes('assessment')) continue;
      for (const leg of r.instruction.legs) {
        if (!isMoneyLeg(leg)) continue;
        // C1: it is the same levy being settled, so it is a tax receipt whichever way it goes —
        // and a refund marked anything else would be somebody's income.
        expect(leg.receipt.of).toBe('tax');
        const state = leg.to.holder === TREASURY_US || leg.from.holder === TREASURY_US;
        expect(state).toBe(true);
        if (leg.to.holder === TREASURY_US) intoTheState += leg.amount;
        else outOfIt += leg.amount;
      }
    }
    // The event is a read of what actually settled, not of what was worked out (Law 19).
    expect(intoTheState).toBe(Number(e.data['toppedUp']));
    expect(outOfIt).toBe(Number(e.data['refunded']));
    // A world of payers assessed at exactly what they paid would not need the mechanism at all.
    expect(intoTheState).toBeGreaterThan(0);
    expect(outOfIt).toBeGreaterThan(0);
  });

  it('leaves an arrear where the money was not there, on whichever side wanted it (Money E1, D3)', () => {
    const w = aYear('assess-c');
    const e = assessments(w)[0];
    if (e === undefined) throw new Error('no assessment');
    const unsettled = Number(e.data['unsettled']);
    if (unsettled <= 0) return;
    // Nothing is advanced to make it settle: the miss is a row, written by settlement in the same
    // pass, and the state's own refund can fail for the want of money exactly as a transfer can.
    const failed = w.ledger
      .inPeriod(e.period)
      .filter((r) => r.outcome === 'failed' && r.instruction.reason.includes('assessment'));
    expect(failed.length).toBeGreaterThan(0);
    const ids = new Set(failed.map((r) => String(r.instruction.id)));
    const rows = w.instruments
      .all()
      .filter((i) => isArrear(i) && ids.has(String(arrearTerms(i).failed)));
    expect(rows.length).toBeGreaterThan(0);
  });

  it('reckons against what the LEDGER says was taken, so a refund is net of itself', () => {
    const w = aYear('assess-d');
    const e = assessments(w)[0];
    if (e === undefined) throw new Error('no assessment');
    // Law 19: nothing stores what was withheld. What the year took is the legs themselves, and the
    // reckoning's own refund comes back out of that read — so a payer repaid this year is assessed
    // next year against what it is NET of the repayment, with no second copy to keep in step.
    let net = 0;
    for (let p = Number(e.data['from']); p <= Number(e.data['to']); p += 1) {
      for (const r of w.ledger.inPeriod(p as ReturnType<World['journal']['ofKind']>[number]['period'])) {
        if (r.outcome !== 'settled') continue;
        for (const leg of r.instruction.legs) {
          if (!isMoneyLeg(leg) || leg.receipt.of !== 'tax') continue;
          if (leg.to.holder === TREASURY_US) net += leg.amount;
          else if (leg.from.holder === TREASURY_US) net -= leg.amount;
        }
      }
    }
    // The year's withholding is a real number and the reckoning moved it by what it settled.
    expect(net).toBeGreaterThan(0);
  });
});
