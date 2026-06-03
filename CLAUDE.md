# typst_template — notes for Claude

Rust workspace providing a reusable, publishable library for generating PDFs
with [Typst](https://typst.app).

## Crates

- `typst-template` — the library: a Typst `World` split into a long-lived
  **base** (real files + fonts) and a cheap per-generation **concrete** world
  (virtual in-memory files + `sys.inputs`), plus the `ToValue`/`ToDict` traits
  for turning Rust data into Typst values.
- `typst-template-derive` — proc-macro crate providing `#[derive(ToDict)]`. It is
  re-exported from `typst-template` behind the default `derive` feature; users
  depend only on `typst-template`.

Both crates are published to crates.io, so everything public is documented with
tested examples.

## Conventions

- **Comments**: concise and clear. Explain *what the code does*, not the
  internal design decisions behind it. No long-winded rationale.
- **Type paths**: don't fully-qualify types. Prefer importing and using the
  short name. Only qualify to disambiguate a real clash, or for idioms that
  read better qualified (e.g. `std::cmp::min(a, b)` over a free `min`, since
  `a.max(b)` is the readable form for the method case).
- **Optional integrations** (chrono, rust_decimal, serde_json, uuid, serde)
  are behind cargo features. Keep the default build lean; gate anything that
  is not integral.
- **API guidelines**: aspire to follow the
  [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) where they
  aren't excessive for a crate this size (e.g. `Debug` on public types,
  documented items, additive features, sensible conversions).

## Workflow

- `make check` runs lint + format-check + tests. Clippy (`-D warnings`) is the
  checkpoint — there is no bare `cargo check` step.
- `make format` runs `cargo sort` + nightly `cargo fmt`.
- `make doc` builds the all-features docs locally.

## Git

- Never add a `Co-Authored-By` (or any AI attribution) trailer to commits.

## Working with sub-agents

- Never take a sub-agent's output at face value. Always confirm its claims
  against the actual code before acting on them.
- Weigh what it reports against the decisions and reasoning already established
  in the current discussion — flag and re-check anything that contradicts that
  context rather than silently accepting it.
