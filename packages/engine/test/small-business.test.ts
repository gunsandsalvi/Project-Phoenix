/**
 * The small-business tier: firms with a weight, and a distribution rather than an average.
 *
 * @spec Small-Business Pools A1 Small-Business Pools A2 Small-Business Pools A2.a Small-Business Pools A3 Small-Business Pools A5 Small-Business Pools A6 Small-Business Pools A6.a Small-Business Pools A6.b Small-Business Pools E5 Seed B1.a XI-15 Law 2 Law 15
 *
 * ITEM 11, STEPS 1–4. §42 was 2 of 28 MET and both were the generic cell kernel: nothing in this
 * engine was a small firm. What is asserted here is that the SECTOR EXISTS and that it is a
 * distribution — A2.a is the clause the whole item turns on, and a draw that came out flat would
 * satisfy every other clause and remove the credit content of §42 without failing anything.
 */
import { describe, expect, it } from 'vitest';
import {
  BANK,
  CAPITAL_KINDS,
  SMALL_FIRM,
  about,
  capacityFrom,
  goodId,
  goodMarketId,
  goodTerms,
  isPlant,
  lineOf,
  plantTerms,
  rentedRoom,
  vintagesHeld,
  VEHICLE,
  isTranche,
  SMALL_PER_NAMED,
  assemble,
  drawSmallBusiness,
  saleable,
  type MechanismContext,
  type SystemModule,
} from '../src/index.js';
import { abroadWorld, mergeModules, rigDraw, rigFor, rigSpec, rigWorld } from './rig.js';

