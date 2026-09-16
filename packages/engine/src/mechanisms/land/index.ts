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
 *
 * ITEM 15.1: WHO SELLS IT, AND AT WHAT. A LOCAL AUTHORITY present in each place holds that place's
 * unbuilt ground — a country's one treasury selling every place's ground was a cross-border trade
 * in this world's accounts — and it offers a period what a PLANNING POLICY says it may (parliament's
 * number, never everything it holds), at its own outlook of what ground here has fetched, or at
 * whatever the book gives where nothing has fetched anything yet. Nothing here posts a price of one
 * cent to say "yes": the two asks that did are gone. The BUYER is the firm's investment decision
 * (`registry/capital.ts project`): a project needs ground as it needs machines, and the most it
 * will pay a hectare is what the ground is worth to it — the project's surplus over the plant it
 * has to buy, spread over the hectares it takes — which is the residual land value the world this
 * reflects prices ground at (Law 1). The capital programme refuses to stand plant on ground the
 * firm does not hold, and pledges the ground to the vintage while it stands.
 */
import type { Order } from '../../clearing/solver.js';
import { asPerPiece, type PerPiece } from '../../core/measure.js';
import type { MarketDecl } from '../../clearing/market.js';
import { atMost, div, sum } from '../../core/num.js';
import { none, some } from '../../core/option.js';
import { about } from '../../world/context.js';
import { CENT_TICK, HECTARES_PER_KM2 } from '../../registry/grid.js';
import { asQty, downTick, type Qty } from '../../core/tick.js';
import { type InstrumentId, type PartyId, type RegionId } from '../../core/ids.js';
import type { InstrumentKindProfile, PartyKindProfile } from '../../registry/kinds.js';
import { shareOf, tilesOf } from '../../registry/geography.js';
import { groundUnder, isPlant, plantTerms } from '../../registry/physical.js';
import type { GeographyDecl, RatioReads } from '../../registry/geography.js';
import { HECTARE, LAND, LOCAL_AUTHORITY, PLANNING_RELEASE, authorityIdFor, hectaresOf, landId, landMarket } from '../../registry/land.js';
import type { ParticipantView, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

// 15.1, Law 4: the names of the ground are the registry's, so a project and the programme read them.
export { HECTARE, LAND, LOCAL_AUTHORITY, PLANNING_RELEASE, authorityIdFor, landId, landMarket } from '../../registry/land.js';

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
   * So the authority carries what nobody has bought at nothing, and books what somebody pays when
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
 * Item 15.1: THE LOCAL AUTHORITY. One per place, holding what nobody has built on there, selling
 * it as its planning policy allows. It holds ground as a duty (`itsOffice`) and has nobody to
 * enrich by keeping it, which is the reason it sells and the reason it sets no price. It fails on
 * cash like any office; it borrows nothing and buys nothing on terms.
 */
export const localAuthorityKind: PartyKindProfile = {
  id: LOCAL_AUTHORITY,
  representation: 'named',
  objective: 'itsOffice',
  moneyIssuer: null,
  fails: ['cash'],
  borrows: false,
  buysOnTerms: false,
  depositClass: null,
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

/**
 * Item 15.1, C1, §46 B1, Law 3: WHAT THE AUTHORITY OFFERS, AND AT WHAT. The quantity is the
 * planning policy's — so many hectares a period, and never more than it holds, which is arithmetic
 * — and the level is its OWN outlook of what ground here has fetched where it has formed one, the
 * last print where it has not, and where nothing has ever fetched anything it takes what the book
 * gives: a seller with no opinion and no history names no level, and the first price of a place's
 * ground is what the first buyer's reason came to. Nothing here says "one cent".
 */
export function authorityOrders(view: ParticipantView, m: MarketDecl): readonly Order[] {
  if (m.instrument !== landId(view.self.region)) return [];
  const held = view.free(m.instrument);
  if (held <= 0) return [];
  const release = asQty(view.params.count(PLANNING_RELEASE), 'the hectares the policy releases a period');
  const qty = atMost(release, held, 'it cannot offer ground it does not hold');
  if (qty <= 0) return [];
  const outlook = view.outlook(about({ on: 'price', instrument: m.instrument }));
  const print = view.print(m.instrument);
  const level = outlook.some
    ? some(asPerPiece(outlook.value.expected, 'where its own outlook puts the ground'))
    : print.some
      ? some(print.value.price)
      : none<PerPiece>();
  if (!level.some) return [{ party: view.self.id, side: 'sell', price: 'market', qty: asQty(qty, 'the hectares it releases') }];
  const price = view.registry.onQuoteGrid(view.instruments.get(m.instrument).kind, m.ccy, level.value);
  if (price <= 0) return [{ party: view.self.id, side: 'sell', price: 'market', qty: asQty(qty, 'the hectares it releases') }];
  return [{ party: view.self.id, side: 'sell', price, qty: asQty(qty, 'the hectares it releases') }];
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
    partyKinds: [localAuthorityKind],
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
    params: [
      {
        id: PLANNING_RELEASE,
        value: 100,
        unit: 'hectares a period',
        dimension: 'count',
        kind: 'policy',
        owner: 'parliament',
        why: 'Capital Programme C1 (15.1): how much of a place\u2019s unbuilt ground its authority releases a period — a square kilometre. A rule about consents (POLICY, parliament\u2019s from worklist 14) and not a forecast of what anybody will build: the authority never offers everything it holds, and what a hectare fetches is what the book says.',
      },
    ],
    families: [],
    participants: [
      {
        /**
         * C1, 15.1: THE AUTHORITY SELLS WHAT NOBODY HAS BUILT ON — in its own place, because it is
         * the party present there — and it sells because of what it is for (`itsOffice`), read off
         * the kind and never off its name (Law 15).
         */
        partyKind: LOCAL_AUTHORITY,
        orders: (view: ParticipantView, m: MarketDecl): readonly Order[] =>
          view.objectiveOf(view.self.id) === 'itsOffice' ? authorityOrders(view, m) : [],
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
         * from the first period. What the opening's plant already stands on is its holder's, at
         * nothing — the seed states the plant and the ground under it together (Seed C4: an opening
         * is a stock, not a purchase) — and the rest is the authority's, which is what an authority
         * is, and why the first firm to want to build has a named counterparty to buy from.
         */
        const total = hectaresOf(buildableKm2(ctx.registry.geography, ctx.params, r.id));
        if (total <= 0) continue;
        let underPlant: Qty = asQty(0, 'nothing yet');
        for (const p of ctx.parties.all()) {
          if (p.representation !== 'named' || p.region !== r.id) continue;
          const standing = groundUnderAt(ctx, p.id, id);
          if (standing <= 0) continue;
          ctx.endowUnits(p.id, id, standing, 0);
          underPlant = asQty(underPlant + standing, 'the ground the opening\u2019s plant stands on');
        }
        const authority = authorityFor(ctx, r.id);
        if (authority === undefined) continue;
        const rest = downTick(total - underPlant);
        if (rest > 0) ctx.endowUnits(authority, id, rest, 0);
      }
    },
  };
}

/** Seed C4: the whole hectares a named party's opening plant stands on, off its own holdings. */
function groundUnderAt(ctx: SeedContext, party: PartyId, land: InstrumentId): Qty {
  const rows: { readonly capitalKind: string; readonly units: Qty }[] = [];
  for (const h of ctx.register.holdingsOf(party)) {
    if (h.instrument === land) continue;
    const i = ctx.instruments.get(h.instrument);
    if (!i.status.live || !isPlant(i)) continue;
    rows.push({ capitalKind: plantTerms(i).capitalKind, units: ctx.register.quantity(party, h.instrument) });
  }
  if (rows.length === 0) return asQty(0, 'no plant');
  return hectaresOf(groundUnder(ctx, rows));
}

/**
 * 15.1: THE AUTHORITY OF A PLACE, made here if the foundation has not, banked at the first bank of
 * the place. A place with no bank has nowhere for an office to hold its money and gets none, which
 * is a seed finding and not a default.
 */
function authorityFor(ctx: SeedContext, region: RegionId): PartyId | undefined {
  const id = authorityIdFor(region);
  if (ctx.parties.has(id)) return id;
  const banks = [...ctx.parties.all()].filter((p) => ctx.registry.issuesMoney(p.kind) && p.bank !== p.id);
  const country = ctx.registry.regions.get(region)?.country;
  // At the bank of its own place where there is one, else at a bank of its country: an office
  // banks where its money is, and a place with no bank of its own still has a country's.
  const bank = banks.find((p) => p.region === region) ?? banks.find((p) => ctx.registry.regions.get(p.region)?.country === country);
  if (bank === undefined) return undefined;
  const name = ctx.registry.regions.get(region)?.name ?? String(region);
  ctx.parties.add({
    id,
    kind: LOCAL_AUTHORITY,
    region,
    name: `${name} Council`,
    bank: bank.id,
    representation: 'named',
    status: { alive: true, standing: 'good' },
  });
  return id;
}
