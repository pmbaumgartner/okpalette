from typing import List, Optional, Tuple

def generate_palette_rs(
    palette_size: int,
    seed_colors: Optional[List[str]] = ...,
    avoid_colors: Optional[List[str]] = ...,
    background: Optional[str] = ...,
    lightness: Optional[Tuple[float, float]] = ...,
    chroma: Optional[Tuple[Optional[float], Optional[float]]] = ...,
    hue: Optional[Tuple[float, float]] = ...,
    grid_step: int = ...,
    lightness_weight: float = ...,
    chroma_weight: float = ...,
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
