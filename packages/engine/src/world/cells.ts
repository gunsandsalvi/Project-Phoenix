/**
 * Cell events (XI-15): the five ways a weight changes, and nothing else.
 *
 * @spec XI-15 Households F1.a Households F1.b Small-Business Pools A6.c Small-Business Pools E5 Appendix A
 *
 * A re-key moves `floor(total × n / weight)` pieces of every holding to the cell on the new key
 * (0f.4); a merge requires an identical key, adds totals and weights, and carries what the mover
 * issued (0f.10). Entry, death and promotion move the weight with a cause and a date. Every event
 * is journaled.
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import { forbid } from '../core/assert.js';
import { type PartyId, partyId } from '../core/ids.js';
import { positiveCount } from '../core/num.js';
import type { Journal } from '../journal/journal.js';
import type { CellParty, Parties, WeightEventKind } from '../parties/party.js';
import type { Register } from '../register/register.js';
import type { Instruments } from '../register/instruments.js';
import { succeedAgreements, type SuccessionDeps } from './succession.js';

/** Register F2: a cell is a party, so what it owed moves with it when it ceases (`succession`). */
export interface CellDeps extends SuccessionDeps {
  readonly parties: Parties;
  readonly register: Register;
  readonly instruments: Instruments;
  readonly journal: Journal;
}


/** Merge `b` into `a`: both must have the same key; totals and weights add, and what `b` issued is `a`'s to owe. */
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
  /**
   * A-3: THE REGISTER FIRST. This used to grow the absorbing cell's weight and THEN ask whether the
   * merge was legal at all — so a merge the register refuses had already moved a weight, and the
   * journal entry that would have said where it came from comes after the throw. The throw stops
   * the run so nothing persisted, but the order was backwards and it costs nothing to put right:
   * ask, apply, journal.
   */
  const moved = d.register.merge(a, b, d.parties.cell(a).weight, d.parties.cell(b).weight, period, cycle);
  d.parties.applyWeight({
    kind: 'merge',
    party: a,
    before: ca.weight,
    after: ca.weight + cb.weight,
    period,
    cause,
  });
  d.parties.cease(b, period, a);
  /**
   * XI-8, Register B3, 0f.10: WHAT THE MOVER ISSUED IS THE ABSORBING CELL'S TO OWE. A cell that
   * borrowed (0f.7b) and then merged left its loan naming an issuer that had ceased, and the next
   * maturity was refused at settlement (Money E4). The row is per (lender, cell); the cell it is
   * on is the one standing. A cell of borrowers and a cell of the debt-free sharing one key is a
   * lattice question, positioned at 12a.4 (a `leverage` band on the household lattice).
   */
  for (const i of d.instruments.issuedBy(b)) {
    if (i.status.live) d.instruments.reseat(i.id, a);
  }
  succeedAgreements(b, a, period, cycle, d);
  d.journal.record(
    period,
    cycle,
    'weight',
    [a, b],
    {
      kind: 'merge',
      /**
       * XI-15, Money D3 (21.100): WHERE THE BOOK WENT IS `to`, in the word every other weight event
       * uses. This said `into`, and the flows family reads `to` — so a merge's destination was
       * credited with nothing and every merge in this world reported as a holding that moved with
       * no leg behind it: three landlord cells of two hundred members became one of six hundred in
       * period 3 of the scale model, their eight billion dwellings came with them, and `Money D3`
       * called it unexplained. One fact, one name.
       */
      to: a,
      into: a,
      from: b,
      members: cb.weight,
      cause,
      moved: Object.fromEntries(moved),
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
  const moved = d.register.moveShare(cell, id, members, c.weight, period, cycle);
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
    // 0f.6: WHAT MOVED, per instrument, so the flows family reads it as a leg's worth of
    // explanation on both sides rather than inferring a copy (Law 19).
    { kind: 'promotion', from: cell, to: id, members, key, cause, moved: Object.fromEntries(moved) },
    true,
  );
  return id;
}

/**
 * XI-15, Households F1.b, F2, Appendix B (13d.1): A CELL WHOSE PEOPLE HAVE ALL DIED, and it may only
 * die EMPTY.
 *
 * The five events move a weight and none of them takes one to nothing, which is right — a cell of
 * nobody is nobody's. This is the other thing that can happen to a cell: it ends. The death of every
 * member it stands for, journaled as the weight event it is, and the party ceasing to a NAMED
 * SUCCESSOR the way every other party that ceases does (Register F2).
 *
 * It refuses a cell that still holds something. "No death without a destination" and "no residual
 * with no holder" are one rule read twice, and the place to enforce it is here rather than in
 * whichever module happened to call: what the dead held must already have gone somewhere by name.
 */
