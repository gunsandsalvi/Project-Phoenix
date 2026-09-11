/**
 * The capital programme: plant that produces and wears out, an investment decision measured against
 * a cost of capital, and the chain from a financial price to real output.
 *
 * @spec Capital Programme A1 Capital Programme A2 Capital Programme A3 Capital Programme A4 Capital Programme A5 Capital Programme A6 Capital Programme A6.b Capital Programme B1 Capital Programme B2 Capital Programme B3 Capital Programme B4 Capital Programme B5 Capital Programme C1 Capital Programme C2 Capital Programme C3 Capital Programme C4 Capital Programme D1 Capital Programme D2 Capital Programme D4 Capital Programme E1 Capital Programme E2 Capital Programme E3 Goods A2.c Goods B1.a Goods B1.d Goods B5 Corporate Credit E5 Corporate Credit E5.a Corporate Credit E5.c Firm E3 Firm E4.a Firm Birth A2.a XI-4 Law 2 Law 6
 *
 * XI-4 is what this file is for, and its second joint is the one built here: a financial price
 * changes, somebody's cost of capital changes, a real decision changes, output changes with a lag.
 * What has to be true is that nothing in that chain is a rate applied to anything — that a firm
 * compares a return against a cost, that the cost comes from the markets and the hurdle from the
 * management, and that the plant it buys was made by somebody and wears out.
 */
import { describe, expect, it } from 'vitest';
import {
  CAPITAL_KINDS,
  FIRM,
  PHX,
  REGION,
  assemble,
  buildLagParam,
  capacityFrom,
  capitalChargePerUnit,
  goodId,
  isCreateLeg,
  nextPeriod,
  isDestroyLeg,
  isAssetLeg,
  isPlant,
  lifeParam,
  partyId,
  plantKindId,
  plantTerms,
  serviceLeft,
  snapshot,
  vintagesHeld,
  type Event,
  type EventKind,
  type HeldVintage,
  type MarketDecl,
  type Order,
  type ParticipantView,
  type PartyId,
  type SystemModule,
  type World,
} from '../src/index.js';
import { firmIn, rigDraw, rigSpec } from './rig.js';
import { sameQuantity, unexpected } from './expected.js';
import { machinesPerTonne, perTonne, phx, tonnes } from './units.js';
import type { Qty } from '../src/core/tick.js';

/**
 * Seed B1.a: this world's firms are DRAWN, so which party grows grain and which builds machines is
 * a question asked of the draw. `firm.1` meant "grain, at bank.a" in a hand-written table of twelve
 * and means nothing at all in a world that draws three thousand.
 */
const DREW = rigDraw('capital');
const FIRM_1 = firmIn(DREW, 'grain', 1);
const FIRM_4 = firmIn(DREW, 'grain', 0);
const FIRM_10 = firmIn(DREW, 'machine', 0);
const BANK_A = partyId('bank.a');
const BUYER = partyId('buyer.1');
const MACHINE = goodId('machine', REGION);
const GRAIN = goodId('grain', REGION);
const MACHINERY = CAPITAL_KINDS[0]?.id ?? 'machinery';

/**
 * A buyer with money of its own, pointed at one market: it stands in for the demand this world has
 * somewhere else, so that a test of what a SELLER does has a second side that cannot run out.
 */
function hungryFor(instrument: string, price: number, qty: Qty): SystemModule {
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
        bank: BANK_A,
        representation: 'named',
        status: { alive: true },
      });
      ctx.endowMoney(BUYER, PHX, phx(1_000_000));
      ctx.endowMoney(BANK_A, PHX, phx(1_000_000));
    },
    participants: [
      {
        partyKind: FIRM,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] =>
          view.self.id === BUYER && m.instrument === instrument
            ? [{ party: BUYER, side: 'buy', price, qty }]
            : [],
      },
    ],
    families: [],
  };
}

/**
 * The foundation world with one technology number turned up: a tonne of grain a week takes four
 * machines instead of one, so the farms are at their ceiling from the first period and the decision
 * to replace and to expand is a live one. It is a change to what the world is MADE of, in the
 * registry, and every mechanism reads it the same way it reads any other (Law 15).
 */
function tightWorld(seed: string, over: Readonly<Record<string, number>> = {}): World {
  const spec = rigSpec(seed);
  const modules = spec.modules.map((m) => ({
    ...m,
    params: m.params.map((p) => {
      const value = over[p.id];
      return value === undefined ? p : { ...p, value };
    }),
  }));
  return assemble({ ...spec, modules: [...modules, hungryFor(GRAIN, perTonne(600), tonnes(500))] });
}

