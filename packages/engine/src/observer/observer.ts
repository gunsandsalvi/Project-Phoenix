/**
 * The observer surface: everything shown is derived from the state (Observer D3), a stale mark is
 * visibly stale (A1.a), a missing number is missing (F4), and looking changes nothing (E3): the
 * snapshot is a plain copy the engine never reads back.
 *
 * @spec Sovereign D3 Sovereign D3.b Observer A1 Observer A1.a Observer A2 Observer A3 Observer A4 Observer D1 Observer D3 Observer E1 Observer E3 Observer F1 Observer F2 Observer F4 Law 9
 */
import { formatCivil } from '../calendar/civil.js';
import { periodicityLabel } from '../core/rate.js';
import type { PartyId } from '../core/ids.js';
import { weightOf } from '../parties/party.js';
import { struckIn, type Print } from '../prices/price-store.js';
import { displayName } from '../registry/naming.js';
import type { World } from '../world/world.js';
import type { AuditReport } from '../audit/audit.js';
import type { ParamReport } from '../registry/params.js';
import type { Event, EventKind } from '../journal/journal.js';

/** A4: an inspector's full view and a participant's partial view are different products. */
export type Scope =
  { readonly kind: 'inspector' } | { readonly kind: 'party'; readonly party: PartyId };

export interface PrintView {
  readonly instrument: string;
  readonly name: string;
  /**
   * Law 8: money PIECES for one PIECE of the thing, which is what the state holds. What a person
   * reads is money for one NAMED unit, and turning one into the other needs both subdivisions —
   * this unit's and the currency's, both in `subdivisions`.
   */
  readonly price: number;
  readonly ccy: string;
  /** The unit one of these is counted in, so a reader can put the price in a person's terms. */
  readonly unit: string;
  readonly provenance: Print['provenance'];
  /** Periods since the price last actually traded or opened (A1.a). */
  readonly age: number;
  readonly period: number;
}

/** Sovereign D3: a curve as it is read, with what each point is made of (D3.b). */
export interface CurveView {
  readonly family: string;
  readonly name: string;
  readonly compounding: string;
  readonly points: readonly {
    instrument: string;
    name: string;
    tenorYears: number;
    yield: number;
    provenance: string;
  }[];
}

/** Observer F2: fixed income shows the price AND what is derived from it, never one alone. */
export interface YieldView {
  readonly instrument: string;
  /** Missing when this line is on no curve, never zero (F4). */
  readonly yield: number | null;
  readonly tenorYears: number | null;
}

export interface PositionView {
  readonly holder: string;
  readonly instrument: string;
  readonly name: string;
  readonly qtyPerMember: number;
  readonly qtyTotal: number;
  readonly unit: string;
  /** Missing when the instrument has no mark (F4), never zero. */
  readonly valuePerMember: number | null;
  readonly ccy: string;
}

/**
 * Expectations A1, A2, Observer A2: what a party expects, shown beside the party that expects it.
 * There is no sector outlook and no consensus here, because there is none in the model (A2.b), and
 * a viewer that is a party sees its own and nobody else's (Observer A2: no private state).
 */
export interface OutlookView {
  readonly party: string;
  readonly variable: string;
  readonly expected: number;
  readonly unit: string;
  readonly per: string;
  /** B3: how wide this party's own recent surprises have been — a read, never a stated number. */
  readonly confidence: number;
  readonly formed: number;
}

export interface PartyView {
  readonly id: string;
  readonly name: string;
  readonly kind: string;
  readonly representation: string;
  readonly weight: number;
  readonly region: string;
  readonly alive: boolean;
  /**
   * XI-8, Firm Birth D5: where a reference to it goes now that it has ceased — its estate, or
   * itself where the chain of successors ends. Null while it is still here. Without it a dead party
   * is a name that stops, and the estate winding it up is a party nobody can connect it to.
   */
  readonly successor: string | null;
  /** Only in scope (A4): the party's own equity account per member, else null. */
  readonly equityPerMember: number | null;
  /** The money its own region books in, so a reader knows what the equity above is counted in. */
  readonly ccy: string;
}

