//! Contains helper functions for evaluating the success of support commands
//! during the main phase of a turn.

use super::calc;
use super::{Adjudicate, MappedMainOrder, OrderState, ResolverContext, ResolverState};
use crate::geo::{Map, ProvinceKey};
use crate::order::{Command, MainCommand, SupportedOrder};

fn order_cuts<'a, A: Adjudicate>(
    ctx: &'a ResolverContext<'a>,
    resolver: &mut ResolverState<'a, A>,
    support_order: &MappedMainOrder,
    cutting_order: &MappedMainOrder,
) -> bool {
    // Only moves can cut support
    if let Some(ref dst) = cutting_order.command.move_dest() {
        // If the support order is attacking the cutting order's province, then
        // support is not cut
        let supporting_attack_on_cutter = match support_order.command {
            MainCommand::Support(SupportedOrder::Move(_, _, ref supported_dst)) => {
                cutting_order.region.province() == supported_dst.province()
            }
            _ => false,
        };

        dst == &support_order.region.province()
            && !supporting_attack_on_cutter
            && support_order.nation != cutting_order.nation
            && calc::path_exists(ctx, resolver, cutting_order)
    } else {
        false
    }
}

/// Find all orders which cut a specified support order.
pub fn find_cutting_order<'a, A: Adjudicate>(
    ctx: &'a ResolverContext<'a>,
    resolver: &mut ResolverState<'a, A>,
    support_order: &MappedMainOrder,
) -> Option<&'a MappedMainOrder> {
    ctx.orders()
        .iter()
        .find(|order| order_cuts(ctx, resolver, support_order, order))
}

/// A SUPPORT decision of a unit ordered to support results in 'cut' when:
/// At least one of the units ordered to move to the area of the supporting unit
/// has a minimum ATTACK STRENGTH of one or more. Again, if the support order is
/// a move support, then the unit that is on the area where the move is directed,
/// should not be taken into account. Finally, the SUPPORT decisions also results
/// in 'cut' when the DISLODGE decision of the unit has status 'dislodged' (dislodge rule).
///
/// This method short-circuits the search after any hit has been found.
pub fn is_order_cut<'a, A: Adjudicate>(
    ctx: &'a ResolverContext<'a>,
    resolver: &mut ResolverState<'a, A>,
    support_order: &MappedMainOrder,
) -> bool {
    ctx.orders()
        .iter()
        .any(|order| order_cuts(ctx, resolver, support_order, &order))
}

pub fn is_supporting_self(support_order: &MappedMainOrder) -> bool {
    if let MainCommand::Support(SupportedOrder::Hold(_, ref loc)) = support_order.command {
        loc.province() == &support_order.region
    } else {
        false
    }
}

/// Returns the province which support needs to reach to benefit `supported`.
/// For move orders, this is the **destination** province. For all other orders,
/// it is the **currently occupied** province.
fn needed_at(supported: &MappedMainOrder) -> &ProvinceKey {
    use crate::order::MainCommand::*;
    match supported.command {
        Move(ref dest) => dest.province(),
        Hold | Support(..) | Convoy(..) => supported.region.province(),
    }
}

/// Determines if a support order can reach the province where it is needed.
/// This requires a border from the unit's current region to the province
/// where support is needed.
fn can_reach<'a>(
    world_map: &'a Map,
    supported: &'a MappedMainOrder,
    support_order: &'a MappedMainOrder,
) -> bool {
    world_map
        .find_borders_between(&support_order.region, needed_at(supported))
        .iter()
        .any(|b| b.is_passable_by(support_order.unit_type))
}

/// Returns true if an order is a legal support order.
fn is_legal(support_order: &MappedMainOrder) -> bool {
    use crate::order::MainCommand::*;

    match support_order.command {
        Support(SupportedOrder::Hold(_, ref tgt)) => {
            tgt.province() != support_order.region.province()
        }

        // test case 6.d.34; support targeting own area not allowed.
        Support(SupportedOrder::Move(_, _, ref dst)) => {
            dst.province() != support_order.region.province()
        }
        Hold | Move(..) | Convoy(..) => false,
    }
}

