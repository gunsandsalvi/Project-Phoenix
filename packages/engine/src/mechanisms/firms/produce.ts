/**
 * The line: inputs consumed, a batch that carries what it cost, and what comes off it.
 *
 * @spec Firm B2 Firm B3 Firm B4 Goods B1.a Goods B1.b Goods B1.d Goods B2 Goods B3 Goods B4 Goods B5 Goods B5.a Goods B5.b Goods E1 Goods E5 Goods F5.a Goods F5.b Goods G4 Capital Programme A2 Capital Programme D4 Commodities Spot F1 Law 19
 *
 * PRODUCTION CONSUMES WHAT IT CONSUMES (B2): the recipe says how much, so the draw is not a second
 * decision — the firm chose the batch, and the tonnage follows. What it drew and what it made are
 * one instruction, and the goods module's own audit checks the draw against the recipe.
 *
 * A BATCH IS A THING (B3). It is created as units of work in progress carrying what it has cost —
 * the inputs at what they cost the firm (E1, E5) and the wages the period paid — and it sits on the
 * firm's own book until the lead time is up. That is why the balance sheet is true at every instant
 * rather than having a hole in it between the spending and the selling.
 *
 * WHAT COMES OFF (B4) is less than what went on: the units that did not make it are a loss of
 * units at the point they would have been made, and the whole batch's cost lands on the ones that
 * did, so a survivor is dearer than a unit started. Nothing is written down and nothing is written
 * off: the cost simply follows the units.
 *
 * NO UNITS, NO CAPITALISED COST (B5.a): a period that starts nothing capitalises nothing, and the
 * wage is then what it is — a period expense, which is what idle capacity costs (F5.a). A period
 * that starts a small batch puts the whole period's cost on it (B5.b), which is what running a line
 * below its rate does to unit cost. Either way the cost is in exactly one place (F5.b).
 */
import type { InstrumentId, PartyId } from '../../core/ids.js';
import { div, finite, material, mul, sub, sum } from '../../core/num.js';
import { asQty, upTick } from '../../core/tick.js';
import { none } from '../../core/option.js';
import type { Leg } from '../../ledger/instruction.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import { conditionsFor } from '../../registry/environment.js';
import { goodId, wipId } from '../../registry/physical.js';
import { costOfDraw, dueFromLine } from '../../register/register.js';
import { capacityFrom, utilisation, vintagesHeld } from '../../registry/physical.js';
import type { FirmDecl } from './data.js';
import { technologyOf } from './decide.js';

/** What this period's own wage bill came to for this firm, read from its own record (Law 19). */
function wagesThisPeriod(ctx: MechanismContext, firm: PartyId): number {
  const events = ctx.journal
    .ofKind('labour.wages')
    .filter((e) => e.period === ctx.period && e.subjects.includes(firm));
  const last = events[events.length - 1];
  if (last === undefined) return 0;
  const paid = last.data['paid'];
  return typeof paid === 'number' ? paid : 0;
}

/** The batch this firm said it would start, read back from its own published plan. */
function plannedBatch(view: ParticipantView): number {
  const own = view.lastOwn('firms.plan');
  if (!own.some || own.value.period !== view.period) return 0;
  const batch = own.value.data['batch'];
  return typeof batch === 'number' ? batch : 0;
}

/** Labour C2, Goods B1.c: the hours it has that can make something, this period. */
function productiveHours(ctx: MechanismContext, firm: PartyId): number {
  const events = ctx.journal
    .ofKind('labour.wages')
    .filter((e) => e.period === ctx.period && e.subjects.includes(firm));
  const last = events[events.length - 1];
  if (last === undefined) return 0;
  const hours = last.data['productive'];
  return typeof hours === 'number' ? hours : 0;
}

/** E1, E5: what the units this draw takes cost the firm, read off the lots they come out of. */
function heldCost(ctx: MechanismContext, firm: PartyId, instrument: InstrumentId, qty: number): number {
  const h = ctx.register.holding(firm, instrument);
  return h.some ? costOfDraw(h.value.lots, qty) : 0;
}

/** The whole line for one firm, in one period: what it starts, and what comes off it. */
export function runLine(ctx: MechanismContext, line: FirmDecl): void {
  const firm = line.firm as PartyId;
  if (!ctx.parties.has(firm) || !ctx.parties.get(firm).status.alive) return;
  const view = ctx.participant(firm);
  const tech = technologyOf(view, line);
  start(ctx, view, line, tech);
  yieldBatch(ctx, view, line, tech);
}

