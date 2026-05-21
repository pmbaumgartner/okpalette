from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Literal, TypeAlias, cast

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
CvdSimulation: TypeAlias = Literal["protan", "deutan", "tritan"]
CvdMatrix: TypeAlias = tuple[
    tuple[float, float, float],
    tuple[float, float, float],
    tuple[float, float, float],
]
CVD_MATRICES: dict[CvdSimulation, CvdMatrix] = {
    "protan": (
        (0.152_286, 1.052_583, -0.204_868),
        (0.114_503, 0.786_281, 0.099_216),
        (-0.003_882, -0.048_116, 1.051_998),
    ),
    "deutan": (
        (0.367_322, 0.860_646, -0.227_968),
        (0.280_085, 0.672_501, 0.047_413),
        (-0.011_820, 0.042_940, 0.968_881),
    ),
    "tritan": (
        (1.255_528, -0.076_749, -0.178_779),
        (-0.078_411, 0.930_809, 0.147_602),
        (0.004_733, 0.691_367, 0.303_900),
    ),
}


@dataclass(frozen=True)
class PaletteSmokeCase:
    name: str
    build_palette: Callable[[], list[ColorLike]]
    width: int = 1246
    height: int = 154


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


PALETTE_SMOKE_CASES = [
    PaletteSmokeCase(
        "palette-default-24",
        lambda: cast(list[ColorLike], create_palette(24)),
    ),
    PaletteSmokeCase(
        "palette-wcag-white-24",
        lambda: cast(
            list[ColorLike],
            create_palette(24, background="#ffffff", background_contrast="wcag"),
        ),
    ),
    PaletteSmokeCase(
        "palette-cvd-protan-24",
        lambda: cast(list[ColorLike], create_palette(24, colorblind_mode="protan")),
    ),
    PaletteSmokeCase(
        "palette-cvd-deutan-24",
        lambda: cast(list[ColorLike], create_palette(24, colorblind_mode="deutan")),
    ),
    PaletteSmokeCase(
        "palette-cvd-tritan-24",
        lambda: cast(list[ColorLike], create_palette(24, colorblind_mode="tritan")),
    ),
    PaletteSmokeCase(
        "palette-cvd-all-24",
        lambda: cast(list[ColorLike], create_palette(24, colorblind_mode="all")),
    ),
    PaletteSmokeCase(
        "palette-wcag-white-cvd-all-24",
        lambda: cast(
            list[ColorLike],
            create_palette(
                24,
                background="#ffffff",
                background_contrast="wcag",
                colorblind_mode="all",
            ),
        ),
    ),
    PaletteSmokeCase(
        "brand-extended-16",
        lambda: cast(list[ColorLike], extend_palette(["#0057b8", "#ffd700"], 16)),
    ),
    PaletteSmokeCase(
        "brand-extended-cvd-all-16",
        lambda: cast(
            list[ColorLike],
            extend_palette(["#0057b8", "#ffd700"], 16, colorblind_mode="all"),
        ),
    ),
    PaletteSmokeCase(
        "palette-warm-16",
        lambda: cast(list[ColorLike], create_palette(16, hue=(330, 100))),
        width=998,
        height=141,
    ),
    PaletteSmokeCase(
        "palette-cool-16",
        lambda: cast(list[ColorLike], create_palette(16, hue=(150, 280))),
        width=998,
        height=141,
    ),
]


@pytest.mark.parametrize("case", PALETTE_SMOKE_CASES, ids=lambda case: case.name)
def test_plot_review_images_are_written(
    case: PaletteSmokeCase,
) -> None:
    _require_plot_smoke()

    svg_path, png_path = _save_review_images(
        case.name,
        case.build_palette(),
        width=case.width,
        height=case.height,
    )

    assert svg_path.is_file()
    assert png_path.is_file()


def test_palette_cvd_simulation_grid_is_written() -> None:
    _require_plot_smoke()

    palettes = [
        ("default", cast(list[str], create_palette(24))),
        (
            "wcag white",
            cast(
                list[str],
                create_palette(24, background="#ffffff", background_contrast="wcag"),
            ),
        ),
        ("cvd all", cast(list[str], create_palette(24, colorblind_mode="all"))),
        (
            "wcag white + cvd all",
            cast(
                list[str],
                create_palette(
                    24,
                    background="#ffffff",
                    background_contrast="wcag",
                    colorblind_mode="all",
                ),
            ),
        ),
    ]

    svg_path, png_path = _save_palette_simulation_grid(
        _output_dir() / "palette-cvd-simulation-grid",
        palettes,
    )

    assert svg_path.is_file()
    assert png_path.is_file()


