/**
 * Land: a place has only so much of it, somebody holds it, and what it costs is cleared.
 *
 * @spec Capital Programme A2 Capital Programme C1 Commodities Spot A3 Goods B4 Clearing A2 Clearing A3 Law 1 Law 2 Law 3 Law 6 Law 19
 *
 * `RegionDecl` was `{id, name, country}`: three fields, no profile, no behaviours, NO SUPPLY. A
 * place was a label. Land could not be owned, used up, run out or priced, so a tile carried its
 * thousandth building at the price of its first.
 *
 * TWO DECLARATIONS WERE ALREADY THERE AND NOTHING READ EITHER. `TerrainDecl.buildKm2` — what
 * putting up a unit of plant on ground like this takes, against ordinary flat ground at one — is
 * declared for every terrain in the map and is read by nobody. And `CapitalKindDecl.landPerUnit` —
 * the ground a unit of plant stands on — is read only by the yield arithmetic, which asks how good
 * the MARGINAL hectare is and never whether there is one. `geography.ts` already does rising
 * marginal cost properly for resource yield, which proves the mechanism is understood; it had
 * nowhere to attach for BUILT space.
 *
 * SO LAND IS AN INSTRUMENT. One line per place, its issue the effective buildable area of that
 * place, held by named parties from the first period — the state holds what nobody has built on,
 * which is what a state is for and gives the residual a holder (Appendix B). A firm that wants to
 * put up plant buys the ground first, in a market that clears (Law 3): there is no land price
 * anywhere in this file and no formula that could produce one.
 *
 * IT IS NOT MADE AND IT DOES NOT WEAR OUT, which is why it is not a capital kind. A capital kind is
 * made from a good and has a life (`CapitalKindDecl`); land is neither, and giving it a fake life
 * would put a wearing-out nobody pays into the capital charge.
 */
import type { Order } from '../../clearing/solver.js';
import type { MarketDecl } from '../../clearing/market.js';
import { div, sub, sum } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import { CENT_TICK, HECTARES_PER_KM2 } from '../../registry/grid.js';
import { asQty, downTick } from '../../core/tick.js';
import {
  instrumentId,
  instrumentKindId,
  marketId,
  unitId,
  type InstrumentId,
  type MarketId,
  type PartyId,
  type RegionId,
} from '../../core/ids.js';
import type { InstrumentKindProfile } from '../../registry/kinds.js';
import { shareOf, tilesOf } from '../../registry/geography.js';
import { groundUnderPlant, vintagesHeld } from '../../registry/physical.js';
import type { GeographyDecl, RatioReads } from '../../registry/geography.js';