describe('the sector exists, and it is cells with weights (A1, A6, XI-15)', () => {
  it('opens with small firms in it, each a cell standing for a count of them', () => {
    const w = rigWorld('sb');
    const cells = w.parties.ofKind(SMALL_FIRM);
    expect(cells.length).toBeGreaterThan(0);
    for (const c of cells) {
      // XI-15, E5: a weight is a COUNT of real firms, never a share and never a fraction.
      expect(c.representation).toBe('cell');
      if (c.representation !== 'cell') continue;
      expect(Number.isInteger(c.weight)).toBe(true);
      expect(c.weight).toBeGreaterThan(0);
    }
  });

  it('keys them on what their members must all share, and NEVER on a lender (A6.a)', () => {
    const w = rigWorld('sb');
    for (const c of w.parties.ofKind(SMALL_FIRM)) {
      if (c.representation !== 'cell') continue;
      // A6.a: region and bank are dimensions of the key; the line is a third, and it is forced —
      // a cell whose members were in different lines would have an averaged cost base. 0f.3: the
      // rest of the key is the lattice's — its age and its bands — placed by the kernel at the seal.
      for (const dim of ['bank', 'line', 'region', 'age', 'size', 'leverage']) {
        expect(Object.keys(c.key), dim).toContain(dim);
      }
      // And the LENDER is not one of them: a lender is a loan row per (lender, cell), and lifting
      // it into the key is the relationship the model would then be unable to name.
      expect(Object.keys(c.key)).not.toContain('lender');
      // A cell lives where its bank books, and there is one statement of that (Law 4, Law 19).
      expect(c.key['region']).toBe(String(w.parties.get(c.bank).region));
    }
  });

  it('is a DISTRIBUTION and not an average (A2.a — the clause the item turns on)', () => {
    const drew = rigDraw('sb');
    const sizes = drew.small.map((r) => r.size);
    expect(sizes.length).toBeGreaterThan(1);
    // A2.a: no representative small firm. A mean-preserving spread has to be able to move the count
    // of defaults, which it cannot if every member is the same size.
    expect(Math.max(...sizes)).toBeGreaterThan(Math.min(...sizes) * 2);
    // A3: and the dispersion is WITHIN a key, not only across keys — otherwise every firm banking
    // at one bank in one line is the same firm, which is A2.a one level down.
    const one = drew.small[0];
    if (one === undefined) throw new Error('this world drew no small firms');
    const first = drew.small.filter((r) => r.bank === one.bank && r.line === one.line);
    expect(new Set(first.map((r) => r.size)).size).toBeGreaterThan(1);
  });

  it('scales with the world and states no count of its own (Seed B1.a)', () => {
    const drew = rigDraw('sb');
    const total = drew.small.length;
    // The sector is this world's named firms times a multiple, so a bigger world has more corner
    // shops. A count written in the module would have made it a fixed size the seed grew away from.
    expect(total).toBeGreaterThan(drew.firms.length);
    expect(SMALL_PER_NAMED).toBeGreaterThan(1);
  });

  it('makes its line out of its members\u2019 hours and what it holds, and sells what it made (A1, 11.0a)', () => {
    // A small firm in `power` burns coal it buys from a mine, and a rig of twelve firms has drawn
    // no mine as often as not: the test asks for a world that has one (Seed B1.a).
    const { world: w } = rigFor('sb-makes', { makes: ['coalRaw'] });
    // The cells as the seed cut them: a cell that merged onto another's key by the year's end
    // (0f.4) is gone from the parties store, and its legs are still in the ledger.
    const cells = new Set(w.parties.ofKind(SMALL_FIRM).map((p) => String(p.id)));
    for (let i = 0; i < 8; i += 1) w.step();
    const made = w.journal.ofKind('smallBusiness.produced').filter((e) => e.data['settled'] === true);
    expect(made.length).toBeGreaterThan(0);
    // A1: "small firms are firms" — what it made reached a buyer through a cleared book, as an
    // asset leg from a small-firm cell in a settled instruction with money coming back.
    let sold = 0;
    for (const r of w.ledger.all()) {
      if (r.outcome !== 'settled') continue;
      for (const leg of r.instruction.legs) {
        if (leg.kind === 'asset' && cells.has(String(leg.from))) sold += 1;
      }
    }
    expect(sold).toBeGreaterThan(0);
  });

  it('names its owner as a row and draws what it does not need to its owner (A6.a, 11.0d)', () => {
    const { world: w } = rigFor('sb-owner', { makes: ['coalRaw'] });
    const cells = new Set(w.parties.ofKind(SMALL_FIRM).map((p) => String(p.id)));
    // A6.a: the relationship is a ROW per cell, never an attribute inside it.
    const rows = w.agreements.all().filter((a) => String(a.terms.kind) === 'smallBusiness.ownership');
    expect(rows.length).toBe(cells.size);
    for (const r of rows) expect(String(w.parties.get(r.creditor).kind)).toBe('household');
    for (let i = 0; i < 8; i += 1) w.step();
    let drawn = 0;
    for (const r of w.ledger.all()) {
      if (r.outcome !== 'settled') continue;
      for (const leg of r.instruction.legs) {
        if (leg.kind === 'money' && leg.receipt?.of === 'dividend' && cells.has(String(leg.from.holder))) drawn += 1;
      }
    }
    // Households B3: income that is not a wage reached a household cell, from a named payer.
    expect(drawn).toBeGreaterThan(0);
  });

  it('asks its bank for what a period of trading needs beyond what it has (A5, Corporate Credit A1, 11.0e)', () => {
    const { world: w } = rigFor('sb-borrows', { makes: ['coalRaw'] });
    const cells = new Set(w.parties.ofKind(SMALL_FIRM).map((p) => String(p.id)));
    for (let i = 0; i < 4; i += 1) w.step();
    // A5: bank-dependent — the ask goes through the one door every borrower uses, under the
    // cell's own name, and a bank reads it next period. Whether one lends is the lender's.
    const asked = w.journal.ofKind('credit.request').filter((e) => e.subjects.some((s) => cells.has(s)));
    expect(asked.length).toBeGreaterThan(0);
    // A5, A6.a (11.2): bank-dependent means ITS bank — the one in its key — is the one that
    // quotes it, whatever another bank would have said.
    // A cell that moved its deposits (`deposit.moved`) is keyed on its new bank and was quoted by
    // its old one before it went; the cells that never moved are the clean read.
    const moved = new Set(w.journal.ofKind('deposit.moved').flatMap((e) => e.subjects.map(String)));
    const quoted = w.journal
      .ofKind('credit.quoted')
      .filter((e) => e.subjects.some((s) => cells.has(s) && !moved.has(s)));
    expect(quoted.length).toBeGreaterThan(0);
    for (const q of quoted) {
      const cell = w.parties.get(q.subjects[0] as never);
      expect(cell.representation === 'cell' && cell.key['bank']).toBe(q.data['bank']);
    }
  });

  it('fails together, by region, with no parameter that says so (B3, B4, B4.a, 11.3)', () => {
    // B4: the same demand, the same rates and the same region hit all of them — so the failures
    // in a region land in the same periods, and two regions at IDENTICAL draws (one seed, one
    // world with four countries in it) do not fail alike. B4.a: the pool's loss is not the sum of
    // independent draws. Nothing declares a correlation: the count comes out of each cell failing
    // on its own cash in the world it is in (Law 2, Law 15).
    const w = abroadWorld('sb-correlation');
    const region = new Map<string, string>();
    for (const p of w.parties.ofKind(SMALL_FIRM)) region.set(String(p.id), String(p.region));
    const PERIODS = 12;
    for (let i = 0; i < PERIODS; i += 1) w.step();
    expect(w.params.all().some((d) => /correl/i.test(String(d.id)))).toBe(false);
    // XI-3, XI-8: a cell that failed on its cash went to its estate; that is the failure event.
    const failed = w.journal.ofKind('estate.opened').filter((e) => e.subjects.some((s) => region.has(s)));
    expect(failed.length).toBeGreaterThan(0);
    const byRegion = new Map<string, { count: number; periods: Set<number> }>();
    for (const e of failed) {
      const cell = e.subjects.map(String).find((s) => region.has(s));
      if (cell === undefined) continue;
      const r = region.get(cell) ?? '';
      const row = byRegion.get(r) ?? { count: 0, periods: new Set<number>() };
      row.count += 1;
      row.periods.add(e.period);
      byRegion.set(r, row);
    }
    // Different regions, different counts, at identical draws.
    const counts = [...byRegion.values()].map((r) => r.count);
    expect(new Set(counts).size).toBeGreaterThan(1);
    // And within a region they cluster: fewer periods with a failure in them than failures.
    for (const r of byRegion.values()) if (r.count > 1) expect(r.periods.size).toBeLessThan(r.count);
  });

  it('its loan row is one a bank can put into a vehicle (C1, E1, XI-11, 11.4)', () => {
    // What a bank would sell is read off two facts every kind declares — no market, a named
    // obligor — and a cell's row has both. A probe asks each bank's own view, the way the
    // arranger does, and the rows it is shown name a cell among them.
    const seen = new Set<string>();
    const probe: SystemModule = {
      id: 'test.saleable',
      spec: 'Securitisation D1',
      requires: ['securitisation', 'small-business'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.saleable',
          spec: 'Securitisation D1',
          anchor: { after: 'lending.write' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            for (const bank of ctx.parties.ofKind(BANK)) {
              if (!bank.status.alive) continue;
              for (const row of saleable(ctx.participant(bank.id), ctx.registry.currencyOf(bank.region))) {
                if (row.issuer.some && ctx.parties.get(row.issuer.value).kind === SMALL_FIRM) seen.add(String(row.id));
              }
            }
          },
        },
      ],
      participants: [],
      families: [],
    };
    const { banks, firms } = rigFor('sb-borrows', { makes: ['coalRaw'] });
    const spec = rigSpec('sb-borrows', banks, firms);
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    for (let i = 0; i < 4; i += 1) w.step();
    expect(seen.size).toBeGreaterThan(0);
  });

  it('opens with the plant its line takes, and its start is limited by it (Capital Programme A2, 11.2a)', () => {
    const { world: w } = rigFor('sb-plant', { makes: ['coalRaw'] });
    let withPlant = 0;
    for (const cell of w.parties.ofKind(SMALL_FIRM)) {
      if (cell.representation !== 'cell') continue;
      const line = w.instruments.get(goodId(cell.key['line'] ?? '', cell.region));
      const needs = goodTerms(line).recipe.plant.filter((n) =>
        w.instruments.has(goodId(CAPITAL_KINDS.find((k) => k.id === n.capitalKind)?.madeFrom ?? '', cell.region)),
      );
      for (const need of needs) {
        // Seed D1: a whole machine of each kind its recipe names that this world makes, held by
        // the cell that runs on it — never a number on the cell.
        const held = w.register
          .holdingsOf(cell.id)
          .filter((h) => {
            const i = w.instruments.get(h.instrument);
            return isPlant(i) && plantTerms(i).capitalKind === need.capitalKind;
          });
        expect(held.length).toBeGreaterThan(0);
        withPlant += 1;
      }
    }
    expect(withPlant).toBeGreaterThan(0);
    // And it still makes its line out of it — the limit is a real stock, not a wall.
    for (let i = 0; i < 3; i += 1) w.step();
    expect(w.journal.ofKind('smallBusiness.produced').length).toBeGreaterThan(0);
  });

  it('bids for plant only when its plant is short of what it expects to sell (Capital Programme B1–B4, 11.2a.2)', () => {
    // One `project` in the registry, the named firm's own arithmetic. In the scale model no cell
    // bids, and the test says WHY rather than asserting the absence: every cell opened holding
    // the whole machine its members' batch reaches, so what its plant lets it run at is above what
    // it expects to sell, and a management with no gap builds nothing (B3). The day a cell's sales
    // outgrow its room, this assertion is the one that turns.
    const shortOfPlant: string[] = [];
    const probe: SystemModule = {
      id: 'test.plantGap',
      spec: 'Capital Programme B3',
      requires: ['small-business', 'capital-programme'],
      instrumentKinds: [],
      partyKinds: [],
      curveFamilies: [],
      units: [],
      params: [],
      phases: [
        {
          name: 'test.plantGap',
          spec: 'Capital Programme B3',
          anchor: { after: 'smallBusiness.decide' },
          reads: [],
          writes: [],
          run: (ctx: MechanismContext) => {
            for (const cell of ctx.parties.ofKind(SMALL_FIRM)) {
              if (!cell.status.alive) continue;
              const view = ctx.participant(cell.id);
              const line = lineOf(view);
              if (!line.some) continue;
              const sales = view.outlook(about({ on: 'sold', instrument: line.value.output }));
              const capacity = capacityFrom(line.value.plant, vintagesHeld(view, ctx.calendar.startOf(ctx.period)), rentedRoom(view));
              if (sales.some && capacity.some && capacity.value.perPeriod < sales.value.expected) shortOfPlant.push(String(cell.id));
            }
          },
        },
      ],
      participants: [],
      families: [],
    };
    const { banks, firms } = rigFor('sb-plantbid', { makes: ['coalRaw'] });
    const spec = rigSpec('sb-plantbid', banks, firms);
    const w = assemble({ ...spec, modules: mergeModules(spec.modules, [probe]) });
    const cells = new Set(w.parties.ofKind(SMALL_FIRM).map((p) => String(p.id)));
    for (let i = 0; i < 12; i += 1) w.step();
    const plantMarkets = new Set(
      CAPITAL_KINDS.flatMap((k) => w.parties.all().filter((p) => cells.has(String(p.id))).map((p) => String(goodMarketId(k.madeFrom, p.region)))),
    );
    const bids = w.journal
      .ofKind('smallBusiness.plan')
      .filter((e) => JSON.stringify(e.data['orders'] ?? []).split('"').some((s) => plantMarkets.has(s)));
    // A bid where there is a gap, and none where there is not: the two sides of one rule.
    if (shortOfPlant.length === 0) expect(bids).toHaveLength(0);
    else expect(bids.length).toBeGreaterThan(0);
  });

  it('draws nothing where there is nothing to draw from (App A)', () => {
    expect(drawSmallBusiness([], [{ bank: 'bank.a', size: 1 }], 100, 's')).toEqual([]);
    expect(drawSmallBusiness(['bakery'], [], 100, 's')).toEqual([]);
    expect(drawSmallBusiness(['bakery'], [{ bank: 'bank.a', size: 1 }], 0, 's')).toEqual([]);
  });
});

