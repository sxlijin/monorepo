use anyhow::{anyhow, Context, Result};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pythonize::depythonize;
use serde::Deserialize;
use std::ffi::CString;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SeparationResult {
    pub success: bool,
    pub stem_dir: Option<String>,
    #[serde(default)]
    pub generated_files: Vec<String>,
    #[serde(default)]
    pub drum_split_performed: bool,
    pub error: Option<String>,
}

const PY_SOURCE: &str = include_str!("stem_separation.py");

pub fn separate_stems(audio_path: &str, stems_root: &Path) -> Result<SeparationResult> {
    Python::with_gil(|py| -> Result<SeparationResult> {
        let code_cstr = CString::new(PY_SOURCE)
            .context("Failed to create CString from stem separation code")?;
        let globals = PyDict::new(py);
        globals
            .set_item("__builtins__", py.import("builtins")?)
            .context("Failed to populate builtins for stem separation scope")?;
        py.run(&code_cstr, Some(&globals), Some(&globals))
            .context("Failed to execute stem_separation.py")?;

        let separate_fn = globals
            .get_item("separate_stems")
            .context("Failed to get 'separate_stems' function from Python namespace")?
            .context("Python 'separate_stems' function not found")?;

        let stems_root_str = stems_root
            .to_str()
            .ok_or_else(|| anyhow!("Stems root path contains invalid UTF-8"))?;

        let py_result = separate_fn
            .call1((audio_path, stems_root_str))
            .context("Failed to call Python separate_stems function")?;

        let result: SeparationResult = depythonize(&py_result)
            .context("Failed to deserialize Python separation result")?;

        Ok(result)
    })
}
