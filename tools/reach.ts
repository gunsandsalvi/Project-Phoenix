/**
 * WHAT THE WORLD ACTUALLY REACHES.
 *
 * A MET mark is a claim that the cited module implements the clause AND is reached. The citation
 * check proves the path resolves; nothing proved the code behind it ever runs. Two systems were
 * marked complete on modules the assembled world does not call.
 *
 * An item is reached when production code outside its own definition names it. `#[cfg(test)]`
 * blocks and `src/bin` are excluded: a test exercising a helper and a diagnostic binary
 * constructing its own inputs are not the world running it.
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

const DECLARES = /^[ \t]*pub(?:\([a-z]+\))?[ \t]+(?:fn|struct|enum|trait|const|type)[ \t]+(\w+)/gm;

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

function rustFiles(at: string, out: string[] = []): string[] {
  for (const entry of readdirSync(at)) {
    const path = resolve(at, entry);
    if (statSync(path).isDirectory()) rustFiles(path, out);
    else if (path.endsWith('.rs')) out.push(path);
  }
  return out;
}

/** Every declared item production code never names outside its own definition. */
export function unreached(kernel: string = KERNEL): Unreached[] {
  const files = rustFiles(kernel).filter((f) => !f.includes(`${'/'}bin${'/'}`));
  const source = new Map(files.map((f) => [f, withoutTests(readFileSync(f, 'utf8'))]));
  const out: Unreached[] = [];
  for (const [file, text] of source) {
    const declared = new Set(Array.from(text.matchAll(DECLARES), (m) => m[1] ?? ''));
    for (const item of declared) {
      if (item === '') continue;
      const names = new RegExp(`\\b${item}\\b`, 'g');
      const defines = new RegExp(
        `^[ \\t]*pub(?:\\([a-z]+\\))?[ \\t]+(?:fn|struct|enum|trait|const|type)[ \\t]+${item}\\b`,
        'gm',
      );
      let uses = 0;
      for (const [other, body] of source) {
        const found = (body.match(names) ?? []).length;
        uses += other === file ? found - (body.match(defines) ?? []).length : found;
      }
      if (uses <= 0) out.push({ file: relative(root, file), item });
    }
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
    if (!DECLARES.test(text)) continue;
    DECLARES.lastIndex = 0;
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

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const closed = deadModules();
  console.log(`${String(closed.length)} module(s) no other module enters:`);
  for (const file of closed) console.log(`  ${file}`);
  console.log('');
  const dead = unreached();
  const byFile = new Map<string, string[]>();
  for (const d of dead) byFile.set(d.file, [...(byFile.get(d.file) ?? []), d.item]);
  console.log(`${String(dead.length)} declared item(s) no production code names:`);
  for (const [file, items] of [...byFile].sort()) {
    console.log(`  ${file.padEnd(46)} ${items.join(', ')}`);
  }
}
