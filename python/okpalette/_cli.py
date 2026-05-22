from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from dataclasses import dataclass
from typing import Any, NoReturn, cast

from . import create_palette, extend_palette
from ._types import BackgroundContrast, ColorblindMode, ColorFormat, ColorLike
from .skill_installer import AgentName, install_skill

_COLOR_FORMATS = ("hex", "rgb", "rgb01")
_BACKGROUND_CONTRASTS = ("normal", "high", "wcag")
_COLORBLIND_MODES = ("protan", "deutan", "tritan", "red-green", "all")
_AGENTS = ("codex", "claude")


@dataclass(frozen=True)
class _PaletteCliOptions:
    format: ColorFormat
    seed_colors: Sequence[ColorLike]
    avoid_colors: Sequence[ColorLike] | None
    background: Sequence[ColorLike] | None
    background_contrast: BackgroundContrast | None
    colorblind_mode: ColorblindMode | None


class _ParserExit(Exception):
    def __init__(self, status: int) -> None:
        self.status = status


class _ArgumentError(Exception):
    def __init__(self, parser: argparse.ArgumentParser, message: str) -> None:
        self.parser = parser
        super().__init__(message)


class _ArgumentParser(argparse.ArgumentParser):
    def error(self, message: str) -> NoReturn:
        raise _ArgumentError(self, message)

    def exit(self, status: int = 0, message: str | None = None) -> NoReturn:
        if message:
            stream = sys.stderr if status else sys.stdout
            print(message, file=stream, end="")
        raise _ParserExit(status)


def build_parser() -> argparse.ArgumentParser:
    parser = _ArgumentParser(
        prog="okpalette",
        description="Create deterministic OKLab categorical palettes.",
    )
    subparsers = parser.add_subparsers(
        dest="command",
        required=True,
        parser_class=_ArgumentParser,
    )

    create = subparsers.add_parser(
        "create",
        help="Create a deterministic palette.",
    )
    create.add_argument("size", type=int, metavar="SIZE")
    _add_common_palette_options(create)
    create.set_defaults(func=_create_command)

    extend = subparsers.add_parser(
        "extend",
        help="Extend an existing palette.",
    )
    extend.add_argument("target_size", type=int, metavar="TARGET_SIZE")
    extend.add_argument(
        "--color",
        dest="colors",
        action="append",
        required=True,
        metavar="COLOR",
        help="Existing palette color. Repeat for multiple colors.",
    )
    extend.add_argument(
        "--generated-only",
        action="store_true",
        help="Return only generated colors instead of prepending existing colors.",
    )
    _add_common_palette_options(extend)
    extend.set_defaults(func=_extend_command)

    install = subparsers.add_parser(
        "install-skill",
        help="Install the packaged agent skill.",
    )
    install.add_argument("--agent", choices=_AGENTS, required=True)
    install.add_argument("--overwrite", action="store_true")
    install.add_argument("--dry-run", action="store_true")
    install.set_defaults(func=_install_skill_command)

    return parser


def _add_common_palette_options(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--format", choices=_COLOR_FORMATS, default="hex")
    parser.add_argument(
        "--seed-color",
        dest="seed_colors",
        action="append",
        metavar="COLOR",
        help="Seed color to use as a distance anchor. Repeat for multiple colors.",
    )
    parser.add_argument(
        "--avoid-color",
        dest="avoid_colors",
        action="append",
        metavar="COLOR",
        help="Color to exclude and use as a distance anchor. Repeat for multiple colors.",
    )
    parser.add_argument(
        "--background",
        dest="background",
        action="append",
        metavar="COLOR",
        help="Background color to separate from. Repeat for multiple backgrounds.",
    )
    parser.add_argument("--background-contrast", choices=_BACKGROUND_CONTRASTS)
    parser.add_argument("--colorblind-mode", choices=_COLORBLIND_MODES)


def _palette_options(args: argparse.Namespace) -> _PaletteCliOptions:
    return _PaletteCliOptions(
        format=cast(ColorFormat, args.format),
        seed_colors=cast(Sequence[ColorLike], args.seed_colors or ()),
        avoid_colors=cast(Sequence[ColorLike] | None, args.avoid_colors),
        background=cast(Sequence[ColorLike] | None, args.background),
        background_contrast=cast(BackgroundContrast | None, args.background_contrast),
        colorblind_mode=cast(ColorblindMode | None, args.colorblind_mode),
    )


def _create_command(args: argparse.Namespace) -> int:
    options = _palette_options(args)
    colors = create_palette(
        cast(int, args.size),
        seed_colors=options.seed_colors,
        avoid_colors=options.avoid_colors,
        background=options.background,
        background_contrast=options.background_contrast,
        colorblind_mode=options.colorblind_mode,
        format=options.format,
    )
    _write_palette(colors, options.format)
    return 0


def _extend_command(args: argparse.Namespace) -> int:
    options = _palette_options(args)
    include_existing = not cast(bool, args.generated_only)
    existing_colors = cast(Sequence[ColorLike], args.colors)
    kwargs: dict[str, object] = {
        "format": options.format,
        "avoid_colors": options.avoid_colors,
        "background": options.background,
        "background_contrast": options.background_contrast,
        "colorblind_mode": options.colorblind_mode,
    }
    seed_colors = tuple(options.seed_colors)
    if seed_colors:
        existing_count = len(existing_colors)
        target_size = args.target_size + len(seed_colors) if include_existing else args.target_size
        colors = extend_palette(
            [*existing_colors, *seed_colors],
            target_size,
            include_existing=include_existing,
            **kwargs,
        )
        if include_existing:
            colors = [*colors[:existing_count], *colors[existing_count + len(seed_colors) :]]
    else:
        colors = extend_palette(
            existing_colors,
            args.target_size,
            include_existing=include_existing,
            **kwargs,
        )
    _write_palette(colors, options.format)
    return 0


def _install_skill_command(args: argparse.Namespace) -> int:
    agent = cast(AgentName, args.agent)
    result = install_skill(
        agent,
        overwrite=cast(bool, args.overwrite),
        dry_run=cast(bool, args.dry_run),
    )
    action = "Would install" if result.dry_run else "Installed"
    print(f"{action} {agent} skill: {result.path}")
    return 0


def _write_palette(colors: object, output_format: ColorFormat) -> None:
    json.dump(
        {"colors": colors, "format": output_format},
        sys.stdout,
        separators=(",", ":"),
    )
    print()


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    tokens = list(sys.argv[1:] if argv is None else argv)
    try:
        args = parser.parse_args(tokens)
        command = cast(Any, args).func
        return int(command(args))
    except _ArgumentError as error:
        print(error.parser.format_usage(), file=sys.stderr, end="")
        print(f"error: {error}", file=sys.stderr)
        return 2
    except _ParserExit as exit_error:
        return exit_error.status
    except (FileExistsError, ImportError, OSError, TypeError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
