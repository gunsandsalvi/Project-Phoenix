/**
 * Does this system EXIST? — the check that makes a `MET` mark falsifiable.
 *
 * `docs/COVERAGE.md` holds one row per spec clause, and a `MET` is a claim about the SOURCE rather
 * than about the world. Nothing ever summed those claims, so a sector nobody wrote left no trace:
 * not in the source, and not in the claims made about it.
 *
 * So: per spec system, how many clauses are MET, PARTIAL, MISSING and OUT OF SCOPE, and how many of
 * the MET carry **UNMEASURED** — a module that cites the clause and has never produced an outcome.
 * A system with NO clause MET is an ABSENT SECTOR and is named at the top.
 *
 * It reads two files and runs no world. Which system a clause belongs to is the SPEC's fact, so it
 * is joined from the spec index rather than parsed out of COVERAGE's headings.
 *
 * Four things then refuse a row that cannot be read against its clause: a citation that does not
 * resolve, a MET naming an item `reach.ts` finds outside the world's closure, a MET naming
 * something no file in the tree contains, and the three ratchets below.
 *
 * `--verify` fails when the generated table differs from the one in `docs/IMPLEMENTATION.md`
 * Part 0, so the plan's own statement of what exists cannot go stale in silence.
 */
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, relative, resolve } from 'node:path';
import { buildSpecIndex } from './spec-index.js';
import { readCoverage, planPointers, type CoverageRow } from './spec-coverage.js';
import {
  absentCitations,
  deadModules,
  hollowClaims,
  itemsNamed,
  reachedNames,
  namesInTree,
  unreached,
} from './reach.js';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');

/** Where the plan states what exists. The table between these two markers is what `--verify` reads. */
const PLAN = resolve(root, 'docs', 'IMPLEMENTATION.md');
const HEADER_CELLS = ['system', 'MET', 'PARTIAL', 'MISSING', 'UNMEASURED', 'total'];
const TABLE_START = `| ${HEADER_CELLS.join(' | ')} |`;

/** A markdown row as its cells, so column padding and emphasis are not part of the comparison. */
function cells(line: string): string[] {
  return line
    .replace(/\*\*/g, '')
    .split('|')
    .slice(1, -1)
    .map((c) => c.trim());
}

function isSeparator(row: readonly string[]): boolean {
  return row.length > 0 && row.every((c) => /^:?-+:?$/.test(c));
}

/**
 * A RATCHET: MET rows that name no item of the module they cite.
 *
 * `where` is the evidence. A row naming a function can be read against that function and refused by
 * `check:reach` when nothing calls it; a row saying only that a module "implements" the clause can
 * be read against nothing. The defect is spread over every system, so the count falls and never
 * rises, and at zero this allowance is deleted and the rule becomes absolute.
 */
const NAMELESS_MET_ALLOWED = 85;

/**
 * A SECOND RATCHET: rows that give the same reason as another row.
 *
 * One sentence pasted across a whole system says what the system is, not what the clause is, so it
 * cannot be read against the clause and cannot be wrong. It falls the same way and for the same
 * reason as the one above it.
 */
const SHARED_REASON_ALLOWED = 700;

/**
 * A THIRD RATCHET: rows that are not MET and name something the source does not contain.
 *
 * A MET row citing a name the tree has never had is refused outright, because the claim is the
 * citation. A row saying what is SHORT is still wrong when it names a helper nobody wrote — it
 * describes a shape that is not there — and the defect is spread across the file, so the count
 * falls and never rises.
 */
const ABSENT_CITATION_ALLOWED = 34;

/** MET rows whose reason names no item at all. */
export function namelessClaims(rows: readonly CoverageRow[]): CoverageRow[] {
  return rows.filter((r) => r.status === 'MET' && itemsNamed(r.where).length === 0);
}

/** Rows whose reason is word for word another row's. */
export function sharedReasons(rows: readonly CoverageRow[]): CoverageRow[] {
  const times = new Map<string, number>();
  for (const r of rows) times.set(r.where, (times.get(r.where) ?? 0) + 1);
  return rows.filter((r) => (times.get(r.where) ?? 0) > 1);
}

/** Prints where a ratchet stands and answers whether it is off its allowance in either direction. */
function ratchet(what: string, now: number, allowance: number): boolean {
  const said =
    now > allowance
      ? `ROSE to ${String(now)} — a ratchet only falls`
      : now < allowance
        ? `has fallen to ${String(now)}: lower the allowance, or the check goes slack`
        : `${String(now)} left`;
  console.log('');
  console.log(`[${what}] allowance ${String(allowance)} · ${said}`);
  return now !== allowance;
}

