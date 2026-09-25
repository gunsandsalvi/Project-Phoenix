//! A household as the processes change it: its persons' roles, a person taking another role with the values that
//! role holds, the head's place taken when it falls empty, and the key's counts read back from the persons.

use if_pop::{
    ADULT, ADULT_COUNT, ADULT_GROUPS, BIRTH_YEAR_AT, CHILD_COUNTS, CHILD_GROUPS, CHILDREN, EDUCATION_UNRECORDED, HEAD,
    HEAD_AGE, LIFE, PARTNER, PARTNERS,
};
use phx_core::{Household, Person, component};
use phx_num::violation;
use phx_rand::float::index;

/// A person's role: the head, the partner, another adult, or a child of an age band.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Head,
    Partner,
    Adult,
    Child(u32),
}

impl Role {
    pub(crate) fn of(name: &str) -> Role {
        if name == HEAD.name {
            return Role::Head;
        }
        if name == PARTNER.name {
            return Role::Partner;
        }
        if name == ADULT.name {
            return Role::Adult;
        }
        let Some(b) = CHILDREN.iter().position(|r| r.name == name).and_then(|b| u32::try_from(b).ok()) else {
            violation!(clause = "REP.26", "a role the household does not hold");
        };
        Role::Child(b)
    }

    /// The role a life group belongs to.
    pub(crate) fn of_group(group: &str) -> Role {
        if let Some(i) = ADULT_GROUPS.iter().position(|(life, _)| life.name == group) {
            return [Role::Head, Role::Partner, Role::Adult].get(i).copied().unwrap_or_else(|| {
                violation!(clause = "REP.26", "an adult life group of no role", group = i);
            });
        }
        let Some(b) = CHILD_GROUPS.iter().position(|g| g.name == group).and_then(|b| u32::try_from(b).ok()) else {
            violation!(clause = "REP.32", "a life group of no role the household holds");
        };
        Role::Child(b)
    }

    fn name(self) -> &'static str {
        match self {
            Role::Head => HEAD.name,
            Role::Partner => PARTNER.name,
            Role::Adult => ADULT.name,
            Role::Child(b) => at(CHILDREN, b).name,
        }
    }

    /// The role's life group, and its schooling group if it is an adult's.
    fn groups(self) -> (&'static str, Option<&'static str>) {
        let adult = |i: u32| {
            let (life, schooling) = at(ADULT_GROUPS, i);
            (life.name, Some(schooling.name))
        };
        match self {
            Role::Head => adult(0),
            Role::Partner => adult(1),
            Role::Adult => adult(2),
            Role::Child(b) => (at(CHILD_GROUPS, b).name, None),
        }
    }

    pub(crate) fn life_group(self) -> &'static str {
        self.groups().0
    }
}

fn at<T: Copy>(list: &[T], i: u32) -> T {
    let Some(x) = list.get(index(u64::from(i))) else {
        violation!(clause = "REP.26", "an age band beyond the household's roles", band = i);
    };
    *x
}

/// A person's life value: birth year, sex and health, joint.
pub(crate) fn life(p: &Person) -> u32 {
    let Some(v) = p.value(Role::of(p.role).life_group()) else {
        violation!(clause = "REP.32", "a person without its role's life value");
    };
    v
}

/// A person moved to another role: its life value carried, its schooling carried between adult roles, and an adult
/// who was a child holding its schooling as not yet recorded.
pub(crate) fn take_role(p: &mut Person, to: Role) {
    let schooling = match Role::of(p.role).groups() {
        (_, Some(g)) => {
            let Some(v) = p.value(g) else { violation!(clause = "REP.32", "an adult without its schooling") };
            v
        }
        (_, None) => EDUCATION_UNRECORDED,
    };
    let (life_to, schooling_to) = to.groups();
    let mut values = vec![(life_to, life(p))];
    if let Some(g) = schooling_to {
        values.push((g, schooling));
    }
    *p = Person { role: to.name(), values, gone: p.gone };
}

/// The household's key counts read from its persons still there: the partner, the other adults and the children of
/// each band.
pub(crate) fn settle(h: &mut Household) {
    let count = |h: &Household, role: Role| {
        let n = h.present().filter(|(_, p)| Role::of(p.role) == role).count();
        let Ok(n) = u32::try_from(n) else { violation!(clause = "REP.26", "persons of a role beyond counting") };
        n
    };
    let (partners, adults) = (count(h, Role::Partner), count(h, Role::Adult));
    h.set_attr(PARTNERS.name, partners);
    h.set_attr(ADULT_COUNT.name, adults);
    for (b, attr) in (0_u32..).zip(CHILD_COUNTS) {
        let n = count(h, Role::Child(b));
        h.set_attr(attr.name, n);
    }
}

