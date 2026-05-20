---
kata: sd4r
created: 2026-05-20
---

# Position-Aware Label Color Mapping

**Status:** Final recommendation. Decision-owner signoff happened in chat on
2026-05-20.

**Finding:**
`okpalette` should add a position-aware label palette generator. The feature should
use point positions to estimate which labels are locally confusing, then generate
colors so nearby or overlapping labels are more distinct.

**Recommendation:**
Add one user-facing feature with two Python entry points:

```python
create_label_palette(positions, labels, ...)
create_label_palette_from_columns(data, *, positions, label, ...)
```

Both return `dict[label, color]`. Labels may be any hashable Python object. The
dict preserves first-seen label order.

Use Rust for the spatial graph and palette generation. Use Python for input
normalization, arbitrary labels, dataframe-like column extraction, color format
conversion, and error shaping.

## Public API

Core API:

```python
from collections.abc import Hashable, Mapping, Sequence
from typing import Optional, Union

def create_label_palette(
    positions: Sequence[Union[float, Sequence[float]]],
    labels: Sequence[Hashable],
    *,
    fixed_colors: Optional[Mapping[Hashable, ColorLike]] = None,
    seed_colors: Sequence[ColorLike] = (),
    avoid_colors: Optional[Sequence[ColorLike]] = None,
    background: Optional[BackgroundLike] = None,
    background_contrast: Optional[BackgroundContrast] = None,
    lightness: Optional[tuple[float, float]] = (0.20, 0.90),
    chroma: Optional[tuple[Optional[float], Optional[float]]] = (0.04, None),
    hue: Optional[tuple[float, float]] = None,
    grid_size: GridSize = "medium",
    lightness_weight: float = 1.0,
    chroma_weight: float = 1.0,
    neighbors: int = 8,
    max_points: Optional[int] = 50_000,
    format: ColorFormat = "hex",
) -> dict[Hashable, ColorOut]:
    ...
```

Dataframe convenience API:

```python
def create_label_palette_from_columns(
    data: object,
    *,
    positions: Sequence[Hashable],
    label: Hashable,
    **kwargs: object,
) -> dict[Hashable, ColorOut]:
    ...
```

`ColorOut` means `str` for `format="hex"`, `Rgb8` for `format="rgb"`, and
`Rgb01` for `format="rgb01"`.

Do not add `data=`, `x=`, `y=`, `z=`, or `label=` arguments to the core
function. Keep dataframe handling in the wrapper.

## Input Rules

- `len(positions) == len(labels)` is required.
- `positions` may be 1D scalars or 1D, 2D, or 3D coordinate rows.
- Coordinates must be finite.
- Labels must be hashable.
- Label IDs are assigned by first appearance.
- Duplicate positions are allowed.
- `fixed_colors` preassigns colors to labels and returns them unchanged.
- `seed_colors`, `avoid_colors`, `background`, and `background_contrast` keep
  their current `create_palette()` meanings.
- `neighbors` defaults to 8.
- `max_points=50_000` is the default graph budget.
- `max_points=None` opts into exact all-points graph construction.

The core function should accept Python lists, tuples, NumPy arrays, and similar
array-like objects by duck typing. It should not import NumPy.

The dataframe wrapper should support pandas, polars, and similar objects without
runtime dependencies:

1. Read columns with `data[column_name]`.
2. Convert columns with `.to_numpy()`, `.to_list()`, `.tolist()`, or iteration.
3. Combine position columns row-wise.
4. Delegate to `create_label_palette()`.

## Spatial Graph

Build a weighted, undirected label graph. An edge means two labels have nearby
points and should receive distinct colors.

Default graph construction:

1. Normalize positions to Rust vectors plus dimensionality.
2. If input points exceed `max_points`, take a deterministic label-balanced
   sample. Keep rare labels represented.
3. Build a Rust k-d tree with `kiddo`.
4. For each retained point, find nearest different-label contacts.
5. Accumulate label-label edge weights with rank and distance decay.
6. Normalize edge weights to `0..1`.

This treats the graph as a stable estimate of local label confusion, not a
perfect contact census. That matches the goal: fast and good enough.

Use exact all-points preprocessing only when `max_points=None`. Keep exact
all-pairs scans only for tiny inputs or tests.

## Color Generation

Generate colors with the existing candidate constraints:

- lightness
- chroma
- hue
- grid size
- distance weights
- seed colors
- avoid colors
- background
- fixed label colors

Then use the label graph during position-aware assignment:

1. Assign fixed label colors first.
2. Process remaining labels by graph degree, then fixed-neighbor weight, then
   first-seen label ID.
3. Score candidate colors with global OKLab separation plus local graph-neighbor
   separation.
4. Break ties by lower candidate index.
5. Run a small deterministic swap pass over non-fixed labels.

Quality metric:

```text
Q(mapping) = sum(edge_weight(i, j) * OKLab_distance_squared(color_i, color_j))
```

Regression tests should show position-aware assignment beats first-seen
`create_palette(label_count)` assignment on this score for representative
fixtures.

## Why This Design

- It returns the object users need: `dict[label, color]`.
- It keeps the core API clean.
- It supports Python lists, NumPy arrays, pandas, and polars without Python
  runtime dependencies.
- It handles large datasets with a bounded default graph budget.
- It lets users opt into exact graph construction.
- It keeps the package a palette tool, not a plotting or dataframe library.

## Rejected Alternatives

- Fixed-palette reassignment only: too weak for v1 because the graph cannot
  influence which colors enter the palette.
- Centroid-only label graph: misses overlap, multimodal clusters, and boundary
  contacts.
- Exact all-points graph by default: slower than needed for a good-enough
  palette helper.
- No Rust spatial index: forces quadratic scans or default sampling without an
  index.
- Dataframe arguments on `create_label_palette()`: muddies the core API.
- Plotting-library or dataframe-specific dependencies: out of scope.
- Exact/global graph coloring: out of scope for v1.

## Follow-Up

Red implementation issue: `vndt` (`[Red] Implement position-aware label palettes`).

Keep this as one implementation issue for now. Split later only if the work
reveals a real ownership or uncertainty boundary.
