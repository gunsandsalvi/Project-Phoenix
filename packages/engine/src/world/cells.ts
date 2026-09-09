/**
 * Cell events (XI-15): the five ways a weight changes, and nothing else.
 *
 * @spec XI-15 Households F1.a Households F1.b Small-Business Pools A6.c Small-Business Pools E5 Appendix A
 *
 * A split is exact: the affected members become a new cell with the same per-member state. A merge
 * requires an identical key and identical state. Entry, death and promotion move the weight with a
 * cause and a date. Every event is journaled.
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import { forbid } from '../core/assert.js';
import { type PartyId, partyId } from '../core/ids.js';
import { positiveCount } from '../core/num.js';
import type { Journal } from '../journal/journal.js';
import type { CellParty, Parties, WeightEventKind } from '../parties/party.js';
import type { Register } from '../register/register.js';

export interface CellDeps {
  readonly parties: Parties;
  readonly register: Register;
  readonly journal: Journal;
}

/** Split `members` members off `cell` into a new cell with an identical state. Returns the new id. */
export function splitCell(
  cell: PartyId,
  members: number,
  cause: string,
  period: Period,
  cycle: Cycle,
  d: CellDeps,
): PartyId {
  const c = d.parties.cell(cell);
  positiveCount(members, 'members split');
  forbid(
    members < c.weight,
    'XI-15',
    `cannot split ${members} of ${c.weight} members off ${cell}; a whole cell moves as itself`,
  );
  const id = nextSplitId(c, d.parties);
  const fresh: CellParty = { ...c, id, name: `${c.name} / split ${id}`, weight: members };
  d.parties.add(fresh);
  d.register.copyMemberState(cell, id);
  d.parties.applyWeight({
    kind: 'split',
    party: cell,
    before: c.weight,
    after: c.weight - members,
    period,
    cause,
  });
  d.journal.record(period, cycle, 'weight', [cell, id], {
    kind: 'split',
    from: cell,
    to: id,
    members,
    cause,
  });
  return id;
}

/** Merge `b` into `a`: both must have the same key and identical per-member state. */
export function mergeCells(
  a: PartyId,
  b: PartyId,
  cause: string,
  period: Period,
  cycle: Cycle,
  d: CellDeps,
): void {
  const ca = d.parties.cell(a);
  const cb = d.parties.cell(b);
  forbid(a !== b, 'XI-15', 'a cell cannot merge with itself');
  forbid(sameKey(ca, cb), 'XI-15', `cells ${a} and ${b} have different keys and cannot merge`);
  forbid(
    sameState(a, b, d.register),
    'XI-15',
    `cells ${a} and ${b} differ in state and cannot merge`,
  );
  d.parties.applyWeight({
    kind: 'merge',
    party: a,
    before: ca.weight,
    after: ca.weight + cb.weight,
    period,
    cause,
  });
  d.register.forget(b);
  d.parties.cease(b, period, a);
  d.journal.record(period, cycle, 'weight', [a, b], {
    kind: 'merge',
    into: a,
    from: b,
    members: cb.weight,
    cause,
  });
}

/** Entry, death or promotion change a weight by a count of members, with a cause (XI-15). */
export function weightEvent(
  cell: PartyId,
  kind: Exclude<WeightEventKind, 'split' | 'merge'>,
  members: number,
  cause: string,
  period: Period,
  cycle: Cycle,
  d: CellDeps,
): void {
  const c = d.parties.cell(cell);
  positiveCount(members, `${kind} members`);
  const after = kind === 'entry' ? c.weight + members : c.weight - members;
  forbid(
    after > 0,
    'XI-15',
    `${kind} of ${members} would leave ${cell} with ${after} members; a cell of nobody is nobody's`,
  );
  d.parties.applyWeight({ kind, party: cell, before: c.weight, after, period, cause });
  d.journal.record(period, cycle, 'weight', [cell], {
    kind,
    members,
    before: c.weight,
    after,
    cause,
  });
}

function sameKey(a: CellParty, b: CellParty): boolean {
  return (
    a.key.region === b.key.region &&
    a.key.cohort === b.key.cohort &&
    a.key.bank === b.key.bank &&
    a.kind === b.kind
  );
}

function sameState(a: PartyId, b: PartyId, r: Register): boolean {
  const ha = r.holdingsOf(a);
  const hb = r.holdingsOf(b);
  if (ha.length !== hb.length) return false;
  for (const x of ha) {
    const y = hb.find((h) => h.instrument === x.instrument);
    if (y === undefined) return false;
    if (x.lots.length !== y.lots.length || x.liens.length !== y.liens.length) return false;
    for (let i = 0; i < x.lots.length; i += 1) {
      const p = x.lots[i];
      const q = y.lots[i];
      if (p === undefined || q === undefined) return false;
      if (p.qty !== q.qty || p.basisPerUnit !== q.basisPerUnit || p.acquired !== q.acquired)
        return false;
    }
  }
  const ea = r.hasEquityAccount(a) ? r.equity(a) : undefined;
  const eb = r.hasEquityAccount(b) ? r.equity(b) : undefined;
  return ea === eb;
}

function nextSplitId(c: CellParty, parties: Parties): PartyId {
  for (let n = 1; ; n += 1) {
    const id = partyId(`${c.id}.${n}`);
    if (!parties.has(id)) return id;
  }
}
