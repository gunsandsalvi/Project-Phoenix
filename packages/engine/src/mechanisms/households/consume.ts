/**
 * What a household decides to spend, and the demand it takes to market with it.
 *
 * @spec Households A2.a Households A2.b Households A2.f Households C1 Households C1.a Households C1.b Households C1.c Households C1.d Households C2 Households C3 Households C4 Households D6 Goods A2.a Goods C1 Expectations B3 Expectations C1 Fund Shares C2.b XI-2 XI-15 XI-16 Law 2 Law 6
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
 * WHAT IT BUYS IS A BASKET (13c.2). Its cohort declares two physical quantities per good — what a
 * member has before anything else, and what it takes on top when the money reaches — so what it
 * takes to market is those quantities, not a division of its money. It fills the needs first and
 * spreads what is left over the wants, at the prices IT expects; what share of its income ends up
 * going on food is an outcome of that meeting the prices it actually meets.
 *
 * WHAT IT WILL PAY is a schedule, not a point (Goods C1, Clearing A2). How much of a good it can
 * have depends on the price, and the curve through those pairs IS its demand: the money it set
 * aside divided by the price while the money binds, flat at what it wanted once it does not. It
 * posts that curve over the range of prices it thinks are possible — its own expectation, widened
 * by its own surprises about that price — so a cell that has seen prices move bids across a wider
 * range than one that has not.
 */
