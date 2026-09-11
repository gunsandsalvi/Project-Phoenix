/**
 * Corporate actions: what an instrument's terms say falls due, applied by the kernel. A coupon pays
 * to the holders of record, read at the moment it is applied (Register E1, E1.a); a maturity pays
 * face and extinguishes the instrument (Register E2, B4, Bond N10). Which actions fall due is the
 * instrument kind's profile to say (Law 15); the kernel only knows the vocabulary.
 *
 * A payment that FAILS is where a default begins (XI-1). The kernel does not know what a default
 * is — that is the instrument's own definition, observable by a holder (Bond N12) — so when a coupon
 * or a maturity does not settle it asks the profile, and if the answer is yes it journals the event
 * publicly and writes the status. It never decides, never draws and never repairs: a default here is
 * the consequence of a payment that did not happen, which is Firm Birth C2.a's whole requirement.
 *
 * @spec Register E1 Register E1.a Register E2 Register B4 Register E5 Bond N10 Bond N12 Bond N13 Money C1.c Money E1 Money E1.a Money G3.a Banks Lending E1 Banks Lending E2 Firm Birth C1 Firm Birth C3 XI-1 Law 15
 */
import { issuedBy, issuerOf } from '../register/instruments.js';
import type { Calendar, Cycle, Period } from '../calendar/calendar.js';
import { assertNever } from '../core/assert.js';
import { currencyUnit, type CurrencyCode, type PartyId } from '../core/ids.js';
import { mul } from '../core/num.js';
import { none, some, type Option } from '../core/option.js';
import type { Journal } from '../journal/journal.js';
import type { CellSide, Failed, InstructionDraft, Leg } from '../ledger/instruction.js';
import { cellSide, shareFor, type Settlement, totalFor } from '../ledger/settlement.js';
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
    if (issuedBy(i, holderId)) continue;
    const holder = d.parties.get(holderId);
    const perMemberUnits = d.register.quantity(holderId, i.id);
    if (perMemberUnits <= 0) continue;
    // Law 8: WHAT IS OWED IS WHAT EXISTS. A coupon is a rate on a holding, so what it comes to is
    // a fraction of a tick more often than not, and the payment is the tick below it — for each
    // member of a cell separately, because each of them is a real holder with a real account
    // (XI-15). What the fraction would have been is not owed, because it is not money.
    const share = shareFor(d.registry, holder, currencyUnit(i.ccy), mul(perMemberUnits, perUnit, 'coupon cash'));
    const perMemberCash = share.perMember;
    const total = share.total;
    if (total <= 0) continue;
    const leg: Leg = {
      kind: 'money',
      from: d.accountOf(issuerOf(i), i.ccy),
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
    const record = d.settlement.settle(draft, period, cycle);
    if (record.outcome === 'failed') defaulted(i, record, holderId, total, period, cycle, d);
  }
}

/**
 * XI-1, Bond N12: a payment fell due and did not happen. Whether that is a DEFAULT is the
 * instrument's own definition to say — a sovereign bond has no covenants, so nothing but this can
 * be one; a loan will have both (worklist 6) — and a kind that cannot default does not answer.
 *
 * What the kernel does when the answer is yes is exactly three things, and nothing else: it says so
 * publicly, because a default is an event others react to (Firm Birth C3); it writes the status,
 * because a status no path writes is not a status (Banks Lending E2); and it leaves the claim where
 * it is. It books no provision and takes no loss: whose loss it is, and how much, is the holders'
 * own assessment and belongs to them (Banks Lending D2), not to the machinery that noticed.
 */
function defaulted(
  i: Instrument,
  failed: Failed,
  holder: PartyId,
  amountDue: number,
  period: Period,
  cycle: Cycle,
  d: ActionDeps,
): void {
  const profile = d.registry.instrumentKind(i.kind);
  if (profile.defaultOn === undefined) return;
  const met = profile.defaultOn(i, failed);
  if (met === undefined) return;
  const issuer = issuerOf(i);
  const rank = profile.ranking(i);
  d.journal.record(
    period,
    cycle,
    'credit.default',
    [issuer, i.id, holder],
    {
      issuer,
      instrument: i.id,
      holder,
      amountDue,
      definition: met.met,
      seniority: rank.seniority,
      claim: rank.claim,
    },
    true,
  );
  d.instruments.markDefaulted(i.id);
  accelerate(i, period, cycle, d);
}

