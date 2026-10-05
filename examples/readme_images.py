from __future__ import annotations

import argparse
import math
from collections.abc import Mapping
from pathlib import Path
from random import Random
from typing import Any

from okpalette import create_label_palette, create_palette, extend_palette
from okpalette_word_scatter import _hex_to_oklab, _oklab_distance

Point2D = tuple[float, float]

SWATCH_HEIGHT = 0.5
SWATCH_DPI = 200
BRAND = ["#0057b8", "#ffd700"]
DARK_BACKGROUND = "#1e1e1e"

# Labels whose cluster centers are closer than this count as touching.
TOUCHING_DISTANCE = 1.6
# Touching labels whose OKLab distance falls below this count as easy to confuse.
CONFUSABLE_DISTANCE = 0.25


def gallery() -> dict[str, dict[str, Any]]:
    return {
        "default": {"colors": create_palette(12)},
        "extend": {"colors": extend_palette(BRAND, 12), "anchors": len(BRAND)},
        "background": {
            "colors": create_palette(
                12,
                background=DARK_BACKGROUND,
                background_contrast="high",
            ),
            "background": DARK_BACKGROUND,
        },
        "red-green": {"colors": create_palette(12, colorblind_mode="red-green")},
        "muted": {"colors": create_palette(12, chroma=(0.02, 0.12))},
        "bright": {"colors": create_palette(12, chroma=(0.10, None))},
        "mid-lightness": {"colors": create_palette(12, lightness=(0.30, 0.80))},
        "warm": {"colors": create_palette(12, hue=(330, 100))},
        "cool": {"colors": create_palette(12, hue=(150, 280))},
        "coarse": {"colors": create_palette(12, grid_size="coarse")},
        "fine": {"colors": create_palette(12, grid_size="fine")},
    }


def save_swatch(
    path: Path,
    colors: list[str],
    *,
    anchors: int = 0,
    background: str | None = None,
) -> None:
    import matplotlib.pyplot as plt
    from matplotlib.patches import Rectangle

    gap = 0.35 if anchors else 0.0
    pad = 0.25 if background else 0.0
    width = len(colors) + gap + 2 * pad
    height = 1.0 + 2 * pad

    fig = plt.figure(figsize=(width * SWATCH_HEIGHT, height * SWATCH_HEIGHT))
    axis = fig.add_axes((0, 0, 1, 1))
    axis.set_xlim(0, width)
    axis.set_ylim(0, height)
    axis.axis("off")
    if background:
        axis.add_patch(Rectangle((0, 0), width, height, color=background))
    for index, color in enumerate(colors):
        x = pad + index + (gap if anchors and index >= anchors else 0.0)
        axis.add_patch(Rectangle((x, pad), 1.0, 1.0, color=color))

    fig.savefig(path, dpi=SWATCH_DPI, transparent=True)
    plt.close(fig)


def build_clusters(
    count: int = 16,
    *,
    seed: int = 2,
    points_per_cluster: int = 60,
) -> tuple[list[Point2D], list[Point2D], list[int]]:
    rng = Random(seed)
    centers: list[Point2D] = []
    while len(centers) < count:
        center = (rng.uniform(0, 10), rng.uniform(0, 4))
        if all(math.dist(center, other) > 0.9 for other in centers):
            centers.append(center)

    positions: list[Point2D] = []
    labels: list[int] = []
    for label, (x, y) in enumerate(centers):
        for _ in range(points_per_cluster):
            positions.append((rng.gauss(x, 0.35), rng.gauss(y, 0.35)))
            labels.append(label)
    return centers, positions, labels


def confusable_pairs(centers: list[Point2D], palette: Mapping[Any, str]) -> list[tuple[int, int]]:
    return [
        (left, right)
        for left in range(len(centers))
        for right in range(left + 1, len(centers))
        if math.dist(centers[left], centers[right]) < TOUCHING_DISTANCE
        and _oklab_distance(_hex_to_oklab(palette[left]), _hex_to_oklab(palette[right]))
        < CONFUSABLE_DISTANCE
    ]


def save_label_comparison(path: Path) -> None:
    import matplotlib.pyplot as plt

    centers, positions, labels = build_clusters()
    in_order = dict(enumerate(create_palette(len(centers))))
    position_aware = create_label_palette(positions, labels)
    panels = [
        ("create_palette(16), assigned in label order", in_order),
        ("create_label_palette(positions, labels)", position_aware),
    ]

    fig, axes = plt.subplots(1, 2, figsize=(10, 2.4), facecolor="white")
    for axis, (title, palette) in zip(axes, panels, strict=True):
        pairs = confusable_pairs(centers, palette)
        axis.scatter(
            [x for x, _ in positions],
            [y for _, y in positions],
            c=[palette[label] for label in labels],
            s=4,
            linewidths=0,
        )
        for left, right in pairs:
            (x0, y0), (x1, y1) = centers[left], centers[right]
            axis.plot([x0, x1], [y0, y1], color="black", linewidth=2.0, linestyle=(0, (2, 1.5)))
        noun = "pair" if len(pairs) == 1 else "pairs"
        axis.set_title(
            f"{title}\n{len(pairs)} touching {noun} with similar colors",
            fontsize=10,
            family="monospace",
        )
        axis.set_aspect("equal", adjustable="box")
        axis.set_xticks([])
        axis.set_yticks([])
        for spine in axis.spines.values():
            spine.set_visible(False)

    fig.subplots_adjust(wspace=0.05)
    fig.savefig(path, dpi=SWATCH_DPI, facecolor="white", bbox_inches="tight", pad_inches=0.1)
    plt.close(fig)


def main() -> int:
    import matplotlib

    matplotlib.use("Agg")

    parser = argparse.ArgumentParser(description="Write the images shown in the README.")
    parser.add_argument("--output-dir", type=Path, default=Path("examples/output"))
    args = parser.parse_args()
    output_dir: Path = args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)

    for name, swatch in gallery().items():
        path = output_dir / f"palette-{name}.png"
        save_swatch(
            path,
            swatch["colors"],
            anchors=swatch.get("anchors", 0),
            background=swatch.get("background"),
        )
        print(f"wrote {path}")

    path = output_dir / "label-palette-comparison.png"
    save_label_comparison(path)
    print(f"wrote {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
