//! PyO3 bindings exposing the CodeAtlas core to Python as `codeatlas._core`.

use pyo3::prelude::*;

/// Return the core engine version string.
#[pyfunction]
fn version() -> &'static str {
    codeatlas_core::VERSION
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
