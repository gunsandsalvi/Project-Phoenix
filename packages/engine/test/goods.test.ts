/**
 * Goods: a thing with a physical unit, made from fixed quantities of other things, held at what it
 * cost, sold in a market of its own, and perishing in store.
 *
 * @spec Goods A1 Goods A2 Goods A2.a Goods A2.b Goods A3 Goods A4 Goods C1 Goods C2 Goods C4 Goods C5 Goods C6 Goods E1 Goods E2 Goods E2.a Goods E2.c Goods E4 Goods E4.a Commodities Spot D5 Commodities Spot F1 Law 9 XI-6
 */
import { describe, expect, it } from 'vitest';
import {
  FIRM,
  PHX,
  REGION,
  assemble,
  displayName,
  foundationSpec,
  goodId,
  goodKindId,
  goodMarketId,
  goodTerms,
  goods,
  isDestroyLeg,
  isMoneyLeg,
  none,
  partyId,
  recipeParam,
  spoilageParam,
  type GoodDecl,
  type MechanismContext,
  type Order,
  type SystemModule,
  type World,
} from '../src/index.js';

const FIRM_1 = partyId('firm.1');
const FIRM_2 = partyId('firm.2');
const FIRM_3 = partyId('firm.3');

/** Two goods that do not perish, so a market test says what the market did and nothing else. */
const STONE: GoodDecl = {
  subUnit: 'stone',
  unit: 'tonnes',
  name: 'stone',
  spoilagePerPeriod: 0,
  spoilageWhy: 'Stone in a yard is stone next week.',
  inputs: [],
  labourHoursPerUnit: 4,
  labourWhy: 'Hours at the quarry per tonne.',
  yieldRate: 1,
  yieldWhy: 'Nothing is lost cutting stone, so a batch test says what the batch did and nothing else.',
  leadTimePeriods: 0,
  leadTimeWhy: 'Cut and stacked inside the week.',
};

const GRAVEL: GoodDecl = {
  subUnit: 'gravel',
  unit: 'tonnes',
  name: 'gravel',
  spoilagePerPeriod: 0,
  spoilageWhy: 'Nor does gravel.',
  inputs: [{ subUnit: 'stone', qtyPerUnit: 1.5, why: 'A tonne and a half of stone per tonne.' }],
  labourHoursPerUnit: 1,
  labourWhy: 'Hours at the crusher per tonne.',
  yieldRate: 1,
  yieldWhy: 'Nor crushing it.',
  leadTimePeriods: 0,
  leadTimeWhy: 'Crushed inside the week.',
};

const KEEPS: readonly GoodDecl[] = [STONE, GRAVEL];

const STONE_ID = goodId('stone', REGION);
const GRAVEL_ID = goodId('gravel', REGION);
const BREAD_ID = goodId('bread', REGION);

/** The kernel, the foundation's parties, and one goods registry under test. */
function world(rows: readonly GoodDecl[], ...extra: SystemModule[]): World {
  const spec = foundationSpec('goods');
  const kernelOnly = spec.modules.filter(
    (m) => m.id === 'sovereign-instruments' || m.id === 'seed.foundation',
  );
  return assemble({ ...spec, modules: [...kernelOnly, goods(rows), ...extra] });
}

/** A module that runs one function each period: whatever the test is about. */
function acts(run: (ctx: MechanismContext) => void, participants: SystemModule['participants'] = []): SystemModule {
  return {
    id: 'test.acts',
    spec: 'Goods B',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      { name: 'test.acts', spec: 'Goods B1', cycle: 0, anchor: { after: 'corporateActions' }, run },
    ],
    participants,
    families: [],
  };
}

/** Units of a good on somebody's book, at what they cost (Goods E1). */
function make(ctx: MechanismContext, holder: string, id: string, qty: number, cost: number): void {
  ctx.settle({
    legs: [
      {
        kind: 'create',
        party: partyId(holder),
        instrument: goodId(id, REGION),
        qty,
        costPerUnit: cost,
        toCell: none(),
      },
    ],
    cause: 'seed',
    reason: `the opening stock of ${id}`,
  });
}

