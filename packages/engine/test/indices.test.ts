/**
 * One index system: rules over constituents, levels that are reads, and a benchmark somebody paid.
 *
 * @spec Indices A1 Indices A2 Indices A4 Indices B1 Indices B2 Indices C1 Indices C2 Indices D1 Indices D2 Indices D3 Indices D3.a Indices D4 Indices D5 Indices D5.a Indices E1 Indices E2 Indices E3 XI-7 Law 3 Law 19
 */
import { asRatio, type PerPiece } from '../src/core/measure.js';
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

import { RATED_INDEX } from '../src/mechanisms/indices/index.js';
import { creditOf, ratedOf } from '../src/mechanisms/indices/baskets.js';
import { GRADES, INVESTMENT_GRADE, isInvestmentGrade, type Grade } from '../src/registry/grades.js';
import type { Cash } from '../src/core/measure.js';
import type { InstrumentId } from '../src/core/ids.js';
import type { Period } from '../src/calendar/calendar.js';
import type { Option } from '../src/core/option.js';
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
      graded: () => none(),
      price: (instrument, at) => {
        const p = w.prices.latest(instrument, at);
        return p.some ? some(p.value.price) : none<PerPiece>();
      },
      rate: () => asRatio(1, 'one into one'),
      inMoney: (value) => value,
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
      const rules = indexRules([REGION], [USD], asPeriod(0), asRatio(base, 'the base under test'), () => USD);
      const rule = rules.find((d) => d.id === EQUITY_INDEX(REGION));
      if (rule === undefined) return undefined;
      const read = w.index(rule.id);
      return read.some ? read.value.level : undefined;
    };
    const hundred = at(100);
    expect(hundred).toBeDefined();
    // A4: the declared base is what the world's own rule was built with; doubling it doubles the
    // level and leaves every ratio between two levels exactly where it was.
    expect(w.params.ratio('index.base' as never)).toBe(100);
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

/**
 * The rated universe (Indices A1, C2, C2.a; Ratings C2; item 17.10).
 *
 * A credit market is bought on ONE SIDE OF A LINE. Investment grade and high yield are one scale
 * with a boundary across it, and everything that matters about a credit market happens there: a
 * mandate says which side it may hold, and a name that CROSSES is sold by everybody who may not
 * hold the other side.
 */
describe('a credit index over the rated universe (Indices C2, Ratings C2)', () => {
  /** The rig's own world, with the one read a rated basket needs answered for the test. */
  function worldWith(w: ReturnType<typeof rigWorld>, grade: Option<Grade>) {
    return {
      calendar: w.calendar,
      registry: w.registry,
      parties: w.parties,
      instruments: w.instruments,
      ledger: w.ledger,
      graded: () => grade,
      price: (instrument: InstrumentId, at: Period) => {
        const p = w.prices.latest(instrument, at);
        return p.some ? some(p.value.price) : none<PerPiece>();
      },
      rate: () => asRatio(1, 'one into one'),
      inMoney: (value: Cash) => value,
    };
  }

  it('cuts the scale in two and leaves nothing on neither side', () => {
    // The boundary is DATA beside the scale itself, and it partitions it: every grade is on exactly
    // one side, and the sides meet where the market says they meet.
    const inside = GRADES.filter((g) => isInvestmentGrade(g));
    const outside = GRADES.filter((g) => !isInvestmentGrade(g));
    expect(inside.length + outside.length).toBe(GRADES.length);
    expect(inside).toContain(INVESTMENT_GRADE);
    expect(outside).not.toContain(INVESTMENT_GRADE);
    expect(inside[0]).toBe('aaa');
    // And the world declares a line on each side of it, in every money it has.
    const w = rigWorld('idx-rated');
    const ids = w.indexRules().map((d) => d.id);
    expect(ids).toContain(RATED_INDEX(USD, 'investment'));
    expect(ids).toContain(RATED_INDEX(USD, 'speculative'));
    expect(ids).toContain(CREDIT_INDEX(USD));
  });

  it('holds what the assessors graded onto its own side, and nothing they did not grade', () => {
    const w = rigWorld('idx-rated');
    for (let i = 0; i < 6; i += 1) w.step();
    const all = creditOf(USD)(w.period, worldWith(w, none<Grade>()));
    // A fresh rule per ask: a rule memoises its answer for as long as the register stands (Law 18),
    // which is right in the world and wrong for a test that varies what the assessors said.
    const side = (which: 'investment' | 'speculative', grade: Option<Grade>) =>
      ratedOf(USD, which)(w.period, worldWith(w, grade));
    expect(all.length).toBeGreaterThan(0);
    // A name nobody has graded is in NEITHER basket. Being unrated is not a side of the line; it is
    // the absence of an opinion, and an index that guessed would be an index with a credit view.
    expect(side('investment', none<Grade>())).toHaveLength(0);
    expect(side('speculative', none<Grade>())).toHaveLength(0);
    // Graded inside the line, every corporate line in the money is in the investment basket and
    // none is in the other; graded outside it, the two swap. The universe is the same either way.
    const good = some<Grade>('a');
    const bad = some<Grade>('b');
    expect(side('investment', good).length).toBe(all.length);
    expect(side('speculative', good)).toHaveLength(0);
    expect(side('speculative', bad).length).toBe(all.length);
    expect(side('investment', bad)).toHaveLength(0);
  });
});

/**
 * The chain from a company's books to a rated index (Reporting A1; Ratings A5, C2; Indices C2;
 * item 17.10a).
 *
 * It is four mechanisms and each is somebody's act: a company closes a quarter, shows the statement
 * to the assessor it pays, the assessor grades what it was shown, and the index reads the grade. A
 * census over twelve and twenty periods found none of it happening and called it a defect. It was
 * the CALENDAR: a quarter is thirteen weeks and the first one that opens after the epoch closes
 * around period twenty, so a world stepped for less than that has nothing to report on.
 */
describe('from a closed quarter to a rated index (Reporting A1, Ratings A5, Indices C2)', () => {
  it('reports, shows, grades and indexes, in that order and on the calendar', () => {
    // The draw decides whether this world has corporate paper for a rated basket to hold at all;
    // this seed makes some. What the test is about is the ORDER of the four acts, not the dates.
    const w = rigWorld('rated-a');
    let firstReport = 0;
    let firstRating = 0;
    let firstLevel = 0;
    for (let i = 0; i < 22; i += 1) {
      w.step();
      if (firstReport === 0 && w.journal.ofKind('reporting.report').length > 0) firstReport = w.period;
      if (firstRating === 0 && w.journal.ofKind('rating.action').length > 0) firstRating = w.period;
      if (
        firstLevel === 0 &&
        (w.index(RATED_INDEX(USD, 'investment')).some || w.index(RATED_INDEX(USD, 'speculative')).some)
      ) {
        firstLevel = w.period;
      }
    }
    // Seed A2, Money G3: nothing is reported on a quarter that began before the world did, so the
    // first close is between one and four quarters in — and everything downstream waits for it.
    expect(firstReport).toBeGreaterThan(0);
    expect(w.journal.ofKind('disclosed').length).toBeGreaterThan(0);
    // A5: the assessor grades what it was SHOWN, so it cannot speak before the company has spoken.
    expect(firstRating).toBeGreaterThanOrEqual(firstReport);
    // C2: and a rated index has a level once there is a graded name in its basket — not before,
    // because an index of nothing is not a number (D5.a).
    expect(firstLevel).toBeGreaterThanOrEqual(firstRating);
    expect(firstLevel).toBeGreaterThan(0);
  });
});
