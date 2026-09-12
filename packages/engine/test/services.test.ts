/**
 * The rest of the economy (13c.2): the things that cannot be put in a box, the distance between the
 * gate and the shelf, and a basket whose preference is a quantity.
 *
 * @spec Goods A1 Goods A2.a Goods C1 Goods C3 Households A2.a Households A2.b Households C3 Households C4 Commodities Spot A3 Freight A3 Law 2 Law 6
 */
import { describe, expect, it } from 'vitest';
import { FIRM, downTick, paramId, regionId, rungsUpTo, sum } from '../src/index.js';
import { rigWorld } from './rig.js';
import { CONSUMPTION } from '../src/mechanisms/households/data.js';
import { MERCHANT_SPREAD, drawMerchants, merchants } from '../src/mechanisms/merchants/index.js';

import { GOODS, RETAIL, retailOf } from '../src/mechanisms/goods/data.js';
import { OCCUPATION_OF } from '../src/mechanisms/firms/data.js';
import { OCCUPATIONS } from '../src/mechanisms/labour/data.js';

describe('a thing that cannot be put in a box (Freight A3, Commodities Spot A3)', () => {
  it('says of every line whether a unit of it can be somewhere other than where it was made', () => {
    for (const g of GOODS) expect(typeof g.portable).toBe('boolean');
  });

  it('never declares a store for something that cannot be moved', () => {
    // A store is somewhere a thing waits that is not where it will be used, and getting it there is
    // the move the line has just said is impossible. The seed throws on the pair; this is the same
    // statement made where a reader can see it.
    for (const g of GOODS) {
      if (!g.portable) expect(g.storagePerUnit).toBeNull();
    }
  });
});

describe('a preference is a QUANTITY, not a share of spending (Goods A2.a, Households C3)', () => {
  it('declares what a member takes in its own physical unit and never a share', () => {
    const units = new Map(GOODS.map((g) => [g.subUnit, g.unit]));
    for (const row of CONSUMPTION) {
      // 13c.2: the row that used to say `share: 1` said nothing about bread and everything about
      // money. These two say how much bread, which is a fact about people (Law 2).
      expect(row).not.toHaveProperty('share');
      expect(units.has(row.subUnit)).toBe(true);
      expect(row.neededPerMember).toBeGreaterThanOrEqual(0);
      expect(row.wantedPerMember).toBeGreaterThanOrEqual(0);
      expect(row.neededPerMember + row.wantedPerMember).toBeGreaterThan(0);
    }
  });

  it('lets two cohorts take different amounts of the same thing, which is what A2.a needs', () => {
    const byGood = new Map<string, number[]>();
    for (const row of CONSUMPTION) {
      const list = byGood.get(row.subUnit) ?? [];
      list.push(row.neededPerMember + row.wantedPerMember);
      byGood.set(row.subUnit, list);
    }
    const shared = [...byGood.values()].filter((l) => l.length > 1);
    expect(shared.length).toBeGreaterThan(0);
    // A2.a: the same income in different hands is different demand, and it can only be that if the
    // hands want different things. Two cohorts wanting identical baskets would be one cohort.
    expect(shared.some((l) => new Set(l).size > 1)).toBe(true);
  });
});

