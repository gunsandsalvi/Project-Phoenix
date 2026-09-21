//! BANKS — FUNDING AND LIQUIDITY: deposits are not one thing, the cost of funds is the bank's own,
//! and a model with one deposit type cannot have a run.
//!
//! @spec 24 A1 · 24 A1.a · 24 A1.b · 24 A1.c · 24 A1.d · 24 A2 · 24 A2.a · 24 A3 · 24 A4 · 24 A5 ·
//! @spec 24 B1 · 24 B1.a · 24 B1.b · 24 B2 · 24 B2.a · 24 B2.b · 24 B3 · 24 C1 · 24 C1.a · 24 C2 ·
//! @spec 24 C2.a · 24 C3 · 24 C3.a · 24 C4 · 24 D1 · 24 D2 · 24 D3 · 24 D4 · 24 D4.a · 24 D5 · 24 D6 ·
//! @spec 24 D6.a · 24 E1 · 24 E2 · 24 E2.a · 24 E3 · 24 E3.a · 24 E4 · 24 E4.a · 24 E5 · 24 F1 ·
//! @spec 24 F2 · 24 F3 · XI-15 · XI-2 · Law 4, Law 5, Law 6, Law 19

use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::ledger::{Cause, Delivery, Leg, Receipt, Units};
use crate::module::{Mechanism, MechanismContext};
use crate::stores::standing;
use std::collections::BTreeMap;

/// Deposits are not one thing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// Many, small, sticky, and insured up to a limit.
    Retail,
    /// Fewer, larger, operational — a firm banks where it transacts.
    Corporate,
    /// Few, very large, and RATE-SENSITIVE.
    Wholesale,
}

/// A deposit line by class, as a read of who actually banks there.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Line {
    pub depositor: PartyId,
    pub class: Class,
    pub balance: f64,
    /// How many real depositors this line is.
    pub members: f64,
    /// The rate the BANK sets on it, which depositors respond to.
    pub rate: f64,
}

impl Line {
    /// The insured amount is `weight × min(member balance, limit)`, which is EXACT because the cell
    /// is homogeneous — and that is what makes the break in the run loop real rather than notional.
    pub fn insured(&self, limit_per_member: f64) -> f64 {
        if self.class != Class::Retail || self.members <= 0.0 {
            return 0.0;
        }
        let per_member = self.balance / self.members;
        let covered = if per_member < limit_per_member {
            per_member
        } else {
            limit_per_member
        };
        self.members * covered
    }

    /// How much of this line leaves when the depositors see something.
    pub fn leaves(&self, on_signals: f64, limit_per_member: f64, runs: Eagerness) -> f64 {
        let exposed = self.balance - self.insured(limit_per_member);
        let going = exposed * runs.of(self.class) * on_signals;
        // They cannot take more than they have.
        if going < self.balance {
            going
        } else {
            self.balance
        }
    }
}

/// HOW FAST EACH KIND OF DEPOSITOR RUNS. Wholesale money is watching and goes first, corporate
/// money is slower, and insured retail is slowest of all — three PREFERENCES of the depositor, and
/// whoever wires this supplies them rather than the branch that reads them carrying them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Eagerness {
    pub wholesale: f64,
    pub corporate: f64,
    pub retail: f64,
}

impl Eagerness {
    fn of(self, class: Class) -> f64 {
        match class {
            Class::Wholesale => self.wholesale,
            Class::Corporate => self.corporate,
            Class::Retail => self.retail,
        }
    }
}

/// Each source has a price, the prices differ, and the mix is a DECISION.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Funding {
    Deposits(Class),
    /// Short, and it ROLLS — which is where a funding squeeze bites.
    Wholesale,
    /// Equity and subordinated debt, which do not run.
    Capital,
    /// The central bank, on the corridor's terms.
    CentralBank,
}

/// The bank pays a rate on each source, and it is a real payment to a real holder.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Source {
    pub kind: Funding,
    pub amount: f64,
    pub rate: f64,
}

/// The blended cost of funds is a READ across the mix — the bank's own, which is what lets a funding
/// condition reach a borrower at all.
pub fn blended(mix: &[Source]) -> Option<f64> {
    let size: f64 = mix.iter().map(|s| s.amount).sum();
    if size <= 0.0 {
        return None;
    }
    Some(mix.iter().map(|s| s.rate * s.amount).sum::<f64>() / size)
}

/// A deposit rate the bank SETS — bounded above by the cheaper of its own wholesale cost and the
/// money fund's yield, on the contested share of its base.
pub fn will_pay_on_deposits(own_wholesale_cost: f64, money_fund_yield: f64) -> f64 {
    if own_wholesale_cost < money_fund_yield {
        own_wholesale_cost
    } else {
        money_fund_yield
    }
}

/// Net interest margin is what it earns minus its cost of funds, and it can be NEGATIVE.
pub fn net_interest_margin(earned: f64, mix: &[Source]) -> Option<f64> {
    Some(earned - blended(mix)?)
}

/// Liquid assets — reserves, and securities it can sell or pledge — differ in how fast and how
/// surely they convert: a haircut and a market depth.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Liquid {
    pub value: f64,
    /// What it fetches when sold in a hurry, as a fraction of its mark.
    pub converts_at: f64,
    /// And how much of it the market can take this week.
    pub depth: f64,
}

impl Liquid {
    /// What this asset would actually raise, now.
    pub fn raises(&self) -> f64 {
        let sellable = if self.value < self.depth {
            self.value
        } else {
            self.depth
        };
        sellable * self.converts_at
    }
}

