/**
 * The doors item 4 needs: a module's own state, what a party expects, things that are made and used
 * up rather than issued, and a write-down that only ever goes one way.
 *
 * @spec Expectations A1 Expectations A2 Expectations A2.b Goods A1 Commodities Spot D5 Goods E1 Goods E2 Goods E2.c Goods E4 Commodities Spot F1 Law 4 Observer E3
 */
import { describe, expect, it } from 'vitest';
import {
  ANNUAL,
  BANK_A,
  BANK_B,
  FIRM,
  InvalidRegistry,
  USD,
  assemble,
  instrumentId,
  instrumentKindId,
  marketId,
  moneyInstrumentId,
  none,
  partyId,
  some,
  unitId,
  type InstrumentKindProfile,
  type Leg,
  type MechanismContext,
  type Outlook,
  type SystemModule,
  type World,
} from '../src/index.js';
import { CENT_TICK } from '../src/registry/grid.js';
import { rigSpec, withDependencies, mergeModules } from './rig.js';
import { paidTo } from './expected.js';
import { notDealing } from './no-dealing.js';
import { asQty } from '../src/core/tick.js';

const WHEAT = instrumentKindId('good.wheat');
const TONNES = unitId('tonnes');
const WHEAT_ID = instrumentId('good.wheat.north');
const FIRM_1 = partyId('firm.1');

/** A physical kind: made and used up, nobody's liability, carried at what it cost (Goods A1, E1). */
const wheat: InstrumentKindProfile = {
  id: WHEAT,
  pricing: 'cleared',
  // Law 8: a kind somebody can post a limit in says what its smallest increment is, and a commodity
  // is quoted in cents the tonne — the same grid this world's own goods use (registry/grid.ts).
  priceTick: CENT_TICK,
  carry: 'cost',
  liabilityOfIssuer: false,
  physical: true,
  unit: () => TONNES,
  ranking: () => ({ seniority: 0, secured: [], claim: 'nothing: it is owned outright' }),
  validateTerms: () => undefined,
  displayName: () => 'wheat, North',
  due: () => [],
  accrued: () => 0,
  cashFlows: () => [],
  // E2, E2.a: lower of cost and what it would fetch, and never the other way (E2.c).
  carriedAt: (_i, lot, marked) =>
    marked.some && marked.value < lot.basisPerUnit ? some(marked.value) : none<number>(),
};

/** The same kind, but claiming it may be carried above cost: what E2.c forbids for a non-dealer. */
const wheatMarkedUp: InstrumentKindProfile = {
  ...wheat,
  carriedAt: (_i, _lot, marked) => marked,
};

function goodsModule(profile: InstrumentKindProfile, run: (ctx: MechanismContext) => void): SystemModule {
  return {
    id: 'test.goods',
    spec: 'Goods A, E',
    requires: ['seed.foundation'],
    instrumentKinds: [profile],
    partyKinds: [],
    curveFamilies: [],
    // Law 4: the tonne is the GOODS module's unit and this world has it. A module that declared it
    // again would be a second writer of how many grams one is, and assembly refuses that — which is
    // the guard doing its job rather than a problem with the guard.
    units: [],
    params: [],
    phases: [
      {
        name: 'test.goods',
        spec: 'Goods B',
        cycle: 0,
        anchor: { after: 'corporateActions' },
        run,
      },
    ],
    participants: [],
    families: [],
  };
}

const WHEAT_MARKET = marketId('mkt.wheat.north');

/** Somebody on the other side of the wheat market, so it prints (Clearing A1.a). */
function trader(price: number): SystemModule {
  return {
    id: 'test.trader',
    spec: 'Clearing B2',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [],
    participants: [
      {
        partyKind: FIRM,
        orders: (view, m) => {
          if (m.instrument !== WHEAT_ID || view.period < 2) return [];
          if (view.self.id === FIRM_1)
            return [{ party: FIRM_1, side: 'sell' as const, price, qty: asQty(1) }];
          if (view.self.id === partyId('firm.2'))
            return [{ party: partyId('firm.2'), side: 'buy' as const, price, qty: asQty(1) }];
          return [];
        },
      },
    ],
    families: [],
  };
}

