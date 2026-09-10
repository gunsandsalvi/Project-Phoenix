/**
 * Recount the plan's completion (Appendix C: recount rather than adjust).
 *
 * `docs/plan/manifest.json` lists every item file with the number of steps it had when written. An
 * item file that still exists contributes its checked steps (`- [x]`); an item file that has been
 * deleted counts as fully done. Coverage comes from docs/COVERAGE.md. The result is written between
 * the markers in docs/PLAN.md. Run: `npm run plan:progress`; `npm run check` runs it with `--check`, which
 * fails if the block is stale.
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
}

/**
 * The item ids on the worklist, in the order it works them (Law 10).
 *
 * A row is `| <id> | ...`, and the id is the first cell. Read rather than restated: the worklist is
 * the one ordered list, and a second copy of it here would be the defect this function exists to
 * catch (Law 19).
 */
export function worklistIds(): string[] {
  const text = readFileSync(resolve(root, 'docs', 'WORKLIST.md'), 'utf8');
  const out: string[] = [];
  for (const line of text.split('\n')) {
    const m = /^\|\s*([A-Za-z0-9][A-Za-z0-9.]*)\s*\|/.exec(line);
    if (m?.[1] !== undefined && m[1] !== 'item') out.push(m[1]);
  }
  return out;
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
  crossCheck(
    manifest.map((m) => m.id),
    worklistIds(),
  );
  return manifest.map((m) => {
    const path = resolve(root, 'docs', 'plan', m.file);
    if (!existsSync(path)) return { ...m, done: m.steps, present: false };
    const text = readFileSync(path, 'utf8');
    const checked = (text.match(/^- \[x\]/gim) ?? []).length;
    const unchecked = (text.match(/^- \[ \]/gm) ?? []).length;
    if (checked + unchecked !== m.steps) {
      throw new Error(
        `${m.file}: manifest says ${m.steps} steps, file has ${checked + unchecked}; update the manifest in the same change`,
      );
    }
    return { ...m, done: checked, present: true };
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
    // An item worked without a plan file has no planned steps, and saying "0 of 0" would read as an
    // item that was free. It says what happened instead; the steps column is empty because there
    // were none to count, not because none were done.
    const unplanned = !i.present && i.steps === 0;
    const state = unplanned
      ? 'closed (no item file)'
      : !i.present
        ? 'closed'
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