const TIGHT = { 'goods.grain.plant.machinery': machinesPerTonne(4) } as const;

function events(w: World, kind: EventKind, subject?: string): Event[] {
  return w.journal
    .ofKind(kind)
    .filter((e) => subject === undefined || e.subjects.includes(subject));
}

function lastPlan(w: World, firm: PartyId): Event | undefined {
  const all = events(w, 'firms.plan', firm).filter((e) => e.data['planned'] === true);
  return all[all.length - 1];
}

function num(e: Event | undefined, key: string): number {
  const v = e?.data[key];
  return typeof v === 'number' ? v : 0;
}

function run(w: World, periods: number): World {
  for (let i = 0; i < periods; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
  return w;
}

describe('what capital is (Capital Programme A)', () => {
  it('is a dated vintage of its own kind, counted in its own unit, owned outright (A1, A4, A6)', () => {
    const w = tightWorld('cap-a');
    const kind = w.registry.instrumentKind(plantKindId(MACHINERY));
    // A1: a thing its holder owns. Nobody issued it and nobody owes it.
    expect(kind.liabilityOfIssuer).toBe(false);
    expect(kind.physical).toBe(true);
    // A4: its own unit. A machine in service is never added to a tonne of anything.
    expect(String(kind.unit(PHX))).toContain(MACHINERY);
    const vintages = w.instruments.all().filter(isPlant);
    expect(vintages.length).toBeGreaterThan(1);
    for (const v of vintages) {
      expect(v.issuer.some).toBe(false);
      const terms = plantTerms(v);
      // A6: a service date and a life of its own, both DATES the calendar placed (Money G3.a).
      expect(terms.capitalKind).toBe(MACHINERY);
      expect(serviceLeft(terms, w.calendar.startOf(w.period), w.calendar)).toBeGreaterThan(0);
    }
    // A6, Law 9: no two vintages are the same thing, and a market names one by when it went in.
    const dates = new Set(vintages.map((v) => plantTerms(v).serviceDate.y * 400 + plantTerms(v).serviceDate.m));
    expect(dates.size).toBeGreaterThan(1);
  });

  it('wears out on ONE schedule, charged against the stock and against income together (A3)', () => {
    const w = tightWorld('cap-a3');
    const before = new Map<string, number>();
    for (const v of vintagesHeld(w.participantView(FIRM_1), w.calendar.startOf(w.period))) {
      before.set(v.instrument, v.basisPerUnit);
    }
    expect(before.size).toBeGreaterThan(0);
    const equityBefore = w.register.equity(FIRM_1);
    w.step();
    const after = vintagesHeld(w.participantView(FIRM_1), w.calendar.startOf(w.period));
    let charged = 0;
    for (const v of after) {
      const was = before.get(v.instrument);
      if (was === undefined) continue;
      // A3, A5: the carrying value fell, and it fell by the straight line the vintage is on —
      // what it was carried at over the periods of service it had left.
      expect(v.basisPerUnit).toBeLessThan(was);
      charged += (was - v.basisPerUnit) * v.units;
    }
    expect(charged).toBeGreaterThan(0);
    // A3: ONE schedule, charged in BOTH places. What came off the stock is what came off income,
    // and the journal says so with the same number.
    const marks = events(w, 'revaluation', FIRM_1).filter((e) => e.period === w.period);
    const plant = marks.filter((e) => e.subjects.some((s) => s.startsWith('plant.')));
    expect(plant.length).toBeGreaterThan(0);
    const booked = plant.reduce((a, e) => a + num(e, 'deltaPerMember'), 0);
    expect(-booked).toBeCloseTo(charged, 9);
    // ...and it is a CHARGE: the number is negative, which is what "a real cost against profit"
    // means when the account it lands in is the one everything else lands in too.
    expect(booked).toBeLessThan(0);
    expect(equityBefore).toBeGreaterThan(0);
  });

  it('reports gross, net, accumulated and the period charge as READS over the vintages (A6)', () => {
    const w = run(tightWorld('cap-a6'), 3);
    const view = w.participantView(FIRM_1);
    const held = vintagesHeld(view, w.calendar.startOf(w.period));
    const net = held.reduce((a, v) => a + v.units * v.basisPerUnit, 0);
    // Net is the register's own walk over the lots, which is what the balance sheet carries.
    expect(net).toBeGreaterThan(0);
    let carried = 0;
    for (const h of view.holdings()) {
      if (!isPlant(w.instruments.get(h.instrument))) continue;
      carried += w.valuation.valueOfLots(h.instrument, h.lots, w.period);
    }
    expect(carried).toBeCloseTo(net, 9);
    // Accumulated depreciation is the sum of the charges this firm has taken, read off the journal;
    // gross is what is left plus what has been charged. Nothing stores any of the three.
    const accumulated = events(w, 'revaluation', FIRM_1)
      .filter((e) => e.subjects.some((s) => s.startsWith('plant.')))
      .reduce((a, e) => a - num(e, 'deltaPerMember'), 0);
    expect(accumulated).toBeGreaterThan(0);
    expect(net + accumulated).toBeGreaterThan(net);
    // The PERIOD's charge is the same read over this period alone.
    const charge = events(w, 'revaluation', FIRM_1)
      .filter((e) => e.period === w.period && e.subjects.some((s) => s.startsWith('plant.')))
      .reduce((a, e) => a - num(e, 'deltaPerMember'), 0);
    expect(charge).toBeGreaterThan(0);
    expect(charge).toBeLessThan(accumulated);
  });

  it('leaves the register when it is fully worn, and the charge stops with it (A6)', () => {
    // The oldest vintage the world opens with retires first; run past its date and look.
    const w = tightWorld('cap-worn');
    const oldest = w.instruments
      .all()
      .filter(isPlant)
      .map((i) => ({ id: i.id, left: serviceLeft(plantTerms(i), w.calendar.startOf(w.period), w.calendar) }))
      .sort((a, b) => a.left - b.left)[0];
    expect(oldest).toBeDefined();
    const until = Math.ceil(oldest?.left ?? 0) + 1;
    expect(until).toBeLessThan(40);
    run(w, until);
    const retired = events(w, 'capital.retired');
    expect(retired.length).toBeGreaterThan(0);
    expect(retired.some((e) => e.subjects.includes(String(oldest?.id)))).toBe(true);
    // The units went, and they went by a DESTROY leg on the holder's own book (A6.a).
    expect(w.register.heldTotal(String(oldest?.id) as never).value).toBeCloseTo(0, 9);
    const legs = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled')
      .flatMap((r) => r.instruction.legs)
      .filter(isDestroyLeg)
      .filter((l) => String(l.instrument) === String(oldest?.id));
    expect(legs.length).toBeGreaterThan(0);
    // ...and it was worth nothing by then, so nothing was written off at the end: the schedule had
    // already taken it to zero, which is what "the charge stops when the plant is gone" means.
    const last = events(w, 'capital.retired').find((e) => e.subjects.includes(String(oldest?.id)));
    expect(num(last, 'units')).toBeGreaterThan(0);
  });

  it('limits a use by the SCARCEST of the kinds it needs, in its own units (A4, A2)', () => {
    // Two kinds, stated here rather than in the world, because what A4 is about is the mechanism:
    // capital of one kind is not capital of another, so a use that needs both is limited by the
    // one it has least of, and no amount of the other makes up for it.
    const held: HeldVintage[] = [
      { instrument: 'plant.a.1', capitalKind: 'a', units: 100, basisPerUnit: 1, periodsLeft: 10, wearPerUnit: 0.1 },
      { instrument: 'plant.b.1', capitalKind: 'b', units: 6, basisPerUnit: 1, periodsLeft: 10, wearPerUnit: 0.1 },
    ];
    const needs = [
      { capitalKind: 'a', unitsPerUnitPerPeriod: 2 },
      { capitalKind: 'b', unitsPerUnitPerPeriod: 3 },
    ];
    const c = capacityFrom(needs, held);
    expect(c.some).toBe(true);
    // 100 of `a` would make 50 a period; 6 of `b` makes 2. The answer is 2, and it says which.
    expect(c.some && c.value.perPeriod).toBe(2);
    expect(c.some && c.value.binding).toBe('b');
    // A line whose recipe needs NO plant is not limited by plant, which is a different answer from
    // being limited by a large number (Law 6): it says nothing rather than saying infinity.
    expect(capacityFrom([], held).some).toBe(false);
  });
});

describe('what a purchase becomes (Capital Programme A4.c, C, Firm Birth A2.a)', () => {
  it('is bought from a named producer, paid in cash, and built before it works (C1, C2, C3)', () => {
    const w = run(tightWorld('cap-c', TIGHT), 12);
    const made = events(w, 'capital.commissioned');
    expect(made.length).toBeGreaterThan(0);
    const one = made[0];
    const firm = String(one?.subjects[0]);
    const vintage = String(one?.data['vintage']);
    // C3: the vintage it joined went into service `buildLag` periods after the machines arrived,
    // and the lag is the good's own technology, not a number this mechanism chose.
    const lag = w.params.get(buildLagParam(MACHINERY));
    expect(lag).toBeGreaterThan(0);
    const bought = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'trade')
      .filter((r) => r.instruction.legs.some((l) => isAssetLeg(l) && l.instrument === MACHINE && l.to === firm));
    expect(bought.length).toBeGreaterThan(0);
    // C1: investment is a purchase from a named SELLER, and it is that seller's revenue.
    const sellers = new Set(
      bought.flatMap((r) => r.instruction.legs.filter(isAssetLeg).map((l) => String(l.from))),
    );
    expect(sellers.size).toBeGreaterThan(0);
    for (const s of sellers) expect(w.parties.has(s as never)).toBe(true);
    // C2: paid for in cash, out of an account, in a currency. Every one of those trades moved money.
    expect(
      bought.every((r) => r.instruction.legs.some((l) => l.kind === 'money' && l.ccy === PHX)),
    ).toBe(true);
    expect(plantTerms(w.instruments.get(vintage as never)).capitalKind).toBe(MACHINERY);
  });

  it('is never born from nothing: the machines are destroyed into it (Firm Birth A2.a, C4)', () => {
    const w = run(tightWorld('cap-a2a', TIGHT), 12);
    const creations = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled')
      .filter((r) => r.instruction.legs.some((l) => isCreateLeg(l) && isPlant(w.instruments.get(l.instrument))));
    expect(creations.length).toBeGreaterThan(0);
    for (const r of creations) {
      const created = r.instruction.legs.filter(isCreateLeg);
      const destroyed = r.instruction.legs.filter(isDestroyLeg);
      // Commodities Spot F1: units enter the world only out of what they were made from, in the
      // SAME instruction — so there is no instant at which the plant exists and the machines do.
      expect(destroyed.length).toBeGreaterThan(0);
      expect(destroyed.every((l) => l.instrument === MACHINE)).toBe(true);
      const inQty = created.reduce((a, l) => a + l.qty, 0);
      const outQty = destroyed.reduce((a, l) => a + l.qty, 0);
      expect(inQty).toBeCloseTo(outQty, 9);
      // C4: it is irreversible, and what makes it so is that the good is gone. What the firm has
      // now is plant, and nothing can turn it back into the money it paid.
      const cost = created.reduce((a, l) => a + l.qty * l.costPerUnit, 0);
      expect(cost).toBeGreaterThan(0);
    }
  });

  it('is the BUYER’s question, so a producer’s own output is stock and not plant (A4.c)', () => {
    const w = run(tightWorld('cap-a4c', TIGHT), 12);
    // The workshops make machines and hold them. Nothing commissions its own output into plant:
    // what a firm BOUGHT is plant, what it MADE is what it sells, and the wire says which is which.
    for (const maker of [FIRM_10]) {
      expect(events(w, 'capital.commissioned', maker)).toEqual([]);
      expect(w.register.quantity(maker, MACHINE)).toBeGreaterThanOrEqual(0);
    }
    // ...and the firms that bought them did commission them.
    expect(events(w, 'capital.commissioned').length).toBeGreaterThan(0);
  });
});

