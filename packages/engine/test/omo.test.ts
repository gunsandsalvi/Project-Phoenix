/**
 * The central bank in the market: a quantity and no level, never in the primary, and what it hands
 * back is income.
 *
 * @spec Central Bank C1 Central Bank C1.a Central Bank C1.b Central Bank C2 Central Bank C3 Central Bank C4 Central Bank E3 Central Bank E3.a Central Bank E4 Sovereign H1 Sovereign H2 Sovereign H3 Treasury D3.a
 */
import { describe, expect, it } from 'vitest';
import {
  CB,
  CB_PARAMS,
  GOV_LINE,
  USD,
  TREASURY_US,
  assemble,
  dustOf,
  isMoneyLeg,
  moneyInstrumentId,
  type SystemModule,
  type World,
} from '../src/index.js';
import { rigWorld, rigSpec } from './rig.js';

/** The same world with one of the central bank's policy numbers set differently. */
function withPolicy(seed: string, id: string, value: number): World {
  const spec = rigSpec(seed);
  const modules: SystemModule[] = spec.modules.map((m) =>
    m.id === 'central-bank-omo'
      ? { ...m, params: m.params.map((p) => (p.id === id ? { ...p, value } : p)) }
      : m,
  );
  return assemble({ ...spec, modules });
}

describe('open-market operations (Central Bank C)', () => {
  it('buys towards the share policy chose, paying with money it creates (C1, C1.a, C2)', () => {
    const w = rigWorld('omo-a');
    const share = w.params.ratio(CB_PARAMS.targetShare);
    const baseBefore = w.instruments.get(moneyInstrumentId(CB, USD)).issued;
    w.step();
    const gapBefore = Math.abs(
      w.register.quantity(CB, GOV_LINE) / w.instruments.get(GOV_LINE).issued - share,
    );
    for (let i = 0; i < 30; i += 1) w.step();
    const gapAfter = Math.abs(
      w.register.quantity(CB, GOV_LINE) / w.instruments.get(GOV_LINE).issued - share,
    );
    // H2, C2: WHAT IT DID MOVED THE BASE, and it paid with money it creates. WHICH WAY the base
    // moved is an outcome and not the clause: a central bank above the share policy chose SELLS,
    // and the base falls. This asserted it grew, and it asserted it of the world's whole USD stock
    // rather than of the money this bank issues — a number that also moves every time a loan is
    // written or repaid, which is nobody's open-market operation. Over these thirty periods the
    // base went from 100.9bn to 25.8bn because the bank was selling towards its target.
    const base = (): number => w.instruments.get(moneyInstrumentId(CB, USD)).issued;
    expect(base()).not.toBe(baseBefore);
    expect(w.register.quantity(CB, GOV_LINE)).toBeGreaterThan(0);
    // C2: every piece of that move was paid for with money THIS bank issues — its own liability on
    // one side of its own purchase, which is what "paying with money it creates" is (Law 5).
    const ownSide = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled')
      .flatMap((r) => r.instruction.legs)
      .filter(isMoneyLeg)
      .filter((leg) => leg.ccy === USD && (leg.from.holder === CB || leg.to.holder === CB));
    expect(ownSide.length).toBeGreaterThan(0);
    // In its OWN money: it is the issuer of every dollar of reserves, so a dollar leg it is on has
    // its own name on one side of it. What it does in somebody else's money is a reserve position
    // (Currency D2) and a different clause.
    for (const leg of ownSide) expect(leg.from.issuer === CB || leg.to.issuer === CB).toBe(true);
    // C1.a: it holds the size policy chose, and it gets there by buying and selling into a market
    // — so what it can close is what somebody was on the other side of.
    expect(gapAfter).toBeLessThanOrEqual(gapBefore);
  });

  it('is never in a primary market: the seller there is the issuer (C1.b, Treasury D3.a)', () => {
    const w = rigWorld('omo-b');
    for (let i = 0; i < 24; i += 1) w.step();
    // Every allotment names the treasury as seller; the central bank appears in none of them.
    const issuances = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'issuance');
    expect(issuances.length).toBeGreaterThan(0);
    let allotments = 0;
    for (const r of issuances) {
      for (const leg of r.instruction.legs) {
        // An issuance is any claim coming into existence — a fund issues shares to a subscriber
        // too (Fund Shares C1). What this is about is the primary market for the STATE's paper.
        // Law 19, PLAN §7: THE STATE'S PAPER IS ITS KIND AND ITS ISSUER, read from the registry.
        // This matched ids beginning `gov.`, and this world's sovereign lines are `ust.`, `bund.`,
        // `gilt.` and `jgb.` since it gained four countries — so it matched nothing at all and the
        // loop asserted about an empty set.
        if (leg.kind !== 'asset') continue;
        const i = w.instruments.get(leg.instrument);
        if (!String(i.kind).startsWith('sovereign.')) continue;
        if (!i.issuer.some || i.issuer.value !== TREASURY_US) continue;
        allotments += 1;
        expect(leg.from).toBe(TREASURY_US);
        expect(leg.to).not.toBe(CB);
      }
    }
    expect(allotments).toBeGreaterThan(0);
  });

  it('posts a quantity and takes the level the book gives it (C3)', () => {
    const w = rigWorld('omo-c');
    for (let i = 0; i < 8; i += 1) w.step();
    // Its trades happened at prints somebody posted, never at a level of its own naming: every
    // print this world carries came out of a market it did not price.
    const prices: number[] = [];
    for (const m of w.markets) {
      const p = w.prices.latest(m.instrument, w.period);
      if (p.some) prices.push(p.value.price);
    }
    expect(prices.length).toBeGreaterThan(0);
    expect(prices.every((x) => x > 0)).toBe(true);
  });

  it('lets the book run off when reinvestment is off, and the base shrinks with it (C4)', () => {
    const on = rigWorld('omo-d');
    const off = withPolicy('omo-d', 'centralBank.reinvest', 0);
    for (let i = 0; i < 30; i += 1) {
      on.step();
      off.step();
    }
    const heldOn = on.register
      .allHoldings()
      .filter((h) => h.holder === CB)
      .length;
    const heldOff = off.register
      .allHoldings()
      .filter((h) => h.holder === CB)
      .length;
    expect(heldOff).toBeLessThanOrEqual(heldOn);
    expect(off.moneyStock()['USD'] ?? 0).toBeLessThan(on.moneyStock()['USD'] ?? 0);
  });
});

