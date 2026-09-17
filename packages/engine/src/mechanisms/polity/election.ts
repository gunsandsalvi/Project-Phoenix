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

export const BALLOTS_CAST = 'polity.ballot';
export const SEATS_TAKEN = 'polity.seats';

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

/** A3, B3, C1: the vote, the tally and the house. */
export function hold(ctx: MechanismContext): void {
  if (!electsThisPeriod(ctx)) return;
  const said = platformPositions(PLATFORMS, ctx.params.all());
  const ballots: Ballot[] = [];
  for (const cell of ctx.parties.ofKind(HOUSEHOLD)) {
    if (!cell.status.alive || cell.representation !== 'cell') continue;
    const view = ctx.participant(cell.id);
    const expects = view.outlook(about({ on: 'income' }));
    /**
     * B1, B1.a, XI-15: WHAT THIS CELL KNOWS. Its own outlook of its own income, in its own money,
     * and how many households it is. A cell that has never been paid has no expectation of pay and
     * no position to compare — it has nothing to vote WITH, which is a real state and not a zero,
     * and it shows up in the turnout as a household that could not choose rather than one that
     * chose to stay home.
     */
    const ccy = ctx.registry.currencyOf(cell.region);
    const ballot = ballotOf(
      {
        cell: cell.id,
        weight: weightOf(cell),
        expects: expects.some
          ? asCash(expects.value.expected, ccy, 'what it expects to be paid in a period')
          : undefined,
      },
      said,
      TURNS_ON,
    );
    ballots.push(ballot);
    // Observer A4: a BALLOT IS SECRET. What this cell did is recorded against this cell and is not
    // public — the house is public, and how one household voted is nobody else's read.
    ctx.record(
      BALLOTS_CAST,
      [cell.id],
      {
        cell: cell.id,
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
  ctx.record(
    SEATS_TAKEN,
    [...seats.keys()],
    {
      seats: Object.fromEntries(seats),
      votes: Object.fromEntries(votes),
      cast: out.cast,
      able: out.able,
      cells: ballots.length,
    },
    true,
  );
}
