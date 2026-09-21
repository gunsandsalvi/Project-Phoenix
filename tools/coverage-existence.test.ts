/**
 * What `check:existence` asserts about a citation, against FIXTURES rather than against today's
 * COVERAGE (21.119: a test that encodes the state of a document goes red when the document is
 * corrected, which is the opposite of what a test is for).
 *
 * Run by `npm run check:tools` (node's own runner, through tsx).
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { citedPaths, deadCitations } from './coverage-existence.js';
import { planPointers } from './spec-coverage.js';

test('a path is read out of the prose around it, without the sentence punctuation', () => {
  // Rows are written as prose, so a path is followed by whatever comes next: a comma, a bracket,
  // a full stop. None of those is part of the path.
  assert.deepEqual(citedPaths('packages/kernel-rs/src/ledger.rs, and tools/plan-gaps.ts.'), [
    'packages/kernel-rs/src/ledger.rs',
    'tools/plan-gaps.ts',
  ]);
  assert.deepEqual(citedPaths('it is refused at packages/kernel-rs/src/audit.rs)'), [
    'packages/kernel-rs/src/audit.rs',
  ]);
});

test('docs/ is not a citation: a row cites the SOURCE that implements the clause', () => {
  assert.deepEqual(citedPaths('see docs/ARCHITECTURE.md 6.1 and docs/spec/PROJECT_PHOENIX.md'), []);
});

test('a row with no path at all cites nothing, which is not a dead citation', () => {
  assert.deepEqual(citedPaths('nothing implements this; the mechanism is missing'), []);
});

/** A COVERAGE file with the rows given, and a tree with the files given. */
function fixture(rows: readonly string[], files: readonly string[]): [string, string] {
  const at = mkdtempSync(join(tmpdir(), 'phoenix-citations-'));
  for (const f of files) {
    mkdirSync(join(at, f.slice(0, f.lastIndexOf('/'))), { recursive: true });
    writeFileSync(join(at, f), '');
  }
  const path = join(at, 'COVERAGE.md');
  writeFileSync(path, ['| requirement | status | where / why |', '|---|---|---|', ...rows].join('\n'));
  return [path, at];
}

test('a citation that resolves is not reported, whatever the row says', () => {
  const [path, at] = fixture(
    ['| `Money A1` | MET | packages/kernel-rs/src/instruments.rs asserts an issuer |'],
    ['packages/kernel-rs/src/instruments.rs'],
  );
  assert.deepEqual(deadCitations(path, at), []);
});

test('a citation that does not resolve fails, and names the row and the path', () => {
  // The defect this exists for: 1,121 rows citing `packages/engine`, which the Rust port deleted,
  // and nothing anywhere went red.
  const [path, at] = fixture(
    [
      '| `Money A1` | MET | packages/engine/src/registry/profiles.ts |',
      '| `Money A2` | MET | packages/kernel-rs/src/instruments.rs |',
    ],
    ['packages/kernel-rs/src/instruments.rs'],
  );
  assert.deepEqual(deadCitations(path, at), [
    { id: 'Money A1', status: 'MET', path: 'packages/engine/src/registry/profiles.ts' },
  ]);
});

test('it is not only MET that has to resolve — a PARTIAL cites the half that IS there', () => {
  const [path, at] = fixture(
    ['| `Register B3` | PARTIAL | packages/kernel-rs/src/gone.rs reads the liability |'],
    ['packages/kernel-rs/src/instruments.rs'],
  );
  assert.deepEqual(deadCitations(path, at).map((d) => d.status), ['PARTIAL']);
});

test('a line that points into the plan is reported, with the line it is on', () => {
  const text = [
    'A row says what the source does.',
    'The other half arrives with item 0r.',
    'And a comment says why.',
    'Implementation items 1.2 and 1.5 own the rest.',
  ].join('\n');
  assert.deepEqual(
    planPointers(text).map((p) => p.line),
    [2, 4],
  );
});

test('a clause id, a tenor and a count of weeks are not pointers into the plan', () => {
  // The plan is named by the word beside the number. Everything else here is a number this
  // world already uses for something, and a rule that took one would report every row.
  const text = [
    'N5.b has no representation and there are 12 of them.',
    'It arrives at 13h, or at XI-7, or in Part XII.',
    'Money G2.c says the population changes before anybody acts.',
    'The paper runs 13 weeks.',
  ].join('\n');
  assert.deepEqual(planPointers(text), []);
});
