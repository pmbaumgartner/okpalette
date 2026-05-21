"""Fast OKLab categorical color palettes powered by Rust."""

from __future__ import annotations

from collections.abc import Mapping as MappingABC
from dataclasses import dataclass
from importlib.metadata import PackageNotFoundError, version
from typing import Any, Hashable, List, Optional, Sequence, Tuple, Union, cast

from ._format import (
    Palette,
    convert_hex_palette,
    load_generate_label_palette_rs,
    load_generate_palette_rs,
    normalize_background_colors,
    normalize_color_sequence,
    resolve_grid_step,
    validate_background_contrast,
    validate_chroma,
    validate_colorblind_mode,
    validate_format,
    validate_hue,
    validate_lightness,
    validate_positive_size,
    validate_weights,
)
from ._label import (
    _column_to_list,
    _normalize_fixed_colors,
    _normalize_labels,
    _normalize_position_columns,
    _normalize_positions,
    _read_column,
)
from ._plot import PaletteView, palette_png, palette_svg, save_palette, view_palette
from ._types import (
    BackgroundContrast,
    BackgroundLike,
    ColorblindMode,
    ColorFormat,
    ColorLike,
    GridSize,
    Rgb01,
    Rgb8,
)

try:
    __version__ = version("okpalette")
except PackageNotFoundError:
    __version__ = "0.0.0"

_EXTEND_KWARGS = {
    "avoid_colors",
    "background",
    "background_contrast",
    "lightness",
    "chroma",
    "hue",
    "grid_size",
    "lightness_weight",
    "chroma_weight",
    "colorblind_mode",
    "format",
}

ColorOut = Union[str, Rgb8, Rgb01]


@dataclass(frozen=True)
class _PaletteOptions:
    seed_colors: Sequence[ColorLike] = ()
    avoid_colors: Optional[Sequence[ColorLike]] = None
    background: Optional[BackgroundLike] = None
    background_contrast: Optional[BackgroundContrast] = None
    lightness: Optional[Tuple[float, float]] = (0.20, 0.90)
    chroma: Optional[Tuple[Optional[float], Optional[float]]] = (0.04, None)
    hue: Optional[Tuple[float, float]] = None
    grid_size: GridSize = "medium"
    lightness_weight: float = 1.0
    chroma_weight: float = 1.0
    colorblind_mode: Optional[ColorblindMode] = None


@dataclass(frozen=True)
class _NormalizedPaletteOptions:
    seed_hex: List[str]
    avoid_hex: List[str]
    background_hex: List[str]
    background_contrast: Optional[BackgroundContrast]
    lightness_bounds: Optional[Tuple[float, float]]
    chroma_bounds: Optional[Tuple[Optional[float], Optional[float]]]
    hue_bounds: Optional[Tuple[float, float]]
    grid_step: int
    lightness_weight: float
    chroma_weight: float
    colorblind_mode: Optional[ColorblindMode]


def create_palette(
    palette_size: int,
    *,
    seed_colors: Sequence[ColorLike] = (),
    avoid_colors: Optional[Sequence[ColorLike]] = None,
    background: Optional[BackgroundLike] = None,
    background_contrast: Optional[BackgroundContrast] = None,
    lightness: Optional[Tuple[float, float]] = (0.20, 0.90),
    chroma: Optional[Tuple[Optional[float], Optional[float]]] = (0.04, None),
    hue: Optional[Tuple[float, float]] = None,
    grid_size: GridSize = "medium",
    lightness_weight: float = 1.0,
    chroma_weight: float = 1.0,
    colorblind_mode: Optional[ColorblindMode] = None,
    format: ColorFormat = "hex",
) -> Palette:
    """Create a deterministic categorical palette."""

    size = validate_positive_size("palette_size", palette_size)
    output_format = validate_format(format)
    palette = _generate_palette_hex(
        size,
        _PaletteOptions(
            seed_colors=seed_colors,
            avoid_colors=avoid_colors,
            background=background,
            background_contrast=background_contrast,
            lightness=lightness,
            chroma=chroma,
            hue=hue,
            grid_size=grid_size,
            lightness_weight=lightness_weight,
            chroma_weight=chroma_weight,
            colorblind_mode=colorblind_mode,
        ),
    )
    return convert_hex_palette(palette, output_format)


