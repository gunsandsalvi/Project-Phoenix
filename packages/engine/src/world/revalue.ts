/**
 * Revaluation: the print becomes the mark for everyone holding the instrument, and the change is
 * real P&L to the holder and, for a liability, the reverse to the issuer (Clearing D4, Equity C4).
 *
 * @spec Clearing D4 Equity C4 Corporate Credit E4 Corporate Credit E4.a Audit B5 XI-6 Currency D3
 *
 * Per lot: qty x (mark now - carrying), where carrying is what the equity account has recognised so
 * far (prices/value.ts). Runs after every market has printed and before the audit, so nothing values
 * at a new mark against a book still carried at the old (Currency D3).
 */
import type { Cycle, Period } from '../calendar/calendar.js';
import type { PartyId } from '../core/ids.js';
import { addTo, finite, mul } from '../core/num.js';
import type { Journal } from '../journal/journal.js';
import type { Parties } from '../parties/party.js';
import { weightOf } from '../parties/party.js';
import type { Valuation } from '../prices/value.js';
import type { Instruments } from '../register/instruments.js';
import type { Register } from '../register/register.js';
import { INSTRUMENT_PROFILES } from '../registry/profiles.js';

export interface RevalueDeps {
  readonly parties: Parties;
  readonly instruments: Instruments;
  readonly register: Register;
  readonly valuation: Valuation;
  readonly journal: Journal;
}

export function revalue(period: Period, cycle: Cycle, d: RevalueDeps): void {
  const issuerMoves = new Map<PartyId, number>();
  for (const h of d.register.allHoldings()) {
    const inst = d.instruments.get(h.instrument);
    const profile = INSTRUMENT_PROFILES[inst.kind];
    if (profile.pricing !== 'cleared') continue;
    const mark = d.valuation.markPerUnit(inst.id, period);
    let delta = 0;
    for (const lot of h.lots) {
      const carrying = d.valuation.carryingPerUnit(inst.id, lot, period);
      delta = finite(delta + mul(lot.qty, mark - carrying, 'revaluation'), 'revaluation');
    }
    if (delta === 0) continue;
    d.register.moveEquity({
      party: h.holder,
      delta,
      cause: `revaluation of ${inst.id} in period ${period}`,
    });
    d.journal.record(period, cycle, 'revaluation', [h.holder, inst.id], {
      deltaPerMember: delta,
      mark,
    });
    if (profile.liabilityOfIssuer) {
      const total = mul(delta, weightOf(d.parties.get(h.holder)), 'issuer revaluation');
      addTo(issuerMoves, inst.issuer, -total);
    }
  }
  for (const [issuer, delta] of issuerMoves) {
    if (delta === 0) continue;
    d.register.moveEquity({
      party: issuer,
      delta,
      cause: `revaluation of own liabilities in period ${period}`,
    });
    d.journal.record(period, cycle, 'revaluation', [issuer], {
      deltaPerMember: delta,
      liabilities: true,
    });
  }
}
