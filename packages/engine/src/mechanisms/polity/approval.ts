/**
 * Approval: what the electorate would do today, asked today and published later, causing nothing.
 *
 * @spec Polity E4 Polity F4 Polity B1.a Polity B3 Observer A4 Observer B2.a XI-15 Law 4 Law 19
 *
 * F4 is a FORBID with a shape: approval is never an INPUT. Nothing in this world reads what is
 * published here — no party's decision takes it, no mandate moves because of it, and the government
 * that is behind in the polls governs exactly as it did before. It exists because a world with a
 * parliament and no polls is missing something everybody in it can see, and because an approval
 * that CAUSED something would be §45 B2.a's news moving a price.
 *
 * IT IS THE SAME QUESTION THE ELECTION ASKS (`ballotsToday`), which is why it is one function:
 * a second way of counting how the cells would vote would be a second answer to one question
 * (Law 4). The difference is only what happens to the count — seats, or a number in a newspaper.
 *
 * AND IT IS PUBLISHED WITH A LAG, like any statistic. The poll is TAKEN every period and is that
 * period's private fact — a survey in the field is not a published one, and a reader who could see
 * it the moment it was taken would be reading something nobody has yet been told. What is published
 * this period is the poll of `lagPeriods` ago, read back from this module's own journal (Law 19:
 * the poll is the source and republishing it is not a second count of anything).
 */
import { paramId, type ParamId } from '../../core/ids.js';
import { div } from '../../core/num.js';
import { period } from '../../calendar/calendar.js';
import { platformPositions, PLATFORMS } from '../../registry/platforms.js';
import type { MechanismContext } from '../../world/context.js';
import { ballotsToday, SEATS_TAKEN } from './election.js';
import { tally, turnoutOf } from './vote.js';

export const POLL_TAKEN = 'polity.poll';
export const APPROVAL = 'polity.approval';

/**
 * F4: how long it takes to ask and to count. It is a TECHNOLOGY of measurement and nobody's policy
 * — a parliament that could shorten the lag on the polls about itself would be setting when it is
 * judged — and it is the model's, for the same reason the reporting lag of a statistic is.
 */
export const APPROVAL_LAG = paramId('polity.approval.lagPeriods');

/** B3, XI-15: the poll taken today, weighted by cells, recorded as the private fact it is. */
export function takePoll(ctx: MechanismContext): void {
  const said: ReadonlyMap<string, ReadonlyMap<ParamId, number>> = platformPositions(
    PLATFORMS,
    ctx.params.all(),
  );
  const ballots = ballotsToday(ctx, said);
  if (ballots.length === 0) return;
  const votes = tally(ballots);
  const out = turnoutOf(ballots);
  ctx.record(
    POLL_TAKEN,
    [],
    {
      votes: Object.fromEntries(votes),
      cast: out.cast,
      able: out.able,
      cells: ballots.length,
      asked: ctx.period,
    },
    false,
  );
}

/**
 * F4, E4: and what was asked `lagPeriods` ago is published now — with the period it is ABOUT, so a
 * reader can see how old it is rather than take it for today (Observer A1.a).
 *
 * It names the standing government's share as well as each party's, because "approval" is what the
 * people who govern have: the government is read off the last seats event, which is the public
 * fact about who governs, and a world that has not voted yet has no government to approve of and
 * publishes the shares alone.
 */
export function publishApproval(ctx: MechanismContext): void {
  const lag = ctx.params.periods(APPROVAL_LAG);
  if (ctx.period < lag) return;
  const asked = period(ctx.period - lag);
  for (const poll of ctx.journal.ofKindIn(POLL_TAKEN, asked)) {
    const votes = poll.data['votes'];
    if (typeof votes !== 'object' || votes === null) continue;
    const shares = votes as Record<string, number>;
    const cast = Number(poll.data['cast']);
    const last = ctx.journal.ofKind(SEATS_TAKEN).at(-1);
    const governing =
      last === undefined ? [] : String(last.data['government']).split(',').filter((m) => m !== '');
    let forThem = 0;
    for (const m of governing) {
      // A governing party with no votes in this poll is not in the tally at all: nobody voted for
      // it, which is a real state and is what it contributes.
      const its = shares[m];
      if (its !== undefined) forThem += its;
    }
    ctx.record(
      APPROVAL,
      governing,
      {
        about: asked,
        lag,
        government: governing.join(','),
        /**
         * The share of the votes CAST, which is what a poll reports; the ones who stayed home are
         * the difference between `cast` and `able` and are not a party. A poll in which nobody
         * could choose has no share to report and says so — it is MISSING, never a zero, which
         * would read as a government nobody approves of.
         */
        approves: cast > 0 ? div(forThem, cast, 'the share of the votes cast that went to the government') : null,
        votes: shares,
        cast,
        able: Number(poll.data['able']),
      },
      true,
    );
  }
}
