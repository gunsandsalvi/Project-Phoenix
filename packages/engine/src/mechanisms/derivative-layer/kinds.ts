/**
 * What the layer itself issues: margin claims, default-fund contributions, and the claim a
 * close-out leaves on an estate.
 *
 * @spec Derivative D9 Derivative D9.a Derivative D10 Derivative Layer C3 Derivative Layer C3.a Derivative Layer C3.b Derivative Layer C4 Derivative Layer C4.b Derivative Layer C4.c Derivative Layer D3 Derivative Layer F2 Derivative Layer F3 XI-8 Bond N13 Bond N13.a Law 15
 *
 * All three are claims on a named party carried at what they cost: none of them has a market, and
 * what each is worth is what the issuer owes on it. They differ only in where they RANK, which is
 * the whole of the waterfall (C4) — so the ranking is stated on each kind and the estate reads it
 * off the instrument like every other claim (Law 15: nothing branches on which one it is).
 */
import type { Civil } from '../../calendar/civil.js';
import { currencyUnit, instrumentKindId, type PartyId } from '../../core/ids.js';
import { InvalidRegistry } from '../../core/errors.js';
import type { Namer } from '../../registry/naming.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';
import type { Instrument, Terms } from '../../register/instruments.js';

export const MARGIN_CLAIM = instrumentKindId('margin.claim');
export const FUND_CONTRIBUTION = instrumentKindId('defaultFund.contribution');
export const CLOSE_OUT_CLAIM = instrumentKindId('closeOut.claim');

/** D9.a, C3.a: cash posted as margin is the poster's ASSET at whoever holds it, returnable. */
export interface MarginTerms extends Terms {
  readonly kind: typeof MARGIN_CLAIM;
  /** Who posted it: the party whose asset this is. */
  readonly poster: PartyId;
  /** Who holds the cash and owes it back — a house, or a bilateral counterparty. */
  readonly holder: PartyId;
}

/** C3.b: what a member paid into the fund its house keeps against another member's default. */
export interface FundTerms extends Terms {
  readonly kind: typeof FUND_CONTRIBUTION;
  readonly member: PartyId;
  readonly house: PartyId;
}

/** F2, F3, C4.c: what a close-out left owing, on an estate, ranking with the unsecured. */
export interface CloseOutTerms extends Terms {
  readonly kind: typeof CLOSE_OUT_CLAIM;
  readonly owedBy: PartyId;
  readonly owedTo: PartyId;
  readonly struck: Civil;
}

/**
 * Law 15: the guards read the SHAPE of the terms and not the kind id on them. A module asking "is
 * this one of mine" by comparing kind ids is the branch the lint refuses, and the shape is the
 * honest question anyway: what makes these terms a margin claim is that they name a poster and a
 * holder.
 */
export const isMarginTerms = (t: Terms): t is MarginTerms => 'poster' in t && 'holder' in t;
export const isFundTerms = (t: Terms): t is FundTerms => 'member' in t && 'house' in t;
export const isCloseOutTerms = (t: Terms): t is CloseOutTerms =>
  'owedBy' in t && 'owedTo' in t && 'struck' in t;

/** The three share everything but their ranking: a claim on a name, at what it cost, no market. */
function claimKind(
  id: InstrumentKindProfile['id'],
  ranking: InstrumentKindProfile['ranking'],
  validateTerms: (t: Terms) => void,
  displayName: (i: Instrument, namer: Namer) => string,
): InstrumentKindProfile {
  return {
    id,
    pricing: 'carriedAtCost',
    carry: 'cost',
    liabilityOfIssuer: true,
    unit: (ccy) => currencyUnit(ccy),
    validateTerms,
    displayName,
    due: () => [],
    accrued: () => 0,
    cashFlows: () => [],
    ranking,
    accelerates: false,
  };
}

export const marginKind: InstrumentKindProfile = claimKind(
  MARGIN_CLAIM,
  // D3, C3.a: it is held, not consumed, and the poster gets it back — so it stands ahead of what
  // the holder merely owes. What makes it answer for the poster's OWN default is C4's first line,
  // which redeems it, and not a ranking that put it behind anything.
  () => ({
    seniority: 0,
    secured: [],
    claim: 'the return of cash posted as margin',
  }),
  (t) => {
    if (!isMarginTerms(t)) throw new InvalidRegistry('Derivative D9', 'not margin terms');
    if (t.poster === t.holder) {
      throw new InvalidRegistry('Derivative D9', 'margin is posted to somebody else');
    }
  },
  (i, namer) => {
    const who = namer.issuer.some ? namer.issuer.value : String(i.id);
    return isMarginTerms(i.terms) ? `${String(i.terms.poster)} margin at ${who}` : String(i.id);
  },
);

export const defaultFundKind: InstrumentKindProfile = claimKind(
  FUND_CONTRIBUTION,
  // C4, C4.a: the third and fourth lines of the waterfall are written down against it, so it ranks
  // BEHIND margin and behind everything else the house owes: that is what mutualisation is.
  () => ({
    // Behind the unsecured (seniority 1): a contribution is the fourth line of the waterfall and is
    // written down before the house's creditors are touched. That is what mutualisation IS (C4.a).
    seniority: 2,
    secured: [],
    claim: 'a share of the default fund, written down by the waterfall before it comes back',
  }),
  (t) => {
    if (!isFundTerms(t)) throw new InvalidRegistry('Derivative Layer C3.b', 'not fund terms');
    if (t.member === t.house) {
      throw new InvalidRegistry('Derivative Layer C3.b', 'a member and its house are two parties');
    }
  },
  (i, namer) => {
    const who = namer.issuer.some ? namer.issuer.value : String(i.id);
    return isFundTerms(i.terms) ? `${String(i.terms.member)} fund at ${who}` : String(i.id);
  },
);

export const closeOutKind: InstrumentKindProfile = claimKind(
  CLOSE_OUT_CLAIM,
  // C4.c: what a close-out left owing is an UNSECURED claim on the estate and is not paid ahead of
  // every ranked claim. The same seniority the money market's unsecured rows take (XI-8).
  () => ({
    seniority: 1,
    secured: [],
    claim: 'an unsecured claim for what a terminated contract was worth when it was closed out',
  }),
  (t) => {
    if (!isCloseOutTerms(t)) throw new InvalidRegistry('Derivative Layer F2', 'not close-out terms');
    if (t.owedBy === t.owedTo) {
      throw new InvalidRegistry('Derivative Layer F2', 'a claim has two sides, and they differ');
    }
  },
  (i, namer) => {
    const who = namer.issuer.some ? namer.issuer.value : String(i.id);
    return isCloseOutTerms(i.terms)
      ? `${who} close-out to ${String(i.terms.owedTo)}`
      : String(i.id);
  },
);
