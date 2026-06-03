# typst-template

Generate PDFs with [Typst](https://typst.app) straight from your Rust types.

Derive `ToDict` on a struct and it becomes a Typst input dictionary — feed it to
a `.typ` template as `sys.inputs` and render to PDF, with no hand-written dict
building. The `ToValue` / `ToDict` traits handle the Rust-to-Typst value
conversion, with optional `chrono`, `time`, `rust_decimal`, `serde_json`, and
`uuid` support.

Rendering is split into two pieces so repeated generation stays cheap:

- **`WorldBase`** — built once; loads fonts (the expensive step) and roots file
  access at a directory.
- **`ConcreteWorld`** — one per document; attaches this generation's `sys.inputs`
  and any virtual in-memory files (images, generated assets, …) without touching
  disk.

## Example

```rust
use typst_template::{ToDict, WorldBaseConfig};

#[derive(ToDict)]
struct Invoice {
    client: String,
    total: i64,
}

// Build the base once (here using only the embedded fonts).
let base = WorldBaseConfig::new("path/to/template").system_fonts(false).build();

let world = base
    .concrete("main.typ")
    .file("main.typ", "Invoice for #sys.inputs.client: #sys.inputs.total EUR")
    .inputs(Invoice { client: "ACME".into(), total: 42 })
    .build();

let pdf = world.render_pdf_default().output.expect("render succeeds");
assert_eq!(&pdf[..4], b"%PDF");
```

`compile()` and `render_pdf()` return Typst's native `Warned<SourceResult<…>>`,
so you get warnings and full diagnostics (with spans) without any lossy
wrapping.

## Derive attributes

On the struct (`#[typst(...)]`):

- `rename_all = "..."` — `lowercase`, `UPPERCASE`, `PascalCase`, `camelCase`,
  `snake_case`, `SCREAMING_SNAKE_CASE`, `kebab-case`, `SCREAMING-KEBAB-CASE`.

On a field (`#[typst(...)]`):

- `rename = "name"` — fixed key, overriding `rename_all`.
- `skip` — leave the field out.
- `with = path` — call `path(field) -> Value` instead of the field's `ToValue`.
- `flatten` — merge the field's own dict in place of nesting it (field must be
  `ToDict`).

## `datetime.today()`

Without a time feature, `datetime.today()` is disabled by default (returns
nothing); pin a value with `.today(dt)`. With `chrono` or `time` enabled it
defaults to the system clock, and `.today_system_in(tz)` (with `chrono-tz` or
`time-tz`) sets "local" to a DST-aware named zone — handy when the document's
time zone differs from the server's. A `Fixed` value answers only offset-less
requests; an explicit `datetime.today(offset: N)` returns nothing.

## Features

- `derive` *(default)* — the `ToDict` derive macro.
- `chrono` — `ToValue` for `chrono` date/time types, plus a system-clock
  `datetime.today()`.
- `time` — `ToValue` for `time` date/time types, plus a system-clock
  `datetime.today()`. `chrono` takes precedence if both are enabled.
- `chrono-tz` / `time-tz` — named-time-zone support for `.today_system_in`.
- `rust_decimal` — `ToValue` for `rust_decimal::Decimal`.
- `serde_json` — `ToValue`/`ToDict` for `serde_json` values.
- `uuid` — `ToValue` for `uuid::Uuid`.

## License

MIT
