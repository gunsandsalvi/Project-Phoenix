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
 * AND DYING IS HERE, which this header used to say it was not. 13d.1 closed and the comment did
 * not move, so it told a reader the opposite of what the file does (A-22, Law 16). What a dead cell
 * held has to reach somebody, and the arithmetic that would hand it straight to the survivors of
 * its own key is a re-strike a weight event cannot do in whole pieces — so the route that IS exact
 * runs through a NAMED estate, because a named party holds totals and a cell can pay one to the
 * piece. `PROBATE`, `handToProbate`, `die`, `heirsOf` and `settleEstates` below are that route.
 */
import { period as periodOf } from '../../calendar/calendar.js';
import { yearFraction } from '../../calendar/daycount.js';
import { assertNever } from '../../core/assert.js';
import { add, div, mul, sub } from '../../core/num.js';
import { asRatio, heldAsMoney, over, scale } from '../../core/measure.js';
import { none, some } from '../../core/option.js';
import { asQty, scaleQty } from '../../core/tick.js';
import { partyId, partyKindId, type CurrencyCode, type PartyId, type RegionId } from '../../core/ids.js';
import type { FailReason, Leg, Unpaid } from '../../ledger/instruction.js';
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
  /** item 15: a DUTY, and duties are not interests — it has no residual and nobody to enrich. */
  objective: 'itsOffice',
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

/**
 * A-21, F2, Law 2: WHO INHERITS. The survivors where the dead lived and banked, in proportion to
 * how many people each cell stands for.
 *
 * IT USED TO BE `.find` — the FIRST cell the parties store happened to return, of the first cohort.
 * Every estate in a (region, bank) went to that one cell and to no other, so one household cell in
 * each region accumulated the wealth of everybody who died there and the rest inherited nothing,
 * ever. Which cell it was depended on the parties store's insertion order, which is a seed-draw
 * artefact and not a fact about the world.
 *
 * Two things were wrong with it and both are gone. The distribution was an OUTCOME written as a
 * lookup (Law 2), and it was a decision taken at no party's own reason — nobody chose an heir,
 * nobody had a claim, and there was no mechanism behind it. What replaces it is not a choice
 * either: it is a SHARE, and the share is the population. Nobody is picked, and a region whose
 * cells are all the same size divides equally because that is what its cells are, not because a
 * rule says to.
 *
 * `handToProbate`'s own header says the survivors of the dead cell's own key are the natural
 * somebody. The office pools the estates of every cohort that banks there, so by the time it pays
 * it cannot say whose is whose — which is a real limitation of pooling and is why this is every
 * cell in the (region, bank) rather than the dead's own cohort.
 */
function heirsOf(
  ctx: MechanismContext,
  region: RegionId,
  bank: PartyId,
): readonly { readonly id: PartyId; readonly weight: number }[] {
  const out: { id: PartyId; weight: number }[] = [];
  for (const p of ctx.parties.ofKind(HOUSEHOLD)) {
    if (!p.status.alive || p.representation !== 'cell') continue;
    if (p.region !== region || p.bank !== bank) continue;
    const weight = weightOf(p);
    if (weight <= 0) continue;
    out.push({ id: p.id, weight });
  }
  return out;
}


/** Money D2: money moves by a money leg. A deposit is a holding, and it is not moved as one. */
function isMoney(ctx: MechanismContext, instrument: InstrumentId): boolean {
  return ctx.registry.instrumentKind(ctx.instruments.get(instrument).kind).pricing === 'money';
}

/**
 * Law 5, Law 8: everything this cell holds, to a NAMED party, in full and to the piece.
 *
 * 13j: EVERY MONEY IT HELD, and not only its own. A world with four of them has households paid a
 * coupon in one they do not bank in (Currency C4), and this used to move the cash of the cell's own
 * region and leave the rest — so a cell that had ever been paid abroad could not die, because what
 * the dead held has to go to somebody by name FIRST (Appendix B) and some of it was still there.
 */
