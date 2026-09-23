use phx_id::{CountryId, Day, SystemCode};
use phx_macros::clause;
use phx_num::violation;

use crate::calendar::Calendar;
use crate::register::values::PrimType;
use crate::register::{Prim, PrimKind, Register};

/// A change of a policy: announced on one day, in force from another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Announcement<T> {
    pub announced: Day,
    pub effective: Day,
    pub value: T,
}

/// Why an announcement is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnounceRefused {
    /// In force before the next business day after it is announced.
    TooSoon { earliest: Day },
    /// Announced by a system other than the policy's owner.
    NotOwner,
}

/// A policy primitive its owner may change during a run: its opening value from the register, and its dated
/// announcements, kept in order of the day they take effect.
#[clause("POL.7")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyValue<T> {
    owner: SystemCode,
    country: CountryId,
    opening: T,
    announcements: Vec<Announcement<T>>,
}

impl<T: Copy> PolicyValue<T> {
    /// The policy's opening value for a country; the primitive must be a policy.
    #[must_use]
    pub fn open<P>(prim: Prim<P>, register: &Register, country: CountryId) -> PolicyValue<T>
    where
        P: PrimType,
        for<'a> P::Read<'a>: Into<T>,
    {
        let decl = prim.decl(register);
        let Ok(owner) = decl.owner() else {
            violation!(clause = "NUM.3", "a policy whose identity names no system");
        };
        if decl.kind != PrimKind::Policy {
            violation!(clause = "NUM.3", "a primitive that is not a policy opened as a policy value");
        }
        PolicyValue { owner, country, opening: prim.get(register, country).into(), announcements: Vec::new() }
    }

    /// Records a change the owner's decision announces, in force from `effective`, which is at least the next
    /// business day of the policy's country after `announced`.
    ///
    /// # Errors
    /// When the writer is not the owner, or the change would take effect too soon.
    pub fn announce(
        &mut self,
        writer: SystemCode,
        calendar: &Calendar,
        announcement: Announcement<T>,
    ) -> Result<(), AnnounceRefused> {
        if writer != self.owner {
            return Err(AnnounceRefused::NotOwner);
        }
        let earliest = calendar.next_business(self.country, announcement.announced);
        if announcement.effective < earliest {
            return Err(AnnounceRefused::TooSoon { earliest });
        }
        let at = self.announcements.partition_point(|a| a.effective <= announcement.effective);
        self.announcements.insert(at, announcement);
        Ok(())
    }

    /// The value in force on a day: the last change effective on or before it, or the opening value.
    #[must_use]
    pub fn value_on(&self, day: Day) -> T {
        let at = self.announcements.partition_point(|a| a.effective <= day);
        at.checked_sub(1).and_then(|i| self.announcements.get(i)).map_or(self.opening, |a| a.value)
    }

    #[must_use]
    pub fn announcements(&self) -> &[Announcement<T>] {
        &self.announcements
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{CountryId, Date, SystemCode};

    use super::{AnnounceRefused, Announcement, PolicyValue};
    use crate::calendar::testing::calendar;

    #[test]
    fn policy_value_effective_dates() {
        let cal = calendar(2020);
        let day = |y, m, d| cal.day(Date::new(y, m, d).unwrap()).unwrap();
        let owner = SystemCode::new("CB").unwrap();
        let mut policy = PolicyValue { owner, country: CountryId::new(0), opening: 100_i64, announcements: vec![] };
        // Announced on Friday 7 March 2025: the next business day is Monday 10 March.
        let friday = day(2025, 3, 7);
        let at = |effective, value| Announcement { announced: friday, effective, value };
        assert_eq!(
            policy.announce(owner, &cal, at(friday, 1)),
            Err(AnnounceRefused::TooSoon { earliest: day(2025, 3, 10) }),
            "the same day"
        );
        assert!(
            policy.announce(owner, &cal, at(day(2025, 3, 8), 1)).is_err(),
            "a Saturday before the next business day"
        );
        assert_eq!(
            policy.announce(SystemCode::new("TRS").unwrap(), &cal, at(day(2025, 3, 10), 1)),
            Err(AnnounceRefused::NotOwner)
        );
        policy.announce(owner, &cal, at(day(2025, 4, 1), 150)).unwrap();
        policy.announce(owner, &cal, at(day(2025, 3, 10), 120)).unwrap();
        assert_eq!(policy.value_on(day(2025, 3, 9)), 100);
        assert_eq!(policy.value_on(day(2025, 3, 10)), 120);
        assert_eq!(policy.value_on(day(2025, 3, 31)), 120);
        assert_eq!(policy.value_on(day(2025, 4, 1)), 150);
    }
}
