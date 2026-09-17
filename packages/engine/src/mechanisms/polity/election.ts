/**
 * The election: every cell's own ballot, on the day the constitution says, and the seats that follow.
 *
 * @spec Polity A3 Polity A4 Polity B1 Polity B1.a Polity B2 Polity B2.a Polity B2.b Polity B3 Polity C1 XI-15 XI-17 Law 15 Money G3.a
 *
 * WHAT A CELL IS HANDED is `WhatACellKnows` and nothing else — its own outlook of its own income
 * and how many households it is. There is no view of the world in the vote, so B1.a's forbid is a
 * SHAPE rather than a rule anybody has to remember: a cell cannot vote on a published unemployment
 * rate here because the function that votes has never been given one.
 *
 * WHAT IT PRODUCES is two public events: what each cell did (privately, per cell — a ballot is that
 * cell's own) and what the house came to (publicly). The seats are the allotment rule's, applied to
 * the weighted tally, and the rule is read from the register by the position it was declared at.
 */
import { addMonths, compareCivil } from '../../calendar/civil.js';
import { period } from '../../calendar/calendar.js';
import type { ParamId } from '../../core/ids.js';
import { HOUSEHOLD } from '../../registry/profiles.js';
import { weightOf } from '../../parties/party.js';
import { platformPositions, PLATFORMS } from '../../registry/platforms.js';
import { about, type MechanismContext } from '../../world/context.js';
import { asCash } from '../../core/measure.js';
import { POLITY_PARAMS, ruleAt } from './data.js';
import { ballotOf, tally, turnoutOf, type Ballot, type WhatItTurnsOn } from './vote.js';
import { formGovernment, mandateOf } from './government.js';
import type { Family, Violation } from '../../audit/audit.js';

export const BALLOTS_CAST = 'polity.ballot';
export const SEATS_TAKEN = 'polity.seats';
export const MANDATE_GIVEN = 'polity.mandate';
export const MANDATE_TAKEN = 'polity.mandate.inForce';

/**
 * B2: the parliament-owned numbers a cell's own position turns on. They are named here, once,
 * because the vote must not go looking through the register for numbers it recognises: what a
 * position is made of is a statement this file makes and a reader can check (Law 16).
 */
const TURNS_ON: WhatItTurnsOn = {
  income: 'treasury.tax.income' as ParamId,
  consumption: 'treasury.tax.consumption' as ParamId,
  transfers: 'treasury.outlays.transfers.perMember' as ParamId,
  pension: 'pensions.contribution.employeeShare' as ParamId,
};

/** A4, Money G3.a: the period that crosses an election day, walked from the day the world opened. */
export function electsThisPeriod(ctx: MechanismContext): boolean {
  const months = ctx.params.months(POLITY_PARAMS.termMonths);
  if (months <= 0 || ctx.period === 0) return false;
  const opened = ctx.calendar.startOf(period(0));
  const today = ctx.calendar.startOf(ctx.period);
  const before = ctx.calendar.startOf(period(ctx.period - 1));
  let last = opened;
  for (let next = addMonths(opened, months); compareCivil(next, today) <= 0; ) {
    last = next;
    next = addMonths(last, months);
  }
  return compareCivil(last, opened) !== 0 && compareCivil(last, before) > 0;
}

/**
 * A3, B1, B3, XI-15: EVERY LIVING CELL'S BALLOT, AS IT WOULD BE CAST TODAY.
 *
 * It is the election's own count when an election falls today, and the straw poll's when it does
 * not (F4) — ONE function, because "how the cells would vote" is one question and a second
 * implementation of it would be a second answer to it (Law 4).
 *
 * WHAT THIS CELL KNOWS is its own outlook of its own income, in its own money, and how many
 * households it is. A cell that has never been paid has no expectation of pay and no position to
 * compare — it has nothing to vote WITH, which is a real state and not a zero, and it shows up in
 * the turnout as a household that could not choose rather than one that chose to stay home.
 */
