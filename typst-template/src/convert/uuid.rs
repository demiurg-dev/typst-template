use typst::foundations::Value;

use crate::ToValue;

impl ToValue for uuid::Uuid {
    fn into_value(self) -> Value {
        Value::Str(self.to_string().into())
    }
}
