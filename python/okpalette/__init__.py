"""Fast OKLab categorical color palettes powered by Rust."""

from __future__ import annotations

from collections.abc import Mapping as MappingABC
from dataclasses import dataclass
from importlib.metadata import PackageNotFoundError, version
from typing import TYPE_CHECKING, Hashable, List, Optional, Sequence, Tuple, Union, cast

from ._format import (
    Palette,
    coerce_chroma_pair,
    coerce_float,
    coerce_float_pair,
    convert_hex_palette,
    load_palette_generator_rs,
    normalize_background_colors,
    normalize_color_sequence,
    resolve_grid_step,
    validate_format,
    validate_nonnegative_size,
    validate_positive_size,
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

if TYPE_CHECKING:
    from ._core import _PaletteGenerator

try:
    __version__ = version("okpalette")
except PackageNotFoundError:
    __version__ = "0.0.0"

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
    """Create a deterministic categorical palette; zero returns an empty palette."""

    size = validate_nonnegative_size("palette_size", palette_size)
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
    """Extend to a final target size, optionally returning only the generated portion."""

    if type(include_existing) is not bool:
        raise ValueError("include_existing must be a boolean")

    existing = normalize_color_sequence(colors, "colors")
    target = validate_nonnegative_size("target_size", target_size)
    output_format = validate_format(format)
    palette_options = _PaletteOptions(
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
    )

    generator = _build_generator(palette_options)
    palette = generator.extend(existing, target)
    if not include_existing:
        palette = palette[len(existing) :]
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

    return create_label_palette(
        cast(Sequence[Union[float, Sequence[float]]], combined_positions),
        cast(Sequence[Hashable], label_values),
        fixed_colors=fixed_colors,
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
        neighbors=neighbors,
        max_points=max_points,
        format=format,
    )


def _generate_palette_hex(
    palette_size: int,
    options: _PaletteOptions,
) -> List[str]:
    generator = _build_generator(options)

    if palette_size == 0:
        return []

    return generator.generate(palette_size)


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
    generator = _build_generator(options)
    neighbors = validate_positive_size("neighbors", neighbors)
    if max_points is not None:
        max_points = validate_positive_size("max_points", max_points)

    return generator.generate_for_labels(
        list(coordinates),
        dimension,
        list(label_ids),
        label_count,
        list(fixed_colors),
        neighbors,
        max_points,
    )


def _build_generator(options: _PaletteOptions) -> _PaletteGenerator:
    return load_palette_generator_rs()(
        normalize_color_sequence(options.seed_colors, "seed_colors"),
        normalize_color_sequence(options.avoid_colors, "avoid_colors"),
        None
        if options.background is None
        else normalize_background_colors(options.background, "background"),
        options.background_contrast,
        coerce_float_pair(options.lightness, "lightness"),
        coerce_chroma_pair(options.chroma),
        coerce_float_pair(options.hue, "hue"),
        resolve_grid_step(options.grid_size),
        coerce_float(options.lightness_weight, "lightness_weight"),
        coerce_float(options.chroma_weight, "chroma_weight"),
        options.colorblind_mode,
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
