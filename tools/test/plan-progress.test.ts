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
    expect([...states].filter((s) => s !== 'state' && s !== '-----').sort()).toEqual([
      'done',
      'open',
    ]);
  });
});
