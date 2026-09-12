/**
 * The voyage store: what is on its way somewhere, and where it has got to.
 *
 * @spec Freight A1 Freight A3 Freight A3.a Freight B2 Freight B4 Freight D4 Freight E1 Freight E2 Freight E3 Law 4 Law 6 Law 19
 *
 * WHY IT IS A STORE AND NOT A HOLDING. What is on a voyage IS a holding — units of an in-transit
 * instrument on the shipper's own book, which is A3.a's working capital — and the register owns
 * that. What the register cannot say is WHERE those units have got to, because a holding is a
 * quantity and not a position. So there are two facts and two writers: the register owns how many
 * and whose, this owns where they are and how far they have left to go (Law 4).
 *
 * WHY THE HULLS ARE LIENED AND NOT MOVED. A carrier's hulls are plant it owns, and plant on a
 * voyage is still its plant — it is simply not available for anything else. The register already
 * refuses to move encumbered units, so a lien is the whole of "a hull cannot be sold, sent on a
 * second voyage, or counted as capacity twice" with no rule written anywhere (Law 12: the fix that
 * removes code; Freight E2: no capacity without a carrier that owns it).
 *
 * WHERE IT IS is `kmTravelled` along `tiles`, so a position is tile-exact and the place it is in is
 * a read of the tile it has reached — which is how the weather finds it (Freight B4).
 *
 * NOTHING HERE DECIDES ANYTHING. Settlement is the one caller, as it is for the register and the
 * contract store: a cargo leaving, getting on, being lost or arriving is a change of state and goes
 * over the wire like every other.
 */
import { forbid } from '../core/assert.js';
import { Missing } from '../core/errors.js';
import type {
  InstrumentId,
  LienId,
  PartyId,
  PlaceId,
  TileIndex,
  VoyageId,
} from '../core/ids.js';
import { atMost, finite } from '../core/num.js';
import type { Qty } from '../core/tick.js';
import type { Period } from '../calendar/calendar.js';

export interface VoyageDecl {
  /** B1: the named party whose hulls are carrying it. */
  readonly carrier: PartyId;
  /** A2: the named party whose goods they are, the whole way (A3.a). */
  readonly shipper: PartyId;
  /** The capital kind doing the carrying — hulls over water, vehicles over land. Data (Law 15). */
  readonly by: string;
  /** The tiles it crosses, in order, and how far that is. Read off the map and never declared. */
  readonly tiles: readonly TileIndex[];
  readonly km: number;
  /** The units on it, at the in-transit instrument the register holds them under. */
  readonly cargo: InstrumentId;
  readonly qty: Qty;
  /** B2: the lien binding the hulls to this voyage. It releases when the cargo lands. */
  readonly hulls: LienId;
  /** D2: what the shipper agreed to pay for it, which is part of the delivered price. */
  readonly freight: number;
}

export interface Voyage extends VoyageDecl {
  readonly id: VoyageId;
  readonly departed: Period;
  /** A3, D4: how far along its own path it has got. A gale is a period it made little of. */
  readonly kmTravelled: number;
  /** The units still on it: a storm takes hulls and what they were carrying with them. */
  readonly aboard: Qty;
  readonly landed: boolean;
}

/** The tile a voyage has reached, and therefore the place the weather will find it in. */
export function tileReached(v: Voyage): TileIndex {
  const tiles = v.tiles;
  const last = tiles[tiles.length - 1];
  const first = tiles[0];
  if (first === undefined || last === undefined) {
    throw new Missing('Freight A1', `voyage ${v.id} has no path`);
  }
  if (v.km <= 0) return last;
  const through = (v.kmTravelled / v.km) * (tiles.length - 1);
  return tiles[Math.floor(through)] ?? last;
}

export class Voyages {
  private readonly rows = new Map<VoyageId, Voyage>();
  private readonly byParty = new Map<PartyId, Set<VoyageId>>();
  private next = 1;