/// A buffer preference derived from its OWN liabilities, not a stated ratio — a bank funded by
/// wholesale money needs more than one funded by insured retail, and that is the whole of A1.d
/// showing up as a number.
pub fn buffer_wanted(
    lines: &[Line],
    limit_per_member: f64,
    on_signals: f64,
    runs: Eagerness,
) -> f64 {
    lines
        .iter()
        .map(|l| l.leaves(on_signals, limit_per_member, runs))
        .sum()
}

/// Maturity transformation is the business — it funds long assets with short liabilities, and that
/// gap is why it earns anything.
pub fn transformation(asset_years: f64, liability_years: f64) -> f64 {
    asset_years - liability_years
}

/// What a bank that is short actually does, in order, each a real act with a counterparty.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Short {
    BorrowsInTheMarket {
        amount: f64,
    },
    /// Sells or pledges liquid assets — a real order in a real book.
    SellsLiquid {
        raising: f64,
    },
    /// Bids up for deposits, and pays for them.
    BidsForDeposits {
        paying: f64,
    },
    /// It stops lending and lets the book run off — this is the credit crunch, a funding problem
    /// transmitted into the credit decision.
    StopsLending {
        by: f64,
    },
    /// The facility, collateralised and at a penalty.
    DrawsTheWindow {
        amount: f64,
    },
    CannotFund {
        short_by: f64,
    },
}

pub fn when_short(
    short_by: f64,
    market_will_lend: f64,
    liquid: &[Liquid],
    deposits_biddable: f64,
    window: f64,
    book_that_can_run_off: f64,
) -> Short {
    if market_will_lend >= short_by {
        return Short::BorrowsInTheMarket { amount: short_by };
    }
    let from_sales: f64 = liquid.iter().map(|l| l.raises()).sum();
    if market_will_lend + from_sales >= short_by {
        return Short::SellsLiquid {
            raising: short_by - market_will_lend,
        };
    }
    let so_far = market_will_lend + from_sales;
    if so_far + deposits_biddable >= short_by {
        return Short::BidsForDeposits {
            paying: short_by - so_far,
        };
    }
    let so_far = so_far + deposits_biddable;
    if so_far + window >= short_by {
        return Short::DrawsTheWindow {
            amount: short_by - so_far,
        };
    }
    let so_far = so_far + window;
    if so_far + book_that_can_run_off >= short_by {
        return Short::StopsLending {
            by: short_by - so_far,
        };
    }
    Short::CannotFund {
        short_by: short_by - so_far - book_that_can_run_off,
    }
}

/// They leave because they observe something, and what they observe must be OBSERVABLE — a capital
/// ratio, a facility draw, a rate paid up, a rating action, a run of weeks ending short.
#[derive(Clone, Copy, Debug)]
pub struct Observed {
    pub capital_ratio_published: f64,
    pub drew_the_window: bool,
    pub paid_up_for_deposits: bool,
    pub downgraded: bool,
    pub periods_ending_short: u32,
}

impl Observed {
    /// How many things a depositor can actually see.
    pub fn signals(&self, capital_that_worries: f64) -> f64 {
        let mut n = self.periods_ending_short as f64;
        if self.capital_ratio_published < capital_that_worries {
            n += 1.0;
        }
        if self.drew_the_window {
            n += 1.0;
        }
        if self.paid_up_for_deposits {
            n += 1.0;
        }
        if self.downgraded {
            n += 1.0;
        }
        n
    }
}

/// The deposit leaves WITH THE RESERVES BEHIND IT, so the bank is shorter at the next close — and
/// that is the loop.
pub fn after_outflow(reserves: f64, left: f64) -> f64 {
    reserves - left
}

/// Turn a residual funding shortfall into bounded sales of named, priced holdings. What remains is
/// the liquidity failure; an absent price or unavailable unit contributes nothing.
pub fn liquidates(
    mut short: f64,
    holdings: &[(InstrumentId, f64, Option<f64>)],
) -> (Vec<(InstrumentId, f64)>, f64) {
    let mut sales = Vec::new();
    for &(line, free, price) in holdings {
        let Some(price) = price else { continue };
        if price <= 0.0 || free <= 0.0 || short <= 0.0 {
            continue;
        }
        let wanted = short / price;
        let units = if free < wanted { free } else { wanted };
        sales.push((line, units));
        short -= units * price;
    }
    (sales, short)
}

/// Collateral left after market sales supports a named central-bank advance, at a haircut. The
/// returned units are the units actually encumbered, rather than a notional collateral value.
pub fn pledges(
    mut short: f64,
    advance_rate: f64,
    holdings: &[(InstrumentId, f64, Option<f64>)],
) -> (Vec<(InstrumentId, f64)>, f64, f64) {
    assert!(
        advance_rate > 0.0 && advance_rate < 1.0,
        "a collateral advance rate is between zero and one"
    );
    let mut pledged = Vec::new();
    let mut advanced = 0.0;
    for &(line, free, price) in holdings {
        let Some(price) = price else { continue };
        if price <= 0.0 || free <= 0.0 || short <= 0.0 {
            continue;
        }
        let lends_per_unit = price * advance_rate;
        let wanted = short / lends_per_unit;
        let units = if free < wanted { free } else { wanted };
        let lends = units * lends_per_unit;
        pledged.push((line, units));
        advanced += lends;
        short -= lends;
    }
    (pledged, advanced, short)
}

/// Assets equal liabilities plus equity, in the bank's own money, every week.
pub fn balances(assets: f64, liabilities: f64, equity: f64, terms: usize) -> Option<f64> {
    let off = assets - (liabilities + equity);
    if off.abs() <= crate::num::dust(terms, &[assets, liabilities, equity]) {
        return None;
    }
    Some(off)
}

pub fn external_liability(total_held: f64, held_by_issuer: f64) -> f64 {
    assert!(
        total_held >= held_by_issuer,
        "an issuer cannot hold more of a line than exists"
    );
    total_held - held_by_issuer
}

