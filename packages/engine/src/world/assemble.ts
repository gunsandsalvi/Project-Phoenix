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
import { balanceSheet, type BalanceReads } from '../audit/families/accounts.js';
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
  pieceShift: paramId('resolution.pieceShift'),
} as const;

export function assemble(spec: AssemblySpec): World {
  const modules = orderModules(spec.modules);
  const declared = [...spec.params, ...modules.flatMap((m) => m.params)];
  // How fine every unit's grid is, is itself a declared number (Law 8), and the registry is built
  // on the answer — so the register is built twice out of the one list of declarations. The first
  // has no units and can answer only the numbers that are not amounts of one, which is what
  // `pieceShift` is; the second is built against the registry and holds every declared AMOUNT as
  // the count of pieces the state actually counts in (Law 2: the amounts then move with the grid).
  const registry = new Registry(
    {
      ...spec.registry,
      units: [...spec.registry.units, ...modules.flatMap((m) => m.units)],
      instrumentKinds: [moneyKind, ...modules.flatMap((m) => m.instrumentKinds)],
      partyKinds: [...KERNEL_PARTY_KINDS, ...modules.flatMap((m) => m.partyKinds)],
      curveFamilies: modules.flatMap((m) => m.curveFamilies),
    },
    new ParamRegister(declared).get(KERNEL_PARAMS.pieceShift),
  );
  const params = new ParamRegister(declared, registry);
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
    for (const p of m.venueParticipants ?? []) world.addVenueParticipant(p);
    const outlooks = m.outlooks;
    if (outlooks !== undefined) world.provideOutlooks(m.id, outlooks);
    for (const d of m.creditDecisions ?? []) world.provideCreditDecision(m.id, d.partyKind, d.decide);
    for (const d of m.bankChoices ?? []) {
      world.provideBankChoice(m.id, d.partyKind, (view) => d.chooses(view));
    }
    for (const k of m.resolves ?? []) world.provideResolution(m.id, k);
    for (const i of m.indices?.(world.params) ?? []) world.addIndex(i, m.id);
    for (const v of m.marks ?? []) world.provideMark(m.id, v.instrumentKind, v.value);
    requireBankChoices(m);
  }
  const ctx = seedContext(world);
  for (const m of modules) m.seed?.(ctx);
  stateEquityAsRead(world);
  world.seal();
  return world;
}

/**
 * Banks Funding A1.d, E1: A MODULE THAT DECLARES A DEPOSITOR MUST SAY HOW IT LEAVES.
 *
 * A party kind with a `depositClass` is somebody's deposit base — it is what a bank prices its
 * board against and pays a premium for. If nothing ever asks it where it wants to bank then it can
 * never leave, and that is A1.d's stickiness arriving as an omission instead of as a cost somebody
 * bears: the bank it funds can pay it less for ever and no run reaches it. So the module that
 * declares the kind declares the reason too, or the world does not open.
 *
 * It asks each MODULE about its OWN declaration rather than asking the registry about every kind,
 * and that is what makes it a check rather than a nuisance: a world assembled from four modules to
 * exercise one kernel door has the kernel's party kinds in it and nothing to speak for them, and a
 * guard that fired there would be refusing a legitimate world for a defect that is not in it.
 *
 * IT COVERS EVERY DEPOSITOR NOW, with no exception list, because `firm` and `household` are
 * declared by `firms` and `households` rather than by the kernel (worklist 11.6). The kernel's own
 * kinds are the ones money needs, and not one of them is anybody's deposit base — so there is no
 * kind this can reach that no module has to answer for.
 */
function requireBankChoices(m: SystemModule): void {
  const answered = new Set((m.bankChoices ?? []).map((d) => String(d.partyKind)));
  for (const kind of m.partyKinds) {
    if (kind.depositClass === null || answered.has(String(kind.id))) continue;
    throw new InvalidRegistry(
      'Banks Funding E1',
      `${m.id} declares ${kind.id} as a "${kind.depositClass}" depositor and never says where it banks`,
    );
  }
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
    valuation: w.valuation,
    openMarket: (decl) => {
      w.addMarket(decl);
    },
    openVenue: (decl) => {
      w.addVenue(decl);
    },
    // Law 8, Seed A3: the opening world holds what EXISTS. A seed states an intent — so much money,
    // so many tonnes — and what a party can actually hold is the whole pieces of it, per member,
    // like every movement afterwards. The seed does not get to open the world off the grid that
    // every later payment has to land on.
    endowMoney: (party: PartyId, ccy: CurrencyCode, perMember: number) => {
      const p = w.parties.get(party);
      const inst = moneyInstrumentId(p.bank, ccy);
      forbid(w.instruments.has(inst), 'Money A1', `${p.bank} issues no money in ${ccy}`);
      const held = w.registry.payable(ccy, perMember);
      if (held <= 0) return;
      store.moneyDelta(p.id, inst, held, w.period, false);
      w.instruments.adjustIssued(inst, held * weightOf(p));
    },
    endowUnits: (
      party: PartyId,
      instrument: InstrumentId,
      perMember: number,
      basisPerUnit: number,
    ) => {
      const p = w.parties.get(party);
      const held = w.registry.deliverable(w.instruments.get(instrument).unit, perMember);
      if (held <= 0) return;
      store.credit(p.id, instrument, held, basisPerUnit, w.period);
      w.instruments.adjustIssued(instrument, held * weightOf(p));
    },
    market: (id) => w.market(id),
  };
}

/**
 * Seed C1, Audit B5.b, Law 4: at period zero the equity IS the read; from then on only events move
 * it. And it is the SAME read — `balanceSheet`, the one the audit checks the account against and the
 * one a public company publishes (Reporting A2).
 *
 * It had its own copy of that sum, and the copy was one conversion short: it added `valueOfLots`
 * across every holding in whatever money the instrument was priced in, where the read it is checked
 * against converts each one into the party's own (Currency D2). At a rate of one the two agreed and
 * nothing showed; at any other rate the world opened with a stated equity that its own balance sheet
 * contradicted, for every party holding anything foreign — a central bank's whole reserve position.
 * Two implementations of one fact is the defect (Law 4), and the fix is the one that deletes one.
 */
function stateEquityAsRead(w: World): void {
  const store = w.seedStore();
  const reads: BalanceReads = {
    period: w.period,
    registry: w.registry,
    parties: { get: (id) => w.parties.get(id) },
    instruments: w.instruments,
    register: store,
    valuation: w.valuation,
  };
  for (const p of w.parties.all()) {
    const sheet = balanceSheet(reads, p.id);
    store.stateEquity(p.id, sheet.assets.value - sheet.liabilities.value);
  }
}
