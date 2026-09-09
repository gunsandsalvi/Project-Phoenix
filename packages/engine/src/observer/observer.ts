/**
 * The observer surface: everything shown is derived from the state (Observer D3), a stale mark is
 * visibly stale (A1.a), a missing number is missing (F4), and looking changes nothing (E3): the
 * snapshot is a plain copy the engine never reads back.
 *
 * @spec Sovereign D3 Sovereign D3.b Observer A1 Observer A1.a Observer A2 Observer A3 Observer A4 Observer D1 Observer D3 Observer E1 Observer E3 Observer F1 Observer F2 Observer F4 Law 9
 */
import { formatCivil } from '../calendar/civil.js';
import type { PartyId } from '../core/ids.js';
import { weightOf } from '../parties/party.js';
import { struckIn, type Print } from '../prices/price-store.js';
import { displayName } from '../registry/naming.js';
import type { World } from '../world/world.js';
import type { AuditReport } from '../audit/audit.js';
import type { ParamReport } from '../registry/params.js';
import type { Event } from '../journal/journal.js';

/** A4: an inspector's full view and a participant's partial view are different products. */
export type Scope =
  { readonly kind: 'inspector' } | { readonly kind: 'party'; readonly party: PartyId };

export interface PrintView {
  readonly instrument: string;
  readonly name: string;
  readonly price: number;
  readonly ccy: string;
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

export interface PartyView {
  readonly id: string;
  readonly name: string;
  readonly kind: string;
  readonly representation: string;
  readonly weight: number;
  readonly region: string;
  readonly alive: boolean;
  /** Only in scope (A4): the party's own equity account per member, else null. */
  readonly equityPerMember: number | null;
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
  }[];
  readonly prints: readonly PrintView[];
  readonly curves: readonly CurveView[];
  readonly yields: readonly YieldView[];
  readonly positions: readonly PositionView[];
  readonly audit: AuditReport | null;
  readonly params: ParamReport;
  readonly moneyStock: Readonly<Record<string, number>>;
  readonly journal: readonly Event[];
  readonly ledgerLength: number;
  readonly phases: readonly { name: string; cycle: number; spec: string }[];
}

/** `journalTail`: how many recent events the viewer asked for; the surface adds nothing of its own. */
export function snapshot(w: World, scope: Scope, journalTail: number): Snapshot {
  const visible = (party: PartyId): boolean => scope.kind === 'inspector' || scope.party === party;
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
      equityPerMember:
        visible(p.id) && w.register.hasEquityAccount(p.id) ? w.register.equity(p.id) : null,
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
    })),
    prints,
    curves,
    yields,
    positions,
    audit: w.last?.audit ?? null,
    params: w.params.report(),
    moneyStock: w.moneyStock(),
    journal:
      scope.kind === 'inspector'
        ? w.journal.tail(journalTail)
        : w.journal.visibleTo(scope.party, journalTail),
    ledgerLength: w.ledger.length,
    phases: w.phases.map((p) => ({ name: p.name, cycle: p.cycle, spec: p.spec })),
  };
}
