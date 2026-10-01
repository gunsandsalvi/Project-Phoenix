//! The sovereigns' bills on the core. Each country's treasury auctions bills at its fund stage on its declared weekday,
//! before the facilities: its minister offers the face that keeps its cash at its buffer of weeks of last week's
//! outflow after the week's maturities; each bank's chief executive bids its reserves above its target at the price
//! whose yield is the deposit facility's rate; a uniform-price auction clears them, and what is not sold is not issued.
//! A bill is a contract from the treasury to its holder, its balance the price paid, reckoned from terms at the yield
//! its price gives and paid in full at maturity. At the opening each bank holds the government paper its country's
//! sheet gives it as bills maturing over a bill's weeks, one tranche a week.

use std::collections::BTreeMap;

use if_state::kinds::{Allotment, Bid, BidIn, BillKind, BillLaw, SizeIn};
use phx_core::OpeningCountry;
use phx_core::calendar::Calendar;
use phx_core::calendar::daycount::DayCount;
use phx_core::findings::{Finding, FindingOwner, Unit};
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_ledger::algebra::{Leg, Reference, Schedule};
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::consts::reason::REPAID;
use crate::consts::{DAYS_A_WEEK, DAYS_A_YEAR, KIND_ROWS_PER_CHUNK};
use crate::core::Core;
use crate::core_central::Step;
use crate::core_day::{DatedFamily, Due};
use phx_core::capacity::KIND_ROWS;

/// The bills' family: each a treasury's debt to a bank.
pub const BILLS: &str = crate::consts::families::BILLS;

/// An auction's result: its day and country, the bills offered, bid for and sold, the price every winner paid a unit
/// of face, the cover (bids over offer) and the tail (the mean accepted bid's price over the price paid).
#[derive(Clone, Copy, Debug, PartialEq, phx_macros::Saved)]
pub struct Auction {
    pub day: u32,
    pub country: u8,
    pub offered: i64,
    pub bid: i64,
    pub sold: i64,
    pub price: Missing<f64>,
    pub cover: Missing<f64>,
    pub tail: Missing<f64>,
}

/// An issue of bills: its country, the day it was issued and its maturity, its schedule among the family's, what its
/// holders paid for it and the face it pays at maturity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Issue {
    pub country: u8,
    pub day: Day,
    pub maturity: Day,
    pub schedule: u32,
    pub issued: i128,
    pub face: i128,
}

/// The bills: the kind and each country's law, each treasury's cash at its last auction, the auctions and issues, and
/// the debt issued, redeemed and written off since the opening.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Bills {
    #[saved(skip)]
    pub(crate) kind: Option<BillKind>,
    laws: Vec<Option<BillLaw>>,
    last_cash: Vec<Option<i64>>,
    pub auctions: Vec<Auction>,
    pub issues: Vec<Issue>,
    pub issued: i128,
    pub redeemed: i128,
    pub written: i128,
}

/// A weekday's place in the week, Monday first.
fn weekday_index(date: phx_id::Date) -> Option<u32> {
    use phx_id::Weekday::{Friday, Monday, Saturday, Sunday, Thursday, Tuesday, Wednesday};
    let week = [Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday];
    week.iter().position(|w| *w == date.weekday()).and_then(|i| u32::try_from(i).ok())
}

/// The price a unit of face whose yield over `days` is `rate` a year.
fn price_at(rate: f64, days: f64) -> f64 {
    1.0 / (1.0 + rate * days / DAYS_A_YEAR)
}

/// An auction's tail: the mean price its winners bid for what they won, over the price they paid.
fn tail(a: &Allotment, bids: &[Bid]) -> Missing<f64> {
    let won: i64 = a.won.iter().map(|(_, n)| *n).sum();
    if won <= 0 {
        return Missing::Absent;
    }
    let mut taken = 0.0;
    for (b, n) in &a.won {
        let mut best = a.price;
        for x in bids.iter().filter(|x| x.bidder == *b) {
            if x.price > best {
                best = x.price;
            }
        }
        taken += best * phx_rand::float::from_i64(*n);
    }
    Missing::Present(taken / phx_rand::float::from_i64(won) - a.price)
}

