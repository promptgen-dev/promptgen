# PromptGen

A modular prompt system for generative AI with support for hierarchical templates, library references, and random selection.

## Project Structure

```
promptgen/
├── promptgen-core/     # Core Rust library (parsing, evaluation, WASM)
├── promptgen-cli/      # Command-line interface
├── client/             # Desktop & web UI (Tauri + React)
│   ├── apps/desktop/   # Tauri desktop application
│   └── packages/       # Shared packages
├── xtask/              # Build automation (Rust)
└── .cargo/config.toml  # Cargo aliases
```

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) 18+ and [pnpm](https://pnpm.io/)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (for WASM builds)

### Development Commands

```bash
# Run all tests
cargo test

# Run CLI
cargo run -p promptgen-cli -- <command>

# Run desktop app
cd client && pnpm dev:desktop
```

## Build Tasks (xtask)

Build automation is handled by the `xtask` crate, which provides cross-platform Rust scripts.

### Available Tasks

```bash
# Build the WASM module for browser/web usage
cargo xtask build-wasm

# Show help
cargo xtask help
```

### WASM Build

The `build-wasm` task compiles `promptgen-core` to WebAssembly:

```bash
# Install wasm-pack (one-time)
cargo install wasm-pack

# Build WASM module
cargo xtask build-wasm
```

Output files are written to `client/packages/core-wasm/src/wasm/`:
- `promptgen_core.js` - JavaScript bindings
- `promptgen_core.d.ts` - TypeScript definitions
- `promptgen_core_bg.wasm` - WASM binary

## Packages

### promptgen-core

The core library providing:
- Template parsing (using Chumsky parser combinator)
- Multi-library workspace management
- Template validation with "Did you mean?" suggestions
- Autocomplete support
- Deterministic rendering with seeded RNG
- WASM bindings for browser usage

### promptgen-cli

Command-line interface for:
- Creating new libraries
- Listing groups and templates
- Parsing and validating templates
- Rendering prompts

See [promptgen-cli/README.md](promptgen-cli/README.md) for usage.

### Desktop App

Tauri-based desktop application with:
- Library management
- Visual template editor (CodeMirror 6)
- Real-time preview
- Import/export

## Template Syntax

```
@Hair                    # Reference a group by name
@"Eye Color"             # Quoted name (spaces allowed)
@"MyLib:Hair"            # Qualified reference (specific library)
{blonde|red|black}       # Inline random selection
{{ CharacterName }}      # Freeform slot (user input)
# This is a comment      # Comments are ignored
```

## Testing

```bash
# Run all workspace tests
cargo test

# Run specific package tests
cargo test -p promptgen-core
cargo test -p promptgen-cli

# Run tests with WASM feature
cargo test -p promptgen-core --features wasm
```

## License

MIT
