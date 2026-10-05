from __future__ import annotations

import argparse
import math
from pathlib import Path
from random import Random
from typing import Any

from okpalette import create_label_palette, create_palette, extend_palette
from okpalette_word_scatter import turbo_colors

Point2D = tuple[float, float]

SWATCH_HEIGHT = 0.5
SWATCH_DPI = 200
BRAND = ["#0057b8", "#ffd700"]
DARK_BACKGROUND = "#1e1e1e"


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


def build_chain(
    count: int = 10,
    *,
    seed: int = 0,
    points_per_group: int = 70,
) -> tuple[list[Point2D], list[int]]:
    rng = Random(seed)
    positions: list[Point2D] = []
    labels: list[int] = []
    for label in range(count):
        x, y = label * 1.0, 0.9 * math.sin(label * 0.8)
        for _ in range(points_per_group):
            positions.append((rng.gauss(x, 0.32), rng.gauss(y, 0.32)))
            labels.append(label)
    return positions, labels


def save_label_comparison(path: Path) -> None:
    import matplotlib.pyplot as plt

    positions, labels = build_chain()
    label_count = len(set(labels))
    panels = [
        ("Turbo colormap, sampled in label order", dict(enumerate(turbo_colors(label_count)))),
        ("create_label_palette(positions, labels)", create_label_palette(positions, labels)),
    ]

    fig, axes = plt.subplots(2, 1, figsize=(8, 3.2), facecolor="white")
    for axis, (title, palette) in zip(axes, panels, strict=True):
        axis.scatter(
            [x for x, _ in positions],
            [y for _, y in positions],
            c=[palette[label] for label in labels],
            s=6,
            linewidths=0,
        )
        axis.set_title(title, fontsize=10, family="monospace")
        axis.set_aspect("equal")
        axis.axis("off")

    fig.tight_layout()
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
