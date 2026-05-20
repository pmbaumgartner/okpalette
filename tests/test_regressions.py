from __future__ import annotations

from glasbey_rs import create_palette

DEFAULT_10 = [
    "#080050",
    "#e00800",
    "#1078ff",
    "#00b800",
    "#ff00ff",
    "#405000",
    "#800090",
    "#ffa800",
    "#30d0f0",
    "#90ff00",
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
    "#300018",
    "#e000d0",
    "#887800",
    "#ff9090",
    "#980040",
    "#f8d800",
    "#484010",
    "#f01800",
    "#a06898",
    "#c0a840",
]

COOL_HUE_10 = [
    "#080050",
    "#209048",
    "#6048ff",
    "#00f878",
    "#48c0ff",
    "#004828",
    "#0000c0",
    "#586088",
    "#78c088",
    "#60f8f0",
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
