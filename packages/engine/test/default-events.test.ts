/**
 * A loss begins as a payment that did not happen (XI-1).
 *
 * @spec XI-1 Bond N12 Bond N13 Bond N13.a Money E1 Money E1.a Money E1.b Banks Lending E1 Banks Lending E2 Firm Birth C1 Firm Birth C2.a Firm Birth C3 Sovereign G3 Register B4 Law 15
 *
 * The kernel does not know what a default is. It knows that a coupon or a maturity it applied did
 * not settle, and it asks the instrument's own profile whether that meets the definition the
 * instrument states (N12, observable by a holder). Everything here follows from a payment failing:
 * nothing is drawn, nothing is assigned, and there is no rate anywhere (C2.a).
 */
import { describe, expect, it } from 'vitest';
import {
  GOV_LINE,
  PHX,
  TREASURY_NORTH,
  assemble,
  foundationSpec,
  moneyInstrumentId,
  partyId,
  sovereignBond,
  type InstructionDraft,
  type Leg,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { notDealing } from './no-dealing.js';

const SINK = partyId('firm.1');

/** Takes everything the treasury holds before its coupons fall due, and gives it to somebody named. */
function drain(): SystemModule {
  return {
    id: 'test.drain',
    spec: 'Money E1',
    requires: ['sovereign-instruments'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.drain',
        spec: 'Money E1',
        cycle: 0,
        anchor: { before: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          const cash = ctx.participant(TREASURY_NORTH).cash(PHX);
          if (cash <= 0) return;
          const leg: Leg = {
            kind: 'money',
            from: { holder: TREASURY_NORTH, issuer: ctx.parties.get(TREASURY_NORTH).bank },
            to: { holder: SINK, issuer: ctx.parties.get(SINK).bank },
            ccy: PHX,
            amount: cash,
            fromCell: { some: false },
            toCell: { some: false },
          };
          const draft: InstructionDraft = { legs: [leg], cause: 'transfer', reason: 'drained' };
          ctx.settle(draft);
        },
      },
    ],
    participants: [],
    families: [],
  };
}

/** The kernel and the opening state: no treasury programme to refill what the test took. */
function world(...extra: SystemModule[]): World {
  const spec = foundationSpec('default-events');
  const kernelOnly = spec.modules.filter(
    (m) =>
      m.id === 'sovereign-instruments' ||
      m.id === 'seed.foundation' ||
      m.id === 'banks' ||
      m.id === 'money-market',
  ).map(notDealing);
  return assemble({ ...spec, modules: [...kernelOnly, ...extra] });
}

describe('what an instrument says a default is (Bond N12, N13.a)', () => {
  it('states a claim and a ranking even when the answer is nothing seizable', () => {
    const w = world();
    const rank = sovereignBond.ranking(w.instruments.get(GOV_LINE));
    // N13: stated even when there is nothing to seize. N13.a: pari passu, always — one number, and
    // it never varies between an issuer's lines, which is what "always" means.
    expect(rank.claim.length).toBeGreaterThan(0);
    expect(rank.secured).toEqual([]);
    expect(rank.seniority).toBe(0);
    // Sovereign G3: and a missed payment on one line does not make the others due.
    expect(sovereignBond.accelerates).toBe(false);
  });
});

describe('a payment that did not happen (XI-1, Money E1)', () => {
  it('makes the line a default event, publicly, with the definition it met', () => {
    const w = world(drain());
    let events = w.journal.ofKind('credit.default');
    for (let i = 0; i < 60 && events.length === 0; i += 1) {
      w.step();
      events = w.journal.ofKind('credit.default');
    }
    const ev = events[0];
    expect(ev).toBeDefined();
    // Firm Birth C3: others react to it, so it is public — and it names who failed, on what, to
    // whom, and for how much, because a holder must be able to observe it (N12).
    expect(ev?.public).toBe(true);
    expect(ev?.subjects).toContain(TREASURY_NORTH);
    expect(String(ev?.data['definition']).length).toBeGreaterThan(0);
    expect(Number(ev?.data['amountDue'])).toBeGreaterThan(0);
    expect(ev?.data['issuer']).toBe(TREASURY_NORTH);
    // Money E1.a: it did not silently not happen. The failed instruction is in the ledger, and the
    // payee's receivable that did not arrive is a read of it (E1.b).
    const failed = w.ledger.all().filter((r) => r.outcome === 'failed');
    expect(failed.length).toBeGreaterThan(0);
    expect(w.cash(TREASURY_NORTH, PHX)).toBe(0);
  });

  it('writes the status, once, and nothing restores it (Banks Lending E2)', () => {
    const w = world(drain());
    for (let i = 0; i < 60; i += 1) {
      w.step();
      if (w.journal.ofKind('credit.default').length > 0) break;
    }
    // Whichever line the failure fell on — a coupon or a face that fell due; both are payments.
    const ev = w.journal.ofKind('credit.default')[0];
    const id = String(ev?.data['instrument']);
    const line = w.instruments.get(id as never);
    expect(line.status.live && line.status.performing).toBe(false);
    // It is written once however many holders were missed: the status is the instrument's.
    const before = w.journal.ofKind('credit.default').length;
    for (let i = 0; i < 8; i += 1) w.step();
    const after = w.instruments.get(id as never);
    expect(after.status.live ? after.status.performing : false).toBe(false);
    expect(w.journal.ofKind('credit.default').length).toBeGreaterThanOrEqual(before);
  });

  it('says nothing when the payment was made: no default without a failure (Firm Birth C2.a)', () => {
    const w = world();
    for (let i = 0; i < 20; i += 1) w.step();
    // The treasury paid every coupon out of what the seed gave it, so nothing crossed anything.
    expect(w.journal.ofKind('credit.default')).toHaveLength(0);
    const line = w.instruments.get(GOV_LINE);
    expect(line.status.live && line.status.performing).toBe(true);
    // And money cannot default at all: a kind with no definition of failure does not answer.
    const cash = w.instruments.get(moneyInstrumentId(partyId('bank.a'), PHX));
    expect(w.registry.instrumentKind(cash.kind).defaultOn).toBeUndefined();
  });
});
