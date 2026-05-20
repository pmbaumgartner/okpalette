"""Public type aliases for glasbey_rs."""

from typing import Literal, Tuple, Union

Rgb8 = Tuple[int, int, int]
Rgb01 = Tuple[float, float, float]
ColorLike = Union[str, Rgb8, Rgb01]
ColorFormat = Literal["hex", "rgb", "rgb01"]
GridSize = Union[Literal["coarse", "medium", "fine"], int]

__all__ = ["ColorFormat", "ColorLike", "GridSize", "Rgb01", "Rgb8"]
