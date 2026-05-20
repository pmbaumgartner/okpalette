from __future__ import annotations

import struct
from pathlib import Path
from typing import Any, Callable

import pytest

import glasbey_rs
from glasbey_rs import palette_png, palette_svg, save_palette, view_palette
from glasbey_rs import _plot


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def png_dimensions(png: bytes) -> tuple[int, int]:
    return (
        struct.unpack(">I", png[16:20])[0],
        struct.unpack(">I", png[20:24])[0],
    )


def test_palette_svg_renders_normalized_swatches() -> None:
    svg = palette_svg(["#F00", (0.0, 1.0, 0.0)], width=20, height=6)

    assert svg.startswith('<svg xmlns="http://www.w3.org/2000/svg" width="20" height="6"')
    assert '<rect x="0" y="0" width="10" height="6" fill="#ff0000"/>' in svg
    assert '<rect x="10" y="0" width="10" height="6" fill="#00ff00"/>' in svg
    assert svg.endswith("</svg>")


def test_palette_png_renders_png_bytes() -> None:
    png = palette_png(["#F00", "#00F"], width=20, height=6)

    assert png.startswith(PNG_SIGNATURE)
    assert png_dimensions(png) == (20, 6)


def test_view_palette_is_displayable_and_saves_files(tmp_path: Path) -> None:
    view = view_palette(["#F00", "#00F"], width=20, height=6)

    assert isinstance(view, glasbey_rs.PaletteView)
    assert view.colors == ["#ff0000", "#0000ff"]
    assert view._repr_svg_() == view.svg()
    assert view._repr_png_() == view.png()

    svg_path = view.save(tmp_path / "palette.svg")
    png_path = view.save(tmp_path / "palette.png")

    assert svg_path.read_text(encoding="utf-8").startswith("<svg")
    assert png_path.read_bytes().startswith(PNG_SIGNATURE)


def test_save_palette_rejects_unknown_suffix(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match=r"\.svg or \.png"):
        save_palette(["#ff0000"], tmp_path / "palette.txt")


@pytest.mark.parametrize(
    "call",
    [
        lambda: palette_svg([]),
        lambda: palette_png([]),
        lambda: palette_svg(["#ff0000"], width=0),
        lambda: palette_png(["#ff0000"], height=0),
        lambda: palette_png(["#ff0000", "#00ff00"], width=1),
        lambda: view_palette(["#ff0000"], width=True),
    ],
)
def test_preview_helpers_reject_invalid_inputs(call: Callable[[], object]) -> None:
    with pytest.raises(ValueError):
        call()


def test_native_extension_missing_error_is_actionable(monkeypatch: pytest.MonkeyPatch) -> None:
    def missing_renderers() -> tuple[Any, Any]:
        raise ImportError("native extension unavailable")

    monkeypatch.setattr(_plot, "_load_renderers", missing_renderers)

    with pytest.raises(ImportError, match="native extension unavailable"):
        palette_svg(["#ff0000"])