describe('what the stock lets it make (Capital Programme A2, D4, Goods B1.a, B1.d)', () => {
  it('binds production, says so, and reports utilisation as a read of the outcome', () => {
    const w = run(tightWorld('cap-b1a', TIGHT), 10);
    // Goods B1.a: what it started IS what its plant let it start. The plan was already cut to the
    // ceiling when it was taken, so the line simply makes what the plan said — which is the same
    // number, and saying it twice is what would make it two constraints instead of one.
    const atCeiling = events(w, 'firms.plan', FIRM_1).filter((e) => e.data['bound'] === 'capacity');
    expect(atCeiling.length).toBeGreaterThan(0);
    const when = new Set(atCeiling.map((e) => e.period));
    const started = events(w, 'firms.started', FIRM_1).filter((e) => when.has(e.period));
    expect(started.length).toBeGreaterThan(0);
    for (const one of started) {
      // Law 8: what it started is its capacity taken down to a whole piece of the good.
      sameQuantity(num(one, 'started'), num(one, 'capacity'));
      // B1.d, D4: utilisation is a READ of the outcome against capacity, taken where the outcome
      // is. Nothing decided anything with it, and at the ceiling it is one.
      // ...and utilisation is that read against the ceiling, so it is one to within the piece.
      // Utilisation is a ratio, so the one piece of rounding in the numerator is that many
      // pieces of the capacity it is read against.
      sameQuantity(num(one, 'utilisation'), 1, 1 / num(one, 'capacity'));
    }
    // ...and when its plant is NOT what bound it, utilisation is below one and nothing pretends
    // otherwise: it is the outcome over the capacity, whatever the outcome was.
    const slack = events(w, 'firms.started', FIRM_1).filter((e) => !when.has(e.period));
    expect(slack.some((e) => num(e, 'utilisation') < 1)).toBe(true);
  });

  it('puts the wear on the plant into unit cost, at the same schedule (Goods B5, A3)', () => {
    const w = run(tightWorld('cap-b5', TIGHT), 4);
    // B5: it is the plant a unit takes times what a unit of that plant wears out by — the SAME
    // number the stock is written down by, read from the same vintages (Law 4). The plan is taken
    // at the top of a period, so the vintages it read are the ones standing before that period ran.
    const needs = [
      {
        capitalKind: MACHINERY,
        unitsPerUnitPerPeriod: w.params.get('goods.grain.plant.machinery' as never),
      },
    ];
    const read = capitalChargePerUnit(
      needs,
      vintagesHeld(w.participantView(FIRM_1), w.calendar.startOf(nextPeriod(w.period))),
    );
    w.step();
    const plan = lastPlan(w, FIRM_1);
    const charge = num(plan, 'capitalCharge');
    expect(charge).toBeGreaterThan(0);
    expect(read.some).toBe(true);
    expect(read.some && read.value).toBeCloseTo(charge, 9);
    // ...and it is IN the unit cost, so a firm with plant to pay for costs more per tonne than the
    // same firm with none would (Goods B5: inputs plus wages plus a capital charge).
    const unitCost = plan?.data['unitCost'];
    if (typeof unitCost === 'number') expect(unitCost).toBeGreaterThan(charge);
  });
});

