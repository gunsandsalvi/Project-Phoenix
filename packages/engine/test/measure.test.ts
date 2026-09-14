/**
 * A number that carries what it counts, and the programs that stop compiling because of it.
 *
 * @spec Law 4 Law 7 Law 8 Money A2.b Currency C4 XI-15
 *
 * The findings this type is for are not runtime bugs to catch — they are programs that should never
 * have compiled. So most of what this file asserts, it asserts with `@ts-expect-error`: the test
 * PASSES when the compiler refuses the line, and fails when it accepts it.
 */
import { describe, expect, it } from 'vitest';
import { period } from '../src/calendar/calendar.js';
import { currencyCode, instrumentId, marketId } from '../src/core/ids.js';
import { sum } from '../src/core/num.js';
import { asQty, type Qty } from '../src/core/tick.js';
import type { Print } from '../src/prices/price-store.js';
import type { Lot } from '../src/register/register.js';
import type { CashFlow } from '../src/registry/kinds.js';
import {
  absolute,
  acrossMembers,
  asAmount,
  asCash,
  asMoney,
  asPerPiece,
  negated,
  over,
  type Cash,
  asPerMember,
  asPrice,
  asRatio,
  asTotal,
  eachMember,
  minus,
  plus,
  pricedAt,
  ratioOf,
  scale,
  valueAt,
  type Money,
} from '../src/core/measure.js';

const usd = asMoney<'USD'>(100, 'a hundred dollars');
const eur = asMoney<'EUR'>(100, 'a hundred euros');
const tonnes = asAmount<'tonne'>(4, 'four tonnes');
const perTonne = asPrice<'USD', 'tonne'>(25, 'twenty-five dollars a tonne');
const half = asRatio(0.5, 'a half');

describe('two currencies do not add (A-23, A-38, A-47, A-51, A-61)', () => {
  it('adds two of the same money and refuses two of different ones', () => {
    expect(plus(usd, usd, 'what it holds')).toBe(200);
    expect(minus(usd, usd, 'what is left')).toBe(0);
    // A cell's wealth was the sum of every money it had ever been paid in, and it compiled.
    // @ts-expect-error two currencies are two dimensions and they do not meet here
    expect(() => plus(usd, eur, 'wealth')).toBeDefined();
  });
});

describe('money times money has no inhabitant (A-65)', () => {
  it('a price times a quantity is money, and the unit cancels', () => {
    const paid: Money<'USD'> = valueAt(perTonne, tonnes, 'what four tonnes came to');
    expect(paid).toBe(100);
    // Every option premium in this world is a money-squared number that does not depend on the
    // strike, and the line that produced it was `mul(money, money, 'the premium')`.
    // @ts-expect-error money is not a price and a premium is not money squared
    expect(() => valueAt(usd, tonnes, 'the premium')).toBeDefined();
    // And a price of the wrong unit does not meet a quantity of this one.
    const perHour = asPrice<'USD', 'hour'>(9, 'nine dollars an hour');
    // @ts-expect-error a price per hour does not price a tonne
    expect(() => valueAt(perHour, tonnes, 'what it came to')).toBeDefined();
  });

  it('a price comes from a payment and a delivery, and nothing else', () => {
    expect(pricedAt(usd, tonnes, 'what it went for')).toBe(25);
    // XI-6: nothing was delivered, so there is no price it was delivered at. Not a zero.
    expect(() => pricedAt(usd, asAmount<'tonne'>(0, 'none'), 'the price')).toThrow(/no price/);
  });
});

describe('a ratio can never be an amount (A-44, A-58)', () => {
  it('scales a dimension without changing it, and is not one', () => {
    expect(scale(usd, half, 'half of it')).toBe(50);
    expect(scale(tonnes, half, 'half of it')).toBe(2);
    // A spread declared "over what a deposit returns" used as an absolute rate; a leverage ratio
    // called a cost of funds. Both are a ratio where a level was wanted.
    // @ts-expect-error a ratio is not money and cannot be paid
    expect(() => plus(usd, half, 'what it holds')).toBeDefined();
    // Two of the same thing divided IS a pure number, and that is the only way to reach one.
    expect(ratioOf(usd, usd, 'the share')).toBe(1);
    // @ts-expect-error a share of a tonne in dollars is not a number
    expect(() => ratioOf(usd, tonnes, 'the share')).toBeDefined();
    expect(() => ratioOf(usd, asMoney<'USD'>(0, 'none'), 'the share')).toThrow(/share of nothing/);
  });
});

