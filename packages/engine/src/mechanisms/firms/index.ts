/**
 * Firms: named parties that make a thing out of other things and the hours of named people, sell it
 * to named buyers, and keep whatever is left.
 *
 * @spec Firm A1 Firm A2 Firm A3 Firm B1 Firm B2 Firm B3 Firm B4 Firm B4.a Firm B4.b Firm B5 Firm B6 Firm C1 Firm C4 Firm D1 Firm E1 Firm E2 Firm E6 Firm E7 Firm F1 Firm F2 Firm F3 Goods B1 Goods B1.b Goods B1.c Goods B2 Goods B3 Goods B4 Goods B5 Goods B5.a Goods B5.b Goods C1 Goods C3 Goods C5 Goods F5 Goods F5.a Goods F5.b Labour C1 Labour C1.a Labour C5 Labour D1 Expectations C2 XI-16 Law 2 Law 6 Law 15
 *
 * The module owns the firm's DECISIONS and its LINE, and owns no data about the world beyond which
 * firm is in which line (its own registry). Everything it decides with — what a thing takes to make,
 * what an hour costs, what a unit fetched — is read from the kernel: the good's own terms carry the
 * recipe (docs/ARCHITECTURE.md 4.9b), the labour venue is found by what makes it itself, and the
 * wage bill is a record under the firm's own name. It employs nobody directly and it pays no wage:
 * the employment relationship is the labour module's and there is one writer of it (Law 4).
 *
 * WHAT IT MUST BE ABLE TO DO is fail. It pays out of a balance and the balance can hit zero (D1):
 * a wage that does not settle is recorded and the people are not paid; a trade it cannot fund
 * fails; a line whose output no longer covers what it takes to make stops, and the people go.
 * Nothing here catches any of that.
 *
 * WHAT IT CANNOT DO is decide differently because of what it makes (F4, Law 15). There is one
 * decision function, and the industry is data: a recipe, a lead time, a yield and an occupation.
 */
import type { Family, Violation } from '../../audit/audit.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { Order } from '../../clearing/solver.js';
import type { PartyId } from '../../core/ids.js';
import { combineDust, sub, sum, withinDust } from '../../core/num.js';
import { isCreateLeg } from '../../ledger/instruction.js';
import { FIRM } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { FIRMS, type FirmDecl } from './data.js';
import { ordersFrom, plan, venueOf, type PlannedOrder } from './decide.js';
import { publishExpectation, runLine } from './produce.js';

export * from './data.js';
export { plan, technologyOf, expectedPrice } from './decide.js';
export type { Offering, Plan, Planned, PlannedOrder } from './decide.js';

/** The line a named firm is in, if this world put it in one (Law 15: the data says). */
function lineOf(rows: readonly FirmDecl[], firm: PartyId): FirmDecl | undefined {
  return rows.find((r) => r.firm === firm);
}

/**
 * Goods B5, F5.b, Firm B4.b: production moves no value except what the period paid for it.
 *
 * A batch is created carrying what it drew — the inputs at what they cost their holder, which is
 * what settlement charged it for destroying them — plus the wage bill the period actually paid. So
 * over a firm's own production instructions in a period, what its equity account moved by is
 * exactly the wages it capitalised, and nothing else: a good coming off the line moves nothing at
 * all, because the cost of the units that did not make it simply follows the ones that did (B4).
 *
 * That makes the two ways of costing a batch agree — what it consumed, and what it says it cost —
 * and it is the check F5.b asks for: a cost capitalised into a batch AND expensed in the period is
 * counted twice, and it would show up here as an equity move nothing paid for.
 */
function productionCosts(rows: readonly FirmDecl[]): Family {
  return {
    name: 'flows',
    contributor: 'firms',
    spec: 'Firm B4.b Firm B6 Goods B5 Goods F5.b',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const moved = new Map<PartyId, number[]>();
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled' || r.instruction.cause !== 'production') continue;
        if (!r.instruction.legs.some(isCreateLeg)) continue;
        for (const e of r.equity) {
          if (lineOf(rows, e.party) === undefined) continue;
          const list = moved.get(e.party) ?? [];
          list.push(e.delta);
          moved.set(e.party, list);
        }
      }
      for (const [firm, deltas] of moved) {
        const capitalised = view.journal
          .ofKind('firms.started')
          .filter((e) => e.period === view.period && e.subjects.includes(firm))
          .map((e) => e.data['wages'])
          .filter((w): w is number => typeof w === 'number');
        const wages = sum(capitalised);
        const effect = sum(deltas);
        if (withinDust(effect.value, wages.value, combineDust(effect, wages))) continue;
        out.push({
          family: 'flows',
          spec: 'Goods F5.b',
          owner: firm,
          size: sub(effect.value, wages.value, 'value production moved'),
          unit: view.registry.region(view.parties.get(firm).region).ccy,
          period: view.period,
          message: `${firm}: its line moved ${effect.value} of value and the period paid ${wages.value} into it`,
        });
      }
      return out;
    },
  };
}

