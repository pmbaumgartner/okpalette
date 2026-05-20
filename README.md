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

```python
from okpalette import create_palette

colors = create_palette(8)
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

## Tune Appearance

By default, white is treated as a background color to avoid.

```python
colors = create_palette(
    32,
    background="#ffffff",
    lightness=(0.20, 0.75),
    chroma=(0.05, None),
)
```

Allow white when you need it:

```python
colors = create_palette(8, background=None, lightness=None, chroma=None)
```

Avoid other colors:

```python
colors = create_palette(
    16,
    avoid_colors=["#000000"],
    background="#ffffff",
)
```

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

## API

```python
create_palette(
    palette_size,
    *,
    seed_colors=(),
    avoid_colors=None,
    background="#ffffff",
    lightness=(0.20, 0.90),
    chroma=(0.04, None),
    hue=None,
    grid_size="medium",
    lightness_weight=1.0,
    chroma_weight=1.0,
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

The result is deterministic, fast, and stable when extending a palette. It is
not a global optimizer.
