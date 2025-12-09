# PromptGen Client

This directory contains the frontend applications and shared packages for PromptGen.

## Structure

```
client/
├── apps/
│   ├── desktop/     # Tauri v2 desktop application
│   └── web/         # Web application (planned)
└── packages/
    ├── backend/     # Shared types and backend interface
    └── ui/          # Shared React UI components
```

## Prerequisites

- Node.js >= 18
- pnpm >= 8
- Rust (for Tauri desktop app)
- Tauri CLI v2

## Installation

```bash
# From the client directory
pnpm install
```

## Development

### Desktop App (Tauri)

```bash
# Run the desktop app in development mode
pnpm dev:desktop

# Or from the desktop app directory
cd apps/desktop
pnpm tauri dev
```

### Build

```bash
# Build the desktop app
pnpm build:desktop

# Or from the desktop app directory
cd apps/desktop
pnpm tauri build
```

## Package Scripts

| Command | Description |
|---------|-------------|
| `pnpm dev:desktop` | Start desktop app in dev mode |
| `pnpm dev:web` | Start web app in dev mode (when available) |
| `pnpm build:desktop` | Build desktop app for production |
| `pnpm build:web` | Build web app for production (when available) |

## Architecture

The client uses a shared UI pattern:

- **`@promptgen/backend`**: Defines the `PromptgenBackend` interface and shared types
- **`@promptgen/ui`**: Contains all React components, Zustand stores, and hooks
- **Desktop/Web apps**: Provide platform-specific backend implementations

This allows the same UI to work across desktop (Tauri) and web (future) platforms.

## Technology Stack

- React 19
- TypeScript
- Tailwind CSS + shadcn/ui
- Zustand (state management)
- Vite (bundler)
- Tauri v2 (desktop framework)
