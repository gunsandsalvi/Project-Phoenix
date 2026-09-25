//! A household as the processes change it: its persons' roles, a person taking another role, and the head's place
//! taken when it falls empty.

use if_pop::{ADULT, CHILD, HEAD, PARTNER};
use phx_core::Household;
use phx_num::violation;

/// A person's role: the head, the partner, another adult, or a child under the age of majority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Head,
    Partner,
    Adult,
    Child,
}

impl Role {
    pub(crate) fn of(name: &str) -> Role {
        match name {
            n if n == HEAD.name => Role::Head,
            n if n == PARTNER.name => Role::Partner,
            n if n == ADULT.name => Role::Adult,
            n if n == CHILD.name => Role::Child,
            _ => violation!(clause = "REP.26", "a role the household does not hold"),
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Role::Head => HEAD.name,
            Role::Partner => PARTNER.name,
            Role::Adult => ADULT.name,
            Role::Child => CHILD.name,
        }
    }
}

/// A household whose head has gone takes the partner, else the eldest other adult, else the eldest child as its
/// head, the eldest the earliest born and the first of those. A household no one is left in keeps no head.
pub(crate) fn succeed(h: &mut Household) {
    if h.present().any(|(_, p)| Role::of(p.role) == Role::Head) {
        return;
    }
    let eldest = |h: &Household, wanted: Role| {
        let mut best: Option<(usize, phx_id::Date)> = None;
        for (i, p) in h.present().filter(|(_, p)| Role::of(p.role) == wanted) {
            if best.is_none_or(|(_, b)| p.born < b) {
                best = Some((i, p.born));
            }
        }
        best.map(|(i, _)| i)
    };
    let next = eldest(h, Role::Partner).or_else(|| eldest(h, Role::Adult)).or_else(|| eldest(h, Role::Child));
    let Some(i) = next else { return };
    let Some(p) = h.persons.get_mut(i) else { violation!(clause = "REP.26", "a successor beyond the household") };
    p.role = Role::Head.name();
}

#[cfg(test)]
mod tests {
    use if_pop::REGION;
    use phx_core::{Household, Person};
    use phx_id::Date;

    use super::{Role, succeed};

    fn person(role: Role, year: i32) -> Person {
        Person { role: role.name(), born: Date::new(year, 5, 1).unwrap(), attrs: Vec::new(), gone: false }
    }

    fn household(persons: Vec<Person>) -> Household {
        Household { attrs: vec![(REGION.name, 3)], persons }
    }

    #[test]
    fn roles_read_back_from_their_names() {
        for role in [Role::Head, Role::Partner, Role::Adult, Role::Child] {
            assert_eq!(Role::of(role.name()), role);
        }
    }

    /// The partner succeeds a dead head, else the eldest adult, else the eldest child.
    #[test]
    fn the_head_is_succeeded_in_order() {
        let mut h = household(vec![
            person(Role::Head, 1960),
            person(Role::Partner, 1962),
            person(Role::Adult, 1990),
            person(Role::Child, 2015),
        ]);
        let kill_head = |h: &mut Household| {
            let (i, _) = h.present().find(|(_, p)| p.role == "head").unwrap();
            h.persons[i].gone = true;
            succeed(h);
        };
        kill_head(&mut h);
        assert_eq!(h.persons[1].role, "head", "the partner");
        kill_head(&mut h);
        assert_eq!(h.persons[2].role, "head", "the adult");
        kill_head(&mut h);
        assert_eq!(h.persons[3].role, "head", "the child");
        kill_head(&mut h);
        assert_eq!(h.present().count(), 0, "no one left, no head");
    }

    #[test]
    fn the_eldest_of_two_adults_succeeds() {
        let mut h = household(vec![person(Role::Head, 1950), person(Role::Adult, 1980), person(Role::Adult, 1975)]);
        h.persons[0].gone = true;
        succeed(&mut h);
        assert_eq!(h.persons[2].role, "head");
    }
}
