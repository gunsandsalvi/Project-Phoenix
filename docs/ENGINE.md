# The engine: how it runs, how others run, and what it is not doing that it should

*A second research note, written against `291e146`, in the same shape as `docs/OPENING.md`: measure
first, read how other fields solve it, then propose. Where that one asked how a world should BEGIN,
this asks how a world should RUN — how fast, at what scale, and what a run entitles anybody to say.*

---

## 1. What the engine does now, measured

Readings from the rig (3 banks, 12 firms) at `291e146`.

**The market loop.** Twelve periods, counting every session the period loop ran:

| outcome | sessions |
|---|---|
| `noDemand` | 3,749 |
| `noOverlap` | 158 |
| `noSupply` | 236 |
| **`cleared`** | **15** |

**4,158 auctions, 15 trades — 0.36%.** In period 12 alone: 356 books, 321 with no demand, 0 cleared.
Of **358 books ever asked, 8 have ever cleared.** The engine spends essentially all of its time
opening books nobody comes to.

**Speed.** 554 ms/period at that rung after this session's work (1,354 this morning), so a 52-period
year is about half a minute at twelve firms; 26 periods cost 9.0 s and 52 cost 33.9 s, so the second
half of a year costs 2.8× the first per period. 288,210 journal events in a year at the smallest
scale; heap ~380 MB.

**Evidence.** Every number in this project, including every one above, comes from **one run of one
seed**. There is no ensemble anywhere, and no test asserts anything about a distribution over seeds.

**Gates.** `check:opens` (does it throw?), lint, typecheck, `check:spec`, `check:forbids`,
`check:deaths`, `check:existence`, `plan:check`. All of them are consistency or hygiene. **Nothing
gates on the world being alive**, and nothing gates on speed: the first rung went from 5.1 s a year
at 0g.1 to 70 s a year today without any check noticing.

**Tests.** 132 files. Three suites are red in full (`credit-events`, `ratings`, `equity` — 38 cases),
`ladder.test.ts`'s own invariance assertion is red, and the full suite is slow enough that it is not
run per commit. So the suite is documentation, not a gate.

---

## 2. Three questions wearing one name

As with the opening, "make the model better" is three problems, and they want different answers.

**(I) SPEED** — how long a run takes. This is 0g, and it is now the *least* important of the three:
99.6% of the engine's market work produces nothing, so the first-order speed fix is not a faster
auction, it is *not holding the auction*.

**(II) SCALE** — how big a world can be held. Phoenix's answer is cells (XI-15), which is a level of
detail applied by KIND. Nothing moves a party between levels as it becomes interesting, and nothing
measures what the choice costs.

**(III) EVIDENCE** — what a run entitles you to say. This is where Phoenix is weakest and where the
literature is most developed. One deterministic run is an anecdote; the project's own findings are
each read off a single trajectory.

---

## 3. Speed — how others do it

