"""Private label palette input normalization helpers."""

from __future__ import annotations

import math
from collections.abc import Hashable as HashableABC
from collections.abc import Mapping as MappingABC
from typing import Any, Hashable, Optional, Sequence, Union, cast

from ._format import normalize_color
from ._types import ColorLike


def _normalize_labels(
    labels: Sequence[Hashable],
) -> tuple[list[Hashable], list[int], dict[Hashable, int]]:
    if isinstance(labels, (str, bytes)):
        raise ValueError("labels must be a sequence of hashable objects, not a string")

    try:
        label_values = list(labels)
    except TypeError as error:
        raise ValueError("labels must be a sequence of hashable objects") from error

    ordered_labels: list[Hashable] = []
    label_ids: list[int] = []
    label_to_id: dict[Hashable, int] = {}

    for label in label_values:
        if not isinstance(label, HashableABC):
            raise ValueError("labels must be hashable")

        try:
            label_id = label_to_id[label]
        except TypeError as error:
            raise ValueError("labels must be hashable") from error
        except KeyError:
            label_id = len(ordered_labels)
            label_to_id[label] = label_id
            ordered_labels.append(label)

        label_ids.append(label_id)

    return ordered_labels, label_ids, label_to_id


def _normalize_positions(
    positions: Sequence[Union[float, Sequence[float]]],
    expected_length: int,
) -> tuple[list[float], int]:
    if isinstance(positions, (str, bytes)):
        raise ValueError("positions must be a sequence of coordinates, not a string")

    try:
        position_values = list(positions)
    except TypeError as error:
        raise ValueError("positions must be a sequence of coordinates") from error

    if len(position_values) != expected_length:
        raise ValueError("positions and labels must have the same length")

    if not position_values:
        return [], 1

    rows: list[list[float]] = []
    expected_dimension: Optional[int] = None
    expected_kind: Optional[str] = None
    for position in position_values:
        row, kind = _normalize_position(position)
        if expected_kind is None:
            expected_kind = kind
        elif kind != expected_kind:
            raise ValueError("positions must be all scalars or all coordinate rows")

        if expected_dimension is None:
            expected_dimension = len(row)
        elif len(row) != expected_dimension:
            raise ValueError("positions must all have the same dimensionality")

        rows.append(row)

    dimension = expected_dimension or 1
    return [coordinate for row in rows for coordinate in row], dimension


def _normalize_position(position: object) -> tuple[list[float], str]:
    if _is_coordinate_scalar(position):
        return [_as_coordinate(position)], "scalar"

    if isinstance(position, (str, bytes)):
        raise ValueError("positions must contain numeric coordinates")

    try:
        row = list(cast(Any, position))
    except TypeError as error:
        raise ValueError("positions must contain numeric coordinates") from error

    if not 1 <= len(row) <= 3:
        raise ValueError("position coordinate rows must have length 1, 2, or 3")

    return [_as_coordinate(coordinate) for coordinate in row], "row"


def _is_coordinate_scalar(value: object) -> bool:
    if isinstance(value, bool):
        return False

    try:
        float(cast(Any, value))
    except (TypeError, ValueError):
        return False

    return True


def _as_coordinate(value: object) -> float:
    if isinstance(value, bool):
        raise ValueError("coordinates must be finite numbers")

    try:
        coordinate = float(cast(Any, value))
    except (TypeError, ValueError) as error:
        raise ValueError("coordinates must be finite numbers") from error

    if not math.isfinite(coordinate):
        raise ValueError("coordinates must be finite numbers")

    return coordinate


def _normalize_fixed_colors(
    fixed_colors: Optional[MappingABC[Hashable, ColorLike]],
    label_to_id: MappingABC[Hashable, int],
    label_count: int,
) -> list[Optional[str]]:
    fixed_hex: list[Optional[str]] = [None] * label_count
    if fixed_colors is None:
        return fixed_hex

    if not isinstance(fixed_colors, MappingABC):
        raise ValueError("fixed_colors must be a mapping from label to color")

    for label, color in fixed_colors.items():
        if not isinstance(label, HashableABC):
            raise ValueError("fixed_colors labels must be hashable")

        try:
            label_id = label_to_id[label]
        except TypeError as error:
            raise ValueError("fixed_colors labels must be hashable") from error
        except KeyError as error:
            raise ValueError("fixed_colors labels must be present in labels") from error

        fixed_hex[label_id] = normalize_color(color)

    return fixed_hex


def _normalize_position_columns(positions: Sequence[Hashable]) -> list[Hashable]:
    if isinstance(positions, (str, bytes)):
        raise ValueError("positions must be a sequence of column names")

    try:
        columns = list(positions)
    except TypeError as error:
        raise ValueError("positions must be a sequence of column names") from error

    if not 1 <= len(columns) <= 3:
        raise ValueError("positions must name 1, 2, or 3 columns")

    return columns


def _read_column(data: object, column: Hashable) -> object:
    try:
        return cast(Any, data)[column]
    except Exception as error:
        raise ValueError(f"could not read column {column!r}") from error


def _column_to_list(column_data: object, column: Hashable) -> list[object]:
    for method_name in ("to_numpy", "to_list", "tolist"):
        method = getattr(column_data, method_name, None)
        if callable(method):
            try:
                return list(method())
            except TypeError as error:
                raise ValueError(f"column {column!r} must be iterable") from error

    try:
        return list(cast(Any, column_data))
    except TypeError as error:
        raise ValueError(f"column {column!r} must be iterable") from error
