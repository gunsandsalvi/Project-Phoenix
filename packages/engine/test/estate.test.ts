/**
 * Nothing is immortal: a party fails, an estate takes what it held and what it owed, sells, pays in
 * rank order, and ends — and the real economy feels it.
 *
 * @spec XI-2 XI-3 XI-8 Firm Birth D1 Firm Birth D2 Firm Birth D2.a Firm Birth D2.b Firm Birth D3 Firm Birth D4 Firm Birth D4.a Firm Birth D5 Firm Birth D6 Firm Birth D6.a Firm Birth E1 Firm D4 Labour C4 Labour F1 Register F2 Money E4 Banks Lending E5 Corporate Credit G5.a Households C1.d Observer B5 Law 2 Law 13
 *
 * XI-3 is what these tests are for: an immortal party is the termination condition of every loss
 * chain in the model, and a cascade that reaches one stops there without saying so. So the thing to
 * assert is not that a death is tidy — it is that it HAPPENS, that everything it touched still has
 * a named counterparty afterwards, and that the loss lands on somebody rather than evaporating.
 */
import { describe, expect, it } from 'vitest';
import {
  FIRM,
  HOUSEHOLD,
  PHX,
  REGION,
  assemble,
  currencyUnit,
  foundationSpec,
  goodId,
  instrumentId,
  instrumentKindId,
  none,
  partyId,
  snapshot,
  some,
  type InstrumentKindId,
  type InstrumentKindProfile,
  type MarketDecl,
  type MechanismContext,
  type Order,
  type ParticipantView,
  type SystemModule,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

const DEBTOR = partyId('firm.1');
const SENIOR_HOLDER = partyId('firm.2');
const OTHER_SENIOR_HOLDER = partyId('firm.3');
const JUNIOR_HOLDER = partyId('firm.4');
const ESTATE_OF_DEBTOR = partyId('estate.firm.1');

const SENIOR_KIND = instrumentKindId('test.senior');
const JUNIOR_KIND = instrumentKindId('test.junior');
const SENIOR = instrumentId('test.senior.line');
const JUNIOR = instrumentId('test.junior.line');

/**
 * Bond N13.a: two claims on one borrower that differ ONLY in where they stand. Subordination is
 * decorative unless the junior one can recover nothing, so the test needs a pair whose ranks the
 * profile states and the waterfall reads.
 */
function claimKind(id: InstrumentKindId, seniority: number): InstrumentKindProfile {
  return {
    id,
    pricing: 'carriedAtCost',
    carry: 'cost',
    liabilityOfIssuer: true,
    ranking: () => ({ seniority, secured: [], claim: 'the face, from whatever there is' }),
    unit: (ccy) => currencyUnit(ccy),
    validateTerms: () => undefined,
    displayName: (i) => String(i.id),
    due: () => [],
    accrued: () => 0,
    cashFlows: () => [],
  };
}

/**
 * A firm that owes far more than it holds, before anything else runs: its liabilities are past its
 * assets on the first period it is looked at, which is one of the two things Firm D4 calls failure.
 * The senior line has two holders so a rank has something to be divided pro rata BETWEEN.
 */
function owesMoreThanItHas(): SystemModule {
  return {
    id: 'test.owes',
    spec: 'Firm D4 XI-8',
    requires: ['seed.foundation'],
    instrumentKinds: [claimKind(SENIOR_KIND, 0), claimKind(JUNIOR_KIND, 1)],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    participants: [],
    families: [],
    seed(ctx) {
      for (const [id, kind] of [
        [SENIOR, SENIOR_KIND],
        [JUNIOR, JUNIOR_KIND],
      ] as const) {
        ctx.instruments.add({
          id,
          kind,
          issuer: some(DEBTOR),
          ccy: PHX,
          terms: { kind },
          market: none(),
        });
      }
      ctx.endowUnits(SENIOR_HOLDER, SENIOR, 1e6, 1);
      ctx.endowUnits(OTHER_SENIOR_HOLDER, SENIOR, 1e6, 1);
      ctx.endowUnits(JUNIOR_HOLDER, JUNIOR, 1e6, 1);
    },
  };
}

/**
 * XI-3, Firm D4: the OTHER failure, and it is a different one — a firm with plenty of assets and no
 * cash. It promises more than it holds, its bank refuses (the world is built with no room), and the
 * payment fails; what it could not pay it still cannot, which is the cash trigger. It stays solvent
 * throughout, so its estate has more than its claims are worth and money is left over at the end.
 */
function cannotPay(amount: number): SystemModule {
  return {
    id: 'test.cannot-pay',
    spec: 'Firm D4 Money E1',
    requires: ['seed.foundation'],
    instrumentKinds: [claimKind(SENIOR_KIND, 0)],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.cannot-pay',
        spec: 'Money E1',
        cycle: 0,
        anchor: { before: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          if (ctx.period !== 1) return;
          const to = ctx.parties.get(SENIOR_HOLDER);
          ctx.settle({
            legs: [
              {
                kind: 'money',
                from: { holder: DEBTOR, issuer: ctx.parties.get(DEBTOR).bank },
                to: { holder: SENIOR_HOLDER, issuer: to.bank },
                ccy: PHX,
                amount,
                fromCell: none(),
                toCell: none(),
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
    seed(ctx) {
      ctx.instruments.add({
        id: SENIOR,
        kind: SENIOR_KIND,
        issuer: some(DEBTOR),
        ccy: PHX,
        terms: { kind: SENIOR_KIND },
        market: none(),
      });
      ctx.endowUnits(SENIOR_HOLDER, SENIOR, 1, 1);
    },
  };
}

/**
 * Firm Birth D6: a payment out of an estate to somebody with no claim on it. Nothing in the model
 * does this — which is the point: a FORBID that holds breaks silently, so the guard against it is
 * pointed at a mechanism written to break it.
 */
function paysAStranger(): SystemModule {
  return {
    id: 'test.leak',
    spec: 'Firm Birth D6',
    requires: ['estate'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.leak',
        spec: 'Firm Birth D6',
        cycle: 'anchor',
        anchor: { after: 'revaluation' },
        run: (ctx: MechanismContext) => {
          if (!ctx.parties.has(ESTATE_OF_DEBTOR)) return;
          const estate = ctx.parties.get(ESTATE_OF_DEBTOR);
          if (!estate.status.alive) return;
          const stranger = ctx.parties.get(JUNIOR_HOLDER);
          ctx.settle({
            legs: [
              {
                kind: 'money',
                from: { holder: ESTATE_OF_DEBTOR, issuer: estate.bank },
                to: { holder: JUNIOR_HOLDER, issuer: stranger.bank },
                ccy: PHX,
                amount: 1,
                fromCell: none(),
                toCell: none(),
              },
            ],
            cause: 'transfer',
            reason: 'a payment to somebody with no claim',
          });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

/** The kernel and the two modules a death needs: what fails, and where it goes. */
function failingWorld(...extra: readonly SystemModule[]): World {
  const spec = foundationSpec('estate');
  const kernel = spec.modules
    .filter((m) =>
      [
        'sovereign-instruments',
        'seed.foundation',
        'bank-lending',
        'credit-events',
        'estate',
      ].includes(m.id),
    )
    // No bank in this world will lend a penny, so a party that cannot pay simply does not pay: the
    // refusal is the answer B3.c wants and the failure is a real state (Money E1).
    .map((m) => ({
      ...m,
      params: m.params.map((p) =>
        p.id.startsWith('bank.limitPerBorrower.') ? { ...p, value: 0 } : p,
      ),
    }));
  return assemble({ ...spec, modules: [...kernel, ...extra] });
}

/**
 * A buyer of flour far bigger than the crop that makes it: the mills bid the grain price up against
 * each other until one of them is paying more for the grain than the flour fetches. That is a firm
 * dying of its own decisions in a world that is otherwise running normally — which is the only kind
 * of death worth testing the consequences of.
 */
function hungryBuyer(): SystemModule {
  const BUYER = partyId('buyer.1');
  const instrument = goodId('flour', REGION);
  return {
    id: 'test.buyer',
    spec: 'Goods C3',
    requires: ['goods', 'seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    seed(ctx) {
      ctx.parties.add({
        id: BUYER,
        kind: FIRM,
        region: REGION,
        name: 'A buyer',
        bank: partyId('bank.a'),
        representation: 'named',
        status: { alive: true },
      });
      ctx.endowMoney(BUYER, PHX, 100000);
      ctx.endowMoney(partyId('bank.a'), PHX, 100000);
    },
    participants: [
      {
        partyKind: FIRM,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] =>
          view.self.id === BUYER && m.instrument === instrument
            ? [{ party: BUYER, side: 'buy', price: 1.2, qty: 400 }]
            : [],
      },
    ],
    families: [],
  };
}

/** The world the seed opens, with somebody hungry enough in it to kill a mill. */
function worldWithADeathInIt(seed = 'estate'): World {
  const spec = foundationSpec(seed);
  return assemble({ ...spec, modules: [...spec.modules, hungryBuyer()] });
}

describe('a party that fails (XI-3, Firm D4, Firm Birth D1)', () => {
  it('opens an estate that takes what it held and assumes what it owed (D5, Register F2)', () => {
    const w = failingWorld(owesMoreThanItHas());
    const held = w.cash(DEBTOR, PHX);
    expect(held).toBeGreaterThan(0);
    expect(unexpected(w.step().audit)).toEqual([]);

    const opened = w.journal.ofKind('estate.opened');
    expect(opened).toHaveLength(1);
    expect(opened[0]?.public).toBe(true);
    expect(opened[0]?.data['dead']).toBe(DEBTOR);
    // D4: the answer says WHICH failure fired, because they are different failures (Banks Capital C1.a).
    expect(String(opened[0]?.data['because'])).toContain('liabilities are past its assets');

    // Register F2: the move is an instruction, not an assignment — the ledger shows it.
    const moves = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.instruction.reason === `${DEBTOR} to its estate`);
    expect(moves.length).toBeGreaterThan(0);
    expect(moves.every((r) => r.outcome === 'settled')).toBe(true);
    expect(w.cash(DEBTOR, PHX)).toBe(0);
    // It arrived and then went straight out again in the same phase: what the estate had is what
    // it distributed, which is D2.a's whole point — a recovery is what the assets fetched.
    const out = w.journal.ofKind('estate.paid').filter((e) => e.data['estate'] === ESTATE_OF_DEBTOR);
    expect(out.reduce((t, e) => t + Number(e.data['paid']), 0)).toBeCloseTo(held, 9);

    // D5: what it ISSUED did not vanish with it. Same holders, same units, a live issuer.
    expect(w.instruments.get(SENIOR).issuer).toEqual(some(ESTATE_OF_DEBTOR));
    expect(w.instruments.get(JUNIOR).issuer).toEqual(some(ESTATE_OF_DEBTOR));
    expect(w.register.holdersOf(SENIOR)).toContain(SENIOR_HOLDER);
    expect(w.register.holdersOf(SENIOR)).toContain(OTHER_SENIOR_HOLDER);
    expect(w.instruments.get(SENIOR).issued).toBeGreaterThan(0);
    // Law 5: the obligation moved between two balance sheets in one numbered instruction.
    const assumed = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.instruction.legs.some((l) => l.kind === 'assume'));
    expect(assumed).toHaveLength(2);
    expect(assumed.every((r) => r.outcome === 'settled')).toBe(true);
  });

  it('is what every reference to it resolves to, for as many hops as it takes (D5)', () => {
    const w = failingWorld(owesMoreThanItHas());
    for (let i = 0; i < 12; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const gone = w.parties.get(DEBTOR).status;
    expect(gone.alive).toBe(false);
    expect(gone.alive ? null : gone.successor).toBe(ESTATE_OF_DEBTOR);
    // The estate is where the chain ENDS: it succeeds itself, and resolving still terminates.
    expect(w.journal.ofKind('estate.closed')).toHaveLength(1);
    expect(w.parties.resolve(DEBTOR).id).toBe(ESTATE_OF_DEBTOR);
    expect(w.parties.resolve(ESTATE_OF_DEBTOR).id).toBe(ESTATE_OF_DEBTOR);
    // D6.a: and it ended holding nothing, which is what lets it end at all.
    expect(w.cash(ESTATE_OF_DEBTOR, PHX)).toBe(0);
  });
});

describe('the waterfall (XI-8, Firm Birth D2, D2.a)', () => {
  it('pays senior first, pro rata within the rank, and the junior recovers nothing (G5.a)', () => {
    const w = failingWorld(owesMoreThanItHas());
    const had = w.cash(DEBTOR, PHX);
    w.step();
    const paid = w.journal.ofKind('estate.paid').filter((e) => e.period === w.period);
    // Two claims at the top rank, one at the bottom, and not enough for the top one.
    expect(paid.map((e) => e.data['holder']).sort()).toEqual(
      [SENIOR_HOLDER, OTHER_SENIOR_HOLDER].sort(),
    );
    expect(paid.every((e) => e.data['instrument'] === SENIOR)).toBe(true);
    const a = Number(paid[0]?.data['paid']);
    const b = Number(paid[1]?.data['paid']);
    // D2.a: what they got is what the assets fetched, split by what they were owed — never a rate.
    expect(a).toBeCloseTo(b, 9);
    expect(a + b).toBeCloseTo(had, 9);
    expect(w.register.quantity(JUNIOR_HOLDER, JUNIOR)).toBe(1e6);
  });

  it('writes off what it never paid, and the loss lands on the holders (D3, E5)', () => {
    const w = failingWorld(owesMoreThanItHas());
    const seniorBefore = w.register.equity(SENIOR_HOLDER);
    const juniorBefore = w.register.equity(JUNIOR_HOLDER);
    for (let i = 0; i < 12; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    // D3: the loss is on named holders, in proportion to what each was owed and not paid.
    expect(w.register.quantity(SENIOR_HOLDER, SENIOR)).toBe(0);
    expect(w.register.quantity(JUNIOR_HOLDER, JUNIOR)).toBe(0);
    expect(w.register.equity(SENIOR_HOLDER)).toBeLessThan(seniorBefore);
    // The junior was paid nothing at all, so it lost the whole of what it was carrying.
    expect(juniorBefore - w.register.equity(JUNIOR_HOLDER)).toBeCloseTo(1e6, 6);
    // ...and it lost MORE than the senior did, which is the whole of what being junior means.
    expect(juniorBefore - w.register.equity(JUNIOR_HOLDER)).toBeGreaterThan(
      seniorBefore - w.register.equity(SENIOR_HOLDER),
    );
  });
});

describe('what a death costs the real economy (Firm Birth D4, D4.a)', () => {
  it('releases the dead firm‘s workers through the labour market‘s own path (Labour C4, F1)', () => {
    const w = worldWithADeathInIt();
    for (let i = 0; i < 14; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const died = w.journal.ofKind('estate.opened');
    expect(died.length).toBeGreaterThan(0);
    const dead = String(died[0]?.data['dead']);
    // D4.a: a death drops a headcount only through the separation path, so every job that ended
    // ended as a separation event with the death as its cause — never by a row disappearing.
    const released = w.journal
      .ofKind('labour.separation')
      .filter((e) => e.data['employer'] === dead && e.data['cause'] === `${dead} ceased`);
    expect(released.length).toBeGreaterThan(0);
    expect(released.every((e) => Number(e.data['members']) > 0)).toBe(true);
    // Labour F1: and no job is left at a firm that has ceased — the audit family that says so is
    // built, so its silence is a real absence.
    const names = w.step().audit.families.filter((f) => f.family === 'names');
    expect(names.every((f) => f.built)).toBe(true);
    expect(names.flatMap((f) => f.violations)).toEqual([]);
    // D2.b: what it owed them ranks with the other unsecured claims and is paid in the winding-up.
    // There is no instrument for a claim like that yet (worklist 13), so it is owed and unpaid —
    // said out loud on the event rather than quietly paid by an estate that had nothing.
    expect(released.every((e) => e.data['severancePaid'] === false)).toBe(true);
    expect(released.every((e) => Number(e.data['severanceRanking']) > 0)).toBe(true);
  });

  it('shows the estate, its programme and where the dead party went (Observer B5)', () => {
    const w = worldWithADeathInIt();
    for (let i = 0; i < 14; i += 1) w.step();
    const view = snapshot(w, { kind: 'inspector' }, 200);
    const dead = String(w.journal.ofKind('estate.opened')[0]?.data['dead']);
    // D5: a dead party is not a name that stops — the surface says what it resolves to.
    const gone = view.parties.find((p) => p.id === dead);
    expect(gone?.alive).toBe(false);
    expect(gone?.successor).toBe(`estate.${dead}`);
    expect(view.parties.find((p) => p.id === `estate.${dead}`)?.alive).toBe(true);
    // B5: the estate is a story that develops — what it is winding up and how long it has left.
    const book = (view.state?.['estate/estates'] ?? {}) as {
      estates?: Record<string, { closesAfter?: number }>;
    };
    const winding = book.estates?.[`estate.${dead}`];
    expect(winding).toBeDefined();
    expect(Number(winding?.closesAfter)).toBeGreaterThan(w.period - 1);
    // The paper it owes is on the surface too, still outstanding and now issued by the estate.
    expect(
      view.instruments.some((i) => i.issuer === `estate.${dead}` && i.live && i.issued > 0),
    ).toBe(true);
  });
});

describe('the other failure, and what an estate may not do (Firm D4, Firm Birth D6)', () => {
  it('dies of no cash while solvent, and pays only the people with a claim on it', () => {
    // A world where no bank will lend: what it could not pay it still cannot, which is the cash
    // failure, and it is a different one from the balance sheet's (Banks Capital C1.a).
    const w = failingWorld(cannotPay(1e6), paysAStranger());
    const violations: string[] = [];
    for (let i = 0; i < 3; i += 1) {
      violations.push(...w.step().audit.families.flatMap((f) => f.violations).map((v) => v.message));
    }
    const opened = w.journal.ofKind('estate.opened');
    expect(opened).toHaveLength(1);
    expect(String(opened[0]?.data['because'])).toContain('could not pay');
    // D6: the mechanism built to break the rule broke it, and the family said whose money went
    // where. A guard nobody ever fires is a guard nobody knows works.
    expect(violations.some((m) => m.includes(`paid 1 to ${JUNIOR_HOLDER}, who has no claim on it`))).toBe(
      true,
    );
  });
});

describe('what does not open an estate', () => {
  it('leaves household cells alone, because nothing commits one past its cash (C1.d, XI-3)', () => {
    const w = worldWithADeathInIt();
    for (let i = 0; i < 14; i += 1) w.step();
    const cells = w.parties.ofKind(HOUSEHOLD).map((p) => p.id);
    const named: readonly string[] = cells;
    expect(cells.length).toBeGreaterThan(0);
    // XI-3: a household cell dissolves into a named heir cell, and what happens when its members
    // cannot pay is their lender's enforcement — both worklist 13d. Neither is an estate.
    expect(
      w.journal.ofKind('estate.opened').filter((e) => named.includes(String(e.data['dead']))),
    ).toEqual([]);
    // ...and there is nothing for either to fire on yet, because a cell spends what it holds. That
    // absence is the clause (C1.d) and it breaks silently, so it is asserted rather than assumed.
    for (const cell of cells) expect(w.participantView(cell).failedPayments(200)).toEqual([]);
  });
});

describe('a year with a death in it', () => {
  it('stays green and gives the same world twice from the same seed (Law 13, Observer E3)', () => {
    const a = worldWithADeathInIt('a-year');
    for (let i = 0; i < 52; i += 1) expect(unexpected(a.step().audit)).toEqual([]);
    expect(a.journal.ofKind('estate.opened').length).toBeGreaterThan(0);
    // A death is a finding about the world, not a fault in it: the run continues past one.
    expect(a.journal.ofKind('estate.closed').length).toBeGreaterThan(0);
    const b = worldWithADeathInIt('a-year');
    for (let i = 0; i < 52; i += 1) b.step();
    expect(snapshot(b, { kind: 'inspector' }, 50)).toEqual(snapshot(a, { kind: 'inspector' }, 50));
  });
});
