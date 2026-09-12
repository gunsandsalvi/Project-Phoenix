/**
 * What a swap book's cleared rate says against the rest of the world.
 *
 * @spec IRS C3 IRS C3.a IRS E2 Sovereign D3 Observer A1 Law 3 Law 19
 *
 * The SWAP SPREAD: the fixed rate this book cleared against the yield on the sovereign's own paper
 * at the same tenor. Both halves are levels somebody paid — one in a contract book, one in a cash
 * market — so the difference is a read and never a curve anybody fits (E2: nothing here turns a
 * discount curve into a par rate, because there is no discount curve in this module to turn).
 *
 * It lives beside the class because the class is what knows it (Law 15): the surface that shows it
 * asks the kind, and a world with one more class gets one more measurement without being edited.
 */
import type { CurrencyCode, PartyId } from '../../core/ids.js';
import { sub } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import { curveFamilyOf } from '../../prices/curve.js';
import type { MarketDecl } from '../../clearing/market.js';
import type { ContractMeasure } from '../../registry/derivatives.js';
import type { WorldReads } from '../../world/context.js';
import { irsLineOf } from './data.js';
import { isIrs } from './contract.js';

/** C3, C3.a: the swap spread — the cleared fixed rate against the sovereign's own yield. A READ. */
export function swapSpread(
  ctx: WorldReads,
  ccy: CurrencyCode,
  tenorYears: number,
  sovereign: PartyId,
): Option<number> {
  const swap = ctx.prices.latest(irsLineOf(ccy, tenorYears), ctx.period);
  if (!swap.some) return none<number>();
  const risk = ctx.curve(curveFamilyOf(sovereign, ccy)).at(tenorYears);
  if (!risk.yield.some) return none<number>();
  return some(sub(swap.value.price, risk.yield.value, 'the swap against the sovereign'));
}

/**
 * C3: the measurement this book carries, asked of the kind by whoever is showing it. A money with
 * no sovereign curve has a spread against nothing, which is Missing and not a zero.
 */
export function irsMeasures(m: MarketDecl, reads: WorldReads): readonly ContractMeasure[] {
  const decl = m.contract;
  if (decl === undefined || !isIrs(decl.terms)) return [];
  const t = decl.terms;
  const sovereign = reads.sovereignCurveIn(t.ccy);
  if (!sovereign.some) return [];
  const spread = swapSpread(reads, t.ccy, t.tenorYears, sovereign.value.issuer);
  if (!spread.some) return [];
  return [
    {
      subject: String(t.ccy),
      measure: 'the swap against the sovereign',
      tenorYears: t.tenorYears,
      level: spread.value,
      unit: 'rate',
    },
  ];
}
