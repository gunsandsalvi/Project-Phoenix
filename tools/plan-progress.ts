/**
 * Recount the plan's completion (Appendix C: recount rather than adjust).
 *
 * `docs/plan/manifest.json` lists every item file with the number of steps it had when written. An
 * item file that still exists contributes its checked steps (`- [x]`). WHETHER AN ITEM IS CLOSED IS
 * READ FROM `docs/WORKLIST.md`, never inferred here (Law 4, Law 19): the worklist's state column is
 * the one writer of that fact, and a missing plan file is not a second one. It used to be — a
 * deleted file counted as fully done — and item 14's file was folded into `docs/IMPLEMENTATION.md` while the
 * item was still open, at which point the figure would have claimed fourteen steps nobody had
 * worked. Coverage comes from docs/COVERAGE.md. The result is written between the markers in
 * docs/PLAN.md. Run: `npm run plan:progress`; `npm run check` runs it with `--check`, which fails if
 * the block is stale.
 */
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { readCoverage } from './spec-coverage.js';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');

export interface ManifestItem {
  readonly id: string;
  readonly title: string;
  readonly file: string;
  readonly steps: number;
}

export interface ItemProgress extends ManifestItem {
  readonly done: number;
  readonly present: boolean;
  /** The worklist's state column, read (Law 19), not inferred from whether the file is there. */
  readonly closed: boolean;
}

/** An item as the worklist states it: its id and the state in its last cell. */
export interface WorklistItem {
  readonly id: string;
  readonly state: string;
}

/**
 * The item ids on the worklist, in the order it works them (Law 10).
 *
 * A row is `| <id> | ...`, and the id is the first cell. Read rather than restated: the worklist is
 * the one ordered list, and a second copy of it here would be the defect this function exists to
 * catch (Law 19).
 */
export function worklistItems(): WorklistItem[] {
  const text = readFileSync(resolve(root, 'docs', 'WORKLIST.md'), 'utf8');
  const out: WorklistItem[] = [];
  for (const line of text.split('\n')) {
    const m = /^\|\s*([A-Za-z0-9][A-Za-z0-9.]*)\s*\|/.exec(line);
    if (m?.[1] === undefined || m[1] === 'item') continue;
    // The state is the LAST cell, not the third: one item's prose has pipes in it, and counting
    // from the left would read its middle as its state.
    const cells = line
      .trim()
      .replace(/^\||\|$/g, '')
      .split('|');
    const last = cells[cells.length - 1];
    out.push({ id: m[1], state: (last ?? '').trim() });
  }
  return out;
}

export function worklistIds(): string[] {
  return worklistItems().map((w) => w.id);
}

/**
 * Law 10, Appendix C: the manifest and the worklist name the same items.
 *
 * Three items — 10.1, 10.2 and 10.4 — were inserted, worked and closed without ever reaching the
 * manifest, so the progress figure counted twenty-nine items where the worklist had thirty-two and
 * `plan:check` could not see it: it validated the step counts of files that still exist and had no
 * way to know an item existed at all until the manifest named it. A figure that silently omits
 * closed work is worse than no figure, so the two lists are compared in both directions here, on
 * the commit that makes them differ.
 */
export function crossCheck(manifest: readonly string[], worklist: readonly string[]): void {
  const onList = new Set(worklist);
  const inManifest = new Set(manifest);
  const missing = [...onList].filter((id) => !inManifest.has(id));
  const extra = [...inManifest].filter((id) => !onList.has(id));
  if (missing.length > 0) {
    throw new Error(
      `docs/WORKLIST.md has items the manifest does not: ${missing.join(', ')}. Add the row in the change that inserts the item (docs/PLAN.md §5)`,
    );
  }
  if (extra.length > 0) {
    throw new Error(
      `docs/plan/manifest.json has items the worklist does not: ${extra.join(', ')}. An item exists on the one ordered list or it does not exist (Law 10)`,
    );
  }
}