describe('the curve under a want (Goods C1, Clearing A2, Law 6)', () => {
  const LEVELS = [10, 8, 6, 4, 2];

  it('is the money divided by the price while the money binds', () => {
    // A want it cannot reach at any of these levels: the curve is the budget curve exactly.
    const rungs = rungsUpTo(LEVELS, 100, 1000);
    expect(sum(rungs.map((r) => r.qty)).value).toBe(downTick(100 / 2));
    expect(rungs[0]?.qty).toBe(downTick(100 / 10));
  });

  it('is flat at what it wanted once the price has fallen far enough', () => {
    const rungs = rungsUpTo(LEVELS, 100, 12);
    // It never takes more than it wanted, however cheap the thing got: a household does not buy
    // grain by the lorry-load because it is cheap, and nothing caps it — the want is a quantity it
    // named itself and the arithmetic takes whichever ran out first.
    expect(sum(rungs.map((r) => r.qty)).value).toBe(downTick(12));
    let running = 0;
    for (const r of rungs) {
      running += r.qty;
      expect(running).toBeLessThanOrEqual(downTick(12));
      expect(r.qty).toBeGreaterThan(0);
    }
  });

  it('refines without moving: every level of a coarse grid says the same on a fine one (Law 2)', () => {
    const coarse = rungsUpTo([10, 6, 2], 100, 12);
    const fine = rungsUpTo([10, 8, 6, 4, 2], 100, 12);
    for (const level of [10, 6, 2]) {
      const upToCoarse = sum(coarse.filter((r) => r.price >= level).map((r) => r.qty)).value;
      const upToFine = sum(fine.filter((r) => r.price >= level).map((r) => r.qty)).value;
      expect(upToFine).toBe(upToCoarse);
    }
  });

  it('asks for nothing when it has no money or wants none of it', () => {
    expect(rungsUpTo(LEVELS, 0, 10)).toEqual([]);
    expect(rungsUpTo(LEVELS, 100, 0)).toEqual([]);
    expect(rungsUpTo([0, -1], 100, 10)).toEqual([]);
  });
});

describe('the sixteen lines that cannot be put in a box (13c.2)', () => {
  const services = GOODS.filter((g) => !g.portable);

  it('has a service economy at all, and it is most of the lines a person works in', () => {
    expect(services.length).toBeGreaterThanOrEqual(16);
  });

  it('keeps nothing, waits for nothing and stands on no ground of its own', () => {
    for (const g of services) {
      // An hour nobody bought is not an hour waiting: it is gone, and the wages were paid anyway.
      expect(g.spoilagePerPeriod).toBe(1);
      // Made to order, which is what having no stock to make it from means.
      expect(g.leadTimePeriods).toBe(0);
      expect(g.storagePerUnit).toBeNull();
      // The ground is in the premises it is made in; counting it again would be counting it twice.
      expect(g.standsOn).toBeNull();
    }
  });

  it('is made of labour first, and of real things after (A2.a, A2.c)', () => {
    for (const g of services) {
      expect(g.labourHoursPerUnit).toBeGreaterThan(0);
      expect(g.plant.length).toBeGreaterThan(0);
      // Every input is a physical good this world makes, in its own physical unit.
      const known = new Set(GOODS.map((x) => x.subUnit));
      for (const i of g.inputs) expect(known.has(i.subUnit)).toBe(true);
    }
  });

  it('never consumes a good that consumes a service, so the chain still has a bottom', () => {
    // `openingLevels` walks the recipes from the bottom up and a cycle has no bottom. A world where
    // a machine works buys an engineer's week and the engineer's week buys a machine would have no
    // cost to work out, and the seed would be right to refuse it.
    const byName = new Map(GOODS.map((g) => [g.subUnit, g]));
    const serviceNames = new Set(services.map((g) => g.subUnit));
    const buysAService = new Set(
      GOODS.filter((g) => g.inputs.some((i) => serviceNames.has(i.subUnit))).map((g) => g.subUnit),
    );
    for (const s of services) {
      const seen = new Set<string>();
      const walk = (name: string): void => {
        if (seen.has(name)) return;
        seen.add(name);
        for (const i of byName.get(name)?.inputs ?? []) walk(i.subUnit);
      };
      for (const i of s.inputs) walk(i.subUnit);
      for (const reached of seen) expect(buysAService.has(reached)).toBe(false);
    }
  });

  it('is bought by firms as well as by people: the made goods name the services they use', () => {
    const serviceNames = new Set(services.map((g) => g.subUnit));
    const buyers = GOODS.filter((g) => g.inputs.some((i) => serviceNames.has(i.subUnit)));
    // A world where services were only ever final demand would have half of what services are.
    expect(buyers.length).toBeGreaterThan(5);
  });

  it('employs its own trade, and no two lines share one (Labour A3.a)', () => {
    const trades = services.map((g) => OCCUPATION_OF[g.subUnit]);
    for (const t of trades) expect(OCCUPATIONS.some((o) => o.id === t)).toBe(true);
    // A nurse out of work is not a bricklayer's vacancy filled, and a world where every service was
    // one trade would answer a shortage of clinicians by sending it a security guard.
    expect(new Set(trades).size).toBe(trades.length);
  });
});

