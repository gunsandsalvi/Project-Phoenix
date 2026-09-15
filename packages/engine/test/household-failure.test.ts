/**
 * A household that cannot pay fails (XI-3, Households F1.b, Corporate Credit E5, 12a.5).
 *
 * @spec XI-3 Households F1.b Households C1.d Corporate Credit E5 Money E1 Register F2
 */
import { describe, expect, it } from 'vitest';
import { HOUSEHOLD, assemble, type MechanismContext, type SystemModule } from '../src/index.js';
import { mergeModules, rigFor, rigSpec } from './rig.js';

describe('a household cell that cannot pay what fell due fails into probate with a defaulted record (12a.5)', () => {
  it('owes an arrear it cannot cover, fails on cash, and its people go to probate marked defaulted', () => {
    let cell: string | undefined;
    const probe: SystemModule = {
      id: 'test.hhCannotPay',
      spec: 'Money E1',
      requires: ['households'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.hhCannotPay',
          spec: 'Money E1',
          anchor: { after: 'corporateActions' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            if (ctx.period !== 2) return;
            const cells = ctx.parties.ofKind(HOUSEHOLD).filter((p) => p.representation === 'cell' && p.status.alive && p.key['estate'] === 'living');
            const a = cells[0];
            const b = cells[1];
            if (a === undefined || b === undefined) return;
            const ccy = ctx.registry.currencyOf(a.region);
            const r = ctx.settle({
              legs: [{ kind: 'money', from: ctx.accountOf(a.id, ccy), to: ctx.accountOf(b.id, ccy), receipt: { of: 'transfer' }, ccy, amount: 1_000_000_000_000_000 as never }],
              cause: 'transfer',
              reason: `${String(a.id)} pays ${String(b.id)} what it has not got`,
            });
            if (r.outcome === 'failed') cell = String(a.id);
          },
        },
      ],
      participants: [],
      families: [],
    };
    const { banks, firms } = rigFor('hh-fails', { makes: ['coalRaw'] });
    const spec = rigSpec('hh-fails', banks, firms);
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    for (let i = 0; i < 4; i += 1) w.step();
    expect(cell).toBeDefined();
    if (cell === undefined) return;
    // Register F2: the payer as it is now — it may have moved bank since and merged onto the standing
    // cell of its new key, which then owes the arrear (the row was reseated) and is the one that fails.
    const now = String(w.parties.resolve(cell as never).id);
    const sameLine = (who: string): boolean => String(w.parties.resolve(who as never).id) === now;
    // XI-3: it failed, this module said so with the reason, and its people went to probate.
    const failed = w.journal.ofKind('households.lifecycle').find((e) => e.data['event'] === 'failed' && sameLine(String(e.subjects[0])));
    expect(failed).toBeDefined();
    expect(String(failed?.data['why'])).toContain('could not pay');
    const estate = w.parties.resolve(cell as never);
    expect(estate.representation === 'cell' && estate.key['estate']).toBe('probate');
    // E5: and the record follows them — a lender reads it off the key of the cell they are in now.
    // A whole cell that fails moves to the probate key in place (no weight changed, so no weight
    // event); the failed event above is the record of the move, and the key is its outcome.
    expect(estate.representation === 'cell' && estate.key['credit']).toBe('defaulted');
  });
});
