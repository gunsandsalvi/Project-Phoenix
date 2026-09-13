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
import {
  acrossMembers,
  asAmount,
  asMoney,
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

describe('what it costs at runtime', () => {
  it('nothing: it IS a number, so behaviour cannot change (Law 18)', () => {
    // The phantom erases completely — no wrapper, no allocation, no arithmetic that was not there.
    // What changes is which programs compile, which is the whole of the guarantee.
    expect(typeof usd).toBe('number');
    expect(usd + 1).toBe(101);
    expect(JSON.stringify({ usd })).toBe('{"usd":100}');
  });
});