  /** A1, A2: a journey opens with two named parties on it, and cargo that is somebody's. */
  open(decl: VoyageDecl, at: Period): Voyage {
    forbid(decl.tiles.length > 0, 'Freight A1', 'a voyage goes somewhere, so it has a path');
    forbid(decl.km > 0, 'Freight E1', 'no instantaneous transport: a voyage has a distance');
    forbid(decl.qty > 0, 'Freight A1', 'a voyage carries something');
    forbid(
      decl.carrier !== decl.shipper,
      'Freight A2',
      'freight is bought from somebody: the carrier and the shipper are two parties',
    );
    const id = this.next as VoyageId;
    this.next += 1;
    const row: Voyage = {
      ...decl,
      id,
      departed: at,
      kmTravelled: 0,
      aboard: decl.qty,
      landed: false,
    };
    this.rows.set(id, row);
    for (const p of [decl.carrier, decl.shipper]) {
      const mine = this.byParty.get(p) ?? new Set<VoyageId>();
      mine.add(id);
      this.byParty.set(p, mine);
    }
    return row;
  }

  /**
   * D4, B4: a period's progress. It is added and never set, because how far a thing has come is the
   * sum of what each period let it do — and a period can let it do very little (Law 6: the advance
   * is positive at every wind and never zero, so a voyage crawls rather than stopping).
   */
  advance(id: VoyageId, km: number): Voyage {
    const v = this.get(id);
    forbid(!v.landed, 'Freight A3', `voyage ${id} has already landed`);
    forbid(km > 0, 'Freight B4', 'a period moves a voyage forward, however little');
    const travelled = finite(v.kmTravelled + km, 'how far it has come');
    const row: Voyage = {
      ...v,
      // A voyage cannot come further than its own path is long: that is the arithmetic of a finite
      // journey and not a limit on an outcome (Law 6).
      kmTravelled: atMost(travelled, v.km, 'a journey ends where its path does'),
    };
    this.rows.set(id, row);
    return row;
  }

  /** B4, E3: what a storm took. The units are gone from the shipper's book by the same instruction. */
  lose(id: VoyageId, units: Qty): Voyage {
    const v = this.get(id);
    forbid(!v.landed, 'Freight A3', `voyage ${id} has already landed`);
    forbid(units > 0 && units <= v.aboard, 'Freight E3', 'a voyage loses what it was carrying');
    const row: Voyage = { ...v, aboard: (v.aboard - units) as Qty };
    this.rows.set(id, row);
    return row;
  }

  /** A3, E3: it is there. The cargo is reseated and the lien on the hulls releases. */
  land(id: VoyageId): Voyage {
    const v = this.get(id);
    forbid(!v.landed, 'Freight A3', `voyage ${id} has already landed`);
    const row: Voyage = { ...v, landed: true, kmTravelled: v.km };
    this.rows.set(id, row);
    return row;
  }

  get(id: VoyageId): Voyage {
    const v = this.rows.get(id);
    if (v === undefined) throw new Missing('Freight A1', `voyage ${id} does not exist`, { id });
    return v;
  }

  has(id: VoyageId): boolean {
    return this.rows.has(id);
  }

  /** Every voyage still at sea, in the order they opened. */
  underWay(): readonly Voyage[] {
    return [...this.rows.values()].filter((v) => !v.landed);
  }

  /** A3.a: what a party has on the water, on either side of it. */
  of(party: PartyId): readonly Voyage[] {
    const mine = this.byParty.get(party);
    return mine === undefined ? [] : [...mine].map((id) => this.get(id));
  }

  /** B4: what is in a place right now, which is what a gale there reaches. */
  in(place: PlaceId, placeOf: (t: TileIndex) => PlaceId): readonly Voyage[] {
    return this.underWay().filter((v) => placeOf(tileReached(v)) === place);
  }

  all(): readonly Voyage[] {
    return [...this.rows.values()];
  }
}