/**
 * Corporate Credit G2: a default on one of an issuer's instruments makes the others DUE, where
 * their own terms say so. Which is a thing the terms say and not a thing the kernel decides: a
 * sovereign's do not (Sovereign G3), and a corporate line's will (worklist 13f).
 *
 * Being made due is being redeemed now, through the same path a maturity takes — so an issuer that
 * cannot pay the accelerated face fails that too, and each failure is its own event. There is no
 * separate acceleration machinery for a state to get out of step with.
 */
function accelerate(defaultedOn: Instrument, period: Period, cycle: Cycle, d: ActionDeps): void {
  if (d.registry.instrumentKind(defaultedOn.kind).accelerates !== true) return;
  const issuer = issuerOf(defaultedOn);
  for (const other of d.instruments.all()) {
    if (other.id === defaultedOn.id || !other.status.live) continue;
    if (!issuedBy(other, issuer)) continue;
    if (d.registry.instrumentKind(other.kind).accelerates !== true) continue;
    d.journal.record(
      period,
      cycle,
      'credit.accelerated',
      [issuer, other.id, defaultedOn.id],
      { issuer, instrument: other.id, because: defaultedOn.id },
      true,
    );
    redeem(other, period, cycle, d);
  }
}

function redeem(i: Instrument, period: Period, cycle: Cycle, d: ActionDeps): void {
  for (const holderId of [...d.register.holdersOf(i.id)]) {
    if (issuedBy(i, holderId)) continue;
    const holder = d.parties.get(holderId);
    const perMemberUnits = d.register.quantity(holderId, i.id);
    if (perMemberUnits <= 0) continue;
    const units = totalFor(holder, perMemberUnits);
    const cashPerMember = perMemberUnits; // face: one unit of par pays one unit of currency (N10)
    // N10: par is money, so the units and the cash are on the same grid by construction; nothing
    // needs rounding here and rounding it would leave a stub of a line outstanding.
    const legs: Leg[] = [
      {
        kind: 'asset',
        from: holderId,
        to: issuerOf(i),
        instrument: i.id,
        qty: units,
        pricePerUnit: some(1),
        accruedPerUnit: none(),
        fromCell: optionalCell(
          holder.representation === 'cell' ? cellSide(holder, perMemberUnits) : undefined,
        ),
        toCell: none(),
      },
      {
        kind: 'money',
        from: d.accountOf(issuerOf(i), i.ccy),
        to: d.accountOf(holderId, i.ccy),
        ccy: i.ccy,
        amount: units,
        fromCell: none(),
        toCell: optionalCell(
          holder.representation === 'cell' ? cellSide(holder, cashPerMember) : undefined,
        ),
      },
    ];
    const record = d.settlement.settle(
      { legs, cause: 'maturity', reason: `maturity of ${i.id} to ${holderId}` },
      period,
      cycle,
    );
    // N10, N12: face that fell due and was not paid is a missed payment like any other.
    if (record.outcome === 'failed') defaulted(i, record, holderId, units, period, cycle, d);
  }
  const still = d.instruments.get(i.id);
  if (still.issued === 0 || Math.abs(still.issued) <= d.register.heldTotal(i.id).dust) {
    if (still.issued !== 0) d.instruments.adjustIssued(i.id, -still.issued);
    d.instruments.cease(i.id, period);
    d.journal.record(period, cycle, 'instrument.ceased', [i.id], { reason: 'maturity' }, true);
  }
}

function optionalCell(c: ReturnType<typeof cellSide>): Option<CellSide> {
  return c === undefined ? none() : some(c);
}

/**
 * Money B1, A1; Currency A2, B1: an account is (holder, issuer, currency), and every party banks
 * somewhere — IN THE MONEY IT BANKS IN.
 *
 * A party's own bank issues its own region's money and nobody else's (A1: no money without an
 * issuer, and an issuer issues one). So a party that is paid in a money its bank does not issue is
 * paid into an account at THAT money's own central bank — which is what a correspondent account
 * abroad is, and the only place a claim in a foreign money can be without somebody inventing one.
 *
 * It is a read of the registry (`centralBankOf`) and not a rule about who may hold what: any party
 * can be owed any money, and where the claim sits follows from whose liability that money IS.
 */
export function accountResolver(
  parties: Parties,
  centralBankOf: (ccy: CurrencyCode) => PartyId,
  homeOf: (party: PartyId) => CurrencyCode,
): (party: PartyId, ccy: CurrencyCode) => { holder: PartyId; issuer: PartyId } {
  return (party, ccy) => {
    const p = parties.get(party);
    if (homeOf(party) === ccy) return { holder: party, issuer: p.bank };
    return { holder: party, issuer: centralBankOf(ccy) };
  };
}
