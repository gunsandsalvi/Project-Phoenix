/**
 * An arrear: a payment that fell due and was not made (Money E1, E1.a, E1.b; XI-8; 12a.1).
 *
 * @spec Money E1 Money E1.a Money E1.b Money E2 XI-8 Firm Birth D2 Law 4 Law 5
 *
 * A payer that cannot pay is a real state with a real consequence (E1): the payee has a receivable
 * that did not arrive (E1.b), and it is a ROW — issued by the payer, held by the payee, in the
 * money it was due in, carried at what it is (there is no market in it) — written by settlement in
 * the same pass as the fail, so a missed payment is never a number nobody holds. It names the
 * instruction that failed and the CLASS of payment it was (a wage, a rent, a coupon...), because
 * the class is what an estate ranks it by (XI-8, Firm Birth D2) and what a later due of the same
 * class is presented behind (12a.3). It is redeemed when it is paid (E2.a: a new payment, itself
 * traceable) and never reversed.
 */
import { currencyUnit, instrumentId, instrumentKindId, type InstrumentId } from '../core/ids.js';
import { InvalidRegistry } from '../core/errors.js';
import type { PartyId } from '../core/ids.js';
import type { Instrument, Terms } from './instruments.js';
import type { InstrumentKindProfile } from '../registry/kinds.js';
import { issuerName } from '../registry/naming.js';

export const ARREAR = instrumentKindId('arrear');

/** The class of payment it was — the receipt the leg carried, or none. */
export type PaymentClass =
  | 'wage'
  | 'rent'
  | 'interest'
  | 'dividend'
  | 'disposal'
  | 'sale'
  | 'returnOfCapital'
  | 'borrowing'
  | 'transfer'
  | 'tax'
  | 'claim'
  | 'unclassified';

export interface ArrearTerms extends Terms {
  readonly kind: typeof ARREAR;
  /** The instruction that failed: the record this row is the consequence of (E1.a). */
  readonly failed: number;
  readonly class: PaymentClass;
  readonly payer: PartyId;
  readonly payee: PartyId;
}

export function isArrearTerms(t: Terms): t is ArrearTerms {
  return t.kind === ARREAR && typeof (t as { failed?: unknown }).failed === 'number';
}

export function isArrear(i: Instrument): boolean {
  return isArrearTerms(i.terms);
}

export function arrearTerms(i: Instrument): ArrearTerms {
  if (!isArrearTerms(i.terms)) {
    throw new InvalidRegistry('Money E1', `${i.id} is not an arrear`);
  }
  return i.terms;
}

export const arrearId = (failed: number, n: number): InstrumentId =>
  instrumentId(`arrear.${String(failed)}.${String(n)}`);

/**
 * XI-8, Firm Birth D2: WHERE A CLASS RANKS in an estate — an ORDER, senior first, because the order
 * of claims is a fact about the law of the place and never something a mechanism works out. Wages
 * first, then what the state is owed, then the roof, then interest and borrowing, then what was
 * owed for goods and services (an unclassified one ranks with them), and the owners' claims last.
 */
export const CLASS_ORDER: readonly (readonly PaymentClass[])[] = [
  ['wage'],
  // 14.3: a policyholder's unpaid claim ranks with what the state and a counterparty are owed.
  ['transfer', 'tax', 'claim'],
  ['rent'],
  ['interest', 'borrowing'],
  ['disposal', 'sale', 'unclassified'],
  ['dividend', 'returnOfCapital'],
];

export function seniorityOf(c: PaymentClass): number {
  const at = CLASS_ORDER.findIndex((tier) => tier.includes(c));
  return at < 0 ? CLASS_ORDER.length : at;
}

export const arrearKind: InstrumentKindProfile = {
  id: ARREAR,
  // No market exists for a missed payment; it is carried at what it is (Money E1.b).
  pricing: 'carriedAtCost',
  carry: 'cost',
  liabilityOfIssuer: true,
  owes: 'face',
  unit: (ccy) => currencyUnit(ccy),
  validateTerms: (t) => {
    if (!isArrearTerms(t)) throw new InvalidRegistry('Money E1', 'not arrear terms');
    if (t.payer === t.payee) throw new InvalidRegistry('Law 5', 'an arrear has two parties, and they differ');
  },
  // Law 9: a market would name it by who owes it, for what, since when.
  displayName: (i: Instrument, namer) => {
    const t = isArrearTerms(i.terms) ? i.terms : undefined;
    const who = namer.issuer.some ? namer.issuer.value : issuerName(namer, i.id);
    return t === undefined ? String(i.id) : `${who} ${t.class} in arrears (instruction ${String(t.failed)})`;
  },
  // 12a.3: IT FALLS DUE EVERY PERIOD UNTIL IT IS PAID — the whole of it, at par, presented before
  // any new due of the payer's (the kernel walks arrears first); what is paid is redeemed (E2.a: a
  // new payment, itself traceable) and what is not stands, and settlement writes no row on a row.
  due: (_i, period, cal) => [{ kind: 'maturity', date: cal.startOf(period) }],
  accrued: () => 0,
  cashFlows: () => [],
  ranking: (i) => ({
    seniority: seniorityOf(isArrearTerms(i.terms) ? i.terms.class : 'unclassified'),
    secured: [],
    claim: 'a payment that fell due and was not made, as an unsecured claim of its class',
  }),
};
