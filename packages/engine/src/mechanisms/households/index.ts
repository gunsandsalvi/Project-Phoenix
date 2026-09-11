/**
 * Households: cells that earn, consume, save and own, each deciding for one possible household and
 * carrying how many of them it is.
 *
 * @spec Equity B1 Equity B3 Equity C2 Equity C2.a Households A1 Households A2 Households A2.a Households A2.b Households A2.c Households A2.d Households A2.e Households A2.f Households A3 Households B1 Households B2 Households B3 Households B3.a Households B5 Households C1 Households C1.a Households C1.b Households C1.c Households C1.d Households C2 Households C3 Households C4 Households C5 Households D1 Households D1.a Households D3 Households D5 Households D5.a Households D6 Goods C1 Goods C3 Expectations C1 Sovereign E2.f XI-15 XI-16 Law 2 Law 4 Law 6
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
import type { Family, Violation } from '../../audit/audit.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import { period } from '../../calendar/calendar.js';
import { marketId, paramId, type MarketId, type PartyId } from '../../core/ids.js';
import { addTo, combineDust, div, material, mul, sub, sum, withinDust, zeroIfNone } from '../../core/num.js';
import { isAssetLeg, isMoneyLeg } from '../../ledger/instruction.js';
import { weightOf } from '../../parties/party.js';
import { HOUSEHOLD } from '../../registry/profiles.js';
import type { ParamDecl } from '../../registry/params.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { householdChoosesBank, HOUSEHOLD_SWITCHING_COST } from './bank.js';
import { CONSUMPTION, type ConsumptionDecl } from './data.js';
import { demandOf, spendPerMember, type HouseholdParams } from './consume.js';
import {
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

export * from './data.js';
export { householdChoosesBank, HOUSEHOLD_SWITCHING_COST } from './bank.js';
export { demandOf, spendPerMember } from './consume.js';
export { levelsBelow, rungsOver } from './demand.js';
export type { Rung } from './demand.js';
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
import { asQty, scaleQty } from '../../core/tick.js';
export type { DemandStep, HouseholdParams, Spending } from './consume.js';
export type { FundOrder, FundPosition, PaperBid, SavingLine, ShareOrder } from './portfolio.js';

export const HOUSEHOLD_PARAMS = {
  patience: paramId('households.patience'),
  buffer: paramId('households.buffer.periods'),
  liquidityPremium: paramId('households.liquidityPremium'),
  horizon: paramId('households.horizon.periods'),
  steps: paramId('households.demand.steps'),
} as const;

/** Treasury C1: the rate a household pays on what it buys, which it must find on top of the price. */
export const CONSUMPTION_TAX = paramId('treasury.tax.consumption');

function paramsOf(): ParamDecl[] {
  return [
    {
      id: HOUSEHOLD_PARAMS.patience,
      value: 6,
      unit: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Households C1: over how many of its own periods a household closes the gap between the cash it holds and the cushion it wants. It is the whole of its patience: a windfall it means to keep reaches its spending over this many weeks, and a hole it has fallen into is refilled over the same.',
    },
    {
      id: HOUSEHOLD_PARAMS.buffer,
      value: 4,
      unit: 'periods of its own income',
      kind: 'preference',
      owner: 'model',
      why: 'Households C1.d, §46 B3: how many periods of what it expects a household wants to be sitting on. It is widened by how wrong its own income has recently been, which is a read of its own surprises and not a second number.',
    },
    {
      id: HOUSEHOLD_PARAMS.liquidityPremium,
      value: 0.005,
      unit: 'per annum over what a deposit returns',
      kind: 'preference',
      owner: 'model',
      why: 'Households D5, D5.a: what a saver wants for giving up instant access to its money. It is the whole of the substitution between a deposit and paper held directly, and it is what makes a rate reach a saver at all.',
    },
    {
      id: HOUSEHOLD_PARAMS.horizon,
      value: 52,
      unit: 'periods',
      kind: 'preference',
      owner: 'model',
      why: 'Households D5: how long a household will tie its money up. Paper that comes back inside it is a substitute for its deposit; anything longer it would have to sell at a price nobody can tell it, which is D5 other two reasons — yield against risk — and it cannot weigh those until something in this world prices risk (worklist 9).',
    },
    {
      id: HOUSEHOLD_SWITCHING_COST,
      value: 40,
      denominated: true,
      unit: 'of the money the account is in, per move, per member',
      kind: 'preference',
      owner: 'model',
      why: 'Banks Funding A1.d, E1: what it costs one household to move its account, ONCE, as an amount of its own money. It is weighed against what staying has already cost it — its own balance times the gap between the boards over as long as it has stayed — so a bigger balance moves for a smaller gap and the class drains instead of crossing at one instant. Retail money is the stickiest because the amount is large beside what a household holds, and that is A1.a arriving as a cost somebody bears rather than as a stated stickiness.',
    },
    {
      id: HOUSEHOLD_PARAMS.steps,
      value: 5,
      unit: 'count',
      kind: 'resolution',
      owner: 'model',
      why: 'Goods C1, Clearing A2: how finely a household posts its own demand curve into the book. Its shape is the cell own — what it spends divided by the price — and this is only how many levels of it the book sees; change it and the answer must not move.',
    },
  ];
}