function world(...extra: SystemModule[]): World {
  const spec = rigSpec('doors');
  const kernelOnly = withDependencies(spec.modules, (m) =>
m.id === 'sovereign-instruments' ||
      m.id === 'seed.foundation' ||
      m.id === 'seed.funding' ||
      m.id === 'banks' ||
      m.id === 'money-market',
  ).map(notDealing);
  return assemble({ ...spec, modules: mergeModules(kernelOnly, extra) });
}

/** Bring the wheat instrument into the world, then make and use up units of it. */
function openWheat(ctx: MechanismContext): void {
  if (ctx.instruments.has(WHEAT_ID)) return;
  ctx.issue({
    id: WHEAT_ID,
    kind: WHEAT,
    issuer: none(),
    ccy: USD,
    terms: { kind: WHEAT },
    market: some(WHEAT_MARKET),
  });
  ctx.openMarket({
    id: WHEAT_MARKET,
    name: 'wheat, North',
    instrument: WHEAT_ID,
    ccy: USD,
    rationing: 'proRata',
  });
}

describe("a module's own state (Law 4)", () => {
  it('is created once, kept between periods, and shown to the observer as the data it is', () => {
    const seen: number[] = [];
    const w = world(
      goodsModule(wheat, (ctx) => {
        const s = ctx.state<{ count: number }>('ledger', () => ({ count: 0 }));
        s.count += 1;
        seen.push(s.count);
      }),
    );
    w.step();
    w.step();
    w.step();
    expect(seen).toEqual([1, 2, 3]);
    const slots = w.stateSlots();
    expect(slots['test.goods/ledger']).toEqual({ count: 3 });
    // Observer E3: what the surface gets is a copy; writing to it changes nothing in the world.
    (slots['test.goods/ledger'] as { count: number }).count = 99;
    expect((w.stateSlots()['test.goods/ledger'] as { count: number }).count).toBe(3);
  });
});

describe('what a party expects (Expectations A2)', () => {
  it('has no answer at all until a module says what a party expects', () => {
    const w = world();
    expect(w.participantView(FIRM_1).outlook('goods.price.wheat').some).toBe(false);
  });

  it('comes from the one module that keeps them, through that module own state', () => {
    const outlooks: SystemModule = {
      ...goodsModule(wheat, () => undefined),
      id: 'test.outlooks',
      outlooks: {
        of: (ctx, party, variable) => {
          const s = ctx.state<Record<string, number>>('outlooks', () => ({ 'firm.1|x': 3 }));
          const v = s[`${party}|${variable}`];
          return v === undefined
            ? none<Outlook>()
            : some<Outlook>({
                expected: v,
                unit: 'USD',
                per: ANNUAL,
                confidence: 0,
                formed: ctx.period,
              });
        },
        variables: (ctx, party) =>
          Object.keys(ctx.state<Record<string, number>>('outlooks', () => ({ 'firm.1|x': 3 })))
            .filter((k) => k.startsWith(`${party}|`))
            .map((k) => k.slice(`${party}|`.length)),
      },
    };
    const w = world(outlooks);
    const got = w.participantView(FIRM_1).outlook('x');
    expect(got.some).toBe(true);
    expect(got.some ? got.value.expected : null).toBe(3);
    // A2: the door also says WHICH variables this party has an outlook of, so nothing has to guess
    // a name — and a party that has observed nothing answers with nothing.
    expect(w.outlookVariables(FIRM_1)).toEqual(['x']);
    expect(w.outlookVariables(BANK_A)).toEqual([]);
    // A2.b: nobody else's expectation is reachable, and an unobserved variable has none.
    expect(w.participantView(FIRM_1).outlook('y').some).toBe(false);
    expect(w.participantView(BANK_A).outlook('x').some).toBe(false);
  });

  it('refuses a second writer of what a party expects (Law 4)', () => {
    const nothing = { of: () => none<Outlook>(), variables: () => [] };
    const one: SystemModule = { ...goodsModule(wheat, () => undefined), id: 'a', outlooks: nothing };
    const two: SystemModule = { ...goodsModule(wheat, () => undefined), id: 'b', outlooks: nothing };
    expect(() => world(one, two)).toThrow(InvalidRegistry);
  });
});

