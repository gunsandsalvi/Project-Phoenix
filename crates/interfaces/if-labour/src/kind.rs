//! The labour kind as `sys-lab` declares it: the employment line kind, the person attributes of the labour state, the
//! streams its matching draws from, the compile of each country's law, and its decisions.

use phx_core::decisions::DecisionPointDecl;
use phx_core::{OpeningCountry, Register};

use crate::decisions::{AcceptIn, PostIn, PostOut, RetireIn, SearchIn, SelectIn};
use crate::law::Law;

/// The labour kind: the employment line kind's name and the reasons hires, separations and severance move under; the
/// person attributes of the labour state, the occupation and the last wage point; the streams of tastes, of the
/// meeting of an application and of the employers' lots; the visits on which employers decide, their production
/// schedule's; the hazard persons retire by; each country's law; its decisions; and its wage points' arithmetic.
#[derive(Clone, Copy, Debug)]
pub struct LabourKind {
    pub line: &'static str,
    pub hired: &'static str,
    pub separated: &'static str,
    pub severance: &'static str,
    pub state: &'static str,
    pub occupation: &'static str,
    pub last_point: &'static str,
    pub education: &'static str,
    pub taste_stream: &'static str,
    pub meeting_stream: &'static str,
    pub lot_stream: &'static str,
    pub layoff_stream: &'static str,
    pub employer_visits: &'static [&'static str],
    pub retirement: &'static str,
    pub law: fn(&Register, &OpeningCountry) -> Result<Law, String>,
    pub search: &'static DecisionPointDecl<SearchIn, Vec<u32>>,
    pub accept: &'static DecisionPointDecl<AcceptIn, bool>,
    pub retire: &'static DecisionPointDecl<RetireIn, bool>,
    pub select: fn(&SelectIn) -> Vec<u32>,
    pub post: fn(&PostIn) -> PostOut,
    pub wage_at: fn(&Law, i64) -> f64,
    pub point_near: fn(&Law, f64) -> Option<i64>,
    pub least_point: fn(&Law, f64) -> Option<i64>,
}
