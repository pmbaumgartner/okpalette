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
