#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub use native::NativeStorage;

use std::path::{Path, PathBuf};

use promptgen_core::Library;
use thiserror::Error;

/// Errors that can occur during storage operations
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Library not found")]
    NotFound,

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("No workspace selected")]
    NoWorkspace,
}

/// Abstraction over library storage for desktop vs web
#[allow(dead_code)]
pub trait StorageBackend {
    /// Load the library from the workspace (returns the library and its path)
    fn load_library(&self) -> Result<(Library, PathBuf), StorageError>;

    /// Save a library to the given path
    fn save_library(&self, library: &Library, path: &Path) -> Result<(), StorageError>;

    /// Get current workspace path (None on web)
    fn workspace_path(&self) -> Option<&Path>;

    /// Set the workspace path
    fn set_workspace_path(&mut self, path: PathBuf);
}
