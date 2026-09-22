/**
 * Read docs/COVERAGE.md. It never edits the file: re-marking is done by hand in the same change
 * that meets a requirement.
 */
import { readFileSync } from 'node:fs';

export interface CoverageRow {
  readonly id: string;
  readonly status: 'MET' | 'MISSING' | 'OUT OF SCOPE' | 'PARTIAL';
  readonly where: string;
}

const STATUSES = ['MET', 'MISSING', 'OUT OF SCOPE', 'PARTIAL'];

/**
 * A row as its three cells. Column padding is markdown's, not the row's, so `npm run format` must
 * be able to pad this table without the readers of it going blind.
 */
export function coverageCells(line: string): CoverageRow | undefined {
  const cells = line.split('|').slice(1, -1);
  if (cells.length < 3) return undefined;
  const id = /^\s*`([^`]+)`\s*$/.exec(cells[0] ?? '')?.[1];
  const status = (cells[1] ?? '').trim();
  if (id === undefined || !STATUSES.includes(status)) return undefined;
  return { id, status: status as CoverageRow['status'], where: (cells[2] ?? '').trim() };
}

export function readCoverage(path: string): CoverageRow[] {
  const out: CoverageRow[] = [];
  for (const line of readFileSync(path, 'utf8').split('\n')) {
    if (!line.trimStart().startsWith('|')) continue;
    const row = coverageCells(line);
    if (row !== undefined) out.push(row);
  }
  return out;
}

/**
 * Whether this text points at a number a reader cannot follow.
 *
 * The plan changes every week; the specification, the architecture, the coverage ledger and the
 * source do not. A pointer from a fixed file into a moving one is dead the week after it is
 * written, and a reader cannot tell a live item from one that closed and was deleted. A coverage
 * row says what the source does and what of the clause is short; a comment says why. Neither says
 * who will fix it.
 *
 * `VERIFICATION N.N` is the same defect with the moving file already gone: 731 rows cited a
 * numbering no document defines, so the reference could not be followed at all.
 */
export function namesAPlanItem(text: string): boolean {
  return /\b(worklist|item)s?\s+[0-9]/i.test(text) || /\bVERIFICATION\s+[0-9]/i.test(text);
}

/** Every line of `text` that points into the plan, numbered from one. */
export function planPointers(text: string): { line: number; says: string }[] {
  return text
    .split('\n')
    .map((says, at) => ({ line: at + 1, says: says.trim() }))
    .filter((l) => namesAPlanItem(l.says));
}
