# Changelog

All notable changes to this project are documented here.

## Unreleased

### Added

- A typed Rust API for palette generation, extension, label-aware assignment,
  color constraints, colorblind simulation, parsing, and rendering.
- crates.io package metadata and Rust 1.90 minimum-version verification.

### Changed

- The PyO3 bridge is optional behind the `python` Cargo feature.
- Python now configures a typed native generator instead of duplicating a long
  positional extension-function contract.
- Rust distance weights and candidate ranges are validated when constructed.
- `extend_palette()` accepts extra seed anchors without returning them.
- `target_size` now consistently means final palette size when Python or the CLI
  returns generated colors only.
- Python accepts all integer RGB triples as 8-bit RGB colors and accepts zero as
  an empty palette size, matching Rust.
- Rust exposes `OkPaletteError` as the preferred error name, validates custom
  grid steps at construction, and rejects empty background lists immediately.
