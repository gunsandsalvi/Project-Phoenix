/**
 * Households: cells that earn, consume, save and own, each deciding for one possible household and
 * carrying how many of them it is.
 *
 * @spec Equity B1 Equity B3 Equity C2 Equity C2.a Households A1 Households A2 Households A2.a Households A2.b Households A2.c Households A2.d Households A2.e Households A2.f Households A3 Households B1 Households B2 Households B3 Households B3.a Households B5 Households C1 Households C1.a Households C1.b Households C1.c Households C1.d Households C2 Households C3 Households C4 Households C5 Households D1 Households D1.a Households D3 Households D5 Households D5.a Households D6 Goods C1 Goods C3 Labour B1 Labour B3 Labour D1.c Clearing B2 Observer A4 Expectations C1 Sovereign E2.f XI-15 XI-16 Law 2 Law 4 Law 6
 *
 * EVERY DECISION IS THE CELL'S, taken for one member and carried at the cell's weight (A2.e, A2.f).
 * The sector's consumption is the weighted sum of what its cells decided, and there is no number
 * anywhere in this module that a sector took: no representative household, no propensity applied to
 * an aggregate, no average anybody could have crossed a threshold at.
 *
 * WHAT IT DOES: it decides what to spend from its own outlook of its own income, its own cash and
 * its own recent surprises (C1); it takes that to market as a demand curve, not a point (C3, D6);
 * it keeps a cushion in its account and puts what is over it into paper when paper pays it enough
 * to give up access (C2, D5.a); and it sells its members' hours in the labour venue, which the
 * labour module runs. What it does not do is borrow — nobody lends to it yet (worklist 6) — so its
 * budget is its own cash, and what it cannot pay for it does not buy.
 *
 * WHAT REACHES IT: wages from named employers (B1), the standing mandate from the treasury (B2),
 * and coupons on the paper it holds (B3) — all of them money that actually arrived, because income
 * a household did not receive is not income (B3.a).
 */
import { noCash, sumCash } from '../../core/measure.js';
import { TERM_MONTHS } from '../../registry/credit.js';
import { levelsBelow, rungsUpTo } from '../../clearing/schedule.js';
import {
  COVER_TERM,
  PENSION_DIM,
  PENSION_MEMBER,
  PENSION_NONE,
  coverVenue,
} from '../../registry/insurance.js';
import { none, some, type Option } from '../../core/option.js';
import { about, type OutlookVariable } from '../../world/context.js';
import { goodId } from '../../registry/physical.js';
import {
  pricedAt,
  asCash,
  asRatio,
  type Cash,
  heldAsMoney,
  minus,
  over,
  plus,
  scale,
  valueAt,
  amountOf,
  asAmount,
  asPerPiece,
  type PerPiece,
} from '../../core/measure.js';
import type { Family, Violation } from '../../audit/audit.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order, OrderPrice } from '../../clearing/solver.js';
import type { VenueDecl } from '../../clearing/venue.js';
import { period, type Period } from '../../calendar/calendar.js';
import {
  instrumentId,
  paramId,
  partyId,
  type InstrumentId,
  type MarketId,
  type PartyId,
} from '../../core/ids.js';
import {
  addTo,
  atMost,
  combineDust,
  material,
  sum,
  withinDust,
  zeroIfNone,
  raised,
} from '../../core/num.js';
import { isAssetLeg, isMoneyLeg } from '../../ledger/instruction.js';
import { keyOf, weightOf, type CellParty } from '../../parties/party.js';
import { HOUSEHOLD } from '../../registry/profiles.js';
import type { LatticeDecl, LatticeReads } from '../../registry/lattice.js';
import { PEOPLE_PARAMS } from '../../registry/registry.js';
import type { ParamDecl } from '../../registry/params.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { householdChoosesBank, HOUSEHOLD_SWITCHING_COST, ownDepositRate } from './bank.js';
import { CONSUMPTION, MORTALITY, type ConsumptionDecl } from './data.js';
import { basketOf, demandOf, spendPerMember, type HouseholdParams } from './consume.js';
import {
  age,
  die,
  ESTATE_UNDELIVERED,
  mortalityParams,
  probateKind,
  settleEstates,
  form,
  fail,
} from './lifecycle.js';
import {
  cushionForFund,
  fundOrders,
  fundPositions,
  paperBids,
  savingLines,
  shareOrders,
  shortForSpending,
  sparePerMember,
} from './portfolio.js';

export * from './data.js';
export { householdChoosesBank, HOUSEHOLD_SWITCHING_COST } from './bank.js';
export { demandOf, spendPerMember } from './consume.js';
export {
  cushionForFund,
  fundOrders,
  fundPositions,
  ownUncertainty,
  paperBids,
  savingLines,
  shareOrders,
  shortForSpending,
  sparePerMember,
} from './portfolio.js';
import { asQty, downTick, scaleQty, type Qty } from '../../core/tick.js';
import { goingRateIn } from '../../registry/wages.js';
import { employedKey } from '../../register/employment.js';
import { rentOwedBy, shortfallOf, strikesPublished } from '../../registry/funding.js';
export type { DemandStep, HouseholdParams, Spending } from './consume.js';
export type { FundOrder, FundPosition, PaperBid, SavingLine, ShareOrder } from './portfolio.js';

/** 0f.3: the edges a household lattice bands on — RESOLUTION, each tested by invariance (0f.10). */
export const HOUSEHOLD_EDGES = {
  liquidWeeks: [
    paramId('households.lattice.liquidWeeks.1'),
    paramId('households.lattice.liquidWeeks.2'),
    paramId('households.lattice.liquidWeeks.3'),
  ],
  illiquid: [paramId('households.lattice.illiquid.1'), paramId('households.lattice.illiquid.2')],
  leverage: [paramId('households.lattice.leverage.1'), paramId('households.lattice.leverage.2')],
  spell: [paramId('households.lattice.spell.1'), paramId('households.lattice.spell.2')],
  tenure: [paramId('households.lattice.tenure.1')],
} as const;

/**
 * XI-15, A2.e, A2.f, 0f.3: THE LATTICE A POPULATION OF HOUSEHOLDS LIVES ON. Categorical: where it
 * is, where it banks, its cohort, whether its people work, and whether they have defaulted — each
 * owned by the one event that moves it. Banded: what a member holds liquid in weeks of what it
 * expects to earn (Deaton 1991, Carroll 1997: the buffer-stock is a threshold on this), what it
 * holds that it cannot spend this week, how levered it is, and how long its people have been out
 * of work (Mortensen–Pissarides 1994). The edges are RESOLUTION, not a claim about the answer.
 */
