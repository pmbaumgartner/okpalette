from __future__ import annotations

import argparse
from pathlib import Path

from okpalette import create_palette, save_palette


def main() -> int:
    parser = argparse.ArgumentParser(description="Write the palette swatches shown in the README.")
    parser.add_argument("--output-dir", type=Path, default=Path("examples/output"))
    args = parser.parse_args()

    swatches = {
        "okpalette-default-12.png": create_palette(12),
        "okpalette-muted-12.png": create_palette(12, chroma=(0.02, 0.12)),
    }
    args.output_dir.mkdir(parents=True, exist_ok=True)
    for name, colors in swatches.items():
        path = args.output_dir / name
        save_palette(colors, path)
        print(f"wrote {path}: {colors}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
