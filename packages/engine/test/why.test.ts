/**
 * `why(party, period)`: the query every hand-written probe in the record was (0h.4).
 *
 * @spec Audit E1 Audit E2 Audit E3 Part XII Observer A4 Law 19
 *
 * The material was always recorded — the journal has every event with its subjects, the ledger every
 * instruction with its cause and its failure — and what was missing was the read that puts them in
 * front of one party, together with the thing nothing recorded at all: what that party ASKED this
 * world for and did not get.
 */
import { describe, expect, it } from 'vitest';
import { FIRM, HOUSEHOLD, type World } from '../src/index.js';
import { rigWorld } from './rig.js';

function ran(seed: string, periods: number): World {
  const w = rigWorld(seed);
  for (let i = 0; i < periods; i += 1) w.step();
  return w;
}

describe('why a party did what it did (0h.4)', () => {
  it('names what it was a side of, with the cause settlement recorded', () => {
    const w = ran('why-a', 5);
    const cell = w.parties.ofKind(HOUSEHOLD)[0];
    if (cell === undefined) throw new Error('no cell');
    const y = w.why(cell.id, w.period);
    expect(y.party).toBe(cell.id);
    expect(y.period).toBe(w.period);
    expect(y.alive).toBe(true);
    expect(y.bornAt.some).toBe(true);
    expect(y.settled.length).toBeGreaterThan(0);
    for (const i of y.settled) {
      // Law 19: every field is the ledger's own — the cause it was settled under, and for a
      // failure the reason settlement itself gave and the party whose want stopped it.
      expect(i.cause.length).toBeGreaterThan(0);
      if (i.outcome === 'failed') {
        expect(i.failed).toBeDefined();
        expect(i.against).toBeDefined();
      } else {
        expect(i.failed).toBeUndefined();
      }
    }
  });

  it('names the reads it asked for and did not get, in the words the door was asked in', () => {
    const w = ran('why-b', 5);
    // Whoever it is: some party in a world this incomplete wanted something it could not have.
    const wanting = w.parties
      .alive()
      .map((p) => w.why(p.id, w.period))
      .filter((y) => y.wanted.length > 0);
    expect(wanting.length).toBeGreaterThan(0);
    for (const y of wanting) {
      for (const want of y.wanted) {
        // The name is the read, not a message: `outlook.<variable>`, `print.<instrument>`,
        // `mark.<instrument>`. A diagnosis reads it and knows which door answered nothing.
        expect(/^(outlook|print|mark)\./.test(want.read)).toBe(true);
        expect(want.times).toBeGreaterThan(0);
      }
    }
  });

  it('names the books it was asked about and posted nothing into, apart from the reads', () => {
    const w = ran('why-c', 5);
    const firm = w.parties.ofKind(FIRM)[0];
    if (firm === undefined) throw new Error('no firm');
    const y = w.why(firm.id, w.period);
    // A firm is asked about every book of its kind and bids in a few: the rest is this list, and
    // it is kept apart from `wanted` because otherwise it swamps it (forty venues to one read).
    expect(y.quietIn.length).toBeGreaterThan(0);
    for (const book of y.quietIn) expect(y.wanted.map((x) => x.read)).not.toContain(book);
  });

  it('answers about the period being run, and says which period that is', () => {
    const w = ran('why-d', 4);
    const cell = w.parties.ofKind(HOUSEHOLD)[0];
    if (cell === undefined) throw new Error('no cell');
    const now = w.why(cell.id, w.period);
    expect(now.wantedIsAbout).toBe(w.period);
    // An earlier period is answered from the journal and the ledger, which ARE that history — and
    // its wants are empty rather than wrong, with `wantedIsAbout` saying so (Missing is missing).
    const before = w.why(cell.id, (w.period - 1) as typeof w.period);
    expect(before.wanted).toEqual([]);
    expect(before.quietIn).toEqual([]);
    expect(before.wantedIsAbout).toBe(w.period);
    expect(before.events.length).toBeGreaterThan(0);
  });

  it('reads and changes nothing: asking twice answers the same', () => {
    const w = ran('why-e', 4);
    const firm = w.parties.ofKind(FIRM)[0];
    if (firm === undefined) throw new Error('no firm');
    const once = w.why(firm.id, w.period);
    const twice = w.why(firm.id, w.period);
    expect(twice).toEqual(once);
  });
});
