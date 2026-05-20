"""Fast Glasbey categorical color palettes powered by Rust and OKLab."""

from __future__ import annotations

from importlib.metadata import PackageNotFoundError, version
from typing import List, Optional, Sequence, Tuple, cast

from ._format import (
    Palette,
    convert_hex_palette,
    load_generate_palette_rs,
    normalize_color_sequence,
    normalize_optional_color,
    resolve_grid_step,
    validate_chroma,
    validate_format,
    validate_hue,
    validate_lightness,
    validate_positive_size,
    validate_weights,
)
from ._plot import PaletteView, palette_png, palette_svg, save_palette, view_palette
from ._types import ColorFormat, ColorLike, GridSize, Rgb01, Rgb8

try:
    __version__ = version("glasbey-rs")
except PackageNotFoundError:
    __version__ = "0.0.0"

_EXTEND_KWARGS = {
    "avoid_colors",
    "background",
    "lightness",
    "chroma",
    "hue",
    "grid_size",
    "lightness_weight",
    "chroma_weight",
    "format",
}


def create_palette(
    palette_size: int,
    *,
    seed_colors: Sequence[ColorLike] = (),
    avoid_colors: Optional[Sequence[ColorLike]] = None,
    background: Optional[ColorLike] = "#ffffff",
    lightness: Optional[Tuple[float, float]] = (0.20, 0.90),
    chroma: Optional[Tuple[Optional[float], Optional[float]]] = (0.04, None),
    hue: Optional[Tuple[float, float]] = None,
    grid_size: GridSize = "medium",
    lightness_weight: float = 1.0,
    chroma_weight: float = 1.0,
    format: ColorFormat = "hex",
) -> Palette:
    """Create a deterministic Glasbey palette."""

    size = validate_positive_size("palette_size", palette_size)
    output_format = validate_format(format)
    palette = _generate_palette_hex(
        size,
        seed_colors=seed_colors,
        avoid_colors=avoid_colors,
        background=background,
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
        background=cast(Optional[ColorLike], kwargs.get("background", "#ffffff")),
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


def _generate_palette_hex(
    palette_size: int,
    *,
    seed_colors: Sequence[ColorLike],
    avoid_colors: Optional[Sequence[ColorLike]],
    background: Optional[ColorLike],
    lightness: Optional[Tuple[float, float]],
    chroma: Optional[Tuple[Optional[float], Optional[float]]],
    hue: Optional[Tuple[float, float]],
    grid_size: GridSize,
    lightness_weight: float,
    chroma_weight: float,
) -> List[str]:
    seed_hex = normalize_color_sequence(seed_colors, "seed_colors")
    avoid_hex = normalize_color_sequence(avoid_colors, "avoid_colors")
    background_hex = normalize_optional_color(background, "background")
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
        background_hex,
        lightness_bounds,
        chroma_bounds,
        hue_bounds,
        grid_step,
        lightness_weight,
        chroma_weight,
    )


__all__ = [
    "ColorFormat",
    "ColorLike",
    "GridSize",
    "PaletteView",
    "Rgb01",
    "Rgb8",
    "__version__",
    "create_palette",
    "extend_palette",
    "palette_png",
    "palette_svg",
    "save_palette",
    "view_palette",
]