def extend_palette(
    colors: Sequence[ColorLike],
    target_size: int,
    *,
    include_existing: bool = True,
    **kwargs: object,
) -> Palette:
    """Extend an existing palette to a target size."""

    unexpected = sorted(set(kwargs) - _EXTEND_KWARGS)
    if unexpected:
        name = unexpected[0]
        raise TypeError(f"extend_palette() got an unexpected keyword argument {name!r}")

    if type(include_existing) is not bool:
        raise ValueError("include_existing must be a boolean")

    existing = normalize_color_sequence(colors, "colors")
    target = validate_positive_size("target_size", target_size)
    output_format = validate_format(kwargs.get("format", "hex"))
    palette_options = _extend_palette_options(existing, kwargs)

    if include_existing and target < len(existing):
        raise ValueError("target_size must be greater than or equal to len(colors)")

    generated_size = target - len(existing) if include_existing else target
    generated = _generate_palette_hex(generated_size, palette_options)
    palette = existing + generated if include_existing else generated
    return convert_hex_palette(palette, output_format)


def create_label_palette(
    positions: Sequence[Union[float, Sequence[float]]],
    labels: Sequence[Hashable],
    *,
    fixed_colors: Optional[MappingABC[Hashable, ColorLike]] = None,
    seed_colors: Sequence[ColorLike] = (),
    avoid_colors: Optional[Sequence[ColorLike]] = None,
    background: Optional[BackgroundLike] = None,
    background_contrast: Optional[BackgroundContrast] = None,
    lightness: Optional[Tuple[float, float]] = (0.20, 0.90),
    chroma: Optional[Tuple[Optional[float], Optional[float]]] = (0.04, None),
    hue: Optional[Tuple[float, float]] = None,
    grid_size: GridSize = "medium",
    lightness_weight: float = 1.0,
    chroma_weight: float = 1.0,
    colorblind_mode: Optional[ColorblindMode] = None,
    neighbors: int = 8,
    max_points: Optional[int] = 50_000,
    format: ColorFormat = "hex",
) -> dict[Hashable, ColorOut]:
    """Create a deterministic palette keyed by labels and informed by positions."""

    output_format = validate_format(format)
    ordered_labels, label_ids, label_to_id = _normalize_labels(labels)
    coordinates, dimension = _normalize_positions(positions, len(label_ids))
    fixed_hex = _normalize_fixed_colors(fixed_colors, label_to_id, len(ordered_labels))

    if not ordered_labels:
        return {}

    palette = _generate_label_palette_hex(
        coordinates,
        dimension,
        label_ids,
        len(ordered_labels),
        fixed_hex,
        options=_PaletteOptions(
            seed_colors=seed_colors,
            avoid_colors=avoid_colors,
            background=background,
            background_contrast=background_contrast,
            lightness=lightness,
            chroma=chroma,
            hue=hue,
            grid_size=grid_size,
            lightness_weight=lightness_weight,
            chroma_weight=chroma_weight,
            colorblind_mode=colorblind_mode,
        ),
        neighbors=neighbors,
        max_points=max_points,
    )
    converted = convert_hex_palette(palette, output_format)
    return dict(zip(ordered_labels, cast(Sequence[ColorOut], converted)))


def create_label_palette_from_columns(
    data: object,
    *,
    positions: Sequence[Hashable],
    label: Hashable,
    **kwargs: object,
) -> dict[Hashable, ColorOut]:
    """Create a label palette from dataframe-like columns."""

    position_columns = _normalize_position_columns(positions)
    label_values = _column_to_list(_read_column(data, label), label)
    position_values = [
        _column_to_list(_read_column(data, column), column) for column in position_columns
    ]

    for column, values in zip(position_columns, position_values):
        if len(values) != len(label_values):
            raise ValueError(f"position column {column!r} length must match label column length")

    if len(position_values) == 1:
        combined_positions = position_values[0]
    else:
        combined_positions = list(zip(*position_values))

    create_label_palette_any = cast(Any, create_label_palette)
    return cast(
        dict[Hashable, ColorOut],
        create_label_palette_any(
            cast(Sequence[Union[float, Sequence[float]]], combined_positions),
            cast(Sequence[Hashable], label_values),
            **kwargs,
        ),
    )


def _generate_palette_hex(
    palette_size: int,
    options: _PaletteOptions,
) -> List[str]:
    normalized = _normalize_palette_options(options)
    generate_palette_rs = load_generate_palette_rs()

    if palette_size == 0:
        return []

    return generate_palette_rs(
        palette_size,
        normalized.seed_hex or None,
        normalized.avoid_hex or None,
        normalized.background_hex or None,
        normalized.background_contrast,
        normalized.lightness_bounds,
        normalized.chroma_bounds,
        normalized.hue_bounds,
        normalized.grid_step,
        normalized.lightness_weight,
        normalized.chroma_weight,
        normalized.colorblind_mode,
    )


