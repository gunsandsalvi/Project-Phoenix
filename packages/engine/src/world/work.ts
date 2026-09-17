/**
 * HOW MANY QUESTIONS A PERIOD ASKED — the count `reach` and `wants` never kept.
 *
 * @spec Law 18 Clearing B2 Part XII Audit E1
 *
 * `reach` counts what a capability PRODUCED and `wants` what a party asked for and did not get.
 * Neither counts the questions themselves, and the questions are what a period costs: a session
 * asks every party of a kind whether it has an order in it, so the work is `parties × books` and
 * grows as the SQUARE of the world. `ParticipantDecl.markets` and `VenueParticipantDecl.venues`
 * are the doors that narrow it, both optional, and absent means every book of the kind — so the
 * default is the quadratic and nothing said which declarations were taking it.
 *
 * THIS IS THAT NUMBER. Asks per period, per declaration, beside what each ask produced: a row with
 * a million asks and four hundred orders is a declaration answering "no" a million times, and the
 * ratio is the size of the door that is missing. It is the honest measure of how much of this
 * engine is quadratic, exactly as `nouns`' homeless count is the measure of how much ontology is
 * missing — and, like `wants`, it is THIS PERIOD'S: the history is the journal's.
 *
 * A row is handed out ONCE PER DECLARATION PER BOOK and incremented in place, because a tally that
 * cost a map lookup per ask would be measuring itself (the loops it counts run ten million times).
 * No time is kept here: the engine has no clock (`docs/ARCHITECTURE.md` error discipline), and a
 * count is the cause where a duration is the symptom.
 */
import type { Period } from '../calendar/calendar.js';

/** One declaration's questions this period, incremented in place by the loop that asks them. */
export interface WorkRow {
  /** The declaration, as `reach` names it. */
  readonly capability: string;
  /** Where the questions were asked: a book's participant, a venue's, a phase, or the audit. */
  readonly at: 'book' | 'venue' | 'phase' | 'audit';
  /** Parties asked for a schedule. */
  asks: number;
  /** Orders that came back. */
  orders: number;
  /** Parties asked which books they could be in at all — the narrowing door's own cost. */
  narrows: number;
  /** Elementary kernel reads spent answering, from `core/ops.ts` — what an ask COSTS. */
  reads: number;
}

export class Work {
  private at: Period | undefined;
  private readonly rows = new Map<string, WorkRow>();

  /** The row to increment for this declaration. Held by the caller across a loop, never per ask. */
  row(capability: string, at: 'book' | 'venue' | 'phase' | 'audit', period: Period): WorkRow {
    this.rollTo(period);
    const key = `${at}|${capability}`;
    const held = this.rows.get(key);
    if (held !== undefined) return held;
    const made: WorkRow = { capability, at, asks: 0, orders: 0, narrows: 0, reads: 0 };
    this.rows.set(key, made);
    return made;
  }

  /** Every declaration's questions this period, most asked first. */
  all(): readonly WorkRow[] {
    return [...this.rows.values()].sort((a, b) =>
      b.reads === a.reads ? a.capability.localeCompare(b.capability) : b.reads - a.reads,
    );
  }

  /** Questions this period, over every declaration — the one number that has to fall. */
  asks(): number {
    let total = 0;
    for (const r of this.rows.values()) total += r.asks + r.narrows;
    return total;
  }

  /** The period these are about, so a stale tally cannot be read as a quiet one. */
  period(): Period | undefined {
    return this.at;
  }

  /** A tally is about ONE period: the one being run. Reaching a new one empties it. */
  private rollTo(at: Period): void {
    if (this.at === at) return;
    this.rows.clear();
    this.at = at;
  }
}
