import re
from typing import Any, cast

import pytest

from glasbey_rs import create_palette, extend_palette

HEX_COLOR = re.compile(r"#[0-9a-f]{6}\Z")


def test_create_palette_import_and_default_deterministic_hex_output():
    first = create_palette(24)
    second = create_palette(24)

    assert first == second
    assert len(first) == 24
    assert all(isinstance(color, str) and HEX_COLOR.fullmatch(color) for color in first)


@pytest.mark.parametrize(
    ("output_format", "expected_type"),
    [
        ("hex", str),
        ("rgb", tuple),
        ("rgb01", tuple),
    ],
)
def test_create_palette_formats(output_format, expected_type):
    palette = create_palette(4, grid_size="coarse", format=output_format)

    assert len(palette) == 4
    assert all(isinstance(color, expected_type) for color in palette)
    if output_format == "rgb":
        assert all(
            len(color) == 3 and all(isinstance(component, int) for component in color)
            for color in palette
        )
    if output_format == "rgb01":
        assert all(
            len(color) == 3 and all(isinstance(component, float) for component in color)
            for color in palette
        )


def test_extend_palette_includes_existing_colors_first():
    palette = extend_palette(["#F00", "0F0"], 5, include_existing=True, grid_size="coarse")

    assert len(palette) == 5
    assert palette[:2] == ["#ff0000", "#00ff00"]


def test_extend_palette_can_return_only_generated_colors():
    palette = extend_palette(["#ff0000", "#00ff00"], 3, include_existing=False, grid_size="coarse")

    assert len(palette) == 3
    assert "#ff0000" not in palette
    assert "#00ff00" not in palette


@pytest.mark.parametrize(
    "call",
    [
        lambda: create_palette(1, format=cast(Any, "hsl")),
        lambda: create_palette(1, grid_size=cast(Any, "tiny")),
        lambda: create_palette(0),
        lambda: extend_palette([], 0),
        lambda: create_palette(1, seed_colors=[(1, 0, 0)]),
        lambda: extend_palette(["#ff0000", "#00ff00"], 1, include_existing=True),
    ],
)
def test_invalid_api_inputs_raise_value_error(call):
    with pytest.raises(ValueError):
        call()
