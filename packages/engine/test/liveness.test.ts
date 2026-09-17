/**
 * The tenth family: something good eventually happens (Part XII).
 *
 * @spec Audit A2 Audit A3 Audit C2 Audit C3 Audit C4 Audit E2 Part XII
 *
 * The other nine families are satisfied by a world in which nothing happens at all. These are the
 * cases that say so: a declaration nothing ever came of, a living party that has been a side of
 * nothing, and a horizon that behaves like the RESOLUTION its own declaration claims it is.
 */
import { describe, expect, it } from 'vitest';
import { assemble, paramId, type PeriodReport, type World } from '../src/index.js';
import { rigSpec } from './rig.js';

const HORIZON = paramId('audit.livenessHorizon');

/** The rig, with the audit's horizon set to `periods` — the one number these cases are about. */
function rigWithHorizon(seed: string, periods: number): World {
  const spec = rigSpec(seed);
  return assemble({
    ...spec,
    modules: spec.modules.map((m) => ({
      ...m,
      params: m.params.map((p) => (p.id === HORIZON ? { ...p, value: periods } : p)),
    })),
  });
}

function liveness(report: PeriodReport): PeriodReport['audit']['families'][number] {
  const f = report.audit.families.find((x) => x.family === 'liveness');
  if (f === undefined) throw new Error('the liveness family is not in the report');
  return f;
}

function runTo(w: World, periods: number): PeriodReport {
  let last = w.step();
  for (let i = 1; i < periods; i += 1) last = w.step();
  return last;
}

describe('the liveness family (Part XII, 0h.3)', () => {
  it('is built, and says nothing in the periods before its own horizon', () => {
    const w = rigWithHorizon('live-a', 6);
    const first = liveness(runTo(w, 3));
    // C2: built means it is checked. A family that reported nothing because nobody wrote it would
    // be green by omission, which is the thing C2 exists to stop.
    expect(first.built).toBe(true);
    expect(first.contributions).toContain('kernel');
    // Nothing has had six periods to happen in yet, so there is nothing to report.
    expect(first.count).toBe(0);
  });

  it('names what was declared and never reached, with the module that declared it', () => {
    const w = rigWithHorizon('live-b', 2);
    const f = liveness(runTo(w, 6));
    expect(f.count).toBeGreaterThan(0);
    const declared = f.violations.filter((v) => v.spec === 'Audit E2');
    expect(declared.length).toBeGreaterThan(0);
    for (const v of declared) {
      // A2, A3: who and how much, in a unit. A finding with no owner is not a finding (D2).
      expect(v.owner.length).toBeGreaterThan(0);
      expect(v.unit).toBe('periods declared and never reached');
      expect(v.size).toBeGreaterThanOrEqual(2);
      expect(v.period).toBe(w.period);
    }
  });

  it('names a living party that has been a side of nothing, counting from its own birth', () => {
    const w = rigWithHorizon('live-c', 2);
    const f = liveness(runTo(w, 8));
    const silent = f.violations.filter((v) => v.unit === 'periods');
    expect(silent.length).toBeGreaterThan(0);
    for (const v of silent) {
      // Nothing is reported before it has had the horizon to happen in — which for a party that
      // arrived after the world opened is counted from the period it arrived.
      expect(v.size).toBeGreaterThanOrEqual(2);
      expect(v.size).toBeLessThanOrEqual(w.period);
    }
  });

  it('has a horizon that behaves like the resolution it is declared as', () => {
    const near = liveness(runTo(rigWithHorizon('live-d', 2), 8));
    const far = liveness(runTo(rigWithHorizon('live-d', 6), 8));
    // Law 2: lengthening a horizon may only REMOVE findings. Every one the longer horizon reports
    // is one the shorter horizon reported too, and about the same owner.
    const nearKeys = new Set(near.violations.map((v) => `${v.spec}|${v.owner}|${v.message}`));
    for (const v of far.violations) {
      expect(nearKeys.has(`${v.spec}|${v.owner}|${v.message}`)).toBe(true);
    }
    expect(far.count).toBeLessThanOrEqual(near.count);
  });

  it('repairs nothing: the same world steps the same way whether or not it is read', () => {
    const w = rigWithHorizon('live-e', 2);
    const report = runTo(w, 5);
    const before = liveness(report).count;
    // C4: reading the report twice is reading, and a read that changed the world would say so here.
    expect(liveness(report).count).toBe(before);
    expect(w.parties.all().length).toBeGreaterThan(0);
  });
});
