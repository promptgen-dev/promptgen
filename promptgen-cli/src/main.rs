//! PromptGen CLI
//!
//! Command-line interface for PromptGen, a modular prompt system for generative AI.

use clap::{Parser, Subcommand, ValueEnum};
use promptgen_core::{
    EvalContext, Library, PromptTemplate, RenderError, Workspace,
    io::parse_pack,
    parser::parse_template,
    render,
};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "promptgen")]
#[command(about = "A modular prompt system for generative AI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a template and show its structure
    Parse {
        /// Path to the library file
        #[arg(short, long)]
        lib: Option<PathBuf>,

        /// Name of a template in the library to parse
        #[arg(short, long)]
        template: Option<String>,

        /// Inline template string to parse
        #[arg(short, long)]
        inline: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// List parts of the library
    List {
        /// What to list (groups or templates)
        what: ListTarget,

        /// Path to the library file
        #[arg(short, long)]
        lib: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// Render a template to a final prompt string
    Render {
        /// Path to the library file
        #[arg(short, long)]
        lib: PathBuf,

        /// Name of the template to render
        #[arg(short, long)]
        template: Option<String>,

        /// Inline template string to render
        #[arg(short, long)]
        inline: Option<String>,

        /// Slot values as JSON object (e.g., '{"SceneDescription": "a forest"}')
        #[arg(long)]
        slots: Option<String>,

        /// Random seed for deterministic output
        #[arg(short, long)]
        seed: Option<u64>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,
    },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, ValueEnum)]
enum ListTarget {
    Groups,
    Templates,
}

// ============================================================================
// Error handling
// ============================================================================

#[derive(Debug)]
enum CliError {
    Io(std::io::Error),
    Parse(String),
    Yaml(String),
    Render(RenderError),
    InvalidArgs(String),
    Json(serde_json::Error),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Io(e) => write!(f, "I/O error: {e}"),
            CliError::Parse(e) => write!(f, "Parse error: {e}"),
            CliError::Yaml(e) => write!(f, "YAML error: {e}"),
            CliError::Render(e) => write!(f, "Render error: {e}"),
            CliError::InvalidArgs(e) => write!(f, "Invalid arguments: {e}"),
            CliError::Json(e) => write!(f, "JSON error: {e}"),
        }
    }
}

impl CliError {
    fn exit_code(&self) -> ExitCode {
        match self {
            CliError::Io(_) => ExitCode::from(1),
            CliError::Parse(_) => ExitCode::from(2),
            CliError::Yaml(_) => ExitCode::from(3),
            CliError::Render(_) => ExitCode::from(4),
            CliError::InvalidArgs(_) => ExitCode::from(5),
            CliError::Json(_) => ExitCode::from(6),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}

impl From<promptgen_core::IoError> for CliError {
    fn from(e: promptgen_core::IoError) -> Self {
        CliError::Yaml(e.to_string())
    }
}

impl From<promptgen_core::ParseError<'_>> for CliError {
    fn from(e: promptgen_core::ParseError<'_>) -> Self {
        CliError::Parse(e.to_string())
    }
}

impl From<RenderError> for CliError {
    fn from(e: RenderError) -> Self {
        CliError::Render(e)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(e: serde_json::Error) -> Self {
        CliError::Json(e)
    }
}

// ============================================================================
// Main entry point
// ============================================================================

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            e.exit_code()
        }
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Parse { lib, template, inline, format } => {
            cmd_parse(lib, template, inline, format)
        }
        Commands::List { what, lib, format } => {
            cmd_list(what, lib, format)
        }
        Commands::Render { lib, template, inline, slots, seed, format } => {
            cmd_render(lib, template, inline, slots, seed, format)
        }
    }
}

// ============================================================================
// Parse command
// ============================================================================

#[derive(Serialize)]
struct ParseOutput {
    nodes: Vec<NodeInfo>,
    library_refs: Vec<String>,
    slots: Vec<String>,
}

#[derive(Serialize)]
struct NodeInfo {
    #[serde(rename = "type")]
    node_type: String,
    content: String,
}

