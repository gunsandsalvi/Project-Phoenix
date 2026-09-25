//! The household and its persons, the vocabulary every system that reads or writes a household shares: its roles,
//! its attributes and its persons' attributes. `sys-dem` declares them; other systems read and extend them.

pub mod consts;

use phx_core::{AttrDecl, PersonAttrDecl, RoleDecl};

use crate::consts::{EDUCATION_VALUES, REGIONS};

/// The population kind of households.
pub const HOUSEHOLD: &str = "household";

/// A person's sex: female, male.
pub const SEX: PersonAttrDecl = PersonAttrDecl { name: "DEM.sex", values: 2, clause: "REP.26" };
/// A person's health: able, or disabled lastingly.
pub const HEALTH: PersonAttrDecl = PersonAttrDecl { name: "DEM.health", values: 2, clause: "POP.4" };
/// An adult's highest level of education: none, incomplete primary, primary, lower secondary, upper secondary, short
/// post-secondary, bachelor, master and higher; and a last value, not yet recorded, for a child and for a person who
/// reached adulthood in the run before schooling is kept.
pub const EDUCATION: PersonAttrDecl =
    PersonAttrDecl { name: "DEM.education", values: EDUCATION_VALUES, clause: "REP.26" };
/// The education value of a person whose schooling the run has not recorded, the last.
pub const EDUCATION_UNRECORDED: u32 = EDUCATION_VALUES - 1;
pub const FEMALE: u32 = 0;
pub const MALE: u32 = 1;
pub const ABLE: u32 = 0;
pub const DISABLED: u32 = 1;

/// The region a household lives in.
pub const REGION: AttrDecl = AttrDecl { name: "DEM.region", values: REGIONS, clause: "REP.41" };

pub const HEAD: RoleDecl = RoleDecl { name: "head", clause: "REP.26" };
pub const PARTNER: RoleDecl = RoleDecl { name: "partner", clause: "REP.26" };
/// Another adult of the household.
pub const ADULT: RoleDecl = RoleDecl { name: "adult", clause: "REP.26" };
/// A child under the age of majority.
pub const CHILD: RoleDecl = RoleDecl { name: "child", clause: "REP.26" };
