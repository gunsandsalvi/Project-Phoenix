# Findings

Every red test in this tree, named, with what was measured and where it was seen. The rule this file
exists for is CLAUDE.md's: a finding not written down is lost, and a finding leaves this file only
by being PLACED — moved into the `docs/plan/<item>.md` of the item that should fix it, or inserted
as its own item at its dependency position.

Measured on the whole suite: **82 red of 662** after item 13j gave this world four economies. It was
74 before it, and the eight are named in their own section at the foot of this file. What follows
groups them by CAUSE rather than by file, because the files are a symptom of about six causes.

One of the fixes below does not show in that count and is the largest of them: `equity`'s waterfall
test threw `Register C1` and stopped the file; it now runs to an ordinary assertion. A crash and a
red are both one line in a summary and they are not the same thing.

---

## Fixed in this pass

| what | where it was seen |
|---|---|
| A ceased securitisation vehicle was still being instructed, five hard `Money E4` throws across five files. A vehicle can fail; when it does, its estate assumes its paper and winds the pool up, and the module has nothing left to instruct. | `irs`, `bank-capital`, `bank-resolution`, `households`, `money-market` |
| A firm holding NONE of an input reached MINUS ONE piece of it, so the least limit was negative and a **negative batch** went to settlement, where the register refused it (`Register C1`). The piece subtracted to leave room for the draw's round-up was never needed. | `equity` |
| "Gets dearer for a borrower that has failed to pay" compared `credit.written[0]` from two worlds — the period-2 overdraft against the period-6 request. Like for like the mechanism was always right: 0.0272 against 0.0219. | `loans` |
| Thirty-two lines with no store carried a `storageWhy` explaining why there is none — the same fact `portable: false` already states (Law 4). | `storage` |
| Capital was asserted to be the equity account, which stopped being true when item 11 built the subordinated layer. It is the residual PLUS what stands in front of the creditors (A2.b). | `bank-capital` |

---

## 1. The rig has no firm in most lines (≈ 20 red)

`RIG_PER_LINE` is 0, so twelve firms spread over sixty-two lines by their real shares leave most of
them empty. A test that asks the draw for a mill, a bakery or a storage builder is told "it drew 0",
and mechanisms that need a producer — storage, spoilage, the consumer basket, a cargo to ship —
never run, which is what "expected 0 to be greater than 0" says thirty times over.

**Measured, not assumed**: setting it to ONE takes the suite from 79 red to **139**. A rig with every
line in it is a different small world — more firms, more seekers in a labour venue, different fills,
different prices — and the assertions across a dozen files are written against the world this rig
makes today. Those tests are not wrong, they are SPECIFIC.

One blocker to raising it is now gone: `rigSpec` sized the population from the number of firms
REQUESTED rather than the number the draw made, so every extra firm moved the people-to-firm ratio
under it. It reads the draw now.

→ **Item 16**, as one bounded change: re-size the scale model and re-derive what every affected test
asserts. `capital.test.ts` and `firms.test.ts` fail at FILE level on this.

## 2. The foreign countries are stubs (≈ 6 red)

Three of the four countries are a central bank, a treasury and a bond line. There is no foreign
economy, so no FX book has ever had a bid, nothing crosses a border as goods, and every claim that
depends on either measures nothing. This is the session's own `12d-18`, `12d-15`, `12d-19`, `12b-3`,
`12b-4` under one cause.

→ **13i**, which closed with the external accounts built and the foreign economies not.

## 3. A bank that is insolvent is never resolved (≈ 9 red)

Finding `13b-12`, measured before this session and unchanged by it: a bank published capital of
−31,237,415,456 and went on making a market for twenty-three periods. Both triggers must exist and
the resolution must name which one fired (Banks Capital C1.a); whether the solvency trigger is not
reading the published position, or is reading it and the resolution is not being run, is still
unmeasured — and that is the question, not the quote.

→ **13f's** carried findings. `bank-resolution`, `resolution/banks`, `resolution/cells`, parts of
`deposits` and `run`.

