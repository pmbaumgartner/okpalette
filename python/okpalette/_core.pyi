from typing import List, Optional, Self, Tuple, final

from ._types import ColorblindMode

@final
class _PaletteGenerator:
    def __new__(
        cls,
        seed_colors: List[str],
        avoid_colors: List[str],
        backgrounds: Optional[List[str]],
        background_contrast: Optional[str],
        lightness: Optional[Tuple[float, float]],
        chroma: Optional[Tuple[Optional[float], Optional[float]]],
        hue: Optional[Tuple[float, float]],
        grid_step: int,
        lightness_weight: float,
        chroma_weight: float,
        colorblind_mode: Optional[ColorblindMode],
    ) -> Self: ...
    def generate(self, palette_size: int) -> List[str]: ...
    def extend(self, colors: List[str], target_size: int) -> List[str]: ...
    def generate_extension(self, colors: List[str], count: int) -> List[str]: ...
    def generate_for_labels(
        self,
        coordinates: List[float],
        dimension: int,
        label_ids: List[int],
        label_count: int,
        fixed_colors: List[Optional[str]],
        neighbors: int,
        max_points: Optional[int],
    ) -> List[str]: ...

def palette_svg_rs(
    colors: List[str],
    width: int = ...,
    height: int = ...,
) -> str: ...
def palette_png_rs(
    colors: List[str],
    width: int = ...,
    height: int = ...,
) -> bytes: ...

__all__ = ["_PaletteGenerator", "palette_svg_rs", "palette_png_rs"]
