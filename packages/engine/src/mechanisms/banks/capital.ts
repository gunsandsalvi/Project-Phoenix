/**
 * How much capital a bank must have, how much it has, and which of the two rules is the one biting.
 *
 * @spec Banks Capital B1 Banks Capital B1.a Banks Capital B1.b Banks Capital B1.c Banks Capital B2 Banks Capital B3 Banks Capital B3.a Banks Capital A1 Banks Capital A1.a Banks Capital A4 Banks Lending F3 Banks Lending B2.a Sovereign E5 XI-3 Law 2 Law 15 Law 19
 *
 * B1 is a requirement against RISK-WEIGHTED assets and B1.b is a leverage backstop that uses no
 * weights at all. Both are rules somebody wrote, both are here, and B1.c is the point of having
 * two: WHICH ONE BINDS IS AN OUTCOME. A bank stuffed with sovereign paper weighs almost nothing and
 * is stopped by the backstop; a bank whose book is unsecured lending is stopped by the weighted
 * rule; and neither of those is stated anywhere — it falls out of what each bank actually holds.
 *
 * B1.a: THE WEIGHT IS A PROPERTY OF WHAT THE ASSET IS, and this asks the one question that decides
 * it: can the party behind this claim fail? A claim on a party whose kind names no way to die, in
 * the money that party issues, is the zero-weighted asset the standard means (Sovereign E5, XI-3) —
 * and XI-3's two exceptions are exactly the central bank and a treasury in its own money. Nothing
 * here branches on a kind id (Law 15); it reads the same profile the resolution reads to decide
 * whether a party can fail at all, so the two can never disagree about what is safe (Law 4).
 *
 * B2's buffer is the BANK'S OWN, declared as its own caution and not as a ratio anybody imposed,
 * and B3 is what happens when it is breached: the position is published (B3.a), which is what a
 * depositor, a rival and a lender all read, and the bank's own credit decision has less room in it.
 */
import { currencyUnit, moneyInstrumentId, type CurrencyCode, type InstrumentId, type PartyId } from '../../core/ids.js';
import {
  add,
  atLeast,
  atMost,
  div,
  mul,
  sub,
  sum,
} from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { Instrument } from '../../register/instruments.js';
import type { MechanismContext } from '../../world/context.js';
import { isLoan } from './loan.js';
import { subordinatedOf } from './subordinated.js';

/** What binds a bank's book: the weighted rule, the backstop, or neither (B1.c). */
export type Binding = 'weighted' | 'leverage' | 'nothing';

export interface CapitalPosition {
  readonly bank: PartyId;
  readonly ccy: CurrencyCode;
  /** A1: capital is the RESIDUAL — its own equity account, not a fund. */
  readonly capital: number;
  /** B1: what its book weighs, asset by asset. */
  readonly weighted: number;
  /**
   * Law 7: HOW MANY TERMS THAT SUM HAD and what magnitude it passed through. A reader comparing
   * something against this number is comparing against a walk over the bank's whole book, and the
   * dust it is entitled to is the dust of THAT arithmetic — not of the handful of terms the reader
   * itself could see. Published with the number, because a tolerance derived from one side of a
   * comparison is a tolerance derived from the wrong thing (worklist 13a, finding 12b-5).
   */
  readonly weightedTerms: number;
  readonly weightedMagnitude: number;
  /** B1.b: and what it comes to with no weights at all. */
  readonly assets: number;
  readonly weightedRatio: Option<number>;
  readonly leverageRatio: Option<number>;
  readonly minWeighted: number;
  readonly minLeverage: number;
  /** B2: how far above both lines this bank has chosen to run. */
  readonly buffer: number;
  /** B1.c: which rule leaves it the least room, in units of the asset it would add. */
  readonly binds: Binding;
  /** What it could still put on, at the weight of the asset it is deciding about. */
  readonly headroom: number;
  /** B3: below the line it chose for itself, which is where the consequences start. */
  readonly breach: boolean;
  /** C1: and below the line the rule itself draws, which is a different and worse thing. */
  readonly belowRequirement: boolean;
  /** Banks Lending F3: the most it will fund for one name, which its names can read (XI-2). */
  readonly limitPerName: number;
  /**
   * Dealer Desks F2, Banks Capital B1: WHAT EACH LINE OF THE BANK IS USING, off the same walk that
   * produced `weighted` — one traversal, so a line's capital and the bank's can never disagree
   * (Law 4). Three answers, because there are three reasons this bank holds anything: it lent it,
   * its treasury wants it for liquidity, or its dealing line is carrying it above that.
   */
  readonly byLine: LineWeights;
}

/** What each reason for holding something asks of the bank's capital (Dealer Desks F2). */
export interface LineWeights {
  readonly lending: number;
  readonly liquidity: number;
  readonly dealing: number;
}