function handToProbate(
  ctx: MechanismContext,
  from: PartyId,
  to: PartyId,
  region: RegionId,
): readonly Unpaid[] {
  const view = ctx.participant(from);
  const weight = weightOf(ctx.parties.get(from));
  const legs: Leg[] = [];
  const monies = new Set<CurrencyCode>([ctx.registry.currencyOf(region)]);
  for (const h of view.holdings()) {
    if (isMoney(ctx, h.instrument)) {
      monies.add(ctx.instruments.get(h.instrument).ccy);
      continue;
    }
    const perMember = view.free(h.instrument);
    if (perMember <= 0) continue;
    const print = view.print(h.instrument);
    legs.push({
      kind: 'asset',
      from,
      to,
      instrument: h.instrument,
      // XI-15: the register holds per MEMBER, and what moves is what all of them held between them.
      qty: scaleQty(perMember, weight, 'what they held between them'),
      pricePerUnit: print.some ? some(print.value.price) : none(),
      accruedPerUnit: none(),
      fromCell: some({ perMember, weight }),
      toCell: none(),
    });
  }
  const cash: { readonly ccy: CurrencyCode; readonly amount: number }[] = [];
  for (const ccy of monies) {
    const perMember = view.cash(ccy);
    if (perMember <= 0) continue;
    const amount = scaleQty(perMember, weight, 'the money they had between them');
    cash.push({ ccy, amount });
    legs.push({
      kind: 'money',
      from: ctx.accountOf(from, ccy),
      to: ctx.accountOf(to, ccy),
      ccy,
      amount: asQty(amount),
      fromCell: some({ perMember, weight }),
      toCell: none(),
    });
  }
  if (legs.length === 0) return [];
  const r = ctx.settle({ legs, cause: 'corporateAction', reason: `the estate of ${from} goes to probate` });
  /**
   * A-20, Money E1, Register C3.b: A FAIL IS A RECORDED STATE, AND THE MODULE HAS TO READ IT.
   *
   * This call discarded what `settle` returned, and settlement is atomic — so on a fail NOTHING
   * moved and the estate still held everything, which then met `dieCell`'s "no death without a
   * destination" and threw a `PhoenixError` the engine never catches. A failed estate transfer
   * STOPPED THE WORLD. What it actually is is an outcome: the money did not arrive, so it is still
   * owed by the estate to the office (D3), and the reader below decides what to do about it.
   */
  if (r.outcome === 'settled') return [];
  const why = failedBecause(r.reason);
  return cash.map((c) => ({ payer: from, payee: to, amount: c.amount, ccy: c.ccy, reason: why }));
}

