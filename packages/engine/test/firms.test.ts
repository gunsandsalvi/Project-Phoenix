/**
 * Firms: a decision taken from the firm's own view, a line that consumes what its recipe names and
 * carries what it cost, and a supply schedule with a reason behind every step.
 *
 * @spec Firm B1 Firm B2 Firm B3 Firm B4 Firm B5 Firm C1 Firm D1 Firm E1 Firm E2 Firm E6 Firm E7 Goods B1 Goods B1.b Goods B2 Goods B3 Goods B4 Goods B5 Goods B5.a Goods B5.b Goods C1 Goods C5 Goods E1 Goods F5.a Labour C1 Labour C5 Expectations C2 XI-16 Law 2
 */
import { describe, expect, it } from 'vitest';
import {
  HOUSEHOLD,
  PHX,
  REGION,
  assemble,
  firms,
  foundationSpec,
  goodId,
  goodMarketId,
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

const FIRM_1 = partyId('firm.1'); // grain, from field labour alone
const FIRM_2 = partyId('firm.2'); // flour, from grain
const FIRM_3 = partyId('firm.3'); // bread, from flour

const GRAIN = goodId('grain', REGION);
const FLOUR = goodId('flour', REGION);
const BREAD = goodId('bread', REGION);
const WIP_GRAIN = wipId('grain', REGION);

/** What the seed will state at 4.7: what things were fetching, and a stock for the flows to act on. */
interface Opening {
  readonly prices: Readonly<Record<string, number>>;
  readonly stock: readonly { readonly firm: string; readonly good: string; readonly qty: number; readonly basis: number }[];
}

function opening(o: Opening): SystemModule {
  return {
    id: 'test.opening',
    spec: 'Seed C4',
    requires: ['goods', 'firms', 'seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    participants: [],
    families: [],
    seed(ctx) {
      for (const [subUnit, price] of Object.entries(o.prices)) {
        ctx.prices.write({
          instrument: goodId(subUnit, REGION),
          market: goodMarketId(subUnit, REGION),
          period: ctx.period,
          price,
          ccy: PHX,
          provenance: { kind: 'opening' },
        });
      }
      for (const row of o.stock) {
        ctx.endowUnits(partyId(row.firm), goodId(row.good, REGION), row.qty, row.basis);
      }
    },
  };
}

/**
 * A stand-in for the demand that arrives at 4.6: a household cell that bids for what it consumes.
 * It is the retired cohort, which never takes a job and therefore never splits, so what it bids is
 * the same every period and a test can say what the market did.
 */
function buyer(subUnit: string, price: number, qtyPerCell: number): SystemModule {
  const instrument = goodId(subUnit, REGION);
  return {
    id: 'test.buyer',
    spec: 'Goods C3',
    requires: ['goods'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    participants: [
      {
        partyKind: HOUSEHOLD,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          if (m.instrument !== instrument) return [];
          if (view.self.representation !== 'cell' || view.self.key.cohort !== 'retired') return [];
          return [{ party: view.self.id, side: 'buy', price, qty: qtyPerCell }];
        },
      },
    ],
    families: [],
  };
}

function world(...extra: readonly SystemModule[]): World {
  const spec = foundationSpec('firms');
  return assemble({ ...spec, modules: [...spec.modules, ...extra] });
}

/** A world where grain has a price, a small stock and a buyer: the chain the seed will open with. */
function grainWorld(qtyPerCell = 10): World {
  return world(
    opening({
      prices: { grain: 0.05, flour: 0.08, bread: 0.15 },
      stock: [{ firm: 'firm.1', good: 'grain', qty: 50, basis: 0.04 }],
    }),
    buyer('grain', 0.05, qtyPerCell),
  );
}

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
    const w = grainWorld();
    w.step();
    const sold = w.register.quantity(FIRM_1, GRAIN);
    expect(sold).toBeLessThan(50);
    const r = w.step();
    expect(r.audit.total).toBe(0);
    const plan = last(w, 'firms.plan', FIRM_1);
    expect(plan?.data['planned']).toBe(true);
    expect(plan?.data['hours']).toBeGreaterThan(0);
    // Labour C1, C1.a: the most it will pay for an hour is what an hour is worth to it — the
    // output an hour makes possible, at the price it expects, less what the recipe else takes.
    // The price it expects is the one it sold at, which is its own and not the opening print.
    const expected = plan?.data['expectedPrice'];
    const worth = ((typeof expected === 'number' ? expected : 0) * 0.92) / 9;
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
    const w = grainWorld();
    for (let i = 0; i < 3; i += 1) w.step();
    const started = last(w, 'firms.started', FIRM_1);
    expect(started).toBeDefined();
    const batch = started?.data['started'];
    expect(typeof batch === 'number' ? batch : 0).toBeGreaterThan(0);
    // B3: the batch is a thing on its own book, carrying what it has cost so far.
    expect(w.register.quantity(FIRM_1, WIP_GRAIN)).toBeCloseTo(typeof batch === 'number' ? batch : 0, 9);
    // B5: what it cost is the inputs it drew — none, for a thing grown from labour — plus the
    // wage bill the period paid, and that is the whole of it.
    expect(started?.data['cost']).toBe(started?.data['wages']);
    // B3: grain takes two periods, so nothing has come off the line yet.
    expect(events(w, 'firms.produced', FIRM_1)).toHaveLength(0);
    const r = w.step();
    expect(r.audit.total).toBe(0);
    expect(events(w, 'firms.produced', FIRM_1)).toHaveLength(0);
    const yielded = w.step();
    expect(yielded.audit.total).toBe(0);
    const made = last(w, 'firms.produced', FIRM_1);
    expect(made).toBeDefined();
    const startedUnits = made?.data['started'];
    const finished = made?.data['finished'];
    // B4: not everything started is finished, and the scrap is units, at the point they would
    // have been made — never a rate applied to a value.
    expect(typeof finished === 'number' ? finished : 0).toBeCloseTo(
      (typeof startedUnits === 'number' ? startedUnits : 0) * 0.92,
      9,
    );
    expect(made?.data['scrapped']).toBeGreaterThan(0);
    // B4: what survives carries the whole batch, so a survivor is dearer than a unit started.
    const perUnit = made?.data['costPerUnit'];
    expect(typeof perUnit === 'number' ? perUnit : 0).toBeGreaterThan(0);
  });

  it('capitalises nothing in a period that starts nothing (Goods B5.a, F5.a)', () => {
    const w = grainWorld();
    w.step();
    w.step();
    // It has just hired: nobody is productive yet, so the plan cannot be started and the wage
    // bill is what it is — a period expense, in the period it was incurred.
    const r = w.step();
    expect(r.audit.total).toBe(0);
    const idle = events(w, 'firms.idle', FIRM_1);
    const started = events(w, 'firms.started', FIRM_1);
    expect(idle.length + started.length).toBeGreaterThan(0);
    for (const e of idle) expect(e.data['bound']).toBe('labour');
  });

  it('is bound by the inputs on hand, and says which one bound it (Goods B1.b, B5.b)', () => {
    // The mill has flour to sell and a buyer for it, and only ten tonnes of grain to make more
    // from — and nobody selling grain, because the farm has none. Its line is throttled.
    const w = world(
      opening({
        prices: { grain: 0.05, flour: 0.4, bread: 0.15 },
        stock: [
          { firm: 'firm.2', good: 'flour', qty: 60, basis: 0.3 },
          { firm: 'firm.2', good: 'grain', qty: 10, basis: 0.05 },
        ],
      }),
      buyer('flour', 0.4, 10),
    );
    for (let i = 0; i < 4; i += 1) {
      const r = w.step();
      expect(r.audit.total).toBe(0);
    }
    const throttled = events(w, 'firms.started', FIRM_2).find((e) => e.data['bound'] === GRAIN);
    expect(throttled).toBeDefined();
    const planned = Number(throttled?.data['planned']);
    const started = Number(throttled?.data['started']);
    const cost = Number(throttled?.data['cost']);
    const wages = Number(throttled?.data['wages']);
    expect(started).toBeLessThan(planned);
    // B5.b: the line's whole cost for the period lands on the batch it managed to start, so a
    // throttled period IS a higher unit cost — which is what running a line below its rate does.
    const wageBill = w.journal
      .ofKind('labour.wages')
      .find((e) => e.period === throttled?.period && e.subjects.includes(FIRM_2));
    expect(wages).toBe(wageBill?.data['paid']);
    expect(cost / started).toBeGreaterThan(cost / planned);
  });
});