/** The files that do not change, and so may not point at the one that does. */
const FIXED_FILES = [
  resolve(root, 'docs', 'COVERAGE.md'),
  resolve(root, 'docs', 'ARCHITECTURE.md'),
  resolve(root, 'docs', 'spec', 'PROJECT_PHOENIX.md'),
  resolve(root, 'CLAUDE.md'),
  resolve(root, 'packages', 'kernel-rs', 'src'),
];

/** Every line under these paths that points into the plan. A directory is walked for `.rs`. */
function planPointersIn(paths: readonly string[]): { file: string; line: number; says: string }[] {
  const out: { file: string; line: number; says: string }[] = [];
  const visit = (at: string): void => {
    if (!existsSync(at)) return;
    if (statSync(at).isDirectory()) {
      for (const entry of readdirSync(at)) visit(resolve(at, entry));
      return;
    }
    if (!/\.(md|rs)$/.test(at)) return;
    for (const p of planPointers(readFileSync(at, 'utf8'))) {
      out.push({ file: relative(root, at), line: p.line, says: p.says.slice(0, 92) });
    }
  };
  for (const p of paths) visit(p);
  return out;
}

export interface SystemExistence {
  readonly system: string;
  readonly met: number;
  readonly partial: number;
  readonly missing: number;
  readonly outOfScope: number;
  /** MET rows whose `where` says the module has never produced an outcome. A subset of `met`. */
  readonly neverReached: number;
  readonly total: number;
}

/**
 * One row per spec system, in the spec's own order.
 *
 * A clause with no COVERAGE row at all is counted MISSING: an unanswered clause and one answered
 * "not built" are the same amount of world, and treating an absent row as absent-from-the-count
 * would let a system look complete by having fewer rows (Part II: never delete a clause to look
 * better).
 */
export function existence(
  coveragePath: string = resolve(root, 'docs', 'COVERAGE.md'),
  specPath?: string,
): SystemExistence[] {
  const idx = specPath === undefined ? buildSpecIndex() : buildSpecIndex(specPath);
  const rows = new Map(readCoverage(coveragePath).map((r) => [r.id, r]));
  const bySystem = new Map<string, SystemExistence>();
  for (const r of idx.requirements) {
    const s = bySystem.get(r.system) ?? {
      system: r.system,
      met: 0,
      partial: 0,
      missing: 0,
      outOfScope: 0,
      neverReached: 0,
      total: 0,
    };
    const row = rows.get(r.id);
    const status = row === undefined ? 'MISSING' : row.status;
    bySystem.set(r.system, {
      ...s,
      total: s.total + 1,
      met: s.met + (status === 'MET' ? 1 : 0),
      partial: s.partial + (status === 'PARTIAL' ? 1 : 0),
      missing: s.missing + (status === 'MISSING' ? 1 : 0),
      outOfScope: s.outOfScope + (status === 'OUT OF SCOPE' ? 1 : 0),
      neverReached:
        s.neverReached + (status === 'MET' && row?.where.includes('UNMEASURED') === true ? 1 : 0),
    });
  }
  return [...bySystem.values()];
}

/**
 * Spec clauses with no COVERAGE row at all, sub-clauses included.
 *
 * A clause nobody has answered and a clause somebody deleted look identical from inside
 * COVERAGE.md, and a clause is never deleted to look better. A sub-clause that only elaborates its
 * parent still says something the world either does or does not do, so it is answered on its own.
 */
export function unanswered(
  coveragePath: string = resolve(root, 'docs', 'COVERAGE.md'),
  specPath?: string,
): string[] {
  const idx = specPath === undefined ? buildSpecIndex() : buildSpecIndex(specPath);
  const rows = new Set(readCoverage(coveragePath).map((r) => r.id));
  return idx.requirements.filter((r) => !rows.has(r.id)).map((r) => r.id);
}

/**
 * Every path a row cites is opened. MET means the cited module implements the clause, so the
 * citation is the whole of the evidence.
 *
 * It proves a citation resolves, not that the assessment is still true. There is no exemption: a
 * path in a `where` cell is a citation by construction.
 */
export function citedPaths(where: string): string[] {
  const out: string[] = [];
  // `docs/` is deliberately not read: a row cites the SOURCE that implements it, and this file's
  // own siblings move for reasons that have nothing to do with a clause being met.
  for (const m of where.matchAll(/\b(?:packages|tools)\/[A-Za-z0-9_.\-/]+/g)) {
    // Prose runs on after a path: `…ledger.rs, …` and `…audit.rs)`. The trailing punctuation is the
    // sentence's and not the path's.
    const path = m[0].replace(/[.,;:)]+$/, '');
    if (path.length > 0) out.push(path);
  }
  return out;
}