describe('what a good is (Goods A)', () => {
  it('has a physical unit, a market in the money of where it is, and nobody who issued it', () => {
    const w = world(KEEPS);
    const stone = w.instruments.get(STONE_ID);
    expect(stone.issuer.some).toBe(false);
    expect(stone.unit).toBe('tonnes');
    expect(stone.ccy).toBe(PHX);
    const profile = w.registry.instrumentKind(goodKindId('stone'));
    expect(profile.physical).toBe(true);
    // XI-6, E1: its price comes from its market; what a holder carries it at is what it cost.
    expect(profile.pricing).toBe('cleared');
    expect(profile.carry).toBe('cost');
    expect(profile.liabilityOfIssuer).toBe(false);
    // C6, C2: one market per (region, sub-unit), clearing in that region's money.
    const market = w.market(goodMarketId('stone', REGION));
    expect(market.instrument).toBe(STONE_ID);
    expect(market.ccy).toBe(PHX);
    expect(market.rationing).toBe('proRata');
    // Law 9: named as a market names it — what it is and where — never by its identifier.
    expect(displayName(stone, w.parties, w.registry)).toBe('stone, North');
  });

  it('is made from fixed physical quantities, declared in physical units (A2.a, A2.b)', () => {
    const w = world(KEEPS);
    const decl = w.params.decl(recipeParam('gravel', 'stone'));
    expect(decl.value).toBe(1.5);
    expect(decl.unit).toBe('tonnes of stone per tonnes of gravel');
    expect(decl.kind).toBe('technology');
    // The recipe in the terms names the declared number rather than carrying a second copy of it.
    const terms = goodTerms(w.instruments.get(GRAVEL_ID));
    expect(terms.recipe.inputs[0]?.qtyPerUnit).toBe(recipeParam('gravel', 'stone'));
    expect(w.params.get(spoilageParam('stone'))).toBe(0);
  });

  it('refuses a recipe denominated in money: that is a substitution nobody declared (A2.b)', () => {
    const spec = foundationSpec('goods');
    const kernelOnly = spec.modules.filter(
      (m) => m.id === 'sovereign-instruments' || m.id === 'seed.foundation',
    );
    const inMoney = goods(KEEPS);
    const doctored: SystemModule = {
      ...inMoney,
      params: inMoney.params.map((p) =>
        p.id === recipeParam('gravel', 'stone') ? { ...p, unit: `PHX per tonne of gravel` } : p,
      ),
    };
    expect(() => assemble({ ...spec, modules: [...kernelOnly, doctored] })).toThrow(/A2\.b/);
  });

  it('makes a thing out of the things it is made from, and nothing out of nothing (Commodities Spot F1)', () => {
    const w = world(
      KEEPS,
      acts((ctx) => {
        if (ctx.period === 1) make(ctx, 'firm.1', 'stone', 15, 2);
        if (ctx.period === 2) {
          ctx.settle({
            legs: [
              {
                kind: 'destroy',
                party: FIRM_1,
                instrument: STONE_ID,
                qty: 1.5,
                why: 'consumed',
                fromCell: none(),
              },
              {
                kind: 'create',
                party: FIRM_1,
                instrument: GRAVEL_ID,
                qty: 1,
                costPerUnit: 3,
                toCell: none(),
              },
            ],
            cause: 'production',
            reason: 'a tonne of gravel from a tonne and a half of stone',
          });
        }
      }),
    );
    w.step();
    const r = w.step();
    expect(r.audit.total).toBe(0);
    expect(w.register.quantity(FIRM_1, STONE_ID)).toBeCloseTo(13.5, 12);
    expect(w.register.quantity(FIRM_1, GRAVEL_ID)).toBe(1);
  });
});

