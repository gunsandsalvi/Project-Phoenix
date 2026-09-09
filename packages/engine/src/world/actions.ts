/**
 * Corporate actions read from instrument terms: a coupon pays to the holders of record, read at the
 * moment it is applied (Register E1, E1.a); a maturity pays face and extinguishes the instrument
 * (Register E2, B4, Bond N10). Dates are placed on the one calendar by date (Money G3.a).
 *
 * @spec Register E1 Register E1.a Register E2 Register B4 Register E5 Bond N4 Bond N5.a Bond N6 Bond N10 Sovereign F1 Sovereign F3 Sovereign B1 Money C1.c Money G3.a Money G3.c
 */
import type { Calendar, Cycle, Period } from '../calendar/calendar.js';
import { type Civil, compareCivil } from '../calendar/civil.js';
import { yearFraction } from '../calendar/daycount.js';
import { assertNever } from '../core/assert.js';
import type { CurrencyCode, PartyId } from '../core/ids.js';
import { mul } from '../core/num.js';
import { none, some } from '../core/option.js';
import type { Journal } from '../journal/journal.js';
import type { InstructionDraft, Leg } from '../ledger/instruction.js';
import { cellSide, type Settlement, totalFor } from '../ledger/settlement.js';
import type { Parties } from '../parties/party.js';
import type { Instrument, Instruments } from '../register/instruments.js';
import type { Register } from '../register/register.js';
import type { AccountResolver } from '../clearing/market.js';

export interface ActionDeps {
  readonly calendar: Calendar;
  readonly parties: Parties;
  readonly instruments: Instruments;
  readonly register: Register;
  readonly settlement: Settlement;
  readonly journal: Journal;
  readonly accountOf: AccountResolver;
}

/** What an instrument's terms say falls due in a period. */
export type DueAction =
  | { readonly kind: 'coupon'; readonly date: Civil; readonly amountPerUnit: number }
  | { readonly kind: 'maturity'; readonly date: Civil };

/** The dated actions an instrument's terms place in `period` (Money G3.a). */
export function dueActions(i: Instrument, period: Period, cal: Calendar): DueAction[] {
  const t = i.terms;
  switch (t.kind) {
    case 'money':
      return [];
    case 'sovereign.bill': {
      return cal.place(t.maturity) === period ? [{ kind: 'maturity', date: t.maturity }] : [];
    }
    case 'sovereign.bond': {
      const out: DueAction[] = [];
      const dates = cal.schedule(t.issueDate, t.maturity, t.couponPeriodicity);
      let prev = t.issueDate;
      for (const date of dates) {
        if (cal.place(date) === period) {
          // N6: the coupon for the accrual period, by the instrument's own day count (G3.c).
          const frac = yearFraction(t.dayCount, prev, date);
          out.push({ kind: 'coupon', date, amountPerUnit: mul(t.coupon.amount, frac, 'coupon') });
        }
        prev = date;
      }
      if (cal.place(t.maturity) === period) out.push({ kind: 'maturity', date: t.maturity });
      return out.sort((a, b) => compareCivil(a.date, b.date));
    }
    default:
      return assertNever(t, 'InstrumentTerms');
  }
}

/** Run every action due this period, one instruction per holder of record (Register E1.a). */
export function runCorporateActions(period: Period, cycle: Cycle, d: ActionDeps): void {
  for (const i of d.instruments.all()) {
    if (!i.status.live) continue;
    for (const action of dueActions(i, period, d.calendar)) {
      switch (action.kind) {
        case 'coupon':
          payToHolders(i, action.amountPerUnit, 'coupon', `coupon on ${i.id}`, period, cycle, d);
          break;
        case 'maturity':
          redeem(i, period, cycle, d);
          break;
        default:
          assertNever(action, 'DueAction');
      }
    }
  }
}

function payToHolders(
  i: Instrument,
  perUnit: number,
  cause: 'coupon',
  reason: string,
  period: Period,
  cycle: Cycle,
  d: ActionDeps,
): void {
  for (const holderId of d.register.holdersOf(i.id)) {
    if (holderId === i.issuer) continue;
    const holder = d.parties.get(holderId);
    const perMemberUnits = d.register.quantity(holderId, i.id);
    if (perMemberUnits <= 0) continue;
    const perMemberCash = mul(perMemberUnits, perUnit, 'coupon cash');
    const total = totalFor(holder, perMemberCash);
    const leg: Leg = {
      kind: 'money',
      from: d.accountOf(i.issuer, i.ccy),
      to: d.accountOf(holderId, i.ccy),
      ccy: i.ccy,
      amount: total,
      fromCell: none(),
      toCell: optionalCell(
        holder.representation === 'cell' ? cellSide(holder, perMemberCash) : undefined,
      ),
    };
    const draft: InstructionDraft = { legs: [leg], cause, reason: `${reason} to ${holderId}` };
    d.settlement.settle(draft, period, cycle);
  }
}

function redeem(i: Instrument, period: Period, cycle: Cycle, d: ActionDeps): void {
  for (const holderId of [...d.register.holdersOf(i.id)]) {
    if (holderId === i.issuer) continue;
    const holder = d.parties.get(holderId);
    const perMemberUnits = d.register.quantity(holderId, i.id);
    if (perMemberUnits <= 0) continue;
    const units = totalFor(holder, perMemberUnits);
    const cashPerMember = perMemberUnits; // face: one unit of par pays one unit of currency (N10)
    const legs: Leg[] = [
      {
        kind: 'asset',
        from: holderId,
        to: i.issuer,
        instrument: i.id,
        qty: units,
        pricePerUnit: some(1),
        fromCell: optionalCell(
          holder.representation === 'cell' ? cellSide(holder, perMemberUnits) : undefined,
        ),
        toCell: none(),
      },
      {
        kind: 'money',
        from: d.accountOf(i.issuer, i.ccy),
        to: d.accountOf(holderId, i.ccy),
        ccy: i.ccy,
        amount: units,
        fromCell: none(),
        toCell: optionalCell(
          holder.representation === 'cell' ? cellSide(holder, cashPerMember) : undefined,
        ),
      },
    ];
    d.settlement.settle(
      { legs, cause: 'maturity', reason: `maturity of ${i.id} to ${holderId}` },
      period,
      cycle,
    );
  }
  const still = d.instruments.get(i.id);
  if (still.issued === 0 || Math.abs(still.issued) <= d.register.heldTotal(i.id).dust) {
    if (still.issued !== 0) d.instruments.adjustIssued(i.id, -still.issued);
    d.instruments.cease(i.id, period);
    d.journal.record(period, cycle, 'instrument.ceased', [i.id], { reason: 'maturity' });
  }
}

function optionalCell(c: ReturnType<typeof cellSide>): Leg['fromCell'] {
  return c === undefined ? none() : some(c);
}

/** Money B1: an account is (holder, issuer, currency); every party banks somewhere. */
export function accountResolver(
  parties: Parties,
): (party: PartyId, ccy: CurrencyCode) => { holder: PartyId; issuer: PartyId } {
  return (party) => {
    const p = parties.get(party);
    return { holder: party, issuer: p.bank };
  };
}
