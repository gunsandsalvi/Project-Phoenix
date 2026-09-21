import { test } from 'node:test';
import assert from 'node:assert/strict';
import { gapsIn, render } from './plan-gaps.js';
import type { Requirement } from './spec-index.js';

const requirements: Requirement[] = [
  { id: 'Money A1', system: 'Money', node: 'A1', form: 'REASON', line: 10, text: '' },
  { id: 'Money A2', system: 'Money', node: 'A2', form: 'VERIFY', line: 11, text: '' },
];

test('every unmet clause becomes one uniquely owned to-do point', () => {
  const coverage = [
    '| `Money A1` | MISSING | no implementation |',
    '| `Money A2` | PARTIAL | the production caller is missing (item 1) |',
  ].join('\n');
  const output = render(gapsIn(coverage, requirements), ['Money']);

  assert.match(output, /TODO 1\.MONEY\.A1.*`Money A1` MISSING/);
  assert.match(output, /TODO 1\.MONEY\.A2.*`Money A2` PARTIAL/);
  assert.equal(output.match(/\*\*TODO /g)?.length, 2);
});

test('every implementation block requires the specification and production code review', () => {
  const coverage = '| `Money A1` | MISSING | no implementation |';
  const output = render(gapsIn(coverage, requirements), ['Money']);

  assert.match(output, /Required review before this block/);
  assert.match(output, /docs\/spec\/PROJECT_PHOENIX\.md/);
  assert.match(output, /packages\/kernel-rs\/src\/mechanisms\/money\.rs/);
  assert.match(output, /packages\/kernel-rs\/src\/systems\.rs/);
});
