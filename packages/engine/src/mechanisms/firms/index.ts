/**
 * Firms: named parties that make a thing out of other things and the hours of named people, sell it
 * to named buyers, and keep whatever is left.
 *
 * @spec Firm A1 Firm A2 Firm A3 Firm E4 Firm E4.a Firm E5 Firm B1 Firm B2 Firm B3 Firm B4 Firm B4.a Firm B4.b Firm B5 Firm B6 Firm C1 Firm C4 Firm D1 Firm E1 Firm E2 Firm E3 Firm E6 Firm E7 Firm F1 Firm F2 Firm F3 Goods B1 Goods B1.a Goods B1.b Goods B1.c Goods B1.d Goods B2 Goods B3 Goods B4 Goods B5 Goods B5.a Goods B5.b Goods C1 Goods C3 Goods C5 Goods F5 Goods F5.a Goods F5.b Labour C1 Labour C1.a Labour C5 Labour D1 Capital Programme B1 Capital Programme B2 Capital Programme B3 Capital Programme B4 Capital Programme C1 Capital Programme C2 Expectations C2 XI-4 XI-16 Law 2 Law 6 Law 15
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
import type { MarketId, PartyId } from '../../core/ids.js';
import { add, combineDust, dustOf, mul, sub, sum, withinDust } from '../../core/num.js';
import { isCreateLeg } from '../../ledger/instruction.js';
import { FIRM } from '../../registry/profiles.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { firmChoosesBank, FIRM_SWITCHING_COST } from './bank.js';
import { firmParam, labourScaleId, type FirmDecl } from './data.js';
import { marketsIn, ordersFrom, plan, venueOf, type Planned, type PlannedOrder } from './decide.js';
import { publishExpectation, runLine } from './produce.js';
import { NO_QTY } from '../../core/tick.js';

export * from './data.js';
export { firmChoosesBank, FIRM_SWITCHING_COST } from './bank.js';
export { plan, technologyOf, expectedPrice } from './decide.js';
export type { Offering, Plan, Planned, PlannedOrder } from './decide.js';

/**
 * The line a named firm is in, if this world put it in one (Law 15: the data says).
 *
 * Law 18: BY NAME, not by walking the list. Every market asks every firm whether it has an order in
 * it, so this is asked once per firm per market — three thousand firms and two hundred and sixty
 * markets is three quarters of a million times a period, and a scan of the rows made it two
 * billion comparisons. The index is the same rows under a different arrangement, built once with
 * the module, so there is nothing to go stale (Law 4).
 */
function indexOf(rows: readonly FirmDecl[]): ReadonlyMap<string, FirmDecl> {
  return new Map(rows.map((r) => [r.firm, r]));
}

