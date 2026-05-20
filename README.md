# glasbey-rs

Fast, dependency-free Glasbey categorical color palettes for Python, powered by Rust and OKLab.

`glasbey-rs` is the package name on PyPI. Import it as `glasbey_rs` in Python:

```python
from glasbey_rs import create_palette

colors = create_palette(8)
```

The core package has no required Python runtime dependencies. Palette generation runs in a Rust
extension and returns ordinary Python values.

## Installation

```bash
pip install glasbey-rs
```

## Basic Use

Create a categorical palette with lowercase hex colors:

```python
from glasbey_rs import create_palette

colors = create_palette(10)
# ["#080050", "#e00800", "#1078ff", ...]
```

The output is deterministic for the same arguments, so category-to-color assignments stay stable
between runs.

Choose another output format when you want RGB tuples instead of hex strings:

```python
rgb = create_palette(5, format="rgb")
# [(8, 0, 80), (224, 8, 0), ...]

rgb01 = create_palette(5, format="rgb01")
# [(0.03137254901960784, 0.0, 0.3137254901960784), ...]
```

## Extend Existing Colors

Use `extend_palette()` when you already have brand colors or a small palette and want more colors
that avoid crowding the originals.

```python
from glasbey_rs import extend_palette

brand = ["#0057b8", "#ffd700"]
colors = extend_palette(brand, 12)

assert colors[:2] == ["#0057b8", "#ffd700"]
assert len(colors) == 12
```

Set `include_existing=False` to use existing colors as anchors without returning them:

```python
new_colors = extend_palette(brand, 10, include_existing=False)
```

Accepted color inputs are:

```python
"#0fA"
"00ffaa"
(255, 128, 0)
(1.0, 0.5, 0.0)
```

Integer RGB tuples use `0..255`. Normalized RGB tuples use floats in `0.0..1.0`. Ambiguous integer
tuples such as `(1, 0, 0)` are rejected; write `(1.0, 0.0, 0.0)` for normalized RGB.

## White Backgrounds

By default, `create_palette()` treats white (`"#ffffff"`) as a background color to avoid. For plots
on white backgrounds, you can also restrict lightness and require enough chroma to keep colors
visible:

```python
colors = create_palette(
    32,
    background="#ffffff",
    lightness=(0.20, 0.75),
    chroma=(0.05, None),
)
```

Pass `background=None` if white should be allowed in the palette:

```python
colors = create_palette(8, background=None, lightness=None, chroma=None)
```

Use `avoid_colors` for other colors that should repel generated colors without appearing in the
output:

```python
colors = create_palette(
    16,
    avoid_colors=["#000000"],
    background="#ffffff",
)
```

## Warm And Cool Palettes

Hue constraints use OKLCH degrees in `0..360`. Ranges can wrap around zero, which is useful for reds
and warm colors.

```python
warm = create_palette(10, hue=(330, 100))
cool = create_palette(10, hue=(150, 280))
```

Other useful constraints:

```python
muted = create_palette(12, chroma=(0.02, 0.12))
bright = create_palette(12, chroma=(0.10, None))
mid_lightness = create_palette(12, lightness=(0.30, 0.80))
```

`lightness` is OKLab `L` in `0..1`, not `0..100`.

## Grid Size

`grid_size` controls the sRGB candidate grid. Smaller steps search more candidate colors and take
more time.

```python
quick = create_palette(24, grid_size="coarse")  # step 16
default = create_palette(24, grid_size="medium")  # step 8
fine = create_palette(24, grid_size="fine")  # step 4
custom = create_palette(24, grid_size=12)
```

If constraints leave too few candidate colors, `glasbey_rs` raises `ValueError` with a hint to relax
`lightness`, `chroma`, `hue`, or `grid_size`.

## How It Works

A Glasbey palette is built greedily. The algorithm starts with anchor colors such as seeds, avoid
colors, and the background. It then repeatedly chooses the candidate color whose nearest selected or
anchor color is as far away as possible.

This package keeps a running nearest-distance value for each candidate, so each newly selected color
updates the candidate pool once. Distances are measured in OKLab, using squared distance because the
square root is unnecessary for ranking.

OKLab is a perceptual color space: distances are intended to line up better with human color
differences than raw RGB distances. Constraints such as `lightness`, `chroma`, and `hue` are applied
through OKLab and OKLCH before colors are selected.

The result is a deterministic greedy generator, not a global optimizer. That tradeoff keeps results
fast, reproducible, and stable when you extend a palette.

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

