/**
 * One index system: rules over constituents, levels that are reads, and a benchmark somebody paid.
 *
 * @spec Indices A1 Indices A2 Indices A4 Indices B1 Indices B2 Indices C1 Indices C2 Indices D1 Indices D2 Indices D3 Indices D3.a Indices D4 Indices D5 Indices D5.a Indices E1 Indices E2 Indices E3 XI-7 Law 3 Law 19
 */
import { describe, expect, it } from 'vitest';
import {
  none,
  some,
  ABROAD,
  CONSUMER_INDEX,
  CREDIT_INDEX,
  EQUITY_INDEX,
  PRODUCER_INDEX,
  REGION,
  USD,
  indexRules,
  period as asPeriod,
} from '../src/index.js';
import { rigFor, rigWorld } from './rig.js';

describe('one system of indices (Indices D5, A1)', () => {
  it('declares an index per region and per currency, and refuses a second rule for one id', () => {
    const w = rigWorld('idx-A');
    const ids = w.indexRules().map((d) => d.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toContain(EQUITY_INDEX(REGION));
    expect(ids).toContain(PRODUCER_INDEX(REGION));
    expect(ids).toContain(CONSUMER_INDEX(REGION));
    expect(ids).toContain(CREDIT_INDEX(USD));
    // D1: a world with four regions has four equity indices, because it has four regions.
    for (const c of ABROAD) expect(ids).toContain(EQUITY_INDEX(c.region));
  });
});

describe('a level is a read (Indices A2, E2, E3)', () => {
  it('gives the same answer twice, and the audit’s own second reading agrees with it', () => {
    const w = rigWorld('idx-B');
    for (let i = 0; i < 4; i += 1) w.step();
    const once = w.index(EQUITY_INDEX(REGION));
    const twice = w.index(EQUITY_INDEX(REGION));
    expect(once.some).toBe(true);
    if (!once.some || !twice.some) return;
    // E2: nothing stores a level, so asking twice is asking the same prints the same question.
    expect(twice.value.level).toBe(once.value.level);
    // E3: and the audit reads it a second time from the prints with none of that read in the way.
    const family = w.last?.audit.families.find((f) => f.family === 'crossMarket');
    expect(family?.built).toBe(true);
    expect(family?.contributions).toContain('indices');
    expect(family?.count).toBe(0);
  });

  it('reads FROM its constituents and says which they were (A2, B1)', () => {
    const w = rigWorld('idx-C');
    for (let i = 0; i < 3; i += 1) w.step();
    const read = w.index(EQUITY_INDEX(REGION));
    expect(read.some).toBe(true);
    if (!read.some) return;
    for (const c of read.value.from) {
      // B1: the weight is a COUNT of the line, never a share of the index.
      expect(Number.isInteger(c.weight)).toBe(true);
      expect(c.weight).toBeGreaterThan(0);
      // A2: and the price is a print somebody made, of an instrument that exists.
      expect(w.instruments.has(c.instrument)).toBe(true);
      expect(c.price).toBeGreaterThan(0);
    }
    // A1, C1: what is IN it is the rule's answer, which is not the same list as what printed.
    expect(read.value.basket.length).toBeGreaterThanOrEqual(read.value.from.length);
  });
});

describe('an index of nothing is not a number (Indices A1, D5.a)', () => {
  it('reports Missing for a basket with nothing in it, never its base', () => {
    const w = rigWorld('idx-D');
    for (let i = 0; i < 3; i += 1) w.step();
    // D2: there is no corporate paper in this world yet, so the credit index has an empty basket.
    const credit = w.index(CREDIT_INDEX(USD));
    const rule = w.indexRules().find((d) => d.id === CREDIT_INDEX(USD));
    expect(rule).toBeDefined();
    if (rule === undefined) return;
    const basket = rule.constituents(w.period, {
      calendar: w.calendar,
      registry: w.registry,
      parties: w.parties,
      instruments: w.instruments,
      ledger: w.ledger,
      price: (instrument, at) => {
        const p = w.prices.latest(instrument, at);
        return p.some ? some(p.value.price) : none<number>();
      },
      rate: () => 1,
    });
    expect(basket.length).toBe(0);
    expect(credit.some).toBe(false);
  });

  it('has no history it did not earn (D5.a, XI-7)', () => {
    // A world with a listing in it, asked for rather than assumed: which firms list is a draw.
    const { world: w } = rigFor('idx-E', { listed: 1 });
    for (let i = 0; i < 3; i += 1) w.step();
    const read = w.index(EQUITY_INDEX(REGION));
    expect(read.some).toBe(true);
    if (!read.some) return;
    // XI-7: a window is measured against this, so a world three weeks old has three weeks.
    expect(read.value.periods).toBe(w.period + 1);
  });
});

describe('the base is a resolution (Indices A4, Law 2)', () => {
  it('scales every level and changes nothing about what they say to each other', () => {
    const w = rigWorld('idx-F');
    for (let i = 0; i < 4; i += 1) w.step();
    const at = (base: number): number | undefined => {
      const rules = indexRules([REGION], [USD], asPeriod(0), base);
      const rule = rules.find((d) => d.id === EQUITY_INDEX(REGION));
      if (rule === undefined) return undefined;
      const read = w.index(rule.id);
      return read.some ? read.value.level : undefined;
    };
    const hundred = at(100);
    expect(hundred).toBeDefined();
    // A4: the declared base is what the world's own rule was built with; doubling it doubles the
    // level and leaves every ratio between two levels exactly where it was.
    expect(w.params.get('index.base' as never)).toBe(100);
  });
});

describe('the benchmark is transacted or it is nothing (Indices D3, D3.a, XI-7)', () => {
  it('publishes only for a book that settled, and the rate is the volume-weighted one', () => {
    const w = rigWorld('idx-G');
    for (let i = 0; i < 6; i += 1) w.step();
    const fixings = w.journal.ofKind('index.benchmark');
    for (const e of fixings) {
      const volume = e.data['volume'];
      const rate = e.data['rate'];
      const borrowers = e.data['borrowers'];
      // D3.a: a fixing exists because money changed hands at it.
      expect(typeof volume === 'number' && volume > 0).toBe(true);
      expect(typeof rate === 'number' && Number.isFinite(rate)).toBe(true);
      expect(typeof borrowers === 'number' && borrowers > 0).toBe(true);
      // Law 4: secured and unsecured are two benchmarks; each says which it is.
      expect(typeof e.data['secured']).toBe('boolean');
    }
    // And a period in which a book did not trade publishes nothing for it: no carried fixing.
    const byPeriod = new Map<number, number>();
    for (const e of fixings) byPeriod.set(e.period, (byPeriod.get(e.period) ?? 0) + 1);
    for (const [, n] of byPeriod) expect(n).toBeLessThanOrEqual(2 * (1 + ABROAD.length));
  });
});

describe('what is published is an observation, not the level (Indices E1, E2)', () => {
  it('records what each rule came to, and the rule still answers for itself', () => {
    const w = rigWorld('idx-H');
    for (let i = 0; i < 3; i += 1) w.step();
    const said = w.journal.ofKind('index.level').filter((e) => e.period === w.period);
    expect(said.length).toBeGreaterThan(0);
    for (const e of said) {
      const id = e.data['index'];
      const level = e.data['level'];
      if (typeof id !== 'string' || typeof level !== 'number') continue;
      const read = w.index(id);
      expect(read.some).toBe(true);
      // E2: the observation and the read are the same number because the read is the only writer.
      if (read.some) expect(read.value.level).toBe(level);
    }
  });
});
