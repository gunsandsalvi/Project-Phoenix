//! The asked hours' arithmetic over two activities and two occupations written out by hand.
#![cfg(test)]

use super::{Asked, Staffed};

/// A product whose self-employed work a quarter of its 100 and 300 hours, and an agency of 50 and 0 hours.
fn asked() -> Asked {
    Asked {
        staffed: vec![
            Staffed { activity: 0, owned: 0.25, hours: vec![100.0, 300.0] },
            Staffed { activity: 21, owned: 0.0, hours: vec![50.0, 0.0] },
        ],
    }
}

#[test]
fn hours_split_between_employees_and_the_self_employed() {
    assert_eq!(asked().by_occupation(), vec![[125.0, 25.0], [225.0, 75.0]]);
}

#[test]
fn an_activitys_share_of_employees_hours() {
    assert_eq!(asked().employees_share(21), vec![Some(0.4), Some(0.0)]);
    assert_eq!(asked().employees_share(0), vec![Some(0.6), Some(1.0)]);
}

#[test]
fn no_share_where_no_employee_is_asked() {
    let a = Asked { staffed: vec![Staffed { activity: 0, owned: 1.0, hours: vec![10.0] }] };
    assert_eq!(a.employees_share(0), vec![None]);
}

#[test]
fn persons_shared_by_hours() {
    assert_eq!(asked().persons(90.0), vec![(0, 80.0), (21, 10.0)]);
}