/**
 * §42 E1–E6 (11.5): what must not happen, each measured over one scale model. A FORBID that holds
 * breaks silently, which is why every one of them is a test and not a comment (Part II).
 */
describe('what must not happen (Small-Business Pools E1–E6, 11.5)', () => {
  const PERIODS = 20;
  const w = rigWorld('sb-e');
  const opening = w.parties
    .ofKind(SMALL_FIRM)
    .reduce((t, p) => t + (p.representation === 'cell' ? p.weight : 1), 0);
  const unitsReds: string[] = [];
  for (let i = 0; i < PERIODS; i += 1) {
    const r = w.step();
    for (const f of r.audit.families) {
      for (const v of f.violations) if (v.family === 'units' && v.owner === String(SMALL_FIRM)) unitsReds.push(v.message);
    }
  }
  const vehicles = w.parties.ofKind(VEHICLE);
  const deals = w.journal.ofKind('securitisation.cut');

  it('E1: no pool without loans to named borrowers', () => {
    for (const v of vehicles) {
      let rows = 0;
      for (const h of w.register.holdingsOf(v.id)) {
        const i = w.instruments.get(h.instrument);
        const profile = w.registry.instrumentKind(i.kind);
        if (profile.pricing === 'money' || isTranche(i.terms)) continue;
        // A row somebody owes, and who: never a loss rate, never anonymous exposure.
        expect(profile.liabilityOfIssuer && i.issuer.some).toBe(true);
        rows += 1;
      }
      if (v.status.alive) expect(rows).toBeGreaterThan(0);
    }
  });

  it('E2: no tranche without a holder', () => {
    for (const i of w.instruments.all()) {
      if (!i.status.live || !isTranche(i.terms)) continue;
      const holders = w.register.holdersOf(i.id).filter((h) => w.register.quantity(h, i.id) > 0);
      expect(holders.length).toBeGreaterThan(0);
    }
  });

  it('E3: no risk transfer without a transferee', () => {
    // Every deal names a vehicle that is a party holding the rows; the arranger holds none of them.
    for (const e of deals) {
      const vehicle = e.data['vehicle'];
      const arranger = e.data['arranger'];
      expect(typeof vehicle === 'string' && w.parties.has(vehicle as never)).toBe(true);
      if (typeof vehicle !== 'string' || typeof arranger !== 'string') continue;
      for (const h of w.register.holdingsOf(vehicle as never)) {
        const i = w.instruments.get(h.instrument);
        if (w.registry.instrumentKind(i.kind).pricing === 'money' || isTranche(i.terms)) continue;
        expect(w.register.quantity(arranger as never, i.id)).toBe(0);
      }
    }
  });

  it('E4: the population is not constant, and every change is a dated event with a cause', () => {
    const now = w.parties
      .ofKind(SMALL_FIRM)
      .filter((p) => p.status.alive)
      .reduce((t, p) => t + (p.representation === 'cell' ? p.weight : 1), 0);
    expect(now).not.toBe(opening);
    let byEvents = 0;
    for (const e of w.journal.ofKind('weight')) {
      const who = e.subjects[0];
      if (who === undefined || w.parties.get(who as never).kind !== SMALL_FIRM) continue;
      const before = e.data['before'];
      const after = e.data['after'];
      if (typeof before !== 'number' || typeof after !== 'number') continue;
      byEvents += after - before;
    }
    expect(now - opening).toBe(byEvents);
  });

  it('E5: no weight that is not a count, moved by five events only', () => {
    for (const p of w.parties.ofKind(SMALL_FIRM)) {
      if (p.representation !== 'cell') continue;
      expect(Number.isInteger(p.weight)).toBe(true);
      if (p.status.alive) expect(p.weight).toBeGreaterThan(0);
    }
    for (const e of w.journal.ofKind('weight')) {
      expect(['entry', 'death', 'promotion', 'split', 'merge']).toContain(e.data['kind']);
      expect(typeof e.data['cause']).toBe('string');
      expect(Number.isInteger(e.period)).toBe(true);
    }
    // The family that guards it stayed silent on this kind in every period.
    expect(unitsReds).toEqual([]);
  });

  it('E6: no loss allocated to a pool rather than to its cells', () => {
    // A loss is struck against a row somebody named owes, and lands on a named holder of a layer.
    for (const e of w.journal.ofKind('credit.impaired')) {
      expect(typeof e.data['issuer']).toBe('string');
      expect(w.parties.has(e.data['issuer'] as never)).toBe(true);
    }
    for (const e of w.journal.ofKind('tranche.writtenDown')) {
      expect(typeof e.data['holder'] === 'string' && w.parties.has(e.data['holder'] as never)).toBe(true);
    }
  });
});
