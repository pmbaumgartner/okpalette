# okpalette

[![PyPI](https://img.shields.io/pypi/v/okpalette)](https://pypi.org/project/okpalette/)
[![Python](https://img.shields.io/pypi/pyversions/okpalette)](https://pypi.org/project/okpalette/)
[![crates.io](https://img.shields.io/crates/v/okpalette)](https://crates.io/crates/okpalette)
[![docs.rs](https://img.shields.io/docsrs/okpalette)](https://docs.rs/okpalette)
[![CI](https://github.com/pmbaumgartner/okpalette/actions/workflows/ci.yml/badge.svg)](https://github.com/pmbaumgartner/okpalette/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](https://github.com/pmbaumgartner/okpalette/blob/main/LICENSE)

Fast, deterministic categorical color palettes for Python, Rust, and the command line.

![Twelve default okpalette colors](https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-default.png)

Use `okpalette` when you need distinct, stable colors for labels, plots, dashboards, or reports.

## Why okpalette?

- **Any number of colors.** Fixed palettes like `tab10` or `tab20` run out; `okpalette` generates as many maximally distinct colors as you ask for.
- **Deterministic.** The same inputs always produce the same colors, so charts stay consistent across runs and machines.
- **Stable extension.** Grow a palette or extend your brand colors without changing the colors you already use.
- **Position-aware label colors.** Labels that sit close together in a plot get the most distinct colors.
- **Constraints.** Control lightness, chroma, and hue; keep colors separated from a background; optimize under colorblind simulations.
- **Lightweight.** No required Python dependencies. The core is written in Rust and is also available as a Rust crate.

See the [gallery](#gallery) for what each option produces.

**When not to use it:** `okpalette` is for categorical data. For ordered or
continuous values, use a sequential or diverging colormap instead. Beyond
roughly 20–30 categories, no palette stays easy to tell apart; consider grouping
categories or labeling them directly.

## Install

```bash
pip install okpalette
```

With uv:

```bash
uv add okpalette
```

Run the CLI without installing, as a [uv tool](https://docs.astral.sh/uv/concepts/tools/):

```bash
uvx okpalette create 8
```

Rust:

```bash
cargo add okpalette
```

## Quickstart

Create colors for categories:

```python
from okpalette import create_palette

colors = create_palette(8)
# ["#000058", "#90ff00", "#ff38ff", ...]
```

Extend colors you already have:

```python
from okpalette import extend_palette

colors = extend_palette(["#0057b8", "#ffd700"], 8)
# ["#0057b8", "#ffd700", "#ff0070", ...]
```

Give nearby labels more distinct colors:

```python
from okpalette import create_label_palette

positions = [(0.0, 0.0), (0.2, 0.0), (5.0, 0.0), (5.2, 0.0)]
labels = ["control", "treated", "control", "outlier"]

colors = create_label_palette(positions, labels)
# {"control": "#90ff00", "treated": "#000058", "outlier": "#ff38ff"}
```

From a shell:

```bash
okpalette create 8
# {"colors":["#000058","#90ff00","#ff38ff","#886800","#1058ff","#88c8ff","#800078","#ffa038"],"format":"hex"}
```

## Gallery

Each palette below has 12 colors. The options are explained in
[Extend Colors](#extend-colors) and [Tune Appearance](#tune-appearance).

**Default:** `create_palette(12)`

<img src="https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-default.png" alt="default palette swatch" width="600">

**Extend brand colors (the first two are kept):** `extend_palette(["#0057b8", "#ffd700"], 12)`

<img src="https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-extend.png" alt="extend palette swatch" width="600">

**High contrast against a dark background:** `create_palette(12, background="#1e1e1e", background_contrast="high")`

<img src="https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-background.png" alt="background palette swatch" width="600">

**Red-green colorblind-aware:** `create_palette(12, colorblind_mode="red-green")`

<img src="https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-red-green.png" alt="red green palette swatch" width="600">

**Muted:** `create_palette(12, chroma=(0.02, 0.12))`

<img src="https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-muted.png" alt="muted palette swatch" width="600">

**Bright:** `create_palette(12, chroma=(0.10, None))`

<img src="https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-bright.png" alt="bright palette swatch" width="600">

**Mid lightness:** `create_palette(12, lightness=(0.30, 0.80))`

<img src="https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-mid-lightness.png" alt="mid lightness palette swatch" width="600">

**Warm hues:** `create_palette(12, hue=(330, 100))`

<img src="https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-warm.png" alt="warm palette swatch" width="600">

**Cool hues:** `create_palette(12, hue=(150, 280))`

<img src="https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-cool.png" alt="cool palette swatch" width="600">

**Coarse grid (faster search):** `create_palette(12, grid_size="coarse")`

<img src="https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-coarse.png" alt="coarse palette swatch" width="600">

**Fine grid (wider search):** `create_palette(12, grid_size="fine")`

<img src="https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/palette-fine.png" alt="fine palette swatch" width="600">

## Use With Plotting Libraries

`okpalette` has no required Matplotlib, Altair, or Plotly dependency. The
default output is a list of lowercase hex strings, which all three libraries
accept directly. Use `format="rgb01"` for Matplotlib APIs that specifically
expect normalized RGB tuples.

Matplotlib color cycles and colormaps:

```python
import matplotlib.pyplot as plt
from matplotlib.colors import ListedColormap
from okpalette import create_palette

colors = create_palette(6)

fig, ax = plt.subplots()
ax.set_prop_cycle(color=colors)
ax.plot(x, y1)
ax.plot(x, y2)

cmap = ListedColormap(create_palette(12), name="okpalette")
ax.scatter(x, y, c=values, cmap=cmap)
```

Altair categorical scales and raw color columns:

```python
import altair as alt
from okpalette import create_palette

categories = ["control", "treated", "outlier"]
colors = create_palette(len(categories))

chart = alt.Chart(data).mark_point().encode(
    x="x:Q",
    y="y:Q",
    color=alt.Color(
        "group:N",
        scale=alt.Scale(domain=categories, range=colors),
    ),
)

# When a dataframe column already contains okpalette hex strings:
chart = alt.Chart(data).mark_point().encode(
    x="x:Q",
    y="y:Q",
    color=alt.Color("color:N", scale=None),
)
```

Plotly Express discrete sequences and maps:

```python
import plotly.express as px
from okpalette import create_palette

categories = ["control", "treated", "outlier"]
colors = create_palette(len(categories))
color_map = dict(zip(categories, colors))

fig = px.scatter(
    data,
    x="x",
    y="y",
    color="group",
    color_discrete_map=color_map,
)
```

## Extend Colors

Use `extend_palette()` when you already have brand colors or a small palette.
Existing colors are kept as-is, and new colors are chosen to be distinct from
them.

```python
from okpalette import extend_palette

brand = ["#0057b8", "#ffd700"]
colors = extend_palette(brand, 12)

assert colors[:2] == ["#0057b8", "#ffd700"]
assert len(colors) == 12
```

Use existing colors as anchors without returning them:

```python
new_colors = extend_palette(brand, 10, include_existing=False)
```

Here `target_size=10` still describes the final palette size, so `new_colors`
contains eight generated colors when `brand` contains two colors.

## Map Labels To Colors

Use `create_label_palette()` when positions should influence which label gets
which color. Nearby or overlapping labels are assigned more distinct colors.

![Sixteen clusters colored in label order and with create_label_palette](https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/label-palette-comparison.png)

Each cluster above is a separate label. Dashed lines join touching clusters
whose colors are easy to confuse. Assigning `create_palette()` colors in label
order (left) puts similar colors side by side; `create_label_palette()` (right)
chooses and assigns colors so neighbors stay distinct. For a larger example,
see the [word scatterplot](https://raw.githubusercontent.com/pmbaumgartner/okpalette/main/examples/output/okpalette-word-scatter.png).

```python
from okpalette import create_label_palette

positions = [(0.0, 0.0), (0.2, 0.0), (5.0, 0.0), (5.2, 0.0)]
labels = ["control", "treated", "control", "outlier"]

colors = create_label_palette(positions, labels)
# {"control": "#90ff00", "treated": "#000058", "outlier": "#ff38ff"}
```

`positions` may be 1D scalars or 1D, 2D, or 3D coordinate rows, as lists,
tuples, NumPy arrays, or similar array-likes. Coordinates must be finite, and
`positions` and `labels` must have the same length. Labels may be strings,
integers, tuples, or other hashable Python objects. The returned dict preserves
first-seen label order.

Keep specific label colors fixed:

```python
colors = create_label_palette(
    positions,
    labels,
    fixed_colors={"control": "#0057b8"},
)
```

For pandas, polars, and other dataframe-like objects, read position and label
columns by duck typing (no dataframe dependency is required):

```python
from okpalette import create_label_palette_from_columns

colors = create_label_palette_from_columns(
    data,
    positions=["x", "y"],
    label="cluster",
)
```

## Tune Appearance

### Background Contrast

By default, palettes are generated without a background constraint. Pass both
`background` and `background_contrast` when you want colors separated from a
known plotting background. Use `"normal"` for the OKLab background-separation
heuristic, or `"high"` / `"wcag"` for WCAG 2.2 non-text contrast of at least
`3.0:1` against every configured background color.

```python
colors = create_palette(
    32,
    background="#ffffff",
    background_contrast="normal",
)
```

Avoid other colors:

```python
colors = create_palette(
    16,
    avoid_colors=["#000000"],
    background=["#ffffff", "#f2f2f2"],
    background_contrast="high",
)
```

`background` accepts one color or a sequence of colors and filters candidates
against those backgrounds. `avoid_colors` keeps exact colors out of the palette
and uses them as distance anchors.

### Colorblind-Aware Generation

Opt into colorblind-aware generation when pairwise palette separability should
be tested under selected color vision deficiency simulations:

```python
colors = create_palette(12, colorblind_mode="all")
```

`colorblind_mode` may be `None`, `"protan"`, `"deutan"`, `"tritan"`,
`"red-green"`, or `"all"`. The `"red-green"` mode optimizes against protan and
deutan simulations without including tritan; `"daltonism"` is accepted as a
compatibility alias, but `"red-green"` is the preferred spelling. The feature
uses Machado, Oliveira, and Fernandes 2009 severity-`1.0` matrices on linear
sRGB and optimizes the worst-case OKLab distance across ordinary vision and the
selected simulations. It does not make a palette universally accessible or
colorblind-safe. If you also set
`background_contrast="high"` or `"wcag"`, WCAG contrast is still checked against
the ordinary sRGB background, not against simulated colors.

### Lightness, Chroma, And Hue

```python
muted = create_palette(12, chroma=(0.02, 0.12))
bright = create_palette(12, chroma=(0.10, None))
mid_lightness = create_palette(12, lightness=(0.30, 0.80))

warm = create_palette(10, hue=(330, 100))
cool = create_palette(10, hue=(150, 280))
```

`lightness` is OKLab `L` in `0..1`. `chroma` is OKLCH chroma; `None` leaves a
bound open. `hue` is OKLCH degrees in `0..360`; ranges can wrap around zero.

### Grid Size

`grid_size` controls how many candidate colors are searched.

```python
quick = create_palette(24, grid_size="coarse")  # step 16
default = create_palette(24, grid_size="medium")  # step 8
fine = create_palette(24, grid_size="fine")  # step 4
custom = create_palette(24, grid_size=12)
```

If constraints leave too few candidates, `okpalette` raises `ValueError` with a hint
to relax `lightness`, `chroma`, `hue`, or `grid_size`.

## Output Formats

Use RGB tuples when that fits your plotting library better:

```python
rgb = create_palette(5, format="rgb")
# [(0, 0, 88), (144, 255, 0), ...]

rgb01 = create_palette(5, format="rgb01")
# [(0.0, 0.0, 0.34509803921568627), ...]
```

## Preview And Save

```python
from okpalette import create_palette, save_palette, view_palette

colors = create_palette(12)

view_palette(colors)
save_palette(colors, "palette.svg")
save_palette(colors, "palette.png")
```

`view_palette()` works in notebooks through `_repr_svg_()` and `_repr_png_()`.

For raw preview bytes:

```python
from okpalette import palette_png, palette_svg

svg = palette_svg(colors)
png = palette_png(colors)
```

## CLI

The CLI covers palette creation and extension:

```bash
okpalette create 10
okpalette create 5 --format rgb
okpalette create 12 --colorblind-mode red-green
okpalette extend 12 --color "#0057b8" --color "#ffd700"
okpalette extend 10 --color "#0057b8" --generated-only
```

`okpalette create` and `okpalette extend` always write JSON on success, with
the stable shape:

```json
{"colors":["#000058","#90ff00","#ff38ff"],"format":"hex"}
```

For `--format rgb` and `--format rgb01`, tuple colors are serialized as JSON
arrays. Validation and generation errors write a short message to stderr and
leave stdout empty.

## Rust API

Rust crates can use the generator directly without depending on Python:

```toml
[dependencies]
okpalette = "1.1"
```

The one-shot functions use the same defaults as the Python API and CLI:

```rust
use okpalette::{extend_palette, generate_palette, Rgb8};

let colors = generate_palette(8).unwrap();
let brand = [
    "#0057b8".parse::<Rgb8>().unwrap(),
    "#ffd700".parse().unwrap(),
];
let extended = extend_palette(&brand, 8).unwrap();
```

Use `PaletteGenerator` for reusable, typed configuration:

```rust
use okpalette::{
    BackgroundContrast, CandidateConstraints, ChromaRange, ColorblindMode,
    DistanceWeights, GridSize, LightnessRange, PaletteGenerator, Rgb8,
};

let generator = PaletteGenerator::new()
    .grid_size(GridSize::Fine)
    .constraints(
        CandidateConstraints::new()
            .with_lightness(LightnessRange::new(0.2, 0.9).unwrap())
            .with_chroma(ChromaRange::new(Some(0.04), None).unwrap()),
    )
    .distance_weights(DistanceWeights::new(0.8, 1.2).unwrap())
    .backgrounds(
        [Rgb8::new(255, 255, 255)],
        BackgroundContrast::Normal,
    )
    .unwrap()
    .colorblind_mode(ColorblindMode::All);

let colors = generator.generate(12).unwrap();
let new_colors = generator.generate_extension(&colors[..2], 10).unwrap();
```

`LabelPaletteRequest` exposes the position-aware label generator. Typed colors,
constraints, parsing, and SVG/PNG rendering are also available from the crate
root; candidate search and label-assignment internals remain private. See the
full API on [docs.rs](https://docs.rs/okpalette).

## Agent Skill

`okpalette` includes an optional packaged agent skill for simple JSON CLI usage.
Install it into a personal Codex or Claude skill directory:

```bash
okpalette install-skill --agent codex
okpalette install-skill --agent claude
```

Use `--dry-run` to print the target path without writing, and `--overwrite` to
replace an existing installed skill. The Codex installer respects `$CODEX_HOME`
and otherwise writes under `~/.codex`; Claude skills are installed under
`~/.claude`.

## Python API Reference

```python
create_palette(
    palette_size,
    *,
    seed_colors=(),
    avoid_colors=None,
    background=None,
    background_contrast=None,
    lightness=(0.20, 0.90),
    chroma=(0.04, None),
    hue=None,
    grid_size="medium",
    lightness_weight=1.0,
    chroma_weight=1.0,
    colorblind_mode=None,
    format="hex",
)
```

```python
extend_palette(
    colors,
    target_size,
    *,
    include_existing=True,
    seed_colors=(),
    avoid_colors=None,
    background=None,
    background_contrast=None,
    lightness=(0.20, 0.90),
    chroma=(0.04, None),
    hue=None,
    grid_size="medium",
    lightness_weight=1.0,
    chroma_weight=1.0,
    colorblind_mode=None,
    format="hex",
)
```

```python
create_label_palette(
    positions,
    labels,
    *,
    fixed_colors=None,
    seed_colors=(),
    avoid_colors=None,
    background=None,
    background_contrast=None,
    lightness=(0.20, 0.90),
    chroma=(0.04, None),
    hue=None,
    grid_size="medium",
    lightness_weight=1.0,
    chroma_weight=1.0,
    colorblind_mode=None,
    neighbors=8,
    max_points=50_000,
    format="hex",
)
```

```python
create_label_palette_from_columns(data, *, positions, label, **create_label_palette_options)
```

```python
view_palette(palette, *, width=1246, height=154)
palette_svg(palette, *, width=1246, height=154)
palette_png(palette, *, width=1246, height=154)
save_palette(palette, path, *, width=1246, height=154)
```

## How It Works

`okpalette` uses a greedy Glasbey-style algorithm. It starts with anchor colors
such as seeds, avoid colors, and the background, then repeatedly chooses the
candidate color that is farthest from the nearest anchor or selected color.

Distances are measured in OKLab. Lightness, chroma, and hue constraints are
applied through OKLab and OKLCH before colors are selected.

When `colorblind_mode` is enabled, candidate distances are scored as the minimum
of ordinary OKLab distance and the OKLab distance after each selected Machado
2009 severity-`1.0` simulation. This is a generation objective for palettes
tested under selected CVD simulations, not an accessibility certification.

For label palettes, `okpalette` builds a weighted label-neighborhood graph from
the input positions, then uses that graph while choosing and assigning colors.
The default `max_points=50_000` bounds graph construction for large datasets;
use `max_points=None` to opt into all-points preprocessing.

The result is deterministic, fast, and stable when extending a palette. It is
not a global optimizer.

## Python And Wheels

`okpalette` supports Python 3.12 and newer. Prebuilt wheels use the Python
`cp312-abi3` stable ABI and are built for Linux `x86_64` and `aarch64`
manylinux2014, macOS `aarch64`, and Windows `x64`.

Other platforms install from the source distribution when a wheel is not
available, which requires Rust and the normal Python build toolchain. Intel
macOS, musllinux, Windows ARM64, and other secondary targets are not prebuilt
wheel targets.

## References

These references describe the methods and standards that inform `okpalette`.
They do not make generated palettes WCAG-compliant, colorblind-safe, or globally
optimal.

- Glasbey-style categorical palette generation:
  [lmcinnes/glasbey](https://github.com/lmcinnes/glasbey) and the
  [Colorcet categorical guide](https://colorcet.holoviz.org/user_guide/Categorical.html).
- OKLab and OKLCH:
  [A perceptual color space for image processing](https://bottosson.github.io/posts/oklab/)
  and
  [MDN OKLCH documentation](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Values/color_value/oklch).
- WCAG 2.2 contrast:
  [WCAG 2.2](https://www.w3.org/TR/WCAG22/),
  [Non-text Contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html),
  and
  [relative luminance](https://www.w3.org/TR/WCAG22/#dfn-relative-luminance).
- Color vision deficiency simulation:
  [Machado, Oliveira, and Fernandes 2009](https://www.inf.ufrgs.br/~oliveira/pubs_files/CVD_Simulation/Machado_Oliveira_Fernandes_CVD_Vis2009_final.pdf)
  and the
  [CVD simulation project page](https://www.inf.ufrgs.br/~oliveira/pubs_files/CVD_Simulation/CVD_Simulation.html).

## Development

The extension module is built with [maturin](https://www.maturin.rs/). Before
sending changes, run:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
uv run --extra dev ruff check .
uv run --extra dev ty check
```

README images are regenerated with:

```bash
uv run --extra dev python examples/readme_images.py
uv run --extra dev python examples/okpalette_word_scatter.py
```

See the [changelog](https://github.com/pmbaumgartner/okpalette/blob/main/CHANGELOG.md)
for release history. `okpalette` is released under the
[MIT License](https://github.com/pmbaumgartner/okpalette/blob/main/LICENSE).
