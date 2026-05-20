from __future__ import annotations

import os
from pathlib import Path
from typing import Callable

import pytest

from glasbey_rs import create_palette, extend_palette, save_palette
from glasbey_rs._types import ColorLike

pytestmark = pytest.mark.plot_smoke


def _require_plot_smoke() -> None:
    if os.environ.get("GLASBEY_RS_PLOT_SMOKE") != "1":
        pytest.skip("set GLASBEY_RS_PLOT_SMOKE=1 to write palette preview artifacts")


def _output_dir() -> Path:
    path = Path(os.environ.get("GLASBEY_RS_PLOT_SMOKE_DIR", "output/plot_smoke"))
    path.mkdir(parents=True, exist_ok=True)
    return path


def _save_review_images(
    name: str,
    palette: list[ColorLike],
    *,
    width: int,
    height: int,
) -> tuple[Path, Path]:
    output_dir = _output_dir()
    svg_path = save_palette(palette, output_dir / f"{name}.svg", width=width, height=height)
    png_path = save_palette(palette, output_dir / f"{name}.png", width=width, height=height)

    assert svg_path.stat().st_size > 0
    assert png_path.stat().st_size > 0
    return svg_path, png_path


@pytest.mark.parametrize(
    ("name", "build_palette", "width", "height"),
    [
        (
            "default-24",
            lambda: create_palette(24),
            1246,
            154,
        ),
        (
            "brand-extended-16",
            lambda: extend_palette(["#0057b8", "#ffd700"], 16),
            1246,
            154,
        ),
        (
            "warm-16",
            lambda: create_palette(16, hue=(330, 100)),
            998,
            141,
        ),
        (
            "cool-16",
            lambda: create_palette(16, hue=(150, 280)),
            998,
            141,
        ),
    ],
)
def test_plot_review_images_are_written(
    name: str,
    build_palette: Callable[[], list[ColorLike]],
    width: int,
    height: int,
) -> None:
    _require_plot_smoke()

    svg_path, png_path = _save_review_images(name, build_palette(), width=width, height=height)

    assert svg_path.is_file()
    assert png_path.is_file()
