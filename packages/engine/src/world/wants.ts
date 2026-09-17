/**
 * What a party asked this world for and did not get — the other half of `reach.ts`.
 *
 * @spec Audit E1 Audit E2 Audit E3 Part XII Observer A4 Law 19
 *
 * `reach` says what a capability has PRODUCED; nothing said why one produced nothing. A participant
 * that declines returns `[]`, and a module that fails closed on a missing read returns nothing at
 * all — so the record of every refusal in this world was the absence of a record, and every
 * diagnosis in `docs/RECORD.md` was a hand-written probe that went looking for one.
 *
 * THE READ THAT WAS MISSING IS THE ANSWER, and the kernel already produces it: a view's doors hand
 * back `Missing` where there is nothing to hand back, and that is the moment the fact exists. So it
 * is tallied there, at the one place it is known, rather than at the fifteen call sites that would
 * each have to remember to say so (Law 4, Law 12 — the alternative was a comment in fifteen files).
 *
 * IT IS THIS PERIOD'S. A party's refusals are a diagnostic of the period being run, and keeping
 * every party's every missing read for every period would be a second history beside the journal's.
 * Earlier periods are answered from the journal and the ledger, which ARE that history (Law 19).
 */
import type { Period } from '../calendar/calendar.js';
import type { PartyId } from '../core/ids.js';
import type { Option } from '../core/option.js';

/** One read a party wanted and did not get, and how many times it wanted it. */
export interface Want {
  readonly read: string;
  readonly times: number;
}

export class Wants {
  private at: Period | undefined;
  private readonly byParty = new Map<PartyId, Map<string, number>>();
  /**
   * The books and venues a party was ASKED about and posted nothing in. It is kept apart from the
   * missing reads because it is a different fact and swamps them: every party is asked about every
   * venue of its kind, and a mill that posted in one labour venue posted in none of the other
   * forty. The two together are the diagnosis — what it did not bid in, and what it was short of.
   */
  private readonly silentIn = new Map<PartyId, Set<string>>();

  /** A door answered `Missing` for this party. The period is the world's, never the caller's. */
  missed(party: PartyId, read: string, at: Period): void {
    this.rollTo(at);
    const mine = this.byParty.get(party);
    if (mine === undefined) {
      this.byParty.set(party, new Map([[read, 1]]));
      return;
    }
    // Missing is missing: a read nobody has wanted yet has no count, and the first want is one.
    const had = mine.get(read);
    mine.set(read, had === undefined ? 1 : had + 1);
  }

  /** It was asked about this book or venue and posted nothing into it. */
  postedNothing(party: PartyId, book: string, at: Period): void {
    this.rollTo(at);
    const mine = this.silentIn.get(party);
    if (mine === undefined) this.silentIn.set(party, new Set([book]));
    else mine.add(book);
  }

  /** The books and venues this party was asked about and posted nothing in, in their own order. */
  quietIn(party: PartyId, at: Period): readonly string[] {
    if (this.at !== at) return [];
    const mine = this.silentIn.get(party);
    return mine === undefined ? [] : [...mine].sort();
  }

  /** A tally is about ONE period: the one being run. Reaching a new one empties it. */
  private rollTo(at: Period): void {
    if (this.at === at) return;
    this.byParty.clear();
    this.silentIn.clear();
    this.at = at;
  }

  /** What this party wanted and did not get, most wanted first. Nothing, for another period. */
  of(party: PartyId, at: Period): readonly Want[] {
    if (this.at !== at) return [];
    const mine = this.byParty.get(party);
    if (mine === undefined) return [];
    return [...mine]
      .map(([read, times]) => ({ read, times }))
      .sort((a, b) => (b.times === a.times ? a.read.localeCompare(b.read) : b.times - a.times));
  }

  /** The period these are about, so a reader cannot mistake a stale tally for an empty one. */
  period(): Period | undefined {
    return this.at;
  }
}

/** 0h.4: one instruction this party was a side of, and what became of it. */
export interface WhyInstruction {
  readonly instruction: string;
  readonly cause: string;
  readonly outcome: 'settled' | 'failed';
  /** What settlement said stopped it, in its own words — `undefined` where nothing did. */
  readonly failed?: string;
  /** Whose want stopped it: a failure that was not this party's says nothing about this party. */
  readonly against?: string;
}

/**
 * 0h.4: WHY A PARTY DID WHAT IT DID IN A PERIOD — a read assembled from the ledger, the journal and
 * this period's wants. Nothing here is stored: every field is a read of a record that already
 * exists, which is what makes it a query and not a fourth history (Law 19).
 */
export interface Why {
  readonly party: PartyId;
  readonly period: Period;
  readonly kind: string;
  readonly alive: boolean;
  readonly bornAt: Option<Period>;
  readonly settled: readonly WhyInstruction[];
  readonly events: readonly { readonly kind: string; readonly times: number }[];
  /** What it asked for and did not get — EMPTY for any period but the one the world is running. */
  readonly wanted: readonly Want[];
  /** The books and venues it was asked about and posted nothing in. This period's, like `wanted`. */
  readonly quietIn: readonly string[];
  /** Which period `wanted` is about, so an empty list cannot be mistaken for a quiet party. */
  readonly wantedIsAbout: Period | undefined;
}