/** A row whose citation does not resolve, and the path that does not. */
export interface DeadCitation {
  readonly id: string;
  readonly status: CoverageRow['status'];
  readonly path: string;
}

/** Every one of them, in the file's own order. */
export function deadCitations(
  coveragePath: string = resolve(root, 'docs', 'COVERAGE.md'),
  at: string = root,
): DeadCitation[] {
  const out: DeadCitation[] = [];
  for (const r of readCoverage(coveragePath)) {
    for (const path of citedPaths(r.where)) {
      if (!existsSync(resolve(at, path))) out.push({ id: r.id, status: r.status, path });
    }
  }
  return out;
}

/**
 * The systems with nothing built at all.
 *
 * This is the read the last plan did not have. Five of these were 0 of 19 to 0 of 32 while the
 * document that was supposed to be the ordered list of everything open carried them as one row of
 * one finding, under an item marked done.
 */
export function absent(rows: readonly SystemExistence[]): SystemExistence[] {
  return rows.filter((r) => r.met === 0);
}

/**
 * Systems that are built on paper and have produced nothing — every MET they have is UNMEASURED.
 *
 * The same absence as `absent()` wearing a better word. A reader scanning for "0 MET" walks past
 * `M&A 10 MET` without pausing, and all ten of those are marks on a module that has never produced
 * an outcome. Named separately because the remedy is different: an absent sector needs writing, and
 * one of these needs a way IN to what is already written.
 */
export function builtAndDead(rows: readonly SystemExistence[]): SystemExistence[] {
  return rows.filter((r) => r.met > 0 && r.neverReached === r.met);
}

/** Bold a figure that is the story of its row, so the table reads at a glance. */
function mark(n: number, heavy: boolean): string {
  return heavy ? `**${String(n)}**` : String(n);
}

export function renderTable(rows: readonly SystemExistence[]): string {
  const out: string[] = [TABLE_START, '|---|---|---|---|---|---|'];
  for (const r of rows) {
    const bare = r.met === 0;
    const thin = r.met * 4 < r.total;
    out.push(
      `| ${bare || thin ? `**${r.system}**` : r.system} | ${mark(r.met, bare || thin)} | ` +
        `${String(r.partial)} | ${mark(r.missing, bare || thin)} | ` +
        `${r.neverReached > 0 ? `**${String(r.neverReached)}**` : '0'} | ${String(r.total)} |`,
    );
  }
  return out.join('\n');
}

/** Compare on the numbers only: the emphasis is for a reader and is not a fact. */
function figures(table: string): string[] {
  return table
    .split('\n')
    .filter((l) => l.trimStart().startsWith('|'))
    .map(cells)
    .filter((row) => !isSeparator(row))
    .map((row) => row.join(' | '));
}

/**
 * The table is found by its HEADER CELLS, not by an exact line. `npm run format` pads a markdown
 * table's columns, and a check that only recognises one spelling of the header stops finding it.
 */
function planTable(): string[] {
  const lines = readFileSync(PLAN, 'utf8').split('\n');
  const at = lines.findIndex(
    (l) => l.trimStart().startsWith('|') && cells(l).join('|') === HEADER_CELLS.join('|'),
  );
  if (at < 0) return [];
  const rest = lines.slice(at);
  const end = rest.findIndex((l, i) => i > 0 && !l.trimStart().startsWith('|'));
  return figures(rest.slice(0, end < 0 ? rest.length : end).join('\n'));
}

