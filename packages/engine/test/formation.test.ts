/**
 * Households form (Households F1, F1.b, 12.2): formation is ENTRY, dated, with a cause — and never a rate.
 *
 * @spec Households F1 Households F1.b Households F4 XI-15 Law 2 Law 6
 */
import { describe, expect, it } from 'vitest';
import { HOUSEHOLD } from '../src/index.js';
import { rigWorld } from './rig.js';

describe('a household forms when its people can pay for a roof, and waits when they cannot (F1.b, 12.2)', () => {
  it('every period, the people at the boundary either enter or are told why not, on the record', () => {
    const w = rigWorld('hh-form');
    const first = String(w.registry.cohorts[0]?.id);
    let waitingBefore = 0;
    let formed = 0;
    for (let i = 0; i < 8; i += 1) {
      w.step();
      const events = w.journal
        .ofKindIn('households.lifecycle', w.period)
        .filter((e) => e.data['event'] === 'formed' || e.data['event'] === 'notFormed');
      // No rate: what stands at the boundary is the band's own geometry, whole people, carried.
      let waiting = 0;
      for (const e of events) {
        const cell = w.parties.get(e.subjects[0] as never);
        expect(cell.kind).toBe(HOUSEHOLD);
        expect(cell.representation === 'cell' && cell.key['cohort']).toBe(first);
        const members = e.data['members'];
        expect(typeof members === 'number' && Number.isInteger(members) && members > 0).toBe(true);
        if (e.data['event'] === 'formed') {
          formed += members as number;
          // XI-15: an entry event of exactly those members, on that cell, with its cause.
          const entered = w.journal
            .ofKindIn('weight', w.period)
            .some((x) => x.data['kind'] === 'entry' && x.data['members'] === members && x.data['cause'] === 'formed' && x.subjects[0] === e.subjects[0]);
          expect(entered).toBe(true);
        } else {
          expect(typeof e.data['why']).toBe('string');
          waiting += members as number;
        }
      }
      // Whoever did not form is still standing there next period: nobody is deleted (Law 6, XI-15).
      if (formed === 0) expect(waiting).toBeGreaterThanOrEqual(waitingBefore);
      waitingBefore = waiting;
    }
    expect(formed + waitingBefore).toBeGreaterThan(0);
  });
});
