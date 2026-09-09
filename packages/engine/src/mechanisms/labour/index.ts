/**
 * Labour: hours of a person's time, supplied by a named household to a named firm, at a wage that
 * clears — and a relationship that persists between them.
 *
 * @spec Labour A1 Labour A2 Labour A3 Labour A3.a Labour A4 Labour A4.a Labour A4.b Labour A4.c Labour B1 Labour B1.a Labour B2 Labour B3 Labour B4 Labour B5 Labour C2 Labour C3 Labour C4 Labour C5 Labour D1 Labour D1.a Labour D1.c Labour D2 Labour D3 Labour D5 Labour E1 Labour F1 Labour F2 Labour F3 XI-10 XI-15 Law 2 Law 15
 *
 * The module owns the employment register and the venue where jobs are struck; it does not own the
 * decision to hire, which is the firm's (C1, worklist 4.5), nor what a household does with its wage
 * (worklist 4.6). An employer states the hours it wants at the wage it offers by posting into the
 * venue for its region and occupation; the difference against what it already employs is a vacancy
 * or a separation, and this module executes it.
 *
 * Unemployment is a READ here and nowhere a number (F3): the cells that participate and hold no row.
 * The wage bill, the going rate and the separation flow are reads over the same rows (A4.a, D1.c).
 */
import type { Family, Violation } from '../../audit/audit.js';
import {
  paramId,
  unitId,
  venueId,
  type RegionId,
  type VenueId,
} from '../../core/ids.js';
import { addTo, sum, zeroIfNone } from '../../core/num.js';
import { weightOf } from '../../parties/party.js';
import type { ParamDecl } from '../../registry/params.js';
import { HOUSEHOLD } from '../../registry/profiles.js';
import type { MechanismContext, SeedContext } from '../../world/context.js';
import type { SystemModule } from '../../world/module.js';
import { LABOUR_NUMBERS, OCCUPATIONS, type OccupationDecl } from './data.js';
import { payWages, publishGoingRate, runVenue, type LabourParams } from './matching.js';
import { allRows, emptyBook, rowOfWorker, type EmploymentBook } from './register.js';

export * from './data.js';
export * from './register.js';

export const HOURS = unitId('hours');

export const LABOUR_PARAMS = {
  hoursPerMember: paramId('labour.hoursPerMember'),
  retirementAge: paramId('labour.retirementAge'),
  hiringLag: paramId('labour.hiring.lagPeriods'),
  severance: paramId('labour.severance.periods'),
} as const;

/** The venue where one occupation's jobs in one region are struck (Labour D1). */
export const labourVenue = (region: RegionId, occupation: string): VenueId =>
  venueId(`labour.${region}.${occupation}`);

function numbers(ctx: MechanismContext): LabourParams {
  return {
    hoursPerMember: ctx.params.get(LABOUR_PARAMS.hoursPerMember),
    retirementAge: ctx.params.get(LABOUR_PARAMS.retirementAge),
    hiringLagPeriods: ctx.params.get(LABOUR_PARAMS.hiringLag),
    severancePeriods: ctx.params.get(LABOUR_PARAMS.severance),
  };
}

function paramsOf(): ParamDecl[] {
  return [
    {
      id: LABOUR_PARAMS.hoursPerMember,
      value: LABOUR_NUMBERS.hoursPerMember,
      unit: 'hours per person per period',
      kind: 'technology',
      owner: 'model',
      why: 'Labour A1, B2: what one person has to sell in a week. The workforce is people and this is their time, so a headcount and an hour count are the same fact read two ways (F2).',
    },
    {
      id: LABOUR_PARAMS.retirementAge,
      value: LABOUR_NUMBERS.retirementAge,
      unit: 'years',
      kind: 'policy',
      owner: 'parliament',
      why: 'Labour B3: the age from which a cohort is out of the workforce. It is a policy and parliament owns it from worklist 14; until then it stands at the age the cohorts were drawn around.',
    },
    {
      id: LABOUR_PARAMS.hiringLag,
      value: LABOUR_NUMBERS.hiringLagPeriods,
      unit: 'periods',
      kind: 'technology',
      owner: 'model',
      why: 'Labour C2: finding somebody is not having them. The person is paid from the start and productive after this, which is what makes a hire an investment rather than a switch.',
    },
    {
      id: LABOUR_PARAMS.severance,
      value: LABOUR_NUMBERS.severancePeriods,
      unit: 'periods of pay',
      kind: 'policy',
      owner: 'parliament',
      why: 'Labour C3: what a firing costs, paid to the person separated. It is the cost that makes a firm hold labour through a soft patch and shed it when it is sure, and the asymmetry with hiring is where the employment cycle comes from. A pair of adjustment speeds is not this.',
    },
  ];
}

/**
 * B5, A4.c, F2: every row's headcount is its cell's whole weight, nobody holds two jobs, and
 * employed plus unemployed plus inactive is the population — exactly, because a cell is only ever
 * in one state. The two records are the register's own headcounts and the cells' weights.
 */