pub fn reconciled_funding(
    class: crate::instruments::Class,
    total_held: f64,
    held_by_issuer: f64,
) -> Option<(Funding, f64)> {
    let amount = external_liability(total_held, held_by_issuer);
    match class {
        crate::instruments::Class::Money => Some((Funding::Deposits(Class::Retail), amount)),
        crate::instruments::Class::Claim => Some((Funding::Wholesale, amount)),
        _ => None,
    }
}

pub fn reconciled_reserves(
    bank: PartyId,
    settlement_bank: PartyId,
    account_issuer: PartyId,
    balance: f64,
) -> Option<f64> {
    if settlement_bank == bank || settlement_bank != account_issuer || balance < 0.0 {
        return None;
    }
    Some(balance)
}

/// The amount a bank has irrevocably promised but has not yet advanced. Committed facilities are
/// agreements, not loans inferred from spare cash: the first party is the lender and `drawn` is
/// the portion already recognised among the bank's funded assets.
pub fn reconciled_undrawn_commitments(
    bank: PartyId,
    agreements: &crate::stores::Agreements,
) -> f64 {
    agreements
        .of_party(bank)
        .iter()
        .map(|row| crate::stores::AgreementId(*row))
        .filter(|agreement| {
            agreements.live(*agreement)
                && agreements.kind_of(*agreement) == crate::stores::agreed::COMMITTED_CREDIT
                && agreements.between(*agreement).0 == bank
        })
        .map(|agreement| match agreements.terms(agreement) {
            crate::stores::AgreementTerms::CommittedCredit { limit, drawn, .. } => limit - drawn,
            _ => unreachable!("committed-credit kind has committed-credit terms"),
        })
        .sum()
}

/// Cash consideration actually received for funding-driven asset sales. A requested or cleared
/// sale contributes nothing here; `Processes::realises` is called only after DvP settlement.
pub fn reconciled_asset_sale_proceeds(bank: PartyId, processes: &crate::stores::Processes) -> f64 {
    processes
        .of_owner(bank)
        .iter()
        .map(|row| crate::stores::ProcessId(*row))
        .filter(|process| {
            processes.kind_of(*process) == crate::stores::afoot::WORKOUT
                && processes.door(*process)
                    == Some(crate::stores::WorkoutDoor::FundingWithdrawn as u32)
        })
        .map(|process| processes.proceeds(process))
        .sum()
}

