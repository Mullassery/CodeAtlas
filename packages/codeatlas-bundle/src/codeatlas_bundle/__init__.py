"""CodeAtlas Bundle: version CodeAtlas architecture maps inside any repository."""

from codeatlas_bundle.bundle import BundleResult, bundle, check, detect_repo_root
from codeatlas_bundle.config import Config, init_config, load_config

__all__ = [
    "BundleResult",
    "Config",
    "bundle",
    "check",
    "detect_repo_root",
    "init_config",
    "load_config",
    "__version__",
]

__version__ = "0.1.0"