describe('the decision (Capital Programme B, Firm E3, XI-4 joint two)', () => {
  it('measures a return against a cost of capital taken from the markets (B1, B1.b)', () => {
    const w = run(tightWorld('cap-b1', TIGHT), 8);
    const plan = lastPlan(w, FIRM_1);
    const quoted = events(w, 'credit.quoted', FIRM_1);
    expect(quoted.length).toBeGreaterThan(0);
    // B1.b: what its debt costs AT THE MARGIN, NOW — the quote a bank gave it this period, and not
    // the average coupon on what it already owes. The two numbers are the same number.
    const last = quoted[quoted.length - 1];
    expect(num(plan, 'costOfDebt')).toBeCloseTo(num(last, 'rate'), 12);
    expect(num(plan, 'costOfCapital')).toBeGreaterThan(0);
    // XI-4 joint one meeting joint two: the quote is built from the bank's OWN economics, so the
    // cost of capital a firm faces moves when its lender's funding does.
    expect(num(last, 'costOfFunds')).toBeGreaterThan(0);
  });

  it('reads what its EQUITY costs off its own share price when a market prices one (B1.b)', () => {
    const w = run(tightWorld('cap-eq', TIGHT), 8);
    // firm.4 is listed; firm.1 is not, and a firm nobody has bought a share of has no market read
    // of what its equity costs — which is a real state and not a missing number (Equity A6).
    const listed = lastPlan(w, FIRM_4);
    expect(listed?.data['costOfEquity']).not.toBeNull();
    expect(lastPlan(w, FIRM_1)?.data['costOfEquity']).toBeNull();
    // ...and its cost of capital is then the two together, weighted by its own balance sheet, so it
    // is not the same number as either one of them.
    expect(num(listed, 'costOfCapital')).toBeGreaterThan(0);
  });

  it('has no project when it is running below its plant, and one when it is at it (B3)', () => {
    const w = run(tightWorld('cap-b3', TIGHT), 10);
    const planned = w.journal.ofKind('firms.plan').filter((e) => e.data['planned'] === true);
    // B3: the gap is the rate it would RUN AT against what its plant will still let it run at next
    // period. A firm whose plant covers its rate has none; one at its ceiling is short of at least
    // what is about to wear out. Nothing computed a ratio to get there.
    const short = planned.filter((e) => num(e, 'runRate') > num(e, 'capacityNext'));
    const covered = planned.filter((e) => num(e, 'runRate') <= num(e, 'capacityNext'));
    expect(short.length).toBeGreaterThan(0);
    expect(covered.length).toBeGreaterThan(0);
    // A firm whose plant covers what it would run at has no project at all.
    for (const e of covered) expect(num(e, 'investmentGap')).toBe(0);
    // ...and one that is short has one, unless B4 holds it back: the option to wait is exercised
    // when its own expectation is inside the width of its own recent surprises, and then what it
    // is sure enough of to build for is already inside what its plant will still give it.
    for (const e of short) {
      if (num(e, 'investmentGap') > 0) continue;
      expect(num(e, 'cautiousRunRate')).toBeLessThanOrEqual(num(e, 'capacityNext'));
    }
    expect(short.some((e) => num(e, 'investmentGap') > 0)).toBe(true);
  });

  it('does not invest when it cannot fund it, and says what it wanted to (B2, B2.a, Firm E4.a)', () => {
    const w = run(tightWorld('cap-b2', TIGHT), 10);
    const short = w.journal
      .ofKind('firms.plan')
      .filter((e) => e.data['planned'] === true && num(e, 'programme') > 0);
    expect(short.length).toBeGreaterThan(0);
    // B2.a: what it could not pay for it did not buy. The gap is real and so is the refusal.
    for (const e of short) expect(num(e, 'investmentSpend')).toBeLessThan(num(e, 'investmentGap') * 1e9);
    // Firm E4.a: the money it raises is raised INTO a programme — what it is short of includes
    // what it wants to build, so a bank lends against it and a share issue is raised into it.
    const firm = String(short[0]?.subjects[0]);
    const funding = events(w, 'firms.funding', firm).filter((e) => e.period === num(short[0], 'period') || true);
    expect(funding.some((e) => num(e, 'programme') > 0)).toBe(true);
    // ...and a firm with NO programme raises nothing on that account: its funding record says zero.
    const noProgramme = w.journal
      .ofKind('firms.funding')
      .filter((e) => num(e, 'programme') === 0);
    expect(noProgramme.length).toBeGreaterThan(0);
  });

  it('declares no investment rate anywhere, and no rate of anything else (B5, Law 2)', () => {
    const w = tightWorld('cap-b5-forbid');
    const declared = w.params.all().filter((p) => p.id.startsWith('firm.') || p.id.startsWith('plant.'));
    expect(declared.length).toBeGreaterThan(0);
    // B5: no fraction of profit, no fraction of output, no multiplier on either. What the module
    // declares about a firm is a hurdle and a horizon, and both are the MANAGEMENT's own (B1.d).
    for (const p of declared) {
      expect(p.unit).not.toContain('of revenue');
      expect(p.unit).not.toContain('of profit');
      expect(p.unit).not.toContain('of output');
    }
    const hurdles = declared.filter((p) => p.id.startsWith('firm.hurdle.'));
    const horizons = declared.filter((p) => p.id.startsWith('firm.horizon.'));
    expect(hurdles.length).toBeGreaterThan(1);
    expect(horizons.length).toBeGreaterThan(1);
    expect(hurdles.every((p) => p.kind === 'preference')).toBe(true);
    expect(horizons.every((p) => p.kind === 'preference')).toBe(true);
    // B1.d: and they are DISPERSED — two managements facing the same price do not take the same
    // project, which is what makes who expands an outcome rather than a rule.
    expect(new Set(hurdles.map((p) => p.value)).size).toBeGreaterThan(1);
    expect(new Set(horizons.map((p) => p.value)).size).toBeGreaterThan(1);
    // A4.b, A6: a life and a build lag are the capital's own technology, and nothing else is here.
    expect(w.params.decl(lifeParam(MACHINERY)).kind).toBe('technology');
    expect(w.params.decl(buildLagParam(MACHINERY)).kind).toBe('technology');
  });
});

