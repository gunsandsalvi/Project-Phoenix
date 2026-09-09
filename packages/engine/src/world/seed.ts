/**
 * The seed: a complete, consistent opening world that passes the audit at period zero (Seed A1, A2).
 *
 * @spec Seed A1 Seed A2 Seed A3 Seed A4 Seed A5 Seed B1 Seed B1.a Seed B2 Seed B3 Seed B4 Seed C1 Seed C2 Seed C3 Seed C3.a Seed C4 Seed C4.a Seed C4.b Seed E2 XI-14 XI-15 Money A1 Money D2
 *
 * The foundation seed is one region, one currency, a central bank, a treasury, two banks, three firms
 * and four household cells, with reserves, deposits and one sovereign benchmark line outstanding at an
 * opening price. Every deposit is a bank's liability; every bond is the treasury's; nothing exists
 * because something needed it to (A4). Endowments are seed STATE, not parameters; the one number the
 * seed states that a mechanism should produce (the opening price of the line) is registered as a
 * placeholder with its scheduled death (XI-14).
 *
 * Seeded TERMS (Seed C4.b) are permanent structure and are justified here: the line is a ten-year
 * fixed 2% semi-annual bond on ACT/ACT, chosen as a plain benchmark shape a treasury would issue,
 * with a remaining life so the seed has a maturity ahead of it (C3) rather than a bond at issue.
 */
import { Calendar } from '../calendar/calendar.js';
import { civil } from '../calendar/civil.js';
import {
  cohortId,
  currencyCode,
  currencyUnit,
  instrumentId,
  marketId,
  moneyInstrumentId,
  paramId,
  partyId,
  regionId,
  unitId,
  type PartyId,
} from '../core/ids.js';
import { none, some } from '../core/option.js';
import { ANNUAL, SEMI_ANNUAL, rate } from '../core/rate.js';
import type { CellParty, NamedParty } from '../parties/party.js';
import { ParamRegister } from '../registry/params.js';
import { INSTRUMENT_PROFILES } from '../registry/profiles.js';
import { Registry } from '../registry/registry.js';
import { World } from './world.js';

export const PHX = currencyCode('PHX');
export const REGION = regionId('north');
export const PAR = unitId('par');

const CB = partyId('cb.north');
const TREASURY = partyId('treasury.north');
const BANK_A = partyId('bank.a');
const BANK_B = partyId('bank.b');
const GOV_LINE = instrumentId('gov.north.2.0.2036-03-15');
const GOV_MARKET = marketId('mkt.gov.north.2036');

export function foundationRegistry(): Registry {
  return new Registry({
    currencies: [{ code: PHX, name: 'Phoenix unit', centralBank: CB }],
    regions: [{ id: REGION, name: 'North', ccy: PHX }],
    units: [
      { id: currencyUnit(PHX), name: 'PHX', countable: false },
      { id: PAR, name: 'units of par', countable: false },
      { id: unitId('shares'), name: 'shares', countable: true },
    ],
    cohorts: [
      { id: cohortId('working'), name: 'working age', fromAge: 18 },
      { id: cohortId('retired'), name: 'retired', fromAge: 65 },
    ],
    cellKey: ['region', 'cohort', 'bank'],
    lotFlow: 'FIFO',
  });
}

export function foundationParams(): ParamRegister {
  return new ParamRegister([
    {
      id: paramId('calendar.periodDays'),
      value: 7,
      unit: 'days',
      kind: 'resolution',
      owner: 'model',
      why: 'A period is a week (docs/ARCHITECTURE.md 4.7); coarser cannot place a weekly cycle, finer buys nothing yet.',
    },
    {
      id: paramId('calendar.cyclesPerPeriod'),
      value: 5,
      unit: 'cycles',
      kind: 'resolution',
      owner: 'model',
      why: 'Money G1: a period holds more than one settlement cycle; five stands for business days.',
    },
    {
      id: paramId('audit.worstInstances'),
      value: 5,
      unit: 'count',
      kind: 'resolution',
      owner: 'model',
      why: 'Audit D2: how many worst instances a family reports; a reporting depth, not a behaviour.',
    },
    {
      id: paramId('seed.openingPrice.gov.north.2036'),
      value: 0.98,
      unit: 'PHX per unit of par',
      kind: 'placeholder',
      owner: 'model',
      why: 'Seed C4: an opening condition the first clearing replaces; there is no auction yet to print one.',
      standsInFor: {
        mechanism: 'Sovereign C (the auction) and D (the secondary market)',
        worklistItem: '3',
      },
    },
  ]);
}

