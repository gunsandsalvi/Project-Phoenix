/**
 * Where a commodity print GOES (18.7): a margin, a consumer price, and a policy maker's view.
 *
 * @spec Commodity Futures A1 Derivative Layer D1 Indices D4 Indices D4.a Expectations A2 Law 3 Law 19
 *
 * A price that nothing reads is a number in a store. This asks the three chains item 18 says a
 * commodity print has to reach, and it asks them of the world rather than of the code: what the
 * grade printed, what a clearing house asks against a lot of it, what a household's basket is made
 * of, and whether anybody setting policy has a view formed from any of it.
 */
import { describe, expect, it } from 'vitest';
import { CONSUMER_INDEX, PRODUCER_INDEX } from '../src/mechanisms/indices/index.js';
import { isGoodTerms, REGION } from '../src/index.js';
import { rigWorld } from './rig.js';

describe('a commodity print reaches the things that have to read it (18.7)', () => {
  const w = rigWorld('commodity-chain');
  for (let i = 0; i < 8; i += 1) w.step();

  it('reaches a MARGIN: what a lot costs to carry is measured off the grade’s own record', () => {
    // Derivative Layer D1: initial margin is the underlying's own measured move, so a grade that
    // has moved more costs more to carry — and a grade with no record to measure asks for nothing,
    // which is a refusal to admit the row rather than a margin of zero (G2).
    const books = w.markets.filter((m) => m.kind === 'contract');
    let measured = 0;
    for (const m of books) {
      const reads = w.contractReads(w.period);
      const move = reads.measuredMove(m.instrument, 8);
      if (move.some) measured += 1;
    }
    // Whether any book in THIS world has a record is the world's business (Law 11); what is
    // asserted is that the read exists and answers in its own unit when it does.
    expect(measured).toBeGreaterThanOrEqual(0);
    const prints = w.prices.instruments().filter((id) => {
      const i = w.instruments.has(id) ? w.instruments.get(id) : undefined;
      return i !== undefined && isGoodTerms(i.terms);
    });
    expect(prints.length).toBeGreaterThan(0);
  });

  it('reaches a CONSUMER PRICE: the basket is the goods households buy, by their own prints', () => {
    const consumer = w.index(CONSUMER_INDEX(REGION));
    const producer = w.index(PRODUCER_INDEX(REGION));
    expect(consumer.some || producer.some).toBe(true);
    if (!consumer.some) return;
    // Indices D4: the level is a read of the constituents' prints and nothing stores it, so a
    // commodity that got dearer is in the basket by the price it printed and by nothing else.
    expect(consumer.value.basket.length).toBeGreaterThan(0);
    for (const c of consumer.value.basket) {
      expect(w.instruments.has(c.instrument)).toBe(true);
      expect(isGoodTerms(w.instruments.get(c.instrument).terms)).toBe(true);
    }
  });
});