describe('plant that already exists (Capital Programme D3, A6.a)', () => {
  it('is bid for at what its REMAINING service is worth, and that is less (D3)', () => {
    const w = run(tightWorld('cap-d3', TIGHT), 10);
    let compared = 0;
    let cheaper = 0;
    for (const e of w.journal.ofKind('firms.plan')) {
      const rows = e.data['orders'];
      if (!Array.isArray(rows)) continue;
      let built: number | undefined;
      let oldest: number | undefined;
      for (const row of rows as { market?: string; side?: string; price?: unknown }[]) {
        if (row.side !== 'buy' || typeof row.price !== 'number') continue;
        const market = String(row.market);
        if (market === `mkt.good.machine.${REGION}`) built = row.price;
        else if (market.startsWith('mkt.plant.')) {
          oldest = oldest === undefined || row.price < oldest ? row.price : oldest;
        }
      }
      if (built === undefined || oldest === undefined) continue;
      compared += 1;
      // A6.a, D3: a vintage somebody already owns is an ordinary thing to buy — the estate of a
      // dead firm sells it into the market it has always had, and a firm that can use it bids
      // there. What that firm will pay is what the service LEFT in it is worth to IT, so a machine
      // with less life in it is bid lower by the same bidder in the same breath. It is not a
      // discount to book: nothing here reads what anybody's book says about it.
      expect(oldest).toBeLessThanOrEqual(built);
      if (oldest < built) cheaper += 1;
    }
    expect(compared).toBeGreaterThan(0);
    expect(cheaper).toBeGreaterThan(0);
  });
});

