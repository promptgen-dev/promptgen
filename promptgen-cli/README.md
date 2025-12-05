# PromptGen CLI

A command-line interface for PromptGen - a modular prompt system for generative AI.

## Installation

### From source

```bash
cargo install --path .
```

This installs the `promptgen` binary to your Cargo bin directory.

### Development / Local Testing

Run commands directly with cargo:

```bash
cargo run -p promptgen-cli -- <command> [options]
```

## Commands

### `promptgen create <name>`

Create a new library file in the current directory.

```bash
# Creates my-library.yml with example groups and templates
promptgen create my-library
```

### `promptgen list <groups|templates> -l <path>`

List groups or templates in a library.

```bash
# List all groups
promptgen list groups -l example.yml

# List all templates
promptgen list templates -l example.yml

# Output as JSON (for scripting/editor integration)
promptgen list groups -l example.yml -f json
```

### `promptgen parse -l <path> [-t <name> | -i <source>]`

Validate and inspect a template's structure.

```bash
# Parse a template from the library
promptgen parse -l example.yml -t "Character"

# Parse an inline template string
promptgen parse -l example.yml -i '{Hair} with {{ EyeColor }} eyes'

# Output as JSON
promptgen parse -l example.yml -i '{Hair}' -f json
```

### `promptgen render -l <path> [-t <name> | -i <source>] [options]`

Render a template to a final prompt string.

```bash
# Render a named template
promptgen render -l example.yml -t "Character"

# Render an inline template
promptgen render -l example.yml -i '{Hair}, {Eyes}'

# Use a specific seed for reproducible output
promptgen render -l example.yml -t "Character" -s 42

# Provide values for freeform slots
promptgen render -l example.yml -i '{Hair} in {{ Scene }}' \
  --slots '{"Scene": "a dark forest"}'

# Output as JSON (includes chosen options)
promptgen render -l example.yml -t "Character" -f json
```

## Options

Common options available across commands:

| Short | Long | Description |
|-------|------|-------------|
| `-l` | `--lib` | Path to the library file |
| `-t` | `--template` | Template name |
| `-i` | `--inline` | Inline template source |
| `-s` | `--seed` | Random seed for deterministic output |
| `-f` | `--format` | Output format (`text` or `json`) |

## Output Formats

All commands support `-f`/`--format` with two options:

- `text` (default) - Human-readable output
- `json` - Machine-readable JSON for scripting and editor integration

## Exit Codes

- `0` - Success
- `1` - Parse/validation error or invalid arguments
- `2` - IO error (file not found, etc.)

## Example Library

An example library (`example.yml`) is included in this directory:

```yaml
id: example
name: example
description: Example Prompts

groups:
  - tags: [Hair, appearance]
    options:
      - blonde hair
      - red hair
      - black hair
  - tags: [Eyes]
    options:
      - blue eyes
      - green eyes

templates:
  - name: Character
    description: A basic character description
    source: "{Hair}, {Eyes}"
```

## Template Syntax

- `{Tag}` - Select randomly from groups with this tag
- `{Tag1 + Tag2}` - Select from groups with Tag1 OR Tag2
- `{Tag - exclude}` - Select from Tag groups, excluding groups tagged "exclude"
- `{{ SlotName }}` - Freeform slot for user input
- `# comment` - Comments (ignored in output)
- `[[ "Tag" | some | assign("var") ]]` - Expression blocks with pipelines

## Development Testing

```bash
# Build the CLI
cargo build -p promptgen-cli

# Run tests for the entire workspace
cargo test

# Test with the included example library
cargo run -p promptgen-cli -- list groups -l promptgen-cli/example.yml
cargo run -p promptgen-cli -- list templates -l promptgen-cli/example.yml
cargo run -p promptgen-cli -- parse -l promptgen-cli/example.yml -t "Character"
cargo run -p promptgen-cli -- render -l promptgen-cli/example.yml -t "Character" -s 42
```
