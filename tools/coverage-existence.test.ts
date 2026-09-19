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
import { readCoverage, unattributedPartials } from './spec-coverage.js';

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

// 0j.7: PLAN §5, wired into the check at last. The rule was written, documented and never called,
// so it held for exactly as long as somebody remembered it.

test('a PARTIAL that names no item is reported, and one that names an item is not', () => {
  const [path] = fixture(
    [
      '| `Money B3` | PARTIAL | the credit decision is there; the overdraft is not |',
      '| `Money B4` | PARTIAL | the other half arrives with item 0r |',
    ],
    [],
  );
  assert.deepEqual(unattributedPartials(readCoverage(path)).map((r) => r.id), ['Money B3']);
});

test('only a PARTIAL is asked — a MISSING row promises nothing and owes no item', () => {
  // A MISSING says the clause is not built. That is an answer, not a promise, so nothing is owed.
  const [path] = fixture(
    [
      '| `Money B3` | MISSING | nothing builds it |',
      '| `Money B4` | MET | packages/kernel-rs/src/ledger.rs |',
    ],
    [],
  );
  assert.deepEqual(unattributedPartials(readCoverage(path)), []);
});

test('a bare number is not an item, because half the clause ids in the file are one', () => {
  // The whole defect is a row that names nobody, and a rule taking "12" would pass one.
  const [path] = fixture(
    [
      '| `Bond N5` | PARTIAL | N5.b has no representation and there are 12 of them |',
      '| `Bond N6` | PARTIAL | it arrives at 13h |',
      '| `Bond N7` | PARTIAL | XI-7 builds the fixing |',
    ],
    [],
  );
  assert.deepEqual(unattributedPartials(readCoverage(path)).map((r) => r.id), ['Bond N5']);
});