function report(): number {
  const rows = existence();
  const gone = absent(rows);
  const clauses = rows.reduce((a, r) => a + r.total, 0);
  const met = rows.reduce((a, r) => a + r.met, 0);
  const dead = rows.reduce((a, r) => a + r.neverReached, 0);
  const partial = rows.reduce((a, r) => a + r.partial, 0);
  const missing = rows.reduce((a, r) => a + r.missing, 0);

  console.log(`ABSENT SECTORS — no clause MET at all: ${String(gone.length)}`);
  for (const r of gone) {
    console.log(`  ${r.system.padEnd(26)} 0 of ${String(r.total)}`);
  }
  console.log('');
  console.log(
    `${String(clauses)} clauses: ${String(met)} MET (${String(dead)} UNMEASURED), ` +
      `${String(partial)} PARTIAL, ${String(missing)} MISSING`,
  );
  const dead2 = builtAndDead(rows);
  console.log('');
  console.log(
    `BUILT AND DEAD — every clause MET, and every one of them UNMEASURED: ${String(dead2.length)}`,
  );
  for (const r of dead2) {
    console.log(`  ${r.system.padEnd(26)} ${String(r.met)} of ${String(r.total)}, none reached`);
  }
  console.log('');
  const blank = unanswered();
  if (blank.length > 0) {
    console.log(`${String(blank.length)} spec clause(s) have NO row: ${blank.join(', ')}`);
  }
  console.log('');
  console.log(renderTable(rows));

  // The citation is the evidence, so a citation that does not resolve is a row asserting
  // nothing. It fails whatever the mode, because a report that printed it and returned 0 would be
  // the silence this check exists to end.
  const unresolved = deadCitations();
  if (unresolved.length > 0) {
    console.log('');
    console.log(
      `${String(unresolved.length)} citation(s) in docs/COVERAGE.md name a path that is not in the tree. ` +
        'A MET is a claim about the cited module, so a citation that does not resolve asserts nothing:',
    );
    for (const d of unresolved)
      console.log(`  ${d.id.padEnd(28)} ${d.status.padEnd(12)} ${d.path}`);
  }

  // The plan is the one file that changes; these are the ones that do not. A row that named the
  // item finishing it, or a comment that named the item building it, was a live reference for
  // exactly as long as that item stayed open — and 104 of 119 of them were pointing at items
  // deleted when they closed. A row says what is short; the plan says who fixes it.
  const pointers = planPointersIn(FIXED_FILES);
  if (pointers.length > 0) {
    console.log('');
    console.log(
      `${String(pointers.length)} line(s) in a file that does not change point into docs/IMPLEMENTATION.md, ` +
        'which does. An item closes and is deleted, and the pointer outlives it:',
    );
    for (const p of pointers)
      console.log(`  ${`${p.file}:${String(p.line)}`.padEnd(44)} ${p.says}`);
  }

  // A citation that resolves still says nothing if the world never calls what it cites.
  const hollow = hollowClaims(
    readCoverage(resolve(root, 'docs', 'COVERAGE.md')),
    unreached(),
    deadModules(),
    reachedNames(),
  );
  if (hollow.length > 0) {
    console.log('');
    console.log(
      `${String(hollow.length)} MET row(s) in docs/COVERAGE.md name an item no production code reaches. ` +
        'A clause met by code nothing calls is not met:',
    );
    for (const h of hollow) console.log(`  ${h.id.padEnd(26)} ${h.names.padEnd(34)} ${h.why}`);
  }

  const coverage = readCoverage(resolve(root, 'docs', 'COVERAGE.md'));

  // A citation the tree does not contain at all resolves to nothing. On a MET row that is the
  // whole of the claim, so it is refused; on a row saying what is short it is a shape nobody wrote,
  // and the ratchet below holds it.
  const cited = absentCitations(coverage, namesInTree());
  const invented = cited.filter((a) => a.status === 'MET');
  if (invented.length > 0) {
    console.log('');
    console.log(
      `${String(invented.length)} MET row(s) in docs/COVERAGE.md name something no file in the tree ` +
        'contains. A claim whose citation resolves to nothing cannot be read against the clause:',
    );
    for (const a of invented) console.log(`  ${a.id.padEnd(26)} ${a.names}`);
  }

  // All three are read, because a reader who fixed one needs to see where the others now stand.
  const nameless = ratchet(
    'MET rows naming no item',
    namelessClaims(coverage).length,
    NAMELESS_MET_ALLOWED,
  );
  const shared = ratchet(
    'rows sharing a reason',
    sharedReasons(coverage).length,
    SHARED_REASON_ALLOWED,
  );
  const unknown = ratchet(
    'unmet rows naming what the tree has not got',
    cited.length - invented.length,
    ABSENT_CITATION_ALLOWED,
  );
  const ratchets = nameless || shared || unknown;

  if (
    unresolved.length > 0 ||
    pointers.length > 0 ||
    hollow.length > 0 ||
    invented.length > 0 ||
    ratchets
  ) {
    console.log('');
    console.log('Re-read the clause against the source and re-mark the row from what is there.');
    return 1;
  }

  if (process.argv.includes('--missing')) return blank.length > 0 ? 1 : 0;
  if (!process.argv.includes('--verify')) return 0;
  const mine = figures(renderTable(rows));
  const theirs = planTable();
  if (theirs.length === 0) {
    console.log(
      `\ndocs/IMPLEMENTATION.md has no existence table. Paste the one above into Part 0.`,
    );
    return 1;
  }
  const drift = mine.filter((l, i) => theirs[i] !== l);
  if (drift.length > 0 || mine.length !== theirs.length) {
    console.log('\ndocs/IMPLEMENTATION.md Part 0 does not say what COVERAGE.md says:');
    for (const l of drift) console.log(`  is now: ${l}`);
    for (const l of theirs.filter((t) => !mine.includes(t))) console.log(`  it says: ${l}`);
    console.log('\nRe-generate Part 0 with `npm run check:existence` in the same change.');
    return 1;
  }
  return 0;
}

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  process.exitCode = report();
}