export function itemProgress(): ItemProgress[] {
  const manifest = JSON.parse(
    readFileSync(resolve(root, 'docs', 'plan', 'manifest.json'), 'utf8'),
  ) as ManifestItem[];
  const worklist = worklistItems();
  crossCheck(
    manifest.map((m) => m.id),
    worklist.map((w) => w.id),
  );
  const state = new Map(worklist.map((w) => [w.id, w.state]));
  return manifest.map((m) => {
    const closed = state.get(m.id) === 'done';
    const path = resolve(root, 'docs', 'plan', m.file);
    // A closed item's file is deleted and its steps are all done; an OPEN item whose file is gone
    // has had its plan moved somewhere the manifest does not count, and none of its steps are done
    // until the worklist says the item is.
    if (!existsSync(path)) return { ...m, done: closed ? m.steps : 0, present: false, closed };
    const text = readFileSync(path, 'utf8');
    const checked = (text.match(/^- \[x\]/gim) ?? []).length;
    const unchecked = (text.match(/^- \[ \]/gm) ?? []).length;
    if (checked + unchecked !== m.steps) {
      throw new Error(
        `${m.file}: manifest says ${m.steps} steps, file has ${checked + unchecked}; update the manifest in the same change`,
      );
    }
    return { ...m, done: checked, present: true, closed };
  });
}

export function render(items: readonly ItemProgress[]): string {
  const total = items.reduce((s, i) => s + i.steps, 0);
  const done = items.reduce((s, i) => s + i.done, 0);
  const rows = readCoverage(resolve(root, 'docs', 'COVERAGE.md'));
  const met = rows.filter((r) => r.status === 'MET').length;
  const partial = rows.filter((r) => r.status === 'PARTIAL').length;
  const scope = rows.filter((r) => r.status === 'OUT OF SCOPE').length;
  const pct = (a: number, b: number): string => (b === 0 ? '0.0' : ((100 * a) / b).toFixed(1));
  const lines: string[] = [];
  lines.push(
    `**Plan completion: ${pct(done, total)}%** (${done} of ${total} steps across ${items.length} items).`,
  );
  lines.push(
    `**Requirement coverage: ${pct(met + scope, rows.length)}%** (${met} MET, ${partial} PARTIAL, ${scope} OUT OF SCOPE of ${rows.length} REASON/VERIFY/FORBID clauses).`,
  );
  lines.push('');
  lines.push('| item | steps | done | state |');
  lines.push('|---|---|---|---|');
  for (const i of items) {
    // An item with no plan file has no planned steps, and saying "0 of 0" would read as an item
    // that was free. It says what happened instead; the steps column is empty because there were
    // none to count, not because none were done. Five such items are OPEN — inserted from a sweep
    // with their reasoning in the worklist row and no plan written yet.
    const unplanned = !i.present && i.steps === 0;
    const state = unplanned
      ? i.closed
        ? 'closed (no item file)'
        : 'open (no item file)'
      : !i.present
        ? i.closed
          ? 'closed'
          : 'open (plan elsewhere)'
        : i.done === 0
          ? 'open'
          : i.done === i.steps
            ? 'closing'
            : 'in progress';
    const link = i.present ? `[${i.id} — ${i.title}](plan/${i.file})` : `${i.id} — ${i.title}`;
    const steps = unplanned ? '—' : String(i.steps);
    const done = unplanned ? '—' : String(i.done);
    lines.push(`| ${link} | ${steps} | ${done} | ${state} |`);
  }
  return lines.join('\n');
}

export function updatePlan(): string {
  const path = resolve(root, 'docs', 'PLAN.md');
  const text = readFileSync(path, 'utf8');
  const start = '<!-- progress:start -->';
  const end = '<!-- progress:end -->';
  const a = text.indexOf(start);
  const b = text.indexOf(end);
  if (a < 0 || b < 0 || b < a) throw new Error('docs/PLAN.md has no progress markers');
  const block = render(itemProgress());
  const next = `${text.slice(0, a + start.length)}\n${block}\n${text.slice(b)}`;
  if (next !== text) writeFileSync(path, next);
  return block;
}

/** `--check`: fail when the block in docs/PLAN.md is stale instead of rewriting it (used by `npm run check`). */
export function checkPlan(): string {
  const path = resolve(root, 'docs', 'PLAN.md');
  const text = readFileSync(path, 'utf8');
  const start = '<!-- progress:start -->';
  const end = '<!-- progress:end -->';
  const a = text.indexOf(start);
  const b = text.indexOf(end);
  if (a < 0 || b < 0 || b < a) throw new Error('docs/PLAN.md has no progress markers');
  const block = render(itemProgress());
  const norm = (t: string): string =>
    t
      .split('\n')
      .map((l) =>
        l
          .replace(/-+/g, '-')
          .replace(/[ \t]+/g, '')
          .trim(),
      )
      .filter((l) => l.length > 0)
      .join('\n');
  if (norm(text.slice(a + start.length, b)) !== norm(block)) {
    throw new Error('docs/PLAN.md progress block is stale: run `npm run plan:progress` and commit');
  }
  return block;
}

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  console.log(process.argv.includes('--check') ? checkPlan() : updatePlan());
}
