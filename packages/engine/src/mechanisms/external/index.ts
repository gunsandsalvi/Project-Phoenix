/**
 * A region's accounts with the rest of the world: a READ that sums to zero.
 *
 * @spec Cross-Border E1 Cross-Border E2 Cross-Border E3 Cross-Border E4 Cross-Border F1 Cross-Border F2 Currency E1 Money D3 Law 4 Law 5 Law 19 XI-12
 *
 * A COUNTRY'S EXTERNAL ACCOUNTS ARE NOT DATA. Every textbook writes them as a table of flows and
 * every model that imports one has imported an equilibrium (Law 2, Appendix B: no exogenous trade
 * or capital-flow series). Here they are a walk over the settled ledger: what actually crossed a
 * border this period, leg by leg, read off instructions that already happened.
 *
 * AND THEY SUM TO ZERO BECAUSE EVERY TRANSACTION HAD TWO SIDES. That is not an identity anybody
 * enforces here and not a residual anybody plugs — it is Law 5 seen from a country's end. A grain
 * cargo that leaves is a claim that arrives; a bond somebody abroad buys is money that comes in. If
 * the two halves did not cancel, a leg would be missing from the wire, which is the defect the audit
 * family below is looking for and the only thing it could ever find.
 *
 * WHAT GOES IN WHICH HALF is a question about what moved, not about who moved it. A thing crossing
 * is TRADE; a claim crossing is FINANCE; and money is a claim on the bank that issued it, so it is
 * finance too. Nothing here asks what kind of instrument anything is (Law 15) — it asks the
 * register whether the thing that moved is somebody's promise, which is the same question the
 * balance sheet asks and has one answer.
 */
import type { PartyId, RegionId } from '../../core/ids.js';
import { currencyUnit } from '../../core/ids.js';
import { add, sub, sum } from '../../core/num.js';
import { isAssetLeg, isMoneyLeg, type Leg } from '../../ledger/instruction.js';
import type { Violation, Family } from '../../audit/audit.js';
import type { MechanismContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

/**
 * E1, E2: the two halves, in the one direction that makes them add up. Both are stated FROM this
 * region's side: positive is something coming in to it.
 */
export interface External {
  readonly region: RegionId;
  /**
   * E1: what it sold abroad less what it bought — goods and services that physically changed hands
   * across a border. A thing crossing is trade whatever the paperwork called it.
   */
  readonly trade: number;
  /**
   * E2: claims. What foreigners took of its paper and its money, less what it took of theirs. A
   * deficit is FINANCED by somebody who chose to, at a price, and this is who and how much.
   */
  readonly finance: number;
  /** How many cross-border legs were behind the two numbers, so a reader can see it is a walk. */
  readonly legs: number;
}

/**
 * E1–E4, Law 19: THE WALK. Every settled instruction of the period, every leg of it, and the two
 * parties on that leg. A leg inside one region is not in anybody's external accounts; a leg between
 * two is in both, with the signs opposite, which is why the world's accounts sum to zero as well.
 *
 * Nothing is inferred by subtraction and no total is stored. Ask again next period and it is walked
 * again — which is what makes it impossible for this to drift away from what the wire says.
 */
export function externalOf(ctx: ExternalReads, region: RegionId): External {
  const trade: number[] = [];
  const finance: number[] = [];
  let legs = 0;
  for (const r of ctx.ledger.inPeriod(ctx.period)) {
    if (r.outcome !== 'settled') continue;
    const crossings = r.instruction.legs
      .map((leg) => sidesOf(ctx, leg))
      .filter((c): c is Crossing => c !== undefined && (c.from === region) !== (c.to === region));
    if (crossings.length === 0) continue;
    /**
     * E1: WAS ANYTHING DELIVERED ON THE WIRE? A sale of grain abroad has two crossing legs — the
     * cargo out and the money in — and they cancel on their own. A WAGE has one: the money goes,
     * and the week of work that earned it is not an instrument anybody holds. Labour, rent and a
     * fee are like that, and they are most of what crosses a border in a service economy.
     *
     * So a payment with nothing delivered beside it BOOKS BOTH HALVES: the money that moved, and
     * the thing it bought. What the thing was worth is what was paid for it — which is a read of
     * the payment and not a number invented for it (Law 19) — and that is exactly what a current
     * account is: the other side of every payment whose subject never appeared in a register.
     */
    const delivered = crossings.some((c) => !c.money);
    for (const c of crossings) {
      legs += 1;
      // Positive is INTO this region. Every instruction moves the same value both ways, which is
      // why the two halves cancel and why E3 is Law 5 seen from a country's end.
      const signed = c.to === region ? c.amount : -c.amount;
      if (c.claim) finance.push(signed);
      else trade.push(signed);
      if (c.money && !delivered) {
        // The service, the work or the use of a thing that the payment was for. It went the other
        // way, so it carries the other sign.
        trade.push(-signed);
      }
    }
  }
  return { region, trade: sum(trade).value, finance: sum(finance).value, legs };
}

/** One leg that crossed a border: between whom, for how much, and what sort of thing moved. */
interface Crossing {
  readonly from: RegionId;
  readonly to: RegionId;
  readonly amount: number;
  readonly claim: boolean;
  /** Whether what moved was MONEY, which is the case with a subject that may not be on the wire. */
  readonly money: boolean;
}

/** Which two regions a leg was between, what it came to, and whether what moved was a promise. */
function sidesOf(
  ctx: ExternalReads,
  leg: Leg,
): Crossing | undefined {
  if (isMoneyLeg(leg)) {
    // Money A1: a deposit is a claim on the bank that issued it, so money crossing a border is
    // finance and never trade. A country that counted its own currency leaving as an export would
    // be counting a promise as a thing.
    return {
      from: regionOf(ctx, leg.from.holder),
      to: regionOf(ctx, leg.to.holder),
      amount: leg.amount,
      claim: true,
      money: true,
    };
  }
  // A pledge, a release or a voyage moves nothing across a border: nobody's holdings change hands.
  if (!isAssetLeg(leg)) return undefined;
  const i = ctx.instruments.get(leg.instrument);
  /**
   * Law 15: what decides the half is whether the thing that moved is SOMEBODY'S PROMISE, which the
   * register already knows because it is the same question the balance sheet asks. A tonne of grain
   * is nobody's liability and is trade; a bond is somebody's and is finance.
   */
  const claim = ctx.registry.instrumentKind(i.kind).liabilityOfIssuer || i.issuer.some;
  const price = leg.pricePerUnit;
  // XI-6: a leg with no price on it is a movement nobody valued, and a movement nobody valued
  // cannot be added to a country's accounts. Missing is Missing, never a zero that reads as free.
  if (!price.some) return undefined;
  return {
    from: regionOf(ctx, leg.from),
    to: regionOf(ctx, leg.to),
    amount: leg.qty * price.value,
    claim,
    money: false,
  };
}

const regionOf = (ctx: ExternalReads, who: PartyId): RegionId => ctx.parties.get(who).region;

/**
 * What the walk needs and nothing more: the settled ledger, who a party is, and what a line's kind
 * says. Written as its own shape so the AUDIT can run exactly the same walk the phase does — one
 * derivation of a country's accounts, read from two places (Law 4).
 */
export interface ExternalReads {
  readonly period: MechanismContext['period'];
  readonly ledger: Pick<MechanismContext['ledger'], 'inPeriod'>;
  readonly parties: Pick<MechanismContext['parties'], 'get'>;
  readonly instruments: Pick<MechanismContext['instruments'], 'get'>;
  readonly registry: Pick<MechanismContext['registry'], 'instrumentKind'>;
}

/**
 * E3, Law 5, Money D3: THE ONE THING THAT MUST BE TRUE, and it is true by construction or the wire
 * is broken. What a region sold the world and what the world lent it are two halves of the same
 * settled legs, so they cancel — and if they ever did not, a leg went out with nothing coming back,
 * which is the one-sided flow Law 5 forbids and the only thing this check can find.
 *
 * It MEASURES and never repairs. A residual plugged here would be exactly the thing Appendix B
 * calls a residual with no holder, and it would hide the missing leg rather than report it.
 */
function accounts(): Family {
  return {
    name: 'flows',
    contributor: 'external',
    spec: 'Cross-Border E1 Cross-Border E3 Money D3 Law 5',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const seen = new Set<RegionId>();
      for (const p of view.parties.alive()) seen.add(p.region);
      for (const region of seen) {
        const said = externalOf(view, region);
        const net = add(said.trade, said.finance, 'what crossed, both halves');
        // Law 7: the dust of the walk, derived from its own terms and magnitudes. Never a band.
        const magnitude = add(Math.abs(said.trade), Math.abs(said.finance), 'what it passed through');
        const dust = (said.legs + 2) * Number.EPSILON * magnitude;
        if (Math.abs(net) <= dust) continue;
        out.push({
          family: 'flows',
          spec: 'Cross-Border E3',
          owner: String(region),
          size: net,
          // Law 8: in the region's OWN money, because that is what both halves were summed in.
          unit: String(currencyUnit(view.registry.currencyOf(region))),
          period: view.period,
          message: `${String(region)}: ${said.trade} crossed as goods and ${said.finance} as claims, which do not cancel`,
        });
      }
      return out;
    },
  };
}

