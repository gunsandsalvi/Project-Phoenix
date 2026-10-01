//! The catalogue over hand-built declaration sets: handles in name order whatever the registration order, every
//! reference resolved, a market's form, days, settlement and participants compiled, and each inconsistency refused.
#![cfg(test)]

use phx_id::Weekday;
use phx_num::Missing;

use super::{
    CapitalDecl, DecisionEntry, Declared, FamilyCode, FamilyCodes, FamilyDecl, FamilyKind, FamilyStatus, FormDecl,
    HazardEntry, KindEntry, LineDecl, MarketDecl, MarketForm, MeetingDays, ProductDecl, ReasonDecl, WayDecl, compile,
};
use crate::kinds::{Feature, Owners, Place};

const COMPANY: FormDecl<'static> = FormDecl {
    system: "FRM",
    name: "company",
    may_hold: &["any"],
    features: &[Feature::SeparateParty, Feature::LimitedLiability, Feature::HasOwners],
    endings: &["insolvency"],
    owners: Owners::Shareholders,
    offices: &["board", "chief_executive"],
};
const HOUSEHOLD: FormDecl<'static> = FormDecl {
    system: "DEM",
    name: "household",
    may_hold: &["LAB.employment", "BNK.household_loan"],
    features: &[],
    endings: &["dissolution"],
    owners: Owners::Members,
    offices: &["head"],
};
const KINDS: [KindEntry<'static>; 3] = [
    KindEntry { system: "FRM", name: "firm", form: "company", place: Place::Site { word: 0 } },
    KindEntry { system: "BNK", name: "bank", form: "company", place: Place::Site { word: 0 } },
    KindEntry { system: "DEM", name: "household", form: "household", place: Place::Sited },
];
const FAMILIES: [FamilyDecl<'static>; 2] = [
    FamilyDecl {
        system: "LAB",
        name: "LAB.employment",
        reason: "wage",
        kinds: &["firm", "household"],
        jobs: true,
        slots: 1,
    },
    FamilyDecl {
        system: "BNK",
        name: "BNK.household_loan",
        reason: "interest",
        kinds: &["bank", "household"],
        jobs: false,
        slots: 1,
    },
];
const LINES: [LineDecl<'static>; 2] =
    [LineDecl { system: "ACC", name: "wages" }, LineDecl { system: "ACC", name: "interest_paid" }];
const REASONS: [ReasonDecl<'static>; 2] = [
    ReasonDecl {
        system: "LAB",
        name: "wage",
        lines: &["wages"],
        payment_order: Missing::Present(1),
        gate: Missing::Absent,
    },
    ReasonDecl {
        system: "BNK",
        name: "interest",
        lines: &["interest_paid"],
        payment_order: Missing::Present(2),
        gate: Missing::Present("sanctions"),
    },
];
const MARKET: MarketDecl<'static> = MarketDecl {
    system: "BNK",
    name: "BNK.loans",
    form: MarketForm::Bilateral,
    operator: "bank",
    days: MeetingDays::Weekly(Weekday::Monday),
    settles_after: 2,
    participants: &["bank", "household"],
    trades: "BNK.household_loan",
    tick: "BNK.loan_tick",
    points: "BNK.loan_points",
    valuer_method: Missing::Present("BNK.loan_method"),
};
const PRIMS: [&str; 3] = ["BNK.loan_tick", "BNK.loan_points", "BNK.loan_method"];

fn row(code: u8, name: &str, status: FamilyStatus) -> FamilyCode {
    FamilyCode { code, name: name.to_owned(), kind: FamilyKind::Contract, step: "S1.260".to_owned(), status }
}

/// The fixtures' two families' rows, made once for every test's declarations to borrow.
fn codes() -> &'static [FamilyCode] {
    let rows =
        vec![row(0, "LAB.employment", FamilyStatus::Declared), row(2, "BNK.household_loan", FamilyStatus::Declared)];
    Box::leak(rows.into_boxed_slice())
}

