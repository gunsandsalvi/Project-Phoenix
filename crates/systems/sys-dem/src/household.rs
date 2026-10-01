//! A household as the processes change it: the head's place taken when it falls empty.

use if_pop::{ADULT, CHILD, HEAD, PARTNER};
use phx_core::Household;
use phx_core::person_word::ROLE;
use phx_num::violation;

/// A household whose head has gone takes the partner, else the eldest other adult, else the eldest child as its
/// head, the eldest the earliest born and the first of those. A household no one is left in keeps no head.
pub(crate) fn succeed(h: &mut Household) {
    if h.present().any(|(_, p)| p.role() == HEAD.value) {
        return;
    }
    let eldest = |h: &Household, wanted: u32| {
        let mut best: Option<(usize, phx_id::Date)> = None;
        for (i, p) in h.present().filter(|(_, p)| p.role() == wanted) {
            if best.is_none_or(|(_, b)| p.born() < b) {
                best = Some((i, p.born()));
            }
        }
        best.map(|(i, _)| i)
    };
    let next = eldest(h, PARTNER.value).or_else(|| eldest(h, ADULT.value)).or_else(|| eldest(h, CHILD.value));
    let Some(i) = next else { return };
    let Some(p) = h.persons.get_mut(i) else { violation!(clause = "REP.26", "a successor beyond the household") };
    p.set(ROLE, HEAD.value);
}

#[cfg(test)]
mod tests {
    use if_pop::{ADULT, CHILD, HEAD, PARTNER};
    use phx_core::person_word::PersonWord;
    use phx_core::{Household, HouseholdState, Person};
    use phx_id::Date;

    use super::succeed;

    fn person(role: u32, year: i32) -> Person {
        Person::of(PersonWord::new(Date::new(year, 5, 1).unwrap(), role))
    }

    fn household(persons: Vec<Person>) -> Household {
        Household { state: HouseholdState::formed(3), persons }
    }

    /// The partner succeeds a dead head, else the eldest adult, else the eldest child.
    #[test]
    fn the_head_is_succeeded_in_order() {
        let mut h = household(vec![
            person(HEAD.value, 1960),
            person(PARTNER.value, 1962),
            person(ADULT.value, 1990),
            person(CHILD.value, 2015),
        ]);
        let kill_head = |h: &mut Household| {
            let (i, _) = h.present().find(|(_, p)| p.role() == HEAD.value).unwrap();
            h.persons[i].gone = true;
            succeed(h);
        };
        kill_head(&mut h);
        assert_eq!(h.persons[1].role(), HEAD.value, "the partner");
        kill_head(&mut h);
        assert_eq!(h.persons[2].role(), HEAD.value, "the adult");
        kill_head(&mut h);
        assert_eq!(h.persons[3].role(), HEAD.value, "the child");
        kill_head(&mut h);
        assert_eq!(h.present().count(), 0, "no one left, no head");
    }

    #[test]
    fn the_eldest_of_two_adults_succeeds() {
        let mut h = household(vec![person(HEAD.value, 1950), person(ADULT.value, 1980), person(ADULT.value, 1975)]);
        h.persons[0].gone = true;
        succeed(&mut h);
        assert_eq!(h.persons[2].role(), HEAD.value);
    }
}
