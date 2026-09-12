/**
 * STORAGE, AND WHAT IT COSTS TO HOLD A THING: the stock a spot price reads is a stock somebody pays
 * to keep.
 *
 * @spec Commodities Spot A3 Commodities Spot A4 Commodities Spot D2.a Commodities Spot D3 Commodities Spot F2 Clearing A2 Clearing A3 Law 3 Law 5 Law 6 Law 19
 *
 * A stock that carries across periods is the state a spot price reads (D2.a). A stock that carries
 * FOR NOTHING is a state nobody pays to hold, and a world with free holding has no carry, no reason
 * to sell rather than wait, and a commodity curve that is arithmetic instead of a market.
 *
 * So space is PLANT — a stock of productive assets with a life, owned by a named party (A4) — and
 * what it costs to hold a tonne for a week is a PRICE, cleared in a venue per region from owners
 * with spare space meeting holders who want to keep what they have (D3). Both sides come from the
 * parties' own state, read through each one's own view (Clearing A3).
 *
 * AND THERE IS NO FREE HOLDING (A3), which is what makes the fee worth paying. Room BINDS what a
 * line can have at the end of a period, exactly as its machinery binds what it can make (Capital
 * Programme A2, D4): a good that takes space declares how much, and the space a firm has — its own
 * plus what it rented this period — is one more plant need in the same arithmetic that already
 * takes the scarcest. So a farm with a full barn does not start a batch it would have nowhere to
 * put, and a farm that wants to grow rents a barn or builds one. Nothing is destroyed, nothing is
 * capped and nothing is refused at a door: what is missing is simply room, and room is a thing
 * somebody owns and sells (Law 6: the compensating mechanism, not a bound).
 */
import type { CurrencyCode, PartyId, RegionId } from '../../core/ids.js';
import { add, atMost, div, mul, sub, sum, zeroIfNone } from '../../core/num.js';
import { asQty, downTick } from '../../core/tick.js';
import { none } from '../../core/option.js';
import { clear, isCleared, type Order } from '../../clearing/solver.js';
import {
  isGoodTerms,
  plantHeld,
  spaceFor,
  spacePerPiece,
  STORAGE,
  STORAGE_SESSION,
  vintagesHeld,
  wearPerPlantUnit,
} from '../../registry/physical.js';
import type { MechanismContext, ParticipantView } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';

export { spaceFor, STORAGE, STORAGE_KIND, storageMarket, storageVenue } from './data.js';

/** What one party needs and what it has, in units of covered space, all in its own region. */
interface Room {
  readonly party: PartyId;
  readonly region: RegionId;
  /** A3: what everything it holds takes up, read off the register and the goods' own terms. */
  readonly needs: number;
  /** A4: the space it owns itself. It fills its own barn before it rents anybody else's. */
  readonly owns: number;
  /** What a unit of its own space costs it per period: its reservation as a LETTER of space. */
  readonly costsPerUnit: number | undefined;
}

/** A3, Law 19: what a party's holdings take up, read from the goods' own terms and never stored. */
function roomFor(ctx: MechanismContext, view: ParticipantView, party: PartyId): Room {
  const region = view.self.region;
  const terms: number[] = [];
  for (const h of view.holdings()) {
    const i = ctx.instruments.get(h.instrument);
    if (!i.status.live || !isGoodTerms(i.terms)) continue;
    const per = i.terms.storagePerUnit;
    if (per === null || i.terms.region !== region) continue;
    const units = view.quantity(h.instrument);
    if (units <= 0) continue;
    terms.push(spaceFor(ctx, i.unit, units, ctx.params.ratio(per)));
  }
  const vintages = vintagesHeld(view, ctx.calendar.startOf(ctx.period));
  const wear = wearPerPlantUnit(vintages, STORAGE);
  return {
    party,
    region,
    needs: sum(terms).value,
    owns: plantHeld(vintages, STORAGE),
    costsPerUnit: wear.some ? wear.value : undefined,
  };
}


/**
 * D3, Clearing A3: THE TWO SIDES, each from its own party's state.
 *
 * A letter offers what it owns and is not using, at what that space costs IT to keep standing —
 * below which letting is worse than leaving it empty. A taker bids for what it is short of, at what
 * a unit of the thing is worth to it: it will pay up to the whole value of what would otherwise be
 * lost, because what it is buying is the difference between having the thing and not having it.
 */
