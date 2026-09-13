/**
 * Getting older, and retiring: a cell moving from one key to another.
 *
 * @spec Households F1 Households F1.a Households F3 Labour B3 XI-15 Law 2 Law 6 Law 19
 *
 * A cohort is an age band and it is a key dimension of the cell (XI-15), so getting older is a cell
 * moving from one key to another — and a cell cannot half move. What happens is a SPLIT with a
 * different key on the part that crossed (`ctx.cells.reKey`): exact, per-member state and all,
 * totals preserved by construction. Nobody appears, nobody disappears, and NOTHING CROSSES — which
 * is the whole reason it is built this way, because a cell carries its holdings per member in whole
 * pieces and two cells of different weights have no quantity they could both denominate.
 *
 * HOW MANY CROSS IS NOT A RATE ANYBODY DECLARED (Law 2). A band spans the years between its own
 * entry age and the next one's, and a period is a known fraction of a year, so the share of it
 * standing at the boundary in any period is one over that span in periods. It is a SHAPE — it says
 * the ages inside a band are spread evenly, which is a claim about an answer — and its death is
 * written: a finer set of bands makes it finer, and an age carried per member would end it.
 *
 * RETIREMENT IS THE SAME EVENT. There is nothing else to it: the last band is the one the registry
 * puts past working age, labour reads the band to decide who is in the workforce (B3), and a cell
 * that crossed stops offering its hours because of what its key now says. No retirement mechanism
 * exists anywhere, and that is right — it is an age.
 *
 * WHAT IS NOT HERE IS DYING, and it is named rather than missing. What a dead cell held has to
 * reach somebody, and the survivors of its own key are the natural somebody — but the arithmetic
 * that would give it to them is the re-strike a weight event cannot do in whole pieces. The route
 * that IS exact runs through a NAMED estate (a named party holds totals, so a cell can pay one to
 * the piece), and that is 13d.1's remaining step rather than something to approximate here.
 */
import { period as periodOf } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import { div, mul, sub } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import { asQty } from '../../core/tick.js';
import { partyId, partyKindId, type PartyId, type RegionId } from '../../core/ids.js';
import type { Leg } from '../../ledger/instruction.js';
import type { InstrumentId } from '../../core/ids.js';
import type { PartyKindProfile } from '../../registry/kinds.js';
import { paramId, type ParamId } from '../../core/ids.js';
import { MORTALITY, type MortalityDecl } from './data.js';

/** F1.b: what this cohort's members die at, per period. One parameter per cohort (Law 2). */
export const mortalityParam = (cohort: string): ParamId =>
  paramId(`households.mortality.${cohort}`);

export function mortalityParams(rows: readonly MortalityDecl[] = MORTALITY): readonly {
  readonly id: ParamId;
  readonly value: number;
  readonly unit: string;
  readonly dimension: 'ratio';
  readonly kind: 'technology';
  readonly owner: 'model';
  readonly why: string;
}[] {
  return rows.map((r) => ({
    id: mortalityParam(r.cohort),
    value: r.perPeriod,
    unit: 'of the cohort per period',
    dimension: 'ratio' as const,
    kind: 'technology' as const,
    owner: 'model' as const,
    why: r.why,
  }));
}
import { keyOf, weightOf } from '../../parties/party.js';
import { HOUSEHOLD } from '../../registry/profiles.js';
import type { MechanismContext } from '../../world/context.js';

export const LIFECYCLE = 'households.lifecycle';

/**
 * F1, F1.a: THE CROSSING. One over the band's own span in periods, read off the registry's own age
 * boundaries and the calendar's own week. The last band has no exit and nobody crosses out of it.
 */
function crossingShare(ctx: MechanismContext, cohort: string): number | undefined {
  const cohorts = ctx.registry.cohorts;
  const at = cohorts.findIndex((c) => String(c.id) === cohort);
  const here = cohorts[at];
  const next = cohorts[at + 1];
  if (at < 0 || here === undefined || next === undefined) return undefined;
  const years = sub(next.fromAge, here.fromAge, 'the years this band spans');
  if (years <= 0) return undefined;
  // Law 2, Law 19: what a year is belongs to the day count and what a period is to the calendar.
  const ofAYear = yearFraction(
    'ACT/365F',
    ctx.calendar.startOf(ctx.period),
    ctx.calendar.startOf(periodOf(ctx.period + 1)),
  );
  if (ofAYear <= 0) return undefined;
  const periods = div(years, ofAYear, 'periods this band spans');
  if (periods <= 0) return undefined;
  return div(1, periods, 'the share of it standing at the boundary');
}