export interface CapitalRules {
  /** F3: the share of its own capital it will have out to any one name. */
  readonly limitPerName: number;
  readonly minWeighted: number;
  readonly minLeverage: number;
  readonly buffer: number;
  /** The weight of the asset the decision is about — a loan, unless the caller says otherwise. */
  readonly weight: number;
  readonly sovereignWeight: number;
  /** Dealer Desks D2, F2: what a unit of a TRADING position consumes, whatever it is a claim on. */
  readonly tradingWeight: number;
  /**
   * Dealer Desks C2.a, F2, Banks Funding C2: where this bank's own treasury wants each line held,
   * in its own money. It is what tells a holding it is there because the treasury decided so from
   * a holding it is running as a POSITION, and the two do not weigh the same.
   */
  readonly targets: ReadonlyMap<InstrumentId, number>;
}

/**
 * B1.a, Sovereign E5, XI-3: what one unit of this asset weighs. A claim on a party that cannot
 * fail, in the money that party issues, is the zero-weighted asset; everything else weighs what the
 * rule says an ordinary exposure weighs. A holding that is nobody's liability — a good, a share —
 * is not a claim on anybody at all and weighs the same as an exposure that can go wrong, because it
 * can.
 */
export function riskWeightOf(ctx: MechanismContext, i: Instrument, rules: CapitalRules): number {
  if (!i.issuer.some) return rules.weight;
  const issuer = ctx.parties.get(ctx.parties.resolve(i.issuer.value).id);
  const canFail = ctx.registry.partyKind(issuer.kind).fails ?? [];
  if (canFail.length > 0) return rules.weight;
  return ctx.registry.region(issuer.region).ccy === i.ccy ? rules.sovereignWeight : rules.weight;
}

/**
 * B1, B1.b, B1.c: the whole position, read off the register at the marks in force (Law 19). What it
 * is NOT is a stored ratio: every number here is a walk over what the bank holds this instant.
 */
export function capitalOf(
  ctx: MechanismContext,
  bank: PartyId,
  ccy: CurrencyCode,
  rules: CapitalRules,
): CapitalPosition {
  const own = moneyInstrumentId(bank, ccy);
  const held: number[] = [];
  const weighted: number[] = [];
  const byLine = { lending: 0, liquidity: 0, dealing: 0 };
  for (const h of ctx.register.holdingsOf(bank)) {
    if (h.instrument === own) continue;
    const value = ctx.valuation.valueOfLots(h.instrument, h.lots, ctx.period);
    if (value === 0) continue;
    held.push(value);
    // B1.a, Dealer Desks D2, F2: WHAT AN ASSET WEIGHS DEPENDS ON WHY IT IS HELD. A holding up to
    // where its own treasury wants the line is there because the treasury decided so, and it
    // weighs what a claim on that issuer weighs — nothing, for a party that cannot fail in the
    // money it issues. What is held ABOVE that is a position somebody took with a view, and a view
    // can be wrong whoever it is about, so it weighs what a trading position weighs. That is F2
    // with nothing exempt: the dealing book consumes its own bank's capital because it IS the
    // bank's, and B1.a's "risk weights differ by asset, and that is why a bank prefers some assets
    // to others" now has something to bite on.
    //
    // Only the part ABOVE. A line it is SHORT of its target is a position too — it will have to
    // buy — but it is not an asset it holds, and capital stands against what a bank owns.
    const want = rules.targets.get(h.instrument);
    const banking = want === undefined ? value : atMost(value, want, 'the treasury cannot claim more of a line than there is of it');
    const trading = sub(value, banking, 'the part it is running as a position');
    const asBanking = mul(
      banking,
      riskWeightOf(ctx, ctx.instruments.get(h.instrument), rules),
      'weighted',
    );
    const asTrading = mul(trading, rules.tradingWeight, 'what a position weighs');
    weighted.push(add(asBanking, asTrading, 'what this holding asks of its capital'));
    // The same three answers the weighting already gives, kept rather than summed away: what it
    // LENT (a claim it wrote, which no market prices and no target names), what its treasury wants
    // held for liquidity, and what its dealing line is carrying above that.
    if (isLoan(ctx.instruments.get(h.instrument).terms)) {
      byLine.lending = add(byLine.lending, asBanking, 'what its lending uses');
    } else {
      byLine.liquidity = add(byLine.liquidity, asBanking, 'what its liquidity uses');
    }
    byLine.dealing = add(byLine.dealing, asTrading, 'what its dealing uses');
  }
  // A2, A3: CAPITAL IS LAYERED, and both layers are here — the equity that absorbs first and fully
  // (A2.a) and the subordinated claims that absorb next (A2.b). A requirement met with equity alone
  // would be a requirement no bank could ever raise its way back over except by earning it, and
  // C2's "recapitalisation first, if somebody will provide it" would have nothing to provide.
  const capital = add(
    ctx.participant(bank).equity(),
    subordinatedOf(ctx, bank),
    'what stands in front of its creditors',
  );
  const assets = sum(held).value;
  const weightedSum = sum(weighted);
  const rwa = weightedSum.value;
  const askedWeighted = add(rules.minWeighted, rules.buffer, 'the line it runs to');
  const askedLeverage = add(rules.minLeverage, rules.buffer, 'the backstop it runs to');
  // What each rule leaves it, in units of the asset it would add: the weighted rule counts that
  // asset at its weight and the backstop counts it whole, which is why a zero-weighted asset is
  // free under one and not under the other. That difference IS B1.b's reason to exist.
  const byWeighted = sub(div(capital, askedWeighted, 'weighted assets it can carry'), rwa, 'weighted room');
  const byLeverage = sub(div(capital, askedLeverage, 'assets it can carry'), assets, 'leverage room');
  const inUnits = rules.weight > 0 ? div(byWeighted, rules.weight, 'units it could add') : byWeighted;
  const binds: Binding =
    byLeverage < inUnits ? 'leverage' : inUnits < byLeverage ? 'weighted' : 'nothing';
  return {
    bank,
    ccy,
    capital,
    weighted: rwa,
    weightedTerms: weightedSum.terms,
    weightedMagnitude: weightedSum.magnitude,
    assets,
    weightedRatio: rwa > 0 ? some(div(capital, rwa, 'its weighted ratio')) : none<number>(),
    leverageRatio: assets > 0 ? some(div(capital, assets, 'its leverage ratio')) : none<number>(),
    minWeighted: rules.minWeighted,
    minLeverage: rules.minLeverage,
    buffer: rules.buffer,
    binds,
    headroom: atMost(byLeverage, inUnits, 'the rule that leaves it less is the room there is'),
    limitPerName: mul(capital, rules.limitPerName, 'the most it will fund for one name'),
    byLine,
    breach: capital < mul(rwa, askedWeighted, 'what the line asks') ||
      capital < mul(assets, askedLeverage, 'what the backstop asks'),
    belowRequirement:
      capital < mul(rwa, rules.minWeighted, 'the requirement') ||
      capital < mul(assets, rules.minLeverage, 'the backstop'),
  };
}

