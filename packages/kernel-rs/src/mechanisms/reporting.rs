//! REPORTING AND ESTIMATES: a public company publishes what its own books produced, banks publish
//! their own estimates of it, and the report SETTLES every expectation standing against it.
//!
//! @spec 48 A1 · 48 A1.a · 48 A2 · 48 A2.a · 48 A3 · 48 A4 · 48 A4.a · 48 A5 · 48 B1 · 48 B2 ·
//! @spec 48 B4 · 48 C1 · 48 C2 · 48 C3 · 48 C5 · 48 C6 · 48 D3 · 48 D3.a · 48 E1 · 48 E2 · 48 E3 ·
//! @spec 48 F1 · 48 F2.a · 48 F3 · 48 G2 · 48 G3 · 48 G4 · 48 G5 · 48 G6 · 46 A3 · Law 2, Law 4,
//! @spec Law 8, Law 19

use crate::calendar::Week;
use crate::ids::{InstrumentId, PartyId};
use crate::instruments::Class;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::standing;

/// Public is a state read from the register.
pub fn reports(shares_listed: bool, units_held_by_outsiders: f64) -> bool {
    shares_listed && units_held_by_outsiders > 0.0
}

/// A fiscal week is placed by DATE on the one calendar, not by a count of weeks, and it is a
/// whole number of weeks only by accident.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Fiscal {
    pub opens: Week,
    pub closes: Week,
    /// The books close, then the report comes out.
    pub published: Week,
}

impl Fiscal {
    pub fn new(opens: Week, closes: Week, published: Week) -> Fiscal {
        assert!(
            closes > opens,
            "48 A3: a fiscal week that does not span days is not one"
        );
        assert!(
            published > closes,
            "48 A4: a report published before its books close has nothing to report"
        );
        Fiscal {
            opens,
            closes,
            published,
        }
    }

    /// The only real information asymmetry this world has.
    pub fn asymmetry(&self) -> i64 {
        self.published.0 - self.closes.0
    }

    pub fn holds_at(&self, day: Week) -> bool {
        day >= self.opens && day <= self.closes
    }
}

/// What its own books produced over the fiscal week.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Books {
    /// The equity account at the two dates — the register's, not a statement of anybody's.
    pub equity_at_open: f64,
    pub equity_at_close: f64,
    /// A4/G2: what moved equity other than being earned — capital raised in, distributions out.
    pub capital_raised: f64,
    pub distributed: f64,
    pub shares_outstanding: f64,
}

/// No earnings that were not earned.
pub fn income(books: &Books) -> f64 {
    (books.equity_at_close - books.equity_at_open) - books.capital_raised + books.distributed
}

/// The two dated book closes from which a report is made. Keeping the opening close beside the
/// closing close prevents a later restatement from silently changing the comparative.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Comparative {
    pub opened_on: Week,
    pub closed_on: Week,
    pub books: Books,
}

impl Comparative {
    pub fn income(&self) -> Option<f64> {
        (self.closed_on > self.opened_on).then(|| income(&self.books))
    }
}

/// One immutable publication. A correction appends another version with the same comparative.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ReportVersion {
    pub issuer: PartyId,
    pub version: u32,
    pub comparative: Comparative,
    pub published_on: Week,
    pub income: f64,
}

pub fn publish(
    issuer: PartyId,
    comparative: Comparative,
    published_on: Week,
    previous: &[ReportVersion],
) -> Option<ReportVersion> {
    if published_on <= comparative.closed_on {
        return None;
    }
    let earned = comparative.income()?;
    let version = previous
        .iter()
        .filter(|r| {
            r.issuer == issuer
                && r.comparative.opened_on == comparative.opened_on
                && r.comparative.closed_on == comparative.closed_on
        })
        .fold(0, |next, r| {
            if r.version >= next {
                r.version + 1
            } else {
                next
            }
        });
    Some(ReportVersion {
        issuer,
        version,
        comparative,
        published_on,
        income: earned,
    })
}

/// No per-share figure that is a primitive.
pub fn per_share(books: &Books) -> Option<f64> {
    if books.shares_outstanding <= 0.0 {
        return None;
    }
    Some(income(books) / books.shares_outstanding)
}

/// A figure can be RESTATED — republished with a correction, dated, with the original standing (§2 A
/// correction is a new entry, never an erasure).
#[derive(Clone, Debug)]
pub struct Line {
    pub first: f64,
    pub first_on: Week,
    /// Every restatement since, in order.
    pub restated: Vec<(Week, f64)>,
}

