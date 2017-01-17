#![cfg(test)]

extern crate diplomacy;
mod util;
mod basic;

use diplomacy::order::{Order, MainCommand, SupportedOrder};
use diplomacy::geo;
use diplomacy::judge::{adjudicate, OrderState};

use diplomacy::{Nation, UnitType};

use util::*;

#[test]
fn it_works() {
    
}

#[test]
fn dipmath_figure9() {
    let map = geo::standard_map();
    let eng = Nation("eng".into());
    let ger = Nation("ger".into());
    let rus = Nation("rus".into());
    
    let orders = vec![
        Order::new(eng, UnitType::Fleet, reg("nwg"), MainCommand::Move(reg("nth"))),
        Order::new(ger.clone(), UnitType::Fleet, reg("nth"), MainCommand::Move(reg("nwy"))),
        Order::new(rus, UnitType::Fleet, reg("nwy"), MainCommand::Move(reg("nwg"))),
        Order::new(ger.clone(), UnitType::Fleet, reg("ska"), MainCommand::Support(SupportedOrder::Move(reg("nth"), reg("nwy")))),
    ];
    
    let result = adjudicate(&map, orders);
    
    for (_, r) in result.iter() {
        assert_eq!(&OrderState::Succeeds, r);
    }
}

#[test]
fn dipmath_figure6() {
    let aus = Nation("aus".into());
    let ger = Nation("ger".into());
    let rus = Nation("rus".into());
    
    let orders = vec![
        Order::new(ger.clone(), UnitType::Army, reg("ber"), MainCommand::Move(reg("sil"))),
        Order::new(ger.clone(), UnitType::Army, reg("mun"), MainCommand::Support(SupportedOrder::Move(reg("ber"), reg("sil")))),
        Order::new(rus, UnitType::Army, reg("war"), MainCommand::Move(reg("sil"))),
        Order::new(aus, UnitType::Army, reg("boh"), MainCommand::Move(reg("sil")))
    ];
    
    assert!(geo::standard_map().find_border_between(&reg("ber"), &reg("sil")).is_some());
    assert!(geo::standard_map().find_border_between(&reg("war"), &reg("sil")).is_some());
    assert!(geo::standard_map().find_border_between(&reg("sil"), &reg("boh")).is_some());
    
    let result = adjudicate(geo::standard_map(), orders);
    for (o, r) in &result {
        if o.nation == ger {
            assert_eq!(r, &OrderState::Succeeds);
        } else {
            assert_eq!(r, &OrderState::Fails);
        }
    }
}