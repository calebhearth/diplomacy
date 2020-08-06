//! Resolver for build phases.

use super::MappedBuildOrder;
use crate::geo::{Map, ProvinceKey, RegionKey, SupplyCenter};
use crate::order::BuildCommand;
use crate::{Nation, UnitType};
use std::collections::{HashMap, HashSet};

mod outcome;

pub use self::outcome::*;

/// Provider for the resolver to get state about the game world that it needs to successfully
/// judge a build phase.
pub trait WorldState {
    /// Get the set of nations in the game. This must include nations that issued no
    /// orders this turn, and may include nations that have no units if those units
    /// are entitled to build.
    fn nations(&self) -> HashSet<&Nation>;
    /// Get the nation with a unit _currently in_ the specified province. This should
    /// return `None` if the province is vacant, even if it's controlled by a nation.
    fn occupier(&self, province: &ProvinceKey) -> Option<&Nation>;
    /// Get the number of units owned by the specified nation
    fn unit_count(&self, nation: &Nation) -> u8;
    /// Get the units owned by the specified nation
    fn units(&self, nation: &Nation) -> HashSet<(UnitType, RegionKey)>;
}

/// The immutable pieces of a build-phase order resolution
pub struct ResolverContext<'a, W: WorldState, A> {
    last_time: &'a HashMap<ProvinceKey, Nation>,
    this_time: &'a W,
    rules: A,
    orders: Vec<&'a MappedBuildOrder>,
}

impl<'a, W: WorldState, A: Adjudicate> ResolverContext<'a, W, A> {
    /// Create a new context for resolution.
    ///
    /// # First Winter
    /// The first build phase of the game should pass the initial supply center ownerships to
    /// `last_time` to ensure the resolver knows never-since-occupied home SCs belong to their
    /// home power.
    pub fn new(
        rules: A,
        last_time: &'a HashMap<ProvinceKey, Nation>,
        this_time: &'a W,
        orders: Vec<&'a MappedBuildOrder>,
    ) -> Self {
        if last_time.is_empty() {
            panic!("At least one supply center must have been owned by at least one nation. Did you forget to pass the initial world state?");
        }

        Self {
            rules,
            last_time,
            this_time,
            orders,
        }
    }

    pub fn current_owner(&'a self, province: &ProvinceKey) -> Option<&'a Nation> {
        self.this_time
            .occupier(province)
            .or_else(|| self.last_time.get(province))
    }

    pub fn resolve(&self) -> Outcome<'a> {
        let mut state = ResolverState::new(self, self.rules.initialize(self));
        for order in &self.orders {
            state.resolve_order(self, order);
        }

        self.rules.finish(self, state)
    }
}

impl<W: WorldState, A> WorldState for ResolverContext<'_, W, A> {
    fn nations(&self) -> HashSet<&Nation> {
        self.this_time.nations()
    }

    fn occupier(&self, province: &ProvinceKey) -> Option<&Nation> {
        self.this_time.occupier(province)
    }

    fn unit_count(&self, nation: &Nation) -> u8 {
        self.this_time.unit_count(nation)
    }

    fn units(&self, nation: &Nation) -> HashSet<(UnitType, RegionKey)> {
        self.this_time.units(nation)
    }
}

pub struct ResolverState<'a, D> {
    pub data: D,
    orders: HashMap<&'a MappedBuildOrder, OrderOutcome>,
    live_units: HashMap<&'a Nation, HashSet<(UnitType, RegionKey)>>,
}

impl<'a, D> ResolverState<'a, D> {
    fn new(context: &ResolverContext<'a, impl WorldState, impl Adjudicate>, data: D) -> Self {
        Self {
            data,
            orders: HashMap::new(),
            live_units: context
                .this_time
                .nations()
                .into_iter()
                .map(|nation| (nation, context.this_time.units(nation)))
                .collect(),
        }
    }

    fn resolve_order<A>(
        &mut self,
        context: &ResolverContext<'a, impl WorldState, A>,
        order: &'a MappedBuildOrder,
    ) -> OrderOutcome
    where
        A: Adjudicate<CustomState = D>,
    {
        if let Some(outcome) = self.orders.get(order) {
            return *outcome;
        }

        let outcome = context.rules.adjudicate(context, self, order);

        self.orders.insert(order, outcome);

        if outcome == OrderOutcome::Succeeds {
            let positions = self
                .live_units
                .entry(&order.nation)
                .or_insert_with(HashSet::new);
            match order.command {
                BuildCommand::Build => {
                    positions.insert((order.unit_type, order.region.clone()));
                }
                BuildCommand::Disband => {
                    positions.remove(&(order.unit_type, order.region.clone()));
                }
            }
        }

        outcome
    }

    pub fn occupier(&self, province: &ProvinceKey) -> Option<&'a Nation> {
        for (nation, units) in &self.live_units {
            for (_, region) in units {
                if region.province() == province {
                    return Some(nation);
                }
            }
        }

        None
    }
}

pub trait Adjudicate: Sized {
    /// Mutable information used by the rulebook during adjudication.
    type CustomState;

    fn initialize<'a, W: WorldState>(
        &self,
        context: &'a ResolverContext<'a, W, Self>,
    ) -> Self::CustomState;

    /// Adjudicate a single build-phase order, returning its outcome. Orders are passed to
    /// this function in the order they are received.
    fn adjudicate<'a, W: WorldState>(
        &self,
        context: &ResolverContext<'a, W, Self>,
        state: &mut ResolverState<'a, Self::CustomState>,
        order: &MappedBuildOrder,
    ) -> OrderOutcome;

    /// Complete a build-phase adjudication. The implementation of this function should
    /// perform any forcible disbands needed to bring a nation's units down to its
    /// carrying capacity.
    fn finish<'a, W: WorldState>(
        &self,
        context: &ResolverContext<'a, W, Self>,
        state: ResolverState<'a, Self::CustomState>,
    ) -> Outcome<'a>;
}

/// Convert a map into an initial ownership state where each nation owns their home
/// supply centers and all other supply centers are unowned.
pub fn to_initial_ownerships(map: &Map) -> HashMap<ProvinceKey, Nation> {
    map.provinces()
        .filter_map(|province| {
            if let SupplyCenter::Home(nat) = &province.supply_center {
                Some((province.into(), nat.clone()))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::to_initial_ownerships;
    use crate::geo::{standard_map, ProvinceKey};
    use crate::Nation;

    #[test]
    fn to_initial_ownerships_for_standard_map() {
        let ownerships = to_initial_ownerships(standard_map());

        assert_eq!(
            Some(&Nation::from("AUS")),
            ownerships.get(&ProvinceKey::from("bud"))
        );

        assert_eq!(None, ownerships.get(&ProvinceKey::from("bel")));
    }
}
