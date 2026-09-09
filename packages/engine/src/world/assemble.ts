/**
 * Assembly: kernel plus modules, in dependency order, then the seed, then the seal.
 *
 * @spec Law 15 Part XIII Seed A1 Seed A2 Seed A5 Seed C1 Audit B5.b XI-14
 *
 * The registry is the union of the kernel's kinds and every module's; the parameter register is the
 * union of every module's declarations; phases are anchored to the kernel's; participants are
 * evaluated per party of their kind. The seed modules write the opening state through SeedContext,
 * the kernel states every equity account once as the read (Seed C1), and the world is sealed with
 * the audit at period zero (Seed A2).
 */
import { Calendar } from '../calendar/calendar.js';
import type { Civil } from '../calendar/civil.js';
import { forbid } from '../core/assert.js';
import { InvalidRegistry } from '../core/errors.js';
import {
  type CurrencyCode,
  type InstrumentId,
  moneyInstrumentId,
  paramId,
  type PartyId,
} from '../core/ids.js';
import { weightOf } from '../parties/party.js';
import { type ParamDecl, ParamRegister } from '../registry/params.js';
import { KERNEL_PARTY_KINDS, moneyKind } from '../registry/profiles.js';
import { Registry, type RegistryData } from '../registry/registry.js';
import type { SeedContext } from './context.js';
import type { SystemModule } from './module.js';
import { World } from './world.js';

/** Everything that assembles a world, as data (Law 15). */
export interface AssemblySpec {
  readonly seed: string;
  readonly epoch: Civil;
  /** Registry data without kinds: kinds come from the kernel and the modules. */
  readonly registry: Omit<RegistryData, 'instrumentKinds' | 'partyKinds'>;
  /** Kernel parameters: calendar and audit resolution. */
  readonly params: readonly ParamDecl[];
  readonly modules: readonly SystemModule[];
}

/** The kernel's own parameter ids every assembly must declare. */
export const KERNEL_PARAMS = {
  periodDays: paramId('calendar.periodDays'),
  cyclesPerPeriod: paramId('calendar.cyclesPerPeriod'),
  worstInstances: paramId('audit.worstInstances'),
} as const;

export function assemble(spec: AssemblySpec): World {
  const modules = orderModules(spec.modules);
  const registry = new Registry({
    ...spec.registry,
    units: [...spec.registry.units, ...modules.flatMap((m) => m.units)],
    instrumentKinds: [moneyKind, ...modules.flatMap((m) => m.instrumentKinds)],
    partyKinds: [...KERNEL_PARTY_KINDS, ...modules.flatMap((m) => m.partyKinds)],
  });
  const params = new ParamRegister([...spec.params, ...modules.flatMap((m) => m.params)]);
  const calendar = new Calendar({
    epoch: spec.epoch,
    periodDays: params.get(KERNEL_PARAMS.periodDays),
    cyclesPerPeriod: params.get(KERNEL_PARAMS.cyclesPerPeriod),
  });
  const world = new World({
    seed: spec.seed,
    registry,
    params,
    calendar,
    families: modules.flatMap((m) => m.families),
  });
  for (const m of modules) {
    for (const p of m.phases) world.addPhase(p, m.id);
    for (const p of m.participants) world.addParticipant(p);
  }
  const ctx = seedContext(world);
  for (const m of modules) m.seed?.(ctx);
  stateEquityAsRead(world);
  world.seal();
  return world;
}

/** Modules in an order that satisfies `requires` (Part XIII), stable for equal rank. */
export function orderModules(modules: readonly SystemModule[]): SystemModule[] {
  const byId = new Map(modules.map((m) => [m.id, m]));
  forbid(byId.size === modules.length, 'Law 4', 'a module id is declared twice');
  const out: SystemModule[] = [];
  const seen = new Set<string>();
  const visiting = new Set<string>();
  const visit = (m: SystemModule): void => {
    if (seen.has(m.id)) return;
    if (visiting.has(m.id))
      throw new InvalidRegistry('Part XIII', `module dependency cycle at ${m.id}`);
    visiting.add(m.id);
    for (const r of m.requires) {
      const dep = byId.get(r);
      if (dep === undefined)
        throw new InvalidRegistry(
          'Part XIII',
          `module ${m.id} requires ${r}, which is not assembled`,
        );
      visit(dep);
    }
    visiting.delete(m.id);
    seen.add(m.id);
    out.push(m);
  };
  for (const m of modules) visit(m);
  return out;
}

function seedContext(w: World): SeedContext {
  const store = w.seedStore();
  return {
    period: w.period,
    calendar: w.calendar,
    registry: w.registry,
    params: w.params,
    rng: w.mechanismContext('seed').rng,
    parties: w.parties,
    instruments: w.instruments,
    register: store,
    prices: w.prices,
    openMarket: (decl) => {
      w.addMarket(decl);
    },
    endowMoney: (party: PartyId, ccy: CurrencyCode, perMember: number) => {
      const p = w.parties.get(party);
      const inst = moneyInstrumentId(p.bank, ccy);
      forbid(w.instruments.has(inst), 'Money A1', `${p.bank} issues no money in ${ccy}`);
      store.moneyDelta(p.id, inst, perMember, w.period, false);
      w.instruments.adjustIssued(inst, perMember * weightOf(p));
    },
    endowUnits: (
      party: PartyId,
      instrument: InstrumentId,
      perMember: number,
      basisPerUnit: number,
    ) => {
      const p = w.parties.get(party);
      store.credit(p.id, instrument, perMember, basisPerUnit, w.period);
      w.instruments.adjustIssued(instrument, perMember * weightOf(p));
    },
    market: (id) => w.market(id),
  };
}

/** Seed C1: at period zero the equity is the read; from then on only events move it (Audit B5.b). */
function stateEquityAsRead(w: World): void {
  const store = w.seedStore();
  for (const p of w.parties.all()) {
    let assets = 0;
    for (const h of store.holdingsOf(p.id)) {
      assets += w.valuation.valueOfLots(h.instrument, h.lots, w.period);
    }
    let liabilities = 0;
    for (const inst of w.instruments.all()) {
      if (inst.issuer !== p.id || !w.registry.instrumentKind(inst.kind).liabilityOfIssuer) continue;
      for (const holder of store.holdersOf(inst.id)) {
        const h = store.holding(holder, inst.id);
        if (!h.some) continue;
        liabilities +=
          w.valuation.valueOfLots(inst.id, h.value.lots, w.period) *
          weightOf(w.parties.get(holder));
      }
    }
    // Holdings are per member already; liabilities are held by others in total (XI-15).
    store.stateEquity(p.id, assets - liabilities / weightOf(p));
  }
}
