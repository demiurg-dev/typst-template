//! Files served by resolvers mounted in a concrete world.

use std::sync::{Arc, Mutex};

use typst_template::{Bytes, ConcreteWorldBuilder, FileError, FileResult, WorldBase, WorldBaseConfig};

/// The paths a resolver was asked for.
type Asked = Arc<Mutex<Vec<String>>>;

fn base() -> WorldBase {
    WorldBaseConfig::new(".").system_fonts(false).build()
}

/// A world whose main file is `main`.
fn concrete(main: &str) -> ConcreteWorldBuilder {
    base().concrete("main.typ").file("main.typ", main)
}

/// A resolver serving `lib.typ` and `sub/lib.typ`, each defining `x` as `x`,
/// that notes every path it is asked for in `asked`.
fn library(x: &'static str, asked: &Asked) -> impl Fn(&str) -> FileResult<Bytes> + Send + Sync + 'static {
    let asked = asked.clone();
    move |path: &str| {
        asked.lock().unwrap().push(path.to_owned());
        match path {
            "lib.typ" | "sub/lib.typ" => Ok(Bytes::from_string(format!("#let x = \"{x}\""))),
            _ => Err(FileError::NotFound(path.into())),
        }
    }
}

fn asked(asked: &Asked) -> Vec<String> {
    asked.lock().unwrap().clone()
}

/// Checks that importing `path` gives `x`.
fn imports(path: &str, x: &str) -> String {
    format!("#import \"{path}\": x\n#assert.eq(x, \"{x}\")\n")
}

#[test]
fn a_mount_serves_the_files_below_it() {
    let noted = Asked::default();
    let main = imports("/stored/sub/lib.typ", "a") + "#assert.eq(read(\"/stored/lib.typ\"), \"#let x = \\\"a\\\"\")";
    let world = concrete(&main)
        .mount("/stored", library("a", &noted))
        .build();
    assert!(world.compile().output.is_ok());
    assert_eq!(asked(&noted), ["sub/lib.typ", "lib.typ"]);

    let world = concrete("#import \"/stored/missing.typ\"")
        .mount("stored/", library("a", &noted))
        .build();
    let errors = world.compile().output.expect_err("fails");
    assert_eq!(errors[0].message, "file not found (searched at missing.typ)");
}

#[test]
fn a_path_beside_a_mount_or_the_mount_itself_is_the_base_s() {
    let noted = Asked::default();
    for main in ["#import \"/storedx/lib.typ\"", "#read(\"/stored\")", "#import \"/lib.typ\""] {
        let world = concrete(main)
            .mount("/stored", library("a", &noted))
            .build();
        assert!(world.compile().output.is_err(), "{main}");
    }
    assert!(asked(&noted).is_empty(), "{:?}", asked(&noted));
}

#[test]
fn the_innermost_mount_answers_whichever_comes_first() {
    let noted = Asked::default();
    let main = imports("/a/b/lib.typ", "inner") + &imports("/a/lib.typ", "outer");
    let outer_first = concrete(&main)
        .mount("/a", library("outer", &noted))
        .mount("/a/b", library("inner", &noted))
        .build();
    assert!(outer_first.compile().output.is_ok());
    let inner_first = concrete(&main)
        .mount("/a/b", library("inner", &noted))
        .mount("/a", library("outer", &noted))
        .build();
    assert!(inner_first.compile().output.is_ok());
}

#[test]
fn a_path_mounted_again_takes_the_last_resolver() {
    let noted = Asked::default();
    let world = concrete(&imports("/stored/lib.typ", "last"))
        .mount("/stored", library("first", &noted))
        .mount("/stored", library("last", &noted))
        .build();
    assert!(world.compile().output.is_ok());
}

#[test]
fn a_mount_at_the_root_serves_every_file_but_the_virtual_ones() {
    let noted = Asked::default();
    let world = concrete(&imports("/lib.typ", "root"))
        .mount("/", library("root", &noted))
        .build();
    assert!(world.compile().output.is_ok());
    assert_eq!(asked(&noted), ["lib.typ"]);
}

#[test]
fn a_virtual_file_comes_before_a_mount() {
    let noted = Asked::default();
    let world = concrete(&imports("/stored/lib.typ", "virtual"))
        .file("stored/lib.typ", "#let x = \"virtual\"")
        .mount("/stored", library("mounted", &noted))
        .build();
    assert!(world.compile().output.is_ok());
    assert!(asked(&noted).is_empty());
}

#[test]
fn a_world_asks_for_each_file_once() {
    let noted = Asked::default();
    let main = imports("/stored/lib.typ", "a") + "#read(\"/stored/lib.typ\")\n#import \"/stored/none.typ\"";
    let world = concrete(&main)
        .mount("/stored", library("a", &noted))
        .build();
    assert!(world.compile().output.is_err());
    assert!(world.compile().output.is_err());
    assert_eq!(asked(&noted), ["lib.typ", "none.typ"]);
}

#[test]
fn a_package_is_not_looked_for_in_a_mount() {
    let noted = Asked::default();
    let world = concrete("#import \"@preview/nothing:0.1.0\"")
        .mount("/", library("a", &noted))
        .build();
    assert!(world.compile().output.is_err());
    assert!(asked(&noted).is_empty(), "{:?}", asked(&noted));
}

#[test]
fn a_file_added_after_building_replaces_what_the_world_gave() {
    let mut world = concrete(&imports("lib.typ", "two"))
        .file("lib.typ", "#let x = \"one\"")
        .build();
    assert!(world.compile().output.is_err());
    world.add_file("lib.typ", "#let x = \"two\"");
    assert!(world.compile().output.is_ok());
}
