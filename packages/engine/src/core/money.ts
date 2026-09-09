/**
 * Money and quantities as value objects.
 *
 * @spec Money A2 Money A2.b Currency A3 Currency A3.a Currency A4 Appendix A
 *
 * A Money carries its currency; a Qty carries its unit. Combining across currencies or units throws
 * Mismatch. Both are immutable.
 */
import { Mismatch } from './errors.js';
import { currencyUnit, type CurrencyCode, type UnitId } from './ids.js';
import { finite } from './num.js';

export interface Money {
  readonly amount: number;
  readonly ccy: CurrencyCode;
}

export interface Qty {
  readonly amount: number;
  readonly unit: UnitId;
}

export function money(amount: number, ccy: CurrencyCode): Money {
  return Object.freeze({ amount: finite(amount, `money ${ccy}`), ccy });
}

export function qty(amount: number, unit: UnitId): Qty {
  return Object.freeze({ amount: finite(amount, `qty ${unit}`), unit });
}

export function sameCcy(a: Money, b: Money, what: string): void {
  if (a.ccy !== b.ccy) {
    throw new Mismatch('Money A2.b', `${what}: ${a.ccy} and ${b.ccy} are never added`, { a, b });
  }
}

export function sameUnit(a: Qty, b: Qty, what: string): void {
  if (a.unit !== b.unit) {
    throw new Mismatch('Appendix A', `${what}: ${a.unit} and ${b.unit} are different units`, {
      a,
      b,
    });
  }
}

export function addMoney(a: Money, b: Money): Money {
  sameCcy(a, b, 'add');
  return money(a.amount + b.amount, a.ccy);
}

export function subMoney(a: Money, b: Money): Money {
  sameCcy(a, b, 'subtract');
  return money(a.amount - b.amount, a.ccy);
}

export function scaleMoney(a: Money, k: number): Money {
  return money(a.amount * finite(k, 'scale'), a.ccy);
}

export function negMoney(a: Money): Money {
  return money(-a.amount, a.ccy);
}

export function addQty(a: Qty, b: Qty): Qty {
  sameUnit(a, b, 'add');
  return qty(a.amount + b.amount, a.unit);
}

export function subQty(a: Qty, b: Qty): Qty {
  sameUnit(a, b, 'subtract');
  return qty(a.amount - b.amount, a.unit);
}

export function scaleQty(a: Qty, k: number): Qty {
  return qty(a.amount * finite(k, 'scale'), a.unit);
}

/** Money is a quantity of its currency's unit (Money D2). */
export function moneyAsQty(m: Money): Qty {
  return qty(m.amount, currencyUnit(m.ccy));
}

export function isZeroMoney(m: Money): boolean {
  return m.amount === 0;
}