export function ballotsToday(
  ctx: MechanismContext,
  said: ReadonlyMap<string, ReadonlyMap<ParamId, number>>,
): readonly Ballot[] {
  const ballots: Ballot[] = [];
  for (const cell of ctx.parties.ofKind(HOUSEHOLD)) {
    if (!cell.status.alive || cell.representation !== 'cell') continue;
    const view = ctx.participant(cell.id);
    const expects = view.outlook(about({ on: 'income' }));
    const ccy = ctx.registry.currencyOf(cell.region);
    ballots.push(
      ballotOf(
        {
          cell: cell.id,
          weight: weightOf(cell),
          expects: expects.some
            ? asCash(expects.value.expected, ccy, 'what it expects to be paid in a period')
            : undefined,
        },
        said,
        TURNS_ON,
      ),
    );
  }
  return ballots;
}

/** A3, B3, C1: the vote, the tally and the house. */
export function hold(ctx: MechanismContext): void {
  if (!electsThisPeriod(ctx)) return;
  const said = platformPositions(PLATFORMS, ctx.params.all());
  const ballots = ballotsToday(ctx, said);
  for (const ballot of ballots) {
    const cell = ballot.cell;
    // Observer A4: a BALLOT IS SECRET. What this cell did is recorded against this cell and is not
    // public — the house is public, and how one household voted is nobody else's read.
    ctx.record(
      BALLOTS_CAST,
      [cell],
      {
        cell,
        voted: ballot.voted ?? '',
        votes: ballot.votes,
        weight: ballot.weight,
        positions: Object.fromEntries(ballot.positions.map((x) => [x.platform, x.left.pieces])),
      },
      false,
    );
  }
  const votes = tally(ballots);
  const seats = ruleAt(ctx.params.count(POLITY_PARAMS.allotmentRule)).allot(
    votes,
    ctx.params.count(POLITY_PARAMS.seats),
  );
  const out = turnoutOf(ballots);
  /**
   * C2, C2.a: WHO GOVERNS. The largest party adds the nearest platform it may sit with until it
   * holds a majority, and a parliament where no such coalition exists is HUNG — reported, never
   * repaired, because a parliament that cannot form a government is a thing that happens.
   */
  const house = ctx.params.count(POLITY_PARAMS.seats);
  const government = formGovernment(
    seats,
    said,
    ctx.params.ratio(POLITY_PARAMS.coalitionMaxDistance),
    house,
  );
  /**
   * C3, C4 (19.6): AND WHAT THE PARLIAMENT SAYS, journaled with the period it takes effect from.
   *
   * The mandate is read here, at the count, because it is a read OF THE PARLIAMENT and the
   * parliament is what the count just produced. It is not applied here: a government is formed and
   * then it governs (C4), so the numbers move `mandateLag` periods later, which is why a change of
   * parliament shows in the deficit later rather than the same week (E3).
   *
   * A hung parliament journals no mandate. What is standing stays standing, and the seats event
   * above says why — which is the difference between a government that chose the old numbers and a
   * parliament that could not choose at all.
   */
  const mandate = mandateOf(government, seats, said, ctx.params.all());
  if (mandate !== undefined) {
    const from = period(ctx.period + ctx.params.periods(POLITY_PARAMS.mandateLag));
    ctx.record(
      MANDATE_GIVEN,
      [...government.members],
      {
        government: government.members.join(','),
        seats: government.seats,
        from,
        elected: ctx.period,
        // C3: every number parliament owns, at the value the coalition's seats came to.
        mandate: Object.fromEntries([...mandate].map(([id, v]) => [String(id), v])),
      },
      true,
    );
  }
  ctx.record(
    SEATS_TAKEN,
    [...seats.keys()],
    {
      seats: Object.fromEntries(seats),
      votes: Object.fromEntries(votes),
      cast: out.cast,
      able: out.able,
      cells: ballots.length,
      house,
      // C4: which parties, how many seats, and whether anybody could govern at all.
      government: government.members.join(','),
      governmentSeats: government.seats,
      hung: government.hung,
    },
    true,
  );
}

