/**
 * What the parliament controls, and what no parliament may reach.
 *
 * @spec Polity D1 Polity D2 Polity D3 Polity D3.a Polity D4 Polity D5 Polity F2 Law 2 Law 15
 *
 * D5 gives every POLICY primitive an owner and the register refuses a setter that is not it — which
 * says who may move a number and says nothing about whether that owner should have it. This is the
 * other half: for `parliament`, WHICH POWER each of its numbers is an exercise of, named against
 * the clause that grants it. A module that declares a number parliament's and names no power is not
 * caught by anything else in this world — it simply appears on three ballot papers next period —
 * and here the world does not open.
 *
 * It is DATA (Law 15): a table of what a parliament may decide, which is a constitutional question
 * and not a mechanism's. A world that gives its parliament the capital ratio adds a row and says
 * which clause; a world that takes the planning release away from it deletes one. Neither is a
 * change to the polity.
 *
 * D3.a IS THE OTHER DIRECTION and it is the one that matters: Law 2 admits three kinds of primitive
 * and the parliament owns the third. A mandate naming an interest rate, a wage or an exchange rate
 * would be a written path with a majority behind it (F2), so a parliament-owned number that is a
 * PRICE is refused by its declared dimension, and the central bank's rate is refused by name — it
 * is the one number whose independence has a clause of its own (D4, §31 A4).
 */
import type { ParamId } from '../core/ids.js';
import { InvalidRegistry } from '../core/errors.js';
import type { ParamDecl } from './params.js';

export type Power = 'Polity D1' | 'Polity D2' | 'Polity D3' | 'Polity D4';

export interface PowerDecl {
  /** The clause that grants it: fiscal, spending, regulatory, or the central bank's target. */
  readonly clause: Power;
  /** A parameter id, or a family prefix ending in a dot — `treasury.tax.` covers every base. */
  readonly over: string;
  readonly why: string;
}

/** Whether a power covers a parameter: its own name, or the family it names. */
const covers = (over: string, id: ParamId): boolean =>
  over.endsWith('.') ? String(id).startsWith(over) : String(id) === over;

/**
 * D1–D4: THE POWERS, one row each, in the order the spec grants them.
 *
 * Every row is a thing a parliament decides in the world this models — what the state takes, what
 * it hands back, what it buys, who may be sold what, how long a firing is paid for, when a company
 * must publish, how much ground gets consent — and every row names the clause it comes under, so
 * "does parliament decide this?" is answered by a read rather than by an argument.
 */
export const WHAT_PARLIAMENT_CONTROLS: readonly PowerDecl[] = [
  {
    clause: 'Polity D1',
    over: 'treasury.tax.',
    why: 'The rate on each tax base (§30 C1). What the state takes, from whom, on what — the oldest thing a parliament is for.',
  },
  {
    clause: 'Polity D1',
    over: 'treasury.outlays.',
    why: 'The transfer rates (§30 B1): what the state hands back, per member, to the households it hands it to.',
  },
  {
    clause: 'Polity D1',
    over: 'treasury.buffer.periods',
    why: 'The cash buffer the treasury holds (§30 D4.b). How many periods of known outlays a state keeps in hand is how much it is willing to depend on the next auction clearing, and that is a decision somebody is answerable for rather than a treasurer’s taste.',
  },
  {
    clause: 'Polity D2',
    over: 'treasury.purchases.perPeriod',
    why: 'The size of the purchase programme (§30 B1): what the state buys, which it buys in a market at cleared prices like anybody else.',
  },
  {
    clause: 'Polity D2',
    over: 'treasury.publicService.hours',
    why: 'The public payroll (§30 B1): the hours the state employs directly rather than buying the same work from somebody who employs.',
  },
  {
    clause: 'Polity D3',
    over: 'regulation.depositInsurance.',
    why: 'How wide the guarantee is and what the banks pay for it. A guarantee is a promise the state makes with public money, so its size is not the regulator’s to choose.',
  },
  {
    clause: 'Polity D3',
    over: 'regulation.riskWeight.cds.sold',
    why: 'What writing protection costs in capital. D3 names the ratios and floors that are POLICY primitives; the rest of the weights are the standard-setter’s (D5), and the difference is which body this world says answers for them.',
  },
  {
    clause: 'Polity D3',
    over: 'pensions.',
    why: 'The contributions and the replacement rate — D3 names a replacement rate by name. What a pension promises and who pays for it is the same question as a transfer, one generation further out.',
  },
  {
    clause: 'Polity D3',
    over: 'law.arrears.gracePeriods',
    why: 'How long a payment may be late before the commitment behind it is broken (Money E1, XI-8). A grace period is a legislature’s number and a court’s instrument, and it is what separates a payer who is late from one who has defaulted on the thing itself.',
  },
  {
    clause: 'Polity D3',
    over: 'labour.severance.periods',
    why: 'What a firing costs whoever ends it. A job is a household’s whole income, and how dearly that is protected is what a labour law is.',
  },
  {
    clause: 'Polity D3',
    over: 'labour.retirementAge',
    why: 'When a member stops working and the pension starts paying. It is a rule about eligibility and never a forecast of when anybody actually stops.',
  },
  {
    clause: 'Polity D3',
    over: 'funds.accreditedWealthPerMember',
    why: 'Which households may be sold what a pool sells: a line between protecting somebody and deciding for them, which is exactly the sort of thing a parliament draws.',
  },
  {
    clause: 'Polity D3',
    over: 'reporting.lag.days',
    why: 'How long a company may take to publish (§48). Everybody else prices on what it publishes, so when it must is a public rule and not an accounting preference.',
  },
  {
    clause: 'Polity D3',
    over: 'land.planning.',
    why: 'How much unbuilt ground an authority releases a period (15.1): a rule about CONSENTS. What a hectare then fetches is the book’s, which is the difference between a rule and a price.',
  },
  {
    clause: 'Polity D4',
    over: 'centralBank.target.',
    why: 'The central bank’s mandate text and its target are the parliament’s (§31 A3): what the bank is aiming at is a public choice. What it DOES about it is not — see below.',
  },
];