describe('the shelf is a place (13c.2, Commodities Spot C6, Indices D4)', () => {
  const byName = new Map(GOODS.map((g) => [g.subUnit, g]));

  it('turns one unit of the wholesale line into one unit on the shelf and nothing else', () => {
    for (const d of RETAIL) {
      const line = byName.get(retailOf(d.of));
      expect(line).toBeDefined();
      if (line === undefined) continue;
      const one = line.inputs.find((i) => i.subUnit === d.of);
      // A shop transforms nothing: what it adds is being somewhere, in small quantities, at an hour
      // a person can get there — and all of that is in its hours and its premises.
      expect(one?.qtyPerUnit).toBe(1);
      expect(line.unit).toBe(byName.get(d.of)?.unit);
    }
  });

  it('pays for staff, a shop, a delivery round and a bin at the back', () => {
    for (const d of RETAIL) {
      const line = byName.get(retailOf(d.of));
      if (line === undefined) continue;
      expect(line.labourHoursPerUnit).toBeGreaterThan(0);
      expect(line.plant.some((q) => q.capitalKind === 'premises')).toBe(true);
      expect(line.inputs.some((i) => i.subUnit === 'transport')).toBe(true);
      expect(line.inputs.some((i) => i.subUnit === 'logistics')).toBe(true);
      expect(line.spoilagePerPeriod).toBeGreaterThan(0);
      // The shelf IS the store and the shelf is the premises; renting the room again would be
      // charging twice for one thing (Law 4).
      expect(line.storagePerUnit).toBeNull();
    }
  });

  it('declares no mark-up anywhere, because a margin is two prints (Law 3)', () => {
    // What a shop DECLARES is what it does — hours, a shop, a round, a bin — and never what it
    // earns. A declared mark-up would be a price from a formula (Law 3) and a spread table
    // (Appendix B); the margin is the gap between two prints less those costs, or it is nothing.
    const fields = new Set(RETAIL.flatMap((d) => Object.keys(d)));
    for (const f of fields) {
      expect(f.toLowerCase()).not.toContain('markup');
      expect(f.toLowerCase()).not.toContain('margin');
      expect(f.toLowerCase()).not.toContain('price');
    }
  });

  it('is what a household buys: never the line at the gate (Goods G1.a, G1.b)', () => {
    const retailed = new Set(RETAIL.map((d) => d.of));
    for (const row of CONSUMPTION) {
      // A household never buys bread from a bakery at the bakery's own price. Where this world has
      // a shop for a thing, the shop is where the household is.
      expect(retailed.has(row.subUnit)).toBe(false);
    }
    for (const d of RETAIL) {
      expect(CONSUMPTION.some((c) => c.subUnit === retailOf(d.of))).toBe(true);
    }
  });

  it('gives a household a basket rather than a loaf', () => {
    const lines = new Set(CONSUMPTION.map((c) => c.subUnit));
    expect(lines.size).toBeGreaterThanOrEqual(18);
    // And it buys services, which is most of what people spend money on.
    const services = new Set(GOODS.filter((g) => !g.portable).map((g) => g.subUnit));
    expect([...lines].filter((l) => services.has(l)).length).toBeGreaterThanOrEqual(6);
  });
});