/** F1, F3: the people who crossed a band boundary this period become a cell with the next key. */
export function age(ctx: MechanismContext): void {
  const cohorts = ctx.registry.cohorts;
  for (const cell of [...ctx.parties.ofKind(HOUSEHOLD)]) {
    if (cell.representation !== 'cell' || !cell.status.alive) continue;
    const cohort = keyOf(cell, 'cohort');
    const at = cohorts.findIndex((c) => String(c.id) === cohort);
    const next = cohorts[at + 1];
    if (next === undefined) continue;
    const share = crossingShare(ctx, cohort);
    if (share === undefined) continue;
    // XI-15: a weight is a COUNT of people, so what crosses is whole people, and the fraction that
    // is not somebody stays where it is until enough of it has accumulated to be somebody.
    const crossing = Math.floor(mul(weightOf(cell), share, 'the people standing at the boundary'));
    if (crossing <= 0 || crossing >= weightOf(cell)) continue;
    const moved = ctx.cells.reKey(
      cell.id,
      crossing,
      { cohort: String(next.id) },
      `reached ${String(next.id)}`,
    );
    ctx.record(
      LIFECYCLE,
      [cell.id, moved],
      { event: 'aged', from: cohort, to: String(next.id), members: crossing },
      true,
    );
  }
}

/* ------------------------------------------------------------------------------------------------
 * DYING, AND WHERE WHAT THE DEAD HELD GOES (13d.1).
 *
 * @spec Households F1.b Households F2 Households F2.a XI-8 XI-15 Law 5 Law 8 Appendix B
 *
 * A cell cannot pay another cell. Both sides of a leg on a cell must be a whole number of pieces
 * for every member of it, and two cells of different weights have no such number — so an estate
 * cannot simply be handed from the dead to their heirs. What CAN take it to the piece is a NAMED
 * party, because a named party stands for one of itself and its side of a leg is the total.
 *
 * So there is PROBATE: one named party per place and bank, which is what probate IS — where what
 * somebody held waits until it can be divided. The dead split off with their own share of
 * everything (a split is exact), hand it to probate in full, and the cell ends holding nothing,
 * which is the only state a cell is allowed to end in (Appendix B: no death without a destination,
 * no residual with no holder). Probate then pays it out to the heirs in whole pieces for every one
 * of them, and keeps what will not divide until enough of it has accumulated to divide. The odd
 * penny an estate cannot split is a real thing and it is visible on a real book.
 * ---------------------------------------------------------------------------------------------- */

/** F2: the party where what the dead held waits. One per place and bank, and it never trades. */
export const PROBATE = partyKindId('probate');

export const probateId = (region: RegionId, bank: PartyId): PartyId =>
  partyId(`probate.${region}.${bank}`);

export const probateKind: PartyKindProfile = {
  id: PROBATE,
  representation: 'named',
  moneyIssuer: null,
  // It is not a firm and it cannot fail: it owes nobody. What it holds it owes to the living, and
  // that is not a liability anybody can call — it is an estate in the course of being divided.
  fails: [],
  borrows: false,
  // Banks Funding E1: it does not CHOOSE a bank and it never moves — it banks where the family it
  // is winding up banked, which is what its own id says. A depositor that shops around is a
  // depositor with a decision to take, and probate has none: it is an estate in the course of
  // being divided, and it is over in a period or two.
  depositClass: null,
};

/** The cell an estate goes to: the working people in the same place, banking where the family banks. */
function heirOf(ctx: MechanismContext, region: RegionId, bank: PartyId): PartyId | undefined {
  const first = String(ctx.registry.cohorts[0]?.id ?? '');
  return ctx.parties
    .ofKind(HOUSEHOLD)
    .find(
      (p) =>
        p.status.alive &&
        p.representation === 'cell' &&
        p.region === region &&
        p.bank === bank &&
        keyOf(p, 'cohort') === first,
    )?.id;
}


/** Money D2: money moves by a money leg. A deposit is a holding, and it is not moved as one. */
function isMoney(ctx: MechanismContext, instrument: InstrumentId): boolean {
  return ctx.registry.instrumentKind(ctx.instruments.get(instrument).kind).pricing === 'money';
}

