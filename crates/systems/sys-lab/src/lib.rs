//! LAB, labour: each adult's labour state and the jobs the opening draws, each a row on the employment line of its
//! class; the employers' vacancies and layoffs, the searchers' applications and answers, and retirement, whose rules
//! are here and whose rounds the kernel runs.

mod consts;
mod jobs;
pub mod law;
pub mod points;
pub mod retire;
pub mod rules;
pub mod wages;

use if_labour::kind::LabourKind;
use phx_core::{
    Declarations, EventKindDecl, PersonAttrDecl, StreamDef, System, declare_hazard, declare_prim, declare_stream,
};
use phx_num::{Count, Fixed, Missing};

pub use jobs::{AS_EMPLOYEE, AS_OWNER, Drawn, Jobs, JobsStream, Rule};

declare_stream! { pub TasteStream = "LAB.match_taste" { purpose: Meeting, keyed: false, clause: "REP.22" } }
declare_stream! { pub MeetingStream = "LAB.meeting" { purpose: Meeting, keyed: false, clause: "LAB.8" } }
declare_stream! { pub LotStream = "LAB.select_lot" { purpose: Meeting, keyed: false, clause: "LAB.7" } }
declare_stream! { pub LayoffStream = "LAB.layoff" { purpose: Meeting, keyed: false, clause: "REP.23" } }
declare_stream! { pub RetirementStream = "LAB.retirement" { purpose: Birthday, keyed: false, clause: "LAB.6" } }
declare_stream! { pub ReviewStream = "LAB.review_phase" { purpose: SchedulePhase, keyed: false, clause: "LAB.17" } }

/// A person's labour state: not searching, searching or retired.
pub const STATE: PersonAttrDecl = PersonAttrDecl {
    name: "LAB.state",
    values: if_labour::class::STATES,
    clause: "PTY.3",
    initial: Missing::Present(if_labour::class::NOT_SEARCHING),
};
/// The occupation family a person last worked in, or none.
pub const OCCUPATION_ATTR: PersonAttrDecl = PersonAttrDecl {
    name: "LAB.occupation",
    values: if_labour::class::OCCUPATIONS,
    clause: "PTY.3",
    initial: Missing::Present(if_labour::class::NO_OCCUPATION),
};
/// The wage point of a person's last job, or of the wage it would have earned at the opening; none for one who has
/// never been counted in work, the last value.
pub const LAST_POINT: PersonAttrDecl = PersonAttrDecl {
    name: "LAB.last_point",
    values: if_labour::class::WAGE_POINTS,
    clause: "PTY.3",
    initial: Missing::Present(if_labour::class::NO_POINT),
};

declare_hazard! {
    pub RETIREMENT = "LAB.retirement" {
        acts_on: Persons("household"), rate: "SOC.pension_age", axes: ["DEM.age", "DEM.sex"],
        changes: [Birthday], outcome: "LAB.retired", scheme: Scheduled, stream: "LAB.retirement",
        source: "an adult retires on the first birthday it has reached its country's pension age", clause: "LAB.6",
    }
}

declare_prim! {
    /// Each sex's and ten-year age band's employment rate over the rate of both sexes from 15.
    pub EMPLOYMENT_BY_AGE = "LAB.employment_by_age" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// Women's share of each occupation family's employed (ISCO-08 major group).
    pub WOMEN = "LAB.women_by_occupation" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The employed making each product who are self-employed, running their own business: employers and
    /// own-account workers.
    pub SELF_EMPLOYED = "LAB.self_employed_shares" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The ratio between neighbouring wage points, the round numbers wages are offered and paid at.
    pub WAGE_POINT_RATIO = "LAB.wage_point_ratio" {
        kind: Resolution, value: Fixed { exp: 6 }, clause: "REP.34", scope: Shared
    }
}

declare_prim! {
    /// The unemployed among the labour force at the opening, by sex.
    pub UNEMPLOYMENT = "LAB.unemployment_rates" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The employees working part time, by sex.
    pub PART_TIME = "LAB.part_time_shares" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The hours a week of a part-time job.
    pub PART_TIME_HOURS = "LAB.part_time_hours" { kind: Endowment, value: Count, clause: "LAB.1", scope: PerCountry }
}

declare_prim! {
    /// The hours a week of a full-time job, the law's normal week.
    pub FULL_TIME_HOURS = "LAB.full_time_hours" {
        kind: Policy, decided_by: "parliament", value: Count, clause: "LAB.16", scope: PerCountry
    }
}

declare_prim! {
    /// The days of notice an employer gives before a layoff takes effect.
    pub NOTICE_DAYS = "LAB.notice_days" {
        kind: Policy, decided_by: "parliament", value: Count, clause: "LAB.16", scope: PerCountry
    }
}

declare_prim! {
    /// The days of wages owed as severance for each whole year of service.
    pub SEVERANCE_DAYS = "LAB.severance_days_a_year" {
        kind: Policy, decided_by: "parliament", value: Count, clause: "LAB.16", scope: PerCountry
    }
}

declare_prim! {
    /// The least a full-time job may pay, as a share of the mean wage at the opening.
    pub MINIMUM_SHARE = "LAB.minimum_wage_share" {
        kind: Policy, decided_by: "parliament", value: Fixed { exp: 4 }, clause: "LAB.11", scope: PerCountry
    }
}

declare_prim! {
    /// The employees by the years since their job began, as the shares of each band (axis: each band's first year, and
    /// last the end of the last band, which holds no share).
    pub TENURE = "LAB.tenure_shares" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The days a vacancy stands before its employer raises its offer a point.
    pub PATIENCE_DAYS = "LAB.vacancy_patience_days" {
        kind: Preference, value: Count, clause: "LAB.4", scope: Shared
    }
}

