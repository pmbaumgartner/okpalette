from __future__ import annotations

import re
import struct
from collections.abc import Hashable, Iterable, Sequence
from typing import Any, TypeVar, cast

import pytest

from okpalette import create_label_palette, create_palette

HEX_COLOR_RE = re.compile(r"#[0-9a-f]{6}\Z")
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"

LabelT = TypeVar("LabelT", bound=Hashable)


def assert_hex_palette(
    palette: Iterable[object],
    size: int | None = None,
    *,
    unique: bool = True,
) -> None:
    colors = list(palette)
    if size is not None:
        assert len(colors) == size
    assert all(isinstance(color, str) and HEX_COLOR_RE.fullmatch(color) for color in colors)
    if unique:
        assert len(set(colors)) == len(colors)


def assert_rgb_palette(
    palette: Iterable[object],
    size: int | None = None,
    *,
    normalized: bool = False,
) -> None:
    colors = list(palette)
    if size is not None:
        assert len(colors) == size

    assert all(
        isinstance(color, tuple)
        and len(color) == 3
        and all(_is_rgb_component(component, normalized=normalized) for component in color)
        for color in colors
    )


def _is_rgb_component(component: object, *, normalized: bool) -> bool:
    if normalized:
        if type(component) is not float:
            return False
        value = cast(float, component)
        return 0.0 <= value <= 1.0

    if type(component) is not int:
        return False
    return 0 <= component <= 255


def assert_png_dimensions(png: bytes, width: int, height: int) -> None:
    assert png.startswith(PNG_SIGNATURE)
    assert (
        struct.unpack(">I", png[16:20])[0],
        struct.unpack(">I", png[20:24])[0],
    ) == (width, height)


def raw_palette(size: int, **kwargs: Any) -> list[str]:
    options: dict[str, Any] = {
        "grid_size": 255,
        "lightness": None,
        "chroma": None,
        "background": None,
    }
    options.update(kwargs)
    return cast(list[str], create_palette(size, **options))


def raw_label_palette(
    positions: object,
    labels: Sequence[LabelT],
    **kwargs: Any,
) -> dict[LabelT, str]:
    options: dict[str, Any] = {
        "grid_size": 255,
        "lightness": None,
        "chroma": None,
        "background": None,
    }
    options.update(kwargs)
    return cast(
        dict[LabelT, str],
        create_label_palette(cast(Any, positions), labels, **options),
    )


def first_seen_label_palette(labels: Iterable[LabelT], **palette_kwargs: Any) -> dict[LabelT, str]:
    unique_labels = list(dict.fromkeys(labels))
    return dict(zip(unique_labels, raw_palette(len(unique_labels), **palette_kwargs)))


@pytest.fixture
def separated_label_fixture() -> tuple[list[tuple[float, float]], list[str]]:
    return (
        [(0.0, 0.0), (10.0, 0.0), (0.1, 0.0), (10.1, 0.0)],
        ["a", "b", "c", "d"],
    )