/// Principal currently owed by a bank to named central banks. The liability is read from live
/// facility contracts rather than inferred from its reserve balance, which may have other causes.
pub fn reconciled_central_bank_borrowing(
    bank: PartyId,
    agreements: &crate::stores::Agreements,
) -> f64 {
    agreements
        .of_party(bank)
        .iter()
        .map(|row| crate::stores::AgreementId(*row))
        .filter(|agreement| {
            agreements.live(*agreement)
                && agreements.kind_of(*agreement) == crate::stores::agreed::CENTRAL_BANK_FACILITY
                && agreements.between(*agreement).1 == bank
        })
        .map(|agreement| match agreements.terms(agreement) {
            crate::stores::AgreementTerms::CentralBankFacility { principal, .. } => *principal,
            _ => unreachable!("central-bank-facility kind has facility terms"),
        })
        .sum()
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LiquidityFlow {
    pub due: crate::calendar::Week,
    pub incoming: f64,
    pub outgoing: f64,
}

/// The bank's dated contractual cash flows in one settlement currency. Bilateral receivables name
/// the bank; instrument receivables are weighted by its settled holding of the paying line.
pub fn liquidity_ladder(
    bank: PartyId,
    settlement: crate::ids::CurrencyCode,
    today: crate::calendar::Week,
    schedules: &crate::stores::Schedules,
    register: &crate::register::Register,
) -> Vec<LiquidityFlow> {
    let mut by_day: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    for row in 0..schedules.len() as u32 {
        let due = crate::stores::DueId(row);
        if schedules.paid(due) || schedules.ccy(due) != settlement || schedules.due(due) < today {
            continue;
        }
        let remaining = schedules.amount(due) - schedules.recovered(due);
        let entry = by_day.entry(schedules.due(due).0).or_insert((0.0, 0.0));
        if schedules.owed_by(due) == bank {
            entry.1 += remaining;
        }
        if schedules.owed_by(due) != bank {
            match schedules.on(due) {
                crate::stores::Owed::To(payee) if payee == bank => entry.0 += remaining,
                crate::stores::Owed::On(line) => {
                    let (issued, _) = register.held_total(line);
                    if issued > 0.0 {
                        let held = register.quantity(register.row(bank, line));
                        entry.0 += remaining * held / issued;
                    }
                }
                _ => {}
            }
        }
    }
    by_day
        .into_iter()
        .map(|(day, (incoming, outgoing))| LiquidityFlow {
            due: crate::calendar::Week(day),
            incoming,
            outgoing,
        })
        .collect()
}

/// Reserves the bank must have by the weekly_funding horizon after its own dated inflows offset its
/// own dated outflows. Later buckets do not become an invented present liquidity need.
pub fn weekly_funding_reservation(ladder: &[LiquidityFlow], through: crate::calendar::Week) -> f64 {
    let net: f64 = ladder
        .iter()
        .filter(|flow| flow.due <= through)
        .map(|flow| flow.outgoing - flow.incoming)
        .sum();
    if net > 0.0 {
        net
    } else {
        0.0
    }
}

/// A standing-facility penalty is a spread over the transacted weekly_funding print. The central
/// bank's policy setting is a different observation and never substitutes for a dark market.
pub fn facility_rate(weekly_funding_market_print: f64, penalty: f64) -> f64 {
    weekly_funding_market_print + penalty
}

/// The standing facility is a bank-liquidity door, not a general account overdraft.  Requiring the
/// borrower's declared failure mode here makes both halves explicit: an insolvent bank proceeds to
/// resolution, while a treasury (and every other non-bank kind) cannot enter the facility at all.
pub fn facility_borrower(mode: crate::registry::FailureMode, booked_equity: Option<f64>) -> bool {
    mode == crate::registry::FailureMode::Bank && booked_equity.is_some_and(|equity| equity >= 0.0)
}

/// A BANK SETS THE RATE IT PAYS ON DEPOSITS.
pub struct BankFunding {
    pub kind: u32,
    /// A funding shortfall left after the money-market book and saleable collateral.
    pub failed: u32,
    pub at_short: u32,
    pub facility_advance: &'static str,
    pub facility_penalty: &'static str,
    pub facility_drawn: u32,
    pub at_rate: u32,
    /// The benchmark fixing, which is what a money fund would earn.
    pub weekly_funding_fixing: u32,
}

struct FacilityDraw {
    bank: PartyId,
    central_bank: PartyId,
    reserves: InstrumentId,
    amount: f64,
    rate: f64,
    pledged: Vec<(InstrumentId, f64)>,
}

impl Mechanism for BankFunding {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        // The last fixing.
        let mut money_fund_yield: Option<f64> = None;
        for &row in ctx.journal().of_kind(self.weekly_funding_fixing) {
            if let Some(Value::Num(rate)) = ctx.journal().says(row, 0) {
                money_fund_yield = Some(rate);
            }
        }
        let Some(money_fund_yield) = money_fund_yield else {
            return;
        };

        let mut set: Vec<(PartyId, f64, f64, f64, f64, f64, f64)> = Vec::new();
        let mut sales: Vec<(PartyId, InstrumentId, f64)> = Vec::new();
        let mut failed: Vec<(PartyId, f64)> = Vec::new();
        let mut facilities: Vec<FacilityDraw> = Vec::new();
        for &bank in ctx.parties().of_kind(kinds::BANK) {
            let who = PartyId(bank);
            if !ctx.parties().alive(who) {
                continue;
            }
            // Its OWN mix, read off what it has issued and what it pays on each.
            let mut mix: Vec<Source> = Vec::new();
            for &line in ctx.instruments().of_issuer(who) {
                let what = InstrumentId::at(line);
                let (held, _) = ctx.register().held_total(what);
                let class = ctx.instruments().class_of(what);
                let Some((funding, outstanding)) = reconciled_funding(
                    class,
                    held,
                    ctx.register().quantity(ctx.register().row(who, what)),
                ) else {
                    continue;
                };
                if outstanding <= 0.0 {
                    continue;
                }
                match class {
                    crate::instruments::Class::Money => mix.push(Source {
                        kind: funding,
                        amount: outstanding,
                        // What it is paying now is what it last stood behind, and nothing where it
                        // has never set one — a bank that has not set a rate is not paying zero.
                        rate: match ctx.standing().of_party_about(
                            who,
                            PartyId::NONE,
                            standing::DEPOSIT_RATE,
                        ) {
                            Some(s) => ctx.standing().terms(s)[0],
                            None => continue,
                        },
                    }),
                    // Short, and it ROLLS — which is where a funding squeeze bites.
                    crate::instruments::Class::Claim => mix.push(Source {
                        kind: funding,
                        amount: outstanding,
                        rate: match ctx.instruments().coupon_of(what) {
                            Some(c) => c,
                            None => continue,
                        },
                    }),
                    _ => {}
                }
            }
            // `None` where it funds with nothing — answering zero would say it funds free.
            let own_wholesale_cost = match blended(&mix) {
                Some(cost) => cost,
                // A bank that has never funded wholesale has its own cost to find, and the benchmark
                // is the only thing it can read.
                None => money_fund_yield,
            };
            let pledged_value = ctx
                .register()
                .of_holder(who)
                .iter()
                .map(|row| {
                    let holding = crate::ids::HoldingId(*row);
                    let line = ctx.register().instrument_of(holding);
                    let price = ctx
                        .prints()
                        .of_line(line, ctx.week())
                        .map(|print| print.price);
                    match price {
                        Some(price) => ctx.register().pledged(holding) * price,
                        None => 0.0,
                    }
                })
                .sum();
            let undrawn = reconciled_undrawn_commitments(who, ctx.agreements());
            let sale_proceeds = reconciled_asset_sale_proceeds(who, ctx.processes());
            let central_bank_borrowing = reconciled_central_bank_borrowing(who, ctx.agreements());
            let Some(account) = crate::ledger::account_of(ctx.parties(), ctx.instruments(), who)
            else {
                continue;
            };
            let ladder = liquidity_ladder(
                who,
                ctx.instruments().ccy_of(account),
                ctx.today(),
                ctx.schedules(),
                ctx.register(),
            );
            let reservation =
                weekly_funding_reservation(&ladder, crate::calendar::Week(ctx.today().0 + 1));
            set.push((
                who,
                will_pay_on_deposits(own_wholesale_cost, money_fund_yield),
                pledged_value,
                undrawn,
                sale_proceeds,
                central_bank_borrowing,
                reservation,
            ));

            // The weekly_funding book has already cleared at the books stage. What remains short now
            // must be met by selling named liquid holdings, or become an explicit liquidity
            // failure; it is never silently treated as a capital failure.
            let Some(cash) = reconciled_reserves(
                who,
                ctx.parties().bank_of(who),
                ctx.instruments().issuer_of(account),
                ctx.register().quantity(ctx.register().row(who, account)),
            ) else {
                continue;
            };
            let mut short = reservation - cash;
            if short <= 0.0 {
                continue;
            }
            let mut saleable = Vec::new();
            let mut collateral = Vec::new();
            for row in ctx.register().of_holder(who) {
                let holding = crate::ids::HoldingId(*row);
                let line = ctx.register().instrument_of(holding);
                if ctx.instruments().class_of(line) == crate::instruments::Class::Money {
                    continue;
                }
                let free = ctx.register().free(holding);
                if ctx
                    .processes()
                    .running(crate::stores::afoot::WORKOUT)
                    .iter()
                    .any(|process| {
                        ctx.processes().owner(*process) == who
                            && ctx.processes().subject(*process) == Some(line)
                    })
                {
                    continue;
                }
                let holding = (
                    line,
                    free,
                    ctx.prints()
                        .of_line(line, ctx.week())
                        .map(|print| print.price),
                );
                // The facility accepts claims; other priced assets must be sold through their
                // books. Keeping the sets disjoint prevents a unit being promised to a future sale
                // after it has already been pledged at the window.
                if ctx.instruments().class_of(line) == crate::instruments::Class::Claim {
                    collateral.push(holding);
                } else {
                    saleable.push(holding);
                }
            }
            let (planned, _) = liquidates(short, &saleable);
            for (line, units) in planned {
                sales.push((who, line, units));
            }
            // A sale order is not cash. The reserve balance above already contains proceeds from
            // prior settled DvP fills; planned sales do not reduce today's residual shortfall.
            // A solvent bank may draw reserves from the named issuer of its reserve account. The
            // collateral is pledged in the same atomic instruction as the reserve creation and
            // transfer; no anonymous residual buyer and no uncollateralised overdraft exists.
            let booked_equity = crate::instruments::booked_equity(
                who,
                ctx.register(),
                ctx.instruments(),
                ctx.prints(),
                ctx.claims(),
                ctx.week(),
            );
            let borrower_mode = ctx
                .registry()
                .profile(ctx.parties().kind_of(who))
                .expect("Law 15: a borrower kind needs a declared failure capability")
                .failure;
            if short > 0.0 && facility_borrower(borrower_mode, booked_equity) {
                let central_bank = ctx.instruments().issuer_of(account);
                let issuer_profile = ctx.registry().profile(ctx.parties().kind_of(central_bank));
                if issuer_profile.is_some_and(|profile| {
                    profile.issues_money && profile.banks == crate::registry::Banks::Nowhere
                }) {
                    let advance = ctx.params().ratio(self.facility_advance);
                    let (pledged, amount, left) = pledges(short, advance, &collateral);
                    if amount > 0.0 {
                        let rate = facility_rate(
                            money_fund_yield,
                            ctx.params().per_annum(self.facility_penalty),
                        );
                        facilities.push(FacilityDraw {
                            bank: who,
                            central_bank,
                            reserves: account,
                            amount,
                            rate,
                            pledged,
                        });
                    }
                    short = left;
                }
            }
            if short > 0.0 {
                failed.push((who, short));
            }
        }

        for (
            who,
            rate,
            pledged_value,
            undrawn,
            sale_proceeds,
            central_bank_borrowing,
            reservation,
        ) in set
        {
            // A POSTED rate — depositors respond to it, so it is one-sided terms the bank stands
            // behind until it changes them, and what it was paying stays readable beside it.
            ctx.now_stands(standing::DEPOSIT_RATE, who, PartyId::NONE, vec![rate]);
            ctx.say(
                self.kind,
                &[who.0],
                &[
                    (0, Value::Num(rate)),
                    (1, Value::Num(pledged_value)),
                    (2, Value::Num(undrawn)),
                    (3, Value::Num(sale_proceeds)),
                    (4, Value::Num(central_bank_borrowing)),
                    (5, Value::Num(reservation)),
                ],
                true,
            );
        }
        for (who, line, units) in sales {
            ctx.opens(crate::module::Opens {
                kind: crate::stores::afoot::WORKOUT,
                owner: who,
                subject: Some(line),
                door: Some(crate::stores::WorkoutDoor::FundingWithdrawn as u32),
                closes: Some(ctx.week() + 1),
                size: units,
            });
        }
        for draw in facilities {
            let mut legs = Vec::with_capacity(draw.pledged.len() + 2);
            for &(line, qty) in &draw.pledged {
                legs.push(Leg::Pledge {
                    holder: draw.bank,
                    instrument: line,
                    to: draw.central_bank,
                    qty: Units::new(qty).expect("a facility pledges positive units"),
                });
            }
            let amount = Units::new(draw.amount).expect("a facility advances a positive amount");
            legs.push(Leg::Mint {
                issuer: draw.central_bank,
                money: draw.reserves,
                amount,
            });
            legs.push(Leg::Money {
                from: draw.central_bank,
                to: draw.bank,
                instrument: draw.reserves,
                amount,
                receipt: Receipt::Principal,
            });
            ctx.propose(
                legs,
                Cause::Settlement,
                Delivery::Nothing,
                "a collateralised central-bank facility draw",
            );

            let today = ctx.today();
            let due = ctx
                .calendar()
                .at(crate::calendar::Week(i64::from(ctx.week() + 1)));
            let ccy = ctx.instruments().ccy_of(draw.reserves);
            let mut payments = vec![crate::stores::Payment {
                from: today,
                due,
                amount: amount.get(),
                of: crate::stores::Owing::Principal,
            }];
            let interest = amount.get()
                * draw.rate
                * crate::calendar::Convention::Actual365.year_fraction(today, due);
            if interest > 0.0 {
                payments.push(crate::stores::Payment {
                    from: today,
                    due,
                    amount: interest,
                    of: crate::stores::Owing::Interest,
                });
            }
            ctx.contracts(crate::module::ContractObligation {
                agreement: crate::module::Agrees {
                    kind: crate::stores::agreed::CENTRAL_BANK_FACILITY,
                    one: draw.central_bank,
                    other: draw.bank,
                    terms: crate::stores::AgreementTerms::CentralBankFacility {
                        principal: amount.get(),
                        rate: draw.rate,
                        settlement: ccy,
                        collateral: draw.pledged,
                    },
                    until: Some(due),
                },
                owed_by: draw.bank,
                ccy,
                payments,
            });
            ctx.say(
                self.facility_drawn,
                &[draw.bank.0, draw.central_bank.0],
                &[
                    (self.at_short, Value::Num(amount.get())),
                    (self.at_rate, Value::Num(draw.rate)),
                ],
                true,
            );
        }
        for (who, short) in failed {
            ctx.say(
                self.failed,
                &[who.0],
                &[(self.at_short, Value::Num(short))],
                true,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tests' own depositors, and this is the one place these three are written.
    fn runs() -> Eagerness {
        Eagerness {
            wholesale: 1.0,
            corporate: 0.4,
            retail: 0.15,
        }
    }

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn line(depositor: u32, class: Class, balance: f64, members: f64) -> Line {
        Line {
            depositor: party(depositor),
            class,
            balance,
            members,
            rate: 0.01,
        }
    }

    fn book() -> Vec<Line> {
        vec![
            line(50, Class::Retail, 100_000.0, 10_000.0),
            line(51, Class::Corporate, 60_000.0, 40.0),
            line(52, Class::Wholesale, 80_000.0, 4.0),
        ]
    }

    #[test]
    fn a_model_with_one_deposit_type_cannot_have_a_run() {
        // Stickiness differs by class and it is the whole of liquidity risk.
        let signals = 1.0;
        let retail = line(50, Class::Retail, 80_000.0, 10_000.0).leaves(signals, 50.0, runs());
        let corporate = line(51, Class::Corporate, 80_000.0, 40.0).leaves(signals, 50.0, runs());
        let wholesale = line(52, Class::Wholesale, 80_000.0, 4.0).leaves(signals, 50.0, runs());
        assert!(wholesale > corporate);
        assert!(corporate > retail);
    }

    #[test]
    fn the_insured_amount_is_exact_because_the_cell_is_homogeneous() {
        // Weight × min(member balance, limit).
        let small = line(50, Class::Retail, 100_000.0, 10_000.0);
        assert_eq!(small.insured(50.0), 100_000.0);
        let large = line(50, Class::Retail, 100_000.0, 100.0);
        assert_eq!(large.insured(50.0), 5_000.0);
        // And insurance does not reach wholesale at all.
        assert_eq!(
            line(52, Class::Wholesale, 100_000.0, 4.0).insured(50.0),
            0.0
        );
    }

    #[test]
    fn a_run_is_a_wholesale_phenomenon_first() {
        // Deposit insurance breaks the loop for retail and not for wholesale.
        let leaving: Vec<f64> = book().iter().map(|l| l.leaves(1.0, 50.0, runs())).collect();
        // The fully insured retail line barely moves; the wholesale line goes entirely.
        assert_eq!(leaving[0], 0.0);
        assert_eq!(leaving[2], 80_000.0);
    }

    #[test]
    fn deposits_reconcile_to_money_held_outside_the_issuing_bank() {
        assert_eq!(external_liability(1_250.0, 50.0), 1_200.0);
        assert_eq!(external_liability(1_250.0, 1_250.0), 0.0);
    }

    #[test]
    fn wholesale_funding_reconciles_to_claims_held_outside_the_issuing_bank() {
        assert_eq!(
            reconciled_funding(crate::instruments::Class::Claim, 900.0, 150.0),
            Some((Funding::Wholesale, 750.0))
        );
        assert!(reconciled_funding(crate::instruments::Class::Share, 900.0, 0.0).is_none());
    }

    #[test]
    fn undrawn_commitments_are_live_named_lines_where_the_bank_is_lender() {
        let bank = party(7);
        let borrower = party(8);
        let mut agreements = crate::stores::Agreements::new();
        let live = agreements.strike(
            crate::stores::agreed::COMMITTED_CREDIT,
            bank,
            borrower,
            crate::stores::AgreementTerms::CommittedCredit {
                limit: 1_000.0,
                drawn: 350.0,
                margin: 0.02,
                fee_on_undrawn: 0.005,
            },
            crate::calendar::Week(0),
            None,
        );
        agreements.strike(
            crate::stores::agreed::COMMITTED_CREDIT,
            borrower,
            bank,
            crate::stores::AgreementTerms::CommittedCredit {
                limit: 400.0,
                drawn: 100.0,
                margin: 0.03,
                fee_on_undrawn: 0.006,
            },
            crate::calendar::Week(0),
            None,
        );
        assert_eq!(reconciled_undrawn_commitments(bank, &agreements), 650.0);
        agreements.end(live, crate::calendar::Week(1));
        assert_eq!(reconciled_undrawn_commitments(bank, &agreements), 0.0);
    }

    #[test]
    fn asset_sale_reconciliation_counts_only_settled_funding_workouts() {
        let bank = party(7);
        let mut processes = crate::stores::Processes::new();
        let sale = processes.begin_for(
            crate::stores::afoot::WORKOUT,
            bank,
            0,
            Some(1),
            10.0,
            crate::stores::ProcessTarget {
                door: Some(crate::stores::WorkoutDoor::FundingWithdrawn as u32),
                subject: Some(InstrumentId::at(4)),
            },
        );
        assert_eq!(reconciled_asset_sale_proceeds(bank, &processes), 0.0);
        processes.realises(sale, 4.0, 120.0);
        assert_eq!(reconciled_asset_sale_proceeds(bank, &processes), 120.0);
    }

    #[test]
    fn central_bank_borrowing_is_the_principal_of_live_facilities_owed_by_the_bank() {
        let central_bank = party(6);
        let bank = party(7);
        let mut agreements = crate::stores::Agreements::new();
        let facility = agreements.strike(
            crate::stores::agreed::CENTRAL_BANK_FACILITY,
            central_bank,
            bank,
            crate::stores::AgreementTerms::CentralBankFacility {
                principal: 240.0,
                rate: 0.04,
                settlement: crate::ids::CurrencyCode::at(1),
                collateral: vec![(InstrumentId::at(4), 3.0)],
            },
            crate::calendar::Week(0),
            Some(crate::calendar::Week(30)),
        );
        assert_eq!(reconciled_central_bank_borrowing(bank, &agreements), 240.0);
        agreements.end(facility, crate::calendar::Week(30));
        assert_eq!(reconciled_central_bank_borrowing(bank, &agreements), 0.0);
    }

    #[test]
    fn liquidity_ladder_nets_only_the_banks_own_dated_currency_flows() {
        let bank = party(7);
        let counterparty = party(8);
        let ccy = crate::ids::CurrencyCode::at(1);
        let mut schedules = crate::stores::Schedules::new();
        schedules.owes(
            crate::stores::Owed::To(counterparty),
            bank,
            ccy,
            crate::stores::Payment {
                from: crate::calendar::Week(0),
                due: crate::calendar::Week(2),
                amount: 90.0,
                of: crate::stores::Owing::Principal,
            },
        );
        schedules.owes(
            crate::stores::Owed::To(bank),
            counterparty,
            ccy,
            crate::stores::Payment {
                from: crate::calendar::Week(0),
                due: crate::calendar::Week(2),
                amount: 40.0,
                of: crate::stores::Owing::Principal,
            },
        );
        schedules.owes(
            crate::stores::Owed::To(counterparty),
            bank,
            crate::ids::CurrencyCode::at(2),
            crate::stores::Payment {
                from: crate::calendar::Week(0),
                due: crate::calendar::Week(2),
                amount: 500.0,
                of: crate::stores::Owing::Principal,
            },
        );
        let register = crate::register::Register::default();
        assert_eq!(
            liquidity_ladder(bank, ccy, crate::calendar::Week(1), &schedules, &register),
            vec![LiquidityFlow {
                due: crate::calendar::Week(2),
                incoming: 40.0,
                outgoing: 90.0
            },]
        );
    }

    #[test]
    fn weekly_funding_reservation_uses_only_net_flows_inside_the_horizon() {
        let ladder = [
            LiquidityFlow {
                due: crate::calendar::Week(1),
                incoming: 20.0,
                outgoing: 90.0,
            },
            LiquidityFlow {
                due: crate::calendar::Week(3),
                incoming: 0.0,
                outgoing: 500.0,
            },
        ];
        assert_eq!(
            weekly_funding_reservation(&ladder, crate::calendar::Week(1)),
            70.0
        );
        let covered = [LiquidityFlow {
            due: crate::calendar::Week(1),
            incoming: 100.0,
            outgoing: 90.0,
        }];
        assert_eq!(
            weekly_funding_reservation(&covered, crate::calendar::Week(1)),
            0.0
        );
    }

    #[test]
    fn facility_penalty_is_applied_to_the_weekly_funding_print() {
        let policy_rate = 0.01;
        let weekly_funding_print = 0.035;
        assert!((facility_rate(weekly_funding_print, 0.02) - 0.055).abs() < f64::EPSILON);
        assert_ne!(
            facility_rate(weekly_funding_print, 0.02),
            policy_rate + 0.02
        );
    }

    #[test]
    fn only_a_solvent_bank_can_borrow_at_the_standing_facility() {
        use crate::registry::FailureMode;

        assert!(facility_borrower(FailureMode::Bank, Some(1.0)));
        assert!(facility_borrower(FailureMode::Bank, Some(0.0)));
        assert!(!facility_borrower(FailureMode::Bank, Some(-1.0)));
        assert!(!facility_borrower(FailureMode::Bank, None));
        assert!(!facility_borrower(FailureMode::Sovereign, Some(1_000.0)));
    }

    #[test]
    fn reserves_reconcile_to_the_banks_account_at_its_settlement_bank() {
        let bank = PartyId::at(3);
        let central_bank = PartyId::at(9);
        assert_eq!(
            reconciled_reserves(bank, central_bank, central_bank, 240.0),
            Some(240.0)
        );
        assert!(reconciled_reserves(bank, bank, bank, 240.0).is_none());
        assert!(reconciled_reserves(bank, central_bank, PartyId::at(8), 240.0).is_none());
    }

    #[test]
    fn the_buffer_is_derived_from_its_own_liabilities_and_not_from_a_ratio() {
        // A bank funded by wholesale money needs more than one funded by insured retail.
        let wholesale_funded = [line(52, Class::Wholesale, 200_000.0, 4.0)];
        let retail_funded = [line(50, Class::Retail, 200_000.0, 20_000.0)];
        assert!(
            buffer_wanted(&wholesale_funded, 50.0, 1.0, runs())
                > buffer_wanted(&retail_funded, 50.0, 1.0, runs())
        );
    }

    #[test]
    fn the_blended_cost_is_the_banks_own_and_a_bank_funding_with_nothing_has_none() {
        // A bank with no cost of funds prices every loan as if it funded at the policy rate whatever
        // its own position, and then no funding condition can reach a borrower.
        let cheap = [
            Source {
                kind: Funding::Deposits(Class::Retail),
                amount: 8_000.0,
                rate: 0.005,
            },
            Source {
                kind: Funding::Wholesale,
                amount: 2_000.0,
                rate: 0.04,
            },
        ];
        let dear = [
            Source {
                kind: Funding::Deposits(Class::Retail),
                amount: 2_000.0,
                rate: 0.005,
            },
            Source {
                kind: Funding::Wholesale,
                amount: 8_000.0,
                rate: 0.04,
            },
        ];
        assert!(blended(&dear).unwrap() > blended(&cheap).unwrap());
        assert!(blended(&[]).is_none());
        // And the margin can be negative.
        assert!(net_interest_margin(0.02, &dear).unwrap() < 0.0);
    }

    #[test]
    fn the_deposit_rate_is_set_against_the_cheaper_of_wholesale_and_the_money_fund() {
        // Past that point the bank would rather fund wholesale — a decision, not a rule.
        assert_eq!(will_pay_on_deposits(0.045, 0.030), 0.030);
        assert_eq!(will_pay_on_deposits(0.020, 0.030), 0.020);
    }

    #[test]
    fn a_liquid_asset_raises_what_the_market_can_take_at_what_it_converts_at() {
        // They differ in how fast and how surely they convert.
        let deep = Liquid {
            value: 10_000.0,
            converts_at: 0.99,
            depth: 50_000.0,
        };
        let thin = Liquid {
            value: 10_000.0,
            converts_at: 0.80,
            depth: 2_000.0,
        };
        assert!(deep.raises() > thin.raises());
        assert_eq!(thin.raises(), 1_600.0);
    }

    #[test]
    fn a_short_bank_works_through_its_options_and_can_still_fail_to_fund() {
        // Failure to fund is REACHABLE, which is what makes the buffer worth holding.
        let liquid = [Liquid {
            value: 5_000.0,
            converts_at: 0.9,
            depth: 5_000.0,
        }];
        assert_eq!(
            when_short(1_000.0, 4_000.0, &liquid, 0.0, 0.0, 0.0),
            Short::BorrowsInTheMarket { amount: 1_000.0 }
        );
        assert_eq!(
            when_short(5_000.0, 1_000.0, &liquid, 0.0, 0.0, 0.0),
            Short::SellsLiquid { raising: 4_000.0 }
        );
        assert!(matches!(
            when_short(7_000.0, 1_000.0, &liquid, 2_000.0, 0.0, 0.0),
            Short::BidsForDeposits { .. }
        ));
        assert!(matches!(
            when_short(9_000.0, 1_000.0, &liquid, 2_000.0, 3_000.0, 0.0),
            Short::DrawsTheWindow { .. }
        ));
        // The credit crunch — it stops lending and lets the book run off.
        assert!(matches!(
            when_short(12_000.0, 1_000.0, &liquid, 2_000.0, 3_000.0, 5_000.0),
            Short::StopsLending { .. }
        ));
        // And past all of that it cannot fund itself.
        assert!(matches!(
            when_short(99_000.0, 1_000.0, &liquid, 2_000.0, 3_000.0, 5_000.0),
            Short::CannotFund { .. }
        ));
    }

    #[test]
    fn a_post_market_shortfall_becomes_named_sales_and_an_explicit_residual() {
        let one = InstrumentId::at(7);
        let two = InstrumentId::at(8);
        let (sales, failed) = liquidates(100.0, &[(one, 3.0, Some(20.0)), (two, 2.0, Some(10.0))]);
        assert_eq!(sales, vec![(one, 3.0), (two, 2.0)]);
        assert_eq!(failed, 20.0);
        let (sales, failed) = liquidates(50.0, &[(one, 10.0, Some(10.0))]);
        assert_eq!(sales, vec![(one, 5.0)]);
        assert_eq!(failed, 0.0);
    }

    #[test]
    fn a_window_draw_is_bounded_by_haircut_collateral_and_leaves_a_residual() {
        let one = InstrumentId::at(7);
        let two = InstrumentId::at(8);
        let (pledged, advanced, failed) = pledges(
            100.0,
            0.8,
            &[(one, 3.0, Some(20.0)), (two, 2.0, Some(10.0))],
        );
        assert_eq!(pledged, vec![(one, 3.0), (two, 2.0)]);
        assert_eq!(advanced, 64.0);
        assert_eq!(failed, 36.0);
    }

    #[test]
    fn what_depositors_observe_is_observable_and_the_loop_reinforces() {
        // The deposit leaves WITH THE RESERVES BEHIND IT, so the bank is shorter at the next close —
        // and more signals mean more leaves.
        let quiet = Observed {
            capital_ratio_published: 0.14,
            drew_the_window: false,
            paid_up_for_deposits: false,
            downgraded: false,
            periods_ending_short: 0,
        };
        let visible = Observed {
            capital_ratio_published: 0.06,
            drew_the_window: true,
            paid_up_for_deposits: true,
            downgraded: true,
            periods_ending_short: 2,
        };
        assert_eq!(quiet.signals(0.10), 0.0);
        assert_eq!(visible.signals(0.10), 6.0);
        let leaving = buffer_wanted(&book(), 50.0, visible.signals(0.10), runs());
        assert!(leaving > buffer_wanted(&book(), 50.0, quiet.signals(0.10), runs()));
        // And the reserves go with them.
        assert!(after_outflow(200_000.0, leaving) < 200_000.0);
    }

    #[test]
    fn maturity_transformation_is_the_business_and_a_bank_with_none_is_not_a_bank() {
        assert!(transformation(7.0, 0.5) > 0.0);
        assert_eq!(transformation(0.5, 0.5), 0.0);
    }

    #[test]
    fn the_balance_sheet_balances_or_the_discrepancy_is_reported() {
        // A VERIFY on derived dust, repairing nothing.
        assert!(balances(10_000.0, 9_000.0, 1_000.0, 3).is_none());
        assert_eq!(balances(10_000.0, 9_000.0, 800.0, 3), Some(200.0));
    }
}