export const HOUSEHOLD_LATTICE: LatticeDecl = {
  kind: HOUSEHOLD,
  categorical: [
    {
      dim: 'region',
      movedBy: 'entry',
      why: 'a member is a real person with a real account, and an account is in a region',
    },
    {
      dim: 'bank',
      movedBy: 'bank.choice',
      why: 'a deposit is a claim on a NAMED issuer; two cells at two banks hold two instruments (Money A1)',
    },
    {
      dim: 'cohort',
      movedBy: 'households.lifecycle',
      why: 'people age, and a cell whose members were not all in one cohort could not be aged as one',
    },
    {
      dim: PENSION_DIM,
      movedBy: 'pension.enrolled',
      // 14.6, Insurers A2: whether a person is a member of the pension scheme is a fact about the
      // person — somebody whose wage has carried a contribution — and a lifetime carries it: a
      // working member ages into a retired member, and the promise is to the cell whose members
      // ARE members. The opening is a read of the record, as a hire's is: nobody the seed places
      // has paid in yet, so nobody is a member until a payroll enrols them (a scheme that owed the
      // seed's retired would be a seeded promise, 22a).
      opening: (reads: LatticeReads, cell: PartyId): string =>
        reads.lastEvent('pension.enrolled', cell).some ? PENSION_MEMBER : PENSION_NONE,
      why: 'Insurers A2, E1: a pension is promised to a named cell of members; a cell of members and one of non-members are owed different things',
    },
    {
      dim: 'estate',
      movedBy: 'households.lifecycle',
      opening: (): string => 'living',
      why: 'XI-8, 0f.4: the dying move to the standing probate cell of their key before what they held is handed to the office; a probate cell is a cell of the dead, not an age',
    },
    {
      dim: 'employment',
      movedBy: 'labour.hire',
      // 12b.2a: the value names the JOB (`employedKey`), so a cell is one employer's one row.
      opening: (reads: LatticeReads, cell: PartyId): string => {
        const hired = reads.lastEvent('labour.hire', cell);
        if (!hired.some) return 'unemployed';
        const d = hired.value.data;
        return employedKey(
          partyId(String(d['employer'])),
          String(d['occupation']),
          hired.value.period,
          d['moved'] === true ? 'anywhere' : 'trade',
        );
      },
      why: 'a wage is the one receipt a job pays; a cell with jobs and a cell without face different weeks (Labour A3.a)',
    },
    {
      dim: 'credit',
      movedBy: 'credit.default',
      opening: (reads: LatticeReads, cell: PartyId): string =>
        reads.lastEvent('credit.default', cell).some ? 'defaulted' : 'clean',
      why: 'a lender reads the record (Corporate Credit E5); a cell that has defaulted and one that has not are two borrowers',
    },
  ],
  banded: [
    {
      dim: 'liquidWeeks',
      quantity: (reads: LatticeReads, cell: PartyId): Option<number> => {
        const income = reads.expectedIncome(cell);
        if (!income.some || income.value <= 0) return none<number>();
        return some(reads.cashPerMember(cell, reads.homeCurrency(cell)) / income.value);
      },
      edges: HOUSEHOLD_EDGES.liquidWeeks,
      why: 'Deaton 1991, Carroll 1997: consumption is a threshold rule on liquid wealth in weeks of expected income',
    },
    {
      dim: 'illiquid',
      quantity: (reads: LatticeReads, cell: PartyId): Option<number> => {
        let worth = 0;
        let any = false;
        for (const h of reads.holdingsOf(cell)) {
          const w = reads.worthPerMember(cell, h.instrument);
          if (!w.some) continue;
          any = true;
          worth += w.value;
        }
        return any ? some(worth) : none<number>();
      },
      edges: HOUSEHOLD_EDGES.illiquid,
      why: 'Kaplan–Violante 2014: liquid and illiquid wealth give different marginal propensities, and a cell holding both is not one household',
    },
    {
      dim: 'tenure',
      quantity: (reads: LatticeReads, cell: PartyId): Option<number> => {
        const dwellings = reads
          .holdingsOf(cell)
          .filter((h) => String(h.instrument).startsWith('good.dwelling.'));
        if (dwellings.length === 0) return some(0);
        return some(dwellings.reduce((t, h) => t + reads.perMember(cell, h.instrument), 0));
      },
      edges: HOUSEHOLD_EDGES.tenure,
      why: 'Housing D3, Mian–Sufi 2011: an owner and a renter face a shock through different channels',
    },
    {
      dim: 'spell',
      quantity: (reads: LatticeReads, cell: PartyId): Option<number> => {
        const parted = reads.lastEvent('labour.separation', cell);
        const hired = reads.lastEvent('labour.hire', cell);
        if (!parted.some) return none<number>();
        if (hired.some && hired.value.period >= parted.value.period) return some(0);
        return some(reads.period - parted.value.period);
      },
      edges: HOUSEHOLD_EDGES.spell,
      why: 'Kroft–Lange–Notowidigdo 2013: the length of a spell changes what a person is offered and will take',
    },
  ],
};

export const HOUSEHOLD_PARAMS = {
  /** C1.c, XI-16 A3 (0f.7c): the weeks of buffer a cell wants — the MEAN it draws its own around. */
  patience: paramId('households.patience'),
  /** 0f.7c: how wide the draw is, as a share of the mean, the way `expectations.memory.dispersion` is. */
  patienceDispersion: paramId('households.patience.dispersion'),
  liquidityPremium: paramId('households.liquidityPremium'),
  horizon: paramId('households.horizon.periods'),
  toTheMarket: paramId('households.toTheMarket'),
  steps: paramId('households.demand.steps'),
} as const;

/** 0f.7c: the store a cell's own patience is drawn into, once, at its first decision. */
export const PATIENCE = 'households.patience';
interface OwnPatience {
  weeks: number | undefined;
}
const notYetDrawn = (): OwnPatience => ({ weeks: undefined });

/** The name of the store a cell's spend phase leaves its plan in, declared in the module's nouns. */
export const DECIDED = 'households.decided';

/** One order this cell decided to post, in the types it decided it in. */
interface PlannedOrder {
  readonly market: MarketId;
  readonly side: 'buy' | 'sell';
  /** XI-2: a cell selling because it needs the money names no price. Everything else is a level. */
  readonly price: OrderPrice;
  readonly qty: Qty;
}

/**
 * Law 8: WHAT THIS CELL DECIDED, AND IN WHICH PERIOD. The period is part of the fact: a plan from
 * last period is not a plan to post now, which is what `lastOwnSince(..., view.period)` was saying
 * while the journal stood in for this store.
 */
interface DecidedThisPeriod {
  at: Period | undefined;
  orders: readonly PlannedOrder[];
  /**
   * Law 18, 0g.5 (taken at 0g.1's measurement): what its members' needs cost, per member, as the
   * decision costed them — read by `willWork` for every venue it is asked about. Costing the
   * basket again per venue was a twelfth of a period at the first rung.
   */
  needsPerMember: Cash | undefined;
}

/** An empty slot: a cell that has not decided this period has no period and no orders. */
export const nothingDecided = (): DecidedThisPeriod => ({
  at: undefined,
  orders: [],
  needsPerMember: undefined,
});

/** Treasury C1: the rate a household pays on what it buys, which it must find on top of the price. */
export const CONSUMPTION_TAX = paramId('treasury.tax.consumption');

