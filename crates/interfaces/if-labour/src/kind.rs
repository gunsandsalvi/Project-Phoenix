//! The labour kind as `sys-lab` declares it: the employment line kind, the person attributes of the labour state, the
//! streams its matching draws from, the compile of each country's law, and its decisions.

use phx_core::decisions::DecisionPointDecl;
use phx_core::{OpeningCountry, Register};

use crate::decisions::{AcceptIn, AnswerIn, PostIn, PostOut, RetireIn, ReviewIn, SearchIn, SelectIn};
use crate::law::Law;

/// The offer an employer's fill history sets: from its last fill's point and days, its newest staff's point, the mean
/// wage's point and the days of the quickest match.
pub type Adapt = fn(phx_num::Missing<(i64, u32)>, phx_num::Missing<i64>, i64, u32) -> i64;

/// The labour kind: the employment line kind's name and the reasons hires, separations, severance and reviews move under; the
/// person attributes of the labour state, the occupation and the last wage point; the streams of tastes, of the
/// meeting of an application, of the employers' lots and of their reviews' phase; the visits on which employers decide, their production
/// schedule's; the hazard persons retire by; each country's law; its decisions; its wage points' arithmetic, the offer its fill history sets and the severance a separation owes.
#[derive(Clone, Copy, Debug)]
pub struct LabourKind {
    pub line: &'static str,
    pub hired: &'static str,
    pub separated: &'static str,
    pub severance: &'static str,
    pub renegotiated: &'static str,
    pub state: &'static str,
    pub occupation: &'static str,
    pub last_point: &'static str,
    pub education: &'static str,
    pub taste_stream: &'static str,
    pub meeting_stream: &'static str,
    pub lot_stream: &'static str,
    pub layoff_stream: &'static str,
    pub review_stream: &'static str,
    pub employer_visits: &'static [&'static str],
    pub retirement: &'static str,
    pub law: fn(&Register, &OpeningCountry) -> Result<Law, String>,
    pub search: &'static DecisionPointDecl<SearchIn, Vec<u32>>,
    pub accept: &'static DecisionPointDecl<AcceptIn, bool>,
    pub retire: &'static DecisionPointDecl<RetireIn, bool>,
    pub answer: &'static DecisionPointDecl<AnswerIn, i64>,
    pub review: fn(&ReviewIn) -> i64,
    pub conclude: fn(i64, i64, i64) -> phx_num::Missing<i64>,
    pub select: fn(&SelectIn) -> Vec<u32>,
    pub post: fn(&PostIn) -> PostOut,
    pub wage_at: fn(&Law, i64) -> f64,
    pub point_near: fn(&Law, f64) -> Option<i64>,
    pub least_point: fn(&Law, f64) -> Option<i64>,
    pub adapt: Adapt,
    pub owed: fn(&Law, f64, u32, i64, u32) -> f64,
}
