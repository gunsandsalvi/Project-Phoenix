/**
 * The ledger: the numbered, append-only record of every instruction, settled or failed (Money D1.a).
 * Positions are replayable from it, and the Flows audit family replays them (Money D3).
 *
 * @spec Money D1 Money D1.a Money D1.b Money E1 Money E2 Money E2.a Register C3.b
 */
import type { Period } from '../calendar/calendar.js';
import type { InstructionId } from '../core/ids.js';
import { subjectsOf, type Failed, type SettlementRecord } from './instruction.js';

export class Ledger {
  private readonly records: SettlementRecord[] = [];
  /**
   * Law 18: the same records under a second arrangement, written where they are appended so there
   * is one writer and nothing to go stale (Law 4). `inPeriod` is asked once per bank per period —
   * every bank reads what its lines earned off the wire (Law 19) — and filtering the whole ledger
   * for it made the cost of a period the cost of every period before it.
   */
  private readonly byPeriod = new Map<Period, SettlementRecord[]>();
  /**
   * Law 18, Money E1.b: the FAILED records under the parties they name. A party asks what it failed
   * to pay — and an assessor asks it of every issuer it rates, every period (Ratings A2) — and
   * filtering the whole ledger for it made the cost of a period the cost of every period before it.
   * Only the failures are indexed: they are the ones a party reads back, and they are rare.
   */
  private readonly failedBy = new Map<string, Failed[]>();
  private next = 1;

  /** The next instruction number; settlement stamps it on the instruction it is about to apply. */
  allocate(): InstructionId {
    const id = this.next as InstructionId;
    this.next += 1;
    return id;
  }

  /** Append is the only write. There is no reversal (E2); a correction is a new instruction (E2.a). */
  append(r: SettlementRecord): void {
    this.records.push(r);
    const at = r.instruction.period;
    const list = this.byPeriod.get(at);
    if (list === undefined) this.byPeriod.set(at, [r]);
    else list.push(r);
    if (r.outcome !== 'failed') return;
    for (const who of subjectsOf(r.instruction)) {
      const mine = this.failedBy.get(who);
      if (mine === undefined) this.failedBy.set(who, [r]);
      else mine.push(r);
    }
  }

  /**
   * Money E1.b: what this party's own payments did SINCE a period, most recent last.
   *
   * It used to take a COUNT — the last n failures, whenever they happened — and the one caller that
   * wanted a window got the other thing: an assessor asking for "the failures in its own memory"
   * received every failure that party had ever had, so an issuer that missed one payment in its
   * first week was graded the worst there is for the rest of the run (item 12a). A count of events
   * is not a horizon, and a rating is a judgement about a party's state NOW (§44 A2).
   */
  failedFor(party: string, since: Period): readonly Failed[] {
    const mine = this.failedBy.get(party);
    if (mine === undefined) return EMPTY_FAILED;
    return mine.filter((r) => r.instruction.period >= since);
  }

  all(): readonly SettlementRecord[] {
    return this.records;
  }

  inPeriod(period: Period): readonly SettlementRecord[] {
    return this.byPeriod.get(period) ?? EMPTY;
  }

  get length(): number {
    return this.records.length;
  }
}

const EMPTY: readonly SettlementRecord[] = Object.freeze([]);
const EMPTY_FAILED: readonly Failed[] = Object.freeze([]);