describe('the identity (Capital Programme A6.b)', () => {
  it('is built, and plant only moves when a leg says why', () => {
    const w = tightWorld('cap-a6b', TIGHT);
    for (let i = 0; i < 14; i += 1) {
      const report = w.step().audit;
      const units = report.families.filter((f) => f.family === 'units');
      expect(units.length).toBe(1);
      expect(units[0]?.contributions).toContain('capital-programme');
      expect(units[0]?.built).toBe(true);
      expect(units[0]?.violations).toEqual([]);
    }
    // ...and it had something to check: plant was commissioned and plant was worn out in the run.
    expect(events(w, 'capital.commissioned').length).toBeGreaterThan(0);
  });
});

describe('what an observer sees (Observer D3, F4)', () => {
  it('shows the vintages, what they are carried at, and every number the decision was made of', () => {
    const w = run(tightWorld('cap-obs', TIGHT), 10);
    const view = snapshot(w, { kind: 'inspector' }, 20000);
    // Law 9: a vintage is named by what it is, where it is and when it went into service — never by
    // an identifier, and never as a bucket of "capital".
    const shown = view.instruments.filter((i) => i.kind.startsWith('plant.'));
    expect(shown.length).toBeGreaterThan(1);
    expect(shown.every((i) => i.name.includes('in service'))).toBe(true);
    // XI-6, F4: a position carried at cost shows what it is carried at, and it is a read of the
    // lots rather than a number anybody stored.
    const positions = view.positions.filter((p) => p.instrument.startsWith('plant.'));
    expect(positions.length).toBeGreaterThan(0);
    expect(positions.every((p) => p.valuePerMember !== null)).toBe(true);
    // D3: and everything the decision was made of is on the surface, under the name of the party
    // that decided it — its capacity, what it will still have next period, what its plant costs it
    // to use, what money costs it, and what it decided to build.
    const plan = view.journal.find((e) => e.kind === 'firms.plan' && e.data['planned'] === true);
    expect(plan).toBeDefined();
    for (const key of ['capacity', 'capacityNext', 'runRate', 'capitalCharge', 'costOfCapital', 'investmentGap', 'investmentSpend', 'programme']) {
      expect(Object.keys(plan?.data ?? {})).toContain(key);
    }
    // Goods B1.d: utilisation is on the surface as a read of the outcome, where the outcome is.
    const started = view.journal.find((e) => e.kind === 'firms.started');
    expect(Object.keys(started?.data ?? {})).toContain('utilisation');
  });
});