describe('inventory (Goods E)', () => {
  it('holds lots at what they cost, and values them there while nothing has printed', () => {
    const w = world(
      KEEPS,
      acts((ctx) => {
        if (ctx.period === 1) make(ctx, 'firm.1', 'stone', 10, 2);
      }),
    );
    const r = w.step();
    expect(r.audit.total).toBe(0);
    const h = w.register.holding(FIRM_1, STONE_ID);
    expect(h.some && h.value.lots[0]?.basisPerUnit).toBe(2);
    // XI-6: value is a function of the lots, and for a good carried at cost that is what it cost.
    expect(w.valuation.valueOfLots(STONE_ID, h.some ? h.value.lots : [], w.period)).toBe(20);
    // A good that has never traded has no print, and nothing pretends otherwise.
    expect(w.prices.latest(STONE_ID, w.period).some).toBe(false);
  });

  it('is written down to the print when the market falls below cost, and never up (E2, E2.a, E2.c)', () => {
    const build = (at: number): World =>
      world(
        KEEPS,
        acts(
          (ctx) => {
            if (ctx.period === 1) make(ctx, 'firm.1', 'stone', 10, 2);
          },
          [
            {
              partyKind: FIRM,
              orders: (view, m): readonly Order[] => {
                if (m.instrument !== STONE_ID || view.period < 2) return [];
                if (view.self.id === FIRM_1) return [{ party: FIRM_1, side: 'sell', price: at, qty: 1 }];
                if (view.self.id === FIRM_2) return [{ party: FIRM_2, side: 'buy', price: at, qty: 1 }];
                return [];
              },
            },
          ],
        ),
      );
    const down = build(1);
    down.step();
    const before = down.register.equity(FIRM_1);
    const r = down.step();
    expect(r.audit.total).toBe(0);
    const h = down.register.holding(FIRM_1, STONE_ID);
    expect(h.some && h.value.lots[0]?.basisPerUnit).toBe(1);
    // E3: a charge to income in the period it happens — one realised on the tonne that sold at 1
    // against a cost of 2, and one on each of the nine that stayed and are now worth less.
    expect(down.register.equity(FIRM_1)).toBeCloseTo(before - 10, 9);

    const up = build(3);
    up.step();
    const flat = up.register.equity(FIRM_1);
    up.step();
    const held = up.register.holding(FIRM_1, STONE_ID);
    expect(held.some && held.value.lots[0]?.basisPerUnit).toBe(2);
    // The sale at 3 is a realised gain on the one tonne that left; the nine that stayed do not
    // move at all, because inventory is never carried above cost (E2.c).
    expect(up.register.equity(FIRM_1)).toBeCloseTo(flat + 1, 9);
  });
});

describe('what perishes (Goods E4)', () => {
  const bread = (): World => {
    const spec = foundationSpec('goods.perish');
    const modules = spec.modules.filter(
      (m) => m.id === 'sovereign-instruments' || m.id === 'seed.foundation' || m.id === 'goods',
    );
    return assemble({
      ...spec,
      modules: [
        ...modules,
        acts((ctx) => {
          if (ctx.period === 1) make(ctx, 'firm.1', 'bread', 10, 2);
        }),
      ],
    });
  };

  it('leaves the world at the lot own cost, with nobody on the other side and no money moved', () => {
    const w = bread();
    const r = w.step();
    expect(r.audit.total).toBe(0);
    // A quarter of the ten tonnes went stale in the week it sat there.
    expect(w.register.quantity(FIRM_1, BREAD_ID)).toBeCloseTo(7.5, 12);
    // What is issued is what is held, by everybody: this baker is not the only one in the world.
    expect(w.instruments.get(BREAD_ID).issued).toBeCloseTo(
      w.register.heldTotal(BREAD_ID).value,
      9,
    );
    const spoiled = w.ledger
      .inPeriod(w.period)
      .filter(
        (x) =>
          x.outcome === 'settled' &&
          x.instruction.legs.some((l) => isDestroyLeg(l) && l.party === FIRM_1 && l.instrument === BREAD_ID),
      );
    expect(spoiled).toHaveLength(1);
    const legs = spoiled[0]?.instruction.legs ?? [];
    expect(legs).toHaveLength(1);
    // E4.a: what perished is units. A storage fee is cash paid to whoever stores the goods, and
    // there is nobody storing them here, so no money moves with the spoilage at all.
    expect(legs.some(isMoneyLeg)).toBe(false);
    const leg = legs[0];
    expect(leg !== undefined && isDestroyLeg(leg) && leg.why).toBe('perished');
    // It was made at 2 and 2.5 tonnes of it perished: the charge is what those units cost, and
    // settlement is what said so — the event carries what it charged, not a number recomputed.
    const ev = w.journal
      .ofKind('goods.perished')
      .filter((e) => e.subjects.includes(FIRM_1) && e.subjects.includes(BREAD_ID));
    expect(ev).toHaveLength(1);
    expect(ev[0]?.data['unitsPerMember']).toBeCloseTo(2.5, 12);
    expect(ev[0]?.data['chargePerMember']).toBeCloseTo(-2.5 * 2, 9);
    expect(ev[0]?.public).toBe(false);
  });

  it('goes on perishing, on what is left', () => {
    const w = bread();
    w.step();
    const r = w.step();
    expect(r.audit.total).toBe(0);
    expect(w.register.quantity(FIRM_1, BREAD_ID)).toBeCloseTo(7.5 * 0.75, 12);
  });
});

