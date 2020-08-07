//! Builders for different variations on the Diplomacy rules.
//!
//! The core of the Diplomacy crate is built to support the main game, but rule variations are supported
//! through the phase-specific `Adjudicate` traits.

mod standard;

pub use self::standard::Standard;

/// Get the standard game of Diplomacy, without any customizations.
pub fn standard() -> Standard<'static> {
    Standard::default()
}