describe('what it offers, and what nobody takes (Goods C1, C5)', () => {
  it('offers what it cannot keep at whatever the book gives, and holds the rest above it', () => {
    const w = grainWorld(1);
    const r = w.step();
    expect(r.audit.total).toBe(0);
    // Demand is four tonnes against fifty offered, so what nobody took stays where it was: that
    // is what illiquidity in goods is (C5), and nothing absorbed the rest.
    expect(w.register.quantity(FIRM_1, GRAIN)).toBeGreaterThan(40);
    const print = w.journal
      .ofKind('print')
      .filter((e) => e.subjects.includes(GRAIN))
      .pop();
    // The value of holding it is what it expects to fetch less what perishes, and the market
    // cleared at that: nobody was paid more than a buyer posted (Clearing C4.c).
    expect(print?.data['price']).toBeCloseTo(0.05 * (1 - 0.004), 9);
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
    const w = world(
      opening({ prices: { grain: 0.05, flour: 0.08, bread: 0.15 }, stock: [] }),
      conjure,
    );
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
  it('declares no number of its own: every number it decides with is somebody else technology', () => {
    expect(firms().params).toHaveLength(0);
  });

  it('runs three lines that never meet through one decision (Firm F4)', () => {
    const w = world(
      opening({
        prices: { grain: 0.05, flour: 0.4, bread: 1.2 },
        stock: [
          { firm: 'firm.1', good: 'grain', qty: 50, basis: 0.04 },
          { firm: 'firm.2', good: 'flour', qty: 40, basis: 0.3 },
          { firm: 'firm.3', good: 'bread', qty: 20, basis: 1 },
        ],
      }),
      buyer('bread', 1.2, 4),
    );
    for (let i = 0; i < 6; i += 1) {
      const r = w.step();
      expect(r.audit.total).toBe(0);
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