function paramsOf(): ParamDecl[] {
  return [
    {
      id: paramId('households.lattice.liquidWeeks.1'),
      value: 4,
      unit: 'weeks of expected income',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the household lattice on liquidWeeks. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('households.lattice.liquidWeeks.2'),
      value: 13,
      unit: 'weeks of expected income',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the household lattice on liquidWeeks. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('households.lattice.liquidWeeks.3'),
      value: 52,
      unit: 'weeks of expected income',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the household lattice on liquidWeeks. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('households.lattice.leverage.1'),
      value: 0.5,
      unit: 'ratio of debt to what it holds',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the household lattice on leverage. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('households.lattice.leverage.2'),
      value: 1,
      unit: 'ratio of debt to what it holds',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the household lattice on leverage. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('households.lattice.spell.1'),
      value: 4,
      unit: 'periods out of work',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the household lattice on spell. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('households.lattice.spell.2'),
      value: 26,
      unit: 'periods out of work',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: '0f.3, XI-15: an edge of the household lattice on spell. A RESOLUTION: refine every edge by two and the world\u2019s aggregates must move by less than derived dust (0f.10), or this is a shape.',
    },
    {
      id: paramId('households.lattice.tenure.1'),
      value: 1,
      unit: 'dwellings per member',
      dimension: 'ratio',
      kind: 'resolution',
      owner: 'model',
      why: 'Housing D3, 0f.3: the boundary between a member that owns a dwelling and one that does not; a count boundary, declared here so that it is one number in one place.',
    },
    {
      id: paramId('households.lattice.illiquid.1'),
      value: 1,
      unit: 'weeks of expected income, in money',
      dimension: 'amount' as const,
      denominated: 'money' as const,
      kind: 'resolution' as const,
      owner: 'model' as const,
      why: '0f.3: the first edge of the illiquid-wealth band, stated in money because the read is money; a resolution, tested by invariance (0f.10).',
    },
    {
      id: paramId('households.lattice.illiquid.2'),
      value: 10,
      unit: 'in money',
      dimension: 'amount' as const,
      denominated: 'money' as const,
      kind: 'resolution' as const,
      owner: 'model' as const,
      why: '0f.3: the second edge of the illiquid-wealth band; a resolution, tested by invariance (0f.10).',
    },
    {
      id: HOUSEHOLD_PARAMS.patience,
      value: 4,
      unit: 'weeks of its own income',
      dimension: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Households C1.c, §46 B3, XI-16 A3 (0f.7c): how many weeks of what it expects a household wants to be sitting on — the mean of the draw each cell makes once. It is widened by how wrong its own income has recently been, which is a read of its own surprises and not a second number. The gap-closing rate this id used to name is gone: below its target a cell buys its basket and keeps the rest, above it the rest is placed, and there is no speed a windfall reaches spending at other than the cell holding what it wants to hold.',
    },
    {
      id: HOUSEHOLD_PARAMS.patienceDispersion,
      value: 0.5,
      unit: 'of the mean, either way',
      dimension: 'ratio',
      kind: 'preference',
      owner: 'model',
      why: 'XI-16 A3, 0f.7c: how far apart two cells\u2019 patience can be drawn. The disagreement is load-bearing: cells that want different cushions place and spend differently out of the same income, which is what gives a saving line two sides in a sector that would otherwise trade once and stop.',
    },
    {
      id: HOUSEHOLD_PARAMS.liquidityPremium,
      value: 0.005,
      unit: 'per annum over what a deposit returns',
      dimension: 'perAnnum',
      kind: 'preference',
      owner: 'model',
      why: 'Households D5, D5.a: what a saver wants for giving up instant access to its money. It is the whole of the substitution between a deposit and paper held directly, and it is what makes a rate reach a saver at all.',
    },
    {
      id: HOUSEHOLD_PARAMS.horizon,
      value: 52,
      unit: 'periods',
      dimension: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Households D5: how long a household will tie its money up. Paper that comes back inside it is a substitute for its deposit; anything longer it would have to sell at a price nobody can tell it, which is D5 other two reasons — yield against risk — and it cannot weigh those until something a household can SEE prices risk. Worklist 9 closed and a rating and a spread exist; what is still missing is a household that reads either, which is item 12d (observation).',
    },
    {
      id: HOUSEHOLD_PARAMS.toTheMarket,
      value: 0.6,
      unit: 'of what it saves',
      dimension: 'ratio',
      kind: 'preference',
      owner: 'model',
      why: 'Households D5, Fund Shares A4, Indices C2 (13d): how much of what it saves goes into something that HOLDS THE MARKET rather than into lines it picked. A household with no view on any particular company still wants to be invested, and that is what an index fund is for — so this is what makes retail flow undifferentiated across names, which is what a tracker\u2019s simultaneity is actually made of. It is a share of a BUDGET and not of a price: what either half buys is a quantity meeting a price in a book, and neither half is sheltered from the other, which is the whole difference between this and the substitution assumption `ConsumptionDecl` used to make.',
    },
    {
      id: HOUSEHOLD_SWITCHING_COST,
      value: 40,
      denominated: 'money',
      unit: 'of the money the account is in, per move, per member',
      dimension: 'amount',
      kind: 'preference',
      owner: 'model',
      why: 'Banks Funding A1.d, E1: what it costs one household to move its account, ONCE, as an amount of its own money. It is weighed against what staying has already cost it — its own balance times the gap between the boards over as long as it has stayed — so a bigger balance moves for a smaller gap and the class drains instead of crossing at one instant. Retail money is the stickiest because the amount is large beside what a household holds, and that is A1.a arriving as a cost somebody bears rather than as a stated stickiness.',
    },
    {
      id: HOUSEHOLD_PARAMS.steps,
      value: 5,
      unit: 'count',
      dimension: 'count',
      kind: 'resolution',
      owner: 'model',
      why: 'Goods C1, Clearing A2: how finely a household posts its own demand curve into the book. Its shape is the cell own — what it spends divided by the price — and this is only how many levels of it the book sees; change it and the answer must not move.',
    },
  ];
}

/**
 * C1.c, XI-16 A3, 0f.7c: THIS CELL'S OWN PATIENCE, drawn once from the declared mean and width and
 * kept — the way its memory is (`expectations`).
 *
 * 0f.10: DRAWN UNDER THE POPULATION'S NAME, NOT THE CELL'S. A cell is what the lattice makes of a
 * population — the people of one cohort at one bank, cut by the bands they fall in — and a member
 * who crosses an edge moves to another cell of the same population. A draw keyed by the cell's id
 * would give that member a new patience for having crossed, and the world's answer would move
 * with the grain: the seeded dimensions of its key are what a population IS, so the draw is keyed
 * on them, and every cell of one population holds the one draw. What still disagrees is what
 * XI-16 A3 needs to disagree: two populations, two draws.
 */
function patienceOf(view: ParticipantView): number {
  const own = view.working(PATIENCE, notYetDrawn);
  if (own.weeks !== undefined) return own.weeks;
  const self = view.self;
  const population =
    self.representation === 'cell'
      ? `${keyOf(self, 'region')}|${keyOf(self, 'cohort')}|${keyOf(self, 'bank')}`
      : String(self.id);
  const mean = view.params.periods(HOUSEHOLD_PARAMS.patience);
  const spread = view.params.ratio(HOUSEHOLD_PARAMS.patienceDispersion);
  const draw = view.rng.derive(`patience/${population}`).next();
  // Uniform on [mean − spread·mean, mean + spread·mean): centred, and a mean of nothing draws nothing.
  own.weeks = mean + mean * spread * (2 * draw - 1);
  return own.weeks;
}

function numbers(view: ParticipantView): HouseholdParams {
  return {
    patience: patienceOf(view),
    steps: view.params.count(HOUSEHOLD_PARAMS.steps),
    consumptionTax: view.params.ratio(CONSUMPTION_TAX),
  };
}

/**
 * Insurers B3, A4.b, Households F1.b, §46 C3 (14.2): A CELL BIDS FOR LIFE COVER ON WHAT IT OWES, AT
 * WHAT IT EXPECTS OF ITS OWN MORTALITY. What a member's death leaves is its debts; a member covers
 * them, so the units a member asks for are what a member owes (a read of its own rows), and the
 * most a unit is worth to it is the chance it dies within the term — its own outlook of its
 * cohort's death share, formed from the deaths it has observed, compounded over the term. A ladder
 * down from there (Clearing C3), out of the cash a member holds. A cell that owes nothing, or has
 * seen nobody die, bids for nothing.
 */