/** Law 5, Law 8: everything this cell holds, to a NAMED party, in full and to the piece. */
function handToProbate(ctx: MechanismContext, from: PartyId, to: PartyId, region: RegionId): void {
  const view = ctx.participant(from);
  const weight = weightOf(ctx.parties.get(from));
  const ccy = ctx.registry.currencyOf(region);
  const legs: Leg[] = [];
  for (const h of view.holdings()) {
    if (isMoney(ctx, h.instrument)) continue;
    const perMember = view.free(h.instrument);
    if (perMember <= 0) continue;
    const print = view.print(h.instrument);
    legs.push({
      kind: 'asset',
      from,
      to,
      instrument: h.instrument,
      // XI-15: the register holds per MEMBER, and what moves is what all of them held between them.
      qty: asQty(mul(perMember, weight, 'what they held between them')),
      pricePerUnit: print.some ? some(print.value.price) : none(),
      accruedPerUnit: none(),
      fromCell: some({ perMember, weight }),
      toCell: none(),
    });
  }
  const cash = view.cash(ccy);
  if (cash > 0) {
    legs.push({
      kind: 'money',
      from: ctx.accountOf(from, ccy),
      to: ctx.accountOf(to, ccy),
      ccy,
      amount: asQty(mul(cash, weight, 'the money they had between them')),
      fromCell: some({ perMember: cash, weight }),
      toCell: none(),
    });
  }
  if (legs.length > 0) {
    ctx.settle({ legs, cause: 'corporateAction', reason: `the estate of ${from} goes to probate` });
  }
}

/** F1.b: the people who died this period, and everything they held, by name and to the piece. */
export function die(ctx: MechanismContext, rows: readonly MortalityDecl[]): void {
  for (const cell of [...ctx.parties.ofKind(HOUSEHOLD)]) {
    if (cell.representation !== 'cell' || !cell.status.alive) continue;
    const cohort = keyOf(cell, 'cohort');
    if (!rows.some((r) => r.cohort === cohort)) continue;
    const rate = ctx.params.ratio(mortalityParam(cohort));
    // XI-15: whole people. The fraction that is not somebody waits until it is.
    const dying = Math.floor(mul(weightOf(cell), rate, 'the people who die this period'));
    if (dying <= 0 || dying >= weightOf(cell)) continue;
    const office = probateId(cell.region, cell.bank);
    if (!ctx.parties.has(office)) continue;
    const estate = ctx.cells.split(cell.id, dying, 'died');
    handToProbate(ctx, estate, office, cell.region);
    ctx.cells.die(estate, office, 'died');
    ctx.record(LIFECYCLE, [estate, office], { event: 'died', cohort, members: dying }, true);
  }
}

/**
 * F2, F2.a: probate pays out. To each heir, a whole number of pieces for every one of its members,
 * and what will not divide stays here until enough of it has. Nothing is lost and nothing is
 * rounded away: the remainder is on a named book where a reader can see it.
 */
export function settleEstates(ctx: MechanismContext): void {
  for (const office of ctx.parties.ofKind(PROBATE)) {
    if (!office.status.alive) continue;
    const heir = heirOf(ctx, office.region, office.bank);
    if (heir === undefined) continue;
    const to = ctx.parties.get(heir);
    const weight = weightOf(to);
    if (weight <= 0) continue;
    const view = ctx.participant(office.id);
    const ccy = ctx.registry.currencyOf(office.region);
    const legs: Leg[] = [];
    for (const h of view.holdings()) {
      if (isMoney(ctx, h.instrument)) continue;
      const held = view.free(h.instrument);
      if (held <= 0) continue;
      const unit = ctx.instruments.get(h.instrument).unit;
      const perMember = ctx.registry.deliverable(unit, div(held, weight, 'each of them gets'));
      if (perMember <= 0) continue;
      const print = view.print(h.instrument);
      legs.push({
        kind: 'asset',
        from: office.id,
        to: heir,
        instrument: h.instrument,
        qty: asQty(mul(perMember, weight, 'what they get between them')),
        pricePerUnit: print.some ? some(print.value.price) : none(),
        accruedPerUnit: none(),
        fromCell: none(),
        toCell: some({ perMember, weight }),
      });
    }
    const cash = view.cash(ccy);
    if (cash > 0) {
      const perMember = ctx.registry.payable(ccy, div(cash, weight, 'each of them gets'));
      if (perMember > 0) {
        legs.push({
          kind: 'money',
          from: ctx.accountOf(office.id, ccy),
          to: ctx.accountOf(heir, ccy),
          ccy,
          amount: asQty(mul(perMember, weight, 'what they get between them')),
          fromCell: none(),
          toCell: some({ perMember, weight }),
        });
      }
    }
    if (legs.length === 0) continue;
    const r = ctx.settle({ legs, cause: 'corporateAction', reason: `${office.id} divides an estate` });
    ctx.record(
      LIFECYCLE,
      [office.id, heir],
      { event: 'divided', heir, lines: legs.length, settled: r.outcome === 'settled' },
      true,
    );
  }
}
