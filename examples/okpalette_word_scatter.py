from __future__ import annotations

import argparse
import itertools
import math
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from random import Random
from typing import Any, TypeAlias, cast

from okpalette import create_label_palette

Point2D: TypeAlias = tuple[float, float]
Oklab: TypeAlias = tuple[float, float, float]
WeightedEdge: TypeAlias = tuple[int, int, float]

WORD = "okpalette"


@dataclass(frozen=True)
class WordScatter:
    positions: list[Point2D]
    labels: list[str]
    label_order: list[str]


def build_word_scatter(word: str = WORD, *, point_step: float = 0.036) -> WordScatter:
    from matplotlib.font_manager import FontProperties
    from matplotlib.textpath import TextPath
    from matplotlib.transforms import Affine2D

    font = FontProperties(family="DejaVu Sans", weight="bold")
    seen: Counter[str] = Counter()
    cursor = 0.0
    spacing = point_step * 2.0
    positions: list[Point2D] = []
    labels: list[str] = []
    label_order: list[str] = []

    for character_index, character in enumerate(word):
        seen[character] += 1
        label = f"{character}{seen[character]}"
        label_order.append(label)

        glyph = TextPath((0.0, 0.0), character, size=1.0, prop=font)
        bounds = glyph.get_extents()
        transform = Affine2D().translate(cursor - bounds.x0, -bounds.y0)
        glyph = transform.transform_path(glyph)
        bounds = glyph.get_extents()

        glyph_points = _sample_path(glyph, point_step, seed=character_index)
        positions.extend(glyph_points)
        labels.extend([label] * len(glyph_points))

        cursor = bounds.x1 + spacing

    x_mid = _midpoint([x for x, _ in positions])
    y_mid = _midpoint([y for _, y in positions])
    centered_positions = [(x - x_mid, y - y_mid) for x, y in positions]
    return WordScatter(
        positions=centered_positions,
        labels=labels,
        label_order=label_order,
    )


def _sample_path(path: Any, step: float, *, seed: int) -> list[Point2D]:
    bounds = path.get_extents()
    candidates: list[Point2D] = []
    y = bounds.y0
    while y <= bounds.y1:
        x = bounds.x0
        while x <= bounds.x1:
            candidates.append((x, y))
            x += step
        y += step

    inside = path.contains_points(candidates)
    jitter = step * 0.22
    rng = Random(seed)
    return [
        (
            point[0] + rng.uniform(-jitter, jitter),
            point[1] + rng.uniform(-jitter, jitter),
        )
        for point, is_inside in zip(candidates, inside, strict=True)
        if is_inside
    ]


def _midpoint(values: list[float]) -> float:
    return (min(values) + max(values)) / 2.0


def turbo_palette(label_order: list[str]) -> dict[str, str]:
    return dict(zip(label_order, turbo_colors(len(label_order)), strict=True))


def turbo_colors(count: int) -> list[str]:
    from matplotlib import colormaps
    from matplotlib.colors import to_hex

    cmap = colormaps["turbo"]
    if count == 1:
        samples = [0.5]
    else:
        samples = [0.04 + index * (0.92 / (count - 1)) for index in range(count)]
    return [to_hex(cmap(sample), keep_alpha=False) for sample in samples]


def position_aware_turbo_palette(data: WordScatter) -> dict[str, str]:
    colors = turbo_colors(len(data.label_order))
    edges = _position_edges(data, neighbors=2)
    best_order = _best_color_order(colors, edges)
    return {
        label: colors[color_index]
        for label, color_index in zip(data.label_order, best_order, strict=True)
    }


def okpalette_generated_palette(data: WordScatter) -> dict[str, str]:
    return cast(
        dict[str, str],
        create_label_palette(
            data.positions,
            data.labels,
            neighbors=12,
            max_points=None,
        ),
    )


def _position_edges(data: WordScatter, *, neighbors: int) -> list[WeightedEdge]:
    grouped: dict[str, list[Point2D]] = {label: [] for label in data.label_order}
    for position, label in zip(data.positions, data.labels, strict=True):
        grouped[label].append(position)

    pair_distances: dict[tuple[int, int], float] = {}
    for left_index, left_label in enumerate(data.label_order):
        for right_index in range(left_index + 1, len(data.label_order)):
            right_label = data.label_order[right_index]
            pair_distances[(left_index, right_index)] = _closest_distance(
                grouped[left_label],
                grouped[right_label],
            )

    selected_pairs: set[tuple[int, int]] = set()
    for label_index in range(len(data.label_order)):
        nearest = sorted(
            (
                (distance, pair)
                for pair, distance in pair_distances.items()
                if label_index in pair
            ),
            key=lambda item: (item[0], item[1]),
        )
        selected_pairs.update(pair for _distance, pair in nearest[:neighbors])

    return [
        (left, right, 1.0 / (pair_distances[(left, right)] + 0.02))
        for left, right in sorted(selected_pairs)
    ]


def _closest_distance(left_points: list[Point2D], right_points: list[Point2D]) -> float:
    best_squared = math.inf
    for left_x, left_y in left_points:
        for right_x, right_y in right_points:
            dx = left_x - right_x
            dy = left_y - right_y
            best_squared = min(best_squared, dx * dx + dy * dy)
    return math.sqrt(best_squared)


