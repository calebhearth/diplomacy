use std::str::FromStr;

use geo::RegionKey;
use order::{Order, Command, MainCommand, BuildCommand, SupportedOrder, ConvoyedMove,
            RetreatCommand};
use ::Nation;

mod error;

pub use self::error::{Error, ErrorKind};

/// A parser which operates on whitespace-delimited words from an input string.
pub trait FromWords: Sized {
    /// The associated error which can be returned from parsing.
    type Err;

    /// Performs the conversion.
    fn from_words(w: &[&str]) -> Result<Self, Self::Err>;
}

type ParseResult<T> = Result<T, Error>;

impl<C: Command<RegionKey> + FromWords<Err = Error>> FromStr for Order<RegionKey, C> {
    type Err = Error;

    fn from_str(s: &str) -> ParseResult<Self> {
        let words = s.split_whitespace().collect::<Vec<_>>();

        let nation = Nation(words[0].trim_right_matches(":").into());
        let unit_type = words[1].parse()?;
        let location = words[2].parse()?;
        let cmd = C::from_words(&words[3..])?;

        Ok(Order {
            nation: nation,
            unit_type: unit_type,
            region: location,
            command: cmd,
        })
    }
}

impl FromWords for MainCommand<RegionKey> {
    type Err = Error;

    fn from_words(words: &[&str]) -> ParseResult<Self> {
        match &(words[0].to_lowercase())[..] {
            "holds" | "hold" => Ok(MainCommand::Hold),
            "->" => Ok(MainCommand::Move(words[1].parse()?)),
            "supports" => Ok(SupportedOrder::from_words(&words[1..])?.into()),
            "convoys" => Ok(ConvoyedMove::from_words(&words[1..])?.into()),
            cmd => Err(Error::new(ErrorKind::UnknownCommand, cmd)),
        }
    }
}

impl FromWords for SupportedOrder<RegionKey> {
    type Err = Error;

    fn from_words(w: &[&str]) -> ParseResult<SupportedOrder<RegionKey>> {
        match w.len() {
            1 => Ok(SupportedOrder::Hold(w[0].parse()?)),
            3 => Ok(SupportedOrder::Move(w[0].parse()?, w[2].parse()?)),
            _ => Err(Error::new(ErrorKind::MalformedSupport, w.join(" "))),
        }
    }
}

impl FromWords for ConvoyedMove<RegionKey> {
    type Err = Error;

    fn from_words(w: &[&str]) -> ParseResult<Self> {
        if w.len() == 3 {
            Ok(ConvoyedMove::new(w[0].parse()?, w[2].parse()?))
        } else {
            Err(Error::new(ErrorKind::MalformedConvoy, w.join(" ")))
        }
    }
}

impl FromWords for RetreatCommand<RegionKey> {
    type Err = Error;

    fn from_words(w: &[&str]) -> ParseResult<Self> {
        match &w[0].to_lowercase()[..] {
            "hold" | "holds" => Ok(RetreatCommand::Hold),
            "->" => Ok(RetreatCommand::Move(w[1].parse()?)),
            cmd => Err(Error::new(ErrorKind::UnknownCommand, cmd)),
        }
    }
}

impl FromWords for BuildCommand {
    type Err = Error;

    fn from_words(w: &[&str]) -> ParseResult<Self> {
        match &w[0].to_lowercase()[..] {
            "build" => Ok(BuildCommand::Build),
            "disband" => Ok(BuildCommand::Disband),
            cmd => Err(Error::new(ErrorKind::UnknownCommand, cmd)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use order::{MainCommand, Order};
    use geo::RegionKey;

    type OrderParseResult = Result<Order<RegionKey, MainCommand<RegionKey>>, Error>;

    #[test]
    fn hold() {
        let h_order: OrderParseResult = "AUS: F Tri hold".parse();
        println!("{}", h_order.unwrap());
    }

    #[test]
    fn army_move() {
        let m_order: OrderParseResult = "ENG: A Lon -> Bel".parse();
        println!("{}", m_order.unwrap());
    }
}