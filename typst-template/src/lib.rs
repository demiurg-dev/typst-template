//! Generate PDFs with [Typst](https://typst.app) straight from your Rust types.
//!
//! Derive [`ToDict`](macro@ToDict) on a struct and it becomes a Typst input
//! dictionary — feed it to a `.typ` template as `sys.inputs` and render to PDF,
//! with no hand-written dict building. The [`ToValue`] / [`ToDict`] traits do
//! the Rust-to-Typst value conversion, with optional `chrono`, `time`,
//! `rust_decimal`, `serde_json`, and `uuid` support.
//!
//! Rendering uses two parts:
//!
//! - [`WorldBase`] — built once; loads fonts and roots file access at a directory.
//! - [`ConcreteWorld`] — one per document; attaches this generation's `sys.inputs` and any virtual
//!   in-memory files (images, generated assets, …).
//!
//! # Example
//!
//! ```
//! use typst_template::{ToDict, WorldBaseConfig};
//!
//! #[derive(ToDict)]
//! struct Invoice {
//!     client: String,
//!     total: i64,
//! }
//!
//! // A base that only uses the embedded fonts (no system font lookup).
//! let base = WorldBaseConfig::new(".").system_fonts(false).build();
//!
//! let world = base
//!     .concrete("main.typ")
//!     .file("main.typ", "Invoice for #sys.inputs.client: #sys.inputs.total EUR")
//!     .inputs(Invoice { client: "ACME".into(), total: 42 })
//!     .build();
//!
//! let pdf = world.render_pdf_default().output.expect("render succeeds");
//! assert_eq!(&pdf[..4], b"%PDF");
//! ```
//!
//! # Reusing a base
//!
//! Build the [`WorldBase`] once, then derive a [`ConcreteWorld`] per document:
//!
//! ```
//! use typst_template::WorldBaseConfig;
//!
//! let base = WorldBaseConfig::new(".").system_fonts(false).build();
//!
//! for name in ["Alice", "Bob"] {
//!     let pdf = base
//!         .concrete("main.typ")
//!         .file("main.typ", "Hello, #sys.inputs.name!")
//!         .input("name", name)
//!         .build()
//!         .render_pdf_default()
//!         .output
//!         .expect("render succeeds");
//!     assert_eq!(&pdf[..4], b"%PDF");
//! }
//! ```
//!
//! # `datetime.today()`
//!
//! With no `chrono`/`time` feature, `datetime.today()` is disabled by default
//! and returns nothing; set a value with
//! [`today`](ConcreteWorldBuilder::today). With `chrono` or `time` enabled it
//! defaults to the system clock ([`today_system`](ConcreteWorldBuilder::today_system)),
//! and [`today_system_in`](ConcreteWorldBuilder::today_system_in) (with
//! `chrono-tz` or `time-tz`) pins "local" to a DST-aware named zone.
//!
//! Note: with the `time` backend, `today_system` falls back to UTC when the
//! OS-local offset is unavailable (e.g. in a multithreaded process); use a named
//! zone (or `chrono`) for a specific local zone.
//!
//! # Features
//!
//! - `derive` *(default)* — the [`ToDict`](macro@ToDict) derive macro.
//! - `chrono` — [`ToValue`] for `chrono` date/time types, plus a system-clock `datetime.today()`.
//! - `time` — [`ToValue`] for `time` date/time types, plus a system-clock `datetime.today()`.
//!   `chrono` takes precedence if both are enabled.
//! - `chrono-tz` / `time-tz` — named-time-zone support for
//!   [`today_system_in`](ConcreteWorldBuilder::today_system).
//! - `rust_decimal` — [`ToValue`] for `rust_decimal::Decimal`.
//! - `serde_json` — [`ToValue`]/[`ToDict`] for `serde_json` values.
//! - `uuid` — [`ToValue`] for `uuid::Uuid`.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

// Let the `ToDict` derive refer to this crate as `::typst_template` even from
// within the crate's own tests and examples.
extern crate self as typst_template;

mod convert;
mod value;
mod world;

/// Implementation details used by the [`ToDict`](macro@ToDict) derive. Not part
/// of the public API.
#[doc(hidden)]
pub mod __private {
    pub use crate::value::merge_dict;
}

// Re-export the time-zone crates so callers can name the zone passed to
// `today_system_in` without depending on them directly.
#[cfg(feature = "chrono-tz")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono-tz")))]
pub use chrono_tz;
#[cfg(feature = "time-tz")]
#[cfg_attr(docsrs, doc(cfg(feature = "time-tz")))]
pub use time_tz;
pub use typst::diag::{SourceDiagnostic, SourceResult, Warned};
pub use typst::foundations::{Dict, Str, Value};
pub use typst_pdf::PdfOptions;
/// Derives [`ToDict`](trait@ToDict) (and, through it, [`ToValue`]) for a named
/// struct. See the [derive crate](typst_template_derive::ToDict) for the full list
/// of `#[typst(...)]` attributes.
///
/// ```
/// use typst_template::{ToDict, ToValue, Value};
///
/// fn cents(amount: i64) -> Value {
///     Value::Str(format!("{}.{:02}", amount / 100, amount % 100).into())
/// }
///
/// #[derive(ToDict)]
/// struct Meta {
///     locale: String,
/// }
///
/// #[derive(ToDict)]
/// #[typst(rename_all = "camelCase")]
/// struct Invoice {
///     client_name: String,
///     #[typst(rename = "amount", with = "cents")]
///     amount_cents: i64,
///     #[typst(skip)]
///     internal_id: u64,
///     #[typst(flatten)]
///     meta: Meta,
/// }
///
/// let dict = Invoice {
///     client_name: "ACME".into(),
///     amount_cents: 1050,
///     internal_id: 7,
///     meta: Meta { locale: "hr".into() },
/// }
/// .into_dict();
///
/// assert_eq!(dict.get("clientName").unwrap(), &Value::Str("ACME".into()));
/// assert_eq!(dict.get("amount").unwrap(), &Value::Str("10.50".into()));
/// assert_eq!(dict.get("locale").unwrap(), &Value::Str("hr".into())); // flattened
/// assert!(dict.get("internalId").is_err()); // skipped
/// ```
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use typst_template_derive::ToDict;
pub use value::{ToDict, ToValue};
#[cfg(any(feature = "chrono-tz", feature = "time-tz"))]
#[cfg_attr(docsrs, doc(cfg(any(feature = "chrono-tz", feature = "time-tz"))))]
pub use world::Zone;
pub use world::{ConcreteWorld, ConcreteWorldBuilder, WorldBase, WorldBaseConfig};
// Re-export the Typst crates and the handful of types that appear in this
// crate's public API, so downstream users don't need to depend on them
// directly or match versions by hand.
pub use {typst, typst_pdf};