/**
 * Banks Funding F1, F2, F4, Banks Capital B3.a, Observer A5: WHAT A BANK LOOKS LIKE FROM OUTSIDE.
 *
 * Every number here is one the bank itself published about itself and nothing is recomputed on the
 * way out (Law 4, Law 19, Observer E3) — its deposit lines by class (F1), its reserve balance as a
 * read of its one account (F2), the liquidity metric somebody outside can see (F4), and where its
 * capital stands against the two rules (B3.a). A5 is why `asOf` and `age` travel with them: a
 * published report is what the bank said AT THE CLOSE, and a surface that showed it as though it
 * were now would be inventing a freshness nobody has.
 */
export interface BankView {
  readonly bank: string;
  readonly ccy: string;
  readonly reserves: number;
  readonly liquid: number;
  readonly couldLeave: number;
  readonly metric: number | null;
  readonly deposits: Readonly<Record<string, number>>;
  readonly base: number;
  readonly capital: number;
  readonly weighted: number;
  readonly assets: number;
  readonly weightedRatio: number | null;
  readonly leverageRatio: number | null;
  readonly binds: string | null;
  readonly breach: boolean;
  readonly limitPerName: number | null;
  /**
   * Dealer Desks A1, D5, E4: what its DEALING LINE is carrying and what room it has left. A bank's
   * dealing book is a sub-ledger of the balance sheet above, not a second party's, so it belongs on
   * the bank's own card rather than on one of its own — which is what a surface showing a "desk"
   * used to say and what it no longer can.
   */
  readonly dealingBook: number | null;
  readonly dealingRoom: number | null;
  /** The period each of those was published in, and how many periods ago that was (A5). */
  readonly asOf: number | null;
  readonly age: number | null;
}

export interface Snapshot {
  readonly seed: string;
  readonly period: number;
  readonly date: string;
  readonly scope: Scope;
  readonly parties: readonly PartyView[];
  readonly instruments: readonly {
    id: string;
    name: string;
    kind: string;
    issuer: string | null;
    issued: number;
    unit: string;
    ccy: string;
    live: boolean;
    /** Banks Lending E2: false once a payment it promised was missed; null once it has ceased. */
    performing: boolean | null;
  }[];
  readonly prints: readonly PrintView[];
  readonly curves: readonly CurveView[];
  readonly yields: readonly YieldView[];
  readonly positions: readonly PositionView[];
  /** F1, F2, F4, B3.a: what each bank last published about its funding and its capital. */
  readonly banks: readonly BankView[];
  readonly audit: AuditReport | null;
  readonly params: ParamReport;
  readonly moneyStock: Readonly<Record<string, number>>;
  /**
   * Law 8, Observer F1: HOW MANY PIECES ONE NAMED UNIT IS, by unit name — a hundred cents to the
   * PHX, a million grams to the tonne.
   *
   * Every quantity in this snapshot is a COUNT OF PIECES, because that is what the state holds and
   * a surface that quietly rewrote them would be a surface with arithmetic of its own in it. What a
   * person reads is the named unit, so the reader is handed the one number that turns one into the
   * other and nothing is converted on the way out.
   */
  readonly subdivisions: Readonly<Record<string, number>>;
  readonly journal: readonly Event[];
  /**
   * Observer B1, D1: the recent events of each kind the viewer said it follows, at the same depth.
   * A page shows things a party says once a period — an issuer's programme, an auction's result —
   * and the feed above is the last N of EVERYTHING, so what it reaches back to shrinks every time
   * the world finds more to say. These are read by kind, so it reaches them whatever else happened.
   */
  readonly followed: Readonly<Record<string, readonly Event[]>>;
  readonly ledgerLength: number;
  readonly phases: readonly { name: string; cycle: number; spec: string }[];
  /** A2: every party's outlooks for an inspector; only its own for a party (A2: private state). */
  readonly outlooks: readonly OutlookView[];
  /**
   * Observer A4: what the modules keep — employment rows, inventories, the outlook book. It is the
   * INSPECTOR's product and is null for a party, because a module's state is other parties' private
   * state as often as not, and a surface that showed it would be showing it to them.
   */
  readonly state: Readonly<Record<string, unknown>> | null;
}

