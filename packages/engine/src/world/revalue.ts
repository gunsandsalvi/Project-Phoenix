/**
 * Revaluation: the print becomes the mark for everyone holding the instrument, and the change is
 * real P&L to the holder and, for a liability, the reverse to the issuer (Clearing D4, Equity C4).
 *
 * @spec Goods E2 Goods E2.a Goods E2.c Capital Programme A3 Capital Programme A6 Clearing D4 Equity C4 Corporate Credit E4 Corporate Credit E4.a Audit B5 XI-6 Currency D3
 *
 * Per lot: qty x (mark now - carrying), where carrying is what the equity account has recognised so
 * far (prices/value.ts). Runs after every market has printed and before the audit, so nothing values
 * at a new mark against a book still carried at the old (Currency D3).
 *
 * A kind carried at cost is asked LOT BY LOT what that lot is carried at now, and the difference
 * lands on the lot and on the equity account in the same step (Capital Programme A3: one schedule,
 * charged in both places). It is per lot because the answer is: a good is written down to what its
 * market says (Goods E2), and a vintage of plant wears out on a schedule of its own, from what its
 * own holder paid for it (A6).
 */
import { issuerOf } from '../register/instruments.js';
import type { Calendar, Cycle, Period } from '../calendar/calendar.js';
import type { InstrumentId, PartyId } from '../core/ids.js';
import { impossible } from '../core/assert.js';
import { none, type Option } from '../core/option.js';
import { addTo, div, finite, mul, sub, zeroIfNone } from '../core/num.js';
import type { Journal } from '../journal/journal.js';
import type { Parties } from '../parties/party.js';
import { weightOf } from '../parties/party.js';
import type { Valuation } from '../prices/value.js';
import type { Instrument, Instruments } from '../register/instruments.js';
import type { Holding, Register } from '../register/register.js';
import type { Registry } from '../registry/registry.js';

export interface RevalueDeps {
  /** What a market last said a unit is worth, for a kind whose lots are carried at cost. */
  marked(instrument: InstrumentId, period: Period): Option<number>;
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly parties: Parties;
  readonly instruments: Instruments;
  readonly register: Register;
  readonly valuation: Valuation;
  readonly journal: Journal;
}

export function revalue(period: Period, cycle: Cycle, d: RevalueDeps): void {
  const issuerMoves = new Map<PartyId, number>();
  // Law 7: what the re-marking passed THROUGH, which is the position's whole value and not the
  // change in it. A book re-marked from 6000 to 6000.01 moved by a penny and rounded a 6000, and
  // an account that is zero by construction (a fund's, Fund Shares A3) is nothing but that residue.
  const issuerThrough = new Map<PartyId, number>();
  for (const h of d.register.allHoldings()) {
    const inst = d.instruments.get(h.instrument);
    const profile = d.registry.instrumentKind(inst.kind);
    const written = profile.carriedAt;
    let moved: Moved;
    if (profile.pricing === 'cleared' && profile.carry === 'mark') {
      moved = toTheMark(inst, h, period, d);
    } else if (written !== undefined) {
      moved = toWhatTheKindSays(inst, h, period, d);
    } else continue;
    const { delta, through, carried } = moved;
    if (delta === 0) continue;
    d.register.moveEquity({
      party: h.holder,
      delta,
      cause: `revaluation of ${inst.id} in period ${period}`,
      through,
    });
    d.journal.record(
      period,
      cycle,
      'revaluation',
      [h.holder, inst.id],
      {
        deltaPerMember: delta,
        mark: carried,
      },
      false,
    );
    if (profile.liabilityOfIssuer) {
      const weight = weightOf(d.parties.get(h.holder));
      addTo(issuerMoves, issuerOf(inst), -mul(delta, weight, 'issuer revaluation'));
      addTo(issuerThrough, issuerOf(inst), mul(through, weight, 'what its liability passed through'));
    }
  }
  for (const [issuer, delta] of issuerMoves) {
    if (delta === 0) continue;
    d.register.moveEquity({
      party: issuer,
      delta,
      cause: `revaluation of own liabilities in period ${period}`,
      through: zeroIfNone(issuerThrough.get(issuer)),
    });
    d.journal.record(
      period,
      cycle,
      'revaluation',
      [issuer],
      {
        deltaPerMember: delta,
        liabilities: true,
      },
      false,
    );
  }
}

/**
 * What one holding's re-marking came to: the move, the magnitude it passed through (Law 7), and
 * what a unit of it is carried at when it is done.
 */
interface Moved {
  readonly delta: number;
  readonly through: number;
  readonly carried: number;
}

/** Clearing D4: a position carried at the mark moves to this period's print, both ways. */
function toTheMark(inst: Instrument, h: Holding, period: Period, d: RevalueDeps): Moved {
  const mark = d.valuation.markPerUnit(inst.id, period);
  let delta = 0;
  for (const lot of h.lots) {
    const carrying = d.valuation.carryingPerUnit(inst.id, lot, period);
    delta = finite(delta + mul(lot.qty, mark - carrying, 'revaluation'), 'revaluation');
  }
  return {
    delta,
    through: mul(
      h.lots.reduce((t, l) => t + Math.abs(l.qty), 0),
      Math.abs(mark),
      'what the re-marking passed through',
    ),
    carried: mark,
  };
}

/**
 * Goods E2, Capital Programme A3: a kind that says what its lots are carried at is asked lot by
 * lot, and the difference lands on the lot and on the equity account together.
 */
function toWhatTheKindSays(
  inst: Instrument,
  h: Holding,
  period: Period,
  d: RevalueDeps,
): Moved {
  const profile = d.registry.instrumentKind(inst.kind);
  const written = profile.carriedAt;
  const priced = d.marked(inst.id, period);
  let delta = 0;
  let through = 0;
  let units = 0;
  let value = 0;
  for (const lot of h.lots) {
    units = finite(units + Math.abs(lot.qty), 'units re-marked');
    const now = written === undefined ? none<number>() : written(inst, lot, priced, period, d.calendar);
    const per = now.some ? now.value : lot.basisPerUnit;
    value = finite(value + mul(lot.qty, per, 'what the position is carried at'), 'carrying');
    if (!now.some) continue;
    const one = mul(lot.qty, sub(now.value, lot.basisPerUnit, 'what the carrying moved by'), 'write-down');
    // Goods E2.c: which way a lot may move is the KIND's rule, and this is the one place that
    // holds it. Inventory is written down and never up; a claim moves both ways, because a
    // provision unwinds when its holder stops expecting the loss (Banks Lending D2.a).
    impossible(
      one <= 0 || profile.fairValueThroughIncome === true,
      'Goods E2.c',
      `${inst.id} would be carried above cost by ${one}: only a dealer's book marks up`,
      { instrument: inst.id, holder: h.holder, delta: one },
    );
    if (one === 0) continue;
    // The lot itself is what the book carries it at, so the re-measurement lands there and in
    // the equity account together: one fact, one writer, two reads that agree (Law 4).
    d.register.remark(h.holder, inst.id, lot.id, now.value);
    delta = finite(delta + one, 'write-down');
    through = finite(
      through + Math.abs(mul(lot.qty, now.value, 'what the re-marking passed through')),
      'what the re-marking passed through',
    );
  }
  return { delta, through, carried: units === 0 ? 0 : div(value, units, 'carried per unit') };
}
