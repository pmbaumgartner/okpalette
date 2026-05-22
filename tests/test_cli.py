from __future__ import annotations

import json
import tomllib
from pathlib import Path
from typing import Any

import pytest

from conftest import HEX_COLOR_RE
from okpalette import _cli


def test_create_outputs_json_hex_palette(capsys: pytest.CaptureFixture[str]) -> None:
    exit_code = _cli.main(["create", "3"])

    captured = capsys.readouterr()
    payload = json.loads(captured.out)
    assert exit_code == 0
    assert captured.err == ""
    assert payload["format"] == "hex"
    assert len(payload["colors"]) == 3
    assert all(HEX_COLOR_RE.fullmatch(color) for color in payload["colors"])


@pytest.mark.parametrize("output_format", ["rgb", "rgb01"])
def test_create_serializes_tuple_formats_as_json_arrays(
    output_format: str,
    capsys: pytest.CaptureFixture[str],
) -> None:
    exit_code = _cli.main(["create", "2", "--format", output_format])

    captured = capsys.readouterr()
    payload = json.loads(captured.out)
    assert exit_code == 0
    assert payload["format"] == output_format
    assert all(isinstance(color, list) and len(color) == 3 for color in payload["colors"])


def test_create_accepts_common_palette_options(capsys: pytest.CaptureFixture[str]) -> None:
    exit_code = _cli.main(
        [
            "create",
            "3",
            "--seed-color",
            "#ff0000",
            "--avoid-color",
            "#000000",
            "--background",
            "#ffffff",
            "--background-contrast",
            "normal",
            "--colorblind-mode",
            "red-green",
        ]
    )

    captured = capsys.readouterr()
    payload = json.loads(captured.out)
    assert exit_code == 0
    assert captured.err == ""
    assert len(payload["colors"]) == 3


def test_extend_outputs_json_and_preserves_existing_colors(
    capsys: pytest.CaptureFixture[str],
) -> None:
    exit_code = _cli.main(["extend", "4", "--color", "#ff0000", "--color", "#00ff00"])

    captured = capsys.readouterr()
    payload = json.loads(captured.out)
    assert exit_code == 0
    assert payload["format"] == "hex"
    assert len(payload["colors"]) == 4
    assert payload["colors"][:2] == ["#ff0000", "#00ff00"]


def test_extend_generated_only_omits_existing_colors(capsys: pytest.CaptureFixture[str]) -> None:
    exit_code = _cli.main(
        ["extend", "4", "--color", "#ff0000", "--color", "#00ff00", "--generated-only"]
    )

    captured = capsys.readouterr()
    payload = json.loads(captured.out)
    assert exit_code == 0
    assert len(payload["colors"]) == 4
    assert "#ff0000" not in payload["colors"]
    assert "#00ff00" not in payload["colors"]


def test_extend_accepts_extra_seed_colors_without_returning_them(
    capsys: pytest.CaptureFixture[str],
) -> None:
    exit_code = _cli.main(["extend", "4", "--color", "#ff0000", "--seed-color", "#00ff00"])

    captured = capsys.readouterr()
    payload = json.loads(captured.out)
    assert exit_code == 0
    assert len(payload["colors"]) == 4
    assert payload["colors"][0] == "#ff0000"
    assert "#00ff00" not in payload["colors"]


def test_api_validation_failure_leaves_stdout_empty(
    capsys: pytest.CaptureFixture[str],
) -> None:
    exit_code = _cli.main(["create", "3", "--background", "#ffffff"])

    captured = capsys.readouterr()
    assert exit_code == 1
    assert captured.out == ""
    assert "background_contrast must be provided" in captured.err
    assert "Traceback" not in captured.err


def test_invalid_arguments_leave_stdout_empty(capsys: pytest.CaptureFixture[str]) -> None:
    exit_code = _cli.main(["extend", "4"])

    captured = capsys.readouterr()
    assert exit_code == 2
    assert captured.out == ""
    assert "error:" in captured.err
    assert "Traceback" not in captured.err


def test_install_skill_dry_run_reports_target_and_writes_nothing(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    codex_home = tmp_path / "codex-home"
    monkeypatch.setenv("CODEX_HOME", str(codex_home))

    exit_code = _cli.main(["install-skill", "--agent", "codex", "--dry-run"])

    captured = capsys.readouterr()
    destination = codex_home / "skills" / "okpalette" / "SKILL.md"
    assert exit_code == 0
    assert captured.out == f"Would install codex skill: {destination}\n"
    assert captured.err == ""
    assert not destination.exists()


def test_console_script_entry_point_is_declared() -> None:
    pyproject = tomllib.loads(Path("pyproject.toml").read_text(encoding="utf-8"))

    assert pyproject["project"]["scripts"] == {"okpalette": "okpalette._cli:main"}
    assert callable(getattr(_cli, "main"))


def test_main_uses_sys_argv_when_argv_is_omitted(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    monkeypatch.setattr("sys.argv", ["okpalette", "create", "1"])

    exit_code = _cli.main()

    captured = capsys.readouterr()
    payload: dict[str, Any] = json.loads(captured.out)
    assert exit_code == 0
    assert payload["format"] == "hex"
    assert len(payload["colors"]) == 1
