use serde_json::{Map, Number, Value as Json};
use typst::foundations::{Decimal, Dict, Value};

use crate::{ToDict, ToValue};

impl ToValue for Json {
    fn into_value(self) -> Value {
        match self {
            Json::Null => Value::None,
            Json::Bool(value) => Value::Bool(value),
            Json::Number(number) => number_into_value(number),
            Json::String(value) => Value::Str(value.into()),
            Json::Array(values) => Value::Array(values.into_iter().map(ToValue::into_value).collect()),
            Json::Object(map) => Value::Dict(map.into_dict()),
        }
    }
}

impl ToDict for Map<String, Json> {
    fn into_dict(self) -> Dict {
        let mut dict = Dict::new();
        for (key, value) in self {
            dict.insert(key.into(), value.into_value());
        }
        dict
    }
}

impl ToValue for Map<String, Json> {
    fn into_value(self) -> Value {
        Value::Dict(self.into_dict())
    }
}

fn number_into_value(number: Number) -> Value {
    if let Some(int) = number.as_i64() {
        Value::Int(int)
    } else if let Some(uint) = number.as_u64() {
        i64::try_from(uint)
            .map(Value::Int)
            .unwrap_or(Value::Float(uint as f64))
    } else if let Some(float) = number.as_f64() {
        Value::Float(float)
    } else {
        // Arbitrary-precision number: keep full precision as a decimal, falling
        // back to a string if it doesn't fit.
        match number.to_string().parse::<Decimal>() {
            Ok(decimal) => Value::Decimal(decimal),
            Err(_) => Value::Str(number.to_string().into()),
        }
    }
}