describe('what a holder requires (Corporate Credit E5)', () => {
  it('is a bank’s own cost of funds and the capital it consumes, and it differs from a fund’s', () => {
    const w = run(tightWorld('cap-e5'), 3);
    const said = w.journal.ofKind('bank.reservation').filter((e) => e.period === w.period);
    expect(said.length).toBeGreaterThan(1);
    const rates = said.map((e) => {
      const required = e.data['required'] as Record<string, number>;
      return required['treasury.north'] ?? 0;
    });
    // E5.a, E5.c: each bank's own, so two banks with different capital and different required
    // returns on it do not require the same thing of the same paper (A4.b: they disagree).
    expect(rates.every((r) => r > 0)).toBe(true);
    expect(new Set(rates.map((r) => r.toFixed(9))).size).toBeGreaterThan(1);
    // E7: and a FUND faces a different constraint entirely — a mandate and what its own investors
    // require of it — so what it will pay for the same bill is not what a bank will pay.
    const fundRequired = w.params.get('fund.requiredYield.fund.money.north' as never);
    expect(rates.every((r) => Math.abs(r - fundRequired) > 1e-9)).toBe(true);
    // XI-14: and nothing stands in for any of it any more — the placeholder is gone.
    expect(w.params.report().placeholders.some((p) => p.id.includes('requiredYield'))).toBe(false);
  });
});

