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
import type { InstrumentId, PartyId } from '../../core/ids.js';
import { sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { issuedBy } from '../../register/instruments.js';
import { curveFamilyOf, yieldOf } from '../../prices/curve.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { ContractMeasure } from '../../registry/derivatives.js';
import type { WorldReads } from '../../world/context.js';
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
  let net = 0;
  for (const c of ctx.contracts.open_()) {
    if (!isCds(c.terms)) continue;
    if (c.terms.reference !== reference) continue;
    // C2: a cleared trade is two rows against the house, so counting both would count the
    // protection twice. The member's side is the one that exists in the world.
    if (c.house !== null && c.a === c.house) continue;
    net = sub(net, -c.notional, 'protection written on this name');
  }
  return net;
}

/**
 * C3, C3.a, C3.b: THE BASIS — what protection costs against what the cash bond's own spread is.
 *
 * It is a READ and a standing observation, never a check and never a target: the two are different
 * instruments with different funding and different holders, and that they differ is the thing worth
 * watching rather than a discrepancy to be closed.
 */
export function basisFor(
  ctx: WorldReads,
  reference: PartyId,
  tenorYears: number,
  sovereign: PartyId,
): Option<number> {
  const protection = ctx.prices.latest(cdsLineOf(reference, tenorYears), ctx.period);
  if (!protection.some) return none<number>();
  const debt = defaultableDebtOf(ctx, reference);
  if (!debt.some) return none<number>();
  const i = ctx.instruments.get(debt.value);
  const cash = ctx.prices.latest(debt.value, ctx.period);
  if (!cash.some) return none<number>();
  // The bond's own yield, derived FROM its price and the flows its terms promise (Sovereign D2,
  // Law 3): a price is the input here and never the output.
  const on = ctx.calendar.startOf(ctx.period);
  const flows = ctx.registry.instrumentKind(i.kind).cashFlows(i, on, ctx.calendar);
  if (flows.length === 0) return none<number>();
  const own = yieldOf(flows, cash.value.price, on, CDS_DAY_COUNT, `the yield of ${i.id}`);
  if (!own.some) return none<number>();
  const risk = ctx.curve(curveFamilyOf(sovereign, i.ccy)).at(tenorYears);
  if (!risk.yield.some) return none<number>();
  const cashSpread = sub(own.value, risk.yield.value, 'the bond over the sovereign');
  return some(sub(protection.value.price, cashSpread, 'protection against the cash bond'));
}

/**
 * A1.d, C3, E3: the two measurements this book carries, asked of the kind by whoever is showing
 * them. A book whose reference has no cash bond, or whose money has no sovereign curve, has a
 * basis against nothing — which is Missing and not a zero.
 */
export function cdsMeasures(m: MarketDecl, reads: WorldReads): readonly ContractMeasure[] {
  const decl = m.contract;
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
  const basis = basisFor(reads, t.reference, t.tenorYears, sovereign.value.issuer);
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