describe('things that are made and used up (Goods E4, Commodities Spot F1)', () => {
  it('come into existence on one book, at what they cost, and the stock says so', () => {
    const w = world(
      goodsModule(wheat, (ctx) => {
        openWheat(ctx);
        if (ctx.period !== 1) return;
        ctx.settle({
          legs: [
            { kind: 'create', party: FIRM_1, instrument: WHEAT_ID, qty: 10, costPerUnit: 2, toCell: none() },
          ],
          cause: 'seed',
          reason: 'the opening harvest',
        });
      }),
    );
    const r = w.step();
    expect(r.audit.total).toBe(0);
    expect(w.register.quantity(FIRM_1, WHEAT_ID)).toBe(10);
    expect(w.instruments.get(WHEAT_ID).issued).toBe(10);
    // E1: the lot carries what it cost, so what it is worth is a question about the market.
    const h = w.register.holding(FIRM_1, WHEAT_ID);
    expect(h.some && h.value.lots[0]?.basisPerUnit).toBe(2);
  });

  it('leave it again, and the units identity is checked against what said why (Part XII)', () => {
    const w = world(
      goodsModule(wheat, (ctx) => {
        openWheat(ctx);
        if (ctx.period === 1) {
          ctx.settle({
            legs: [
              { kind: 'create', party: FIRM_1, instrument: WHEAT_ID, qty: 10, costPerUnit: 2, toCell: none() },
            ],
            cause: 'seed',
            reason: 'the opening harvest',
          });
        }
        if (ctx.period === 2) {
          ctx.settle({
            legs: [
              { kind: 'destroy', party: FIRM_1, instrument: WHEAT_ID, qty: 4, why: 'perished', fromCell: none() },
            ],
            cause: 'production',
            reason: 'a batch that spoiled',
          });
        }
      }),
    );
    w.step();
    const r = w.step();
    expect(r.audit.total).toBe(0);
    expect(w.register.quantity(FIRM_1, WHEAT_ID)).toBe(6);
    expect(w.instruments.get(WHEAT_ID).issued).toBe(6);
  });

  it('refuses units that come into the world by anything but production (Commodities Spot F1)', () => {
    const w = world(
      goodsModule(wheat, (ctx) => {
        openWheat(ctx);
        if (ctx.period !== 1) return;
        ctx.settle({
          legs: [
            { kind: 'create', party: FIRM_1, instrument: WHEAT_ID, qty: 1, costPerUnit: 1, toCell: none() },
          ],
          // A trade moves units that exist; it does not make them.
          cause: 'trade',
          reason: 'wheat from nowhere',
        });
      }),
    );
    expect(() => w.step()).toThrow();
  });

  it('lets a thing drawn from labour and land alone be produced with nothing destroyed (Goods A2)', () => {
    // The first stage of every chain is made from labour and land, so there are no units to
    // consume. What a batch had to draw is its recipe's business and the goods module audits it;
    // requiring a destroy here would have made a harvest impossible (Goods B2).
    const w = world(
      goodsModule(wheat, (ctx) => {
        openWheat(ctx);
        if (ctx.period !== 1) return;
        ctx.settle({
          legs: [
            { kind: 'create', party: FIRM_1, instrument: WHEAT_ID, qty: 3, costPerUnit: 1, toCell: none() },
          ],
          cause: 'production',
          reason: 'the harvest',
        });
      }),
    );
    const r = w.step();
    expect(r.audit.total).toBe(0);
    expect(w.register.quantity(FIRM_1, WHEAT_ID)).toBe(3);
  });

  it('refuses to make a claim: a claim is issued and redeemed, never made (Goods A1)', () => {
    const w = world(
      goodsModule(wheat, (ctx) => {
        if (ctx.period !== 1) return;
        const line = ctx.instruments.all().find((i) => i.issuer.some);
        if (line === undefined) throw new Error('no claim in the world');
        ctx.settle({
          legs: [
            { kind: 'create', party: FIRM_1, instrument: line.id, qty: 1, costPerUnit: 1, toCell: none() },
          ],
          cause: 'seed',
          reason: 'a bond from nowhere',
        });
      }),
    );
    expect(() => w.step()).toThrow();
  });
});

