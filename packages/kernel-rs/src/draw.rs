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
        draft: Draft::Created { issuer: central_bank, what: reserves, units: created },
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
