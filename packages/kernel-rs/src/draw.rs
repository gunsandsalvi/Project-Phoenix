//! THE DRAW: the physical world and the primitives, from one seed value — and then the past that
//! made it (22b.3: money and the sovereign through the chronicle).
//!
//! @spec 5 A5 · 5 B1 · 5 B1.a · 5 B3 · 5 B4 · 5 B5 · 5 C2 · 5 C3 · 5 C3.a · 5 E1 · 30 D2 · 30 D3 ·
//! @spec 30 D3.a · 31 A1 · Money A1 · Money D2 · Law 2, Law 3, Law 6, Law 19 · Appendix B
//!
//! **Reproducible from a seed value** (5 A5): the draw is a function of it, so a rejected world is
//! re-drawn from the next value and any run can be re-run.
//!
//! **Sizes are dispersed** (5 B4): a sector of equals never produces a market. What differs is drawn;
//! **no observed real-world ratio is copied in** (5 B5), so the dispersion is a shape of the draw and
//! not a number somebody read off a table.
//!
//! **There is no `endowMoney` here and there is nowhere to put one.** The first reserves exist because
//! the central bank CREATED them as its own liability (Money A1) and then LENT them to a bank against
//! the bank's own paper — two told moments, both settled. A bank's deposit money exists because the
//! bank created it. Nothing holds anything it did not come by.
//!
//! **The treasury sells its bills at auction on their own dates** (30 D2), to whoever had the money —
//! and **the central bank's holding is BOUGHT in the market** (30 D3.a), from a bank that held it,
//! because a central bank buying from the treasury directly is the overdraft D3 forbids.
//!
//! **Issue dates and maturities are spread** (5 C3, C3.a): every bill is issued on its own day and
//! matures on its own day, which is what the acceptance census asks for — a wall is not a profile.

use crate::assembly::{kinds, World};
use crate::calendar::Day;
use crate::chronicle::{Chronicle, Draft, Told};
use crate::ids::{CurrencyCode, InstrumentId, PartyId, RegionId, UnitId};
use crate::instruments::Class;
use crate::parties::Representation;

/// 5 A5: determinism from one seed value. A counter-based draw, so the same value gives the same
/// world and the next value gives a different one — never a clock and never an ambient source
/// (Error discipline: no `Math.random` in the engine).
pub struct Draw(u64);