function schedules(ctx: MechanismContext, region: RegionId): readonly Order[] {
  const out: Order[] = [];
  for (const p of ctx.parties.alive()) {
    if (p.region !== region) continue;
    const view = ctx.participant(p.id);
    const room = roomFor(ctx, view, p.id);
    if (room.needs <= 0 && room.owns <= 0) continue;
    const spare = sub(room.owns, room.needs, 'the space it is not using itself');
    if (spare > 0 && room.costsPerUnit !== undefined) {
      const qty = downTick(spare);
      if (qty > 0) out.push({ party: p.id, side: 'sell', price: room.costsPerUnit, qty });
      continue;
    }
    const short = sub(room.needs, room.owns, 'the space it is short of');
    if (short <= 0) continue;
    const worth = perUnitWorth(ctx, view, region);
    if (worth === undefined) continue;
    const qty = downTick(short);
    if (qty > 0) out.push({ party: p.id, side: 'buy', price: worth, qty });
  }
  return out;
}

/**
 * A3, Law 3: WHAT A UNIT OF SPACE IS WORTH TO A TAKER — the value of the thing that would otherwise
 * be lost, per unit of the space it occupies, at the party's own mark. It is its own number and not
 * a rate: a holder of something dear will outbid a holder of something cheap for the same shed,
 * which is what a storage market is for.
 */
function perUnitWorth(
  ctx: MechanismContext,
  view: ParticipantView,
  region: RegionId,
): number | undefined {
  let dearest: number | undefined;
  for (const h of view.holdings()) {
    const i = ctx.instruments.get(h.instrument);
    if (!i.status.live || !isGoodTerms(i.terms)) continue;
    const per = i.terms.storagePerUnit;
    if (per === null || i.terms.region !== region) continue;
    const perNamed = ctx.params.ratio(per);
    if (perNamed <= 0) continue;
    const mark = view.mark(h.instrument);
    if (!mark.some) continue;
    // What one piece of space saves it: the mark of a piece of the thing, over the pieces of space
    // that piece takes. It is a RATIO between two counts and is not on any grid — rounding it to a
    // whole piece of space makes it nothing, and a taker that thought space was worth nothing would
    // never bid for any (Law 8: what is a count lands on the grid; what is a ratio does not).
    const space = spacePerPiece(ctx, i.unit, perNamed);
    if (space <= 0) continue;
    const worth = div(mark.value, space, 'what a piece of space saves it');
    if (dearest === undefined || worth > dearest) dearest = worth;
  }
  return dearest;
}

/**
 * D3, Law 5: the fee is a payment, so it has two named sides and both legs sit in one instruction.
 *
 * Clearing D2: A TAKER RENTS FROM NAMED LETTERS, not from the venue. What cleared is a volume at a
 * rate, and who faces whom is a walk down the two books — the same walk a market's fills are paired
 * by — so every unit of space let has one party on each side of it and the sum of what the takers
 * pay is the sum of what the letters receive, exactly (Law 5).
 */