/** B3.a, Observer A5: the position, said out loud. It is what a depositor and a rival both read. */
export function publish(ctx: MechanismContext, p: CapitalPosition): void {
  ctx.record(
    'bank.capital',
    [p.bank],
    {
      bank: p.bank,
      ccy: p.ccy,
      capital: p.capital,
      weighted: p.weighted,
      weightedTerms: p.weightedTerms,
      weightedMagnitude: p.weightedMagnitude,
      assets: p.assets,
      weightedRatio: p.weightedRatio.some ? p.weightedRatio.value : null,
      leverageRatio: p.leverageRatio.some ? p.leverageRatio.value : null,
      minWeighted: p.minWeighted,
      minLeverage: p.minLeverage,
      buffer: p.buffer,
      binds: p.binds,
      headroom: p.headroom,
      breach: p.breach,
      belowRequirement: p.belowRequirement,
      // F3, XI-2: THE MOST IT WILL FUND FOR ANY ONE NAME, published because the names it funds have
      // to be able to see it. It is its own capital times its own limit, so when its capital falls
      // the line falls with it — and a party carrying more than the line is carrying more than its
      // funder will stand behind, which is the third of XI-2's four doors.
      limitPerName: p.limitPerName,
      unit: currencyUnit(p.ccy),
    },
    true,
  );
  // B3: BEFORE FAILURE, and each of the three is a real consequence rather than an adjective. The
  // plan is demanded in public, the supervision that follows is public with it, and the money it
  // may still put to work is the headroom above — which its own credit decision reads (B2.b).
  if (!p.breach) return;
  const shortWeighted = sub(
    mul(p.weighted, add(p.minWeighted, p.buffer, 'the line'), 'what the weighted rule asks'),
    p.capital,
    'short of the weighted line',
  );
  const shortLeverage = sub(
    mul(p.assets, add(p.minLeverage, p.buffer, 'the backstop'), 'what the backstop asks'),
    p.capital,
    'short of the backstop',
  );
  ctx.record(
    'bank.capitalPlan',
    [p.bank],
    {
      bank: p.bank,
      // What it would have to raise to be above BOTH lines, which is the bigger of the two — a
      // plan that answered only the rule it happened to breach first would leave it in breach.
      short: atLeast(shortWeighted, shortLeverage, 'the rule it is further under is what it must find'),
      binds: p.binds,
      belowRequirement: p.belowRequirement,
    },
    true,
  );
}