describe('a write-down that only goes one way (Goods E2.c)', () => {
  /**
   * Wheat clears in a market and is carried at what it cost: two different questions about one
   * thing (Goods C1, E1). The seller posts what it has at a level, a buyer takes it, and the print
   * is what the holder's inventory is written DOWN to if it is worth less than it cost.
   */
  const build = (profile: InstrumentKindProfile, offered: number): World =>
    world(
      goodsModule(profile, (ctx) => {
        if (ctx.period !== 1) return;
        ctx.issue({
          id: WHEAT_ID,
          kind: WHEAT,
          issuer: none(),
          ccy: USD,
          terms: { kind: WHEAT },
          market: some(WHEAT_MARKET),
        });
        ctx.openMarket({
          id: WHEAT_MARKET,
          name: 'wheat, North',
          instrument: WHEAT_ID,
          ccy: USD,
          rationing: 'proRata',
        });
        ctx.settle({
          legs: [
            { kind: 'create', party: FIRM_1, instrument: WHEAT_ID, qty: 10, costPerUnit: 2, toCell: none() },
          ],
          cause: 'seed',
          reason: 'the opening harvest',
        });
      }),
      trader(offered),
    );

  it('carries inventory at cost while it clears in a market, and writes it down to the print', () => {
    // The market prints 1 against a cost of 2: the holder's inventory is worth less than it cost.
    const w = build(wheat, 1);
    w.step();
    const before = w.register.equity(FIRM_1);
    const r = w.step();
    expect(r.audit.total).toBe(0);
    const print = w.prices.latest(WHEAT_ID, w.period);
    expect(print.some && print.value.price).toBe(1);
    // E2: down to the print on what it still holds, and nothing more. What its bank paid it for
    // the week's deposit is its bank's business and not the write-down's, so it comes out.
    const held = w.register.quantity(FIRM_1, WHEAT_ID);
    expect(w.register.equity(FIRM_1) - paidTo(w, FIRM_1, 'coupon')).toBeLessThan(before);
    expect(held).toBeLessThan(10);
  });

  it('refuses to carry it above cost for a holder that is not a dealer (E2.c)', () => {
    const up = build(wheatMarkedUp, 5);
    up.step();
    expect(() => up.step()).toThrow();
  });
});

/**
 * Money Market B3.c, Register D5: collateral is bound and freed by the wire, and what is bound is
 * neither sellable nor pledgeable a second time.
 */
const GOV = instrumentId('ust.2036-03-15');

function pledgeModule(run: (ctx: MechanismContext) => void): SystemModule {
  return {
    id: 'test.pledge',
    spec: 'Register D5',
    requires: ['seed.foundation'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      { name: 'test.pledge', spec: 'Register D5', cycle: 0, anchor: { after: 'corporateActions' }, run },
    ],
    participants: [],
    families: [],
  };
}

