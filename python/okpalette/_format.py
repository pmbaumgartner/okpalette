"""Private input normalization and output conversion helpers."""

from __future__ import annotations

import math
import re
from importlib import import_module
from typing import TYPE_CHECKING, List, Optional, Sequence, Tuple, Union, cast

from ._types import (
    BackgroundLike,
    ColorFormat,
    ColorLike,
    GridSize,
    Rgb01,
    Rgb8,
)

if TYPE_CHECKING:
    from ._core import _PaletteGenerator

Palette = Union[List[str], List[Rgb8], List[Rgb01]]


_HEX_RE = re.compile(r"#?([0-9a-fA-F]{3}|[0-9a-fA-F]{6})\Z")
_GRID_STEPS = {"coarse": 16, "medium": 8, "fine": 4}
_FORMATS = {"hex", "rgb", "rgb01"}


def normalize_color(color: ColorLike) -> str:
    if isinstance(color, str):
        return _normalize_hex_color(color)

    if isinstance(color, tuple):
        return _normalize_rgb_tuple(color)

    raise ValueError("color must be a hex string or RGB tuple")


def normalize_color_sequence(
    colors: Optional[Sequence[ColorLike]],
    name: str,
) -> List[str]:
    if colors is None:
        return []

    if isinstance(colors, str):
        raise ValueError(f"{name} must be a sequence of colors, not a string")

    try:
        return [normalize_color(color) for color in colors]
    except TypeError as error:
        raise ValueError(f"{name} must be a sequence of colors") from error


def normalize_optional_color(color: Optional[ColorLike], name: str) -> Optional[str]:
    if color is None:
        return None

    try:
        return normalize_color(color)
    except ValueError as error:
        raise ValueError(f"{name} must be a hex string, RGB tuple, or None") from error


def normalize_background_colors(background: Optional[BackgroundLike], name: str) -> List[str]:
    if background is None:
        return []

    if isinstance(background, str) or _is_rgb_tuple_like(background):
        try:
            return [normalize_color(cast(ColorLike, background))]
        except ValueError as error:
            raise ValueError(f"{name} must be a color, a sequence of colors, or None") from error

    try:
        return normalize_color_sequence(cast(Sequence[ColorLike], background), name)
    except ValueError as error:
        raise ValueError(f"{name} must be a color, a sequence of colors, or None") from error


def resolve_grid_step(grid_size: GridSize) -> int:
    if isinstance(grid_size, str):
        try:
            return _GRID_STEPS[grid_size]
        except KeyError as error:
            raise ValueError(
                "grid_size must be 'coarse', 'medium', 'fine', or an integer in 1..255"
            ) from error

    if type(grid_size) is int:
        return grid_size

    raise ValueError("grid_size must be 'coarse', 'medium', 'fine', or an integer in 1..255")


def validate_positive_size(name: str, value: int) -> int:
    if type(value) is not int:
        raise ValueError(f"{name} must be an integer")

    if value <= 0:
        raise ValueError(f"{name} must be positive")

    return value


def validate_nonnegative_size(name: str, value: int) -> int:
    if type(value) is not int:
        raise ValueError(f"{name} must be an integer")

    if value < 0:
        raise ValueError(f"{name} must be non-negative")

    return value


def validate_format(output_format: object) -> ColorFormat:
    if output_format not in _FORMATS:
        raise ValueError("format must be 'hex', 'rgb', or 'rgb01'")

    return cast(ColorFormat, output_format)


def coerce_float_pair(
    value: Optional[Tuple[float, float]],
    name: str,
) -> Optional[Tuple[float, float]]:
    if value is None:
        return None

    if not isinstance(value, tuple) or len(value) != 2:
        raise ValueError(f"{name} must be a tuple of two floats or None")

    return (
        coerce_float(value[0], f"{name} minimum"),
        coerce_float(value[1], f"{name} maximum"),
    )

def coerce_chroma_pair(
    value: Optional[Tuple[Optional[float], Optional[float]]],
) -> Optional[Tuple[Optional[float], Optional[float]]]:
    if value is None:
        return None
    if not isinstance(value, tuple) or len(value) != 2:
        raise ValueError("chroma must be a tuple of two bounds or None")
    return (
        None if value[0] is None else coerce_float(value[0], "chroma minimum"),
        None if value[1] is None else coerce_float(value[1], "chroma maximum"),
    )


def coerce_float(value: object, name: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{name} must be a number")
    return float(value)


def convert_hex_palette(colors: Sequence[str], output_format: ColorFormat) -> Palette:
    if output_format == "hex":
        return list(colors)

    rgb_colors = [_hex_to_rgb(color) for color in colors]
    if output_format == "rgb":
        return rgb_colors

    return [(r / 255.0, g / 255.0, b / 255.0) for r, g, b in rgb_colors]


def load_palette_generator_rs() -> type[_PaletteGenerator]:
    try:
        core = import_module("okpalette._core")
    except ImportError as error:
        raise ImportError(
            "okpalette native extension is unavailable; install the okpalette wheel "
            "or run `maturin develop` in the source checkout."
        ) from error

    return cast("type[_PaletteGenerator]", getattr(core, "_PaletteGenerator"))


def _normalize_hex_color(color: str) -> str:
    match = _HEX_RE.fullmatch(color)
    if match is None:
        raise ValueError("hex colors must be #RGB, #RRGGBB, RGB, or RRGGBB with ASCII hex digits")

    hex_digits = match.group(1).lower()
    if len(hex_digits) == 3:
        hex_digits = "".join(channel * 2 for channel in hex_digits)

    return f"#{hex_digits}"


def _normalize_rgb_tuple(color: Tuple[object, ...]) -> str:
    if len(color) != 3:
        raise ValueError("RGB tuples must have exactly 3 components")

    if all(type(component) is int for component in color):
        red, green, blue = cast(Rgb8, color)
        for component in (red, green, blue):
            if not 0 <= component <= 255:
                raise ValueError("integer RGB tuple components must be in 0..255")
        return f"#{red:02x}{green:02x}{blue:02x}"

    if all(type(component) is float for component in color):
        red_float, green_float, blue_float = cast(Rgb01, color)
        for component in (red_float, green_float, blue_float):
            if not math.isfinite(component) or not 0.0 <= component <= 1.0:
                raise ValueError("normalized RGB tuple components must be in 0.0..1.0")

        red = int(round(red_float * 255.0))
        green = int(round(green_float * 255.0))
        blue = int(round(blue_float * 255.0))
        return f"#{red:02x}{green:02x}{blue:02x}"

    raise ValueError("RGB tuple components must be all int or all float")


def _is_rgb_tuple_like(value: object) -> bool:
    return (
        isinstance(value, tuple)
        and len(value) == 3
        and (
            all(type(component) is int for component in value)
            or all(type(component) is float for component in value)
        )
    )


def _hex_to_rgb(color: str) -> Rgb8:
    return (int(color[1:3], 16), int(color[3:5], 16), int(color[5:7], 16))
