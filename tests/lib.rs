#![cfg(test)]

extern crate diplomacy;

mod util;

use diplomacy::geo;
use diplomacy::judge::{adjudicate, OrderState};
use diplomacy::order::{ConvoyedMove, MainCommand, MoveCommand, Order, SupportedOrder};

use diplomacy::{Nation, UnitType};

use crate::util::*;

#[test]
fn dipmath_figure9() {
    let map = geo::standard_map();
    let eng = Nation::from("eng");
    let ger = Nation::from("ger");
    let rus = Nation::from("rus");

    let orders = vec![
        Order::new(
            eng,
            UnitType::Fleet,
            reg("nwg"),
            MoveCommand::new(reg("nth")).into(),
        ),
        Order::new(
            ger.clone(),
            UnitType::Fleet,
            reg("nth"),
            MoveCommand::new(reg("nwy")).into(),
        ),
        Order::new(
            rus,
            UnitType::Fleet,
            reg("nwy"),
            MoveCommand::new(reg("nwg")).into(),
        ),
        Order::new(
            ger,
            UnitType::Fleet,
            reg("ska"),
            MainCommand::Support(SupportedOrder::Move(
                UnitType::Fleet,
                reg("nth"),
                reg("nwy"),
            )),
        ),
    ];

    let result = adjudicate(&map, orders);

    for (_, r) in result.iter() {
        assert_eq!(&OrderState::Succeeds, r);
    }
}

#[test]
fn dipmath_figure6() {
    let aus = Nation::from("aus");
    let ger = Nation::from("ger");
    let rus = Nation::from("rus");

    let orders = vec![
        Order::new(
            ger.clone(),
            UnitType::Army,
            reg("ber"),
            MoveCommand::new(reg("sil")).into(),
        ),
        Order::new(
            ger.clone(),
            UnitType::Army,
            reg("mun"),
            MainCommand::Support(SupportedOrder::Move(UnitType::Army, reg("ber"), reg("sil"))),
        ),
        Order::new(
            rus,
            UnitType::Army,
            reg("war"),
            MoveCommand::new(reg("sil")).into(),
        ),
        Order::new(
            aus,
            UnitType::Army,
            reg("boh"),
            MoveCommand::new(reg("sil")).into(),
        ),
    ];

    assert!(geo::standard_map()
        .find_border_between(&reg("ber"), &reg("sil"))
        .is_some());
    assert!(geo::standard_map()
        .find_border_between(&reg("war"), &reg("sil"))
        .is_some());
    assert!(geo::standard_map()
        .find_border_between(&reg("sil"), &reg("boh"))
        .is_some());

    let result = adjudicate(geo::standard_map(), orders);
    for (o, r) in &result {
        if o.nation == ger {
            assert_eq!(r, &OrderState::Succeeds);
        } else {
            assert_eq!(r, &OrderState::Fails);
        }
    }
}

#[test]
fn dipmath_figure16() {
    use diplomacy::UnitType::*;

    let tur = Nation::from("tur");
    let aus = Nation::from("aus");
    let ita = Nation::from("ita");

    let orders = vec![
        Order::new(
            tur.clone(),
            Fleet,
            reg("aeg"),
            MoveCommand::new(reg("ion")).into(),
        ),
        Order::new(
            tur,
            Fleet,
            reg("gre"),
            SupportedOrder::Move(UnitType::Fleet, reg("aeg"), reg("ion")).into(),
        ),
        Order::new(
            aus,
            Fleet,
            reg("alb"),
            SupportedOrder::Move(UnitType::Fleet, reg("aeg"), reg("ion")).into(),
        ),
        Order::new(
            ita.clone(),
            Army,
            reg("tun"),
            MoveCommand::new(reg("gre")).into(),
        ),
        Order::new(
            ita.clone(),
            Fleet,
            reg("ion"),
            ConvoyedMove::new(reg("tun"), reg("gre")).into(),
        ),
    ];

    let state = adjudicate(geo::standard_map(), orders);
    for (o, r) in &state {
        if o.nation == ita {
            assert_eq!(r, &OrderState::Fails);
        } else {
            assert_eq!(r, &OrderState::Succeeds);
        }
    }
}
