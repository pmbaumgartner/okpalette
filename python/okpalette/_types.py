"""Public type aliases for okpalette."""

from typing import Literal, Sequence, Tuple, Union

Rgb8 = Tuple[int, int, int]
Rgb01 = Tuple[float, float, float]
ColorLike = Union[str, Rgb8, Rgb01]
BackgroundLike = Union[ColorLike, Sequence[ColorLike]]
BackgroundContrast = Literal["normal", "high"]
ColorFormat = Literal["hex", "rgb", "rgb01"]
GridSize = Union[Literal["coarse", "medium", "fine"], int]

__all__ = [
    "BackgroundContrast",
    "BackgroundLike",
    "ColorFormat",
    "ColorLike",
    "GridSize",
    "Rgb01",
    "Rgb8",
]