function workforceIdentity(book: EmploymentBook): Family {
  return {
    name: 'units',
    contributor: 'labour',
    spec: 'Labour A4.c Labour B3 Labour B5 Labour F2',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      const v = (spec: string, owner: string, size: number, message: string): void => {
        out.push({
          family: 'units',
          spec,
          owner,
          size,
          unit: 'people',
          period: view.period,
          message,
        });
      };
      const held = new Set<string>();
      const rowsHere = new Map<string, number>();
      for (const row of allRows(book)) {
        if (!view.parties.has(row.worker)) {
          v('Labour A4.a', row.worker, row.headcount, `row ${row.id} names a worker that is not here`);
          continue;
        }
        const cell = view.parties.get(row.worker);
        const weight = weightOf(cell);
        // A4.c: a cell with some members employed and some not is two populations in one state.
        if (row.headcount !== weight) {
          v(
            'Labour A4.c',
            row.worker,
            row.headcount - weight,
            `row ${row.id} carries ${row.headcount} of a cell of ${weight}`,
          );
        }
        if (held.has(row.worker)) v('Labour B3', row.worker, 1, `${row.worker} holds two jobs`);
        held.add(row.worker);
        if (!cell.status.alive) {
          v('Labour F1', row.worker, row.headcount, `row ${row.id} employs a cell that has ceased`);
        }
        addTo(rowsHere, row.region, row.headcount);
      }
      const retirementAge = view.params.get(LABOUR_PARAMS.retirementAge);
      const byRegion = new Map<string, { people: number; employed: number; states: number }>();
      for (const p of view.parties.ofKind(HOUSEHOLD)) {
        if (p.representation !== 'cell' || !p.status.alive) continue;
        const acc = byRegion.get(p.region) ?? { people: 0, employed: 0, states: 0 };
        const working = view.registry.cohort(p.key.cohort).fromAge < retirementAge;
        const isEmployed = rowOfWorker(book, p.id) !== undefined;
        acc.people += p.weight;
        acc.employed += isEmployed ? p.weight : 0;
        // B3: exactly one of the three, counted once each way round.
        acc.states +=
          (isEmployed ? p.weight : 0) +
          (!isEmployed && working ? p.weight : 0) +
          (!isEmployed && !working ? p.weight : 0);
        byRegion.set(p.region, acc);
      }
      for (const [region, acc] of byRegion) {
        if (acc.states !== acc.people) {
          v(
            'Labour B5',
            region,
            acc.states - acc.people,
            `employed plus unemployed plus inactive is ${acc.states} against ${acc.people} people`,
          );
        }
        const inRows = zeroIfNone(rowsHere.get(region));
        if (inRows !== acc.employed) {
          v(
            'Labour F2',
            region,
            inRows - acc.employed,
            `the register employs ${inRows} and the cells employed hold ${acc.employed}`,
          );
        }
        if (acc.employed > acc.people) {
          v('Labour F2', region, acc.employed - acc.people, `more people employed than live there`);
        }
      }
      const stray = sum([...rowsHere].filter(([r]) => !byRegion.has(r)).map(([, n]) => n));
      if (stray.value > 0) {
        v('Labour F2', 'workforce', stray.value, `rows employ people in a region with no population`);
      }
      return out;
    },
  };
}

/** F1: no employment without an employer — every row is a job at a named firm that still exists. */
function employersExist(book: EmploymentBook): Family {
  return {
    name: 'names',
    contributor: 'labour',
    spec: 'Labour F1',
    built: true,
    check: (view) => {
      const out: Violation[] = [];
      for (const row of allRows(book)) {
        const known = view.parties.has(row.employer);
        if (known && view.parties.get(row.employer).status.alive) continue;
        out.push({
          family: 'names',
          spec: 'Labour F1',
          owner: row.employer,
          size: row.headcount,
          unit: 'people',
          period: view.period,
          message: known
            ? `row ${row.id} is a job at ${row.employer}, which has ceased`
            : `row ${row.id} is a job at ${row.employer}, which does not exist`,
        });
      }
      return out;
    },
  };
}

/**
 * Built per world: the employment register is this world's own, and the audit reads the same rows
 * the phases write — one book, one writer (Law 4).
 */
export function labour(occupations: readonly OccupationDecl[] = OCCUPATIONS): SystemModule {
  const book = emptyBook();
  // The observer sees the register as the data it is; the slot holds this very object.
  const bookOf = (ctx: MechanismContext): EmploymentBook =>
    ctx.state<EmploymentBook>('employment', () => book);
  const mine = (v: { readonly clearedBy: string }): boolean => v.clearedBy === 'labour';
  return {
    id: 'labour',
    spec: 'Labour, XI-10',
    // A cell decides what it will work for from its own outlook of what it lives on (B1.a).
    requires: ['expectations'],
    instrumentKinds: [],
    partyKinds: [],
    curveFamilies: [],
    units: [{ id: HOURS, name: 'hours', countable: false }],
    params: paramsOf(),
    phases: [
      {
        name: 'labour.match',
        spec: 'Labour C3 Labour C5 Labour D1 Labour D1.a Labour D1.c Labour D3',
        cycle: 1,
        anchor: { before: 'markets' },
        run: (ctx: MechanismContext) => {
          const b = bookOf(ctx);
          const p = numbers(ctx);
          publishGoingRate(ctx, b, ctx.venues.filter(mine), ctx.period);
          for (const v of ctx.venues.filter(mine)) runVenue(ctx, b, v, p);
        },
      },
      {
        name: 'labour.pay',
        spec: 'Labour E1 Labour E2 Labour F1',
        cycle: 2,
        anchor: { after: 'markets' },
        run: (ctx: MechanismContext) => {
          payWages(ctx, bookOf(ctx));
        },
      },
    ],
    participants: [],
    families: [workforceIdentity(book), employersExist(book)],
    seed(ctx: SeedContext): void {
      for (const region of ctx.registry.regions.values()) {
        for (const o of occupations) {
          ctx.openVenue({
            id: labourVenue(region.id, o.id),
            name: `${o.name}, ${region.name}`,
            clearedBy: 'labour',
            unit: HOURS,
            ccy: region.ccy,
            key: { region: region.id, occupation: o.id, skill: o.skill, sector: o.sector },
          });
        }
      }
    },
  };
}
