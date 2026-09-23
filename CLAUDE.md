# Project Phoenix — standing rules

A bottom-up economic and financial world — hundreds of millions of people, millions of small firms, three
countries — in which every outcome is caused, played turn by turn on a phone.

## The one document

`docs/spec/PROJECT_PHOENIX.md` is the specification and the authority on every question of mechanism. It says what
exists, who decides what from what, what must hold, what is measured and what must never exist. It says nothing
about how to build.

- **Read before deciding.** Part I (the laws), Part II (how requirements are written), the system's own section and
  every section in its **Depends on** line. A question the text does not settle is settled by Part I; if Part I
  does not settle it, it is an open decision (Appendix E), and nothing is invented in its place.
- **Follow it, never blindly.** The laws are the guide. Where a clause and a law disagree, the clause is wrong:
  fix the clause, in its own change, and say why.
- **Update it in the same change as the thing it describes.** It says what is true, never what a change found or
  did; it is not a diary. Identifiers are permanent: a retired clause keeps its number and says why.
- **Owner decisions** are the ones the spec reserves: the open items of Appendix E, the performance budget and
  accuracy for play (N8), the save time (N8.10), the settling length (GEN.6), the size of the map. Ask for those;
  derive everything else from the spec and state it in the commit. The answers are recorded in the plan's §12.

## Building

- **`docs/ARCHITECTURE.md`** records how the world is built — language, crates and their layers, how the population
  is laid out in memory, the day's sub-steps, the budgets — each decision taken against the spec and the budget
  (N8). A change of design is made there first, in the same change as the work that needs it.
- **`docs/IMPLEMENTATION.md`** is the plan: every step of every stage, its clauses, files, design, tests, live
  checks, budget and **Done when**, the findings (§11), the owner's decisions (§12) and the clause map (§13), which
  names the one step that completes each clause. Work one step at a time, in its order; mark its status there.
- **Two reviewers per block.** A block of the plan, and each step's code, is attacked by two independent reviews —
  spec and laws; architecture, budget and shortcuts (the prompts are in §0.7 of the plan) — and their findings are
  fixed before it is final.
- **Build in the order of Part O**, one stage at a time, each system to its **Done when** before the next. A need
  for a later system is met by bringing it forward, or by a placeholder that names the system that retires it.
- **Clauses in code** are carried by the `#[clause]` attribute (plan §2.4), never by comments.
- **The budget is a requirement** (N8): 1 s median and 2 s worst per business day on a Pixel 11 Pro, sustained,
  within 4.5 GB of memory and 4 GB of saves. It is measured on the device at the end of every stage. When it is
  missed, change how the world is represented and traversed, then the play resolution — never a mechanism, never
  the population.
- **The population representation is a hypothesis** (REP, Appendix E 14). It is proven against the reference run
  of the same world at weight one, from Stage 1 on (PTY.12, N8.5).
- **One change, one commit**, saying what and why.

## Findings

- A number that looks wrong, an audit family that fires, a mechanism that never runs, a measure that misses: each
  is a **finding** about a missing or wrong mechanism. Write it down with what was measured and where, and carry on
  with the work in hand.
- **Never tune** a primitive, a rule or the opening world to make a result look right (N7, GEN.11). Never add a
  bound to stop a number exploding: build the mechanism that holds it (Law 6).
- The exception is a state that cannot exist — a one-sided flow, a negative count of units, a fact with two
  writers. Fix it where it is.
- The realism tests (N3) come last, and credit only what the world regenerated (GEN.10).

## Code

- **Every declared number** comes from the primitive register with its kind, unit and owner (NUM.3). No numeric
  literal in a mechanism beyond 0, 1, −1 and 2.
- **No bounds**: no clamp, cap, floor or rescale of a number the world decides.
- **Missing is missing**: no default standing in for an absent value, no absent value read as zero.
- **No kind branches** (Law 10): what differs between kinds is declared data a mechanism reads.
- **Chance only through named, seeded streams** (CHN); never a clock, never unseeded randomness, never iteration
  order.
- **Contract violations stop the run** at the site, with the clause they break. **Invariant violations are audit
  findings**, reported with owner, size, day and clause; the audit never repairs (II.5, N1).
- **Comments say why**, in a sentence or two. No references to documents, clauses or history in comments, and no
  comment that repeats the code. A stale comment is a defect.

## Testing

- A test is either **at compile level** — the type refuses the defect — or **at logic level** — a pure function
  over values it is handed, asserting arithmetic or a stated refusal.
- **No test builds a world.** A world arranged by the same hand that wrote the code is a second world. A question
  about the world is answered against the real one: the audit, the liveness reads (N2), the reference run and the
  resolution ladder (PTY.12), the causal-chain tests (N4), the realism tests (N3).

## Working here

- Be brief in replies and in code comments.
- Develop on the branch you are given; push when a change is complete.
