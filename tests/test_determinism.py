from __future__ import annotations

import re
from typing import Any, cast

from hypothesis import given, settings
from hypothesis import strategies as st

from okpalette import ColorLike, GridSize, create_palette, extend_palette

HEX_COLOR = re.compile(r"#[0-9a-f]{6}\Z")
HEX_DIGITS = "0123456789abcdef"


def assert_hex_palette(palette: list[str], palette_size: int) -> None:
    assert len(palette) == palette_size
    assert all(isinstance(color, str) and HEX_COLOR.fullmatch(color) for color in palette)
    assert len(set(palette)) == len(palette)


def test_repeated_default_calls_produce_identical_unique_output() -> None:
    first = cast(list[str], create_palette(24))
    second = cast(list[str], create_palette(24))

    assert first == second
    assert_hex_palette(first, 24)


@given(
    palette_size=st.integers(min_value=1, max_value=20),
    grid_size=st.sampled_from(["coarse", 64, 32]),
    background=st.sampled_from([None, "#ffffff"]),
    lightness=st.sampled_from([None, (0.20, 0.90), (0.0, 1.0)]),
    chroma=st.sampled_from([None, (0.04, None), (0.0, None)]),
)
@settings(max_examples=30, deadline=None)
def test_small_valid_palettes_are_unique_hex_and_deterministic(
    palette_size: int,
    grid_size: GridSize,
    background: str | None,
    lightness: tuple[float, float] | None,
    chroma: tuple[float | None, float | None] | None,
) -> None:
    first = cast(
        list[str],
        create_palette(
            palette_size,
            grid_size=grid_size,
            background=background,
            lightness=lightness,
            chroma=chroma,
        ),
    )
    second = create_palette(
        palette_size,
        grid_size=grid_size,
        background=background,
        lightness=lightness,
        chroma=chroma,
    )

    assert first == second
    assert_hex_palette(first, palette_size)


@st.composite
def valid_color_examples(draw: st.DrawFn) -> tuple[ColorLike, str]:
    kind = draw(st.sampled_from(["full_hex", "short_hex", "rgb8", "rgb01"]))

    if kind == "full_hex":
        red, green, blue = draw(
            st.tuples(
                st.integers(min_value=0, max_value=255),
                st.integers(min_value=0, max_value=255),
                st.integers(min_value=0, max_value=255),
            )
        )
        hex_digits = f"{red:02x}{green:02x}{blue:02x}"
        if draw(st.booleans()):
            hex_digits = hex_digits.upper()
        prefix = "#" if draw(st.booleans()) else ""
        return f"{prefix}{hex_digits}", f"#{red:02x}{green:02x}{blue:02x}"

    if kind == "short_hex":
        red, green, blue = draw(
            st.tuples(
                st.integers(min_value=0, max_value=15),
                st.integers(min_value=0, max_value=15),
                st.integers(min_value=0, max_value=15),
            )
        )
        hex_digits = f"{HEX_DIGITS[red]}{HEX_DIGITS[green]}{HEX_DIGITS[blue]}"
        if draw(st.booleans()):
            hex_digits = hex_digits.upper()
        prefix = "#" if draw(st.booleans()) else ""
        return f"{prefix}{hex_digits}", f"#{red * 17:02x}{green * 17:02x}{blue * 17:02x}"

    if kind == "rgb8":
        red, green, blue = draw(
            st.tuples(
                st.integers(min_value=0, max_value=255),
                st.integers(min_value=0, max_value=255),
                st.integers(min_value=0, max_value=255),
            ).filter(lambda rgb: any(component not in (0, 1) for component in rgb))
        )
        return (red, green, blue), f"#{red:02x}{green:02x}{blue:02x}"

    red, green, blue = draw(
        st.tuples(
            st.integers(min_value=0, max_value=255),
            st.integers(min_value=0, max_value=255),
            st.integers(min_value=0, max_value=255),
        )
    )
    return (red / 255.0, green / 255.0, blue / 255.0), f"#{red:02x}{green:02x}{blue:02x}"


@given(color_case=valid_color_examples())
@settings(max_examples=40, deadline=None)
def test_extend_palette_normalizes_generated_valid_existing_color(
    color_case: tuple[ColorLike, str],
) -> None:
    color, expected = color_case

    assert extend_palette([cast(Any, color)], 1) == [expected]