def test_label_palette_position_review_artifact_is_written() -> None:
    _require_plot_smoke()

    positions, labels = _label_smoke_dataset_2d()
    first_seen = first_seen_label_palette(
        labels,
        grid_size=32,
    )
    first_seen_cvd_all = first_seen_label_palette(
        labels,
        grid_size=32,
        colorblind_mode="all",
    )
    first_seen_wcag_cvd_all = first_seen_label_palette(
        labels,
        grid_size=32,
        background=MATPLOTLIB_3D_PANE_COLORS,
        background_contrast="wcag",
        colorblind_mode="all",
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
    position_aware_cvd_all = raw_label_palette(
        positions,
        labels,
        grid_size=32,
        colorblind_mode="all",
        max_points=None,
    )
    position_aware_wcag_cvd_all = raw_label_palette(
        positions,
        labels,
        grid_size=32,
        background=MATPLOTLIB_3D_PANE_COLORS,
        background_contrast="wcag",
        colorblind_mode="all",
        max_points=None,
    )

    assert set(position_aware.values()) == set(first_seen.values())
    assert set(position_aware_cvd_all.values()) == set(first_seen_cvd_all.values())
    assert set(position_aware_wcag_cvd_all.values()) == set(first_seen_wcag_cvd_all.values())
    assert not set(position_aware_wcag.values()) & set(MATPLOTLIB_3D_PANE_COLORS)
    assert not set(position_aware_wcag_cvd_all.values()) & set(MATPLOTLIB_3D_PANE_COLORS)

    output_dir = _output_dir()
    comparison_svg, comparison_png = _save_label_scatter_comparison(
        output_dir / "label-position-aware-2d-comparison",
        positions,
        labels,
        [
            ("First seen (default)", first_seen),
            ("Position aware", position_aware),
            ("Position aware + WCAG", position_aware_wcag),
            ("Position aware + CVD all", position_aware_cvd_all),
            ("Position aware + WCAG + CVD all", position_aware_wcag_cvd_all),
        ],
    )
    swatch_svg, swatch_png = _save_review_images(
        "label-position-aware-wcag-cvd-all-100-label-swatch",
        list(position_aware_wcag_cvd_all.values()),
        width=1400,
        height=96,
    )

    assert comparison_svg.stat().st_size > 0
    assert comparison_png.stat().st_size > 0
    for path in _save_simulated_pngs(comparison_png):
        assert path.is_file()
        assert path.stat().st_size > 0
    assert swatch_svg.is_file()
    assert swatch_png.is_file()


def test_label_palette_3d_position_review_artifact_is_written() -> None:
    _require_plot_smoke()

    positions, labels = _label_smoke_dataset_3d()
    first_seen = first_seen_label_palette(
        labels,
        grid_size=32,
    )
    first_seen_cvd_all = first_seen_label_palette(
        labels,
        grid_size=32,
        colorblind_mode="all",
    )
    first_seen_wcag_cvd_all = first_seen_label_palette(
        labels,
        grid_size=32,
        background=MATPLOTLIB_3D_PANE_COLORS,
        background_contrast="wcag",
        colorblind_mode="all",
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
    position_aware_cvd_all = raw_label_palette(
        positions,
        labels,
        grid_size=32,
        colorblind_mode="all",
        max_points=None,
    )
    position_aware_wcag_cvd_all = raw_label_palette(
        positions,
        labels,
        grid_size=32,
        background=MATPLOTLIB_3D_PANE_COLORS,
        background_contrast="wcag",
        colorblind_mode="all",
        max_points=None,
    )

    assert set(position_aware.values()) == set(first_seen.values())
    assert set(position_aware_cvd_all.values()) == set(first_seen_cvd_all.values())
    assert set(position_aware_wcag_cvd_all.values()) == set(first_seen_wcag_cvd_all.values())
    assert not set(position_aware_wcag.values()) & set(MATPLOTLIB_3D_PANE_COLORS)
    assert not set(position_aware_wcag_cvd_all.values()) & set(MATPLOTLIB_3D_PANE_COLORS)

    comparison_svg, comparison_png = _save_label_scatter_3d_comparison(
        _output_dir() / "label-position-aware-3d-composition",
        positions,
        labels,
        [
            ("First seen (default)", first_seen),
            ("Position aware", position_aware),
            ("Position aware + WCAG", position_aware_wcag),
            ("Position aware + CVD all", position_aware_cvd_all),
            ("Position aware + WCAG + CVD all", position_aware_wcag_cvd_all),
        ],
    )

    assert comparison_svg.stat().st_size > 0
    assert comparison_png.stat().st_size > 0
    for path in _save_simulated_pngs(comparison_png):
        assert path.is_file()
        assert path.stat().st_size > 0


def _save_palette_simulation_grid(
    path_base: Path,
    palettes: list[tuple[str, list[str]]],
) -> tuple[Path, Path]:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from matplotlib.patches import Rectangle

    simulation_rows: list[tuple[str, CvdSimulation | None]] = [
        ("normal", None),
        ("protan simulation", "protan"),
        ("deutan simulation", "deutan"),
        ("tritan simulation", "tritan"),
    ]
    swatch_count = max(len(palette) for _, palette in palettes)
    row_count = len(palettes) * len(simulation_rows)

    fig, axis = plt.subplots(figsize=(15, max(4.0, row_count * 0.34)))
    axis.set_xlim(-8.5, swatch_count)
    axis.set_ylim(0, row_count)
    axis.axis("off")
    fig.suptitle("Palette smoke matrix rendered under selected CVD simulations")

    for palette_index, (palette_name, palette) in enumerate(palettes):
        for simulation_index, (simulation_label, simulation) in enumerate(simulation_rows):
            row_index = palette_index * len(simulation_rows) + simulation_index
            y = row_count - row_index - 1
            row_palette = (
                palette
                if simulation is None
                else [_simulate_machado_hex(color, simulation) for color in palette]
            )
            axis.text(
                -0.35,
                y + 0.5,
                f"{palette_name} / {simulation_label}",
                ha="right",
                va="center",
                fontsize=8,
            )
            for color_index, color in enumerate(row_palette):
                axis.add_patch(
                    Rectangle(
                        (color_index, y),
                        1.0,
                        1.0,
                        facecolor=color,
                        edgecolor="none",
                    )
                )

    svg_path = path_base.with_suffix(".svg")
    png_path = path_base.with_suffix(".png")
    fig.savefig(svg_path, format="svg", bbox_inches="tight")
    fig.savefig(png_path, format="png", dpi=160, bbox_inches="tight")
    plt.close(fig)
    return svg_path, png_path


def _save_simulated_pngs(path: Path) -> list[Path]:
    import matplotlib.image as mpimg

    image = mpimg.imread(path)
    output_paths = []
    for simulation in CVD_MATRICES:
        simulated = _simulate_image_array(image, simulation)
        output_path = path.with_name(f"{path.stem}-{simulation}.png")
        mpimg.imsave(output_path, simulated)
        output_paths.append(output_path)

    return output_paths


def _simulate_image_array(image: Any, simulation: CvdSimulation) -> Any:
    import numpy as np

    rgb = image[..., :3].astype(float)
    alpha = image[..., 3:4] if image.shape[-1] == 4 else None
    linear = np.where(rgb <= 0.04045, rgb / 12.92, ((rgb + 0.055) / 1.055) ** 2.4)
    matrix = np.array(CVD_MATRICES[simulation], dtype=float).T
    simulated_linear = np.clip(linear @ matrix, 0.0, 1.0)
    simulated_rgb = np.where(
        simulated_linear <= 0.003_130_8,
        12.92 * simulated_linear,
        1.055 * (simulated_linear ** (1.0 / 2.4)) - 0.055,
    )

    if alpha is None:
        return simulated_rgb

    return np.concatenate([simulated_rgb, alpha], axis=-1)


def _simulate_machado_hex(color: str, simulation: CvdSimulation) -> str:
    matrix = CVD_MATRICES[simulation]
    red = _srgb_to_linear(int(color[1:3], 16))
    green = _srgb_to_linear(int(color[3:5], 16))
    blue = _srgb_to_linear(int(color[5:7], 16))
    simulated = [
        _linear_to_srgb_channel(row[0] * red + row[1] * green + row[2] * blue)
        for row in matrix
    ]
    return f"#{simulated[0]:02x}{simulated[1]:02x}{simulated[2]:02x}"


def _srgb_to_linear(channel: int) -> float:
    value = channel / 255.0
    if value <= 0.04045:
        return value / 12.92
    return ((value + 0.055) / 1.055) ** 2.4


def _linear_to_srgb_channel(value: float) -> int:
    clamped = min(max(value, 0.0), 1.0)
    if clamped <= 0.003_130_8:
        encoded = 12.92 * clamped
    else:
        encoded = 1.055 * (clamped ** (1.0 / 2.4)) - 0.055
    return round(encoded * 255.0)


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
