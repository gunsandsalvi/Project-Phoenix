/**
 * Sovereign instruments: the bond and the bill, two instruments and not one with a flag
 * (Sovereign B1), answering the bond contract the sovereign's way.
 *
 * @spec Bond N9.b Sovereign F2 Sovereign B1 Sovereign B2 Sovereign B4 Sovereign B5 Sovereign B6 Sovereign B7 Sovereign F1 Sovereign F3 Bond N4 Bond N5 Bond N5.a Bond N5.c Bond N6 Bond N10 Bond N11 Bond N12 Bond N13 Bond N13.a Bond N14 Money G3.a Money G3.c Law 9 Law 15
 *
 * A module: it registers kinds and profiles and touches no kernel store. The auction, the curve and
 * the treasury's programme are separate modules (worklist 3); this one is only the paper.
 */
import { FACE_TICK } from '../../registry/grid.js';
import { MONEY_PIECES } from '../../registry/grid.js';
import { compareCivil, formatCivil } from '../../calendar/civil.js';
import { InvalidRegistry } from '../../core/errors.js';
import { percent } from '../../core/format.js';
import { issuerOf } from '../../register/instruments.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';
import { issuerName } from '../../registry/naming.js';
import type { SystemModule } from '../../world/module.js';

export * from '../../registry/claims.js';
import {
  isBill,
  isBond,
  PAR,
  SOVEREIGN_BILL,
  SOVEREIGN_BOND,
  validateDates,
  accruedOf,
  cashFlowsOf,
  dueOf,
} from '../../registry/claims.js';

export const sovereignBond: InstrumentKindProfile = {
  id: SOVEREIGN_BOND,
  pricing: 'cleared',
  // Law 8: government paper is quoted as a fraction of its own face, and it moves in
  // ten-thousandths of one — a basis point of price, which is the grid a sovereign book
  // actually quotes on. A cent of face would be a whole percentage point of a bond.
  priceTick: FACE_TICK,
  carry: 'mark',
  liabilityOfIssuer: true,
  // Register B3, XI-9: A SOVEREIGN OWES THE FACE. Its paper falling in price is the holder's loss,
  // never the treasury's gain — a state that grew richer as its own credit went is a state with no
  // funding constraint at all, which is the one thing XI-9 exists to prevent.
  owes: 'face',
  unit: () => PAR,
  // Bond N12, N13, N13.a as §8's table answers them, and every answer is stated rather than
  // implied. There are no covenants to breach, so nothing but a missed payment can be a default.
  // There is no estate — nothing of a state is seizable — so the claim is a negotiated exchange and
  // the sanction is exclusion from the market (Sovereign G3, G5). And the ranking never varies:
  // pari passu, always, which is why every line of an issuer carries the same seniority number.
  ranking: () => ({
    seniority: 0,
    secured: [],
    claim: 'nothing seizable: a negotiated exchange, and exclusion from the market until there is one',
  }),
  defaultOn: (i, failed) =>
    failed.reason.party === issuerOf(i)
      ? { met: 'a payment fell due and the issuer did not make it' }
      : undefined,
  // Sovereign G3: a missed payment on one line does not make the others due.
  accelerates: false,
  validateTerms: (t) => {
    if (!isBond(t)) throw new InvalidRegistry('Sovereign B1', 'not sovereign bond terms');
    validateDates(t.issueDate, t.maturity, 'sovereign bond');
    // eslint-disable-next-line phoenix/no-kind-branch -- a periodicity tag, not a party or product kind
    if (t.coupon.per.kind !== 'annual') {
      throw new InvalidRegistry('Law 8', 'a bond coupon is quoted per annum');
    }
  },
  displayName: (i, namer) => {
    const who = issuerName(namer, i.id);
    if (!isBond(i.terms)) return `${who} bond`;
    // Law 9: issuer + coupon + maturity.
    return `${who} ${percent(i.terms.coupon.amount)} ${formatCivil(i.terms.maturity)}`;
  },
  // N6, N9.b, N5.a, N10 (13f): the schedule is the kernel's, because when a coupon falls and how it
  // accrues is the same fact for any dated bond whoever issued it (Law 4). What is sovereign about
  // this line is who can fail, what ranks where and that nothing may be breached — stated above.
  due: (i, period, cal) => (isBond(i.terms) ? dueOf(i.terms, period, cal) : []),
  accrued: (i, on, cal) => (isBond(i.terms) ? accruedOf(i.terms, on, cal) : 0),
  cashFlows: (i, after, cal) =>
    isBond(i.terms) ? cashFlowsOf(i.terms, after, cal) : [],
};


