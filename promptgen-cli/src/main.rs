use clap::{Parser, Subcommand, ValueEnum};
use promptgen_core::{
    Library,
    eval::{EvalContext, render},
    io::parse_pack,
    parser::parse_template,
};
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
    /// Create a new library in the current directory
    Create {
        /// Name of the library to create
        name: String,
    },

    /// Validate a template and show its structure
    Parse {
        /// Path to the library file
        #[arg(short, long)]
        lib: PathBuf,

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

#[derive(Debug)]
enum CliError {
    Io(std::io::Error),
    Parse(String),
    Render(String),
    InvalidArgs(String),
}

impl CliError {
    fn exit_code(&self) -> ExitCode {
        match self {
            CliError::Io(_) => ExitCode::from(2),
            CliError::Parse(_) => ExitCode::from(1),
            CliError::Render(_) => ExitCode::from(1),
            CliError::InvalidArgs(_) => ExitCode::from(1),
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Io(e) => write!(f, "IO error: {e}"),
            CliError::Parse(msg) => write!(f, "Parse error: {msg}"),
            CliError::Render(msg) => write!(f, "Render error: {msg}"),
            CliError::InvalidArgs(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Create { name } => cmd_create(&name),
        Commands::Parse {
            lib,
            template,
            inline,
            format,
        } => cmd_parse(&lib, template.as_deref(), inline.as_deref(), &format),
        Commands::List { what, lib, format } => cmd_list(&lib, &what, &format),
        Commands::Render {
            lib,
            template,
            inline,
            slots,
            seed,
            format,
        } => cmd_render(
            &lib,
            template.as_deref(),
            inline.as_deref(),
            slots.as_deref(),
            seed,
            &format,
        ),
    }
}

fn cmd_create(name: &str) -> Result<(), CliError> {
    let filename = format!("{name}.yml");
    let path = PathBuf::from(&filename);

    if path.exists() {
        return Err(CliError::InvalidArgs(format!(
            "File '{filename}' already exists"
        )));
    }

    let content = format!(
        r#"id: {name}
name: {name}
description: A new prompt library

groups:
  - tags: [Example]
    options:
      - example option 1
      - example option 2

templates:
  - name: Example Template
    source: "{{Example}}"
"#
    );

    fs::write(&path, content)?;
    println!("Created library: {filename}");
    Ok(())
}

fn load_library(path: &PathBuf) -> Result<Library, CliError> {
    let content = fs::read_to_string(path)?;
    parse_pack(&content).map_err(|e| CliError::Parse(e.to_string()))
}

fn cmd_parse(
    lib_path: &PathBuf,
    template_name: Option<&str>,
    inline: Option<&str>,
    format: &OutputFormat,
) -> Result<(), CliError> {
    // We still need to load the library to find template sources
    let content = fs::read_to_string(lib_path)?;

    let source = match (template_name, inline) {
        (Some(name), None) => {
            // Parse the YAML to get the source text (before it's parsed into AST)
            let pack: serde_yaml_ng::Value =
                serde_yaml_ng::from_str(&content).map_err(|e| CliError::Parse(e.to_string()))?;

            let templates = pack
                .get("templates")
                .and_then(|t| t.as_sequence())
                .ok_or_else(|| CliError::Parse("No templates in library".to_string()))?;

            let template = templates
                .iter()
                .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(name))
                .ok_or_else(|| CliError::InvalidArgs(format!("Template '{name}' not found")))?;

            template
                .get("source")
                .and_then(|s| s.as_str())
                .ok_or_else(|| CliError::Parse(format!("Template '{name}' has no source")))?
                .to_string()
        }
        (None, Some(s)) => s.to_string(),
        (Some(_), Some(_)) => {
            return Err(CliError::InvalidArgs(
                "Cannot specify both --template and --inline".to_string(),
            ));
        }
        (None, None) => {
            return Err(CliError::InvalidArgs(
                "Must specify either --template or --inline".to_string(),
            ));
        }
    };

    let result = parse_template(&source);

    match format {
        OutputFormat::Text => match result {
            Ok(template) => {
                println!("✓ Template is valid\n");
                println!("Nodes ({}):", template.nodes.len());
                for (i, (node, _span)) in template.nodes.iter().enumerate() {
                    println!("  {}: {:?}", i + 1, node);
                }

                // Collect referenced tags
                let mut tags: Vec<&str> = Vec::new();
                for (node, _) in &template.nodes {
                    if let promptgen_core::ast::Node::TagQuery(query) = node {
                        for tag in &query.include {
                            if !tags.contains(&tag.as_str()) {
                                tags.push(tag);
                            }
                        }
                    }
                }
                if !tags.is_empty() {
                    println!("\nReferenced tags: {}", tags.join(", "));
                }

                // Collect freeform slots
                let mut slots: Vec<&str> = Vec::new();
                for (node, _) in &template.nodes {
                    if let promptgen_core::ast::Node::FreeformSlot(name) = node
                        && !slots.contains(&name.as_str())
                    {
                        slots.push(name);
                    }
                }
                if !slots.is_empty() {
                    println!("Freeform slots: {}", slots.join(", "));
                }
            }
            Err(e) => {
                println!("✗ Parse error: {e}");
                return Err(CliError::Parse("Template has errors".to_string()));
            }
        },
        OutputFormat::Json => match result {
            Ok(template) => {
                let output = serde_json::json!({
                    "valid": true,
                    "node_count": template.nodes.len(),
                    "errors": [],
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
            Err(e) => {
                let output = serde_json::json!({
                    "valid": false,
                    "node_count": 0,
                    "errors": [{ "message": e.to_string() }],
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
                return Err(CliError::Parse("Template has errors".to_string()));
            }
        },
    }

    Ok(())
}

fn cmd_list(lib_path: &PathBuf, what: &ListTarget, format: &OutputFormat) -> Result<(), CliError> {
    let library = load_library(lib_path)?;

    match what {
        ListTarget::Groups => match format {
            OutputFormat::Text => {
                println!("Groups in '{}':\n", library.name);
                for (i, group) in library.groups.iter().enumerate() {
                    println!(
                        "  {}. [{}] ({} options)",
                        i + 1,
                        group.tags.join(", "),
                        group.options.len()
                    );
                }
                println!("\nTotal: {} groups", library.groups.len());
            }
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "library": library.name,
                    "groups": library.groups.iter().map(|g| serde_json::json!({
                        "tags": g.tags,
                        "option_count": g.options.len(),
                        "options": g.options.iter().map(|o| &o.text).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
        },
        ListTarget::Templates => match format {
            OutputFormat::Text => {
                println!("Templates in '{}':\n", library.name);
                for (i, tmpl) in library.templates.iter().enumerate() {
                    println!("  {}. {}", i + 1, tmpl.name);
                    if !tmpl.description.is_empty() {
                        println!("     {}", tmpl.description);
                    }
                }
                println!("\nTotal: {} templates", library.templates.len());
            }
            OutputFormat::Json => {
                let output = serde_json::json!({
                    "library": library.name,
                    "templates": library.templates.iter().map(|t| serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                    })).collect::<Vec<_>>(),
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            }
        },
    }

    Ok(())
}

fn cmd_render(
    lib_path: &PathBuf,
    template_name: Option<&str>,
    inline: Option<&str>,
    slots_json: Option<&str>,
    seed: Option<u64>,
    format: &OutputFormat,
) -> Result<(), CliError> {
    let library = load_library(lib_path)?;

    // Parse slots JSON if provided
    let slots: HashMap<String, String> = if let Some(json) = slots_json {
        serde_json::from_str(json)
            .map_err(|e| CliError::InvalidArgs(format!("Invalid slots JSON: {e}")))?
    } else {
        HashMap::new()
    };

    // Create the eval context
    let mut ctx = match seed {
        Some(s) => EvalContext::with_seed(&library, s),
        None => EvalContext::new(&library),
    };

    // Add slot overrides
    for (k, v) in &slots {
        ctx.set_slot(k.clone(), v.clone());
    }

    // Get the template to render
    let template = match (template_name, inline) {
        (Some(name), None) => library
            .templates
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| CliError::InvalidArgs(format!("Template '{name}' not found")))?,
        (None, Some(source)) => {
            // Parse inline source and create a temporary template
            let ast = parse_template(source).map_err(|e| CliError::Parse(e.to_string()))?;
            // We need to create a PromptTemplate, but we can't return a reference to a local
            // So we'll handle this case specially
            let temp_template = promptgen_core::library::PromptTemplate::new("inline", ast);

            let result =
                render(&temp_template, &mut ctx).map_err(|e| CliError::Render(e.to_string()))?;

            match format {
                OutputFormat::Text => {
                    println!("{}", result.text);
                }
                OutputFormat::Json => {
                    let output = serde_json::json!({
                        "output": result.text,
                        "chosen_options": result.chosen_options.iter().map(|c| serde_json::json!({
                            "query": format!("{:?}", c.query),
                            "option": c.option_text,
                        })).collect::<Vec<_>>(),
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
            }
            return Ok(());
        }
        (Some(_), Some(_)) => {
            return Err(CliError::InvalidArgs(
                "Cannot specify both --template and --inline".to_string(),
            ));
        }
        (None, None) => {
            return Err(CliError::InvalidArgs(
                "Must specify either --template or --inline".to_string(),
            ));
        }
    };

    let result = render(template, &mut ctx).map_err(|e| CliError::Render(e.to_string()))?;

    match format {
        OutputFormat::Text => {
            println!("{}", result.text);
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "output": result.text,
                "chosen_options": result.chosen_options.iter().map(|c| serde_json::json!({
                    "query": format!("{:?}", c.query),
                    "option": c.option_text,
                })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
    }

    Ok(())
}
