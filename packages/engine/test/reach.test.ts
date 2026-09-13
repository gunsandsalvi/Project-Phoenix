import { describe, expect, it } from 'vitest';
import { reachOf } from '../src/world/reach.js';
import { rigWorld } from './rig.js';

/** Three periods: enough for a market to open, clear and be marked, and cheap enough to run here. */
function reached(seed: string): ReturnType<typeof rigWorld>['reach'] extends () => infer R
  ? R
  : never {
  const w = rigWorld(seed);
  for (let i = 0; i < 3; i += 1) w.step();
  return w.reach();
}

describe('reach: what was declared, against what has ever come of it (Audit E1, E2)', () => {
  it('names every capability before anything runs, so "never" is a state and not a silence', () => {
    const all = reached('reach');
    const kinds = new Set(all.map((c) => c.kind));
    // All seven: two tallied as they happen, five derived from the stores that already answer.
    expect([...kinds].sort()).toEqual([
      'derivativeKind',
      'instrumentKind',
      'market',
      'participant',
      'partyKind',
      'store',
      'venueParticipant',
    ]);
    // Every one has somebody to answer for it. A finding with no owner is not a finding (Audit D2).
    for (const c of all) expect(c.owner.length).toBeGreaterThan(0);
  });

  it('counts, and the count is the point', () => {
    const all = reached('reach');
    const s = reachOf(all);
    expect(s.declared).toBe(all.length);
    expect(s.reached + s.never).toBe(s.declared);
    // The measurement this exists to publish. It was 221 of 397 the first time it ran, which is
    // what six passes of reading the source had been finding one at a time.
    expect(s.never).toBeGreaterThan(0);
    expect(s.reached).toBeGreaterThan(0);
  });

  it('finds the sectors three reads of the source found, and finds them in one period', () => {
    const never = new Set(reached('reach').filter((c) => c.produced === 0).map((c) => `${c.kind}:${c.id}`));
    // Each of these cost a day of reading to establish and is now a line in a report.
    expect(never).toContain('instrumentKind:corporate.bond'); // B-1: nothing ever issues one
    expect(never).toContain('partyKind:insurance'); // B-2: no insurer is ever created
    expect(never).toContain('instrumentKind:policy'); // B-2: no policy is ever written
    expect(never).toContain('participant:banks/bank'); // A-60: no bank quotes in any market
    expect(never).toContain('venueParticipant:housing/household'); // A-54, B-5: the tenancy venue
    // A-66, B-7: the derivative books. Every class the layer declares, and not one of them prints.
    for (const k of ['cds', 'cds.index', 'irs', 'option', 'bond.future', 'commodity.future']) {
      expect(never).toContain(`derivativeKind:${k}`);
    }
  });

  it('a capability that HAS produced is not in the list, so the list means something', () => {
    const all = reached('reach');
    const live = all.filter((c) => c.produced > 0);
    expect(live.length).toBeGreaterThan(0);
    // Money is issued, households exist, and their outlooks are written every period: three kinds
    // reached three ways — an instrument kind, a party kind and a module's own store.
    const ids = new Set(live.map((c) => `${c.kind}:${c.id}`));
    expect(ids).toContain('partyKind:household');
    expect(ids).toContain('store:expectations/outlooks');
    for (const c of live) expect(c.lastAt.some).toBe(true);
  });

  it('the audit report carries it every period, as a read and never as a family', () => {
    // Audit E1: the audit cannot find an absence, so this is not a violation and has no family.
    const { audit } = rigWorld('reach').step();
    const { reads, families } = audit;
    expect(reads.reach.declared).toBeGreaterThan(reads.reach.reached);
    expect(reads.reach.never).toBeGreaterThan(0);
    expect(families.map((f) => f.family)).not.toContain('reach');
    // And item 0's count rides with it: how much ontology is still in a module's bag.
    expect(reads.nouns['noun']).toBeGreaterThan(0);
  });
});
