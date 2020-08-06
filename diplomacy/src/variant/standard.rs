use crate::geo::{Map, ProvinceKey, SupplyCenter};
use crate::judge::{build, build::WorldState, MappedBuildOrder};
use crate::order::BuildCommand;
use crate::{Nation, ShortName};
use std::collections::{HashMap, HashSet};

pub struct Standard<'map> {
    map: &'map Map,
}

impl Standard<'_> {
    pub fn map(&self) -> &Map {
        self.map
    }

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
            home_scs,
        }
    }
}

pub struct StandardBuildJudge<'a> {
    map: &'a Map,
    home_scs: HashMap<&'a Nation, HashSet<ProvinceKey>>,
}

impl build::Adjudicate for StandardBuildJudge<'_> {
    type CustomState = HashMap<Nation, (BuildCommand, i16)>;

    fn initialize<'a, W: build::WorldState>(
        &self,
        context: &build::ResolverContext<'a, W, Self>,
    ) -> Self::CustomState {
        let mut ownerships = HashMap::new();
        for province in self.map.provinces().filter(|p| p.is_supply_center()) {
            let key = ProvinceKey::from(province);
            if let Some(nation) = context.current_owner(&key).cloned() {
                *ownerships.entry(nation).or_insert(0) += 1;
            }
        }

        ownerships
            .into_iter()
            .filter_map(|(nation, ownerships)| {
                let adjustment = ownerships - context.unit_count(&nation) as i16;
                match adjustment {
                    0 => None,
                    x if x > 0 => Some((nation, (BuildCommand::Build, x))),
                    x => Some((nation, (BuildCommand::Disband, -x))),
                }
            })
            .collect()
    }

    fn adjudicate<'a, W: WorldState>(
        &self,
        context: &build::ResolverContext<'a, W, Self>,
        state: &mut build::ResolverState<'a, Self::CustomState>,
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

                let region = if let Some(region) = self.map.find_region(&order.region.short_name())
                {
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

    fn finish<'a, W: WorldState>(
        &self,
        _context: &build::ResolverContext<'a, W, Self>,
        _state: build::ResolverState<'a, Self::CustomState>,
    ) -> build::Outcome<'a> {
        unimplemented!()
    }
}
