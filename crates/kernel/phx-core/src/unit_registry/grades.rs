//! Each grade's content per unit — the metal in a tonne of ore, the heat in a tonne of coal — in a content unit, so
//! a way stating a graded input in content reads the units of a grade it needs; content is never held per holding.

use phx_macros::{clause, opening};
use phx_num::{Fixed, Missing, UnitId, violation};

use crate::catalogue::{Catalogue, GradeH};
use crate::consts::CONTENT_PLACES;

/// A grade's content per unit: how much of its content unit one unit of the grade holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Content {
    pub unit: UnitId,
    pub per_unit: Fixed<CONTENT_PLACES>,
}

/// Every grade's content, by product then grade, each product's grades one run; a grade declares its content once.
#[clause("GDS.1", "GDS.13")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GradeContents {
    starts: Vec<usize>,
    contents: Vec<Missing<Content>>,
}

impl GradeContents {
    /// A run for each of the catalogue's products, as many as its grades, none declared yet.
    #[opening]
    #[must_use]
    pub fn new(catalogue: &Catalogue) -> GradeContents {
        let mut starts = Vec::new();
        let mut total = 0;
        for p in catalogue.products() {
            starts.push(total);
            total += usize::from(catalogue.grades(p));
        }
        starts.push(total);
        GradeContents { starts, contents: vec![Missing::Absent; total] }
    }

    /// A grade's place among the contents; a grade its product has not stops the run.
    fn at(&self, grade: GradeH) -> usize {
        let p = usize::from(grade.product.get());
        match (self.starts.get(p), self.starts.get(p + 1)) {
            (Some(start), Some(end)) if start + usize::from(grade.grade) < *end => start + usize::from(grade.grade),
            _ => violation!(clause = "GDS.1", "a grade its product has not", grade = grade.grade),
        }
    }

    /// A grade's content declared; declared twice is a fact with two writers and stops the run.
    #[opening]
    pub fn declare(&mut self, grade: GradeH, content: Content) {
        let at = self.at(grade);
        match self.contents.get_mut(at) {
            Some(slot @ Missing::Absent) => *slot = Missing::Present(content),
            _ => violation!(clause = "GDS.13", "a grade's content declared twice", grade = grade.grade),
        }
    }

    /// A grade's content per unit, absent where its grade declares none.
    pub fn content(&self, grade: GradeH) -> Missing<Content> {
        match self.contents.get(self.at(grade)) {
            Some(c) => *c,
            None => violation!(clause = "GDS.1", "a grade past the contents", grade = grade.grade),
        }
    }
}
