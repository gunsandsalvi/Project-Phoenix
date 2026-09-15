/**
 * What a firm invests: a project, a return, a cost of capital, and whether it can fund it.
 *
 * @spec Capital Programme B1 Capital Programme B1.a Capital Programme B1.b Capital Programme B1.c Capital Programme B1.d Capital Programme B2 Capital Programme B2.a Capital Programme B3 Capital Programme B4 Capital Programme B5 Capital Programme C1 Capital Programme C2 Capital Programme D3 Capital Programme E3 Firm E3 Firm E4 Firm E4.a Clearing A2 XI-4 Law 2 Law 6 Law 8
 *
 * XI-4 JOINT TWO, and it is the joint this whole document exists to have. A financial price changes;
 * somebody's cost of capital changes; a real decision changes; output changes, with a lag. Every
 * step of that is here, and none of it is a rate applied to anything:
 *
 * - **The return** (B1.a) comes from expected demand and price: the contribution one more unit of
 *   output a period would bring, at the price this firm expects and the costs it faces.
 * - **The cost** (B1.b) comes from the markets: what its debt costs **at the margin, now** — the
 *   quote it has been given this period, never the average coupon on what it already owes — and
 *   what its equity costs, which is what its own shareholders are paid over what they paid for a
 *   share. Weighted by its own capital structure, which is a read of its own balance sheet.
 * - **The hurdle and the horizon are the management's own** (B1.d): two preferences, dispersed,
 *   because a management's risk aversion and its patience are its own and not the market's.
 * - **Uncertainty delays it** (B4): the spend is irreversible, so what the firm commits to is what
 *   it expects to sell LESS the width of its own recent surprises. There is no coefficient: the
 *   margin IS its confidence read, applied to the quantity it is about to build for, which is where
 *   the number lives and where its units are (Law 8). A firm whose expectation is inside its own
 *   dispersion waits, and waiting is the option being exercised.
 * - **Capacity utilisation is a reason** (B3): the gap is what it would run at against what its
 *   plant will still let it make NEXT period, once this period's wear is off it. A firm running
 *   empty has no gap and no project; a firm running full is short by what is about to wear out,
 *   which is why replacement is investment and why a world that stops investing loses capacity.
 *   Nothing computes a utilisation ratio to decide any of it: it is the two quantities themselves.
 * - **It must be able to fund it** (B2, B2.a): what it can pay for now is cash it is not about to
 *   need, and what it cannot fund now it publishes as a programme — which is what a bank lends
 *   against and what a share issue is raised into (Firm E4.a). A good project with no funding does
 *   not invest, and that is a credit constraint doing real work.
 *
 * B5 IS WHAT IS NOT HERE: no fraction of profit, no fraction of output, no multiplier on either.
 * The only way anything is bought below is that a unit of capacity was worth more to this firm than
 * what the market is asking for the plant that makes one.
 */
