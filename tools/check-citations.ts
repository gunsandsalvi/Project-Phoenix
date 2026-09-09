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
    } else if (nodeRe.test(t)) {
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
    problems.push(...checkFile(idx, f));
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
  console.log(`all ${citations} @spec tags resolve`);
}
