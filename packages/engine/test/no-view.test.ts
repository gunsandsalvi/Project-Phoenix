/**
 * Every goods book has a party with a view in it (XI-13, Clearing A1.a, §46 C3, 12d.4).
 *
 * `market.noView` is the kernel's own read: a book whose every order came from a mandate, with
 * nobody in it who named a level from what it thinks the thing is worth and put its own money
 * behind it, says so every period it runs. After 12d every party holds an outlook on every price it
 * is exposed to, so a goods book should never run without a view — this test is the measurement.
 *
 * @spec XI-13 Clearing A1.a Expectations C3 Expectations A3
 */
import { describe, expect, it } from 'vitest';
import { rigWorld } from './rig.js';

declare const console: { log: (line: string) => void };

const PERIODS = 30;

describe('no goods book runs without a view in it (12d.4)', () => {
  it('journals market.noView for no goods book in the census', () => {
    const w = rigWorld('no-view');
    for (let i = 0; i < PERIODS; i += 1) w.step();
    const goods = w.markets.filter((m) => String(m.id).startsWith('mkt.good.'));
    expect(goods.length).toBeGreaterThan(0);
    const perBook = new Map<string, number>();
    for (const e of w.journal.ofKind('market.noView')) {
      const market = String(e.subjects[0]);
      if (!market.startsWith('mkt.good.')) continue;
      perBook.set(market, (perBook.get(market) ?? 0) + 1);
    }
    const lastPeriod = w.journal.ofKindIn('market.noView', w.period).filter((e) => String(e.subjects[0]).startsWith('mkt.good.'));
    const rows = [...perBook].sort((a, b) => b[1] - a[1]).map(([m, n]) => `${m}:${String(n)}`);
    console.log(`no-view census over ${String(PERIODS)} periods: ${String(perBook.size)} of ${String(goods.length)} goods books ran without a view at least once; in period ${String(w.period)}: ${String(lastPeriod.length)}; ${rows.join(' ')}`);
    expect(lastPeriod.map((e) => String(e.subjects[0]))).toEqual([]);
  });
});
