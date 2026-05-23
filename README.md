# okpalette

Categorical color palettes for Python.

Use `okpalette` when you need distinct, stable colors for labels, plots, dashboards, or reports.

```bash
pip install okpalette
```

With uv:

```bash
uv add okpalette
```

## Python And Wheels

`okpalette` supports Python 3.12 and newer. The 1.0 wheels use the Python
`cp312-abi3` stable ABI and are built for Linux `x86_64` and `aarch64`
manylinux2014, macOS `aarch64`, and Windows `x64`.

Other platforms install from the source distribution when a wheel is not
available, which requires Rust and the normal Python build toolchain. Intel
macOS, musllinux, Windows ARM64, and other secondary targets are not 1.0 wheel
targets.

```python
from okpalette import create_palette

colors = create_palette(8)
```

From the command line:

```bash
okpalette create 8
```

CLI success output is JSON only:

```json
{"colors":["#080050","#e00800","#1078ff"],"format":"hex"}
```

## Create A Palette

`create_palette()` returns lowercase hex colors by default.

```python
from okpalette import create_palette

colors = create_palette(10)
# ["#080050", "#e00800", "#1078ff", ...]
```

The same inputs produce the same colors, so category mappings stay stable across runs.

Use RGB tuples when that fits your plotting library better:

```python
rgb = create_palette(5, format="rgb")
# [(8, 0, 80), (224, 8, 0), ...]

rgb01 = create_palette(5, format="rgb01")
# [(0.03137254901960784, 0.0, 0.3137254901960784), ...]
```

## Extend Colors

Use `extend_palette()` when you already have brand colors or a small palette.

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

The same basic workflows are available through the CLI:

```bash
okpalette create 10
okpalette create 5 --format rgb
okpalette extend 12 --color "#0057b8" --color "#ffd700"
okpalette extend 10 --color "#0057b8" --generated-only
```

`okpalette create` and `okpalette extend` always write JSON on success, with
the stable shape `{"colors":[...],"format":"hex"}`. For `--format rgb` and
`--format rgb01`, tuple colors are serialized as JSON arrays. Validation and
generation errors write a short message to stderr and leave stdout empty.

## Map Labels To Colors

Use `create_label_palette()` when positions should influence which label gets
which color. Nearby or overlapping labels are assigned more distinct colors.

```python
from okpalette import create_label_palette

positions = [(0.0, 0.0), (0.2, 0.0), (5.0, 0.0), (5.2, 0.0)]
labels = ["control", "treated", "control", "outlier"]

colors = create_label_palette(positions, labels)
# {"control": "#080050", "treated": "#e00800", "outlier": ...}
```

Labels may be strings, integers, tuples, or other hashable Python objects. The
returned dict preserves first-seen label order.

Keep specific label colors fixed:

```python
colors = create_label_palette(
    positions,
    labels,
    fixed_colors={"control": "#0057b8"},
)
```

For dataframe-like objects, read position and label columns by duck typing:

```python
colors = create_label_palette_from_columns(
    data,
    positions=["x", "y"],
    label="cluster",
)
```

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
    color_discrete_sequence=colors,
)

fig = px.scatter(
    data,
    x="x",
    y="y",
    color="group",
    color_discrete_map=color_map,
)
```

## Tune Appearance

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
    lightness=(0.20, 0.75),
    chroma=(0.05, None),
)
```

Leave background filtering off:

```python
colors = create_palette(8, lightness=None, chroma=None)
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

Opt into colorblind-aware generation when pairwise palette separability should
be tested under selected color vision deficiency simulations:

```python
colors = create_palette(12, colorblind_mode="all")
```

CLI:

```bash
okpalette create 12 --colorblind-mode red-green
okpalette create 12 --colorblind-mode all
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

Limit hue ranges:

```python
warm = create_palette(10, hue=(330, 100))
cool = create_palette(10, hue=(150, 280))
```

Common constraints:

```python
muted = create_palette(12, chroma=(0.02, 0.12))
bright = create_palette(12, chroma=(0.10, None))
mid_lightness = create_palette(12, lightness=(0.30, 0.80))
```

`lightness` is OKLab `L` in `0..1`. `hue` is OKLCH degrees in `0..360`; ranges can wrap around zero.

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

## Color Inputs

Accepted color inputs:

```python
"#0fA"
"00ffaa"
(255, 128, 0)
(1.0, 0.5, 0.0)
```

Integer RGB tuples use `0..255`. Normalized RGB tuples use floats in `0.0..1.0`.
Ambiguous integer tuples such as `(1, 0, 0)` are rejected; write `(1.0, 0.0, 0.0)`
for normalized RGB.

## Grid Size

`grid_size` controls how many candidate colors are searched.

```python
quick = create_palette(24, grid_size="coarse")  # step 16
default = create_palette(24, grid_size="medium")  # step 8
fine = create_palette(24, grid_size="fine")  # step 4
custom = create_palette(24, grid_size=12)
```

If constraints leave too few candidates, `okpalette` raises `ValueError` with a hint
to relax `lightness`, `chroma`, `hue`, or `grid_size`.

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

## API

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
    **create_palette_options,
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
