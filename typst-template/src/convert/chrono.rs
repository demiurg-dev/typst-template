use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use typst::foundations::{Datetime, Value};

use crate::ToValue;
use crate::convert::datetime;

// Dates outside Typst's representable range (years beyond ±9999) convert to
// `none` rather than panicking.
fn from_naive(value: &NaiveDateTime) -> Value {
    datetime(
        value.year(),
        value.month() as u8,
        value.day() as u8,
        value.hour() as u8,
        value.minute() as u8,
        value.second() as u8,
    )
    .map_or(Value::None, Value::Datetime)
}

impl ToValue for NaiveDateTime {
    fn into_value(self) -> Value {
        from_naive(&self)
    }
}

impl ToValue for NaiveDate {
    fn into_value(self) -> Value {
        Datetime::from_ymd(self.year(), self.month() as u8, self.day() as u8).map_or(Value::None, Value::Datetime)
    }
}

impl ToValue for NaiveTime {
    fn into_value(self) -> Value {
        Datetime::from_hms(self.hour() as u8, self.minute() as u8, self.second() as u8)
            .map_or(Value::None, Value::Datetime)
    }
}

/// Uses the value's local wall-clock time.
impl<Tz: TimeZone> ToValue for DateTime<Tz> {
    fn into_value(self) -> Value {
        from_naive(&self.naive_local())
    }
}
