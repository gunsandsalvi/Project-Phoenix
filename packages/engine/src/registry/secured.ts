/**
 * What a pledge is worth to whoever is owed, and what that makes a claim worth.
 *
 * @spec Banks Lending A4 Banks Lending D5 Corporate Credit E5 Bond N13 Housing C5.a Money Market B3.b Law 4 Law 15 Law 19
 *
 * A security is worth something to a lender or it is decoration, and what it is worth is the same
 * arithmetic wherever the question is asked: a bank provisioning a loan, a desk pricing a secured
 * line, a lender deciding what to advance against paper in a repo. THREE MODULES, ONE DERIVATION
 * (Law 4) \u2014 so it lives here, where all of them can reach it and none of them has to import another.
 *
 * NOTHING HERE DECIDES ANYTHING. Every input is somebody else's published number or a print the
 * market made: the expected loss is the lender's own, under its own name; what the security fetches
 * is the market's; what is pledged is the KIND's, read off its ranking, so no caller names a kind.
 */
import { asRatio, minus, noCash, type Cash, type PerPiece, type Ratio, ratioOf, scale, sumCash, valueAt, asCash } from '../core/measure.js';
import type { InstrumentId } from '../core/ids.js';
import type { Instrument } from '../register/instruments.js';
import { asQty } from '../core/tick.js';
import { none, some, type Option } from '../core/option.js';
import type { ParticipantView } from '../world/context.js';
import { expectedLossOn, requiredOf } from './banking.js';

/**
 * Housing C5.a, Banks Lending D5 (17.7c): HOW MUCH OF A CLAIM ITS SECURITY DOES NOT STAND BEHIND, at the market's own price of
 * that security, as a share of what is owed.
 *
 * It is the whole of what a pledge does to a lender's arithmetic, and it is separated from the loss
 * itself because two readers need it and they need different things (Law 4). A lender provisioning a
 * row wants the loss: this share times what it expects to lose on the name. A desk pricing a claim
 * on a name it has already published an expected loss for wants the SHARE — its published number is
 * for the name, and what this claim is secured on is what makes this claim different from it.
 *
 * One with nothing pledged, nothing where the security is worth more than the claim, and everything
 * in between is arithmetic. A pledge of something nobody prices covers nothing, which is a real
 * answer and not a zero anybody chose.
 */
export function uncoveredShare(
  security: readonly { readonly instrument: InstrumentId; readonly qty: number }[],
  worthOf: (instrument: InstrumentId) => PerPiece | undefined,
  owed: Cash,
): Ratio {
  if (security.length === 0 || owed.pieces <= 0) return asRatio(1, 'nothing stands behind it');
  const behind = sumCash(
    owed.ccy,
    security.map((row) => {
      const price = worthOf(row.instrument);
      if (price === undefined) return noCash(owed.ccy);
      return valueAt(price, asQty(row.qty, 'the units pledged'), owed.ccy, 'what the security is worth');
    }),
    'what stands behind it',
  ).value;
  const uncovered = minus(owed, behind, 'the part the security does not cover');
  if (uncovered.pieces <= 0) return asRatio(0, 'covered: nothing of it is at risk');
  return ratioOf(uncovered, owed, 'of every unit lent');
}


/**
 * §42 C6, Law 9, Law 19 (17g.2): WHAT A POOL OF CLAIMS IS SECURED ON, read off the claims themselves.
 *
 * *"A pool of loans"* is not a description anybody can price. A holder of a note needs to know
 * whether what stands behind it is houses, shops or nothing, because that is what decides what it
 * is worth when the borrowers stop paying — and the answer is already in the world: every claim's
 * kind says what is pledged behind it (`ranking(i).secured`, the same read an estate uses), and the
 * pledged things have kinds of their own. So this counts them, and nothing anywhere types a label.
 *
 * Unsecured claims are counted as such rather than left out: a pool that is half houses and half
 * nothing is a different thing from a pool that is all houses, and a reader that saw only the
 * houses would be reading the better half of it.
 */
export function securedOn(
  rows: readonly Instrument[],
  kinds: (i: InstrumentId) => string,
  ranking: (i: Instrument) => { readonly secured: readonly { readonly instrument: InstrumentId }[] },
): readonly { readonly on: string; readonly rows: number }[] {
  const count = new Map<string, number>();
  for (const row of rows) {
    const pledged = ranking(row).secured;
    const on =
      pledged.length === 0
        ? 'nothing'
        : [...new Set(pledged.map((p) => kinds(p.instrument)))].sort().join(' and ');
    const had = count.get(on);
    count.set(on, had === undefined ? 1 : had + 1);
  }
  return [...count]
    .map(([on, n]) => ({ on, rows: n }))
    .sort((a, b) => (b.rows === a.rows ? a.on.localeCompare(b.on) : b.rows - a.rows));
}

/** Law 9: what it is secured on, in the words a market would use for it. */
export function securedOnSaid(behind: readonly { readonly on: string; readonly rows: number }[]): string {
  if (behind.length === 0) return 'nothing';
  return behind.map((b) => `${b.rows} on ${b.on}`).join(', ');
}

/**
 * Banks Lending A4, D5, Corporate Credit E5 (17.7c): WHAT THIS BANK REQUIRES TO HOLD **THIS CLAIM**
 * — which is not the same question as what it requires of the NAME, and the difference is the
 * security.
 *
 * Its published reservation is about a name: what it costs it to fund, what it expects to lose on
 * that name, and what the capital consumes. A claim on the same name with something pledged behind
 * it loses less when the name fails — as much less as the security covers, at the market's own price
 * of that security — so what it requires of the claim is what it requires of the name, less the part
 * of the expected loss the pledge takes away. Nothing new is believed here: the expected loss is the
 * bank's own published number, and the share covered is arithmetic over prints (Law 19).
 *
 * WHAT IS SECURED IS THE KIND'S TO SAY (Law 15). It is read off `ranking(i).secured` — the same
 * answer an estate reads when it decides who takes what — so a loan, a covered bond and anything
 * else with a pledge behind it are all priced by this one rule and none of them is named here.
 *
 * Nothing where the bank has published no view of the name: a claim it cannot price unsecured it
 * cannot price secured either.
 */
export function requiredOnClaim(view: ParticipantView, instrument: InstrumentId): Option<number> {
  const i = view.instruments.get(instrument);
  if (!i.issuer.some) return none<number>();
  const name = i.issuer.value;
  const said = requiredOf(view, String(name));
  const required: Option<number> = said.some ? some(said.value as number) : none<number>();
  if (!required.some) return required;
  const secured = view.registry.instrumentKind(i.kind).ranking(i).secured;
  if (secured.length === 0) return required;
  const expected = expectedLossOn(view, String(name));
  if (!expected.some) return required;
  // Housing C5.a: what is owed on the claim, which is what the pledge has to cover to cover it.
  const owed = asCash(i.issued, i.ccy, `what ${instrument} owes`);
  const share = uncoveredShare(
    secured,
    (pledged: InstrumentId) => {
      const print = view.print(pledged);
      return print.some ? print.value.price : undefined;
    },
    owed,
  );
  const taken = scale(
    expected.value,
    minus(asRatio(1, 'the whole of it'), share, 'the part the pledge covers'),
    'the expected loss the pledge takes away',
  );
  return some(minus(required.value as unknown as Ratio, taken, 'what it requires of this claim') as unknown as number);
}

