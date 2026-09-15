/**
 * The state fails only in a money it does not issue (Sovereign G1, G2, G3, G5, Treasury D3, 12a.6).
 *
 * @spec Sovereign G1 Sovereign G2 Sovereign G3 Sovereign G5 Treasury D3 Money E1 XI-3
 */
import { describe, expect, it } from 'vitest';
import { FIRM, TREASURY, assemble, isArrear, type MechanismContext, type SystemModule } from '../src/index.js';
import { RIG_BANKS, RIG_FIRMS, mergeModules, rigFor, rigSpec } from './rig.js';
import { COUNTRIES } from '../src/seeds/foundation.js';

/** A probe that makes the first treasury pay a firm, in a money it may or may not issue, more than it has. */
function cannotPay(inMoneyOf: 'itsOwn' | 'another', onFail: (treasury: string, ccy: string) => void): SystemModule {
  return {
    id: 'test.treasuryCannotPay',
    spec: 'Money E1',
    requires: ['treasury'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.treasuryCannotPay',
        spec: 'Money E1',
        anchor: { after: 'corporateActions' },
        reads: [],
        writes: [],
        run: (ctx: MechanismContext) => {
          if (ctx.period !== 2) return;
          const t = ctx.parties.ofKind(TREASURY).find((p) => p.status.alive);
          if (t === undefined) return;
          const own = ctx.registry.currencyOf(t.region);
          const payee = ctx.parties.ofKind(FIRM).find((p) => p.status.alive && (inMoneyOf === 'itsOwn') === (ctx.registry.currencyOf(p.region) === own));
          if (payee === undefined) return;
          const ccy = ctx.registry.currencyOf(payee.region);
          const r = ctx.settle({
            legs: [{ kind: 'money', from: ctx.accountOf(t.id, ccy), to: ctx.accountOf(payee.id, ccy), receipt: { of: 'transfer' }, ccy, amount: 1_000_000_000_000_000 as never }],
            cause: 'transfer',
            reason: `${String(t.id)} pays ${String(payee.id)} what it has not got`,
          });
          if (r.outcome === 'failed') onFail(String(t.id), String(ccy));
        },
      },
    ],
    participants: [],
    families: [],
  };
}

describe('a treasury that cannot pay (12a.6)', () => {
  it('in its own money does not pay, is not in default, opens no estate, and keeps funding itself', () => {
    let treasury: string | undefined;
    const { banks, firms } = rigFor('treasury-own', { makes: ['coalRaw'] });
    const spec = rigSpec('treasury-own', banks, firms);
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [cannotPay('itsOwn', (t) => { treasury = t; })]) });
    for (let i = 0; i < 6; i += 1) w.step();
    expect(treasury).toBeDefined();
    if (treasury === undefined) return;
    // Money E1: the row stands, issued by the state.
    const rows = [...w.instruments.issuedBy(treasury as never)].filter((i) => i.status.live && isArrear(i));
    expect(rows.length).toBeGreaterThan(0);
    // G1, Treasury D3: not a default, not a death.
    expect(w.journal.ofKind('credit.default').filter((e) => String(e.data['party']) === treasury)).toHaveLength(0);
    expect(w.journal.ofKind('treasury.defaulted').filter((e) => String(e.subjects[0]) === treasury)).toHaveLength(0);
    expect(w.journal.ofKind('estate.opened').filter((e) => String(e.data['dead']) === treasury)).toHaveLength(0);
    expect(w.parties.get(treasury as never).status.alive).toBe(true);
    // G1: a shortfall is not a default, so it is not excluded — it keeps bringing paper.
    expect(w.journal.ofKind('treasury.excluded').filter((e) => String(e.subjects[0]) === treasury)).toHaveLength(0);
    expect(w.journal.ofKind('auction.announced').filter((e) => String(e.subjects[0]) === treasury && e.period > 2).length).toBeGreaterThan(0);
  });

  it('in a money it does not issue is in default, and still has no estate', () => {
    let treasury: string | undefined;
    let ccy: string | undefined;
    const probe = cannotPay('another', (t, c) => { treasury = t; ccy = c; });
    const spec = rigSpec('treasury-abroad', RIG_BANKS * COUNTRIES.length, RIG_FIRMS * COUNTRIES.length, COUNTRIES);
    const w2 = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    for (let i = 0; i < 4; i += 1) w2.step();
    expect(treasury).toBeDefined();
    if (treasury === undefined || ccy === undefined) return;
    const rows = [...w2.instruments.issuedBy(treasury as never)].filter((i) => i.status.live && isArrear(i) && String(i.ccy) === ccy);
    expect(rows.length).toBeGreaterThan(0);
    // G2: a real default; G3: no estate; G5: out of the market while the row stands.
    expect(w2.journal.ofKind('credit.default').filter((e) => String(e.data['party']) === treasury).length).toBeGreaterThan(0);
    expect(w2.journal.ofKind('treasury.defaulted').filter((e) => String(e.subjects[0]) === treasury).length).toBeGreaterThan(0);
    const excluded = w2.journal.ofKind('treasury.excluded').filter((e) => String(e.subjects[0]) === treasury);
    expect(excluded.length).toBeGreaterThan(0);
    const since = excluded[0]?.period ?? 0;
    expect(w2.journal.ofKind('auction.announced').filter((e) => String(e.subjects[0]) === treasury && e.period >= since)).toHaveLength(0);
    expect(w2.journal.ofKind('estate.opened').filter((e) => String(e.data['dead']) === treasury)).toHaveLength(0);
    expect(w2.parties.get(treasury as never).status.alive).toBe(true);
  });
});