fn declared<'a>(forms: &'a [FormDecl<'a>], kinds: &'a [KindEntry<'a>], markets: &'a [MarketDecl<'a>]) -> Declared<'a> {
    Declared {
        forms,
        kinds,
        families: &FAMILIES,
        lines: &LINES,
        reasons: &REASONS,
        markets,
        hazards: &[HazardEntry { system: "DEM", name: "DEM.death", acts_on: "household" }],
        decisions: &[DecisionEntry {
            system: "FRM",
            name: "FRM.price_review",
            taker: "firm",
            office: "chief_executive",
        }],
        products: &[
            ProductDecl { system: "GDS", name: "grain", grades: 2 },
            ProductDecl { system: "GDS", name: "bread", grades: 1 },
        ],
        ways: &[WayDecl { system: "TEC", name: "baking", inputs: &[("grain", 1)] }],
        capitals: &[CapitalDecl { system: "CAP", name: "ovens", classes: &["bread", "grain"] }],
        prims: &PRIMS,
        codes: codes(),
    }
}

fn refused(d: &Declared<'_>) -> Vec<String> {
    compile(d).err().unwrap_or_default()
}

#[test]
fn handles_independent_of_order() {
    let forms = [COMPANY, HOUSEHOLD];
    let a = compile(&declared(&forms, &KINDS, &[MARKET])).unwrap();
    let (reversed_forms, mut reversed_kinds) = ([HOUSEHOLD, COMPANY], KINDS);
    reversed_kinds.reverse();
    let b = compile(&declared(&reversed_forms, &reversed_kinds, &[MARKET])).unwrap();
    assert_eq!(a, b, "the same declarations in another order compile alike");
    assert_eq!(a.names.kinds.iter().map(AsRef::as_ref).collect::<Vec<&str>>(), vec!["bank", "firm", "household"]);
}

#[test]
fn every_reference_resolved() {
    let forms = [COMPANY, HOUSEHOLD];
    let c = compile(&declared(&forms, &KINDS, &[MARKET])).unwrap();
    let kind = |n: &str| super::KindH(u16::try_from(c.names.kinds.iter().position(|k| &**k == n).unwrap()).unwrap());
    let family = super::FamilyH(0);
    assert_eq!(&*c.names.families[0], "BNK.household_loan");
    assert_eq!(c.family_kinds(family), &[kind("bank"), kind("household")]);
    assert!(!c.family(family).jobs);
    let reason = c.family(family).reason;
    assert_eq!(&*c.names.reasons[usize::from(reason.get())], "interest");
    assert_eq!(c.reason(reason).payment_order, 2);
    assert!(matches!(c.reason(reason).gate, Missing::Present(_)));
    assert!(c.has(c.kind(kind("firm")).form, Feature::HasOwners));
    assert!(!c.has(c.kind(kind("household")).form, Feature::HasOwners));
    assert_eq!(c.hazard(super::HazardH(0)), kind("household"));
    assert_eq!(c.decision(super::DecisionH(0)).office, 1, "the chief executive, the company's second office");
    let baking = c.way_inputs(super::WayH(0));
    assert_eq!((c.names.products[usize::from(baking[0].product.get())].as_ref(), baking[0].grade), ("grain", 1));
    assert_eq!(c.classes(super::CapitalH(0)).len(), 2);
    assert!(c.bytes() > 0);
}

#[test]
fn market_declares_form_days_convention_participants() {
    let forms = [COMPANY, HOUSEHOLD];
    let c = compile(&declared(&forms, &KINDS, &[MARKET])).unwrap();
    let m = c.market(super::MarketH(0));
    assert_eq!((m.form, m.days, m.settles_after), (MarketForm::Bilateral, MeetingDays::Weekly(Weekday::Monday), 2));
    assert_eq!(c.participants(super::MarketH(0)).len(), 2);
    assert_eq!(&*c.names.kinds[usize::from(m.operator.get())], "bank");
    assert!(matches!(m.valuer_method, Missing::Present(_)));
}

#[test]
fn market_without_participants_refused() {
    let forms = [COMPANY, HOUSEHOLD];
    let lonely = MarketDecl { participants: &[], ..MARKET };
    assert!(refused(&declared(&forms, &KINDS, &[lonely])).iter().any(|e| e.contains("no participants")));
    // A participant whose form may not hold what the market trades.
    let wrong = MarketDecl { trades: "LAB.employment", participants: &["bank"], ..MARKET };
    let forms_narrow = [FormDecl { may_hold: &["BNK.household_loan"], ..COMPANY }, HOUSEHOLD];
    assert!(refused(&declared(&forms_narrow, &KINDS, &[wrong])).iter().any(|e| e.contains("may not trade")));
}

#[test]
fn decision_office_refused() {
    let forms = [COMPANY, HOUSEHOLD];
    let mut d = declared(&forms, &KINDS, &[MARKET]);
    let decisions = [DecisionEntry { system: "HH", name: "HH.spend", taker: "household", office: "board" }];
    d.decisions = &decisions;
    assert!(refused(&d).iter().any(|e| e.contains("office `board`")));
}

#[test]
fn family_capacity_refused() {
    // The household may hold anything here, so no form names a family the test replaces.
    let forms = [COMPANY, FormDecl { may_hold: &["any"], ..HOUSEHOLD }];
    let names: Vec<String> = (0..256).map(|i| format!("F.f{i}")).collect();
    let codes: Vec<FamilyCode> = (0_u8..255).zip(&names).map(|(c, n)| row(c, n, FamilyStatus::Declared)).collect();
    let many: Vec<FamilyDecl<'_>> = names
        .iter()
        .map(|n| FamilyDecl { system: "X", name: n, reason: "wage", kinds: &["firm"], jobs: false, slots: 1 })
        .collect();
    let mut d = declared(&forms, &KINDS, &[]);
    d.families = &many;
    d.codes = &codes;
    assert!(refused(&d).iter().any(|e| e.contains("256 families")));
    d.families = &many[..255];
    assert!(compile(&d).is_ok(), "255 families fit beside holdings' code");
    let wide = [FamilyDecl { slots: crate::consts::FAMILY_SLOTS + 1, ..many[0] }];
    d.families = &wide;
    assert!(refused(&d).iter().any(|e| e.contains("16777217 rows")));
}

#[test]
fn kind_census_refused() {
    let forms = [COMPANY, FormDecl { may_hold: &["any"], ..HOUSEHOLD }];
    let names: Vec<String> = (0..32).map(|i| format!("k{i}")).collect();
    let many: Vec<KindEntry<'_>> =
        names.iter().map(|n| KindEntry { system: "X", name: n, form: "company", place: Place::Sited }).collect();
    let mut d = declared(&forms, &many, &[]);
    d.families = &[];
    d.hazards = &[];
    d.decisions = &[];
    assert!(refused(&d).iter().any(|e| e.contains("32 party kinds")));
    d.kinds = &many[..31];
    assert!(compile(&d).is_ok(), "31 kinds fit beside nature");
}

#[test]
fn reason_gate_compiled() {
    let forms = [COMPANY, HOUSEHOLD];
    let c = compile(&declared(&forms, &KINDS, &[MARKET])).unwrap();
    assert_eq!(c.names.gates.iter().map(AsRef::as_ref).collect::<Vec<&str>>(), vec!["sanctions"]);
    let wage = super::ReasonH(u16::try_from(c.names.reasons.iter().position(|r| &**r == "wage").unwrap()).unwrap());
    assert_eq!(c.reason(wage).gate, Missing::Absent, "a reason with no gate reads none");
    // A reason of no place in the payment order is refused.
    let mut d = declared(&forms, &KINDS, &[MARKET]);
    let reasons = [REASONS[0], ReasonDecl { payment_order: Missing::Absent, ..REASONS[1] }];
    d.reasons = &reasons;
    assert!(refused(&d).iter().any(|e| e.contains("payment order")));
}

#[test]
fn absent_primitive_refused() {
    let forms = [COMPANY, HOUSEHOLD];
    let mut d = declared(&forms, &KINDS, &[MARKET]);
    d.prims = &PRIMS[..2];
    assert!(refused(&d).iter().any(|e| e.contains("primitive `BNK.loan_method`")));
}

#[test]
fn undeclared_references_and_twins_refused() {
    let forms = [COMPANY, HOUSEHOLD, COMPANY];
    let kinds =
        [KINDS[0], KINDS[1], KINDS[2], KindEntry { system: "X", name: "ghost", form: "nowhere", place: Place::Sited }];
    let errors = refused(&declared(&forms, &kinds, &[MARKET]));
    assert!(errors.iter().any(|e| e.contains("legal form `company` declared twice")), "{errors:?}");
    assert!(errors.iter().any(|e| e.contains("legal form `nowhere`")), "{errors:?}");
}

#[test]
fn legal_form_needs_ending() {
    let endless = FormDecl { endings: &[], ..COMPANY };
    let forms = [endless, HOUSEHOLD];
    assert!(refused(&declared(&forms, &KINDS, &[MARKET])).iter().any(|e| e.contains("no way to end")));
    let issuer = FormDecl { endings: &[], features: &[Feature::IssuesCurrency], ..COMPANY };
    let forms = [issuer, HOUSEHOLD];
    assert!(compile(&declared(&forms, &KINDS, &[MARKET])).is_ok(), "a central bank in its own currency");
    let members = FormDecl { owners: Owners::Members, ..COMPANY };
    let forms = [members, HOUSEHOLD];
    assert!(refused(&declared(&forms, &KINDS, &[MARKET])).iter().any(|e| e.contains("its own members")));
}

#[test]
fn family_codes_within_capacity() {
    #[derive(serde::Deserialize)]
    struct File {
        family: Vec<FamilyCode>,
    }
    let file: File = toml::from_str(include_str!("../../../../../data/shared/families.toml")).unwrap();
    let codes = FamilyCodes::new(file.family).unwrap();
    assert!(codes.rows().len() <= crate::consts::FAMILY_CODES);
    assert!(codes.rows().iter().all(|r| r.code < crate::consts::HOLDINGS_CODE), "holdings' code is never a row's");
}

#[test]
fn retired_code_not_reused() {
    let rows = vec![row(9, "SOV.bills", FamilyStatus::Retired), row(9, "SOV.notes", FamilyStatus::Planned)];
    assert_eq!(FamilyCodes::new(rows), Err(vec!["family code 9 of retired `SOV.bills` reissued".to_owned()]));
    // A retired family is not declared again under its old code.
    let forms = [COMPANY, HOUSEHOLD];
    let mut d = declared(&forms, &KINDS, &[MARKET]);
    let codes = [row(0, "LAB.employment", FamilyStatus::Declared), row(2, "BNK.household_loan", FamilyStatus::Retired)];
    d.codes = &codes;
    assert!(refused(&d).iter().any(|e| e.contains("retired; its code 2")));
}

#[test]
fn family_row_required() {
    let forms = [COMPANY, HOUSEHOLD];
    let mut d = declared(&forms, &KINDS, &[MARKET]);
    let codes = [row(0, "LAB.employment", FamilyStatus::Declared)];
    d.codes = &codes;
    assert!(refused(&d).iter().any(|e| e.contains("`BNK.household_loan` has no row")));
    let c = compile(&declared(&forms, &KINDS, &[MARKET])).unwrap();
    assert_eq!(c.family(super::FamilyH(0)).code, 2, "a family's code is its row's, not its place among the declared");
}

#[test]
fn duplicate_code_refused() {
    let rows = vec![row(3, "BNK.firm_loans", FamilyStatus::Declared), row(3, "BNK.firm_loan", FamilyStatus::Planned)];
    assert!(FamilyCodes::new(rows).unwrap_err().iter().any(|e| e.contains("code 3 held by")));
    let twice = vec![row(3, "BNK.firm_loans", FamilyStatus::Declared), row(4, "BNK.firm_loans", FamilyStatus::Planned)];
    assert!(FamilyCodes::new(twice).unwrap_err().iter().any(|e| e.contains("two rows")));
    let holdings = vec![row(crate::consts::HOLDINGS_CODE, "holdings", FamilyStatus::Planned)];
    assert!(FamilyCodes::new(holdings).is_err(), "holdings' code is reserved");
}