export const sovereignBill: InstrumentKindProfile = {
  id: SOVEREIGN_BILL,
  pricing: 'cleared',
  // Law 8: government paper is quoted as a fraction of its own face, and it moves in
  // ten-thousandths of one — a basis point of price, which is the grid a sovereign book
  // actually quotes on. A cent of face would be a whole percentage point of a bond.
  priceTick: FACE_TICK,
  carry: 'mark',
  liabilityOfIssuer: true,
  // Register B3, XI-9: A SOVEREIGN OWES THE FACE. Its paper falling in price is the holder's loss,
  // never the treasury's gain — a state that grew richer as its own credit went is a state with no
  // funding constraint at all, which is the one thing XI-9 exists to prevent.
  owes: 'face',
  unit: () => PAR,
  // Bond N12, N13, N13.a as §8's table answers them, and every answer is stated rather than
  // implied. There are no covenants to breach, so nothing but a missed payment can be a default.
  // There is no estate — nothing of a state is seizable — so the claim is a negotiated exchange and
  // the sanction is exclusion from the market (Sovereign G3, G5). And the ranking never varies:
  // pari passu, always, which is why every line of an issuer carries the same seniority number.
  ranking: () => ({
    seniority: 0,
    secured: [],
    claim: 'nothing seizable: a negotiated exchange, and exclusion from the market until there is one',
  }),
  defaultOn: (i, failed) =>
    failed.reason.party === issuerOf(i)
      ? { met: 'a payment fell due and the issuer did not make it' }
      : undefined,
  // Sovereign G3: a missed payment on one line does not make the others due.
  accelerates: false,
  validateTerms: (t) => {
    if (!isBill(t)) throw new InvalidRegistry('Sovereign B1', 'not sovereign bill terms');
    validateDates(t.issueDate, t.maturity, 'sovereign bill');
  },
  displayName: (i, namer) =>
    isBill(i.terms)
      ? `${issuerName(namer, i.id)} bill ${formatCivil(i.terms.maturity)}`
      : `${issuerName(namer, i.id)} bill`,
  // N5.c: no coupon; the return is the discount to par, and the bill accretes against its own print.
  due: (i, period, cal) =>
    isBill(i.terms) && cal.periodOf(i.terms.maturity) === period
      ? [{ kind: 'maturity', date: i.terms.maturity }]
      : [],
  // F2: a bill accretes against its own cleared price; nothing accrues on the paper itself, so
  // nothing travels with a trade beyond the price.
  accrued: () => 0,
  // N5.c: one payment, par at maturity. The discount to it is the whole return.
  cashFlows: (i, after) =>
    isBill(i.terms) && compareCivil(i.terms.maturity, after) > 0
      ? [{ date: i.terms.maturity, perUnit: 1 }]
      : [],
};

export const sovereignInstruments: SystemModule = {
  id: 'sovereign-instruments',
  spec: 'Sovereign B',
  requires: [],
  instrumentKinds: [sovereignBond, sovereignBill],
  partyKinds: [],
  curveFamilies: [],
  // Bond N2, Law 8: par is money, so its smallest piece is money's — a line is issued, traded and
  // redeemed in the same grid the coupons on it are paid in.
  units: [{ id: PAR, name: 'units of par', perUnit: MONEY_PIECES }],
  params: [],
  phases: [],
  participants: [],
  families: [],
};
