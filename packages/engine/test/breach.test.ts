/**
 * A payment that stayed late long enough to break what it was a payment on (19.8).
 *
 * @spec Money E1 Money E1.a XI-8 Polity D3 Law 2 Law 5
 *
 * `breached` was a state the register could hold and nothing could reach: an arrear stood, and the
 * commitment behind it read `performing` however long it stood. What decides that a late payment
 * has become a broken one is the LAW — a period of grace somebody voted for — so the number is
 * parliament's and the writer is the module that already says what a failure to pay makes somebody.
 */
import { describe, expect, it } from 'vitest';
import {
  agreementKindId,
  asQty,
  assemble,
  moneyInstrumentId,
  mul,
  partyId,
  upTick,
  USD,
  type AgreementTerms,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { BREACH_AFTER } from '../src/mechanisms/credit-events/index.js';
import { mergeModules, rigDraw, rigShapeFor, rigSpec, rigWorld } from './rig.js';

const LATE = agreementKindId('test.aBillToPay');

/**
 * Two firms of the same line — whichever two the draw made — with a commitment between them and a
 * payment on it the payer cannot make. A test asks the draw for a mill; it does not name one.
 */
function overdue(seed: string): { readonly world: World; readonly grace: number } {
  const shape = rigShapeFor(seed, { makes: ['coalRaw'] });
  const draw = rigDraw(seed, shape.banks, shape.firms);
  // Whichever two the draw made: one firm that owes another, with no interest in which lines.
  const two = [...draw.firms].sort((a, b) => b.size - a.size);
  const payer = partyId(two[0]?.firm ?? '');
  const payee = partyId(two[1]?.firm ?? '');
  const terms = { kind: LATE } as AgreementTerms;
  const probe: SystemModule = {
    id: 'test.late',
    spec: 'Money E1',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    agreementKinds: [
      { id: LATE, what: 'a bill one firm owes another', binds: 'whoeverSucceeds' },
    ],
    phases: [
      {
        name: 'test.late',
        spec: 'Money E1',
        anchor: { before: 'corporateActions' },
        reads: [],
        writes: [],
        run: (ctx: MechanismContext) => {
          if (ctx.period === 1) {
            ctx.owes({ debtor: payer, creditor: payee, ccy: USD, owed: 0, terms, why: 'a bill, for the scale model' });
            return;
          }
          if (ctx.period !== 2) return;
          // What it cannot pay is read off its own account, so nothing here is a written number.
          const held = ctx.register.quantity(payer, moneyInstrumentId(ctx.parties.get(payer).bank, USD));
          ctx.settle({
            legs: [
              {
                kind: 'money',
                receipt: { of: 'transfer' },
                from: { holder: payer, issuer: ctx.parties.get(payer).bank },
                to: { holder: payee, issuer: ctx.parties.get(payee).bank },
                ccy: USD,
                // More than it has, whatever it has — and more than nothing, for a firm whose
                // account is empty at this point in the period.
                amount: asQty(upTick(mul(held, 2, 'more than it has')) + 1_000),
              },
            ],
            cause: 'transfer',
            reason: 'a bill it had promised to pay',
          });
        },
      },
    ],
    participants: [],
    families: [],
  };
  const spec = rigSpec(seed, shape.banks, shape.firms);
  const world = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
  return { world, grace: world.params.periods(BREACH_AFTER) };
}

describe('an arrear becomes a breach when the law says so (Money E1, XI-8, Polity D3, 19.8)', () => {
  it('leaves the row performing while the grace runs, and breaks it the period it expires', () => {
    const { world, grace } = overdue('law.breach');
    expect(grace).toBeGreaterThan(0);
    const rows = (): string[] =>
      world.agreements.all().filter((a) => a.terms.kind === LATE).map((a) => a.state);
    for (let i = 0; i <= 2; i += 1) world.step();
    // The payment failed and the payer owes a row for it (Money E1.b) — and the commitment behind
    // it is LATE, which is not the same thing as broken.
    expect(world.journal.ofKind('credit.default').length).toBeGreaterThan(0);
    expect(rows()).toEqual(['performing']);
    for (let i = 0; i < grace - 1; i += 1) world.step();
    expect(rows()).toEqual(['performing']);
    world.step();
    // The grace expired with the arrear still standing, so the row is broken — and it is still a
    // row: what was owed is still owed and an estate would still divide it.
    expect(rows()).toEqual(['breached']);
    const said = world.journal.ofKind('credit.breach');
    expect(said.length).toBe(1);
    expect(Number(said[0]?.data['grace'])).toBe(grace);
    expect(world.journal.ofKind('agreement.breached').length).toBe(1);
  });

  it('is parliament’s number, and a parliament that allows longer breaks nothing yet', () => {
    const w = rigWorld('law.breach.policy');
    const d = w.params.decl(BREACH_AFTER);
    expect(d.kind).toBe('policy');
    expect(d.owner).toBe('parliament');
    expect(d.dimension).toBe('periods');
    // The same world, with a grace long enough that the same arrear is still merely late.
    const { world, grace } = overdue('law.breach');
    world.params.setByMandate(BREACH_AFTER, grace * 4, 'parliament', 'a longer grace, for the test.');
    for (let i = 0; i <= grace + 3; i += 1) world.step();
    expect(world.journal.ofKind('credit.breach').length).toBe(0);
    expect(world.agreements.all().filter((a) => a.state === 'breached').length).toBe(0);
  });
});
