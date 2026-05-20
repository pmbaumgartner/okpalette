"""Dependency-free palette preview helpers."""

from __future__ import annotations

from os import PathLike
from pathlib import Path
from typing import Callable, List, Sequence, Union

from ._format import normalize_color_sequence
from ._types import ColorLike

PathInput = Union[str, PathLike[str]]
SvgRenderer = Callable[[List[str], int, int], str]
PngRenderer = Callable[[List[str], int, int], bytes]
DEFAULT_WIDTH = 1246
DEFAULT_HEIGHT = 154


class PaletteView:
    """Displayable SVG/PNG palette preview."""

    def __init__(
        self,
        palette: Sequence[ColorLike],
        *,
        width: int = DEFAULT_WIDTH,
        height: int = DEFAULT_HEIGHT,
    ) -> None:
        self._colors = _normalize_palette(palette)
        self._width = _validate_dimension("width", width)
        self._height = _validate_dimension("height", height)
        _validate_width_for_palette(self._colors, self._width)

    @property
    def colors(self) -> List[str]:
        return list(self._colors)

    def svg(self) -> str:
        return palette_svg(
            self._colors,
            width=self._width,
            height=self._height,
        )

    def png(self) -> bytes:
        return palette_png(
            self._colors,
            width=self._width,
            height=self._height,
        )

    def save(self, path: PathInput) -> Path:
        return save_palette(
            self._colors,
            path,
            width=self._width,
            height=self._height,
        )

    def _repr_svg_(self) -> str:
        return self.svg()

    def _repr_png_(self) -> bytes:
        return self.png()


def view_palette(
    palette: Sequence[ColorLike],
    *,
    width: int = DEFAULT_WIDTH,
    height: int = DEFAULT_HEIGHT,
) -> PaletteView:
    """Return a notebook-displayable palette preview."""

    return PaletteView(
        palette,
        width=width,
        height=height,
    )


def palette_svg(
    palette: Sequence[ColorLike],
    *,
    width: int = DEFAULT_WIDTH,
    height: int = DEFAULT_HEIGHT,
) -> str:
    """Render a palette as an SVG string."""

    colors = _normalize_palette(palette)
    width = _validate_dimension("width", width)
    height = _validate_dimension("height", height)
    _validate_width_for_palette(colors, width)
    palette_svg_rs, _palette_png_rs = _load_renderers()
    return palette_svg_rs(colors, width, height)


def palette_png(
    palette: Sequence[ColorLike],
    *,
    width: int = DEFAULT_WIDTH,
    height: int = DEFAULT_HEIGHT,
) -> bytes:
    """Render a palette as PNG bytes."""

    colors = _normalize_palette(palette)
    width = _validate_dimension("width", width)
    height = _validate_dimension("height", height)
    _validate_width_for_palette(colors, width)
    _palette_svg_rs, palette_png_rs = _load_renderers()
    return palette_png_rs(colors, width, height)


def save_palette(
    palette: Sequence[ColorLike],
    path: PathInput,
    *,
    width: int = DEFAULT_WIDTH,
    height: int = DEFAULT_HEIGHT,
) -> Path:
    """Save a palette preview to an SVG or PNG file."""

    output_path = Path(path)
    suffix = output_path.suffix.lower()
    if suffix == ".svg":
        output_path.write_text(
            palette_svg(
                palette,
                width=width,
                height=height,
            ),
            encoding="utf-8",
        )
    elif suffix == ".png":
        output_path.write_bytes(
            palette_png(
                palette,
                width=width,
                height=height,
            )
        )
    else:
        raise ValueError("path must end with .svg or .png")

    return output_path


def _normalize_palette(palette: Sequence[ColorLike]) -> List[str]:
    colors = normalize_color_sequence(palette, "palette")
    if not colors:
        raise ValueError("palette must contain at least one color")
    return colors


def _validate_dimension(name: str, value: int) -> int:
    if type(value) is not int:
        raise ValueError(f"{name} must be an integer")
    if value <= 0:
        raise ValueError(f"{name} must be positive")
    return value


def _validate_width_for_palette(colors: Sequence[str], width: int) -> None:
    if width < len(colors):
        raise ValueError("width must be at least the number of colors")


def _load_renderers() -> tuple[SvgRenderer, PngRenderer]:
    try:
        from ._core import palette_png_rs, palette_svg_rs
    except ImportError as error:
        raise ImportError(
            "okpalette native extension is unavailable; install the okpalette wheel "
            "or run `maturin develop` in the source checkout."
        ) from error

    return palette_svg_rs, palette_png_rs


__all__ = [
    "DEFAULT_HEIGHT",
    "DEFAULT_WIDTH",
    "PaletteView",
    "palette_png",
    "palette_svg",
    "save_palette",
    "view_palette",
]
