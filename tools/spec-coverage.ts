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

export function readCoverage(path: string): CoverageRow[] {
  const text = readFileSync(path, 'utf8');
  const row = /^\| `([^`]+)` \| (MET|MISSING|OUT OF SCOPE|PARTIAL) \| ([^|]*) \|/gm;
  const out: CoverageRow[] = [];
  let m: RegExpExecArray | null;
  while ((m = row.exec(text)) !== null) {
    if (m[1] !== undefined && m[2] !== undefined && m[3] !== undefined) {
      out.push({ id: m[1], status: m[2] as CoverageRow['status'], where: m[3].trim() });
    }
  }
  return out;
}

/**
 * Whether this text points into the implementation plan.
 *
 * The plan changes every week; the specification, the architecture, the coverage ledger and the
 * source do not. A pointer from a fixed file into a moving one is dead the week after it is
 * written, and a reader cannot tell a live item from one that closed and was deleted. A coverage
 * row says what the source does and what of the clause is short; a comment says why. Neither says
 * who will fix it.
 */
export function namesAPlanItem(text: string): boolean {
  return /\b(worklist|item)s?\s+[0-9]/i.test(text);
}

/** Every line of `text` that points into the plan, numbered from one. */
export function planPointers(text: string): { line: number; says: string }[] {
  return text
    .split('\n')
    .map((says, at) => ({ line: at + 1, says: says.trim() }))
    .filter((l) => namesAPlanItem(l.says));
}
