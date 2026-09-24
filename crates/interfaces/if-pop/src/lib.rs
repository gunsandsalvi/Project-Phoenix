//! The household and its persons, the vocabulary every system that reads or writes a household shares: its roles,
//! the key attributes that place it and count its persons, and the profile groups that describe them. `sys-dem`
//! declares them; other systems read and extend them.

pub mod consts;

use phx_core::{GroupDecl, KeyAttrDecl, ProfileComponent, RoleCount, RoleDecl};

use crate::consts::{ADULT_CLASSES, BIRTH_YEARS, EDUCATION_VALUES, PERSONS_PER_ROLE, REGIONS};

/// The population kind of households.
pub const HOUSEHOLD: &str = "household";

/// A person's birth year, counted from `consts::FIRST_BIRTH_YEAR`.
pub const BIRTH_YEAR: ProfileComponent = ProfileComponent { name: "birth_year", values: BIRTH_YEARS };
/// A person's sex: female, male.
pub const SEX: ProfileComponent = ProfileComponent { name: "sex", values: 2 };
/// A person's health: able, or disabled lastingly.
pub const HEALTH: ProfileComponent = ProfileComponent { name: "health", values: 2 };
/// An adult's highest level of education: none, incomplete primary, primary, lower secondary, upper secondary, short
/// post-secondary, bachelor, master and higher; and a last value, not yet recorded, for a person who reached adulthood
/// in the run before schooling is kept.
pub const EDUCATION: ProfileComponent = ProfileComponent { name: "education", values: EDUCATION_VALUES };
/// The education value of an adult whose schooling the run has not yet recorded, the last.
pub const EDUCATION_UNRECORDED: u32 = EDUCATION_VALUES - 1;
pub const FEMALE: u32 = 0;
pub const MALE: u32 = 1;
pub const ABLE: u32 = 0;
pub const DISABLED: u32 = 1;

/// Birth year, sex and health, joint, which mortality and illness read.
pub const LIFE: &[ProfileComponent] = &[BIRTH_YEAR, SEX, HEALTH];
pub const SCHOOLING: &[ProfileComponent] = &[EDUCATION];

/// The region a household lives in.
pub const REGION: KeyAttrDecl = KeyAttrDecl { name: "DEM.region", values: REGIONS, clause: "REP.19" };
/// The head's age class, and the partner's; a household with no partner holds its partner's class at nought.
pub const HEAD_AGE: KeyAttrDecl = KeyAttrDecl { name: "DEM.head_age", values: ADULT_CLASSES, clause: "REP.25" };
pub const PARTNER_AGE: KeyAttrDecl = KeyAttrDecl { name: "DEM.partner_age", values: ADULT_CLASSES, clause: "REP.25" };
/// Whether the head has a partner in the household.
pub const PARTNERS: KeyAttrDecl = KeyAttrDecl { name: "DEM.partners", values: 2, clause: "REP.26" };

pub const HEAD: RoleDecl = RoleDecl { name: "head", per_member: RoleCount::One, clause: "REP.26" };
pub const PARTNER: RoleDecl =
    RoleDecl { name: "partner", per_member: RoleCount::Key("DEM.partners"), clause: "REP.26" };

