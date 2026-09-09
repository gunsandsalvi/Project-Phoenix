/**
 * Corporate actions: what an instrument's terms say falls due, applied by the kernel. A coupon pays
 * to the holders of record, read at the moment it is applied (Register E1, E1.a); a maturity pays
 * face and extinguishes the instrument (Register E2, B4, Bond N10). Which actions fall due is the
 * instrument kind's profile to say (Law 15); the kernel only knows the vocabulary.
 *
 * @spec Register E1 Register E1.a Register E2 Register B4 Register E5 Bond N10 Money C1.c Money G3.a Law 15
 */
import type { Calendar, Cycle, Period } from '../calendar/calendar.js';
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
import type { Registry } from '../registry/registry.js';
import type { AccountResolver } from '../clearing/market.js';

export interface ActionDeps {
  readonly calendar: Calendar;
  readonly registry: Registry;
  readonly parties: Parties;
  readonly instruments: Instruments;
  readonly register: Register;
  readonly settlement: Settlement;
  readonly journal: Journal;
  readonly accountOf: AccountResolver;
}

/** Run every action due this period, one instruction per holder of record (Register E1.a). */
export function runCorporateActions(period: Period, cycle: Cycle, d: ActionDeps): void {
  for (const i of d.instruments.all()) {
    if (!i.status.live) continue;
    const profile = d.registry.instrumentKind(i.kind);
    for (const action of profile.due(i, period, d.calendar)) {
      switch (action.kind) {
        case 'coupon':
          payToHolders(i, action.amountPerUnit, `coupon on ${i.id}`, period, cycle, d);
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
    const draft: InstructionDraft = {
      legs: [leg],
      cause: 'coupon',
      reason: `${reason} to ${holderId}`,
    };
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
    d.journal.record(period, cycle, 'instrument.ceased', [i.id], { reason: 'maturity' }, true);
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