type Technology = ReturnType<typeof technologyOf>;

/** B1.b, B2, B3, B5: start what it planned, as far as its inputs and its people reach. */
function start(
  ctx: MechanismContext,
  view: ParticipantView,
  line: FirmDecl,
  tech: Technology,
): void {
  const firm = line.firm as PartyId;
  const planned = plannedBatch(view);
  const wages = wagesThisPeriod(ctx, firm);
  if (planned <= 0) return;
  // B1.b, B1.c: what it can actually make is the least of what it planned, the hours it has that
  // can make something, and what each of its inputs on hand reaches. The shortage is read here and
  // it binds, and which one bound it is recorded: a constraint nobody reads is not a constraint.
  // Goods B1.a, Capital Programme A2: and its plant, because output is limited by the stock. A
  // line whose recipe needs no plant is not limited by one, which is a different answer from being
  // limited by a large number (Law 6).
  const vintages = vintagesHeld(view, ctx.calendar.startOf(ctx.period));
  const capacity = capacityFrom(tech.plant, vintages);
  const limits: readonly { readonly qty: number; readonly bound: string }[] = [
    { qty: planned, bound: 'plan' },
    {
      qty: div(productiveHours(ctx, firm), tech.hoursPerUnit, 'what its people can make'),
      bound: 'labour',
    },
    ...(capacity.some ? [{ qty: capacity.value.perPeriod, bound: `capacity.${capacity.value.binding}` }] : []),
    ...tech.inputs.map((input) => ({
      // Law 8: it must draw WHOLE pieces of the input, and a recipe met with the piece below is a
      // recipe not met — so what its stock reaches is a piece short of what dividing would say.
      qty: div(
        sub(ctx.register.free(firm, input.instrument), 1, 'stock it can commit'),
        input.qtyPerUnit,
        'what the stock on hand reaches',
      ),
      bound: String(input.instrument),
    })),
  ];
  const binding = limits.reduce((a, b) => (b.qty < a.qty ? b : a));
  const wip = wipId(tech.terms.subUnit, tech.terms.region);
  // Law 8: what goes on the line is a whole number of the smallest piece of it.
  const batch = ctx.registry.deliverable(ctx.instruments.get(wip).unit, binding.qty);
  const bound = binding.bound;
  const room = capacity.some ? capacity.value.perPeriod : null;
  if (!material(batch, tech.inputs.length + tech.plant.length + 2, planned)) {
    // B5.a: it started nothing, so it capitalises nothing; the wage stands as a period expense.
    ctx.record('firms.idle', [firm], { planned, wages, bound, capacity: room, utilisation: 0 }, false);
    return;
  }
  const legs: Leg[] = [];
  const costs: number[] = [wages];
  for (const input of tech.inputs) {
    const qty = upTick(mul(batch, input.qtyPerUnit, 'what the recipe draws'));
    costs.push(heldCost(ctx, firm, input.instrument, qty));
    legs.push({
      kind: 'destroy',
      party: firm,
      instrument: input.instrument,
      qty,
      why: 'consumed',
      fromCell: none(),
    });
  }
  const cost = sum(costs);
  legs.push({
    kind: 'create',
    party: firm,
    instrument: wip,
    qty: batch,
    // B5, B5.b: the inputs it drew plus what the period's labour cost, over the batch it started.
    costPerUnit: div(cost.value, batch, 'what a unit on the line has cost'),
    toCell: none(),
  });
  const record = ctx.settle({
    legs,
    cause: 'production',
    reason: `${firm} started ${batch} of ${tech.terms.subUnit}`,
  });
  const started = record.outcome === 'settled' ? batch : 0;
  const used = room === null ? none<number>() : utilisation(started, room);
  ctx.record(
    'firms.started',
    [firm],
    {
      planned,
      started,
      bound,
      wages,
      cost: cost.value,
      settled: record.outcome === 'settled',
      // Goods B1.d, D4, Goods G4: utilisation is a READ of the outcome against capacity, taken
      // here because here is where the outcome is. Nothing decided anything with it.
      capacity: room,
      utilisation: used.some ? used.value : null,
    },
    false,
  );
}

