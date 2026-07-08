# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1]

### Security

- Update `crossbeam-epoch` to 0.9.20, clearing RUSTSEC-2026-0204.
- Ignore two transitive `quick-xml` DoS advisories (RUSTSEC-2026-0194,
  RUSTSEC-2026-0195) in `cargo audit` config: they are only reachable through
  `typst`, are unfixable without a breaking `typst` 0.15 bump, and require
  parsing untrusted XML — which this crate never does.

## [0.1.0]

Initial release: core PDF generation (reusable base world + per-document
concrete world), the `ToValue`/`ToDict` traits with `#[derive(ToDict)]`, and
optional ecosystem integrations behind cargo features.
