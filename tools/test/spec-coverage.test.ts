import { describe, expect, it } from 'vitest';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { readCoverage, unattributedPartials, type CoverageRow } from '../spec-coverage.js';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

function row(over: Partial<CoverageRow>): CoverageRow {
  return { id: 'Money A1', status: 'PARTIAL', where: 'not built', ...over };
}

describe('a PARTIAL row names the item that finishes it (PLAN §5)', () => {
  it('reports a row that names nobody', () => {
    expect(unattributedPartials([row({ where: 'family declared and not built' })])).toHaveLength(1);
  });

  it('takes a worklist id, an item, a Part or a Part XI mechanism', () => {
    const named = [
      row({ where: 'arrives with the currency layer (worklist 12)' }),
      row({ where: 'the driver arrives at item 13g' }),
      row({ where: 'measured by the programme (Part XII)' }),
      row({ where: 'default-into-recovery arrives with XI-1' }),
      row({ where: "C2.e's insiders are 13g" }),
      row({ where: 'the odd piece is 10.3' }),
    ];
    expect(unattributedPartials(named)).toEqual([]);
  });

  it('does not take a bare number for an item', () => {
    // "12" is a tenor, a count of periods and half the clause ids in the file. A rule that took it
    // would pass the rows this guard exists to catch.
    expect(
      unattributedPartials([row({ where: 'holdings may still have 12 periods to run' })]),
    ).toHaveLength(1);
  });

  it('says nothing about MET, MISSING or OUT OF SCOPE rows', () => {
    // MISSING is work nobody has promised yet, which is honest; the rule is about the promise.
    expect(
      unattributedPartials([
        row({ status: 'MISSING', where: '' }),
        row({ status: 'MET', where: 'packages/engine/src/core/money.ts' }),
        row({ status: 'OUT OF SCOPE', where: 'stated reason, clause kept' }),
      ]),
    ).toEqual([]);
  });

  it('finds none in the tree as it stands', () => {
    const rows = readCoverage(resolve(root, 'docs', 'COVERAGE.md'));
    expect(rows.filter((r) => r.status === 'PARTIAL').length).toBeGreaterThan(0);
    expect(unattributedPartials(rows).map((r) => r.id)).toEqual([]);
  });
});