impl Core {
    /// The bills opened: the family, each country's law, and each bank's government paper from its country's sheet,
    /// shared over its banks by their weights, held as bills maturing one tranche a week over a bill's weeks, each at
    /// the deposit facility's rate.
    ///
    /// # Errors
    /// A bill law the state does not hold for a country whose sheet gives its banks paper.
    #[clause("SOV.1", "GEN.2")]
    pub(crate) fn open_bills(
        &mut self,
        state: &crate::state::State,
        (countries, sheets): (&[OpeningCountry], &[crate::opening::sheet::Sheet]),
        (calendar, today): (&Calendar, Day),
    ) -> Result<(), String> {
        let (Some(treasury), Some(bank)) = (self.bound.kinds.treasury, self.bank_kind) else {
            return Ok(());
        };
        // A treasury is founded by its state with no preference of its own: its minister's decisions read none.
        self.found(crate::core::kind_number(treasury), crate::core_decide::Founding::Shared(phx_core::Prefs::NONE));
        let laws: Vec<Option<BillLaw>> = state
            .countries
            .iter()
            .map(|c| match &c.bills {
                Missing::Present(l) => Some(*l),
                Missing::Absent => None,
            })
            .collect();
        self.bills = Bills { kind: state.bills, last_cash: vec![None; laws.len()], laws, ..Bills::default() };
        let rows = [self.kind_rows(treasury), self.kind_rows(usize::from(bank))];
        let family = DatedFamily {
            name: BILLS,
            store: phx_core::store::Family::new(
                &mut self.space,
                ([crate::core::kind_number(treasury), bank], rows),
                (KIND_ROWS, KIND_ROWS_PER_CHUNK),
                [true, true],
                (today.succ(), phx_core::capacity::WHEEL_DAYS),
            ),
            reason: REPAID,
            schedules: Vec::new(),
            classes: Vec::new(),
            terms: Vec::new(),
            ends_after: Vec::new(),
            finishing: Vec::new(),
            alike: BTreeMap::new(),
            alike_upto: 0,
            moves: crate::core_day::LoanMoves::default(),
            lost: 0,
        };
        self.add_family(family);
        for (c, (country, sheet)) in countries.iter().zip(sheets).enumerate() {
            let total = phx_ledger::opening::whole(
                sheet.at(crate::consts::sheet::GOVERNMENT_PAPER, crate::consts::sheet::BANKS) * country.gdp,
            );
            if total <= 0 {
                continue;
            }
            let (Some(Some(law)), Some(Some(debtor)), Some(corridor)) = (
                self.bills.laws.get(c).copied(),
                self.treasuries.get(c).copied(),
                self.central.corridors.get(c).copied(),
            ) else {
                return Err(format!("country {c}: its banks hold government paper and it has no bills"));
            };
            let banks = self.banks_of.get(c).cloned().unwrap_or_default();
            let weights: Vec<u64> = banks.iter().map(|(_, w)| *w).collect();
            let parts = self.apportion(("banks' government paper", country.id.get()), total, &weights);
            for ((s, _), part) in banks.iter().zip(parts).filter(|(_, p)| *p > 0) {
                let holder = PartyKey::new(bank, Slot::new(*s));
                let weeks = usize::from(law.weeks);
                let tranches = crate::core_firms::apportion_amount(part, &vec![1_u64; weeks]);
                for (k, amount) in (1_u16..).zip(tranches).filter(|(_, a)| *a > 0) {
                    let bill = (debtor, holder, amount, corridor.deposit_rate);
                    self.open_bill(bill, (k, c, today), calendar);
                }
            }
        }
        Ok(())
    }

    /// A bill opened for `weeks` weeks from `day`: its holder's balance, reckoned at `rate`, on its issue's schedule.
    fn open_bill(
        &mut self,
        (debtor, holder, amount, rate): (PartyKey, PartyKey, i64, f64),
        (weeks, country, day): (u16, usize, Day),
        calendar: &Calendar,
    ) {
        let Some(family) = self.bound.families.bills else { return };
        let (Ok(ccy), Some(period)) = (u8::try_from(country), phx_core::calendar::period::Period::weeks(weeks)) else {
            violation!(clause = "SOV.1", "a bill's weeks that are no period", weeks = weeks);
        };
        let dates = phx_core::calendar::period::ScheduleDates {
            anchor: calendar.date(day),
            period,
            eom: phx_core::calendar::period::EndOfMonth::Plain,
            convention: phx_core::calendar::bizday::BusinessDayConvention::Following,
            country: CountryId::new(ccy),
        };
        let terms = phx_ledger::opening::plain_terms(
            phx_ledger::opening::currency(CountryId::new(ccy)),
            vec![
                Leg::RateOnNotional {
                    reference: Reference::Fixed(sys_bnk::rate(rate * crate::consts::PERCENT)),
                    day_count: DayCount::Act365F,
                },
                Leg::Amortising,
            ],
            Schedule { dates, count: Missing::Present(1) },
        );
        let maturity = dates.nth(calendar, 1);
        let Some(f) = self.families.get_mut(family) else { return };
        let schedule = f.schedule_in(ccy, [0, 0, 0], terms);
        let _ = f
            .store
            .open(Due { ends: [debtor, holder], amount, nth: 1, schedule, person: 0, arrears: 0 }, Some(maturity));
        f.moves.lent.push((holder, amount));
        let days = phx_rand::float::from_i64(i64::from(maturity.get()) - i64::from(day.get()));
        let face = i128::from(phx_ledger::opening::whole(
            phx_rand::float::from_i64(amount) * (1.0 + rate * days / DAYS_A_YEAR),
        ));
        self.bills.issued += i128::from(amount);
        match self.bills.issues.iter_mut().find(|i| i.schedule == schedule && i.day == day) {
            Some(i) => (i.issued, i.face) = (i.issued + i128::from(amount), i.face + face),
            None => self.bills.issues.push(Issue {
                country: ccy,
                day,
                maturity,
                schedule,
                issued: i128::from(amount),
                face,
            }),
        }
    }