import type { InstrumentId, MarketId } from '../../core/ids.js';
import { add, atLeast, atMost, div, material, mul, sub, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { keyOf, type CellParty } from '../../parties/party.js';
import type { ParticipantView } from '../../world/context.js';
import { goodId, goodMarketId } from '../../registry/physical.js';
import type { ConsumptionDecl } from './data.js';
import { rungsUpTo } from './demand.js';

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
  /**
   * The MOST it will spend, per member. What it actually spends is what its basket cost it, and
   * since 13c.2 that can be less: a cell whose needs and wants are covered for less than this keeps
   * the difference, which is a saving nobody decided on separately (`demandOf`).
   */
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
  const ccy = view.registry.currencyOf(view.self.region);
  const cash = view.cash(ccy);
  const budget = add(cash, onDemand, 'what it can pay with');
  const wealth = add(wealthOf(view, cash), onDemand, 'what it owns');
  // C1.c, §46 B3: the cushion is so many periods of what it expects, widened by how wrong that
  // expectation has recently been. Confidence is in the same unit as the variable, so a cell whose
  // income has been unpredictable by a given amount wants that much more in hand per period.
  const buffer = add(
    mul(
      p.bufferPeriods,
      add(income.value.expected, income.value.confidence, 'what a period could cost it'),
      'the cushion it wants against its income',
    ),
    // XI-2, §46 B3, Fund Shares C2.b (13d): AND AGAINST ITS SAVINGS. What it holds can move, and how
    // far it thinks it can move is its own uncertainty about those very prices — the same outlooks
    // it buys and sells with, read here for what they say about risk rather than about level. So a
    // cell that has been surprised BY A PRICE wants more cash in hand and bids lower in the same
    // read, which is how a market shock reaches consumption and how it reaches a money fund: the
    // redemption is a household wanting its cushion, not a coefficient anybody added.
    atRisk(view),
    'the cushion it wants altogether',
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
 * §46 B3, XI-2: WHAT ITS OWN SAVINGS COULD MOVE BY, per member, by its own reckoning. Its holdings
 * at what it holds of them, times how wrong it has been about each of those prices. A cell that has
 * never been surprised by a price wants nothing extra; one that has just watched a market move
 * wants that much more in hand, and it wants it the same period it bids lower for the same reason.
 *
 * Law 19: every term is a read — the register's units and this cell's own outlooks — and there is
 * no risk aversion parameter anywhere. What it will not risk is what it has seen happen.
 */
function atRisk(view: ParticipantView): number {
  const terms: number[] = [];
  for (const h of view.holdings()) {
    const outlook = view.outlook(`price.${String(h.instrument)}`);
    if (!outlook.some || outlook.value.confidence <= 0) continue;
    const units = sum(h.lots.map((l) => l.qty));
    if (units.value <= 0) continue;
    terms.push(mul(units.value, outlook.value.confidence, 'what this line could move by'));
  }
  return sum(terms).value;
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
 * C3, C4, 13c.2: WHAT IT TAKES TO MARKET, which is a basket and not a division of its money.
 *
 * Its cohort's preference is two quantities per good (`data.ts`): what it will have before anything
 * else, and what it takes on top when the money reaches. So it fills its NEEDS first out of what it
 * decided to spend, and spreads what is left over its WANTS in the proportion it wanted them — and
 * if what is left will not cover them it takes the same fraction of each, because nothing here
 * knows which of two wants it would give up first and inventing an order would be inventing a
 * preference nobody declared.
 *
 * WHAT COSTS WHAT is the cell's own expectation of each price (§46 B3), never a print it has not
 * seen and never a level stated anywhere. So the split is made at the prices it EXPECTS, and the
 * prices it MEETS are the market's business: a cell whose expectations were wrong buys less of a
 * thing that turned out dear and keeps the change on one that turned out cheap. That is the whole
 * of the income effect, and there is no elasticity anywhere in it.
 *
 * The share of its income that goes on food is therefore an OUTCOME of two declared quantities
 * meeting two prices, and it falls as income rises with nothing stating that it does: a cell whose
 * needs are covered puts everything further into wants, and a cell whose needs are not covered puts
 * everything into needs. Engel's law as arithmetic (A2.a).
 *
 * C4: the tax is part of what a purchase costs it, so it is in what it works the basket out
 * against, and out of the price it posts — what it bids is what reaches the seller.
 */
export function demandOf(
  view: ParticipantView,
  rows: readonly ConsumptionDecl[],
  p: HouseholdParams,
  spend: number,
): DemandStep[] {
  const self = view.self;
  if (self.representation !== 'cell') return [];
  const lines: Line[] = [];
  for (const row of rows) {
    if (row.cohort !== keyOf(self, 'cohort')) continue;
    const line = lineFor(view, self, row, p);
    if (line !== undefined) lines.push(line);
  }
  const needCost = sum(lines.map((l) => mul(l.row.neededPerMember, l.perUnit, 'what it must have costs'))).value;
  const wantCost = sum(lines.map((l) => mul(l.row.wantedPerMember, l.perUnit, 'what it wants on top costs'))).value;
  // C1.d: it eats before it does anything else, and it eats out of money it has.
  const toNeeds = atMost(spend, needCost, 'it needs what it needs and pays with what it has');
  const left = sub(spend, toNeeds, 'what is left when it has had what it must have');
  let needScale = 0;
  if (needCost > 0) needScale = div(toNeeds, needCost, 'the fraction of what it must have it can pay for');
  let wantScale = 0;
  if (wantCost > 0) {
    wantScale = atMost(
      div(left, wantCost, 'the fraction of what it wants on top it can pay for'),
      1,
      'it takes what it wanted and not more because it could afford more',
    );
  }
  const out: DemandStep[] = [];
  for (const l of lines) {
    const qty = add(
      mul(l.row.neededPerMember, needScale, 'of what it must have'),
      mul(l.row.wantedPerMember, wantScale, 'of what it wants on top'),
      'what one member takes this period',
    );
    if (qty <= 0) continue;
    // C4: what it set aside is what the units cost it INCLUDING the tax; what reaches the seller is
    // that less the tax, and that is the money the curve is drawn against.
    const set = mul(qty, l.perUnit, 'what it set aside for this line');
    const net = div(set, add(1, p.consumptionTax, 'with the tax it will owe on it'), 'what reaches the seller');
    if (!material(net, 2, spend)) continue;
    for (const r of rungsUpTo(pricesOver(l.expected, l.width, p.steps), net, qty)) {
      out.push({
        market: l.market,
        instrument: l.instrument,
        price: r.price,
        qty: mul(r.qty, self.weight, 'what the cell asks for'),
      });
    }
  }
  return out;
}

/** One line of a cell's basket: the good, where it would buy it, and what it expects to be charged. */
interface Line {
  readonly row: ConsumptionDecl;
  readonly instrument: InstrumentId;
  readonly market: MarketId;
  /** C4: what ONE unit costs it, tax and all, at the price it expects. What the basket is priced at. */
  readonly perUnit: number;
  /** §46 B3: the price it expects, and how wrong it has been about this one. */
  readonly expected: number;
  readonly width: number;
}

/**
 * Goods C1, §46 B3: what the cell thinks this line costs, or nothing — a good whose price it has
 * never seen and has no outlook for is one it cannot put in a basket, and that is a real answer
 * rather than a guess at a level (Law 3).
 */
function lineFor(
  view: ParticipantView,
  self: CellParty,
  row: ConsumptionDecl,
  p: HouseholdParams,
): Line | undefined {
  const instrument = goodId(row.subUnit, self.region);
  const outlook = view.outlook(`price.${instrument}`);
  const print = view.print(instrument);
  const expected = outlook.some
    ? outlook.value.expected
    : print.some
      ? print.value.price
      : undefined;
  if (expected === undefined || expected <= 0) return undefined;
  return {
    row,
    instrument,
    market: goodMarketId(row.subUnit, self.region),
    perUnit: mul(expected, add(1, p.consumptionTax, 'and the tax on it'), 'what a unit costs it'),
    expected,
    width: outlook.some ? outlook.value.confidence : 0,
  };
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
