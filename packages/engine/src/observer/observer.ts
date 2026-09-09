/**
 * The observer surface: everything shown is derived from the state (Observer D3), a stale mark is
 * visibly stale (A1.a), a missing number is missing (F4), and looking changes nothing (E3): the
 * snapshot is a plain copy the engine never reads back.
 *
 * @spec Observer A1 Observer A1.a Observer A2 Observer A3 Observer A4 Observer D1 Observer D3 Observer E1 Observer E3 Observer F1 Observer F2 Observer F4 Law 9
 */
import { formatCivil } from '../calendar/civil.js';
import type { PartyId } from '../core/ids.js';
import { weightOf } from '../parties/party.js';
import type { Print } from '../prices/price-store.js';
import { displayName } from '../registry/naming.js';
import { INSTRUMENT_PROFILES } from '../registry/profiles.js';
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
    issuer: string;
    issued: number;
    unit: string;
    ccy: string;
    live: boolean;
  }[];
  readonly prints: readonly PrintView[];
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
    if (INSTRUMENT_PROFILES[i.kind].pricing !== 'cleared') continue;
    const latest = w.prices.latest(i.id, w.period);
    if (!latest.some) continue;
    const p = latest.value;
    const from = p.provenance.kind === 'stale' ? p.provenance.from : p.period;
    prints.push({
      instrument: i.id,
      name: displayName(i, w.parties),
      price: p.price,
      ccy: p.ccy,
      provenance: p.provenance,
      age: w.period - from,
      period: p.period,
    });
  }
  const positions: PositionView[] = [];
  for (const h of w.register.allHoldings()) {
    if (!visible(h.holder)) continue;
    const i = w.instruments.get(h.instrument);
    const qty = h.lots.reduce((s, l) => s + l.qty, 0);
    let value: number | null = null;
    const pricing = INSTRUMENT_PROFILES[i.kind].pricing;
    if (pricing === 'money') value = qty;
    else if (pricing === 'cleared' && w.prices.latest(i.id, w.period).some)
      value = w.valuation.valueOfLots(
        i.id,
        h.lots,
        w.prices.latest(i.id, w.period).some
          ? (w.prices.latest(i.id, w.period) as { value: Print }).value.period
          : w.period,
      );
    else if (pricing === 'carriedAtCost') value = w.valuation.valueOfLots(i.id, h.lots, w.period);
    positions.push({
      holder: h.holder,
      instrument: i.id,
      name: displayName(i, w.parties),
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
      name: displayName(i, w.parties),
      kind: i.kind,
      issuer: i.issuer,
      issued: i.issued,
      unit: i.unit,
      ccy: i.ccy,
      live: i.status.live,
    })),
    prints,
    positions,
    audit: w.last?.audit ?? null,
    params: w.params.report(),
    moneyStock: w.moneyStock(),
    journal: w.journal
      .tail(journalTail)
      .filter(
        (e) =>
          scope.kind === 'inspector' ||
          e.subjects.includes(scope.party) ||
          e.kind === 'print' ||
          e.kind === 'audit',
      ),
    ledgerLength: w.ledger.length,
    phases: w.phases.map((p) => ({ name: p.name, cycle: p.cycle, spec: p.spec })),
  };
}