/// The other adults of each age class, as many as the key counts; the classes beyond the partition's stay empty.
pub const ADULTS: &[RoleDecl] = &[
    RoleDecl { name: "adult_0", per_member: RoleCount::Key("DEM.adults_0"), clause: "REP.26" },
    RoleDecl { name: "adult_1", per_member: RoleCount::Key("DEM.adults_1"), clause: "REP.26" },
    RoleDecl { name: "adult_2", per_member: RoleCount::Key("DEM.adults_2"), clause: "REP.26" },
    RoleDecl { name: "adult_3", per_member: RoleCount::Key("DEM.adults_3"), clause: "REP.26" },
    RoleDecl { name: "adult_4", per_member: RoleCount::Key("DEM.adults_4"), clause: "REP.26" },
    RoleDecl { name: "adult_5", per_member: RoleCount::Key("DEM.adults_5"), clause: "REP.26" },
    RoleDecl { name: "adult_6", per_member: RoleCount::Key("DEM.adults_6"), clause: "REP.26" },
    RoleDecl { name: "adult_7", per_member: RoleCount::Key("DEM.adults_7"), clause: "REP.26" },
];
/// The children of each age band, as many as the key counts.
pub const CHILDREN: &[RoleDecl] = &[
    RoleDecl { name: "child_0", per_member: RoleCount::Key("DEM.children_0"), clause: "REP.26" },
    RoleDecl { name: "child_1", per_member: RoleCount::Key("DEM.children_1"), clause: "REP.26" },
    RoleDecl { name: "child_2", per_member: RoleCount::Key("DEM.children_2"), clause: "REP.26" },
    RoleDecl { name: "child_3", per_member: RoleCount::Key("DEM.children_3"), clause: "REP.26" },
    RoleDecl { name: "child_4", per_member: RoleCount::Key("DEM.children_4"), clause: "REP.26" },
    RoleDecl { name: "child_5", per_member: RoleCount::Key("DEM.children_5"), clause: "REP.26" },
    RoleDecl { name: "child_6", per_member: RoleCount::Key("DEM.children_6"), clause: "REP.26" },
    RoleDecl { name: "child_7", per_member: RoleCount::Key("DEM.children_7"), clause: "REP.26" },
];
/// The counts of other adults by age class and children by age band.
pub const ADULT_COUNTS: &[KeyAttrDecl] = &[
    KeyAttrDecl { name: "DEM.adults_0", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.adults_1", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.adults_2", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.adults_3", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.adults_4", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.adults_5", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.adults_6", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.adults_7", values: PERSONS_PER_ROLE, clause: "REP.26" },
];
pub const CHILD_COUNTS: &[KeyAttrDecl] = &[
    KeyAttrDecl { name: "DEM.children_0", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.children_1", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.children_2", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.children_3", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.children_4", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.children_5", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.children_6", values: PERSONS_PER_ROLE, clause: "REP.26" },
    KeyAttrDecl { name: "DEM.children_7", values: PERSONS_PER_ROLE, clause: "REP.26" },
];

/// Each adult role's groups: its life, then its schooling.
pub const ADULT_GROUPS: &[(GroupDecl, GroupDecl)] = &[
    (
        GroupDecl { name: "DEM.head_life", role: "head", components: LIFE, clause: "REP.32" },
        GroupDecl { name: "DEM.head_schooling", role: "head", components: SCHOOLING, clause: "REP.32" },
    ),
    (
        GroupDecl { name: "DEM.partner_life", role: "partner", components: LIFE, clause: "REP.32" },
        GroupDecl { name: "DEM.partner_schooling", role: "partner", components: SCHOOLING, clause: "REP.32" },
    ),
    (
        GroupDecl { name: "DEM.adult_0_life", role: "adult_0", components: LIFE, clause: "REP.32" },
        GroupDecl { name: "DEM.adult_0_schooling", role: "adult_0", components: SCHOOLING, clause: "REP.32" },
    ),
    (
        GroupDecl { name: "DEM.adult_1_life", role: "adult_1", components: LIFE, clause: "REP.32" },
        GroupDecl { name: "DEM.adult_1_schooling", role: "adult_1", components: SCHOOLING, clause: "REP.32" },
    ),
    (
        GroupDecl { name: "DEM.adult_2_life", role: "adult_2", components: LIFE, clause: "REP.32" },
        GroupDecl { name: "DEM.adult_2_schooling", role: "adult_2", components: SCHOOLING, clause: "REP.32" },
    ),
    (
        GroupDecl { name: "DEM.adult_3_life", role: "adult_3", components: LIFE, clause: "REP.32" },
        GroupDecl { name: "DEM.adult_3_schooling", role: "adult_3", components: SCHOOLING, clause: "REP.32" },
    ),
    (
        GroupDecl { name: "DEM.adult_4_life", role: "adult_4", components: LIFE, clause: "REP.32" },
        GroupDecl { name: "DEM.adult_4_schooling", role: "adult_4", components: SCHOOLING, clause: "REP.32" },
    ),
    (
        GroupDecl { name: "DEM.adult_5_life", role: "adult_5", components: LIFE, clause: "REP.32" },
        GroupDecl { name: "DEM.adult_5_schooling", role: "adult_5", components: SCHOOLING, clause: "REP.32" },
    ),
    (
        GroupDecl { name: "DEM.adult_6_life", role: "adult_6", components: LIFE, clause: "REP.32" },
        GroupDecl { name: "DEM.adult_6_schooling", role: "adult_6", components: SCHOOLING, clause: "REP.32" },
    ),
    (
        GroupDecl { name: "DEM.adult_7_life", role: "adult_7", components: LIFE, clause: "REP.32" },
        GroupDecl { name: "DEM.adult_7_schooling", role: "adult_7", components: SCHOOLING, clause: "REP.32" },
    ),
];
/// Each child role's life.
pub const CHILD_GROUPS: &[GroupDecl] = &[
    GroupDecl { name: "DEM.child_0_life", role: "child_0", components: LIFE, clause: "REP.32" },
    GroupDecl { name: "DEM.child_1_life", role: "child_1", components: LIFE, clause: "REP.32" },
    GroupDecl { name: "DEM.child_2_life", role: "child_2", components: LIFE, clause: "REP.32" },
    GroupDecl { name: "DEM.child_3_life", role: "child_3", components: LIFE, clause: "REP.32" },
    GroupDecl { name: "DEM.child_4_life", role: "child_4", components: LIFE, clause: "REP.32" },
    GroupDecl { name: "DEM.child_5_life", role: "child_5", components: LIFE, clause: "REP.32" },
    GroupDecl { name: "DEM.child_6_life", role: "child_6", components: LIFE, clause: "REP.32" },
    GroupDecl { name: "DEM.child_7_life", role: "child_7", components: LIFE, clause: "REP.32" },
];
