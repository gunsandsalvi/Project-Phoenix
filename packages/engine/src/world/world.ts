/**
 * The world: every store, and the one period loop.
 *
 * @spec Money G1 Money G2 Money G4 Clearing F1 Clearing F1.a Clearing F3 Audit C1 Audit C2 Audit C3 Currency D3 Seed A5 Observer E3
 *
 * A period is an ordered list of phases held as data. Every phase runs every period (Audit C3);
 * markets clear at their stated point (Clearing F1); revaluation runs after every market has printed
 * and before the audit (Currency D3); the audit closes the period (Audit C1, C2).
 */
import { Audit, type AuditReport, type Reads } from '../audit/audit.js';
import { standardFamilies } from '../audit/families/index.js';
import { type AuditMemory, emptyMemory, remember } from '../audit/memory.js';
import type { AuditView } from '../audit/view.js';
import {
  type Calendar,
  type Cycle,
  type Period,
  nextPeriod,
  period,
} from '../calendar/calendar.js';
import { forbid } from '../core/assert.js';
import { Missing } from '../core/errors.js';
import {
  type CurrencyCode,
  type MarketId,
  moneyInstrumentId,
  paramId,
  type PartyId,
} from '../core/ids.js';
import { addTo } from '../core/num.js';
import { runMarket, type MarketDecl, type MarketResult } from '../clearing/market.js';
import type { Order } from '../clearing/solver.js';
import { Journal } from '../journal/journal.js';
import { Ledger } from '../ledger/ledger.js';
import { Settlement } from '../ledger/settlement.js';
import { Parties } from '../parties/party.js';
import { weightOf } from '../parties/party.js';
import { PriceStore } from '../prices/price-store.js';
import { Valuation } from '../prices/value.js';
import { Instruments } from '../register/instruments.js';
import { Register } from '../register/register.js';
import { INSTRUMENT_PROFILES } from '../registry/profiles.js';
import type { ParamRegister } from '../registry/params.js';
import type { Registry } from '../registry/registry.js';
import { type Prng, prng } from '../rng/prng.js';
import { accountResolver, runCorporateActions } from './actions.js';
import { revalue } from './revalue.js';

/** A participant's reason to be in a market, expressed as orders (Clearing A3, B2). */
export type OrderProvider = (market: MarketDecl, world: World) => Order[];

export interface Phase {
  readonly name: string;
  readonly spec: string;
  /** The settlement cycle this phase runs in (Money G2). */
  readonly cycle: number;
  run(world: World): void;
}

export interface PeriodReport {
  readonly period: Period;
  readonly markets: readonly MarketResult[];
  readonly audit: AuditReport;
}

export interface WorldSpec {
  readonly seed: string;
  readonly registry: Registry;
  readonly params: ParamRegister;
  readonly calendar: Calendar;
}

export class World {
  readonly seed: string;
  readonly registry: Registry;
  readonly params: ParamRegister;
  readonly calendar: Calendar;
  readonly parties = new Parties();
  readonly instruments = new Instruments();
  readonly register: Register;
  readonly prices = new PriceStore();
  readonly valuation: Valuation;
  readonly ledger = new Ledger();
  readonly journal = new Journal();
  readonly settlement: Settlement;
  readonly rng: Prng;
  readonly accountOf: ReturnType<typeof accountResolver>;
  private readonly marketList: MarketDecl[] = [];
  private readonly providers: OrderProvider[] = [];
  private readonly phaseList: Phase[];
  private readonly audit: Audit;
  private readonly memory: AuditMemory = emptyMemory();
  private currentPeriod: Period = period(0);
  private currentCycle: Cycle;
  private lastReport: PeriodReport | undefined;
  private lastMarkets: MarketResult[] = [];

  constructor(spec: WorldSpec) {
    this.seed = spec.seed;
    this.registry = spec.registry;
    this.params = spec.params;
    this.calendar = spec.calendar;
    this.register = new Register(this.parties);
    this.valuation = new Valuation(this.instruments, this.prices);
    this.rng = prng(spec.seed);
    this.accountOf = accountResolver(this.parties);
    this.settlement = new Settlement({
      registry: this.registry,
      calendar: this.calendar,
      parties: this.parties,
      instruments: this.instruments,
      register: this.register,
      prices: this.prices,
      valuation: this.valuation,
      ledger: this.ledger,
      journal: this.journal,
    });
    this.currentCycle = this.calendar.cycle(0);
    this.audit = new Audit(
      standardFamilies(this.memory),
      this.params.get(paramId('audit.worstInstances')),
    );
    this.phaseList = [
      {
        name: 'corporateActions',
        spec: 'Register E1 Register E2',
        cycle: 0,
        run: (w) => {
          runCorporateActions(w.period, w.cycle, {
            calendar: w.calendar,
            parties: w.parties,
            instruments: w.instruments,
            register: w.register,
            settlement: w.settlement,
            journal: w.journal,
            accountOf: w.accountOf,
          });
        },
      },
      {
        name: 'markets',
        spec: 'Clearing F1',
        cycle: 1,
        run: (w) => {
          w.lastMarkets = w.marketList.map((m) => w.runOne(m));
        },
      },
      {
        name: 'revaluation',
        spec: 'Clearing D4 Currency D3',
        cycle: this.calendar.cyclesPerPeriod - 1,
        run: (w) => {
          revalue(w.period, w.cycle, {
            parties: w.parties,
            instruments: w.instruments,
            register: w.register,
            valuation: w.valuation,
            journal: w.journal,
          });
        },
      },
    ];
  }

  get period(): Period {
    return this.currentPeriod;
  }

  get cycle(): Cycle {
    return this.currentCycle;
  }

  get markets(): readonly MarketDecl[] {
    return this.marketList;
  }

