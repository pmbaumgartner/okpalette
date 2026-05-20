from __future__ import annotations

import re
from typing import Any, cast

import pytest

from okpalette import create_palette, extend_palette

HEX_COLOR = re.compile(r"#[0-9a-f]{6}\Z")


def test_hex_format_returns_lowercase_hex_strings() -> None:
    palette = create_palette(4, grid_size="coarse", format="hex")

    assert len(palette) == 4
    assert all(isinstance(color, str) and HEX_COLOR.fullmatch(color) for color in palette)


def test_rgb_format_returns_integer_tuples() -> None:
    palette = create_palette(4, grid_size="coarse", format="rgb")

    assert len(palette) == 4
    assert all(
        isinstance(color, tuple)
        and len(color) == 3
        and all(type(component) is int and 0 <= component <= 255 for component in color)
        for color in palette
    )


def test_rgb01_format_returns_normalized_float_tuples() -> None:
    palette = create_palette(4, grid_size="coarse", format="rgb01")

    assert len(palette) == 4
    assert all(
        isinstance(color, tuple)
        and len(color) == 3
        and all(type(component) is float and 0.0 <= component <= 1.0 for component in color)
        for color in palette
    )


@pytest.mark.parametrize(
    ("color", "expected"),
    [
        ("#0fA", "#00ffaa"),
        ("0fA", "#00ffaa"),
        ("#00ffaa", "#00ffaa"),
        ("00ffaa", "#00ffaa"),
        ("Cc33aA", "#cc33aa"),
    ],
)
def test_hex_inputs_accept_short_full_optional_hash_and_case(
    color: str,
    expected: str,
) -> None:
    assert extend_palette([color], 1) == [expected]


@pytest.mark.parametrize(
    ("color", "expected"),
    [
        ((255, 128, 1), "#ff8001"),
        ((2, 1, 0), "#020100"),
        ((0.0, 0.5, 1.0), "#0080ff"),
    ],
)
def test_rgb_tuples_are_normalized(color: object, expected: str) -> None:
    assert extend_palette([cast(Any, color)], 1) == [expected]


@pytest.mark.parametrize("color", [(0, 0, 0), (1, 0, 0), (1, 1, 1)])
def test_ambiguous_integer_rgb_tuples_are_rejected(color: tuple[int, int, int]) -> None:
    with pytest.raises(ValueError, match="ambiguous integer RGB tuple"):
        extend_palette([cast(Any, color)], 1)


@pytest.mark.parametrize(
    "call",
    [
        lambda: create_palette(1, seed_colors=cast(Any, "#fff")),
        lambda: extend_palette(cast(Any, "#fff"), 1),
    ],
)
def test_color_sequences_reject_plain_strings(call: object) -> None:
    with pytest.raises(ValueError, match="sequence of colors, not a string"):
        cast(Any, call)()


def test_unknown_output_format_is_rejected() -> None:
    with pytest.raises(ValueError, match="format must be"):
        create_palette(1, format=cast(Any, "hsl"))
