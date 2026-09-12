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

---

## 13c.1-2 — a hull cannot be repositioned, so ballast is not buildable

**Measured.** Freight's capacity is now the hulls a carrier has FREE, read off the register through
the lien a voyage binds them with. But `vintagesHeld` reads a party's plant **in its own region**
(`registry/physical.ts`: `if (terms.region !== view.self.region) continue`), and a plant vintage is
an instrument per `(kind, region, serviceDate)`. So a hull that lands somewhere else is still an
instrument of the region it was built in.

**What is ruled out.** Not the lien: it binds and releases correctly, and a hull on a voyage is
genuinely unavailable. Not the leg: a hull boards at its port and the path is real.

**Why it stops step 10.** Ballast is a carrier sailing a hull empty to where it is worth most, and
that means the hull's PLACE changes. Moving it needs either a plant reseat the kernel does not have
(`Instruments.reseat` changes an issuer, not a region) or the destination vintage instrument to
exist, and creating an instrument is a seed-only door. Either is a kernel change of real size.

**What was built instead, and what it costs.** A carrier serves the legs out of the place it is
based in, and its capacity there is its free hulls. That keeps everything Freight B2 and E2 ask for
— capacity fixed in the short run, none of it without an owner, none of it counted twice — and
loses one thing: a shortage on one leg cannot pull hulls off another. So freight capacity does not
reallocate between legs, and the price on a busy leg stays high longer than it should.

**Positioned, not chased.** It wants a kernel read that moves plant between places, which is an
inserted item of its own — it is the same door a firm would need to sell a working vintage to
somebody in another region, which 13c.2's file already names as belonging with 13g. The record says
where it lands when this item closes.

---

## 13c.1-3 — a period costs twenty-five seconds, and `markets` is twenty-one of them

**Measured.** One period of the foundation world, by phase:

```
one period: 25,817ms
  markets          22,614ms
  lending.write        630ms
  ratings.assess       363ms
  ... everything else under 250ms each
  freight.session      131ms
```

**What is ruled out — and this matters, because the map is the obvious suspect.** The same profile
run on `a702c37`, the commit before any map code, gives **24,575ms with `markets` at 21,493ms**. So
the cost is not the map, not the drawn places, not the sail phase and not the freight session: it is
what `markets` already cost.

There are 256 markets and **231 of them are equity**, one per listed line. Each sweeps every one of
the 3,185 parties for participation.

**What the map did add, and what was fixed.** Declaring goods in every home place put four lines
into two places with nobody in them — eight markets that cannot clear, each paying a full party
sweep every period. Goods are now declared where the firms that make them are (`settled()` in the
seed, sharing one `bestGround` with the placement so there is one rule and not two), and the goods
markets are back to four. It did not move the total, because the equity markets are the cost.

**Positioned, not chased.** This is 12c.1's territory (*the suite that got slower every period*) and
it is a traversal, not a mechanism — Law 18 says the economics may not change and the layout is
free. It wants a market to ask only the parties that could participate rather than all of them.
Named to **16**, where measurement lives.
