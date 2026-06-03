use typst_template::{ToDict, ToValue, Value};

#[derive(ToDict)]
struct Simple {
    a: i64,
    b: String,
}

#[test]
fn fields_map_by_name() {
    let dict = Simple { a: 1, b: "x".into() }.into_dict();
    assert_eq!(dict.get("a").unwrap(), &Value::Int(1));
    assert_eq!(dict.get("b").unwrap(), &Value::Str("x".into()));
}

#[derive(ToDict)]
#[typst(rename_all = "camelCase")]
struct Renamed {
    first_name: String,
    last_name: String,
}

#[test]
fn rename_all_applies() {
    let dict = Renamed { first_name: "a".into(), last_name: "b".into() }.into_dict();
    assert!(dict.get("firstName").is_ok());
    assert!(dict.get("lastName").is_ok());
}

#[derive(ToDict)]
#[typst(rename_all = "lowercase")]
struct Lower {
    foo_bar: i64,
}

#[derive(ToDict)]
#[typst(rename_all = "UPPERCASE")]
struct Upper {
    foo_bar: i64,
}

#[derive(ToDict)]
#[typst(rename_all = "PascalCase")]
struct Pascal {
    foo_bar: i64,
}

#[derive(ToDict)]
#[typst(rename_all = "kebab-case")]
struct Kebab {
    foo_bar: i64,
}

#[derive(ToDict)]
#[typst(rename_all = "SCREAMING_SNAKE_CASE")]
struct ScreamingSnake {
    foo_bar: i64,
}

#[derive(ToDict)]
#[typst(rename_all = "SCREAMING-KEBAB-CASE")]
struct ScreamingKebab {
    foo_bar: i64,
}

#[test]
fn rename_all_rules() {
    assert!(Lower { foo_bar: 1 }.into_dict().get("foobar").is_ok());
    assert!(Upper { foo_bar: 1 }.into_dict().get("FOOBAR").is_ok());
    assert!(Pascal { foo_bar: 1 }.into_dict().get("FooBar").is_ok());
    assert!(Kebab { foo_bar: 1 }.into_dict().get("foo-bar").is_ok());
    assert!(
        ScreamingSnake { foo_bar: 1 }
            .into_dict()
            .get("FOO_BAR")
            .is_ok()
    );
    assert!(
        ScreamingKebab { foo_bar: 1 }
            .into_dict()
            .get("FOO-BAR")
            .is_ok()
    );
}

#[derive(ToDict)]
#[typst(rename_all = "camelCase")]
struct Combo {
    #[typst(rename = "EXACT")]
    some_field: i64,
    other_field: i64,
}

#[test]
fn field_rename_overrides_rename_all() {
    let dict = Combo { some_field: 1, other_field: 2 }.into_dict();
    assert!(dict.get("EXACT").is_ok());
    assert!(dict.get("otherField").is_ok());
}

#[derive(ToDict)]
struct FieldOpts {
    #[typst(rename = "N")]
    n: i64,
    #[typst(skip)]
    _hidden: i64,
}

#[test]
fn field_rename_and_skip() {
    let dict = FieldOpts { n: 5, _hidden: 9 }.into_dict();
    assert_eq!(dict.get("N").unwrap(), &Value::Int(5));
    assert_eq!(dict.len(), 1);
}

fn shout(value: String) -> Value {
    Value::Str(value.to_uppercase().into())
}

#[derive(ToDict)]
struct WithAttr {
    #[typst(with = "shout")]
    s: String,
}

#[test]
fn with_overrides_conversion() {
    let dict = WithAttr { s: "hi".into() }.into_dict();
    assert_eq!(dict.get("s").unwrap(), &Value::Str("HI".into()));
}

#[derive(ToDict)]
struct Inner {
    v: i64,
}

#[derive(ToDict)]
struct Outer {
    inner: Inner,
    list: Vec<Inner>,
    opt: Option<i64>,
}

#[test]
fn nested_types() {
    let dict = Outer { inner: Inner { v: 1 }, list: vec![Inner { v: 2 }], opt: None }.into_dict();
    assert!(matches!(dict.get("inner").unwrap(), Value::Dict(_)));
    assert!(matches!(dict.get("list").unwrap(), Value::Array(_)));
    assert_eq!(dict.get("opt").unwrap(), &Value::None);
}

#[derive(ToDict)]
struct Meta {
    id: i64,
    tag: String,
}

#[derive(ToDict)]
struct Flattened {
    name: String,
    #[typst(flatten)]
    meta: Meta,
}

#[test]
fn flatten_merges_fields() {
    let dict = Flattened { name: "n".into(), meta: Meta { id: 7, tag: "t".into() } }.into_dict();
    // `meta`'s fields sit alongside `name`, not nested under a `meta` key.
    assert!(dict.get("meta").is_err());
    assert_eq!(dict.get("name").unwrap(), &Value::Str("n".into()));
    assert_eq!(dict.get("id").unwrap(), &Value::Int(7));
    assert_eq!(dict.get("tag").unwrap(), &Value::Str("t".into()));
}

#[derive(ToDict)]
struct Wrap<T> {
    value: T,
}

#[test]
fn generic_struct() {
    let dict = Wrap { value: 3_i64 }.into_dict();
    assert_eq!(dict.get("value").unwrap(), &Value::Int(3));
    // Also reachable through ToValue (the derive implements both).
    assert!(matches!(Wrap { value: "s" }.into_value(), Value::Dict(_)));
}
