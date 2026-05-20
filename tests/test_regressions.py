from __future__ import annotations

from okpalette import create_palette

DEFAULT_10 = [
    "#000058",
    "#90ff00",
    "#ff38ff",
    "#886800",
    "#1058ff",
    "#88c8ff",
    "#800078",
    "#ffa038",
    "#f80050",
    "#004000",
]

SEEDED_WHITE_BLACK_10 = [
    "#6008ff",
    "#009000",
    "#ff1060",
    "#601810",
    "#28ff00",
    "#48b0f8",
    "#e8a810",
    "#000078",
    "#e000ff",
    "#286890",
]

WARM_HUE_10 = [
    "#201800",
    "#ffc8f8",
    "#b000a0",
    "#a08800",
    "#f8d800",
    "#ff0020",
    "#882800",
    "#ff60f0",
    "#b078a0",
    "#580050",
]

COOL_HUE_10 = [
    "#000058",
    "#70ff98",
    "#7078ff",
    "#086830",
    "#0800e0",
    "#c0c8f8",
    "#18b090",
    "#404880",
    "#083018",
    "#0080a8",
]


def test_default_palette_snapshot() -> None:
    assert create_palette(10) == DEFAULT_10


def test_seeded_white_black_palette_snapshot() -> None:
    assert (
        create_palette(10, seed_colors=["#ffffff", "#000000"], background=None)
        == SEEDED_WHITE_BLACK_10
    )


def test_warm_hue_palette_snapshot() -> None:
    assert create_palette(10, hue=(330, 100)) == WARM_HUE_10


def test_cool_hue_palette_snapshot() -> None:
    assert create_palette(10, hue=(150, 280)) == COOL_HUE_10
