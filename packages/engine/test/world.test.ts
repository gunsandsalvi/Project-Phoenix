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
import { unexpected } from './expected.js';
import { phx } from './units.js';

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

/** The bare world where no bank will lend a penny: every limit for every name is nothing (C3). */
function noLending(seed: string, ...extra: SystemModule[]): World {
  const spec = foundationSpec(seed);
  const modules = spec.modules
    .filter(
      (m) =>
        m.id === 'sovereign-instruments' ||
        m.id === 'seed.foundation' ||
        m.id === 'bank-lending' ||
        m.id === 'money-market',
    )
    .map((m) => ({
      ...m,
      params: m.params.map((p) =>
        p.id.startsWith('bank.limitPerBorrower.') ? { ...p, value: 0 } : p,
      ),
    }));
  return assemble({ ...spec, modules: [...modules, ...extra] });
}

/**
 * The kernel and the opening state, with none of the mechanisms that act on it except the two that
 * answer Money B3.a — a bank's overdraft is its bank's credit decision and a bank's own overdraft at
 * the central bank is the corridor's, and a world where either kind has nobody to answer cannot be
 * sealed. Tests about the kernel itself use this so what they measure is the kernel's, not a
 * treasury's decisions.
 */
function bare(seed: string, ...extra: SystemModule[]): World {
  const spec = foundationSpec(seed);
  const kernelOnly = spec.modules.filter(
    (m) =>
      m.id === 'sovereign-instruments' ||
      m.id === 'seed.foundation' ||
      m.id === 'bank-lending' ||
      m.id === 'money-market',
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
    // XI-14: one placeholder stands, and it names the worklist item that deletes it — the bank's
    // liquidity buffer (11). The holder's required yield went with the cost of capital (10): what a
    // bank requires to hold paper is now built from its own cost of funds and the capital the
    // position consumes, so there is nothing left standing in for it.
    expect(report?.reads.placeholders).toBe(1);
    // And eight shapes. Five of them are the levels the world opens at (Seed C4) — the four goods
    // and the opening yield: a market that has never traded has no price, so a world that opens
    // with stock in it opens with a level for that stock, and no worklist item will ever delete
    // that, which is why they are shapes and not placeholders with a death nobody could keep. The
    // sixth is the width of the one preference whose dispersion is still stated (§46 B1.a). What is
    // unequal about households is not here: it is what happened to them. The last two are the two
    // management fees, one per fund: what a manager charges is what competition among managers
    // settles at (worklist 13h), and until there is any, it is a claim about the answer rather than
    // a number this world produced.
    expect(report?.reads.shapes).toBe(8);
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
    // A different seed value is a different world: nothing about the OPENING state is drawn any
    // more — the seed states no dispersion at all — so what differs is what the parties then did,
    // starting from the memories they were each given (§46 B1.a).
    const c = foundationWorld('seed-C');
    for (let i = 0; i < 30; i += 1) c.step();
    expect(snapshot(c, { kind: 'inspector' }, 10).positions).not.toEqual(
      snapshot(a, { kind: 'inspector' }, 10).positions,
    );
  });

  it('states no dispersion and produces one: the cells start equal and do not stay so (Seed B4)', () => {
    const w = foundationWorld('seed-D');
    const cells = w.parties.ofKind(HOUSEHOLD);
    expect(cells.length).toBe(8);
    const weights = cells.map((c) => (c.representation === 'cell' ? c.weight : 0));
    expect(sum(weights).value).toBe(4000);
    // Seed E: the seed decides nothing about how rich anybody is. Every cell opens with nothing.
    expect(new Set(cells.map((c) => w.cash(c.id, PHX)))).toEqual(new Set([0]));
    for (let i = 0; i < 8; i += 1) w.step();
    // B4: and a sector of equals never produces a market — so the dispersion has to come from
    // somewhere. It comes from what happened: who was hired, at what wage, and what each of them
    // did with it. It is an outcome now, where it used to be a number the seed stated.
    const alive = w.parties.ofKind(HOUSEHOLD).filter((c) => c.status.alive);
    expect(new Set(alive.map((c) => w.cash(c.id, PHX))).size).toBeGreaterThan(1);
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
      expect(unexpected(w.step().audit)).toEqual([]);
    }
    // The treasury funded itself: it announced, the dealers bid, and the paper was allotted.
    expect(w.journal.ofKind('auction.result').length).toBeGreaterThan(8);
    // XI-9, Treasury D3: nothing advanced it, ever. Where it could not pay, the mandate simply
    // went unpaid and the shortfall is a recorded event the next programme reads — which is the
    // whole of the constraint, and it is what a treasury with an overdraft would never show.
    const advances = w.journal
      .ofKind('reserve.overdraft')
      .filter((e) => e.subjects.includes(TREASURY_NORTH));
    expect(advances).toHaveLength(0);
    for (const e of w.journal.ofKind('treasury.shortfall')) {
      expect(Number(e.data['unpaid'])).toBeGreaterThan(0);
    }
    // And it came back: what it could not pay out of an empty account it funded at the next
    // auction, so the constraint bites and then releases rather than ending the world.
    expect(w.cash(TREASURY_NORTH, PHX)).toBeGreaterThan(0);
  });

  it('runs a year of the whole chain, and every family it has built is green (Part XII)', () => {
    const w = foundationWorld('seed-E3');
    // Nothing is forgiven here any more. Until the corridor existed this world reported one red
    // every auction cycle — a bank that paid for what it won out of reserves its own customers had
    // already moved, with nobody able to lend it the difference overnight (Money B3.b). The window
    // is what a bank goes to for that (Money Market C, worklist 11), and now that it is there the
    // whole year is clean: every family the world has built, every period, no exceptions.
    for (let i = 0; i < 52; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const report = w.last?.audit;
    // Seven families are built and green, named so a green run says what it checked; two say they
    // are NOT BUILT rather than being green by omission (Audit C2.a) — cross-market arbitrage needs
    // a second venue for one thing (worklist 12) and zero-sum needs the derivative layer (13a).
    // Seven families are built; the money one is built AND currently red for the reason above.
    expect(report?.families.filter((f) => f.built).map((f) => f.family)).toEqual([
      'money',
      'ownership',
      'prices',
      'accounts',
      'names',
      'flows',
      'units',
    ]);
    expect(report?.families.filter((f) => !f.built).map((f) => f.family)).toEqual([
      'crossMarket',
      'zeroSum',
    ]);
    // And the chain really ran the whole way: batches were started out of a recipe, people were
    // hired and paid, and households bought the finished good from a named seller.
    expect(w.journal.ofKind('firms.started').length).toBeGreaterThan(0);
    expect(w.journal.ofKind('labour.hire').length).toBeGreaterThan(0);
    expect(w.journal.ofKind('labour.wages').length).toBeGreaterThan(0);
    // A GOOD, not any asset: this counted sovereign paper until item 4a, so it passed while the
    // households were buying no food at all. What it is for is the last link of the chain.
    const cells = new Set(w.parties.ofKind(HOUSEHOLD).map((p) => p.id));
    const ate = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled')
      .flatMap((r) => r.instruction.legs)
      .filter(
        (l) =>
          l.kind === 'asset' &&
          cells.has(l.to) &&
          w.registry.instrumentKind(w.instruments.get(l.instrument).kind).physical === true,
      );
    expect(ate.length).toBeGreaterThan(0);
  });

  it('pays coupons to holders of record and the cash lands in named accounts (Register E1)', () => {
    const w = bare('seed-F');
    const cell = w.parties.ofKind(HOUSEHOLD)[0];
    if (cell === undefined) throw new Error('no cell');
    const treasuryBefore = w.cash(TREASURY_NORTH, PHX);
    const target = w.calendar.periodOf({ y: 2026, m: 9, d: 15 });
    while (w.period < target) {
      w.step();
    }
    expect(violations(w)).toEqual([]);
    // A coupon is what a LINE pays its holders of record, and in a world with a funding market
    // the sovereign is not the only payer of one: a bank pays its depositors and the central bank
    // pays for the cash it took in overnight, both by the same cause. This is about the bond.
    const coupons = w.ledger
      .all()
      .filter(
        (r) =>
          r.instruction.cause === 'coupon' &&
          r.instruction.reason.startsWith(`coupon on ${GOV_LINE}`),
      );
    // E1: one instruction per holder of record per bond that paid — the register says who they
    // are, and nobody who was not holding it is paid anything.
    expect(coupons.length).toBeGreaterThan(0);
    expect(coupons.every((r) => r.outcome === 'settled')).toBe(true);
    let reachedCb = 0;
    for (const r of coupons) {
      for (const leg of r.instruction.legs) {
        if (leg.kind !== 'money') continue;
        expect(leg.from.holder).toBe(TREASURY_NORTH);
        expect(leg.to.holder).not.toBe(TREASURY_NORTH);
        if (leg.to.holder === CB) reachedCb += leg.amount;
      }
    }
    // The central bank is a holder of record too and the coupon reached it. Its EQUITY is not the
    // read for that any more: with a corridor in the world it also pays for the reserves parked
    // with it overnight (Central Bank B2, D1), so its own funding cost is inside that number.
    expect(reachedCb).toBeGreaterThan(0);
    // A holder of record was paid: the banks hold this paper, and what reached them is the coupon
    // on what the register says they held (E1). Its ACCOUNT is not the read for that any more —
    // a bank lends its spare cash to the central bank overnight at the floor and the balance leaves
    // the banking system with it (Money Market C1.a) — so this is what landed, not what stayed.
    let reachedBank = 0;
    for (const r of coupons) {
      for (const leg of r.instruction.legs) {
        if (leg.kind === 'money' && leg.to.holder === BANK_A) reachedBank += leg.amount;
      }
    }
    expect(reachedBank).toBeGreaterThan(0);
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
    // The bare world carries the lending and money-market modules too: an overdraft at a bank and
    // an overdraft at the central bank are both credit decisions (Money B3.a), and a world with
    // nobody to take either cannot be sealed. Their phases are here, in the order assembly gave.
    expect(w.phases.map((p) => p.name)).toEqual([
      'corporateActions',
      'lending.write',
      'moneyMarket.rates',
      'probe',
      'markets',
      'moneyMarket.clear',
      'lending.book',
      'moneyMarket.book',
      // Banks Capital C3.b: a bank does not go to an estate, so this is where it goes instead —
      // just before the marks are taken, so the book it moves is re-marked on the acquirer's own
      // balance sheet in the same period it lands there.
      'moneyMarket.resolve',
      'revaluation',
      // Banks Capital A1, B1: capital is the residual, so a bank can only know what its own is
      // once the marks are in — and what it may lend next period is what it closed this one with.
      'banks.capital',
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
        // Sized to the cash the buyer holds: what this tests is delivery against payment, not a
        // buyer that cannot pay — that is the next test.
        if (party === 'firm.1')
          return [{ party: partyId(party), side: 'buy', price: 0.99, qty: phx(60_000) }];
        if (party === 'bank.b')
          return [{ party: partyId(party), side: 'sell', price: 0.97, qty: phx(60_000) }];
        return [];
      }),
    );
    const r = w.step();
    expect(violations(w)).toEqual([]);
    const gov = r.markets.find((m) => m.market === GOV_MARKET);
    expect(gov?.outcome).toBe('cleared');
    expect(gov?.settledVolume).toBe(phx(60_000));
    expect(w.register.quantity(partyId('firm.1'), GOV_LINE)).toBe(phx(60_000));
    expect(w.register.quantity(BANK_B, GOV_LINE)).toBe(phx(90_000));
    const print = w.prices.printOrThrow(GOV_LINE, w.period);
    expect(print.provenance.kind).toBe('traded');
    expect([0.97, 0.99]).toContain(print.price);
  });

  it('a buyer without the cash fails the whole trade, not half of it (Register C3.b)', () => {
    // Its bank will lend it nothing, so the shortfall is a refusal and not an overdraft (B3.a).
    // A bank with no appetite for a name is a real bank, and it is what makes a fail a fail.
    const w = noLending(
      'seed-J',
      traders((instrument, party) => {
        if (instrument !== GOV_LINE) return [];
        if (party === 'firm.2')
          return [{ party: partyId(party), side: 'buy', price: 1.2, qty: phx(140_000) }];
        if (party === 'bank.a')
          return [{ party: partyId(party), side: 'sell', price: 1.2, qty: phx(140_000) }];
        return [];
      }),
    );
    const r = w.step();
    const gov = r.markets.find((m) => m.market === GOV_MARKET);
    expect(gov?.failedTrades).toBe(1);
    expect(gov?.settledVolume).toBe(0);
    expect(w.register.quantity(partyId('firm.2'), GOV_LINE)).toBe(0);
    expect(w.cash(partyId('firm.2'), PHX)).toBe(phx(50_000));
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
    expect(view.cash(PHX)).toBe(phx(65_000));
    expect(view.holdings().every((h) => h.holder === 'firm.1')).toBe(true);
    expect(view.print(GOV_LINE).some).toBe(true);
    const keys = Object.keys(view);
    // WHO somebody is, is public — a market knows whose paper it trades (Observer A3) — and what
    // anybody holds, owes or has been paid is not: there is no register and no ledger here.
    expect(view.parties.get(partyId('bank.a')).name).toBe('Bank A');
    // And it cannot change who is here: the facade is a real one, at runtime as well as in types.
    expect(Object.keys(view.parties)).not.toContain('add');
    expect(Object.keys(view.parties)).not.toContain('cease');
    expect(Object.isFrozen(view.parties)).toBe(true);
    expect(keys).not.toContain('register');
    expect(keys).not.toContain('ledger');
    expect(keys).not.toContain('valuation');
    const events = view.publicEvents(100);
    expect(events.every((e) => e.public || e.subjects.includes('firm.1'))).toBe(true);
    expect(events.some((e) => e.kind === 'print')).toBe(true);
    // What it sees of the wire is its own: an instruction it was a side of names it, and one it
    // was not is not here at all (A4).
    for (const e of events.filter((x) => x.kind === 'instruction.settled')) {
      expect(e.subjects).toContain('firm.1');
    }
  });
});

