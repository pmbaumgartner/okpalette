from __future__ import annotations

import math
from typing import Any, cast

import pytest

from conftest import raw_palette
from okpalette import create_palette, extend_palette


@pytest.mark.parametrize("grid_size", ["coarse", "medium", "fine", 1, 16, 255])
def test_grid_size_accepts_names_and_integer_steps(grid_size: object) -> None:
    palette = raw_palette(1, grid_size=cast(Any, grid_size))

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
        raw_palette(2, lightness=(0.99, 1.0))


def test_avoid_and_background_colors_are_not_returned() -> None:
    palette = raw_palette(
        6,
        avoid_colors=["#000000"],
        background="#ffffff",
        background_contrast="normal",
    )

    assert len(palette) == 6
    assert "#000000" not in palette
    assert "#ffffff" not in palette


def test_background_accepts_single_color_or_sequence() -> None:
    from_tuple = raw_palette(
        7,
        background=(255, 255, 255),
        background_contrast="normal",
    )
    from_sequence = raw_palette(
        6,
        background=["#ffffff", (0, 0, 255)],
        background_contrast="normal",
    )

    assert "#ffffff" not in from_tuple
    assert "#ffffff" not in from_sequence
    assert "#0000ff" not in from_sequence


def test_background_contrast_rejects_invalid_preset() -> None:
    with pytest.raises(ValueError, match="background_contrast"):
        create_palette(1, background_contrast=cast(Any, "maximum"))


@pytest.mark.parametrize("background_contrast", ["normal", "high", "wcag"])
def test_background_contrast_accepts_supported_values(background_contrast: object) -> None:
    palette = raw_palette(
        4,
        background="#ffffff",
        background_contrast=cast(Any, background_contrast),
    )

    assert len(palette) == 4


def test_background_requires_background_contrast() -> None:
    with pytest.raises(ValueError, match="background_contrast must be provided"):
        create_palette(1, background="#ffffff")


def test_background_contrast_requires_background() -> None:
    with pytest.raises(ValueError, match="background must be provided"):
        create_palette(1, background_contrast="normal")


def test_background_contrast_requires_non_empty_background() -> None:
    with pytest.raises(ValueError, match="background must contain at least one color"):
        create_palette(1, background=[], background_contrast="high")


def test_high_and_wcag_background_contrast_are_aliases() -> None:
    high = raw_palette(12, background="#ffffff", background_contrast="high", grid_size=32)
    wcag = raw_palette(12, background="#ffffff", background_contrast="wcag", grid_size=32)

    assert high == wcag


def test_high_background_contrast_filters_against_pale_background() -> None:
    background = "#d1dde4"
    palette = raw_palette(
        12,
        background=background,
        background_contrast="high",
        grid_size=32,
    )

    assert all(_contrast_ratio(color, background) >= 3.0 for color in palette)
    assert _contrast_ratio("#c0e0e0", background) < 3.0


def test_high_background_contrast_checks_multiple_backgrounds() -> None:
    backgrounds = ["#ffffff", "#000000"]
    palette = raw_palette(
        4,
        background=backgrounds,
        background_contrast="high",
        grid_size=32,
    )

    assert all(
        _contrast_ratio(color, background) >= 3.0
        for color in palette
        for background in backgrounds
    )


def test_high_background_contrast_rejects_failing_seed_color() -> None:
    with pytest.raises(ValueError, match=r"seed_colors color #ffffff.*background #ffffff"):
        raw_palette(
            1,
            seed_colors=["#ffffff"],
            background="#ffffff",
            background_contrast="high",
        )


def test_high_background_contrast_rejects_failing_existing_color() -> None:
    with pytest.raises(ValueError, match=r"seed_colors color #ffffff.*background #ffffff"):
        extend_palette(
            ["#ffffff"],
            2,
            background="#ffffff",
            background_contrast="high",
            lightness=None,
            chroma=None,
        )


def _contrast_ratio(left: str, right: str) -> float:
    left_luminance = _relative_luminance(left)
    right_luminance = _relative_luminance(right)
    light = max(left_luminance, right_luminance)
    dark = min(left_luminance, right_luminance)
    return (light + 0.05) / (dark + 0.05)


def _relative_luminance(color: str) -> float:
    red = _srgb_to_linear(int(color[1:3], 16))
    green = _srgb_to_linear(int(color[3:5], 16))
    blue = _srgb_to_linear(int(color[5:7], 16))
    return 0.2126 * red + 0.7152 * green + 0.0722 * blue


def _srgb_to_linear(channel: int) -> float:
    value = channel / 255
    if value <= 0.04045:
        return value / 12.92
    return ((value + 0.055) / 1.055) ** 2.4