/**
 * D3.a, D4: WHAT NO MANDATE MAY REACH, whoever declared it parliament's.
 *
 * Named families rather than a judgement: the policy rate has a clause of its own (§31 A4,
 * operationally independent — a parliament that set it would delete the corridor's reason to
 * exist), and everything else a parliament must not set is caught by its DIMENSION below.
 */
export const NEVER_PARLIAMENT: readonly { readonly over: string; readonly why: string }[] = [
  {
    over: 'centralBank.policyRate.',
    why: 'Polity D4, §31 A4: the bank’s rate is operationally independent. A parliament that set it would be setting the price of time by majority.',
  },
  {
    over: 'centralBank.corridor.',
    why: 'Polity D4: the two facilities are the rate plus and minus its spreads, so a parliament with the spreads would have the rate through the side door.',
  },
];

/** D3.a: a price is never a policy primitive, so a parliament-owned one is a written price. */
const PRICES = ['price', 'pricePerUnit'];

/**
 * D1–D4, D3.a: WHICH POWER EACH OF PARLIAMENT'S NUMBERS IS, checked at assembly, once.
 *
 * It throws rather than reporting, for the reason `platformPositions` does: a parameter declaration
 * is a declaration, and one that puts a number in a parliament's hands without saying which power
 * that is, is a defect in the declaration and not a finding about a world that ran.
 */
export function whatParliamentControls(
  policies: readonly ParamDecl[],
): Map<ParamId, PowerDecl> {
  const mine = policies.filter((d) => d.kind === 'policy' && d.owner === 'parliament');
  const out = new Map<ParamId, PowerDecl>();
  for (const d of mine) {
    // D3.a: never a price. The dimension is the half of the unit a machine can check, and this is
    // the one check it was always for: a number stated in money for one of something is a price,
    // and a price is cleared (Law 3) whoever would rather it were not.
    if (PRICES.includes(d.dimension)) {
      throw new InvalidRegistry(
        'Polity D3.a',
        `${String(d.id)} is a price ("${d.unit}") and is declared parliament’s: a price is cleared, never voted for`,
        { id: String(d.id), dimension: d.dimension },
      );
    }
    const never = NEVER_PARLIAMENT.find((n) => covers(n.over, d.id));
    if (never !== undefined) {
      throw new InvalidRegistry(
        'Polity D4',
        `${String(d.id)} is declared parliament’s and no parliament may have it: ${never.why}`,
        { id: String(d.id), family: never.over },
      );
    }
    const found = WHAT_PARLIAMENT_CONTROLS.filter((p) => covers(p.over, d.id));
    if (found.length === 0) {
      throw new InvalidRegistry(
        'Polity D1',
        `${String(d.id)} is declared parliament’s and no clause of D1–D4 grants it: name the power or give the number another owner`,
        { id: String(d.id) },
      );
    }
    if (found.length > 1) {
      throw new InvalidRegistry(
        'Polity D5',
        `${String(d.id)} is granted by ${String(found.length)} powers (${found.map((p) => p.over).join(', ')}): one number, one power`,
        { id: String(d.id) },
      );
    }
    const only = found[0];
    if (only !== undefined) out.set(d.id, only);
  }
  // A power over nothing is a power this world does not actually grant — a number renamed, a module
  // dropped, a row written for a mechanism that was never built. It reads as scope and is not.
  for (const p of WHAT_PARLIAMENT_CONTROLS) {
    if (![...out.values()].includes(p)) {
      throw new InvalidRegistry(
        'Polity D5',
        `no number parliament owns is covered by ${p.over}, which ${p.clause} grants it`,
        { over: p.over, clause: p.clause },
      );
    }
  }
  return out;
}