describe('the observer surface (Observer A2, A4, D3)', () => {
  it('shows the inspector what the modules keep, and a party only its own outlook', () => {
    const w = foundationWorld('seed-O');
    for (let i = 0; i < 6; i += 1) w.step();
    const inspector = snapshot(w, { kind: 'inspector' }, 10);
    // A4: the inspector's product is the whole of it — the employment rows, the inventories a
    // module tracks, the outlook book. Derived at the read; the engine never reads it back (E3).
    expect(inspector.state).not.toBeNull();
    expect(Object.keys(inspector.state ?? {})).toContain('labour/employment');
    expect(inspector.outlooks.length).toBeGreaterThan(0);
    // XI-14: every number that shapes behaviour is declared, and the placeholders name their death.
    expect(inspector.params.placeholders.every((x) => x.worklistItem.length > 0)).toBe(true);
    // A2: a party sees its own outlooks and nobody else's, and no module state at all.
    const firm = partyId('firm.1');
    const own = snapshot(w, { kind: 'party', party: firm }, 10);
    expect(own.state).toBeNull();
    expect(own.outlooks.every((o) => o.party === firm)).toBe(true);
    expect(own.outlooks.length).toBeGreaterThan(0);
    expect(inspector.outlooks.some((o) => o.party !== firm)).toBe(true);
    // A2.b: what it expects is its own number, with its own confidence — there is no consensus row.
    expect(own.outlooks.every((o) => o.unit.length > 0 && o.confidence >= 0)).toBe(true);
  });

  it('reaches what is said once a period however much else was said (B1, D1)', () => {
    const w = foundationWorld('seed-O2');
    for (let i = 0; i < 6; i += 1) w.step();
    // A period says far more than ten things, so the programme the treasury published this period
    // is nowhere near the back of a ten-deep feed of everything.
    const kind = 'treasury.programme';
    const shallow = snapshot(w, { kind: 'inspector' }, 10, [kind]);
    expect(shallow.journal.some((e) => e.kind === kind)).toBe(false);
    const followed = shallow.followed[kind];
    expect(followed?.length).toBe(w.journal.ofKind(kind).length);
    expect(followed?.[followed.length - 1]?.period).toBe(w.period);
    // A3, A4: it comes back by the same visibility rule as the feed — a party gets what is public
    // and what names it, and nothing else, whatever kind it asks for.
    const firm = partyId('firm.1');
    const settled = 'instruction.settled';
    const party = snapshot(w, { kind: 'party', party: firm }, 10, [kind, settled]);
    expect(party.followed[kind]?.length).toBe(followed?.length);
    expect(party.followed[settled]?.every((e) => e.public || e.subjects.includes(firm))).toBe(true);
    expect(
      snapshot(w, { kind: 'inspector' }, 10, [settled]).followed[settled]?.some(
        (e) => !e.subjects.includes(firm),
      ),
    ).toBe(true);
    // A viewer that names no kind follows none: the surface adds nothing of its own (E3).
    expect(snapshot(w, { kind: 'inspector' }, 10).followed).toEqual({});
  });
});

/**
 * Law 8: a test pays real money, so it pays a whole number of the smallest piece of it — a count of
 * cents. Half a cent is not one, and the wire refuses it, which is the rule doing its job on the
 * person writing the test as much as on the engine.
 */
const PAYMENT = phx(1);

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
        const side = cellSide(cell, PAYMENT);
        const draft: InstructionDraft = {
          legs: [
            {
              kind: 'money',
              from: { holder: partyId('firm.1'), issuer: BANK_A },
              to: { holder: cell.id, issuer: BANK_B },
              ccy: PHX,
              amount: totalFor(cell, PAYMENT),
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
        // Counts of cents, so the two sides are EXACTLY equal and no closeness is asked for.
        expect(ctx.register.quantity(BANK_A, moneyInstrumentId(CB, PHX))).toBe(
          reservesA - totalFor(cell, PAYMENT),
        );
        expect(ctx.register.quantity(cell.id, moneyInstrumentId(BANK_B, PHX))).toBe(
          cellBefore + PAYMENT,
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
    // The kernel's own behaviour and nothing else: a world where somebody is also hiring out of
    // this cell would be measuring the labour market, not the split.
    const w = bareWith(
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