impl Line {
    pub fn published(value: f64, on: Week) -> Line {
        Line {
            first: value,
            first_on: on,
            restated: Vec::new(),
        }
    }

    pub fn restate(&mut self, value: f64, on: Week) {
        assert!(
            on > self.first_on,
            "48 A5: a restatement is dated after what it restates"
        );
        self.restated.push((on, value));
    }

    /// What the line says now — the last correction, or the original where there has been none.
    pub fn standing(&self) -> f64 {
        match self.restated.last() {
            Some(&(_, value)) => value,
            None => self.first,
        }
    }

    /// And what it said first, still there.
    pub fn as_first_published(&self) -> f64 {
        self.first
    }
}

/// Management's expectation of the coming fiscal week.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Guidance {
    pub by: PartyId,
    /// The same figure the firm's own decisions read.
    pub outlook: f64,
    /// A horizon and a unit are part of the number.
    pub over: Fiscal,
    pub on: Week,
}

/// Revised between reports, or withdrawn.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Standing {
    Live(Guidance),
    Withdrawn { by: PartyId, on: Week },
}

/// What a bank may have OBSERVED of a company.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Observed {
    /// The company's own published report line.
    Reported { value: f64, on: Week },
    /// Its guidance.
    Guided { value: f64, on: Week },
    /// What the bank saw of the company's own markets — its goods clearing, its borrowing.
    OwnMarkets { value: f64, on: Week },
}

impl Observed {
    pub fn value(&self) -> f64 {
        match self {
            Observed::Reported { value, .. }
            | Observed::Guided { value, .. }
            | Observed::OwnMarkets { value, .. } => *value,
        }
    }

    pub fn on(&self) -> Week {
        match self {
            Observed::Reported { on, .. }
            | Observed::Guided { on, .. }
            | Observed::OwnMarkets { on, .. } => *on,
        }
    }
}

/// A bank's own estimate of a covered company's coming report — named and dated, in the lines that
/// report will carry.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Estimate {
    pub by: PartyId,
    pub of: PartyId,
    pub figure: f64,
    pub over: Fiscal,
    pub on: Week,
}

/// C1, §46 A2/B1: formed adaptively from what this bank has observed, weighted by ITS OWN memory — a
/// PREFERENCE, and its own.
pub fn estimate(
    by: PartyId,
    of: PartyId,
    seen: &[Observed],
    memory: f64,
    over: Fiscal,
    on: Week,
) -> Option<Estimate> {
    assert!(
        memory > 0.0 && memory < 1.0,
        "46 A2: a memory of {memory} is not a weighting"
    );
    if seen.is_empty() {
        return None;
    }
    let mut ordered: Vec<&Observed> = seen.iter().collect();
    ordered.sort_by_key(|o| o.on().0);
    let mut held = ordered[0].value();
    for o in ordered.iter().skip(1) {
        held = held * memory + o.value() * (1.0 - memory);
    }
    Some(Estimate {
        by,
        of,
        figure: held,
        over,
        on,
    })
}

/// Coverage is uneven, and how many cover a name is an OUTCOME.
pub fn covering(of: PartyId, estimates: &[Estimate]) -> Vec<Estimate> {
    estimates.iter().filter(|e| e.of == of).copied().collect()
}

/// The consensus is a read, computed from the estimates at the moment of looking, like an index from
/// its constituents.
pub fn consensus(of: PartyId, estimates: &[Estimate]) -> Option<f64> {
    let figures: Vec<f64> = covering(of, estimates).iter().map(|e| e.figure).collect();
    crate::num::mean(&figures)
}

/// Only estimates that existed before the books closed can comprise the frozen forecast.
pub fn eligible_consensus(of: PartyId, fiscal: Fiscal, estimates: &[Estimate]) -> Option<f64> {
    let figures: Vec<f64> = estimates
        .iter()
        .filter(|e| e.of == of && e.over == fiscal && e.on <= fiscal.closes)
        .map(|e| e.figure)
        .collect();
    crate::num::mean(&figures)
}

pub fn consensus_surprise(report: &ReportVersion, estimates: &[Estimate]) -> Option<Surprise> {
    let fiscal = Fiscal::new(
        report.comparative.opened_on,
        report.comparative.closed_on,
        report.published_on,
    );
    let expected = eligible_consensus(report.issuer, fiscal, estimates)?;
    Some(Surprise {
        held_by: report.issuer,
        about: report.issuer,
        expected,
        observed: report.income,
        on: report.published_on,
    })
}

