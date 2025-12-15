#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub use native::NativeStorage;

use std::path::{Path, PathBuf};

use promptgen_core::Library;
use thiserror::Error;

use crate::state::AppConfig;

/// Summary information about a library (for listing without full load)
#[derive(Debug, Clone)]
pub struct LibrarySummary {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

/// Errors that can occur during storage operations
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Library not found: {0}")]
    NotFound(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("No workspace selected")]
    NoWorkspace,
}

/// Abstraction over library storage for desktop vs web
pub trait StorageBackend {
    /// List all libraries in the workspace
    fn list_libraries(&self) -> Result<Vec<LibrarySummary>, StorageError>;

    /// Load a library by ID
    fn load_library(&self, id: &str) -> Result<Library, StorageError>;

    /// Load all libraries from the workspace
    fn load_all_libraries(&self) -> Result<Vec<Library>, StorageError>;

    /// Save a library
    fn save_library(&self, library: &Library) -> Result<(), StorageError>;

    /// Delete a library by ID
    fn delete_library(&self, id: &str) -> Result<(), StorageError>;

    /// Get current workspace path (None on web)
    fn workspace_path(&self) -> Option<&Path>;

    /// Set the workspace path
    fn set_workspace_path(&mut self, path: PathBuf);

    /// Load application config
    fn load_config(&self) -> AppConfig;

    /// Save application config
    fn save_config(&self, config: &AppConfig) -> Result<(), StorageError>;
}

/// Create the appropriate storage backend for the current platform
#[cfg(not(target_arch = "wasm32"))]
pub fn create_storage() -> Box<dyn StorageBackend> {
    Box::new(NativeStorage::new())
}