## 4. An exchange-traded fund cannot create (≈ 7 red)

`12d-8`: a creation delivers a slice of the book, and a desk that has not got the basket does not
create (`funds/etf.ts`, F1). Nothing moves a share back to a desk, so `E3` runs one way only and the
premium — measured at 0.28 of NAV, twenty-six times a period of carry — has nobody able to close it.

The mechanism that answers it now EXISTS: securities lending was built this session, where a desk
borrows the units it must deliver. Wiring it in is not a fix but a build — a module may not import
another, so "whoever must deliver may borrow" needs a kernel door of the shape `termsOffered` has.

→ **13f**, as its own step, now that the borrow market is there to call.

## 5. Two banks, and everything that needs a third (≈ 4 red)

`research` wants coverage of a name to VARY across banks, `deposits` wants a class to split rather
than cross, `dealing` wants a market to fail when the desks step back. With three banks and one
listed line, a count that should be an outcome has one value, and a coincidence is indistinguishable
from a rule.

→ **Item 16**, with 1: it is the same scale-model question.

## 6. Singletons, each its own cause (≈ 9 red)

`omo` remittance and run-off; `raise` when nobody will; `ratings` ageing out; `equity-anchor` against
the published book; `indices` observation; `tick`; `environment`'s crop; `treasury`'s receipts;
`world`'s year-long chain. Each needs reading on its own and none is grouped here out of laziness —
they are simply not the same bug.

→ **Item 16**, which is where Part XII measures this world and is the item that should read them.


---

## What item 13j cost: eight reds, and the shape of them

13j turned three stubs into economies. Measured across the whole suite, three times:

| world the rig builds | red of 662 |
|---|---|
| before 13j (one country, the other three stubs) | 74 |
| 13j with the rig opening all four countries | **303** |
| 13j with the rig opening one, and the currency tests four | **82** |

The 303 is the finding and not the bug: a dozen firms and three banks spread over four countries
gives each a country with two firms and no banking system its own depositors could fund, and
`Seed D1` refuses to open exactly that — ninety-four times. **A world's count of countries is a
RESOLUTION** like its count of banks and its count of firms, so the rig opens one and scales it, and
the mechanisms that cannot exist in a world of one country — the currency layer, the pairs, the
open-market book, the per-region indices — get `abroadWorld`, the same construction with four rows
and four times the draw.

The eight that remain are all one shape: **a test that named the world it was written against.**
They are not mechanism failures — the audit reports nothing on the four-country world, every party
banks in its own money, and every region's external accounts net to zero over thousands of crossing
legs on both sides. They are assertions about a world with one sovereign issuer, one banking system
and one money, being made of a world with four.

| where | what it names that has changed |
|---|---|
| `currency`, `spot-fx` | Counts of pairs, desks and positions taken against a world whose only foreign parties were three reserve managers. There are now firms, banks and households on both sides of every pair. |
| `omo` | The base, the remittance and the run-off measured against one central bank's book. There are four, and the rig's four-country model is a quarter the size of its one-country one per country. |
| `indices` | An index per region and per currency, asserted against the four regions of a world whose three abroad had no constituents. Every one of them now has a market with firms listed in it. |
| `opening-liquidity`, `bank-capital`, `deposits`, `money-market` | The opening banking system, asserted against every bank in the world being American. A bank's country now decides whose money it issues, whose window it can go to and whose guarantee stands behind it. |
| `world`, `run`, `households`, `etf`, `equity`, `estate`, `goods`, `storage`, `freight`, `funds`, `raise`, `ratings`, `research`, `treasury`, `environment`, `equity-anchor` | Sizes, counts and totals of a world that is a different size. |

**Where they go.** These are POSITIONED to item 16 (Measure), which is where the plan already puts
the work of making every measurement true of the world as it then stands — it carries the level from
eight earlier measurements of it for the same reason. They are not positioned to 13j, because 13j's
own exit is a world that opens, runs and audits clean at four countries, and it does.