describe('the market (Goods C)', () => {
  const trading = (bids: readonly { party: string; price: number; qty: number }[]): World =>
    world(
      KEEPS,
      acts(
        (ctx) => {
          if (ctx.period === 1) make(ctx, 'firm.1', 'stone', 10, 1);
        },
        [
          {
            partyKind: FIRM,
            orders: (view, m): readonly Order[] => {
              if (m.instrument !== STONE_ID || view.period < 2) return [];
              if (view.self.id === FIRM_1) return [{ party: FIRM_1, side: 'sell', price: 1, qty: 4 }];
              const bid = bids.find((b) => b.party === view.self.id);
              return bid === undefined
                ? []
                : [{ party: partyId(bid.party), side: 'buy', price: bid.price, qty: bid.qty }];
            },
          },
        ],
      ),
    );

  it('leaves with the seller what nobody bought, and pays it in its own money (C5, C6)', () => {
    const w = trading([{ party: 'firm.2', price: 1.2, qty: 1 }]);
    w.step();
    const cash = w.cash(FIRM_1, PHX);
    const r = w.step();
    expect(r.audit.total).toBe(0);
    const m = r.markets.find((x) => x.market === goodMarketId('stone', REGION));
    expect(m?.outcome).toBe('cleared');
    expect(m?.price.some === true && m.price.value).toBe(1);
    expect(m?.settledVolume).toBe(1);
    // C5: illiquidity in goods is unsold stock. Nine tonnes stayed where they were.
    expect(w.register.quantity(FIRM_1, STONE_ID)).toBe(9);
    expect(w.cash(FIRM_1, PHX)).toBeCloseTo(cash + 1, 9);
  });

  it('rations pro rata when the buyers want more than there is (C4)', () => {
    const w = trading([
      { party: 'firm.2', price: 1.2, qty: 6 },
      { party: 'firm.3', price: 1.2, qty: 2 },
    ]);
    w.step();
    const r = w.step();
    expect(r.audit.total).toBe(0);
    expect(w.register.quantity(FIRM_2, STONE_ID)).toBeCloseTo(3, 12);
    expect(w.register.quantity(FIRM_3, STONE_ID)).toBeCloseTo(1, 12);
    expect(w.register.quantity(FIRM_1, STONE_ID)).toBeCloseTo(6, 12);
  });
});

describe('the units identity (Part XII)', () => {
  it('is a contribution of its own and holds across making, selling and perishing', () => {
    const spec = foundationSpec('goods.units');
    const modules = spec.modules.filter(
      (m) => m.id === 'sovereign-instruments' || m.id === 'seed.foundation' || m.id === 'goods',
    );
    const w = assemble({
      ...spec,
      modules: [
        ...modules,
        acts(
          (ctx) => {
            if (ctx.period === 1) make(ctx, 'firm.1', 'bread', 8, 2);
            if (ctx.period === 3) {
              ctx.settle({
                legs: [
                  {
                    kind: 'destroy',
                    party: FIRM_1,
                    instrument: BREAD_ID,
                    qty: 1,
                    why: 'consumed',
                    fromCell: none(),
                  },
                ],
                cause: 'production',
                reason: 'a tonne eaten',
              });
            }
          },
          [
            {
              partyKind: FIRM,
              orders: (view, m): readonly Order[] => {
                if (m.instrument !== BREAD_ID || view.period !== 2) return [];
                if (view.self.id === FIRM_1) return [{ party: FIRM_1, side: 'sell', price: 2, qty: 2 }];
                if (view.self.id === FIRM_2) return [{ party: FIRM_2, side: 'buy', price: 2, qty: 2 }];
                return [];
              },
            },
          ],
        ),
      ],
    });
    for (let i = 0; i < 6; i += 1) {
      const r = w.step();
      const units = r.audit.families.find((f) => f.family === 'units');
      expect(units?.contributions).toContain('goods');
      expect(units?.count).toBe(0);
    }
  });
});