function insureLife(
  ctx: MechanismContext,
  view: ParticipantView,
  self: CellParty,
  p: HouseholdParams,
  cashPerMember: Cash,
): void {
  const cohort = keyOf(self, 'cohort');
  const mortality = view.outlook(about({ on: 'mortality', cohort }));
  if (!mortality.some || mortality.value.expected <= 0) return;
  const ccy = view.registry.currencyOf(self.region);
  let owed = 0;
  for (const a of view.commitments()) {
    if (a.debtor === self.id && a.state === 'performing' && a.ccy === ccy) owed += a.owed;
  }
  if (owed <= 0) return;
  const perMember = ctx.registry.cashFor(
    asCash(owed / weightOf(self), ccy, 'what one member owes'),
  );
  if (perMember <= 0) return;
  const term = view.params.periods(COVER_TERM);
  const dies =
    1 - raised(1 - mortality.value.expected, term, 'the chance a member lives out the term');
  if (dies <= 0) return;
  const top = asPerPiece(dies, 'what a unit of cover on its debts is worth to a member');
  const rungs = rungsUpTo(levelsBelow(top, top, p.steps), cashPerMember, perMember);
  for (const r of rungs) {
    ctx.post(coverVenue(ccy), {
      party: self.id,
      side: 'buy',
      price: r.price,
      qty: scaleQty(r.qty, weightOf(self), 'over its members'),
    });
  }
}

/**
 * C5, D6, Firm F1 from the buying end: what a household consumed is what it paid a named seller
 * for. Units of a physical thing reaching a household with no money going the other way in the same
 * instruction is a gift nobody gave, and money going out with no units coming back is a payment for
 * nothing; either would make the sector's consumption a number rather than a sum of purchases.
 */
function consumptionIsBought(): Family {
  return {
    name: 'flows',
    contributor: 'households',
    spec: 'Households C5 Households D6 Goods F1',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const cells = new Set(view.parties.ofKind(HOUSEHOLD).map((p) => p.id));
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled') continue;
        const bought = new Map<PartyId, number>();
        const paid = new Map<PartyId, Qty>();
        for (const leg of r.instruction.legs) {
          if (isAssetLeg(leg) && cells.has(leg.to)) {
            const physical = view.registry.instrumentKind(
              view.instruments.get(leg.instrument).kind,
            ).physical;
            if (physical !== true) continue;
            /**
             * A-25, Goods F1, Audit A1.a: AN UNPRICED PHYSICAL LEG IS ITS OWN DEFECT, not a zero.
             *
             * This read `pricePerUnit.some ? value : 0` and carried the zero into the comparison, so
             * a cell that bought and paid was reported as `took 0 of goods and paid X` — a violation
             * whose size and message are about the MISSING PRICE and whose spec citation is about
             * the flow. The reader then has a number that is neither what moved nor what is wrong.
             *
             * A physical thing handed to a household at no stated price is a real defect in whoever
             * drafted the instruction (C2.a: a trade carries its print), and it is named as itself.
             */
            if (!leg.pricePerUnit.some) {
              out.push({
                family: 'flows',
                spec: 'Goods F1',
                owner: leg.to,
                size: leg.qty,
                unit: view.instruments.get(leg.instrument).unit,
                period: view.period,
                message: `${leg.to} was handed ${leg.qty} of ${leg.instrument} with no price on the leg`,
              });
              continue;
            }
            addTo(
              bought,
              leg.to,
              valueAt(
                leg.pricePerUnit.value,
                leg.qty,
                view.instruments.get(leg.instrument).ccy,
                'what it took',
              ).pieces,
            );
          } else if (isMoneyLeg(leg) && cells.has(leg.from.holder)) {
            addTo(paid, leg.from.holder, leg.amount);
          }
        }
        for (const [cell, value] of bought) {
          // A cell that took units and paid nothing paid nothing: absence of a payment is zero
          // money, which is the one place absence becomes a number (core/num.ts).
          const money = sum([zeroIfNone(paid.get(cell))]);
          const took = sum([value]);
          // Law 8: what it paid is what the goods came to ROUNDED TO REAL MONEY — a whole number of
          // pieces for each of its members (core/tick.ts). The comparison is therefore entitled to
          // the arithmetic's dust and to half a piece per member on top, and to nothing else: that
          // is the granularity of the money itself, derived here rather than allowed as a band.
          const who = view.parties.get(cell);
          // 0f.6: derived dust only. The half-piece-per-member band was the old representation's —
          // goods are bought and paid for in whole pieces of the total now (Law 7).
          if (withinDust(took.value, money.value, combineDust(took, money))) continue;
          out.push({
            family: 'flows',
            spec: 'Households C5',
            owner: cell,
            size: took.value - money.value,
            unit: view.registry.currencyOf(who.region),
            period: view.period,
            message: `${cell} took ${took.value} of goods and paid ${money.value} for them`,
          });
        }
      }
      return out;
    },
  };
}

/**
 * B5, Observer A5: what the sector was actually paid, published about the period that closed. It is
 * a SUM of what named payers paid named cells, read from the ledger — never an identity solved for
 * — and it causes nothing: no decision in this world can read it, because a cell's own outlook is
 * formed from what reached it and nothing else (§46 A2.b).
 */
function publishSectorIncome(ctx: MechanismContext): void {
  if (ctx.period === 0) return;
  const cells = new Set(ctx.parties.ofKind(HOUSEHOLD).map((p) => p.id));
  const terms: number[] = [];
  let payers = 0;
  const seen = new Set<PartyId>();
  const closed = period(ctx.period - 1);
  for (const r of ctx.ledger.inPeriod(closed)) {
    if (r.outcome !== 'settled') continue;
    for (const leg of r.instruction.legs) {
      if (!isMoneyLeg(leg) || !cells.has(leg.to.holder)) continue;
      terms.push(leg.amount);
      if (!seen.has(leg.from.holder)) {
        seen.add(leg.from.holder);
        payers += 1;
      }
    }
  }
  if (terms.length === 0) return;
  ctx.record('households.income', [], { of: closed, received: sum(terms).value, payers }, true);
}

/** The module. `rows` is what each cohort spends its money on (Law 15: the data says). */
/* --------------------------------------------------------------------------------------------
 * WHAT IT WILL WORK FOR
 * ------------------------------------------------------------------------------------------ */

/**
 * Labour B1, B3, D1.c, Observer A4, `A-43` (item 9.6): A HOUSEHOLD DECIDES WHAT IT WILL WORK FOR,
 * and this is where that decision lives.
 *
 * `MechanismContext.gather`'s own contract says a venue's schedules are *"built by the module that
 * owns that party, with that party's own view … building somebody else's schedule inside the
 * clearing phase instead is that module deciding for a party it does not own"* — and the labour
 * market was the one venue where the seller's schedule was built by the buyer's market. It walked
 * every household cell, read each one's outlook through `ctx.participant`, decided what that cell
 * would work for and posted the order itself. No private state leaked; what it cost is that a
 * household's reservation wage could not be changed without editing `labour`, which is why `A-38`'s
 * defect — the outside option being every kind of money received — lived there too.
 *
 * THE DIVISION IS NOT "EVERYTHING MOVES". What a household decides is what it will work for and how
 * many hours it has. Whether it is ALREADY employed, whether it has this trade, and which round it
 * is are facts about the labour market's own book and its own rules (B3: a person is in exactly one
 * state), and the venue applies those to what it gathers — a market deciding who is in its book is
 * the market's business, and deciding what a seller will accept is not.
 */
