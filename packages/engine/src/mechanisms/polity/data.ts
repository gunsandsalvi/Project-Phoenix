/**
 * The constitution: the numbers a polity is made of, and the rule that turns votes into seats.
 *
 * @spec Polity A1 Polity A2 Polity A4 Polity C1 Polity C2 Polity C4 Polity D5 XI-17 Law 2 Law 15
 *
 * These are the primitives §47 names and they are POLICY of the CONSTITUTION — not parliament's,
 * which is the point of a constitution: the body a rule elects does not get to rewrite the rule
 * that elected it. Parliament owns the tax rates, the transfers and the target; it does not own the
 * seat count, the term, the allotment rule or the lag, and the register refuses it if it tries
 * (19.1: a setter that is not the declared owner).
 *
 * THE ALLOTMENT RULE IS A DISPATCH TABLE (Law 15). How votes become seats is exactly the kind of
 * thing that differs between polities and must not be a branch inside a mechanism: the rule is DATA
 * with a function beside it, chosen by an id in the register, and a world that wants first-past-the-
 * post picks the other row rather than editing the count.
 */
import { paramId, type ParamId } from '../../core/ids.js';
import { asRatio, type Ratio } from '../../core/measure.js';
import { div, mul, sub, sum } from '../../core/num.js';

export const POLITY_PARAMS = {
  /** A1: how many seats there are. A constitution's number, and the first thing one states. */
  seats: paramId('polity.seats'),
  /** A4: how long a parliament sits before there is another election — in MONTHS, placed by date. */
  termMonths: paramId('polity.termMonths'),
  /** C1: which rule turns votes into seats. An id into the table below, never a branch. */
  allotmentRule: paramId('polity.allotmentRule'),
  /**
   * C2: how far apart two platforms may be and still form a government together. It is a DISTANCE
   * between positions — the same arithmetic the vote uses to say which platform a cell prefers —
   * and it is what makes a coalition a fact about the platforms rather than a deal nobody can see.
   */
  coalitionMaxDistance: paramId('polity.coalitionMaxDistance'),
  /** C4: how long after the election the mandate takes effect, in periods. A government is not instant. */
  mandateLag: paramId('polity.mandateLag'),
} as const;

/**
 * C1: a rule that turns a weighted vote into whole seats. It is handed the votes each platform got
 * and how many seats there are, and it answers with seats per platform — whole ones, summing to the
 * house, because a seat is a person and half a person does not sit.
 */
export interface AllotmentRule {
  readonly id: string;
  readonly what: string;
  readonly allot: (votes: ReadonlyMap<string, number>, seats: number) => Map<string, number>;
}

/**
 * C1: PROPORTIONAL, by the largest remainder. Each platform gets the whole seats its share earns,
 * and the seats left over — there are always some, because shares do not divide into a house — go
 * to the platforms with the largest fractions left. It is the oldest apportionment rule there is
 * (Hamilton's), it is stated in advance, and nothing about it is a judgement at the count.
 */
const proportional: AllotmentRule = {
  id: 'proportional',
  what: 'seats in proportion to votes, the remainders to the largest fractions',
  allot: (votes, seats) => {
    const out = new Map<string, number>();
    const total = sum([...votes.values()]).value;
    if (total <= 0 || seats <= 0) return out;
    const fractions: { id: string; left: number }[] = [];
    let given = 0;
    for (const [id, cast] of votes) {
      const exact = div(mul(cast, seats, 'the seats this share earns'), total, 'of the house');
      const whole = Math.floor(exact);
      out.set(id, whole);
      given += whole;
      fractions.push({ id, left: sub(exact, whole, 'the fraction it is owed') });
    }
    // The house is full: what the floors left over goes to the largest fractions, in order, and a
    // tie is broken by the name so the same votes always give the same parliament (determinism).
    fractions.sort((a, b) => (b.left === a.left ? a.id.localeCompare(b.id) : b.left - a.left));
    for (const f of fractions) {
      if (given >= seats) break;
      // Its whole seats were set above, for every platform in the vote, so this is a read of what
      // is there and not a default for what is missing (`no-numeric-default`).
      const whole = out.get(f.id);
      if (whole === undefined) continue;
      out.set(f.id, whole + 1);
      given += 1;
    }
    return out;
  },
};

/**
 * C1: FIRST PAST THE POST, in one national district. Whoever has the most votes takes the house.
 * It is here because C1 says the rule is a PRIMITIVE and a primitive with one possible value is not
 * one — a world can be run under either, and the difference between the two parliaments the same
 * votes produce is a thing this world can show rather than assert.
 */
const firstPastThePost: AllotmentRule = {
  id: 'firstPastThePost',
  what: 'every seat to whichever platform has the most votes',
  allot: (votes, seats) => {
    const out = new Map<string, number>();
    let best: { id: string; cast: number } | undefined;
    for (const [id, cast] of votes) {
      out.set(id, 0);
      if (best === undefined || cast > best.cast || (cast === best.cast && id < best.id)) {
        best = { id, cast };
      }
    }
    if (best !== undefined && seats > 0) out.set(best.id, seats);
    return out;
  },
};

export const ALLOTMENT_RULES: readonly AllotmentRule[] = [proportional, firstPastThePost];

/** Law 15: the rule this world runs under, by its id. A rule nobody declared is a defect. */
export function allotmentBy(id: string): AllotmentRule {
  const rule = ALLOTMENT_RULES.find((r) => r.id === id);
  if (rule === undefined) {
    throw new Error(`Polity C1: no allotment rule called ${id}`);
  }
  return rule;
}

/** A1, C1: the constitution's numbers, stated once, publicly, in advance. */
export const CONSTITUTION = {
  seats: 100,
  termMonths: 48,
  allotmentRule: proportional.id,
  coalitionMaxDistance: asRatio(0.25, 'how far apart two platforms may be and still govern'),
  mandateLag: 4,
} as const;

/** Law 8: the rule id is carried as a number in the register, so the table is indexed by position. */
export const ruleIndexOf = (id: string): number => ALLOTMENT_RULES.findIndex((r) => r.id === id);
export const ruleAt = (n: number): AllotmentRule => {
  const rule = ALLOTMENT_RULES[n];
  if (rule === undefined) throw new Error(`Polity C1: no allotment rule at ${n}`);
  return rule;
};

export type { Ratio as PolityRatio };
export const POLITY_PARAM_IDS: readonly ParamId[] = Object.values(POLITY_PARAMS);
