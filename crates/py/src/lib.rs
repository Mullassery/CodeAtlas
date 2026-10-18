//! PyO3 bindings exposing the CodeAtlas core to Python as `codeatlas._core`.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use codeatlas_core::pipeline::{analyze, AnalyzeOptions};

/// Return the core engine version string.
#[pyfunction]
fn version() -> &'static str {
    codeatlas_core::VERSION
}

/// Run a full deterministic analysis of `repo_path`.
///
/// Returns a `(analysis_json, fingerprint_json, file_coverage_json)` tuple of
/// serialized strings. The Python wrapper is responsible for writing them.
#[pyfunction]
#[pyo3(signature = (repo_path, project_name=None, depth_cap=3, ignore_paths=None))]
fn analyze_repo(
    repo_path: String,
    project_name: Option<String>,
    depth_cap: usize,
    ignore_paths: Option<Vec<String>>,
) -> PyResult<(String, String, String)> {
    let repo_path = std::path::PathBuf::from(repo_path);
    let project_name = project_name
        .or_else(|| {
            repo_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "codebase".to_string());

    let opts = AnalyzeOptions {
        repo_path,
        project_name,
        depth_cap,
        ignore_paths: ignore_paths
            .unwrap_or_default()
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect(),
    };

    let result = analyze(&opts).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    let analysis_json = serde_json::to_string_pretty(&result.analysis)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    let fingerprint_json = serde_json::to_string_pretty(&serde_json::json!({
        "files": result.fingerprint
    }))
    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    let file_coverage_json = serde_json::to_string_pretty(&result.file_coverage)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    Ok((analysis_json, fingerprint_json, file_coverage_json))
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(analyze_repo, m)?)?;
    Ok(())
}
