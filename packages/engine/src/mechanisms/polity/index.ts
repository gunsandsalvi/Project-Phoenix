/**
 * The polity: who decides the numbers parliament owns, and how they come to decide them.
 *
 * @spec Polity A1 Polity A2 Polity A4 Polity C1 Polity C2 Polity C4 Polity D1 Polity D5 XI-17 Law 2 Law 15
 *
 * §47 is the last mechanism this world was missing on the decision side: every POLICY number in the
 * register names an owner, and for `parliament` that owner did not exist — the rates, the transfers
 * and the target stood where the seed declared them for ever, which is a standing mandate nobody
 * ever voted on. What this module builds, item by item, is the body that votes: the constitution's
 * own numbers first (here), then the platforms, the vote, the seats, the coalition and the mandate.
 *
 * WHAT IT MAY DO AND WHAT IT MAY NOT. It speaks for `parliament`'s mandate and for nothing else
 * (19.1), so it may move a tax rate and may not move the central bank's rate, a price, a quantity
 * or an outcome — the register refuses the first by owner and this world has no door at all for the
 * rest (§47 D5, F2: no policy path, and no price parliament sets).
 */
import { CONSTITUTION, POLITY_PARAMS, ALLOTMENT_RULES, ruleIndexOf } from './data.js';
import type { ParamDecl } from '../../registry/params.js';
import { platformPositions } from '../../registry/platforms.js';
import { PLATFORMS } from './platforms.js';
import type { MechanismContext, SeedContext } from '../../world/context.js';
import {
  BALLOTS_CAST,
  MANDATE_GIVEN,
  MANDATE_TAKEN,
  SEATS_TAKEN,
  hold,
  mandateStands,
  takeEffect,
} from './election.js';
import type { SystemModule } from '../../world/module.js';

export * from './data.js';
export * from './platforms.js';
export * from './vote.js';
export * from './government.js';
export * from './election.js';

function paramsOf(): ParamDecl[] {
  return [
    {
      id: POLITY_PARAMS.seats,
      value: CONSTITUTION.seats,
      unit: 'seats',
      dimension: 'count',
      kind: 'policy',
      owner: 'constitution',
      why: 'Polity A1: how many seats the parliament has. It is the CONSTITUTION’s and not parliament’s — a body does not get to rewrite the rule that elected it, and the register refuses a setter that is not the declared owner (19.1).',
    },
    {
      id: POLITY_PARAMS.termMonths,
      value: CONSTITUTION.termMonths,
      unit: 'months',
      dimension: 'months',
      kind: 'policy',
      owner: 'constitution',
      why: 'Polity A4: how long a parliament sits before the next election. In MONTHS, so the election falls on a DAY walked from the day this world opened and the period that crosses it is the one that votes (Money G3.a) — a term counted in period indices would be a second calendar.',
    },
    {
      id: POLITY_PARAMS.allotmentRule,
      value: ruleIndexOf(CONSTITUTION.allotmentRule),
      unit: `the rule at this position in the table (${ALLOTMENT_RULES.map((r) => r.id).join(', ')})`,
      dimension: 'count',
      kind: 'policy',
      owner: 'constitution',
      why: 'Polity C1: which rule turns votes into seats. It is DATA with a function beside it (Law 15) and never a branch in a mechanism: a world that wants first past the post points at the other row, and the difference between the two parliaments the same votes produce is a thing this world can show rather than assert.',
    },
    {
      id: POLITY_PARAMS.coalitionMaxDistance,
      value: CONSTITUTION.coalitionMaxDistance,
      unit: 'of the distance between the furthest-apart platforms',
      dimension: 'ratio',
      kind: 'policy',
      owner: 'constitution',
      why: 'Polity C2: how far apart two platforms may be and still form a government together. It is a DISTANCE between stated positions — the same arithmetic a cell uses to say which platform it prefers — so a coalition is a fact about the platforms and not a deal nobody can see.',
    },
    {
      id: POLITY_PARAMS.mandateLag,
      value: CONSTITUTION.mandateLag,
      unit: 'periods',
      dimension: 'periods',
      kind: 'policy',
      owner: 'constitution',
      why: 'Polity C4: how long after the election the mandate takes effect. A government is not instant — it is formed, and then it governs — and the lag is why a change of parliament shows in the deficit later rather than the same week (E3).',
    },
  ];
}

export const polity: SystemModule = {
  id: 'polity',
  spec: 'Polity',
  // It reads what households are and what the treasury does; it writes neither until the vote is
  // built (19.4), and what it will write then is parliament's own numbers and nothing else.
  requires: ['households', 'treasury'],
  instrumentKinds: [],
  partyKinds: [],
  curveFamilies: [],
  units: [],
  params: paramsOf(),
  // 19.1: it speaks for PARLIAMENT and for no other mandate. The central bank's rate is the money
  // market's, and the constitution's own numbers are nobody's to move at all — which is what makes
  // them a constitution (Polity D5).
  mandates: ['parliament'],
  phases: [
    {
      /**
       * C3.a, C4 (19.6): THE MANDATE GOVERNS. Every period, before the world runs: the period the
       * lag lands in is the one where the numbers move, and every other period this does nothing.
       *
       * It runs BEFORE the election phase below because that is the order the two things happen in
       * — a parliament elected four periods ago governs this period, and the parliament elected
       * this period governs four periods from now. A mandate applied after the count would be a
       * government legislating before it was formed.
       */
      name: 'polity.mandate',
      spec: 'Polity C3 Polity C3.a Polity C4',
      anchor: { after: 'corporateActions' },
      // A read of HISTORY: what an earlier parliament said, which is why it orders nothing.
      reads: [{ kind: 'event', name: MANDATE_GIVEN, of: 'anyPeriod' }],
      writes: [{ kind: 'event', name: MANDATE_TAKEN }],
      run: (ctx: MechanismContext): void => {
        takeEffect(ctx);
      },
    },
    {
      /**
       * A4, B1, B3, C4 (19.4): THE ELECTION. It falls on a DAY — every `termMonths` from the day
       * this world opened — and the period that crosses that day is the one that votes, which is
       * the same rule the credit series rolls on and the central bank meets on (Money G3.a).
       *
       * At the top of the period, before anything else this world does, because what a parliament
       * decides is the frame the rest of the period runs in — and because a cell voting after its
       * own wage had been paid would be voting on a state the election had already changed.
       */
      name: 'polity.election',
      spec: 'Polity A3 Polity A4 Polity B1 Polity B2 Polity B3 XI-15 XI-17',
      anchor: { after: 'corporateActions' },
      reads: [],
      writes: [
        { kind: 'event', name: BALLOTS_CAST },
        { kind: 'event', name: SEATS_TAKEN },
        { kind: 'event', name: MANDATE_GIVEN },
      ],
      run: (ctx: MechanismContext): void => {
        hold(ctx);
      },
    },
  ],
  participants: [],
  /** C3.b: the register holds what the parliament said, every period, exactly — or the audit says so. */
  families: [mandateStands()],
  /**
   * A2, D5 (19.3): THE PLATFORMS ARE CHECKED AGAINST THE REGISTER, at assembly, once.
   *
   * What parliament owns is a read of the world's own declarations, so the seed is the first moment
   * the question can be asked at all: by now every module has declared its numbers. A platform that
   * says nothing about one of them, or says something about a number parliament does not own, or
   * says two things about one — the world does not open. That is a declaration being wrong, not a
   * finding about a world that ran.
   */
  seed(ctx: SeedContext): void {
    platformPositions(PLATFORMS, ctx.params.all());
  },
};