describe('a firm whose business is the gap (13c.2, Freight D3)', () => {
  const who = [
    { firm: 'firm.1', region: regionId('us.1') },
    { firm: 'firm.2', region: regionId('us.1') },
  ];

  it('draws two preferences per merchant and declares nothing else', () => {
    const rows = drawMerchants(who, 'merchants-a');
    expect(rows.length).toBe(2);
    for (const r of rows) {
      expect(r.margin).toBeGreaterThanOrEqual(MERCHANT_SPREAD.margin.low);
      expect(r.margin).toBeLessThanOrEqual(MERCHANT_SPREAD.margin.high);
      expect(r.appetite).toBeGreaterThanOrEqual(MERCHANT_SPREAD.appetite.low);
      expect(r.appetite).toBeLessThanOrEqual(MERCHANT_SPREAD.appetite.high);
    }
    // Firm A3: two merchants facing one gap do not take the same position, which is why the basis is
    // closed by somebody in particular rather than by arithmetic.
    expect(rows[0]?.margin).not.toBe(rows[1]?.margin);
  });

  it('adds no party, no instrument, no market and no phase: the firms are already here', () => {
    const m = merchants(drawMerchants(who, 'merchants-a'));
    expect(m.phases).toEqual([]);
    expect(m.instrumentKinds).toEqual([]);
    expect(m.partyKinds).toEqual([]);
    expect(typeof m.seed).toBe('undefined');
    // What it adds is a reason to buy something it will not use, and that is a participant.
    expect(m.participants.length).toBe(1);
    expect(m.participants[0]?.speculative).toBe(true);
  });

  it('declares preferences and never a rate, a fee or a mark-up (Law 3)', () => {
    const m = merchants(drawMerchants(who, 'merchants-a'));
    expect(m.params.length).toBe(4);
    for (const d of m.params) {
      expect(d.kind).toBe('preference');
      for (const word of ['rate', 'fee', 'markup', 'spread', 'price']) {
        expect(String(d.id).toLowerCase()).not.toContain(word);
      }
    }
  });

  it('is a FIRM, not a kind of its own (Law 15, the carrier precedent)', () => {
    const m = merchants(drawMerchants(who, 'merchants-a'));
    expect(m.participants[0]?.partyKind).toBe(FIRM);
  });
});

describe('the commodity future (13c steps 11-13, Commodity Futures A1-A4, C1-C4)', () => {
  it('measures a lot off the good’s own storage and declares no contract size (A1.a, Law 19)', () => {
    const w = rigWorld('futures-a');
    for (const d of w.params.all()) {
      const id = String(d.id);
      if (!id.startsWith('commodity.future.')) continue;
      // A world that changes how much room a tonne takes changes the lot, which is what a lot is.
      expect(id).not.toContain('contractSize');
      expect(id).not.toContain('lot');
    }
    // What IS declared is a convention of the exchange: how many dates and how far apart.
    for (const [id, kind] of [
      ['commodity.future.series', 'technology'],
      ['commodity.future.spacing.periods', 'technology'],
      ['commodity.future.margin.window', 'resolution'],
    ] as const) {
      expect(w.params.decl(paramId(id)).kind).toBe(kind);
    }
  });

  it('declares no convenience yield, no basis target and no curve shape (Law 3)', () => {
    const w = rigWorld('futures-a');
    for (const d of w.params.all()) {
      const id = String(d.id).toLowerCase();
      expect(id).not.toContain('convenience');
      if (id.startsWith('commodity.future')) {
        expect(id).not.toContain('basis');
        expect(id).not.toContain('contango');
        expect(id).not.toContain('curve');
      }
    }
  });

  it('lists only grades that can actually be handed over', () => {
    const w = rigWorld('futures-a');
    const deliverable = new Set(
      GOODS.filter((g) => g.portable && g.storagePerUnit !== null).map((g) => g.subUnit),
    );
    for (const m of w.markets) {
      const id = String(m.id);
      if (!id.includes('commodity.future')) continue;
      // A contract to deliver a thing nobody can hold to the date is not a contract anybody can be
      // short of, and a thing that cannot be moved cannot be handed over where it was not made.
      expect([...deliverable].some((g) => id.includes(`good.${g}.`))).toBe(true);
    }
  });
});
