/**
 * The corporate bond: a firm borrows from the market instead of from a bank.
 *
 * @spec Corporate Credit A1 Corporate Credit A2 Corporate Credit A2.a Corporate Credit A3 Corporate Credit B1 Corporate Credit B2 Corporate Credit B2.a Corporate Credit B3 Corporate Credit B4 Corporate Credit G1 Corporate Credit G2 Bond N4 Bond N5.a Bond N6 Bond N11 Bond N12 Bond N13 Bond N13.a Reporting A2 Law 2 Law 3 Law 4 Law 9 Law 19
 *
 * A FIRM WITH ONE LENDER HAS ONE OPINION ABOUT IT. Until now every borrower in this world went to a
 * bank, took the keenest quote it was given, and that was the whole of the credit market: one
 * assessor per borrower, and a funding cost that moved only when a bank's own cost of funds moved.
 * A bond is the other channel — many holders, each pricing the name from its own view, and a price
 * that is struck in a book rather than offered across a desk.
 *
 * WHAT IS SOVEREIGN ABOUT A SOVEREIGN BOND IS NOT THE SCHEDULE. When a coupon falls and how it
 * accrues is the same fact for any dated bond whoever issued it, so both kinds read ONE statement
 * of it (`registry/claims.ts`). What differs is the three things that make corporate credit a
 * different subject: the issuer can FAIL, so a missed payment is a default and the estate has a
 * ranking to read; the line is SENIOR or SUBORDINATED, and the number is on the instrument where
 * the waterfall already looks (N13.a); and there are COVENANTS, which is the half of credit that
 * happens before anybody misses a payment.
 *
 * A COVENANT IS A TERM OF THE ISSUE AND NOT A BOUND (Law 6). It is what this issuer promised when
 * it borrowed — a line its own accounts must stay the right side of — and breaching it is an EVENT
 * with consequences, never a number that gets pushed back inside a range. It is tested on the
 * PUBLISHED accounts (Reporting A2), with the lag publishing already has, because a covenant a
 * lender could test on private books is not a covenant, it is surveillance.
 */
