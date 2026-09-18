/**
 * READ docs/COVERAGE.md (Appendix C: recount rather than adjust any tally).
 *
 * It never edits the file: re-marking is done by hand in the same change that meets a requirement.
 *
 * **The half of this that scanned the engine is gone with the engine** (0g.45). Counting `@spec`
 * citations in the source is now `tools/phoenix-check`'s, which does it for Rust and refuses a module
 * that cites nothing. What is left here is the half that reads the DOC, which is what
 * `coverage-existence` and `plan-progress` ask for.
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
 * PLAN §5: "no PARTIAL row without a named item".
 *
 * A PARTIAL row is a promise that the rest of a clause is coming. A promise with nobody to keep it
 * is a MISSING row wearing a better word, and seven of them had accumulated — two of them about the
 * currency layer, which was the very next item. So a row says which item finishes it, and this
 * reports the ones that do not. A named item is a worklist id (`worklist 12`, `item 13g`) or the
 * Part whose programme owns it (`Part XII`); a Part XI mechanism counts too, because the mechanism
 * names the item that builds it.
 */
export function unattributedPartials(rows: readonly CoverageRow[]): CoverageRow[] {
  return rows.filter((r) => r.status === 'PARTIAL' && !namesAnItem(r.where));
}

/**
 * Whether this reason names something that will finish the clause.
 *
 * A BARE NUMBER IS NOT ACCEPTED. "12" is a tenor, a count of periods and half the clause ids in the
 * file, and a rule that took it would pass rows that name nobody — which is the whole defect. What
 * counts is a form that can only be an item: the word beside it (`worklist 12`, `item 13g`), an id
 * that carries a letter or a point (`13h`, `4a`, `10.3`, `pre12`), the Part whose programme owns it,
 * or a Part XI mechanism, which names the item that builds it.
 */
function namesAnItem(where: string): boolean {
  return (
    /\b(worklist|item)s?\s*[0-9]/i.test(where) ||
    /\bPart\s+XI{1,2}\b/.test(where) ||
    /\bXI-[0-9]/.test(where) ||
    /\b[0-9]{1,2}[a-i]\b/.test(where) ||
    /\b[0-9]{1,2}\.[0-9]\b/.test(where) ||
    /\bpre[0-9]/i.test(where)
  );
}
