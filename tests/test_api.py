from __future__ import annotations

import sys
from importlib import import_module
from typing import Any, assert_type, cast

import pytest

from conftest import raw_palette
import okpalette
from okpalette import palette_png, palette_svg, save_palette, view_palette
from okpalette import create_label_palette, create_label_palette_from_columns
from okpalette import create_palette, extend_palette
from okpalette import _format


def test_public_imports_and_exports() -> None:
    module = import_module("okpalette")

    assert module is okpalette
    assert create_palette is okpalette.create_palette
    assert extend_palette is okpalette.extend_palette
    assert create_label_palette is okpalette.create_label_palette
    assert create_label_palette_from_columns is okpalette.create_label_palette_from_columns
    assert view_palette is okpalette.view_palette
    assert palette_svg is okpalette.palette_svg
    assert palette_png is okpalette.palette_png
    assert save_palette is okpalette.save_palette
    assert set(getattr(okpalette, "__all__")) == {
        "BackgroundContrast",
        "BackgroundLike",
        "ColorblindMode",
        "ColorFormat",
        "ColorLike",
        "GridSize",
        "PaletteView",
        "Rgb01",
        "Rgb8",
        "__version__",
        "create_label_palette",
        "create_label_palette_from_columns",
        "create_palette",
        "extend_palette",
        "palette_png",
        "palette_svg",
        "save_palette",
        "view_palette",
    }


def test_literal_formats_have_precise_return_types() -> None:
    assert_type(create_palette(1), list[str])
    assert_type(create_palette(1, format="rgb"), list[tuple[int, int, int]])
    assert_type(create_palette(1, format="rgb01"), list[tuple[float, float, float]])


@pytest.mark.parametrize("palette_size", [1, 2, 10])
def test_create_palette_returns_requested_size(palette_size: int) -> None:
    palette = create_palette(palette_size, grid_size="coarse")

    assert len(palette) == palette_size


def test_zero_sized_palette_is_empty() -> None:
    assert create_palette(0) == []
    assert extend_palette([], 0) == []
    assert extend_palette([], 0, include_existing=False) == []


@pytest.mark.parametrize(
    "call",
    [
        lambda: create_palette(-1),
        lambda: create_palette(cast(Any, True)),
        lambda: extend_palette([], -1),
        lambda: extend_palette([], cast(Any, True)),
        lambda: extend_palette(["#ff0000", "#00ff00"], 1, include_existing=True),
        lambda: extend_palette(["#ff0000", "#00ff00"], 1, include_existing=False),
    ],
)
def test_invalid_size_inputs_raise_value_error(call: object) -> None:
    with pytest.raises(ValueError):
        cast(Any, call)()


def test_extend_palette_includes_existing_colors_first() -> None:
    palette = extend_palette(["#F00", "0F0"], 5, include_existing=True, grid_size="coarse")

    assert len(palette) == 5
    assert palette[:2] == ["#ff0000", "#00ff00"]
    assert len(set(palette)) == len(palette)


@pytest.mark.parametrize("format", ["hex", "rgb", "rgb01"])
def test_extend_palette_can_return_only_generated_colors(format: Any) -> None:
    existing = ["#F00", "0F0"]
    full = extend_palette(
        existing, 3, seed_colors=["#00f"], grid_size="coarse", format=format
    )
    palette = extend_palette(
        existing,
        3,
        include_existing=False,
        seed_colors=["#00f"],
        grid_size="coarse",
        format=format,
    )

    assert len(palette) == 1
    assert palette == full[2:]
    assert palette[0] not in full[:2]


@pytest.mark.parametrize("include_existing", [True, False])
def test_extend_palette_at_existing_size(include_existing: bool) -> None:
    palette = extend_palette(["#F00", "0F0"], 2, include_existing=include_existing)

    assert palette == (["#ff0000", "#00ff00"] if include_existing else [])


def test_extend_palette_requires_boolean_include_existing() -> None:
    with pytest.raises(ValueError, match="include_existing must be a boolean"):
        extend_palette(["#f00"], 2, include_existing=cast(Any, 1))


def test_rgb8_black_and_near_black_are_accepted() -> None:
    assert extend_palette([(0, 0, 0), (1, 1, 1)], 2) == ["#000000", "#010101"]


@pytest.mark.parametrize("include_existing", [True, False])
def test_extend_palette_accepts_extra_seed_colors_without_returning_them(
    include_existing: bool,
) -> None:
    palette = extend_palette(
        ["#ff0000"],
        3,
        include_existing=include_existing,
        seed_colors=["#00ff00"],
        grid_size="coarse",
    )

    assert len(palette) == (3 if include_existing else 2)
    if include_existing:
        assert palette[0] == "#ff0000"
    else:
        assert "#ff0000" not in palette
    assert "#00ff00" not in palette


def test_default_background_is_unconstrained_when_white_is_on_grid() -> None:
    palette = create_palette(7, grid_size=255, lightness=None, chroma=None)

    assert len(palette) == 7
    assert "#ffffff" in palette


def test_background_can_be_disabled() -> None:
    palette = raw_palette(8)

    assert len(palette) == 8
    assert "#ffffff" in palette


def test_native_extension_missing_error_is_actionable(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setitem(sys.modules, "okpalette._core", None)

    with pytest.raises(ImportError, match="native extension is unavailable") as error:
        _format.load_palette_generator_rs()

    assert isinstance(error.value.__cause__, ImportError)
