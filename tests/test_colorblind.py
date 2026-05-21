from __future__ import annotations

from typing import Any, cast

import pytest

from okpalette import (
    create_label_palette,
    create_label_palette_from_columns,
    create_palette,
    extend_palette,
)

COLORBLIND_10 = {
    "protan": [
        "#000058",
        "#90ff38",
        "#a080ff",
        "#586810",
        "#88e0ff",
        "#e0a060",
        "#4000c8",
        "#801000",
        "#e83890",
        "#304068",
    ],
    "deutan": [
        "#000058",
        "#f8e000",
        "#a078ff",
        "#a01800",
        "#58f0ff",
        "#0000d8",
        "#48b850",
        "#500000",
        "#a02898",
        "#80b0c8",
    ],
    "tritan": [
        "#000058",
        "#58f8ff",
        "#e80000",
        "#5858ff",
        "#f8a0ff",
        "#800078",
        "#c008ff",
        "#80c060",
        "#105800",
        "#480000",
    ],
    "all": [
        "#000058",
        "#48ffd0",
        "#b040ff",
        "#106800",
        "#00b800",
        "#d80090",
        "#c8a0ff",
        "#500000",
        "#c8f000",
        "#f090a0",
    ],
}


@pytest.mark.parametrize("mode", ["protan", "deutan", "tritan", "all"])
def test_colorblind_mode_palette_snapshots(mode: str) -> None:
    assert create_palette(10, colorblind_mode=cast(Any, mode)) == COLORBLIND_10[mode]


def test_all_colorblind_mode_scores_ordinary_palette_generation() -> None:
    palette = create_palette(
        4,
        grid_size=255,
        lightness=None,
        chroma=None,
        colorblind_mode="all",
    )

    assert palette == ["#000000", "#ffffff", "#0000ff", "#ff0000"]


def test_colorblind_mode_uses_seed_colors_as_anchors() -> None:
    palette = create_palette(
        4,
        seed_colors=["#ff0000"],
        grid_size=255,
        lightness=None,
        chroma=None,
        colorblind_mode="all",
    )

    assert palette == ["#000000", "#ffffff", "#0000ff", "#00ff00"]
    assert "#ff0000" not in palette


def test_extend_palette_accepts_colorblind_mode() -> None:
    palette = extend_palette(
        ["#ff0000", "#00ff00"],
        5,
        grid_size=255,
        lightness=None,
        chroma=None,
        colorblind_mode="all",
    )

    assert palette == ["#ff0000", "#00ff00", "#000000", "#0000ff", "#ffffff"]


@pytest.mark.parametrize("background_contrast", ["high", "wcag"])
def test_colorblind_mode_composes_with_wcag_background_contrast(
    background_contrast: str,
) -> None:
    background = "#ffffff"
    palette = cast(
        list[str],
        create_palette(
            8,
            background=background,
            background_contrast=cast(Any, background_contrast),
            colorblind_mode="all",
            grid_size=32,
        ),
    )

    assert all(_contrast_ratio(color, background) >= 3.0 for color in palette)


def test_label_palette_colorblind_mode_preserves_fixed_colors() -> None:
    positions = [(0.0, 0.0), (10.0, 0.0), (0.1, 0.0), (10.1, 0.0)]
    labels = ["a", "b", "c", "d"]

    palette = create_label_palette(
        positions,
        labels,
        fixed_colors={"a": "#ff0000"},
        grid_size=255,
        lightness=None,
        chroma=None,
        colorblind_mode="all",
    )

    assert palette == {
        "a": "#ff0000",
        "b": "#000000",
        "c": "#0000ff",
        "d": "#ffffff",
    }


def test_label_palette_from_columns_accepts_colorblind_mode() -> None:
    data = {
        "x": [0.0, 10.0, 0.1, 10.1],
        "y": [0.0, 0.0, 0.0, 0.0],
        "label": ["a", "b", "c", "d"],
    }

    palette = create_label_palette_from_columns(
        data,
        positions=["x", "y"],
        label="label",
        grid_size=255,
        lightness=None,
        chroma=None,
        colorblind_mode="all",
    )

    assert palette == {
        "a": "#ffffff",
        "b": "#0000ff",
        "c": "#000000",
        "d": "#ff0000",
    }


@pytest.mark.parametrize(
    "call",
    [
        lambda: create_palette(1, colorblind_mode=cast(Any, "protanopia")),
        lambda: extend_palette(["#ff0000"], 2, colorblind_mode=cast(Any, "safe")),
        lambda: create_label_palette([0.0], ["a"], colorblind_mode=cast(Any, "none")),
    ],
)
def test_colorblind_mode_rejects_invalid_values(call: object) -> None:
    with pytest.raises(ValueError, match="colorblind_mode"):
        cast(Any, call)()


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
