/**
 * The two rows the money market writes: an unsecured claim on a name, and a claim secured on paper.
 *
 * @spec Money Market B2 Money Market B3 Money Market B3.c Money Market B6 Money Market B7 Banks Funding A2 Banks Funding A2.a Bond N12 Bond N13 Bond N13.a Central Bank D1 Law 9 Law 15
 *
 * They are two kinds because a market names them as two things (Law 9) and because what a holder is
 * entitled to when the borrower does not pay differs: the unsecured lender queues at the estate, and
 * the secured one has paper in its hand. Nothing in this module branches on which is which — the
 * venue says what it strikes and the profile answers for the row it wrote (Law 15).
 *
 * Both are ROWS in the same register as anything else, with a lender of record, a borrower and a
 * date they fall due; a bank's interbank book is the sum of them and is never a number anywhere.
 * B6: the tenor is in the terms, because overnight and term are the same instrument for a different
 * number of days, and A2.a's roll is what happens when the day arrives and the row is not renewed.
 */
import { period as asPeriod, type Period } from '../../calendar/calendar.js';
import { compareCivil, formatCivil, type Civil } from '../../calendar/civil.js';
import { yearFraction, type DayCount } from '../../calendar/daycount.js';
import { InvalidRegistry } from '../../core/errors.js';
import { percent } from '../../core/format.js';
import {
  currencyUnit,
  instrumentId,
  instrumentKindId,
  type InstrumentId,
  type PartyId,
} from '../../core/ids.js';
import { add, mul } from '../../core/num.js';
import { issuerOf, type Instrument, type Terms } from '../../register/instruments.js';
import type { CashFlow, DueAction, InstrumentKindProfile } from '../../registry/kinds.js';
import type { Namer } from '../../registry/naming.js';

export const INTERBANK = instrumentKindId('interbank');
export const REPO = instrumentKindId('repo');

/** The period after this one; the calendar counts, this only names the next index (Money G3.a). */
const next = (p: Period): Period => asPeriod(p + 1);

/** What one unit of collateral stands behind, and how much of it (Register D5.b). */
export interface Pledged {
  readonly instrument: InstrumentId;
  /** Total units bound. */
  readonly qty: number;
  /** B3.b: what the lender valued a unit at when it lent — its own number, not the market's. */
  readonly valuedAt: number;
}

/**
 * A money-market row. The borrower is the instrument's issuer: it owes the money.
 *
 * `collateral` is empty for an unsecured row and never empty for a secured one, and it is named
 * `collateral` rather than `security` on purpose: a bank's loan book and its money-market book are
 * two books, and a reader that could not tell a repo from a term loan to a firm would be adding
 * them (Law 4).
 */
export interface RowTerms extends Terms {
  readonly kind: typeof INTERBANK | typeof REPO;
  readonly lender: PartyId;
  readonly borrower: PartyId;
  /** B4: what cleared in the session that struck it, per annum. */
  readonly rate: number;
  readonly drawn: Civil;
  readonly maturity: Civil;
  readonly dayCount: DayCount;
  /** B6: which book struck it, in the words the venue uses. */
  readonly tenor: string;
  readonly collateral: readonly Pledged[];
}

export function isRow(t: Terms): t is RowTerms {
  return 'lender' in t && 'borrower' in t && 'collateral' in t;
}

export function rowTerms(i: Instrument): RowTerms {
  if (!isRow(i.terms)) throw new InvalidRegistry('Money Market B1', `${i.id} is not a money-market row`);
  return i.terms;
}

export const isSecured = (t: RowTerms): boolean => t.collateral.length > 0;

/** One row per (lender, borrower, drawing), named so a reader can see whose it is (Law 9). */
export function rowId(
  what: 'interbank' | 'repo',
  lender: PartyId,
  borrower: PartyId,
  n: number,
): InstrumentId {
  return instrumentId(`${what}:${lender}:${borrower}:${n}`);
}

function interestTo(t: RowTerms, from: Civil, to: Civil): number {
  return mul(t.rate, yearFraction(t.dayCount, from, to), 'interest');
}

/**
 * A2.a, B6: INTEREST FALLS DUE WITH THE PRINCIPAL, on the day the row runs out. That is what a
 * money-market row is: cash for a stated number of days, back with what it earned. Nothing accrues
 * period by period into a payment stream, because the row does not live long enough to have one —
 * and a row written inside a period would otherwise have its first period's interest fall due
 * before it existed, which is a payment nobody could have made.
 */