def _generate_label_palette_hex(
    coordinates: Sequence[float],
    dimension: int,
    label_ids: Sequence[int],
    label_count: int,
    fixed_colors: Sequence[Optional[str]],
    *,
    options: _PaletteOptions,
    neighbors: int,
    max_points: Optional[int],
) -> List[str]:
    normalized = _normalize_palette_options(options)
    neighbors = validate_positive_size("neighbors", neighbors)
    if max_points is not None:
        max_points = validate_positive_size("max_points", max_points)
    generate_label_palette_rs = load_generate_label_palette_rs()

    return generate_label_palette_rs(
        list(coordinates),
        dimension,
        list(label_ids),
        label_count,
        list(fixed_colors),
        normalized.seed_hex or None,
        normalized.avoid_hex or None,
        normalized.background_hex or None,
        normalized.background_contrast,
        normalized.lightness_bounds,
        normalized.chroma_bounds,
        normalized.hue_bounds,
        normalized.grid_step,
        normalized.lightness_weight,
        normalized.chroma_weight,
        normalized.colorblind_mode,
        neighbors,
        max_points,
    )


def _extend_palette_options(
    seed_colors: Sequence[ColorLike],
    kwargs: MappingABC[str, object],
) -> _PaletteOptions:
    return _PaletteOptions(
        seed_colors=seed_colors,
        avoid_colors=cast(Optional[Sequence[ColorLike]], kwargs.get("avoid_colors")),
        background=cast(Optional[BackgroundLike], kwargs.get("background")),
        background_contrast=cast(
            Optional[BackgroundContrast],
            kwargs.get("background_contrast"),
        ),
        lightness=cast(
            Optional[Tuple[float, float]],
            kwargs.get("lightness", (0.20, 0.90)),
        ),
        chroma=cast(
            Optional[Tuple[Optional[float], Optional[float]]],
            kwargs.get("chroma", (0.04, None)),
        ),
        hue=cast(Optional[Tuple[float, float]], kwargs.get("hue")),
        grid_size=cast(GridSize, kwargs.get("grid_size", "medium")),
        lightness_weight=cast(float, kwargs.get("lightness_weight", 1.0)),
        chroma_weight=cast(float, kwargs.get("chroma_weight", 1.0)),
        colorblind_mode=cast(Optional[ColorblindMode], kwargs.get("colorblind_mode")),
    )


def _normalize_palette_options(options: _PaletteOptions) -> _NormalizedPaletteOptions:
    seed_hex = normalize_color_sequence(options.seed_colors, "seed_colors")
    avoid_hex = normalize_color_sequence(options.avoid_colors, "avoid_colors")
    background_contrast = validate_background_contrast(options.background_contrast)
    if options.background is None:
        if background_contrast is not None:
            raise ValueError("background must be provided when background_contrast is set")
        background_hex: List[str] = []
    else:
        if background_contrast is None:
            raise ValueError("background_contrast must be provided when background is set")
        background_hex = normalize_background_colors(options.background, "background")
        if not background_hex:
            raise ValueError("background must contain at least one color")
    lightness_bounds = validate_lightness(options.lightness)
    chroma_bounds = validate_chroma(options.chroma)
    hue_bounds = validate_hue(options.hue)
    grid_step = resolve_grid_step(options.grid_size)
    lightness_weight, chroma_weight = validate_weights(
        options.lightness_weight,
        options.chroma_weight,
    )
    colorblind_mode = validate_colorblind_mode(options.colorblind_mode)
    return _NormalizedPaletteOptions(
        seed_hex=seed_hex,
        avoid_hex=avoid_hex,
        background_hex=background_hex,
        background_contrast=background_contrast,
        lightness_bounds=lightness_bounds,
        chroma_bounds=chroma_bounds,
        hue_bounds=hue_bounds,
        grid_step=grid_step,
        lightness_weight=lightness_weight,
        chroma_weight=chroma_weight,
        colorblind_mode=colorblind_mode,
    )


__all__ = [
    "BackgroundContrast",
    "BackgroundLike",
    "ColorblindMode",
    "ColorFormat",
    "ColorLike",
    "GridSize",
    "PaletteView",
    "Rgb01",
    "Rgb8",
    "__version__",
    "create_label_palette",
    "create_label_palette_from_columns",
    "create_palette",
    "extend_palette",
    "palette_png",
    "palette_svg",
    "save_palette",
    "view_palette",
]