/// The disagreement among the estimates on a name — a READ, never a target.
pub fn disagreement(of: PartyId, estimates: &[Estimate]) -> Option<f64> {
    let figures: Vec<f64> = covering(of, estimates).iter().map(|e| e.figure).collect();
    crate::num::dispersion(&figures)
}

/// The report settles every expectation standing against it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Surprise {
    pub held_by: PartyId,
    pub about: PartyId,
    pub expected: f64,
    pub observed: f64,
    pub on: Week,
}

impl Surprise {
    pub fn size(&self) -> f64 {
        self.observed - self.expected
    }
}

/// Settling the estimates AND the guidance, because management holds a view like anybody else and a
/// report that settled only the analysts would leave the one expectation the firm acted on
/// unmeasured.
pub fn settle(
    about: PartyId,
    reported: f64,
    on: Week,
    estimates: &[Estimate],
    guidance: Option<Guidance>,
) -> Vec<Surprise> {
    let mut out: Vec<Surprise> = covering(about, estimates)
        .iter()
        .map(|e| Surprise {
            held_by: e.by,
            about,
            expected: e.figure,
            observed: reported,
            on,
        })
        .collect();
    if let Some(g) = guidance {
        out.push(Surprise {
            held_by: g.by,
            about,
            expected: g.outlook,
            observed: reported,
            on,
        });
    }
    out
}

/// A bank's record is a read — how wide its own past errors on a name have been, visible to
/// everyone.
pub fn record(by: PartyId, past: &[Surprise]) -> Option<f64> {
    let errors: Vec<f64> = past
        .iter()
        .filter(|s| s.held_by == by)
        .map(|s| s.size().abs())
        .collect();
    crate::num::mean(&errors)
}

/// No analyst always right, and none always wrong by a fixed amount — either is the answer with an
/// offset, which is the answer.
pub fn is_the_answer_with_an_offset(by: PartyId, past: &[Surprise]) -> bool {
    let errors: Vec<f64> = past
        .iter()
        .filter(|s| s.held_by == by)
        .map(|s| s.size())
        .collect();
    if errors.len() < 2 {
        return false;
    }
    match crate::num::dispersion(&errors) {
        // Every error identical: either always right, or always wrong by the same amount.
        Some(spread) => spread <= crate::num::dust(errors.len(), &errors),
        None => false,
    }
}

/// A PUBLIC COMPANY PUBLISHES WHAT ITS OWN BOOKS PRODUCED.
pub struct Publishes {
    /// The event kind the accounts are published under.
    pub kind: u32,
    /// Key rows for the figures, so a reader takes them by name rather than by position.
    pub at_equity: u32,
    pub at_income: u32,
    pub at_shares: u32,
    /// Which fiscal close a report is FOR.
    pub at_closed: u32,
    /// How many days after the books close the report comes out.
    pub asymmetry: &'static str,
}

impl Mechanism for Publishes {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let today = ctx.today();
        let asymmetry = ctx.params().weeks(self.asymmetry) as i64;

        // What each company last published, read off the journal's own rows — one pass, not one walk
        // of the world's history per company (Law 19: the read replaces the walk).
        let mut last: std::collections::HashMap<u32, (u32, f64)> = std::collections::HashMap::new();
        // And which fiscal close each report was ABOUT, so a quarter is published once and a
        // restatement is a different act.
        let mut reported: std::collections::HashSet<(u32, i64)> = std::collections::HashSet::new();
        for &row in ctx.journal().of_kind(self.kind) {
            let when = ctx.journal().period_of(row);
            if let (Some(&who), Some(Value::Num(equity))) = (
                ctx.journal().subjects_of(row).first(),
                ctx.journal().says(row, self.at_equity),
            ) {
                last.insert(who, (when, equity));
                if let Some(Value::Num(about)) = ctx.journal().says(row, self.at_closed) {
                    reported.insert((who, about as i64));
                }
            }
        }

