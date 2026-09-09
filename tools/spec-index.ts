/**
 * Parse the specification into an index of requirement ids, so code citations can be checked and
 * coverage can be counted (docs/ARCHITECTURE.md 7).
 *
 * Citation grammar: `<System> <Node>` where System is a short name (Money, Register, Clearing, ...)
 * or `Law N`, `XI-N`, `Bond`, `Derivative`, `Part XII`, `Appendix A/B/C`; Node is A1, A1.a, N5.b, ...
 */
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

export interface Requirement {
  /** Canonical citation, e.g. "Money C2.a". */
  readonly id: string;
  readonly system: string;
  readonly node: string;
  readonly form: 'REASON' | 'VERIFY' | 'FORBID' | 'NOTE';
  readonly line: number;
  readonly text: string;
}

export interface SpecIndex {
  readonly requirements: readonly Requirement[];
  readonly systems: readonly string[];
  readonly byId: ReadonlyMap<string, Requirement>;
  /** Citable headings that are not numbered requirements: laws, mechanisms, parts, appendices. */
  readonly headings: ReadonlySet<string>;
}

const here = dirname(fileURLToPath(import.meta.url));
export const SPEC_PATH = resolve(here, '..', 'docs', 'spec', 'PROJECT_PHOENIX.md');

/** Short names for the numbered systems, in the spec's own words. */
const SYSTEM_NAMES: Readonly<Record<string, string>> = {
  '1': 'Money',
  '2': 'Register',
  '3': 'Clearing',
  '4': 'Audit',
  '5': 'Seed',
  '6': 'Currency',
  '7': 'Corporate Credit',
  '8': 'Sovereign',
  '9': 'Short-Term Debt',
  '10': 'Equity',
  '11': 'Money Market',
  '12': 'Spot FX',
  '13': 'Fund Shares',
  '14': 'Securities Lending',
  '15': 'Prime Brokerage',
  '16': 'Derivative Layer',
  '17': 'CDS',
  '18': 'IRS',
  '19': 'FX Forwards',
  '20': 'Commodity Futures',
  '21': 'Commodities Spot',
  '22': 'Indices',
  '23': 'Banks Lending',
  '24': 'Banks Funding',
  '25': 'Banks Capital',
  '26': 'Dealer Desks',
  '27': 'Insurers',
  '28': 'Hedge Funds',
  '29': 'Private Equity',
  '30': 'Treasury',
  '31': 'Central Bank',
  '32': 'Firm',
  '33': 'Capital Programme',
  '34': 'Firm Birth',
  '35': 'M&A',
  '36': 'Trade Credit',
  '37': 'Goods',
  '38': 'Freight',
  '39': 'Labour',
  '40': 'Housing',
  '41': 'Households',
  '42': 'Small-Business Pools',
  '43': 'Cross-Border',
  '44': 'Ratings',
  '45': 'Observer',
  '46': 'Expectations',
  '47': 'Polity',
};

export function systemName(n: string): string {
  const s = SYSTEM_NAMES[n];
  if (s === undefined) throw new Error(`no short name for system ${n}`);
  return s;
}

export function buildSpecIndex(path: string = SPEC_PATH): SpecIndex {
  const text = readFileSync(path, 'utf8');
  const lines = text.split('\n');
  const requirements: Requirement[] = [];
  const headings = new Set<string>();
  const systems: string[] = [];
  let current: string | undefined;

  const systemHeading = /^## (\d+)\. /;
  const contractHeading = /^## THE (BOND|DERIVATIVE) /;
  const lawHeading = /^### (\d+)\. /;
  const mechHeading = /^## (XI-\d+)\./;
  const partHeading = /^# (PART [IVX]+)/;
  const appendixHeading = /^# (APPENDIX [ABC])/;
  const node = /^\s*- \*\*([A-Z]{1,2}\d+(?:\.[a-z])?)\*\*(?: (REASON|VERIFY|FORBID))?/;

  lines.forEach((raw, i) => {
    const line = raw.trimEnd();
    let m = systemHeading.exec(line);
    if (m?.[1] !== undefined) {
      current = systemName(m[1]);
      systems.push(current);
      headings.add(current);
      return;
    }
    m = contractHeading.exec(line);
    if (m?.[1] !== undefined) {
      current = m[1] === 'BOND' ? 'Bond' : 'Derivative';
      systems.push(current);
      headings.add(current);
      return;
    }
    m = lawHeading.exec(line);
    if (m?.[1] !== undefined && i < 300) {
      headings.add(`Law ${m[1]}`);
      return;
    }
    m = mechHeading.exec(line);
    if (m?.[1] !== undefined) {
      headings.add(m[1]);
      current = m[1];
      return;
    }
    m = partHeading.exec(line);
    if (m?.[1] !== undefined) {
      const part = m[1].replace('PART ', 'Part ');
      headings.add(part);
      if (part === 'Part XII' || part === 'Part XIII' || part === 'Part II') current = part;
      return;
    }
    m = appendixHeading.exec(line);
    if (m?.[1] !== undefined) {
      headings.add(m[1].replace('APPENDIX ', 'Appendix '));
      current = undefined;
      return;
    }
    m = node.exec(line);
    if (m?.[1] !== undefined && current !== undefined && !current.startsWith('Part')) {
      const id = `${current} ${m[1]}`;
      const form = (m[2] as Requirement['form'] | undefined) ?? 'NOTE';
      requirements.push({ id, system: current, node: m[1], form, line: i + 1, text: line.trim() });
    }
  });
  headings.add('Granularity');
  return {
    requirements,
    systems,
    byId: new Map(requirements.map((r) => [r.id, r])),
    headings,
  };
}

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  const idx = buildSpecIndex();
  const counts = { REASON: 0, VERIFY: 0, FORBID: 0, NOTE: 0 };
  for (const r of idx.requirements) counts[r.form] += 1;
  console.log(
    `${idx.requirements.length} requirements across ${idx.systems.length} systems and contracts`,
  );
  console.log(counts);
}