/// A household whose head has gone takes the partner, else the eldest other adult, else the eldest child as its
/// head, the eldest the earliest born and the first of those; `class` gives the new head's age class. A household no
/// one is left in keeps no head.
pub(crate) fn succeed(h: &mut Household, class: impl FnOnce(&Person) -> u32) {
    if h.present().any(|(_, p)| Role::of(p.role) == Role::Head) {
        return;
    }
    let eldest = |h: &Household, wanted: &dyn Fn(Role) -> bool| {
        let mut best: Option<(usize, u32)> = None;
        for (i, p) in h.present().filter(|(_, p)| wanted(Role::of(p.role))) {
            let born = component(LIFE, life(p), BIRTH_YEAR_AT);
            if best.is_none_or(|(_, b)| born < b) {
                best = Some((i, born));
            }
        }
        best.map(|(i, _)| i)
    };
    let next = eldest(h, &|r| r == Role::Partner)
        .or_else(|| eldest(h, &|r| r == Role::Adult))
        .or_else(|| eldest(h, &|r| matches!(r, Role::Child(_))));
    let Some(i) = next else { return };
    let Some(p) = h.persons.get_mut(i) else { violation!(clause = "REP.26", "a successor beyond the household") };
    let head_class = class(p);
    take_role(p, Role::Head);
    h.set_attr(HEAD_AGE.name, head_class);
}

#[cfg(test)]
mod tests {
    use if_pop::{ADULT_COUNT, CHILD_COUNTS, EDUCATION_UNRECORDED, HEAD_AGE, LIFE, PARTNERS, REGION};
    use phx_core::{Household, Person, joint};

    use super::{Role, life, settle, succeed, take_role};

    fn person(role: Role, born: u32) -> Person {
        let (life_group, schooling) = role.groups();
        let mut values = vec![(life_group, joint(LIFE, &[born, 1, 0]))];
        if let Some(g) = schooling {
            values.push((g, 4));
        }
        Person { role: role.name(), values, gone: false }
    }

    fn household(persons: Vec<Person>) -> Household {
        let mut key = vec![(REGION.name, 3), (HEAD_AGE.name, 5), (PARTNERS.name, 0), (ADULT_COUNT.name, 0)];
        key.extend(CHILD_COUNTS.iter().map(|a| (a.name, 0)));
        let mut h = Household { key, persons };
        settle(&mut h);
        h
    }

    #[test]
    fn roles_name_their_groups_and_back() {
        for role in [Role::Head, Role::Partner, Role::Adult, Role::Child(0), Role::Child(7)] {
            assert_eq!(Role::of(role.name()), role);
            assert_eq!(Role::of_group(role.life_group()), role);
        }
    }

    /// A child who becomes an adult keeps its life value and holds its schooling as not yet recorded; an adult who
    /// becomes the head keeps its schooling.
    #[test]
    fn a_person_takes_a_role_with_its_values() {
        let mut child = person(Role::Child(2), 108);
        let born = life(&child);
        take_role(&mut child, Role::Adult);
        assert_eq!(child.role, "adult");
        assert_eq!(child.values, [("DEM.adult_life", born), ("DEM.adult_schooling", EDUCATION_UNRECORDED)]);
        let mut adult = person(Role::Adult, 70);
        take_role(&mut adult, Role::Head);
        assert_eq!(adult.value("DEM.head_schooling"), Some(4));
    }

    /// The partner succeeds a dead head, else the eldest adult, else the eldest child; the key counts follow.
    #[test]
    fn the_head_is_succeeded_in_order() {
        let mut h = household(vec![
            person(Role::Head, 60),
            person(Role::Partner, 62),
            person(Role::Adult, 90),
            person(Role::Child(1), 115),
        ]);
        assert_eq!((h.key(PARTNERS.name), h.key(ADULT_COUNT.name), h.key("DEM.children_1")), (1, 1, 1));
        let kill_head = |h: &mut Household| {
            let (i, _) = h.present().find(|(_, p)| p.role == "head").unwrap();
            h.persons[i].gone = true;
            succeed(h, |_| 6);
            settle(h);
        };
        kill_head(&mut h);
        assert_eq!(h.persons[1].role, "head", "the partner");
        assert_eq!((h.key(HEAD_AGE.name), h.key(PARTNERS.name)), (6, 0));
        kill_head(&mut h);
        assert_eq!(h.persons[2].role, "head", "the adult");
        assert_eq!(h.key(ADULT_COUNT.name), 0);
        kill_head(&mut h);
        assert_eq!(h.persons[3].role, "head", "the child, its schooling not yet recorded");
        assert_eq!(h.persons[3].value("DEM.head_schooling"), Some(EDUCATION_UNRECORDED));
        assert_eq!(h.key("DEM.children_1"), 0);
        kill_head(&mut h);
        assert_eq!(h.present().count(), 0, "no one left, no head");
    }

    #[test]
    fn the_eldest_of_two_adults_succeeds() {
        let mut h = household(vec![person(Role::Head, 50), person(Role::Adult, 80), person(Role::Adult, 75)]);
        h.persons[0].gone = true;
        succeed(&mut h, |_| 7);
        assert_eq!(h.persons[2].role, "head");
    }
}
