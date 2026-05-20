"""Fast Glasbey categorical color palettes powered by Rust and OKLab."""

from importlib.metadata import PackageNotFoundError, version

try:
    __version__ = version("glasbey-rs")
except PackageNotFoundError:
    __version__ = "0.0.0"

__all__ = ["__version__"]