import { compareCivil } from '../../calendar/civil.js';
import {
  instrumentId,
  instrumentKindId,
  type InstrumentId,
  type PartyId,
} from '../../core/ids.js';
import { InvalidRegistry } from '../../core/errors.js';
import { div, mul, sub } from '../../core/num.js';
import { percent } from '../../core/format.js';
import { formatCivil } from '../../calendar/civil.js';
import { issuerOf, type Instrument, type Terms } from '../../register/instruments.js';
import {
  accruedOf,
  cashFlowsOf,
  dueOf,
  type CouponSchedule,
} from '../../registry/claims.js';
import { FACE_TICK, MONEY_PIECES } from '../../registry/grid.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';
import { unitId } from '../../core/ids.js';
import { issuerName } from '../../registry/naming.js';
import type { Violation, Family } from '../../audit/audit.js';
import type { MechanismContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

export const CORPORATE_BOND = instrumentKindId('corporate.bond');

/** N9: quoted as a fraction of its own face, like any other bond. */
export const CORPORATE_PAR = unitId('corporate.par');

/**
 * B2, B2.a: WHAT THIS ISSUER PROMISED ITS LENDERS, struck when it borrowed and stated on the paper.
 *
 * Two lines, because they are the two questions a lender actually asks and they fail in different
 * worlds: how much it owes against what it has (a balance-sheet test, which a fall in asset prices
 * breaks), and what it earns against what falls due (an income test, which a bad year breaks). A
 * firm can pass either while failing the other, and which one goes says what went wrong.
 *
 * Neither is a parameter. They are TERMS — this issuer's own commitment at this issue — and what a
 * given firm promised is an outcome of what it had to promise to be lent to.
 */
export interface Covenants {
  /** B2: the most it may owe against what it holds, as the issuer's own published accounts read. */
  readonly leverage: number;
  /** B2: the least it must earn against what falls due, on the same published accounts. */
  readonly coverage: number;
}

export interface CorporateBondTerms extends Terms, CouponSchedule {
  readonly kind: typeof CORPORATE_BOND;
  readonly issuer: PartyId;
  /**
   * N13.a: WHERE IT RANKS, as a number the waterfall already orders by. Nothing anywhere asks what
   * kind of instrument it is — a claim that ranks behind another is a claim with a bigger number.
   */
  readonly seniority: number;
  readonly covenants: Covenants;
}

export const isCorporateBond = (t: Terms): t is CorporateBondTerms =>
  'covenants' in t && 'seniority' in t && 'coupon' in t;

export function corporateBondTerms(i: Instrument): CorporateBondTerms {
  if (!isCorporateBond(i.terms)) {
    throw new InvalidRegistry('Corporate Credit A1', `${i.id} is not a corporate bond`);
  }
  return i.terms;
}

/** Law 9: a market names a bond by its issuer, its coupon and its maturity. Never by an id. */
export const corporateBondId = (issuer: PartyId, n: number): InstrumentId =>
  instrumentId(`bond:${issuer}:${n}`);

export const corporateBond: InstrumentKindProfile = {
  id: CORPORATE_BOND,
  pricing: 'cleared',
  priceTick: FACE_TICK,
  carry: 'mark',
  liabilityOfIssuer: true,
  // Register B3, XI-3: A BORROWER OWES THE FACE. Its paper falling because the market has lost
  // faith in it is the HOLDER's loss and never the issuer's gain — a firm that booked a profit on
  // the way down would grow more solvent the closer it came to failing, which puts the solvency
  // trigger out of reach exactly when it is supposed to fire.
  owes: 'face',
  unit: () => CORPORATE_PAR,
  ranking: (i) => ({
    seniority: isCorporateBond(i.terms) ? i.terms.seniority : 0,
    secured: [],
    claim: 'an unsecured claim on the estate, ranking where the paper says it ranks',
  }),
  /** N12: a payment fell due and the issuer did not make it. That is the whole definition. */
  defaultOn: (i, failed) =>
    failed.reason.party === issuerOf(i)
      ? { met: 'a payment fell due and the issuer did not make it' }
      : undefined,
  /**
   * G2: AND IT IS DUE ON THE OTHERS TOO. A firm that misses one line has missed them all: its
   * lenders do not wait their turn while the estate empties. A sovereign has no cross-default
   * (Sovereign G3) and says so; a firm has one and says so here.
   */
  accelerates: true,
  validateTerms: (t) => {
    if (!isCorporateBond(t)) throw new InvalidRegistry('Corporate Credit A1', 'not bond terms');
    if (compareCivil(t.issueDate, t.maturity) >= 0) {
      throw new InvalidRegistry('Bond N4', 'a bond matures after it is issued');
    }
    // eslint-disable-next-line phoenix/no-kind-branch -- a periodicity tag, not a party or product
    if (t.coupon.per.kind !== 'annual') {
      throw new InvalidRegistry('Law 8', 'a bond coupon is quoted per annum');
    }
    if (t.covenants.leverage <= 0 || t.covenants.coverage <= 0) {
      throw new InvalidRegistry(
        'Corporate Credit B2',
        'a covenant line of nothing is a promise to nothing',
      );
    }
  },
  displayName: (i, namer) => {
    const who = issuerName(namer, i.id);
    if (!isCorporateBond(i.terms)) return `${who} bond`;
    return `${who} ${percent(i.terms.coupon.amount)} ${formatCivil(i.terms.maturity)}`;
  },
  // N6, N9.b, N5.a, N10: the kernel's schedule, because when a coupon falls is not a corporate fact.
  due: (i, period, cal) => (isCorporateBond(i.terms) ? dueOf(i.terms, period, cal) : []),
  accrued: (i, on, cal) => (isCorporateBond(i.terms) ? accruedOf(i.terms, on, cal) : 0),
  cashFlows: (i, after, cal) => (isCorporateBond(i.terms) ? cashFlowsOf(i.terms, after, cal) : []),
};

/* --------------------------------------------------------------------------------------------
 * THE COVENANT
 * ------------------------------------------------------------------------------------------ */

/**
 * B2, B2.a, G1, Reporting A2: WHAT THE PUBLISHED ACCOUNTS SAY ABOUT WHAT WAS PROMISED.
 *
 * Both tests are reads of one published report and of nothing else (Law 19, Law 4). A covenant
 * tested on numbers only the issuer can see is not a covenant; a covenant tested on a second set of
 * accounts computed here would be a second set of accounts (A2.a).
 *
 * A breach is an EVENT and it is all this does. It never adjusts a number, never accelerates by
 * itself and never repairs anything: what happens next is the holders' decision, and a decision is
 * not a rule.
 */
export function testCovenants(ctx: MechanismContext): void {
  const book = state(ctx);
  for (const i of ctx.instruments.all()) {
    if (!i.status.live || !isCorporateBond(i.terms)) continue;
    const t = i.terms;
    const said = latestReport(ctx, t.issuer);
    if (said === undefined) continue;
    if (book.tested[String(i.id)] === said.quarter) continue;
    book.tested[String(i.id)] = said.quarter;
    const broke: string[] = [];
    // B2: how much it owes against what it holds. A firm with no assets has no ratio that means
    // anything and has breached, which is what the worst case IS rather than a number pushed back.
    if (said.assets <= 0 || div(said.liabilities, said.assets, 'leverage') > t.covenants.leverage) {
      broke.push('leverage');
    }
    // B2: what it earns against what falls due on this line over a year of it.
    const owed = mul(t.coupon.amount, 1, 'what a unit of it costs a year');
    const face = ctx.register.heldTotal(i.id).value;
    const annual = mul(owed, face, 'what this line costs it a year');
    if (annual > 0 && div(said.earned, annual, 'coverage') < t.covenants.coverage) {
      broke.push('coverage');
    }
    if (broke.length === 0) continue;
    ctx.record(
      'covenant.breached',
      [t.issuer, i.id],
      {
        issuer: String(t.issuer),
        bond: String(i.id),
        quarter: said.quarter,
        broke: broke.join(' and '),
        leverage: said.assets > 0 ? div(said.liabilities, said.assets, 'leverage') : null,
        promised: t.covenants.leverage,
        earned: said.earned,
        owedPerYear: annual,
      },
      true,
    );
  }
}

interface Said {
  readonly quarter: string;
  readonly assets: number;
  readonly liabilities: number;
  readonly earned: number;
}

/** Reporting A2: the last accounts this issuer published, read off the wire and never rebuilt. */
function latestReport(ctx: MechanismContext, issuer: PartyId): Said | undefined {
  const said = ctx.journal.ofKind('reporting.report');
  for (let i = said.length - 1; i >= 0; i -= 1) {
    const e = said[i];
    if (e?.data['company'] !== issuer) continue;
    const { quarter, assets, liabilities, earned } = e.data;
    if (
      typeof quarter !== 'string' ||
      typeof assets !== 'number' ||
      typeof liabilities !== 'number' ||
      typeof earned !== 'number'
    ) {
      return undefined;
    }
    return { quarter, assets, liabilities, earned };
  }
  return undefined;
}

interface Book {
  /** Law 4: one test per line per set of accounts. A covenant is not breached twice on one report. */
  readonly tested: Record<string, string>;
}

const state = (ctx: MechanismContext): Book => ctx.state<Book>('covenants', () => ({ tested: {} }));

/**
 * B3, N13.a: every live line says where it ranks and what it promised. A bond whose ranking nobody
 * stated is the one that silently becomes a waterfall the first time somebody needs one, and a
 * covenant nobody can read is a promise nobody can hold the issuer to.
 */
function paper(): Family {
  return {
    name: 'names',
    contributor: 'corporate-bond',
    spec: 'Corporate Credit B2 Corporate Credit B3 Bond N13 Bond N13.a',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const i of view.instruments.all()) {
        if (!i.status.live || !isCorporateBond(i.terms)) continue;
        const holders = view.register.holdersOf(i.id);
        if (holders.length > 0) continue;
        out.push({
          family: 'names',
          spec: 'Corporate Credit B3',
          owner: i.id,
          size: view.register.heldTotal(i.id).value,
          unit: i.unit,
          period: view.period,
          message: `${i.id}: paper outstanding and nobody holding it`,
        });
      }
      return out;
    },
  };
}