function dueOn(
  i: Instrument,
  period: Period,
  cal: { startOf: (p: Period) => Civil; place: (c: Civil) => Period },
): readonly DueAction[] {
  if (!isRow(i.terms)) return [];
  const t = i.terms;
  if (cal.place(t.maturity) !== period) return [];
  const out: DueAction[] = [];
  const amountPerUnit = interestTo(t, t.drawn, t.maturity);
  if (amountPerUnit > 0) out.push({ kind: 'coupon', date: t.maturity, amountPerUnit });
  out.push({ kind: 'maturity', date: t.maturity });
  return out;
}

/** One payment, on one day: the principal and everything it earned (Sovereign D2 reads these). */
function flows(i: Instrument, after: Civil): readonly CashFlow[] {
  if (!isRow(i.terms)) return [];
  const t = i.terms;
  if (compareCivil(t.maturity, after) <= 0) return [];
  return [{ date: t.maturity, perUnit: add(1, interestTo(t, t.drawn, t.maturity), 'at maturity') }];
}

function shared(id: typeof INTERBANK | typeof REPO): Omit<InstrumentKindProfile, 'ranking' | 'displayName'> {
  return {
    id,
    // A1.a's reasoning, applied here: these rows have no market, so they have no market price. They
    // are carried at what they cost and the loss lands when the estate pays less than that (XI-1).
    pricing: 'carriedAtCost',
    carry: 'cost',
    liabilityOfIssuer: true,
    unit: (ccy) => currencyUnit(ccy),
    validateTerms: (t) => {
      if (!isRow(t)) throw new InvalidRegistry('Money Market B1', 'not money-market row terms');
      if (compareCivil(t.drawn, t.maturity) >= 0) {
        throw new InvalidRegistry('Money Market B6', 'a row matures after it is drawn');
      }
      if (t.lender === t.borrower) {
        throw new InvalidRegistry('Money Market B1', 'a row has two parties, and they differ');
      }
      if ((t.kind === REPO) !== t.collateral.length > 0) {
        throw new InvalidRegistry(
          'Money Market B3',
          'a repo is secured on something and an unsecured row is secured on nothing',
        );
      }
    },
    due: dueOn,
    // N9.b: what it has earned since it was drawn, which for a row that pays at the end is all of
    // it. Nothing trades these, so nothing pays it across — it is stated because the contract says
    // it is owed, and a reader that wanted it would otherwise have to re-derive it (Law 19).
    accrued: (i, on) => (isRow(i.terms) ? interestTo(i.terms, i.terms.drawn, on) : 0),
    cashFlows: flows,
    // B7, D4: a row that is not repaid is a default, and it is the event the lender acts on.
    defaultOn: (i, failed) =>
      failed.reason.party === issuerOf(i)
        ? { met: 'a payment on the money-market row fell due and the borrower did not make it' }
        : undefined,
    accelerates: false,
  };
}

/** B2: an unsecured claim on the name. If the name fails, the lender queues with the others. */
export const interbankKind: InstrumentKindProfile = {
  ...shared(INTERBANK),
  displayName: (i: Instrument, namer: Namer) => {
    if (!isRow(i.terms)) return String(i.id);
    const who = namer.issuer.some ? namer.issuer.value : String(i.terms.borrower);
    return `${who} ${i.terms.tenor} ${percent(i.terms.rate)} ${formatCivil(i.terms.maturity)}`;
  },
  ranking: () => ({
    seniority: 1,
    secured: [],
    claim: 'an unsecured claim on whatever the estate realises',
  }),
};

/** B3: a claim secured on named paper. If the name fails, the lender has the paper (B3.c). */
export const repoKind: InstrumentKindProfile = {
  ...shared(REPO),
  displayName: (i: Instrument, namer: Namer) => {
    if (!isRow(i.terms)) return String(i.id);
    const who = namer.issuer.some ? namer.issuer.value : String(i.terms.borrower);
    const on = i.terms.collateral.map((c) => String(c.instrument)).join(', ');
    return `${who} ${i.terms.tenor} repo ${percent(i.terms.rate)} vs ${on}`;
  },
  ranking: (i) => ({
    seniority: 0,
    secured: isRow(i.terms) ? i.terms.collateral.map((c) => ({ instrument: c.instrument, qty: c.qty })) : [],
    claim: 'the pledged paper, and an unsecured claim for anything it does not cover',
  }),
};
