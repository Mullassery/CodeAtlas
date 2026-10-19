"""Analysis cache tests."""

from __future__ import annotations

from codeatlas_rs.cache import AnalysisCache, CacheEntry, repo_fingerprint


def test_fingerprint_changes_when_source_changes(tmp_path):
    (tmp_path / "a.py").write_text("x = 1\n")
    first = repo_fingerprint(tmp_path)
    (tmp_path / "a.py").write_text("x = 2\n")
    assert repo_fingerprint(tmp_path) != first


def test_cache_round_trip(tmp_path):
    cache = AnalysisCache(tmp_path / "cache")
    entry = CacheEntry("abc", "{}", "{}", "{}")
    cache.store(entry)
    loaded = cache.load("abc")
    assert loaded is not None
    assert loaded.analysis_json == "{}"


def test_missing_cache_returns_none(tmp_path):
    assert AnalysisCache(tmp_path / "cache").load("missing") is None