function willWork(view: ParticipantView, venue: VenueDecl): readonly Order[] {
  if (venue.clearedBy !== 'labour') return [];
  const self = view.self;
  if (self.representation !== 'cell' || !self.status.alive) return [];
  if (venue.key['region'] !== String(self.region)) return [];
  const people = weightOf(self);
  if (people <= 0) return [];
  // Law 8, XI-15: whole hours for every member the cell stands for, in the venue's own unit.
  const each = view.params.amount(PEOPLE_PARAMS.hoursPerMember, venue.unit);
  const hours = scaleQty(each, people, 'hours offered');
  if (hours <= 0) return [];
  /**
   * B1, B3, D1.c, 0f.7a: WHAT IT WILL WORK FOR IS WHAT FEEDS ITS PEOPLE, and the wage it is
   * against is in the basket, not in a `benefit` outlook. A cell asks what its members' NEEDS cost
   * over the hours it offers, and will not offer into a trade paying less than that — which is
   * what being out of the workforce IS, and it is reversible, because the going rate is
   * employment-weighted actual pay (D1.c, public every period) and employers bidding it up brings
   * the discouraged back. A trade NOBODY is employed in has no going rate and nothing to be
   * discouraged by, which is how a new trade gets its first worker at all. A cell that cannot cost
   * its basket has never seen a price and does not answer: missing is missing.
   *
   * 0f.10: NOT ITS SPELL BAND. The first cut of this read the cell's `spell` band — just separated
   * asks the going rate, a long spell asks the basket — and a decision that reads a BAND INDEX
   * makes the band's edge a preference: refine the edges and the world's answer moves, which is
   * the test that says an edge is a shape. The spell stays a dimension of the key, because it is a
   * fact about the population and the lattice stratifies on it; nothing decides on it.
   */
  const going = goingRateIn(view, venue.id);
  // Its own decision costed the basket this period (`decide` runs before the labour venue); a cell
  // that has not decided has no basket to work for.
  const decided = view.working(DECIDED, nothingDecided);
  if (decided.at !== view.period || decided.needsPerMember === undefined) return [];
  const mine = pricedAt(
    decided.needsPerMember,
    each,
    'what an hour must bring in to feed a member',
  );
  if (mine <= 0) return [];
  if (going.some && going.value < mine) return [];
  return [{ party: self.id, side: 'sell', price: mine, qty: hours }];
}

export function households(rows: readonly ConsumptionDecl[] = CONSUMPTION): SystemModule {
  return {
    id: 'households',
    agreementKinds: [
      {
        id: ESTATE_UNDELIVERED,
        what: 'what a dead cell\u2019s estate could not hand to probate',
        // XI-8: undelivered is owed. It is the residual that must have a holder, so it passes.
        binds: 'whoeverSucceeds',
      },
    ],
    spec: 'Households, Sovereign E2.f',
    nouns: [
      {
        name: DECIDED,
        kind: 'working',
        holds:
          'the orders each cell decided to post this period, and the period it decided them in',
        why: 'it is how this module gets from its spend phase to its own `markets` and `orders`, and nothing outside it has an opinion about an order nobody has posted yet (0e\u2032.4). It was a PRIVATE `households.plan` event read back by its own writer in the same period, with every order going out through `unknown[]` and back and any that did not survive the round trip dropped in silence. The event stays as the record of what the cell decided; this is the decision, and the door that says a size is a COUNT now sits at the WRITE.',
      },
      {
        name: PATIENCE,
        kind: 'working',
        holds: 'each cell\u2019s own weeks of buffer, drawn once at its first decision',
        why: 'XI-16 A3, 0f.7c: a preference is the cell\u2019s own and is drawn once, so it has to be kept somewhere between periods; it is this module\u2019s and nothing outside it has an opinion about how patient a household is.',
      },
      {
        name: 'households.waiting',
        kind: 'physics',
        holds:
          'the part of a person standing at each cell’s cohort boundary and at its mortality, carried between periods',
        why: 'XI-15 says a weight is a COUNT, so ageing and dying move whole people and the fraction below one has to WAIT rather than be deleted (A-18). Who is partway through the year in which they cross is this sector’s own demography: nothing else in this world has an opinion about it and the kernel wants no store for it — what is public is the weight, which is a read.',
      },
    ],
    // It buys goods, it acts on its own outlook, and it sells its members' hours in the venue the
    // labour module runs — so all three must be there before it decides anything.
    // Part XIII, Law 4: and `treasury`, because what a household is charged at the counter includes
    // the tax on it (`treasury.tax.consumption`) and that number is the state's, read by id. A
    // module that reads another's parameter depends on it whether or not it imports it — and a
    // world assembled without the one that declares it fails on a missing number rather than on a
    // missing dependency, which is the same defect wearing a worse message.
    requires: ['expectations', 'goods', 'labour', 'treasury'],
    instrumentKinds: [],
    // ARCHITECTURE 4.9b: A KIND IS OWNED BY THE MODULE THAT OWNS ITS BEHAVIOUR (worklist 11.6). A
    // household's representation, its failure modes and its deposit class are all this module's
    // subject, and while the kernel declared them the guard that asks a module how its depositors
    // leave (Banks Funding A1.d, E1) said nothing about the stickiest class of deposit there is.
    // The ID stays in the kernel's registry: the seed, `labour` and `treasury` all name a household.
    //
    // XI-3: a household cell dissolves into a NAMED HEIR CELL rather than into an estate (Households
    // F1, F2), and what happens when its members cannot pay is their lender's enforcement. Both are
    // the household life cycle and consumer credit, which is worklist 13d.
    // Households C1.d: nobody lends to a household in this world; consumer credit is 13d.
    // Banks Funding A1.a: many, small, sticky, insured to a limit — which is what a cell IS (XI-15).
    partyKinds: [
      // F2 (13d.1): where what the dead held waits until it can be divided. It is named, because a
      // cell cannot pay a cell — two weights share no whole number of pieces — and a named party
      // can take a thing to the piece and hand it on.
      probateKind,
      {
        /** item 15: the people it stands for, and no residual beyond them (XI-15). */
        objective: 'itsMembers',
        id: HOUSEHOLD,
        representation: 'cell',
        /**
         * XI-15, F1.a: WHERE THESE PEOPLE LIVE, WHEN THEY WERE BORN AND WHERE THEY BANK.
         *
         * The cohort is this sector's own: people age, and a cell whose members are not all in one
         * cohort could not be aged as one (`lifecycle`). The bank is here because a deposit is a
         * claim on a NAMED issuer — two cells at two banks hold two different instruments, and
         * merging them would net a claim on one bank against a claim on another (Money A1).
         */
        lattice: HOUSEHOLD_LATTICE,
        moneyIssuer: null,
        // XI-3 (12a.5): nothing is immortal — a household that cannot pay what fell due fails on
        // cash, and this module says where its people go (`resolves`).
        fails: ['cash'],
        // C1.d, Housing C1 (12a.5): it borrows — a mortgage on a schedule, consumer credit at its
        // option — through the one door every borrower uses; whether a bank lends is the bank's.
        borrows: true,
        buysOnTerms: false,
        // 17.8: Banks Lending A1.a, D4.a: a loan is not a security and is never distributed outside the banking system, so nobody of this kind is ever owed one.
        banking: false,
        depositClass: 'retail',
      },
    ],
    curveFamilies: [],
    units: [],
    params: [...paramsOf(), ...mortalityParams(MORTALITY)],
    phases: [
      {
        name: 'households.decide',
        spec: 'Households C1 Households C2 Households C3 Households D5 Households B5',
        anchor: { before: 'labour.match' },
        reads: [{ kind: 'event', name: 'fund.struck', of: 'anyPeriod' }],
        writes: [
          { kind: 'event', name: 'households.income' },
          { kind: 'event', name: 'households.plan' },
          // E2, 0f.7b: a cell short of a roof asks its bank through the one door every borrower uses.
          { kind: 'event', name: 'credit.request' },
        ],
        run: (ctx: MechanismContext) => {
          publishSectorIncome(ctx);
          for (const p of ctx.parties.ofKind(HOUSEHOLD)) {
            if (p.status.alive) decide(ctx, p.id, rows);
          }
        },
      },
      {
        name: 'households.lifecycle',
        spec: 'Households F1 Households F1.a Households F3 XI-15',
        // After everything else has happened to them: somebody who crossed into retirement this
        // period worked this period, and ageing them first would be backdating it.
        anchor: { after: 'revaluation' },
        // F1.b (12.2): who forms reads the rent its region last struck and its own plan.
        reads: [
          { kind: 'event', name: 'housing.rent', of: 'anyPeriod' },
          { kind: 'event', name: 'households.plan', of: 'anyPeriod' },
        ],
        writes: [{ kind: 'event', name: 'households.lifecycle' }],
        run: (ctx: MechanismContext) => {
          // XI-3 (12a.5): who could not pay goes to probate first, with the record that says so.
          fail(ctx);
          // F1.b: the people reaching adulthood form before the year's ageing moves the rest.
          form(ctx);
          age(ctx);
          die(ctx, MORTALITY);
          // F2: and what is in probate is divided, after the deaths that put it there — an estate
          // that arrived this period is divided this period if it will divide.
          settleEstates(ctx);
        },
      },
    ],
    // Clearing B2, Labour B1, `A-43` (item 9.6): WHAT THIS CELL WILL WORK FOR, decided by the
    // module that owns it and posted through the door `gather` is — the last venue in the engine
    // whose sellers' schedules were built by the buyers' market.
    venueParticipants: [{ partyKind: HOUSEHOLD, orders: willWork }],
    participants: [
      {
        partyKind: HOUSEHOLD,
        // XI-13, Equity B3: A SAVER IS IN A SHARE BOOK FOR A VIEW, and worklist 12c is what made
        // that true rather than a claim. It names a level from what the company itself published it
        // owns net of what it owes and what it earns on that, at what THIS cell requires of a claim
        // that promises nothing — its own money behind its own opinion, and its own loss when the
        // claim turns out to be worth less. Before that its only reason was a liquidity ladder over
        // sovereign paper, which is a rule, and the share books had nobody in them but the desks.
        speculative: true,
        // Law 18: the books this cell could be in, off the plan its orders come off (Law 19). A
        // cell buys three or four goods and holds a ladder in a handful of lines; the world it is
        // in has 261 markets, and asking it about every one of them was four fifths of what a
        // period cost.
        markets: (view: ParticipantView): readonly MarketId[] => {
          const decided = view.working(DECIDED, nothingDecided);
          return decided.at === view.period ? marketsIn(decided) : [];
        },
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          const decided = view.working(DECIDED, nothingDecided);
          return decided.at === view.period ? ordersFrom(decided, m.id, view.self.id) : [];
        },
      },
    ],
    families: [consumptionIsBought()],
    // XI-3 (12a.5): the estate module leaves a household's failure to this one (`resolvesItsOwn`).
    resolves: [HOUSEHOLD],
    bankChoices: [{ partyKind: HOUSEHOLD, chooses: householdChoosesBank }],
    /**
     * §46 A1, Households C3 (0h.1): WHAT A CELL IS ABOUT TO ACT ON — the price of every good in the
     * basket its cohort declares, in its own place. It is the decision it takes every period, and
     * the first time it takes it the good is one it has never bought: without this it had no view
     * of what a loaf costs until it had bought a loaf, and it could not buy one without a view.
     *
     * The list is a read of the same rows `decide` costs the basket from (Law 4) — nobody wrote a
     * watchlist — and it carries no levels: what each good PRINTED reaches the cell as one more
     * thing observed (A2.a), and what the cell makes of it is its own from the next period on.
     */
    watches: [
      {
        partyKind: HOUSEHOLD,
        watches: (view: ParticipantView): readonly OutlookVariable[] =>
          rows.map((row) =>
            about({ on: 'price', instrument: goodId(row.subUnit, view.self.region) }),
          ),
      },
    ],
  };
}

