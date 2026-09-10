# Phoenix — the rules that bite (digest of CLAUDE.md; the full file is the authority)

EVERY DECLARED NUMBER is exactly one of: technology | preference | policy | resolution | shape.
  A shape that names the item which kills it is a PLACEHOLDER, declared with standsInFor.
  Everything else — ownership, prices, quantities, shares, allocations — is an OUTCOME.
NO BOUND. No cap, floor, ceiling, clamp, damper, "not less than zero", no Math.min/max outside
  core/num.ts. Only arithmetic impossibility. A number that explodes means a missing mechanism:
  build it and delete the bound in the same change. A long comment justifying a bound is the tell.
EVERY FLOW HAS TWO SIDES, both legs, same pass, same period, same currency. One-sided is a defect
  even when nothing fails. One writer per fact; hunt parallel formulas and mirrored copies.
TOLERANCE IS DERIVED DUST: terms × ε × Σ|magnitudes|, per check. Never a percentage, never widened.
  A check that only passes with a band is reporting a defect.
EVERY PRICE IS CLEARED from real supply meeting real demand. Yield, spread, DM, OAS, PE derive FROM
  price — never into it. Only administered central-bank rates are exempt, with a real quantity
  response booked on both balance sheets.
READ THE SOURCE. Never recompute, sum a copy, infer by subtraction, or re-price what a market
  printed. Every deletion names the read that replaces it.
FIX THE CAUSE. A cause has one fix and it REMOVES code. A fix that is a list — add a bound, a check,
  a flag — is a symptom patch.
MISSING IS MISSING. No ?? 0, no || 0, no numeric defaults, no optional-means-unset numbers.
NO KIND BRANCH in a mechanism. Data in a registry; kind-varying behaviour in a profile behind a
  dispatch table. A module never imports another module or world/world.ts.
A STALE COMMENT OR DOC IS A DEFECT. A comment says why.
DO NOT MEASURE MID-BUILD. A misbehaving number is not a work item; the missing mechanism is.
  Never roll back: a bad number is a finding. Only a change wrong on its own terms is undone.

THE LOOP. Take the FIRST open item in docs/WORKLIST.md. Read docs/PLAN.md and docs/plan/<item>.md
  before writing code. A new idea is INSERTED at its dependency position, never appended — say
  where. One item, one bounded change, one commit, carrying its RECORD entry and its COVERAGE
  re-mark. Tick the item's steps; delete the item file when it closes.
BEFORE EVERY COMMIT: npm run check. All green or the item is not done.

A RULE THAT CAN BE A CHECK SHOULD BE ONE. Text in context is a reminder; ParamRegister throwing is
  a guard. When a rule is broken twice, write the check — that is what tools/eslint-rules and the
  registry guards are for.
