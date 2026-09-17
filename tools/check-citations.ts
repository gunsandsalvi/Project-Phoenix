/**
 * Every `@spec` tag in the engine must name a requirement, law, mechanism or heading that exists in
 * the specification. A citation that does not resolve is a stale reference (Law 16) and fails the
 * build. Also verifies `docs/COVERAGE.md` cites only real requirements.
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';
import { buildSpecIndex, type SpecIndex } from './spec-index.js';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');

export function listTs(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) out.push(...listTs(p));
    else if (p.endsWith('.ts')) out.push(p);
  }
  return out;
}

/** Split an `@spec` tag's text into citations: "Money C2 C2.a Law 7 XI-15 Appendix A". */
export function parseCitations(tag: string): string[] {
  const tokens = tag.trim().split(/\s+/);
  const out: string[] = [];
  let system: string[] = [];
  let emitted = false; // a node has been emitted for the current system name
  const nodeRe = /^[A-Z]{1,2}\d+(?:\.[a-z])?$/;
  const sectionRe = /^[A-Z]$/;
  const flush = (): void => {
    if (system.length > 0 && !emitted) out.push(system.join(' '));
    system = [];
    emitted = false;
  };
  for (const t of tokens) {
    if (/^XI-\d+$/.test(t)) {
      flush();
      out.push(t);
    } else if (t === 'Law' || t === 'Part' || t === 'Appendix') {
      flush();
      system = [t];
    } else if (
      system.length === 1 &&
      (system[0] === 'Law' || system[0] === 'Part' || system[0] === 'Appendix')
    ) {
      system.push(t);
      flush();
    } else if (nodeRe.test(t) || sectionRe.test(t)) {
      // 21.92: a SECTION is a place in the spec too — "Sovereign D" is where the borrowing
      // constraint lives, and a module's or a phase's `spec` cites one as readily as a node.
      out.push(`${system.join(' ')} ${t}`);
      emitted = true;
    } else if (emitted) {
      // A new system name begins after a node: "Money C2 Register A1".
      system = [t];
      emitted = false;
    } else {
      system.push(t);
    }
  }
  flush();
  return out;
}

export function resolves(idx: SpecIndex, citation: string): boolean {
  if (idx.byId.has(citation)) return true;
  if (idx.headings.has(citation)) return true;
  // A system name alone cites the whole system.
  if (idx.systems.includes(citation)) return true;
  // 21.92: and a SECTION of one — "Sovereign D" — where that system has a node under that letter.
  const section = /^(.+) ([A-Z])$/.exec(citation);
  if (section !== null) {
    const [, system, letter] = section;
    if (system !== undefined && letter !== undefined && idx.systems.includes(system)) {
      for (const id of idx.byId.keys()) {
        if (id.startsWith(`${system} ${letter}`) && /\d/.test(id.slice(system.length + 2, system.length + 3))) {
          return true;
        }
      }
    }
  }
  return false;
}

export function checkFile(idx: SpecIndex, path: string): string[] {
  const text = readFileSync(path, 'utf8');
  const problems: string[] = [];
  const re = /@spec\s+([^\n*]+)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    const tag = m[1];
    if (tag === undefined) continue;
    for (const c of parseCitations(tag)) {
      if (!resolves(idx, c)) problems.push(`${path}: citation "${c}" does not resolve`);
    }
  }
  return problems;
}

/**
 * 0h.3, 21.92: AND EVERY CITATION THAT IS NOT IN AN `@spec` TAG. A phase, an audit family and every
 * violation carry a `spec:` string, and those are what a READER of a finding is given — "[Money
 * C1.a] a payment from an account to itself is not a payment". Nothing checked them, so the
 * liveness family was written citing `Audit B10`, which does not exist (§Audit's own run B1 to B8),
 * and every gate stayed green. It was caught by reading the spec, which is the thing this checker
 * exists to make unnecessary; a rule that can be a check should be one.
 */
/**
 * The citations in a `spec` field that do not resolve, where a COMMA SEPARATES CITATIONS: a field is
 * written as a reader would say it — "Money Market, Banks Funding, Central Bank B, D" — so each
 * piece is a citation, and a piece that names no system carries the last one's ("D" is Central
 * Bank D). It is the same grammar an `@spec` tag has with the punctuation a sentence has.
 */
export function unresolved(idx: SpecIndex, tag: string): string[] {
  const out: string[] = [];
  let system = '';
  for (const piece of tag.split(/[,;]/)) {
    const text = piece.trim();
    if (text.length === 0) continue;
    const whole = parseCitations(text);
    const bad = whole.filter((c) => !resolves(idx, c));
    if (bad.length === 0) {
      const named = /^([A-Za-z][A-Za-z ]*?)(?: [A-Z]\d|$| [A-Z]$)/.exec(text);
      if (named?.[1] !== undefined) system = named[1].trim();
      continue;
    }
    // A piece that resolves under the system the last one named — "…, D" after "Central Bank B".
    const carried = parseCitations(`${system} ${text}`);
    if (system.length > 0 && carried.every((c) => resolves(idx, c))) continue;
    out.push(...bad);
  }
  return out;
}

export function checkSpecStrings(idx: SpecIndex, path: string): string[] {
  const text = readFileSync(path, 'utf8');
  const problems: string[] = [];
  const re = /\bspec:\s*'([^']+)'/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    const tag = m[1];
    if (tag === undefined) continue;
    problems.push(...unresolved(idx, tag).map((c) => `${path}: spec "${c}" does not resolve`));
  }
  return problems;
}

export function checkCoverage(idx: SpecIndex, path: string): string[] {
  const problems: string[] = [];
  let text: string;
  try {
    text = readFileSync(path, 'utf8');
  } catch {
    return [`${path}: missing`];
  }
  const row = /^\| `([^`]+)` \|/gm;
  let m: RegExpExecArray | null;
  while ((m = row.exec(text)) !== null) {
    const c = m[1];
    if (c !== undefined && !resolves(idx, c))
      problems.push(`${path}: coverage row "${c}" does not resolve`);
  }
  return problems;
}

export function run(): { problems: string[]; citations: number } {
  const idx = buildSpecIndex();
  const files = listTs(resolve(root, 'packages', 'engine', 'src'));
  const problems: string[] = [];
  let citations = 0;
  for (const f of files) {
    const text = readFileSync(f, 'utf8');
    citations += (text.match(/@spec/g) ?? []).length;
    citations += (text.match(/\bspec:\s*'/g) ?? []).length;
    problems.push(...checkFile(idx, f));
    problems.push(...checkSpecStrings(idx, f));
  }
  problems.push(...checkCoverage(idx, resolve(root, 'docs', 'COVERAGE.md')));
  return { problems, citations };
}

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const { problems, citations } = run();
  if (problems.length > 0) {
    for (const p of problems) console.error(p);
    console.error(`${problems.length} citation problem(s)`);
    process.exit(1);
  }
  console.log(`all ${citations} citations resolve (@spec tags and the spec: a phase, a family and a violation carry)`);
}
