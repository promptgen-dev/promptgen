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
    /// Current library home directory
    library_home: Mutex<Option<PathBuf>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            libraries: Mutex::new(HashMap::new()),
            library_home: Mutex::new(None),
        }
    }
}

// ============================================================================
// Config persistence
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    library_home: Option<String>,
}

/// Get the path to the config file in the app data directory.
fn get_config_path() -> Option<PathBuf> {
    dirs::data_dir().map(|p| p.join("promptgen").join("config.json"))
}

/// Load the app config from disk.
fn load_config() -> AppConfig {
    get_config_path()
        .and_then(|path| fs::read_to_string(&path).ok())
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

/// Save the app config to disk.
fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path().ok_or("Could not determine config path")?;

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;

    Ok(())
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

/// Get the library home directory from state, or return None if not set.
fn get_library_home(state: &tauri::State<AppState>) -> Option<PathBuf> {
    state.library_home.lock().unwrap().clone()
}

/// Set the library home directory and persist it to config.
#[tauri::command]
fn set_library_home(path: String, state: tauri::State<AppState>) -> Result<(), String> {
    let lib_path = PathBuf::from(&path);

    if !lib_path.exists() {
        return Err(format!("Directory does not exist: {}", path));
    }

    if !lib_path.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    // Clear existing libraries when changing home
    {
        let mut libs = state.libraries.lock().unwrap();
        libs.clear();
    }

    // Set the new home in state
    {
        let mut home = state.library_home.lock().unwrap();
        *home = Some(lib_path);
    }

    // Persist to config file
    let config = AppConfig {
        library_home: Some(path),
    };
    save_config(&config)?;

    Ok(())
}

/// Get the current library home directory.
/// If not set in state, tries to load from persisted config.
#[tauri::command]
fn get_library_home_cmd(state: tauri::State<AppState>) -> Option<String> {
    // First check if we have it in state
    if let Some(path) = get_library_home(&state) {
        return Some(path.to_string_lossy().to_string());
    }

    // Try to load from persisted config
    let config = load_config();
    if let Some(ref path_str) = config.library_home {
        let path = PathBuf::from(path_str);
        // Verify the directory still exists
        if path.exists() && path.is_dir() {
            // Update state with the loaded value
            let mut home = state.library_home.lock().unwrap();
            *home = Some(path);
            return Some(path_str.clone());
        }
    }

    None
}

/// List all libraries in the library home directory.
#[tauri::command]
fn list_libraries(state: tauri::State<AppState>) -> Result<Vec<LibrarySummary>, String> {
    let libs_dir = match get_library_home(&state) {
        Some(dir) => dir,
        None => return Ok(vec![]), // No home set yet
    };

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

/// Create a new library in the library home directory.
#[tauri::command]
fn create_library(name: String, state: tauri::State<AppState>) -> Result<LibraryDto, String> {
    let libs_dir = get_library_home(&state)
        .ok_or_else(|| "No library home set. Please select a folder first.".to_string())?;

    let lib = Library::new(&name);

    // Create filename from name (sanitize for filesystem)
    let filename = format!("{}.yaml", sanitize_filename(&name));
    let lib_path = libs_dir.join(&filename);

    // Check if file already exists
    if lib_path.exists() {
        return Err(format!("A library named '{}' already exists", name));
    }

    // Save the library
    core_save_library(&lib, &lib_path).map_err(|e| e.to_string())?;

    // Store in state
    {
        let mut libs = state.libraries.lock().unwrap();
        libs.insert(lib.id.clone(), (lib.clone(), lib_path.clone()));
    }

    let mut dto = LibraryDto::from(&lib);
    dto.path = lib_path.to_string_lossy().to_string();
    Ok(dto)
}

/// Sanitize a string for use as a filename.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
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
// Prompt Group Commands
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptGroupDto {
    pub name: String,
    pub options: Vec<String>,
}

/// Create a new prompt group in a library.
#[tauri::command]
fn create_prompt_group(
    library_id: String,
    name: String,
    state: tauri::State<AppState>,
) -> Result<PromptGroupDto, String> {
    let mut libs = state.libraries.lock().unwrap();

    if let Some((lib, path)) = libs.get_mut(&library_id) {
        // Check if group already exists
        if lib.find_group(&name).is_some() {
            return Err(format!("A group named '{}' already exists", name));
        }

        // Create new group
        let group = promptgen_core::PromptGroup::new(&name, vec![]);
        lib.groups.push(group);

        // Save to disk
        core_save_library(lib, path).map_err(|e| e.to_string())?;

        Ok(PromptGroupDto {
            name,
            options: vec![],
        })
    } else {
        Err(format!("Library not found: {}", library_id))
    }
}

/// Update a prompt group's options.
#[tauri::command]
fn update_prompt_group(
    library_id: String,
    name: String,
    options: Vec<String>,
    state: tauri::State<AppState>,
) -> Result<PromptGroupDto, String> {
    let mut libs = state.libraries.lock().unwrap();

    if let Some((lib, path)) = libs.get_mut(&library_id) {
        // Find and update the group
        if let Some(group) = lib.groups.iter_mut().find(|g| g.name == name) {
            group.options = options.clone();

            // Save to disk
            core_save_library(lib, path).map_err(|e| e.to_string())?;

            Ok(PromptGroupDto { name, options })
        } else {
            Err(format!("Group not found: {}", name))
        }
    } else {
        Err(format!("Library not found: {}", library_id))
    }
}

/// Rename a prompt group.
#[tauri::command]
fn rename_prompt_group(
    library_id: String,
    old_name: String,
    new_name: String,
    state: tauri::State<AppState>,
) -> Result<PromptGroupDto, String> {
    let mut libs = state.libraries.lock().unwrap();

    if let Some((lib, path)) = libs.get_mut(&library_id) {
        // Check if new name already exists
        if lib.find_group(&new_name).is_some() {
            return Err(format!("A group named '{}' already exists", new_name));
        }

        // Find and rename the group
        if let Some(group) = lib.groups.iter_mut().find(|g| g.name == old_name) {
            group.name = new_name.clone();
            let options = group.options.clone();

            // Save to disk
            core_save_library(lib, path).map_err(|e| e.to_string())?;

            Ok(PromptGroupDto {
                name: new_name,
                options,
            })
        } else {
            Err(format!("Group not found: {}", old_name))
        }
    } else {
        Err(format!("Library not found: {}", library_id))
    }
}

/// Delete a prompt group.
#[tauri::command]
fn delete_prompt_group(
    library_id: String,
    name: String,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let mut libs = state.libraries.lock().unwrap();

    if let Some((lib, path)) = libs.get_mut(&library_id) {
        let initial_len = lib.groups.len();
        lib.groups.retain(|g| g.name != name);

        if lib.groups.len() == initial_len {
            return Err(format!("Group not found: {}", name));
        }

        // Save to disk
        core_save_library(lib, path).map_err(|e| e.to_string())?;

        Ok(())
    } else {
        Err(format!("Library not found: {}", library_id))
    }
}

// ============================================================================
// Template Commands
// ============================================================================

/// Create a new template in a library.
#[tauri::command]
fn create_template(
    library_id: String,
    name: String,
    content: String,
    state: tauri::State<AppState>,
) -> Result<TemplateDto, String> {
    let mut libs = state.libraries.lock().unwrap();

    if let Some((lib, path)) = libs.get_mut(&library_id) {
        // Parse the content
        let ast = parse_template(&content).map_err(|e| e.to_string())?;

        // Create new template
        let template = PromptTemplate::new(&name, ast);
        let id = template.id.clone();
        lib.templates.push(template);

        // Save to disk
        core_save_library(lib, path).map_err(|e| e.to_string())?;

        Ok(TemplateDto { id, name, content })
    } else {
        Err(format!("Library not found: {}", library_id))
    }
}

/// Update a template's content.
#[tauri::command]
fn update_template(
    library_id: String,
    template_id: String,
    name: String,
    content: String,
    state: tauri::State<AppState>,
) -> Result<TemplateDto, String> {
    let mut libs = state.libraries.lock().unwrap();

    if let Some((lib, path)) = libs.get_mut(&library_id) {
        // Parse the content
        let ast = parse_template(&content).map_err(|e| e.to_string())?;

        // Find and update the template
        if let Some(template) = lib.templates.iter_mut().find(|t| t.id == template_id) {
            template.name = name.clone();
            template.ast = ast;

            // Save to disk
            core_save_library(lib, path).map_err(|e| e.to_string())?;

            Ok(TemplateDto {
                id: template_id,
                name,
                content,
            })
        } else {
            Err(format!("Template not found: {}", template_id))
        }
    } else {
        Err(format!("Library not found: {}", library_id))
    }
}

/// Delete a template.
#[tauri::command]
fn delete_template(
    library_id: String,
    template_id: String,
    state: tauri::State<AppState>,
) -> Result<(), String> {
    let mut libs = state.libraries.lock().unwrap();

    if let Some((lib, path)) = libs.get_mut(&library_id) {
        let initial_len = lib.templates.len();
        lib.templates.retain(|t| t.id != template_id);

        if lib.templates.len() == initial_len {
            return Err(format!("Template not found: {}", template_id));
        }

        // Save to disk
        core_save_library(lib, path).map_err(|e| e.to_string())?;

        Ok(())
    } else {
        Err(format!("Library not found: {}", library_id))
    }
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
            set_library_home,
            get_library_home_cmd,
            list_libraries,
            load_library,
            save_library,
            create_library,
            delete_library,
            parse_template_cmd,
            render_template,
            open_file,
            // Prompt group commands
            create_prompt_group,
            update_prompt_group,
            rename_prompt_group,
            delete_prompt_group,
            // Template commands
            create_template,
            update_template,
            delete_template,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
