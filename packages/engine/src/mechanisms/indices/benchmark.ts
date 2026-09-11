/**
 * The rate benchmark: what overnight money actually changed hands at.
 *
 * @spec Indices D3 Indices D3.a Indices D3.b Indices E2 XI-7 Law 3 Law 19 Appendix B
 *
 * D3.a, XI-7: IT IS TRANSACTED OR IT IS NOTHING. The benchmark is the volume-weighted rate of the
 * overnight lending that SETTLED this period, read off the money market's own public prints — one
 * per borrower, each carrying the rate it paid and the volume it raised. A period in which nobody
 * borrowed overnight has no benchmark, and that is the answer: a rate nobody paid is not a fixing,
 * and carrying yesterday's forward would be a posted benchmark, which Appendix B forbids outright.
 *
 * D3.b: AND THE POLICY RATE IS NOT IT. What the central bank administers is one price in one
 * facility with a real quantity behind it (Law 3's single exception); what the market paid each
 * other is a different number, and the gap between them is the thing worth measuring. Nothing here
 * reads the corridor.
 *
 * E2: nothing stores it. It is recomputed from the same events every time it is asked for, so it
 * cannot be revised and cannot be stale. What the publishing phase writes is an OBSERVATION of it,
 * the same way a print is a record of a trade — not a level anybody later reads back as the number.
 */
import type { CurrencyCode } from '../../core/ids.js';
import { add, div, mul, sum } from '../../core/num.js';
import { none, some, type Option } from '../../core/option.js';
import type { MechanismContext } from '../../world/context.js';

/** What one overnight book did this period, in one currency. */
export interface Benchmark {
  readonly ccy: CurrencyCode;
  /**
   * D3, Law 4: WHICH overnight book. Money lent against collateral and money lent on a name are
   * two prices for two different things — one carries the borrower's credit and the other carries
   * the paper's — so each is its own benchmark and neither stands for the other. Averaging them
   * would be one number over two markets, and a world where only one of them transacts would then
   * publish a fixing for a market that did not trade.
   */
  readonly secured: boolean;
  /** The volume-weighted rate, per period, as the rows themselves carry it. */
  readonly rate: number;
  /** What it was measured over: how much settled, and how many borrowers paid it. */
  readonly volume: number;
  readonly borrowers: number;
}

/** D3, D3.a: the fixing for one book in one currency, or nothing at all if it did not trade. */
export function benchmark(
  ctx: MechanismContext,
  ccy: CurrencyCode,
  secured: boolean,
): Option<Benchmark> {
  const weighted: number[] = [];
  const volumes: number[] = [];
  for (const e of ctx.journal.ofKind('moneyMarket.print')) {
    if (e.period !== ctx.period) continue;
    if (e.data['ccy'] !== ccy || e.data['tenor'] !== 'overnight') continue;
    if (e.data['secured'] !== secured) continue;
    const rate = e.data['rate'];
    const volume = e.data['volume'];
    if (typeof rate !== 'number' || typeof volume !== 'number' || volume <= 0) continue;
    weighted.push(mul(rate, volume, 'what this borrower paid, for what it took'));
    volumes.push(volume);
  }
  const volume = sum(volumes).value;
  if (volume <= 0) return none<Benchmark>();
  return some({
    ccy,
    secured,
    rate: div(sum(weighted).value, volume, 'the rate the market paid'),
    volume,
    borrowers: add(weighted.length, 0, 'borrowers'),
  });
}
