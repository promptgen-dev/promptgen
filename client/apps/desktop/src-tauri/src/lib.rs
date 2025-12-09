//! PromptGen Desktop Application - Tauri Backend
//!
//! This module exposes promptgen-core functionality to the frontend via Tauri commands.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use promptgen_core::{
    load_library as core_load_library, parse_template, render, save_library as core_save_library,
    EvalContext, Library, ParseError, PromptTemplate,
};

// ============================================================================
// State management
// ============================================================================

/// Application state for managing libraries.
pub struct AppState {
    /// Map of library ID -> (Library, path)
    libraries: Mutex<HashMap<String, (Library, PathBuf)>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            libraries: Mutex::new(HashMap::new()),
        }
    }
}

// ============================================================================
// DTOs for frontend communication
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub template_count: usize,
    pub last_modified: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub templates: Vec<TemplateDto>,
    pub wildcards: HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDto {
    pub id: String,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResultDto {
    pub success: bool,
    pub ast: Option<serde_json::Value>,
    pub errors: Option<Vec<ParseErrorDto>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseErrorDto {
    pub message: String,
    pub span: SpanDto,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpanDto {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderInput {
    pub template_id: String,
    pub library_id: String,
    pub bindings: Option<HashMap<String, String>>,
    pub seed: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenderResultDto {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

// ============================================================================
// Conversion helpers
// ============================================================================

impl From<&Library> for LibraryDto {
    fn from(lib: &Library) -> Self {
        LibraryDto {
            id: lib.id.clone(),
            name: lib.name.clone(),
            path: String::new(), // Will be set by caller
            templates: lib.templates.iter().map(TemplateDto::from).collect(),
            wildcards: lib
                .groups
                .iter()
                .map(|g| (g.name.clone(), g.options.clone()))
                .collect(),
        }
    }
}

impl From<&PromptTemplate> for TemplateDto {
    fn from(template: &PromptTemplate) -> Self {
        TemplateDto {
            id: template.id.clone(),
            name: template.name.clone(),
            content: template
                .ast
                .nodes
                .iter()
                .map(|(node, _)| node_to_string(node))
                .collect::<String>(),
        }
    }
}

fn node_to_string(node: &promptgen_core::Node) -> String {
    match node {
        promptgen_core::Node::Text(s) => s.clone(),
        promptgen_core::Node::Comment(s) => format!("# {}", s),
        promptgen_core::Node::Slot(name) => format!("{{{{ {} }}}}", name),
        promptgen_core::Node::LibraryRef(lib_ref) => {
            if let Some(lib) = &lib_ref.library {
                format!("@\"{}:{}\"", lib, lib_ref.group)
            } else if lib_ref.group.contains(' ') {
                format!("@\"{}\"", lib_ref.group)
            } else {
                format!("@{}", lib_ref.group)
            }
        }
        promptgen_core::Node::InlineOptions(opts) => {
            let inner: Vec<String> = opts
                .iter()
                .map(|opt| match opt {
                    promptgen_core::OptionItem::Text(s) => s.clone(),
                    promptgen_core::OptionItem::Nested(nodes) => {
                        nodes.iter().map(|(n, _)| node_to_string(n)).collect()
                    }
                })
                .collect();
            format!("{{{}}}", inner.join("|"))
        }
    }
}

fn parse_error_to_dto(err: &ParseError) -> ParseErrorDto {
    ParseErrorDto {
        message: err.to_string(),
        span: SpanDto {
            start: 0, // ParseError doesn't expose span currently
            end: 0,
        },
    }
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Get the default library directory.
fn get_libraries_dir() -> PathBuf {
    dirs::document_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("PromptGen")
        .join("libraries")
}

/// List all libraries in the default directory.
#[tauri::command]
fn list_libraries(state: tauri::State<AppState>) -> Result<Vec<LibrarySummary>, String> {
    let libs_dir = get_libraries_dir();

    if !libs_dir.exists() {
        return Ok(vec![]);
    }

    let mut summaries = vec![];

    let entries = fs::read_dir(&libs_dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .map(|ext| ext == "yml" || ext == "yaml")
            .unwrap_or(false)
        {
            if let Ok(lib) = core_load_library(&path) {
                let metadata = fs::metadata(&path).ok();
                let last_modified = metadata
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|d| d.as_secs().to_string())
                    })
                    .unwrap_or_default();

                // Store in state
                {
                    let mut libs = state.libraries.lock().unwrap();
                    libs.insert(lib.id.clone(), (lib.clone(), path.clone()));
                }

                summaries.push(LibrarySummary {
                    id: lib.id.clone(),
                    name: lib.name.clone(),
                    path: path.to_string_lossy().to_string(),
                    template_count: lib.templates.len(),
                    last_modified,
                });
            }
        }
    }

    Ok(summaries)
}

/// Load a specific library by ID.
#[tauri::command]
fn load_library(id: String, state: tauri::State<AppState>) -> Result<LibraryDto, String> {
    let libs = state.libraries.lock().unwrap();

    if let Some((lib, path)) = libs.get(&id) {
        let mut dto = LibraryDto::from(lib);
        dto.path = path.to_string_lossy().to_string();
        Ok(dto)
    } else {
        Err(format!("Library not found: {}", id))
    }
}

/// Save a library to disk.
#[tauri::command]
fn save_library(lib: LibraryDto, state: tauri::State<AppState>) -> Result<(), String> {
    let mut libs = state.libraries.lock().unwrap();

    if let Some((existing_lib, path)) = libs.get_mut(&lib.id) {
        // Update the existing library
        existing_lib.name = lib.name;

        // Update templates
        existing_lib.templates.clear();
        for template_dto in lib.templates {
            let ast = parse_template(&template_dto.content).map_err(|e| e.to_string())?;
            existing_lib.templates.push(PromptTemplate::with_id(
                template_dto.id,
                template_dto.name,
                ast,
            ));
        }

        // Update groups/wildcards
        existing_lib.groups.clear();
        for (name, options) in lib.wildcards {
            existing_lib
                .groups
                .push(promptgen_core::PromptGroup::new(name, options));
        }

        // Save to disk
        core_save_library(existing_lib, path).map_err(|e| e.to_string())?;

        Ok(())
    } else {
        Err(format!("Library not found: {}", lib.id))
    }
}

/// Create a new library.
#[tauri::command]
fn create_library(name: String, path: String) -> Result<LibraryDto, String> {
    let lib = Library::new(&name);
    let lib_path = PathBuf::from(&path);

    // Create parent directory if needed
    if let Some(parent) = lib_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Save the library
    core_save_library(&lib, &lib_path).map_err(|e| e.to_string())?;

    let mut dto = LibraryDto::from(&lib);
    dto.path = path;
    Ok(dto)
}

/// Delete a library.
#[tauri::command]
fn delete_library(id: String, state: tauri::State<AppState>) -> Result<(), String> {
    let mut libs = state.libraries.lock().unwrap();

    if let Some((_, path)) = libs.remove(&id) {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!("Library not found: {}", id))
    }
}

/// Parse a template string and return the result.
#[tauri::command]
fn parse_template_cmd(text: String) -> ParseResultDto {
    match parse_template(&text) {
        Ok(_ast) => ParseResultDto {
            success: true,
            ast: None, // TODO: Serialize AST if needed
            errors: None,
        },
        Err(err) => ParseResultDto {
            success: false,
            ast: None,
            errors: Some(vec![parse_error_to_dto(&err)]),
        },
    }
}

/// Render a template with the given bindings.
#[tauri::command]
fn render_template(
    input: RenderInput,
    state: tauri::State<AppState>,
) -> Result<RenderResultDto, String> {
    let libs = state.libraries.lock().unwrap();

    let (library, _) = libs
        .get(&input.library_id)
        .ok_or_else(|| format!("Library not found: {}", input.library_id))?;

    let template = library
        .templates
        .iter()
        .find(|t| t.id == input.template_id)
        .ok_or_else(|| format!("Template not found: {}", input.template_id))?;

    let mut ctx = match input.seed {
        Some(seed) => EvalContext::with_seed(library, seed),
        None => EvalContext::new(library),
    };

    // Add slot bindings if provided
    if let Some(bindings) = input.bindings {
        for (name, value) in bindings {
            ctx.set_slot(&name, &value);
        }
    }

    match render(template, &mut ctx) {
        Ok(result) => Ok(RenderResultDto {
            success: true,
            output: Some(result.text),
            error: None,
        }),
        Err(err) => Ok(RenderResultDto {
            success: false,
            output: None,
            error: Some(err.to_string()),
        }),
    }
}

/// Open a library file from disk.
#[tauri::command]
fn open_file(path: String, state: tauri::State<AppState>) -> Result<LibraryDto, String> {
    let lib_path = PathBuf::from(&path);
    let lib = core_load_library(&lib_path).map_err(|e| e.to_string())?;

    // Store in state
    {
        let mut libs = state.libraries.lock().unwrap();
        libs.insert(lib.id.clone(), (lib.clone(), lib_path));
    }

    let mut dto = LibraryDto::from(&lib);
    dto.path = path;
    Ok(dto)
}

// ============================================================================
// Tauri App Entry Point
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_libraries,
            load_library,
            save_library,
            create_library,
            delete_library,
            parse_template_cmd,
            render_template,
            open_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
