//! WebAssembly bindings for promptgen-core.
//!
//! This module provides JavaScript-callable functions for:
//! - Building workspaces from library data
//! - Parsing and validating templates
//! - Getting autocomplete suggestions
//! - Rendering templates
//!
//! All complex types are serialized via serde-wasm-bindgen.

use wasm_bindgen::prelude::*;

use crate::eval::{self, EvalContext};
use crate::library::{Library, PromptGroup};
use crate::workspace::{DiagnosticError, DiagnosticWarning, Workspace, WorkspaceBuilder};

/// Initialize panic hook for better error messages in browser console.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

// ============================================================================
// Library Input (JS -> Rust)
// ============================================================================

/// Library data passed from JavaScript.
/// This is the input format - we convert it to the internal Library type.
#[derive(serde::Deserialize)]
pub struct LibraryInput {
    pub id: String,
    pub name: String,
    pub groups: Vec<GroupInput>,
}

#[derive(serde::Deserialize)]
pub struct GroupInput {
    pub name: String,
    pub options: Vec<String>,
}

impl From<LibraryInput> for Library {
    fn from(input: LibraryInput) -> Self {
        let mut lib = Library::with_id(input.id, input.name);
        for group in input.groups {
            lib.groups.push(PromptGroup::with_options(group.name, group.options));
        }
        lib
    }
}

// ============================================================================
// WASM Workspace
// ============================================================================

/// A workspace that can be used from JavaScript.
/// Wraps the Rust Workspace and provides wasm_bindgen-compatible methods.
#[wasm_bindgen]
pub struct WasmWorkspace {
    inner: Workspace,
}

#[wasm_bindgen]
impl WasmWorkspace {
    /// Create an empty workspace.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Workspace::new(),
        }
    }

    /// Add or update a library in the workspace.
    /// Returns a new workspace (immutable update).
    #[wasm_bindgen(js_name = withLibrary)]
    pub fn with_library(&self, library_js: JsValue) -> Result<WasmWorkspace, JsError> {
        let input: LibraryInput = serde_wasm_bindgen::from_value(library_js)?;
        let library: Library = input.into();
        Ok(Self {
            inner: self.inner.with_library(library),
        })
    }

    /// Remove a library from the workspace.
    /// Returns a new workspace (immutable update).
    #[wasm_bindgen(js_name = withoutLibrary)]
    pub fn without_library(&self, library_id: &str) -> WasmWorkspace {
        Self {
            inner: self.inner.without_library(library_id),
        }
    }

    /// Get all library IDs in the workspace.
    #[wasm_bindgen(js_name = getLibraryIds)]
    pub fn get_library_ids(&self) -> Vec<String> {
        self.inner.library_ids().into_iter().map(String::from).collect()
    }

    /// Parse and validate a template against the workspace.
    /// Returns ParseResult with AST (if valid) and any errors/warnings.
    #[wasm_bindgen(js_name = parseTemplate)]
    pub fn parse_template(&self, source: &str) -> Result<JsValue, JsError> {
        let result = self.inner.parse_template(source);

        // Convert to a JS-friendly format
        let js_result = JsParseResult {
            has_ast: result.ast.is_some(),
            errors: result.errors,
            warnings: result.warnings,
        };

        Ok(serde_wasm_bindgen::to_value(&js_result)?)
    }

    /// Get autocomplete suggestions at a cursor position.
    #[wasm_bindgen(js_name = getCompletions)]
    pub fn get_completions(&self, source: &str, cursor_pos: usize) -> Result<JsValue, JsError> {
        let completions = self.inner.get_completions(source, cursor_pos);
        Ok(serde_wasm_bindgen::to_value(&completions)?)
    }

    /// Render a template with the given slot values and optional seed.
    /// Returns RenderResult with the output text and chosen options.
    #[wasm_bindgen]
    pub fn render(
        &self,
        source: &str,
        slot_values_js: JsValue,
        seed: Option<u64>,
    ) -> Result<JsValue, JsError> {
        // Parse the template
        let parse_result = self.inner.parse_template(source);

        let ast = parse_result.ast.ok_or_else(|| {
            JsError::new(&format!(
                "Template has parse errors: {:?}",
                parse_result.errors.first().map(|e| &e.message)
            ))
        })?;

        if !parse_result.errors.is_empty() {
            return Err(JsError::new(&format!(
                "Template has validation errors: {:?}",
                parse_result.errors.first().map(|e| &e.message)
            )));
        }

        // Deserialize slot values
        let slot_values: std::collections::HashMap<String, String> =
            serde_wasm_bindgen::from_value(slot_values_js)?;

        // Create eval context with seed
        let mut ctx = EvalContext::with_seed(&self.inner, seed.unwrap_or(0));
        for (name, value) in slot_values {
            ctx.set_slot(name, value);
        }

        // Render
        let result = eval::render(&ast, &mut ctx)
            .map_err(|e| JsError::new(&format!("Render error: {}", e)))?;

        // Convert to JS-friendly format
        let js_result = JsRenderResult {
            output: result.text,
            choices: result
                .chosen_options
                .into_iter()
                .map(|c| JsChoiceRecord {
                    ref_name: format!(
                        "{}{}",
                        c.library_name.map(|l| format!("{}:", l)).unwrap_or_default(),
                        c.group_name
                    ),
                    chosen: c.option_text,
                    index: c.option_index,
                })
                .collect(),
        };

        Ok(serde_wasm_bindgen::to_value(&js_result)?)
    }

    /// Extract slot names from a template source.
    #[wasm_bindgen(js_name = getSlots)]
    pub fn get_slots(&self, source: &str) -> Result<Vec<String>, JsError> {
        let parse_result = self.inner.parse_template(source);

        match parse_result.ast {
            Some(ast) => Ok(self.inner.get_slots(&ast)),
            None => Err(JsError::new("Cannot extract slots from invalid template")),
        }
    }

    /// Extract library references from a template source.
    #[wasm_bindgen(js_name = getReferences)]
    pub fn get_references(&self, source: &str) -> Result<JsValue, JsError> {
        let parse_result = self.inner.parse_template(source);

        match parse_result.ast {
            Some(ast) => {
                let refs = self.inner.get_references(&ast);
                Ok(serde_wasm_bindgen::to_value(&refs)?)
            }
            None => Err(JsError::new("Cannot extract references from invalid template")),
        }
    }

    /// Get all group names across all libraries.
    #[wasm_bindgen(js_name = getGroupNames)]
    pub fn get_group_names(&self, library_id: Option<String>) -> Result<JsValue, JsError> {
        let groups = self.inner.group_names(library_id.as_deref());
        Ok(serde_wasm_bindgen::to_value(&groups)?)
    }

    /// Search for groups matching the query across all libraries.
    ///
    /// Returns all groups if query is empty. Results are sorted by score (highest first).
    /// Search is case-insensitive.
    #[wasm_bindgen(js_name = searchGroups)]
    pub fn search_groups(&self, query: &str) -> Result<JsValue, JsError> {
        let results = self.inner.search_groups(query);
        Ok(serde_wasm_bindgen::to_value(&results)?)
    }

    /// Search for options matching the query, optionally filtered to a specific group.
    ///
    /// Returns all options if query is empty. Results are sorted by best match score.
    /// Search is case-insensitive.
    #[wasm_bindgen(js_name = searchOptions)]
    pub fn search_options(
        &self,
        query: &str,
        group_filter: Option<String>,
    ) -> Result<JsValue, JsError> {
        let results = self.inner.search_options(query, group_filter.as_deref());
        Ok(serde_wasm_bindgen::to_value(&results)?)
    }

    /// Unified search with syntax parsing.
    ///
    /// Supports the following query syntax:
    /// - `@group` or `@group_query` - Search for groups
    /// - `@group/option` - Search for options within a specific group
    /// - `@/option` - Search for options across all groups
    /// - Plain text without `@` prefix - Search for groups (default)
    #[wasm_bindgen]
    pub fn search(&self, query: &str) -> Result<JsValue, JsError> {
        let result = self.inner.search(query);
        Ok(serde_wasm_bindgen::to_value(&result)?)
    }
}

