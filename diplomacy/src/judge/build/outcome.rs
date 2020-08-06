use crate::{geo::RegionKey, judge::MappedBuildOrder, judge::OrderState, Nation, UnitType};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Outcome<'a> {
    pub orders: HashMap<&'a MappedBuildOrder, OrderOutcome>,
    pub civil_disorder: HashSet<(UnitType, RegionKey)>,
    pub final_units: HashMap<&'a Nation, HashSet<(UnitType, RegionKey)>>,
}

/// The outcome of a build-turn order.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OrderOutcome {
    /// The build or disband order was successful, resulting in a change in units
    /// in the world.
    Succeeds,
    /// A nation cannot issue "build" and "disband" commands in the same turn,
    /// as this would constitute an illegal teleportation of power from the
    /// disbanding region to the building region.
    RedeploymentProhibited,
    /// The build command was to a province where the issuing nation cannot build.
    InvalidProvince,
    /// The build command was to a home SC for the issuing power, but another
    /// power currently controls it.
    ForeignControlled,
    /// Build failed because the target province already has a friendly unit in it.
    OccupiedProvince,
    /// The build command is to a region that is qualified for
    InvalidTerrain,
    /// Disband failed because no unit exists at that location.
    DisbandingNonexistentUnit,
    /// Disband failed because the issuing power does not control the unit at that location.
    DisbandingForeignUnit,
    /// The issuing nation has already had as many successful builds as they are allowed.
    AllBuildsUsed,
    /// The issuing nation has already had as many successful disbands as they are allowed.
    AllDisbandsUsed,
}

impl From<OrderOutcome> for OrderState {
    fn from(outcome: OrderOutcome) -> Self {
        if outcome == OrderOutcome::Succeeds {
            OrderState::Succeeds
        } else {
            OrderState::Fails
        }
    }
}