        let mut out: Vec<(u32, f64, Option<f64>, f64, i64)> = Vec::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            // Listed, and held by outsiders.
            let mut listed = 0.0;
            let mut outsiders = 0.0;
            for &line in ctx.instruments().of_issuer(who) {
                let share = InstrumentId::at(line);
                if ctx.instruments().class_of(share) != Class::Share {
                    continue;
                }
                let (held, _) = ctx.register().held_total(share);
                listed += held;
                outsiders += held - ctx.register().quantity(ctx.register().row(who, share));
            }
            if !reports(listed > 0.0, outsiders) {
                continue;
            }
            // THE FISCAL PERIOD IS A QUARTER, placed by DATE from the day this company started —
            // three months of calendar, which is a whole number of weeks only by accident.
            let born = ctx
                .calendar()
                .at(crate::calendar::Week(i64::from(ctx.parties().since(who))));
            // The tick a quarter's books close on is the one before the tick its end date falls on.
            let ends = |months: u32| Week(ctx.calendar().months_after(born, months).0 - 1);
            let mut elapsed = 3;
            let mut opens = born;
            let mut closes = ends(elapsed);
            // The LAST quarter whose report is due.
            while ends(elapsed + 3).0 + asymmetry <= today.0 {
                opens = Week(closes.0 + 1);
                elapsed += 3;
                closes = ends(elapsed);
            }
            if closes.0 >= today.0 {
                continue;
            }
            let fiscal = Fiscal::new(opens, closes, Week(closes.0 + asymmetry));
            if today < fiscal.published {
                continue;
            }
            // And it publishes each quarter ONCE.
            if reported.contains(&(row, fiscal.closes.0)) {
                continue;
            }
            // Audit B5.a: the figure a company publishes is its own ACCOUNT, moved by capital paid
            // in, income earned and losses booked. Publishing the residual instead would make the
            // audit's comparison a restatement of one number.
            let now = ctx.equity().balance_of(who);
            // Income is the MOVEMENT against what it last published.
            let income = last.get(&row).map(|&(_, was)| now - was);
            out.push((row, now, income, listed, fiscal.closes.0));
        }
        // And the banks that cover a name estimate what it will report.
        let mut estimating: Vec<(PartyId, PartyId, f64)> = Vec::new();
        for (who, worth, _, _, _) in &out {
            let company = PartyId(*who);
            for &line in ctx.instruments().of_issuer(company) {
                for &row in ctx.register().of_instrument(InstrumentId::at(line)) {
                    let row = crate::ids::HoldingId(row);
                    let bank = ctx.register().holder_of(row);
                    let covers_credit = ctx
                        .registry()
                        .profile(ctx.parties().kind_of(bank))
                        .is_some_and(|profile| {
                            profile.issues_money
                                && profile.banks == crate::registry::Banks::AtTheCentralBank
                        });
                    if bank == company || !covers_credit || ctx.register().quantity(row) <= 0.0 {
                        continue;
                    }
                    let held =
                        match ctx
                            .standing()
                            .of_party_about(bank, company, standing::ESTIMATE)
                        {
                            Some(st) => ctx.standing().terms(st)[0],
                            // A bank that has seen nothing of a name has no estimate of it and is not
                            // covering it — its first is what the first report it saw said.
                            None => *worth,
                        };
                    let memory = ctx.parties().outlook_memory(bank);
                    estimating.push((bank, company, held + (*worth - held) / memory));
                }
            }
        }
        for (bank, company, figure) in estimating {
            // It HOLDS it, so it can be shown to have been wrong — and F1's surprise is the report
            // against what was standing when it arrived.
            ctx.now_stands(standing::ESTIMATE, bank, company, vec![figure]);
        }

        for (who, worth, income, shares, closed) in out {
            let mut data = vec![
                (self.at_equity, Value::Num(worth)),
                (self.at_shares, Value::Num(shares)),
                // WHICH fiscal close this is the report for.
                (self.at_closed, Value::Num(closed as f64)),
            ];
            if let Some(earned) = income {
                data.push((self.at_income, Value::Num(earned)));
            }
            // Published, which is what makes it something anybody else may read.
            ctx.say(self.kind, &[who], &data, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn quarter() -> Fiscal {
        Fiscal::new(Week(0), Week(91), Week(112))
    }

    fn books() -> Books {
        Books {
            equity_at_open: 10_000.0,
            equity_at_close: 10_800.0,
            capital_raised: 0.0,
            distributed: 200.0,
            shares_outstanding: 1_000.0,
        }
    }

    #[test]
    fn reports_keep_dated_comparatives_versions_and_a_frozen_consensus() {
        let comparative = Comparative {
            opened_on: Week(10),
            closed_on: Week(20),
            books: books(),
        };
        assert!(publish(party(9), comparative, Week(20), &[]).is_none());
        let first = publish(party(9), comparative, Week(21), &[]).unwrap();
        let restated = publish(party(9), comparative, Week(22), &[first]).unwrap();
        assert_eq!((first.version, restated.version), (0, 1));
        let fiscal = Fiscal::new(Week(10), Week(20), Week(21));
        let estimates = [
            Estimate {
                by: party(1),
                of: party(9),
                figure: 900.0,
                over: fiscal,
                on: Week(19),
            },
            Estimate {
                by: party(2),
                of: party(9),
                figure: 5_000.0,
                over: fiscal,
                on: Week(21),
            },
        ];
        let surprise = consensus_surprise(&first, &estimates).unwrap();
        assert_eq!(surprise.expected, 900.0);
        assert_eq!(surprise.observed, 1_000.0);
    }

    #[test]
    fn being_public_is_read_from_the_register_and_is_not_a_kind_of_firm() {
        // A firm becomes public when its shares are listed and held by outsiders, and stops when
        // they cease.
        assert!(reports(true, 400.0));
        assert!(!reports(true, 0.0));
        assert!(!reports(false, 400.0));
    }

    #[test]
    fn income_is_the_equity_accounts_movement_and_not_a_figure_management_chose() {
        // 800 of movement with 200 distributed out of it is 1,000 earned.
        let earned = income(&books());
        assert!((earned - 1_000.0).abs() <= crate::num::dust(4, &[10_800.0, 10_000.0, 200.0]));
        // Capital raised in is not earnings, and taking it out is the whole of the decomposition.
        let raised = Books {
            capital_raised: 500.0,
            ..books()
        };
        assert!(
            (income(&raised) - 500.0).abs() <= crate::num::dust(4, &[10_800.0, 10_000.0, 500.0])
        );
    }

    #[test]
    fn earnings_per_share_is_two_reads_divided_and_never_a_primitive() {
        // A stated one would be an outcome written down.
        assert_eq!(per_share(&books()), Some(1.0));
        let unshared = Books {
            shares_outstanding: 0.0,
            ..books()
        };
        assert!(per_share(&unshared).is_none());
    }

    #[test]
    fn the_calendar_is_placed_by_date_and_the_lag_is_the_one_asymmetry_this_world_has() {
        // Days, not a count of weeks.
        assert_eq!(quarter().asymmetry(), 21);
        assert!(quarter().holds_at(Week(50)));
        assert!(!quarter().holds_at(Week(100)));
    }

    #[test]
    #[should_panic(expected = "has nothing to report")]
    fn a_report_cannot_be_published_before_its_books_close() {
        Fiscal::new(Week(0), Week(91), Week(80));
    }

    #[test]
    fn a_restatement_is_a_new_entry_and_the_original_stands() {
        // A correction is never an erasure — a restatement is information about the management,
        // which it cannot be if the first number is gone.
        let mut line = Line::published(1_000.0, Week(112));
        line.restate(880.0, Week(200));
        assert_eq!(line.standing(), 880.0);
        assert_eq!(line.as_first_published(), 1_000.0);
        assert_eq!(line.restated.len(), 1);
    }

    #[test]
    fn two_banks_with_different_histories_of_a_name_estimate_differently() {
        // The disagreement is load-bearing — it is one of the reasons a share book has two sides.
        let seen_early = [
            Observed::Reported {
                value: 900.0,
                on: Week(20),
            },
            Observed::OwnMarkets {
                value: 950.0,
                on: Week(40),
            },
        ];
        let seen_late = [
            Observed::Reported {
                value: 900.0,
                on: Week(20),
            },
            Observed::OwnMarkets {
                value: 1_200.0,
                on: Week(60),
            },
        ];
        let a = estimate(party(1), party(9), &seen_early, 0.6, quarter(), Week(90)).unwrap();
        let b = estimate(party(2), party(9), &seen_late, 0.6, quarter(), Week(90)).unwrap();
        assert!(b.figure > a.figure);
        // Named and dated, both of them.
        assert_eq!(a.by, party(1));
        assert_eq!(b.by, party(2));
    }

    #[test]
    fn a_bank_that_has_seen_nothing_of_a_name_has_no_estimate_of_it() {
        // Coverage is uneven and the count is an OUTCOME.
        assert!(estimate(party(1), party(9), &[], 0.6, quarter(), Week(90)).is_none());
    }

    #[test]
    fn coverage_is_uneven_and_the_count_is_a_read() {
        // A widely held name carries many estimates and a small one none or one.
        let e = |by: u32, of: u32, figure: f64| Estimate {
            by: party(by),
            of: party(of),
            figure,
            over: quarter(),
            on: Week(90),
        };
        let all = [
            e(1, 9, 1_000.0),
            e(2, 9, 1_100.0),
            e(3, 9, 900.0),
            e(1, 8, 50.0),
        ];
        assert_eq!(covering(party(9), &all).len(), 3);
        assert_eq!(covering(party(8), &all).len(), 1);
        assert_eq!(covering(party(7), &all).len(), 0);
    }

    #[test]
    fn the_consensus_is_computed_at_the_read_and_stored_nowhere() {
        // Like an index from its constituents.
        let e = |by: u32, figure: f64| Estimate {
            by: party(by),
            of: party(9),
            figure,
            over: quarter(),
            on: Week(90),
        };
        let all = [e(1, 1_000.0), e(2, 1_100.0), e(3, 900.0)];
        assert_eq!(consensus(party(9), &all), Some(1_000.0));
        assert!(consensus(party(7), &all).is_none());
        // Adding a fourth estimate changes the read immediately, because there is nothing between
        // the estimates and the answer.
        let more = [e(1, 1_000.0), e(2, 1_100.0), e(3, 900.0), e(4, 1_400.0)];
        assert_eq!(consensus(party(9), &more), Some(1_100.0));
    }

    #[test]
    fn the_report_settles_every_expectation_standing_against_it_including_managements() {
        // Observed minus expected, PER HOLDER OF A VIEW, with a name on it.
        let e = |by: u32, figure: f64| Estimate {
            by: party(by),
            of: party(9),
            figure,
            over: quarter(),
            on: Week(90),
        };
        let all = [e(1, 1_000.0), e(2, 1_200.0)];
        let g = Guidance {
            by: party(9),
            outlook: 1_150.0,
            over: quarter(),
            on: Week(10),
        };
        let settled = settle(party(9), 1_100.0, Week(112), &all, Some(g));
        assert_eq!(settled.len(), 3);
        assert_eq!(settled[0].size(), 100.0);
        assert_eq!(settled[1].size(), -100.0);
        assert_eq!(settled[2].held_by, party(9));
        // And there is no function here from a surprise to a price move.
    }

    #[test]
    fn a_banks_record_is_a_read_and_a_bank_with_none_has_none() {
        // How wide its own past errors have been, visible to everyone — what makes one bank's
        // estimate weigh differently from another's in a holder's own outlook.
        let s = |by: u32, expected: f64| Surprise {
            held_by: party(by),
            about: party(9),
            expected,
            observed: 1_000.0,
            on: Week(112),
        };
        let past = [s(1, 990.0), s(1, 1_030.0), s(2, 1_400.0)];
        let tight = record(party(1), &past).unwrap();
        let wide = record(party(2), &past).unwrap();
        assert!(wide > tight);
        assert!(record(party(5), &past).is_none());
    }

    #[test]
    fn an_analyst_always_wrong_by_the_same_amount_is_the_answer_with_an_offset() {
        // Either shape is the answer with an offset, which is the answer.
        let s = |expected: f64, observed: f64| Surprise {
            held_by: party(1),
            about: party(9),
            expected,
            observed,
            on: Week(112),
        };
        let offset = [s(900.0, 1_000.0), s(1_100.0, 1_200.0), s(800.0, 900.0)];
        assert!(is_the_answer_with_an_offset(party(1), &offset));
        let always_right = [s(1_000.0, 1_000.0), s(1_200.0, 1_200.0)];
        assert!(is_the_answer_with_an_offset(party(1), &always_right));
        let honest = [s(900.0, 1_000.0), s(1_100.0, 1_050.0), s(800.0, 960.0)];
        assert!(!is_the_answer_with_an_offset(party(1), &honest));
    }

    #[test]
    #[should_panic(expected = "is not a weighting")]
    fn a_memory_that_is_not_a_weighting_is_refused() {
        // The memory is a PREFERENCE and it is the bank's own, but a weight of 1 would mean it never
        // learns and a weight of 0 that it has no history at all.
        let seen = [Observed::Reported {
            value: 900.0,
            on: Week(20),
        }];
        let _ = estimate(party(1), party(9), &seen, 1.0, quarter(), Week(90));
    }
}
