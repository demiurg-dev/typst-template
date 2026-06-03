//! Conversion from Rust data into Typst values.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

use typst::foundations::{Bytes, Datetime, Decimal, Dict, Str, Value};

/// Converts a Rust value into a Typst [`Value`].
///
/// This is the leaf conversion used by [`ToDict`] and by the
/// [`ToDict`](macro@crate::ToDict) derive. It is implemented for the common
/// scalar, collection, and Typst value types; enable cargo features for
/// `chrono`, `rust_decimal`, `serde_json`, and `uuid` to convert those too.
///
/// ```
/// use typst_template::{ToValue, Value};
///
/// assert_eq!(42_i64.into_value(), Value::Int(42));
/// assert_eq!(Some("hi").into_value(), Value::Str("hi".into()));
/// assert_eq!(Option::<i64>::None.into_value(), Value::None);
/// ```
pub trait ToValue {
    /// Consumes `self` and produces the Typst value.
    fn into_value(self) -> Value;
}

/// Converts a Rust struct into a Typst [`Dict`].
///
/// Derive it with `#[derive(ToDict)]` (see the
/// [`ToDict`](macro@crate::ToDict) macro), which also implements [`ToValue`] by
/// wrapping the dict, so derived types can nest.
///
/// ```
/// use typst_template::{ToDict, ToValue, Value};
///
/// #[derive(ToDict)]
/// struct Point {
///     x: i64,
///     y: i64,
/// }
///
/// let dict = Point { x: 1, y: 2 }.into_dict();
/// assert_eq!(dict.get("x").unwrap(), &Value::Int(1));
/// ```
pub trait ToDict {
    /// Consumes `self` and produces the Typst dictionary.
    fn into_dict(self) -> Dict;
}

// ── Pass-through Typst types ────────────────────────────────────────────────

impl ToValue for Value {
    fn into_value(self) -> Value {
        self
    }
}

impl ToValue for Str {
    fn into_value(self) -> Value {
        Value::Str(self)
    }
}

impl ToValue for Bytes {
    fn into_value(self) -> Value {
        Value::Bytes(self)
    }
}

impl ToValue for Datetime {
    fn into_value(self) -> Value {
        Value::Datetime(self)
    }
}

impl ToValue for Decimal {
    fn into_value(self) -> Value {
        Value::Decimal(self)
    }
}

impl ToDict for Dict {
    fn into_dict(self) -> Dict {
        self
    }
}

/// Inserts every entry of `source` into `target`. Used by the `flatten`
/// attribute of the [`ToDict`](macro@crate::ToDict) derive.
#[doc(hidden)]
pub fn merge_dict(target: &mut Dict, source: Dict) {
    for (key, value) in source {
        target.insert(key, value);
    }
}

impl ToValue for Dict {
    fn into_value(self) -> Value {
        Value::Dict(self)
    }
}

// ── Scalars ─────────────────────────────────────────────────────────────────

impl ToValue for () {
    fn into_value(self) -> Value {
        Value::None
    }
}

impl ToValue for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }
}

/// Integers map to Typst `int` (`i64`), falling back to `float` when they don't
/// fit.
macro_rules! impl_into_value_int {
    ($($ty:ty),*) => {$(
        impl ToValue for $ty {
            fn into_value(self) -> Value {
                match i64::try_from(self) {
                    Ok(int) => Value::Int(int),
                    Err(_) => Value::Float(self as f64),
                }
            }
        }
    )*};
}

impl_into_value_int!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, i128, u128);

impl ToValue for f32 {
    fn into_value(self) -> Value {
        Value::Float(self.into())
    }
}

impl ToValue for f64 {
    fn into_value(self) -> Value {
        Value::Float(self)
    }
}

impl ToValue for String {
    fn into_value(self) -> Value {
        Value::Str(self.into())
    }
}

impl ToValue for &str {
    fn into_value(self) -> Value {
        Value::Str(self.into())
    }
}

impl ToValue for Cow<'_, str> {
    fn into_value(self) -> Value {
        Value::Str(self.into_owned().into())
    }
}

impl ToValue for char {
    fn into_value(self) -> Value {
        Value::Str(self.to_string().into())
    }
}

// ── Containers ──────────────────────────────────────────────────────────────

impl<T: ToValue> ToValue for Option<T> {
    fn into_value(self) -> Value {
        match self {
            Some(value) => value.into_value(),
            None => Value::None,
        }
    }
}

impl<T: ToValue> ToValue for Vec<T> {
    fn into_value(self) -> Value {
        Value::Array(self.into_iter().map(ToValue::into_value).collect())
    }
}

impl<T: ToValue, const N: usize> ToValue for [T; N] {
    fn into_value(self) -> Value {
        Value::Array(self.into_iter().map(ToValue::into_value).collect())
    }
}

/// String-keyed maps map to a Typst dict.
macro_rules! impl_into_dict_map {
    ($ty:ident $(, $bound:ident)?) => {
        impl<K: Into<Str>, V: ToValue $(, $bound)?> ToDict for $ty<K, V $(, $bound)?> {
            fn into_dict(self) -> Dict {
                let mut dict = Dict::new();
                for (key, value) in self {
                    dict.insert(key.into(), value.into_value());
                }
                dict
            }
        }

        impl<K: Into<Str>, V: ToValue $(, $bound)?> ToValue for $ty<K, V $(, $bound)?> {
            fn into_value(self) -> Value {
                Value::Dict(self.into_dict())
            }
        }
    };
}

impl_into_dict_map!(BTreeMap);
impl_into_dict_map!(HashMap, S);
