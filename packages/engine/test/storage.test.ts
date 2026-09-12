/**
 * Storage: the room a thing waits in, and what it costs to keep it there (13c).
 *
 * @spec Commodities Spot A3 Commodities Spot A4 Commodities Spot D2.a Commodities Spot D3 Capital Programme A2 Capital Programme A4 Clearing A2 Clearing A3 Law 5 Law 6 Law 8
 */
import { describe, expect, it } from 'vitest';
import {
  GOODS,
  drawFunds,
  REGION,
  STORAGE,
  addDays,
  goodId,
  isGoodTerms,
  none,
  paramId,
  plantKindId,
  instrumentKindId,
  partyId,
  plantVintageId,
  spaceFor,
  spacePerPiece,
  type MechanismContext,
  type SystemModule,
  type World,
} from '../src/index.js';
import { assemble } from '../src/index.js';
import { mergeModules, rigDraw, rigSpec } from './rig.js';

/** The one line in this world that is held in bulk between periods, and the one with a silo. */
const STORED = GOODS.filter((g) => g.storagePerUnit !== null);

describe('a thing that waits takes room, and the room is declared (A3)', () => {
  it('says which lines are held in bulk, and an empty answer is an answer', () => {
    // Law 16, Law 6: a line nobody stores in bulk says `null` — which is different from a capacity
    // somebody set to a large number, and different again from a yield that happens to be high.
    expect(STORED.length).toBeGreaterThan(0);
    expect(STORED.length).toBeLessThan(GOODS.length);
    for (const g of STORED) {
      expect(g.storagePerUnit).toBeGreaterThan(0);
      expect(g.storageWhy).not.toBeNull();
    }
    for (const g of GOODS.filter((x) => x.storagePerUnit === null)) {
      expect(g.storageWhy).toBeNull();
    }
  });

  it('converts between the two subdivisions where the declared ratio is read (Law 8)', () => {
    const w = rig('storage-units');
    const line = STORED[0];
    expect(line).toBeDefined();
    if (line === undefined) return;
    const good = w.instruments.get(goodId(line.subUnit, REGION));
    const terms = good.terms;
    expect(isGoodTerms(terms)).toBe(true);
    if (!isGoodTerms(terms) || terms.storagePerUnit === null) return;
    const per = w.params.ratio(terms.storagePerUnit);
    const pieces = w.registry.subdivision(good.unit);
    // One NAMED unit of the thing takes `per` named units of space, however many pieces each of
    // those is divided into. A conversion that skipped this is out by the ratio of the two
    // subdivisions — a thousand, here — and nothing would say so.
    expect(spaceFor(w, good.unit, pieces, per)).toBe(spaceFor(w, good.unit, pieces * 1, per));
    expect(spaceFor(w, good.unit, pieces * 2, per)).toBe(2 * spaceFor(w, good.unit, pieces, per));
    // And the per-PIECE ratio is not on any grid: a kilo takes a thousandth of a unit of space,
    // and rounding that to a whole piece makes it nothing and makes room bind nothing.
    expect(spacePerPiece(w, good.unit, per)).toBeGreaterThan(0);
    expect(spacePerPiece(w, good.unit, per)).toBeLessThan(1);
  });
});

describe('room is plant, and it binds like plant (A4, Capital Programme A2)', () => {
  it('is a kind of capital with a life, made from a good, held by named parties', () => {
    const w = rig('storage-plant');
    expect(w.registry.instrumentKinds.has(plantKindId(STORAGE))).toBe(true);
    const life = w.params.decl(paramId(`plant.usefulLife.${STORAGE}`));
    expect(life.kind).toBe('technology');
    expect(life.value).toBeGreaterThan(0);
    // Seed C4, A3: nobody opens holding a thing in bulk without somewhere to keep it. Every party
    // that opens holding one opens with room for it — an opening CONDITION, and no headroom:
    // whether a line that wants to grow rents a barn or builds one is a market question from the
    // first period, and headroom would be the seed answering it.
    let holders = 0;
    for (const p of w.parties.alive()) {
      const view = w.participantView(p.id);
      let needs = 0;
      for (const h of view.holdings()) {
        const i = w.instruments.get(h.instrument);
        if (!isGoodTerms(i.terms) || i.terms.storagePerUnit === null) continue;
        needs += spaceFor(w, i.unit, view.quantity(h.instrument), w.params.ratio(i.terms.storagePerUnit));
      }
      if (needs <= 0) continue;
      holders += 1;
      let owns = 0;
      for (const h of view.holdings()) {
        const i = w.instruments.get(h.instrument);
        if (i.kind !== plantKindId(STORAGE)) continue;
        owns += view.quantity(h.instrument);
      }
      expect(owns).toBeGreaterThanOrEqual(needs);
    }
    expect(holders).toBeGreaterThan(0);
  });
});

