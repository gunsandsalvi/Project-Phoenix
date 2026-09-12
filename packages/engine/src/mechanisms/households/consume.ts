/**
 * What a household decides to spend, and the demand it takes to market with it.
 *
 * @spec Households A2.f Households C1 Households C1.a Households C1.b Households C1.c Households C1.d Households C2 Households C3 Households C4 Households D6 Goods C1 Expectations B3 Expectations C1 XI-15 XI-16 Law 2 Law 6
 *
 * EVERY NUMBER HERE IS PER MEMBER of the cell that decided it (A2.f, XI-15). A cell is one possible
 * household carried with a multiplicity, so what it decides is what one household decides, and the
 * sector's number is the weighted sum of those — never a decision taken at the sector's average.
 *
 * IT SPENDS WHAT IT EXPECTS TO HAVE, corrected towards the cushion it wants to be sitting on. Its
 * own outlook of its own income says the first (C1.a, §46 C1); the second is a stock — so many
 * periods of its own income, widened by how wrong its income has recently been (C1.c: confidence is
 * a read of its own surprises, never a stated mood) — and the gap between what it OWNS and that
 * cushion is closed at its own patience. What it owns is its cash and what the market last said its
 * holdings are worth (C1.b, D1, D3: a read, never a stored number), which is why an asset price
 * moving reaches demand at all. A cell that has been surprised widely holds more and spends less,
 * which is B3 made concrete rather than asserted.
 *
 * IT CANNOT BORROW (C1.d): nobody lends to a household in this world yet, so what it cannot pay for
 * it does not buy. That is not a bound on the decision — it is what a budget is — and when credit
 * exists (worklist 6) the constraint becomes a decision somebody else takes.
 *
 * WHAT IT WILL PAY is a schedule, not a point (Goods C1, Clearing A2). It has decided what to spend
 * on a good; how much of the good that is depends on the price, and the curve through those pairs
 * IS its demand. It posts that curve over the range of prices it thinks are possible — its own
 * expectation, widened by its own surprises about that price — so a cell that has seen prices move
 * bids across a wider range than one that has not.
 */
