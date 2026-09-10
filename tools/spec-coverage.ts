/**
 * Recount requirement coverage (Appendix C: recount rather than adjust any tally).
 *
 * Reads every `@spec` tag in the engine and docs/COVERAGE.md, and prints, per system, how many
 * requirements are cited by code, marked MET, MISSING or OUT OF SCOPE. It never edits COVERAGE.md:
 * re-marking is done by hand in the same change that meets a requirement.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { listTs, parseCitations } from './check-citations.js';
import { buildSpecIndex } from './spec-index.js';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');

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

export function citedByCode(): Set<string> {
  const cited = new Set<string>();
  for (const f of listTs(resolve(root, 'packages', 'engine', 'src'))) {
    const text = readFileSync(f, 'utf8');
    const re = /@spec\s+([^\n*]+)/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(text)) !== null) {
      const tag = m[1];
      if (tag !== undefined) for (const c of parseCitations(tag)) cited.add(c);
    }
  }
  return cited;
}

/**
 * Requirements a module cites but only partly meets, with what is missing. Kept here so the initial
 * COVERAGE.md is reproducible; after that the file is re-marked by hand (Appendix C).
 */
const PARTIAL: Readonly<Record<string, string>> = {
  'Treasury B3':
    'outlays vary with the standing mandate; the cycle and unemployment arrive with the real economy (worklist 4) and policy with the polity (worklist 14)',
  'Treasury C2':
    'receipts follow what was actually collected; income and consumption bases arrive with the real economy (worklist 4)',
  'Sovereign D6':
    'the bid-offer is whatever the schedules produce; there are no dealers to produce one until worklist 9',
  'Sovereign E2':
    'banks hold for the liquidity buffer and the central bank for policy; the other holder classes arrive with their own systems (worklist 4, 8, 12)',
  'Central Bank A3':
    'the mandate exists as a stated objective in the register; parliament owns its text and its target from worklist 14',
  'Central Bank A4':
    'financially owned by the treasury through remittance; operational independence has no rate to be independent about until the corridor (worklist 11)',
  'Central Bank E5':
    'the two statements are both true of the books; measuring them is the measurement programme (worklist 16)',
  'Money B3.b':
    'reserve overdraft is allowed and recorded, not yet priced by the corridor (worklist 11)',
  'Money B3.c':
    'refusal is recorded; a lender row for an allowed overdraft arrives with the corridor (worklist 11)',
  'Money B3.a':
    'the bank refuses every customer overdraft until the credit decision exists (worklist 6)',
  'Money E1':
    'a fail is recorded; the default state and its downstream consequences arrive with XI-1 (worklist 5)',
  'Money E1.b':
    "the payee's missing receivable is the failed record; a receivable row arrives with Trade Credit",
  'Money E4':
    'a ceased party is refused by name; re-seating on the estate arrives with XI-8 (worklist 7)',
  'Register D5.b': 'liens name a beneficiary; a re-pledging chain arrives with Securities Lending',
  'Register E3': 'a default converts nothing yet; XI-1 (worklist 5)',
  'Register E4':
    'split, buyback and new issue apply through issuance legs; no corporate-action driver yet',
  'Register E5':
    'every register event so far moves money; no explicit why-not record for the exceptions',
  'Register B4': 'maturity ceases an instrument; default-into-recovery arrives with XI-1',
  'Clearing B3': 'no dealer exists yet (worklist 9)',
  'Clearing C4.a':
    'a failed clearing prints stale; consequences to issuers and rollers arrive with their systems',
  'Clearing E3': 'no dealer schedules yet (worklist 9)',
  'Clearing F3': 'to be measured once markets read period state (Part XII)',
  'Audit B4': 'family declared and reported as not built',
  'Audit B8': 'independence is measured once a defect can light families (Part XII)',
  'Audit D4': 'run-length comparison is a Part XII measurement',
  'Seed B1':
    'the foundation seed has one instance of several kinds; populations are cells with weights',
  'Seed B4': 'sizes are dispersed by hand in the foundation seed; nothing draws them',
  'Seed D1':
    'coupons are payable from the treasury account; wages and work in progress arrive with worklist 4',
  'Seed E2': 'no reasons exist yet; the seed sets endowments only',
  'Currency A5': 'one currency in the foundation registry; several are data',
  'Currency D3':
    'ordering enforced for marks; foreign positions cannot exist until the currency layer',
  'Observer A5': 'no published aggregates with a lag yet',
  'Observer A3':
    'public state is instrument terms and prints; issuer publications arrive with firms',
  'Bond N11': 'the sovereign answers none; the corporate regime arrives with Corporate Credit',
  'XI-15':
    'cells, weights, split and merge exist; promotion has no cause yet, resolution not yet measured',
  'XI-5': 'DvP and fails are structural; basis is recorded; the no-print rider holds',
  'XI-6':
    'value is a function; carried-at-cost is declared per kind; inventory, plant and dwellings have no unit yet',
  'XI-14': 'every number is declared with its kind and owner; the placeholders that remain name the mechanism and the item that deletes them',
};