/// Returns true if a given support order successfully supports the specified supported order.
pub fn is_successful<'a, A: Adjudicate>(
    ctx: &'a ResolverContext<'a>,
    resolver: &mut ResolverState<'a, A>,
    supported: &MappedMainOrder,
    support_order: &'a MappedMainOrder,
) -> bool {
    if let MainCommand::Support(ref beneficiary) = support_order.command {
        is_legal(support_order)
            && beneficiary.is_legal()
            && beneficiary == supported
            && can_reach(&ctx.world_map, supported, support_order)
            && resolver.resolve(ctx, support_order).into()
    } else {
        false
    }
}

/// Finds all successful orders which support a given order.
pub fn find_for<'a, A: Adjudicate>(
    ctx: &'a ResolverContext<'a>,
    resolver: &mut ResolverState<'a, A>,
    supported: &MappedMainOrder,
) -> Vec<&'a MappedMainOrder> {
    ctx.orders()
        .iter()
        .filter(|order| is_successful(ctx, resolver, supported, order))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupportOutcome<'a> {
    NotDisrupted,
    SupportingSelf,
    CutBy(&'a MappedMainOrder),
}

impl<'a> SupportOutcome<'a> {
    pub fn is_successful(&self) -> bool {
        self == &SupportOutcome::NotDisrupted
    }
}

impl<'a> From<SupportOutcome<'a>> for OrderState {
    fn from(so: SupportOutcome<'a>) -> Self {
        so.is_successful().into()
    }
}

#[cfg(test)]
mod test {
    use super::super::{ResolverContext, ResolverState};
    use super::*;
    use crate::geo::{standard_map, RegionKey};
    use crate::order::{MainCommand, Order, SupportedOrder};
    use crate::Nation;
    use crate::UnitType;
    use std::str::FromStr;

    fn reg(s: &str) -> RegionKey {
        RegionKey::from_str(s).unwrap()
    }

    #[test]
    fn is_support_successful() {
        let ger = Nation("ger".into());
        let supp_com = SupportedOrder::Move(UnitType::Fleet, reg("nth"), reg("nwy"));
        let orders = vec![
            Order::new(
                ger.clone(),
                UnitType::Fleet,
                reg("ska"),
                MainCommand::Support(supp_com.clone()),
            ),
            Order::new(
                ger,
                UnitType::Fleet,
                reg("nth"),
                MainCommand::Move(reg("nwy")),
            ),
        ];

        assert_eq!(supp_com, orders[1]);
        assert!(super::can_reach(standard_map(), &orders[1], &orders[0]));

        let resolver_ctx = ResolverContext::new(standard_map(), orders.clone());
        let mut res_state = ResolverState::with_adjudicator(super::super::rulebook::Rulebook);
        let supporters = find_for(&resolver_ctx, &mut res_state, &orders[1]);
        assert!(!supporters.is_empty());
    }

    #[test]
    fn support_t6b04_support_to_unreachable_coast_allowed() {
        let fra = Nation("fra".into());
        let spa_nc = RegionKey::from_str("spa(nc)").unwrap();
        let supp_com = SupportedOrder::Move(UnitType::Fleet, reg("gas"), spa_nc.clone());
        let orders = vec![
            Order::new(
                fra.clone(),
                UnitType::Fleet,
                reg("gas"),
                MainCommand::Move(spa_nc),
            ),
            Order::new(fra, UnitType::Fleet, reg("mar"), supp_com.clone().into()),
            Order::new(
                Nation("ita".into()),
                UnitType::Fleet,
                reg("wes"),
                MainCommand::Move(reg("spa(sc)")),
            ),
        ];

        assert_eq!(supp_com, orders[0]);
        assert!(super::can_reach(standard_map(), &orders[0], &orders[1]));
    }
}