import type { InstrumentId, MarketId } from '../../core/ids.js';
import { add, atLeast, atMost, div, material, mul, sub, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { CellParty } from '../../parties/party.js';
import type { ParticipantView } from '../../world/context.js';
import { goodId, goodMarketId } from '../../registry/physical.js';
import type { ConsumptionDecl } from './data.js';
import { rungsOver } from './demand.js';

/** One line of a cell's demand: a size at a level, in the market it is posted in. */
export interface DemandStep {
  readonly market: MarketId;
  readonly instrument: InstrumentId;
  /** The most it will pay per unit at this step. */
  readonly price: number;
  /** Total units, which is the per-member size times the cell's weight (XI-15). */
  readonly qty: number;
}

/** The numbers a household decides with, all declared by the module (Law 2). */
export interface HouseholdParams {
  readonly patience: number;
  readonly bufferPeriods: number;
  readonly steps: number;
  readonly consumptionTax: number;
}

/**
 * C1: what one member of this cell decides to spend this period, in its own money. It is what it
 * expects to earn, plus the gap between what it owns and the cushion it wants, closed at its own
 * patience — and never more than it can PAY WITH, because nobody lends to it (C1.d). What it can
 * pay with is its account and what a money fund owes it on demand (D2), which is `budget`; what it
 * holds in the account alone is `cash` and has not been the whole of it since money funds existed.
 */
export interface Spending {
  /** What it will actually spend, per member. */
  readonly spend: number;
  /** What it wanted to spend before its own budget had a say (C1.d). */
  readonly wanted: number;
  /** The cushion it wants to be sitting on, per member. */
  readonly buffer: number;
  /** Its own outlook of what it will earn (C1.a). */
  readonly expected: number;
  /** What it holds in its account, per member. */
  readonly cash: number;
  /**
   * C1.d, D2: THE WHOLE OF WHAT IT CAN PAY WITH, per member — its account and what it can ask back
   * from a money fund on demand, because nobody lends to it and those are the two places its money
   * is. It is the number that bound the decision, so it is the number `constrained` is about, and
   * it is published: a reader that had only `cash` would see a cell spending more than it holds.
   */
  readonly budget: number;
  /** C1.b, D3: what it owns, per member — its cash and its holdings at what the market last said. */
  readonly wealth: number;
  /** C1.d: whether its budget bound it, which is a threshold a mean-preserving spread moves cells across (A2.g). */
  readonly constrained: boolean;
}

/**
 * `onDemand`: what it can ask back from a fund at any time, per member (Fund Shares D2). A money
 * fund is a SUBSTITUTE for a deposit, so what is in one is money this cell can pay with — it asks
 * for it in the same period it means to spend it. Anything it cannot get back on demand is wealth
 * (C1.b) but not budget (C1.d), which is the whole difference between a fund share and a bond.
 */
export function spendPerMember(
  view: ParticipantView,
  p: HouseholdParams,
  onDemand: number,
): Option<Spending> {
  const income = view.outlook('income');
  if (!income.some) return none();
  const ccy = view.registry.region(view.self.region).ccy;
  const cash = view.cash(ccy);
  const budget = add(cash, onDemand, 'what it can pay with');
  const wealth = add(wealthOf(view, cash), onDemand, 'what it owns');
  // C1.c, §46 B3: the cushion is so many periods of what it expects, widened by how wrong that
  // expectation has recently been. Confidence is in the same unit as the variable, so a cell whose
  // income has been unpredictable by a given amount wants that much more in hand per period.
  const buffer = mul(
    p.bufferPeriods,
    add(income.value.expected, income.value.confidence, 'what a period could cost it'),
    'the cushion it wants',
  );
  const gap = div(sub(wealth, buffer, 'what it owns over its cushion'), p.patience, 'closed at its own patience');
  const wanted = add(income.value.expected, gap, 'what it decides to spend');
  // C1.d: it spends what it has, whatever it wants. And nobody buys a negative loaf.
  //
  // Law 7: nor an unrepresentable one. A cell whose budget is the rounding of a subtraction would
  // take a demand curve to market in quantities so small that the per-member share of a fill
  // cannot be multiplied back by the weight to give the total again — below the smallest normal
  // number there is no relative precision left, so dust itself underflows to zero and every
  // identity in the wire becomes exact. A spend that is dust of what it has is nothing.
  const affordable = atMost(wanted, budget, 'it buys with the money it has');
  const scale = sum([budget, Math.abs(wanted)]);
  const afforded = material(affordable, scale.terms + 1, scale.value)
    ? affordable
    : 0;
  return some({
    spend: atLeast(afforded, 0, 'there is no less to spend than nothing'),
    wanted,
    buffer,
    expected: income.value.expected,
    cash,
    budget,
    wealth,
    constrained: wanted > budget,
  });
}

/**
 * C1.b, D1, D3: what a household owns, per member: its money, and what the market last said its
 * holdings are worth. It is a read of the register and the price store at the moment it is asked,
 * and there is no stored net worth anywhere. What it holds that nothing has ever printed a price
 * for is not in it: a thing with no price is not wealth a household could count on.
 */
function wealthOf(view: ParticipantView, cash: number): number {
  const terms = [cash];
  for (const h of view.holdings()) {
    const print = view.print(h.instrument);
    if (!print.some) continue;
    const units = sum(h.lots.map((l) => l.qty));
    terms.push(mul(units.value, print.value.price, 'what it holds is worth'));
  }
  return sum(terms).value;
}

/**
 * C3, C4: what it takes to market. The share of its spending that goes on a good is its cohort's
 * preference; what it will actually pay for the units is that share less the tax it will owe on
 * them (C4: it buys at a price it pays, and the tax is part of what a purchase costs it).
 */
export function demandOf(
  view: ParticipantView,
  rows: readonly ConsumptionDecl[],
  p: HouseholdParams,
  spend: number,
): DemandStep[] {
  const self = view.self;
  if (self.representation !== 'cell') return [];
  const out: DemandStep[] = [];
  for (const row of rows) {
    if (row.cohort !== self.key.cohort) continue;
    const budget = div(
      mul(spend, row.share, 'what it spends on this good'),
      add(1, p.consumptionTax, 'with the tax it will owe on it'),
      'what reaches the seller',
    );
    if (!material(budget, 2, spend)) continue;
    out.push(...schedule(view, self, row.subUnit, budget, p.steps));
  }
  return out;
}

/**
 * Goods C1, Clearing A2: the demand curve, posted as the step function a book takes (demand.ts).
 *
 * The range is the cell's OWN uncertainty about that price (§46 B3): a cell that has never been
 * surprised posts one point at what it expects, and one that has seen the price move posts across
 * the width of its own surprises. Nothing states a range.
 */
function schedule(
  view: ParticipantView,
  self: CellParty,
  subUnit: string,
  budget: number,
  steps: number,
): DemandStep[] {
  const instrument = goodId(subUnit, self.region);
  const outlook = view.outlook(`price.${instrument}`);
  const print = view.print(instrument);
  const expected = outlook.some
    ? outlook.value.expected
    : print.some
      ? print.value.price
      : undefined;
  if (expected === undefined || expected <= 0) return [];
  const width = outlook.some ? outlook.value.confidence : 0;
  return rungsOver(pricesOver(expected, width, steps), budget).map((r) => ({
    market: goodMarketId(subUnit, self.region),
    instrument,
    price: r.price,
    qty: mul(r.qty, self.weight, 'what the cell asks for'),
  }));
}

/** The levels a cell posts over, highest first: what it expects, spread by its own surprises. */
function pricesOver(expected: number, width: number, steps: number): number[] {
  if (width <= 0 || steps <= 1) return [expected];
  const out: number[] = [];
  for (let i = 0; i < steps; i += 1) {
    const t = div(sub(mul(2, i, 'step'), sub(steps, 1, 'steps less one'), 'centred'), sub(steps, 1, 'steps less one'), 'position');
    const price = add(expected, mul(width, -t, 'how far from what it expects'), 'a level it would pay');
    if (price > 0) out.push(price);
  }
  return out;
}
