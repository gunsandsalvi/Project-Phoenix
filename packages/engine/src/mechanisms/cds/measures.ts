/**
 * What a credit default swap book's level says against the rest of the world.
 *
 * @spec CDS A4 CDS A4.a CDS C3 CDS C3.a CDS C3.b CDS E3 Observer A1 Law 3 Law 19
 *
 * Two standing measurements, both reads and neither a target: how much protection on one name
 * exists at all, and what that protection costs against what the same name's cash bond is paying.
 * They live beside the class because the class is what knows them (Law 15) — the surface that shows
 * them asks the kind, and a world with one more class gets one more measurement without the surface
 * being touched.
 */
import { negQty } from '../../core/tick.js';
import { asRatio, minus } from '../../core/measure.js';
import { NO_QTY, subQty } from '../../core/tick.js';
import type { InstrumentId, PartyId } from '../../core/ids.js';
import { none, some, type Option } from '../../core/option.js';
import { issuedBy } from '../../register/instruments.js';
import { curveFamilyOf, yieldOf } from '../../prices/curve.js';
import { contractOf, type MarketDecl } from '../../clearing/market.js';
import type { ContractMeasure } from '../../registry/derivatives.js';
import type { ParticipantView, WorldReads } from '../../world/context.js';
import type { Calendar, Period } from '../../calendar/calendar.js';
import type { Registry } from '../../registry/registry.js';
import type { Instrument } from '../../register/instruments.js';
import type { Print } from '../../prices/price-store.js';
import type { CurveRead } from '../../prices/curve.js';
import type { CurrencyCode, CurveFamilyId } from '../../core/ids.js';
import type { Ratio } from '../../core/measure.js';
import { CDS_DAY_COUNT, cdsLineOf } from './data.js';
import { isCds } from './contract.js';

/** A4, A4.a: a reference is a party somebody can watch fail — one with debt that can default. */
export function defaultableDebtOf(ctx: WorldReads, party: PartyId): Option<InstrumentId> {
  for (const i of ctx.instruments.all()) {
    if (!i.status.live) continue;
    if (!issuedBy(i, party)) continue;
    if (!ctx.registry.instrumentKind(i.kind).liabilityOfIssuer) continue;
    if (ctx.registry.instrumentKind(i.kind).pricing !== 'cleared') continue;
    return some(i.id);
  }
  return none<InstrumentId>();
}

/**
 * E3: NET NOTIONAL PER REFERENCE — how much protection on one name actually exists, netted the only
 * way E3 allows it to be: across the contracts on that reference, which is one question about one
 * name and not a netting across counterparties (G3).
 */
export function netNotionalOn(ctx: WorldReads, reference: PartyId): number {
  let net = NO_QTY;
  for (const c of ctx.contracts.open_()) {
    if (!isCds(c.terms)) continue;
    if (c.terms.reference !== reference) continue;
    // C2: a cleared trade is two rows against the house, so counting both would count the
    // protection twice. The member's side is the one that exists in the world.
    if (c.house !== null && c.a === c.house) continue;
    net = subQty(net, negQty(c.notional, 'the other side of it'), 'protection written on this name');
  }
  return net;
}

/**
 * C3, Law 4, XI-13 (18.5): WHAT THE CASH MARKET CHARGES FOR THIS CREDIT — the reference's own bond
 * yield over the sovereign curve at the tenor, and it is ONE derivation with two callers.
 *
 * The basis measurement derived it here and the party quoting the book derived something else over
 * in `participants.ts`: par less the bond's price, divided by the years, which is not a spread at
 * all — it ignores the coupon, ignores the sovereign, and says a bond at par has no credit risk
 * however dear money is. Two answers to *what does the cash market charge for this name* is Law 4's
 * defect exactly, and the one that stays is the one derived from a yield.
 *
 * It is stated over the reads BOTH callers have, because one of them is a party looking at the
 * world through its own view and the other is the world (the shape `carryFromWorld`/`carryFromView`
 * already uses for the commodity carry).
 */
export interface SpreadReads {
  readonly period: Period;
  readonly calendar: Calendar;
  readonly registry: Pick<Registry, 'instrumentKind'>;
  readonly instruments: { has(id: InstrumentId): boolean; get(id: InstrumentId): Instrument };
  print(id: InstrumentId): Option<Print>;
  curve(family: CurveFamilyId): CurveRead;
  sovereignCurveIn(ccy: CurrencyCode): Option<{ readonly issuer: PartyId }>;
}

