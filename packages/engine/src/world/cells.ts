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
  d.journal.record(
    period,
    cycle,
    'weight',
    [cell, id],
    {
      kind: 'split',
      from: cell,
      to: id,
      members,
      cause,
    },
    true,
  );
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
  forbid(
    d.parties.sameKey(ca, cb) && ca.kind === cb.kind,
    'XI-15',
    `cells ${a} and ${b} have different keys and cannot merge`,
  );
  d.parties.applyWeight({
    kind: 'merge',
    party: a,
    before: ca.weight,
    after: ca.weight + cb.weight,
    period,
    cause,
  });
  // XI-15: the register guards its own store, and it refuses a merge of two cells that differ.
  d.register.forget(b, a);
  d.parties.cease(b, period, a);
  d.journal.record(
    period,
    cycle,
    'weight',
    [a, b],
    {
      kind: 'merge',
      into: a,
      from: b,
      members: cb.weight,
      cause,
    },
    true,
  );
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
  d.journal.record(
    period,
    cycle,
    'weight',
    [cell],
    {
      kind,
      members,
      before: c.weight,
      after,
      cause,
    },
    true,
  );
}

/**
 * XI-15, Households F1, F1.a (13d.1): A SPLIT THAT CHANGES THE KEY, which is how a weight moves
 * between keys without value moving with it.
 *
 * What makes two households different is in the cell's key, so getting older, retiring or buying a
 * roof is a cell moving from one key to another. It cannot be a transfer: a cell carries its
 * holdings PER MEMBER in whole pieces, and a leg's quantity must equal `perMember × weight` on each
 * side at once, which two weights in the millions share no useful number for. It cannot be a merge
 * either, because a merge is a renaming and refuses two cells whose state differs.
 *
 * So it is a SPLIT — exact, per-member state and all, totals preserved by construction — with a
 * different key on the part that moved. Nobody appears, nobody disappears, and nothing crosses.
 *
 * The weight event is a PROMOTION, which is what XI-15 has the word for: the members did not enter,
 * they did not die, and the cell they left is not being renamed. It is the fifth thing that happens
 * to a weight, and this is the first thing in this world that does it.
 */
export function reKeyCell(
  cell: PartyId,
  members: number,
  key: Readonly<Record<string, string>>,
  cause: string,
  period: Period,
  cycle: Cycle,
  d: CellDeps,
): PartyId {
  const c = d.parties.cell(cell);
  positiveCount(members, 'members re-keyed');
  forbid(
    members < c.weight,
    'XI-15',
    `cannot re-key ${members} of ${c.weight} members of ${cell}; a whole cell moves as itself`,
  );
  forbid(
    Object.keys(key).every((k) => k in c.key),
    'XI-15',
    `re-key of ${cell} names a dimension its key does not have`,
    { key },
  );
  const id = nextSplitId(c, d.parties);
  const fresh: CellParty = {
    ...c,
    id,
    name: `${c.name} / ${cause}`,
    weight: members,
    key: { ...c.key, ...key },
  };
  d.parties.add(fresh);
  d.register.copyMemberState(cell, id);
  d.parties.applyWeight({
    kind: 'promotion',
    party: cell,
    before: c.weight,
    after: c.weight - members,
    period,
    cause,
  });
  d.journal.record(
    period,
    cycle,
    'weight',
    [cell, id],
    { kind: 'promotion', from: cell, to: id, members, key, cause },
    true,
  );
  return id;
}

function nextSplitId(c: CellParty, parties: Parties): PartyId {
  for (let n = 1; ; n += 1) {
    const id = partyId(`${c.id}.${n}`);
    if (!parties.has(id)) return id;
  }
}
