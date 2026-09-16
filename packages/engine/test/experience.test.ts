/**
 * The insurer's quote is its experience plus the return its capital requires, and cover clears in
 * the world's own book (Insurers A4.a, A4.b, A4.c, XI-4, 14.4).
 *
 * @spec Insurers A4.a Insurers A4.b Insurers A4.c XI-4 Expectations B1 Law 19
 */
import { describe, expect, it } from 'vitest';
import { FIRM, assemble, type MechanismContext, type SystemModule } from '../src/index.js';
import { INSURANCE } from '../src/mechanisms/insurers/index.js';
import { POLICY_ROW, isPolicyTerms } from '../src/registry/insurance.js';
import { vintagesHeld } from '../src/registry/physical.js';
import { about } from '../src/world/context.js';
import { asQty } from '../src/core/tick.js';
import { RIG_BANKS, RIG_FIRMS, mergeModules, rigSpec, rigWorld } from './rig.js';

describe('the insurer quotes, and cover clears in the world’s own book (14.4)', () => {
  it('writes cover to the firms that bid for it, at a price made of nothing but the return its capital requires while it has no experience', () => {
    // `quote-37`: the one seed in forty whose insurer's line the draw listed, so XI-4 reads what its
    // equity costs off a print; an unlisted insurer reads its capital off the curve instead.
    const w = rigWorld('quote-37');
    for (let i = 0; i < 6; i += 1) w.step();
    const sessions = w.journal.ofKind('cover.cleared');
    const written = sessions.filter((e) => Number(e.data['written']) > 0);
    expect(written.length).toBeGreaterThan(0);
    // A4.a: where the insurer wrote nothing it said why, in public.
    for (const e of w.journal.ofKind('insurer.unquoted')) expect(String(e.data['why']).length).toBeGreaterThan(0);
    const policies = w.agreements.ofKind(POLICY_ROW).filter((a) => a.state === 'performing' && isPolicyTerms(a.terms) && a.terms.cover > 0);
    expect(policies.length).toBeGreaterThan(0);
    for (const a of policies) expect(w.parties.get(a.debtor).kind).toBe(INSURANCE);
    const holders = new Set(policies.map((a) => w.parties.get(a.creditor).kind));
    expect(holders.has(FIRM)).toBe(true);
    // A4.c: with no claim ever paid, its outlook of what a unit costs it is nothing — but it exists,
    // because every period with cover out is an observation.
    const insurer = w.parties.ofKind(INSURANCE).find((p) => p.status.alive);
    expect(insurer).toBeDefined();
    if (insurer === undefined) return;
    const seen = w.participantView(insurer.id).outlook(about({ on: 'claims' }));
    expect(seen.some).toBe(true);
    expect(seen.some && seen.value.expected).toBe(0);
  });

  it('forms its experience from the claims it paid, and quotes higher for it', () => {
    let priceBefore: number | undefined;
    const probe: SystemModule = {
      id: 'test.storm2',
      spec: 'Insurers A4.c',
      requires: ['insurers', 'firms', 'capital-programme'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.storm2',
          spec: 'Insurers A4.c',
          anchor: { before: 'insurers.claims' },
          reads: [],
          writes: [{ kind: 'event', name: 'capital.weathered' }],
          run: (ctx: MechanismContext) => {
            if (ctx.period < 3) return;
            // A storm on every covered firm's first vintage, every period from the third: the loss the
            // world's own cover pays, as soon as there is cover out to pay on.
            for (const a of ctx.agreements.ofKind(POLICY_ROW)) {
              if (a.state !== 'performing' || !isPolicyTerms(a.terms) || a.terms.cover <= 0) continue;
              for (const h of [a.creditor]) {
                const p = ctx.parties.get(h);
                if (p.kind !== FIRM || !p.status.alive) continue;
                const v = vintagesHeld(ctx.participant(h), ctx.calendar.startOf(ctx.period))[0];
                if (v === undefined) continue;
                const lost = asQty(v.units < 5 ? v.units : 5, 'what the storm takes');
                const r = ctx.settle({
                  legs: [{ kind: 'destroy', party: h, instrument: v.instrument as never, qty: lost, why: 'scrapped' }],
                  cause: 'corporateAction',
                  reason: `${String(h)} lost ${String(lost)} of ${v.instrument} to the weather`,
                });
                if (r.outcome !== 'settled') continue;
                ctx.record('capital.weathered', [h, v.instrument], { holder: h, vintage: v.instrument, capitalKind: v.capitalKind, units: lost, wind: 4, survived: 0.5, atCost: lost * v.basisPerUnit }, true);
              }
            }
          },
        },
      ],
      participants: [],
      families: [],
    };
    // The rig's own shape (a fuel line in it stops the world at 21.1's grid the period after a
    // promotion, so the shape `rigFor` draws for one is not the one to run a storm in).
    const spec = rigSpec('quote-37', RIG_BANKS, RIG_FIRMS);
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    let firstClaimAt: number | undefined;
    for (let i = 0; i < 16 && (firstClaimAt === undefined || w.period < firstClaimAt + 3); i += 1) {
      w.step();
      const s = w.journal.ofKindIn('cover.cleared', w.period).find((e) => Number(e.data['written']) > 0);
      if (s !== undefined && priceBefore === undefined) priceBefore = Number(s.data['price']);
      if (firstClaimAt === undefined && w.journal.ofKindIn('insurer.claim', w.period).some((e) => e.data['paid'] === true)) firstClaimAt = w.period;
    }
    const insurer = String(w.parties.ofKind(INSURANCE).find((p) => p.status.alive)?.id);
    const claims = w.journal.ofKind('insurer.claim').filter((e) => e.data['paid'] === true);
    expect(claims.length).toBeGreaterThan(0);
    // A4.c: its outlook of what a unit of its cover costs it moved, towards what its periods cost it.
    const seen = w.participantView(insurer as never).outlook(about({ on: 'claims' }));
    expect(seen.some && seen.value.expected).toBeGreaterThan(0);
    // A4.b: worse experience quotes higher — the book clears above where it cleared before the storm.
    const after = w.journal.ofKind('cover.cleared').filter((e) => firstClaimAt !== undefined && e.period > firstClaimAt && Number(e.data['written']) > 0);
    expect(priceBefore).toBeDefined();
    if (after.length > 0 && priceBefore !== undefined) expect(Number(after[after.length - 1]?.data['price'])).toBeGreaterThan(priceBefore);
  });
});
