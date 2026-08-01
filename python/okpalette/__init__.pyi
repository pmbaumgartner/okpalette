from collections.abc import Hashable, Mapping, Sequence
from os import PathLike
from pathlib import Path
from typing import Literal, TypedDict, Unpack, overload

from ._plot import PaletteView as PaletteView
from ._types import (
    BackgroundContrast as BackgroundContrast,
    BackgroundLike as BackgroundLike,
    ColorblindMode as ColorblindMode,
    ColorFormat as ColorFormat,
    ColorLike as ColorLike,
    GridSize as GridSize,
    Rgb01 as Rgb01,
    Rgb8 as Rgb8,
)

__version__: str

type Palette = list[str] | list[Rgb8] | list[Rgb01]
type ColorOut = str | Rgb8 | Rgb01

class _PaletteKwargs(TypedDict, total=False):
    seed_colors: Sequence[ColorLike]
    avoid_colors: Sequence[ColorLike] | None
    background: BackgroundLike | None
    background_contrast: BackgroundContrast | None
    lightness: tuple[float, float] | None
    chroma: tuple[float | None, float | None] | None
    hue: tuple[float, float] | None
    grid_size: GridSize
    lightness_weight: float
    chroma_weight: float
    colorblind_mode: ColorblindMode | None

class _ExtendKwargs(_PaletteKwargs, total=False):
    include_existing: bool

class _LabelKwargs(_PaletteKwargs, total=False):
    fixed_colors: Mapping[Hashable, ColorLike] | None
    neighbors: int
    max_points: int | None

@overload
def create_palette(
    palette_size: int,
    *,
    format: Literal["hex"] = "hex",
    **kwargs: Unpack[_PaletteKwargs],
) -> list[str]: ...
@overload
def create_palette(
    palette_size: int,
    *,
    format: Literal["rgb"],
    **kwargs: Unpack[_PaletteKwargs],
) -> list[Rgb8]: ...
@overload
def create_palette(
    palette_size: int,
    *,
    format: Literal["rgb01"],
    **kwargs: Unpack[_PaletteKwargs],
) -> list[Rgb01]: ...
@overload
def create_palette(
    palette_size: int,
    *,
    format: ColorFormat,
    **kwargs: Unpack[_PaletteKwargs],
) -> Palette: ...

@overload
def extend_palette(
    colors: Sequence[ColorLike],
    target_size: int,
    *,
    format: Literal["hex"] = "hex",
    **kwargs: Unpack[_ExtendKwargs],
) -> list[str]: ...
@overload
def extend_palette(
    colors: Sequence[ColorLike],
    target_size: int,
    *,
    format: Literal["rgb"],
    **kwargs: Unpack[_ExtendKwargs],
) -> list[Rgb8]: ...
@overload
def extend_palette(
    colors: Sequence[ColorLike],
    target_size: int,
    *,
    format: Literal["rgb01"],
    **kwargs: Unpack[_ExtendKwargs],
) -> list[Rgb01]: ...
@overload
def extend_palette(
    colors: Sequence[ColorLike],
    target_size: int,
    *,
    format: ColorFormat,
    **kwargs: Unpack[_ExtendKwargs],
) -> Palette: ...

@overload
def create_label_palette(
    positions: Sequence[float | Sequence[float]],
    labels: Sequence[Hashable],
    *,
    format: Literal["hex"] = "hex",
    **kwargs: Unpack[_LabelKwargs],
) -> dict[Hashable, str]: ...
@overload
def create_label_palette(
    positions: Sequence[float | Sequence[float]],
    labels: Sequence[Hashable],
    *,
    format: Literal["rgb"],
    **kwargs: Unpack[_LabelKwargs],
) -> dict[Hashable, Rgb8]: ...
@overload
def create_label_palette(
    positions: Sequence[float | Sequence[float]],
    labels: Sequence[Hashable],
    *,
    format: Literal["rgb01"],
    **kwargs: Unpack[_LabelKwargs],
) -> dict[Hashable, Rgb01]: ...
@overload
def create_label_palette(
    positions: Sequence[float | Sequence[float]],
    labels: Sequence[Hashable],
    *,
    format: ColorFormat,
    **kwargs: Unpack[_LabelKwargs],
) -> dict[Hashable, ColorOut]: ...

@overload
def create_label_palette_from_columns(
    data: object,
    *,
    positions: Sequence[Hashable],
    label: Hashable,
    format: Literal["hex"] = "hex",
    **kwargs: Unpack[_LabelKwargs],
) -> dict[Hashable, str]: ...
@overload
def create_label_palette_from_columns(
    data: object,
    *,
    positions: Sequence[Hashable],
    label: Hashable,
    format: Literal["rgb"],
    **kwargs: Unpack[_LabelKwargs],
) -> dict[Hashable, Rgb8]: ...
@overload
def create_label_palette_from_columns(
    data: object,
    *,
    positions: Sequence[Hashable],
    label: Hashable,
    format: Literal["rgb01"],
    **kwargs: Unpack[_LabelKwargs],
) -> dict[Hashable, Rgb01]: ...
@overload
def create_label_palette_from_columns(
    data: object,
    *,
    positions: Sequence[Hashable],
    label: Hashable,
    format: ColorFormat,
    **kwargs: Unpack[_LabelKwargs],
) -> dict[Hashable, ColorOut]: ...

def view_palette(
    palette: Sequence[ColorLike], *, width: int = 1246, height: int = 154
) -> PaletteView: ...
def palette_svg(
    palette: Sequence[ColorLike], *, width: int = 1246, height: int = 154
) -> str: ...
def palette_png(
    palette: Sequence[ColorLike], *, width: int = 1246, height: int = 154
) -> bytes: ...
def save_palette(
    palette: Sequence[ColorLike],
    path: str | PathLike[str],
    *,
    width: int = 1246,
    height: int = 154,
) -> Path: ...

__all__: list[str]
