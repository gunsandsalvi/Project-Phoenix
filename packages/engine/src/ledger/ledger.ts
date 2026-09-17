/**
 * The ledger: the numbered, append-only record of every instruction, settled or failed (Money D1.a).
 * Positions are replayable from it, and the Flows audit family replays them (Money D3).
 *
 * @spec Money D1 Money D1.a Money D1.b Money E1 Money E2 Money E2.a Register C3.b
 */
import type { Period } from '../calendar/calendar.js';
import type { InstructionId, InstrumentId, PartyId } from '../core/ids.js';
import type { Qty } from '../core/tick.js';
import { asQty, negQty } from '../core/tick.js';
import { subjectsOf, type Failed, type SettlementRecord } from './instruction.js';
import { none, type Option, some } from '../core/option.js';

/**
 * 0g.2, Law 18: WHAT A PERIOD'S SETTLED RECORDS DID, indexed as they are appended. The audit
 * families used to rebuild these three maps from the period's records every period, each family
 * its own walk (`flows`, `money`, `units`); they are the same deltas under a second arrangement,
 * written by the one writer that appends the record, so there is nothing to go stale (Law 4).
 */
export interface PeriodDeltas {
  /** Holding deltas by `holder|instrument`, in the order they settled. */
  readonly holding: ReadonlyMap<string, readonly Qty[]>;
  /** Issued deltas by instrument, in the order they settled. */
  readonly issued: ReadonlyMap<InstrumentId, readonly Qty[]>;
  /** Units created (+) and destroyed (−) by instrument, off the `create`/`destroy` legs. */
  readonly made: ReadonlyMap<InstrumentId, readonly Qty[]>;
}

interface MutableDeltas {
  readonly holding: Map<string, Qty[]>;
  readonly issued: Map<InstrumentId, Qty[]>;
  readonly made: Map<InstrumentId, Qty[]>;
}

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
  /**
   * 0h.3: WHEN EACH PARTY WAS LAST A SIDE OF A SETTLED INSTRUCTION — the twin of `failedBy`, written
   * where every settlement is appended and read by nothing else. It is not a copy of a derived fact:
   * the alternative is a walk of every record ever appended, every period, for every party.
   */
  private readonly settledBy = new Map<string, Period>();
  /** 0g.2: the period's deltas, written at `append`. */
  private readonly deltasBy = new Map<Period, MutableDeltas>();
  /** 12c.1, Law 18: what each party has ever created of each thing — an index over the create legs, kept as they settle. */
  private readonly madeEver = new Map<string, number>();
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
    if (r.outcome === 'settled') {
      for (const who of subjectsOf(r.instruction)) this.settledBy.set(who, at);
      let d = this.deltasBy.get(at);
      if (d === undefined) {
        d = { holding: new Map(), issued: new Map(), made: new Map() };
        this.deltasBy.set(at, d);
      }
      for (const delta of r.deltas) {
        if (delta.target === 'holding') {
          const key = `${delta.party}|${delta.instrument}`;
          const list = d.holding.get(key);
          if (list === undefined) d.holding.set(key, [delta.qty]);
          else list.push(delta.qty);
        } else {
          const list = d.issued.get(delta.instrument);
          if (list === undefined) d.issued.set(delta.instrument, [delta.qty]);
          else list.push(delta.qty);
        }
      }
      for (const leg of r.instruction.legs) {
        if (leg.kind !== 'create' && leg.kind !== 'destroy') continue;
        if (leg.kind === 'create') {
          const k = `${String(leg.party)}|${String(leg.instrument)}`;
          const had = this.madeEver.get(k);
          this.madeEver.set(k, had === undefined ? leg.qty : had + leg.qty);
        }
        const signed = leg.kind === 'create' ? leg.qty : negQty(leg.qty, 'what left the world');
        const list = d.made.get(leg.instrument);
        if (list === undefined) d.made.set(leg.instrument, [signed]);
        else list.push(signed);
      }
      return;
    }
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

  /**
   * 0h.3, Part XII: the last period this party was a side of an instruction that SETTLED, or
   * nothing at all for one that never has. A failure is not this — the ledger already answers that
   * separately (`failedFor`) — and the difference between the two is what a liveness check reads.
   */
  lastSettledFor(party: string): Option<Period> {
    const at = this.settledBy.get(party);
    return at === undefined ? none<Period>() : some(at);
  }

  all(): readonly SettlementRecord[] {
    return this.records;
  }

  /**
   * Firm A3, Goods A2.c (12c.1): WHAT THIS PARTY HAS EVER MADE OF THIS — every unit its create
   * legs brought into the world, read off the ledger's own index (Law 19). It is what a line's
   * hours per unit is a read against, and it is not a level anybody stores: the legs are the record.
   */
  madeBy(party: PartyId, instrument: InstrumentId): Qty {
    const had = this.madeEver.get(`${String(party)}|${String(instrument)}`);
    // A party with no create leg on this thing has made none of it: that is a count, not a default.
    if (had === undefined) return asQty(0, 'it has made none of it');
    return asQty(had, 'the pieces it has made');
  }

  inPeriod(period: Period): readonly SettlementRecord[] {
    return this.byPeriod.get(period) ?? EMPTY;
  }

  /** 0g.2: what the period's settled records did, by holding, by issued line and by unit made. */
  deltasIn(period: Period): PeriodDeltas {
    return this.deltasBy.get(period) ?? NO_DELTAS;
  }

  get length(): number {
    return this.records.length;
  }
}

const EMPTY: readonly SettlementRecord[] = Object.freeze([]);
const NO_DELTAS: PeriodDeltas = Object.freeze({ holding: new Map(), issued: new Map(), made: new Map() });
const EMPTY_FAILED: readonly Failed[] = Object.freeze([]);
