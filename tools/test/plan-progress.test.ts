import { describe, expect, it } from 'vitest';
import { crossCheck, itemProgress, worklistIds, worklistItems } from '../plan-progress.js';

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

  it('claims no steps for an item with no plan file, closed or open', () => {
    const unplanned = itemProgress().filter((i) => !i.present && i.steps === 0);
    // 10.1, 10.2 and 10.4 were worked and closed without one; 13k-13o were inserted from a sweep
    // with their reasoning in the worklist row and are open with no plan written yet.
    expect(unplanned.map((i) => i.id)).toEqual([
      '10.1',
      '10.2',
      '10.4',
      '13k',
      '13l',
      '13m',
      '13n',
      '13o',
    ]);
    // Zero steps moves neither side of the figure: it says there was no plan to complete, which is
    // what was true, rather than a count somebody invented to fill the column (Appendix C).
    expect(unplanned.every((i) => i.done === 0)).toBe(true);
  });

  it('reads the state from the worklist, not from whether the plan file is there', () => {
    // Item 14's plan file was folded into docs/AUDIT.md while the item was still open. A missing
    // file used to mean "fully done", so the figure would have claimed fourteen worked steps for an
    // item nobody has started. The worklist is the one writer of an item's state (Law 4, Law 19).
    const items = itemProgress();
    const polity = items.find((i) => i.id === '14');
    expect(polity?.present).toBe(false);
    expect(polity?.closed).toBe(false);
    expect(polity?.done).toBe(0);
    // And a closed item whose file is gone still counts everything it planned.
    const cross = items.find((i) => i.id === '13i');
    expect(cross?.present).toBe(false);
    expect(cross?.closed).toBe(true);
    expect(cross?.done).toBe(cross?.steps);
  });

  it('takes the state from the last cell, because one row has pipes in its prose', () => {
    const states = new Set(worklistItems().map((w) => w.state));
    expect([...states].filter((s) => s !== 'state' && s !== '-----').sort()).toEqual([
      'done',
      'open',
    ]);
  });
});
