/**
 * Does this system EXIST? — the check that makes a `done` row and a `MET` mark falsifiable.
 *
 * `docs/COVERAGE.md` holds one row per spec clause and says so in its own header: `MET` is a claim
 * about the SOURCE, not about the world. Nothing ever summed it. Six sectors were placed into items
 * that then closed without them and nothing anywhere said so, because a sector that was never
 * written leaves no trace in the source a read looks at, nor in the claims made about that source —
 * only in the aggregate nobody took (`docs/IMPLEMENTATION.md` Part 0).
 *
 * So: per spec system, how many clauses are MET, PARTIAL, MISSING and OUT OF SCOPE, and how many of
 * the MET carry **UNMEASURED** — a module that cites the clause and has never produced an
 * outcome. A system with NO clause MET is an ABSENT SECTOR and is named at the top.
 *
 * It reads two files and runs no world. Which system a clause belongs to is the SPEC's fact, so it
 * is joined from the spec index rather than parsed out of COVERAGE's headings (Law 4: one writer).
 * The UNMEASURED marks are likewise READ from where `world/reach.ts` caused them to be written,
 * never re-derived (Law 19) — the reach read is what keeps them true, and this is what counts them.
 *
 * `--verify` fails when the generated table differs from the one in `docs/IMPLEMENTATION.md`
 * Part 0, so the plan's own statement of what exists cannot go stale in silence. That is the way
 * the last one went stale.
 */
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { buildSpecIndex } from './spec-index.js';
import { readCoverage } from './spec-coverage.js';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');

/** Where the plan states what exists. The table between these two markers is what `--verify` reads. */
const PLAN = resolve(root, 'docs', 'IMPLEMENTATION.md');
const TABLE_START = '| system | MET | PARTIAL | MISSING | UNMEASURED | total |';

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
    if (r.form === 'NOTE') continue;
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
        s.neverReached +
        (status === 'MET' && row?.where.includes('UNMEASURED') === true ? 1 : 0),
    });
  }
  return [...bySystem.values()];
}

/**
 * COVERAGE rows that mark something the spec does not count as a requirement.
 *
 * A spec sub-clause that carries no REASON/VERIFY/FORBID word is a NOTE — a continuation of its
 * parent — and the index says so. Eight of them carry a `MET` row anyway. Marking work you did is
 * not wrong, and a count that quietly included them would make the denominator disagree with the
 * spec's own, so they are reported instead of absorbed: two files disagreeing about what a
 * requirement IS is the kind of thing this check exists to show.
 */
export function marksOnNotes(
  coveragePath: string = resolve(root, 'docs', 'COVERAGE.md'),
  specPath?: string,
): string[] {
  const idx = specPath === undefined ? buildSpecIndex() : buildSpecIndex(specPath);
  const clauses = new Set(idx.requirements.filter((r) => r.form !== 'NOTE').map((r) => r.id));
  return readCoverage(coveragePath)
    .filter((r) => !clauses.has(r.id))
    .map((r) => r.id);
}

/**
 * Spec clauses with no COVERAGE row at all.
 *
 * Zero today, and it is checked rather than assumed: a clause nobody has answered and a clause
 * somebody deleted look identical from inside COVERAGE.md, and Part II is explicit that a clause is
 * never deleted to look better.
 */
export function unanswered(
  coveragePath: string = resolve(root, 'docs', 'COVERAGE.md'),
  specPath?: string,
): string[] {
  const idx = specPath === undefined ? buildSpecIndex() : buildSpecIndex(specPath);
  const rows = new Set(readCoverage(coveragePath).map((r) => r.id));
  return idx.requirements
    .filter((r) => r.form !== 'NOTE' && !rows.has(r.id))
    .map((r) => r.id);
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
    .filter((l) => l.startsWith('| ') && !l.startsWith('|---'))
    .map((l) => l.replace(/\*\*/g, '').trim());
}

function planTable(): string[] {
  const text = readFileSync(PLAN, 'utf8');
  const at = text.indexOf(TABLE_START);
  if (at < 0) return [];
  const lines = text.slice(at).split('\n');
  const end = lines.findIndex((l, i) => i > 0 && !l.startsWith('|'));
  return figures(lines.slice(0, end < 0 ? lines.length : end).join('\n'));
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
  const notes = marksOnNotes();
  const blank = unanswered();
  if (notes.length > 0) {
    console.log(
      `${String(notes.length)} MET row(s) mark a spec NOTE rather than a requirement, ` +
        `so they are outside the count above: ${notes.join(', ')}`,
    );
  }
  if (blank.length > 0) {
    console.log(`${String(blank.length)} spec clause(s) have NO row: ${blank.join(', ')}`);
  }
  console.log('');
  console.log(renderTable(rows));

  if (!process.argv.includes('--verify')) return 0;
  const mine = figures(renderTable(rows));
  const theirs = planTable();
  if (theirs.length === 0) {
    console.log(`\ndocs/IMPLEMENTATION.md has no existence table. Paste the one above into Part 0.`);
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
