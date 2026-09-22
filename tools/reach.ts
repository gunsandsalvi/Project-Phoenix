/**
 * WHAT THE WORLD ACTUALLY REACHES.
 *
 * A MET mark is a claim that the cited module implements the clause AND is reached. Being NAMED by
 * production code says nothing about the world calling it: a cluster inside one module that only
 * calls itself reads as live, and a module entered for one type carries every item beside it in.
 *
 * So the walk starts where a run starts — the registration table and the doors a binary opens on a
 * `World` — and takes the transitive closure over the names each body uses. What falls outside it
 * is code the assembled world never enters.
 *
 * A name is matched across the whole tree, so two modules declaring a `Route` reach each other's.
 * That is deliberate: the closure over-reaches rather than under-reaches, so everything it reports
 * is unreached under any resolution of the ambiguity.
 *
 * `#[cfg(test)]` blocks and `src/bin` are excluded: a test exercising a helper and a diagnostic
 * binary constructing its own inputs are not the world running it.
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, relative, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');
const KERNEL = resolve(root, 'packages', 'kernel-rs', 'src');

export interface Unreached {
  readonly file: string;
  readonly item: string;
}

/**
 * The doors a run opens, by module stem and item.
 *
 * A binary builds a `World` from the parameters `systems::declare` states, wires it to the rows
 * `systems::all` registers, and steps it. Nothing else is an entry: everything the world does, it
 * does from one of these.
 */
export const ENTRIES: readonly (readonly [string, string])[] = [
  ['systems', 'all'],
  ['systems', 'declare'],
  ['systems', 'Wiring'],
  ['assembly', 'RunConfig'],
  ['assembly', 'World'],
  ['assembly', 'with_parameters'],
  ['assembly', 'wire_up'],
  ['assembly', 'step'],
];

/** A public item, which is one a coverage row can cite and this check can refuse. */
const PUBLIC = /^[ \t]*pub(?:\([a-z]+\))?[ \t]+(?:fn|struct|enum|trait|const|type)[ \t]+(\w+)/;

/** Any item at all. A private helper is a node too: what it calls, its caller reaches. */
const DECLARES =
  /^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?(?:default[ \t]+)?(?:async[ \t]+)?(?:unsafe[ \t]+)?(?:extern[ \t]+"[^"]*"[ \t]+)?(?:const[ \t]+)?(?:fn|struct|enum|trait|type|static|union|const)[ \t]+([A-Za-z_]\w*)/;

/** `impl Trait for Thing` and `impl Thing` both belong to THING: a method is what its type does. */
const IMPL_FOR = /^[ \t]*impl\b.*\bfor[ \t]+&?(?:dyn[ \t]+)?([A-Za-z_][\w:]*)/;
const IMPL_TYPE = /^[ \t]*impl\b(?:[ \t]*<[^>]*>)?[ \t]+([A-Za-z_][\w:]*)/;

/** The source with every `#[cfg(test)]` block removed, by brace depth. */
export function withoutTests(text: string): string {
  let out = '';
  let at = 0;
  for (;;) {
    const marker = text.indexOf('#[cfg(test)]', at);
    if (marker === -1) return out + text.slice(at);
    out += text.slice(at, marker);
    let i = text.indexOf('{', marker);
    if (i === -1) return out;
    let depth = 0;
    for (; i < text.length; i += 1) {
      if (text[i] === '{') depth += 1;
      else if (text[i] === '}') {
        depth -= 1;
        if (depth === 0) break;
      }
    }
    at = i + 1;
  }
}

/** A line with its comment, string and character literals gone, so braces are the code's own. */
function code(line: string): string {
  let out = '';
  for (let i = 0; i < line.length; i += 1) {
    const c = line[i];
    if (c === '/' && line[i + 1] === '/') break;
    if (c === '"') {
      i += 1;
      while (i < line.length && line[i] !== '"') i += line[i] === '\\' ? 2 : 1;
      continue;
    }
    // A char literal, not a lifetime: `'x'` and `'\n'` close, `'a` in `&'a str` does not.
    if (c === "'" && (line[i + 2] === "'" || (line[i + 1] === '\\' && line[i + 3] === "'"))) {
      i += line[i + 1] === '\\' ? 3 : 2;
      continue;
    }
    out += c;
  }
  return out;
}

function owner(line: string): string | undefined {
  const declared = DECLARES.exec(line)?.[1];
  if (declared !== undefined) return declared;
  const implemented = (IMPL_FOR.exec(line) ?? IMPL_TYPE.exec(line))?.[1];
  return implemented?.split('::').pop();
}

/** Every identifier the line uses, which is every name an edge could run along. */
function names(line: string): string[] {
  return [...line.matchAll(/\b[A-Za-z_]\w*/g)].map((m) => m[0]);
}

function rustFiles(at: string, out: string[] = []): string[] {
  for (const entry of readdirSync(at)) {
    const path = resolve(at, entry);
    if (statSync(path).isDirectory()) rustFiles(path, out);
    else if (path.endsWith('.rs')) out.push(path);
  }
  return out;
}

