use std::fs;
use std::path::{Path, PathBuf};

use promptgen_core::{load_library, save_library as core_save_library};

use super::{LibrarySummary, StorageBackend, StorageError};

/// Native filesystem storage backend for desktop
pub struct NativeStorage {
    workspace_path: Option<PathBuf>,
}

impl NativeStorage {
    pub fn new() -> Self {
        Self {
            workspace_path: None,
        }
    }
}

impl Default for NativeStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for NativeStorage {
    fn list_libraries(&self) -> Result<Vec<LibrarySummary>, StorageError> {
        let workspace = self
            .workspace_path
            .as_ref()
            .ok_or(StorageError::NoWorkspace)?;

        let mut summaries = Vec::new();

        // Walk the workspace looking for .yaml files that are valid libraries
        if workspace.is_dir() {
            for entry in fs::read_dir(workspace)? {
                let entry = entry?;
                let path = entry.path();

                // Check for .yaml or .yml files
                if path.is_file()
                    && let Some(ext) = path.extension()
                    && (ext == "yaml" || ext == "yml")
                {
                    // Try to load the library to get its info
                    if let Ok(lib) = load_library(&path) {
                        summaries.push(LibrarySummary {
                            id: lib.id.clone(),
                            name: lib.name.clone(),
                            path,
                        });
                    }
                }
            }
        }

        Ok(summaries)
    }

    fn load_library(&self, id: &str) -> Result<promptgen_core::Library, StorageError> {
        let summaries = self.list_libraries()?;

        summaries
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| StorageError::NotFound(id.to_string()))
            .and_then(|summary| {
                load_library(&summary.path).map_err(|e| StorageError::Parse(e.to_string()))
            })
    }

    fn load_all_libraries(&self) -> Result<Vec<promptgen_core::Library>, StorageError> {
        let summaries = self.list_libraries()?;

        summaries
            .into_iter()
            .map(|summary| {
                load_library(&summary.path).map_err(|e| StorageError::Parse(e.to_string()))
            })
            .collect()
    }

    fn save_library(&self, library: &promptgen_core::Library) -> Result<(), StorageError> {
        let workspace = self
            .workspace_path
            .as_ref()
            .ok_or(StorageError::NoWorkspace)?;

        // Save as {library_name}.yaml in the workspace
        let lib_path = workspace.join(format!("{}.yaml", library.name));

        core_save_library(library, &lib_path).map_err(|e| StorageError::Parse(e.to_string()))
    }

    fn delete_library(&self, id: &str) -> Result<(), StorageError> {
        let summaries = self.list_libraries()?;

        let summary = summaries
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| StorageError::NotFound(id.to_string()))?;

        // Delete the file (not directory, since we're using pack format)
        fs::remove_file(&summary.path)?;
        Ok(())
    }

    fn workspace_path(&self) -> Option<&Path> {
        self.workspace_path.as_deref()
    }

    fn set_workspace_path(&mut self, path: PathBuf) {
        self.workspace_path = Some(path);
    }
}