export function corporateBondModule(): SystemModule {
  return {
    id: 'corporate-bond',
    spec: 'Corporate Credit',
    requires: ['firms', 'reporting'],
    instrumentKinds: [corporateBond],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: CORPORATE_PAR, name: 'units of par', perUnit: MONEY_PIECES }],
    params: [],
    phases: [
      {
        name: 'covenant.test',
        spec: 'Corporate Credit B2 Corporate Credit B2.a Corporate Credit G1 Reporting A2',
        // B2.a: on the PUBLISHED accounts, so after whatever published them this period. A covenant
        // a lender could test on private books is not a covenant, it is surveillance.
        anchor: { before: 'revaluation' },
        cycle: 'anchor',
        run: testCovenants,
      },
    ],
    participants: [],
    families: [paper()],
  };
}

/** B1, A3: what a line costs its issuer a year, per unit of face — a read of its own terms. */
export const annualCostOf = (t: CorporateBondTerms): number =>
  mul(t.coupon.amount, 1, 'what a unit of it costs a year');

/** B2: how far the published accounts are the right side of what was promised. Negative is a breach. */
export const headroomOn = (said: { assets: number; liabilities: number }, c: Covenants): number =>
  said.assets <= 0
    ? -1
    : sub(c.leverage, div(said.liabilities, said.assets, 'leverage'), 'what is left of the promise');
