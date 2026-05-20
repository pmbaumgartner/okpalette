from __future__ import annotations

import math
from typing import Any, cast

import pytest

from okpalette import create_palette


@pytest.mark.parametrize("grid_size", ["coarse", "medium", "fine", 1, 16, 255])
def test_grid_size_accepts_names_and_integer_steps(grid_size: object) -> None:
    palette = create_palette(
        1,
        grid_size=cast(Any, grid_size),
        lightness=None,
        chroma=None,
        background=None,
    )

    assert len(palette) == 1


@pytest.mark.parametrize("grid_size", ["tiny", 0, 256, True, 1.5])
def test_grid_size_rejects_invalid_values(grid_size: object) -> None:
    with pytest.raises(ValueError, match="grid_size"):
        create_palette(1, grid_size=cast(Any, grid_size))


@pytest.mark.parametrize(
    "lightness",
    [
        (-0.1, 0.5),
        (0.2, 1.1),
        (0.9, 0.1),
        (math.inf, 1.0),
        (0.2,),
        [0.2, 0.8],
    ],
)
def test_lightness_rejects_invalid_bounds(lightness: object) -> None:
    with pytest.raises(ValueError, match="lightness"):
        create_palette(1, lightness=cast(Any, lightness))


@pytest.mark.parametrize(
    "chroma",
    [
        (-0.1, None),
        (None, -0.1),
        (0.3, 0.1),
        (math.nan, None),
        (0.1,),
        [0.1, None],
    ],
)
def test_chroma_rejects_invalid_bounds(chroma: object) -> None:
    with pytest.raises(ValueError, match="chroma"):
        create_palette(1, chroma=cast(Any, chroma))


@pytest.mark.parametrize(
    "hue",
    [
        (-1.0, 10.0),
        (10.0, 361.0),
        (math.inf, 10.0),
        (1.0,),
        [1.0, 2.0],
    ],
)
def test_hue_rejects_invalid_bounds(hue: object) -> None:
    with pytest.raises(ValueError, match="hue"):
        create_palette(1, hue=cast(Any, hue))


def test_hue_accepts_wrapping_ranges() -> None:
    palette = create_palette(3, hue=(330.0, 100.0), grid_size="coarse")

    assert len(palette) == 3


@pytest.mark.parametrize(
    "call",
    [
        lambda: create_palette(1, lightness_weight=-1.0),
        lambda: create_palette(1, chroma_weight=-1.0),
        lambda: create_palette(1, lightness_weight=0.0, chroma_weight=0.0),
        lambda: create_palette(1, lightness_weight=math.nan),
    ],
)
def test_distance_weights_reject_invalid_values(call: object) -> None:
    with pytest.raises(ValueError, match="weight"):
        cast(Any, call)()


def test_impossible_constraints_raise_value_error() -> None:
    with pytest.raises(ValueError, match="candidate colors remain"):
        create_palette(
            2,
            grid_size=255,
            lightness=(0.99, 1.0),
            chroma=None,
            background=None,
        )


def test_avoid_and_background_colors_are_not_returned() -> None:
    palette = create_palette(
        6,
        avoid_colors=["#000000"],
        background="#ffffff",
        grid_size=255,
        lightness=None,
        chroma=None,
    )

    assert len(palette) == 6
    assert "#000000" not in palette
    assert "#ffffff" not in palette