    /// Each open country's auction on its weekday: its minister sizes it, its banks bid, and it clears; returns each
    /// bank's bills won as a step of the fund stage, its payment settled with the stage's.
    #[clause("SOV.3", "SOV.4", "SOV.6", "MND.20")]
    pub(crate) fn auctions(&mut self, open: &[usize], (day, calendar): (Day, &Calendar), before: &[Step]) -> Vec<Step> {
        let Some(kind) = self.bills.kind else { return Vec::new() };
        let date = calendar.date(day);
        let mut steps = Vec::new();
        let after = self.reserves_moved(before);
        let deposits = self.bank_deposits();
        let (sizing, bidding) = (self.point(|p| p.size, kind.size), self.point(|p| p.bid, kind.bid));
        for c in open {
            let (Some(Some(law)), Some(Some(treasury)), Some(corridor), Ok(ccy)) = (
                self.bills.laws.get(*c).copied(),
                self.treasuries.get(*c).copied(),
                self.central.corridors.get(*c).copied(),
                u8::try_from(*c),
            ) else {
                continue;
            };
            if weekday_index(date) != Some(law.weekday) {
                continue;
            }
            let Some(offered) = self.offer((*c, ccy, treasury), &law, sizing, (day, calendar)) else { continue };
            let face = phx_rand::float::from_i64(law.face);
            let days = f64::from(law.weeks) * DAYS_A_WEEK;
            let floor_price = price_at(corridor.deposit_rate, days);
            let mut bids: Vec<Bid> = Vec::new();
            let banks: Vec<PartyKey> = self
                .banks_of
                .get(*c)
                .map(|bs| bs.iter().filter_map(|(s, _)| Some(PartyKey::new(self.bank_kind?, Slot::new(*s)))).collect())
                .unwrap_or_default();
            for (at, key) in (0_u32..).zip(&banks) {
                let Some((reserves, target)) = self.reserves_and_target(*key, &after, &deposits) else { continue };
                let asked = self.decide(bidding, *key, |_| BidIn {
                    excess: phx_rand::float::from_i64(reserves - target),
                    floor_price,
                });
                for (price, amount) in asked {
                    let bills = phx_rand::float::floor_to_i64((amount / (price * face)).floor()).unwrap_or(0);
                    if bills > 0 {
                        bids.push(Bid { bidder: at, price, face: bills });
                    }
                }
            }
            let bid: i64 = bids.iter().map(|b| b.face).sum();
            let cleared: Option<Allotment> = if offered > 0 { (kind.clear)(offered, &bids) } else { None };
            let mut record = Auction {
                day: day.get(),
                country: ccy,
                offered,
                bid,
                sold: 0,
                price: Missing::Absent,
                cover: if offered > 0 {
                    Missing::Present(phx_rand::float::from_i64(bid) / phx_rand::float::from_i64(offered))
                } else {
                    Missing::Absent
                },
                tail: Missing::Absent,
            };
            if let Some(a) = cleared {
                record.price = Missing::Present(a.price);
                record.tail = tail(&a, &bids);
                for (b, n) in a.won {
                    let Some(bank) = banks.get(usize::try_from(b).unwrap_or(usize::MAX)).copied() else { continue };
                    let pay = phx_ledger::opening::whole(a.price * face * phx_rand::float::from_i64(n));
                    steps.push(Step::Buy { bank, treasury, bills: n, price: a.price, pay, country: *c });
                }
            }
            self.bills.auctions.push(record);
        }
        steps
    }