/**
 * E4: what a region's position with the world came to, published so a reader can see it move. It
 * causes nothing — it is a read of events that already happened (Observer A5) — and there is no
 * stored level anywhere, because a stock of foreign claims is a walk over who holds what.
 */
export function publishExternal(ctx: MechanismContext): void {
  const seen = new Set<RegionId>();
  for (const p of ctx.parties.alive()) seen.add(p.region);
  for (const region of seen) {
    const said = externalOf(ctx, region);
    if (said.legs === 0) continue;
    ctx.record(
      'external.accounts',
      [],
      {
        region: String(region),
        trade: said.trade,
        finance: said.finance,
        legs: said.legs,
        // E3: stated, so a reader sees the identity rather than being told about it.
        net: add(said.trade, said.finance, 'the two halves'),
      },
      true,
    );
  }
}

export function external(): SystemModule {
  return {
    id: 'external',
    spec: 'Cross-Border',
    requires: [],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    phases: [
      {
        name: 'external.publish',
        spec: 'Cross-Border E1 Cross-Border E2 Cross-Border E4',
        // Last, because it reads the period's settled legs and a period is not settled until it is.
        anchor: { before: 'revaluation' },
        cycle: 'anchor',
        run: publishExternal,
      },
    ],
    participants: [],
    families: [accounts()],
  };
}

/** E2: a deficit is financed by somebody, and this is the sign of it from the region's own end. */
export const financedBy = (said: External): number =>
  sub(0, said.trade, 'what somebody had to lend it to pay for what it bought');

/** E1: what a region sold the world, net — the half a reader usually means by "the trade balance". */
export const tradeBalanceOf = (said: External): number => said.trade;