function lineOf(rows: ReadonlyMap<string, FirmDecl>, firm: PartyId): FirmDecl | undefined {
  return rows.get(String(firm));
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
function productionCosts(byName: ReadonlyMap<string, FirmDecl>): Family {
  return {
    name: 'flows',
    contributor: 'firms',
    spec: 'Firm B4.b Firm B6 Goods B5 Goods F5.b',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const moved = new Map<PartyId, number[]>();
      // Law 7: the dust of this comparison is the dust of the arithmetic that produced it — a walk
      // over the lots each leg drew from, inside settlement, and then a sum over the legs. What is
      // visible here is the legs and what they cost, and that is what the tolerance is derived from.
      const walked = new Map<PartyId, { terms: number; magnitude: number }>();
      for (const r of view.ledger.inPeriod(view.period)) {
        if (r.outcome !== 'settled' || r.instruction.cause !== 'production') continue;
        if (!r.instruction.legs.some(isCreateLeg)) continue;
        for (const e of r.equity) {
          if (lineOf(byName, e.party) === undefined) continue;
          const list = moved.get(e.party) ?? [];
          list.push(e.delta);
          moved.set(e.party, list);
          const walk = walked.get(e.party) ?? { terms: 0, magnitude: 0 };
          // Law 7: what the arithmetic passed THROUGH, which is rarely what it came to. A batch off
          // the line destroys units carried at one value and creates units carried at the same
          // value, so the equity effect is nearly nothing and the magnitude behind it is the whole
          // batch — read off the create legs, which say what a unit cost (Goods E1).
          const carried = sum(
            r.instruction.legs.map((l) => (isCreateLeg(l) ? Math.abs(mul(l.qty, l.costPerUnit, 'batch value')) : 0)),
          ).value;
          walked.set(e.party, {
            terms: walk.terms + r.instruction.legs.length + r.deltas.length,
            magnitude:
              walk.magnitude +
              Math.abs(e.delta) +
              carried +
              sum(r.deltas.map((d) => Math.abs(d.qty))).value,
          });
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
        const walk = walked.get(firm) ?? { terms: 0, magnitude: 0 };
        const dust =
          combineDust(effect, wages) +
          dustOf(walk.terms, walk.magnitude + Math.abs(wages.value));
        if (withinDust(effect.value, wages.value, dust)) continue;
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
export function firms(rows: readonly FirmDecl[]): SystemModule {
  const byName = indexOf(rows);
  return {
    id: 'firms',
    spec: 'Firm, Goods B, Goods F',
    // It reads the recipe from the good's terms, posts openings into the labour venue, and decides
    // from its own outlook: all three must exist before it does (Part XIII step 4). And it decides
    // what to invest, which is measured against the plant it holds — so the kind of thing plant is
    // has to be registered before a firm can be asked what it has (Capital Programme A2).
    requires: ['expectations', 'goods', 'labour', 'capital-programme'],
    instrumentKinds: [],
    // ARCHITECTURE 4.9b: A KIND IS OWNED BY THE MODULE THAT OWNS ITS BEHAVIOUR. Every field here is
    // this module's subject — how a firm fails, that it borrows, what kind of depositor it is — so
    // a kernel that declared them was answering for a system it does not implement, and the guard
    // that asks a module how its depositors leave (Banks Funding A1.d, E1) could say nothing about
    // the largest class of them. The ID stays in the kernel's registry, because `labour`, `equity`,
    // `ratings` and `treasury` all have to NAME a firm and naming it from here would be a
    // cross-module import (worklist 11.6).
    //
    // XI-3, Firm D4: it can fail two ways and they are different — no cash to pay something due, or
    // liabilities exceeding assets. Both, because a firm can be either without the other.
    // Banks Funding A1.b: fewer, larger, operational — a firm banks where it transacts.
    partyKinds: [
      {
        id: FIRM,
        representation: 'named',
        moneyIssuer: null,
        fails: ['cash', 'solvency'],
        borrows: true,
        depositClass: 'corporate',
      },
    ],
    curveFamilies: [],
    units: [],
    // Law 2: one number per firm and nothing else. What a thing is made out of, how long it takes
    // and what survives the line are the GOOD's technology, shared by everyone in the line; what an
    // hour costs is what the market charged it; and there is no target margin, no buffer and no
    // adjustment speed anywhere in it. What is declared here is Firm A3's dispersion: how many hours
    // a tonne takes THIS firm, against the hours the trade takes.
    params: [
      {
        id: FIRM_SWITCHING_COST,
        value: 250,
        denominated: true as const,
        unit: 'of the money the account is in, per move',
        dimension: 'amount',
        kind: 'preference' as const,
        owner: 'model' as const,
        why: 'Banks Funding A1.b, A1.d, E1: what it costs a firm to move the account it transacts through — the payments to redirect, the counterparties to tell. It is weighed against the money it would lose if its bank failed, which for an uninsured corporate balance is the whole of it, so a firm with its float at a bank that drew the window goes and one with little there stays. It is larger than a household\'s because an operational account is entangled with everything the firm does, and smaller than a fund\'s because a fund moves far more money at once.',
      },
      ...rows.flatMap((r) => [
      {
        id: labourScaleId(r.firm),
        value: r.labourScale,
        unit: 'ratio of the hours the recipe names',
        dimension: 'ratio' as const,
        kind: 'technology' as const,
        owner: 'model' as const,
        why: `Firm A3: ${r.why}`,
      },
      {
        id: firmParam(r.firm, 'hurdle'),
        value: r.hurdle,
        unit: 'per annum over its cost of capital',
        dimension: 'perAnnum' as const,
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Capital Programme B1.d, XI-4: the margin ${r.firm}'s management insists on before it commits money it cannot get back. It is the management's own risk aversion and it is why two firms facing the same quote do not take the same project.`,
      },
      {
        id: firmParam(r.firm, 'horizon'),
        value: r.horizonPeriods,
        unit: 'periods of service it counts',
        dimension: 'periods' as const,
        kind: 'preference' as const,
        owner: 'model' as const,
        why: `Capital Programme B1.d: how far ahead ${r.firm}'s management looks. It is its patience, and a short one values a machine at what the years it will look at are worth rather than at what the machine will give.`,
      },
      ]),
    ],
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
            const line = lineOf(byName, p.id);
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
            const line = lineOf(byName, p.id);
            if (line === undefined || !p.status.alive) continue;
            runLine(ctx, line);
          }
        },
      },
    ],
    participants: [
      {
        partyKind: FIRM,
        // Law 18: the books it could be in, off the plan its orders come off. At the real scale
        // there are three thousand firms and 261 markets, and a firm is in two or three of them.
        markets: (view: ParticipantView): readonly MarketId[] => {
          if (lineOf(byName, view.self.id) === undefined) return [];
          const own = view.lastOwn('firms.plan');
          if (!own.some || own.value.period !== view.period) return [];
          return marketsIn(own.value);
        },
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          if (lineOf(byName, view.self.id) === undefined) return [];
          const own = view.lastOwn('firms.plan');
          // Clearing F1: an order is the decision it took this period, read back rather than taken
          // again. A firm that decided nothing this period posts nothing.
          if (!own.some || own.value.period !== view.period) return [];
          return ordersFrom(own.value, m.id, view.self.id);
        },
      },
    ],
    families: [productionCosts(byName)],
    bankChoices: [{ partyKind: FIRM, chooses: firmChoosesBank }],
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
    ctx.record(
      'firms.plan',
      [firm],
      { planned: false, output: p.output, carry: p.carry, orders: orderData(p.orders) },
      false,
    );
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
      qty: wantsNobody ? NO_QTY : p.hours,
    });
  }
  publishFunding(ctx, view, p);
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
      // Capital Programme A2, D4, Goods B1.d: what its plant lets it make, what that plant costs it
      // to use, and what it decided to do about the gap. Utilisation is a READ of what it produced
      // against this, taken where the outcome is (produce.ts), never an input to the decision.
      capacity: p.capacity,
      capacityNext: p.capacityNext,
      runRate: p.runRate,
      capitalCharge: p.capitalCharge,
      // B4: what it is sure enough of to build for — its run rate less the width of its own recent
      // surprises. The gap is measured from THIS, which is the option to wait doing its work.
      cautiousRunRate: p.project === null ? null : p.project.cautiousRunRate,
      costOfCapital: p.costOfCapital === null ? null : p.costOfCapital.perAnnum,
      costOfDebt: p.costOfCapital?.debt.some === true ? p.costOfCapital.debt.value : null,
      costOfEquity: p.costOfCapital?.equity.some === true ? p.costOfCapital.equity.value : null,
      investmentGap: p.project === null ? 0 : p.project.gap,
      investmentSpend: p.project === null ? 0 : p.project.funded,
      programme: p.project === null ? 0 : p.project.programme,
      hours: p.hours,
      wageBid: p.wageBid,
      carry: p.carry,
      orders: orderData(p.orders),
    },
    false,
  );
}