/**
 * C1, C2, C3, D5: the decision, taken once for one member of the cell and carried at its weight,
 * published under the cell's own name and read back by its own orders (Law 4).
 */
function decide(ctx: MechanismContext, cell: PartyId, rows: readonly ConsumptionDecl[]): void {
  const view = ctx.participant(cell);
  const self = view.self;
  if (self.representation !== 'cell') return;
  const p = numbers(view);
  /**
   * C3, B1, Law 18: THE BASKET IS COSTED ONCE, here, and what its needs come to is kept for the
   * labour venue to read (`willWork`) — whether or not the cell goes on to decide a spend. A cell
   * with no income outlook yet decides nothing and still offers its hours for what feeds its
   * people, exactly as it did when the venue costed the basket itself.
   */
  const basket = basketOf(view, rows, p);
  const slot = ctx.workingOf(cell, DECIDED, nothingDecided);
  slot.at = ctx.period;
  slot.orders = [];
  slot.needsPerMember = basket.needs;
  // D2: what it can pay with is its account AND what it can ask back from a fund on demand — that
  // is what makes a money fund a substitute for a deposit rather than an investment (D2).
  const positions = fundPositions(ctx.venues, strikesPublished(ctx.journal), view);
  const ccy = view.registry.currencyOf(self.region);
  const onDemand = sumCash(
    ccy,
    positions.map((f) => f.worthPerMember),
    'what it can get back on demand',
  ).value;
  /**
   * E3, E4, 0f.7c: WHAT FALLS DUE ON IT COMES BEFORE THE BASKET — the service on what it has issued
   * (`owedIn` is its position: what is due less what it holds, so what is due is that plus what it
   * holds) and the rent on its tenancy, both read off the kernel's own books, per member.
   */
  const due = over(
    plus(
      plus(
        heldAsMoney(view.owedIn(ccy), ccy, 'its position in its own money'),
        heldAsMoney(view.cash(ccy), ccy, 'what it holds of it'),
        'what falls due on what it issued',
      ),
      rentOwedBy(view.commitments(), cell, ccy),
      'and the rent on its tenancy',
    ),
    asRatio(weightOf(self), 'the members between whom it falls due'),
    'what falls due on one member',
  );
  const decided = spendPerMember(view, basket, p, onDemand, due);
  if (!decided.some) return;
  const goods = demandOf(view, rows, p, decided.value.spend);
  // Insurers B3, A4.b (14.2): and life cover on what it owes, at its own outlook of its mortality.
  insureLife(ctx, view, self, p, decided.value.cash);
  const spare = sparePerMember(
    // 0f.1: per member, like the spend and the buffer it is set against; 0f.7c: after what is due.
    minus(
      asCash(view.cashPerMember(ccy), ccy, 'what one member has in the account'),
      due,
      'after what falls due',
    ),
    decided.value.spend,
    decided.value.buffer,
  );
  /**
   * D5.a, A-44: what it requires of a claim is WHAT ITS DEPOSIT PAYS IT plus what giving up access
   * costs it, and it is what it compares a fund's offer against. It does NOT reach the paper bid —
   * that is its outlook less its own error (§46 B3), so a rise in the board does not yet make it
   * bid lower for a bill. `E-12`.
   *
   * The premium is declared `per annum OVER WHAT A DEPOSIT RETURNS` and was used as the bare 0.005,
   * so a cell subscribed to a fund offering 0.006 while its own bank's board paid 0.02. The
   * substitution D5.a describes — between a deposit and paper held directly, which is how a rate
   * reaches a saver at all — ran against a constant.
   */
  const required = plus(
    ownDepositRate(view),
    view.params.perAnnum(HOUSEHOLD_PARAMS.liquidityPremium),
    'what it wants before it gives up instant access',
  );
  // D5, §46 B1 (13d): everywhere its savings could go, in one pass, with what IT thinks each is
  // worth — its own outlook of that line where it has one and the last print where it has not,
  // which is the same ladder it buys a loaf on. What it will not do is work out a price from
  // somebody's accounts or somebody's cash flows: a household watches a price, and a world where
  // it did not was a world with one analytical technology handed to everybody.
  const { paper, shares } = savingLines(view, view.params.periods(HOUSEHOLD_PARAMS.horizon));
  // D5: one budget, spread over every place its money could go this period. Deciding it once and
  // dividing it is what stops the same money being committed twice (Law 4) and what stops a rule
  // nobody stated from preferring one class of thing to another.
  /**
   * D5, Fund Shares A4, Indices C2 (13d): AND IT WOULD RATHER OWN THE MARKET THAN PICK NAMES. How
   * much of what it saves goes to something that holds the market instead of to lines it chose is a
   * PREFERENCE — a household with no view on any particular company still wants to be invested —
   * and it is what makes retail flow undifferentiated across names, which is what a tracker's
   * simultaneity (C2) is actually made of. A world where every saver picked lines one at a time had
   * no such flow, and an index fund had nobody to be for.
   *
   * It is a share of a BUDGET and not of a price, which is what makes it a preference rather than
   * the substitution assumption `ConsumptionDecl` was: what it buys with either half is a quantity
   * meeting a price in a book, and neither half is protected from the other.
   */
  /**
   * D5, Housing E1, item 7b: A HOME IS A DURABLE AND IT COMES OUT OF THE SAME BUDGET.
   *
   * `dwelling` had a firm, a recipe, a market, a printed opening price and NO BIDDER EVER (`A-55`):
   * not in the basket, not an input to anything, not portable so no merchant carries it, not in a
   * fund's mandate and not in a bank's `makes`. Owner-occupation was a state the housing module's
   * own header described and no household could be in.
   *
   * It is not consumption — `demandOf`'s basket is a per-period FLOW and a house bought every period
   * for ever is not a house — and it is not a saving line, because a saving line is a claim that
   * promises something and a home is a thing its people live in. It is the third thing, and it
   * comes out of the SAME `spare`, so the money is committed once (Law 4).
   *
   * What it is short of is `housing`'s fact and `housing` publishes it (`housing.shortfall`), read
   * here under this cell's own name — the route every cross-module read uses, because a module never
   * imports a module.
   */
  const home = homeBid(ctx, view, spare, weightOf(self));
  const toSave = home.some
    ? minus(spare, home.value.committedPerMember, 'what is left after what it puts towards a home')
    : spare;
  const toTheMarket = view.params.ratio(HOUSEHOLD_PARAMS.toTheMarket);
  const tracking = [...paper, ...shares].filter((l) => l.tracks).length;
  const picked = paper.length + shares.length - tracking;
  const forTracking = scale(toSave, toTheMarket, 'what it puts into the market as a whole');
  const forPicked = minus(toSave, forTracking, 'what is left for lines it picked');
  const perTracked =
    tracking > 0
      ? over(forTracking, asRatio(tracking, 'the lines that track'), 'into one of them')
      : noCash(ccy);
  const perPicked =
    picked > 0
      ? over(forPicked, asRatio(picked, 'the lines it picked'), 'into one line it picked')
      : noCash(ccy);
  const budgetFor = (l: { readonly tracks: boolean }): Cash => (l.tracks ? perTracked : perPicked);
  const lines = paper.length + shares.length;
  const paperOrders = paperBids(view, paper, budgetFor, lines, weightOf(self));
  // D2, D5: the third thing it can do with its money, and the reason it asks for it back. What the
  // fund published is public (Clearing F1: it acts on what it has already been told), and what it
  // offers is compared against the same requirement a bill is.
  const short = shortForSpending(decided.value.cash, decided.value.spend);
  // C1.d, Corporate Credit A1 (12a.5): CONSUMER CREDIT IS THE SAME ROW, UNSECURED. What its basket
  // needs beyond what it holds it asks its bank for, at its option, through the one door every
  // borrower uses; a bank that says no leaves it short, which is the constrained state A2.g counts.
  if (short.pieces > 0) {
    ctx.request(self.id, {
      ccy,
      short: scale(
        short,
        asRatio(weightOf(self), 'its members'),
        'what the cell is short of between them',
      ),
      repays: 'atOption',
      // 17b.1: the money, not a promise of it — a household short this week is short now.
      wants: 'money',
      // Corporate Credit C9 (17b.8): a line, over the term a line is written for.
      months: ctx.params.months(TERM_MONTHS.working),
    });
  }
  const toFund = cushionForFund(decided.value.cash, decided.value.spend, spare);
  for (const o of fundOrders(positions, required, toFund, short, decided.value.wealth)) {
    if (!material(o.sharesPerMember, 2, o.sharesPerMember)) continue;
    /**
     * Fund Shares A1 (item 10e.6): A DOOR THAT ASKS GETS AN ANSWER, and the answer is public.
     *
     * A vehicle that is not offered to the public cannot see what a saver is worth — no party sees
     * another's register (Observer A4) — so a cell that wants in CERTIFIES: it publishes what it is
     * worth per member, and the fund checks that against the line as the line stands that day. A
     * cell with no interest in such a vehicle publishes nothing, which is why the disclosure is the
     * price of access and not a surveillance of every saver in the world.
     */
    const asks = positions.find((f) => f.venue === o.venue)?.asksOfEntrants;
    if (o.side === 'buy' && asks !== undefined) {
      // `funds` reads this under the cell's own name (`CERTIFIED`), the way every cross-module
      // read works here: one public kind, written at one end and read at the other.
      ctx.record(
        'investor.wealth',
        [cell],
        { wealthPerMember: decided.value.wealth.pieces, ccy: decided.value.wealth.ccy },
        true,
      );
    }
    ctx.post(o.venue, {
      party: cell,
      side: o.side,
      // C1, C2: nobody names a price here. Everybody who asks transacts at the NAV the fund strikes.
      price: 'market',
      // A posting is a total, like every other posting; what the cell decided was per member.
      // XI-15, Law 8: a cell posts a whole number of shares for EVERY member it stands for, so
      // the total is that count times a count of people and no rounding is involved.
      qty: scaleQty(o.sharesPerMember, weightOf(self), 'shares the cell asks about'),
    });
  }
  // Law 15, 0e′.4: the orders go in this cell's own working store, which is what its `markets` and
  // `orders` read back. The event below is the record of what it decided; it is written from the
  // same orders and never read back by this module.
  const plannedOrders: readonly PlannedOrder[] = [
    ...goods.map((g) => ({
      market: g.market,
      side: 'buy' as const,
      price: g.price,
      qty: asQty(g.qty, `${String(cell)}'s posted size in ${String(g.market)}`),
    })),
    ...(home.some ? [home.value.order] : []),
    ...paperOrders.map((b) => ({
      market: b.market,
      side: 'buy' as const,
      price: b.price,
      qty: asQty(b.qty, `${String(cell)}'s posted size in ${String(b.market)}`),
    })),
    // Law 8: a size is a COUNT of pieces, and the door that says so is here, at the WRITE — the
    // one place that knows what it decided. It used to be at the read, after a round trip through
    // the event's `unknown`, which is where a size that was not a count went missing quietly.
    ...shareOrders(view, shares, budgetFor, short, p.steps).map((o) => ({
      market: o.market,
      side: o.side,
      price: o.price,
      qty: asQty(o.qty, `${String(cell)}'s posted size in ${String(o.market)}`),
    })),
  ];
  slot.orders = plannedOrders;
  ctx.record(
    'households.plan',
    [cell],
    {
      // Per member, because that is what it decided (A2.f); the orders carry the cell's weight.
      spendPerMember: decided.value.spend.pieces,
      basketPerMember: decided.value.basket.pieces,
      duePerMember: decided.value.due.pieces,
      bufferPerMember: decided.value.buffer.pieces,
      cashPerMember: decided.value.cash.pieces,
      // C1.d, D2: what it could pay with, which is its account AND what a money fund owes it on
      // demand. A reader with only the account sees a cell spending more than it holds.
      ccy,
      budgetPerMember: decided.value.budget.pieces,
      wealthPerMember: decided.value.wealth.pieces,
      expectedIncome: decided.value.expected.pieces,
      // D5.a, Firm Birth A4 (12.1): what it requires of a claim, per annum — the number a founder
      // measures a line's margin against. Published, because the module that founds is not this one.
      requiredPerAnnum: required,
      // C1.d: its budget bound it. A2.g counts these, and a mean-preserving spread moves cells
      // across the threshold while the weighted mean of what they were paid does not move.
      constrained: decided.value.constrained,
      sparePerMember: spare.pieces,
      // D5: the places its money could go this period, and what goes into one of them.
      linesItMayHold: lines,
      // D5, Fund Shares A4 (13d): and how it split them — what goes into one line it picked, and
      // what goes into one that holds the market instead.
      perPickedLinePerMember: perPicked.pieces,
      perTrackingLinePerMember: perTracked.pieces,
      shortForSpendingPerMember: short.pieces,
      toFundPerMember: toFund.pieces,
      // C2: what it does not spend and does not put into paper is saved where it already is.
      // D5, item 7b: what it put towards a home, and nothing when it owns what its people live in.
      toAHomePerMember: home.some ? home.value.committedPerMember.pieces : 0,
      orders: plannedOrders,
    },
    false,
  );
}

