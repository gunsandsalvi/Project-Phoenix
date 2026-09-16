import { describe, expect, it } from 'vitest';
import { USD } from '../src/seeds/foundation.js';
import { agreementId, agreementKindId, currencyCode, partyId } from '../src/core/ids.js';
import { period } from '../src/calendar/calendar.js';
import type { Agreement } from '../src/register/agreements.js';
import { rentOwedBy, type TenancyTerms } from '../src/registry/funding.js';

/** A tenancy as the housing module writes it, seen only through the kernel's book. */
function tenancy(
  id: string,
  tenant: string,
  state: Agreement['state'],
  rentPerDwelling: number,
  dwellings: number,
): Agreement {
  const terms: TenancyTerms = { kind: agreementKindId('housing.tenancy'), rentPerDwelling, dwellings };
  return {
    id: agreementId(id),
    since: period(1),
    state,
    debtor: partyId(tenant),
    creditor: partyId('landlord.1'),
    ccy: currencyCode('USD'),
    owed: 0,
    terms,
    why: 'a test tenancy',
  };
}

describe('what falls due on a household before its basket (Households E3, 0f.7c)', () => {
  it('sums the rent on every performing tenancy this party is the tenant of, and nothing else', () => {
    const me = partyId('hh.1');
    const book: Agreement[] = [
      tenancy('t1', 'hh.1', 'performing', 30, 2),
      tenancy('t2', 'hh.1', 'terminated', 30, 5),
      tenancy('t3', 'hh.2', 'performing', 30, 5),
      {
        ...tenancy('e1', 'hh.1', 'performing', 0, 0),
        terms: { kind: agreementKindId('labour.employment') },
      },
    ];
    expect(rentOwedBy(book, me, USD).pieces).toBe(60);
    expect(rentOwedBy([], me, USD).pieces).toBe(0);
  });
});
