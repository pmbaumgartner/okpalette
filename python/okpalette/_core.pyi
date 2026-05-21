from typing import List, Optional, Tuple

def generate_palette_rs(
    palette_size: int,
    seed_colors: Optional[List[str]] = ...,
    avoid_colors: Optional[List[str]] = ...,
    backgrounds: Optional[List[str]] = ...,
    background_contrast: Optional[str] = ...,
    lightness: Optional[Tuple[float, float]] = ...,
    chroma: Optional[Tuple[Optional[float], Optional[float]]] = ...,
    hue: Optional[Tuple[float, float]] = ...,
    grid_step: int = ...,
    lightness_weight: float = ...,
    chroma_weight: float = ...,
    colorblind_mode: Optional[str] = ...,
) -> List[str]: ...
def generate_label_palette_rs(
    coordinates: List[float],
    dimension: int,
    label_ids: List[int],
    label_count: int,
    fixed_colors: List[Optional[str]],
    seed_colors: Optional[List[str]] = ...,
    avoid_colors: Optional[List[str]] = ...,
    backgrounds: Optional[List[str]] = ...,
    background_contrast: Optional[str] = ...,
    lightness: Optional[Tuple[float, float]] = ...,
    chroma: Optional[Tuple[Optional[float], Optional[float]]] = ...,
    hue: Optional[Tuple[float, float]] = ...,
    grid_step: int = ...,
    lightness_weight: float = ...,
    chroma_weight: float = ...,
    colorblind_mode: Optional[str] = ...,
    neighbors: int = ...,
    max_points: Optional[int] = ...,
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
