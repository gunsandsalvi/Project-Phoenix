//! The parameter register: every behaviour-shaping number in this world, declared with its kind, its
//! unit and its owner, and read only through here.

use std::cell::RefCell;
use std::collections::HashMap;

/// WHAT KIND OF NUMBER THIS IS — the half of the unit a machine can check.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dimension {
    /// A count of weeks of THIS world's calendar.
    Weeks,
    Years,
    /// A count of things: people, entries, instructions, contracts, machines.
    Count,
    /// A pure share of something, dimensionless.
    Ratio,
    /// A rate per year.
    PerAnnum,
    /// A distance over the ground, and a speed over it.
    Km,
    KmPerDay,
    /// Ground covered.
    SquareKm,
    /// Money for one PIECE of something, at this world's resolution.
    Price,
    /// Money for one NAMED unit: a dollar a tonne, a wage an hour.
    PricePerUnit,
    /// A declared AMOUNT of a unit the reader names.
    Amount(Denomination),
}

/// Which KIND of unit a reader may name for a declared amount, so a number meant as dollars can be
/// measured against this world's dollars.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Denomination {
    /// An amount of whatever money the party reading it deals in.
    Money,
    /// An amount of somebody's time.
    Time,
}

/// WHO IN THE WORLD DECIDES A NUMBER. A number nobody here decides has no owner, and saying so is
/// what stops a claim the model made from passing as somebody's decision.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Owner {
    Parliament,
    CentralBank,
    StandardSetter,
    Constitution,
}

/// Law 2's closed list.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Kind {
    Technology,
    Preference,
    Policy,
    /// Tested by invariance: the world's path must not turn on it.
    Resolution,
    /// A claim about the answer.
    Shape,
    /// A shape with a scheduled death, naming the MECHANISM whose absence it stands in for. What
    /// will build that mechanism is not its business: a number that pointed into a plan would be
    /// wrong the week after the plan moved.
    Placeholder {
        mechanism: String,
    },
}

pub struct ParamDecl {
    pub id: String,
    /// The value AS A PERSON DECLARES IT.
    pub value: f64,
    /// Prose, for a reader.
    pub unit: String,
    pub dimension: Dimension,
    pub kind: Kind,
    pub owner: Option<Owner>,
    pub why: String,
}

/// One declared parameter exactly as a run consumed it.
#[derive(Clone, PartialEq, Debug)]
pub struct ParamSnapshot {
    pub id: String,
    pub value: f64,
    pub unit: String,
    pub dimension: Dimension,
    pub kind: Kind,
    pub owner: Option<Owner>,
    pub why: String,
}

#[derive(Default)]
pub struct Params {
    by_id: HashMap<String, usize>,
    value: Vec<f64>,
    dimension: Vec<Dimension>,
    kind: Vec<Kind>,
    owner: Vec<Option<Owner>>,
    unit: Vec<String>,
    why: Vec<String>,
    read: RefCell<Vec<bool>>,
    /// The world's quantity grid: how many indivisible pieces one named unit is counted in.
    pieces_per_unit: HashMap<Denomination, f64>,
}

impl Params {
    pub fn new(money_pieces: f64, time_pieces: f64) -> Self {
        let mut pieces_per_unit = HashMap::new();
        pieces_per_unit.insert(Denomination::Money, money_pieces);
        pieces_per_unit.insert(Denomination::Time, time_pieces);
        Self {
            pieces_per_unit,
            ..Default::default()
        }
    }

