use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

use typst_template::{ToValue, Value};

#[test]
fn scalars() {
    assert_eq!(7_u8.into_value(), Value::Int(7));
    assert_eq!(true.into_value(), Value::Bool(true));
    assert_eq!(1.5_f64.into_value(), Value::Float(1.5));
    assert_eq!('c'.into_value(), Value::Str("c".into()));
    assert_eq!(().into_value(), Value::None);

    // Strings via every accepted form.
    assert_eq!(String::from("s").into_value(), Value::Str("s".into()));
    assert_eq!("s".into_value(), Value::Str("s".into()));
    assert_eq!(Cow::Borrowed("s").into_value(), Value::Str("s".into()));
}

#[test]
fn ints_too_large_for_i64_fall_back_to_float() {
    assert_eq!(u64::MAX.into_value(), Value::Float(u64::MAX as f64));
    assert_eq!(i128::MAX.into_value(), Value::Float(i128::MAX as f64));
}

#[test]
fn containers() {
    assert_eq!(vec![1_i64, 2].into_value(), Value::Array([Value::Int(1), Value::Int(2)].into_iter().collect()));
    assert_eq!([1_i64, 2].into_value(), Value::Array([Value::Int(1), Value::Int(2)].into_iter().collect()));
    assert_eq!(Some(1_i64).into_value(), Value::Int(1));
    assert_eq!(Option::<i64>::None.into_value(), Value::None);

    let mut btree = BTreeMap::new();
    btree.insert("k", 1_i64);
    assert!(matches!(btree.into_value(), Value::Dict(_)));

    let mut hash = HashMap::new();
    hash.insert("k", 1_i64);
    assert!(matches!(hash.into_value(), Value::Dict(_)));
}

#[cfg(feature = "rust_decimal")]
#[test]
fn rust_decimal_value() {
    use std::str::FromStr;

    let value = rust_decimal::Decimal::from_str("1.50")
        .unwrap()
        .into_value();
    assert_eq!(value, Value::Decimal("1.50".parse().unwrap()));
}

#[cfg(feature = "chrono")]
#[test]
fn chrono_value() {
    use typst_template::typst::foundations::Datetime;

    let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 2).unwrap();
    assert_eq!(date.into_value(), Value::Datetime(Datetime::from_ymd(2026, 6, 2).unwrap()));
}

#[cfg(feature = "chrono")]
#[test]
fn chrono_year_out_of_typst_range_is_none() {
    // Typst dates are limited to years ±9999; chrono allows far more. Such a
    // date must convert to `none`, not panic.
    let date = chrono::NaiveDate::from_ymd_opt(20000, 1, 1).unwrap();
    assert_eq!(date.into_value(), Value::None);
}

#[cfg(feature = "time")]
#[test]
fn time_value() {
    use time::{Date, Month, PrimitiveDateTime, Time};
    use typst_template::typst::foundations::Datetime;

    let date = Date::from_calendar_date(2026, Month::June, 2).unwrap();
    let time = Time::from_hms(8, 30, 0).unwrap();

    assert_eq!(date.into_value(), Value::Datetime(Datetime::from_ymd(2026, 6, 2).unwrap()));
    assert_eq!(time.into_value(), Value::Datetime(Datetime::from_hms(8, 30, 0).unwrap()));

    let datetime = PrimitiveDateTime::new(date, time);
    let expected = Datetime::from_ymd_hms(2026, 6, 2, 8, 30, 0).unwrap();
    assert_eq!(datetime.into_value(), Value::Datetime(expected));
    assert_eq!(datetime.assume_utc().into_value(), Value::Datetime(expected));
}

#[cfg(feature = "uuid")]
#[test]
fn uuid_value() {
    let id = uuid::Uuid::nil();
    assert_eq!(id.into_value(), Value::Str("00000000-0000-0000-0000-000000000000".into()));
}

#[cfg(feature = "serde_json")]
#[test]
fn serde_json_value() {
    use typst_template::ToDict;

    let json = serde_json::json!({ "a": 1, "b": [true, "x"], "c": null });
    let dict = json.as_object().unwrap().clone().into_dict();
    assert_eq!(dict.get("a").unwrap(), &Value::Int(1));
    assert!(matches!(dict.get("b").unwrap(), Value::Array(_)));
    assert_eq!(dict.get("c").unwrap(), &Value::None);
}

#[cfg(feature = "serde_json")]
#[test]
fn serde_json_numbers() {
    // Exercises each branch of the number conversion.
    assert_eq!(serde_json::json!(-3).into_value(), Value::Int(-3));
    assert_eq!(serde_json::json!(1.5).into_value(), Value::Float(1.5));
    // A u64 beyond i64's range falls back to float.
    assert_eq!(serde_json::json!(u64::MAX).into_value(), Value::Float(u64::MAX as f64));
}
