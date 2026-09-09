import { describe, expect, it } from 'vitest';
import {
  BANK_A,
  BANK_B,
  CB,
  Forbidden,
  GOV_LINE,
  GOV_MARKET,
  HOUSEHOLD,
  FIRM,
  InvalidRegistry,
  NotYetProduced,
  PHX,
  TREASURY_NORTH,
  assemble,
  cellSide,
  foundationSeed,
  foundationSpec,
  foundationWorld,
  moneyInstrumentId,
  none,
  orderModules,
  partyId,
  period,
  snapshot,
  some,
  sovereignInstruments,
  sum,
  totalFor,
  type InstructionDraft,
  type MechanismContext,
  type Order,
  type SystemModule,
  type World,
} from '../src/index.js';

function violations(w: World): string[] {
  const r = w.last?.audit;
  if (r === undefined) return [];
  return r.families.flatMap((f) => f.violations.map((v) => `${f.family}: ${v.message}`));
}

/** A module that gives firms and banks reasons to be in the benchmark line's market. */
function traders(orders: (m: string, party: string) => Order[]): SystemModule {
  return {
    id: 'test.traders',
    spec: 'Clearing B2',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    participants: [
      { partyKind: FIRM, orders: (view, m) => orders(m.instrument, view.self.id) },
      {
        partyKind: partyId('bank') as never,
        orders: (view, m) => orders(m.instrument, view.self.id),
      },
    ],
    families: [],
  };
}

/** A module whose phase runs `fn` with the mechanism context, before the markets, every period. */
function phase(fn: (ctx: MechanismContext) => void): SystemModule {
  return {
    id: 'test.phase',
    spec: 'Clearing F1',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'test.phase',
        spec: 'Clearing F1',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run: fn,
      },
    ],
    participants: [],
    families: [],
  };
}

function withModules(seed: string, ...extra: SystemModule[]): World {
  const spec = foundationSpec(seed);
  return assemble({ ...spec, modules: [...spec.modules, ...extra] });
}

/** The bare world with extra modules: the kernel's own behaviour, driven by the test alone. */
function bareWith(seed: string, ...extra: SystemModule[]): World {
  return bare(seed, ...extra);
}

/**
 * The kernel and the opening state, with none of the mechanisms that act on it. Tests about the
 * kernel itself use this so what they measure is the kernel's, not a treasury's decisions.
 */
function bare(seed: string, ...extra: SystemModule[]): World {
  const spec = foundationSpec(seed);
  const kernelOnly = spec.modules.filter(
    (m) => m.id === 'sovereign-instruments' || m.id === 'seed.foundation',
  );
  return assemble({ ...spec, modules: [...kernelOnly, ...extra] });
}

describe('assembly (Law 15, Part XIII)', () => {
  it('orders modules by their requirements and refuses a cycle or a missing dependency', () => {
    const ordered = orderModules([foundationSeed, sovereignInstruments]);
    expect(ordered.map((m) => m.id)).toEqual(['sovereign-instruments', 'seed.foundation']);
    const orphan: SystemModule = { ...foundationSeed, id: 'x', requires: ['nope'] };
    expect(() => orderModules([orphan])).toThrow(InvalidRegistry);
  });

  it('refuses an instrument whose kind has no profile, and validates terms through the profile', () => {
    const w = foundationWorld('seed-K');
    expect(() =>
      w.instruments.add({
        id: 'x' as never,
        kind: 'unknown.kind' as never,
        issuer: some(TREASURY_NORTH),
        ccy: PHX,
        terms: { kind: 'unknown.kind' as never },
        market: none(),
      }),
    ).toThrow();
  });

  it('exposes no register write to a module or the app (Law 4: one writer)', () => {
    const w = foundationWorld('seed-W');
    expect('credit' in w.register).toBe(false);
    expect('moneyDelta' in w.register).toBe(false);
    expect(() => w.seedStore()).toThrow(Forbidden);
    expect(() => w.seal()).toThrow(Forbidden);
  });
});

