from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
from random import Random
from typing import Any, TypeAlias, cast

from okpalette import create_label_palette

Point2D: TypeAlias = tuple[float, float]

WORD = "okpalette"


@dataclass(frozen=True)
class WordGrid:
    positions: list[Point2D]
    labels: list[str]
    label_order: list[str]


def build_word_grid(
    word: str = WORD,
    *,
    rows: int = 9,
    point_step: float = 0.055,
) -> WordGrid:
    from matplotlib.font_manager import FontProperties
    from matplotlib.textpath import TextPath
    from matplotlib.transforms import Affine2D

    font = FontProperties(family="DejaVu Sans", weight="bold")
    cell_width = 0.74
    cell_height = 0.86
    positions: list[Point2D] = []
    labels: list[str] = []
    label_order: list[str] = []

    for row in range(rows):
        for column, character in enumerate(word):
            label = f"r{row + 1}c{column + 1}-{character}"
            label_order.append(label)

            glyph = TextPath((0.0, 0.0), character, size=1.0, prop=font)
            bounds = glyph.get_extents()
            x_offset = column * cell_width + (cell_width - bounds.width) / 2.0 - bounds.x0
            y_offset = -row * cell_height - bounds.y0
            glyph = Affine2D().translate(x_offset, y_offset).transform_path(glyph)

            seed = row * len(word) + column
            glyph_points = _sample_path(glyph, point_step, seed=seed)
            positions.extend(glyph_points)
            labels.extend([label] * len(glyph_points))

    x_mid = _midpoint([x for x, _ in positions])
    y_mid = _midpoint([y for _, y in positions])
    centered_positions = [(x - x_mid, y - y_mid) for x, y in positions]
    return WordGrid(
        positions=centered_positions,
        labels=labels,
        label_order=label_order,
    )


def save_position_aware_logo(output_base: Path) -> Path:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    data = build_word_grid()
    palette = cast(
        dict[str, str],
        create_label_palette(
            data.positions,
            data.labels,
            neighbors=12,
            max_points=None,
        ),
    )

    fig, axis = plt.subplots(figsize=(7.2, 8.2))
    _draw_word_grid(axis, data, palette, point_size=6.5)
    fig.subplots_adjust(left=0.01, right=0.99, bottom=0.01, top=0.99)

    png_path = output_base.with_suffix(".png")
    png_path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(png_path, dpi=260, bbox_inches="tight", pad_inches=0.03)
    plt.close(fig)
    return png_path


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


def _draw_word_grid(
    axis: Any,
    data: WordGrid,
    palette: dict[str, str],
    *,
    point_size: float,
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


def _midpoint(values: list[float]) -> float:
    return (min(values) + max(values)) / 2.0


def _output_base(path: Path) -> Path:
    if path.suffix.lower() == ".png":
        return path.with_suffix("")
    return path


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Create a 9x9 okpalette grid logo with position-aware colors.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("examples/output/okpalette-word-scatter-logo"),
        help="Output path base, or a .png path whose suffix will be replaced.",
    )
    args = parser.parse_args()

    png_path = save_position_aware_logo(_output_base(cast(Path, args.output)))
    print(f"wrote {png_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
