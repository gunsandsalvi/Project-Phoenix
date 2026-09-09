/**
 * Firms: a decision taken from the firm's own view, a line that consumes what its recipe names and
 * carries what it cost, and a supply schedule with a reason behind every step.
 *
 * @spec Firm B1 Firm B2 Firm B3 Firm B4 Firm B5 Firm C1 Firm D1 Firm E1 Firm E2 Firm E6 Firm E7 Goods B1 Goods B1.b Goods B2 Goods B3 Goods B4 Goods B5 Goods B5.a Goods B5.b Goods C1 Goods C5 Goods E1 Goods F5.a Labour C1 Labour C5 Expectations C2 XI-16 Law 2
 */
import { describe, expect, it } from 'vitest';
import {
  FIRMS,
  FIRM,
  PHX,
  REGION,
  assemble,
  firms,
  foundationSpec,
  goodId,
  none,
  partyId,
  wipId,
  type Event,
  type EventKind,
  type MarketDecl,
  type Order,
  type ParticipantView,
  type SystemModule,
  type World,
} from '../src/index.js';
import { unexpected } from './expected.js';

const BANK_A = partyId('bank.a');
const BUYER = partyId('buyer.1');
const FIRM_1 = partyId('firm.1'); // grain, from field labour alone
const FIRM_2 = partyId('firm.2'); // flour, from grain
const FIRM_3 = partyId('firm.3'); // bread, from flour

const GRAIN = goodId('grain', REGION);
const FLOUR = goodId('flour', REGION);
const BREAD = goodId('bread', REGION);
const WIP_GRAIN = wipId('grain', REGION);

/**
 * A buyer with money of its own, pointed at one market: it stands in for demand this world has
 * somewhere else — a household that eats bread, a firm that mills grain — so that a test of what a
 * SELLER does has a second side that cannot run out of cash halfway through and change the subject.
 */