/**
 * The module. `rows` is this world's firms: which named party is in which line. A firm the registry
 * does not name is a firm in no line, which is a real state and not a defect — it holds what it
 * holds and decides nothing, because deciding to enter a line is birth (worklist 13g).
 */
export function firms(rows: readonly FirmDecl[] = FIRMS): SystemModule {
  return {
    id: 'firms',
    spec: 'Firm, Goods B, Goods F',
    // It reads the recipe from the good's terms, posts openings into the labour venue, and decides
    // from its own outlook: all three must exist before it does (Part XIII step 4).
    requires: ['expectations', 'goods', 'labour'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    // Law 2: it declares no number at all. What it makes a thing out of, how long it takes and what
    // survives the line are the good's technology; what an hour costs is what the market charged
    // it; and there is no target margin, no buffer and no adjustment speed anywhere in it.
    params: [],
    phases: [
      {
        name: 'firms.decide',
        spec: 'Firm E1 Firm E2 Firm E6 Firm E7 Goods B1 Labour C5',
        cycle: 0,
        // Before the jobs are struck, which is before the markets: everything it decides is decided
        // on what has already happened (Clearing F1).
        anchor: { before: 'labour.match' },
        run: (ctx: MechanismContext) => {
          for (const p of ctx.parties.ofKind(FIRM)) {
            const line = lineOf(rows, p.id);
            if (line === undefined || !p.status.alive) continue;
            decide(ctx, line);
            publishExpectation(ctx, p.id);
          }
        },
      },
      {
        name: 'firms.produce',
        spec: 'Firm B2 Firm B3 Goods B2 Goods B3 Goods B4 Goods B5',
        cycle: 2,
        // After the wage bill, because what the period's labour cost is part of what the batch cost
        // (Goods B5), and after the markets, because what it bought this period it can draw on.
        anchor: { after: 'labour.pay' },
        run: (ctx: MechanismContext) => {
          for (const p of ctx.parties.ofKind(FIRM)) {
            const line = lineOf(rows, p.id);
            if (line === undefined || !p.status.alive) continue;
            runLine(ctx, line);
          }
        },
      },
    ],
    participants: [
      {
        partyKind: FIRM,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          if (lineOf(rows, view.self.id) === undefined) return [];
          const own = view.lastOwn('firms.plan');
          // Clearing F1: an order is the decision it took this period, read back rather than taken
          // again. A firm that decided nothing this period posts nothing.
          if (!own.some || own.value.period !== view.period) return [];
          return ordersFrom(own.value, m.id, view.self.id);
        },
      },
    ],
    families: [productionCosts(rows)],
  };
}

/**
 * E1, E2, E6: the decision, taken once, published under the firm's own name, and read back by the
 * orders it posts and by the line it runs. A firm with nothing to decide on says nothing.
 */
function decide(ctx: MechanismContext, line: FirmDecl): void {
  const firm = line.firm as PartyId;
  const view = ctx.participant(firm);
  const decided = plan(view, line);
  if (!decided.some) return;
  const p = decided.value;
  if (!p.planned) {
    // Labour C5: it posts no opening at all, which is different from posting an empty one — an
    // empty opening is an employer that has decided it wants nobody. It still offers its stock.
    ctx.record('firms.plan', [firm], { planned: false, output: p.output, orders: orderData(p.orders) }, false);
    return;
  }
  const venue = venueOf(view, line);
  if (venue !== undefined) {
    // Labour D1, C5: a posting is the employment it wants, at the wage it offers. A firm whose
    // output no longer covers what it takes to make wants nobody, and what it posts is an empty
    // opening: there is no wage on hours nobody is offered, and the labour module reads the hours
    // it wants against the hours it has and sheds the difference at its own cost (Labour C3).
    const wantsNobody = p.hours <= 0;
    ctx.post(venue.id, {
      party: firm,
      side: 'buy',
      price: wantsNobody ? 0 : p.wageBid,
      qty: wantsNobody ? 0 : p.hours,
    });
  }
  // Private: what a firm is about to bid is between it and the book until the book clears. What it
  // expects to deliver is the public one (E7), and it is published separately.
  ctx.record(
    'firms.plan',
    [firm],
    {
      planned: true,
      output: p.output,
      expectedPrice: p.expectedPrice,
      unitCost: p.unitCost.some ? p.unitCost.value : null,
      batch: p.batch,
      bound: p.bound,
      hours: p.hours,
      wageBid: p.wageBid,
      orders: orderData(p.orders),
    },
    false,
  );
}

/** The orders as the data they are, so the party's own participant can read them back. */
function orderData(orders: readonly PlannedOrder[]): Record<string, unknown>[] {
  return orders.map((o) => ({ ...o }));
}
