//! `ToValue`/`ToDict` implementations for optional ecosystem types.
//!
//! Each submodule is behind the matching cargo feature.

#[cfg(feature = "chrono")]
mod chrono;
#[cfg(feature = "rust_decimal")]
mod rust_decimal;
#[cfg(feature = "serde_json")]
mod serde_json;
#[cfg(feature = "time")]
mod time;
#[cfg(feature = "uuid")]
mod uuid;

/// Builds a Typst [`Datetime`](typst::foundations::Datetime) from calendar
/// components, shared by the date/time conversions and the system clock.
#[cfg(any(feature = "chrono", feature = "time"))]
pub(crate) fn datetime(
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
) -> Option<typst::foundations::Datetime> {
    typst::foundations::Datetime::from_ymd_hms(year, month, day, hour, minute, second)
}
