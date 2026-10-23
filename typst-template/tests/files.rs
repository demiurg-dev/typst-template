//! Files read from a base's root.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use typst_template::{WorldBase, WorldBaseConfig};

/// Imports `x` from `lib.typ` and checks it is the input `x`.
const IMPORTS: &str = "#import \"lib.typ\": x\n#assert.eq(x, sys.inputs.x)";

/// Reads `data.txt` and checks it is the input `x`.
const READS: &str = "#assert.eq(read(\"data.txt\"), sys.inputs.x)";

/// A directory of its own for one test, emptied first.
fn directory(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("typst-template-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes `text` to `path`, modified at `modified`.
fn write(path: &Path, text: &str, modified: SystemTime) {
    fs::write(path, text).unwrap();
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
}

/// Writes both files the documents read, each holding `x`.
fn write_both(dir: &Path, x: &str, modified: SystemTime) {
    write(&dir.join("lib.typ"), &format!("#let x = \"{x}\""), modified);
    write(&dir.join("data.txt"), x, modified);
}

fn based_at(root: &Path) -> WorldBase {
    WorldBaseConfig::new(root).system_fonts(false).build()
}

/// Whether `main` compiles in a new world of `base` with the input `x`.
fn compiles(base: &WorldBase, main: &str, x: &str) -> bool {
    let world = base
        .concrete("main.typ")
        .file("main.typ", main)
        .input("x", x)
        .build();
    world.compile().output.is_ok()
}

/// The first error of compiling `main` in a new world of `base`.
fn error(base: &WorldBase, main: &str) -> String {
    let world = base.concrete("main.typ").file("main.typ", main).build();
    let errors = world.compile().output.expect_err("fails");
    errors[0].message.to_string()
}

#[test]
fn a_file_is_read_again_when_its_length_or_time_changes() {
    let dir = directory("changes");
    let base = based_at(&dir);
    let then = SystemTime::now() - Duration::from_secs(60);
    write_both(&dir, "one", then);
    assert!(compiles(&base, IMPORTS, "one") && compiles(&base, READS, "one"));
    write_both(&dir, "three", then);
    assert!(compiles(&base, IMPORTS, "three") && compiles(&base, READS, "three"));
    write_both(&dir, "seven", then + Duration::from_secs(10));
    assert!(compiles(&base, IMPORTS, "seven") && compiles(&base, READS, "seven"));
}

#[test]
fn a_file_of_the_same_length_and_time_is_not_read_again() {
    let dir = directory("unchanged");
    let base = based_at(&dir);
    let then = SystemTime::now() - Duration::from_secs(60);
    write_both(&dir, "one", then);
    assert!(compiles(&base, IMPORTS, "one") && compiles(&base, READS, "one"));
    write_both(&dir, "two", then);
    assert!(compiles(&base, IMPORTS, "one") && compiles(&base, READS, "one"));
    assert!(compiles(&base.clone(), IMPORTS, "one"), "clones share what was read");
    let other = based_at(&dir);
    assert!(compiles(&other, IMPORTS, "two") && compiles(&other, READS, "two"));
}

#[test]
fn a_source_first_read_as_bytes_is_parsed_from_them() {
    let dir = directory("bytes-first");
    let base = based_at(&dir);
    let then = SystemTime::now() - Duration::from_secs(60);
    write_both(&dir, "one", then);
    let reads_lib = "#assert.eq(read(\"lib.typ\"), \"#let x = \\\"one\\\"\")";
    assert!(compiles(&base, reads_lib, ""));
    write_both(&dir, "two", then);
    assert!(compiles(&base, IMPORTS, "one"));
}

#[test]
fn a_directory_or_a_missing_file_is_refused() {
    let dir = directory("refused");
    fs::create_dir(dir.join("sub")).unwrap();
    let base = based_at(&dir);
    assert_eq!(error(&base, "#read(\"sub\")"), "failed to load file (is a directory)");
    assert!(error(&base, "#read(\"missing.txt\")").starts_with("file not found"));
    assert!(error(&base, "#import \"missing.typ\"").starts_with("file not found"));
}