function lease(
  ctx: MechanismContext,
  region: RegionId,
  ccy: CurrencyCode,
  said: Map<string, Record<string, unknown>>,
): void {
  const orders = schedules(ctx, region);
  if (orders.length === 0) return;
  const outcome = clear(orders, 'proRata', 'sellersCompete');
  // Clearing C4.b: WHAT THE SESSION DID, said out loud — including that it did not clear and why.
  // A market in space that never clears is a finding about the world (nobody is short, or nobody
  // has spare, or the two do not overlap), and a session that reported nothing would hide it.
  //
  // ONE EVENT FOR THE PERIOD, keyed by region, which is how the labour venue publishes a going
  // wage: a reader asks for the last one of its kind and looks up the place it is in (Observer A3),
  // and a reader that had to walk back through every region's would be counting on how many there
  // are (Law 4).
  said.set(String(region), {
    outcome: outcome.kind,
    takers: orders.filter((o) => o.side === 'buy').length,
    letters: orders.filter((o) => o.side === 'sell').length,
    ...(isCleared(outcome) ? { rate: outcome.price, space: outcome.volume } : {}),
  });
  if (!isCleared(outcome)) return;
  const rate = outcome.price;
  const held = ctx.state<Leases>('commodities.leases', () => ({ byParty: new Map() }));
  const letters = outcome.fills.filter((f) => f.side === 'sell' && f.qty > 0).map((f) => ({ ...f }));
  let at = 0;
  for (const taker of outcome.fills) {
    if (taker.side !== 'buy' || taker.qty <= 0) continue;
    let want: number = taker.qty;
    while (want > 0 && at < letters.length) {
      const letter = letters[at];
      if (letter === undefined) break;
      // Law 6: a letter cannot let more space than it has, and a taker cannot take more
      // than it is short of. Arithmetic impossibility on both sides, named at the site.
      const space = atMost(letter.qty as number, want, 'a letter has only the space it has');
      const due = ctx.registry.payable(ccy, mul(space, rate, 'what the space costs for the period'));
      // Law 8, Money A2: A PAYMENT BELOW ONE PIECE OF MONEY IS NOT A PAYMENT. Space let for nothing
      // is free storage, which is the thing this module exists to remove — so the space is NOT let,
      // the taker keeps looking and the letter keeps its room. It is the same answer the wire gives
      // a money leg of nothing, taken here so no room changes hands for it.
      if (due <= 0) {
        at += 1;
        continue;
      }
      {
        const r = ctx.settle({
          legs: [
            {
              kind: 'money',
              from: ctx.accountOf(taker.party, ccy),
              to: ctx.accountOf(letter.party, ccy),
              ccy,
              amount: due,
              fromCell: none(),
              toCell: none(),
            },
          ],
          cause: 'transfer',
          reason: `${taker.party} rents ${space} of space from ${letter.party}`,
        });
        if (r.outcome === 'settled') {
          held.byParty.set(String(taker.party), add(leased(held, taker.party), space, 'space taken'));
          ctx.record(
            'commodities.leased',
            [String(taker.party), String(letter.party)],
            { taker: taker.party, letter: letter.party, region, space, rate, paid: due },
            true,
          );
        }
      }
      want = sub(want, space, 'what it is still short of');
      letters[at] = { ...letter, qty: asQty(sub(letter.qty, space, 'what it has left to let')) };
      if (letters[at]?.qty === 0) at += 1;
    }
  }
}

/** What each party has rented this period. Emptied at the top of every period: a lease is a week's. */
interface Leases {
  readonly byParty: Map<string, number>;
}

/** A party with no row here rented nothing this period, which is a real answer (Appendix A). */
const leased = (held: Leases, party: PartyId): number =>
  zeroIfNone(held.byParty.get(String(party)));

/**
 * The module. It owns the space, not the things kept in it: a commodity is a PHYSICAL LINE and this
 * world already has those (`registry/physical.ts`, the goods module's mechanism). What was missing
 * is that holding one was free, and that is what this builds.
 */
export function commodities(): SystemModule {
  return {
    id: 'commodities',
    spec: 'Commodities Spot A3, A4, D2.a, D3, F2',
    // A4: space is plant, so the kind has to be registered before anybody can hold a unit of it;
    // and what takes up space is a good, so the goods have to exist to take any up.
    requires: ['goods', 'capital-programme'],
    instrumentKinds: [],
    derivativeKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [],
    params: [],
    participants: [],
    families: [],
    phases: [
      {
        name: 'commodities.storage',
        spec: 'Commodities Spot A3 Commodities Spot D3 Clearing A2 Clearing A3',
        // BEFORE ANYBODY DECIDES WHAT TO MAKE. What room a line has is what binds how much of a
        // thing it can have at the end of the period (Capital Programme A2, D4), so a firm that
        // wants to make more rents the room first and decides afterwards, knowing what it has
        // (Clearing F1: a decision acts on what has already happened).
        cycle: 0,
        anchor: { before: 'firms.decide' },
        run: (ctx: MechanismContext): void => {
          const said = new Map<string, Record<string, unknown>>();
          for (const r of ctx.registry.regions.values()) lease(ctx, r.id, ctx.registry.currencyOf(r.id), said);
          ctx.record(STORAGE_SESSION, [...said.keys()], { byRegion: Object.fromEntries(said) }, true);
        },
      },
    ],
  };
}
