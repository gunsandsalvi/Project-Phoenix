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

export function itemProgress(): ItemProgress[] {
  const manifest = JSON.parse(
    readFileSync(resolve(root, 'docs', 'plan', 'manifest.json'), 'utf8'),
  ) as ManifestItem[];
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
    const state = !i.present
      ? 'closed'
      : i.done === 0
        ? 'open'
        : i.done === i.steps
          ? 'closing'
          : 'in progress';
    const link = i.present ? `[${i.id} — ${i.title}](plan/${i.file})` : `${i.id} — ${i.title}`;
    lines.push(`| ${link} | ${i.steps} | ${i.done} | ${state} |`);
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
