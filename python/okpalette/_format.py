"""Private input normalization and output conversion helpers."""

from __future__ import annotations

import math
import re
from typing import Any, List, Optional, Sequence, Tuple, Union, cast

from ._types import ColorFormat, ColorLike, GridSize, Rgb01, Rgb8

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


def resolve_grid_step(grid_size: GridSize) -> int:
    if isinstance(grid_size, str):
        try:
            return _GRID_STEPS[grid_size]
        except KeyError as error:
            raise ValueError(
                "grid_size must be 'coarse', 'medium', 'fine', or an integer in 1..255"
            ) from error

    if type(grid_size) is int:
        if 1 <= grid_size <= 255:
            return grid_size
        raise ValueError("grid_size must be an integer in 1..255")

    raise ValueError("grid_size must be 'coarse', 'medium', 'fine', or an integer in 1..255")


def validate_positive_size(name: str, value: int) -> int:
    if type(value) is not int:
        raise ValueError(f"{name} must be an integer")

    if value <= 0:
        raise ValueError(f"{name} must be positive")

    return value


def validate_format(output_format: object) -> ColorFormat:
    if output_format not in _FORMATS:
        raise ValueError("format must be 'hex', 'rgb', or 'rgb01'")

    return cast(ColorFormat, output_format)


def validate_lightness(value: Optional[Tuple[float, float]]) -> Optional[Tuple[float, float]]:
    return _validate_float_pair(value, "lightness", minimum=0.0, maximum=1.0, ordered=True)


def validate_chroma(
    value: Optional[Tuple[Optional[float], Optional[float]]],
) -> Optional[Tuple[Optional[float], Optional[float]]]:
    if value is None:
        return None

    if not isinstance(value, tuple) or len(value) != 2:
        raise ValueError("chroma must be a tuple of two bounds or None")

    minimum = None if value[0] is None else _as_float(value[0], "chroma minimum")
    maximum = None if value[1] is None else _as_float(value[1], "chroma maximum")

    if minimum is not None and minimum < 0.0:
        raise ValueError("chroma minimum must be greater than or equal to 0")
    if maximum is not None and maximum < 0.0:
        raise ValueError("chroma maximum must be greater than or equal to 0")
    if minimum is not None and maximum is not None and minimum > maximum:
        raise ValueError("chroma minimum must be less than or equal to maximum")

    return (minimum, maximum)


def validate_hue(value: Optional[Tuple[float, float]]) -> Optional[Tuple[float, float]]:
    return _validate_float_pair(value, "hue", minimum=0.0, maximum=360.0, ordered=False)


def validate_weights(lightness_weight: float, chroma_weight: float) -> Tuple[float, float]:
    lightness = _as_float(lightness_weight, "lightness_weight")
    chroma = _as_float(chroma_weight, "chroma_weight")

    if lightness < 0.0 or chroma < 0.0:
        raise ValueError("distance weights must be greater than or equal to 0")
    if lightness == 0.0 and chroma == 0.0:
        raise ValueError("at least one distance weight must be positive")

    return (lightness, chroma)


def convert_hex_palette(colors: Sequence[str], output_format: ColorFormat) -> Palette:
    if output_format == "hex":
        return list(colors)

    rgb_colors = [_hex_to_rgb(color) for color in colors]
    if output_format == "rgb":
        return rgb_colors

    return [(r / 255.0, g / 255.0, b / 255.0) for r, g, b in rgb_colors]


def load_generate_palette_rs() -> Any:
    try:
        from ._core import generate_palette_rs
    except ImportError as error:
        raise ImportError(
            "okpalette native extension is unavailable; install the okpalette wheel "
            "or run `maturin develop` in the source checkout."
        ) from error

    return generate_palette_rs


def _normalize_hex_color(color: str) -> str:
    match = _HEX_RE.fullmatch(color)
    if match is None:
        raise ValueError(
            "hex colors must be #RGB, #RRGGBB, RGB, or RRGGBB with ASCII hex digits"
        )

    hex_digits = match.group(1).lower()
    if len(hex_digits) == 3:
        hex_digits = "".join(channel * 2 for channel in hex_digits)

    return f"#{hex_digits}"


def _normalize_rgb_tuple(color: Tuple[object, ...]) -> str:
    if len(color) != 3:
        raise ValueError("RGB tuples must have exactly 3 components")

    if all(type(component) is int for component in color):
        red, green, blue = cast(Rgb8, color)
        if all(component in (0, 1) for component in (red, green, blue)):
            raise ValueError(
                "ambiguous integer RGB tuple; use 0..255 integers or 0.0..1.0 floats"
            )
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


def _validate_float_pair(
    value: Optional[Tuple[float, float]],
    name: str,
    *,
    minimum: float,
    maximum: float,
    ordered: bool,
) -> Optional[Tuple[float, float]]:
    if value is None:
        return None

    if not isinstance(value, tuple) or len(value) != 2:
        raise ValueError(f"{name} must be a tuple of two floats or None")

    lower = _as_float(value[0], f"{name} minimum")
    upper = _as_float(value[1], f"{name} maximum")

    if lower < minimum or upper < minimum or lower > maximum or upper > maximum:
        raise ValueError(f"{name} bounds must be in {minimum:g}..{maximum:g}")
    if ordered and lower > upper:
        raise ValueError(f"{name} minimum must be less than or equal to maximum")

    return (lower, upper)


def _as_float(value: object, name: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{name} must be a finite number")

    result = float(value)
    if not math.isfinite(result):
        raise ValueError(f"{name} must be a finite number")

    return result


def _hex_to_rgb(color: str) -> Rgb8:
    return (int(color[1:3], 16), int(color[3:5], 16), int(color[5:7], 16))