/**
 * D5, E2, Housing E1, item 7b, 0f.7b: WHAT THIS CELL BIDS FOR A HOME, AND WHAT IT ASKS ITS BANK FOR.
 *
 * `housing` publishes what each cell needs against what it owns and what it rents; this reads that
 * event under the cell's own name. A cell that is short of nothing bids for nothing, and the whole of
 * its spare goes to the saving lines.
 *
 * The level is its OWN — its outlook of the line where it has one, the last print where it has not,
 * which is the same ladder it buys a loaf on and never a valuation of a house from anybody's
 * accounts (Law 3). It bids for what it is SHORT OF, and no more: a cell does not buy a second
 * home because it could afford one. What it pays with is the whole of its spare — a roof comes
 * before paper because a roof is a need (Housing E1) and paper is not, which is an ORDER and not a
 * share of anything (the half-of-spare rule that stood here was a shape). What its spare does not
 * reach it asks its bank for through the one door every borrower uses (Corporate Credit A1), secured
 * on the dwellings the loan would buy (A4); whether anybody lends is the lender's standard (C5.a),
 * and a cell nobody will lend to goes on renting.
 */
function homeBid(
  ctx: MechanismContext,
  view: ParticipantView,
  spare: Cash,
  weight: number,
): Option<{ readonly committedPerMember: Cash; readonly order: PlannedHomeOrder }> {
  const nothing = none<{ committedPerMember: Cash; order: PlannedHomeOrder }>();
  if (weight <= 0) return nothing;
  const said = shortfallOf(view, String(view.self.id), view.period);
  if (!said.some) return nothing;
  const short = asAmount<'piece'>(said.value.short, 'what it is short of');
  const instrument = instrumentId(said.value.dwelling);
  if (!view.instruments.has(instrument)) return nothing;
  const own = view.outlook(about({ on: 'price', instrument }));
  const print = view.print(instrument);
  const level = own.some
    ? asPerPiece(own.value.expected, 'what it thinks a home here is worth')
    : print.some
      ? print.value.price
      : undefined;
  if (level === undefined || level <= 0) return nothing;
  const market = goodMarketOf(view, instrument);
  if (market === undefined) return nothing;
  const ccy = view.registry.currencyOf(view.self.region);
  // What the roofs it is short of would cost the cell, and what its people have spare between them.
  const cost = valueAt(level, short, ccy, 'what the roofs it is short of would cost');
  const have = scale(spare, asRatio(weight, 'its members'), 'what the cell has spare between them');
  if (cost.pieces > have.pieces) {
    // Housing C1, XI-15 (12a.4): A MORTGAGE IS A ROW PER (LENDER, CELL) IN TOTALS, and a cell asks
    // for what its people's spare does not reach every period it is short of roofs. It asked only
    // while it owed nothing at all — "one mortgage at a time", read at a cell of twenty thousand
    // households as one roof for all of them, ever — and every cell was short of a roof every
    // period for the life of the run (`housing.shortfall`). Whether the bank lends is the bank's.
    const gap = minus(cost, have, 'what its spare does not reach');
    ctx.request(view.self.id, {
      ccy,
      short: gap,
      security: [
        { instrument, qty: asQty(downTick(amountOf(gap, level, 'what the loan would buy'))) },
      ],
      // Housing C2: a mortgage is paid down, interest and principal.
      repays: 'onSchedule',
      // 17b.1: the money, not a promise of it — a buyer of a roof pays for the roof.
      wants: 'money',
      // C2 (17b.8): over the decades a roof is bought over, and it is one number for both the
      // family that lives in it and the landlord that lets it (Law 4).
      months: view.params.months(TERM_MONTHS.mortgage),
    });
  }
  if (spare.pieces <= 0) return nothing;
  // Law 8, XI-15: whole pieces, per member, and DOWN — what its money actually reaches.
  const perMember = downTick(amountOf(spare, level, 'what one member\u2019s spare reaches'));
  if (perMember <= 0) return nothing;
  const wanted = scaleQty(perMember, weight, 'what the cell bids for');
  const qty = atMost(wanted, short, 'and no more than it needs');
  if (qty <= 0) return nothing;
  return some({
    // What this actually commits is what the units it bids for come to, not the whole spare: the
    // rest is still spare and goes to the saving lines with everything else (Law 4).
    committedPerMember: valueAt(
      level,
      asAmount<'piece'>(perMember, 'what one member bids for'),
      ccy,
      'towards a home',
    ),
    order: { market, side: 'buy' as const, price: level, qty: asQty(qty) },
  });
}