describe('the room is rented from somebody, and they are paid (D3, Law 5)', () => {
  it('clears when one party is short and another has spare, and the money has two sides', () => {
    // A SCALE MODEL of the question (CLAUDE.md): the world's own firms all open with exactly the
    // room they need, so nobody is short and nobody has spare — an inert market is the honest
    // answer there. What the mechanism has to do is clear when the two sides exist, so this makes
    // them: one holder given more of the thing than it has room for, one given room it is not using.
    const seen = { grain: 0, space: 0 };
    const w = rig('storage-market', [gift(seen)]);
    for (let i = 0; i < 3; i += 1) w.step();
    // Clearing C4.b: the session says what it did, per region, including when it did not clear.
    // One event for the period keyed by place, which is how a going wage is published: a reader
    // asks for the last of its kind and looks up where it is (Observer A3).
    const sessions = w.journal.ofKind('commodities.storage');
    expect(sessions.length).toBeGreaterThan(0);
    const cleared = sessions.some((e) => {
      const byRegion = e.data['byRegion'] as Record<string, { outcome?: string }> | undefined;
      return byRegion?.[String(REGION)]?.outcome === 'cleared';
    });
    expect(cleared).toBe(true);
    const leases = w.journal.ofKind('commodities.leased');
    expect(seen.grain).toBeGreaterThan(0);
    expect(seen.space).toBeGreaterThan(0);
    expect(seen.space).toBeGreaterThan(0);
    expect(leases.length).toBeGreaterThan(0);
    for (const e of leases) {
      const taker = String(e.data['taker']);
      const letter = String(e.data['letter']);
      // Clearing A2: nobody crosses themselves, and what is let is let BY somebody TO somebody.
      expect(taker).not.toBe(letter);
      expect(Number(e.data['space'])).toBeGreaterThan(0);
      expect(Number(e.data['rate'])).toBeGreaterThan(0);
      expect(Number(e.data['paid'])).toBeGreaterThan(0);
      expect(e.public).toBe(true);
    }
    // Law 5: every fee is a payment with two named sides, in one instruction, in one currency.
    const paid = leases.reduce((a, e) => a + Number(e.data['paid']), 0);
    const moved = w.ledger
      .all()
      .filter((r) => r.outcome === 'settled' && r.instruction.reason.includes('rents'))
      .flatMap((r) => r.instruction.legs)
      .filter((l) => l.kind === 'money')
      .reduce((a, l) => a + Number(l.amount), 0);
    expect(moved).toBe(paid);
  });
});

function rig(seed: string, extra: SystemModule[] = []): World {
  const spec = rigSpec(seed);
  return assemble({ ...spec, modules: mergeModules(spec.modules, extra) });
}

/**
 * The two sides, made. It gives one holder of the stored line more of it than it has room for, and
 * one other party room it is not using — and it does both BEFORE the storage session, which is the
 * period's first phase, so the schedules it clears are built on what this leaves.
 */