    /// One fact, one writer.
    pub fn declare(&mut self, d: ParamDecl) {
        assert!(
            !self.by_id.contains_key(&d.id),
            "Law 4: {} is declared twice",
            d.id
        );
        assert!(d.value.is_finite(), "Law 6: {} is not a number", d.id);
        assert!(
            !d.unit.is_empty(),
            "Law 8: {} is declared with no unit",
            d.id
        );
        assert!(
            !d.why.is_empty(),
            "Law 16: {} is declared with no reason",
            d.id
        );
        // A count of things is a whole one.
        if matches!(
            d.dimension,
            Dimension::Weeks | Dimension::Years | Dimension::Count
        ) {
            assert!(
                d.value.fract() == 0.0,
                "Law 8: {} is {:?} and {} is not a whole one",
                d.id,
                d.dimension,
                d.value
            );
        }
        assert!(
            !matches!(d.kind, Kind::Policy) || d.owner.is_some(),
            "Appendix B: {} is a policy, and a policy nobody sets is one nobody can change",
            d.id
        );
        self.by_id.insert(d.id.clone(), self.value.len());
        self.value.push(d.value);
        self.dimension.push(d.dimension);
        self.kind.push(d.kind);
        self.owner.push(d.owner);
        self.unit.push(d.unit);
        self.why.push(d.why);
        self.read.get_mut().push(false);
    }

    fn at(&self, id: &str) -> usize {
        match self.by_id.get(id) {
            Some(&at) => at,
            None => {
                panic!("XI-14: {id} is not declared — the engine reads numbers only via params")
            }
        }
    }

    /// Every read NAMES what it expects, and a read that names the wrong one says both.
    fn read(&self, id: &str, want: Dimension) -> f64 {
        let at = self.at(id);
        let got = self.dimension[at];
        assert!(
            got == want,
            "Law 8: {id} is declared in {got:?} and was read as {want:?}"
        );
        self.read.borrow_mut()[at] = true;
        self.value[at]
    }

    pub fn weeks(&self, id: &str) -> f64 {
        self.read(id, Dimension::Weeks)
    }
    pub fn days(&self, id: &str) -> f64 {
        self.read(id, Dimension::Weeks)
    }
    pub fn years(&self, id: &str) -> f64 {
        self.read(id, Dimension::Years)
    }
    pub fn count(&self, id: &str) -> f64 {
        self.read(id, Dimension::Count)
    }
    pub fn ratio(&self, id: &str) -> f64 {
        self.read(id, Dimension::Ratio)
    }
    pub fn per_annum(&self, id: &str) -> f64 {
        self.read(id, Dimension::PerAnnum)
    }
    pub fn square_km(&self, id: &str) -> f64 {
        self.read(id, Dimension::SquareKm)
    }
    pub fn price(&self, id: &str) -> f64 {
        self.read(id, Dimension::Price)
    }
    pub fn price_per_unit(&self, id: &str) -> f64 {
        self.read(id, Dimension::PricePerUnit)
    }

    /// A declared AMOUNT is a named amount of a unit the reader names, and what comes back is the
    /// count of PIECES the state holds.
    pub fn amount(&self, id: &str, of: Denomination) -> f64 {
        let value = self.read(id, Dimension::Amount(of));
        value * self.pieces_per_unit[&of]
    }

    pub fn kind_of(&self, id: &str) -> &Kind {
        &self.kind[self.at(id)]
    }

    pub fn owner_of(&self, id: &str) -> Option<Owner> {
        self.owner[self.at(id)]
    }

