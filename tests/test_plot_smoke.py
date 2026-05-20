from __future__ import annotations

import os
from pathlib import Path
from typing import Any, Callable, cast

import pytest

from conftest import first_seen_label_palette, raw_label_palette
from okpalette import create_palette, extend_palette, save_palette
from okpalette._types import ColorLike

pytestmark = pytest.mark.plot_smoke

POSITION_SMOKE_RANDOM_SEED = 42
POSITION_SMOKE_2D_LABEL_COUNT = 100
POSITION_SMOKE_3D_LABEL_COUNT = 50
POSITION_SMOKE_POINTS_PER_LABEL = 10
POSITION_SMOKE_CLUSTER_STD = 0.26
MATPLOTLIB_3D_PANE_COLORS = ("#f2f2f2", "#e6e6e6", "#ececec")


def _require_plot_smoke() -> None:
    if os.environ.get("OKPALETTE_PLOT_SMOKE") != "1":
        pytest.skip(reason="set OKPALETTE_PLOT_SMOKE=1 to write palette preview artifacts")


def _output_dir() -> Path:
    path = Path(os.environ.get("OKPALETTE_PLOT_SMOKE_DIR", "output/plot_smoke"))
    path.mkdir(parents=True, exist_ok=True)
    return path


def _save_review_images(
    name: str,
    palette: list[ColorLike],
    *,
    width: int,
    height: int,
) -> tuple[Path, Path]:
    output_dir = _output_dir()
    svg_path = save_palette(palette, output_dir / f"{name}.svg", width=width, height=height)
    png_path = save_palette(palette, output_dir / f"{name}.png", width=width, height=height)

    assert svg_path.stat().st_size > 0
    assert png_path.stat().st_size > 0
    return svg_path, png_path


@pytest.mark.parametrize(
    ("name", "build_palette", "width", "height"),
    [
        (
            "default-24",
            lambda: create_palette(24),
            1246,
            154,
        ),
        (
            "brand-extended-16",
            lambda: extend_palette(["#0057b8", "#ffd700"], 16),
            1246,
            154,
        ),
        (
            "warm-16",
            lambda: create_palette(16, hue=(330, 100)),
            998,
            141,
        ),
        (
            "cool-16",
            lambda: create_palette(16, hue=(150, 280)),
            998,
            141,
        ),
    ],
)
def test_plot_review_images_are_written(
    name: str,
    build_palette: Callable[[], list[ColorLike]],
    width: int,
    height: int,
) -> None:
    _require_plot_smoke()

    svg_path, png_path = _save_review_images(name, build_palette(), width=width, height=height)

    assert svg_path.is_file()
    assert png_path.is_file()


def test_label_palette_position_review_artifact_is_written() -> None:
    _require_plot_smoke()

    positions, labels = _label_smoke_dataset_2d()
    first_seen = first_seen_label_palette(
        labels,
        grid_size=32,
    )
    position_aware = raw_label_palette(
        positions,
        labels,
        grid_size=32,
        max_points=None,
    )
    position_aware_wcag = raw_label_palette(
        positions,
        labels,
        grid_size=32,
        background=MATPLOTLIB_3D_PANE_COLORS,
        background_contrast="wcag",
        max_points=None,
    )

    assert set(position_aware.values()) == set(first_seen.values())
    assert not set(position_aware_wcag.values()) & set(MATPLOTLIB_3D_PANE_COLORS)

    output_dir = _output_dir()
    comparison_svg, comparison_png = _save_label_scatter_comparison(
        output_dir / "label-position-aware-100-labels",
        positions,
        labels,
        [
            ("First seen (default)", first_seen),
            ("Position aware", position_aware),
            ("Position aware + WCAG", position_aware_wcag),
        ],
    )
    swatch_svg, swatch_png = _save_review_images(
        "label-position-aware-100-label-swatch",
        list(position_aware.values()),
        width=1400,
        height=96,
    )

    assert comparison_svg.stat().st_size > 0
    assert comparison_png.stat().st_size > 0
    assert swatch_svg.is_file()
    assert swatch_png.is_file()


def test_label_palette_3d_position_review_artifact_is_written() -> None:
    _require_plot_smoke()

    positions, labels = _label_smoke_dataset_3d()
    first_seen = first_seen_label_palette(
        labels,
        grid_size=32,
    )
    position_aware = raw_label_palette(
        positions,
        labels,
        grid_size=32,
        max_points=None,
    )
    position_aware_wcag = raw_label_palette(
        positions,
        labels,
        grid_size=32,
        background=MATPLOTLIB_3D_PANE_COLORS,
        background_contrast="wcag",
        max_points=None,
    )

    assert set(position_aware.values()) == set(first_seen.values())
    assert not set(position_aware_wcag.values()) & set(MATPLOTLIB_3D_PANE_COLORS)

    comparison_svg, comparison_png = _save_label_scatter_3d_comparison(
        _output_dir() / "label-position-aware-50-labels-3d",
        positions,
        labels,
        [
            ("First seen (default)", first_seen),
            ("Position aware", position_aware),
            ("Position aware + WCAG", position_aware_wcag),
        ],
    )

    assert comparison_svg.stat().st_size > 0
    assert comparison_png.stat().st_size > 0