fn cmd_parse(
    lib: Option<PathBuf>,
    template: Option<String>,
    inline: Option<String>,
    format: OutputFormat,
) -> Result<(), CliError> {
    let ast = match (&lib, &template, &inline) {
        (Some(lib_path), Some(template_name), None) => {
            // Parse a template from the library
            let content = fs::read_to_string(lib_path)?;
            let library = parse_pack(&content)?;
            let tmpl = library.find_template(template_name).ok_or_else(|| {
                CliError::InvalidArgs(format!("Template '{}' not found in library", template_name))
            })?;
            tmpl.ast.clone()
        }
        (None, None, Some(inline_str)) | (Some(_), None, Some(inline_str)) => {
            // Parse an inline template string
            parse_template(inline_str)?
        }
        _ => {
            return Err(CliError::InvalidArgs(
                "Specify either --template (with --lib) or --inline".to_string(),
            ));
        }
    };

    match format {
        OutputFormat::Text => {
            println!("Template structure:");
            for (node, span) in &ast.nodes {
                let (node_type, content) = describe_node(node);
                println!("  [{}-{}] {}: {}", span.start, span.end, node_type, content);
            }

            // Show library references
            let refs: Vec<_> = ast.nodes.iter().filter_map(|(node, _)| {
                if let promptgen_core::Node::LibraryRef(lib_ref) = node {
                    Some(format_library_ref(lib_ref))
                } else {
                    None
                }
            }).collect();

            if !refs.is_empty() {
                println!("\nLibrary references:");
                for r in &refs {
                    println!("  - {}", r);
                }
            }

            // Show slots
            let slots: Vec<_> = ast.nodes.iter().filter_map(|(node, _)| {
                if let promptgen_core::Node::Slot(name) = node {
                    Some(name.clone())
                } else {
                    None
                }
            }).collect();

            if !slots.is_empty() {
                println!("\nSlots:");
                for s in &slots {
                    println!("  - {{ {} }}", s);
                }
            }
        }
        OutputFormat::Json => {
            let nodes: Vec<NodeInfo> = ast.nodes.iter().map(|(node, _)| {
                let (node_type, content) = describe_node(node);
                NodeInfo { node_type, content }
            }).collect();

            let refs: Vec<String> = ast.nodes.iter().filter_map(|(node, _)| {
                if let promptgen_core::Node::LibraryRef(lib_ref) = node {
                    Some(format_library_ref(lib_ref))
                } else {
                    None
                }
            }).collect();

            let slots: Vec<String> = ast.nodes.iter().filter_map(|(node, _)| {
                if let promptgen_core::Node::Slot(name) = node {
                    Some(name.clone())
                } else {
                    None
                }
            }).collect();

            let output = ParseOutput { nodes, library_refs: refs, slots };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}

fn describe_node(node: &promptgen_core::Node) -> (String, String) {
    match node {
        promptgen_core::Node::Text(text) => ("Text".to_string(), text.clone()),
        promptgen_core::Node::Comment(text) => ("Comment".to_string(), text.clone()),
        promptgen_core::Node::Slot(name) => ("Slot".to_string(), name.clone()),
        promptgen_core::Node::LibraryRef(lib_ref) => {
            ("LibraryRef".to_string(), format_library_ref(lib_ref))
        }
        promptgen_core::Node::InlineOptions(options) => {
            let items: Vec<String> = options.iter().map(|opt| {
                match opt {
                    promptgen_core::OptionItem::Text(t) => t.clone(),
                    promptgen_core::OptionItem::Nested(_) => "[nested]".to_string(),
                }
            }).collect();
            ("InlineOptions".to_string(), items.join(" | "))
        }
    }
}

fn format_library_ref(lib_ref: &promptgen_core::LibraryRef) -> String {
    match &lib_ref.library {
        Some(lib) => format!("{}:{}", lib, lib_ref.group),
        None => lib_ref.group.clone(),
    }
}

// ============================================================================
// List command
// ============================================================================

#[derive(Serialize)]
struct GroupInfo {
    name: String,
    option_count: usize,
}

#[derive(Serialize)]
struct TemplateInfo {
    id: String,
    name: String,
    description: String,
}

fn cmd_list(what: ListTarget, lib: PathBuf, format: OutputFormat) -> Result<(), CliError> {
    let content = fs::read_to_string(&lib)?;
    let library = parse_pack(&content)?;

    match what {
        ListTarget::Groups => list_groups(&library, format),
        ListTarget::Templates => list_templates(&library, format),
    }
}

fn list_groups(library: &Library, format: OutputFormat) -> Result<(), CliError> {
    match format {
        OutputFormat::Text => {
            println!("Groups in '{}':", library.name);
            for group in &library.groups {
                println!("  {} ({} options)", group.name, group.options.len());
            }
        }
        OutputFormat::Json => {
            let groups: Vec<GroupInfo> = library.groups.iter().map(|g| {
                GroupInfo {
                    name: g.name.clone(),
                    option_count: g.options.len(),
                }
            }).collect();
            println!("{}", serde_json::to_string_pretty(&groups)?);
        }
    }
    Ok(())
}

fn list_templates(library: &Library, format: OutputFormat) -> Result<(), CliError> {
    match format {
        OutputFormat::Text => {
            println!("Templates in '{}':", library.name);
            for tmpl in &library.templates {
                if tmpl.description.is_empty() {
                    println!("  {}", tmpl.name);
                } else {
                    println!("  {} - {}", tmpl.name, tmpl.description);
                }
            }
        }
        OutputFormat::Json => {
            let templates: Vec<TemplateInfo> = library.templates.iter().map(|t| {
                TemplateInfo {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    description: t.description.clone(),
                }
            }).collect();
            println!("{}", serde_json::to_string_pretty(&templates)?);
        }
    }
    Ok(())
}

// ============================================================================
// Render command
// ============================================================================

#[derive(Serialize)]
struct RenderOutput {
    prompt: String,
    chosen_options: Vec<ChosenOptionInfo>,
}

#[derive(Serialize)]
struct ChosenOptionInfo {
    group: String,
    library: Option<String>,
    option: String,
}

fn cmd_render(
    lib: PathBuf,
    template: Option<String>,
    inline: Option<String>,
    slots: Option<String>,
    seed: Option<u64>,
    format: OutputFormat,
) -> Result<(), CliError> {
    let content = fs::read_to_string(&lib)?;
    let library = parse_pack(&content)?;

    let tmpl: PromptTemplate = match (&template, &inline) {
        (Some(template_name), None) => {
            library.find_template(template_name).ok_or_else(|| {
                CliError::InvalidArgs(format!("Template '{}' not found in library", template_name))
            })?.clone()
        }
        (None, Some(inline_str)) => {
            let ast = parse_template(inline_str).map_err(|e| CliError::Parse(e.to_string()))?;
            PromptTemplate::new("inline", ast)
        }
        _ => {
            return Err(CliError::InvalidArgs(
                "Specify either --template or --inline".to_string(),
            ));
        }
    };

    // Parse slot overrides
    let slot_overrides: HashMap<String, String> = if let Some(slots_json) = slots {
        serde_json::from_str(&slots_json)?
    } else {
        HashMap::new()
    };

    // Create workspace with the library for evaluation
    let workspace = Workspace::with_single_library(library);

    // Create evaluation context
    let mut ctx = match seed {
        Some(s) => EvalContext::with_seed(&workspace, s),
        None => EvalContext::new(&workspace),
    };
    for (k, v) in slot_overrides {
        ctx.set_slot(&k, v);
    }

    // Render the template
    let result = render(&tmpl.ast, &mut ctx)?;

    match format {
        OutputFormat::Text => {
            println!("{}", result.text);
        }
        OutputFormat::Json => {
            let output = RenderOutput {
                prompt: result.text,
                chosen_options: result.chosen_options.into_iter().map(|c| {
                    ChosenOptionInfo {
                        group: c.group_name,
                        library: c.library_name,
                        option: c.option_text,
                    }
                }).collect(),
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}