    /// The bills a treasury offers at its auction: its minister's size, from its cash, its outflow since its last
    /// auction and the week's maturities, in whole bills.
    fn offer(
        &mut self,
        (c, ccy, treasury): (usize, u8, PartyKey),
        law: &BillLaw,
        sizing: crate::core_decide::Bound<SizeIn, f64>,
        (day, calendar): (Day, &Calendar),
    ) -> Option<i64> {
        let cash = self
            .kinds
            .get(usize::from(treasury.kind()))
            .and_then(|k| k.accounts.as_ref())
            .and_then(|a| Some(a.balance.get(treasury.slot())? + a.pending.get(treasury.slot())?))?;
        let last = self.bills.last_cash.get_mut(c).and_then(|l| l.replace(cash));
        let Some(week) = phx_core::calendar::period::Period::weeks(1) else {
            violation!(clause = "TIME.4", "a week that is no period");
        };
        let next = calendar.plus(day, week);
        let maturing: i128 = self
            .bills
            .issues
            .iter()
            .filter(|i| i.country == ccy && i.maturity > day && i.maturity <= next)
            .map(|i| i.face)
            .sum();
        let size = self.decide(sizing, treasury, |_| SizeIn {
            cash: phx_rand::float::from_i64(cash),
            outflow: last.map_or(0.0, |l| phx_rand::float::from_i64(l - cash)),
            maturing: i64::try_from(maturing).map_or(f64::INFINITY, phx_rand::float::from_i64),
            buffer_weeks: law.buffer_weeks,
        });
        Some(phx_rand::float::floor_to_i64((size / phx_rand::float::from_i64(law.face)).ceil()).unwrap_or(0))
    }

    /// Each bill bought and paid for issued, at the yield its price gives over its weeks; the auction's record of what
    /// was sold.
    pub(crate) fn issue_bills(
        &mut self,
        steps: &[Step],
        unpaid: &std::collections::BTreeSet<u32>,
        (day, calendar): (Day, &Calendar),
    ) {
        for (at, step) in (0_u32..).zip(steps) {
            let Step::Buy { bank, treasury, bills, price, pay, country } = *step else { continue };
            if unpaid.contains(&at) {
                continue;
            }
            let Some(Some(law)) = self.bills.laws.get(country).copied() else { continue };
            let days = f64::from(law.weeks) * DAYS_A_WEEK;
            let rate = (1.0 / price - 1.0) * DAYS_A_YEAR / days;
            self.open_bill((treasury, bank, pay, rate), (law.weeks, country, day), calendar);
            // Its cash at this auction is what it holds once the auction's bills are paid for.
            if let Some(Some(cash)) = self.bills.last_cash.get_mut(country) {
                *cash += pay;
            }
            let ccy = u8::try_from(country).unwrap_or(u8::MAX);
            if let Some(a) = self.bills.auctions.iter_mut().rev().find(|a| a.day == day.get() && a.country == ccy) {
                a.sold += bills;
            }
        }
    }

    /// The day's redemptions and write-offs of bills read off their moves, before the loan books take them; then the
    /// debt outstanding held to what was issued less what was redeemed and written off, a difference a finding.
    #[clause("TRS.6", "N1", "II.5")]
    pub(crate) fn audit_debt(&mut self, day: Day) {
        let Some(f) = self.bound.families.bills.and_then(|i| self.families.get(i)) else { return };
        self.bills.redeemed += f.moves.repaid.iter().map(|(_, _, a)| i128::from(*a)).sum::<i128>();
        self.bills.written += f.moves.written_off.iter().map(|(_, a, _)| i128::from(*a)).sum::<i128>();
        let outstanding: i128 =
            f.store.edges.open_slots().filter_map(|e| f.store.edges.row(e)).map(|r| i128::from(r.amount)).sum();
        let expected = self.bills.issued - self.bills.redeemed - self.bills.written;
        if outstanding != expected {
            self.found.push(Finding {
                family: "debt",
                clause: "TRS.6",
                owner: FindingOwner::Run,
                size: outstanding - expected,
                unit: Unit::Count,
                day,
                detail: format!(
                    "the treasuries owe {outstanding} on bills where {} were issued, {} redeemed and {} written off",
                    self.bills.issued, self.bills.redeemed, self.bills.written
                ),
            });
        }
    }

    /// What each issue's holders hold, by its schedule.
    #[must_use]
    pub fn issue_held(&self) -> BTreeMap<u32, i128> {
        let mut held = BTreeMap::new();
        if let Some(f) = self.bound.families.bills.and_then(|i| self.families.get(i)) {
            for row in f.store.edges.open_slots().filter_map(|e| f.store.edges.row(e)) {
                *held.entry(row.schedule).or_insert(0) += i128::from(row.amount);
            }
        }
        held
    }
}
