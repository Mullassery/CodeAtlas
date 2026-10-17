"""CodeAtlas: Rust-native architecture maps for codebases."""

from codeatlas import _core

__all__ = ["__version__", "_core"]

__version__ = _core.version()