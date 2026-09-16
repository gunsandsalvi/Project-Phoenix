/**
 * The claim: what the weather takes from a covered party is paid by the insurer whose cover it
 * holds, against the cover, in one instruction (Insurers A4, B4, Law 5, 14.3).
 *
 * @spec Insurers A4 Insurers A4.a Insurers B4 Law 5 Law 19
 */
import { describe, expect, it } from 'vitest';
import { FIRM, assemble, type MechanismContext, type SystemModule } from '../src/index.js';
import { INSURANCE, isPolicy, runCover } from '../src/mechanisms/insurers/index.js';
import { vintagesHeld } from '../src/registry/physical.js';
import { asPerPiece } from '../src/core/measure.js';
import { asQty } from '../src/core/tick.js';
import { mergeModules, rigFor, rigSpec } from './rig.js';

describe('a loss on covered plant is a claim on the insurer (14.3)', () => {
  it('pays the holder what the lost units were on its books at, up to the cover it holds, and takes the cover back in the same instruction', () => {
    let firm: string | undefined;
    let insurer: string | undefined;
    let atCost = 0;
    let coverBought = 0;
    const probe: SystemModule = {
      id: 'test.storm',
      spec: 'Insurers A4',
      requires: ['insurers', 'firms', 'capital-programme'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.storm',
          spec: 'Insurers A4',
          anchor: { before: 'insurers.claims' },
          reads: [],
          writes: [{ kind: 'event', name: 'capital.weathered' }],
          run: (ctx: MechanismContext) => {
            if (ctx.period === 2) {
              // The buy side, written by hand at a price: the insurer's own quote is 14.4's.
              const ins = ctx.parties.ofKind(INSURANCE).find((p) => p.status.alive);
              const f = ctx.parties.ofKind(FIRM).find((p) => p.status.alive && vintagesHeld(ctx.participant(p.id), ctx.calendar.startOf(ctx.period)).length > 0);
              if (ins === undefined || f === undefined) return;
              insurer = String(ins.id);
              firm = String(f.id);
              const ccy = ctx.registry.currencyOf(f.region);
              coverBought = 1_000_000;
              runCover(ctx, ccy, [
                { party: ins.id, side: 'sell', price: asPerPiece(0.01, 'what it will write cover at'), qty: asQty(coverBought) },
                { party: f.id, side: 'buy', price: asPerPiece(0.02, 'what it will pay'), qty: asQty(coverBought) },
              ]);
            }
            if (ctx.period === 3 && firm !== undefined) {
              // The storm: the capital programme's own event, written by the test so the claim has a
              // loss to be on this period rather than whenever the draw next blows hard enough.
              const view = ctx.participant(firm as never);
              const v = vintagesHeld(view, ctx.calendar.startOf(ctx.period))[0];
              if (v === undefined) return;
              const lost = asQty(v.units < 10 ? v.units : 10, 'what the storm takes');
              atCost = lost * v.basisPerUnit;
              const r = ctx.settle({
                legs: [{ kind: 'destroy', party: firm as never, instrument: v.instrument as never, qty: lost, why: 'scrapped' }],
                cause: 'corporateAction',
                reason: `${firm} lost ${String(lost)} of ${v.instrument} to the weather`,
              });
              if (r.outcome !== 'settled') return;
              ctx.record('capital.weathered', [firm, v.instrument], { holder: firm, vintage: v.instrument, capitalKind: v.capitalKind, units: lost, wind: 4, survived: 0.5, atCost }, true);
            }
          },
        },
      ],
      participants: [],
      families: [],
    };
    const { banks, firms } = rigFor('storm', { makes: ['coalRaw'] });
    const spec = rigSpec('storm', banks, firms);
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    for (let i = 0; i < 4; i += 1) w.step();
    expect(firm).toBeDefined();
    expect(insurer).toBeDefined();
    expect(atCost).toBeGreaterThan(0);
    if (firm === undefined || insurer === undefined) return;
    const claims = w.journal.ofKind('insurer.claim').filter((e) => e.data['holder'] === firm);
    expect(claims.length).toBeGreaterThan(0);
    const paid = claims.filter((e) => e.data['paid'] === true);
    expect(paid.length).toBeGreaterThan(0);
    const amount = paid.reduce((t, e) => t + Number(e.data['amount']), 0);
    // A4: what the lost units were on its books at, and never more than the cover it held.
    expect(amount).toBeGreaterThan(0);
    expect(amount).toBeLessThanOrEqual(Math.round(atCost) + 1);
    expect(amount).toBeLessThanOrEqual(coverBought);
    // Law 5: the money out and the cover back, both legs of one numbered instruction.
    const at = paid[0]?.period;
    const one = w.ledger.inPeriod(at as never).find((r) => r.outcome === 'settled' && r.instruction.legs.some((l) => l.kind === 'money' && l.receipt?.of === 'claim'));
    expect(one).toBeDefined();
    expect(one?.instruction.legs.filter((l) => l.kind === 'asset' && String(l.instrument).startsWith('policy:'))).toHaveLength(1);
    // The cover used is gone: the policy the claim was paid on has that much less outstanding. (The
    // insurer may have written others in the same session — the firms' own bids of 14.2 were in the
    // book — so it is this policy that is read, not the insurer's whole book.)
    const policy = w.instruments.get(String(paid[0]?.data['policy']) as never);
    expect(isPolicy(policy.terms)).toBe(true);
    expect(policy.issued).toBe(coverBought - amount);
  });
});