describe('the seed (Seed A2)', () => {
  it('passes the audit at period zero, with every family built or saying it is not', () => {
    const w = foundationWorld('seed-A');
    const report = w.last?.audit;
    expect(report?.total).toBe(0);
    expect(report?.families.map((f) => f.family)).toHaveLength(9);
    expect(report?.families.filter((f) => !f.built).map((f) => f.family)).toEqual([
      'crossMarket',
      'zeroSum',
    ]);
    // XI-14: the opening yield, the bank's liquidity buffer and the holder's required yield.
    expect(report?.reads.placeholders).toBe(3);
    expect(report?.reads.shapes).toBe(3);
    expect(report?.reads.populations['household']).toBe(4000);
  });

  it('is reproducible from the seed value (Seed A5, Audit D3)', () => {
    const a = foundationWorld('seed-B');
    const b = foundationWorld('seed-B');
    for (let i = 0; i < 30; i += 1) {
      a.step();
      b.step();
    }
    expect(snapshot(a, { kind: 'inspector' }, 10)).toEqual(snapshot(b, { kind: 'inspector' }, 10));
    const c = foundationWorld('seed-C');
    expect(snapshot(c, { kind: 'inspector' }, 10).positions).not.toEqual(
      snapshot(foundationWorld('seed-B'), { kind: 'inspector' }, 10).positions,
    );
  });

  it('disperses endowments across cells (Seed B4) and the weights sum to the population', () => {
    const w = foundationWorld('seed-D');
    const cells = w.parties.ofKind(HOUSEHOLD);
    expect(cells.length).toBe(8);
    const weights = cells.map((c) => (c.representation === 'cell' ? c.weight : 0));
    expect(sum(weights).value).toBe(4000);
    const deposits = new Set(cells.map((c) => w.cash(c.id, PHX)));
    expect(deposits.size).toBeGreaterThan(1);
  });
});

describe('the period loop', () => {
  it('runs every phase, prints stale when nobody posts, and stays consistent over a year', () => {
    const w = bare('seed-E');
    for (let i = 0; i < 52; i += 1) {
      const r = w.step();
      expect(violations(w)).toEqual([]);
      expect(r.markets.find((m) => m.market === GOV_MARKET)?.outcome).toBe('noDemand');
    }
    const print = w.prices.printOrThrow(GOV_LINE, w.period);
    expect(print.provenance.kind).toBe('stale');
    if (print.provenance.kind === 'stale') expect(print.provenance.from).toBe(0);
  });

  it('runs a year with its mechanisms in it and stays consistent (XI-9)', () => {
    const w = foundationWorld('seed-E2');
    for (let i = 0; i < 52; i += 1) {
      w.step();
      expect(violations(w)).toEqual([]);
    }
    // The treasury funded itself: it announced, the dealers bid, and the paper was allotted.
    expect(w.journal.ofKind('auction.result').length).toBeGreaterThan(8);
    expect(w.journal.ofKind('treasury.shortfall')).toHaveLength(0);
  });

  it('pays coupons to holders of record and the cash lands in named accounts (Register E1)', () => {
    const w = bare('seed-F');
    const cbBefore = w.register.equity(CB);
    const cell = w.parties.ofKind(HOUSEHOLD)[0];
    if (cell === undefined) throw new Error('no cell');
    const hhBefore = w.cash(cell.id, PHX);
    const treasuryBefore = w.cash(TREASURY_NORTH, PHX);
    const target = w.calendar.periodOf({ y: 2026, m: 9, d: 15 });
    while (w.period < target) {
      w.step();
    }
    expect(violations(w)).toEqual([]);
    const coupons = w.ledger.all().filter((r) => r.instruction.cause === 'coupon');
    // Three bonds outstanding, each paying its first coupon to eleven holders of record.
    expect(coupons.length).toBe(3 * (3 + 8));
    expect(coupons.every((r) => r.outcome === 'settled')).toBe(true);
    expect(w.register.equity(CB)).toBeGreaterThan(cbBefore);
    expect(w.cash(cell.id, PHX)).toBeGreaterThan(hhBefore);
    expect(w.cash(TREASURY_NORTH, PHX)).toBeLessThan(treasuryBefore);
  });

  it('a phase cannot read a print the period has not produced (Clearing F1.a)', () => {
    const w = foundationWorld('seed-G');
    expect(() => w.prices.printOrThrow(GOV_LINE, period(1))).toThrow(NotYetProduced);
  });

  it('a module phase is anchored to a kernel phase and runs with a context, not the world', () => {
    const seen: string[] = [];
    const probe: SystemModule = {
      id: 'test.probe',
      spec: 'Clearing F1',
      requires: ['seed.foundation'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'probe',
          spec: 'Clearing F1',
          cycle: 1,
          anchor: { before: 'markets' },
          run: (ctx) => {
            seen.push(`${ctx.period}:${ctx.cycle}`);
            expect('credit' in ctx.register).toBe(false);
            expect(ctx.participant(TREASURY_NORTH).self.id).toBe(TREASURY_NORTH);
          },
        },
      ],
      participants: [],
      families: [],
    };
    const w = bare('seed-H', probe);
    expect(w.phases.map((p) => p.name)).toEqual([
      'corporateActions',
      'probe',
      'markets',
      'revaluation',
    ]);
    w.step();
    expect(seen).toEqual(['1:1']);
  });
});

