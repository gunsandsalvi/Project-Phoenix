/**
 * The population is not constant, and no event is the identity of another (Small-Business Pools
 * E4, Households F4, Firm Birth A4.a, 12.7).
 *
 * @spec Small-Business Pools E4 Small-Business Pools A6.c Households F1.b Households F4 Firm Birth A4.a XI-15
 *
 * Measured over the four-country scale model, where promotion fires. Entries are the two
 * mechanisms 12.1 and 12.2 built — founding and formation — and in the scale models both are
 * refused every period for reasons on the record (no service line trades; no rent clears), so
 * what this asserts about entry is that the mechanism decides every period and never by a rate,
 * and that exit is not its accounting identity. The day a book has two sides, the refusal count
 * here turns into entries and nothing in this test changes.
 */
import { describe, expect, it } from 'vitest';
import { HOUSEHOLD, SMALL_FIRM } from '../src/index.js';
import { abroadWorld } from './rig.js';

describe('entries, exits and promotions over twenty periods (E4, F4, 12.7)', () => {
  it('exits happen, promotions happen to some cells and not all, and entry is decided every period and is never exit’s identity', () => {
    const w = abroadWorld('population');
    const cellsAtOpen = w.parties.ofKind(SMALL_FIRM).filter((p) => p.status.alive).length;
    const householdsAtOpen = w.parties.ofKind(HOUSEHOLD).reduce((t, p) => t + (p.representation === 'cell' ? p.weight : 0), 0);
    const PERIODS = 20;
    let refusalsEveryPeriod = true;
    for (let i = 0; i < PERIODS; i += 1) {
      w.step();
      const founding = w.journal.ofKindIn('smallBusiness.founded', w.period).length + w.journal.ofKindIn('smallBusiness.notFounded', w.period).length;
      const forming = w.journal.ofKindIn('households.lifecycle', w.period).filter((e) => e.data['event'] === 'formed' || e.data['event'] === 'notFormed').length;
      if (founding + forming === 0) refusalsEveryPeriod = false;
    }
    const by = new Map<string, number>();
    const promoted = new Set<string>();
    for (const e of w.journal.ofKind('weight')) {
      const who = e.subjects[0];
      if (who === undefined) continue;
      const kind = w.parties.get(who as never).kind;
      const tier = kind === HOUSEHOLD ? 'hh' : kind === SMALL_FIRM ? 'sb' : 'other';
      const k = `${tier}.${String(e.data['kind'])}`;
      by.set(k, (by.get(k) ?? 0) + Number(e.data['members'] ?? 0));
      if (tier === 'sb' && e.data['kind'] === 'promotion' && typeof e.data['before'] === 'number') promoted.add(who);
    }
    const get = (k: string): number => by.get(k) ?? 0;
    // Exits: both populations lose members by dated events with a cause (F1.b, E5).
    expect(get('hh.death')).toBeGreaterThan(0);
    expect(get('sb.death')).toBeGreaterThan(0);
    // Entry is decided every period — founded or refused with a reason, formed or refused with a
    // reason — and is never exit's accounting identity (E4): what enters is not what left.
    expect(refusalsEveryPeriod).toBe(true);
    expect(get('sb.entry')).not.toBe(get('sb.death'));
    expect(get('hh.entry')).not.toBe(get('hh.death'));
    // Promotions: some small-firm cells outgrew the tier, and not every cell (A6.c).
    expect(promoted.size).toBeGreaterThan(0);
    expect(promoted.size).toBeLessThan(cellsAtOpen);
    // F4: the composition moved, and the aggregate followed from the cells.
    const householdsNow = w.parties.ofKind(HOUSEHOLD).filter((p) => p.status.alive).reduce((t, p) => t + (p.representation === 'cell' ? p.weight : 0), 0);
    expect(householdsNow).not.toBe(householdsAtOpen);
  });
});