impl Default for WasmWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// WASM Workspace Builder
// ============================================================================

/// Builder for constructing a WasmWorkspace.
#[wasm_bindgen]
pub struct WasmWorkspaceBuilder {
    inner: WorkspaceBuilder,
}

#[wasm_bindgen]
impl WasmWorkspaceBuilder {
    /// Create a new workspace builder.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: WorkspaceBuilder::new(),
        }
    }

    /// Add a library to the workspace builder.
    #[wasm_bindgen(js_name = addLibrary)]
    pub fn add_library(self, library_js: JsValue) -> Result<WasmWorkspaceBuilder, JsError> {
        let input: LibraryInput = serde_wasm_bindgen::from_value(library_js)?;
        let library: Library = input.into();
        Ok(Self {
            inner: self.inner.add_library(library),
        })
    }

    /// Build the workspace.
    pub fn build(self) -> WasmWorkspace {
        WasmWorkspace {
            inner: self.inner.build(),
        }
    }
}

impl Default for WasmWorkspaceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// JS-Friendly Result Types
// ============================================================================

/// Parse result in JS-friendly format.
#[derive(serde::Serialize)]
struct JsParseResult {
    /// Whether the AST was successfully parsed (use render/getSlots/getReferences to access it).
    has_ast: bool,
    /// Errors encountered during parsing/validation.
    errors: Vec<DiagnosticError>,
    /// Warnings (non-blocking issues).
    warnings: Vec<DiagnosticWarning>,
}

/// Render result in JS-friendly format.
#[derive(serde::Serialize)]
struct JsRenderResult {
    /// The rendered output text.
    output: String,
    /// Choices made during rendering.
    choices: Vec<JsChoiceRecord>,
}

/// Record of a choice made during rendering.
#[derive(serde::Serialize)]
struct JsChoiceRecord {
    /// The reference that was resolved (e.g., "Hair" or "MyLib:Hair").
    ref_name: String,
    /// The chosen option text.
    chosen: String,
    /// The index of the chosen option.
    index: usize,
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Parse a template source without a workspace (for syntax checking only).
/// Returns parse errors but cannot validate library references.
#[wasm_bindgen(js_name = parseTemplateSource)]
pub fn parse_template_source(source: &str) -> Result<JsValue, JsError> {
    use crate::parser::parse_template;

    match parse_template(source) {
        Ok(_) => {
            let result = JsParseResult {
                has_ast: true,
                errors: vec![],
                warnings: vec![],
            };
            Ok(serde_wasm_bindgen::to_value(&result)?)
        }
        Err(e) => {
            let result = JsParseResult {
                has_ast: false,
                errors: vec![DiagnosticError {
                    message: format!("Parse error: {}", e),
                    span: 0..source.len(),
                    kind: crate::workspace::ErrorKind::Syntax,
                    suggestion: None,
                }],
                warnings: vec![],
            };
            Ok(serde_wasm_bindgen::to_value(&result)?)
        }
    }
}