/**
 * `journalTail`: how many recent events the viewer asked for; the surface adds nothing of its own.
 * `follow`: the kinds it also wants the recent events OF, at that same depth. A viewer that names
 * none follows none.
 */
export function snapshot(
  w: World,
  scope: Scope,
  journalTail: number,
  follow: readonly EventKind[] = [],
): Snapshot {
  const visible = (party: PartyId): boolean => scope.kind === 'inspector' || scope.party === party;
  // A3, A4: an inspector sees the journal whole; a party sees what is public and what names it.
  const sees = (e: Event): boolean =>
    scope.kind === 'inspector' || e.public || e.subjects.includes(scope.party);
  const prints: PrintView[] = [];
  for (const i of w.instruments.all()) {
    if (w.registry.instrumentKind(i.kind).pricing !== 'cleared') continue;
    const latest = w.prices.latest(i.id, w.period);
    if (!latest.some) continue;
    const p = latest.value;
    const from = struckIn(p);
    prints.push({
      instrument: i.id,
      name: displayName(i, w.parties, w.registry),
      price: p.price,
      ccy: p.ccy,
      unit: i.unit,
      provenance: p.provenance,
      age: w.period - from,
      period: p.period,
    });
  }
  // Sovereign D3, Observer F2: the curve as the read it is, and the yield beside every price it
  // came from. Nothing here is stored; asking for the surface builds it and changes nothing (E3).
  const curves: CurveView[] = [];
  const yields: YieldView[] = [];
  for (const family of w.registry.curveFamilies.values()) {
    const read = w.curve(family.id);
    curves.push({
      family: family.id,
      name: family.name,
      compounding: read.compounding,
      points: read.points.map((pt) => ({
        instrument: pt.instrument,
        name: displayName(w.instruments.get(pt.instrument), w.parties, w.registry),
        tenorYears: pt.tenorYears,
        yield: pt.yield,
        provenance: pt.provenance,
      })),
    });
    for (const pt of read.points) {
      yields.push({ instrument: pt.instrument, yield: pt.yield, tenorYears: pt.tenorYears });
    }
  }
  for (const p of prints) {
    if (!yields.some((y) => y.instrument === p.instrument)) {
      yields.push({ instrument: p.instrument, yield: null, tenorYears: null });
    }
  }
  const positions: PositionView[] = [];
  for (const h of w.register.allHoldings()) {
    if (!visible(h.holder)) continue;
    const i = w.instruments.get(h.instrument);
    const qty = h.lots.reduce((s, l) => s + l.qty, 0);
    let value: number | null = null;
    const profile = w.registry.instrumentKind(i.kind);
    const latest = w.prices.latest(i.id, w.period);
    if (profile.pricing === 'money') value = qty;
    // A holding carried at cost is worth what it cost whether or not its market printed (Goods E1);
    // one carried at the mark is worth nothing anyone can state until it has one (XI-6).
    else if (profile.carry === 'cost') value = w.valuation.valueOfLots(i.id, h.lots, w.period);
    else if (latest.some) value = w.valuation.valueOfLots(i.id, h.lots, latest.value.period);
    positions.push({
      holder: h.holder,
      instrument: i.id,
      name: displayName(i, w.parties, w.registry),
      qtyPerMember: qty,
      qtyTotal: qty * weightOf(w.parties.get(h.holder)),
      unit: i.unit,
      valuePerMember: value,
      ccy: i.ccy,
    });
  }
  const banks: BankView[] = [];
  for (const p of w.parties.all()) {
    const liquidity = w.journal.lastOf('bank.liquidity', p.id);
    const capital = w.journal.lastOf('bank.capital', p.id);
    const dealing = w.journal.lastOf('bank.dealing', p.id);
    if (liquidity === undefined && capital === undefined) continue;
    const numOf = (e: Event | undefined, key: string): number => {
      const v = e?.data[key];
      return typeof v === 'number' ? v : 0;
    };
    const textOf = (e: Event | undefined, key: string): string | undefined => {
      const v = e?.data[key];
      return typeof v === 'string' ? v : undefined;
    };
    const orNull = (e: Event | undefined, key: string): number | null => {
      const v = e?.data[key];
      return typeof v === 'number' ? v : null;
    };
    const said = liquidity?.period ?? capital?.period ?? null;
    banks.push({
      bank: String(p.id),
      ccy: textOf(liquidity, 'ccy') ?? textOf(capital, 'ccy') ?? '',
      reserves: numOf(liquidity, 'reserves'),
      liquid: numOf(liquidity, 'liquid'),
      couldLeave: numOf(liquidity, 'couldLeave'),
      metric: orNull(liquidity, 'metric'),
      deposits: (liquidity?.data['deposits'] ?? {}) as Readonly<Record<string, number>>,
      base: numOf(liquidity, 'base'),
      capital: numOf(capital, 'capital'),
      weighted: numOf(capital, 'weighted'),
      assets: numOf(capital, 'assets'),
      weightedRatio: orNull(capital, 'weightedRatio'),
      leverageRatio: orNull(capital, 'leverageRatio'),
      binds: typeof capital?.data['binds'] === 'string' ? capital.data['binds'] : null,
      breach: capital?.data['breach'] === true,
      limitPerName: orNull(capital, 'limitPerName'),
      dealingBook: orNull(dealing, 'book'),
      dealingRoom: orNull(dealing, 'roomLeft'),
      asOf: said,
      age: said === null ? null : w.period - said,
    });
  }
  const outlooks: OutlookView[] = [];
  for (const p of w.parties.all()) {
    if (!visible(p.id)) continue;
    for (const variable of w.outlookVariables(p.id)) {
      const o = w.outlookOf(p.id, variable);
      if (!o.some) continue;
      outlooks.push({
        party: p.id,
        variable,
        expected: o.value.expected,
        unit: o.value.unit,
        per: periodicityLabel(o.value.per),
        confidence: o.value.confidence,
        formed: o.value.formed,
      });
    }
  }
  return {
    seed: w.seed,
    period: w.period,
    date: formatCivil(w.calendar.startOf(w.period)),
    scope,
    parties: w.parties.all().map((p) => ({
      id: p.id,
      name: p.name,
      kind: p.kind,
      representation: p.representation,
      weight: weightOf(p),
      region: p.region,
      alive: p.status.alive,
      successor: p.status.alive ? null : p.status.successor,
      equityPerMember:
        visible(p.id) && w.register.hasEquityAccount(p.id) ? w.register.equity(p.id) : null,
      ccy: w.registry.region(p.region).ccy,
    })),
    instruments: w.instruments.all().map((i) => ({
      id: i.id,
      name: displayName(i, w.parties, w.registry),
      kind: i.kind,
      issuer: i.issuer.some ? i.issuer.value : null,
      issued: i.issued,
      unit: i.unit,
      ccy: i.ccy,
      live: i.status.live,
      // Banks Lending E2: a status is only a status if something shows it. A line that stopped
      // performing looks exactly like one that never missed anything unless the surface says so.
      performing: i.status.live ? i.status.performing : null,
    })),
    prints,
    curves,
    yields,
    positions,
    banks,
    audit: w.last?.audit ?? null,
    params: w.params.report(),
    moneyStock: w.moneyStock(),
    subdivisions: Object.fromEntries(
      [...w.registry.units.values()].map((u) => [u.name, w.registry.subdivision(u.id)]),
    ),
    journal:
      scope.kind === 'inspector'
        ? w.journal.tail(journalTail)
        : w.journal.visibleTo(scope.party, journalTail),
    followed: Object.fromEntries(
      follow.map((kind) => [kind, w.journal.recentOfKind(kind, journalTail, sees)]),
    ),
    ledgerLength: w.ledger.length,
    phases: w.phases.map((p) => ({ name: p.name, cycle: p.cycle, spec: p.spec })),
    outlooks,
    state: scope.kind === 'inspector' ? w.stateSlots() : null,
  };
}
