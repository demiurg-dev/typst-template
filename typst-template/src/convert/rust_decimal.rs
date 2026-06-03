use typst::foundations::{Decimal, Value};

use crate::ToValue;

impl ToValue for rust_decimal::Decimal {
    fn into_value(self) -> Value {
        // Typst has its own 128-bit decimal; round-trip through the string form
        // to preserve the exact value.
        let decimal: Decimal = self
            .to_string()
            .parse()
            .expect("decimal string is well-formed");
        Value::Decimal(decimal)
    }
}