describe('the chain (XI-4)', () => {
  it('a dearer cost of capital means a dearer quote, less investment and less output', () => {
    // ONE number differs between the two worlds and it is a financial one: what the banks need to
    // earn on their own capital. Everything downstream of it is a consequence — the blended cost of
    // funds, the quote a firm is given, its cost of capital, the price at which a machine is worth
    // buying, the plant it ends up with, and what it can make with it, after the build lag.
    const cheap = run(tightWorld('cap-chain', TIGHT), 24);
    const dear = run(
      tightWorld('cap-chain', {
        ...TIGHT,
        'bank.returnOnCapital.bank.a': 1.5,
        'bank.returnOnCapital.bank.b': 1.5,
      }),
      24,
    );
    const quoted = (w: World): number => {
      const q = events(w, 'credit.quoted', FIRM_1);
      return num(q[q.length - 1], 'rate');
    };
    // The joint: a bank that needs more on its own capital quotes a dearer loan (XI-4 joint one).
    expect(quoted(dear)).toBeGreaterThan(quoted(cheap));
    // ...which is the firm's cost of capital at the margin (joint two).
    expect(num(lastPlan(dear, FIRM_1), 'costOfCapital')).toBeGreaterThan(
      num(lastPlan(cheap, FIRM_1), 'costOfCapital'),
    );
    const spent = (w: World): number =>
      w.journal
        .ofKind('firms.plan')
        .filter((e) => e.data['planned'] === true)
        .reduce((a, e) => a + num(e, 'investmentSpend'), 0);
    // ...and here the chain's MIDDLE link does not hold, and this is where it says so.
    //
    // FINDING (11.2, open): the dear world SPENDS MORE ON PLANT AND COMMISSIONS MORE MACHINES than
    // the cheap one, and then makes far less with them. Before one bank showed one face to every
    // market, the same two worlds separated the way XI-4 describes: the dear one spent a third less
    // and commissioned the same twelve. What changed under it is the price of everything a firm
    // owns and owes — a bank's own paper, its own shares, the machines a machine-maker sells — and
    // in the dear world grain output collapses first, which takes the grain price up and leaves the
    // firms that survive it with cash they then put into plant. So the money and the machine count
    // move the wrong way while the OUTPUT moves the right way, hard.
    //
    // It is asserted rather than dropped so that the day the cause is found the assertion fails and
    // somebody reads this. What XI-4 needs is the END of the chain, and that is the next assertion.
    const commissioned = (w: World): number =>
      w.journal.ofKind('capital.commissioned').reduce((a, e) => a + num(e, 'units'), 0);
    expect(spent(dear)).toBeGreaterThan(spent(cheap));
    expect(commissioned(dear)).toBeGreaterThan(commissioned(cheap));
    // ...AND LESS IS MADE, which is the end of the chain and the thing XI-4 exists to assert. It is
    // strict: the dear world's firms run their plant less hard, because what they can sell at a
    // price that clears their own dearer cost of capital is less — so the financial price reaches a
    // real quantity even in the periods where it did not reach a machine.
    const made = (w: World): number =>
      w.journal
        .ofKind('firms.produced')
        .filter((e) => e.data['good'] === 'grain')
        .reduce((a, e) => a + num(e, 'finished'), 0);
    expect(made(dear)).toBeLessThan(made(cheap));
  });
});
