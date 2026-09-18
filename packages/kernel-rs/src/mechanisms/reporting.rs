//! REPORTING AND ESTIMATES: a public company publishes what its own books produced, banks publish
//! their own estimates of it, and the report SETTLES every expectation standing against it.
//!
//! @spec 48 A1 · 48 A1.a · 48 A2 · 48 A2.a · 48 A3 · 48 A4 · 48 A4.a · 48 A5 · 48 B1 · 48 B2 ·
//! @spec 48 B4 · 48 C1 · 48 C2 · 48 C3 · 48 C5 · 48 C6 · 48 D3 · 48 D3.a · 48 E1 · 48 E2 · 48 E3 ·
//! @spec 48 F1 · 48 F2.a · 48 F3 · 48 G2 · 48 G3 · 48 G4 · 48 G5 · 48 G6 · 46 A3 · Law 2, Law 4,
//! @spec Law 8, Law 19
//!
//! **Being public is a state read from the register, never a label** (A1.a): a firm is public while
//! its shares are listed and held by outsiders, and there is no kind of firm that reports (Law 15).
//! `reports` asks the register's two facts and nothing else, so a firm whose outsiders sell stops
//! reporting without anybody relabelling it.
//!
//! **No reported number the books do not produce** (A2.a). `income` is the equity account's MOVEMENT
//! over the fiscal period with the financing that moved it taken out — a read of the ledger, never a
//! figure management chose and never smoothed (G2). Earnings per share is that read divided by the
//! shares outstanding read (G5); a stated one would be an outcome written down.
//!
//! **The calendar is placed by DATE** (A3, G6): a fiscal period opens and closes on days and is a
//! whole number of periods only by accident. The report is published after the books close, and in
//! between the firm knows its result and nobody else does — the only real information asymmetry this
//! world has (A4.a).
//!
//! **An estimate cannot be handed the answer** (C5) **and cannot read the price** (C6). `Observed`
//! names what a bank may have seen, and there is no variant for the model's own forecast or for the
//! share price: an estimate that reads the price is a restatement of the market, cannot disagree with
//! it, and makes the surprise a tautology.
//!
//! **The consensus is a read and nothing decides on it** (E1, E2, E3). `consensus` computes from the
//! estimates that exist at the moment of asking; there is no field to store it in, and no door here
//! that hands it to a party as its outlook — a party may weigh it as one more published statistic,
//! which is its own outlook's business.
//!
//! **What a surprise causes is participants revising their own outlooks** (F2), so there is no
//! function in this module from a surprise to a price move (F2.a): a stated move per unit of surprise
//! is a written price path, and the move must be what the changed schedules cleared at, or nothing.

use crate::calendar::Day;
use crate::ids::PartyId;

/// A1.a: **public is a state read from the register.** Both facts come from the register; neither is
/// a label anybody sets, and a firm that fails either publishes nothing (G4).
pub fn reports(shares_listed: bool, units_held_by_outsiders: f64) -> bool {
    shares_listed && units_held_by_outsiders > 0.0
}

/// A3, G6: **a fiscal period is placed by DATE on the one calendar**, not by a count of periods, and
/// it is a whole number of periods only by accident.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Fiscal {
    pub opens: Day,
    pub closes: Day,
    /// A4: the books close, then the report comes out. The gap is where the asymmetry lives.
    pub published: Day,
}

impl Fiscal {
    pub fn new(opens: Day, closes: Day, published: Day) -> Fiscal {
        assert!(closes > opens, "48 A3: a fiscal period that does not span days is not one");
        assert!(
            published > closes,
            "48 A4: a report published before its books close has nothing to report"
        );
        Fiscal { opens, closes, published }
    }

    /// A4.a: **the only real information asymmetry this world has.** Everything else is public when
    /// it happens; here the firm knows its result and nobody else does, for these days.
    pub fn asymmetry(&self) -> i64 {
        self.published.0 - self.closes.0
    }

    pub fn holds_at(&self, day: Day) -> bool {
        day >= self.opens && day <= self.closes
    }
}