interface PlannedHomeOrder {
  readonly market: MarketId;
  readonly side: 'buy';
  readonly price: PerPiece;
  readonly qty: Qty;
}

/** Clearing D1: the market this line is traded in, off the instrument itself (Law 19). */
function goodMarketOf(view: ParticipantView, instrument: InstrumentId): MarketId | undefined {
  const i = view.instruments.get(instrument);
  return i.market.some ? i.market.value : undefined;
}

/**
 * Law 15, Law 4: WHAT THIS CELL DECIDED, kept where a decision belongs.
 *
 * These two took the cell's own `households.plan` EVENT's `orders` back out as `unknown[]` and
 * re-checked every field, dropping any order that did not survive the round trip. The event is
 * recorded PRIVATE and was read back by its own writer in the same period, which is a store
 * wearing a log's clothes (0e′.4). `ParticipantView.working` is the store, so the orders never
 * leave the type system and both parsers are gone.
 */
function marketsIn(decided: DecidedThisPeriod): MarketId[] {
  const out = new Set<MarketId>();
  for (const o of decided.orders) out.add(o.market);
  return [...out];
}

function ordersFrom(decided: DecidedThisPeriod, market: MarketId, self: PartyId): Order[] {
  const out: Order[] = [];
  for (const o of decided.orders) {
    if (o.market !== market) continue;
    out.push({ party: self, side: o.side, price: o.price, qty: o.qty });
  }
  return out;
}
