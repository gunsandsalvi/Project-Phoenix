//! The industry each opening firm is drawn in, given its size: the firms of a size class spread over the industries
//! as the country's data declares, so large firms lean to the industries whose firms are large.

use phx_core::register::values::Table2;
use phx_macros::clause;
use phx_num::violation;
use phx_rand::float::from_i64;
use phx_rand::{AliasTable, Draws};

/// Each size class, by the smallest size it holds, with its firms' spread over the industries.
#[derive(Clone, Debug)]
pub struct Industries {
    classes: Vec<(i64, AliasTable)>,
}

impl Industries {
    /// The classes of a table whose rows are the classes' smallest sizes, rising, and whose columns are the
    /// industries in order.
    ///
    /// # Errors
    /// What `from_rows` refuses, and columns that are not the industries in order.
    #[clause("GEN.2", "FRM.2")]
    pub fn of(table: &Table2) -> Result<Industries, String> {
        if table.columns().iter().zip(0_i64..).any(|(c, i)| *c != i) {
            return Err("the columns are not the industries in order".to_owned());
        }
        let mut rows = Vec::with_capacity(table.rows().len());
        for row in table.rows() {
            let shares = table
                .columns()
                .iter()
                .map(|c| table.at(*row, *c).map_err(|_| format!("no share at size {row}, industry {c}")))
                .collect::<Result<Vec<i64>, String>>()?;
            rows.push((*row, shares));
        }
        Industries::from_rows(&rows)
    }

    /// The classes from each class's smallest size and its shares over the industries.
    ///
    /// # Errors
    /// No class, classes not rising, a share below nothing, or a class with no firm.
    pub fn from_rows(rows: &[(i64, Vec<i64>)]) -> Result<Industries, String> {
        if rows.is_empty() || rows.windows(2).any(|w| matches!(w, [(a, _), (b, _)] if a >= b)) {
            return Err("the size classes are not rising".to_owned());
        }
        let mut classes = Vec::with_capacity(rows.len());
        for (least, shares) in rows {
            if shares.iter().any(|v| *v < 0) {
                return Err(format!("a share below nothing in the class from size {least}"));
            }
            if shares.iter().all(|v| *v == 0) {
                return Err(format!("the class from size {least} holds no firm"));
            }
            let weights: Vec<f64> = shares.iter().map(|v| from_i64(*v)).collect();
            classes.push((*least, AliasTable::new(&weights)));
        }
        Ok(Industries { classes })
    }

    /// The industry of a firm of `size` persons, drawn from its class.
    #[must_use]
    pub fn draw(&self, size: u64, lot: &mut Draws) -> u16 {
        let Some(class) = class_of(self.classes.iter().map(|(least, _)| *least), size) else {
            violation!(clause = "GEN.2", "a firm smaller than the smallest size class", size = size);
        };
        let Some((_, table)) = self.classes.get(class) else {
            violation!(clause = "GEN.2", "a size class beyond the classes", class = class);
        };
        let Ok(industry) = u16::try_from(table.draw(lot)) else {
            violation!(clause = "GEN.2", "an industry beyond an industry's identity");
        };
        industry
    }
}

/// The class a size falls in: the last whose smallest size it reaches.
fn class_of(least: impl Iterator<Item = i64>, size: u64) -> Option<usize> {
    least.enumerate().filter(|(_, l)| u64::try_from(*l).is_ok_and(|l| l <= size)).map(|(i, _)| i).last()
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Industries, class_of};

    #[test]
    fn a_size_falls_in_the_last_class_it_reaches() {
        let least = [1_i64, 10, 50, 250];
        assert_eq!(class_of(least.into_iter(), 0), None);
        assert_eq!(class_of(least.into_iter(), 9), Some(0));
        assert_eq!(class_of(least.into_iter(), 10), Some(1));
        assert_eq!(class_of(least.into_iter(), 100_000), Some(3));
    }

    #[test]
    fn an_industry_with_no_share_in_a_class_is_never_drawn_there() {
        let t = Industries::from_rows(&[(1, vec![1, 0]), (10, vec![0, 1])]).unwrap();
        let mut lot = Draws::new(stream_key(Seed::new(5), "industry"), Subject::new(SubjectTag::World, 0), 0, 0);
        for _ in 0..100 {
            assert_eq!(t.draw(9, &mut lot), 0);
            assert_eq!(t.draw(10, &mut lot), 1);
        }
    }

    #[test]
    fn a_table_that_is_no_spread_is_refused() {
        assert!(Industries::from_rows(&[]).is_err());
        assert!(Industries::from_rows(&[(10, vec![1]), (1, vec![1])]).is_err());
        assert!(Industries::from_rows(&[(1, vec![0, 0])]).is_err());
        assert!(Industries::from_rows(&[(1, vec![1, -1])]).is_err());
    }
}