def _label_smoke_dataset_2d() -> tuple[list[tuple[float, float]], list[str]]:
    from sklearn.datasets import make_blobs

    centers = [(float(x), float(y)) for y in range(10) for x in range(10)]
    assert len(centers) == POSITION_SMOKE_2D_LABEL_COUNT
    samples, label_ids = make_blobs(
        n_samples=[POSITION_SMOKE_POINTS_PER_LABEL] * len(centers),
        centers=centers,
        cluster_std=POSITION_SMOKE_CLUSTER_STD,
        random_state=POSITION_SMOKE_RANDOM_SEED,
    )

    positions = [(float(x), float(y)) for x, y in samples]
    labels = [f"L{int(label_id):02d}" for label_id in label_ids]
    return positions, labels


def _label_smoke_dataset_3d() -> tuple[list[tuple[float, float, float]], list[str]]:
    from sklearn.datasets import make_blobs

    centers = [(float(x), float(y), float(z)) for z in range(2) for y in range(5) for x in range(5)]
    assert len(centers) == POSITION_SMOKE_3D_LABEL_COUNT
    samples, label_ids = make_blobs(
        n_samples=[POSITION_SMOKE_POINTS_PER_LABEL] * len(centers),
        centers=centers,
        cluster_std=POSITION_SMOKE_CLUSTER_STD,
        random_state=POSITION_SMOKE_RANDOM_SEED,
    )

    positions = [(float(x), float(y), float(z)) for x, y, z in samples]
    labels = [f"L{int(label_id):02d}" for label_id in label_ids]
    return positions, labels


def _save_label_scatter_comparison(
    path_base: Path,
    positions: list[tuple[float, float]],
    labels: list[str],
    palettes: list[tuple[str, dict[str, str]]],
) -> tuple[Path, Path]:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    x_values = [x for x, _ in positions]
    y_values = [y for _, y in positions]
    fig, axes = plt.subplots(
        1,
        len(palettes),
        figsize=(7 * len(palettes), 7),
        sharex=True,
        sharey=True,
    )
    fig.suptitle("Position-aware label palette smoke test: 100 overlapping labels, 1000 points")

    for axis, (title, palette) in zip(axes, palettes):
        colors = [palette[label] for label in labels]
        axis.scatter(x_values, y_values, c=colors, s=22, alpha=0.88, linewidths=0)
        axis.set_title(title)
        axis.set_aspect("equal", adjustable="box")
        axis.set_xticks([])
        axis.set_yticks([])

    svg_path = path_base.with_suffix(".svg")
    png_path = path_base.with_suffix(".png")
    fig.savefig(svg_path, format="svg", bbox_inches="tight")
    fig.savefig(png_path, format="png", dpi=160, bbox_inches="tight")
    plt.close(fig)
    return svg_path, png_path


def _save_label_scatter_3d_comparison(
    path_base: Path,
    positions: list[tuple[float, float, float]],
    labels: list[str],
    palettes: list[tuple[str, dict[str, str]]],
) -> tuple[Path, Path]:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    x_values = [x for x, _, _ in positions]
    y_values = [y for _, y, _ in positions]
    z_values = [z for _, _, z in positions]
    fig = plt.figure(figsize=(7 * len(palettes), 7))
    fig.suptitle("Position-aware label palette smoke test: 3D, 50 overlapping labels, 500 points")

    for index, (title, palette) in enumerate(palettes, start=1):
        axis = cast(Any, fig.add_subplot(1, len(palettes), index, projection="3d"))
        colors = [palette[label] for label in labels]
        axis.scatter(x_values, y_values, z_values, c=colors, s=20, alpha=0.88, linewidths=0)
        axis.set_title(title)
        axis.set_xticks([])
        axis.set_yticks([])
        axis.set_zticks([])
        axis.set_box_aspect((1.0, 1.0, 0.75))
        axis.view_init(elev=24, azim=-55)

    svg_path = path_base.with_suffix(".svg")
    png_path = path_base.with_suffix(".png")
    fig.savefig(svg_path, format="svg", bbox_inches="tight")
    fig.savefig(png_path, format="png", dpi=160, bbox_inches="tight")
    plt.close(fig)
    return svg_path, png_path
