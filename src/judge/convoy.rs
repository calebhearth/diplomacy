use geo::{Map, ProvinceKey};
use order::{Command, MainCommand};
use super::{MappedMainOrder, ResolverState, ResolverContext, Adjudicate};
use UnitType;

/// Failure cases for convoy route lookup.
pub enum ConvoyRouteError {
    /// Only armies can be convoyed.
    CanOnlyConvoyArmy,

    /// Hold, support, and convoy orders cannot be convoyed.
    CanOnlyConvoyMove,
}

/// Checks whether `convoy` is a valid convoy that will carry `mv_ord` from
/// its current location to the destination.
fn is_convoy_for(convoy: &MappedMainOrder, mv_ord: &MappedMainOrder) -> bool {
    match &convoy.command {
        &MainCommand::Convoy(ref cm) => cm == mv_ord,
        _ => false,
    }
}

/// Find all routes from `origin` to `dest` given a set of valid convoys.
fn route_steps<'a>(map: &Map,
                   convoys: Vec<&'a MappedMainOrder>,
                   origin: &ProvinceKey,
                   dest: &ProvinceKey,
                   working_path: Vec<&'a MappedMainOrder>)
                   -> Vec<Vec<&'a MappedMainOrder>> {

    let adjacent_regions = map.find_bordering(origin, None);
    // if we've got a convoy going and there is one hop to the destination,
    // we've found a valid solution.
    if !working_path.is_empty() && adjacent_regions.iter().find(|&&r| r == dest).is_some() {
        vec![working_path]
    } else {
        let mut paths = vec![];
        for convoy in &convoys {
            // move to adjacent, and don't allow backtracking/cycles
            if !working_path.contains(&convoy) && adjacent_regions.contains(&&convoy.region) {
                let mut next_path = working_path.clone();
                next_path.push(&convoy);
                let mut steps = route_steps(map,
                                            convoys.clone(),
                                            (&convoy.region).into(),
                                            dest,
                                            next_path);
                if !steps.is_empty() {
                    paths.append(&mut steps);
                }
            }
        }

        paths
    }
}

/// Finds all valid convoy routes for a given move order.
pub fn routes<'a, A: Adjudicate>(ctx: &'a ResolverContext<'a>,
                                 state: &mut ResolverState<'a, A>,
                                 mv_ord: &MappedMainOrder)
                                 -> Result<Vec<Vec<&'a MappedMainOrder>>, ConvoyRouteError> {
    if mv_ord.unit_type == UnitType::Fleet {
        Err(ConvoyRouteError::CanOnlyConvoyArmy)
    } else {
        if let Some(dst) = mv_ord.move_dest() {
            let mut convoy_steps = vec![];
            for order in ctx.orders_ref() {
                if is_convoy_for(order, mv_ord) && state.resolve(ctx, order).into() {
                    convoy_steps.push(order);
                }
            }

            Ok(route_steps(ctx.world_map,
                           convoy_steps,
                           (&mv_ord.region).into(),
                           dst.into(),
                           vec![]))
        } else {
            Err(ConvoyRouteError::CanOnlyConvoyMove)
        }
    }
}

/// Determines if any valid convoy route exists for the given move order.
pub fn route_exists<'a, A: Adjudicate>(ctx: &'a ResolverContext<'a>,
                                       state: &mut ResolverState<'a, A>,
                                       mv_ord: &MappedMainOrder)
                                       -> bool {
    routes(ctx, state, mv_ord).map(|r| !r.is_empty()).unwrap_or(false)
}

#[cfg(test)]
mod test {
    use order::{ConvoyedMove, Order};
    use geo::{self, RegionKey, ProvinceKey};
    use judge::MappedMainOrder;
    use Nation;
    use UnitType;

    fn convoy(l: &str, f: &str, t: &str) -> MappedMainOrder {
        Order::new(Nation("eng".into()),
                   UnitType::Fleet,
                   RegionKey::new(String::from(l), None),
                   ConvoyedMove::new(RegionKey::new(String::from(f), None),
                                     RegionKey::new(String::from(t), None))
                       .into())
    }

    #[test]
    fn pathfinder() {
        let convoys = vec![
            convoy("ska", "lon", "swe"),
            convoy("eng", "lon", "swe"),
            convoy("nth", "lon", "swe"),
            convoy("nwg", "lon", "swe"),
        ];

        let routes = super::route_steps(geo::standard_map(),
                                 convoys.iter().collect(),
                                 &ProvinceKey::new("lon"),
                                 &ProvinceKey::new("swe"),
                                 vec![]);
        for r in &routes {
            println!("CHAIN");
            for o in r.iter() {
                println!("  {}", o);
            }
        }

        assert_eq!(2, routes.len());
    }
}