/// A1, A2: what its own books produced over the fiscal period. Every figure is a read.
#[derive(Clone, Copy, Debug)]
pub struct Books {
    /// The equity account at the two dates — the register's, not a statement of anybody's.
    pub equity_at_open: f64,
    pub equity_at_close: f64,
    /// A4/G2: what moved equity other than being earned — capital raised in, distributions out.
    /// Both are events with dates, and taking them out is what leaves INCOME behind.
    pub capital_raised: f64,
    pub distributed: f64,
    pub shares_outstanding: f64,
}

/// G2: **no earnings that were not earned.** Reported income is the equity account's movement over
/// the fiscal period, decomposed into what the instructions and the marks did — never a figure
/// management chose, and never smoothed.
pub fn income(books: &Books) -> f64 {
    (books.equity_at_close - books.equity_at_open) - books.capital_raised + books.distributed
}

/// G5: **no per-share figure that is a primitive.** Both sides are reads. `None` where there are no
/// shares: a per-share figure over no shares is not a number, and zero would be a default.
pub fn per_share(books: &Books) -> Option<f64> {
    if books.shares_outstanding <= 0.0 {
        return None;
    }
    Some(income(books) / books.shares_outstanding)
}

/// A5: a figure can be RESTATED — republished with a correction, dated, **with the original
/// standing** (§2 E2.a: a correction is a new entry, never an erasure). A restatement is information
/// about the management, which it cannot be if the first number is gone.
#[derive(Clone, Debug)]
pub struct Line {
    pub first: f64,
    pub first_on: Day,
    /// Every restatement since, in order. The original is `first` and stays there.
    pub restated: Vec<(Day, f64)>,
}

impl Line {
    pub fn published(value: f64, on: Day) -> Line {
        Line { first: value, first_on: on, restated: Vec::new() }
    }

    pub fn restate(&mut self, value: f64, on: Day) {
        assert!(on > self.first_on, "48 A5: a restatement is dated after what it restates");
        self.restated.push((on, value));
    }

    /// What the line says now — the last correction, or the original where there has been none.
    pub fn standing(&self) -> f64 {
        match self.restated.last() {
            Some(&(_, value)) => value,
            None => self.first,
        }
    }

    /// A5: and what it said first, still there.
    pub fn as_first_published(&self) -> f64 {
        self.first
    }
}

/// B1, B4: management's expectation of the coming fiscal period. **No guidance that is a second
/// number**: the published figure is the one the firm's own decisions read (§46 C2), so this carries
/// the outlook itself rather than a copy composed for the audience. A management that guides to a
/// number it is not itself acting on has had its decisions made somewhere else.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Guidance {
    pub by: PartyId,
    /// The same figure the firm's own decisions read. One writer (Law 4).
    pub outlook: f64,
    /// B2, §46 A5: a horizon and a unit are part of the number (Law 8).
    pub over: Fiscal,
    pub on: Day,
}

/// B2: revised between reports, or withdrawn. Both are events with a date, so withdrawal is a state
/// and not an absence.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Standing {
    Live(Guidance),
    Withdrawn { by: PartyId, on: Day },
}

/// C1: what a bank may have OBSERVED of a company. **There is no variant for the model's own
/// forecast** (C5) **and none for the share price** (C6) — an estimate that reads the price is a
/// restatement of the market, cannot disagree with it, and makes the surprise a tautology. The
/// absence is the mechanism: a caller cannot express either.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Observed {
    /// The company's own published report line.
    Reported { value: f64, on: Day },
    /// Its guidance.
    Guided { value: f64, on: Day },
    /// What the bank saw of the company's own markets — its goods clearing, its borrowing.
    OwnMarkets { value: f64, on: Day },
}

impl Observed {
    pub fn value(&self) -> f64 {
        match self {
            Observed::Reported { value, .. }
            | Observed::Guided { value, .. }
            | Observed::OwnMarkets { value, .. } => *value,
        }
    }