export const spreadsFromWorld = (ctx: WorldReads): SpreadReads => ({
  period: ctx.period,
  calendar: ctx.calendar,
  registry: ctx.registry,
  instruments: ctx.instruments,
  print: (id) => ctx.prices.latest(id, ctx.period),
  curve: (family) => ctx.curve(family),
  sovereignCurveIn: (ccy) => ctx.sovereignCurveIn(ccy),
});

export const spreadsFromView = (view: ParticipantView): SpreadReads => ({
  period: view.period,
  calendar: view.calendar,
  registry: view.registry,
  instruments: view.instruments,
  print: (id) => view.print(id),
  curve: (family) => view.curve(family),
  sovereignCurveIn: (ccy) => view.sovereignCurveIn(ccy),
});

/** C3: the bond's own yield, derived FROM its price and the flows its terms promise (Law 3). */
export function cashSpreadOf(
  reads: SpreadReads,
  obligation: InstrumentId,
  tenorYears: number,
  ccy: CurrencyCode,
): Option<Ratio> {
  if (!reads.instruments.has(obligation)) return none<Ratio>();
  const i = reads.instruments.get(obligation);
  const cash = reads.print(obligation);
  if (!cash.some) return none<Ratio>();
  const sovereign = reads.sovereignCurveIn(ccy);
  if (!sovereign.some) return none<Ratio>();
  const on = reads.calendar.startOf(reads.period);
  const flows = reads.registry
    .instrumentKind(i.kind)
    .cashFlows(i, on, reads.calendar, reads.registry as never);
  if (flows.length === 0) return none<Ratio>();
  const own = yieldOf(flows, cash.value.price, on, CDS_DAY_COUNT, `the yield of ${i.id}`);
  if (!own.some) return none<Ratio>();
  const risk = reads.curve(curveFamilyOf(sovereign.value.issuer, i.ccy)).at(tenorYears);
  if (!risk.yield.some) return none<Ratio>();
  // Two YIELDS, both derived from prices, so what the bond pays over the sovereign is a rate.
  return some(minus(own.value, risk.yield.value, 'the bond over the sovereign'));
}

/**
 * C3, C3.a, C3.b: THE BASIS — what protection costs against what the cash bond's own spread is.
 *
 * It is a READ and a standing observation, never a check and never a target: the two are different
 * instruments with different funding and different holders, and that they differ is the thing worth
 * watching rather than a discrepancy to be closed.
 */
export function basisFor(ctx: WorldReads, reference: PartyId, tenorYears: number): Option<number> {
  const protection = ctx.prices.latest(cdsLineOf(reference, tenorYears), ctx.period);
  if (!protection.some) return none<number>();
  const debt = defaultableDebtOf(ctx, reference);
  if (!debt.some) return none<number>();
  // 18.5: the ONE derivation, which the party quoting this book uses too (Law 4). It finds the
  // sovereign curve for the bond's own money itself, so no caller has to name one for it.
  const spread = cashSpreadOf(spreadsFromWorld(ctx), debt.value, tenorYears, ctx.instruments.get(debt.value).ccy);
  if (!spread.some) return none<number>();
  const cashSpread = spread.value;
  // `E-11`: THIS BOOK CLEARS A SPREAD, and `Print.price` is a `PerPiece` because most books clear a
  // level. The crossing is named here rather than assumed — what the protection book printed IS a
  // rate — and the finding is that a book cannot say which of the two its level is.
  const struck = asRatio(protection.value.price, 'the spread this book struck');
  return some(minus(struck, cashSpread, 'protection against the cash bond'));
}

/**
 * A1.d, C3, E3: the two measurements this book carries, asked of the kind by whoever is showing
 * them. A book whose reference has no cash bond, or whose money has no sovereign curve, has a
 * basis against nothing — which is Missing and not a zero.
 */
export function cdsMeasures(m: MarketDecl, reads: WorldReads): readonly ContractMeasure[] {
  const decl = contractOf(m);
  if (decl === undefined || !isCds(decl.terms)) return [];
  const t = decl.terms;
  const out: ContractMeasure[] = [
    {
      subject: String(t.reference),
      measure: 'protection outstanding',
      tenorYears: null,
      level: netNotionalOn(reads, t.reference),
      unit: 'notional',
    },
  ];
  const sovereign = reads.sovereignCurveIn(m.ccy);
  if (!sovereign.some) return out;
  const basis = basisFor(reads, t.reference, t.tenorYears);
  if (!basis.some) return out;
  out.push({
    subject: String(t.reference),
    measure: 'protection against its cash bond',
    tenorYears: t.tenorYears,
    level: basis.value,
    unit: 'rate',
  });
  return out;
}
