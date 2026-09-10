import { describe, expect, it } from 'vitest';
import { crossCheck, itemProgress, worklistIds } from '../plan-progress.js';

describe('the manifest and the worklist name the same items (Law 10)', () => {
  it('refuses a worklist item the manifest never got', () => {
    // What actually happened: 10.1, 10.2 and 10.4 were inserted, worked and closed, and the
    // progress figure counted twenty-nine items where the worklist had thirty-two.
    expect(() => {
      crossCheck(['1', '2'], ['1', '10.1', '2']);
    }).toThrow(/WORKLIST.*10\.1/s);
  });

  it('refuses a manifest item that is on no list', () => {
    // The other direction is worse: an item nobody can take, contributing to a figure about work
    // that was never ordered (Law 10 — one ordered list).
    expect(() => {
      crossCheck(['1', '2', 'ghost'], ['1', '2']);
    }).toThrow(/manifest.*ghost/s);
  });

  it('passes when they agree, whatever the order', () => {
    expect(() => {
      crossCheck(['2', '1'], ['1', '2']);
    }).not.toThrow();
  });
});

describe('the tree as it stands', () => {
  it('has a manifest row for every worklist item and no other', () => {
    expect(() => itemProgress()).not.toThrow();
  });

  it('reads the worklist rather than restating it (Law 19)', () => {
    const ids = worklistIds();
    expect(ids[0]).toBe('0');
    expect(ids).toContain('pre12');
    expect(ids).toContain('10.4');
    // The header row is not an item.
    expect(ids).not.toContain('item');
  });

  it('counts an item worked without a plan file as closed, and claims no steps for it', () => {
    const unplanned = itemProgress().filter((i) => !i.present && i.steps === 0);
    expect(unplanned.map((i) => i.id)).toEqual(['10.1', '10.2', '10.4']);
    // Zero steps moves neither side of the figure: it says there was no plan to complete, which is
    // what was true, rather than a count somebody invented to fill the column (Appendix C).
    expect(unplanned.every((i) => i.done === 0)).toBe(true);
  });
});