interface Item {
  readonly key: string;
  readonly file: string;
  readonly stem: string;
  readonly item: string;
  readonly open: boolean;
}

interface Graph {
  readonly items: Map<string, Item>;
  /** Every item declaring a name, so a use of it reaches all of them. */
  readonly byName: Map<string, string[]>;
  /** The names each item's own body uses. */
  readonly uses: Map<string, Set<string>>;
}

/**
 * The call graph, at the granularity of a declared item.
 *
 * A body's names belong to the innermost item that encloses it, and an `impl` block's contents
 * belong to the type it is on — so what a mechanism does inside `run` is what that mechanism
 * reaches, even though `run` is a trait method nobody names.
 */
export function graph(kernel: string = KERNEL): Graph {
  const items = new Map<string, Item>();
  const byName = new Map<string, string[]>();
  const uses = new Map<string, Set<string>>();

  for (const file of rustFiles(kernel).filter((f) => !f.includes(`${'/'}bin${'/'}`))) {
    const stem = file.replace(/.*\//, '').replace(/\.rs$/, '');
    const shown = relative(root, file);
    const frames: { key: string; depth: number }[] = [];
    let pending: { key: string; depth: number } | null = null;
    let depth = 0;

    for (const raw of withoutTests(readFileSync(file, 'utf8')).split('\n')) {
      const line = code(raw);
      // A declaration nested inside another is a node only when a coverage row could cite it. A
      // trait method, an inherent helper and an `impl` body are what their TYPE does: attributing
      // them to themselves would leave everything a mechanism does inside `run` unreached, because
      // nothing names a trait method.
      const declared =
        frames.length === 0 || PUBLIC.test(line) ? owner(line) : undefined;
      if (declared !== undefined) {
        const key = `${shown}#${declared}`;
        if (!items.has(key)) {
          items.set(key, {
            key,
            file: shown,
            stem,
            item: declared,
            open: PUBLIC.test(line),
          });
          byName.set(declared, [...(byName.get(declared) ?? []), key]);
        }
        pending = { key, depth };
      }

      const mine = pending?.key ?? frames[frames.length - 1]?.key;
      if (mine !== undefined) {
        const seen = uses.get(mine) ?? new Set<string>();
        for (const n of names(line)) seen.add(n);
        uses.set(mine, seen);
      }

      let opened = false;
      let ended = false;
      let brackets = 0;
      for (const ch of line) {
        if (ch === '(' || ch === '[') brackets += 1;
        else if (ch === ')' || ch === ']') brackets -= 1;
        else if (ch === '{') {
          depth += 1;
          opened = true;
        } else if (ch === '}') depth -= 1;
        else if (ch === ';' && brackets === 0 && pending !== null && depth === pending.depth) {
          ended = true;
        }
      }

      if (pending !== null) {
        if (depth > pending.depth) {
          frames.push(pending);
          pending = null;
        } else if (opened || ended) pending = null;
      }
      while (frames.length > 0 && depth <= (frames[frames.length - 1]?.depth ?? 0)) frames.pop();
    }
  }
  return { items, byName, uses };
}

/** Every item the world enters, transitively, from the doors in `ENTRIES`. */
export function reached(g: Graph = graph()): Set<string> {
  const open: string[] = [];
  for (const [stem, item] of ENTRIES) {
    for (const key of g.byName.get(item) ?? []) {
      if (g.items.get(key)?.stem === stem) open.push(key);
    }
  }
  const seen = new Set(open);
  while (open.length > 0) {
    const key = open.pop() as string;
    for (const name of g.uses.get(key) ?? []) {
      for (const next of g.byName.get(name) ?? []) {
        if (seen.has(next)) continue;
        seen.add(next);
        open.push(next);
      }
    }
  }
  return seen;
}

/** Every public item outside the world's closure: declared, and never entered from a run. */
export function unreached(kernel: string = KERNEL): Unreached[] {
  const g = graph(kernel);
  const live = reached(g);
  const out: Unreached[] = [];
  for (const item of g.items.values()) {
    if (item.open && !live.has(item.key)) out.push({ file: item.file, item: item.item });
  }
  return out.sort((a, b) => a.file.localeCompare(b.file) || a.item.localeCompare(b.item));
}

/**
 * The modules nothing else reaches into.
 *
 * Tested on the module PATH rather than on its item names: two modules may declare a `Route` and
 * neither is evidence about the other. A `mod x;` declaration is not an entry either — it compiles
 * the file and calls nothing in it.
 */
export function deadModules(kernel: string = KERNEL): string[] {
  const all = rustFiles(kernel);
  const source = new Map(all.map((f) => [f, withoutTests(readFileSync(f, 'utf8'))]));
  const out: string[] = [];
  for (const [file, text] of source) {
    // A binary assembling the world is the world being entered, so the bins ask this question
    // even though they do not answer the reach one.
    if (file.includes(`${'/'}bin${'/'}`)) continue;
    const stem = file.replace(/.*\//, '').replace(/\.rs$/, '');
    if (stem === 'lib' || stem === 'mod') continue;
    if (!/^[ \t]*pub(?:\([a-z]+\))?[ \t]+(?:fn|struct|enum|trait|const|type)[ \t]+\w/m.test(text)) {
      continue;
    }
    const enters = new RegExp(`\\b${stem}::`);
    const entered = [...source].some(([other, body]) => other !== file && enters.test(body));
    if (!entered) out.push(relative(root, file));
  }
  return out.sort();
}

/**
 * The Rust items a coverage reason names. A row writes one as `worth`, as `kinds::TREASURY` or as
 * `currency_of(region)`, and all three are the same claim: this named thing is the evidence.
 */
export function itemsNamed(text: string): string[] {
  const out: string[] = [];
  for (const m of text.matchAll(/`([A-Za-z_][A-Za-z0-9_:]*)(?:\([^`]*\))?`/g)) {
    for (const segment of (m[1] ?? '').split('::')) if (segment !== '') out.push(segment);
  }
  return out;
}

/**
 * Every identifier the kernel source contains, tests and binaries included.
 *
 * Deliberately the whole text rather than the declarations: a variant, a field, a constant inside a
 * module and a name only a test uses are all things a row may legitimately cite. What this set is
 * for is the citation that names something the tree does not contain at all.
 */
export function namesInTree(kernel: string = KERNEL): Set<string> {
  const out = new Set<string>();
  for (const file of rustFiles(kernel)) {
    for (const m of readFileSync(file, 'utf8').matchAll(/\b[A-Za-z_]\w*/g)) out.add(m[0]);
  }
  return out;
}

/** A row whose citation names something no file in the tree contains. */
export interface AbsentCitation {
  readonly id: string;
  readonly status: string;
  readonly names: string;
}

/**
 * Every row citing a name the source does not have.
 *
 * A citation is the whole of a row's evidence, so one that resolves to nothing cannot be read
 * against the clause and cannot be wrong. It is the same defect as a claim on unreached code, one
 * step further: there is not even an item to fail to enter.
 */
export function absentCitations(
  rows: readonly { id: string; status: string; where: string }[],
  present: ReadonlySet<string>,
): AbsentCitation[] {
  const out: AbsentCitation[] = [];
  for (const r of rows) {
    for (const item of itemsNamed(r.where)) {
      if (!present.has(item)) out.push({ id: r.id, status: r.status, names: item });
    }
  }
  return out;
}

/** A row that claims a clause is met by code nothing reaches. */
export interface HollowClaim {
  readonly id: string;
  readonly names: string;
  readonly why: string;
}

export function hollowClaims(
  rows: readonly { id: string; status: string; where: string }[],
  dead: readonly Unreached[],
  closed: readonly string[],
): HollowClaim[] {
  const byItem = new Map(dead.map((d) => [d.item, d.file]));
  const out: HollowClaim[] = [];
  for (const r of rows) {
    if (r.status !== 'MET') continue;
    for (const item of itemsNamed(r.where)) {
      const file = byItem.get(item);
      if (file !== undefined) out.push({ id: r.id, names: item, why: `unreached, in ${file}` });
    }
    for (const path of closed) {
      if (r.where.includes(path)) {
        out.push({ id: r.id, names: path, why: 'no other module names anything it declares' });
      }
    }
  }
  return out;
}

/**
 * A RATCHET: public items the world's closure does not contain.
 *
 * Code the run never enters is a mechanism that was written and not wired, and the count of it is
 * the honest measure of how much of this tree is a library rather than a world. It falls and never
 * rises, and at zero the allowance is deleted and the rule is absolute.
 */
export const UNREACHED_ALLOWED = 468;

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const closed = deadModules();
  console.log(`${String(closed.length)} module(s) no other module enters:`);
  for (const file of closed) console.log(`  ${file}`);
  console.log('');
  const dead = unreached();
  const byFile = new Map<string, string[]>();
  for (const d of dead) byFile.set(d.file, [...(byFile.get(d.file) ?? []), d.item]);
  console.log(`${String(dead.length)} declared item(s) the world never enters:`);
  for (const [file, items] of [...byFile].sort()) {
    console.log(`  ${file.padEnd(46)} ${items.join(', ')}`);
  }
  console.log('');
  const said =
    dead.length > UNREACHED_ALLOWED
      ? `ROSE to ${String(dead.length)} — a ratchet only falls`
      : dead.length < UNREACHED_ALLOWED
        ? `has fallen to ${String(dead.length)}: lower the allowance, or the check goes slack`
        : `${String(dead.length)} left`;
  console.log(`[items the world never enters] allowance ${String(UNREACHED_ALLOWED)} · ${said}`);
  process.exitCode = dead.length === UNREACHED_ALLOWED ? 0 : 1;
}