/**
 * Firm E4, E5, Banks Lending C2: what it is short of, or what it has spare. A firm pays out of a
 * balance and the balance can hit zero (D1), so what it is about to have to pay — the payroll it
 * has promised and the inputs it decided to buy — against what it holds is a real number about its
 * own position, and it says it. A NEGATIVE short is what it has over, and it is the same number
 * read the other way: one fact, one writer, whichever side of zero it falls (Law 4).
 *
 * It says it and nothing more. Whether anybody lends against it, at what, whether it takes the
 * quote, and whether what it has over goes to its owners are decisions in the modules that own
 * them (Banks Lending C2, Equity D2.c).
 */
function publishFunding(ctx: MechanismContext, view: ParticipantView, p: Planned): void {
  const ccy = ctx.registry.region(view.self.region).ccy;
  const buying = sum(
    p.orders
      .filter((o: PlannedOrder) => o.side === 'buy' && o.price !== 'market')
      .map((o: PlannedOrder) => (typeof o.price === 'number' ? o.price * o.qty : 0)),
  ).value;
  // Firm E4.a, Capital Programme B2: the money raised is raised INTO AN ACTUAL INVESTMENT
  // PROGRAMME. What it wants to spend on plant and cannot pay for out of what it holds is part of
  // what it is short of, so a bank lends against a programme and a share issue is raised into one —
  // and a firm with no programme is short of nothing on that account and raises nothing.
  const programme = p.project === null ? 0 : p.project.programme;
  const owed = add(
    add(buying, wagesPromised(view), 'what it is about to have to pay'),
    programme,
    'and what it wants to build',
  );
  const short = sub(owed, view.cash(ccy), 'what it is short of');
  ctx.record('firms.funding', [view.self.id], { short, owed, programme, ccy }, false);
}

/** D1: the payroll it has already promised, read from its own last wage bill (Law 19). */
function wagesPromised(view: ParticipantView): number {
  const own = view.lastOwn('labour.wages');
  if (!own.some) return 0;
  const due = own.value.data['due'];
  return typeof due === 'number' ? due : 0;
}

/** The orders as the data they are, so the party's own participant can read them back. */
function orderData(orders: readonly PlannedOrder[]): Record<string, unknown>[] {
  return orders.map((o) => ({ ...o }));
}