import { FIRM, TREASURY } from '../../registry/profiles.js';
import type { ParticipantView, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

/** A place's ground, as a line. One per place: land in one place is not land in another (A1.a). */
export const LAND = instrumentKindId('land');
export const HECTARE = unitId('km2');
export const landId = (region: RegionId): InstrumentId => instrumentId(`land.${region}`);
export const landMarket = (region: RegionId): MarketId => marketId(`mkt.land.${region}`);

/**
 * Law 3, XI-6: A CLAIM ON NOBODY, priced by whoever wants it. It is not a liability — the ground
 * owes its holder nothing and there is no issuer to redeem it — and it is `cleared`, because a
 * price for land that came from anywhere but a market meeting a market is the thing Law 3 forbids.
 */
export const landKind: InstrumentKindProfile = {
  id: LAND,
  pricing: 'cleared',
  // Law 8: what a hectare is quoted in. A price is a count of pieces of money like any other, and
  // land is quoted to the cent the way a share and a tonne are.
  priceTick: CENT_TICK,
  /**
   * Goods E1, XI-6: AT WHAT IT COST, which for ground nobody has ever bought is nothing.
   *
   * It is not carried at the mark, and the reason is Seed C4 read honestly. A market that has never
   * traded has no price, and a world that opened with every state holding a continent would have to
   * SAY what a hectare of it was worth before anybody had paid for one — a written price for the
   * largest asset in the world, which is exactly what Law 3 forbids and what `OPENING_SHARE`'s
   * invariance argument cannot be stretched to cover, because an area is a physical fact and not a
   * unit anybody chose.
   *
   * So the state carries what nobody has bought at nothing, and books what somebody pays when
   * somebody pays it. A hectare that has traded has a cleared price like anything else.
   */
  carry: 'cost',
  /**
   * Goods A1, Register B3: A PHYSICAL THING NAMES NOBODY, and the ground is the most physical thing
   * in this world. The `names` family is right to report an unissued claim as a claim on nobody;
   * land is not a claim at all, which is what this says and why it is exempt from that check.
   */
  physical: true,
  liabilityOfIssuer: false,
  owes: 'face',
  ranking: () => ({ seniority: 0, secured: [], claim: 'the ground itself' }),
  unit: () => HECTARE,
  validateTerms: () => undefined,
  displayName: (i) => `the ground of ${String(i.id).replace('land.', '')}`,
  due: () => [],
  accrued: () => 0,
  cashFlows: () => [],
};

/**
 * C1, Goods B4: HOW MUCH OF A PLACE CAN BE BUILT ON, in effective square kilometres.
 *
 * The same shape as `tileYield` beside it: a tile is a mix of grounds, and a km² of ground that
 * takes twelve times as much to build on is a twelfth of a km² of buildable place. `buildKm2` has
 * been declared for every terrain since the map was written and this is the first thing to read it.
 *
 * A READ over primitives, computed where it is used and never stored (Law 19). It cannot be
 * negative or infinite by construction: every `buildKm2` is positive and every share is a share.
 */
export function buildableKm2(g: GeographyDecl, reads: RatioReads, place: RegionId): number {
  const perTile = g.tileKm * g.tileKm;
  const terms: number[] = [];
  for (const t of tilesOf(g, place)) {
    for (const x of g.terrains) {
      const share = shareOf(g, x.id, t);
      if (share <= 0) continue;
      terms.push(div(share * perTile, reads.ratio(x.buildKm2), `buildable ${x.id} in ${place}`));
    }
  }
  return sum(terms).value;
}

export function land(): SystemModule {
  return {
    id: 'land',
    // Nothing is kept: what a party holds is in the register and what a place has is a read.
    nouns: [],
    spec: 'Capital Programme A2 Capital Programme C1 Goods B4 Clearing A2',
    // The plant kinds have to be registered before their `landPerUnit` can be read, and the STATE
    // HAS TO EXIST before it can hold what nobody has built on — which means the foundation seed,
    // because that is what puts the parties in the world (Seed B1).
    requires: ['capital-programme', 'treasury', 'seed.foundation'],
    instrumentKinds: [landKind],
    derivativeKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [
      {
        id: HECTARE,
        name: 'square kilometres of buildable ground',
        // Law 8: a hectare is the smallest piece of ground anybody builds on, and there are a
        // hundred to the square kilometre. Nothing is held in a fraction of one.
        perUnit: HECTARES_PER_KM2,
      },
    ],
    params: [],
    families: [],
    participants: [
      {
        /**
         * C1: THE STATE SELLS WHAT NOBODY HAS BUILT ON. It offers the ground it holds and takes
         * what the book gives it — no reservation, because a state holding ground it is not using
         * has no cost of carrying it and nothing it would rather do with it. What it will not do
         * is set the price: a posted price for land is exactly the "no posted benchmark" Appendix B
         * forbids, and what a hectare fetches is what somebody paid.
         */
        partyKind: TREASURY,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          /**
           * Item 15: IT SELLS BECAUSE OF WHAT IT IS FOR, not because of what it is called. A party
           * whose objective is `itsOffice` holds ground as a duty and has nobody to enrich by
           * keeping it; one whose objective is `theResidual` holds it to build on. When `E-5`'s
           * local authority arrives it will sell here with no change to this file, because the
           * reason is declared on the kind and read here (Law 15).
           */
          if (view.objectiveOf(view.self.id) !== 'itsOffice') return [];
          /**
           * IN ITS OWN PLACE, and that is a limitation with a reason rather than an oversight.
           *
           * A country has one state and several places, and the state holds the ground of all of
           * them — nothing is a residual with no holder. But a seller in one place selling ground
           * in another is a CROSS-BORDER trade in this world's accounts (`external`'s balance of
           * payments reads the parties' regions), and a hectare of us.2 sold to anybody is still in
           * us.2: nothing crossed. What is missing is a party present in each place to sell its
           * ground — a local authority, which is the same noun a port and a planning consent need
           * and which arrives with the rest of 13m (`E-5`).
           */
          if (m.instrument !== landId(view.self.region)) return [];
          const held = view.quantity(m.instrument);
          if (held <= 0) return [];
          /**
           * THE LEAST IT WILL TAKE IS THE LEAST THERE IS. A state holding ground it is not using
           * has no cost of carrying it and nothing it would rather do with it, so it takes whatever
           * the book gives — but not NOTHING: land let go for no money is free land, which is the
           * thing this module exists to remove. One piece of money is not a posted price and not a
           * benchmark (Appendix B); it is the smallest number a price can be (Law 8).
           */
          return [{ party: view.self.id, side: 'sell', price: CENT_TICK, qty: held }];
        },
      },
      {
        /**
         * A2, C1: A FIRM BUYS THE GROUND ITS PLANT STANDS ON, and it is short of exactly what it
         * has built and not bought.
         *
         * Its own read, taken with its own view: the plant it holds, at the ground a unit of each
         * kind takes (`landPerUnit`, declared since the capital programme was written and read
         * until now only by the yield arithmetic — which asks how good the MARGINAL hectare is and
         * never whether there is one).
         *
         * WHAT IT WILL PAY is what it has, and no more: there is no valuation of land anywhere in
         * this file and no formula that could produce one (Law 3). A firm with nothing bids
         * nothing and stands on the ground it already holds.
         */
        partyKind: FIRM,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] => {
          if (m.instrument !== landId(view.self.region)) return [];
          const standing = groundUnderPlant(view, vintagesHeld(view, view.calendar.startOf(view.period)));
          const short = sub(
            standing * HECTARES_PER_KM2,
            Number(view.quantity(m.instrument)),
            'the ground it is standing on and has not bought',
          );
          /**
           * Law 8: A BID FOR LESS THAN ONE HECTARE IS NOT A BID. A hectare is the piece and there
           * is nothing between two of them, so a firm standing on a fraction it has not bought is
           * standing on the piece it already holds. Without this the division below asked what it
           * would pay for a ten-thousandth of a hectare and got 9.6 × 10²⁰ — past what an integer
           * can hold, and the run stopped where it should have shrugged.
           */
          const wanted = downTick(short);
          if (wanted < 1) return [];
          const cash = Number(view.cash(m.ccy));
          if (cash <= 0) return [];
          // Law 8: what it can pay for one piece, in whole pieces of the money. A bid it cannot
          // fund is a bid nobody could settle, and the wire would refuse it (Money E1).
          const most = downTick(div(cash, wanted, 'the most it can pay a hectare'));
          if (most <= 0) return [];
          return [{ party: view.self.id, side: 'buy', price: most, qty: asQty(wanted) }];
        },
      },
    ],
    /**
     * Clearing A2, B1: ONE SOLVER, and this book goes through it like every other. There is no
     * phase here and no venue: a module that cleared its own land market would be a second
     * clearing rule for one kind of thing (Law 4), and what a hectare fetches would depend on
     * which file it was in.
     *
     * `order: 0` puts it FIRST in the period's markets, before the goods and the plant: a firm
     * short of ground buys it and then decides what to build on it, which is Clearing F1's
     * "a decision acts on what has already happened".
     */
    phases: [],
    seed(ctx: SeedContext): void {
      for (const r of ctx.registry.regions.values()) {
        const id = landId(r.id);
        ctx.instruments.add({
          id,
          kind: LAND,
          // A claim on nobody: the ground is not anybody's promise (Law 3, Appendix B).
          issuer: none(),
          ccy: ctx.registry.currencyOf(r.id),
          terms: { kind: LAND },
          market: some(landMarket(r.id)),
        });
        ctx.openMarket({
          id: landMarket(r.id),
          name: `the ground of ${r.name}`,
          instrument: id,
          ccy: ctx.registry.currencyOf(r.id),
          rationing: 'proRata',
          order: 0,
        });
        /**
         * Appendix B: NO RESIDUAL WITH NO HOLDER. Every square kilometre of the place is somebody's
         * from the first period, and the somebody is the state — which is what a state is, and
         * which is why the first firm to want to build has a named counterparty to buy from.
         */
        const total = buildableKm2(ctx.registry.geography, ctx.params, r.id);
        const treasury = treasuryOf(ctx, r.id);
        if (treasury === undefined || total <= 0) continue;
        ctx.endowUnits(treasury, id, downTick(total * HECTARES_PER_KM2), 0);
      }
    },
  };
}

/**
 * The state that holds this place's unbuilt ground. A country has ONE treasury and several regions,
 * so the ground of every one of them is the same state's — which is why this asks the country and
 * not the region, and why a place whose country has no state has no ground to sell and says so.
 */
function treasuryOf(ctx: SeedContext, region: RegionId): PartyId | undefined {
  const country = ctx.registry.regions.get(region)?.country;
  if (country === undefined) return undefined;
  for (const p of ctx.parties.ofKind(TREASURY)) {
    if (ctx.registry.regions.get(p.region)?.country === country) return p.id;
  }
  return undefined;
}
