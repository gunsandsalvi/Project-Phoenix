/**
 * A payment that fell due and was not made is a row (Money E1, E1.a, E1.b, XI-8, 12a.1, 12a.2).
 *
 * @spec Money E1 Money E1.a Money E1.b XI-3 XI-8 Law 4 Law 5
 */
import { describe, expect, it } from 'vitest';
import { ARREAR, FIRM, arrearTerms, assemble, isArrear, type MechanismContext, type SystemModule } from '../src/index.js';
import { mergeModules, rigFor, rigSpec } from './rig.js';

describe('a payer that could not pay owes a row (12a.1)', () => {
  it('issues an arrear to the payee for what did not arrive, in the same pass, named by its class and its instruction', () => {
    let payer: string | undefined;
    let payee: string | undefined;
    let asked = 0;
    let failedId: number | undefined;
    const probe: SystemModule = {
      id: 'test.cannotPay',
      spec: 'Money E1',
      requires: ['firms'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.cannotPay',
          spec: 'Money E1',
          anchor: { after: 'corporateActions' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== 2) return;
            const firms = ctx.parties.ofKind(FIRM).filter((p) => p.status.alive);
            const a = firms[0];
            const b = firms[1];
            if (a === undefined || b === undefined) return;
            const ccy = ctx.registry.currencyOf(a.region);
            const cash = ctx.participant(a.id).cash(ccy);
            // E1.a: it does not silently overdraw — far beyond anything its bank would allow.
            asked = Math.floor(cash * 1000) + 1_000_000_000_000_000;
            const r = ctx.settle({
              legs: [{ kind: 'money', from: ctx.accountOf(a.id, ccy), to: ctx.accountOf(b.id, ccy), receipt: { of: 'transfer' }, ccy, amount: asked as never }],
              cause: 'transfer',
              reason: `${String(a.id)} pays ${String(b.id)} what it has not got`,
            });
            if (r.outcome === 'failed') {
              payer = String(a.id);
              payee = String(b.id);
              failedId = r.instruction.id;
            }
          },
        },
      ],
      participants: [],
      families: [],
    };
    const { banks, firms } = rigFor('arrears', { makes: ['coalRaw'] });
    const spec = rigSpec('arrears', banks, firms);
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    for (let i = 0; i < 3; i += 1) w.step();
    expect(payer).toBeDefined();
    expect(failedId).toBeDefined();
    if (payer === undefined || payee === undefined || failedId === undefined) return;
    // E1.b: the payee holds a receivable that did not arrive; the payer issued it (Law 5: two sides).
    const rows = w.instruments.all().filter((i) => isArrear(i) && arrearTerms(i).failed === failedId);
    expect(rows).toHaveLength(1);
    const row = rows[0];
    if (row === undefined) return;
    expect(row.kind).toBe(ARREAR);
    // The payer, or its estate: it failed on what it owes and the row went where its book went (Register F2).
    expect(row.issuer.some && String(w.parties.resolve(payer as never).id)).toBe(row.issuer.some ? String(row.issuer.value) : '');
    expect(arrearTerms(row).payer).toBe(payer);
    expect(arrearTerms(row).class).toBe('transfer');
    expect(arrearTerms(row).payee).toBe(payee);
    // Written in the same pass as the fail, for the whole of what did not arrive: the issuance is
    // on the ledger, its cause the default, its quantity the amount.
    const written = w.ledger
      .all()
      .find((r) => r.outcome === 'settled' && r.instruction.cause === 'default' && r.instruction.legs.some((l) => l.kind === 'asset' && l.instrument === row.id));
    expect(written).toBeDefined();
    const leg = written?.instruction.legs.find((l) => l.kind === 'asset' && l.instrument === row.id);
    expect(leg?.kind === 'asset' && leg.qty).toBe(asked);
    // The payee, or its estate if it too has since failed, holds what is still owed on it: the
    // payer's estate paid what it had against the row (XI-8), so the rest stands.
    const held = w.register.quantity(w.parties.resolve(payee as never).id, row.id);
    expect(held).toBeGreaterThan(0);
    expect(held).toBeLessThanOrEqual(asked);
    expect(row.issued).toBe(held);
    // 12a.3: it falls due every period until it is paid, and what was paid on it was a redemption
    // at par — the payer's estate paid part, which is why less than the whole still stands.
    expect(w.registry.instrumentKind(row.kind).due(row, w.period, w.calendar, w.registry)).toHaveLength(1);
    const redeemed = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'maturity' && r.instruction.legs.some((l) => l.kind === 'asset' && l.instrument === row.id));
    expect(redeemed.length).toBeGreaterThan(0);
    // XI-8: it ranks by its class, and it has no market.
    expect(w.registry.instrumentKind(row.kind).pricing).toBe('carriedAtCost');
    expect(w.registry.instrumentKind(row.kind).ranking(row).seniority).toBeGreaterThanOrEqual(0);
    // XI-3 (12a.2): what it still owes is read off the row — and a firm that cannot cover it fails.
    const party = w.parties.get(payer as never);
    expect(party.status.alive).toBe(false);
  });
});