    pub fn on(&self) -> Day {
        match self {
            Observed::Reported { on, .. }
            | Observed::Guided { on, .. }
            | Observed::OwnMarkets { on, .. } => *on,
        }
    }
}

/// C1, C2: a bank's own estimate of a covered company's coming report — **named and dated**, in the
/// lines that report will carry.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Estimate {
    pub by: PartyId,
    pub of: PartyId,
    pub figure: f64,
    pub over: Fiscal,
    pub on: Day,
}

/// C1, §46 A2/B1: formed adaptively from what this bank has observed, weighted by ITS OWN memory —
/// a PREFERENCE, and its own. Two banks with different histories of a name estimate differently, and
/// that is C3's disagreement and one of the reasons a share book has two sides (§46 A3).
///
/// `None` from nothing observed: a bank that has seen nothing of a name has no estimate of it, and
/// is not covering it (D3).
pub fn estimate(
    by: PartyId,
    of: PartyId,
    seen: &[Observed],
    memory: f64,
    over: Fiscal,
    on: Day,
) -> Option<Estimate> {
    assert!(memory > 0.0 && memory < 1.0, "46 A2: a memory of {memory} is not a weighting");
    if seen.is_empty() {
        return None;
    }
    let mut ordered: Vec<&Observed> = seen.iter().collect();
    ordered.sort_by_key(|o| o.on().0);
    let mut held = ordered[0].value();
    for o in ordered.iter().skip(1) {
        held = held * memory + o.value() * (1.0 - memory);
    }
    Some(Estimate { by, of, figure: held, over, on })
}

/// D3: **coverage is uneven, and how many cover a name is an OUTCOME.** A read of the estimates that
/// exist — never a count anybody set, which is what D3.a's universal coverage would make it.
pub fn covering(of: PartyId, estimates: &[Estimate]) -> Vec<Estimate> {
    estimates.iter().filter(|e| e.of == of).copied().collect()
}

/// E1, E3: **the consensus is a read**, computed from the estimates at the moment of looking, like
/// an index from its constituents. There is nowhere to store it, so it cannot become a second number
/// that disagrees with the estimates it is made of. `None` where nobody covers the name.
pub fn consensus(of: PartyId, estimates: &[Estimate]) -> Option<f64> {
    let figures: Vec<f64> = covering(of, estimates).iter().map(|e| e.figure).collect();
    crate::num::mean(&figures)
}

/// C3, H1: the disagreement among the estimates on a name — a READ, never a target.
pub fn disagreement(of: PartyId, estimates: &[Estimate]) -> Option<f64> {
    let figures: Vec<f64> = covering(of, estimates).iter().map(|e| e.figure).collect();
    crate::num::dispersion(&figures)
}

/// F1: the report **settles** every expectation standing against it. Observed minus expected, **per
/// holder of a view**, with a name on it — §46 B2's surprise, recorded.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Surprise {
    pub held_by: PartyId,
    pub about: PartyId,
    pub expected: f64,
    pub observed: f64,
    pub on: Day,
}

impl Surprise {
    pub fn size(&self) -> f64 {
        self.observed - self.expected
    }
}

/// F1: settling the estimates AND the guidance, because management holds a view like anybody else
/// and a report that settled only the analysts would leave the one expectation the firm acted on
/// unmeasured.
pub fn settle(
    about: PartyId,
    reported: f64,
    on: Day,
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
        out.push(Surprise { held_by: g.by, about, expected: g.outlook, observed: reported, on });
    }
    out
}

/// F3: **a bank's record is a read** — how wide its own past errors on a name have been, visible to
/// everyone. It is what makes one bank's estimate weigh differently from another's in a holder's own
/// outlook (§46 B3's confidence, applied to somebody else's forecast). `None` where it has no record.
pub fn record(by: PartyId, past: &[Surprise]) -> Option<f64> {
    let errors: Vec<f64> = past.iter().filter(|s| s.held_by == by).map(|s| s.size().abs()).collect();
    crate::num::mean(&errors)
}