/** Build the foundation world. Reproducible from the seed value (Seed A5). */
export function foundationWorld(seed: string): World {
  const registry = foundationRegistry();
  const params = foundationParams();
  const calendar = new Calendar({
    epoch: civil(2026, 1, 5),
    periodDays: params.get(paramId('calendar.periodDays')),
    cyclesPerPeriod: params.get(paramId('calendar.cyclesPerPeriod')),
  });
  const w = new World({ seed, registry, params, calendar });
  const p0 = w.period;

  const named = (
    id: PartyId,
    kind: NamedParty['kind'],
    name: string,
    bank: PartyId,
  ): NamedParty => ({
    id,
    kind,
    region: REGION,
    name,
    bank,
    representation: 'named',
    status: { alive: true },
  });
  w.parties.add(named(CB, 'centralBank', 'Central Bank of North', CB));
  w.parties.add(named(TREASURY, 'treasury', 'Treasury of North', CB));
  w.parties.add(named(BANK_A, 'bank', 'Bank A', CB));
  w.parties.add(named(BANK_B, 'bank', 'Bank B', CB));
  w.parties.add(named(partyId('firm.1'), 'firm', 'Firm One', BANK_A));
  w.parties.add(named(partyId('firm.2'), 'firm', 'Firm Two', BANK_A));
  w.parties.add(named(partyId('firm.3'), 'firm', 'Firm Three', BANK_B));

  const cell = (id: string, cohort: string, bank: PartyId, weight: number): CellParty => ({
    id: partyId(id),
    kind: 'household',
    region: REGION,
    name: `Households ${cohort} at ${bank}`,
    bank,
    representation: 'cell',
    status: { alive: true },
    weight,
    key: { region: REGION, cohort: cohortId(cohort), bank },
  });
  w.parties.add(cell('hh.working.a', 'working', BANK_A, 1000));
  w.parties.add(cell('hh.working.b', 'working', BANK_B, 800));
  w.parties.add(cell('hh.retired.a', 'retired', BANK_A, 500));
  w.parties.add(cell('hh.retired.b', 'retired', BANK_B, 600));

  // Money instruments: one per issuer (Money A1, D2).
  for (const issuer of [CB, BANK_A, BANK_B]) {
    w.instruments.add({
      id: moneyInstrumentId(issuer, PHX),
      kind: 'money',
      issuer,
      ccy: PHX,
      unit: currencyUnit(PHX),
      terms: { kind: 'money' },
      issued: 0,
      status: { live: true },
      market: none(),
    });
  }

  // The sovereign benchmark line, outstanding with a remaining life (Seed C3).
  w.instruments.add({
    id: GOV_LINE,
    kind: 'sovereign.bond',
    issuer: TREASURY,
    ccy: PHX,
    unit: PAR,
    terms: {
      kind: 'sovereign.bond',
      coupon: rate(0.02, ANNUAL),
      couponPeriodicity: SEMI_ANNUAL,
      dayCount: 'ACT/ACT',
      issueDate: civil(2026, 3, 15),
      maturity: civil(2036, 3, 15),
    },
    issued: 0,
    status: { live: true },
    market: some(GOV_MARKET),
  });
  w.addMarket({
    id: GOV_MARKET,
    name: 'North 2% 2036',
    instrument: GOV_LINE,
    ccy: PHX,
    rationing: 'proRata',
  });

  const opening = params.get(paramId('seed.openingPrice.gov.north.2036'));
  w.prices.write({
    instrument: GOV_LINE,
    market: GOV_MARKET,
    period: p0,
    price: opening,
    ccy: PHX,
    provenance: { kind: 'opening' },
  });

  // Endowments, per member for cells (XI-15). Every deposit is somebody's liability (Seed A4, C2).
  const deposit = (holder: string, perMember: number): void => {
    const p = w.parties.get(partyId(holder));
    const inst = moneyInstrumentId(p.bank, PHX);
    w.register.moneyDelta(p.id, inst, perMember, p0, false);
    w.instruments.adjustIssued(inst, perMember * (p.representation === 'cell' ? p.weight : 1));
  };
  const bond = (holder: string, perMember: number): void => {
    const p = w.parties.get(partyId(holder));
    w.register.credit(p.id, GOV_LINE, perMember, opening, p0);
    w.instruments.adjustIssued(GOV_LINE, perMember * (p.representation === 'cell' ? p.weight : 1));
  };

  deposit('treasury.north', 500);
  deposit('bank.a', 400);
  deposit('bank.b', 300);
  deposit('firm.1', 200);
  deposit('firm.2', 150);
  deposit('firm.3', 250);
  deposit('hh.working.a', 0.3);
  deposit('hh.working.b', 0.25);
  deposit('hh.retired.a', 0.4);
  deposit('hh.retired.b', 0.5);

  bond('cb.north', 1500);
  bond('bank.a', 500);
  bond('bank.b', 500);
  bond('hh.working.a', 0.2);
  bond('hh.working.b', 0.125);
  bond('hh.retired.a', 0.22);
  bond('hh.retired.b', 0.15);

  // Equity at period zero is the read (Seed C1); from now on only events move it (Audit B5.b).
  for (const p of w.parties.all()) {
    let assets = 0;
    for (const h of w.register.holdingsOf(p.id))
      assets += w.valuation.valueOfLots(h.instrument, h.lots, p0);
    let liabilities = 0;
    for (const inst of w.instruments.all()) {
      if (inst.issuer !== p.id || !INSTRUMENT_PROFILES[inst.kind].liabilityOfIssuer) continue;
      for (const holder of w.register.holdersOf(inst.id)) {
        const h = w.register.holding(holder, inst.id);
        if (!h.some) continue;
        const hp = w.parties.get(holder);
        liabilities +=
          w.valuation.valueOfLots(inst.id, h.value.lots, p0) *
          (hp.representation === 'cell' ? hp.weight : 1);
      }
    }
    // Holdings are per member already; liabilities are held by others in total (XI-15).
    const perMember = assets - liabilities / (p.representation === 'cell' ? p.weight : 1);
    w.register.stateEquity(p.id, perMember);
  }
  return w;
}
