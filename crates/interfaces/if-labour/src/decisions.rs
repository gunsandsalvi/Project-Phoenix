//! Each labour decision's input and output. A searcher's and an employee's decisions are the household's, which the
//! player takes for its own; an employer's are its rule's.

/// What a searcher reads when it chooses where to apply: the monthly wage each vacancy it can see offers, a taste
/// drawn for each, its reservation, the applications it sends a round, and the weight of the wage in its choice.
/// The output is the places, among those seen, it applies to.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchIn {
    pub wages: Vec<f64>,
    pub tastes: Vec<f64>,
    pub reservation: f64,
    pub applications: u32,
    pub wage_weight: f64,
}

/// What a searcher reads when an offer reaches it: the offer's monthly wage, its reservation, the match's quality
/// drawn as its taste, and the weight of the wage. The output is whether it accepts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AcceptIn {
    pub wage: f64,
    pub reservation: f64,
    pub taste: f64,
    pub wage_weight: f64,
}

/// An applicant as its employer reads it: its skill level, its years of experience, its lot, and the members it
/// brings, its twins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Applicant {
    pub skill: u32,
    pub experience: u32,
    pub lot: u64,
    pub unit: u32,
}

/// What an employer reads when it selects among a vacancy's applicants: them, and the jobs it has open. The output is
/// the places of those it offers the job, in the order chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectIn {
    pub applicants: Vec<Applicant>,
    pub open: u32,
}

/// An employer's work in one occupation family: its hours for a unit of output, the hours its staff give a day, the
/// hours its open vacancies would add, the hours a day of one job, and an hour's wage at the point it would offer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Need {
    pub occupation: u32,
    pub hours_a_unit: f64,
    pub staff_hours: f64,
    pub open_hours: f64,
    pub job_hours: f64,
    pub wage_hour: f64,
}

/// What an employer reads on its production schedule: its output's price outlook, the units a day it plans, what
/// financing an hour's wage until its output sells costs as a share of it, the least an hour may pay, and its work in
/// each occupation family.
#[derive(Clone, Debug, PartialEq)]
pub struct PostIn {
    pub price: f64,
    pub units_a_day: f64,
    pub financing: f64,
    pub minimum_hour: f64,
    pub needs: Vec<Need>,
}

/// An employer's decision: the jobs it posts, the vacancies it withdraws and the jobs it lays off, each by occupation
/// family.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PostOut {
    pub post: Vec<(u32, u32)>,
    pub withdraw: Vec<(u32, u32)>,
    pub layoff: Vec<(u32, u32)>,
}

/// What a person reads when it may retire: its age and the age its country's pension begins at, each in whole months.
/// The output is whether it retires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetireIn {
    pub age_months: i64,
    pub pension_months: i64,
}
