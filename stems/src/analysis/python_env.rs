use anyhow::{Context, Result};
use pyo3::prelude::*;
use std::path::PathBuf;

pub fn bootstrap_venv_site_packages(py: Python<'_>) -> Result<()> {
    let sys = py.import("sys").context("Failed to import sys")?;
    let version_info = sys.getattr("version_info")?;
    let major: u8 = version_info.getattr("major")?.extract()?;
    let minor: u8 = version_info.getattr("minor")?.extract()?;

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let site_packages = repo_root
        .join(".venv")
        .join("lib")
        .join(format!("python{major}.{minor}"))
        .join("site-packages");

    if !site_packages.exists() {
        anyhow::bail!(
            "Virtualenv site-packages not found at {}. Did you run `uv sync`?",
            site_packages.display()
        );
    }

    let site_packages_str = site_packages
        .to_str()
        .context("site-packages path is not valid UTF-8")?;

    let existing: Vec<String> = sys.getattr("path")?.extract()?;
    if !existing.iter().any(|p| p == site_packages_str) {
        sys.getattr("path")?
            .call_method1("insert", (0_usize, site_packages_str))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_enables_venv_imports() {
        Python::with_gil(|py| {
            bootstrap_venv_site_packages(py).expect("bootstrap should succeed");
            py.import("librosa").expect("librosa should import after bootstrap");
            py.import("numpy").expect("numpy should import after bootstrap");
            py.import("soundfile").expect("soundfile should import after bootstrap");
        });
    }
}
