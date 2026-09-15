import { describe, expect, it } from 'vitest';
import { readFileSync, mkdtempSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, join } from 'node:path';
import { tmpdir } from 'node:os';
import {
  existence,
  absent,
  builtAndDead,
  renderTable,
  marksOnNotes,
  unanswered,
} from '../coverage-existence.js';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const PLAN = resolve(root, 'docs', 'IMPLEMENTATION.md');

/** A two-system spec and its coverage, so the counting is asserted on something readable. */
function fixture(): { spec: string; coverage: string } {
  const dir = mkdtempSync(join(tmpdir(), 'existence-'));
  const spec = join(dir, 'spec.md');
  writeFileSync(
    spec,
    [
      '## 9. SHORT-TERM DEBT',
      '- **A1** REASON — it satisfies the bond contract.',
      '- **A2** REASON — its price is what it clears at.',
      '- **A3** VERIFY — the maturity profile is a read.',
      '  - **A3.a** a concentrated profile is a foreseeable wall.',
      '## 10. EQUITY',
      '- **A1** REASON — a share is a residual claim.',
      '- **A2** FORBID — no price from a multiple.',
      '',
    ].join('\n'),
  );
  const coverage = join(dir, 'coverage.md');
  writeFileSync(
    coverage,
    [
      '| requirement | status | where / why |',
      '|---|---|---|',
      '| `Short-Term Debt A1` | MISSING |  |',
      '| `Short-Term Debt A2` | MISSING |  |',
      '| `Short-Term Debt A3` | MISSING |  |',
      '| `Equity A1` | MET | packages/engine/src/mechanisms/equity/index.ts |',
      '| `Equity A2` | MET | packages/engine/src/mechanisms/equity/index.ts — **UNMEASURED**: the module is assembled and has never produced an outcome |',
      '',
    ].join('\n'),
  );
  return { spec, coverage };
}

describe('what exists, per spec system', () => {
  it('counts a clause under the system the SPEC puts it in, not the one COVERAGE heads it with', () => {
    // Law 4: which system a clause belongs to has one writer, and it is the specification.
    const { spec, coverage } = fixture();
    const rows = existence(coverage, spec);
    expect(rows.map((r) => r.system)).toEqual(['Short-Term Debt', 'Equity']);
    expect(rows[0]).toMatchObject({ met: 0, missing: 3, total: 3 });
    expect(rows[1]).toMatchObject({ met: 2, missing: 0, total: 2 });
  });

  it('counts a MET row that says UNMEASURED as MET, and separately', () => {
    // It IS implemented — that is what MET claims — and it has never produced an outcome. Two facts
    // about one row, and collapsing either into the other is what let 95 of them go unnoticed.
    const { spec, coverage } = fixture();
    const equity = existence(coverage, spec)[1];
    expect(equity?.met).toBe(2);
    expect(equity?.neverReached).toBe(1);
  });

  it('does not count a spec NOTE, and says which rows mark one', () => {
    // A3.a carries no REASON/VERIFY/FORBID word, so it is a continuation of A3 rather than its own
    // requirement. A count that swallowed it would disagree with the spec's own denominator.
    const { spec, coverage } = fixture();
    expect(existence(coverage, spec)[0]?.total).toBe(3);
    expect(marksOnNotes(coverage, spec)).toEqual([]);
  });

  it('names a system with no clause MET as an absent sector', () => {
    const { spec, coverage } = fixture();
    expect(absent(existence(coverage, spec)).map((r) => r.system)).toEqual(['Short-Term Debt']);
  });

  it('names a system whose every MET is UNMEASURED, which reads as built and is not', () => {
    // A reader scanning for "0 MET" walks past `M&A 10 MET` — and all ten are marks on a module
    // that has never produced an outcome. Same absence, and it needs saying separately.
    const { spec, coverage } = fixture();
    expect(builtAndDead(existence(coverage, spec))).toEqual([]);
    const real = builtAndDead(existence());
    expect(real.every((r) => r.met === r.neverReached && r.met > 0)).toBe(true);
  });
});

describe('the plan says what COVERAGE says', () => {
  it('every absent sector is named in the plan', () => {
    // The check this project did not have. Five sectors were 0 of 19 to 0 of 32 while the ordered
    // list of open work carried them as one row of one finding, under an item marked done.
    const plan = readFileSync(PLAN, 'utf8');
    for (const r of absent(existence())) {
      expect(plan, `${r.system} is an absent sector and the plan does not name it`).toContain(
        r.system,
      );
    }
  });

  it("Part 0's table is the one the tool generates", () => {
    const plan = readFileSync(PLAN, 'utf8');
    const bare = (t: string): string[] =>
      t
        .split('\n')
        .filter((l) => l.startsWith('| ') && !l.startsWith('|---'))
        .map((l) => l.replace(/\*\*/g, '').trim());
    const mine = bare(renderTable(existence()));
    const head = mine[0];
    expect(head).toBeDefined();
    const at = plan.split('\n').findIndex((l) => l.replace(/\*\*/g, '').trim() === head);
    expect(at, 'docs/IMPLEMENTATION.md Part 0 has no existence table').toBeGreaterThan(-1);
    const after = plan.split('\n').slice(at);
    const end = after.findIndex((l, i) => i > 0 && !l.startsWith('|'));
    expect(bare(after.slice(0, end < 0 ? after.length : end).join('\n'))).toEqual(mine);
  });

  it('every spec clause has a row, so none was deleted to look better', () => {
    expect(unanswered()).toEqual([]);
  });
});