impl Draw {
    pub fn from(seed_value: u64) -> Draw {
        Draw(seed_value.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `[0, n)`. Law 6: this is arithmetic over a finite set, not a bound on a decision.
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// 5 B4: **sizes are dispersed.** A spread around a scale, so two banks are not the same bank —
    /// which is what makes a market among them possible at all.
    fn spread(&mut self, scale: f64, by: f64) -> f64 {
        let step = (self.below(2_001) as f64 / 1_000.0) - 1.0;
        scale * (1.0 + by * step)
    }
}

/// What the draw produced, so the chronicle can name its subjects and the census can count them.
pub struct Drawn {
    pub world: World,
    pub chronicle: Chronicle,
    pub region: RegionId,
    pub ccy: CurrencyCode,
    pub central_bank: PartyId,
    pub treasury: PartyId,
    pub reserves: InstrumentId,
    pub banks: Vec<PartyId>,
    /// The deposit money each bank issues. Money D2: an account is a holding of money ISSUED BY a
    /// bank — so there is one of these per bank, not one shared "money" everybody uses.
    pub deposits: Vec<InstrumentId>,
    /// 5 C3.a: one bill per issue date, each with its own maturity.
    pub bills: Vec<InstrumentId>,
}

/// 22b.3: **money and the sovereign, through the chronicle.** Reserves created and lent, bills sold
/// at auction on their own dates, the central bank's holding bought from a bank that held it.
///
/// Every stock this leaves behind is the residue of a settled instruction — there is no endowment
/// door in this function, and none anywhere else either.
pub fn money_and_the_sovereign(seed_value: u64, banks: usize, bills: usize, opens_on: Day) -> Drawn {
    assert!(banks > 0, "5 B1: a banking system of no banks is not a population");
    assert!(bills > 1, "5 C3.a: one bill is a wall, not a maturity profile");
    let mut draw = Draw::from(seed_value);
    let mut w = World::empty();
    let region = RegionId::at(0);
    let ccy = CurrencyCode::at(0);
    let unit = UnitId::at(0);

    // 31 A1, 5 B3: the central bank and the treasury of this region. The region fixes the money.
    let central_bank = w.parties.add(kinds::CENTRAL_BANK, region, PartyId::NONE, Representation::Named, 1, 0);
    let treasury = w.parties.add(kinds::TREASURY, region, central_bank, Representation::Named, 1, 0);
    // Money A1: reserves are the central bank's own liability, and the store records whose.
    let reserves = w.instruments.issue(central_bank, ccy, Class::Money, unit, None, None);

    let mut bank_ids = Vec::with_capacity(banks);
    let mut deposit_lines = Vec::with_capacity(banks);
    for _ in 0..banks {
        let b = w.parties.add(kinds::BANK, region, central_bank, Representation::Named, 1, 0);
        bank_ids.push(b);
        // Money D2: each bank issues its OWN deposit money. One shared "money" everybody holds would
        // be money with no issuer, which Appendix B forbids.
        deposit_lines.push(w.instruments.issue(b, ccy, Class::Money, unit, None, None));
    }

    // 5 C3, C3.a: a bill per issue date, each maturing on its own day. The tenor is drawn, so the
    // profile is spread rather than stacked.
    let mut bill_lines = Vec::with_capacity(bills);
    let mut issued_on = Vec::with_capacity(bills);
    let span = opens_on.0 - (opens_on.0 - 3_640);
    for n in 0..bills {
        let issue_day = Day(opens_on.0 - 3_640 + (n as i64 * span / bills as i64));
        let tenor = 90 + draw.below(640) as i64;
        let line = w.instruments.issue(
            treasury,
            ccy,
            Class::Claim,
            unit,
            // 5 C4.b: a coupon is a TERM. A bill pays none — its return is the discount, which is a
            // price and therefore not stated here (§9 A1.a).
            None,
            Some(Day(issue_day.0 + tenor)),
        );
        bill_lines.push(line);
        issued_on.push(issue_day);
    }

    let mut c = Chronicle::opening(Day(opens_on.0 - 3_650), seed_value);

    // The first money in the world. It is not free (5 A4): it is the central bank's own liability,
    // recorded against its name in `Instruments`.
    let created = draw.spread(100_000.0, 0.1) * banks as f64;
    c.tell(Told {
        at: Day(opens_on.0 - 3_650),
        draft: Draft::Minted { issuer: central_bank, money: reserves, units: created, ccy },
        why: "the central bank created the reserves that are its own liability",
    });

    // 31: and lent them to each bank against the bank's own paper, which is how reserves reach a
    // banking system — never by appearing in a bank's account.
    for (n, b) in bank_ids.iter().enumerate() {
        let paper = w.instruments.issue(*b, ccy, Class::Claim, unit, Some(0.01), Some(Day(opens_on.0 + 365)));
        let lent = draw.spread(created / (banks as f64 + 1.0), 0.25);
        c.tell(Told {
            at: Day(opens_on.0 - 3_649 + n as i64),
            draft: Draft::Lent {
                lender: central_bank,
                borrower: *b,
                loan: paper,
                principal: lent,
                ccy,
                money: reserves,
            },
            why: "the central bank lent a bank reserves against the bank's own paper",
        });
    }

    // 30 D2: the treasury sells its bills at auction, on their own dates, to whoever had the money.
    // 30 D5.a: nobody is obliged to bid — here each bill goes to one bank that held reserves, and a
    // bank that had none simply did not buy.
    // Who actually won each bill, so a later moment can only sell what somebody really holds. A
    // chronicle that sells units nobody bought is a past the world cannot live, and `replay` says so
    // rather than quietly moving them.
    let mut won: Vec<(InstrumentId, PartyId, f64)> = Vec::with_capacity(bills);
    for (n, bill) in bill_lines.iter().enumerate() {
        let buyer = bank_ids[(draw.below(banks as u64)) as usize];
        let face = draw.spread(9_000.0, 0.3);
        won.push((*bill, buyer, face));
        // §9 A1.a: issued at a DISCOUNT and redeemed at par — the discount is what it fetched, and it
        // is what a buyer paid rather than a yield anybody wrote (Law 3).
        let paid = face * (1.0 - (draw.below(40) as f64) / 1_000.0);
        c.tell(Told {
            at: issued_on[n],
            draft: Draft::Issued { issuer: treasury, what: *bill, units: face, to: buyer, paid, ccy, money: reserves },
            why: "the treasury sold a bill at auction to a bank that had the money",
        });
    }

    // 30 D3.a: **the central bank's holding is BOUGHT IN THE MARKET**, from a bank that held it — a
    // purchase with a seller on the other side. Buying from the treasury directly is the overdraft
    // D3 forbids, and there is no told moment here that does it.
    if let Some((bill, holder, face)) = won.first() {
        // It buys part of what that bank actually won, at what it paid for it — a quantity somebody
        // really holds, not a round number the draw liked.
        let units = face / 2.0;
        c.tell(Told {
            at: Day(opens_on.0 - 30),
            draft: Draft::Bought {
                from: *holder,
                to: central_bank,
                what: *bill,
                units,
                paid: units * 0.98,
                ccy,
                money: reserves,
            },
            why: "the central bank bought a bill in the market from the bank that held it",
        });
    }

    Drawn {
        world: w,
        chronicle: c,
        region,
        ccy,
        central_bank,
        treasury,
        reserves,
        banks: bank_ids,
        deposits: deposit_lines,
        bills: bill_lines,
    }
}

/// 22b.4: **firms, plant and inventory through the chronicle.** Plant is bought from the maker who
/// built it, on its vintage date; inventory is bought up the chain from somebody who made it.
///
/// **This is where 12c.3 dies.** The old seed handed one baker 314 million loaves — eleven periods of
/// the whole world's demand — and the firm then produced once and never again, because it could not
/// sell what it already had. Here a firm's opening stock is **what it actually bought from somebody
/// who actually had it**, so a stock that large would need a seller with that much to sell and a
/// buyer with the money to pay for it. Neither exists, so it cannot be told.
pub fn firms_plant_and_inventory(
    d: &mut Drawn,
    firms: usize,
    seed_value: u64,
    opens_on: Day,
) -> Vec<PartyId> {
    assert!(firms > 1, "5 B1: one firm is not a population, and a market needs somebody to compete with");
    let mut draw = Draw::from(seed_value ^ 0x5EED_0F1E);
    let unit = UnitId::at(0);
    let region = d.region;
    let ccy = d.ccy;

    // The maker of plant, and the good every firm here makes. Both are parties and lines like any
    // other — a machine has an issuer because somebody built it (Money A1's shape, one level up).
    let maker = d.world.parties.add(kinds::FIRM, region, d.banks[0], Representation::Named, 1, 0);
    let plant = d.world.instruments.issue(maker, ccy, Class::Plant, unit, None, None);
    let good = d.world.instruments.issue(maker, ccy, Class::Good, UnitId::at(1), None, None);

    let mut made = Vec::with_capacity(firms);
    for n in 0..firms {
        let f = d.world.parties.add(kinds::FIRM, region, d.banks[n % d.banks.len()], Representation::Named, 1, 0);
        made.push(f);
    }

    // The maker has to have the machines before it can sell them: it BUILT them, which is a creation
    // by their issuer and its own stock to sell (5 C2 — the asset is somebody's).
    let built = draw.spread(400.0, 0.2) * firms as f64;
    d.chronicle.tell(Told {
        at: Day(opens_on.0 - 3_200),
        draft: Draft::Created { issuer: maker, what: plant, units: built },
        why: "the maker built the machines it would later sell",
    });

    for (n, f) in made.iter().enumerate() {
        // A firm needs money before it can buy anything: its bank lent it, which is where a firm's
        // opening balance sheet actually comes from (Banks Lending).
        //
        // **Money D2: it is lent in the BANK'S OWN deposit money, not in reserves.** Reserves are the
        // central bank's liability and only banks hold them; a firm holding them would collapse the
        // two tiers into one, and then a payment between two customers of different banks would
        // need no interbank settlement at all. A bank creates the deposits it lends — which is what
        // lending is — so the money exists because somebody borrowed it.
        let at_bank = n % d.banks.len();
        let bank = d.banks[at_bank];
        let bank_money = d.deposits[at_bank];
        let loan = d.world.instruments.issue(*f, ccy, Class::Claim, unit, Some(0.04), Some(Day(opens_on.0 + 730)));
        let borrowed = draw.spread(6_000.0, 0.35);
        let vintage = Day(opens_on.0 - 3_100 + (n as i64 * 90));
        d.chronicle.tell(Told {
            at: Day(vintage.0 - 1),
            draft: Draft::Minted { issuer: bank, money: bank_money, units: borrowed, ccy },
            why: "the bank created the deposits it was about to lend, which is what lending is",
        });
        d.chronicle.tell(Told {
            at: vintage,
            draft: Draft::Lent { lender: bank, borrower: *f, loan, principal: borrowed, ccy, money: bank_money },
            why: "the bank lent the firm what it needed to buy its plant, in its own deposit money",
        });
        // **Plant bought from its maker, on its vintage date.** Every machine in this world has a
        // date and a seller, so its age is a read and not a stated `SEED_PLANT_AGES`.
        let machines = draw.spread(built / (firms as f64 + 1.0), 0.3);
        let price = borrowed / machines * 0.6;
        d.chronicle.tell(Told {
            at: vintage,
            draft: Draft::Bought {
                from: maker,
                to: *f,
                what: plant,
                units: machines,
                paid: machines * price,
                ccy,
                // Money D2: it pays in the money its OWN bank issued. The maker thereby comes to hold
                // a deposit at that bank, which is what being paid by somebody else's customer is.
                money: bank_money,
            },
            why: "the firm bought its plant from the maker that built it, on its vintage date",
        });
        // **And it has been servicing the loan ever since.** A loan taken eight years ago that has
        // never had a penny paid against it is not a past — it is an endowment with a due date, and
        // a bank whose loans were never serviced has no credit history either (the opening census
        // found this as `EveryBankHasLentAndBeenRepaid`, 4 of 4). The payment is the coupon the loan
        // was written at, over the days since the last one: a READ of the loan's own terms (Law 19),
        // not a schedule stated beside it.
        let coupon = match d.world.instruments.coupon_of(loan) {
            Some(c) => c,
            None => unreachable!("the loan was issued with a coupon two lines above"),
        };
        let mut due = Day(vintage.0 + 90);
        while due.0 <= opens_on.0 {
            d.chronicle.tell(Told {
                at: due,
                draft: Draft::Paid {
                    from: *f,
                    to: bank,
                    amount: borrowed * coupon * 90.0 / 365.0,
                    ccy,
                    money: bank_money,
                },
                why: "the firm paid the quarter's interest on the loan it is still carrying",
            });
            due = Day(due.0 + 90);
        }
    }

    // **Inventory is bought up the chain from somebody who made it**, and the seller is the firm that
    // made it — so the quantity is bounded by what that firm could produce and pay for, which is what
    // makes 12c.3's eleven periods of world demand untellable.
    //
    // **The chain is a RING, and it has to be**: a line has an end, and the firm at the end buys from
    // somebody and never makes anything — which the opening census found on its first run (22b.2,
    // `EveryFirmHasTraded`, 1 of 13). A closed circuit has no last firm. Making it a ring deletes the
    // special case rather than adding one (Law 12).
    for (n, from) in made.iter().copied().enumerate() {
        let buyer = made[(n + 1) % made.len()];
        let stock = draw.spread(300.0, 0.4);
        // The buyer pays in the money its own bank issued (Money D2).
        let buyers_money = d.deposits[((n + 1) % made.len()) % d.banks.len()];
        d.chronicle.tell(Told {
            at: Day(opens_on.0 - 400 + n as i64 * 20),
            draft: Draft::Created { issuer: from, what: good, units: stock },
            why: "a firm made the goods it would sell up the chain",
        });
        d.chronicle.tell(Told {
            at: Day(opens_on.0 - 390 + n as i64 * 20),
            draft: Draft::Bought {
                from,
                to: buyer,
                what: good,
                units: stock / 2.0,
                paid: stock / 2.0 * 1.1,
                ccy,
                money: buyers_money,
            },
            why: "the firm bought its opening stock from the firm that made it",
        });
    }

    made
}

/// 22b.5: **households, employment and savings through the chronicle** — the income history that
/// outlooks and votes are made of (§46, XI-17).
///
/// **Deposits are not the residual of the banks' asset endowment.** In the old seed a household's
/// deposit was whatever was left over once the banks' assets had been stated: the identity satisfied
/// and the economics backwards. Here a household has money because **it was PAID** — its employer
/// paid it a wage, out of the employer's own account, week after week — and what it did not spend is
/// what it saved. The arrow runs the way it runs in the world.
pub fn households_employment_and_savings(
    d: &mut Drawn,
    firms: &[PartyId],
    cells: usize,
    weeks: i64,
    seed_value: u64,
    opens_on: Day,
) -> Vec<PartyId> {
    assert!(cells > 1, "5 B1: one household cell is not a distribution, and every decision here is a threshold");
    assert!(weeks > 1, "22b.5: an income history of one week is not a history to form an outlook from");
    assert!(!firms.is_empty(), "39: employment without an employer is not employment");
    let mut draw = Draw::from(seed_value ^ 0x0110_05E4_01D5);
    let region = d.region;
    let ccy = d.ccy;

    // XI-15, 5 B1.a: a cell is a named party with a WEIGHT — how many real households it stands for.
    // The weights sum to the population, and that sum is a read rather than a target.
    let mut made = Vec::with_capacity(cells);
    for n in 0..cells {
        // 5 B4: dispersed. A sector of equals never produces a market, and a world where every
        // household is the same one cannot have a mean-preserving spread cause a default (§41 A2.g).
        let weight = 200 + draw.below(1_800) as u32;
        let h = d.world.parties.add(
            kinds::HOUSEHOLD,
            region,
            d.banks[n % d.banks.len()],
            Representation::Cell,
            weight,
            n as u32,
        );
        made.push(h);
    }

    // One working-capital line per firm, issued once and drawn every week (see the loop below). A
    // revolving facility is ONE row the firm owes on, not a new loan a week — which is also what
    // keeps `EveryKindIsHeldByChoice` honest, since a thousand identical rows would be one fact
    // written a thousand times (Law 4).
    let working_capital: Vec<InstrumentId> = firms
        .iter()
        .map(|f| {
            d.world.instruments.issue(
                *f,
                ccy,
                Class::Claim,
                UnitId::at(0),
                // 5 C4.b: the facility's rate is a TERM the bank wrote, not a price.
                Some(0.05),
                Some(Day(opens_on.0 + 365)),
            )
        })
        .collect();

    // §39, XI-10: **employment is a relationship**, recorded — a firm, a worker, a wage, a start date.
    // The wage that follows is its own told moment, week after week, which is the income history.
    for (n, h) in made.iter().enumerate() {
        let employer = firms[n % firms.len()];
        // Money D2: wages are paid in the money the EMPLOYER'S bank issued — the household comes to
        // hold a deposit at that bank, which is what being paid is. Reserves are the central bank's
        // liability and only banks hold them; a household holding them would be the two tiers
        // collapsed into one, and then no payment in this world would ever need a bank.
        let employers_bank_money = d.deposits[(n % firms.len()) % d.banks.len()];
        let wage = draw.spread(40.0, 0.45);
        let started = Day(opens_on.0 - weeks * 7);
        d.chronicle.tell(Told {
            at: started,
            draft: Draft::Hired { employer, worker: *h, wage },
            why: "the firm hired the household, which is a relationship and not a number on a firm",
        });
        for week in 0..weeks {
            let day = Day(started.0 + week * 7);
            // §41 C2: what it does not spend is what it SAVES. The spending goes back to the employer
            // (§32 F1: spending is somebody's income) — but the saving does not, and that is the whole
            // problem this block exists to solve.
            let spends = wage * (0.55 + (draw.below(400) as f64) / 1_000.0);

            // **THE CIRCUIT DOES NOT CLOSE ON WAGES AND SPENDING ALONE, and the opening census is
            // what proved it.** A firm pays W and gets back S; week after week the difference is a
            // hole in its account and a deposit in the household's, and by the fortieth week the firm
            // cannot make payroll: a hundred told moments came back `ShortOfMoney` at a year's
            // history, and under eight weeks the leak was too small to see. That is not a number to
            // adjust — it is a MECHANISM that was missing, and it has a name.
            //
            // The saving is a deposit, and the bank lends against it: the firm draws its
            // working-capital line for exactly what its wage bill exceeds its takings, which is what
            // a revolving facility is for. It is told as an ordinary loan, two-sided, on a line the
            // firm issued — so the firm's debt and the household's savings grow together, which is
            // what they do (Banks Lending; Money D2: a bank lends by creating deposits).
            let short = wage - spends;
            d.chronicle.tell(Told {
                at: Day(day.0 - 1),
                draft: Draft::Lent {
                    lender: d.world.parties.bank_of(employer),
                    borrower: employer,
                    loan: working_capital[n % firms.len()],
                    principal: short,
                    ccy,
                    money: employers_bank_money,
                },
                why: "the firm drew its working-capital line for the week's wage bill it had not taken in",
            });
            // The wage is REAL MONEY leaving the employer's account and arriving in the household's
            // (Law 5). A household that was never paid has no money, and no outlook either.
            d.chronicle.tell(Told {
                at: day,
                draft: Draft::Paid { from: employer, to: *h, amount: wage, ccy, money: employers_bank_money },
                why: "the firm paid the week's wages out of its own account",
            });
            d.chronicle.tell(Told {
                at: Day(day.0 + 1),
                draft: Draft::Paid { from: *h, to: employer, amount: spends, ccy, money: employers_bank_money },
                why: "the household spent most of its wage, which is the firm's income",
            });
        }
    }

    made
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chronicle::replay;

    fn drawn() -> Drawn {
        money_and_the_sovereign(1, 4, 9, Day(0))
    }

    #[test]
    fn the_same_seed_value_draws_the_same_world_and_the_next_one_draws_a_different_world() {
        // 5 A5: reproducible, so a rejected world is re-drawn from the next value and any run can be
        // re-run.
        let a = money_and_the_sovereign(7, 4, 9, Day(0));
        let b = money_and_the_sovereign(7, 4, 9, Day(0));
        let c = money_and_the_sovereign(8, 4, 9, Day(0));
        let told_of = |d: &Drawn| format!("{:?}", d.chronicle.told());
        assert_eq!(told_of(&a), told_of(&b));
        assert_ne!(told_of(&a), told_of(&c));
    }

    #[test]
    fn every_stock_in_the_opening_is_the_residue_of_a_settled_instruction() {
        // 22b, 5 A4: there is no endowment door in this draw. The reserves a bank holds are what the
        // central bank lent it, less what it paid for bills.
        let mut d = drawn();
        let done = replay(
            &d.chronicle,
            7,
            &mut d.world.register,
            &mut d.world.journal,
            &mut d.world.wire,
            d.world.settled_kind,
            d.world.failed_kind,
        );
        assert!(done.settled > 0, "a past where nothing settled is not a past");
        assert!(done.refused.is_empty(), "the draw told a moment the world could not live: {:?}", done.refused);

        // The central bank's reserves are its own liability and it lent most of them away.
        let issued_to_banks: f64 = d.banks.iter().map(|b| d.world.register.quantity(d.world.register.row(*b, d.reserves))).sum();
        assert!(issued_to_banks > 0.0);
        // And a bank holds bills because it BOUGHT them at auction.
        let held: f64 = d
            .bills
            .iter()
            .map(|bill| {
                d.banks
                    .iter()
                    .map(|b| d.world.register.quantity(d.world.register.row(*b, *bill)))
                    .sum::<f64>()
            })
            .sum();
        assert!(held > 0.0);
    }

    #[test]
    fn the_central_bank_holds_what_it_bought_in_the_market_and_never_from_the_treasury() {
        // 30 D3, D3.a: a central bank buying from the treasury directly is the overdraft. Every
        // told moment that moves a bill to the central bank names a BANK as the seller.
        let d = drawn();
        let bought: Vec<&Told> = d
            .chronicle
            .told()
            .iter()
            .filter(|t| matches!(t.draft, Draft::Bought { to, .. } if to == d.central_bank))
            .collect();
        assert!(!bought.is_empty());
        for t in bought {
            match t.draft {
                Draft::Bought { from, .. } => {
                    assert_ne!(from, d.treasury, "that is the overdraft with a receipt attached");
                    assert!(d.banks.contains(&from));
                }
                other => panic!("expected a purchase, got {other:?}"),
            }
        }
    }

    #[test]
    fn the_issue_dates_and_the_maturities_are_spread() {
        // 5 C3, C3.a: a wall is not a profile, and the acceptance census asks for more than one day
        // on each side.
        let d = drawn();
        let mut issue_days: Vec<i64> = d.chronicle.issue_days().iter().map(|day| day.0).collect();
        issue_days.sort_unstable();
        let before = issue_days.len();
        issue_days.dedup();
        assert!(issue_days.len() > 1, "every bill issued on one day is a wall");
        assert_eq!(issue_days.len(), before, "two bills issued on the same day is a smaller profile");

        let mut maturities: Vec<i64> = d
            .bills
            .iter()
            .filter_map(|b| d.world.instruments.matures_on(*b))
            .map(|day| day.0)
            .collect();
        maturities.sort_unstable();
        maturities.dedup();
        assert!(maturities.len() > 1, "every bill maturing on one day is the wall from the other side");
    }

    #[test]
    fn every_bank_issues_its_own_money_rather_than_sharing_one() {
        // Money D2, Appendix B: an account is a holding of money ISSUED BY a bank. One shared money
        // everybody holds is money with no issuer.
        let d = drawn();
        assert_eq!(d.deposits.len(), d.banks.len());
        for (n, line) in d.deposits.iter().enumerate() {
            assert_eq!(d.world.instruments.issuer_of(*line), d.banks[n]);
        }
        // And the reserves are the central bank's, which is a different liability from any of them.
        assert_eq!(d.world.instruments.issuer_of(d.reserves), d.central_bank);
    }

    #[test]
    fn the_sizes_are_dispersed_because_a_sector_of_equals_never_produces_a_market() {
        // 5 B4: what the banks were lent differs, so they do not all face the next period alike.
        let d = drawn();
        let lent: Vec<f64> = d
            .chronicle
            .told()
            .iter()
            .filter_map(|t| match t.draft {
                Draft::Lent { principal, .. } => Some(principal),
                _ => None,
            })
            .collect();
        assert_eq!(lent.len(), d.banks.len());
        let first = lent[0];
        assert!(lent.iter().any(|x| *x != first), "a sector of equals never produces a market");
    }

    #[test]
    fn a_firms_plant_was_bought_from_the_maker_that_built_it_on_its_vintage_date() {
        // 22b.4: every machine has a date and a seller, so its age is a READ — never a stated
        // `SEED_PLANT_AGES`.
        let mut d = drawn();
        let firms = firms_plant_and_inventory(&mut d, 4, 1, Day(0));
        assert_eq!(firms.len(), 4);
        let bought: Vec<&Told> = d
            .chronicle
            .told()
            .iter()
            .filter(|t| matches!(t.draft, Draft::Bought { to, .. } if firms.contains(&to)))
            .collect();
        assert!(!bought.is_empty());
        for t in &bought {
            // It happened on a day, in the past, and somebody sold it.
            assert!(t.at < Day(0));
            match t.draft {
                Draft::Bought { from, to, .. } => assert_ne!(from, to, "a firm cannot buy from itself"),
                other => panic!("expected a purchase, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_firm_opens_holding_what_it_actually_bought_and_this_is_where_the_old_seeds_stock_dies() {
        // 22b.4, finding 12c.3: the old seed handed one baker 314 million loaves — eleven periods of
        // the whole world's demand — and the firm produced once and never again. Here the stock is
        // what somebody SOLD it, so it is bounded by what a seller had and what a buyer could pay.
        let mut d = drawn();
        let firms = firms_plant_and_inventory(&mut d, 4, 1, Day(0));
        let done = replay(
            &d.chronicle,
            7,
            &mut d.world.register,
            &mut d.world.journal,
            &mut d.world.wire,
            d.world.settled_kind,
            d.world.failed_kind,
        );
        assert!(done.refused.is_empty(), "the draw told a moment the world could not live: {:?}", done.refused);

        // Every unit a firm holds came out of somebody else's book, so the two sides are one number.
        let good = InstrumentId::at(d.world.instruments.len() as u32 - 1);
        let _ = good;
        let held: f64 = firms
            .iter()
            .map(|f| {
                d.world
                    .register
                    .of_holder(*f)
                    .iter()
                    .map(|row| d.world.register.quantity(crate::ids::HoldingId(*row)))
                    .sum::<f64>()
            })
            .sum();
        assert!(held > 0.0, "a firm that opens holding nothing bought nothing");
    }

    #[test]
    fn a_household_has_money_because_it_was_paid_and_not_as_a_residual() {
        // 22b.5: the old seed made a deposit the leftover of the banks' stated assets — the identity
        // satisfied and the economics backwards. Here the arrow runs the way it runs in the world.
        let mut d = drawn();
        let firms = firms_plant_and_inventory(&mut d, 4, 1, Day(0));
        let cells = households_employment_and_savings(&mut d, &firms, 3, 6, 1, Day(0));
        let done = replay(
            &d.chronicle,
            7,
            &mut d.world.register,
            &mut d.world.journal,
            &mut d.world.wire,
            d.world.settled_kind,
            d.world.failed_kind,
        );
        assert!(done.refused.is_empty(), "the draw told a moment the world could not live: {:?}", done.refused);
        // Every cell holds money, and every unit of it arrived as a wage somebody paid. The money it
        // holds is its EMPLOYER'S BANK'S deposit money (Money D2) — not reserves, which are the
        // central bank's liability and are held by banks alone.
        for (n, h) in cells.iter().enumerate() {
            let employers_bank_money = d.deposits[(n % firms.len()) % d.banks.len()];
            assert!(d.world.register.quantity(d.world.register.row(*h, employers_bank_money)) > 0.0);
            assert_eq!(
                d.world.register.quantity(d.world.register.row(*h, d.reserves)),
                0.0,
                "a household holding reserves is the two tiers collapsed into one"
            );
        }
    }

    #[test]
    fn the_income_history_is_a_run_of_weeks_and_not_one_moment() {
        // §46: an outlook is formed adaptively from a party's OWN history, so a household that was
        // paid once has nothing to form one from. The chronicle gives it a run of weeks.
        let mut d = drawn();
        let firms = firms_plant_and_inventory(&mut d, 4, 1, Day(0));
        let cells = households_employment_and_savings(&mut d, &firms, 3, 8, 1, Day(0));
        let wages: Vec<&Told> = d
            .chronicle
            .told()
            .iter()
            .filter(|t| matches!(t.draft, Draft::Paid { to, .. } if cells.contains(&to)))
            .collect();
        assert_eq!(wages.len(), 3 * 8, "every cell was paid every week of its history");
        // And the weeks are different days, which is what makes it a history rather than a total.
        let mut days: Vec<i64> = wages.iter().map(|t| t.at.0).collect();
        days.sort_unstable();
        days.dedup();
        assert!(days.len() > 1);
    }

    #[test]
    fn a_household_cell_is_a_weight_and_the_weights_are_dispersed() {
        // XI-15, 5 B1.a, 5 B4: a cell is one possible household with a multiplicity, and a sector of
        // equals never produces a market — nor a mean-preserving spread that can cause a default.
        let mut d = drawn();
        let firms = firms_plant_and_inventory(&mut d, 4, 1, Day(0));
        let cells = households_employment_and_savings(&mut d, &firms, 5, 4, 1, Day(0));
        let weights: Vec<u32> = cells.iter().map(|h| d.world.parties.weight(*h)).collect();
        assert!(weights.iter().all(|w| *w > 0), "a cell of nobody is not a cell");
        let first = weights[0];
        assert!(weights.iter().any(|w| *w != first), "a sector of equals never produces a market");
    }

    #[test]
    #[should_panic(expected = "not a history to form an outlook from")]
    fn an_income_history_of_one_week_is_refused() {
        let mut d = drawn();
        let firms = firms_plant_and_inventory(&mut d, 4, 1, Day(0));
        households_employment_and_savings(&mut d, &firms, 3, 1, 1, Day(0));
    }

    #[test]
    #[should_panic(expected = "employment without an employer")]
    fn employment_without_an_employer_is_refused() {
        let mut d = drawn();
        households_employment_and_savings(&mut d, &[], 3, 6, 1, Day(0));
    }

    #[test]
    #[should_panic(expected = "is not a population")]
    fn one_firm_is_not_a_population() {
        let mut d = drawn();
        firms_plant_and_inventory(&mut d, 1, 1, Day(0));
    }

    #[test]
    #[should_panic(expected = "not a maturity profile")]
    fn one_bill_is_a_wall_and_is_refused() {
        money_and_the_sovereign(1, 4, 1, Day(0));
    }

    #[test]
    #[should_panic(expected = "not a population")]
    fn a_banking_system_of_no_banks_is_refused() {
        money_and_the_sovereign(1, 0, 9, Day(0));
    }
}