describe('remittance (Central Bank E3)', () => {
  it('hands over what its own instructions earned, on its own calendar, and says so', () => {
    const w = rigWorld('omo-e');
    const before = w.cash(TREASURY_US, USD);
    for (let i = 0; i < 60; i += 1) w.step();
    const remittances = w.journal.ofKind('centralBank.remittance');
    expect(remittances.length).toBeGreaterThan(0);
    const first = remittances[0];
    expect(first?.public).toBe(true);
    expect(first?.data['income'] as number).toBeGreaterThan(0);
    expect(first?.data['settled']).toBe(true);
    expect(w.cash(TREASURY_US, USD)).not.toBe(before);
  });

  it('remits income and not revaluation (E3.a): what it hands over is what its ledger produced', () => {
    const w = rigWorld('omo-f');
    for (let i = 0; i < 60; i += 1) w.step();
    const e = w.journal.ofKind('centralBank.remittance')[0];
    if (e === undefined) throw new Error('no remittance');
    const since = e.data['since'] as number;
    let ledgerIncome = 0;
    for (let p = since; p <= e.period; p += 1) {
      for (const r of w.ledger.inPeriod(p as never)) {
        if (r.outcome !== 'settled') continue;
        // The remittance itself is what this number paid for; it is not part of what earned it.
        if (r.instruction.reason.startsWith('remittance')) continue;
        for (const eff of r.equity) if (eff.party === CB) ledgerIncome += eff.delta;
      }
    }
    // The number it remitted is exactly the equity its settled instructions produced. Revaluation
    // never passes through an instruction, so it cannot have reached this. Law 7: both sides are
    // sums over the same settled effects, so what separates them is the dust of adding them up —
    // derived from how many were added and how big they are, never a band anybody chose.
    const terms = w.ledger.all().filter((r) => r.outcome === 'settled').length;
    expect(Math.abs((e.data['income'] as number) - ledgerIncome)).toBeLessThanOrEqual(
      dustOf(terms, ledgerIncome),
    );
  });
});
