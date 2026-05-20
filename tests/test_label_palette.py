from __future__ import annotations

import math
from typing import Any, Hashable, cast

import pytest

from okpalette import create_label_palette, create_label_palette_from_columns, create_palette


def test_label_palette_returns_first_seen_label_mapping() -> None:
    labels: list[Hashable] = ["beta", "alpha", "beta", ("tuple", 1)]

    palette = create_label_palette([0.0, 1.0, 2.0, 3.0], labels, grid_size="coarse")

    assert list(palette) == ["beta", "alpha", ("tuple", 1)]
    assert len(set(palette.values())) == 3


def test_label_palette_supports_integer_tuple_and_fixed_labels() -> None:
    labels: list[Hashable] = [10, ("region", 1), 10, "other"]

    palette = create_label_palette(
        [(0.0, 0.0), (0.1, 0.0), (1.0, 0.0), (1.1, 0.0)],
        labels,
        fixed_colors={("region", 1): "#F00"},
        grid_size="coarse",
    )

    assert list(palette) == [10, ("region", 1), "other"]
    assert palette[("region", 1)] == "#ff0000"


def test_label_palette_formats_match_create_palette_formats() -> None:
    rgb = create_label_palette([0.0, 1.0], ["a", "b"], grid_size="coarse", format="rgb")
    rgb01 = create_label_palette([0.0, 1.0], ["a", "b"], grid_size="coarse", format="rgb01")

    assert all(
        isinstance(color, tuple)
        and len(color) == 3
        and all(type(component) is int for component in color)
        for color in rgb.values()
    )
    assert all(
        isinstance(color, tuple)
        and len(color) == 3
        and all(type(component) is float for component in color)
        for color in rgb01.values()
    )


def test_label_palette_is_deterministic() -> None:
    positions = [(0.0, 0.0), (0.2, 0.0), (5.0, 0.0), (5.2, 0.0), (0.1, 0.0)]
    labels = ["a", "b", "c", "d", "a"]

    first = create_label_palette(positions, labels, grid_size="coarse")
    second = create_label_palette(positions, labels, grid_size="coarse")

    assert first == second


def test_empty_label_palette_returns_empty_dict() -> None:
    assert create_label_palette([], []) == {}


@pytest.mark.parametrize(
    "call",
    [
        lambda: create_label_palette([0.0], ["a", "b"]),
        lambda: create_label_palette([(0.0, 0.0), (1.0,)], ["a", "b"]),
        lambda: create_label_palette([math.nan], ["a"]),
        lambda: create_label_palette([0.0], [[1]]),
        lambda: create_label_palette([0.0], ["a"], neighbors=0),
        lambda: create_label_palette([0.0], ["a"], neighbors=cast(Any, True)),
        lambda: create_label_palette([0.0], ["a"], max_points=0),
        lambda: create_label_palette([0.0], ["a"], max_points=cast(Any, True)),
        lambda: create_label_palette([0.0], ["a"], fixed_colors={"missing": "#fff"}),
    ],
)
def test_label_palette_invalid_inputs_raise_value_error(call: object) -> None:
    with pytest.raises(ValueError):
        cast(Any, call)()


def test_label_palette_rejects_too_small_sampling_budget_for_label_count() -> None:
    with pytest.raises(ValueError, match="max_points"):
        create_label_palette([0.0, 1.0, 2.0], ["a", "b", "c"], max_points=2)


class ColumnWithToNumpy:
    def __init__(self, values: list[object]) -> None:
        self._values = values

    def to_numpy(self) -> tuple[object, ...]:
        return tuple(self._values)


class ColumnWithToList:
    def __init__(self, values: list[object]) -> None:
        self._values = values

    def to_list(self) -> list[object]:
        return self._values


class FakeFrame:
    def __init__(self) -> None:
        self.columns = {
            "x": ColumnWithToNumpy([0.0, 0.1, 5.0, 5.1]),
            "y": ColumnWithToList([0.0, 0.0, 0.0, 0.0]),
            "label": ["left", "right", "left", "other"],
        }

    def __getitem__(self, key: Hashable) -> object:
        return self.columns[cast(str, key)]


