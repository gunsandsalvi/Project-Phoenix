/**
 * What this module reads, and the one number it declares.
 *
 * @spec Reporting A3 Reporting A4 Reporting A4.a Reporting G6 Law 2 Seed B1.a
 */
import { paramId } from '../../core/ids.js';
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
      dimension: 'days',
      kind: 'policy',
      owner: 'parliament',
      why: 'Reporting A4, A4.a: the books close, then the report comes out, and in between the firm knows its result and nobody else does. A disclosure rule somebody wrote, and the width of the only information asymmetry this world has. Parliament owns it from worklist 14.',
    },
  ];
}

