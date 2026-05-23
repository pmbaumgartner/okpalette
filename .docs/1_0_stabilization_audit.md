---
kata: n6ke
created: 2026-05-23
---

# 1.0 Stabilization Audit

## Verdict

`okpalette` is close to 1.0 stabilization, but is not ready to declare the 1.0
bar complete yet.

Exact blockers found by this audit:

- `1p77` - add release-facing citations for Glasbey-style generation,
  OKLab/OKLCH, WCAG contrast, and Machado 2009 CVD simulation. The ignored
  `.docs/*_decision.md` files contain source links, but the shipped README and
  wheel metadata do not expose them to users.
- `eexv` - decide and document the 1.0 Python / ABI / wheel platform support
  policy. The current build uses `abi3-py312` and has a release workflow, but
  the intended supported wheel matrix is not stated, and gaps such as macOS
  x86_64 are not explicitly accepted or rejected.

No additional API, deterministic-output, WCAG, colorblind-aware, plotting, or
palette-repair blocker was found.

## Audit Inputs

- Current branch: `main`.
- Audited commit: `4264e8b`.
- Kata context: `a740`, `5d0a`, `p9rd`, `zg3m`, `wwhv`, `3r3z`, `gs3b`,
  `t0m1`, `rq0r`, and `rjz8`.
- Code inventory: codebase-memory graph plus direct source review for public
  package files, Rust bridge files, tests, workflows, and package metadata.
- Local wheel check:
  `uv run maturin build --skip-auditwheel` produced
  `target/wheels/okpalette-0.2.0-cp312-abi3-macosx_11_0_arm64.whl`.

## Public API Freeze Recommendation

Freeze these Python exports for 1.0:

- Functions: `create_palette`, `extend_palette`, `create_label_palette`,
  `create_label_palette_from_columns`, `view_palette`, `palette_svg`,
  `palette_png`, and `save_palette`.
- Class: `PaletteView`.
- Type aliases: `ColorLike`, `Rgb8`, `Rgb01`, `BackgroundLike`,
  `BackgroundContrast`, `ColorblindMode`, `ColorFormat`, and `GridSize`.
- Version export: `__version__`.

Do not add palette repair to the 1.0 public API. The current tracked package
surface has no `repair_palette`, `PaletteRepairResult`, `analyze_palette`, Rust
repair bridge, repair test, or repair benchmark.

Freeze these public option defaults:

| Option | 1.0 default |
| --- | --- |
| `seed_colors` | `()` |
| `avoid_colors` | `None` |
| `background` | `None` |
| `background_contrast` | `None` |
| `lightness` | `(0.20, 0.90)` |
| `chroma` | `(0.04, None)` |
| `hue` | `None` |
| `grid_size` | `"medium"` |
| `lightness_weight` | `1.0` |
| `chroma_weight` | `1.0` |
| `colorblind_mode` | `None` |
| `format` | `"hex"` |
| `include_existing` | `True` for `extend_palette` |
| `neighbors` | `8` for label palettes |
| `max_points` | `50_000` for label palettes |
| preview `width` / `height` | `1246` / `154` |

Freeze these value sets:

- Output formats: `"hex"`, `"rgb"`, and `"rgb01"`.
- Background contrast modes: `"normal"`, `"high"`, and `"wcag"`.
- Colorblind modes: `None`, `"protan"`, `"deutan"`, `"tritan"`,
  `"red-green"`, `"daltonism"` as an alias for `"red-green"`, and `"all"`.
- Grid names: `"coarse"` = step 16, `"medium"` = step 8, `"fine"` = step 4,
  plus integer steps in `1..255`.

The CLI surface is narrow enough to freeze for 1.0:

- `okpalette create SIZE [OPTIONS]`
- `okpalette extend TARGET_SIZE --color COLOR [--color COLOR ...] [OPTIONS]`
- `okpalette install-skill --agent codex|claude [--overwrite] [--dry-run]`
- Success stdout is JSON with `{"colors":[...],"format":"..."}`.
- Validation/generation errors go to stderr without tracebacks and leave stdout
  empty.

## Determinism Freeze Recommendation

Freeze deterministic outputs for the same `okpalette` version, inputs, option
values, and supported platform class.

Patch-release compatibility fixtures should include:

- Exact snapshot tests in `tests/test_regressions.py`: default 10-color
  palette, seeded white/black palette, warm hue palette, and cool hue palette.
- Exact colorblind snapshots in `tests/test_colorblind.py` for `protan`,
  `deutan`, `tritan`, `red-green`, and `all`.
- The `daltonism` alias matching `red-green`.
- Exact small-grid behavior covered by Rust candidate-order and grid-count
  tests.
- CLI JSON shape and RGB/RGB01 JSON-array serialization.

Label-palette exact color assignments are deterministic and tested, but should
not be treated as broadly frozen across every dataset shape before 1.0. Freeze
the documented contract instead: first-seen label order, fixed colors unchanged,
position-aware assignment, deterministic repeated output, bounded
`max_points`, and the same palette option semantics as `create_palette`.

Do not change snapshot fixtures casually after 1.0. If a deterministic output
must change for a documented bug fix, the release notes should name the affected
option set and tests.

## WCAG Contrast Check

Status: pass.

The implementation matches `5d0a`:

- `background_contrast="normal"` remains an OKLab background-separation
  heuristic.
- `background_contrast="high"` and `"wcag"` use the WCAG non-text threshold of
  `3.0:1` against every configured background.
