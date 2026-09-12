import { describe, expect, it } from 'vitest';
import {
  type ParticipantView,
  div,
  mul,
  upTick,
  downToTick,
  upToTick,
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
  USD,
  TREASURY_US,
  assemble,
  cellSide,
  foundationSeedFor,
  drawBanks,
  drawFirms,
  BANK_COUNT,
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
import { RIG_FIRMS, rigWorld, rigSpec, withDependencies, mergeModules, rigMembers } from './rig.js';
import { unexpected } from './expected.js';
import { notDealing } from './no-dealing.js';
import { phx } from './units.js';

function violations(w: World): string[] {
  const r = w.last?.audit;
  if (r === undefined) return [];
  return r.families.flatMap((f) => f.violations.map((v) => `${f.family}: ${v.message}`));
}

/** A module that gives firms and banks reasons to be in the benchmark line's market. */
/**
 * PLAN §7: the closure is handed the party's own VIEW, so a test can size an order off what that
 * party actually holds rather than off an amount it wrote down. Which is what "a buyer without the
 * cash" has to mean now that 11.5 derives this world's scale: how much a firm opens with is an
 * outcome of the seed's arithmetic, and a bid stated in advance is either far under it (so the
 * buyer pays easily and the test tests nothing) or far over it by luck.
 */
function traders(orders: (m: string, party: string, view: ParticipantView) => Order[]): SystemModule {
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
      { partyKind: FIRM, orders: (view, m) => orders(m.instrument, view.self.id, view) },
      {
        partyKind: partyId('bank') as never,
        orders: (view, m) => orders(m.instrument, view.self.id, view),
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
  const spec = rigSpec(seed);
  return assemble({ ...spec, modules: mergeModules(spec.modules, extra) });
}

/** The bare world with extra modules: the kernel's own behaviour, driven by the test alone. */
function bareWith(seed: string, ...extra: SystemModule[]): World {
  return bare(seed, ...extra);
}

/** The bare world where no bank will lend a penny: every limit for every name is nothing (C3). */
function noLending(seed: string, ...extra: SystemModule[]): World {
  const spec = rigSpec(seed);
  const modules = withDependencies(
    spec.modules,
    (m) =>
      m.id === 'sovereign-instruments' ||
      m.id === 'seed.foundation' ||
      m.id === 'seed.funding' ||
      m.id === 'banks' ||
      m.id === 'money-market',
  ).map((m) => ({
      ...notDealing(m),
      params: m.params.map((p) =>
        p.id.startsWith('bank.limitPerBorrower.') ? { ...p, value: 0 } : p,
      ),
    }));
  return assemble({ ...spec, modules: mergeModules(modules, extra) });
}


/**
 * The kernel and the opening state, with none of the mechanisms that act on it except the two that
 * answer Money B3.a — a bank's overdraft is its bank's credit decision and a bank's own overdraft at
 * the central bank is the corridor's, and a world where either kind has nobody to answer cannot be
 * sealed. Tests about the kernel itself use this so what they measure is the kernel's, not a
 * treasury's decisions.
 */
function bare(seed: string, ...extra: SystemModule[]): World {
  const spec = rigSpec(seed);
  const kernelOnly = withDependencies(
    spec.modules,
    (m) =>
      m.id === 'sovereign-instruments' ||
      m.id === 'seed.foundation' ||
      m.id === 'seed.funding' ||
      m.id === 'banks' ||
      m.id === 'money-market',
  ).map(notDealing);
  return assemble({ ...spec, modules: mergeModules(kernelOnly, extra) });
}

describe('assembly (Law 15, Part XIII)', () => {
  it('orders modules by their requirements and refuses a cycle or a missing dependency', () => {
    // Two modules, one of which requires the other: the order is the dependency's, whatever order
    // they were handed in. A module with requirements nobody supplied is the other half of the same
    // rule, and both are what Part XIII means by an order that is declared rather than written.
    const seedOnly: SystemModule = {
      ...foundationSeedFor(drawBanks(BANK_COUNT, 'order'), drawFirms(RIG_FIRMS, 'order')),
      requires: ['sovereign-instruments'],
    };
    const ordered = orderModules([seedOnly, sovereignInstruments]);
    expect(ordered.map((m) => m.id)).toEqual(['sovereign-instruments', 'seed.foundation']);
    const orphan: SystemModule = { ...foundationSeedFor(drawBanks(BANK_COUNT, 'order'), drawFirms(RIG_FIRMS, 'order')), id: 'x', requires: ['nope'] };
    expect(() => orderModules([orphan])).toThrow(InvalidRegistry);
  });

  it('refuses an instrument whose kind has no profile, and validates terms through the profile', () => {
    const w = rigWorld('seed-K');
    expect(() =>
      w.instruments.add({
        id: 'x' as never,
        kind: 'unknown.kind' as never,
        issuer: some(TREASURY_US),
        ccy: USD,
        terms: { kind: 'unknown.kind' as never },
        market: none(),
      }),
    ).toThrow();
  });

  it('exposes no register write to a module or the app (Law 4: one writer)', () => {
    const w = rigWorld('seed-W');
    expect('credit' in w.register).toBe(false);
    expect('moneyDelta' in w.register).toBe(false);
    expect(() => w.seedStore()).toThrow(Forbidden);
    expect(() => w.seal()).toThrow(Forbidden);
  });
});

describe('the seed (Seed A2)', () => {
  it('passes the audit at period zero, with every family built or saying it is not', () => {
    const w = rigWorld('seed-A');
    const report = w.last?.audit;
    expect(report?.total).toBe(0);
    expect(report?.families.map((f) => f.family)).toHaveLength(9);
    // Audit A3: an unbuilt family reports "not built" and NEVER green — and THERE ARE NONE LEFT.
    // `crossMarket` was built at item 12 (the triangular gap and an index against its own
    // constituents); `zeroSum` is built at 13a, because the contract store is a kernel store from
    // there on and the identity that the two sides of a contract negate is checkable in any world,
    // including one with no contracts in it.
    expect(report?.families.filter((f) => !f.built).map((f) => f.family)).toEqual([]);
    // XI-14: TWO PLACEHOLDERS STAND, and they are the two management fees. The bank's liquidity
    // buffer was the last one anybody counted — a stated share of the money it had issued, standing
    // in for a decision nobody had built — and 11.2 built the decision (Banks Funding C2). The fees
    // were never counted, because their death was written in prose: both said "worklist 13h" in
    // their reason and both were declared shapes, so the field guards passed and the honest measure
    // read zero. A shape with a scheduled death IS a placeholder (Law 2), and the register now
    // refuses the other way round.
    // SEVEN, and it goes up and down for honest reasons: 13b declares a TRACKER PER INDEX RULE
    // (Indices B2), so this world has four exchange-traded funds where it had one, and each of them
    // carries a management fee standing in for the same mechanism at the same item; the seventh is
    // the money fund's, and WHICH bank it was launched at is an outcome of the draw (Seed B1.a), so
    // a world redrawn is a world with a differently named fee. A count that stayed put while the
    // world grew more fees would be the register not counting.
    //
    // The one that went earlier is still the point of counting them: `seed.centralBank.
    // openingHoldingShare` named 11.5 as the item that would kill it, 11.5 came, and it is a POLICY
    // the central bank takes now rather than a share somebody stated. A placeholder that dies on
    // the item it named is the register working exactly as XI-14 asks.
    expect(report?.reads.placeholders).toBe(7);
    // PLAN §7: what each stands in for and which item kills it, never the id — a fund's own id
    // carries the bank it was launched at, and WHICH bank is an outcome of the draw (Seed B1.a).
    expect(
      snapshot(w, { kind: 'inspector' }, 10)
        .params.placeholders.map((p) => `${p.mechanism} at ${p.worklistItem}`)
        .sort(),
    ).toEqual([
      'Central Bank F4 at 13h',
      'Fund Shares F3 at 13h',
      'Fund Shares F3 at 13h',
      'Fund Shares F3 at 13h',
      'Fund Shares F3 at 13h',
      'Fund Shares F3 at 13h',
      'Households A5 at 13f',
    ]);
    // XI-14: EIGHTEEN SHAPES, and the count is honest in both directions — it went UP because this
    // world says more, and two of what used to be shapes became placeholders because a worklist
    // item is now named that kills them (Law 2: a shape with a scheduled death IS a placeholder).
    //
    // Five are the levels the world opens at (Seed C4) — the four goods and the opening yield: a
    // market that has never traded has no price, so a world that opens with stock in it opens with
    // a level for that stock, and no worklist item will ever delete that. One is the width of the
    // one preference whose dispersion is still stated (§46 B1.a). One is the level every currency
    // pair opens at, for the same reason the goods do (Spot FX C1). Six are the three assessors'
    // methodologies — where each puts its first grade boundary and how fast its bands widen — and
    // those are shapes about an answer that only a measurement can kill (Ratings E3, Part XII).
    // The rest are the seed's own: the treasury's buffer share, what a firm opens holding in cash
    // and in plant, and the household share of the sovereign's debt.
    expect(report?.reads.shapes).toBe(18);
    // XI-15: the population is the rig's, and the rig keeps the real world's ratio of people to
    // firms — so it is READ from the same place the world was built from rather than written down
    // again here. Two cohorts hold it between them; how many (cohort, bank) keys it is spread over
    // is where it is REPRESENTED and changes nothing about how many people there are.
    expect(report?.reads.populations['household']).toBe(rigMembers(RIG_FIRMS) * 2);
  });

  it('is reproducible from the seed value (Seed A5, Audit D3)', () => {
    const a = rigWorld('seed-B');
    const b = rigWorld('seed-B');
    for (let i = 0; i < 30; i += 1) {
      a.step();
      b.step();
    }
    expect(snapshot(a, { kind: 'inspector' }, 10)).toEqual(snapshot(b, { kind: 'inspector' }, 10));
    // A different seed value is a different world: nothing about the OPENING state is drawn any
    // more — the seed states no dispersion at all — so what differs is what the parties then did,
    // starting from the memories they were each given (§46 B1.a).
    const c = rigWorld('seed-C');
    for (let i = 0; i < 30; i += 1) c.step();
    expect(snapshot(c, { kind: 'inspector' }, 10).positions).not.toEqual(
      snapshot(a, { kind: 'inspector' }, 10).positions,
    );
  });

  it('states no dispersion and produces one: the cells start equal and do not stay so (Seed B4)', () => {
    const w = rigWorld('seed-D');
    const cells = w.parties.ofKind(HOUSEHOLD);
    expect(cells.length).toBe(12);
    const weights = cells.map((c) => (c.representation === 'cell' ? c.weight : 0));
    // XI-15: how many people there are is the rig's, read from where the world was built rather
    // than written down a second time; the twelve cells above are where they are REPRESENTED.
    expect(sum(weights).value).toBe(rigMembers(RIG_FIRMS) * 2);
    // Seed E, Banks Capital B1.b: the seed decides nothing about how rich anybody is — WITHIN A
    // BANK'S DEPOSITORS, which is where the claim can be made now. It used to be "every cell opens
    // with nothing", and that stopped being true when `seed.funding` was built (item 12): a bank's
    // balance sheet follows its depositors, so what a cell opens holding is the residue its own
    // bank needs funding and not a number anybody chose for it. Two banks need different amounts;
    // two cells at ONE bank are identical, and that is the statement the seed is making.
    for (const bank of new Set(cells.map((c) => String(c.bank)))) {
      const here = cells.filter((c) => String(c.bank) === bank);
      expect(new Set(here.map((c) => w.cash(c.id, USD))).size, `cells at ${bank} differ`).toBe(1);
    }
    for (let i = 0; i < 8; i += 1) w.step();
    // B4: and a sector of equals never produces a market — so the dispersion has to come from
    // somewhere. It comes from what happened: who was hired, at what wage, and what each of them
    // did with it. It is an outcome now, where it used to be a number the seed stated.
    const alive = w.parties.ofKind(HOUSEHOLD).filter((c) => c.status.alive);
    expect(new Set(alive.map((c) => w.cash(c.id, USD))).size).toBeGreaterThan(1);
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
    const w = rigWorld('seed-E2');
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
      .filter((e) => e.subjects.includes(TREASURY_US));
    expect(advances).toHaveLength(0);
    for (const e of w.journal.ofKind('treasury.shortfall')) {
      expect(Number(e.data['unpaid'])).toBeGreaterThan(0);
    }
    // And it came back: what it could not pay out of an empty account it funded at the next
    // auction, so the constraint bites and then releases rather than ending the world.
    expect(w.cash(TREASURY_US, USD)).toBeGreaterThan(0);
  });

  it('runs a year of the whole chain, and every family it has built is green (Part XII)', () => {
    const w = rigWorld('seed-E3');
    // Nothing is forgiven here any more. Until the corridor existed this world reported one red
    // every auction cycle — a bank that paid for what it won out of reserves its own customers had
    // already moved, with nobody able to lend it the difference overnight (Money B3.b). The window
    // is what a bank goes to for that (Money Market C, worklist 11), and now that it is there the
    // whole year is clean: every family the world has built, every period, no exceptions.
    for (let i = 0; i < 52; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    const report = w.last?.audit;
    // ALL NINE are built and green, named so a green run says what it checked. `crossMarket`
    // joined them at item 12 (the triangular gap and an index against its own constituents) and
    // `zeroSum` at 13a: the contract store is a kernel store from there on, so the identity that
    // the two sides of a contract negate is checkable in any world — including this one, which has
    // no contracts in it and says so by holding vacuously rather than by not being built.
    expect(report?.families.filter((f) => f.built).map((f) => f.family)).toEqual([
      'money',
      'ownership',
      'prices',
      'crossMarket',
      'accounts',
      'names',
      'flows',
      'zeroSum',
      'units',
    ]);
    expect(report?.families.filter((f) => !f.built).map((f) => f.family)).toEqual([]);
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
    const treasuryBefore = w.cash(TREASURY_US, USD);
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
        expect(leg.from.holder).toBe(TREASURY_US);
        expect(leg.to.holder).not.toBe(TREASURY_US);
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
    expect(w.cash(TREASURY_US, USD)).toBeLessThan(treasuryBefore);
  });

  it('a phase cannot read a print the period has not produced (Clearing F1.a)', () => {
    const w = rigWorld('seed-G');
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
            expect(ctx.participant(TREASURY_US).self.id).toBe(TREASURY_US);
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
      // Banks Capital C2: recapitalisation FIRST — a bank short of capital asks for it before it
      // decides what else to put on its book with what is left.
      'banks.raise',
      'lending.write',
      // Banks Funding B1.a: its board, priced off what money costs it and off its rivals' boards.
      'banks.treasury',
      // ...and then the market PAYS at the rates the banks announced, and the depositors answer.
      'moneyMarket.rates',
      // Dealer Desks D1, Fund Shares E3: the bank's own dealing line, after it has published what
      // money costs it — that is the number that says whether closing a gap is worth doing.
      'banks.arbitrage',
      'probe',
      'markets',
      // Banks Funding C2.a: what its own account did to it this week, and what it therefore holds
      // against a bad one — taken after the flows and before the session that is measured against it.
      'banks.buffer',
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
      // D5, E4: and what its dealing book came to, once the marks are in it.
      'banks.dealing',
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
    // What the seller was holding before the session: how much the seed gives it is an outcome of
    // the seed's own arithmetic (11.5 derives this world's scale), and what DvP owes this test is
    // that the paper which left one side arrived at the other — which is a difference, not a level.
    const sellerHad = w.register.quantity(BANK_B, GOV_LINE);
    const r = w.step();
    expect(violations(w)).toEqual([]);
    const gov = r.markets.find((m) => m.market === GOV_MARKET);
    expect(gov?.outcome).toBe('cleared');
    expect(gov?.settledVolume).toBe(phx(60_000));
    expect(w.register.quantity(partyId('firm.1'), GOV_LINE)).toBe(phx(60_000));
    expect(w.register.quantity(BANK_B, GOV_LINE)).toBe(sellerHad - phx(60_000));
    const print = w.prices.printOrThrow(GOV_LINE, w.period);
    expect(print.provenance.kind).toBe('traded');
    // Clearing C4.c, Law 8: the level is one somebody POSTED, on this market's own grid. A buy is
    // the most it will pay so it goes down to a tick, a sell the least it will accept so it goes up
    // — and `n x tick` carries one rounding, because a ten-thousandth is not a binary fraction
    // (12b.1 says so where the doors are). So the two levels this test posted are what it compares
    // against, gridded the way the book grids them, rather than the two numbers it typed.
    const tick = w.registry.tickFor(w.instruments.get(GOV_LINE).kind, USD);
    expect([upToTick(0.97, tick), downToTick(0.99, tick)]).toContain(print.price);
  });

  it('refuses a party on both sides of one book at crossing prices (Clearing A2, Register D2)', () => {
    // A2: NOBODY TRADES WITH ITSELF. A party is welcome on both sides — that is what a quote is —
    // but a bid at or above its own offer is a party paying itself, and the fill between them
    // would be an instruction with one named side. It is a CONTRACT and it throws at the site: a
    // solver that stepped around it would hide the module that put two deciders behind one name.
    const w = bareWith(
      'seed-selfcross',
      traders((instrument, party) => {
        if (instrument !== GOV_LINE || party !== 'bank.a') return [];
        return [
          { party: BANK_A, side: 'buy', price: 1.2, qty: phx(10_000) },
          { party: BANK_A, side: 'sell', price: 0.8, qty: phx(10_000) },
        ];
      }),
    );
    expect(() => w.step()).toThrow(/both sides/);
  });

  it('a buyer without the cash fails the whole trade, not half of it (Register C3.b)', () => {
    // Its bank will lend it nothing, so the shortfall is a refusal and not an overdraft (B3.a).
    // A bank with no appetite for a name is a real bank, and it is what makes a fail a fail.
    const w = noLending(
      'seed-J',
      traders((instrument, party, view) => {
        if (instrument !== GOV_LINE) return [];
        // Sized to what the seller actually holds: what this tests is a buyer that cannot pay,
        // and a seller that cannot deliver would fail the trade one step earlier for another reason.
        // Register C3.b: MORE THAN ITS MONEY REACHES, read off its own account — twice what it
        // could pay for at the level it is bidding, so the whole trade has to fail and not a part
        // of it. The SELLER offers everything it holds, so the buyer is the short side and the
        // volume that clears is the buyer's own over-bid: a seller that could not deliver would
        // fail the trade one step earlier, for a different reason.
        if (party === 'firm.2') {
          const beyond = upTick(mul(div(view.cash(USD), 1.2, 'what its money reaches'), 2, 'twice it'));
          return [{ party: partyId(party), side: 'buy', price: 1.2, qty: beyond }];
        }
        // EVERY bank offers what it holds, so the supply is the banking system's whole position in
        // the line and the buyer's over-bid is the short side. One bank's holding is an outcome of
        // the seed's draw and need not be bigger than a firm's money.
        const held = view.quantity(GOV_LINE);
        return held > 0 ? [{ party: partyId(party), side: 'sell', price: 1.2, qty: held }] : [];
      }),
    );
    const had = w.cash(partyId('firm.2'), USD);
    // The test is only a test if the seller has more paper than the buyer's money reaches: with
    // less, the buyer could pay for all of it and nothing would fail.
    const offered = w.register.holdersOf(GOV_LINE).reduce((n, h) => n + w.register.quantity(h, GOV_LINE), 0);
    expect(offered, 'the sellers hold less than the buyer can pay for, so nothing would fail').toBeGreaterThan(had / 1.2);
    const r = w.step();
    const gov = r.markets.find((m) => m.market === GOV_MARKET);
    // HOW MANY fail is an outcome of the draw — how many sellers the buyer's money runs out
    // against — and a test that names it is a test naming a party (CLAUDE.md). What the clause is
    // about is that at least one did, and that each one moved NOTHING.
    expect(gov?.failedTrades).toBeGreaterThan(0);
    // C3.b: THE WHOLE TRADE, NOT HALF OF IT. The buyer is matched against several sellers now —
    // one bank's holding is not necessarily bigger than a firm's money, so the whole banking system
    // offers — and it settles the ones its money covers and fails the one that does not. What the
    // clause is about is that the failing one moved NOTHING: the buyer holds exactly what settled,
    // and its money fell by exactly what that cost, with no leg of the failed instruction anywhere.
    const got = w.register.quantity(partyId('firm.2'), GOV_LINE);
    expect(got).toBe(gov?.settledVolume);
    // It paid for what it got and nothing more — read off its account rather than re-derived from a
    // price (Law 19): what it paid is what the settled legs moved, and the failed one moved none.
    expect(w.cash(partyId('firm.2'), USD)).toBeLessThan(had);
    expect(w.cash(partyId('firm.2'), USD)).toBeGreaterThanOrEqual(0);
    const failed = w.ledger.all().filter((x) => x.outcome === 'failed');
    expect(failed.length).toBe(gov?.failedTrades);
    for (const f of failed) {
      expect(f.reason.kind).toBe('overdraftRefused');
      // C3.b: not a leg of it anywhere. A failed instruction is a recorded state, not a partial
      // one — a failed record carries no deltas at all, which is a different thing from carrying
      // an empty list of them.
      expect('deltas' in f).toBe(false);
    }
    expect(violations(w)).toEqual([]);
  });
});

describe('participant views (Observer A4, Expectations D1)', () => {
  it('show a party its own state and the public state, and nothing of anyone else', () => {
    const w = rigWorld('seed-L');
    w.step();
    const view = w.participantView(partyId('firm.1'));
    expect(view.self.id).toBe('firm.1');
    // A4: WHAT IT SEES IS ITS OWN, and the amount is read from the register rather than written
    // down — how much a firm opens with is an outcome of the seed's arithmetic (11.5 derives this
    // world's scale), and what the view owes this test is that it shows the holder its own number.
    expect(view.cash(USD)).toBe(w.cash(partyId('firm.1'), USD));
    expect(view.cash(USD)).toBeGreaterThan(0);
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
    const w = rigWorld('seed-O');
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
    const w = rigWorld('seed-O2');
    for (let i = 0; i < 6; i += 1) w.step();
    // A period says far more than ten things, so the programme the treasury published this period
    // is nowhere near the back of a ten-deep feed of everything.
    const kind = 'treasury.programme';
    const shallow = snapshot(w, { kind: 'inspector' }, 10, [kind]);
    expect(shallow.journal.some((e) => e.kind === kind)).toBe(false);
    const followed = shallow.followed[kind];
    // B1, D1: the last N OF THAT KIND, which is what a followed feed is for and what it says it is.
    // This asked for all of them, and that held only while a world had fewer than ten to give: one
    // treasury saying one thing a period for six periods is six. There are four treasuries now, so
    // there are twenty-four — and the property is unchanged and still the point, that the feed
    // reaches them at all when a period says thousands of other things.
    const said = w.journal.ofKind(kind).length;
    expect(said).toBeGreaterThan(10);
    expect(followed?.length).toBe(10);
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
    const w = rigWorld('seed-M');
    const cell = w.parties.ofKind(HOUSEHOLD)[0];
    if (cell === undefined) throw new Error('no cell');
    const draft: InstructionDraft = {
      legs: [
        {
          kind: 'money',
          from: { holder: partyId('firm.1'), issuer: BANK_A },
          to: { holder: cell.id, issuer: cell.bank },
          ccy: USD,
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
        // Money C2.b's half asks about two accounts at ONE issuer, so the pair is read out of the
        // world rather than assumed of the seed: WHICH firm banks where is the seed's business (it
        // spreads them across whatever banks the world has) and this is a test of settlement.
        const atA = ctx.parties.ofKind(FIRM).filter((f) => f.bank === BANK_A);
        const payer = atA[0]?.id;
        const payee = atA[1]?.id;
        if (payer === undefined || payee === undefined) throw new Error('no two firms at bank a');
        const side = cellSide(cell, PAYMENT);
        const draft: InstructionDraft = {
          legs: [
            {
              kind: 'money',
              from: { holder: payer, issuer: BANK_A },
              to: { holder: cell.id, issuer: BANK_B },
              ccy: USD,
              amount: totalFor(cell, PAYMENT),
              fromCell: none(),
              toCell: side === undefined ? none() : some(side),
            },
          ],
          cause: 'transfer',
          reason: 'wages',
        };
        const reservesA = ctx.register.quantity(BANK_A, moneyInstrumentId(CB, USD));
        const cellBefore = ctx.register.quantity(cell.id, moneyInstrumentId(BANK_B, USD));
        const rec = ctx.settle(draft);
        expect(rec.outcome).toBe('settled');
        if (rec.outcome !== 'settled') return;
        expect(rec.reserveLegs).toHaveLength(2);
        // Counts of cents, so the two sides are EXACTLY equal and no closeness is asked for.
        expect(ctx.register.quantity(BANK_A, moneyInstrumentId(CB, USD))).toBe(
          reservesA - totalFor(cell, PAYMENT),
        );
        expect(ctx.register.quantity(cell.id, moneyInstrumentId(BANK_B, USD))).toBe(
          cellBefore + PAYMENT,
        );

        const same: InstructionDraft = {
          legs: [
            {
              kind: 'money',
              from: { holder: payer, issuer: BANK_A },
              to: { holder: payee, issuer: BANK_A },
              ccy: USD,
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
    const w = rigWorld('seed-P');
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
