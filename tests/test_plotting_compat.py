from __future__ import annotations

import tomllib
from pathlib import Path
from typing import cast

import pytest

from conftest import assert_hex_palette
from okpalette import create_palette

REPO_ROOT = Path(__file__).resolve().parents[1]


def test_core_package_declares_no_runtime_dependencies() -> None:
    pyproject = tomllib.loads((REPO_ROOT / "pyproject.toml").read_text(encoding="utf-8"))

    assert pyproject["project"]["dependencies"] == []


def test_matplotlib_accepts_hex_palette_as_cycle_and_colormap() -> None:
    pytest.importorskip("matplotlib")

    from cycler import cycler
    from matplotlib.colors import ListedColormap

    colors = cast(list[str], create_palette(4, grid_size="coarse"))

    color_cycle = cycler(color=colors)
    colormap = ListedColormap(colors, name="okpalette")

    assert list(color_cycle) == [{"color": color} for color in colors]
    assert cast(list[str], colormap.colors) == colors


def test_altair_and_plotly_examples_use_plain_hex_sequences_and_maps() -> None:
    categories = ["control", "treated", "outlier"]
    colors = cast(list[str], create_palette(len(categories), grid_size="coarse"))

    altair_domain = categories
    altair_range = colors
    plotly_discrete_sequence = colors
    plotly_discrete_map = dict(zip(categories, colors, strict=True))

    assert_hex_palette(altair_range, len(categories))
    assert altair_domain == categories
    assert plotly_discrete_sequence == colors
    assert plotly_discrete_map == {
        "control": colors[0],
        "treated": colors[1],
        "outlier": colors[2],
    }
