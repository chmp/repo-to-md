use anyhow::{Result, bail};
use pathsearch::find_executable_in_path;

/// Check if an executable exists in the system PATH.
pub fn check_executable(name: &str) -> Result<()> {
    if find_executable_in_path(name).is_none() {
        bail!(
            "'{name}' executable not found in PATH. Please install {name} and ensure it is available in your PATH.",
        );
    }
    Ok(())
}
