/**
 * The government: who governs after the count, and what the parliament's mandate then says.
 *
 * @spec Polity C2 Polity C2.a Polity C3 Polity C3.a Polity C4 Law 2 Law 16
 *
 * C2 states the rule and this is the rule, not a version of it: the largest party adds the party
 * whose platform is NEAREST its own, and goes on adding until it holds a majority. Nothing is
 * negotiated, nobody is bought, and there is no portfolio to divide — a coalition here is a fact
 * about how close the platforms are, which is why C2's distance is a constitutional number and not
 * a deal (`coalitionMaxDistance`, §47 A1's kind of primitive).
 *
 * C2.a: a parliament where no such coalition exists is HUNG, and the standing mandate continues.
 * That is a real outcome and it is reported rather than repaired — there is no rule here that keeps
 * trying combinations until one works, because a parliament that cannot form a government is a
 * thing that happens.
 *
 * C3: the mandate is a READ of the parliament — for every number the parliament owns, the
 * SEAT-WEIGHTED position of the coalition's platforms. A coalition of two parties that disagree
 * about a rate governs at the number their seats between them come to, which is what a coalition
 * agreement is when nobody is allowed to invent one.
 */
import type { ParamId } from '../../core/ids.js';
import { div, mul, sum } from '../../core/num.js';
import { distanceBetween, spreadAcross } from '../../registry/platforms.js';

export interface Government {
  /** The platforms in it, largest first — the order they were added by the rule. */
  readonly members: readonly string[];
  readonly seats: number;
  /** C2.a: nobody could form a majority within the distance, so the standing mandate continues. */
  readonly hung: boolean;
}

/**
 * C2: the largest party, then the nearest platform to it that it may sit with, until a majority.
 *
 * "Nearest its own" is measured against how far apart all the platforms are (`distanceBetween`), so
 * the scale is the parties' own disagreement and never one this file invented. Ties are broken by
 * name, because the same parliament must always form the same government (determinism).
 */
export function formGovernment(
  seats: ReadonlyMap<string, number>,
  positions: ReadonlyMap<string, ReadonlyMap<ParamId, number>>,
  maxDistance: number,
  house: number,
): Government {
  const held = [...seats].filter(([, n]) => n > 0).sort((a, b) => (b[1] === a[1] ? a[0].localeCompare(b[0]) : b[1] - a[1]));
  const first = held[0];
  if (first === undefined || house <= 0) return { members: [], seats: 0, hung: true };
  const spread = spreadAcross(positions);
  const members = [first[0]];
  let taken = first[1];
  // A majority is MORE than half the house: half of an even house is not a majority, and a rule
  // that said "at least half" would hand government to a parliament that is exactly split.
  const majority = (n: number): boolean => mul(n, 2, 'both halves of the house') > house;
  while (!majority(taken)) {
    const mine = positions.get(members[0] ?? '');
    if (mine === undefined) break;
    let nearest: { id: string; at: number; seats: number } | undefined;
    for (const [id, n] of held) {
      if (members.includes(id)) continue;
      const theirs = positions.get(id);
      if (theirs === undefined) continue;
      const at = distanceBetween(mine, theirs, spread);
      if (at > maxDistance) continue;
      if (nearest === undefined || at < nearest.at || (at === nearest.at && id < nearest.id)) {
        nearest = { id, at, seats: n };
      }
    }
    // C2.a: there is nobody left it may sit with, and it is short of a majority. Hung.
    if (nearest === undefined) return { members, seats: taken, hung: true };
    members.push(nearest.id);
    taken += nearest.seats;
  }
  return { members, seats: taken, hung: false };
}

/**
 * C3: THE MANDATE — for every number parliament owns, the seat-weighted position of the parties in
 * government. One party governing alone gives its own platform; two give the number their seats
 * between them come to, which is the only honest reading of "what this coalition would do" when
 * nobody may invent a deal.
 *
 * A HUNG parliament produces no mandate at all (`undefined`), not an empty one: what governs then
 * is what was already standing, and saying so with a value of nothing would be a mandate that set
 * every number to zero.
 */
export function mandateOf(
  government: Government,
  seats: ReadonlyMap<string, number>,
  positions: ReadonlyMap<string, ReadonlyMap<ParamId, number>>,
): Map<ParamId, number> | undefined {
  if (government.hung || government.members.length === 0) return undefined;
  // Every member of a government holds seats — that is how it got in — so a member with none is a
  // caller that built a government the rule could not have produced, and it says so (Law 4).
  const weights: { id: string; seats: number }[] = [];
  for (const id of government.members) {
    const n = seats.get(id);
    if (n === undefined) return undefined;
    weights.push({ id, seats: n });
  }
  const total = sum(weights.map((w) => w.seats)).value;
  if (total <= 0) return undefined;
  const out = new Map<ParamId, number>();
  const first = positions.get(government.members[0] ?? '');
  if (first === undefined) return undefined;
  for (const id of first.keys()) {
    const terms: number[] = [];
    for (const w of weights) {
      const said = positions.get(w.id)?.get(id);
      if (said === undefined) continue;
      terms.push(mul(said, w.seats, 'what this party wants, over the seats it holds'));
    }
    out.set(id, div(sum(terms).value, total, 'what the coalition governs at'));
  }
  return out;
}
