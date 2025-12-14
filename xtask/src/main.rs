//! Build tasks for the promptgen workspace.
//!
//! Usage:
//!   cargo xtask build-wasm    Build the WASM module
//!   cargo xtask help          Show help

use std::env;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let task = args.get(1).map(String::as_str);

    match task {
        Some("build-wasm") => build_wasm()?,
        Some("help") | None => print_help(),
        Some(other) => bail!("Unknown task: {}. Run 'cargo xtask help' for usage.", other),
    }

    Ok(())
}

fn print_help() {
    eprintln!(
        r#"
promptgen xtask - Build tasks for the promptgen workspace

USAGE:
    cargo xtask <COMMAND>

COMMANDS:
    build-wasm    Build the WASM module (requires wasm-pack)
    help          Show this help message

EXAMPLES:
    cargo xtask build-wasm
"#
    );
}

/// Build the promptgen-core WASM module.
fn build_wasm() -> Result<()> {
    let workspace_root = workspace_root()?;
    let core_dir = workspace_root.join("promptgen-core");
    let output_dir = workspace_root.join("client/packages/core-wasm/src/wasm");

    println!("Building promptgen-core WASM module...");
    println!("  Core directory: {}", core_dir.display());
    println!("  Output directory: {}", output_dir.display());

    // Check if wasm-pack is installed
    let wasm_pack_check = Command::new("wasm-pack").arg("--version").output();

    if wasm_pack_check.is_err() {
        bail!(
            "wasm-pack is not installed. Install it with:\n\
             cargo install wasm-pack\n\
             \n\
             Or see: https://rustwasm.github.io/wasm-pack/installer/"
        );
    }

    // Create output directory
    std::fs::create_dir_all(&output_dir)
        .context("Failed to create output directory")?;

    // Run wasm-pack build
    let status = Command::new("wasm-pack")
        .arg("build")
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(&output_dir)
        .arg("--features")
        .arg("wasm")
        .arg("--no-default-features")
        .current_dir(&core_dir)
        .status()
        .context("Failed to run wasm-pack")?;

    if !status.success() {
        bail!("wasm-pack build failed");
    }

    // Clean up unnecessary files
    let files_to_remove = [".gitignore", "package.json", "README.md"];
    for file in &files_to_remove {
        let path = output_dir.join(file);
        if path.exists() {
            std::fs::remove_file(&path).ok();
        }
    }

    println!();
    println!("Build complete! WASM files written to:");
    println!("  {}", output_dir.display());
    println!();
    println!("To use in your project:");
    println!("  import init, {{ WasmWorkspace, WasmWorkspaceBuilder }} from './wasm/promptgen_core';");
    println!("  await init();");

    Ok(())
}

/// Get the workspace root directory.
fn workspace_root() -> Result<PathBuf> {
    // The xtask binary is at target/debug/xtask or target/release/xtask
    // We need to find the workspace root, which contains Cargo.toml
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .or_else(|_| {
            // Fallback: use the current exe location and go up
            env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Cannot find workspace root"))
        })?;

    // CARGO_MANIFEST_DIR points to xtask/, so go up one level
    let workspace_root = manifest_dir.parent()
        .context("Cannot find workspace root")?
        .to_path_buf();

    // Verify it's the workspace root by checking for Cargo.toml with [workspace]
    let cargo_toml = workspace_root.join("Cargo.toml");
    if !cargo_toml.exists() {
        bail!("Cannot find workspace Cargo.toml at {}", cargo_toml.display());
    }

    Ok(workspace_root)
}
