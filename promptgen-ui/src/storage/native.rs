use std::path::{Path, PathBuf};

use promptgen_core::{load_library, save_library as core_save_library};

use super::{StorageBackend, StorageError};

/// Native filesystem storage backend for desktop
pub struct NativeStorage {
    /// Path to the currently loaded library file
    library_path: Option<PathBuf>,
}

impl NativeStorage {
    pub fn new() -> Self {
        Self { library_path: None }
    }

    /// Set the library file path
    pub fn set_library_path(&mut self, path: PathBuf) {
        self.library_path = Some(path);
    }

    /// Get the current library file path
    #[allow(dead_code)]
    pub fn library_path(&self) -> Option<&Path> {
        self.library_path.as_deref()
    }
}

impl Default for NativeStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for NativeStorage {
    fn load_library(&self) -> Result<(promptgen_core::Library, PathBuf), StorageError> {
        let path = self
            .library_path
            .as_ref()
            .ok_or(StorageError::NotFound)?;

        let library = load_library(path).map_err(|e| StorageError::Parse(e.to_string()))?;
        Ok((library, path.clone()))
    }

    fn save_library(
        &self,
        library: &promptgen_core::Library,
        path: &Path,
    ) -> Result<(), StorageError> {
        core_save_library(library, path).map_err(|e| StorageError::Parse(e.to_string()))
    }

    fn workspace_path(&self) -> Option<&Path> {
        self.library_path.as_deref()
    }

    fn set_workspace_path(&mut self, path: PathBuf) {
        self.library_path = Some(path);
    }
}
