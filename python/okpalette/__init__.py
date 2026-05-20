"""Fast OKLab categorical color palettes powered by Rust."""

from __future__ import annotations

import math
from collections.abc import Hashable as HashableABC
from collections.abc import Mapping as MappingABC
from importlib.metadata import PackageNotFoundError, version
from typing import Any, Hashable, List, Optional, Sequence, Tuple, Union, cast

from ._format import (
    Palette,
    convert_hex_palette,
    load_generate_label_palette_rs,
    load_generate_palette_rs,
    normalize_background_colors,
    normalize_color,
    normalize_color_sequence,
    resolve_grid_step,
    validate_background_contrast,
    validate_chroma,
    validate_format,
    validate_hue,
    validate_lightness,
    validate_positive_size,
    validate_weights,
)
from ._plot import PaletteView, palette_png, palette_svg, save_palette, view_palette
from ._types import (
    BackgroundContrast,
    BackgroundLike,
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
    "format",
}

ColorOut = Union[str, Rgb8, Rgb01]


def create_palette(
    palette_size: int,
    *,
    seed_colors: Sequence[ColorLike] = (),
    avoid_colors: Optional[Sequence[ColorLike]] = None,
    background: Optional[BackgroundLike] = "#ffffff",
    background_contrast: BackgroundContrast = "normal",
    lightness: Optional[Tuple[float, float]] = (0.20, 0.90),
    chroma: Optional[Tuple[Optional[float], Optional[float]]] = (0.04, None),
    hue: Optional[Tuple[float, float]] = None,
    grid_size: GridSize = "medium",
    lightness_weight: float = 1.0,
    chroma_weight: float = 1.0,
    format: ColorFormat = "hex",
) -> Palette:
    """Create a deterministic categorical palette."""

    size = validate_positive_size("palette_size", palette_size)
    output_format = validate_format(format)
    palette = _generate_palette_hex(
        size,
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

    if include_existing and target < len(existing):
        raise ValueError("target_size must be greater than or equal to len(colors)")

    generated_size = target - len(existing) if include_existing else target
    generated = _generate_palette_hex(
        generated_size,
        seed_colors=existing,
        avoid_colors=cast(Optional[Sequence[ColorLike]], kwargs.get("avoid_colors")),
        background=cast(Optional[BackgroundLike], kwargs.get("background", "#ffffff")),
        background_contrast=cast(
            BackgroundContrast,
            kwargs.get("background_contrast", "normal"),
        ),
        lightness=cast(Optional[Tuple[float, float]], kwargs.get("lightness", (0.20, 0.90))),
        chroma=cast(
            Optional[Tuple[Optional[float], Optional[float]]],
            kwargs.get("chroma", (0.04, None)),
        ),
        hue=cast(Optional[Tuple[float, float]], kwargs.get("hue")),
        grid_size=cast(GridSize, kwargs.get("grid_size", "medium")),
        lightness_weight=cast(float, kwargs.get("lightness_weight", 1.0)),
        chroma_weight=cast(float, kwargs.get("chroma_weight", 1.0)),
    )
    palette = existing + generated if include_existing else generated
    return convert_hex_palette(palette, output_format)


def create_label_palette(
    positions: Sequence[Union[float, Sequence[float]]],
    labels: Sequence[Hashable],
    *,
    fixed_colors: Optional[MappingABC[Hashable, ColorLike]] = None,
    seed_colors: Sequence[ColorLike] = (),
    avoid_colors: Optional[Sequence[ColorLike]] = None,
    background: Optional[BackgroundLike] = "#ffffff",
    background_contrast: BackgroundContrast = "normal",
    lightness: Optional[Tuple[float, float]] = (0.20, 0.90),
    chroma: Optional[Tuple[Optional[float], Optional[float]]] = (0.04, None),
    hue: Optional[Tuple[float, float]] = None,
    grid_size: GridSize = "medium",
    lightness_weight: float = 1.0,
    chroma_weight: float = 1.0,
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
    *,
    seed_colors: Sequence[ColorLike],
    avoid_colors: Optional[Sequence[ColorLike]],
    background: Optional[BackgroundLike],
    background_contrast: BackgroundContrast,
    lightness: Optional[Tuple[float, float]],
    chroma: Optional[Tuple[Optional[float], Optional[float]]],
    hue: Optional[Tuple[float, float]],
    grid_size: GridSize,
    lightness_weight: float,
    chroma_weight: float,
) -> List[str]:
    seed_hex = normalize_color_sequence(seed_colors, "seed_colors")
    avoid_hex = normalize_color_sequence(avoid_colors, "avoid_colors")
    background_hex = normalize_background_colors(background, "background")
    background_distance = validate_background_contrast(background_contrast)
    lightness_bounds = validate_lightness(lightness)
    chroma_bounds = validate_chroma(chroma)
    hue_bounds = validate_hue(hue)
    grid_step = resolve_grid_step(grid_size)
    lightness_weight, chroma_weight = validate_weights(lightness_weight, chroma_weight)
    generate_palette_rs = load_generate_palette_rs()

    if palette_size == 0:
        return []

    return generate_palette_rs(
        palette_size,
        seed_hex or None,
        avoid_hex or None,
        background_hex or None,
        background_distance,
        lightness_bounds,
        chroma_bounds,
        hue_bounds,
        grid_step,
        lightness_weight,
        chroma_weight,
    )


def _generate_label_palette_hex(
    coordinates: Sequence[float],
    dimension: int,
    label_ids: Sequence[int],
    label_count: int,
    fixed_colors: Sequence[Optional[str]],
    *,
    seed_colors: Sequence[ColorLike],
    avoid_colors: Optional[Sequence[ColorLike]],
    background: Optional[BackgroundLike],
    background_contrast: BackgroundContrast,
    lightness: Optional[Tuple[float, float]],
    chroma: Optional[Tuple[Optional[float], Optional[float]]],
    hue: Optional[Tuple[float, float]],
    grid_size: GridSize,
    lightness_weight: float,
    chroma_weight: float,
    neighbors: int,
    max_points: Optional[int],
) -> List[str]:
    seed_hex = normalize_color_sequence(seed_colors, "seed_colors")
    avoid_hex = normalize_color_sequence(avoid_colors, "avoid_colors")
    background_hex = normalize_background_colors(background, "background")
    background_distance = validate_background_contrast(background_contrast)
    lightness_bounds = validate_lightness(lightness)
    chroma_bounds = validate_chroma(chroma)
    hue_bounds = validate_hue(hue)
    grid_step = resolve_grid_step(grid_size)
    lightness_weight, chroma_weight = validate_weights(lightness_weight, chroma_weight)
    neighbors = _validate_positive_int("neighbors", neighbors)
    max_points = _validate_optional_positive_int("max_points", max_points)
    generate_label_palette_rs = load_generate_label_palette_rs()

    return generate_label_palette_rs(
        list(coordinates),
        dimension,
        list(label_ids),
        label_count,
        list(fixed_colors),
        seed_hex or None,
        avoid_hex or None,
        background_hex or None,
        background_distance,
        lightness_bounds,
        chroma_bounds,
        hue_bounds,
        grid_step,
        lightness_weight,
        chroma_weight,
        neighbors,
        max_points,
    )


def _normalize_labels(
    labels: Sequence[Hashable],
) -> tuple[list[Hashable], list[int], dict[Hashable, int]]:
    if isinstance(labels, (str, bytes)):
        raise ValueError("labels must be a sequence of hashable objects, not a string")

    try:
        label_values = list(labels)
    except TypeError as error:
        raise ValueError("labels must be a sequence of hashable objects") from error

    ordered_labels: list[Hashable] = []
    label_ids: list[int] = []
    label_to_id: dict[Hashable, int] = {}

    for label in label_values:
        if not isinstance(label, HashableABC):
            raise ValueError("labels must be hashable")

        try:
            label_id = label_to_id[label]
        except TypeError as error:
            raise ValueError("labels must be hashable") from error
        except KeyError:
            label_id = len(ordered_labels)
            label_to_id[label] = label_id
            ordered_labels.append(label)

        label_ids.append(label_id)

    return ordered_labels, label_ids, label_to_id


def _normalize_positions(
    positions: Sequence[Union[float, Sequence[float]]],
    expected_length: int,
) -> tuple[list[float], int]:
    if isinstance(positions, (str, bytes)):
        raise ValueError("positions must be a sequence of coordinates, not a string")

    try:
        position_values = list(positions)
    except TypeError as error:
        raise ValueError("positions must be a sequence of coordinates") from error

    if len(position_values) != expected_length:
        raise ValueError("positions and labels must have the same length")

    if not position_values:
        return [], 1

    rows: list[list[float]] = []
    expected_dimension: Optional[int] = None
    expected_kind: Optional[str] = None
    for position in position_values:
        row, kind = _normalize_position(position)
        if expected_kind is None:
            expected_kind = kind
        elif kind != expected_kind:
            raise ValueError("positions must be all scalars or all coordinate rows")

        if expected_dimension is None:
            expected_dimension = len(row)
        elif len(row) != expected_dimension:
            raise ValueError("positions must all have the same dimensionality")

        rows.append(row)

    dimension = expected_dimension or 1
    return [coordinate for row in rows for coordinate in row], dimension


def _normalize_position(position: object) -> tuple[list[float], str]:
    if _is_coordinate_scalar(position):
        return [_as_coordinate(position)], "scalar"

    if isinstance(position, (str, bytes)):
        raise ValueError("positions must contain numeric coordinates")

    try:
        row = list(cast(Any, position))
    except TypeError as error:
        raise ValueError("positions must contain numeric coordinates") from error

    if not 1 <= len(row) <= 3:
        raise ValueError("position coordinate rows must have length 1, 2, or 3")

    return [_as_coordinate(coordinate) for coordinate in row], "row"


def _is_coordinate_scalar(value: object) -> bool:
    if isinstance(value, bool):
        return False

    try:
        float(cast(Any, value))
    except (TypeError, ValueError):
        return False

    return True


def _as_coordinate(value: object) -> float:
    if isinstance(value, bool):
        raise ValueError("coordinates must be finite numbers")

    try:
        coordinate = float(cast(Any, value))
    except (TypeError, ValueError) as error:
        raise ValueError("coordinates must be finite numbers") from error

    if not math.isfinite(coordinate):
        raise ValueError("coordinates must be finite numbers")

    return coordinate


def _normalize_fixed_colors(
    fixed_colors: Optional[MappingABC[Hashable, ColorLike]],
    label_to_id: MappingABC[Hashable, int],
    label_count: int,
) -> list[Optional[str]]:
    fixed_hex: list[Optional[str]] = [None] * label_count
    if fixed_colors is None:
        return fixed_hex

    if not isinstance(fixed_colors, MappingABC):
        raise ValueError("fixed_colors must be a mapping from label to color")

    for label, color in fixed_colors.items():
        if not isinstance(label, HashableABC):
            raise ValueError("fixed_colors labels must be hashable")

        try:
            label_id = label_to_id[label]
        except TypeError as error:
            raise ValueError("fixed_colors labels must be hashable") from error
        except KeyError as error:
            raise ValueError("fixed_colors labels must be present in labels") from error

        fixed_hex[label_id] = normalize_color(color)

    return fixed_hex


def _validate_positive_int(name: str, value: int) -> int:
    if type(value) is not int:
        raise ValueError(f"{name} must be an integer")

    if value <= 0:
        raise ValueError(f"{name} must be positive")

    return value


def _validate_optional_positive_int(name: str, value: Optional[int]) -> Optional[int]:
    if value is None:
        return None

    return _validate_positive_int(name, value)


def _normalize_position_columns(positions: Sequence[Hashable]) -> list[Hashable]:
    if isinstance(positions, (str, bytes)):
        raise ValueError("positions must be a sequence of column names")

    try:
        columns = list(positions)
    except TypeError as error:
        raise ValueError("positions must be a sequence of column names") from error

    if not 1 <= len(columns) <= 3:
        raise ValueError("positions must name 1, 2, or 3 columns")

    return columns


def _read_column(data: object, column: Hashable) -> object:
    try:
        return cast(Any, data)[column]
    except Exception as error:
        raise ValueError(f"could not read column {column!r}") from error


def _column_to_list(column_data: object, column: Hashable) -> list[object]:
    for method_name in ("to_numpy", "to_list", "tolist"):
        method = getattr(column_data, method_name, None)
        if callable(method):
            try:
                return list(method())
            except TypeError as error:
                raise ValueError(f"column {column!r} must be iterable") from error

    try:
        return list(cast(Any, column_data))
    except TypeError as error:
        raise ValueError(f"column {column!r} must be iterable") from error


__all__ = [
    "BackgroundContrast",
    "BackgroundLike",
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
