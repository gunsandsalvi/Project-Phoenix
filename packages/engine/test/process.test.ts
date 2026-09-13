/**
 * A procedure that takes more than one period, and who inherits when nobody was chosen.
 *
 * @spec XI-8 Firm Birth D5 Firm Birth F2 Law 2 Law 4 Law 15 Law 19
 */
import { describe, expect, it } from 'vitest';
import { partyId } from '../src/core/ids.js';
import { period } from '../src/calendar/calendar.js';
import { Processes } from '../src/register/processes.js';
import { ranWorld } from './rig.js';

const WHO = partyId('estate.firm.1');

const decl = (over: Partial<Parameters<Processes['open']>[0]> = {}) => ({
  what: 'estate',
  subject: WHO,
  steps: ['selling', 'paying', 'dividing'],
  closesAfter: period(10),
  why: 'XI-8',
  ...over,
});

describe('what a party is in the middle of (XI-8, D5)', () => {
  it('refuses a procedure with no steps and one that closes before it opened', () => {
    const b = new Processes();
    expect(() => {
      b.open(decl({ steps: [] }), period(1));
    }).toThrow(/procedure with no steps/);
    expect(() => {
      b.open(decl({ closesAfter: period(0) }), period(1));
    }).toThrow(/before it opened/);
  });

  it('walks its steps in order, by name, and will not walk past the last', () => {
    /**
     * `Winding` could say started and finished and nothing between — which are the two states an
     * estate spends none of its time in. It sells, it pays in rank order, and it divides.
     */
    const b = new Processes();
    const p = b.open(decl(), period(1));
    expect(b.step(p.id)).toBe('selling');
    expect(b.step(b.advance(p.id).id)).toBe('paying');
    expect(b.step(b.advance(p.id).id)).toBe('dividing');
    // Leaving the last step is ENDING, and ending says so rather than being an off-by-one.
    expect(() => b.advance(p.id)).toThrow(/last step/);
  });

  it('closing and abandoning are different facts, and neither happens twice', () => {
    const b = new Processes();
    const a = b.open(decl(), period(1));
    expect(b.close(a.id).state).toBe('closed');
    expect(() => b.close(a.id)).toThrow(/already closed/);
    expect(() => b.advance(a.id)).toThrow(/closed and cannot advance/);
    const c = b.open(decl(), period(1));
    // It stopped without finishing: the subject died, the offer lapsed, the project was dropped.
    expect(b.abandon(c.id).state).toBe('abandoned');
  });

  it('answers what a party is in the middle of, and what is out of time', () => {
    const b = new Processes();
    const p = b.open(decl(), period(1));
    b.open(decl({ what: 'resolution', closesAfter: period(3) }), period(1));
    expect(b.of(WHO).length).toBe(2);
    expect(b.running('estate').map((x) => x.id)).toEqual([p.id]);
    // D5: running and out of time. A programme that must be over and is not is the caller's to end.
    expect(b.dueBy(period(2))).toEqual([]);
    expect(b.dueBy(period(3)).length).toBe(1);
    expect(b.dueBy(period(10)).length).toBe(2);
  });
});

describe('who inherits when nobody was chosen (A-21)', () => {
  it('is every survivor where the dead lived and banked, and never one of them', () => {
    /**
     * It used to be `.find` — the FIRST cell the parties store happened to return, of the first
     * cohort. Every estate in a (region, bank) went to that one cell and to no other, so one
     * household cell in each region accumulated the wealth of everybody who died there and the rest
     * inherited nothing ever. Which cell it was depended on insertion order, which is a seed-draw
     * artefact and not a fact about the world.
     */
    const w = ranWorld('inherit', 14);
    const divided = w.journal
      .ofKind('households.lifecycle')
      .filter((e) => e.data['event'] === 'divided');
    if (divided.length === 0) return;
    const heirs = new Set(divided.map((e) => String(e.data['heir'])));
    // The whole point: MORE THAN ONE. A world where one cell inherits everything is what this was.
    expect(heirs.size).toBeGreaterThan(1);
    for (const e of divided) {
      // The share is the population, said out loud beside what was paid — so a reader can see the
      // basis rather than infer it (Law 2: an outcome, with its reason).
      expect(Number(e.data['members'])).toBeGreaterThan(0);
      expect(Number(e.data['of'])).toBeGreaterThanOrEqual(Number(e.data['members']));
    }
  });

  it('an estate is a declared process now, and the estate module keeps no programme of its own', () => {
    const w = ranWorld('inherit', 14);
    const opened = w.journal.ofKind('process.opened').filter((e) => e.data['what'] === 'estate');
    if (opened.length === 0) return;
    for (const e of opened) {
      expect(e.public).toBe(true);
      expect(e.data['steps']).toEqual(['selling', 'paying', 'dividing']);
      expect(Number(e.data['closesAfter'])).toBeGreaterThan(e.period);
    }
    // Law 19: anybody can ask what an estate is in the middle of, which a module bag could not say.
    for (const p of w.processes.all()) {
      expect(p.steps.length).toBeGreaterThan(0);
      expect(w.parties.has(p.subject)).toBe(true);
    }
  });
});
