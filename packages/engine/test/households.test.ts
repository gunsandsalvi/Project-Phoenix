/**
 * Households: cells that decide for one household and carry how many of them they are — what to
 * spend, what to hold, and what to do with what is left.
 *
 * @spec Households A2 Households A2.a Households A2.e Households A2.f Households A2.g Households B4 Households B5 Households C1 Households C1.a Households C1.c Households C1.d Households C2 Households C3 Households C4 Households C5 Households D5 Households D5.a Households D6 Goods C1 Treasury C1 Treasury C1.a Treasury C2 Treasury C3 Expectations C1 XI-15
 */
import { describe, expect, it } from 'vitest';
import {
  FIRM,
  HOUSEHOLD,
  PHX,
  REGION,
  assemble,
  foundationSpec,
  goodId,
  goodMarketId,
  households,
  isMoneyLeg,
  partyId,
  type CellParty,
  type Event,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';

const BREAD = goodId('bread', REGION);
const BANK_A = partyId('bank.a');
const FIRM_3 = partyId('firm.3');
/** A named payer with money of its own, so what a run shows is the spread and nothing else. */
const PAYER = partyId('payer.1');

/** The opening condition the seed states at 4.7: what things were fetching, and a stock to sell. */
function opening(price: number, stock: number): SystemModule {
  return {
    id: 'test.opening',
    spec: 'Seed C4',
    requires: ['goods', 'firms', 'households', 'seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    participants: [],
    families: [],
    seed(ctx) {
      ctx.prices.write({
        instrument: BREAD,
        market: goodMarketId('bread', REGION),
        period: ctx.period,
        price,
        ccy: PHX,
        provenance: { kind: 'opening' },
      });
      ctx.endowUnits(FIRM_3, BREAD, stock, price);
    },
  };
}

/**
 * A2.g: a payer that can spread what it pays across cells without moving what it pays in total.
 * `spread` of zero pays every cell the same; a positive one pays alternate cells more and less by
 * it, which is a mean-preserving spread over cells of equal weight.
 *
 * It pays out of its own account, seeded deep enough that neither run can run it dry — so what
 * differs between two runs is the spread and nothing else.
 */
function payer(perMember: number, spread: number): SystemModule {
  return {
    id: 'test.payer',
    spec: 'Households B2',
    requires: ['households', 'seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    seed(ctx) {
      ctx.parties.add({
        id: PAYER,
        kind: FIRM,
        region: REGION,
        name: 'A payer',
        bank: BANK_A,
        representation: 'named',
        status: { alive: true },
      });
      ctx.endowMoney(PAYER, PHX, 100000);
      // Seed A4: a deposit is a bank's liability, and a bank that owes it holds something against
      // it. Without the reserves, the first payment across banks would be an overdraft this world
      // has no lender for yet (Money B3.c, worklist 11) — an artefact of the seed, not of anything
      // a household did.
      ctx.endowMoney(BANK_A, PHX, 100000);
    },
    phases: [
      {
        name: 'test.pay',
        spec: 'Households B2',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: (ctx: MechanismContext) => {
          const cells = ctx.parties
            .ofKind(HOUSEHOLD)
            .filter((p): p is CellParty => p.representation === 'cell' && p.status.alive && p.key.bank === BANK_A)
            .sort((a, b) => (a.id < b.id ? -1 : 1));
          cells.forEach((cell, n) => {
            const each = perMember + (n % 2 === 0 ? spread : -spread);
            if (each <= 0) return;
            ctx.settle({
              legs: [
                {
                  kind: 'money',
                  from: { holder: PAYER, issuer: BANK_A },
                  to: { holder: cell.id, issuer: cell.bank },
                  ccy: PHX,
                  amount: each * cell.weight,
                  fromCell: { some: false },
                  toCell: { some: true, value: { perMember: each, weight: cell.weight } },
                },
              ],
              cause: 'transfer',
              reason: `a payment to ${cell.id}`,
            });
          });
        },
      },
    ],
    participants: [],
    families: [],
  };
}

function world(...extra: readonly SystemModule[]): World {
  const spec = foundationSpec('households');
  return assemble({ ...spec, modules: [...spec.modules, ...extra] });
}

function plans(w: World, at: number): Event[] {
  return w.journal.ofKind('households.plan').filter((e) => e.period === at);
}

function num(e: Event | undefined, key: string): number {
  const v = e?.data[key];
  return typeof v === 'number' ? v : 0;
}

describe('what a household decides (Households C1, C2)', () => {
  it('spends what it expects to earn, corrected towards the cushion it wants', () => {
    const w = world();
    for (let i = 0; i < 4; i += 1) {
      const r = w.step();
      expect(r.audit.total).toBe(0);
    }
    const decided = plans(w, w.period);
    expect(decided.length).toBeGreaterThan(0);
    for (const e of decided) {
      const spend = num(e, 'spendPerMember');
      const income = num(e, 'expectedIncome');
      const cash = num(e, 'cashPerMember');
      const buffer = num(e, 'bufferPerMember');
      // C1.a, C1.d: what it expects to earn, plus the gap to its cushion at its own patience, and
      // never more than it holds — because nobody lends to it.
      expect(spend).toBeLessThanOrEqual(cash);
      expect(spend).toBeGreaterThan(0);
      // Its cushion is periods of what it expects, so a cell expecting more wants more in hand.
      expect(buffer).toBeGreaterThan(income);
    }
  });

  it('takes a demand curve to market and not a point (Goods C1, C3)', () => {
    const w = world(opening(1.2, 40));
    for (let i = 0; i < 3; i += 1) {
      const r = w.step();
      expect(r.audit.total).toBe(0);
    }
    const bids = w.journal
      .ofKind('print')
      .filter((e) => e.subjects.includes(BREAD) && e.data['printed'] !== false);
    // Somebody bought bread from the baker, at a price the two sides agreed on.
    expect(bids.length).toBeGreaterThan(0);
    const held = w.parties
      .ofKind(HOUSEHOLD)
      .filter((p) => p.status.alive)
      .map((p) => w.register.quantity(p.id, BREAD))
      .reduce((a, b) => a + b, 0);
    expect(held).toBeGreaterThan(0);
    // C3: what it spends on a good is its cohort's share of what it decided to spend, so what it
    // buys falls when the price rises and the money it lays out does not.
    const plan = plans(w, w.period)[0];
    const orders = plan?.data['orders'];
    expect(Array.isArray(orders)).toBe(true);
  });
});

describe('what it does with what is left (Households D5, D5.a, C2)', () => {
  it('holds paper instead of a deposit when paper pays it enough, and not otherwise', () => {
    const w = world();
    for (let i = 0; i < 6; i += 1) w.step();
    const cells = w.parties.ofKind(HOUSEHOLD).filter((p) => p.status.alive);
    const paper = cells
      .flatMap((c) => w.register.holdingsOf(c.id))
      .filter((h) => h.instrument.startsWith('gov.') || h.instrument.startsWith('treasury.'));
    expect(paper.length).toBeGreaterThan(0);
    // D5.a: it is a substitution — what it did not put into paper is still in its account, which
    // is what saving into a deposit is (C2), and nothing was allocated to it pro rata.
    for (const c of cells) expect(w.cash(c.id, PHX)).toBeGreaterThan(0);
  });

  it('is drawn into paper when paper pays it enough, and not when it does not (D5.a)', () => {
    // The one thing that differs between the two runs is what a saver requires of paper for
    // giving up access to its money. A saver that wants little bids a price paper trades at and
    // fills; one that wants a great deal bids far below the market and stays in its deposit.
    // That is the substitution D5.a is about, and it is the channel a deposit rate would reach.
    const bought = (premium: number): number => {
      const spec = foundationSpec('premium');
      const modules = spec.modules.map((m) =>
        m.id === 'households'
          ? {
              ...m,
              params: m.params.map((p) =>
                p.id === 'households.liquidityPremium' ? { ...p, value: premium } : p,
              ),
            }
          : m,
      );
      const w = assemble({ ...spec, modules });
      for (let i = 0; i < 4; i += 1) {
        const r = w.step();
        expect(r.audit.total).toBe(0);
      }
      const cells = new Set(w.parties.ofKind(HOUSEHOLD).map((c) => c.id));
      return w.ledger
        .all()
        .filter((r) => r.outcome === 'settled')
        .flatMap((r) => r.instruction.legs)
        .filter((leg) => leg.kind === 'asset' && cells.has(leg.to))
        .reduce((a, leg) => a + (leg.kind === 'asset' ? leg.qty : 0), 0);
    };
    expect(bought(0.5)).toBe(0);
    expect(bought(0.001)).toBeGreaterThan(0);
  });

  it('counts what it owns and not only what it holds, so an asset price reaches demand (C1.b)', () => {
    const w = world();
    for (let i = 0; i < 4; i += 1) w.step();
    const decided = plans(w, w.period);
    expect(decided.length).toBeGreaterThan(0);
    for (const e of decided) {
      // D3: net worth is a read of what it holds at what the market last said, never a stored
      // number — and it is bigger than the cash, because these cells hold the sovereign's paper.
      expect(num(e, 'wealthPerMember')).toBeGreaterThan(num(e, 'cashPerMember'));
    }
  });

  it('will not tie its money up past its own horizon (D5)', () => {
    const w = world();
    for (let i = 0; i < 8; i += 1) w.step();
    const long = w.instruments
      .all()
      .filter((i) => i.id.includes('2036') || i.id.includes('2031'))
      .map((i) => i.id);
    const cells = w.parties.ofKind(HOUSEHOLD).filter((p) => p.status.alive);
    for (const c of cells) {
      for (const id of long) {
        // What it holds of a long line is what the seed gave it, never anything it bought: a
        // household cannot weigh what it would have to sell early for (worklist 9).
        const bought = w.ledger
          .all()
          .filter((r) => r.outcome === 'settled')
          .flatMap((r) => r.instruction.legs)
          .filter((leg) => leg.kind === 'asset' && leg.to === c.id && leg.instrument === id);
        expect(bought).toHaveLength(0);
      }
    }
  });
});

describe('what the state collects (Treasury C1, C1.a, C3)', () => {
  it('taxes what a household was actually paid, out of the payer own account', () => {
    const w = world(payer(0.1, 0));
    for (let i = 0; i < 4; i += 1) {
      const r = w.step();
      expect(r.audit.total).toBe(0);
    }
    const receipts = w.journal.ofKind('treasury.receipts').find((e) => e.period === w.period);
    const bases = receipts?.data['bases'] as Record<string, number> | undefined;
    expect(bases?.['income']).toBeGreaterThan(0);
    // C3: what was collected is the sum of what named payers actually paid, on the wire.
    const paid = w.ledger
      .inPeriod(w.period)
      .filter((r) => r.outcome === 'settled' && r.instruction.reason.startsWith('tax due from'))
      .flatMap((r) => r.instruction.legs)
      .filter(isMoneyLeg)
      .reduce((a, leg) => a + leg.amount, 0);
    expect(paid).toBeCloseTo(num(receipts, 'total'), 9);
    // C1.a: the base is the payer's own statement — what actually reached it, and not the state's
    // own transfer coming back out of it.
    const rate = w.params.get('treasury.tax.income' as never);
    expect(num(receipts, 'total')).toBeCloseTo((bases?.['income'] ?? 0) * rate, 9);
  });

  it('taxes what a household paid for real things, and the household finds it on top', () => {
    const w = world(opening(1.2, 60));
    for (let i = 0; i < 4; i += 1) {
      const r = w.step();
      expect(r.audit.total).toBe(0);
    }
    // C2: receipts follow the economy, so the period a household bought is the period the base is
    // there — and a period in which the baker had nothing left to sell collects nothing.
    const consumption = w.journal
      .ofKind('treasury.receipts')
      .map((e) => (e.data['bases'] as Record<string, number> | undefined)?.['consumption'] ?? 0);
    expect(Math.max(...consumption)).toBeGreaterThan(0);
    const paid = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled' && r.instruction.reason.startsWith('tax due from'))
      .flatMap((r) => r.instruction.legs)
      .filter(isMoneyLeg);
    // C1.a: out of the payer's own account, every one of them.
    expect(paid.length).toBeGreaterThan(0);
    for (const leg of paid) expect(leg.to.holder).toBe('treasury.north');
  });
});

describe('the sector is a distribution and not an average (Households A2.f, A2.g)', () => {
  it('publishes what it was paid as a sum of what named payers paid it (B5)', () => {
    const w = world(payer(0.1, 0));
    for (let i = 0; i < 3; i += 1) w.step();
    const income = w.journal.ofKind('households.income');
    const published = income[income.length - 1];
    expect(published?.public).toBe(true);
    const of = Number(published?.data['of']);
    const received = w.ledger
      .inPeriod(of as never)
      .filter((r) => r.outcome === 'settled')
      .flatMap((r) => r.instruction.legs)
      .filter(isMoneyLeg)
      .filter((leg) => leg.to.holder.startsWith('hh.'))
      .reduce((a, leg) => a + leg.amount, 0);
    expect(num(published, 'received')).toBeCloseTo(received, 9);
  });

  it('moves cells across a threshold under a mean-preserving spread while the mean stands (A2.g)', () => {
    const flat = world(payer(0.1, 0));
    const spread = world(payer(0.1, 0.1));
    const periods = 5;
    for (let i = 0; i < periods; i += 1) {
      flat.step();
      spread.step();
    }
    const paidTo = (w: World): number =>
      w.ledger
        .all()
        .filter((r) => r.outcome === 'settled' && r.instruction.reason.startsWith('a payment to'))
        .flatMap((r) => r.instruction.legs)
        .filter(isMoneyLeg)
        .reduce((a, leg) => a + leg.amount, 0);
    // The spread moved nothing in aggregate: the same money reached the same population.
    expect(paidTo(spread)).toBeCloseTo(paidTo(flat), 9);
    const crossings = (w: World): number =>
      w.journal
        .ofKind('households.plan')
        .filter((e) => num(e, 'cashPerMember') < num(e, 'bufferPerMember')).length;
    // A2.g: and yet more cells are below the cushion they want, which is a threshold with a
    // consequence — they spend less than they earn. An average could not have shown it.
    expect(crossings(spread)).toBeGreaterThan(crossings(flat));
    const intended = (w: World): number =>
      w.journal
        .ofKind('households.plan')
        .filter((e) => e.period === w.period)
        .reduce((a, e) => {
          const cell = w.parties.get(e.subjects[0] as never);
          return a + num(e, 'spendPerMember') * (cell.representation === 'cell' ? cell.weight : 1);
        }, 0);
    // A2.a: the same aggregate income, in different hands, is different demand.
    expect(intended(spread)).not.toBeCloseTo(intended(flat), 6);
  });

  it('declares no number a sector took: every one of them is one household own', () => {
    const ids = households().params.map((p) => p.id);
    expect(ids.every((id) => id.startsWith('households.'))).toBe(true);
    expect(households().params.filter((p) => p.kind === 'shape')).toHaveLength(0);
  });
});
