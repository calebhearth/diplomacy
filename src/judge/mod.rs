//! Contains the logic needed to adjudicate a turn.

pub mod build;
mod calc;
mod convoy;
mod outcome;
mod resolver;
pub mod retreat;
mod rulebook;
mod state_type;
mod strength;
pub mod support;

use std::collections::HashMap;

pub use self::outcome::{OrderOutcome, Outcome};
pub use self::state_type::{OccupationOutcome, OrderState};

pub use self::convoy::ConvoyOutcome;
pub use self::rulebook::AttackOutcome;
pub use self::rulebook::HoldOutcome;
pub use self::support::SupportOutcome;
pub(self) use self::strength::Prevent;

pub use self::resolver::{ResolverContext, ResolverState};
pub use self::rulebook::Rulebook;
use crate::geo::{Border, Map, RegionKey, Terrain};
use crate::order::{BuildOrder, MainCommand, Order, RetreatOrder};
use crate::UnitType;

pub type MappedMainOrder = Order<RegionKey, MainCommand<RegionKey>>;
pub type MappedBuildOrder = BuildOrder<RegionKey>;
pub type MappedRetreatOrder = RetreatOrder<RegionKey>;

/// A clonable container for a rulebook which can be used to adjudicate a turn.
pub trait Adjudicate: Clone {
    /// Determine the success of an order.
    fn adjudicate<'a>(
        &self,
        context: &'a ResolverContext<'a>,
        resolver: &mut ResolverState<'a, Self>,
        order: &'a MappedMainOrder,
    ) -> OrderState;

    fn explain<'a>(
        &self,
        context: &'a ResolverContext<'a>,
        resolver: &mut ResolverState<'a, Self>,
        order: &'a MappedMainOrder,
    ) -> OrderOutcome<'a>;
}

/// Adjudicate a set of orders
pub fn adjudicate<O: IntoIterator<Item = MappedMainOrder>>(
    map: &Map,
    orders: O,
) -> HashMap<MappedMainOrder, OrderState> {
    let ctx = ResolverContext::new(map, orders.into_iter().collect());
    ctx.resolve().into()
}

impl Border {
    fn is_passable_by(&self, unit_type: UnitType) -> bool {
        unit_type.can_occupy(self.terrain())
    }
}

impl UnitType {
    fn can_occupy(self, terrain: Terrain) -> bool {
        match terrain {
            Terrain::Coast => true,
            Terrain::Land => self == UnitType::Army,
            Terrain::Sea => self == UnitType::Fleet,
        }
    }
}