- WCAG contrast is calculated in ordinary sRGB from relative luminance, not
  from CVD-simulated colors.
- `background` and `background_contrast` must be provided together.
- Returned seed and fixed colors are validated when WCAG contrast is active.

Coverage exists in Rust and Python tests for contrast ratio snapshots,
high/wcag aliasing, multiple backgrounds, invalid coupling, and seed/fixed color
errors.

## Colorblind-Aware Generation Check

Status: pass.

The implementation matches `p9rd` and `t0m1`:

- Colorblind-aware generation is opt-in.
- Machado, Oliveira, and Fernandes 2009 severity-`1.0` matrices are used for
  protan, deutan, and tritan simulation.
- `"red-green"` scores protan and deutan only; `"all"` scores protan, deutan,
  and tritan.
- Pair scoring uses the worst case across ordinary OKLab and selected simulated
  OKLab spaces.
- The docs avoid "colorblind-safe" claims.
- WCAG background contrast remains an ordinary-sRGB hard predicate.

Coverage includes Rust parsing/profile tests, Python exact snapshots, extension
behavior, label-palette behavior, WCAG composition, and invalid mode errors.

## Plotting Compatibility Check

Status: pass.

The README documents copyable snippets for:

- Matplotlib color cycles and `ListedColormap`.
- Altair categorical `scale(domain=..., range=...)`.
- Altair raw color columns with `scale=None`.
- Plotly Express `color_discrete_sequence`.
- Plotly Express `color_discrete_map`.

`tests/test_plotting_compat.py` verifies the core package has no required
Python runtime dependencies and that the documented shape is plain hex
sequences/maps usable by the plotting libraries. Matplotlib is optional and
dev-only.

## Errors Check

Status: pass.

Freeze these user-facing categories:

- `ValueError` for invalid public values, impossible generation requests,
  malformed colors, invalid constraints, invalid output formats, invalid
  colorblind modes, and Rust engine errors mapped through PyO3.
- `TypeError` for unexpected `extend_palette` keyword arguments.
- `ImportError` when the native extension is unavailable, with the actionable
  install / `maturin develop` message.
- CLI exit code `2` for argument parsing errors and `1` for API validation,
  generation, installation, or filesystem errors.

Messages are clear enough for 1.0. They name the invalid option or failed
constraint and, for insufficient candidates, suggest relaxing `lightness`,
`chroma`, `hue`, `background_contrast`, or `grid_size`.

## Wheel, Python, ABI, And Dependency Check

Status: blocked by `eexv`.

Observed facts:

- `pyproject.toml` declares `requires-python = ">=3.12"` and classifiers for
  Python 3.12, 3.13, and 3.14.
- CI tests Python 3.12, 3.13, and 3.14.
- PyO3 is configured with `abi3-py312`.
- The local wheel tag is `cp312-abi3-macosx_11_0_arm64`.
- The built wheel includes the native extension, package stubs, `py.typed`,
  CLI, skill installer, packaged skill, metadata, and console entry point.
- Core Python runtime dependencies are empty.
- Rust runtime dependencies are `kiddo`, `png`, `pyo3`, `rayon`, and
  `thiserror`; Criterion is dev-only.
- The release workflow builds Linux x86_64/aarch64 manylinux2014 wheels, macOS
  aarch64, Windows x64, and an sdist, then publishes on tag push.

The blocker is policy clarity rather than a known build failure: the README and
release docs do not state the supported platform set, and the current workflow
does not obviously cover every common platform a user may expect for a 1.0
Python package.

## Docs And Citations Check

Status: blocked by `1p77`.

The README covers:

- Deterministic palette creation and extension.
- Label-aware palettes.
- Matplotlib, Altair, and Plotly compatibility.
- WCAG-backed background contrast and the `"high"` / `"wcag"` behavior.
- Colorblind-aware generation, Machado 2009 severity-`1.0`, and caveats.
- Preview helpers.
- Color inputs, grid size, CLI, packaged agent skill, and API summary.
- OKLab/OKLCH and greedy Glasbey-style behavior.
- Caveats: not colorblind-safe, not an accessibility certification, not a
  global optimizer.

The missing piece is a release-facing references section. Current source links
live in ignored decision artifacts, not in shipped docs or wheel metadata.

## Palette Repair Absence Check

Status: pass for tracked `main` package surface.

Tracked package files contain no:

- `repair_palette`
- `PaletteRepairResult`
- `analyze_palette`
- Rust `repair.rs`
- repair bridge in `src/lib.rs`
- repair README section
- repair tests under `tests/`
- repair benchmark under `benches/`

Ignored `.docs` files still contain historical palette-repair decision and
prototype material, and kata issues record that the repair implementation is
preserved on branch `defer/palette-repair`. Those artifacts are useful context
but are not tracked package docs or shipped public surface on `main`.

## Residual Risks

- Exact deterministic output across CPU architectures has not been verified in
  this local audit. CI and release workflows should be used to confirm
  supported platform behavior before tagging 1.0.
- The release workflow was reviewed locally but not executed from this audit.
- The local wheel build verified macOS arm64 only.
- Label-palette outputs are deterministic but less appropriate as broad
  patch-release color snapshots than the simple palette fixtures.

## Close Recommendation

Do not close `n6ke` as ready for 1.0 while `1p77` and `eexv` are open. After
those blockers are resolved, rerun the development loop and update this artifact
with the final ready/not-ready verdict.
