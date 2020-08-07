use crate::geo::{Map, ProvinceKey, RegionKey, SupplyCenter};
use crate::judge::{build, build::WorldState, MappedBuildOrder};
use crate::order::BuildCommand;
use crate::{Nation, UnitPosition, UnitPositions};
use std::collections::{HashMap, HashSet};

/// Vanilla Diplomacy rules.
///
/// This rulebook works with arbitrary maps.
pub struct Standard<'world> {
    map: &'world Map,
    nations: HashSet<&'world Nation>,
}

impl Standard<'_> {
    /// Get the map for the variant.
    pub fn map(&self) -> &Map {
        self.map
    }

    /// Initialize a build-phase judge from the variant.
    pub fn as_build_judge(&self) -> StandardBuildJudge {
        let mut home_scs = HashMap::new();
        for province in self.map().provinces() {
            if let SupplyCenter::Home(nat) = &province.supply_center {
                home_scs
                    .entry(nat)
                    .or_insert_with(HashSet::new)
                    .insert(province.into());
            }
        }

        StandardBuildJudge {
            map: self.map(),
            nations: &self.nations,
            home_scs,
        }
    }
}

pub struct StandardBuildJudge<'a> {
    map: &'a Map,
    nations: &'a HashSet<&'a Nation>,
    home_scs: HashMap<&'a Nation, HashSet<ProvinceKey>>,
}

impl<'a> StandardBuildJudge<'a> {
    fn get_nation(&self, nation: &Nation) -> Option<&'a Nation> {
        self.nations.iter().find(|n| **n == nation).copied()
    }

    fn try_preserve_position(&self, position: &UnitPosition<'_>) -> Option<UnitPosition<'a>> {
        let UnitPosition { unit, region } = position;
        Some(UnitPosition::new(
            (self.get_nation(unit.nation())?, unit.unit_type()).into(),
            self.map.get_region(region)?.as_ref(),
        ))
    }

    fn preserve_position(&self, position: &UnitPosition<'_>) -> UnitPosition<'a> {
        self.try_preserve_position(position)
            .ok_or(position)
            .expect("Build judge should not try and preserve impossible unit positions")
    }
}

impl<'world, 'turn, W> build::Adjudicate<'turn, W> for StandardBuildJudge<'world>
where
    W: WorldState,
    'world: 'turn,
{
    type CustomState = HashMap<&'world Nation, (BuildCommand, usize)>;

    fn initialize(&self, context: &build::ResolverContext<'turn, W, Self>) -> Self::CustomState {
        // Count the supply centers controlled by every nation who was ever in the game.
        let mut ownerships = self
            .nations
            .iter()
            .copied()
            .map(|nation| (nation, 0))
            .collect::<HashMap<_, _>>();

        // Credit each nation 1 "point" for each supply center it controls
        for province in self.map.provinces().filter(|p| p.is_supply_center()) {
            if let Some(nation) = context
                .current_owner(province.as_ref())
                .and_then(|nation| self.get_nation(nation))
            {
                *ownerships.entry(nation).or_insert(0) += 1;
            }
        }

        ownerships
            .into_iter()
            .filter_map(|(nation, ownerships)| {
                let adjustment = ownerships - context.unit_count(&nation) as i16;
                match adjustment {
                    0 => None,
                    x if x > 0 => Some((nation, (BuildCommand::Build, x as usize))),
                    x => Some((nation, (BuildCommand::Disband, -x as usize))),
                }
            })
            .collect()
    }

    fn adjudicate(
        &self,
        context: &build::ResolverContext<'turn, W, Self>,
        state: &mut build::ResolverState<'turn, Self::CustomState>,
        order: &MappedBuildOrder,
    ) -> build::OrderOutcome {
        use build::OrderOutcome::*;
        let province = order.region.province();

        let (allowed_command, remaining) = if let Some(delta) = state.data.get(&order.nation) {
            (delta.0.clone(), delta.1)
        } else {
            return RedeploymentProhibited;
        };

        if order.command != allowed_command {
            return RedeploymentProhibited;
        }

        match order.command {
            BuildCommand::Build => {
                if !self
                    .home_scs
                    .get(&order.nation)
                    .expect("Every nation should have home SCs")
                    .contains(province)
                {
                    return InvalidProvince;
                }

                if Some(&order.nation) != context.current_owner(province) {
                    return ForeignControlled;
                }

                if state.occupier(province).is_some() {
                    return OccupiedProvince;
                }

                let region = if let Some(region) = self.map.get_region(&order.region) {
                    region
                } else {
                    return InvalidProvince;
                };

                if !order.unit_type.can_occupy(region.terrain()) {
                    return InvalidTerrain;
                }

                if remaining == 0 {
                    return AllBuildsUsed;
                }

                // Debit the nation for the build
                state.data.get_mut(&order.nation).unwrap().1 -= 1;

                Succeeds
            }
            BuildCommand::Disband => match state.occupier(province) {
                None => DisbandingNonexistentUnit,
                Some(nation) if &order.nation != nation => DisbandingForeignUnit,
                _ if remaining == 0 => AllDisbandsUsed,
                _ => {
                    // Debit the nation for the disband
                    state.data.get_mut(&order.nation).unwrap().1 -= 1;
                    Succeeds
                }
            },
        }
    }

    fn finish(
        &self,
        context: &build::ResolverContext<'turn, W, Self>,
        mut state: build::ResolverState<'turn, Self::CustomState>,
    ) -> build::Outcome<'turn> {
        let sc_ownerships = self
            .map
            .provinces()
            .filter(|p| p.is_supply_center())
            .filter_map(|p| {
                Some((
                    p.as_ref(),
                    context
                        .current_owner(&p.into())
                        .and_then(|n| self.get_nation(n))?,
                ))
            })
            .collect();

        let positions_to_disband = {
            let mut to_disband = vec![];
            let positions = state.unit_positions();
            for (nation, delta) in &state.data {
                // If the nation didn't have to disband units, or has disbanded all relevant units,
                // then we don't have to do anything
                if delta.0 == BuildCommand::Build || delta.1 == 0 {
                    continue;
                }

                // Sort the nation's live units into disbandment order
                let mut nation_positions = positions
                    .iter()
                    .filter(|p| p.nation() == *nation)
                    .collect::<Vec<_>>();
                nation_positions.sort_by_cached_key(|p| -(distance_from_home(self.map, *p) as i16));

                // Perform automatic disbandments
                to_disband.extend(
                    nation_positions
                        .drain(0..delta.1)
                        .map(|p| self.preserve_position(&p)),
                );
            }

            to_disband
        };

        for position in positions_to_disband {
            state.disband_unit(position.cloned_region());
        }

        build::Outcome::new(
            state.to_order_outcomes(),
            sc_ownerships,
            state
                .unit_positions()
                .into_iter()
                .map(|p| self.preserve_position(&p)),
        )
        .expect("Standard variant should produce valid outcome")
    }
}

fn distance_from_home(_map: &Map, _unit: &UnitPosition<'_, &RegionKey>) -> usize {
    unimplemented!()
}
