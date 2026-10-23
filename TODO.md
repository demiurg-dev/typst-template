# TODO

## Typst package support

Imports like `#import "@preview/cetz:0.3.4"` give a `FileId` with
`Some(PackageSpec)`. We ignore it (`load_file` only resolves vpaths against the
project root), so package imports break.

- Branch `load_file` on `id.package()`; for `Some`, resolve the vpath inside the
  package root from `typst_kit::package::PackageStorage::prepare_package`
  (`typst-kit` `packages` feature). Cache loaded `Source`/`Bytes`.
- Namespaces: `@preview` downloads + caches; `@local`/any other namespace is
  data-dir only (data dir wins over cache); no private-registry routing exists.
- Gate behind a non-default `packages` feature. Builder knobs: `packages(bool)`,
  `package_cache_dir`, `package_data_dir`, `offline(bool)`, progress callback.
  Default stays project-files-only, no network.
- Issues: offline mode + reproducible/pre-seeded CI cache; trust-on-first-use;
  blocking download I/O in sync `World::file` (prefer a prepare-packages
  preflight or compile on a blocking thread); exact-version pins (no range
  solving); clear errors for unknown namespace / missing version.
