/**
 * Recount the plan's completion (Appendix C: recount rather than adjust).
 *
 * THE PLAN IS `docs/IMPLEMENTATION.md` and this counts ITS sections, in the order they are written.
 * An item is `## <id>. <title>` and its steps are the `- [ ]` / `- [x]` lines under it.
 *
 * It used to walk the rows of `docs/WORKLIST.md` instead, and items 0a to 24 have no row there —
 * they live in the plan alone — so every step of every one of them was invisible: ticking all five
 * of item 0a moved the figure by nothing, which is how the defect was found (item 0a, positioned
 * at 0c). The worklist is still read, for one thing it is the one writer of: which of the items it
 * worked are `done`, so a closed item counts every step it had rather than the none its deleted
 * section shows. Coverage comes from docs/COVERAGE.md. The result is written between the markers in
 * docs/PLAN.md. Run `npm run plan:progress`; `npm run check` runs it with `--check`, which fails if
 * the block is stale.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { readCoverage } from './spec-coverage.js';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');

/** An item as the worklist states it: its id and the state in its last cell. */
export interface WorklistItem {
  readonly id: string;
  readonly state: string;
}

export interface ItemProgress extends WorklistItem {
  readonly title: string;
  readonly steps: number;
  readonly done: number;
  /** Whether `docs/IMPLEMENTATION.md` has a section for it. */
  readonly present: boolean;
  /** The worklist's state column, read (Law 19), never inferred from whether the section is there. */
  readonly closed: boolean;
}

/**
 * The item ids on the worklist, in the order it works them (Law 10). A row is `| <id> | ...`, and
 * the state is the LAST cell: one item's prose has pipes in it.
 */
export function worklistItems(): WorklistItem[] {
  const text = readFileSync(resolve(root, 'docs', 'WORKLIST.md'), 'utf8');
  const out: WorklistItem[] = [];
  for (const line of text.split('\n')) {
    const m = /^\|\s*([A-Za-z0-9][A-Za-z0-9.]*)\s*\|/.exec(line);
    if (m?.[1] === undefined || m[1] === 'item') continue;
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

export interface PlanSection {
  readonly title: string;
  readonly checked: number;
  readonly unchecked: number;
}

/** The plan's sections by item id: `## <id>. <title>` down to the next `## ` heading. */
export function planSections(text: string): Map<string, PlanSection> {
  const out = new Map<string, PlanSection>();
  let current: { id: string; title: string; checked: number; unchecked: number } | undefined;
  const close = (): void => {
    if (current !== undefined) {
      out.set(current.id, {
        title: current.title,
        checked: current.checked,
        unchecked: current.unchecked,
      });
    }
  };
  for (const line of text.split('\n')) {
    const head = /^##\s+(.*)$/.exec(line);
    if (head !== null) {
      close();
      const item = /^([0-9][0-9a-z′]*(?:\.[0-9]+)?)\.\s+(.*)$/.exec(head[1] ?? '');
      current =
        item?.[1] === undefined
          ? undefined
          : { id: item[1], title: item[2] ?? '', checked: 0, unchecked: 0 };
      continue;
    }
    if (current === undefined) continue;
    if (line.toLowerCase().startsWith('- [x]')) current.checked += 1;
    else if (line.startsWith('- [ ]')) current.unchecked += 1;
  }
  close();
  return out;
}

export function itemProgress(): ItemProgress[] {
  const sections = planSections(readFileSync(resolve(root, 'docs', 'IMPLEMENTATION.md'), 'utf8'));
  const worklist = new Map(worklistItems().map((w) => [w.id, w.state]));
  const out: ItemProgress[] = [];
  // The PLAN's own order first: every item that has a section, whether or not the worklist knows it.
  for (const [id, s] of sections) {
    const steps = s.checked + s.unchecked;
    /**
     * Law 4, Law 19: WHO SAYS AN ITEM IS CLOSED. The worklist speaks for the items it WORKED, and
     * only through `done` — that row is what tells this to count a closed item's steps, whose
     * section was deleted when it closed. It speaks for nothing else, because the two files used
     * one id for two items: the worklist's `14` was the polity and the plan's is the insurers.
     *
     * Otherwise the item's own steps say it: every one ticked IS closed, because that is what
     * closing an item means (CLAUDE.md: tick the steps, delete the section). Nothing infers it from
     * a section's absence, which is the other direction and would make an unwritten item look done.
     */
    const closed = worklist.get(id) === 'done' || (steps > 0 && s.unchecked === 0);
    const state = closed ? 'done' : 'open';
    out.push({ id, state, title: s.title, steps, done: closed ? steps : s.checked, present: true, closed });
  }
  // Then the worklist rows the plan has no section for: the items it worked and closed, whose
  // sections were deleted when they closed, and whose steps are in docs/RECORD.md.
  for (const [id, state] of worklist) {
    if (sections.has(id)) continue;
    out.push({ id, state, title: '', steps: 0, done: 0, present: false, closed: state === 'done' });
  }
  return out;
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
  const planned = items.filter((i) => i.present);
  const closedItems = planned.filter((i) => i.steps > 0 && i.done === i.steps).length;
  lines.push(
    `**The plan: ${closedItems} of ${planned.length} items closed** (${done} of ${total} steps).`,
  );
  lines.push(
    `**Requirement coverage: ${pct(met + scope, rows.length)}%** (${met} MET, ${partial} PARTIAL, ${scope} OUT OF SCOPE of ${rows.length} REASON/VERIFY/FORBID clauses).`,
  );
  lines.push('');
  lines.push('| item | steps | done | state |');
  lines.push('|---|---|---|---|');
  for (const i of items) {
    // A worklist row that was superseded says where it went; the plan item it named is the row
    // that counts. Only a row that is neither closed nor moved and has no section is a gap.
    const moved = i.state.startsWith('moved to plan ');
    const state = i.closed
      ? 'closed'
      : moved
        ? i.state
        : !i.present
          ? 'open (no plan section)'
          : i.done === 0
            ? 'open'
            : i.done === i.steps
              ? 'closing'
              : 'in progress';
    const name = i.title === '' ? i.id : `${i.id} — ${i.title}`;
    const steps = i.present ? String(i.steps) : '—';
    const doneCol = i.present ? String(i.done) : '—';
    lines.push(`| ${name} | ${steps} | ${doneCol} | ${state} |`);
  }
  return lines.join('\n');
}

const START = '<!-- progress:start -->';
const END = '<!-- progress:end -->';

function markers(text: string): { a: number; b: number } {
  const a = text.indexOf(START);
  const b = text.indexOf(END);
  if (a < 0 || b < 0 || b < a) throw new Error('docs/PLAN.md has no progress markers');
  return { a, b };
}

export function updatePlan(): string {
  const path = resolve(root, 'docs', 'PLAN.md');
  const text = readFileSync(path, 'utf8');
  const { a, b } = markers(text);
  const block = render(itemProgress());
  const next = `${text.slice(0, a + START.length)}\n${block}\n${text.slice(b)}`;
  if (next !== text) writeFileSync(path, next);
  return block;
}

/** `--check`: fail when the block in docs/PLAN.md is stale instead of rewriting it (used by `npm run check`). */
export function checkPlan(): string {
  const path = resolve(root, 'docs', 'PLAN.md');
  const text = readFileSync(path, 'utf8');
  const { a, b } = markers(text);
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
  if (norm(text.slice(a + START.length, b)) !== norm(block)) {
    throw new Error('docs/PLAN.md progress block is stale: run `npm run plan:progress` and commit');
  }
  return block;
}

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  console.log(process.argv.includes('--check') ? checkPlan() : updatePlan());
}