describe('a market with reasons on both sides', () => {
  it('clears, settles paper against cash in one instruction, and revalues everyone (XI-5, D4)', () => {
    const w = bareWith(
      'seed-I',
      traders((instrument, party) => {
        if (instrument !== GOV_LINE) return [];
        if (party === 'firm.1')
          return [{ party: partyId(party), side: 'buy', price: 0.99, qty: 100 }];
        if (party === 'bank.b')
          return [{ party: partyId(party), side: 'sell', price: 0.97, qty: 100 }];
        return [];
      }),
    );
    const r = w.step();
    expect(violations(w)).toEqual([]);
    const gov = r.markets.find((m) => m.market === GOV_MARKET);
    expect(gov?.outcome).toBe('cleared');
    expect(gov?.settledVolume).toBe(100);
    expect(w.register.quantity(partyId('firm.1'), GOV_LINE)).toBe(100);
    expect(w.register.quantity(BANK_B, GOV_LINE)).toBe(50);
    const print = w.prices.printOrThrow(GOV_LINE, w.period);
    expect(print.provenance.kind).toBe('traded');
    expect([0.97, 0.99]).toContain(print.price);
  });

  it('a buyer without the cash fails the whole trade, not half of it (Register C3.b)', () => {
    const w = bareWith(
      'seed-J',
      traders((instrument, party) => {
        if (instrument !== GOV_LINE) return [];
        if (party === 'firm.2')
          return [{ party: partyId(party), side: 'buy', price: 1.2, qty: 140 }];
        if (party === 'bank.a')
          return [{ party: partyId(party), side: 'sell', price: 1.2, qty: 140 }];
        return [];
      }),
    );
    const r = w.step();
    const gov = r.markets.find((m) => m.market === GOV_MARKET);
    expect(gov?.failedTrades).toBe(1);
    expect(gov?.settledVolume).toBe(0);
    expect(w.register.quantity(partyId('firm.2'), GOV_LINE)).toBe(0);
    expect(w.cash(partyId('firm.2'), PHX)).toBe(150);
    const failed = w.ledger.all().filter((x) => x.outcome === 'failed');
    expect(failed).toHaveLength(1);
    expect(failed[0]?.outcome === 'failed' && failed[0].reason.kind).toBe('overdraftRefused');
    expect(violations(w)).toEqual([]);
  });
});

describe('participant views (Observer A4, Expectations D1)', () => {
  it('show a party its own state and the public state, and nothing of anyone else', () => {
    const w = foundationWorld('seed-L');
    w.step();
    const view = w.participantView(partyId('firm.1'));
    expect(view.self.id).toBe('firm.1');
    expect(view.cash(PHX)).toBe(200);
    expect(view.holdings().every((h) => h.holder === 'firm.1')).toBe(true);
    expect(view.print(GOV_LINE).some).toBe(true);
    const keys = Object.keys(view);
    expect(keys).not.toContain('parties');
    expect(keys).not.toContain('register');
    expect(keys).not.toContain('ledger');
    const events = view.publicEvents(100);
    expect(events.every((e) => e.public || e.subjects.includes('firm.1'))).toBe(true);
    expect(events.some((e) => e.kind === 'print')).toBe(true);
    expect(events.some((e) => e.kind === 'instruction.settled')).toBe(false);
  });
});

