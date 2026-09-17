/**
 * The vote: one cell, every platform applied to its own state, and the one that leaves it best off.
 *
 * @spec Polity A3 Polity B1 Polity B1.a Polity B2 Polity B2.a Polity B2.b Polity B3 XI-15 XI-17 Expectations A2 Law 2
 *
 * B1.a is the whole design: a cell votes on WHAT IT EXPERIENCED and never on a published statistic.
 * So the vote is not handed a view of the world at all — it is handed `WhatACellKnows`, which is
 * its own outlook of its own income, the wage its own row pays and the people it stands for, and
 * there is no door in it onto an unemployment rate, an inflation rate or a sentiment index. That is
 * what `withoutAggregates` means here: not a rule anybody remembers, a shape with nothing else in it.
 *
 * B2: it compares each platform by what that platform would LEAVE IT WITH, at its own expectation —
 * the pay it expects, less what the platform's rates take of it, less what it would put into a
 * pension, plus what the platform would transfer to it, over what the platform's consumption tax
 * makes things cost. Every term is one of the platform's own stated positions against one of the
 * cell's own numbers, and nothing is weighted by a coefficient: the comparison IS the arithmetic.
 *
 * B2.a: there is no turnout number, no swing and no share. B2.b: a cell that would be in exactly
 * the same position whichever of them governed has nothing to vote about and stays home, and that
 * is the only abstention there is. A cell that cannot tell two BEST platforms apart is in the same
 * position — it has no reason to prefer either, so it does not pretend to.
 */
import type { ParamId, PartyId } from '../../core/ids.js';
import { absolute, asCash, asRatio, minus, plus, scale, type Cash } from '../../core/measure.js';
import { dustOf, largest, withinDust } from '../../core/num.js';

/**
 * B1, B1.a: EVERYTHING A CELL MAY VOTE ON, and it is a small list on purpose. Its own expectation
 * of its own income, and how many of it there are. A door onto anything published would be a door
 * onto a number it did not experience, so there is not one.
 */
export interface WhatACellKnows {
  readonly cell: PartyId;
  /** XI-15: how many households this cell IS. Its votes, because one household has one vote (A3). */
  readonly weight: number;
  /** §46 A2: what it expects to be paid in a period, its own outlook of its own income. */
  readonly expects: Cash | undefined;
}

/** What a platform would leave this cell with, and the numbers behind it (B2). */
export interface Position {
  readonly platform: string;
  readonly left: Cash;
}

/** A3, B3: what one cell did, and how many votes went with it. */
export interface Ballot {
  readonly cell: PartyId;
  /** XI-15: how many households it is — what it COULD cast, whether or not it did. */
  readonly weight: number;
  /** B2.b: nothing when it was in the same position under all of them — it stayed home. */
  readonly voted: string | undefined;
  readonly votes: number;
  readonly positions: readonly Position[];
}

/** The parliament-owned numbers a cell's own position actually turns on (B1, B2). */
export interface WhatItTurnsOn {
  readonly income: ParamId;
  readonly consumption: ParamId;
  readonly transfers: ParamId;
  readonly pension: ParamId;
}

/**
 * B2: WHAT THIS PLATFORM WOULD LEAVE THIS CELL WITH, a period, at the cell's own expectation.
 *
 * Its expected pay, less the share of it this platform's income tax takes and the share it would
 * put into a pension, plus what this platform would transfer to each of its members — all of that
 * over what this platform's consumption tax makes a basket cost, because money in hand is worth
 * what it buys. Nothing here is a utility function and nothing is weighted: it is money, per
 * member, under one platform's stated numbers.
 *
 * WHAT IS NOT IN IT, and it is named rather than assumed: interest, gains, the severance a firing
 * would pay it, the age it may retire at and what a planning release would do to the rent it pays.
 * Each is a real difference between these parties and each needs a read a cell does not have yet —
 * its own interest receipts, its own age against a threshold, its own rent against a stock of
 * houses. A cell votes on what it can evaluate from its own state, and what it cannot evaluate it
 * does not pretend to (finding 21.85).
 */