describe('per member and total are different types (A-1, A-18, A-39)', () => {
  it('and the ONLY conversion is the cell own weight', () => {
    const each = asPerMember<'money:USD'>(7, 'seven each');
    const all = acrossMembers(each, 300, 'what they hold between them');
    expect(all).toBe(2100);
    expect(eachMember(all, 300, 'each of them')).toBe(7);
    /**
     * A-39 is this mistake: `perMember × headcount` on one side of a flow whose other side used
     * `perMember × weight`. The wage bill capitalised into inventory was bigger than the wage that
     * was paid, every period, for every row — the largest conservation break in the model.
     */
    // @ts-expect-error a per-member number is not a total and does not add to one
    expect(() => plus(each, all, 'the wage bill')).toBeDefined();
    // A weight is a COUNT OF PEOPLE, and a fraction of one is not (XI-15).
    expect(() => acrossMembers(each, 2.5, 'between them')).toThrow(/count of people/);
    expect(() => eachMember(asTotal<'money:USD'>(9, 'nine'), 0, 'each')).toThrow(/nobody to divide/);
  });
});

describe('stage 1: the kernel carries its dimensions', () => {
  it('a print is a price, and a bare number cannot be one', () => {
    const line = instrumentId('line.under.test');
    const good: Print = {
      instrument: line,
      market: marketId('mkt.under.test'),
      period: period(0),
      price: asPerPiece(3, 'a level'),
      ccy: currencyCode('USD'),
      provenance: { kind: 'opening' },
    };
    expect(good.price).toBe(3);
    const bad: Print = {
      instrument: line,
      market: marketId('mkt.under.test'),
      period: period(0),
      // @ts-expect-error a level a market printed is money per piece, not a bare number
      price: 3,
      ccy: currencyCode('USD'),
      provenance: { kind: 'opening' },
    };
    expect(bad.price).toBe(3);
  });

  it('a lot basis and a cash flow are prices too', () => {
    const lot: Pick<Lot, 'basisPerUnit'> = { basisPerUnit: asPerPiece(2, 'what it cost') };
    expect(lot.basisPerUnit).toBe(2);
    const flow: CashFlow = { date: { y: 2030, m: 1, d: 1 }, perUnit: asPerPiece(1, 'par') };
    expect(flow.perUnit).toBe(1);
    // @ts-expect-error what one unit pays is money per piece and says so
    const wrong: CashFlow = { date: { y: 2030, m: 1, d: 1 }, perUnit: 1 };
    expect(wrong.perUnit).toBe(1);
  });

  it('a register quantity IS an amount, so it prices with nothing in between (Law 4)', () => {
    // `Qty` is `Amount<'piece'>`: one representation of a count, not a second brand beside one.
    const held: Qty = asQty(50, 'fifty pieces');
    const level = asPerPiece(4, 'four a piece');
    const worth: Cash = valueAt(level, held, 'what it is worth');
    expect(worth).toBe(200);
    // And what it is worth is not what it is worth PER PIECE.
    // @ts-expect-error a balance is not a level
    expect(() => plus(worth, level, 'nonsense')).toBeDefined();
  });

  it('a total carries the dimension of its terms out of `sum` (Law 7)', () => {
    const total: Cash = sum([asCash(1, 'one'), asCash(2, 'two')]).value;
    expect(total).toBe(3);
    // @ts-expect-error a sum of money is money, and a level is not
    expect(() => plus(total, asPerPiece(1, 'a level'), 'nonsense')).toBeDefined();
  });

  it('a dimension survives division by a pure number, negation and magnitude', () => {
    expect(over(asCash(10, 'ten'), asRatio(4, 'a four-for-one split'), 'restated')).toBe(2.5);
    expect(negated(asCash(10, 'ten'), 'what is owed')).toBe(-10);
    expect(absolute(asCash(-10, 'owed'), 'how big it is')).toBe(10);
    expect(() => over(asCash(10, 'ten'), asRatio(0, 'none'), 'restated')).toThrow(/into nothing/);
  });
});

describe('what it costs at runtime', () => {
  it('nothing: it IS a number, so behaviour cannot change (Law 18)', () => {
    // The phantom erases completely — no wrapper, no allocation, no arithmetic that was not there.
    // What changes is which programs compile, which is the whole of the guarantee.
    expect(typeof usd).toBe('number');
    expect(usd + 1).toBe(101);
    expect(JSON.stringify({ usd })).toBe('{"usd":100}');
  });
});
