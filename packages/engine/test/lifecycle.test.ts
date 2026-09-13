import { describe, expect, it } from 'vitest';
import { inGoodStanding, STANDINGS } from '../src/parties/party.js';
import { rigWorld } from './rig.js';

describe('the states between alive and dead (XI-3, §25 C1, XI-8)', () => {
  it('names the four the specification does, and nothing is a severity scale', () => {
    expect([...STANDINGS]).toEqual(['good', 'distressed', 'inResolution', 'winding']);
  });

  it('a world opens with everybody in good standing, and `alive` still means alive', () => {
    const w = rigWorld('lifecycle');
    w.step();
    const live = w.parties.all().filter((p) => p.status.alive);
    expect(live.length).toBeGreaterThan(0);
    // 129 readers of `status.alive` did not change: the state was ADDED to the alive branch, so
    // what they meant still holds and what they could not see is now there.
    for (const p of live) expect(inGoodStanding(p.status)).toBe(true);
  });

  it('a change of standing is public, because it is what a depositor runs from', () => {
    const w = rigWorld('lifecycle');
    for (let i = 0; i < 8; i += 1) w.step();
    // Journalled with what it was, what it is and why — the three things a reader needs to act.
    for (const e of w.journal.ofKind('party.standing')) {
      expect(typeof e.data['was']).toBe('string');
      expect(typeof e.data['now']).toBe('string');
      expect(String(e.data['cause']).length).toBeGreaterThan(0);
    }
  });

  it('refuses to move a party that has ceased', () => {
    const w = rigWorld('lifecycle');
    w.step();
    const anyone = w.parties.all().find((p) => p.status.alive);
    if (anyone === undefined) return;
    const heir = w.parties.all().find((p) => p.id !== anyone.id);
    if (heir === undefined) return;
    w.parties.cease(anyone.id, w.period, heir.id);
    // XI-3, Register F2: a status is a fact about a party that is still there. A successor's is
    // its own, and moving a dead party's would be writing over the end of it.
    expect(() => {
      w.parties.standing(anyone.id, 'distressed', 'after the end');
    }).toThrow(/ceased/);
  });

  it('is not a ladder: nothing checks an order between the states', () => {
    // Banks Capital C1.a — a capital trigger takes a bank from `good` straight to `inResolution`
    // with no missed payment anywhere. A lifecycle that insisted on distress first would be an
    // outcome written as a rule.
    const w = rigWorld('lifecycle');
    w.step();
    const p = w.parties.all().find((x) => x.status.alive);
    if (p === undefined) return;
    w.parties.standing(p.id, 'inResolution', 'a capital trigger, with nothing missed');
    const now = w.parties.get(p.id).status;
    expect(now.alive && now.standing).toBe('inResolution');
    // And back, because a resolution can end with the bank still trading (§25 C).
    w.parties.standing(p.id, 'good', 'resolved and returned');
    expect(inGoodStanding(w.parties.get(p.id).status)).toBe(true);
  });
});
