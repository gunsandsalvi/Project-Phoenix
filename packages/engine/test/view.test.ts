import { describe, expect, it } from 'vitest';
import { instrumentId, partyId } from '../src/core/ids.js';
import { about, subjectOf, type Subject } from '../src/world/context.js';
import { rigWorld } from './rig.js';

const EVERY: readonly Subject[] = [
  { on: 'price', instrument: instrumentId('gov.bill.2026-06-15') },
  { on: 'bought', instrument: instrumentId('good.grain.us.1') },
  { on: 'sold', instrument: instrumentId('good.grain.us.1') },
  { on: 'income' },
  { on: 'earnings' },
  { on: 'credit', party: partyId('firm.4') },
];

describe('what a belief is about (§46, XI-16)', () => {
  it('round-trips every subject, so nothing takes a key apart anywhere else', () => {
    for (const s of EVERY) {
      const back = subjectOf(about(s));
      expect(back.some).toBe(true);
      if (back.some) expect(back.value).toEqual(s);
    }
  });

  it('can say what one party thinks of ANOTHER, which the namespace could not', () => {
    /**
     * The whole point. `OutlookVariable` was a bare string and the twenty-five beliefs this world
     * held were all about observables — a price, what was bought, what was sold, its own income.
     * Not one was about another PARTY, which is what a probability of default is, and a rating
     * opinion, and a dealer's adverse-selection charge, and a depositor's confidence. Appendix B
     * requires one PD model per borrower and there was nowhere for one to live.
     */
    const s: Subject = { on: 'credit', party: partyId('firm.4') };
    const back = subjectOf(about(s));
    expect(back.some).toBe(true);
    if (back.some && back.value.on === 'credit') expect(back.value.party).toBe(partyId('firm.4'));
  });

  it('refuses a key nobody made through the door', () => {
    // A string that is not an encoded subject decodes to nothing rather than to a subject somebody
    // guessed at. The type stops it being formed at all; this is the other end of the same rule.
    expect(subjectOf('goods.price.wheat' as never).some).toBe(false);
    expect(subjectOf('' as never).some).toBe(false);
    expect(subjectOf('nonsense.x' as never).some).toBe(false);
  });

  it('a real world holds only subjects, and every one of them decodes', () => {
    const w = rigWorld('view');
    for (let i = 0; i < 3; i += 1) w.step();
    let held = 0;
    const kinds = new Set<string>();
    for (const p of w.parties.all()) {
      for (const v of w.outlookVariables(p.id)) {
        const s = subjectOf(v);
        // Nothing in the store can be a key the door did not make. If this ever fails, something
        // has written a belief under a name nobody declared (Law 15).
        expect(s.some).toBe(true);
        if (s.some) kinds.add(s.value.on);
        held += 1;
      }
    }
    expect(held).toBeGreaterThan(0);
    // And what this world actually believes things about: prices, and its own income and earnings.
    // `credit` is expressible now and nothing forms one yet — that is the next mechanism, not a
    // property of the door (`docs/AUDIT.md` item 6).
    expect(kinds.has('credit')).toBe(false);
  });

  it('the view hands back subjects rather than keys, so no reader parses one', () => {
    const w = rigWorld('view');
    for (let i = 0; i < 3; i += 1) w.step();
    const withViews = w.parties.all().filter((p) => w.outlookVariables(p.id).length > 0);
    expect(withViews.length).toBeGreaterThan(0);
    const first = withViews[0];
    if (first === undefined) return;
    const subjects = w.participantView(first.id).outlookSubjects();
    expect(subjects.length).toBe(w.outlookVariables(first.id).length);
    for (const s of subjects) expect(typeof s.on).toBe('string');
  });
});