def _best_color_order(colors: list[str], edges: list[WeightedEdge]) -> tuple[int, ...]:
    color_labs = [_hex_to_oklab(color) for color in colors]
    distances = [
        [
            _oklab_distance(left, right)
            for right in color_labs
        ]
        for left in color_labs
    ]
    best_order = tuple(range(len(colors)))
    best_score = _assignment_score(best_order, edges, distances)

    for order in itertools.permutations(range(len(colors))):
        score = _assignment_score(order, edges, distances)
        if score > best_score:
            best_order = order
            best_score = score

    return best_order


def _assignment_score(
    order: tuple[int, ...],
    edges: list[WeightedEdge],
    color_distances: list[list[float]],
) -> tuple[float, float, float]:
    edge_distances = [
        color_distances[order[left]][order[right]]
        for left, right, _weight in edges
    ]
    weighted_mean = sum(
        color_distances[order[left]][order[right]] * weight
        for left, right, weight in edges
    ) / sum(weight for _left, _right, weight in edges)
    return (
        _percentile(edge_distances, 0.0),
        _percentile(edge_distances, 0.1),
        weighted_mean,
    )


def save_word_scatter_demo(output_base: Path) -> Path:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    data = build_word_scatter()
    edges = _position_edges(data, neighbors=2)
    palettes = [
        ("Turbo in word order", turbo_palette(data.label_order)),
        ("Same Turbo colors, position-aware reorder", position_aware_turbo_palette(data)),
        ("okpalette generated colors", okpalette_generated_palette(data)),
    ]

    fig, axes = plt.subplots(
        len(palettes),
        1,
        figsize=(9, 6.8),
        sharex=True,
        sharey=True,
    )
    fig.suptitle(
        "Basic position-aware example",
        fontsize=15,
        y=0.98,
    )

    for axis, (title, palette) in zip(axes, palettes, strict=True):
        _draw_word_scatter(axis, data, palette)
        axis.set_title(f"{title}\n{_edge_distance_summary(data.label_order, palette, edges)}")

    fig.subplots_adjust(left=0.03, right=0.99, bottom=0.04, top=0.87, hspace=0.34)

    png_path = output_base.with_suffix(".png")
    png_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(png_path, dpi=220, bbox_inches="tight")
    plt.close(fig)
    return png_path


def _draw_word_scatter(
    axis: Any,
    data: WordScatter,
    palette: dict[str, str],
    *,
    point_size: float = 10.0,
) -> None:
    x_values = [x for x, _ in data.positions]
    y_values = [y for _, y in data.positions]
    colors = [palette[label] for label in data.labels]
    axis.scatter(x_values, y_values, c=colors, s=point_size, alpha=0.92, linewidths=0)
    axis.set_aspect("equal", adjustable="box")
    axis.set_xticks([])
    axis.set_yticks([])
    for spine in axis.spines.values():
        spine.set_visible(False)


def _edge_distance_summary(
    label_order: list[str],
    palette: dict[str, str],
    edges: list[WeightedEdge],
) -> str:
    distances = [
        _oklab_distance(_hex_to_oklab(palette[left]), _hex_to_oklab(palette[right]))
        for left_index, right_index, _weight in edges
        for left, right in [(label_order[left_index], label_order[right_index])]
    ]
    return f"nearby-label OKLab distance: min {_percentile(distances, 0.0):.3f}"


def _percentile(values: list[float], quantile: float) -> float:
    sorted_values = sorted(values)
    if not sorted_values:
        return 0.0
    rank = (len(sorted_values) - 1) * quantile
    low = math.floor(rank)
    high = math.ceil(rank)
    if low == high:
        return sorted_values[low]
    fraction = rank - low
    return sorted_values[low] * (1.0 - fraction) + sorted_values[high] * fraction


def _oklab_distance(left: Oklab, right: Oklab) -> float:
    return math.sqrt(sum((a - b) ** 2 for a, b in zip(left, right, strict=True)))


def _hex_to_oklab(color: str) -> Oklab:
    color = color.removeprefix("#")
    red = _srgb_to_linear(int(color[0:2], 16) / 255.0)
    green = _srgb_to_linear(int(color[2:4], 16) / 255.0)
    blue = _srgb_to_linear(int(color[4:6], 16) / 255.0)

    l_value = 0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue
    m_value = 0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue
    s_value = 0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue

    l_root = math.copysign(abs(l_value) ** (1.0 / 3.0), l_value)
    m_root = math.copysign(abs(m_value) ** (1.0 / 3.0), m_value)
    s_root = math.copysign(abs(s_value) ** (1.0 / 3.0), s_value)

    return (
        0.2104542553 * l_root + 0.7936177850 * m_root - 0.0040720468 * s_root,
        1.9779984951 * l_root - 2.4285922050 * m_root + 0.4505937099 * s_root,
        0.0259040371 * l_root + 0.7827717662 * m_root - 0.8086757660 * s_root,
    )


def _srgb_to_linear(channel: float) -> float:
    if channel <= 0.04045:
        return channel / 12.92
    return ((channel + 0.055) / 1.055) ** 2.4


def _output_base(path: Path) -> Path:
    if path.suffix.lower() == ".png":
        return path.with_suffix("")
    return path


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Create a Turbo reassignment word-shaped scatterplot demo.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("examples/output/okpalette-word-scatter"),
        help="Output path base, or a .png path whose suffix will be replaced.",
    )
    args = parser.parse_args()

    png_path = save_word_scatter_demo(_output_base(cast(Path, args.output)))
    print(f"wrote {png_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
