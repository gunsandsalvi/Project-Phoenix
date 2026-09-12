# Findings (13c.1)

Temporary. Every finding is written here when it is seen and leaves only by being POSITIONED into
the item that should fix it — into a `docs/plan/<item>.md`, or as an inserted item at its dependency
position — with the record saying where each landed (CLAUDE.md). The file goes when it is empty.

---

## 13c.1-1 — good ground stopped the capital programme binding (11 red in `capital.test.ts`)

**Measured.** After the ground reached the yield (13c.1 step 5), `test/capital.test.ts` went from
12 green to 11 red. Every failure is the same shape: `capital.commissioned` is empty where the test
expects at least one, and "has no project when it is running below its plant, and one when it is at
it" fails on the second half.

**Where it was seen.** `ranWorld('cap-c', 4)`, region `us.1`: plant standing on 4,894 km² of a
region whose arable walk gives `groundFor(..., 4894) = 1.3986`. The line's survival is now
`pow(0.92, 1 / (season × 1.3986))` where it was `pow(0.92, 1 / season)` — about 0.942 against 0.920
in an ordinary season, so the same plant and the same hours bring in more tonnes.

**What is ruled out.** Not the unit conversion: the register counts pieces and `landPerUnit` is per
named unit, and the conversion is made where the ratio is read (that defect was found and fixed
first — without it a farm stood on a thousand times its land and every yield in the world went to
nothing). Not the ground read: the same walk gives 1.649 on the best tile of `us.1` and 0.109 on the
single tile of `jp.2`, which is the dispersion the draw made.

**What it looks like.** Not a defect but a consequence: the world got more productive, so plant
stopped binding in a fixture built to make it bind. The tests assert a TIGHTNESS that the new
mechanism changed, and `tightWorld` is the fixture that sets it.

**Not chased** (Law 11, PLAN §5): the item carries on. It is re-measured at 13c.1's own measurement
step, against a world that also has voyages and a raised place resolution — and the fixture is
re-tightened there if it is still what this says it is.