function initialCoverage(): string {
  const idx = buildSpecIndex();
  const cited = citedByCode();
  const files = new Map<string, string[]>();
  for (const f of listTs(resolve(root, 'packages', 'engine', 'src'))) {
    const text = readFileSync(f, 'utf8');
    const re = /@spec\s+([^\n*]+)/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(text)) !== null) {
      const tag = m[1];
      if (tag === undefined) continue;
      for (const c of parseCitations(tag)) {
        const list = files.get(c) ?? [];
        list.push(f.replace(`${root}/`, ''));
        files.set(c, list);
      }
    }
  }
  const lines: string[] = [];
  lines.push('# Requirement coverage');
  lines.push('');
  lines.push(
    'One row per REASON, VERIFY and FORBID in the specification. `MET at <path>` means the cited module',
  );
  lines.push(
    'implements the clause; `PARTIAL` says what is still missing; `MISSING` is work; `OUT OF SCOPE` states a',
  );
  lines.push(
    'reason and keeps the clause (Appendix C). Re-mark in the same change that meets a requirement, and',
  );
  lines.push('recount with `npm run coverage:spec` rather than adjusting a tally.');
  lines.push('');
  let system = '';
  for (const r of idx.requirements) {
    if (r.form === 'NOTE') continue;
    if (r.system !== system) {
      system = r.system;
      lines.push('');
      lines.push(`## ${system}`);
      lines.push('');
      lines.push('| requirement | status | where / why |');
      lines.push('|---|---|---|');
    }
    const partial = PARTIAL[r.id];
    if (partial !== undefined) lines.push(`| \`${r.id}\` | PARTIAL | ${partial} |`);
    else if (cited.has(r.id))
      lines.push(`| \`${r.id}\` | MET | ${[...new Set(files.get(r.id) ?? [])].join(', ')} |`);
    else lines.push(`| \`${r.id}\` | MISSING |  |`);
  }
  lines.push('');
  return lines.join('\n');
}

if (
  process.argv[1] !== undefined &&
  fileURLToPath(import.meta.url) === resolve(process.argv[1]) &&
  process.argv.includes('--init')
) {
  const out = resolve(root, 'docs', 'COVERAGE.md');
  writeFileSync(out, initialCoverage());
  console.log(`wrote ${out}`);
} else if (
  process.argv[1] !== undefined &&
  fileURLToPath(import.meta.url) === resolve(process.argv[1])
) {
  const idx = buildSpecIndex();
  const rows = readCoverage(resolve(root, 'docs', 'COVERAGE.md'));
  const cited = citedByCode();
  const byStatus = new Map<string, number>();
  for (const r of rows) byStatus.set(r.status, (byStatus.get(r.status) ?? 0) + 1);
  const perSystem = new Map<string, { total: number; cited: number; met: number }>();
  for (const r of idx.requirements) {
    if (r.form === 'NOTE') continue;
    const s = perSystem.get(r.system) ?? { total: 0, cited: 0, met: 0 };
    s.total += 1;
    if (cited.has(r.id)) s.cited += 1;
    if (rows.some((row) => row.id === r.id && row.status === 'MET')) s.met += 1;
    perSystem.set(r.system, s);
  }
  console.log(
    `requirements (REASON/VERIFY/FORBID): ${idx.requirements.filter((r) => r.form !== 'NOTE').length}`,
  );
  console.log(`coverage rows: ${rows.length}`, Object.fromEntries(byStatus));
  for (const [system, s] of perSystem) {
    console.log(
      `${system.padEnd(24)} total ${String(s.total).padStart(3)}  cited ${String(s.cited).padStart(3)}  met ${String(s.met).padStart(3)}`,
    );
  }
  const orphans = unattributedPartials(rows);
  if (orphans.length > 0) {
    console.log('');
    for (const r of orphans) {
      console.log(`PARTIAL with no item: ${r.id} — ${r.where}`);
    }
    console.log(
      `\n${orphans.length} PARTIAL row(s) name no item that finishes them (docs/PLAN.md §5).`,
    );
    process.exitCode = 1;
  }
}