/** What a bank pledging `qty` of its own sovereign paper to the other bank posts on the wire. */
function pledgeLeg(qty: number, secures: string): Leg {
  return {
    kind: 'pledge',
    pledgor: BANK_A,
    beneficiary: BANK_B,
    instrument: GOV,
    qty,
    secures,
    pledgorCell: none(),
  };
}

describe('collateral is bound and freed by the wire (Register D5, Money Market B3.c)', () => {
  it('binds units to a named beneficiary without moving them or anything else', () => {
    const w = world(
      pledgeModule((ctx) => {
        if (ctx.period !== 1) return;
        ctx.settle({
          legs: [pledgeLeg(100, 'repo.test.1')],
          cause: 'transfer',
          reason: 'bank.a pledges paper to bank.b',
        });
      }),
    );
    const held = w.register.quantity(BANK_A, GOV);
    const equity = w.register.equity(BANK_A);
    w.step();
    // The units are still held, still on the same book, and worth what they were: an encumbrance
    // moves nothing (Register D5). What changed is that a hundred of them are no longer free.
    expect(w.register.quantity(BANK_A, GOV)).toBeCloseTo(held, 9);
    expect(w.register.equity(BANK_A)).toBeCloseTo(equity, 9);
    expect(w.register.encumbered(BANK_A, GOV)).toBeCloseTo(100, 9);
    expect(w.register.free(BANK_A, GOV)).toBeCloseTo(held - 100, 9);
    const lien = w.register.holding(BANK_A, GOV);
    expect(lien.some && lien.value.liens[0]?.beneficiary).toBe(BANK_B);
    // D5.b: the chain says what it stands behind, in the words of whoever bound it.
    expect(lien.some && lien.value.liens[0]?.reason).toBe('repo.test.1');
  });

  it('refuses to bind what is not free, and the instruction it was part of does not settle', () => {
    let outcome = '';
    let paid = 0;
    const w = world(
      pledgeModule((ctx) => {
        if (ctx.period !== 1) return;
        const free = ctx.register.free(BANK_A, GOV);
        const r = ctx.settle({
          legs: [
            pledgeLeg(free + 1, 'repo.test.2'),
            {
              kind: 'money',
              from: { holder: BANK_B, issuer: BANK_B },
              to: { holder: BANK_A, issuer: BANK_B },
              ccy: USD,
              amount: 10,
              fromCell: none(),
              toCell: none(),
            },
          ],
          cause: 'issuance',
          reason: 'bank.b lends against paper bank.a does not have free',
        });
        outcome = r.outcome === 'failed' ? r.reason.kind : 'settled';
        paid = ctx.register.quantity(BANK_A, moneyInstrumentId(BANK_B, USD));
      }),
    );
    w.step();
    // C4.b: running out of unencumbered paper is how a solvent bank stops being able to borrow —
    // an outcome of the borrowing, not a violation, so it fails and the money never moves.
    expect(outcome).toBe('insufficientCollateral');
    expect(paid).toBe(0);
    expect(w.register.encumbered(BANK_A, GOV)).toBe(0);
  });

  it('will not let the same units stand behind two rows', () => {
    const outcomes: string[] = [];
    const w = world(
      pledgeModule((ctx) => {
        if (ctx.period !== 1) return;
        const free = ctx.register.free(BANK_A, GOV);
        for (const n of [1, 2]) {
          const r = ctx.settle({
            legs: [pledgeLeg(free, `repo.test.twice.${n}`)],
            cause: 'transfer',
            reason: `bank.a pledges everything it has free, attempt ${n}`,
          });
          outcomes.push(r.outcome === 'failed' ? r.reason.kind : 'settled');
        }
      }),
    );
    w.step();
    expect(outcomes).toEqual(['settled', 'insufficientCollateral']);
  });

  it('frees the named lien again, and the paper is sellable once it is', () => {
    let freeAfter = 0;
    const w = world(
      pledgeModule((ctx) => {
        if (ctx.period === 1) {
          ctx.settle({
            legs: [pledgeLeg(100, 'repo.test.3')],
            cause: 'transfer',
            reason: 'bank.a pledges paper to bank.b',
          });
          return;
        }
        if (ctx.period !== 2) return;
        const holding = ctx.register.holding(BANK_A, GOV);
        const lien = holding.some
          ? holding.value.liens.find((l) => l.reason === 'repo.test.3')
          : undefined;
        if (lien === undefined) return;
        ctx.settle({
          legs: [
            {
              kind: 'release',
              pledgor: BANK_A,
              beneficiary: BANK_B,
              instrument: GOV,
              lien: lien.id,
            },
          ],
          cause: 'transfer',
          reason: 'the row is repaid and the paper is bank.a own again',
        });
        freeAfter = ctx.register.free(BANK_A, GOV);
      }),
    );
    w.step();
    const held = w.register.quantity(BANK_A, GOV);
    expect(w.register.free(BANK_A, GOV)).toBeCloseTo(held - 100, 9);
    w.step();
    expect(w.register.encumbered(BANK_A, GOV)).toBe(0);
    expect(freeAfter).toBeCloseTo(w.register.quantity(BANK_A, GOV), 9);
  });

  it('refuses a party that would pledge to itself (Register D5)', () => {
    const w = world(
      pledgeModule((ctx) => {
        if (ctx.period !== 1) return;
        ctx.settle({
          legs: [{ ...pledgeLeg(1, 'repo.test.self'), beneficiary: BANK_A } as Leg],
          cause: 'transfer',
          reason: 'bank.a pledges to itself',
        });
      }),
    );
    expect(() => w.step()).toThrow(/pledge to itself/);
  });
});

