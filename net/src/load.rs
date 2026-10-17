//! Static data read once from the asset files.

use std::path::PathBuf;
use std::sync::OnceLock;

/// The path in the environment variable `var`, or `default`.
pub(crate) fn asset_path(var: &str, default: &str) -> PathBuf {
    std::env::var_os(var).map_or_else(|| default_path(default), PathBuf::from)
}

#[cfg(not(test))]
fn default_path(default: &str) -> PathBuf {
    PathBuf::from(default)
}

/// Tests run in the crate's directory, a level below the assets.
#[cfg(test)]
fn default_path(default: &str) -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(default)
}

/// Load `cell` on first use. A failure is kept and returned to every caller.
pub(crate) fn once<T, E: ToString>(
    cell: &'static OnceLock<Result<T, String>>,
    load: impl FnOnce() -> Result<T, E>,
) -> Result<&'static T, &'static str> {
    cell.get_or_init(|| load().map_err(|e| e.to_string()))
        .as_ref()
        .map_err(String::as_str)
}