/**
 * C3.a, C4 (19.6): THE MANDATE TAKES EFFECT — the numbers move, once, at the lag, through the one
 * door that may move them.
 *
 * `setByMandate` refuses anything that is not a policy, anything whose owner is not parliament and
 * any module but this one (19.1), so C3.a's *no policy set directly* is the shape of the door
 * rather than a rule anybody remembers. What each number was and what it is now is published by the
 * kernel, per number, so a reader can see a government arriving in the register.
 *
 * It runs every period and does something in one of them: the period the lag lands in. A mandate
 * whose value is already standing is not written again — the register would take it, and the event
 * would say a government changed something it did not.
 */
export function takeEffect(ctx: MechanismContext): void {
  const said = ctx.journal.ofKind(MANDATE_GIVEN).filter((e) => Number(e.data['from']) === ctx.period);
  for (const e of said) {
    const values = e.data['mandate'];
    if (typeof values !== 'object' || values === null) continue;
    const moved: string[] = [];
    for (const [id, value] of Object.entries(values as Record<string, unknown>)) {
      if (typeof value !== 'number') continue;
      const held = ctx.params.decl(id as ParamId);
      if (held.value === value) continue;
      ctx.setByMandate(
        id as ParamId,
        value,
        'parliament',
        `The parliament elected in period ${String(e.data['elected'])} governs: ${String(e.data['government'])}.`,
      );
      moved.push(id);
    }
    ctx.record(
      MANDATE_TAKEN,
      [String(e.data['government'])],
      {
        government: String(e.data['government']),
        elected: e.data['elected'],
        moved: moved.length,
        numbers: moved.join(','),
      },
      true,
    );
  }
}

/**
 * C3.b (19.6): THE GUARD ON C3.a — the register says what the parliament said, or the audit does.
 *
 * C3.a is a FORBID, and a forbid breaks silently: `setByMandate` is the only door, and a number
 * that moved through some other door — a seed restated, a module writing its own declaration, a
 * second mandate nobody counted — leaves the register disagreeing with the standing mandate and
 * nothing anywhere says so. So it is MEASURED, every period, exactly: no dust, because a policy
 * primitive is not the sum of anything (Law 7) and the only value it may hold is the one that was
 * written into it.
 *
 * Before the first election there is no standing mandate and the family is not silent about that
 * either: it is BUILT and it finds nothing, which is the true answer — nobody has voted yet, so
 * there is nothing for the register to disagree with.
 */
export function mandateStands(): Family {
  return {
    name: 'names',
    contributor: 'polity',
    spec: 'Polity C3 Polity C3.a Polity C3.b',
    built: true,
    check: (view): Violation[] => {
      const given = view.journal
        .ofKind(MANDATE_GIVEN)
        .filter((e) => Number(e.data['from']) <= view.period);
      const standing = given[given.length - 1];
      if (standing === undefined) return [];
      const values = standing.data['mandate'];
      if (typeof values !== 'object' || values === null) return [];
      const held = new Map(view.params.all().map((d) => [String(d.id), d]));
      const out: Violation[] = [];
      for (const [id, value] of Object.entries(values as Record<string, unknown>)) {
        if (typeof value !== 'number') continue;
        const decl = held.get(id);
        if (decl === undefined) continue;
        if (decl.value === value) continue;
        out.push({
          family: 'names',
          spec: 'Polity C3.b',
          owner: id,
          size: decl.value - value,
          unit: decl.unit,
          period: view.period,
          message: `${id} stands at ${String(decl.value)} and the mandate of the parliament elected in period ${String(standing.data['elected'])} is ${String(value)}: a policy primitive the parliament controls moved by something that is not a mandate (C3.a)`,
        });
      }
      return out;
    },
  };
}