declare_prim! {
    /// The applications a searcher sends a week.
    pub APPLICATIONS = "LAB.applications_a_week" {
        kind: Technology, value: Fixed { exp: 2 }, clause: "LAB.16", scope: Shared
    }
}

declare_prim! {
    /// The chance an application is seen by its employer, the meeting of a searcher and a vacancy.
    pub SEEN_CHANCE = "LAB.seen_chance" { kind: Technology, value: Fixed { exp: 4 }, clause: "LAB.16", scope: Shared }
}

declare_prim! {
    /// The weight of a wage's log in a searcher's choice among vacancies and offers, beside its taste.
    pub WAGE_WEIGHT = "LAB.wage_weight" { kind: Preference, value: Fixed { exp: 3 }, clause: "REP.22", scope: Shared }
}

declare_prim! {
    /// A searcher's reservation as a share of its last wage, until households weigh their own.
    pub RESERVATION_SHARE = "LAB.reservation_share" {
        kind: Shape, value: Fixed { exp: 3 }, clause: "LAB.5", scope: Shared, shape: placeholder("HH")
    }
}

declare_prim! {
    /// The years of a start band, within which contracts begun are alike.
    pub BAND_YEARS = "LAB.band_years" { kind: Resolution, value: Count, clause: "REP.3", scope: Shared }
}

declare_prim! {
    /// Each occupation family's least skill level (axis: ISCO-08 major group).
    pub OCCUPATION_SKILL = "LAB.occupation_skill" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "LAB.3", scope: Shared
    }
}

declare_prim! {
    /// The skill level each level of education gives (axis: the education values).
    pub EDUCATION_SKILL = "LAB.education_skill" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "LAB.3", scope: Shared
    }
}

declare_prim! {
    /// The months between an employer's reviews of its contracts' wages.
    pub REVIEW_MONTHS = "LAB.review_months" { kind: Preference, value: Count, clause: "LAB.17", scope: Shared }
}

/// The labour kind, whose rounds the kernel runs.
pub const LABOUR: LabourKind = LabourKind {
    state: STATE.name,
    occupation: OCCUPATION_ATTR.name,
    last_point: LAST_POINT.name,
    education: if_pop::EDUCATION.name,
    taste_stream: TasteStream::DECL.name,
    meeting_stream: MeetingStream::DECL.name,
    lot_stream: LotStream::DECL.name,
    layoff_stream: LayoffStream::DECL.name,
    review_stream: ReviewStream::DECL.name,
    law: law::law,
    search: &points::SEARCH,
    accept: &points::ACCEPT,
    answer: &points::ANSWER,
    offer: &points::OFFER,
    conclude: rules::renegotiate::conclude,
    select: &points::SELECT,
    post: &points::POST,
    wage_at: wages::wage_at,
    point_near: wages::point_near,
    least_point: wages::least_point,
    adapt: wages::adapt,
    owed: wages::severance,
};

/// Labour.
#[derive(Debug)]
pub struct Lab;

impl System for Lab {
    const CODE: &'static str = "LAB";

    fn declare(d: &mut Declarations) {
        for s in [
            JobsStream::DECL,
            TasteStream::DECL,
            MeetingStream::DECL,
            LotStream::DECL,
            LayoffStream::DECL,
            RetirementStream::DECL,
            ReviewStream::DECL,
        ] {
            d.stream(s);
        }
        let _ = Jobs {
            women: d.prim(&WOMEN),
            by_age: d.prim(&EMPLOYMENT_BY_AGE),
            unemployment: d.prim(&UNEMPLOYMENT),
            part_time: d.prim(&PART_TIME),
            part_time_hours: d.prim(&PART_TIME_HOURS),
            tenure: d.prim(&TENURE),
        };
        for p in [&FULL_TIME_HOURS, &NOTICE_DAYS, &SEVERANCE_DAYS, &PATIENCE_DAYS, &BAND_YEARS, &REVIEW_MONTHS] {
            let _: phx_core::Prim<Count> = d.prim(p);
        }
        let _: phx_core::Prim<Fixed<6>> = d.prim(&WAGE_POINT_RATIO);
        let _: phx_core::Prim<phx_core::register::values::Table1> = d.prim(&SELF_EMPLOYED);
        let _: phx_core::Prim<Fixed<4>> = d.prim(&MINIMUM_SHARE);
        let _: phx_core::Prim<Fixed<2>> = d.prim(&APPLICATIONS);
        let _: phx_core::Prim<Fixed<4>> = d.prim(&SEEN_CHANCE);
        let _: phx_core::Prim<Fixed<3>> = d.prim(&WAGE_WEIGHT);
        let _: phx_core::Prim<Fixed<3>> = d.prim(&RESERVATION_SHARE);
        d.decision(&points::SEARCH);
        d.decision(&points::ACCEPT);
        d.decision(&points::RETIRE);
        d.decision(&points::ANSWER);
        d.decision(&points::POST);
        d.decision(&points::SELECT);
        d.decision(&points::OFFER);
        for t in [&OCCUPATION_SKILL, &EDUCATION_SKILL] {
            let _: phx_core::Prim<phx_core::register::values::Table1> = d.prim(t);
        }
        d.pop_kind(if_pop::HOUSEHOLD).person_attr(STATE).person_attr(OCCUPATION_ATTR).person_attr(LAST_POINT);
        d.event(EventKindDecl { name: "LAB.retired", size_unit: "persons", clause: "LAB.6" });
        d.hazard(RETIREMENT);
        d.pop_process(Box::new(retire::Retirement::default()));
        d.market(Box::new(LABOUR));
    }
}