/** B3, B4: what the lead time says is due comes off the line, and the scrap is units. */
function yieldBatch(
  ctx: MechanismContext,
  view: ParticipantView,
  line: FirmDecl,
  tech: Technology,
): void {
  const firm = line.firm as PartyId;
  const wip = wipId(tech.terms.subUnit, tech.terms.region);
  const holding = ctx.register.holding(firm, wip);
  if (!holding.some) return;
  // Nothing can have been started before the world had periods, so a line whose lead time is
  // longer than the world is old has nothing off it yet — and a batch the seed put on the line
  // came off at period zero plus its lead time, like any other (Goods B3).
  const startedBy = sub(view.period, tech.leadTime, 'started by');
  const due = dueFromLine(holding.value.lots, startedBy);
  if (!material(due, holding.value.lots.length + 1, due)) return;
  const cost = costOfDraw(holding.value.lots, due);
  // Law 8, Goods B4: what comes off the line is a whole number of the smallest piece of the good.
  // The yield takes a batch to a quantity between two pieces more often than not, and what exists
  // is the piece below — a part-finished unit is scrap, not stock.
  const good = goodId(tech.terms.subUnit, tech.terms.region);
  // B4, Commodities Spot B3: THE SEASON THIS BATCH STOOD IN, read off the environment's public
  // event (`registry/environment.ts`). A line exposed to nothing, and a world with no environment
  // in it, stand at one — and neither is a default: the first is a line made indoors and the second
  // is a scale model with no weather to stand in.
  //
  // B4 IS ONE-DIRECTIONAL — "not everything started is finished" — so a good season cannot make
  // more tonnes than went onto the line, and the season moves the SURVIVAL RATE rather than
  // multiplying the batch. `yield ^ (1 / season)` is that: a fraction between nothing and all of it
  // raised to a positive power stays between nothing and all of it, so the result is inside its own
  // range BY ARITHMETIC and not by a clamp (Law 6). A halved season is adversity applied twice over
  // and takes much more than half again; a season twice as kind takes the loss towards none without
  // ever reaching past it. Nothing anywhere is capped, floored or rescaled.
  //
  // And it moves a QUANTITY at the point the thing is made, which is where the loss actually is.
  // Nothing here touches a price: a shortage reaches one by there being less of the thing (Law 3).
  const season = conditionsFor(ctx, tech.terms.region, tech.terms.recipe.exposedTo);
  const survived = finite(Math.pow(tech.yieldRate, 1 / season), 'what this season left of the line');
  const finished = ctx.registry.deliverable(
    ctx.instruments.get(good).unit,
    mul(due, survived, 'what came off the line'),
  );
  if (!material(finished, 2, due) || finished <= 0) return;
  const record = ctx.settle({
    legs: [
      { kind: 'destroy', party: firm, instrument: wip, qty: asQty(due, 'the batch that came off the line'), why: 'consumed', fromCell: none() },
      {
        kind: 'create',
        party: firm,
        instrument: good,
        qty: finished,
        // B4: the whole batch's cost over the units that survived it, so a survivor is dearer.
        costPerUnit: div(cost, finished, 'what a finished unit cost'),
        toCell: none(),
      },
    ],
    cause: 'production',
    reason: `${firm} finished ${finished} of ${tech.terms.subUnit}`,
  });
  if (record.outcome !== 'settled') return;
  ctx.record(
    'firms.produced',
    [firm],
    {
      good: tech.terms.subUnit,
      started: due,
      finished,
      // B4: units, at the point they would have been made. Not a rate and not a write-down.
      scrapped: sub(due, finished, 'scrap'),
      // B3, B4: what the season did and what it left, published so a shortfall has a cause
      // anybody can read — and so that the ordinary yield beside it says how much of the gap is
      // the weather and how much is the line.
      season,
      survived,
      costPerUnit: div(cost, finished, 'what a finished unit cost'),
    },
    false,
  );
}

/** E7, §46 C2: what the management expects to deliver, published, and then judged against. */
export function publishExpectation(ctx: MechanismContext, firm: PartyId): void {
  const outlook = ctx.participant(firm).outlook('earnings');
  if (!outlook.some) return;
  ctx.record(
    'firms.expectation',
    [firm],
    {
      // Its own adaptive read of its own earnings, in its own money, per period (A5).
      earnings: outlook.value.expected,
      unit: outlook.value.unit,
      per: outlook.value.per,
      // B3: how wide its own recent surprises have been. A read, never a stated number.
      confidence: outlook.value.confidence,
      formed: outlook.value.formed,
    },
    true,
  );
}