function numbers(view: ParticipantView): HouseholdParams {
  return {
    patience: view.params.get(HOUSEHOLD_PARAMS.patience),
    bufferPeriods: view.params.get(HOUSEHOLD_PARAMS.buffer),
    steps: view.params.get(HOUSEHOLD_PARAMS.steps),
    consumptionTax: view.params.get(CONSUMPTION_TAX),
  };
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
        const paid = new Map<PartyId, number>();
        for (const leg of r.instruction.legs) {
          if (isAssetLeg(leg) && cells.has(leg.to)) {
            const physical = view.registry.instrumentKind(
              view.instruments.get(leg.instrument).kind,
            ).physical;
            if (physical !== true) continue;
            addTo(bought, leg.to, mul(leg.qty, leg.pricePerUnit.some ? leg.pricePerUnit.value : 0, 'what it took'));
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
          const grain = weightOf(who);
          if (withinDust(took.value, money.value, combineDust(took, money) + grain / 2)) continue;
          out.push({
            family: 'flows',
            spec: 'Households C5',
            owner: cell,
            size: sub(took.value, money.value, 'goods against money'),
            unit: view.registry.region(who.region).ccy,
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
export function households(rows: readonly ConsumptionDecl[] = CONSUMPTION): SystemModule {
  return {
    id: 'households',
    spec: 'Households, Sovereign E2.f',
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
      {
        id: HOUSEHOLD,
        representation: 'cell',
        moneyIssuer: null,
        fails: [],
        borrows: false,
        depositClass: 'retail',
      },
    ],
    curveFamilies: [],
    units: [],
    params: paramsOf(),
    phases: [
      {
        name: 'households.decide',
        spec: 'Households C1 Households C2 Households C3 Households D5 Households B5',
        cycle: 0,
        anchor: { before: 'labour.match' },
        run: (ctx: MechanismContext) => {
          publishSectorIncome(ctx);
          for (const p of ctx.parties.ofKind(HOUSEHOLD)) {
            if (p.status.alive) decide(ctx, p.id, rows);
          }
        },
      },
    ],
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
          const own = view.lastOwn('households.plan');
          if (!own.some || own.value.period !== view.period) return [];
          return marketsIn(own.value.data['orders']);
        },
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          const own = view.lastOwn('households.plan');
          if (!own.some || own.value.period !== view.period) return [];
          return ordersFrom(own.value.data['orders'], m.id, view.self.id);
        },
      },
    ],
    families: [consumptionIsBought()],
    bankChoices: [{ partyKind: HOUSEHOLD, chooses: householdChoosesBank }],
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
  // D2: what it can pay with is its account AND what it can ask back from a fund on demand — that
  // is what makes a money fund a substitute for a deposit rather than an investment (D2).
  const positions = fundPositions(ctx.venues, ctx.journal.ofKind('fund.struck'), view);
  const onDemand = sum(positions.map((f) => f.worthPerMember)).value;
  const decided = spendPerMember(view, p, onDemand);
  if (!decided.some) return;
  const goods = demandOf(view, rows, p, decided.value.spend);
  const spare = sparePerMember(
    view.cash(view.registry.region(self.region).ccy),
    decided.value.spend,
    decided.value.buffer,
  );
  // D5.a: what it requires of paper is what its deposit pays it plus what giving up access costs
  // it. A deposit pays nothing until a bank decides to pay for one (Banks Funding B1, worklist
  // 11), so what it requires now is the premium alone, and that comparison becomes a real one
  // the period a bank starts bidding for deposits.
  const required = view.params.get(HOUSEHOLD_PARAMS.liquidityPremium);
  // D5: everywhere its savings could go, in one pass, with what it thinks each is worth — paper it
  // can price off a public curve, and shares it can only price off what they have been paying.
  const { paper, shares } = savingLines(
    view,
    required,
    ownUncertainty(view),
    view.params.get(HOUSEHOLD_PARAMS.horizon),
  );
  // D5: one budget, spread over every place its money could go this period. Deciding it once and
  // dividing it is what stops the same money being committed twice (Law 4) and what stops a rule
  // nobody stated from preferring one class of thing to another.
  const lines = paper.length + shares.length;
  const perLine = lines > 0 ? div(spare, lines, 'what it puts into one line') : 0;
  const paperOrders = paperBids(view, paper, perLine, lines, weightOf(self));
  // D2, D5: the third thing it can do with its money, and the reason it asks for it back. What the
  // fund published is public (Clearing F1: it acts on what it has already been told), and what it
  // offers is compared against the same requirement a bill is.
  const short = shortForSpending(decided.value.cash, decided.value.spend);
  const toFund = cushionForFund(decided.value.cash, decided.value.spend, spare);
  for (const o of fundOrders(positions, required, toFund, short)) {
    if (!material(o.sharesPerMember, 2, o.sharesPerMember)) continue;
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
  ctx.record(
    'households.plan',
    [cell],
    {
      // Per member, because that is what it decided (A2.f); the orders carry the cell's weight.
      spendPerMember: decided.value.spend,
      wantedPerMember: decided.value.wanted,
      bufferPerMember: decided.value.buffer,
      cashPerMember: decided.value.cash,
      wealthPerMember: decided.value.wealth,
      expectedIncome: decided.value.expected,
      // C1.d: its budget bound it. A2.g counts these, and a mean-preserving spread moves cells
      // across the threshold while the weighted mean of what they were paid does not move.
      constrained: decided.value.constrained,
      sparePerMember: spare,
      // D5: the places its money could go this period, and what goes into one of them.
      linesItMayHold: lines,
      perLinePerMember: perLine,
      shortForSpendingPerMember: short,
      toFundPerMember: toFund,
      // C2: what it does not spend and does not put into paper is saved where it already is.
      orders: [
        ...goods.map((g) => ({ market: g.market, side: 'buy', price: g.price, qty: g.qty })),
        ...paperOrders.map((b) => ({ market: b.market, side: 'buy', price: b.price, qty: b.qty })),
        ...shareOrders(view, shares, perLine, short, p.steps).map((o) => ({
          market: o.market,
          side: o.side,
          price: o.price,
          qty: o.qty,
        })),
      ],
    },
    false,
  );
}

/** The orders this cell decided on, read back from its own plan (Law 4: one decision, one writer). */
function marketsIn(rows: unknown): MarketId[] {
  if (!Array.isArray(rows)) return [];
  const out = new Set<string>();
  for (const row of rows as unknown[]) {
    if (typeof row !== 'object' || row === null) continue;
    const id = (row as Record<string, unknown>)['market'];
    if (typeof id === 'string') out.add(id);
  }
  return [...out].map((id) => marketId(id));
}

function ordersFrom(rows: unknown, market: string, self: PartyId): Order[] {
  if (!Array.isArray(rows)) return [];
  const out: Order[] = [];
  for (const row of rows as unknown[]) {
    if (typeof row !== 'object' || row === null) continue;
    const o = row as Record<string, unknown>;
    const price = o['price'];
    const qty = o['qty'];
    const side = o['side'];
    if (o['market'] !== market || (side !== 'buy' && side !== 'sell')) continue;
    // XI-2: a cell selling because it needs the money names no price. Everything else it posts is
    // a level of its own, and a level is a number.
    if (price !== 'market' && typeof price !== 'number') continue;
    if (typeof qty !== 'number' || qty <= 0) continue;
    // Law 19: read back from what this cell published, through the one door that says a size is a
    // count of pieces — and that throws if what it published was not (core/tick.ts).
    out.push({ party: self, side, price, qty: asQty(qty, `${self}'s posted size in ${market} at ${price}`) });
  }
  return out;
}
