# PromptGen Desktop

A native desktop application for PromptGen built with Tauri v2.

## Prerequisites

- Node.js >= 18
- pnpm >= 8
- Rust (latest stable)
- Platform-specific dependencies for Tauri:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio C++ Build Tools, WebView2
  - **Linux**: `webkit2gtk`, `libappindicator3`, etc.

See [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/) for detailed setup instructions.

## Development

```bash
# Install dependencies (from client root)
cd ../..
pnpm install

# Run in development mode
pnpm tauri dev
```

The app will hot-reload on frontend changes. Rust changes require a restart.

## Build

```bash
# Build for production
pnpm tauri build
```

Built binaries will be in `src-tauri/target/release/bundle/`.

## Project Structure

```
apps/desktop/
├── src/                    # Frontend React code
│   ├── main.tsx           # React entrypoint
│   ├── index.css          # Tailwind styles
│   └── backend/
│       └── desktop.tsx    # Tauri backend adapter
├── src-tauri/             # Rust/Tauri code
│   ├── src/
│   │   ├── lib.rs        # Tauri commands
│   │   └── main.rs       # App entrypoint
│   ├── capabilities/      # Tauri v2 permissions
│   ├── Cargo.toml
│   └── tauri.conf.json
├── index.html
├── vite.config.ts
├── tailwind.config.js
└── package.json
```

## Tauri Commands

The desktop app exposes these commands to the frontend:

| Command | Description |
|---------|-------------|
| `list_libraries` | List all libraries in the default directory |
| `load_library` | Load a library by ID |
| `save_library` | Save a library to disk |
| `create_library` | Create a new library |
| `delete_library` | Delete a library |
| `parse_template_cmd` | Parse a template string |
| `render_template` | Render a template with bindings |
| `open_file` | Open a library file from disk |

## Configuration

- **Default library directory**: `~/Documents/PromptGen/libraries/`
- **Supported file formats**: `.yml`, `.yaml`

## Debugging

### Frontend
Open DevTools with `Cmd+Option+I` (macOS) or `Ctrl+Shift+I` (Windows/Linux).

### Backend (Rust)
```bash
# Run with debug logging
RUST_LOG=debug pnpm tauri dev
```