import { payrollSince } from './decide.js';
import { firmParam } from './data.js';
import {
  amountOf,
  asCash,
  asRatio,
  type Cash,
  minus,
  over,
  type PerPiece,
  plus,
  type Ratio,
  ratioOf,
  scale,
  valueAt,
} from '../../core/measure.js';
import { yearFraction } from '../../calendar/daycount.js';
import { Missing } from '../../core/errors.js';
import type { InstrumentId, MarketId } from '../../core/ids.js';
import { period } from '../../calendar/calendar.js';
import { atLeast, atMost, material, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { ParticipantView } from '../../world/context.js';
import { capacityFrom, plantHeld, type HeldVintage, type PlantNeed } from '../../registry/physical.js';
import type { PlannedOrder } from './decide.js';
import { downTick, subQty, upTick, type Qty } from '../../core/tick.js';
import { about } from '../../world/context.js';

/** B1.b: what money costs this firm at the margin, now, and what it is made of. */
export interface CostOfCapital {
  /** Item 16: every one of these is per annum on a unit of capital — pure numbers, never money. */
  readonly perAnnum: Ratio;
  /** The quote it has been given, per annum. None when no bank has quoted it. */
  readonly debt: Option<Ratio>;
  /** What its shareholders are paid over what a share costs. None when its shares do not trade. */
  readonly equity: Option<Ratio>;
  /** B1.c: what it owes and what its owners hold — the two sides the blend weighs. */
  readonly debtWeight: Cash;
  readonly equityWeight: Cash;
}

/** B1: a project this firm could do, with everything the decision was made of on it. */
export interface Project {
  /** B1.a, B3: the extra output per period it expects to sell and its plant cannot make. */
  readonly gap: number;
  /** B4: the rate it is sure enough of to build for — its run rate less its own surprise width. */
  readonly cautiousRunRate: number;
  /** A6, D1: what its plant will still let it run at next period, once this period's wear is off. */
  readonly capacityNext: number;
  readonly costOfCapital: number;
  readonly hurdle: number;
  /** B1.b, B1.d: what a project has to earn, per annum. */
  readonly required: number;
  /** B1.a: what one more unit of output a period brings in, per annum. */
  readonly contributionPerAnnum: number;
  /** The most it would pay for the plant that makes one more unit a period. */
  readonly worthPerUnitOfCapacity: number;
  /** What the market is asking for that plant. */
  readonly askedPerUnitOfCapacity: PerPiece;
  /** C2, B2: what the whole of it would cost, at what it is bidding. */
  readonly spend: Cash;
  /** B2.a: what it can pay for out of what it has. The rest is the programme it must fund. */
  readonly funded: Cash;
  /** Firm E4.a: what it wants to spend and cannot yet, which is what it raises money into. */
  readonly programme: Cash;
  readonly orders: readonly PlannedOrder[];
}

/**
 * B1.b, XI-4 joint one meeting joint two: the cost of capital, weighted by this firm's own capital
 * structure. Both terms are READ — the debt cost off the quote a bank gave it this period, the
 * equity cost off what its own shareholders have been paid against what the market says a share is
 * worth. A firm nobody has quoted and whose shares do not trade has no cost of capital, and it
 * makes no investment decision at all rather than being handed a number to compare against.
 */
export function costOfCapital(view: ParticipantView): Option<CostOfCapital> {
  const debt = quotedRate(view);
  const equity = requiredOnEquity(view);
  const debtWeight = owes(view);
  const equityWeight = view.equity();
  if (!debt.some) {
    return equity.some
      ? some({ perAnnum: equity.value, debt, equity, debtWeight, equityWeight })
      : none<CostOfCapital>();
  }
  if (!equity.some) {
    // B1.b: what its debt costs at the margin IS its cost of capital when nothing prices its
    // equity. A firm nobody has ever bought a share of has no market read of what its equity costs
    // (Equity A6), and inventing one would be exactly the imported number Law 2 forbids.
    return some({ perAnnum: debt.value, debt, equity, debtWeight, equityWeight });
  }
  const capital = plus(debtWeight, equityWeight, 'its capital');
  if (capital <= 0) return some({ perAnnum: debt.value, debt, equity, debtWeight, equityWeight });
  return some({
    perAnnum: ratioOf(
      plus(
        scale(debtWeight, debt.value, 'what its debt costs it'),
        scale(equityWeight, equity.value, 'what its equity costs it'),
        'what its capital costs it',
      ),
      capital,
      'its cost of capital',
    ),
    debt,
    equity,
    debtWeight,
    equityWeight,
  });
}

/**
 * B1.b: what its debt costs AT THE MARGIN, NOW — the rate a bank has quoted it, not the average
 * coupon on debt already outstanding. XI-4 names that average as the way this joint is deleted.
 */
function quotedRate(view: ParticipantView): Option<Ratio> {
  /**
   * B1.b, E5, item 0 (stop 17): WHAT BORROWING WOULD COST IT, from the keenest thing anybody has
   * said about it — and a firm nobody has quoted LATELY is not a firm nobody has priced.
   *
   * It read only its own `credit.quoted` from the last payroll window, and a bank quotes a firm
   * when the firm ASKS: a firm forms no borrow need until it has a project, and it can have no
   * project without a cost of capital to test it against. So nothing was ever built, in any run,
   * for a reason that was a circle and not an economy.
   *
   * The circle is cut by reading what does not depend on the firm having already asked, in the
   * order of how close each is to what borrowing would actually cost it NOW:
   *
   *   1. the quote it was given in this window — the marginal cost of debt, and the best answer;
   *   2. the last quote it was ever given, stale, and later in the list because it is;
   *   3. the sovereign curve its own money borrows at, which is the rate with no spread in it.
   *
   * The third is the floor of the three and is honest as one: a firm that reads it knows only what
   * money costs the state, and its own hurdle (Firm A3) is what stands between that and a project
   * going ahead. What is NOT here is what BANKS privately require of the name — that is real
   * (Corporate Credit E5) and `ctx.requiredOf` reads it where an ISSUER opens a line, but it is
   * recorded private and a party's own view may not see another party's private state (Observer
   * A4). What is NOT here either is a number nobody said (Law 3).
   */
  const own = view.lastOwnSince('credit.quoted', payrollSince(view.period));
  const quoted = own.some ? own : view.lastOwn('credit.quoted');
  if (quoted.some) {
    const rate = quoted.value.data['rate'];
    // Item 16: a rate re-entering from what was published under this firm's name.
    if (typeof rate === 'number') return some(asRatio(rate, 'what it was quoted, per annum'));
  }
  return sovereignRate(view);
}

/**
 * B1.b, Sovereign D3: what the state that issues its money borrows at, over the life of what it is
 * deciding about — the one rate in this world that is a read for every party, and the least any of
 * them could borrow at. It is a curve point and never a level anybody wrote (Law 3).
 */
function sovereignRate(view: ParticipantView): Option<Ratio> {
  const family = view.sovereignCurveIn(view.registry.currencyOf(view.self.region));
  if (!family.some) return none<Ratio>();
  // B1.d, Law 8: at the tenor of the decision, which is this management's own horizon — in years,
  // because that is what a curve is read at, and the calendar does the crossing.
  const horizon = view.params.periods(firmParam(view.self.id, 'horizon'));
  const years = yearFraction(
    'ACT/365F',
    view.calendar.startOf(view.period),
    view.calendar.startOf(period(view.period + horizon)),
  );
  if (years <= 0) return none<Ratio>();
  const at = view.curve(family.value.id).at(years);
  return at.yield.some ? some(asRatio(at.yield.value, 'what the state borrows at')) : none<Ratio>();
}

/** Firm C2: what this firm owes — the liabilities it has issued, at what is outstanding. */
function owes(view: ParticipantView): Cash {
  const terms: Cash[] = [];
  // Law 19: the register already indexes what a party issued, so this reads its own lines rather
  // than every line in the world to find them.
  for (const i of view.instruments.issuedBy(view.self.id)) {
    if (!i.status.live) continue;
    if (!view.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    // Money A1: a liability is owed at its face, so what is out is what it owes.
    terms.push(asCash(i.issued, `what it owes on ${i.id}`));
  }
  return sum(terms).value;
}

/**
 * B1.b, Equity B4: what its equity costs — what its own shareholders are being paid, over what the
 * market says a share of it is worth. It is a read of a cleared price and of this firm's own outlook
 * of its own earnings (§32 E7), and it exists only where a market prices its shares: a firm nobody
 * has ever bought a share of has no market read of what its equity costs (Equity A6).
 */
function requiredOnEquity(view: ParticipantView): Option<Ratio> {
  const line = shareLine(view);
  if (!line.some) return none<Ratio>();
  const print = view.print(line.value);
  if (!print.some || print.value.price <= 0) return none<Ratio>();
  const outlook = view.outlook(about({ on: 'earnings' }));
  if (!outlook.some) return none<Ratio>();
  const shares = view.instruments.get(line.value).issued;
  if (shares <= 0) return none<Ratio>();
  const cap = valueAt(print.value.price, shares, 'what the market says it is worth');
  if (cap <= 0) return none<Ratio>();
  // Law 8: what it earns is per period and what a return is is per annum, so the period is turned
  // into the fraction of a year the calendar says it is, and never into a periods-per-year.
  const year = yearFraction(
    'ACT/365F',
    view.calendar.startOf(view.period),
    view.calendar.startOf(period(view.period + 1)),
  );
  if (year <= 0) return none<Ratio>();
  return some(
    ratioOf(
      over(
        asCash(outlook.value.expected, 'what it expects to earn'),
        asRatio(year, 'the fraction of a year this period is'),
        'what it earns, per annum',
      ),
      cap,
      'what its equity costs',
    ),
  );
}

/** Equity A1: the residual claim on this firm, if a market prices one. It is a claim nobody owes. */
function shareLine(view: ParticipantView): Option<InstrumentId> {
  for (const i of view.instruments.issuedBy(view.self.id)) {
    if (!i.status.live) continue;
    if (view.registry.instrumentKind(i.kind).liabilityOfIssuer || !i.market.some) continue;
    return some(i.id);
  }
  return none<InstrumentId>();
}

/** What a project needs to know about plant it could buy and what the market asks for it. */
export interface PlantOffer {
  readonly capitalKind: string;
  /** A2: units of plant that let the line start one more unit a period. */
  readonly unitsPerUnitPerPeriod: number;
  readonly market: MarketId;
  /** What a unit of it is asking, which is what this firm expects that market to take. */
  readonly price: PerPiece;
  /** A6: periods of service what is on offer still has. New plant has its whole life. */
  readonly periodsOfService: number;
  /**
   * C1, D3: whether this is plant BUILT — bought from a capital-goods producer — or a vintage
   * somebody already owns, which in this world means a dead firm's, sold by its estate. The
   * replacement bundle is what a project is priced against; a second-hand vintage is worth what its
   * remaining service is worth to this firm, which is less, and that is D3's whole point.
   */
  readonly newBuild: boolean;
}

/**
 * B1, B2, B3, B4: the decision. Everything in it is this firm's own — its outlook, its plant, its
 * cost of capital, its hurdle, its horizon and its cash — and the answer is an ORDER at the price
 * where the return equals what it requires (Clearing A2), never a quantity anybody targeted.
 */
export function project(
  view: ParticipantView,
  needs: readonly PlantNeed[],
  vintages: readonly HeldVintage[],
  /** A6, D1: what its plant will still let it run at NEXT period, computed once and read here. */
  capacityNext: Qty,
  offers: readonly PlantOffer[],
  /** B1.a: the units it would START each period at what it expects to sell (Goods B4's yield in it). */
  wantedPerPeriod: Qty,
  /** B4: how wide its own surprises about that have been, in the same units. */
  surpriseWidth: Qty,
  /** B1.a: the contribution one unit of output brings — what it fetches less what it takes to make. */
  contributionPerUnit: PerPiece,
  hurdle: Ratio,
  horizonPeriods: number,
  cost: CostOfCapital,
  /** B2: what it can pay with now, after what it is already about to have to pay. */
  spendable: Cash,
): Option<Project> {
  if (needs.length === 0 || offers.length === 0) return none<Project>();
  // B4: the spend is irreversible, so what it builds for is what it would run at less the width of
  // its own recent surprises. A firm whose expectation is inside its own dispersion waits, and
  // waiting is the option being exercised. There is no coefficient: the margin IS the width.
  const cautious = subQty(wantedPerPeriod, surpriseWidth, 'what it is sure enough of to build for');
  if (cautious <= 0 || contributionPerUnit <= 0) return none<Project>();
  // B3: the gap IS utilisation as a reason, and it is the two quantities rather than their ratio.
  // What it measures against is what its plant will still let it run at NEXT period — capital now
  // minus what wears out (D1) — so a firm at its ceiling is short of what is about to go.
  const gap = subQty(cautious, capacityNext, 'what it will be short of');
  if (!material(gap, needs.length + 2, cautious) || gap <= 0) return none<Project>();
  const required = plus(cost.perAnnum, hurdle, 'what a project has to earn');
  if (required <= 0) return none<Project>();
  const year = yearFraction(
    'ACT/365F',
    view.calendar.startOf(view.period),
    view.calendar.startOf(period(view.period + 1)),
  );
  if (year <= 0) return none<Project>();
  const contributionPerAnnum = over(
    contributionPerUnit,
    asRatio(year, 'the fraction of a year this period is'),
    'what a unit of capacity brings, per annum',
  );
  // C1: what one unit of capacity costs to BUILD, which is the bundle a project is priced against.
  // A kind nothing is building is a kind this firm cannot get, so there is no project at all.
  const build = new Map<string, PerPiece>();
  for (const o of offers) if (o.newBuild) build.set(o.capitalKind, o.price);
  if (needs.some((n) => !build.has(n.capitalKind))) return none<Project>();
  const asked = sum(
    needs.map((n) =>
      scale(
        priceOfKind(build, n.capitalKind),
        asRatio(n.unitsPerUnitPerPeriod, 'the plant a unit takes'),
        'the plant a unit takes',
      ),
    ),
  ).value;
  if (asked <= 0) return none<Project>();
  // C2, B2: what it would cost is what it expects to PAY — the level the market is asking — and
  // what it BIDS is its own reservation, which is a different number and a different question
  // (Clearing A2). Sizing the spend at its own reservation would have a dearer cost of capital buy
  // MORE of a thing it values less, because its bid would have fallen.
  const wanted: {
    readonly order: PlannedOrder;
    readonly outlay: Cash;
    readonly asking: PerPiece;
  }[] = [];
  for (const o of offers) {
    // B1.d, A6: it counts the service THIS plant will give it, out to its own horizon. A vintage
    // half worn out is worth about half as much, and that is why a dead firm's plant fetches what
    // it fetches rather than a discount somebody wrote down (D3).
    const counted = countedYears(view, o.periodsOfService, horizonPeriods);
    if (counted <= 0) continue;
    const recovery = plus(
      required,
      over(
        asRatio(1, 'the whole of the capital'),
        asRatio(counted, 'the years of service it counts'),
        'what returning the capital costs a year',
      ),
      'what a unit must earn',
    );
    if (recovery <= 0) continue;
    const worth = over(contributionPerAnnum, recovery, 'what a unit of capacity is worth to it');
    // What a unit of THIS plant is worth to it: the capacity it makes possible, less what the rest
    // of the bundle costs to build. It is the same reason its bid for an input is what it is.
    const others = sum(
      needs
        .filter((n) => n.capitalKind !== o.capitalKind)
        .map((n) =>
          scale(
            priceOfKind(build, n.capitalKind),
            asRatio(n.unitsPerUnitPerPeriod, 'what the bundle takes of it'),
            'the rest of the bundle',
          ),
        ),
    ).value;
    const bid = over(
      minus(worth, others, 'what this kind of plant is worth on its own'),
      asRatio(o.unitsPerUnitPerPeriod, 'the units of it a unit of capacity takes'),
      'the most it will pay for a unit of it',
    );
    // B1: it invests when the return exceeds its cost of capital, which is exactly the statement
    // that a unit of this plant is worth more to it than what the market is asking for one.
    if (bid <= 0 || bid <= o.price) continue;
    // Law 8: a machine is a whole machine, and plant that is a fraction short of what the gap
    // needs does not close it — so what it bids for is the whole ones that do.
    const qty = upTick(
      scale(gap, asRatio(o.unitsPerUnitPerPeriod, 'the plant a unit takes'), 'units of plant the gap needs'),
    );
    wanted.push({
      order: { market: o.market, side: 'buy', price: bid, qty },
      outlay: valueAt(o.price, qty, 'what it expects to pay for them'),
      asking: o.price,
    });
  }
  if (wanted.length === 0) return none<Project>();
  // C2: what the whole of it would cost at the level the market is asking. It is what the firm
  // wants to spend, and what it cannot pay for out of what it holds is its PROGRAMME (Firm E4.a).
  const spend = sum(wanted.map((x) => x.outlay)).value;
  if (spend <= 0) return none<Project>();
  // B2, B2.a, Clearing A2: what it POSTS is what it could actually pay for at the price it is
  // bidding — a bid it could not settle is not an order, and a firm that would have to pay its own
  // reservation for all of it can only afford a few. That is what makes its demand for plant a
  // schedule rather than a quantity: the dearer the plant, the less of it this firm is in for.
  const orders: PlannedOrder[] = [];
  const funded: Cash[] = [];
  for (const x of wanted) {
    if (x.asking <= 0) continue;
    // Its money is spread over the places the plant could come from, in the proportions those
    // places are asking for — one stated rule, applied the same way to each of them (Law 4).
    const purse = scale(
      atLeast(spendable, asCash(0, 'nothing'), 'a firm with nothing spare has nothing to spread'),
      ratioOf(x.outlay, spend, 'this line\u2019s share'),
      'what it can put here',
    );
    // How many it can pay for is a question about the PRICE IT EXPECTS TO PAY, not about the most
    // it would pay: a firm that values a machine highly does not thereby buy fewer of them.
    // Law 8: a machine is a whole machine, and this is what its money REACHES — down, because a
    // firm that can pay for four and two thirds of one can pay for four.
    const canPay = downTick(amountOf(purse, x.asking, 'units it can pay for'));
    const qty = atMost(canPay, x.order.qty, 'it buys with the money it has, and only what is offered');
    if (qty <= 0 || !material(qty, wanted.length + 1, x.order.qty)) continue;
    orders.push({ ...x.order, qty });
    funded.push(valueAt(x.asking, qty, 'what it expects to pay for them'));
  }
  const affordable = sum(funded).value;
  return some({
    gap,
    cautiousRunRate: cautious,
    capacityNext,
    costOfCapital: cost.perAnnum,
    hurdle,
    required,
    contributionPerAnnum,
    worthPerUnitOfCapacity: over(contributionPerAnnum, required, 'what its capacity is worth a unit'),
    askedPerUnitOfCapacity: asked,
    spend,
    funded: affordable,
    // Firm E4.a: what it wants to spend and cannot — the programme it raises money into. A firm
    // with no programme raises nothing, and this is the number that says whether it has one.
    programme: minus(spend, affordable, 'what it must fund'),
    orders,
  });
}

/** The asking price of one kind in the replacement bundle; a kind without one is a defect above. */
function priceOfKind(build: ReadonlyMap<string, PerPiece>, capitalKind: string): PerPiece {
  const p = build.get(capitalKind);
  if (p === undefined) {
    throw new Missing('Capital Programme C1', `nobody builds ${capitalKind}`, { capitalKind });
  }
  return p;
}

/** B1.d, A6: the years of service it counts — its own horizon, or what the plant will give it. */
function countedYears(
  view: ParticipantView,
  periodsOfService: number,
  horizonPeriods: number,
): number {
  const counted = atMost(periodsOfService, horizonPeriods, 'there is no service beyond the life the plant has');
  if (counted <= 0) return 0;
  return yearFraction(
    'ACT/365F',
    view.calendar.startOf(view.period),
    view.calendar.startOf(period(view.period + Math.floor(counted))),
  );
}

/** A2, A4: what this firm's plant lets it make now, for whoever is reporting it (D4). */
export function capacityNow(
  needs: readonly PlantNeed[],
  vintages: readonly HeldVintage[],
): Option<number> {
  const c = capacityFrom(needs, vintages);
  return c.some ? some(c.value.perPeriod) : none<number>();
}

/** A4: the units of each kind of plant this firm holds, for the record it publishes about itself. */
export function plantByKind(
  needs: readonly PlantNeed[],
  vintages: readonly HeldVintage[],
): Record<string, number> {
  const out: Record<string, number> = {};
  for (const need of needs) out[need.capitalKind] = plantHeld(vintages, need.capitalKind);
  return out;
}
