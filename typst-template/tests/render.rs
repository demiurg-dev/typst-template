use typst_template::WorldBaseConfig;

/// Builds a base that only uses embedded fonts, so tests don't depend on the
/// host's installed fonts.
fn base() -> typst_template::WorldBase {
    WorldBaseConfig::new(".").system_fonts(false).build()
}

#[test]
fn renders_with_inputs_and_virtual_files() {
    let world = base()
        .concrete("main.typ")
        .file(
            "main.typ",
            r#"#import "lib.typ": greet
#greet(sys.inputs.name)"#,
        )
        .file("lib.typ", "#let greet(name) = [Hello, #name!]")
        .input("name", "World")
        .build();

    let pdf = world.render_pdf_default().output.expect("render succeeds");
    assert!(pdf.starts_with(b"%PDF"));
}

#[test]
fn add_file_after_build() {
    let mut world = base()
        .concrete("main.typ")
        .file("main.typ", "#image(\"logo.svg\")")
        .build();
    world.add_file("logo.svg", br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>"#.to_vec());

    assert!(world.render_pdf_default().output.is_ok());
}

#[test]
fn today_disabled_is_none() {
    let world = base()
        .concrete("main.typ")
        .file("main.typ", "#datetime.today().display()")
        .today_disabled()
        .build();
    assert!(world.compile().output.is_err());
}

#[cfg(any(feature = "chrono", feature = "time"))]
#[test]
fn today_defaults_to_system() {
    // With a time feature enabled, `datetime.today()` works without setup.
    let world = base()
        .concrete("main.typ")
        .file("main.typ", "#datetime.today().display()")
        .build();
    assert!(world.compile().output.is_ok());
}

#[test]
fn today_huge_offset_does_not_panic() {
    // A template-supplied offset must never panic the renderer; an
    // unrepresentable date yields `none`, so compilation just fails.
    let world = base()
        .concrete("main.typ")
        .file("main.typ", "#datetime.today(offset: 999999999999999).display()")
        .build();
    assert!(world.compile().output.is_err());
}

#[test]
fn today_fixed_rejects_explicit_offset() {
    use typst_template::typst::foundations::Datetime;

    // A fixed value only answers offset-less requests.
    let world = base()
        .concrete("main.typ")
        .file("main.typ", "#datetime.today(offset: 2).display()")
        .today(Datetime::from_ymd(2026, 6, 2).unwrap())
        .build();
    assert!(world.compile().output.is_err());
}

#[cfg(feature = "chrono-tz")]
#[test]
fn today_system_in_named_zone_chrono() {
    let world = base()
        .concrete("main.typ")
        .file("main.typ", "#datetime.today().display()")
        .today_system_in(typst_template::chrono_tz::Europe::Zagreb)
        .build();
    assert!(world.compile().output.is_ok());
}

#[cfg(feature = "time-tz")]
#[test]
fn today_system_in_named_zone_time() {
    let zagreb = typst_template::time_tz::timezones::get_by_name("Europe/Zagreb").unwrap();
    let world = base()
        .concrete("main.typ")
        .file("main.typ", "#datetime.today().display()")
        .today_system_in(zagreb)
        .build();
    assert!(world.compile().output.is_ok());
}

#[test]
fn today_fixed_is_available() {
    use typst_template::typst::foundations::Datetime;

    let world = base()
        .concrete("main.typ")
        .file("main.typ", "#datetime.today().display()")
        .today(Datetime::from_ymd(2026, 6, 2).unwrap())
        .build();

    assert!(world.compile().output.is_ok());
}
