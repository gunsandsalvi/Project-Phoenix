//! The labour kind as `sys-lab` declares it: the employment line kind, the person fields of the labour state, the
//! streams its matching draws from, the compile of each country's law, and its decisions.

use phx_core::decisions::DecisionPointDecl;
use phx_core::{OpeningCountry, Register};

use crate::decisions::{AcceptIn, AnswerIn, PostIn, PostOut, ReviewIn, SearchIn, SelectIn};
use crate::law::Law;

/// The offer an employer's fill history sets: from its last fill's point and days, its newest staff's point, the mean
/// wage's point and the days of the quickest match.
pub type Adapt = fn(phx_num::Missing<(i64, u32)>, phx_num::Missing<i64>, i64, u32) -> i64;

/// The labour kind: the employment line kind's name and the reasons hires, separations, severance and reviews move under; the
/// person fields of the labour state, the occupation, the last wage point and the education; the streams of tastes, of the
/// meeting of an application, of the employers' lots and of their reviews' phase; the visits on which employers decide, their production
/// schedule's; the hazard persons retire by; each country's law; its decisions; its wage points' arithmetic, the offer its fill history sets and the severance a separation owes.
#[derive(Clone, Copy, Debug)]
pub struct LabourKind {
    pub state: phx_core::person_word::Field,
    pub occupation: phx_core::person_word::Field,
    pub last_point: phx_core::person_word::Field,
    pub education: phx_core::person_word::Field,
    pub taste_stream: &'static str,
    pub meeting_stream: &'static str,
    pub lot_stream: &'static str,
    pub layoff_stream: &'static str,
    pub review_stream: &'static str,
    pub law: fn(&Register, &OpeningCountry) -> Result<Law, String>,
    pub search: &'static DecisionPointDecl<SearchIn, Vec<u32>>,
    pub accept: &'static DecisionPointDecl<AcceptIn, bool>,
    pub answer: &'static DecisionPointDecl<AnswerIn, i64>,
    pub offer: &'static DecisionPointDecl<ReviewIn, i64>,
    pub conclude: fn(i64, i64, i64) -> phx_num::Missing<i64>,
    pub select: &'static DecisionPointDecl<SelectIn, Vec<u32>>,
    pub post: &'static DecisionPointDecl<PostIn, PostOut>,
    pub wage_at: fn(&Law, i64) -> f64,
    pub point_near: fn(&Law, f64) -> Option<i64>,
    pub least_point: fn(&Law, f64) -> Option<i64>,
    pub adapt: Adapt,
    pub owed: fn(&Law, f64, u32, i64, u32) -> f64,
}
