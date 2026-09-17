/**
 * What a household decides to spend, and the demand it takes to market with it.
 *
 * @spec Households A2.a Households A2.b Households A2.f Households C1 Households C1.a Households C1.b Households C1.c Households C1.d Households C2 Households C3 Households C4 Households D6 Goods A2.a Goods C1 Expectations B3 Expectations C1 Fund Shares C2.b XI-2 XI-15 XI-16 Law 2 Law 6
 *
 * EVERY NUMBER HERE IS PER MEMBER of the cell that decided it (A2.f, XI-15). A cell is one possible
 * household carried with a multiplicity, so what it decides is what one household decides, and the
 * sector's number is the weighted sum of those — never a decision taken at the sector's average.
 *
 * IT BUYS ITS BASKET OUT OF WHAT IS LIQUID, AND WHAT STANDS ABOVE ITS TARGET IS SPARE (0f.7c).
 * What is liquid is its account and what a money fund owes it on demand, LESS what falls due on
 * what it has issued and the rent on its tenancy — both read, never estimated. Its target is a
 * stock: so many weeks of its own expected income, widened by how wrong that income has recently
 * been (C1.c: confidence is a read of its own surprises, never a stated mood) and by what its own
 * holdings could move by. The weeks are its PATIENCE, drawn once per cell the way its memory is
 * (XI-16 A3): two cells with the same income hold different cushions because they drew differently,
 * and that — not a rate anybody stated — is why a windfall reaches one cell's spending before the
 * other's. There is no gap-closing rate: below the target it buys the basket and keeps the rest;
 * above it, the rest is placed (`portfolio.ts`).
 *
 * IT CANNOT BORROW FOR A LOAF (C1.d): what it cannot pay for it does not buy. That is not a bound
 * on the decision — it is what a budget is. What it CAN borrow for is a roof (E2), and it asks its
 * bank for that through the one door every borrower uses (`index.ts homeBid`).
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
import { atLeastCash, atMostCash, noCash, sumCash } from '../../core/measure.js';
import { conditionsStanding } from '../../registry/environment.js';
import { scaleQty } from '../../core/tick.js';
import {
  type Amount,
  type Ratio,
  absolute,
  asAmount,
  asCash,
  asPerPiece,
  asRatio,
  type Cash,
  minus,
  over,
  type PerPiece,
  plus,
  ratioOf,
  scale,
  valueAt,
} from '../../core/measure.js';
import type { InstrumentId, MarketId } from '../../core/ids.js';
import { atMost, material, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { keyOf, type CellParty } from '../../parties/party.js';
import type { ParticipantView } from '../../world/context.js';
import { goodId, goodMarketId } from '../../registry/physical.js';
import type { ConsumptionDecl } from './data.js';
import { pricesOver, rungsUpTo } from '../../clearing/schedule.js';
import { about } from '../../world/context.js';
import { expectedPriceOf } from '../../registry/expectation.js';

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
  /** C1.c, XI-16 A3: THIS CELL's weeks of buffer, drawn once at its first decision (`index.ts`). */
  readonly patience: number;
  readonly steps: number;
  /** The rate the state charges on what a household buys. A `Ratio`: never a level (A-44). */
  readonly consumptionTax: Ratio;
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
   * What it will spend, per member: its basket at the prices it expects, and no more than what is
   * liquid after what falls due — a cell whose basket costs less than it has keeps the difference.
   */
  readonly spend: Cash;
  /** What its basket costs it at the prices it expects (C3), before its money had a say (C1.d). */
  readonly basket: Cash;
  /** C3, B1: the NEEDS alone — what an hour must bring in to feed a member (`index.ts willWork`). */
  readonly needs: Cash;
  /** The cushion it wants to be sitting on, per member: its target. */
  readonly buffer: Cash;
  /** E3, E4, 0f.7c: what falls due on it this period, per member — debt service and rent — before the basket. */
  readonly due: Cash;
  /** Its own outlook of what it will earn (C1.a). */
  readonly expected: Cash;
  /** What it holds in its account, per member. */
  readonly cash: Cash;
  /**
   * C1.d, D2: THE WHOLE OF WHAT IT CAN PAY WITH, per member — its account and what it can ask back
   * from a money fund on demand, LESS what falls due on it first. It is the number that bound the
   * decision, so it is the number `constrained` is about, and it is published: a reader that had
   * only `cash` would see a cell spending more than it holds.
   */
  readonly budget: Cash;
  /** C1.b, D3: what it owns, per member — its cash and its holdings at what the market last said. */
  readonly wealth: Cash;
  /** C1.d: whether its budget bound it — its basket cost more than it had — a threshold a mean-preserving spread moves cells across (A2.g). */
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
  basket: Basket,
  p: HouseholdParams,
  onDemand: Cash,
  due: Cash,
): Option<Spending> {
  const income = view.outlook(about({ on: 'income' }));
  if (!income.some) return none();
  const ccy = view.registry.currencyOf(view.self.region);
  // 0f.1: a cell decides for ONE member and holds a total, so what it has is read per member.
  const cash = asCash(view.cashPerMember(ccy), ccy, 'what one member has in the account');
  // E3, E4: what falls due on it comes first, because a payment that fails is an event and a loaf
  // it did not buy is not (XI-1). What is left is what it can pay with.
  const budget = minus(plus(cash, onDemand, 'what it can pay with'), due, 'after what falls due');
  const wealth = plus(wealthOf(view, cash), onDemand, 'what it owns');
  // C1.c, §46 B3: the target is so many weeks of what it expects, widened by how wrong that
  // expectation has recently been. Confidence is in the same unit as the variable, so a cell whose
  // income has been unpredictable by a given amount wants that much more in hand per week.
  const buffer = plus(
    scale(
      plus(
        asCash(income.value.expected, ccy, 'what it expects to take in'),
        asCash(income.value.confidence, ccy, 'how wrong that has been'),
        'what a week could cost it',
      ),
      asRatio(p.patience, 'the weeks of cushion it wants'),
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
  // C3, 0f.7c: what it means to buy is its basket, at the prices it expects, and no gap rate.
  const wanted = plus(basket.needs, basket.wants, 'what its basket costs it');
  // C1.d: it spends what it has, whatever its basket costs. And nobody buys a negative loaf.
  //
  // Law 7: nor an unrepresentable one. A cell whose budget is the rounding of a subtraction would
  // take a demand curve to market in quantities so small that the per-member share of a fill
  // cannot be multiplied back by the weight to give the total again — below the smallest normal
  // number there is no relative precision left, so dust itself underflows to zero and every
  // identity in the wire becomes exact. A spend that is dust of what it has is nothing.
  const affordable = atMostCash(wanted, budget, 'it buys with the money it has');
  const magnitudes = sum([budget.pieces, absolute(wanted, 'what its basket costs').pieces]);
  const afforded = material(affordable.pieces, magnitudes.terms + 1, magnitudes.value)
    ? affordable
    : noCash(ccy);
  return some({
    spend: atLeastCash(afforded, noCash(ccy), 'there is no less to spend than nothing'),
    basket: wanted,
    needs: basket.needs,
    buffer,
    due,
    expected: asCash(income.value.expected, ccy, 'what it expects to take in'),
    cash,
    budget,
    wealth,
    constrained: wanted.pieces > budget.pieces,
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
function atRisk(view: ParticipantView): Cash {
  const home = view.registry.currencyOf(view.self.region);
  const terms: Cash[] = [];
  for (const h of view.holdings()) {
    const outlook = view.outlook(about({ on: 'price', instrument: h.instrument }));
    if (!outlook.some || outlook.value.confidence <= 0) continue;
    // 0f.7d: PER MEMBER, like the cash it is set against. `h.lots` is the cell's total (0f.1).
    const units = asAmount<'piece'>(view.perMember(h.instrument), 'what one member holds of it');
    if (units <= 0) continue;
    // Currency C4.a, A-23: IN ITS OWN MONEY. What a cell is exposed to is one number in the money
    // it spends, and a household paid a coupon in a money it does not bank in (C4) has holdings in
    // two — so this added the move on a foreign line straight into the total.
    terms.push(
      view.inOwnMoney(
        valueAt(
          asPerPiece(outlook.value.confidence, 'how far it thinks this line can move'),
          units,
          view.instruments.get(h.instrument).ccy,
          'what this line could move by',
        ),
      ),
    );
  }
  return sumCash(home, terms, 'what its lines could move by').value;
}

/**
 * C1.b, D1, D3: what a household owns, per member: its money, and what the market last said its
 * holdings are worth. It is a read of the register and the price store at the moment it is asked,
 * and there is no stored net worth anywhere. What it holds that nothing has ever printed a price
 * for is not in it: a thing with no price is not wealth a household could count on.
 */
function wealthOf(view: ParticipantView, cash: Cash): Cash {
  // Currency B1, C4: a REPORT in the money it reports in, at the rate; what it holds stays what it is.
  const terms: Cash[] = [cash];
  for (const h of view.holdings()) {
    const print = view.print(h.instrument);
    if (!print.some) continue;
    // 0f.7d: what ONE MEMBER holds of it, because `cash` is one member's.
    const units = asAmount<'piece'>(view.perMember(h.instrument), 'what one member holds of it');
    // Currency C4.a, A-23: and the same for what it is worth. `cash` is its own money's balance,
    // so every term added to it has to be in that money.
    terms.push(
      view.inOwnMoney(
        valueAt(
          print.value.price,
          units,
          view.instruments.get(h.instrument).ccy,
          'what it holds is worth',
        ),
      ),
    );
  }
  return sumCash(cash.ccy, terms, 'what it owns').value;
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
/** C3, 0f.7a: what a member's basket costs at the prices this cell expects, needs and wants apart. */
export interface Basket {
  readonly lines: readonly Line[];
  /** What a member must have, costed: the reservation a long spell asks for (`index.ts willWork`). */
  readonly needs: Cash;
  readonly wants: Cash;
}

/**
 * C3, Law 4, 0f.7a: THE BASKET, COSTED ONCE. Two decisions read it — what to buy and what to work
 * for — and a second costing would be a second writer of what a loaf costs this cell.
 */
export function basketOf(
  view: ParticipantView,
  rows: readonly ConsumptionDecl[],
  p: HouseholdParams,
): Basket {
  const self = view.self;
  const lines: Line[] = [];
  if (self.representation === 'cell') {
    for (const row of rows) {
      if (row.cohort !== keyOf(self, 'cohort')) continue;
      const line = lineFor(view, self, row, p);
      if (line !== undefined) lines.push(line);
    }
  }
  const home = view.registry.currencyOf(view.self.region);
  const needs = sumCash(
    home,
    lines.map((l) =>
      view.inMoney(
        valueAt(
          l.perUnit,
          l.needed,
          view.instruments.get(l.instrument).ccy,
          'what it must have costs',
        ),
        home,
      ),
    ),
    'what it must have costs',
  ).value;
  const wants = sumCash(
    home,
    lines.map((l) =>
      view.inMoney(
        valueAt(
          l.perUnit,
          l.wanted,
          view.instruments.get(l.instrument).ccy,
          'what it wants on top costs',
        ),
        home,
      ),
    ),
    'what it wants on top costs',
  ).value;
  return { lines, needs, wants };
}

export function demandOf(
  view: ParticipantView,
  rows: readonly ConsumptionDecl[],
  p: HouseholdParams,
  spend: Cash,
): DemandStep[] {
  const self = view.self;
  if (self.representation !== 'cell') return [];
  const { lines, needs: needCost, wants: wantCost } = basketOf(view, rows, p);
  // C1.d: it eats before it does anything else, and it eats out of money it has.
  const toNeeds = atMostCash(spend, needCost, 'it needs what it needs and pays with what it has');
  const left = minus(spend, toNeeds, 'what is left when it has had what it must have');
  let needScale = asRatio(0, 'it can pay for none of what it must have');
  if (needCost.pieces > 0) {
    needScale = ratioOf(toNeeds, needCost, 'the fraction of what it must have it can pay for');
  }
  let wantScale = asRatio(0, 'it can pay for none of what it wants on top');
  if (wantCost.pieces > 0) {
    wantScale = atMost(
      ratioOf(left, wantCost, 'the fraction of what it wants on top it can pay for'),
      asRatio(1, 'all of it'),
      'it takes what it wanted and not more because it could afford more',
    );
  }
  const out: DemandStep[] = [];
  for (const l of lines) {
    const qty = plus(
      scale(l.needed, needScale, 'of what it must have'),
      scale(l.wanted, wantScale, 'of what it wants on top'),
      'what one member takes this period',
    );
    if (qty <= 0) continue;
    // C4: what it set aside is what the units cost it INCLUDING the tax; what reaches the seller is
    // that less the tax, and that is the money the curve is drawn against.
    const set = valueAt(
      l.perUnit,
      qty,
      view.instruments.get(l.instrument).ccy,
      'what it set aside for this line',
    );
    const net = over(
      set,
      plus(asRatio(1, 'the price itself'), p.consumptionTax, 'with the tax it will owe on it'),
      'what reaches the seller',
    );
    if (!material(net.pieces, 2, spend.pieces)) continue;
    for (const r of rungsUpTo(pricesOver(l.expected, l.width, p.steps), net, qty)) {
      out.push({
        market: l.market,
        instrument: l.instrument,
        price: r.price,
        qty: scaleQty(r.qty, self.weight, 'what the cell asks for'),
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
  readonly perUnit: PerPiece;
  /** §46 B3: the price it expects, and how wrong it has been about this one. */
  readonly expected: PerPiece;
  readonly width: PerPiece;
  /**
   * C3, Goods B4 (12d.3): what ONE MEMBER must have and wants on top THIS PERIOD — the row's normal
   * week over how the conditions it stands against stand, read off the region's public state.
   */
  readonly needed: Amount<'piece'>;
  readonly wanted: Amount<'piece'>;
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
  const outlook = view.outlook(about({ on: 'price', instrument: instrument }));
  // Law 4, Law 19 (0h.1): ITS OWN OUTLOOK, OR WHAT THE VENUE PRINTED — one read, shared with every
  // other party that prices off the tape (`registry/expectation.ts`), which named this basket as
  // one of the three copies of it. The cell HAS an outlook of every good its cohort buys from the
  // period after the world opens, because the basket is what it watches (§46 A1, `index.ts`), so
  // what is left here is the opening period alone.
  const level = expectedPriceOf(view, instrument);
  const expected = level.some ? level.value : undefined;
  if (expected === undefined || expected <= 0) return undefined;
  // 12d.3: a cold week burns more. The condition is a multiple of normal, so what a member takes
  // to stand against it is a normal week's over it — a physical relation, not a coefficient.
  const standing = asRatio(
    conditionsStanding(view, self.region, row.standsAgainst),
    'how the week stands for this line',
  );
  return {
    row,
    instrument,
    market: goodMarketId(row.subUnit, self.region),
    needed: over(
      asAmount<'piece'>(row.neededPerMember, 'what a member must have in a normal week'),
      standing,
      'what a member must have this week',
    ),
    wanted: over(
      asAmount<'piece'>(row.wantedPerMember, 'what a member wants on top in a normal week'),
      standing,
      'what a member wants on top this week',
    ),
    perUnit: scale(
      expected,
      plus(asRatio(1, 'the price itself'), p.consumptionTax, 'and the tax on it'),
      'what a unit costs it',
    ),
    expected,
    width: asPerPiece(
      outlook.some ? outlook.value.confidence : 0,
      'how wrong it has been about this one',
    ),
  };
}