function buyer(subUnit: string, price: number, qty: number): SystemModule {
  const instrument = goodId(subUnit, REGION);
  return {
    id: 'test.buyer',
    spec: 'Goods C3',
    requires: ['goods', 'seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
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
      ctx.endowMoney(BUYER, PHX, 100000);
      ctx.endowMoney(BANK_A, PHX, 100000);
    },
    phases: [],
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

function world(...extra: readonly SystemModule[]): World {
  const spec = foundationSpec('firms');
  return assemble({ ...spec, modules: [...spec.modules, ...extra] });
}

/** The world the seed opens, with somebody buying grain: the farm's own market has two sides. */
function grainWorld(qty = 40): World {
  return world(buyer('grain', GRAIN_OPENS_AT, qty));
}

/** What the seed states the grain market opens at, which is what a buyer of it bids around. */
const GRAIN_OPENS_AT = 0.4;

function events(w: World, kind: EventKind, subject: string): Event[] {
  return w.journal.ofKind(kind).filter((e) => e.subjects.includes(subject));
}

function last(w: World, kind: EventKind, subject: string): Event | undefined {
  const list = events(w, kind, subject);
  return list[list.length - 1];
}

describe('what a firm decides (Firm E1, E2, E6)', () => {
  it('decides nothing about making or employing until it has sold something (Goods B1)', () => {
    const w = grainWorld();
    const r = w.step();
    expect(r.audit.total).toBe(0);
    const plan = last(w, 'firms.plan', FIRM_1);
    // It has a price — the market printed one — but no expectation of demand, because a seller
    // knows demand only through its own fills (§46 C2). So it plans nothing and posts no opening.
    expect(plan?.data['planned']).toBe(false);
    expect(events(w, 'labour.hire', FIRM_1)).toHaveLength(0);
    // It still offers what it holds: a thing made to be sold and not sold perishes (Goods E4).
    expect(w.journal.ofKind('print').some((e) => e.subjects.includes(GRAIN))).toBe(true);
  });

  it('plans and posts an opening once it knows what it sells (Firm E2, Labour C1, C5)', () => {
    const w = grainWorld(90);
    w.step();
    const held = w.register.quantity(FIRM_1, GRAIN);
    expect(held).toBeGreaterThanOrEqual(0);
    let r = w.step();
    for (let i = 0; i < 2 && events(w, 'labour.hire', FIRM_1).length === 0; i += 1) r = w.step();
    expect(r.audit.total).toBe(0);
    const plan = last(w, 'firms.plan', FIRM_1);
    expect(plan?.data['planned']).toBe(true);
    expect(plan?.data['hours']).toBeGreaterThan(0);
    // Labour C1, C1.a: the most it will pay for an hour is what an hour is worth to it — the
    // output an hour makes possible, at the price it expects, less what the recipe else takes:
    // the inputs (a farm draws none) and what the plant that hour runs on wears out by (Goods B5,
    // Capital Programme A3). The price it expects is the one it sold at, not the opening print.
    const expected = plan?.data['expectedPrice'];
    const charge = plan?.data['capitalCharge'];
    const worth =
      ((typeof expected === 'number' ? expected : 0) * 0.92 -
        (typeof charge === 'number' ? charge : 0)) /
      9;
    const bid = plan?.data['wageBid'];
    expect(typeof bid === 'number' ? bid : 0).toBeCloseTo(worth, 12);
    const hired = last(w, 'labour.hire', FIRM_1);
    expect(hired).toBeDefined();
    // B1.a: in a slack market the print falls to what the seekers will work for, and no further.
    const paid = hired?.data['wagePerHour'];
    expect(typeof paid === 'number' ? paid : 1).toBeLessThan(worth);
  });

  it('publishes what it expects to deliver, and is then surprised by what it did (Firm E7)', () => {
    const w = grainWorld();
    w.step();
    w.step();
    const published = last(w, 'firms.expectation', FIRM_1);
    expect(published?.public).toBe(true);
    expect(typeof published?.data['earnings']).toBe('number');
    // §46 C2, D3: the expectation is its own adaptive read of its own earnings, and it is scored.
    const surprises = w.journal
      .ofKind('expectations.surprise')
      .filter((e) => e.subjects.includes(FIRM_1) && e.data['variable'] === 'earnings');
    expect(surprises.length).toBeGreaterThan(0);
  });
});

describe('the line (Goods B2, B3, B4, B5)', () => {
  it('consumes what the recipe says, carries the batch at what it cost, and yields late', () => {
    const w = grainWorld(90);
    // B3: grain takes two periods. What the seed put on the line comes off in the period the lead
    // time says and not before, and nothing has come off it until then.
    const first = w.step();
    expect(first.audit.total).toBe(0);
    expect(w.register.quantity(FIRM_1, WIP_GRAIN)).toBeGreaterThan(0);
    expect(events(w, 'firms.produced', FIRM_1)).toHaveLength(0);
    const second = w.step();
    expect(second.audit.total).toBe(0);
    const made = last(w, 'firms.produced', FIRM_1);
    expect(made).toBeDefined();
    const startedUnits = Number(made?.data['started']);
    const finished = Number(made?.data['finished']);
    // B4: not everything started is finished, and the scrap is units, at the point they would
    // have been made — never a rate applied to a value.
    expect(finished).toBeCloseTo(startedUnits * 0.92, 9);
    expect(Number(made?.data['scrapped'])).toBeGreaterThan(0);
    // B4: what survives carries the whole batch, so a survivor is dearer than a unit started.
    expect(Number(made?.data['costPerUnit'])).toBeGreaterThan(0);
    // And a batch it starts itself is the same: consumed, carried, and off the line two periods on.
    let started = last(w, 'firms.started', FIRM_1);
    for (let i = 0; i < 6 && started === undefined; i += 1) {
      const r = w.step();
      expect(r.audit.total).toBe(0);
      started = last(w, 'firms.started', FIRM_1);
    }
    expect(started).toBeDefined();
    expect(Number(started?.data['started'])).toBeGreaterThan(0);
    // B5: what it cost is the inputs it drew — none, for a thing grown from labour and land —
    // plus the wage bill the period paid, and that is the whole of it.
    expect(started?.data['cost']).toBe(started?.data['wages']);
    expect(w.register.quantity(FIRM_1, WIP_GRAIN)).toBeGreaterThan(0);
  });

  it('capitalises nothing in a period that starts nothing (Goods B5.a, F5.a)', () => {
    const w = grainWorld(90);
    for (let i = 0; i < 8; i += 1) {
      const r = w.step();
      expect(r.audit.total).toBe(0);
    }
    // A period in which it paid a wage bill and started no batch: the cost stands where it fell,
    // in the period it was incurred, and nothing was capitalised into anything.
    const idlePeriods = w.journal
      .ofKind('labour.wages')
      .filter((e) => e.subjects.includes(FIRM_1) && Number(e.data['paid']) > 0)
      .map((e) => e.period)
      .filter((at) => !events(w, 'firms.started', FIRM_1).some((e) => e.period === at));
    expect(idlePeriods.length).toBeGreaterThan(0);
    for (const at of idlePeriods) {
      const created = w.ledger
        .inPeriod(at)
        .filter((r) => r.outcome === 'settled' && r.instruction.cause === 'production')
        .flatMap((r) => r.instruction.legs)
        .filter((leg) => leg.kind === 'create' && leg.party === FIRM_1 && leg.instrument === WIP_GRAIN);
      expect(created).toHaveLength(0);
    }
  });

  it('is bound by the inputs on hand, and says which one bound it (Goods B1.b, B5.b)', () => {
    // A buyer of flour far bigger than the grain the mill can find: the farm has one crop and the
    // mill wants more flour than that crop makes, so its line is throttled by what it could buy.
    const w = world(buyer('flour', 1.2, 400));
    for (let i = 0; i < 14; i += 1) {
      const r = w.step();
      expect(r.audit.total).toBe(0);
    }
    // WHICH mill the grain runs out under is an outcome, not something to name (Law 2): there are
    // three on the line and they differ, so the one that is squeezed is the one whose own cost and
    // own bid left it last in the queue for the crop.
    const mills = FIRMS.filter((f) => f.subUnit === 'flour').map((f) => partyId(f.firm));
    const throttled = mills
      .flatMap((mill) => [...events(w, 'firms.started', mill), ...events(w, 'firms.idle', mill)])
      .find((e) => e.data['bound'] === GRAIN && Number(e.data['started'] ?? 0) > 0);
    expect(throttled).toBeDefined();
    const mill = throttled?.subjects[0] ?? '';
    const planned = Number(throttled?.data['planned']);
    const started = Number(throttled?.data['started']);
    const cost = Number(throttled?.data['cost']);
    const wages = Number(throttled?.data['wages']);
    expect(started).toBeLessThan(planned);
    // B5.b: the line's whole cost for the period lands on the batch it managed to start, so a
    // throttled period IS a higher unit cost — which is what running a line below its rate does.
    const wageBill = w.journal
      .ofKind('labour.wages')
      .find((e) => e.period === throttled?.period && e.subjects.includes(mill));
    expect(wages).toBe(wageBill?.data['paid']);
    expect(cost / started).toBeGreaterThan(cost / planned);
  });
});

describe('what it offers, and what nobody takes (Goods C1, C5)', () => {
  it('offers what it cannot keep at whatever the book gives, and holds the rest above it', () => {
    const w = grainWorld(1);
    const r = w.step();
    expect(r.audit.total).toBe(0);
    // Demand is a few tonnes against the whole crop offered, so what nobody took stays where it
    // was: that is what illiquidity in goods is (C5), and nothing absorbed the rest.
    expect(w.register.quantity(FIRM_1, GRAIN)).toBeGreaterThan(25);
    const print = w.journal
      .ofKind('print')
      .filter((e) => e.subjects.includes(GRAIN))
      .pop();
    // The value of holding it is what it expects to fetch less what perishes, and the market
    // cleared at that: nobody was paid more than a buyer posted (Clearing C4.c).
    expect(print?.data['price']).toBeCloseTo(GRAIN_OPENS_AT * (1 - 0.004), 9);
  });
});

describe('the audit of a line (Goods B5, F5.b)', () => {
  it('catches a batch that cost more than anything paid for it', () => {
    // A create at a cost nobody paid: units appear carrying a value, the firm's equity rises by
    // it, and no wage bill and no drawn input is behind it. That is one cost in two places seen
    // from the other end, and the family says so rather than the world quietly getting richer.
    const conjure: SystemModule = {
      id: 'test.conjure',
      spec: 'Goods B5',
      requires: ['goods', 'firms'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.conjure',
          spec: 'Goods B5',
          cycle: 2,
          anchor: { after: 'firms.produce' },
          run: (ctx) => {
            if (ctx.period !== 1) return;
            ctx.settle({
              legs: [
                {
                  kind: 'create',
                  party: FIRM_1,
                  instrument: GRAIN,
                  qty: 5,
                  costPerUnit: 0.04,
                  toCell: none(),
                },
              ],
              cause: 'production',
              reason: 'grain that cost nobody anything',
            });
          },
        },
      ],
      participants: [],
      families: [],
    };
    const w = world(conjure);
    const r = w.step();
    const flows = r.audit.families.find((f) => f.family === 'flows');
    expect(flows?.contributions).toContain('firms');
    expect(flows?.count).toBeGreaterThan(0);
    expect(flows?.worst[0]?.owner).toBe(FIRM_1);
  });
});

describe('who is in the goods market, and who is not', () => {
  it('does not have the central bank in it (Central Bank C1, Appendix B)', () => {
    const w = grainWorld();
    for (let i = 0; i < 4; i += 1) {
      const r = w.step();
      expect(r.audit.total).toBe(0);
    }
    // Its policy is about sovereign paper. A central bank standing in the grain market with a size
    // set by its own policy is the buyer of last resort this world does not have, and it would be
    // buying the harvest at whatever the book asked.
    expect(w.register.quantity(partyId('cb.north'), GRAIN)).toBe(0);
    expect(w.register.holdersOf(GRAIN)).not.toContain('cb.north');
  });
});

describe('what varies between firms is data (Firm F4, Law 2, Law 15)', () => {
  it('declares what varies between firms and nothing else: its cost and its management', () => {
    const declared = firms().params;
    // Firm A3 and Capital Programme B1.d, and nothing else this module owns. What a thing is made
    // of, how long it takes and what survives the line are the GOOD's technology; what an hour
    // costs is what the market charged it; and there is no margin, buffer or speed anywhere in it.
    // The three per firm are: how many hours a tonne takes IT, the margin over its cost of capital
    // its management insists on, and how far ahead that management looks.
    expect(declared).toHaveLength(FIRMS.length * 3);
    expect(declared.filter((p) => p.kind === 'technology')).toHaveLength(FIRMS.length);
    // B1.d: a hurdle and a horizon are the management's own, which makes them preferences.
    expect(declared.filter((p) => p.kind === 'preference')).toHaveLength(FIRMS.length * 2);
    expect(new Set(declared.map((p) => p.unit)).size).toBe(3);
    // No two firms in a line are alike, which is what gives the venue more than one bid (Seed B4).
    const bakers = FIRMS.filter((f) => f.subUnit === 'bread').map((f) => f.labourScale);
    expect(new Set(bakers).size).toBe(bakers.length);
  });

  it('puts three firms in a line and none of them bids the same wage (Firm A3, Seed B1, B4)', () => {
    const w = world();
    for (let i = 0; i < 6; i += 1) expect(unexpected(w.step().audit)).toEqual([]);
    // Seed B1: a line is a distribution and not a single instance. Seed B4: and they are not equal,
    // because a sector of equals never produces a market — which in a labour venue means one bid.
    for (const subUnit of ['grain', 'flour', 'bread']) {
      expect(FIRMS.filter((f) => f.subUnit === subUnit).length).toBeGreaterThan(1);
    }
    // A3: the dispersion is in COST, and it comes out in what each of them will pay for an hour.
    const bakers = FIRMS.filter((f) => f.subUnit === 'bread');
    const bids = new Map<string, number>();
    for (const f of bakers) {
      const plan = last(w, 'firms.plan', partyId(f.firm));
      if (plan?.data['planned'] === true) bids.set(f.firm, Number(plan.data['wageBid']));
    }
    expect(bids.size).toBeGreaterThan(1);
    expect(new Set(bids.values()).size).toBe(bids.size);
    // The firm that does more with an hour will pay more for one, so the leanest is never the
    // marginal employer and the one that takes the most hours to the tonne is priced out first.
    const ranked = [...bids].sort((a, b) => b[1] - a[1]).map(([firm]) => firm);
    const byScale = [...bakers].sort((a, b) => a.labourScale - b.labourScale).map((f) => f.firm);
    expect(ranked).toEqual(byScale.filter((f) => bids.has(f)));
  });

  it('runs three lines that never meet through one decision (Firm F4)', () => {
    const w = world();
    for (let i = 0; i < 8; i += 1) {
      expect(unexpected(w.step().audit)).toEqual([]);
    }
    // The baker buys flour because bread takes flour, and the miller buys grain because flour
    // takes grain: one decision, three lines, and the chain is the recipes and nothing else.
    expect(w.journal.ofKind('print').some((e) => e.subjects.includes(BREAD))).toBe(true);
    expect(w.journal.ofKind('print').some((e) => e.subjects.includes(FLOUR))).toBe(true);
    for (const firm of [FIRM_1, FIRM_2, FIRM_3]) {
      expect(last(w, 'firms.plan', firm)).toBeDefined();
    }
  });
});
