use time::{Date, OffsetDateTime, PrimitiveDateTime, Time};
use typst::foundations::{Datetime, Value};

use crate::ToValue;
use crate::convert::datetime;

// Dates outside Typst's representable range (years beyond ±9999, reachable only
// with `time`'s `large-dates` feature) convert to `none` rather than panicking.
fn ymd_hms(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Value {
    datetime(year, month, day, hour, minute, second).map_or(Value::None, Value::Datetime)
}

impl ToValue for Date {
    fn into_value(self) -> Value {
        Datetime::from_ymd(self.year(), u8::from(self.month()), self.day()).map_or(Value::None, Value::Datetime)
    }
}

impl ToValue for Time {
    fn into_value(self) -> Value {
        Datetime::from_hms(self.hour(), self.minute(), self.second()).map_or(Value::None, Value::Datetime)
    }
}

impl ToValue for PrimitiveDateTime {
    fn into_value(self) -> Value {
        ymd_hms(self.year(), u8::from(self.month()), self.day(), self.hour(), self.minute(), self.second())
    }
}

/// Uses the value's own UTC offset.
impl ToValue for OffsetDateTime {
    fn into_value(self) -> Value {
        ymd_hms(self.year(), u8::from(self.month()), self.day(), self.hour(), self.minute(), self.second())
    }
}
