//! LAB, labour: here its opening alone — each adult of a household working as an employee holds a job, a row on the
//! employment line of its wage point, whose employers are the country's firms, apportioned by their headcounts. Its
//! decisions arrive with its own step.

mod consts;
mod jobs;

use phx_core::{Declarations, HandlerTable, StreamDef, System, declare_prim};

pub use jobs::{Declared, EMPLOYMENT, Jobs, JobsStream};

declare_prim! {
    /// The employed by status in employment, by sex: the employees' row read, the others running their own work.
    pub STATUS = "LAB.status_shares" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The ratio between neighbouring wage points, the round numbers wages are paid at, until firms post their own.
    pub WAGE_POINT_RATIO = "LAB.wage_point_ratio" {
        kind: Shape, value: Fixed { exp: 6 }, clause: "REP.34", scope: Shared, shape: placeholder("LAB")
    }
}

/// Labour.
#[derive(Debug)]
pub struct Lab;

impl System for Lab {
    const CODE: &'static str = "LAB";

    fn declare(d: &mut Declarations) {
        d.stream(JobsStream::DECL);
        let jobs = Jobs { status: d.prim(&STATUS), ratio: d.prim(&WAGE_POINT_RATIO) };
        d.contribution(Box::new(Declared));
        let draw: Box<dyn phx_ledger::attachments::AttachmentDraw> = Box::new(jobs);
        d.attachment(Box::new(draw));
    }

    fn handlers(_: &mut HandlerTable) {}
}
