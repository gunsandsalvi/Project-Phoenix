/**
 * The firms this world has, and what each one makes.
 *
 * @spec Firm A1 Firm A2 Firm A3 Firm F4 Law 15
 *
 * Data only (Law 15). A firm exists in a region and a sector, and both are load-bearing (A2): the
 * region fixes its money, and the line it is in fixes what it buys, what it sells and who it
 * employs. That is the whole of what varies between firms here — every one of them decides the same
 * way from its own state (F4), and nothing in the mechanism ever asks which industry it is in.
 *
 * One firm per stage of the chain, so that a shortage upstream is a real constraint downstream and
 * an intermediate good has a real buyer that is not a household.
 */

export interface FirmDecl {
  /** The named party (Firm A1). It banks somewhere, and the wage and the invoices leave that account. */
  readonly firm: string;
  /** A2: the good it makes. Its recipe, its lead time and its yield are the good's own technology. */
  readonly subUnit: string;
  /** A2, Labour A3: the occupation it employs, which is the venue it posts its openings in. */
  readonly occupation: string;
}

export const FIRMS: readonly FirmDecl[] = [
  { firm: 'firm.1', subUnit: 'grain', occupation: 'field' },
  { firm: 'firm.2', subUnit: 'flour', occupation: 'mill' },
  { firm: 'firm.3', subUnit: 'bread', occupation: 'bakery' },
];
