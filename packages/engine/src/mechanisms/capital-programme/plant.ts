/**
 * Plant: a dated vintage of productive assets, held by a named firm, wearing out on a schedule of
 * its own.
 *
 * @spec Capital Programme A1 Capital Programme A3 Capital Programme A4 Capital Programme A5 Capital Programme A6 Capital Programme D1 Bond N13 Bond N13.a Law 9 Law 15 XI-6
 *
 * A VINTAGE IS AN INSTRUMENT (A6). Everything that makes one vintage different from another is on
 * it — the kind of capital it is, where it is, the date it went into service and the date it is
 * worn out — so its age survives a sale, which is the whole point: a half-worn machine bought from
 * an estate is half worn in its new owner's hands too. What it COST is the buyer's, and that lives
 * on the lot, which is where a basis lives (Register D4).
 *
 * IT WEARS OUT AS ONE SCHEDULE CHARGED IN BOTH PLACES (A3). The kernel asks the kind what a lot is
 * carried at now; the answer is straight-line over what is left of the vintage's life, and the
 * difference lands on the lot and on the holder's income together. There is no second accumulated
 * total beside it and no charge struck as a share of revenue: a firm that doubles its plant takes
 * twice the charge because it holds twice the units.
 *
 * ITS VALUE IS WHAT IT CAN PRODUCE (A5), and that is exactly what the schedule says: a vintage with
 * a third of its life left is carried at a third of what it cost whoever holds it, because a third
 * of the service is what is left to give. It is not written up, ever, and a market print does not
 * set it: a second-hand price is what one estate's plant fetched on one day and is not evidence
 * about what everybody else's is still able to make.
 */
import { WHOLE_MONEY_TICK } from '../../registry/grid.js';
import { compareCivil, formatCivil } from '../../calendar/civil.js';
import { InvalidRegistry } from '../../core/errors.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';

export * from '../../registry/physical.js';
import {
  carriedAfterWear,
  isPlantTerms,
  plantKindId,
  plantTerms,
  plantUnitId,
  type CapitalKindDecl,
} from '../../registry/physical.js';

/**
 * The kind profile of one kind of capital: everything that varies by kind, in one place (Law 15).
 *
 * It CLEARS and it is CARRIED AT COST, which is the same pair a good has and for the same reason
 * (Goods C2, E1): a market says what somebody will pay for it — a dead firm's plant is sold into
 * one, at what real bidders will give (D3) — and its holder carries it at what it cost, less what
 * it has worn out. The two are different questions and neither is the other's approximation.
 */
export function plantProfile(d: CapitalKindDecl): InstrumentKindProfile {
  const unit = plantUnitId(d.id);
  return {
    id: plantKindId(d.id),
    pricing: 'cleared',
    // Law 8: plant is quoted to the whole money. A machine is not haggled over in cents.
    priceTick: WHOLE_MONEY_TICK,
    carry: 'cost',
    // A1: plant is a thing its holder owns. Nobody promised it and nobody owes it.
    liabilityOfIssuer: false,
    // A machine is nobody's promise, so there is no issuer for a price move to reach.
    owes: 'face',
    physical: true,
    unit: () => unit,
    // Bond N13, N13.a: stated because the clause says to state it even when the answer is nothing.
    ranking: () => ({
      seniority: 0,
      secured: [],
      claim: 'nothing: it is owned outright, and nobody promised it',
    }),
    validateTerms: (t) => {
      if (!isPlantTerms(t)) {
        throw new InvalidRegistry('Capital Programme A6', `${d.id}: these are not a vintage's terms`);
      }
      if (t.capitalKind !== d.id) {
        throw new InvalidRegistry('Capital Programme A4', `${d.id} terms carry kind ${t.capitalKind}`);
      }
      if (compareCivil(t.retires, t.serviceDate) <= 0) {
        throw new InvalidRegistry(
          'Capital Programme A4.b',
          `${d.id} vintage retires on or before the day it went into service`,
        );
      }
    },
    // Law 9: a market names it by what it is, where it is and when it went into service — which is
    // what distinguishes one vintage from another and is the only thing a buyer needs to know.
    displayName: (i, namer) =>
      isPlantTerms(i.terms)
        ? `${d.name}, ${namer.region(i.terms.region)}, in service ${formatCivil(i.terms.serviceDate)}`
        : d.name,
    due: () => [],
    accrued: () => 0,
    cashFlows: () => [],
    // A3: the depreciation charge, as a write-down of the lot. One schedule; the kernel books the
    // same number against the stock and against income. `marked` is not read: what a vintage of
    // somebody's dead plant fetched on one day says nothing about what this one can still make.
    carriedAt: (i, lot, _marked, at, calendar) =>
      carriedAfterWear(plantTerms(i), lot.basisPerUnit, calendar.startOf(at), calendar),
  };
}