export function positionUnder(
  knows: WhatACellKnows,
  platform: string,
  said: ReadonlyMap<ParamId, number>,
  on: WhatItTurnsOn,
): Position | undefined {
  const expects = knows.expects;
  if (expects === undefined) return undefined;
  const income = said.get(on.income);
  const consumption = said.get(on.consumption);
  const transfers = said.get(on.transfers);
  const pension = said.get(on.pension);
  if (income === undefined || consumption === undefined) return undefined;
  if (transfers === undefined || pension === undefined) return undefined;
  const keeps = scale(
    expects,
    asRatio(1 - income - pension, 'what it keeps of what it is paid'),
    'what it keeps',
  );
  const handed = plus(
    keeps,
    asCash(transfers, expects.ccy, 'what this platform would transfer to each of its members'),
    'and what the state hands it',
  );
  return {
    platform,
    left: scale(
      handed,
      asRatio(1 / (1 + consumption), 'what it buys at this platform’s prices'),
      'what this platform leaves it with',
    ),
  };
}

/**
 * B2, B2.b, B3: THE BALLOT. The platform that leaves it best off takes its votes — all of them,
 * because a cell is one possible household with a multiplicity and one household votes once (A3,
 * XI-15). A cell in the same position under all of them, or under both of the best, stays home.
 */
export function ballotOf(
  knows: WhatACellKnows,
  platforms: ReadonlyMap<string, ReadonlyMap<ParamId, number>>,
  on: WhatItTurnsOn,
): Ballot {
  const positions: Position[] = [];
  for (const [platform, said] of platforms) {
    const p = positionUnder(knows, platform, said, on);
    if (p !== undefined) positions.push(p);
  }
  if (positions.length === 0) {
    return { cell: knows.cell, weight: knows.weight, voted: undefined, votes: 0, positions };
  }
  // Law 7: the same position means the same to arithmetic dust of the numbers that made it, and
  // never to a band somebody chose — two platforms that differ by less than the dust of this cell's
  // own money are two platforms it cannot tell apart.
  // Law 7: the dust is derived from the terms and the MAGNITUDE of what was compared — `largest`
  // is core's read of how big the biggest of them is, which is not a bound on anything (core/num).
  const dust = dustOf(
    positions.length + 1,
    largest(
      positions.map((p) => absolute(p.left, 'how big this position is').pieces),
      'the biggest position it compared',
    ),
  );
  let best = positions[0];
  if (best === undefined) {
    return { cell: knows.cell, weight: knows.weight, voted: undefined, votes: 0, positions };
  }
  let tied = false;
  for (const p of positions.slice(1)) {
    if (withinDust(p.left.pieces, best.left.pieces, dust)) {
      tied = true;
      continue;
    }
    if (p.left.pieces > best.left.pieces) {
      best = p;
      tied = false;
    }
  }
  // B2.b: indifferent is not a rate and not a share — it is this cell, in this state, with nothing
  // to choose between. Its votes are not cast and the turnout that results is a READ of that.
  if (tied) {
    return { cell: knows.cell, weight: knows.weight, voted: undefined, votes: 0, positions };
  }
  return {
    cell: knows.cell,
    weight: knows.weight,
    voted: best.platform,
    votes: knows.weight,
    positions,
  };
}

/** B3: Σ vote(xᵢ)·wᵢ — the cells' own ballots, summed. Never a sector's mean voter. */
export function tally(ballots: readonly Ballot[]): Map<string, number> {
  const out = new Map<string, number>();
  for (const b of ballots) {
    if (b.voted === undefined || b.votes <= 0) continue;
    const had = out.get(b.voted);
    out.set(b.voted, had === undefined ? b.votes : had + b.votes);
  }
  return out;
}

/** B2.a: turnout is a READ of who voted against who could have, and never a number anybody set. */
export function turnoutOf(ballots: readonly Ballot[]): { cast: number; able: number } {
  let cast = 0;
  let able = 0;
  for (const b of ballots) {
    // Who COULD have voted is every household in a cell that had platforms to compare at all; who
    // did is the ones whose cells found a difference between them. Turnout is the two, read.
    if (b.positions.length > 0) able += b.weight;
    if (b.voted !== undefined) cast += b.votes;
  }
  return { cast, able };
}

/** What the weighted difference between two positions comes to, for a reader (B4's measurement). */
export const spreadOf = (a: Cash, b: Cash): Cash => minus(a, b, 'what one platform leaves it over another');
