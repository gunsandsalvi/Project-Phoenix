import { Forbidden, Impossible, type SpecCitation } from './errors.js';

/** Exhaustiveness guard for switches over closed unions (docs/ARCHITECTURE.md §5). */
export function assertNever(x: never, what: string): never {
  throw new Impossible('Law 15', `unhandled case in ${what}: ${JSON.stringify(x)}`);
}

/** A precondition that, if false, means a FORBID clause was about to be violated. */
export function forbid(
  condition: boolean,
  spec: SpecCitation,
  message: string,
  details: Record<string, unknown> = {},
): asserts condition {
  if (!condition) throw new Forbidden(spec, message, details);
}

/** A precondition that, if false, is an arithmetic impossibility (Law 6's only admissible bound). */
export function impossible(
  condition: boolean,
  spec: SpecCitation,
  message: string,
  details: Record<string, unknown> = {},
): asserts condition {
  if (!condition) throw new Impossible(spec, message, details);
}
