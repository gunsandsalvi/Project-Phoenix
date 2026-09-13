/**
 * Audit B5: what a party is worth, read off the register and the marks, is what its equity account
 * says — in every money, every period, from the opening.
 *
 * @spec Audit B5 Audit B5.a Audit B5.b Currency C5 Currency D2 Currency D2.a Currency D3 Money A2.b Money D2 Seed C1 Law 4 Law 7 Law 19
 *
 * THE IDENTITY IS THE WHOLE POINT OF THE TWO SIDES BEING SEPARATE. The register and the price store
 * say what a party has; the equity account says what every event since it was born did to it; and
 * neither is computed from the other, so equality is a check and not a tautology. A world where they
 * agree because one is read off the other is a world with no check in it at all.
 *
 * WHAT BROKE IT was one missing conversion, and it broke where a conversion is easiest to forget: a
 * mark is in the money the instrument is PRICED in and an account is in the money its party KEEPS it
 * in, and for everybody holding only their own country's paper those are the same money. So the
 * defect was invisible at a rate of one and silent for as long as every rate stayed there — the seed
 * opens every pair at parity (`seed.openingRate`, Seed C4), and this world's FX market has yet to
 * print anything else.
 *
 * WHICH IS WHY THE SECOND WORLD BELOW OPENS OFF PARITY. A test that only ran the delivered world
 * would assert this identity against arithmetic that never converts anything, pass for ever, and
 * catch nothing — the guard has to be a world where the two moneys are different sizes, or it is
 * measuring `x * 1 === x`.
 */
import { describe, expect, it } from 'vitest';
import {
  assemble,
  drawBanks,
  drawFirms,
  foundationSpec,
  paramId,
  type AssemblySpec,
  type World,
} from '../src/index.js';
import { RIG_PER_LINE, rigMembers } from './rig.js';

const OPENING_RATE = paramId('seed.openingRate');
const MEMBERS = paramId('seed.households.membersPerCohort');

/** A YEAR, which is what the identity has to survive (Part XIII: the calendar is seven days). */
const AYEAR = 52;

/**
 * The rig's own world, with one number moved: what a money costs in another money at the opening.
 *
 * It is a SCALE MODEL like every other rig world — same modules, same seed, same laws, fewer of each
 * — and the one thing it states differently is a declared opening condition (Seed C4), not a rule.
 */
function worldAt(rate: number, seed: string, banks: number, firms: number): World {
  const spec: AssemblySpec = foundationSpec(seed, drawBanks(banks, seed), drawFirms(firms, seed, RIG_PER_LINE));
  const members = rigMembers(firms);
  return assemble({
    ...spec,
    modules: spec.modules.map((m) => ({
      ...m,
      params: m.params.map((p) => {
        if (p.id === OPENING_RATE) return { ...p, value: rate };
        return p.id === MEMBERS ? { ...p, value: members } : p;
      }),
    })),
  });
}

/** Every Audit B5 violation the family reported, over `periods` periods. */
function identityBreaks(w: World, periods: number): string[] {
  const out: string[] = [];
  for (let i = 0; i < periods; i += 1) {
    for (const f of w.step().audit.families) {
      for (const v of f.violations) {
        if (f.family === 'accounts' && v.spec === 'Audit B5') out.push(`p${v.period} ${v.message}`);
      }
    }
  }
  return out;
}

/**
 * What the world has to HAVE for this test to be a test: somebody holding a money that is not their
 * own, and two moneys of different sizes. Both are true by construction here and both have been
 * false at some point in this world's history — the cross holdings moved between kinds of holder
 * once already (worklist 11.5) — so the test says so rather than assuming it.
 */
function foreignPositions(w: World): number {
  let n = 0;
  for (const h of w.register.allHoldings()) {
    const inst = w.instruments.get(h.instrument);
    if (inst.ccy !== w.registry.currencyOf(w.parties.get(h.holder).region)) n += 1;
  }
  return n;
}

describe('assets minus liabilities is the equity account (Audit B5)', () => {
  it('holds for every party, every period of a year, in a world of four moneys', () => {
    const w = worldAt(1, 'identity', 4, 40);
    expect(foreignPositions(w), 'nobody held a foreign position, so nothing was tested').toBeGreaterThan(0);
    expect(identityBreaks(w, AYEAR)).toEqual([]);
  });

  it('holds when those moneys are not the same size, which is where it used to break', () => {
    // 12b: the mark on a foreign holding was booked to its holder's equity in the INSTRUMENT's
    // money, so the gap was `delta x (1 - rate)` — exactly nothing at parity, and a step the size
    // of the re-marking the moment a rate was not one. The opening equity had the same defect from
    // the other end: it was stated by a second copy of the balance sheet that summed `valueOfLots`
    // across four moneys without converting any of them (Money A2.b), so every central bank opened
    // contradicting its own sheet. Both are one cause and this is the world that shows it.
    const w = worldAt(0.8, 'identity', 4, 40);
    expect(foreignPositions(w)).toBeGreaterThan(0);
    expect(identityBreaks(w, AYEAR)).toEqual([]);
  });

  it('is stated at the opening by the same read the audit checks it against (Seed C1, Law 4)', () => {
    // Law 4, Reporting A2.a: there is ONE balance sheet — `balanceSheet` — and the seed, the audit
    // and a published report are three readers of it. When the seed had its own the two could
    // disagree, and at any rate but one they did. The check is that a world opens with the identity
    // already true, before a single event has moved anything.
    const w = worldAt(0.8, 'identity', 4, 40);
    const opening = w.last;
    expect(opening, 'a sealed world reports its period-zero audit').toBeDefined();
    if (opening === undefined) return;
    const broke = opening.audit.families
      .flatMap((f) => (f.family === 'accounts' ? f.violations : []))
      .filter((v) => v.spec === 'Audit B5');
    expect(broke).toEqual([]);
  });
});
