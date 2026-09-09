import { Missing, type SpecCitation } from './errors.js';

/** An explicit maybe. The only way an absent value travels through the engine (Appendix A). */
export type Option<T> = { readonly some: true; readonly value: T } | { readonly some: false };

export const NONE: Option<never> = Object.freeze({ some: false });

export function some<T>(value: T): Option<T> {
  return { some: true, value };
}

export function none<T>(): Option<T> {
  return NONE;
}

/** Take the value or throw Missing naming what was asked for. */
export function unwrap<T>(opt: Option<T>, what: string, spec: SpecCitation = 'Appendix A'): T {
  if (opt.some) return opt.value;
  throw new Missing(spec, `${what} is missing`, { what });
}

export function mapOption<T, U>(opt: Option<T>, f: (t: T) => U): Option<U> {
  return opt.some ? some(f(opt.value)) : NONE;
}