export function dieCell(
  cell: PartyId,
  successor: PartyId,
  cause: string,
  period: Period,
  cycle: Cycle,
  d: CellDeps,
): void {
  forbid(
    d.register.holdingsOf(cell).length === 0,
    'Appendix B',
    `${cell} still holds something and cannot die; what the dead held goes to somebody by name first`,
  );
  ceaseCell(cell, successor, cause, period, cycle, d);
}

/**
 * XI-15, Small-Business Pools E5 (11.5): A CELL THAT CEASES IS A DEATH OF ALL ITS MEMBERS, whatever
 * ceased it — the one writer of that weight event. A cell that FAILED on its cash went to its
 * estate through the kernel's `cease`, which recorded the party ceasing and nothing about its
 * weight, so a period in which forty-nine firms failed was a period in which the population fell
 * by forty-nine with no event behind it, and the units family said so every time. The estate is
 * the successor and takes the book; the members are gone, and the record says how many and why.
 */
export function ceaseCell(
  cell: PartyId,
  successor: PartyId,
  cause: string,
  period: Period,
  cycle: Cycle,
  d: CellDeps,
): void {
  const c = d.parties.cell(cell);
  d.journal.record(
    period,
    cycle,
    'weight',
    [cell, successor],
    { kind: 'death', members: c.weight, before: c.weight, after: 0, successor, cause },
    true,
  );
  // Register F2, Money E4 (12a.5): WHAT THE DEAD OWED IS THE SUCCESSOR'S TO OWE — its loan rows and
  // its arrears — as a merge and a promotion already did; a coupon addressed to the cell after it
  // ceased was Money E4's refusal, and the four-country world stopped there in period 7.
  for (const i of d.instruments.issuedBy(cell)) {
    if (i.status.live) d.instruments.reseat(i.id, successor);
  }
  d.parties.cease(cell, period, successor);
  succeedAgreements(cell, successor, period, cycle, d);
}

function nextSplitId(c: CellParty, parties: Parties): PartyId {
  for (let n = 1; ; n += 1) {
    const id = partyId(`${c.id}.${n}`);
    if (!parties.has(id)) return id;
  }
}

/**
 * XI-15, Small-Business Pools A6.c, Firm Birth A1 (12.4): PROMOTION OUT OF THE POPULATION. The
 * members leave the cell with their share of every lot and become the named party `to` — which
 * entered this period and holds nothing, so what arrives is theirs and nobody's else. The weight
 * event carries before and after, because unlike a re-key the population of cells falls by what
 * left (the units family reads exactly that); it says what moved, per instrument, so the flows
 * family reads the explanation on both sides. When the whole cell goes it ceases with `to` as its
 * successor, and what it had agreed passes to the party it became.
 */
export function promoteCell(
  cell: PartyId,
  members: number,
  to: PartyId,
  cause: string,
  period: Period,
  cycle: Cycle,
  d: CellDeps,
): void {
  const c = d.parties.cell(cell);
  positiveCount(members, 'members promoted');
  forbid(members <= c.weight, 'XI-15', `cannot promote ${members} of ${c.weight} members of ${cell}`);
  const target = d.parties.get(to);
  forbid(target.representation === 'named', 'A6.c', `${to} is a cell; a promotion makes a named party`);
  forbid(target.status.alive, 'XI-3', `${to} is not alive to be promoted into`);
  forbid(d.register.holdingsOf(to).length === 0, 'XI-15', `${to} already holds something; a promotion arrives with the members' own pieces`);
  const moved = d.register.moveShare(cell, to, members, c.weight, period, cycle);
  const after = c.weight - members;
  // A cell of nobody is nobody's (XI-15): the last member does not leave a weight of zero behind,
  // the cell ceases into the party it became, and the event says so with before and after.
  if (after > 0) d.parties.applyWeight({ kind: 'promotion', party: cell, before: c.weight, after, period, cause });
  d.journal.record(
    period,
    cycle,
    'weight',
    [cell, to],
    { kind: 'promotion', from: cell, to, members, before: c.weight, after, cause, moved: Object.fromEntries(moved) },
    true,
  );
  if (after === 0) {
    // What the cell issued — its loan rows — is the party's to owe now, like a merge: a coupon
    // addressed to the cell after it ceased is Money E4's refusal, and the world stopped there.
    for (const i of d.instruments.issuedBy(cell)) {
      if (i.status.live) d.instruments.reseat(i.id, to);
    }
    d.parties.cease(cell, period, to);
    succeedAgreements(cell, to, period, cycle, d);
    d.journal.record(period, cycle, 'party.ceased', [cell, to], { successor: to, cause }, true);
  }
}