describe('the delivery check is exact, because a quantity is a count of pieces (Law 8, Register C4)', () => {
  it('refuses one piece more than is free, and lets exactly what is free go', () => {
    const seen: { over: string; exact: string; left: number; lots: number } = {
      over: '',
      exact: '',
      left: -1,
      lots: -1,
    };
    const w = world(
      pledgeModule((ctx) => {
        if (ctx.period !== 1) return;
        const free = ctx.register.free(BANK_A, GOV);
        const move = (qty: number): string => {
          const r = ctx.settle({
            legs: [
              {
                kind: 'asset',
                from: BANK_A,
                to: BANK_B,
                instrument: GOV,
                qty,
                pricePerUnit: none(),
                accruedPerUnit: none(),
                fromCell: none(),
                toCell: none(),
              },
            ],
            cause: 'transfer',
            reason: `bank.a delivers ${qty} of the gov line`,
          });
          return r.outcome === 'failed' ? r.reason.kind : 'settled';
        };
        /**
         * The register used to allow a delivery over `free` by the dust of the walk —
         * `dustOf(lots + 2, |qty| + |free|)`, about 3e-16 of the magnitude. Every quantity that
         * reaches a lot passes `onTheGrid`, so the smallest excess there can be is ONE WHOLE
         * PIECE, and a piece more than somebody holds is a short position nobody borrowed
         * (Register C4). The band was unreachable here and destroyed units where it was reachable.
         */
        seen.over = move(free + 1);
        seen.exact = move(free);
        seen.left = ctx.register.quantity(BANK_A, GOV);
        const h = ctx.register.holding(BANK_A, GOV);
        seen.lots = h.some ? h.value.lots.length : 0;
      }),
    );
    w.step();
    expect(seen.over).toBe('insufficientUnits');
    expect(seen.exact).toBe('settled');
    // Appendix B, Law 5: a full delivery leaves NOTHING, and it leaves it by having moved every
    // piece to a named holder — never by a branch that cleared the lot book because what was left
    // summed below a tolerance, which is units deleted with no instruction behind them.
    expect(seen.left).toBe(0);
    expect(seen.lots).toBe(0);
  });
});