def test_label_palette_from_columns_uses_dataframe_duck_typing() -> None:
    palette = create_label_palette_from_columns(
        FakeFrame(),
        positions=["x", "y"],
        label="label",
        grid_size="coarse",
    )

    assert list(palette) == ["left", "right", "other"]


def test_label_palette_from_columns_rejects_mismatched_columns() -> None:
    frame = {"x": [0.0], "label": ["a", "b"]}

    with pytest.raises(ValueError, match="length"):
        create_label_palette_from_columns(frame, positions=["x"], label="label")


def test_position_aware_fixture_beats_first_seen_assignment() -> None:
    positions = [(0.0, 0.0), (10.0, 0.0), (0.1, 0.0), (10.1, 0.0)]
    labels = ["a", "b", "c", "d"]
    position_aware = cast(
        dict[Hashable, str],
        create_label_palette(
            positions,
            labels,
            grid_size=255,
            lightness=None,
            chroma=None,
            background=None,
        ),
    )
    first_seen = cast(
        dict[Hashable, str],
        dict(
            zip(
                labels,
                create_palette(4, grid_size=255, lightness=None, chroma=None, background=None),
            )
        ),
    )

    assert set(position_aware.values()) == set(first_seen.values())
    assert _fixture_quality(position_aware) > _fixture_quality(first_seen)


def test_label_palette_preserves_palette_set_with_background_contrast() -> None:
    positions = [(0.0, 0.0), (10.0, 0.0), (0.1, 0.0), (10.1, 0.0)]
    labels = ["a", "b", "c", "d"]
    position_aware = cast(
        dict[Hashable, str],
        create_label_palette(
            positions,
            labels,
            grid_size=255,
            lightness=None,
            chroma=None,
            background=["#ffffff", "#000000"],
            background_contrast="high",
        ),
    )
    first_seen = cast(
        list[str],
        create_palette(
            4,
            grid_size=255,
            lightness=None,
            chroma=None,
            background=["#ffffff", "#000000"],
            background_contrast="high",
        ),
    )

    assert set(position_aware.values()) == set(first_seen)
    assert "#ffffff" not in position_aware.values()
    assert "#000000" not in position_aware.values()


def _fixture_quality(mapping: dict[Hashable, str]) -> float:
    return _oklab_distance_squared(mapping["a"], mapping["c"]) + _oklab_distance_squared(
        mapping["b"], mapping["d"]
    )


def _oklab_distance_squared(left: str, right: str) -> float:
    left_lab = _hex_to_oklab(left)
    right_lab = _hex_to_oklab(right)
    return sum(
        (left_value - right_value) ** 2 for left_value, right_value in zip(left_lab, right_lab)
    )


def _hex_to_oklab(color: str) -> tuple[float, float, float]:
    red = _srgb_to_linear(int(color[1:3], 16))
    green = _srgb_to_linear(int(color[3:5], 16))
    blue = _srgb_to_linear(int(color[5:7], 16))

    linear_l = 0.412_221_46 * red + 0.536_332_55 * green + 0.051_445_995 * blue
    linear_m = 0.211_903_5 * red + 0.680_699_5 * green + 0.107_396_96 * blue
    linear_s = 0.088_302_46 * red + 0.281_718_85 * green + 0.629_978_7 * blue

    l_root = linear_l ** (1.0 / 3.0)
    m_root = linear_m ** (1.0 / 3.0)
    s_root = linear_s ** (1.0 / 3.0)

    return (
        0.210_454_26 * l_root + 0.793_617_8 * m_root - 0.004_072_047 * s_root,
        1.977_998_5 * l_root - 2.428_592_2 * m_root + 0.450_593_7 * s_root,
        0.025_904_037 * l_root + 0.782_771_77 * m_root - 0.808_675_77 * s_root,
    )


def _srgb_to_linear(channel: int) -> float:
    value = channel / 255.0
    if value <= 0.04045:
        return value / 12.92
    return ((value + 0.055) / 1.055) ** 2.4
