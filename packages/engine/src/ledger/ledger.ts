/**
 * The ledger: the numbered, append-only record of every instruction, settled or failed (Money D1.a).
 * Positions are replayable from it, and the Flows audit family replays them (Money D3).
 *
 * @spec Money D1 Money D1.a Money D1.b Money E1 Money E2 Money E2.a Register C3.b
 */
import type { Period } from '../calendar/calendar.js';
import type { InstructionId } from '../core/ids.js';
import type { SettlementRecord } from './instruction.js';

export class Ledger {
  private readonly records: SettlementRecord[] = [];
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
  }

  all(): readonly SettlementRecord[] {
    return this.records;
  }

  inPeriod(period: Period): readonly SettlementRecord[] {
    return this.records.filter((r) => r.instruction.period === period);
  }

  get length(): number {
    return this.records.length;
  }
}