    pub fn len(&self) -> usize {
        self.value.len()
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Whether construction declared this input, without consuming its value.
    pub fn declared(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    /// The declarations this run actually consumed, with their provenance, in stable id order.
    pub fn consumed(&self) -> Vec<ParamSnapshot> {
        let read = self.read.borrow();
        let mut out: Vec<ParamSnapshot> = self
            .by_id
            .iter()
            .filter(|(_, at)| read[**at])
            .map(|(id, &at)| ParamSnapshot {
                id: id.clone(),
                value: self.value[at],
                unit: self.unit[at].clone(),
                dimension: self.dimension[at],
                kind: self.kind[at].clone(),
                owner: self.owner[at],
                why: self.why[at].clone(),
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// The SHAPES, and the placeholders among them with what they stand in for.
    pub fn shapes(&self) -> Vec<(&str, &Kind)> {
        let mut out: Vec<(&str, &Kind)> = Vec::new();
        for (id, &at) in &self.by_id {
            match &self.kind[at] {
                Kind::Shape | Kind::Placeholder { .. } => out.push((id.as_str(), &self.kind[at])),
                _ => {}
            }
        }
        out.sort_by(|a, b| a.0.cmp(b.0));
        out
    }

    /// RESOLUTION, tested by invariance: shifting the grid must not change the world's path.
    pub fn piece_shift(&mut self, by: f64) {
        assert!(by > 0.0, "Law 6: a grid of {by} pieces is not a grid");
        for v in self.pieces_per_unit.values_mut() {
            *v *= by;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> Params {
        Params::new(100.0, 60.0)
    }

    fn decl(id: &str, value: f64, dimension: Dimension, kind: Kind) -> ParamDecl {
        ParamDecl {
            id: id.to_string(),
            value,
            unit: "stated".to_string(),
            dimension,
            kind,
            owner: Some(Owner::StandardSetter),
            why: "because the test says so".to_string(),
        }
    }

    #[test]
    fn a_read_that_names_the_wrong_dimension_says_both() {
        let mut p = params();
        p.declare(decl("loan.term", 7.0, Dimension::Years, Kind::Technology));
        let bad = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| p.weeks("loan.term")));
        let msg = *bad.unwrap_err().downcast::<String>().unwrap();
        assert!(msg.contains("Years"), "{msg}");
        assert!(msg.contains("Weeks"), "{msg}");
    }

    #[test]
    fn a_placeholder_carries_its_death_and_nothing_else_can() {
        let mut p = params();
        p.declare(decl(
            "recovery.rate",
            0.4,
            Dimension::Ratio,
            Kind::Placeholder {
                mechanism: "the recovery a defaulted claim actually fetches".to_string(),
            },
        ));
        p.declare(decl(
            "bank.cushion",
            0.05,
            Dimension::Ratio,
            Kind::Preference,
        ));
        // The count of claims about the answer is what must fall, and it is a read.
        assert_eq!(p.shapes().len(), 1);
        match p.kind_of("recovery.rate") {
            Kind::Placeholder { mechanism } => {
                assert_eq!(mechanism, "the recovery a defaulted claim actually fetches");
            }
            other => panic!("{other:?}"),
        }
        // And a preference has no field to put a death in: it is not a guard, it is the type.
    }

    #[test]
    fn the_run_records_only_parameters_it_actually_consumed() {
        let mut p = params();
        p.declare(decl("used", 0.4, Dimension::Ratio, Kind::Preference));
        p.declare(decl("unused", 0.6, Dimension::Ratio, Kind::Preference));

        assert!(p.consumed().is_empty());
        assert_eq!(p.ratio("used"), 0.4);
        let consumed = p.consumed();
        assert_eq!(consumed.len(), 1);
        assert_eq!(consumed[0].id, "used");
        assert_eq!(consumed[0].value, 0.4);
        assert_eq!(consumed[0].owner, Some(Owner::StandardSetter));
        assert_eq!(consumed[0].unit, "stated");
    }

    #[test]
    fn a_declared_amount_moves_with_the_worlds_resolution() {
        let mut p = params();
        p.declare(decl(
            "wage.floor",
            30_000.0,
            Dimension::Amount(Denomination::Money),
            Kind::Policy,
        ));
        // A hundred pieces to the unit: thirty thousand is three million of them.
        assert_eq!(p.amount("wage.floor", Denomination::Money), 3_000_000.0);
        // Shift the grid and the SAME declaration answers in the new pieces.
        p.piece_shift(10.0);
        assert_eq!(p.amount("wage.floor", Denomination::Money), 30_000_000.0);
    }

    #[test]
    #[should_panic(expected = "is declared twice")]
    fn one_number_has_one_writer() {
        let mut p = params();
        p.declare(decl("x", 1.0, Dimension::Ratio, Kind::Preference));
        p.declare(decl("x", 2.0, Dimension::Ratio, Kind::Preference));
    }

    #[test]
    #[should_panic(expected = "the engine reads numbers only via params")]
    fn a_number_nobody_declared_cannot_be_read() {
        let p = params();
        p.ratio("nobody.declared.this");
    }

    #[test]
    #[should_panic(expected = "is not a whole one")]
    fn a_count_of_things_is_a_whole_one() {
        let mut p = params();
        p.declare(decl("desks", 3.5, Dimension::Count, Kind::Technology));
    }
}