  get phases(): readonly Phase[] {
    return this.phaseList;
  }

  get last(): PeriodReport | undefined {
    return this.lastReport;
  }

  addMarket(m: MarketDecl): void {
    forbid(!this.marketList.some((x) => x.id === m.id), 'Law 4', `market ${m.id} declared twice`);
    const inst = this.instruments.get(m.instrument);
    forbid(
      INSTRUMENT_PROFILES[inst.kind].pricing === 'cleared',
      'Clearing D1',
      `${inst.id} is not priced by clearing`,
    );
    forbid(
      inst.market.some && inst.market.value === m.id,
      'Law 4',
      `${inst.id} names market ${inst.market.some ? inst.market.value : 'none'}, not ${m.id}`,
    );
    forbid(
      inst.ccy === m.ccy,
      'Money A2.b',
      `market ${m.id} clears ${inst.id} in ${m.ccy}, instrument is ${inst.ccy}`,
    );
    this.marketList.push(m);
  }

  market(id: MarketId): MarketDecl {
    const m = this.marketList.find((x) => x.id === id);
    if (m === undefined) throw new Missing('Clearing D1', `market ${id} does not exist`);
    return m;
  }

  /** Mechanisms register the reasons their participants have to be in a market (Clearing B2). */
  addOrderProvider(p: OrderProvider): void {
    this.providers.push(p);
  }

  /** Insert a phase at the position its dependencies put it (Law 10). */
  addPhase(phase: Phase, before?: string): void {
    forbid(
      !this.phaseList.some((p) => p.name === phase.name),
      'Law 4',
      `phase ${phase.name} declared twice`,
    );
    this.calendar.cycle(phase.cycle);
    if (before === undefined) {
      this.phaseList.push(phase);
      return;
    }
    const idx = this.phaseList.findIndex((p) => p.name === before);
    forbid(
      idx >= 0,
      'Law 10',
      `phase ${before} does not exist; a phase is inserted at a named position`,
    );
    this.phaseList.splice(idx, 0, phase);
  }

  /** Run the audit at the current period without stepping (the seed must pass it at period zero: Seed A2). */
  auditNow(): AuditReport {
    const report = this.audit.run(this.view(), this.reads());
    remember(this.view(), this.memory);
    this.journal.record(this.currentPeriod, this.currentCycle, 'audit', [], {
      total: report.total,
    });
    return report;
  }

  /** Advance one period: every phase in order, then the audit (Audit C1). */
  step(): PeriodReport {
    forbid(
      this.memory.period !== undefined,
      'Seed A2',
      'audit the seed (auditNow) before stepping',
    );
    this.currentPeriod = nextPeriod(this.currentPeriod);
    this.currentCycle = this.calendar.cycle(0);
    let lastCycle = 0;
    for (const phase of this.phaseList) {
      forbid(
        phase.cycle >= lastCycle,
        'Money E3',
        `phase ${phase.name} runs in cycle ${phase.cycle} after cycle ${lastCycle}`,
      );
      lastCycle = phase.cycle;
      this.currentCycle = this.calendar.cycle(phase.cycle);
      phase.run(this);
    }
    this.currentCycle = this.calendar.lastCycle;
    const audit = this.audit.run(this.view(), this.reads());
    remember(this.view(), this.memory);
    this.journal.record(this.currentPeriod, this.currentCycle, 'audit', [], { total: audit.total });
    this.lastReport = { period: this.currentPeriod, markets: this.lastMarkets, audit };
    return this.lastReport;
  }

  view(): AuditView {
    return {
      period: this.currentPeriod,
      calendar: this.calendar,
      registry: this.registry,
      params: this.params,
      parties: this.parties,
      instruments: this.instruments,
      register: this.register,
      prices: this.prices,
      valuation: this.valuation,
      ledger: this.ledger,
      journal: this.journal,
      markets: this.marketList,
    };
  }

  /** The money stock per currency is a read of issuers' liabilities (Money A4), never stored. */
  moneyStock(): Record<string, number> {
    const acc = new Map<string, number>();
    for (const i of this.instruments.all()) {
      if (i.kind !== 'money') continue;
      addTo(acc, i.ccy, this.register.heldTotal(i.id).value);
    }
    return Object.fromEntries(acc);
  }

  private runOne(m: MarketDecl): MarketResult {
    const orders = this.providers.flatMap((p) => p(m, this));
    return runMarket(m, orders, this.currentPeriod, this.currentCycle, {
      parties: this.parties,
      prices: this.prices,
      settlement: this.settlement,
      journal: this.journal,
      accountOf: this.accountOf,
    });
  }

  private reads(): Reads {
    const populations = new Map<string, number>();
    for (const p of this.parties.alive()) {
      if (p.representation !== 'cell') continue;
      addTo(populations, p.kind, weightOf(p));
    }
    const inPeriod = this.journal.inPeriod(this.currentPeriod);
    const params = this.params.report();
    return {
      moneyStock: this.moneyStock(),
      populations: Object.fromEntries(populations),
      stalePrints: inPeriod.filter((e) => e.kind === 'print' && e.data['stale'] === true).length,
      reserveOverdrafts: inPeriod.filter((e) => e.kind === 'reserve.overdraft').length,
      failedInstructions: this.ledger
        .inPeriod(this.currentPeriod)
        .filter((r) => r.outcome === 'failed').length,
      placeholders: params.counts.placeholder,
      shapes: params.counts.shape,
    };
  }

  /** A party's balance at its own bank in a currency, per member (Money B1). */
  cash(party: PartyId, ccy: CurrencyCode): number {
    const p = this.parties.get(party);
    return this.register.quantity(p.id, moneyInstrumentId(p.bank, ccy));
  }
}
