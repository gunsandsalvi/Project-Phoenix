/**
 * What one module said about a firm being born, read by another (Firm Birth A1, A6.c, 12.4a.2).
 *
 * @spec Firm Birth A1 Small-Business Pools A6.c Law 15
 *
 * The small-business module says which of its cells have outgrown the tier; the firms module bears
 * a named firm for each member and says it did; the equity module gives the born firm its share
 * line. Each reads the other's fact HERE, by the registry's name for it, never by the event's
 * (0e′.3): a question or a registry read, never a name.
 */
import type { Period } from '../calendar/calendar.js';
import type { Event } from '../journal/journal.js';

const PROMOTION_WANTED = 'smallBusiness.promotion';
const FIRM_BORN = 'firm.born';

export interface BirthReads {
  ofKindIn(kind: string, period: Period): readonly Event[];
}

/** A6.c: a cell whose members have outgrown the tier, and who owns them. */
export interface PromotionWanted {
  readonly cell: string;
  readonly members: number;
  readonly line: string;
  readonly bank: string;
  readonly region: string;
  readonly owner: string;
}

export function promotionsWantedIn(reads: BirthReads, period: Period): readonly PromotionWanted[] {
  const out: PromotionWanted[] = [];
  for (const e of reads.ofKindIn(PROMOTION_WANTED, period)) {
    const d = e.data;
    if (
      typeof d['cell'] !== 'string' || typeof d['members'] !== 'number' || typeof d['line'] !== 'string' ||
      typeof d['bank'] !== 'string' || typeof d['region'] !== 'string' || typeof d['owner'] !== 'string'
    ) continue;
    out.push({ cell: d['cell'], members: d['members'], line: d['line'], bank: d['bank'], region: d['region'], owner: d['owner'] });
  }
  return out;
}

/** A1: a named firm that entered this period, and who it belongs to. */
export interface FirmBorn {
  readonly firm: string;
  readonly owner: string;
  readonly line: string;
}

export function firmsBornIn(reads: BirthReads, period: Period): readonly FirmBorn[] {
  const out: FirmBorn[] = [];
  for (const e of reads.ofKindIn(FIRM_BORN, period)) {
    const d = e.data;
    if (typeof d['firm'] !== 'string' || typeof d['owner'] !== 'string' || typeof d['line'] !== 'string') continue;
    out.push({ firm: d['firm'], owner: d['owner'], line: d['line'] });
  }
  return out;
}