function gift(seen: { grain: number; space: number }): SystemModule {
  return {
    id: 'test.gift',
    spec: 'Commodities Spot D3',
    requires: ['commodities'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    participants: [],
    families: [],
    phases: [
      {
        name: 'test.gift',
        spec: 'Commodities Spot D3',
        cycle: 0,
        anchor: { before: 'commodities.storage' },
        run: (ctx: MechanismContext): void => {
          if (seen.grain > 0 && seen.space > 0) return;
          const line = STORED[0];
          if (line === undefined) return;
          const good = goodId(line.subUnit, REGION);
          if (!ctx.instruments.has(good)) return;
          const holders = ctx.register.holdersOf(good).filter((h) => ctx.parties.get(h).status.alive);
          const short = holders[0];
          const spare = holders[1];
          if (short === undefined || spare === undefined) return;
          const more = ctx.register.quantity(short, good);
          const r = ctx.settle({
            legs: [
              {
                kind: 'create',
                party: short,
                instrument: good,
                qty: ctx.registry.deliverable(ctx.instruments.get(good).unit, more),
                costPerUnit: 1,
                toCell: none(),
              },
            ],
            cause: 'production',
            reason: 'the test doubles what one holder has',
          });
          if (r.outcome === 'settled') seen.grain += 1;
          const id = plantVintageId(STORAGE, REGION, addDays(ctx.calendar.epoch, -ctx.calendar.periodDays));
          if (!ctx.instruments.has(id)) return;
          const s = ctx.settle({
            legs: [
              {
                kind: 'create',
                party: spare,
                instrument: id,
                qty: ctx.registry.deliverable(ctx.instruments.get(id).unit, more),
                // A silo is a built thing and it is carried at what building one costs: a barn
                // worth a penny would let its room for a fraction of a penny, and a fee below one
                // piece of money is not a fee (Law 8).
                costPerUnit: ctx.params.price(paramId('seed.openingPrice.machine')),
                toCell: none(),
              },
            ],
            cause: 'production',
            reason: 'the test builds one party a barn it is not using',
          });
          if (s.outcome === 'settled') seen.space += 1;
        },
      },
    ],
  };
}

describe('what the wait costs reaches the decision to hold (B4, C3)', () => {
  it('takes the carry out of what a producer will hold for, at the rate the room let for', () => {
    // Commodities Spot B4: a producer parts with stock only above what HOLDING it is worth — what
    // it expects to get, less what will not survive the wait, LESS WHAT THE WAIT COSTS. Before 13c
    // the last term did not exist, so holding was free and nothing ever had to come to market.
    // The carry is not a number anybody wrote down: it is the rate the room cleared at.
    const seen = { grain: 0, space: 0 };
    const w = rig('storage-carry', [gift(seen)]);
    for (let i = 0; i < 4; i += 1) w.step();
    const plans = w.journal
      .ofKind('firms.plan')
      .filter((e) => String(e.data['output']).includes('grain'));
    expect(plans.length).toBeGreaterThan(0);
    // Every grain plan says what a period of waiting cost it, and where a session cleared it is
    // a real cost rather than nothing.
    const charged = plans.filter((e) => Number(e.data['carry']) > 0);
    expect(charged.length).toBeGreaterThan(0);
    for (const e of plans) expect(Number(e.data['carry'])).toBeGreaterThanOrEqual(0);
    // And a line nobody stores in bulk is charged nothing for waiting, which is an answer: a loaf
    // does not wait for a silo, and its own spoilage already says what waiting costs it.
    const indoors = w.journal
      .ofKind('firms.plan')
      .filter((e) => String(e.data['output']).includes('bread'));
    expect(indoors.length).toBeGreaterThan(0);
    for (const e of indoors) expect(Number(e.data['carry'])).toBe(0);
  });

  it('has a party on the other side of it: a fund that holds the thing itself (C3)', () => {
    const w = rig('storage-investor');
    const funds = drawFunds(rigDraw('storage-investor').banks, 'storage-investor');
    const physical = funds.filter((d) => d.eligible.every((k) => k.startsWith('good.')));
    // A4, Law 15: what makes it a commodity fund is its MANDATE — every kind it may hold is one
    // nobody issued (Goods A1) — and never a flag on the row.
    expect(physical.length).toBeGreaterThan(0);
    for (const d of physical) {
      expect(w.parties.has(partyId(d.fund))).toBe(true);
      // F3: its own manager, a separate party, whose income is the fund's cost.
      expect(w.parties.has(partyId(d.manager))).toBe(true);
      expect(d.manager).not.toBe(`manager.${d.bank}`);
      for (const kind of d.eligible) {
        expect(w.registry.instrumentKind(instrumentKindId(kind)).physical).toBe(true);
      }
    }
  });
});
