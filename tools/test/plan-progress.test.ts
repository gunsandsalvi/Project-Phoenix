import { describe, expect, it } from 'vitest';
import { itemProgress, planSections, worklistIds, worklistItems } from '../plan-progress.js';

describe('steps are read from the plan sections (Law 19)', () => {
  it('counts ticked and unticked steps under an item heading and stops at the next heading', () => {
    const s = planSections(
      [
        '## 0. The world opens',
        '- [x] 0.1 a',
        '- [ ] 0.2 b',
        '## Part 3 — The index',
        '- [ ] not a step',
      ].join('\n'),
    );
    expect(s.get('0')).toEqual({ title: 'The world opens', checked: 1, unchecked: 1 });
    expect(s.size).toBe(1);
  });

  /**
   * An item that closes some steps and STAYS OPEN has no ticked lines left once they are deleted
   * (CLAUDE.md: tick a step, delete it when it closes; the record is the ledger). 0g had closed
   * eight of fifteen and the generated table read "7 steps, 0 done, open" — a figure that lies. The
   * plan declares the count and this reads it.
   */
  it('reads a declared count of steps that closed and were deleted', () => {
    const s = planSections(
      [
        '## 0g. The core made fast',
        '**Steps closed and deleted: 8** — their outcomes are in the record.',
        '- [ ] 0g.17 the instrument',
        '- [ ] 0g.18 the phase block',
      ].join('\n'),
    );
    expect(s.get('0g')).toEqual({ title: 'The core made fast', checked: 8, unchecked: 2 });
  });

  it('does not count a declared line as a step of the next item', () => {
    const s = planSections(
      ['## 1. One', '- [ ] 1.1 a', '## 2. Two', '**Steps closed and deleted: 3**', '- [ ] 2.1 b'].join('\n'),
    );
    expect(s.get('1')).toEqual({ title: 'One', checked: 0, unchecked: 1 });
    expect(s.get('2')).toEqual({ title: 'Two', checked: 3, unchecked: 1 });
  });

  it('accepts inserted ids with a suffix or a dot', () => {
    const s = planSections(
      ['## 0a. Phases', '- [ ] x', '## 12a. Households', '- [ ] y', '- [ ] z'].join('\n'),
    );
    expect([...s.keys()]).toEqual(['0a', '12a']);
    expect(s.get('12a')?.unchecked).toBe(2);
  });
});

describe('the tree as it stands', () => {
  it('reads the worklist rather than restating it', () => {
    const ids = worklistIds();
    expect(ids[0]).toBe('0');
    expect(ids).toContain('pre12');
    expect(ids).not.toContain('item');
  });

  it('reads the state from the worklist, never from whether a section exists', () => {
    const items = itemProgress();
    for (const i of items) {
      if (i.closed) expect(i.done).toBe(i.steps);
      if (!i.present) expect(i.steps).toBe(0);
    }
  });

  it('takes the state from the last cell, because one row has pipes in its prose', () => {
    const states = new Set(worklistItems().map((w) => w.state));
    const said = [...states].filter((s) => s !== 'state' && s !== '-----');
    // The worklist is history now (item 0c): a row is `done`, or it says which plan item carries
    // it. Nothing is worked from this file, so nothing on it is `open`.
    expect(said).toContain('done');
    expect(said.filter((s) => s !== 'done').every((s) => s.startsWith('moved to plan '))).toBe(true);
  });

  it('counts the PLAN items, which have no worklist row at all (item 0c)', () => {
    const items = itemProgress();
    const byId = new Map(items.map((i) => [i.id, i]));
    // 0a to 24 live in docs/IMPLEMENTATION.md alone. Counting the worklist's rows made every step
    // of every one of them invisible: ticking all five of 0a moved the figure by nothing.
    const zeroA = byId.get('0a');
    expect(zeroA?.present).toBe(true);
    expect(zeroA?.steps).toBeGreaterThan(0);
    expect(zeroA?.done).toBe(zeroA?.steps);
    expect(zeroA?.closed).toBe(true);
    // And an item the worklist closed keeps counting every step it had, section or no section.
    expect(items.filter((i) => i.closed && !i.present).length).toBeGreaterThan(0);
  });

  it('never takes a plan item state from a worklist row of the same id', () => {
    // The two files used one id for two items: this list's `14` was the polity and the plan's is
    // the insurers. The worklist speaks only through `done`.
    const items = itemProgress();
    const moved = worklistItems().filter((w) => w.state.startsWith('moved to plan '));
    expect(moved.length).toBeGreaterThan(0);
    for (const w of moved) {
      const asPlan = items.find((i) => i.id === w.id && i.present);
      if (asPlan === undefined) continue;
      expect(asPlan.state, `${w.id} took its state from a moved worklist row`).not.toContain(
        'moved',
      );
    }
  });
});