/** Law 16: the three reasons a settlement fails, in words, for the agreement that records one. */
function failedBecause(reason: FailReason): string {
  // eslint-disable-next-line phoenix/no-kind-branch -- a fail reason's tag, not a party or product kind
  switch (reason.kind) {
    case 'insufficientUnits':
      return `${reason.party} is ${reason.short} short of ${reason.instrument}`;
    case 'overdraftRefused':
      return `${reason.issuer} refused ${reason.party} an overdraft of ${reason.short} ${reason.ccy}`;
    case 'insufficientCollateral':
      return `${reason.party} has ${reason.short} too little free ${reason.instrument} to pledge`;
    default:
      return assertNever(reason, 'a settlement fail reason');
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
    const short = handToProbate(ctx, estate, office, cell.region);
    for (const u of short) {
      // XI-8: what did not arrive is still owed, by the estate, to the office that was to receive
      // it. Before this there was nowhere for it to be, so the alternative was the throw above.
      ctx.owes({
        debtor: estate,
        creditor: office,
        ccy: u.ccy,
        owed: u.amount,
        what: 'an estate not yet handed to probate',
        why: `the transfer failed: ${u.reason}`,
      });
    }
    /**
     * Appendix B, XI-3: A CELL MAY ONLY DIE EMPTY, and the estate is not empty when the transfer
     * failed or when what it held was ENCUMBERED — a lien is not free, the asset legs are built
     * from what is free, and the units stay. Both used to reach `dieCell` and throw.
     *
     * It does not die, then. It is `winding` (§25 C1): every member it stands for is dead and it
     * still holds something, which is exactly the state between alive and gone. What is still on
     * its book is on its book by name, so nothing is a residual with no holder — and the estate is
     * a party in an open probate, which is what one is in the world this reflects (Law 1).
     */
    const empty = ctx.register.holdingsOf(estate).length === 0;
    if (empty) ctx.cells.die(estate, office, 'died');
    else ctx.standing(estate, 'winding', 'died holding what could not be handed to probate');
    ctx.record(
      LIFECYCLE,
      [estate, office],
      { event: 'died', cohort, members: dying, handedOver: empty },
      true,
    );
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
    /**
     * A-21, item 14: EVERY SURVIVOR WHERE THE DEAD LIVED AND BANKED, in proportion to how many
     * people each cell stands for — and the proportion is the population, not a rule.
     */
    const heirs = heirsOf(ctx, office.region, office.bank);
    const people = heirs.reduce((t, h) => add(t, h.weight, 'the survivors here'), 0);
    if (people <= 0) continue;
    const view = ctx.participant(office.id);
    const ccy = ctx.registry.currencyOf(office.region);
    for (const heir of heirs) {
      const legs: Leg[] = [];
      const share = asRatio(div(heir.weight, people, 'this cell share of what is here'), 'this cell share');
      for (const h of view.holdings()) {
        if (isMoney(ctx, h.instrument)) continue;
        const held = view.free(h.instrument);
        if (held <= 0) continue;
        const unit = ctx.instruments.get(h.instrument).unit;
        /**
         * Law 8, XI-15: WHOLE PIECES, to each member of this cell. What will not divide stays with
         * the office and is divided when enough of it has arrived — nothing is rounded away, which
         * is the same rule the office already applied to one heir and now applies to all of them.
         */
        const perMember = ctx.registry.deliverable(
          unit,
          over(
            scale(held, share, 'this cell share'),
            asRatio(heir.weight, 'the members it has'),
            'each of them gets',
          ),
        );
        if (perMember <= 0) continue;
        const print = view.print(h.instrument);
        legs.push({
          kind: 'asset',
          from: office.id,
          to: heir.id,
          instrument: h.instrument,
          qty: scaleQty(perMember, heir.weight, 'what they get between them'),
          pricePerUnit: print.some ? some(print.value.price) : none(),
          accruedPerUnit: none(),
          fromCell: none(),
          toCell: some({ perMember, weight: heir.weight }),
        });
      }
      const cash = view.cash(ccy);
      if (cash > 0) {
        const perMember = ctx.registry.payable(
          ccy,
          over(
            scale(heldAsMoney(cash, 'what the estate holds'), share, 'this cell share'),
            asRatio(heir.weight, 'the members it has'),
            'each of them gets',
          ),
        );
        if (perMember > 0) {
          legs.push({
            kind: 'money',
            from: ctx.accountOf(office.id, ccy),
            to: ctx.accountOf(heir.id, ccy),
            // Treasury C1, XI-8: an inheritance is a TRANSFER of what somebody already owned, not
            // something the heir earned. It was taxed as income at the wage rate (A-46).
            receipt: { of: 'transfer' },
            ccy,
            amount: scaleQty(perMember, heir.weight, 'what they get between them'),
            fromCell: none(),
            toCell: some({ perMember, weight: heir.weight }),
          });
        }
      }
      if (legs.length === 0) continue;
      const r = ctx.settle({
        legs,
        cause: 'corporateAction',
        reason: `${office.id} divides an estate`,
      });
      ctx.record(
        LIFECYCLE,
        [office.id, heir.id],
        {
          event: 'divided',
          heir: heir.id,
          of: people,
          members: heir.weight,
          lines: legs.length,
          settled: r.outcome === 'settled',
        },
        true,
      );
    }
  }
}
