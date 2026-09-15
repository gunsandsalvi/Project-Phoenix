/**
 * What happens to what a party OWED, and was owed, at the instant it ceases.
 *
 * @spec Register F2 Money E4 XI-8 XI-3 Law 4 Law 5 Law 15
 *
 * Register F2 says every reference resolves to the estate or the successor, and the register kept
 * that promise for what a party HELD: holdings move, instruments are reassigned, a contract is
 * novated by the layer that owns it. The agreement book kept nothing. A backstop a resolved bank
 * had committed, a mandate its manager ran, a wage a dead cell was owed: each still named the party
 * that was gone, and the first module to charge a fee on one of them addressed an instruction to
 * somebody who is not there. Settlement refused it (Money E4) — correctly, and a cycle too late to
 * say whose commitment it had been.
 *
 * SO THE ROWS MOVE WHERE THE PARTY CEASES, once, in the kernel. It is the same act as the holdings
 * moving and it belongs beside it: a module asking "is my counterparty still alive?" on every row
 * of its own book would be answering a kernel question in fourteen places, and the fourteenth would
 * answer it differently (Law 4).
 *
 * WHAT MOVES AND WHAT ENDS IS THE KIND'S OWN ANSWER (`AgreementKindDecl.binds`, Law 15), and the
 * kernel branches on nothing: a debt binds whoever succeeds, a relationship binds a going concern.
 * An estate is the terminal kind — it realises what is there rather than running it — so a mandate
 * or an employment that reaches one ends there, while the wage it still owed passes to it, which is
 * what an estate is for (XI-8).
 *
 * A ROW WHOSE OTHER SIDE IS THE SUCCESSOR ENDS TOO, whatever its kind: a bank resolved into the one
 * it had committed a line to, a cell merged into the one it owed a wage. Moving it would put a
 * party on both sides of its own obligation, which is not a commitment but a statement about itself
 * (Law 5) — the same tear-up the derivative layer does for a contract whose successor is its
 * counterparty (Derivative D1.a).
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import type { PartyId } from '../core/ids.js';
import type { Journal } from '../journal/journal.js';
import type { Parties } from '../parties/party.js';
import type { Registry } from '../registry/registry.js';
import type { Agreements } from '../register/agreements.js';

export interface SuccessionDeps {
  readonly parties: Parties;
  readonly registry: Registry;
  readonly agreements: Agreements;
  readonly journal: Journal;
}

/**
 * Move every live row naming `dead` onto `successor`, or end it. Closed rows are history and keep
 * the names they closed under; only a performing or breached row is a reference anything addresses.
 */
export function succeedAgreements(
  dead: PartyId,
  successor: PartyId,
  period: Period,
  cycle: Cycle,
  d: SuccessionDeps,
): void {
  const goingConcern =
    d.registry.partyKind(d.parties.get(successor).kind).terminal !== true;
  const live = [...d.agreements.owedBy(dead), ...d.agreements.owedTo(dead)].filter(
    (a) => a.state === 'performing' || a.state === 'breached',
  );
  for (const row of live) {
    const onDebtor = row.debtor === dead;
    const other = onDebtor ? row.creditor : row.debtor;
    const binds = d.agreements.kind(row.terms.kind).binds;
    const why =
      other === successor
        ? `${successor} was the other side of it`
        : binds === 'aGoingConcern' && !goingConcern
          ? `${successor} is winding ${dead} up and does not run what it ran`
          : null;
    if (why !== null) {
      d.agreements.terminate(row.id);
      d.journal.record(
        period,
        cycle,
        'agreement.ended',
        [String(row.id), String(dead), String(successor)],
        {
          agreement: row.id,
          was: dead,
          successor,
          side: onDebtor ? 'debtor' : 'creditor',
          owed: row.owed,
          what: row.terms.kind,
          why,
        },
        true,
      );
      continue;
    }
    d.agreements.succeed(row.id, dead, successor);
    d.journal.record(
      period,
      cycle,
      'agreement.succeeded',
      [String(row.id), String(dead), String(successor)],
      {
        agreement: row.id,
        was: dead,
        now: successor,
        side: onDebtor ? 'debtor' : 'creditor',
        owed: row.owed,
        what: row.terms.kind,
      },
      true,
    );
  }
}
