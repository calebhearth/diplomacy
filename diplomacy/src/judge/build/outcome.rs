use crate::{
    geo::ProvinceKey, geo::RegionKey, judge::MappedBuildOrder, judge::OrderState, Nation, Unit,
    UnitPosition, UnitPositions,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Outcome<'a> {
    orders: HashMap<&'a MappedBuildOrder, OrderOutcome>,
    sc_ownerships: HashMap<&'a ProvinceKey, &'a Nation>,
    final_units: Vec<UnitPosition<'a>>,
}

impl<'a> Outcome<'a> {
    /// Create a new build phase outcome with order outcomes, updated supply center ownerships,
    /// and the positions of units for the next phase.
    ///
    /// # Errors
    /// This function will return an error if multiple units are in the same province.
    pub fn new(
        orders: HashMap<&'a MappedBuildOrder, OrderOutcome>,
        sc_ownerships: HashMap<&'a ProvinceKey, &'a Nation>,
        units: impl IntoIterator<Item = UnitPosition<'a>>,
    ) -> Result<Self, OutcomeError<'a>> {
        let mut occupied_provinces = HashSet::new();
        let mut final_units = vec![];

        for unit in units {
            if !occupied_provinces.insert(unit.region.province()) {
                return Err(OutcomeError::MultipleUnitsInSameProvince(
                    unit.region.province(),
                ));
            }

            final_units.push(unit);
        }

        Ok(Self {
            orders,
            sc_ownerships,
            final_units,
        })
    }

    /// Get the outcome of a specific order.
    pub fn get(&self, order: &MappedBuildOrder) -> Option<OrderOutcome> {
        self.orders.get(order).copied()
    }

    /// Get all submitted orders and their outcomes.
    pub fn orders(&self) -> &HashMap<&'a MappedBuildOrder, OrderOutcome> {
        &self.orders
    }

    pub fn sc_ownerships(&self) -> &HashMap<&'a ProvinceKey, &'a Nation> {
        &self.sc_ownerships
    }
}

impl UnitPositions<RegionKey> for Outcome<'_> {
    fn unit_positions(&self) -> Vec<UnitPosition<'_>> {
        self.final_units.clone()
    }

    fn find_province_occupier(
        &self,
        province: &ProvinceKey,
    ) -> Option<UnitPosition<'_, &RegionKey>> {
        self.unit_positions()
            .into_iter()
            .find(|position| position.region == province)
    }

    fn find_region_occupier(&self, region: &RegionKey) -> Option<Unit<'_>> {
        self.unit_positions()
            .into_iter()
            .find(|position| position.region == region)
            .map(|p| p.unit)
    }
}

/// Error when a build-phase outcome blocks continuation of the game.
#[derive(Debug, Clone)]
pub enum OutcomeError<'a> {
    MultipleUnitsInSameProvince(&'a ProvinceKey),
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
