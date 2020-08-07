mod standard;

pub use self::standard::Standard;

/// Get the standard game of Diplomacy, without any customizations.
pub fn standard() -> Standard<'static> {
    Standard::default()
}
