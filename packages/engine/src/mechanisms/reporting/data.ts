/**
 * What this module reads, and the one number it declares.
 *
 * @spec Reporting A3 Reporting A4 Reporting A4.a Reporting G6 Law 2 Seed B1.a
 */
import { MONTHS_IN_YEAR } from '../../calendar/civil.js';
import { paramId } from '../../core/ids.js';
import type { PartyId } from '../../core/ids.js';
import { prng } from '../../rng/prng.js';
import type { ParamDecl } from '../../registry/params.js';

export const REPORTING_PARAMS = {
  lag: paramId('reporting.lag.days'),
} as const;

/**
 * A4, A4.a: how long after the books close a report must be out.
 *
 * It is a POLICY and parliament owns it, because it is a disclosure rule somebody wrote — and it is
 * the width of the ONLY information asymmetry this world has. Everything else here is public the
 * moment it happens; between the close and the publication the company knows its result and nobody
 * else does, and that gap is a real thing a management can act inside (§35's business, not this
 * item's).
 */
export function reportingParams(): ParamDecl[] {
  return [
    {
      id: REPORTING_PARAMS.lag,
      value: 30,
      unit: 'days after the fiscal close',
      kind: 'policy',
      owner: 'parliament',
      why: 'Reporting A4, A4.a: the books close, then the report comes out, and in between the firm knows its result and nobody else does. A disclosure rule somebody wrote, and the width of the only information asymmetry this world has. Parliament owns it from worklist 14.',
    },
  ];
}

/**
 * A3, Seed B1.a: the month a company's fiscal year ends in.
 *
 * IT IS DRAWN AND NOT STATED. Nothing in this world has a reason to prefer December, and if every
 * company closed in the same month there would be one reporting season a year instead of a thing
 * that happens continuously — which is what makes estimates, revisions and surprises land at
 * different times for different names rather than all at once. Deterministic in the world's seed and
 * the company's own identity, so it is the same in every run of the same world and different between
 * companies, with no number written down beside any name.
 */
export function anchorOf(seed: string, company: PartyId): number {
  return prng(seed, 'reporting.anchor').derive(String(company)).int(MONTHS_IN_YEAR) + 1;
}
