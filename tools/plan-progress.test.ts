/**
 * What `plan:check` asserts about the plan, against FIXTURES rather than against today's plan.
 *
 * 21.119: the test this replaces encoded the state of the plan on the day it was written — it named
 * an item and asserted that item had steps — so it went red every time an item it named closed,
 * which is the opposite of what a test is for. What a test of this tool should assert is the RULE,
 * against a fixture. These do.
 *
 * Run by `npm run check:tools` (node's own runner, through tsx).
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { planSections, positionsIn } from './plan-progress.js';

test('a finding is its opening line AND the lines under it', () => {
  // 21.137: the sweep that found seventeen mis-placed findings missed two, because it read only the
  // line a finding opens on and both of those stated their position further down.
  const found = positionsIn(
    ['- [ ] 21.90 something measured on a world that is gone.', '  It is positioned at 0h.3.'].join(
      '\n',
    ),
  );
  assert.deepEqual(found, [{ finding: '21.90', target: '0h.3' }]);
});

test('the operative position is the LAST one stated, because a re-read appends', () => {
  // A finding keeps what it said and adds what it now says. The newest sentence is the live one.
  const found = positionsIn(
    [
      '- [ ] 21.50 the cost of borrowing abroad; positioned at 16.5, where the swap line is.',
      '  **Re-read: 16.5 closed without it. Re-positioned at 21j**, because nothing decides anything.',
    ].join('\n'),
  );
  assert.deepEqual(found, [{ finding: '21.50', target: '21j' }]);
});

test('a finding that states no position is not a position', () => {
  assert.deepEqual(positionsIn('- [ ] 21.1 a thing, with nowhere named.'), []);
});

test('a step counts from its section, and a declared closed count is read rather than inferred', () => {
  // The rule 21.119 asked for: an item with a section is counted FROM its section.
  const sections = planSections(
    [
      '## 21. The local repairs',
      '',
      '**Steps closed and deleted: 3**',
      '',
      '- [ ] 21.1 open.',
      '- [x] 21.2 ticked.',
      '',
      '## 22. Something else',
      '',
      '- [ ] 22.1 open.',
    ].join('\n'),
  );
  assert.equal(sections.get('21')?.checked, 4, 'three declared plus one ticked');
  assert.equal(sections.get('21')?.unchecked, 1);
  assert.equal(sections.get('22')?.unchecked, 1);
});

test('the declared closed-count line is not read as a step of the next item', () => {
  const sections = planSections(['## 9. An item', '', '**Steps closed and deleted: 2**'].join('\n'));
  assert.equal(sections.get('9')?.checked, 2);
  assert.equal(sections.get('9')?.unchecked, 0);
});