describe('settlement contracts', () => {
  it('refuses a cell side without a per-member amount (XI-15)', () => {
    const w = foundationWorld('seed-M');
    const cell = w.parties.ofKind(HOUSEHOLD)[0];
    if (cell === undefined) throw new Error('no cell');
    const draft: InstructionDraft = {
      legs: [
        {
          kind: 'money',
          from: { holder: partyId('firm.1'), issuer: BANK_A },
          to: { holder: cell.id, issuer: cell.bank },
          ccy: PHX,
          amount: 10,
          fromCell: none(),
          toCell: none(),
        },
      ],
      cause: 'transfer',
      reason: 'test',
    };
    expect(() => w.settlement.settle(draft, w.period, w.cycle)).toThrow(Forbidden);
  });

  it('routes reserves between banks and none within one (Money C2.a, C2.b)', () => {
    let checked = false;
    const w = withModules(
      'seed-N',
      phase((ctx) => {
        if (ctx.period !== 1) return;
        const cell = ctx.parties.ofKind(HOUSEHOLD).find((c) => c.bank === BANK_B);
        if (cell?.representation !== 'cell') throw new Error('no cell at bank b');
        const side = cellSide(cell, 0.01);
        const draft: InstructionDraft = {
          legs: [
            {
              kind: 'money',
              from: { holder: partyId('firm.1'), issuer: BANK_A },
              to: { holder: cell.id, issuer: BANK_B },
              ccy: PHX,
              amount: totalFor(cell, 0.01),
              fromCell: none(),
              toCell: side === undefined ? none() : some(side),
            },
          ],
          cause: 'transfer',
          reason: 'wages',
        };
        const reservesA = ctx.register.quantity(BANK_A, moneyInstrumentId(CB, PHX));
        const cellBefore = ctx.register.quantity(cell.id, moneyInstrumentId(BANK_B, PHX));
        const rec = ctx.settle(draft);
        expect(rec.outcome).toBe('settled');
        if (rec.outcome !== 'settled') return;
        expect(rec.reserveLegs).toHaveLength(2);
        expect(ctx.register.quantity(BANK_A, moneyInstrumentId(CB, PHX))).toBeCloseTo(
          reservesA - totalFor(cell, 0.01),
          9,
        );
        expect(ctx.register.quantity(cell.id, moneyInstrumentId(BANK_B, PHX))).toBeCloseTo(
          cellBefore + 0.01,
          12,
        );

        const same: InstructionDraft = {
          legs: [
            {
              kind: 'money',
              from: { holder: partyId('firm.1'), issuer: BANK_A },
              to: { holder: partyId('firm.2'), issuer: BANK_A },
              ccy: PHX,
              amount: 5,
              fromCell: none(),
              toCell: none(),
            },
          ],
          cause: 'transfer',
          reason: 'invoice',
        };
        const rec2 = ctx.settle(same);
        expect(rec2.outcome === 'settled' && rec2.reserveLegs).toEqual([]);
        checked = true;
      }),
    );
    w.step();
    expect(checked).toBe(true);
    expect(violations(w)).toEqual([]);
  });
});

describe('cells (XI-15)', () => {
  it('splits exactly, conserving totals, and merges identical cells back', () => {
    let fresh: string | undefined;
    let original: string | undefined;
    let weight = 0;
    let before = 0;
    const w = withModules(
      'seed-O',
      phase((ctx) => {
        const cell = ctx.parties.ofKind(HOUSEHOLD)[0];
        if (cell?.representation !== 'cell') throw new Error('no cell');
        if (ctx.period === 1) {
          original = cell.id;
          weight = cell.weight;
          before = ctx.register.totalQuantity(cell.id, GOV_LINE);
          fresh = ctx.cells.split(cell.id, 100, 'test split');
        }
        if (ctx.period === 2 && fresh !== undefined) {
          ctx.cells.merge(cell.id, fresh as never, 'test merge');
        }
      }),
    );
    w.step();
    expect(violations(w)).toEqual([]);
    if (original === undefined || fresh === undefined) throw new Error('split did not run');
    expect(w.parties.cell(original as never).weight).toBe(weight - 100);
    expect(w.parties.cell(fresh as never).weight).toBe(100);
    expect(
      w.register.totalQuantity(original as never, GOV_LINE) +
        w.register.totalQuantity(fresh as never, GOV_LINE),
    ).toBeCloseTo(before, 9);
    w.step();
    expect(violations(w)).toEqual([]);
    expect(w.parties.cell(original as never).weight).toBe(weight);
    expect(w.parties.get(fresh as never).status.alive).toBe(false);
    expect(w.register.totalQuantity(original as never, GOV_LINE)).toBeCloseTo(before, 9);
  });

  it('a weight changes only by the five events, and never to nobody', () => {
    const w = foundationWorld('seed-P');
    const cell = w.parties.ofKind(HOUSEHOLD)[0];
    if (cell?.representation !== 'cell') throw new Error('no cell');
    const ctx = w.mechanismContext('test');
    ctx.cells.weight(cell.id, 'death', 100, 'test');
    expect(w.parties.cell(cell.id).weight).toBe(cell.weight - 100);
    expect(() => {
      ctx.cells.weight(cell.id, 'death', cell.weight - 100, 'test');
    }).toThrow(Forbidden);
    expect(() => ctx.cells.split(cell.id, cell.weight - 100, 'test')).toThrow(Forbidden);
  });
});