**Factorio: entities sleep, and waking them is somebody's job.** *"Active entities are the count of
entities that get updated each tick. Entities not in the 'Active' section don't get touched during
normal tick updates"* — and the wake is caused, not polled: *"an inserter that is supposed to fill
an assembler will be deactivated if the assembler's input is filled. Once the assembler produces an
item, the assembler activates the inserter"*
([Factorio wiki, diagnosing performance](https://wiki.factorio.com/Tutorial:Diagnosing_performance_issues),
[forum on active entities](https://forums.factorio.com/viewtopic.php?t=38047)). Their other lessons
are about memory, not logic: *"all active entities are read at every tick… in large factories this
is too much data for caches"*, answered with prefetching and per-chunk allocators
([FFF-204](https://factorio.com/blog/post/fff-204), [FFF-215](https://factorio.com/blog/post/fff-215));
and multithreading is gated by determinism — *"the game needs to remain fully deterministic"*
([FFF-421](https://www.factorio.com/blog/post/fff-421)).

Phoenix's 356 books a period are 356 inserters staring at a full assembler.

**Data-oriented design: the layout is the algorithm.** Structure-of-arrays with archetype storage is
reported at *"10-100x performance improvements over object-oriented architectures"*
([ECS architecture guide](https://cppcat.com/entity-component-system-implementation/),
[data-oriented design](https://en.wikipedia.org/wiki/Data-oriented_design)). This is 0g.11, and it is
the honest ceiling for a JavaScript engine that allocates a `Cash` object per addition.

**Discrete-event vs time-stepped: process what happens, not what might.** *"The default execution
scheme of agent-based modeling relies on fixed-increment time advances, whereas discrete event
simulation uses continuous time… assessed at specific time points triggered by an event"*, and the
discrete-event implementation *"performs better on CPU"*, with hybrids using the minimum gap between
events as the step ([JASSS](https://www.jasss.org/27/1/10.html),
[survey on hardware accelerators](https://arxiv.org/pdf/1807.01014)). Phoenix is time-stepped by
design and should stay so — a week is a real unit here and the audit closes on it — but *within* a
period the same idea applies: a phase should visit what has a reason to be visited.

**GPUs: enormous, and not for this.** FLAME GPU reports *"scaling to hundreds of millions of
agents"* and *"1000x speedup over the next best simulator for a Boids flocking model"*
([NVIDIA](https://developer.nvidia.com/blog/fast-large-scale-agent-based-simulations-on-nvidia-gpus-with-flame-gpu/),
[FLAME GPU 2](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3207)). Those speedups are for
local, homogeneous, embarrassingly parallel agent rules. Phoenix's period is a sequence of global
serialisations — one solver, one settlement, one audit — so the honest read is: **not applicable**,
and saying so is worth more than an aspiration.

**Dwarf Fortress: the wall, and the choice that builds it.** DF *"aggressively rejects the
abstraction philosophy common in game development"* and pays for it: *"there's only so much
optimizing that can be done with hundreds of people pathing around cities, and the same problem
eventually kills fortress performance"*
([Medium: an engineering marvel](https://medium.com/@christian.marques/dwarf-fortress-an-engineering-marvel-of-the-21st-century-2ba3a1e9b95f)).
Phoenix has made the same choice on purpose — Law 1 is that choice — so it should expect the same
wall and plan the same mitigations: abstraction where nobody is looking, detail where they are.

---

## 4. Scale — level of detail as a first-class idea

Phoenix already has the mechanism and does not yet treat it as one. A cell is a level of detail: one
party standing for many, with `perMember` as the read and weight as the count. Promotion exists.
What is missing is that **the level is fixed by kind rather than chosen by importance**, and nothing
measures the error the choice introduces.

The hybrid literature makes this explicit — *"Equation-Based Versus Agent-Based Models: Why Not
Embrace Both for an Efficient Parameter Calibration?"*
([JASSS 26(4)](https://www.jasss.org/26/4/3.html)) — and the practical version in games is exactly
DF's world-scale history: the fortress is simulated in detail, the rest of the world abstractly, and
a civilization becomes detailed when you visit it.

For Phoenix the payoff is concrete: a world with 30 million people at 12 named firms is already at
356 books; at the third rung it is hours a year. The lever is not a faster solver. It is that most
of the world should be a cell most of the time, and that **a promotion should be caused by
something** — a firm that becomes big enough to matter, a household that becomes a landlord.

---

## 5. Evidence — where the gap is widest

**Stylized facts are a weak test, and this project doesn't even use them.** *"Due to
over-parameterization and the corresponding degrees of freedom, almost any simulation output can be
generated with an ABM, and thus replication of stylized facts only represents a weak test for the
validity of ABMs"*
([Towards a validation methodology for macroeconomic ABMs](https://d-nb.info/1246195569/34)). The
right reading for Phoenix is not "so we need stylized facts" but: if even matching real data weakly
identifies a model, then a project whose entire evidence base is *one trajectory of one seed* is
making claims it cannot support. Ensemble runs with averaging are standard practice everywhere in
this literature.

**Statistical model checking is the tool this project has been hand-rolling.** MultiVeStA applied to
a macroeconomic ABM *"can provide a principled analysis layer for a realistic macroeconomic ABM
without rewriting the simulator in a dedicated formalism"*, driven by *"reusable temporal queries,
observable-specific precision targets, and confidence-based stopping rules that automatically
determine the simulation effort required"*, and *"compared to standard Monte Carlo approaches, SMC
automatically determines the minimum number of simulations needed to achieve user-specified
confidence levels"*
([SMC of the Keynes+Schumpeter model](https://arxiv.org/html/2605.10447),
[SMC of the Island model](https://arxiv.org/html/2604.04543),
[MultiVeStA + Mesa](https://link.springer.com/chapter/10.1007/978-3-031-75434-0_26)).

Two things transfer immediately. First, **temporal properties are the right language for this
project's bugs**: "every declared book eventually clears", "every firm is eventually paid", "no party
stays in arrears for ever" are exactly the statements whose falsity the twelve findings report, and none of
them is expressible in the audit as it stands. Second, **how many runs is a computed number**, not a
preference.

**Surrogates and sensitivity: which of the declared numbers matter?** GP emulators plus Sobol indices
are the standard route to sensitivity for models too expensive to sweep
([The use of surrogate models to analyse ABMs](https://www.jasss.org/24/2/3/3.pdf),
[quantile-based emulation](https://doi.org/10.1137/17m1161233)). Phoenix declares over a hundred
behaviour-shaping numbers with kind, unit and owner, and **nobody knows which of them the world
depends on**. That is not only a validation gap; under Law 2 it is a deletion opportunity: a number
nothing is sensitive to is a number that should not be primitive.

**Metamorphic testing is the name for what the ladder already does.** *"Metamorphic relations are
necessary properties of the intended functionality of the software, and must involve multiple
executions"*, and they exist precisely because *"simulation validation… poses a particularly potent
form of the oracle problem, and often no oracle exists"*
([metamorphic testing](https://en.wikipedia.org/wiki/Metamorphic_testing),
[MT for hybrid simulation validation](https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=932547),
[MT with property-based testing tools](https://arxiv.org/abs/2211.12003)). The ladder's
scale-invariance assertion IS a metamorphic relation; so is "double the share resolution and nothing
moves"; so is "reorder the phases that commute and the prints are identical". Phoenix has one MR and
a red test. It should have a family of them, and `fast-check` is already in the toolchain.

---

## 6. What Phoenix is not doing that it should

Ranked by leverage per unit of work, with what each would have caught.

**1. Nothing checks that the world is ALIVE.** Nine audit families check consistency; a world can be
perfectly consistent and asleep. 15 clears in 4,158 sessions passes every gate this project has. A
LIVENESS family — stated as temporal properties over a run, reported with owner and size like any
other violation, never repaired — would have caught 12c.3, 21.72, 21.77, 21.79, 21.81, 21.84 and
21.73 **at the item that introduced each**, instead of one at a time, months later, by hand.
*This is the single highest-value thing missing from the engine.*

**2. One run is the whole evidence base.** An ensemble runner over k seeds, reporting the census as a
distribution, with a finding required to reproduce across seeds before it is called a finding. The
confidence-based stopping rule from SMC sets k rather than a preference.

**3. Books open whether or not anybody has a reason to come.** 99.6% of sessions clear nothing. A
book should open when somebody has posted a reason to be in it; the count of books that never open
becomes a measured fact about the world instead of invisible work. **This is both the largest single
speed win available and a liveness measurement** — which is why it is above the structural
performance items.

**4. No performance gate.** The ladder exists and is not wired to anything. A 14× regression went
unnoticed between 0g.1 and today. `check:forbids` already demonstrates the pattern: a baseline that
may only fall.

**5. The suite is not a gate.** 132 files, three suites red in full, too slow to run per commit.
Triage into invariants that must always hold (fast, gating) and mechanism tests (slow, scheduled),
and let the ladder and the census carry the gate.

**6. No sensitivity analysis over the parameter register.** Sobol on a surrogate, once the world is
alive; then delete or demote every primitive nothing is sensitive to.

**7. No way to ask a world WHY.** Every diagnosis in this project's record was a hand-written probe.
The journal is already an event log with subjects; an explainer over it — *why did this firm stop
producing?* → the chain of its own reads and decisions — would pay for itself in a week of the
owner's time. This is Legends mode for an economy, and the observer surface already renders journals.

**8. Level of detail is fixed by kind.** A promotion should be caused; a world should be able to run
coarse and refine where something interesting is happening.

**9. Determinism is tested at one seed on one platform**, with an Android target in the plan. The
lockstep literature's warning about floating point across platforms applies directly.

---

## 7. The order to do them in

**Speeding up a dead world is the wrong order, and this session demonstrated it.** 0g bought 2.4×
against a world where 99.6% of the market work is empty; the same effort spent on (1) and (3) above
would have made the engine faster *and* told us something true.

So:

- **0h.1 — the liveness family.** Temporal properties as an audit family: every declared book clears
  within N periods; every living firm produces, sells and is paid within N; every bank lends within
  N; no cohort's income is zero for N. Reported, never repaired. N is per property, declared with a
  reason, and the family is the first thing a new item runs against.
- **0h.2 — `npm run ensemble -- <k>`**: k seeds, the census as a distribution, a finding promoted
  only when it reproduces.
- **0h.3 — books open on posted interest.** Both the largest speed win and a liveness measurement;
  the books that never open are a finding with a name.
- **0h.4 — the ladder as a ratchet in `npm run check`**, baseline that may only fall.
- **0h.5 — suite triage**: gate vs scheduled; the three red suites get read, not carried.
- **0h.6 — the explainer** over the journal: why did this party do that?
- **0h.7 — metamorphic relations** as a family: scale invariance, resolution invariance, phase-order
  invariance, seed invariance of properties (not of values).
- **0h.8 — Sobol over the parameter register** on a surrogate, once the world is alive; every
  insensitive primitive is a deletion candidate.
- **then 0g.3 and 0g.11**, which are worth their cost against a world that does something.

**Exit.** A run reports not only that it did not throw but that it LIVED; a finding is a statement
about a distribution over seeds; and the question "why did this happen?" has an answer the engine
gives rather than one the owner reconstructs.

---

## Sources

- [Factorio wiki — Diagnosing performance issues](https://wiki.factorio.com/Tutorial:Diagnosing_performance_issues) ·
  [forum: active entities](https://forums.factorio.com/viewtopic.php?t=38047) ·
  [FFF-204](https://factorio.com/blog/post/fff-204) · [FFF-215](https://factorio.com/blog/post/fff-215) ·
  [FFF-421](https://www.factorio.com/blog/post/fff-421)
- [Data-oriented design](https://en.wikipedia.org/wiki/Data-oriented_design) ·
  [Building a data-oriented ECS in C++](https://cppcat.com/entity-component-system-implementation/) ·
  [The essence of ECS](https://arxiv.org/pdf/2606.14919)
- [A fast embedded language for continuous-time agent-based simulation (JASSS)](https://www.jasss.org/27/1/10.html) ·
  [Survey on agent-based simulation using hardware accelerators](https://arxiv.org/pdf/1807.01014)
- [FLAME GPU on NVIDIA GPUs](https://developer.nvidia.com/blog/fast-large-scale-agent-based-simulations-on-nvidia-gpus-with-flame-gpu/) ·
  [FLAME GPU 2 (SPE 2023)](https://onlinelibrary.wiley.com/doi/full/10.1002/spe.3207)
- [Tarn Adams — Simulation principles from Dwarf Fortress (Game AI Pro 2, ch. 41)](https://www.gameaipro.com/GameAIPro2/GameAIPro2_Chapter41_Simulation_Principles_from_Dwarf_Fortress.pdf) ·
  [Dwarf Fortress: an engineering marvel](https://medium.com/@christian.marques/dwarf-fortress-an-engineering-marvel-of-the-21st-century-2ba3a1e9b95f)
- [Towards a validation methodology for macroeconomic ABMs](https://d-nb.info/1246195569/34) ·
  [Empirical validation of agent-based models](https://www.sciencedirect.com/science/article/abs/pii/S1574002118300030)
- [Statistical model checking of the Keynes+Schumpeter model](https://arxiv.org/html/2605.10447) ·
  [SMC of the Island model](https://arxiv.org/html/2604.04543) ·
  [MultiVeStA + Mesa](https://link.springer.com/chapter/10.1007/978-3-031-75434-0_26) ·
  [Automated and distributed statistical analysis of economic ABMs](https://arxiv.org/pdf/2102.05405)
- [The use of surrogate models to analyse ABMs (JASSS)](https://www.jasss.org/24/2/3/3.pdf) ·
  [Calibrating a stochastic ABM using quantile-based emulation](https://doi.org/10.1137/17m1161233) ·
  [Equation-based versus agent-based models (JASSS)](https://www.jasss.org/26/4/3.html)
- [Metamorphic testing](https://en.wikipedia.org/wiki/Metamorphic_testing) ·
  [Metamorphic testing for hybrid simulation validation (NIST)](https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=932547) ·
  [Property-based testing tools for metamorphic testing](https://arxiv.org/abs/2211.12003) ·
  [Testing scientific software: a systematic literature review](https://arxiv.org/pdf/1804.01954)
