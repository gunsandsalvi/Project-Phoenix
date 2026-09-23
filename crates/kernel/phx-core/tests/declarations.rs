//! Every declaration macro expanded as a system crate writes it, and read back through the vocabulary's types.

use phx_core::handler::{HandlerDecl, IntentDef};
use phx_core::messages::MessageDef;
use phx_core::streams::{Purpose, StreamDef};
use phx_core::{
    DrawScheme, FamilyMode, SubStep, declare_decision, declare_fact, declare_family, declare_handler, declare_hazard,
    declare_message, declare_record, declare_rule, declare_stream,
};

declare_fact! {
    pub Employment = "LAB.employment" {
        value: Flag, kinds: ["household"], writer: "LAB", audience: Party, repr: Key, clause: "LAB.1",
    }
}

declare_fact! {
    pub Wage = "LAB.wage" {
        value: Money, kinds: ["household"], writer: "LAB", audience: PublicAfter(Months(3)), repr: Position,
        clause: "LAB.2",
    }
}

declare_stream! { pub Matching = "LAB.matching" { purpose: Meeting, keyed: false, clause: "CHN.3" } }

#[derive(Debug)]
pub struct Offer(u64);

impl IntentDef for Offer {
    const NAME: &'static str = "LAB.offer";
    fn encode(&self, out: &mut Vec<u64>) {
        out.push(self.0);
    }
}

declare_handler! {
    pub Hire = "LAB.hire" {
        substep: S5c, table: "household", reads: [Employment], writes: [Wage], intents: [Offer], streams: [Matching],
        clause: "LAB.9",
    }
}

declare_message! {
    pub Application = "LAB.application" {
        lives_across_days: false, reaches: ["large_firm"], answering: [("large_firm", "FRM", S5c)],
        acceptance: ("LAB", "LAB.hire"), opens_commitment: false, pins: true, clause: "LAB.4",
    }
}

declare_hazard! {
    pub INJURY = "LAB.injury" {
        acts_on: Role("household", "person"), rate: "LAB.injury_rate", axes: ["LAB.occupation"], outcome: "LAB.injured",
        scheme: Scheduled, stream: "LAB.injury", source: "occupational injury statistics", clause: "CHN.2",
    }
}

fn keep(wage: &[i64; 2]) -> i64 {
    wage[0]
}

declare_decision! {
    pub ASK: [i64; 2] => i64 = "LAB.reservation_wage" {
        rule: keep, schedule: "LAB.quarterly", wakes: [Message], runs_on_non_business: false, clause: "LAB.5",
    }
}

declare_rule! { pub BENEFIT: [i64; 2] => i64 = "SOC.unemployment_benefit" }

declare_record! { pub VACANCIES = "LAB.vacancies" { audience: Public, horizon: Months(24), clause: "OBS.1" } }

declare_family! { pub WAGES = "LAB.wage_flows" { mode: Rolling { cycle_days: 30 }, clause: "LAB.9" } }

#[test]
fn declarations_expand_to_the_vocabulary() {
    assert_eq!((Hire::SUBSTEP, Hire::READS, Hire::WRITES), (SubStep::S5c, &["LAB.employment"][..], &["LAB.wage"][..]));
    assert_eq!((Hire::INTENTS, Hire::STREAMS), (&["LAB.offer"][..], &["LAB.matching"][..]));
    assert_eq!(Matching::DECL.purpose, Purpose::Meeting);
    assert!(Application::DECL.validate().is_ok());
    assert_eq!(INJURY.scheme, DrawScheme::Scheduled { envelope: phx_core::EnvelopeRule::MaxOverProfile });
    assert!(ASK.validate().is_ok() && (ASK.rule)(&[3, 4]) == 3);
    assert_eq!((BENEFIT.name, BENEFIT.implementer), ("SOC.unemployment_benefit", "SOC"));
    assert_eq!((VACANCIES.writer, WAGES.owner, WAGES.mode), ("LAB", "LAB", FamilyMode::Rolling { cycle_days: 30 }));
}