/// G3: **no analyst always right, and none always wrong by a fixed amount** — either is the answer
/// with an offset, which is the answer. This MEASURES the shape and reports it; it is a VERIFY and
/// it repairs nothing.
pub fn is_the_answer_with_an_offset(by: PartyId, past: &[Surprise]) -> bool {
    let errors: Vec<f64> = past.iter().filter(|s| s.held_by == by).map(|s| s.size()).collect();
    if errors.len() < 2 {
        return false;
    }
    match crate::num::dispersion(&errors) {
        // Every error identical: either always right, or always wrong by the same amount. Law 7's
        // dust over the magnitudes that went through the comparison, never a band.
        Some(spread) => spread <= crate::num::dust(errors.len(), &errors),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn quarter() -> Fiscal {
        Fiscal::new(Day(0), Day(91), Day(112))
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
    fn being_public_is_read_from_the_register_and_is_not_a_kind_of_firm() {
        // A1.a, G4: a firm becomes public when its shares are listed and held by outsiders, and
        // stops when they cease. Nothing here branches on what kind of firm it is (Law 15).
        assert!(reports(true, 400.0));
        assert!(!reports(true, 0.0));
        assert!(!reports(false, 400.0));
    }

    #[test]
    fn income_is_the_equity_accounts_movement_and_not_a_figure_management_chose() {
        // G2: 800 of movement with 200 distributed out of it is 1,000 earned. The distribution and
        // the raise are events with dates; what is left is what the instructions and the marks did.
        let earned = income(&books());
        assert!((earned - 1_000.0).abs() <= crate::num::dust(4, &[10_800.0, 10_000.0, 200.0]));
        // Capital raised in is not earnings, and taking it out is the whole of the decomposition.
        let raised = Books { capital_raised: 500.0, ..books() };
        assert!((income(&raised) - 500.0).abs() <= crate::num::dust(4, &[10_800.0, 10_000.0, 500.0]));
    }

    #[test]
    fn earnings_per_share_is_two_reads_divided_and_never_a_primitive() {
        // G5: a stated one would be an outcome written down (Law 2).
        assert_eq!(per_share(&books()), Some(1.0));
        let unshared = Books { shares_outstanding: 0.0, ..books() };
        assert!(per_share(&unshared).is_none());
    }

    #[test]
    fn the_calendar_is_placed_by_date_and_the_lag_is_the_one_asymmetry_this_world_has() {
        // A3, A4.a, G6: days, not a count of periods. Between the close and the publication the
        // firm knows its result and nobody else does.
        assert_eq!(quarter().asymmetry(), 21);
        assert!(quarter().holds_at(Day(50)));
        assert!(!quarter().holds_at(Day(100)));
    }

    #[test]
    #[should_panic(expected = "has nothing to report")]
    fn a_report_cannot_be_published_before_its_books_close() {
        Fiscal::new(Day(0), Day(91), Day(80));
    }

    #[test]
    fn a_restatement_is_a_new_entry_and_the_original_stands() {
        // A5, §2 E2.a: a correction is never an erasure — a restatement is information about the
        // management, which it cannot be if the first number is gone.
        let mut line = Line::published(1_000.0, Day(112));
        line.restate(880.0, Day(200));
        assert_eq!(line.standing(), 880.0);
        assert_eq!(line.as_first_published(), 1_000.0);
        assert_eq!(line.restated.len(), 1);
    }

    #[test]
    fn two_banks_with_different_histories_of_a_name_estimate_differently() {
        // C3, §46 A3: the disagreement is load-bearing — it is one of the reasons a share book has
        // two sides. Same company, different observations, different numbers.
        let seen_early = [
            Observed::Reported { value: 900.0, on: Day(20) },
            Observed::OwnMarkets { value: 950.0, on: Day(40) },
        ];
        let seen_late = [
            Observed::Reported { value: 900.0, on: Day(20) },
            Observed::OwnMarkets { value: 1_200.0, on: Day(60) },
        ];
        let a = estimate(party(1), party(9), &seen_early, 0.6, quarter(), Day(90)).unwrap();
        let b = estimate(party(2), party(9), &seen_late, 0.6, quarter(), Day(90)).unwrap();
        assert!(b.figure > a.figure);
        // C2: named and dated, both of them.
        assert_eq!(a.by, party(1));
        assert_eq!(b.by, party(2));
    }

    #[test]
    fn a_bank_that_has_seen_nothing_of_a_name_has_no_estimate_of_it() {
        // D3, D3.a: coverage is uneven and the count is an OUTCOME. Universal coverage would make
        // the count a constant rather than a read.
        assert!(estimate(party(1), party(9), &[], 0.6, quarter(), Day(90)).is_none());
    }

    #[test]
    fn coverage_is_uneven_and_the_count_is_a_read() {
        // D3: a widely held name carries many estimates and a small one none or one.
        let e = |by: u32, of: u32, figure: f64| Estimate {
            by: party(by),
            of: party(of),
            figure,
            over: quarter(),
            on: Day(90),
        };
        let all = [e(1, 9, 1_000.0), e(2, 9, 1_100.0), e(3, 9, 900.0), e(1, 8, 50.0)];
        assert_eq!(covering(party(9), &all).len(), 3);
        assert_eq!(covering(party(8), &all).len(), 1);
        assert_eq!(covering(party(7), &all).len(), 0);
    }

    #[test]
    fn the_consensus_is_computed_at_the_read_and_stored_nowhere() {
        // E1, E3: like an index from its constituents. There is no field to write, so it cannot
        // become a second number that disagrees with the estimates it is made of. And E2: nothing
        // in this module hands it to a party as an outlook — there is no such door to call.
        let e = |by: u32, figure: f64| Estimate {
            by: party(by),
            of: party(9),
            figure,
            over: quarter(),
            on: Day(90),
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
        // F1: observed minus expected, PER HOLDER OF A VIEW, with a name on it. A report that
        // settled only the analysts would leave unmeasured the one expectation the firm acted on.
        let e = |by: u32, figure: f64| Estimate {
            by: party(by),
            of: party(9),
            figure,
            over: quarter(),
            on: Day(90),
        };
        let all = [e(1, 1_000.0), e(2, 1_200.0)];
        let g = Guidance { by: party(9), outlook: 1_150.0, over: quarter(), on: Day(10) };
        let settled = settle(party(9), 1_100.0, Day(112), &all, Some(g));
        assert_eq!(settled.len(), 3);
        assert_eq!(settled[0].size(), 100.0);
        assert_eq!(settled[1].size(), -100.0);
        assert_eq!(settled[2].held_by, party(9));
        // F2.a: and there is no function here from a surprise to a price move. The move is what the
        // changed schedules cleared at, or nothing.
    }

    #[test]
    fn a_banks_record_is_a_read_and_a_bank_with_none_has_none() {
        // F3: how wide its own past errors have been, visible to everyone — what makes one bank's
        // estimate weigh differently from another's in a holder's own outlook.
        let s = |by: u32, expected: f64| Surprise {
            held_by: party(by),
            about: party(9),
            expected,
            observed: 1_000.0,
            on: Day(112),
        };
        let past = [s(1, 990.0), s(1, 1_030.0), s(2, 1_400.0)];
        let tight = record(party(1), &past).unwrap();
        let wide = record(party(2), &past).unwrap();
        assert!(wide > tight);
        assert!(record(party(5), &past).is_none());
    }

    #[test]
    fn an_analyst_always_wrong_by_the_same_amount_is_the_answer_with_an_offset() {
        // G3: either shape is the answer with an offset, which is the answer. This measures it and
        // repairs nothing — a VERIFY.
        let s = |expected: f64, observed: f64| Surprise {
            held_by: party(1),
            about: party(9),
            expected,
            observed,
            on: Day(112),
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
        // §46 A2: the memory is a PREFERENCE and it is the bank's own, but a weight of 1 would mean
        // it never learns and a weight of 0 that it has no history at all.
        let seen = [Observed::Reported { value: 900.0, on: Day(20) }];
        let _ = estimate(party(1), party(9), &seen, 1.0, quarter(), Day(90));
    }
}
